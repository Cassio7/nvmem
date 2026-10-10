nvmem
=====

> see what's installed in every node version — and copy it to the next one.

Every Node version managed by [nvm](https://github.com/nvm-sh/nvm) (macOS, Linux) or [nvm-windows](https://github.com/coreybutler/nvm-windows) has its own global packages. Install a new Node and your `tsc`, `pnpm`, `shopify` are suddenly gone. `nvmem` lists them per version, flags updates and deprecations, and prints the command to move them over.

Read-only, no `nvm` or `npm` spawned, single binary.

Install
-------

```bash
# macOS / Linux
curl -fsSL https://raw.githubusercontent.com/Cassio7/nvmem/main/install.sh | sh
```

```powershell
# Windows (PowerShell)
irm https://raw.githubusercontent.com/Cassio7/nvmem/main/install.ps1 | iex
```

Run it again to update. Or grab a binary from [releases](https://github.com/Cassio7/nvmem/releases/latest), or `cargo install --git https://github.com/Cassio7/nvmem`.

Set `NVMEM_VERSION` to pin a release and `NVMEM_INSTALL_DIR` to change the folder (default `~/.local/bin`, Windows `%LOCALAPPDATA%\Programs\nvmem`).

Usage
-----

```bash
nvmem                  # every version and its global packages, as a tree
nvmem 24               # only matching versions (22.9, v24.21.0, default, node, lts/*)
nvmem --table          # package × version matrix
nvmem --all            # include npm and corepack
nvmem --offline        # skip the registry check
nvmem --refresh        # ignore cached registry answers

nvmem diff 24 26       # what 24 has that 26 doesn't + the command to install it
nvmem diff 24 26 --exact --command-only
eval "$(nvmem diff 24 26 --command-only)"   # run it

nvmem which tsc        # which versions have it (package or command name)
```

Markers: `↑ 4.9.3` newer on npm · `deprecated` · `(linked)` from `npm link` · `→ tsc` commands it adds · `(default)` / `(current)` the version nvm uses.

Colors turn off when piped or when `NO_COLOR` is set; force with `--color always|never`.

How it works
------------

- reads nvm's folders directly (`$NVM_DIR` or `~/.nvm`; `%NVM_HOME%` or `%APPDATA%\nvm`)
- asks the npm registry (or `$npm_config_registry`) for updates, in parallel, cached for 6 hours — offline just works
- fnm, Volta, n, asdf and mise are not supported

Uninstall
---------

```bash
rm ~/.local/bin/nvmem   # or: cargo uninstall nvmem
```

Windows: delete `%LOCALAPPDATA%\Programs\nvmem` and remove it from your user `PATH`. Optional cache: `~/Library/Caches/nvmem` (macOS), `~/.cache/nvmem` (Linux), `%LOCALAPPDATA%\nvmem` (Windows).

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
