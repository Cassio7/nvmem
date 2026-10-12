//! Guards for text that comes from outside nvmtree (folder names, `package.json`
//! files, the npm registry) before it is printed or used in shell commands
//! or URLs.

use std::borrow::Cow;

/// `text` without characters that can control a terminal or disguise text:
/// C0/C1 control characters (escape sequences start with ESC or CSI, so this
/// also neutralizes color codes, cursor movement, title changes and OSC 52
/// clipboard writes) and Unicode bidirectional overrides ("Trojan Source").
/// Tabs and newlines are dropped too: every caller prints single-line values.
pub fn printable(text: &str) -> Cow<'_, str> {
    let unsafe_char = |c: char| {
        c.is_control()
            || matches!(c, '\u{200E}' | '\u{200F}' | '\u{202A}'..='\u{202E}' | '\u{2066}'..='\u{2069}')
    };
    if text.contains(unsafe_char) {
        Cow::Owned(text.chars().filter(|&c| !unsafe_char(c)).collect())
    } else {
        Cow::Borrowed(text)
    }
}

/// Whether `name` is an npm package name (`name` or `@scope/name`) made only of
/// characters that are safe unquoted in sh, cmd and PowerShell: each part uses
/// `A-Z a-z 0-9 - . _ ~` and doesn't start with `.`, `_`, `-` (would look like
/// an npm option) or `~` (tilde expansion).
pub fn is_safe_package_name(name: &str) -> bool {
    let part_ok = |part: &str| {
        !part.is_empty()
            && !part.starts_with(['.', '_', '-', '~'])
            && part
                .chars()
                .all(|c| c.is_ascii_alphanumeric() || matches!(c, '-' | '.' | '_' | '~'))
    };
    if name.len() > 214 {
        return false;
    }
    match name.strip_prefix('@') {
        Some(scoped) => {
            matches!(scoped.split_once('/'), Some((scope, rest)) if part_ok(scope) && part_ok(rest))
        }
        None => part_ok(name),
    }
}

/// Whether `version` is a semver version: `1.2.3`, optionally with a
/// prerelease (`-beta.1`) and/or build metadata (`+build.5`).
pub fn is_safe_version(version: &str) -> bool {
    let (rest, build) = match version.split_once('+') {
        Some((rest, build)) => (rest, Some(build)),
        None => (version, None),
    };
    let (core, pre) = match rest.split_once('-') {
        Some((core, pre)) => (core, Some(pre)),
        None => (rest, None),
    };
    let identifiers_ok = |s: &str| {
        s.split('.')
            .all(|id| !id.is_empty() && id.chars().all(|c| c.is_ascii_alphanumeric() || c == '-'))
    };
    core.chars().all(|c| c.is_ascii_digit() || c == '.')
        && crate::nvm::parse_version(core).is_some()
        && pre.is_none_or(identifiers_ok)
        && build.is_none_or(identifiers_ok)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn printable_strips_terminal_control_sequences() {
        assert_eq!(printable("typescript 5.0.4"), "typescript 5.0.4");
        assert!(matches!(printable("plain"), Cow::Borrowed(_)));
        // Title change, line erase + carriage return, color, OSC 52 clipboard write.
        assert_eq!(
            printable("1.0.0\u{1b}]0;TITLE\u{7}\u{1b}[2K\rFAKE"),
            "1.0.0]0;TITLE[2KFAKE"
        );
        assert_eq!(printable("\u{1b}[31mred\u{1b}[0m"), "[31mred[0m");
        assert_eq!(printable("x\u{1b}]52;c;ZXZpbA==\u{7}"), "x]52;c;ZXZpbA==");
        // C1 CSI, DEL, tab/newline, bidi override.
        assert_eq!(printable("a\u{9b}2Jb\u{7f}c\td\ne"), "a2Jbcde");
        assert_eq!(printable("safe\u{202E}txt.exe"), "safetxt.exe");
        // Ordinary Unicode is kept.
        assert_eq!(printable("café → ✓ 名前"), "café → ✓ 名前");
    }

    #[test]
    fn accepts_real_package_names() {
        for name in [
            "typescript",
            "@shopify/cli",
            "JSONStream",
            "lodash.merge",
            "a~b",
            "a-b_c",
            "@types/node",
        ] {
            assert!(is_safe_package_name(name), "{name}");
        }
    }

    #[test]
    fn rejects_unsafe_package_names() {
        let too_long = "x".repeat(215);
        for name in [
            "",
            "evil;name",
            "$(id)",
            "a b",
            "a&&b",
            "a|b",
            "`id`",
            "a'b",
            "@scope",
            "@/x",
            "@s/",
            "@a/b/c",
            "a/b",
            ".hidden",
            "_private",
            "-rf",
            "~root",
            "@-x/y",
            "名前",
            "a\u{1b}[31m",
            too_long.as_str(),
        ] {
            assert!(!is_safe_package_name(name), "{name:?}");
        }
    }

    #[test]
    fn accepts_semver_versions() {
        for v in [
            "1.2.3",
            "0.0.1",
            "10.20.30",
            "1.0.0-beta.1",
            "1.0.0-rc-1",
            "1.0.0+build.5",
            "1.0.0-alpha+001",
        ] {
            assert!(is_safe_version(v), "{v}");
        }
    }

    #[test]
    fn rejects_unsafe_versions() {
        for v in [
            "",
            "1.2",
            "v1.2.3",
            "+1.2.3",
            "1.2.3;",
            "1.0.0 && echo INJECTED",
            "1.2.3-",
            "1.2.3+",
            "1.2.3-$(id)",
            "1.2.3-a..b",
            "1.2.3\u{1b}]0;x\u{7}",
        ] {
            assert!(!is_safe_version(v), "{v:?}");
        }
    }
}
