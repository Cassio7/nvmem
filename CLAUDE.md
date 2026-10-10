# nvmem

A cross-platform (Linux, macOS, Windows) Rust CLI that inspects an nvm installation
([nvm-sh](https://github.com/nvm-sh/nvm) on Linux/macOS,
[nvm-windows](https://github.com/coreybutler/nvm-windows) on Windows):
it lists every installed Node.js version and the global packages
(`npm i -g ...`) installed for each of them.

## Status

Commands:

- `nvmem [VERSION] [--table] [--offline] [--refresh]` (same as `nvmem list`):
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
- Global flags: `--all` (include bundled npm/corepack), `--color auto|always|never`.

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
  `registry.json` under the OS cache dir; 404s are cached too; on network
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
cargo clippy -- -D warnings # lint, treat warnings as errors
cargo test                  # tests
```

Cross-compiling from macOS no longer works (`ureq` → rustls → `ring` has C
code needing a target C toolchain); CI's native Linux/macOS/Windows jobs are the
cross-platform check.

## CI / releases

- `.github/workflows/ci.yml` — fmt, clippy, tests on Ubuntu, macOS, Windows;
  shellcheck `install.sh` and parse-check `install.ps1`.
- `.github/workflows/release.yml` — pushing a `v*` tag builds release binaries
  (Linux x64/arm64, macOS arm64/x64, Windows x64) and attaches them to a GitHub
  release, then `verify-install` installs that release with the real install
  scripts on Linux x64/arm64, macOS and Windows (pwsh + PowerShell 5.1).
- Release asset names have NO version (`nvmem-<target>[.exe]`) so
  `releases/latest/download/<asset>` links always work. `install.sh` /
  `install.ps1` depend on these names — keep them in sync.
- To release: bump `version` in `Cargo.toml`, commit, push, then
  `git tag -a vX.Y.Z -m "nvmem vX.Y.Z" && git push origin vX.Y.Z`.
- Keep tests portable: no `*` or other Windows-invalid chars in fixture file
  names; gate symlink tests with `#[cfg(unix)]`.

## Project layout

- `Cargo.toml` — package manifest (Rust edition 2024)
- `install.sh` (POSIX sh, macOS/Linux) / `install.ps1` (Windows) — one-line
  installers served from `main` via raw.githubusercontent.com; download the
  release asset to `~/.local/bin` / `%LOCALAPPDATA%\Programs\nvmem`.
  Env: `NVMEM_VERSION`, `NVMEM_INSTALL_DIR`.
- `src/main.rs` — CLI (clap derive, subcommands; bare `nvmem` = `list`), dispatch.
  `cli_definition_is_valid` test runs clap's `debug_assert` — keep it.
- `src/compare.rs` — `diff`, `install_command` / `npm_install_command`,
  `find`/`suggestions` (pure, no I/O)
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
