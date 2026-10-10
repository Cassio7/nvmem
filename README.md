# nvmem

**See every Node.js version you installed with nvm, and the global packages
(`npm i -g ...`) in each one.**

Every Node version managed by nvm has its own separate set of global packages.
Install a new Node and your `tsc`, `pnpm`, `shopify`, ... are suddenly "gone".
`nvmem` shows you what lives where, which packages have updates or are
deprecated, finds which version has a given tool, and gives you the exact
command to copy your global packages to a new version.

- Works on **macOS and Linux** ([nvm](https://github.com/nvm-sh/nvm)) and
  **Windows** ([nvm-windows](https://github.com/coreybutler/nvm-windows)).
- Read-only: it reads nvm's folders and asks the npm registry about updates;
  it never installs or changes anything.
- Fast: no `nvm` or `npm` processes are spawned, and registry answers are
  cached, so most runs are instant.

## Requirements

- **nvm** on macOS / Linux ([nvm-sh/nvm](https://github.com/nvm-sh/nvm)), or
  **nvm-windows** on Windows ([coreybutler/nvm-windows](https://github.com/coreybutler/nvm-windows)).
  Other version managers (fnm, Volta, n, asdf, mise) and Node installed without
  a version manager are not supported.
- Nothing else: `nvmem` is a single standalone binary, no Node, npm or Rust needed
  to run it.

If nvm lives in a non-standard folder, set `NVM_DIR` (macOS / Linux) or
`NVM_HOME` (Windows) to it.

## Install

**macOS / Linux**

```bash
curl -fsSL https://raw.githubusercontent.com/Cassio7/nvmem/main/install.sh | sh
```

**Windows** (PowerShell)

```powershell
irm https://raw.githubusercontent.com/Cassio7/nvmem/main/install.ps1 | iex
```

Run the same command again to update. The script picks the right build for
your system, installs it to `~/.local/bin` (Windows:
`%LOCALAPPDATA%\Programs\nvmem`, added to your user `PATH`), and tells you if
that folder still needs to be added to your `PATH`.

| Variable | Effect |
|---|---|
| `NVMEM_VERSION` | Install a specific release, e.g. `v0.1.2` (default: latest) |
| `NVMEM_INSTALL_DIR` | Install somewhere else |

```bash
curl -fsSL https://raw.githubusercontent.com/Cassio7/nvmem/main/install.sh | NVMEM_INSTALL_DIR=/usr/local/bin sh
```

### Manual download

| System | Download (always the latest version) |
|---|---|
| macOS (Apple Silicon) | [`nvmem-aarch64-apple-darwin`](https://github.com/Cassio7/nvmem/releases/latest/download/nvmem-aarch64-apple-darwin) |
| macOS (Intel) | [`nvmem-x86_64-apple-darwin`](https://github.com/Cassio7/nvmem/releases/latest/download/nvmem-x86_64-apple-darwin) |
| Linux x64 | [`nvmem-x86_64-unknown-linux-gnu`](https://github.com/Cassio7/nvmem/releases/latest/download/nvmem-x86_64-unknown-linux-gnu) |
| Linux ARM64 | [`nvmem-aarch64-unknown-linux-gnu`](https://github.com/Cassio7/nvmem/releases/latest/download/nvmem-aarch64-unknown-linux-gnu) |
| Windows x64 | [`nvmem-x86_64-pc-windows-msvc.exe`](https://github.com/Cassio7/nvmem/releases/latest/download/nvmem-x86_64-pc-windows-msvc.exe) |

Rename it to `nvmem` (`nvmem.exe` on Windows), put it in a folder on your
`PATH`, and on macOS / Linux make it executable with `chmod +x nvmem`.

The binaries are not signed, so a manually downloaded file may be blocked on
first run:

- **macOS**: "cannot be opened because the developer cannot be verified" →
  run `xattr -d com.apple.quarantine nvmem`, or right-click → Open.
- **Windows**: SmartScreen "Windows protected your PC" → More info → Run anyway.

### With Cargo

If you have [Rust](https://rustup.rs) installed:

```bash
cargo install --git https://github.com/Cassio7/nvmem
```

## Uninstall

`nvmem` is a single file plus a small cache of npm registry answers. Remove
the binary, and optionally the cache:

| System | Cache |
|---|---|
| macOS | `~/Library/Caches/nvmem` |
| Linux | `~/.cache/nvmem` (or `$XDG_CACHE_HOME/nvmem`) |
| Windows | `%LOCALAPPDATA%\nvmem` |

**macOS / Linux** (installed with the script)

```bash
rm ~/.local/bin/nvmem
```

**Windows** (installed with the script; also removes it from your user `PATH`)

```powershell
$dir = "$env:LOCALAPPDATA\Programs\nvmem"
Remove-Item -Recurse -Force $dir
$path = [Environment]::GetEnvironmentVariable('Path', 'User') -split ';' | Where-Object { $_ -and $_ -ne $dir }
[Environment]::SetEnvironmentVariable('Path', ($path -join ';'), 'User')
```

**Cargo**

```bash
cargo uninstall nvmem
```

**Manual download** or a custom `NVMEM_INSTALL_DIR`: delete the `nvmem` file
(`nvmem.exe` on Windows) from the folder you put it in.

## Usage

### List versions and their global packages

```bash
nvmem                # every version, as a tree
nvmem 24             # only versions matching 24 (also: 22.9, v24.21.0, default, node, lts/*)
nvmem --table        # one row per package, one column per version
nvmem --all          # also show packages bundled with Node (npm, corepack)
nvmem --offline      # skip the npm registry check
nvmem --refresh      # ignore cached registry answers and check again now
```

The version nvm uses by default is marked `(default)` (on Windows, the active
one is marked `(current)`). Next to each package:

| Marker | Meaning |
|---|---|
| `↑ 4.9.3` | A newer version is on npm (`↑` in `--table`) |
| `deprecated` | The installed version is deprecated, with npm's message below it (`!` in `--table`) |
| `(linked)` | Installed with `npm link` |
| `→ tsc` | Commands the package adds to your terminal |

When updates are available, `nvmem` ends with the command to update each Node
version (`nvm exec <version> npm i -g <package>@latest`).

#### Update checks

- Registry answers are cached for 6 hours, so most runs make no network
  requests. The first run (or after `--refresh`) takes well under a second.
- Offline or registry unreachable: the last cached answers are used, otherwise
  the list is shown without update info.
- Uses the public npm registry, or `npm_config_registry` if set. Packages the
  registry doesn't know (private ones) are skipped.

### Copy global packages to another version

```bash
nvmem diff 24 26     # what 24 has that 26 doesn't, plus the npm i -g command to install it
```

Options:

| Option | Effect |
|---|---|
| `--exact` | Install the same versions you have now (`@shopify/cli@4.8.4`) instead of the latest |
| `--all` | Also compare npm and corepack |
| `--command-only` | Print only the install command |

To run the command directly (nvm is a shell function, so use `eval`, not a pipe to `sh`):

```bash
eval "$(nvmem diff 24 26 --command-only)"
```

Packages installed with `npm link` are left out of the command, since they
point at a local folder; `nvmem` reminds you to re-link them.

### Find which versions have a package or command

```bash
nvmem which tsc           # by command name
nvmem which typescript    # by package name
```

Works with package names (`typescript`, `@shopify/cli`) or command names
(`tsc`, `npx`). If nothing matches, it suggests similar names.

### Version specs

Anywhere a version is expected (`nvmem <VERSION>`, `nvmem diff <FROM> <TO>`):

| Spec | Matches |
|---|---|
| `24`, `22.9`, `v24.21.0` | Installed versions starting with that number (`diff` uses the newest) |
| `default` / `current` | The version marked `(default)` / `(current)` |
| `node`, `stable` | The newest installed version |
| `lts/*`, `lts/jod`, your own aliases | What the nvm alias points to (macOS/Linux) |

### Colors

Colors turn off automatically when output is piped or `NO_COLOR` is set.
Force them with `--color always` or `--color never`.

## Where it looks

| | macOS / Linux (nvm) | Windows (nvm-windows) |
|---|---|---|
| nvm folder | `$NVM_DIR`, else `~/.nvm` | `%NVM_HOME%`, else `%APPDATA%\nvm` |
| Versions | `versions/node/vX.Y.Z` | `vX.Y.Z` (or `root:` in `settings.txt`) |
| Global packages | `vX.Y.Z/lib/node_modules` | `vX.Y.Z/node_modules` |

## Development

Requires [Rust](https://rustup.rs) (stable, edition 2024).

```bash
git clone https://github.com/Cassio7/nvmem
cd nvmem
cargo run                      # run against your own nvm install
cargo run -- diff 24 26        # pass arguments after --
cargo test                     # unit tests (use fake nvm folders, not yours)
cargo fmt                      # format
cargo clippy -- -D warnings    # lint
cargo install --path .         # install your local build as `nvmem`
```

To try it against a fake nvm folder instead of your real one:

```bash
mkdir -p /tmp/fakenvm/versions/node/v20.1.0/lib/node_modules/foo
echo '{"version":"9.9.9","bin":"cli.js"}' > /tmp/fakenvm/versions/node/v20.1.0/lib/node_modules/foo/package.json
NVM_DIR=/tmp/fakenvm cargo run
```

### Project layout

| File | Purpose |
|---|---|
| `src/main.rs` | Command-line interface and commands |
| `src/nvm.rs` | Reads nvm / nvm-windows folders: versions, aliases, packages |
| `src/compare.rs` | `diff` and `which` logic |
| `src/output.rs` | Tree, table, diff and which output |

### CI and releases

- Every push and pull request runs format, lint and tests on Linux, macOS and
  Windows ([`ci.yml`](.github/workflows/ci.yml)).
- Pushing a tag like `v0.1.0` builds binaries for all platforms and attaches
  them to a GitHub release ([`release.yml`](.github/workflows/release.yml)):

  ```bash
  git tag v0.1.0
  git push origin v0.1.0
  ```

## License

[MIT](LICENSE)
