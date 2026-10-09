# nvmem

**See every Node.js version you installed with nvm, and the global packages
(`npm i -g ...`) in each one.**

Every Node version managed by nvm has its own separate set of global packages.
Install a new Node and your `tsc`, `pnpm`, `shopify`, ... are suddenly "gone".
`nvmem` shows you what lives where, finds which version has a given tool, and
gives you the exact command to copy your global packages to a new version.

```
$ nvmem
v26.11.1
  └─ typescript  7.0.2  → tsc

v24.21.0 (default)
  ├─ @shopify/cli  4.8.4  → shopify
  └─ yoinks        0.3.1  → yoinks
```

- Works on **macOS and Linux** ([nvm](https://github.com/nvm-sh/nvm)) and
  **Windows** ([nvm-windows](https://github.com/coreybutler/nvm-windows)).
- Read-only: it only reads nvm's folders, it never installs or changes anything.
- Fast: no `nvm` or `npm` processes are spawned.

## Install

### Prebuilt binary

Download the file for your system from the
[Releases page](https://github.com/Cassio7/nvmem/releases), put it somewhere on
your `PATH` and make it executable (`chmod +x nvmem` on macOS/Linux).

| System | File |
|---|---|
| macOS (Apple Silicon) | `nvmem-<version>-aarch64-apple-darwin` |
| macOS (Intel) | `nvmem-<version>-x86_64-apple-darwin` |
| Linux x64 | `nvmem-<version>-x86_64-unknown-linux-gnu` |
| Linux ARM64 | `nvmem-<version>-aarch64-unknown-linux-gnu` |
| Windows x64 | `nvmem-<version>-x86_64-pc-windows-msvc.exe` |

The binaries are not signed, so the first run may be blocked:

- **macOS**: "cannot be opened because the developer cannot be verified" →
  run `xattr -d com.apple.quarantine nvmem`, or right-click → Open.
- **Windows**: SmartScreen "Windows protected your PC" → More info → Run anyway.

### With Cargo

If you have [Rust](https://rustup.rs) installed:

```bash
cargo install --git https://github.com/Cassio7/nvmem
```

## Usage

### List versions and their global packages

```bash
nvmem                # every version, as a tree
nvmem 24             # only versions matching 24 (also: 22.9, v24.21.0, default, node, lts/*)
nvmem --table        # one row per package, one column per version
nvmem --all          # also show packages bundled with Node (npm, corepack)
```

The version nvm uses by default is marked `(default)` (on Windows, the active
one is marked `(current)`). Next to each package, `→` lists the commands it
adds to your terminal.

```
$ nvmem --table --all
package       v26.11.1  v24.21.0*
@shopify/cli  -         4.8.4
corepack      -         0.36.0
npm           11.20.0   11.19.0
typescript    7.0.2     -
yoinks        -         0.3.1
* default
```

### Copy global packages to another version

```bash
nvmem diff 24 26
```

```
v24.21.0 (default) → v26.11.1

Only in v24.21.0:
  @shopify/cli  4.8.4
  yoinks        0.3.1

Only in v26.11.1:
  typescript  7.0.2

To install the missing packages in v26.11.1:
  nvm exec 26.11.1 npm i -g @shopify/cli yoinks
```

Copy and run the printed command. Options:

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
nvmem which tsc
```

```
typescript  → tsc
  v26.11.1  7.0.2
  not in: v24.21.0 (default)
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
