#!/bin/sh
# Installs (or updates) nvmtree on macOS and Linux.
#
#   curl -fsSL https://raw.githubusercontent.com/Cassio7/nvmtree/main/install.sh | sh
#
# Environment variables:
#   NVMTREE_VERSION      release tag to install, e.g. v0.1.2 (default: latest)
#   NVMTREE_INSTALL_DIR  where to put the binary (default: ~/.local/bin)

set -eu

REPO="Cassio7/nvmtree"

say() { printf '%s\n' "$*"; }
fail() { printf 'nvmtree install: %s\n' "$*" >&2; exit 1; }
warn() { printf 'nvmtree install: warning: %s\n' "$*" >&2; }

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

    echo "nvmtree-$arch_part-$os_part"
}

download() {
    url=$1
    out=$2
    if command -v curl >/dev/null 2>&1; then
        curl -fsSL -o "$out" "$url"
    elif command -v wget >/dev/null 2>&1; then
        wget -q -O "$out" "$url"
    else
        fail "need curl or wget to download nvmtree"
    fi
}

# Like `download`, but for a file that may not exist: returns 1 if the server
# says it isn't there (HTTP 404), and fails on any other error.
download_optional() {
    url=$1
    out=$2
    if command -v curl >/dev/null 2>&1; then
        # curl's exit code for HTTP errors varies between versions; check the
        # status code instead.
        status=$(curl -sSL -o "$out" -w '%{http_code}' "$url") ||
            fail "download failed: $url"
        case "$status" in
            2??) return 0 ;;
            404) return 1 ;;
            *) fail "download failed (HTTP $status): $url" ;;
        esac
    else
        rc=0
        wget -q -O "$out" "$url" || rc=$?
        case "$rc" in
            0) return 0 ;;
            8) return 1 ;; # the server answered with an error, e.g. 404
            *) fail "download failed: $url" ;;
        esac
    fi
}

# Prints the SHA256 of a file, or nothing if no hashing tool is available.
sha256_of() {
    if command -v sha256sum >/dev/null 2>&1; then
        sha256sum "$1" | cut -d' ' -f1
    elif command -v shasum >/dev/null 2>&1; then
        shasum -a 256 "$1" | cut -d' ' -f1
    fi
}

# Checks a downloaded file against the release's "<hex>  <name>" checksum file.
# Fails on a mismatch; warns and carries on if the checksum can't be checked.
verify_checksum() {
    file=$1
    sum_url=$2
    sum_file=$3
    if ! download_optional "$sum_url" "$sum_file"; then
        # Releases up to v0.1.3 were published without checksums.
        warn "checksum not found for this release, skipping verification"
        return 0
    fi
    expected=$(cut -d' ' -f1 <"$sum_file" | tr 'A-F' 'a-f')
    actual=$(sha256_of "$file")
    if [ -z "$actual" ]; then
        warn "neither sha256sum nor shasum found, skipping checksum verification"
        return 0
    fi
    [ "$actual" = "$expected" ] ||
        fail "checksum mismatch for $(basename "$file"): expected $expected, got $actual"
    say "Checksum verified (SHA256 $actual)."
}

main() {
    asset=$(detect_target)
    version=${NVMTREE_VERSION:-latest}
    if [ "$version" = "latest" ]; then
        url="https://github.com/$REPO/releases/latest/download/$asset"
    else
        url="https://github.com/$REPO/releases/download/$version/$asset"
    fi
    install_dir=${NVMTREE_INSTALL_DIR:-"$HOME/.local/bin"}

    say "Downloading $asset ($version)..."
    tmp_dir=$(mktemp -d)
    trap 'rm -rf "$tmp_dir"' EXIT
    tmp="$tmp_dir/$asset"
    download "$url" "$tmp" || fail "download failed: $url"
    verify_checksum "$tmp" "$url.sha256" "$tmp.sha256"
    chmod +x "$tmp"
    # Harmless if the attribute isn't there; stops Gatekeeper blocking the binary.
    if [ "$(uname -s)" = "Darwin" ]; then
        xattr -d com.apple.quarantine "$tmp" 2>/dev/null || true
    fi

    mkdir -p "$install_dir"
    mv -f "$tmp" "$install_dir/nvmtree"
    rm -rf "$tmp_dir"
    trap - EXIT

    installed=$("$install_dir/nvmtree" --version) || fail "installed binary failed to run: $install_dir/nvmtree"
    say "Installed $installed to $install_dir/nvmtree"

    case ":$PATH:" in
        *":$install_dir:"*) say "Run 'nvmtree' to get started." ;;
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
