#!/bin/sh
# Installs (or updates) nvmem on macOS and Linux.
#
#   curl -fsSL https://raw.githubusercontent.com/Cassio7/nvmem/main/install.sh | sh
#
# Environment variables:
#   NVMEM_VERSION      release tag to install, e.g. v0.1.2 (default: latest)
#   NVMEM_INSTALL_DIR  where to put the binary (default: ~/.local/bin)

set -eu

REPO="Cassio7/nvmem"

say() { printf '%s\n' "$*"; }
fail() { printf 'nvmem install: %s\n' "$*" >&2; exit 1; }

detect_target() {
    os=$(uname -s)
    arch=$(uname -m)

    case "$os" in
        Darwin) os_part="apple-darwin" ;;
        Linux) os_part="unknown-linux-gnu" ;;
        MINGW* | MSYS* | CYGWIN*)
            fail "on Windows, use PowerShell: irm https://raw.githubusercontent.com/$REPO/main/install.ps1 | iex" ;;
        *) fail "unsupported operating system: $os" ;;
    esac

    case "$arch" in
        x86_64 | amd64) arch_part="x86_64" ;;
        arm64 | aarch64) arch_part="aarch64" ;;
        *) fail "unsupported CPU architecture: $arch" ;;
    esac

    # A shell running under Rosetta reports x86_64 on Apple Silicon Macs.
    if [ "$os" = "Darwin" ] && [ "$arch_part" = "x86_64" ] &&
        [ "$(sysctl -n sysctl.proc_translated 2>/dev/null || echo 0)" = "1" ]; then
        arch_part="aarch64"
    fi

    echo "nvmem-$arch_part-$os_part"
}

download() {
    url=$1
    out=$2
    if command -v curl >/dev/null 2>&1; then
        curl -fsSL -o "$out" "$url"
    elif command -v wget >/dev/null 2>&1; then
        wget -q -O "$out" "$url"
    else
        fail "need curl or wget to download nvmem"
    fi
}

main() {
    asset=$(detect_target)
    version=${NVMEM_VERSION:-latest}
    if [ "$version" = "latest" ]; then
        url="https://github.com/$REPO/releases/latest/download/$asset"
    else
        url="https://github.com/$REPO/releases/download/$version/$asset"
    fi
    install_dir=${NVMEM_INSTALL_DIR:-"$HOME/.local/bin"}

    say "Downloading $asset ($version)..."
    tmp=$(mktemp)
    trap 'rm -f "$tmp"' EXIT
    download "$url" "$tmp" || fail "download failed: $url"
    chmod +x "$tmp"
    # Harmless if the attribute isn't there; stops Gatekeeper blocking the binary.
    if [ "$(uname -s)" = "Darwin" ]; then
        xattr -d com.apple.quarantine "$tmp" 2>/dev/null || true
    fi

    mkdir -p "$install_dir"
    mv -f "$tmp" "$install_dir/nvmem"
    trap - EXIT

    installed=$("$install_dir/nvmem" --version) || fail "installed binary failed to run: $install_dir/nvmem"
    say "Installed $installed to $install_dir/nvmem"

    case ":$PATH:" in
        *":$install_dir:"*) say "Run 'nvmem' to get started." ;;
        *)
            say ""
            say "$install_dir is not on your PATH. Add this line to your shell profile"
            say "(~/.zshrc, ~/.bashrc, ...) and open a new terminal:"
            say ""
            say "  export PATH=\"$install_dir:\$PATH\""
            ;;
    esac
}

# Wrapped in a function so a partially downloaded script never runs.
main "$@"
