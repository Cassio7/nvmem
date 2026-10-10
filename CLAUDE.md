# nvmem

A cross-platform (Linux, macOS, Windows) Rust CLI that inspects an nvm installation
([nvm-sh](https://github.com/nvm-sh/nvm) on Linux/macOS,
[nvm-windows](https://github.com/coreybutler/nvm-windows) on Windows):
it lists every installed Node.js version and the global packages
(`npm i -g ...`) installed for each of them.

## Status

Commands:

- `nvmem [VERSION] [-t/--table] [--offline] [--refresh]` (same as `nvmem list`):
  versions + global packages as a tree (with each package's commands from `bin`,
  `(linked)` for `npm link`, `↑ x.y.z` when npm has a newer version, `deprecated`
  + message), or a package × version matrix (`↑` / `!` markers). Ends with a
  summary + `npm i -g <pkg>@latest` commands per version. `VERSION` filters: `24`, `22.9`, `v24.21.0`,
  `default`/`current`, `node`, or an nvm-sh alias like `lts/*`.
- `nvmem diff FROM TO [--exact] [--command-only]`: packages only in FROM / only in
  TO / at different versions, plus the command to install FROM's missing packages
  into TO (`nvm exec <v> npm i -g ...`; two lines on nvm-windows). A spec matching
  several versions uses the newest. Linked packages are left out of the command.
- `nvmem which NAME`: versions that have a package, matched by package name or
  command name (`which tsc` → typescript). Always includes bundled packages.
  Exit code 1 with "did you mean" hints when nothing matches.
- Global flags: `-a/--all` (include bundled npm/corepack), `--color auto|always|never`.

## v1 scope (agreed)

- Plain, non-interactive CLI: print and exit.
- Read-only on disk (only writes the registry cache): list installed Node versions and, under each, its global packages
  with their versions (tree-style output). Mark the nvm-sh `default` alias,
  or the active version on nvm-windows (`current`).
- Hide Node-bundled packages (`npm`, `corepack`) by default; `--all` shows them.
- Colors via `anstream`/`anstyle`: auto-disabled when piped, respects `NO_COLOR`.
- Registry checks (`src/registry.rs`): per package, GET `<registry>/<name>/latest`
  and `<registry>/<name>/<installed>` (~5 KB each, scoped names as `@scope%2Fname`),
  in parallel (≤16 threads, 3 s timeout) via `ureq`. Cached 6 h in
  `registry.json` under the OS cache dir (`~/Library/Caches/nvmem`,
  `$XDG_CACHE_HOME/nvmem` or `~/.cache/nvmem`, `%LOCALAPPDATA%\nvmem\cache`); 404s are cached too; on network
  failure stale cache entries are used. Registry = `$npm_config_registry` or
  npmjs. Never block or fail the listing because of the registry.
- Candidate next features: Homebrew tap, disk sizes, `--json`, shell completions.

## Build & run

```bash
cargo build                 # debug build -> target/debug/nvmem
cargo build --release       # optimized build -> target/release/nvmem
cargo run                   # build + run
cargo run -- <args>         # pass arguments to the CLI
cargo install --path .      # install `nvmem` into ~/.cargo/bin
```

## Quality checks

Run these before committing:

```bash
cargo fmt                   # format
cargo clippy --all-targets -- -D warnings # lint (same as CI), warnings are errors
cargo test                  # tests
```

Cross-compiling from macOS no longer works (`ureq` → rustls → `ring` has C
code needing a target C toolchain); CI's native Linux/macOS/Windows jobs are the
cross-platform check.

## CI / releases

- `.github/workflows/ci.yml` — on push to `main`/`master` and PRs: fmt, clippy, tests on Ubuntu, macOS, Windows;
  shellcheck `install.sh` and parse-check `install.ps1`.
- `.github/workflows/release.yml` — pushing a `v*` tag builds release binaries
  (Linux x64/arm64, macOS arm64/x64, Windows x64) and attaches them to a GitHub
  release, then `verify-install` installs that release with the real install
  scripts on Linux x64/arm64, macOS and Windows (pwsh + PowerShell 5.1).
- Release asset names have NO version (`nvmem-<target>[.exe]`) so
  `releases/latest/download/<asset>` links always work. `install.sh` /
  `install.ps1` depend on these names — keep them in sync.
- To release: bump `version` in `Cargo.toml`, commit (`chore: release vX.Y.Z`),
  push, then create an annotated tag whose message is the release notes
  (markdown bullets) and push it:
  `git tag -a vX.Y.Z --cleanup=verbatim -F notes.md && git push origin vX.Y.Z`.
  The release workflow uses the tag message as the GitHub release description
  (`--cleanup=verbatim` keeps lines starting with `#`).
- `.github/workflows/audit.yml` — `cargo audit` (RustSec advisories) weekly
  (Mondays), on dependency changes, and on demand.
- Keep tests portable: no `*` or other Windows-invalid chars in fixture file
  names; gate symlink tests with `#[cfg(unix)]`.

## Commit messages

[Conventional Commits](https://www.conventionalcommits.org): `<type>: <summary>`
(imperative, lowercase, no trailing period), optional body explaining why.
One logical change per commit; split mixed work into separate commits.

| Type | Use for |
|---|---|
| `feat:` | New user-facing behavior (commands, flags, output) |
| `fix:` | Bug fixes |
| `docs:` | README, CLAUDE.md, comments only |
| `refactor:` | Code changes with no behavior change |
| `test:` | Adding or fixing tests only |
| `build:` | `Cargo.toml`, dependencies, release profile |
| `ci:` | `.github/` workflows, Dependabot config |
| `chore:` | Version bumps (`chore: release vX.Y.Z`), housekeeping |

Breaking changes: `feat!:` / `fix!:` plus a `BREAKING CHANGE:` line in the body.

## Dependencies

Direct crates (see `Cargo.toml`): `clap` (CLI), `anstream`/`anstyle` (colors),
`anyhow` (errors), `serde`/`serde_json` (JSON), `ureq` (HTTPS to the npm
registry); `tempfile` for tests. `Cargo.lock` is committed (binary crate).

- `cargo update` — newest semver-compatible versions (only touches `Cargo.lock`).
- Major bumps (`4.x` → `5.0`): edit `Cargo.toml`, fix code, run quality checks.
- `cargo audit` (install: `cargo install cargo-audit`) — RustSec advisories;
  `cargo outdated --root-deps-only` (install: `cargo install cargo-outdated`).
- Dependabot (`.github/dependabot.yml`, free on GitHub): weekly PRs, Cargo
  minor/patch grouped into one PR (`build(deps): ...`), GitHub Actions grouped
  (`ci(deps): ...`). Merge once CI is green; majors need review. Dependency
  updates don't need a release unless they fix something user-facing (e.g. a
  security advisory in `ureq`/rustls → release a patch version).
- Prefer few dependencies: reading files/JSON covers most needs; add a crate
  only when it saves real complexity.

## Project layout

- `Cargo.toml` / `Cargo.lock` — package manifest (Rust edition 2024) and committed lockfile
- `README.md` — user-facing docs (keep short; details live in `--help`);
  `LICENSE` (MIT); `.gitignore` (`/target`)
- `.github/` — `workflows/{ci,release,audit}.yml`, `dependabot.yml` (see above)
- `install.sh` (POSIX sh, macOS/Linux) / `install.ps1` (Windows) — one-line
  installers served from `main` via raw.githubusercontent.com; download the
  release asset to `~/.local/bin` / `%LOCALAPPDATA%\Programs\nvmem`.
  Env: `NVMEM_VERSION`, `NVMEM_INSTALL_DIR`.
- `src/main.rs` — CLI (clap derive, subcommands; bare `nvmem` = `list`), dispatch.
  `cli_definition_is_valid` test runs clap's `debug_assert` — keep it.
- `src/compare.rs` — `diff`, `install_command` / `npm_install_command`,
  `find`/`suggestions` (pure, no I/O)
- `src/sanitize.rs` — guards for external text (folder names, `package.json`,
  registry): `is_safe_package_name` / `is_safe_version` gate everything that
  goes into generated shell commands or registry URLs (invalid ones are left
  out and reported, never quoted/escaped).
- `src/registry.rs` — npm registry lookups + cache; `lookup` takes the fetch
  function as a parameter so tests use a fake registry (no network in tests)
- `src/output.rs` — tree, table, diff and which renderers; return styled `String`s
  (tests strip ANSI with `anstream::adapter::strip_str`)
- `src/nvm.rs` — `Install` (nvm-sh or nvm-windows layout, chosen by `cfg!(windows)`):
  installed versions, default/current marker, global packages, `load_entries`,
  version-spec `select`. Unit tests use fake nvm dirs via `tempfile`.

## How nvm-sh stores things (Linux/macOS)

- nvm root: `$NVM_DIR`, defaulting to `~/.nvm` when the variable is unset
  (it is often unset in non-interactive shells, so always fall back).
- Installed Node versions: `$NVM_DIR/versions/node/v<X.Y.Z>/`
- Global packages for a version: `$NVM_DIR/versions/node/v<X.Y.Z>/lib/node_modules/`
  - Scoped packages live one level deeper: `node_modules/@scope/<name>/`
  - Each package's version is in its `package.json` (`"version"` field)
  - `npm` and `corepack` ship with Node itself, they were not installed by the user
- Aliases (e.g. `default`): plain-text files in `$NVM_DIR/alias/`, content is a
  version spec like `24` or `v24.21.0` (may need resolving to an installed version).
- Prefer reading the filesystem directly over shelling out to `nvm`/`npm`:
  `nvm` is a shell function (not a binary) and spawning `npm ls -g` per version is slow.

## How nvm-windows stores things

- Home: `%NVM_HOME%`, defaulting to `%APPDATA%\nvm`. Contains `settings.txt`,
  `nvm.exe`, and (by default) the versions.
- `settings.txt` lines `root: <dir>` (where versions live, may differ from home)
  and `path: <dir>` (the active-version symlink).
- Versions: `<root>\v<X.Y.Z>\`; global packages: `<root>\v<X.Y.Z>\node_modules\`
  (no `lib/`).
- No aliases/default. The active version is wherever the symlink
  (`%NVM_SYMLINK%`, usually `C:\Program Files\nodejs`) points.
