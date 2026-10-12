nvmtree
=======

> see what's installed in every node version — and copy it to the next one.

Every Node version managed by [nvm](https://github.com/nvm-sh/nvm) (macOS, Linux) or [nvm-windows](https://github.com/coreybutler/nvm-windows) has its own global packages. Install a new Node and your `tsc`, `pnpm`, `shopify` are suddenly gone. `nvmtree` lists them per version, flags updates and deprecations, and prints the command to move them over.

Read-only, offline by default, no `nvm` or `npm` spawned, single binary.

Formerly `nvmem` (renamed in v0.5.0, since that name is taken by Linux's non-volatile memory subsystem). Upgrading from it? Install `nvmtree` as below, then `rm ~/.local/bin/nvmem`.

Install
-------

```bash
# macOS / Linux
curl -fsSL https://raw.githubusercontent.com/Cassio7/nvmtree/main/install.sh | sh
```

```powershell
# Windows (PowerShell)
irm https://raw.githubusercontent.com/Cassio7/nvmtree/main/install.ps1 | iex
```

Run it again to update. Or grab a binary from [releases](https://github.com/Cassio7/nvmtree/releases/latest), or `cargo install --git https://github.com/Cassio7/nvmtree`.

Set `NVMTREE_VERSION` to pin a release and `NVMTREE_INSTALL_DIR` to change the folder (default `~/.local/bin`, Windows `%LOCALAPPDATA%\Programs\nvmtree`).

The scripts check each download against the release's SHA256 checksum. Release binaries also carry a signed build provenance attestation; to check a downloaded binary yourself: `gh attestation verify <file> --repo Cassio7/nvmtree`.

Usage
-----

```bash
nvmtree                  # every version and its global packages, as a tree
nvmtree 24               # only matching versions (22.9, v24.21.0, default, node, lts/*)
nvmtree --table          # package × version matrix
nvmtree --all            # include npm and corepack
nvmtree -u               # also check npm for updates and deprecations
nvmtree --refresh        # same, ignoring cached registry answers

nvmtree diff 24 26       # what 24 has that 26 doesn't + the command to install it
nvmtree diff 24 26 --exact --command-only
eval "$(nvmtree diff 24 26 --command-only)"  # run it

nvmtree which tsc        # which versions have it (package or command name)

nvmtree npx              # packages cached by npx: size, last used, how to clean
nvmtree npx -u           # same, plus updates and deprecations

nvmtree --json           # one JSON document instead of text (list, diff, which)
nvmtree diff 24 26 --json | jq -r .install_command
```

Markers: `↑ 4.9.3` newer on npm (with `-u`) · `deprecated` (with `-u`) · `(linked)` from `npm link` · `→ tsc` commands it adds · `(default)` / `(current)` the version nvm uses.

`npx <pkg>` doesn't install globally: it caches the package in npm's cache (`~/.npm/_npx`), shared by all Node versions and kept until you clean it. `nvmtree` ends with a one-line summary of that cache, `nvmtree npx` shows the details, and `nvmtree which` tells you when something is only cached by npx.

`--json` prints a single JSON document on stdout (no colors, no extra text) with a `schema_version` field, so other tools can build on it — e.g. keep their own history of what you install. It stays offline unless you add `-u`; without it `update` / `deprecated` are `null` and `updates_checked` is `false`. Errors go to stderr; `which` with no match prints the document with `suggestions` and exits 1.

Colors turn off when piped or when `NO_COLOR` is set; force with `--color always|never`.

How it works
------------

- reads nvm's folders directly (`$NVM_DIR` or `~/.nvm`; `%NVM_HOME%` or `%APPDATA%\nvm`), and npm's npx cache (`$npm_config_cache/_npx`, default `~/.npm/_npx`; Windows `%LOCALAPPDATA%\npm-cache\_npx`)
- never touches the network unless you pass `-u`; then it asks the npm registry (or `$npm_config_registry`) about each package, in parallel, cached for 6 hours. That sends your global package names and versions to the registry
- fnm, Volta, n, asdf and mise are not supported

Uninstall
---------

```bash
rm ~/.local/bin/nvmtree   # or: cargo uninstall nvmtree
```

Windows: delete `%LOCALAPPDATA%\Programs\nvmtree` and remove it from your user `PATH`. Registry cache (only created by `-u`): `~/Library/Caches/nvmtree` (macOS), `~/.cache/nvmtree` (Linux), `%LOCALAPPDATA%\nvmtree` (Windows).

Development
-----------

```bash
cargo run -- diff 24 26                  # run against your own nvm
cargo test                               # uses fake nvm folders
cargo fmt && cargo clippy --all-targets -- -D warnings
```

Releases: push a `v*` tag and CI builds the binaries.

License
-------

[MIT](LICENSE)
