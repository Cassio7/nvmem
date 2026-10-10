//! Looks up the latest version and deprecation notices of packages on the npm
//! registry. Answers are cached on disk so the default command stays fast:
//! most runs make no network requests at all.

use std::collections::{HashMap, HashSet};
use std::path::{Path, PathBuf};
use std::sync::Mutex;
use std::sync::atomic::{AtomicUsize, Ordering};
use std::time::{Duration, SystemTime, UNIX_EPOCH};
use std::{fs, thread};

use serde::{Deserialize, Serialize};

/// How long a registry answer is reused before asking again.
pub const CACHE_TTL: Duration = Duration::from_secs(6 * 60 * 60);
const REQUEST_TIMEOUT: Duration = Duration::from_secs(3);
const MAX_PARALLEL_REQUESTS: usize = 16;

/// The parts of a package manifest we care about.
#[derive(Debug, Clone, Default, PartialEq, Eq, Serialize, Deserialize)]
pub struct Manifest {
    pub version: Option<String>,
    pub deprecated: Option<String>,
}

/// Result of asking the registry for one manifest.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Fetched {
    Found(Manifest),
    /// The registry doesn't know it (e.g. a private package).
    Missing,
    /// Network error, timeout or unexpected response.
    Failed,
}

/// What the registry says about one installed package version.
#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct Info {
    /// Set only when the registry's `latest` is newer than the installed version.
    pub update: Option<String>,
    /// Deprecation message of the installed version.
    pub deprecated: Option<String>,
}

#[derive(Debug, Default)]
pub struct Lookup {
    /// Keyed by `(package name, installed version)`.
    pub info: HashMap<(String, String), Info>,
    /// True when some answers are missing because the registry couldn't be
    /// reached and nothing was cached.
    pub incomplete: bool,
}

#[derive(Debug, Default, Serialize, Deserialize)]
struct Cache {
    entries: HashMap<String, CacheEntry>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
struct CacheEntry {
    /// `None` when the registry answered 404.
    manifest: Option<Manifest>,
    fetched_at: u64,
}

/// Registry path for a manifest: `name/latest` or `name/1.2.3`, with the
/// scope separator escaped as the registry expects (`@scope%2Fname/latest`).
fn manifest_key(name: &str, version: &str) -> String {
    format!("{}/{version}", name.replace('/', "%2F"))
}

/// Looks up `(name, installed version)` pairs, using the cache at
/// `cache_path` for answers younger than `ttl` (all of them are refetched
/// when `refresh`). `fetch` is called in parallel for everything else.
pub fn lookup(
    packages: &[(String, String)],
    cache_path: Option<&Path>,
    ttl: Duration,
    refresh: bool,
    fetch: impl Fn(&str) -> Fetched + Sync,
) -> Lookup {
    let now = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map_or(0, |d| d.as_secs());
    let mut cache: Cache = cache_path
        .and_then(|p| fs::read_to_string(p).ok())
        .and_then(|s| serde_json::from_str(&s).ok())
        .unwrap_or_default();

    let mut keys: Vec<String> = Vec::new();
    let mut seen = HashSet::new();
    for (name, version) in packages {
        for key in [manifest_key(name, "latest"), manifest_key(name, version)] {
            if seen.insert(key.clone()) {
                keys.push(key);
            }
        }
    }

    let is_fresh =
        |entry: &CacheEntry| !refresh && now.saturating_sub(entry.fetched_at) < ttl.as_secs();
    let stale: Vec<String> = keys
        .iter()
        .filter(|k| !cache.entries.get(*k).is_some_and(is_fresh))
        .cloned()
        .collect();

    let mut changed = false;
    let mut failed = HashSet::new();
    for (key, result) in fetch_all(&stale, &fetch) {
        let manifest = match result {
            Fetched::Found(m) => Some(m),
            Fetched::Missing => None,
            // Keep any older cached answer: better slightly stale than nothing.
            Fetched::Failed => {
                if !cache.entries.contains_key(&key) {
                    failed.insert(key);
                }
                continue;
            }
        };
        cache.entries.insert(
            key,
            CacheEntry {
                manifest,
                fetched_at: now,
            },
        );
        changed = true;
    }

    if changed && let Some(path) = cache_path {
        save_cache(path, &cache);
    }

    let manifest = |key: &str| cache.entries.get(key).and_then(|e| e.manifest.as_ref());
    let mut result = Lookup {
        incomplete: !failed.is_empty(),
        ..Lookup::default()
    };
    for (name, version) in packages {
        let latest = manifest(&manifest_key(name, "latest"));
        let installed = manifest(&manifest_key(name, version));
        let info = Info {
            update: latest
                .and_then(|m| m.version.as_deref())
                .filter(|l| is_newer(l, version))
                .map(str::to_string),
            deprecated: installed.and_then(|m| m.deprecated.clone()),
        };
        result.info.insert((name.clone(), version.clone()), info);
    }
    result
}

/// Calls `fetch` for every key using a small pool of threads.
fn fetch_all(keys: &[String], fetch: &(impl Fn(&str) -> Fetched + Sync)) -> Vec<(String, Fetched)> {
    let next = AtomicUsize::new(0);
    let results = Mutex::new(Vec::with_capacity(keys.len()));
    thread::scope(|s| {
        for _ in 0..keys.len().min(MAX_PARALLEL_REQUESTS) {
            s.spawn(|| {
                while let Some(key) = keys.get(next.fetch_add(1, Ordering::Relaxed)) {
                    let fetched = fetch(key);
                    results.lock().unwrap().push((key.clone(), fetched));
                }
            });
        }
    });
    results.into_inner().unwrap()
}

/// Writes the cache atomically; failures are ignored (the cache is optional).
fn save_cache(path: &Path, cache: &Cache) {
    let Some(dir) = path.parent() else { return };
    let Ok(json) = serde_json::to_string(cache) else {
        return;
    };
    let tmp = path.with_extension("json.tmp");
    let _ = fs::create_dir_all(dir)
        .and_then(|_| fs::write(&tmp, json))
        .and_then(|_| fs::rename(&tmp, path));
}

/// `~/Library/Caches/nvmem/registry.json` (macOS),
/// `%LOCALAPPDATA%\nvmem\cache\registry.json` (Windows),
/// `$XDG_CACHE_HOME/nvmem/registry.json` or `~/.cache/nvmem/registry.json` (Linux).
pub fn default_cache_path() -> Option<PathBuf> {
    let env = |key| {
        std::env::var_os(key)
            .filter(|v| !v.is_empty())
            .map(PathBuf::from)
    };
    let dir = if cfg!(windows) {
        env("LOCALAPPDATA")?.join("nvmem").join("cache")
    } else if cfg!(target_os = "macos") {
        env("HOME")?.join("Library/Caches/nvmem")
    } else {
        env("XDG_CACHE_HOME")
            .or_else(|| env("HOME").map(|h| h.join(".cache")))?
            .join("nvmem")
    };
    Some(dir.join("registry.json"))
}

/// Fetches manifests from `$npm_config_registry`, defaulting to the public
/// npm registry.
pub fn http_fetcher() -> impl Fn(&str) -> Fetched + Sync {
    let base = ["npm_config_registry", "NPM_CONFIG_REGISTRY"]
        .iter()
        .find_map(|k| std::env::var(k).ok().filter(|v| !v.is_empty()))
        .unwrap_or_else(|| "https://registry.npmjs.org".to_string());
    let base = base.trim_end_matches('/').to_string();
    let agent: ureq::Agent = ureq::Agent::config_builder()
        .timeout_global(Some(REQUEST_TIMEOUT))
        .http_status_as_error(false)
        .user_agent(concat!("nvmem/", env!("CARGO_PKG_VERSION")))
        .build()
        .into();

    move |key| {
        let Ok(mut response) = agent
            .get(&format!("{base}/{key}"))
            .header("Accept", "application/json")
            .call()
        else {
            return Fetched::Failed;
        };
        match response.status().as_u16() {
            200 => response
                .body_mut()
                .read_to_string()
                .ok()
                .and_then(|body| parse_manifest(&body))
                .map_or(Fetched::Failed, Fetched::Found),
            404 => Fetched::Missing,
            _ => Fetched::Failed,
        }
    }
}

fn parse_manifest(body: &str) -> Option<Manifest> {
    let json: serde_json::Value = serde_json::from_str(body).ok()?;
    let text = |key: &str| {
        json.get(key)
            .and_then(|v| v.as_str())
            .filter(|s| !s.is_empty())
            .map(str::to_string)
    };
    Some(Manifest {
        version: text("version"),
        deprecated: text("deprecated"),
    })
}

/// Whether `candidate` is a newer release than `installed` (semver core
/// compared numerically; a release beats a prerelease of the same version).
pub fn is_newer(candidate: &str, installed: &str) -> bool {
    fn split(v: &str) -> Option<((u64, u64, u64), bool)> {
        let v = v.split('+').next()?;
        let (core, pre) = match v.split_once('-') {
            Some((core, _)) => (core, true),
            None => (v, false),
        };
        Some((crate::nvm::parse_version(core)?, pre))
    }
    match (split(candidate), split(installed)) {
        (Some((c, c_pre)), Some((i, i_pre))) => c > i || (c == i && i_pre && !c_pre),
        _ => false,
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::sync::atomic::AtomicUsize;
    use tempfile::TempDir;

    fn manifest(version: &str, deprecated: Option<&str>) -> Fetched {
        Fetched::Found(Manifest {
            version: Some(version.to_string()),
            deprecated: deprecated.map(str::to_string),
        })
    }

    fn fake_registry(key: &str) -> Fetched {
        match key {
            "@shopify%2Fcli/latest" => manifest("4.9.3", None),
            "@shopify%2Fcli/4.8.4" => manifest("4.8.4", None),
            "request/latest" | "request/2.88.2" => {
                manifest("2.88.2", Some("request has been deprecated"))
            }
            "yoinks/latest" | "yoinks/0.3.1" => manifest("0.3.1", None),
            "private-thing/latest" | "private-thing/1.0.0" => Fetched::Missing,
            _ => Fetched::Failed,
        }
    }

    fn pkgs(list: &[(&str, &str)]) -> Vec<(String, String)> {
        list.iter()
            .map(|(n, v)| (n.to_string(), v.to_string()))
            .collect()
    }

    fn info<'a>(lookup: &'a Lookup, name: &str, version: &str) -> &'a Info {
        &lookup.info[&(name.to_string(), version.to_string())]
    }

    #[test]
    fn reports_updates_and_deprecations() {
        let packages = pkgs(&[
            ("@shopify/cli", "4.8.4"),
            ("request", "2.88.2"),
            ("yoinks", "0.3.1"),
            ("private-thing", "1.0.0"),
        ]);
        let result = lookup(&packages, None, CACHE_TTL, false, fake_registry);

        assert_eq!(
            info(&result, "@shopify/cli", "4.8.4").update.as_deref(),
            Some("4.9.3")
        );
        assert_eq!(info(&result, "@shopify/cli", "4.8.4").deprecated, None);
        assert_eq!(
            info(&result, "request", "2.88.2").deprecated.as_deref(),
            Some("request has been deprecated")
        );
        assert_eq!(info(&result, "request", "2.88.2").update, None);
        assert_eq!(info(&result, "yoinks", "0.3.1"), &Info::default());
        assert_eq!(info(&result, "private-thing", "1.0.0"), &Info::default());
        assert!(!result.incomplete);
    }

    #[test]
    fn uses_cache_and_survives_going_offline() {
        let tmp = TempDir::new().unwrap();
        let cache = tmp.path().join("sub/registry.json");
        let packages = pkgs(&[("@shopify/cli", "4.8.4")]);
        let calls = AtomicUsize::new(0);
        let counting = |key: &str| {
            calls.fetch_add(1, Ordering::Relaxed);
            fake_registry(key)
        };

        lookup(&packages, Some(&cache), CACHE_TTL, false, counting);
        assert_eq!(calls.load(Ordering::Relaxed), 2);
        assert!(cache.exists());

        // Fresh cache: no requests.
        let result = lookup(&packages, Some(&cache), CACHE_TTL, false, counting);
        assert_eq!(calls.load(Ordering::Relaxed), 2);
        assert_eq!(
            info(&result, "@shopify/cli", "4.8.4").update.as_deref(),
            Some("4.9.3")
        );

        // --refresh refetches.
        lookup(&packages, Some(&cache), CACHE_TTL, true, counting);
        assert_eq!(calls.load(Ordering::Relaxed), 4);

        // Expired cache + registry unreachable: stale answers are still used.
        let result = lookup(&packages, Some(&cache), Duration::ZERO, false, |_: &str| {
            Fetched::Failed
        });
        assert_eq!(
            info(&result, "@shopify/cli", "4.8.4").update.as_deref(),
            Some("4.9.3")
        );
        assert!(!result.incomplete);
    }

    #[test]
    fn unreachable_registry_without_cache_is_incomplete() {
        let packages = pkgs(&[("yoinks", "0.3.1")]);
        let result = lookup(&packages, None, CACHE_TTL, false, |_: &str| Fetched::Failed);
        assert!(result.incomplete);
        assert_eq!(info(&result, "yoinks", "0.3.1"), &Info::default());
    }

    #[test]
    fn compares_versions() {
        assert!(is_newer("4.9.3", "4.8.4"));
        assert!(is_newer("10.0.0", "9.9.9"));
        assert!(is_newer("2.0.0", "2.0.0-beta.1"));
        assert!(!is_newer("4.8.4", "4.8.4"));
        assert!(!is_newer("4.8.4", "5.0.0-rc.1"));
        assert!(!is_newer("1.0.0", "1.0.1"));
        assert!(!is_newer("not-a-version", "1.0.0"));
    }

    #[test]
    fn parses_manifests() {
        assert_eq!(
            parse_manifest(r#"{"version":"2.88.2","deprecated":"use fetch"}"#),
            Some(Manifest {
                version: Some("2.88.2".into()),
                deprecated: Some("use fetch".into())
            })
        );
        // Empty deprecation messages mean "un-deprecated".
        assert_eq!(
            parse_manifest(r#"{"version":"1.0.0","deprecated":""}"#)
                .unwrap()
                .deprecated,
            None
        );
        assert_eq!(parse_manifest("not json"), None);
    }
}
