#!/bin/sh
# Nemo installer
#
# Usage:
#   curl -fsSL https://raw.githubusercontent.com/geoffjay/nemo/main/scripts/install.sh | sh
#
# Environment variables:
#   NEMO_VERSION      Release tag to install (e.g. v0.6.0). Default: latest
#   NEMO_INSTALL_DIR  Directory to install the binary into.
#                     Default: $HOME/.local/bin
#
# On Windows, download the .zip from the releases page instead:
#   https://github.com/geoffjay/nemo/releases

set -eu

REPO="geoffjay/nemo"
BINARY="nemo"
VERSION="${NEMO_VERSION:-latest}"
INSTALL_DIR="${NEMO_INSTALL_DIR:-$HOME/.local/bin}"

err() {
    echo "error: $*" >&2
    exit 1
}

info() {
    echo "nemo-install: $*" >&2
}

# --- detect platform -------------------------------------------------------

os="$(uname -s)"
arch="$(uname -m)"

case "$os" in
    Linux) os_part="unknown-linux-gnu" ;;
    Darwin) os_part="apple-darwin" ;;
    *)
        err "unsupported OS '$os'. On Windows, download the .zip from https://github.com/${REPO}/releases"
        ;;
esac

case "$arch" in
    x86_64 | amd64) arch_part="x86_64" ;;
    arm64 | aarch64) arch_part="aarch64" ;;
    *) err "unsupported architecture '$arch'" ;;
esac

target="${arch_part}-${os_part}"
asset="${BINARY}-${target}.tar.gz"

# --- resolve download URLs -------------------------------------------------

if [ "$VERSION" = "latest" ]; then
    base="https://github.com/${REPO}/releases/latest/download"
else
    base="https://github.com/${REPO}/releases/download/${VERSION}"
fi

# --- pick a downloader -----------------------------------------------------

if command -v curl >/dev/null 2>&1; then
    download() { curl -fsSL "$1" -o "$2"; }
elif command -v wget >/dev/null 2>&1; then
    download() { wget -q "$1" -O "$2"; }
else
    err "neither curl nor wget is available"
fi

# --- pick a checksum tool --------------------------------------------------

if command -v sha256sum >/dev/null 2>&1; then
    sha256() { sha256sum "$1" | awk '{print $1}'; }
elif command -v shasum >/dev/null 2>&1; then
    sha256() { shasum -a 256 "$1" | awk '{print $1}'; }
else
    sha256() { echo ""; }
fi

# --- download & verify -----------------------------------------------------

tmp="$(mktemp -d)"
trap 'rm -rf "$tmp"' EXIT

info "downloading ${asset} (${VERSION})"
download "${base}/${asset}" "${tmp}/${asset}" ||
    err "failed to download ${base}/${asset} — is there a release for ${target}?"

if download "${base}/checksums.txt" "${tmp}/checksums.txt" 2>/dev/null; then
    expected="$(grep " ${asset}\$" "${tmp}/checksums.txt" | awk '{print $1}' | head -1)"
    actual="$(sha256 "${tmp}/${asset}")"
    if [ -z "$actual" ]; then
        info "no sha256 tool found; skipping checksum verification"
    elif [ -z "$expected" ]; then
        info "no checksum listed for ${asset}; skipping verification"
    elif [ "$expected" != "$actual" ]; then
        err "checksum mismatch for ${asset} (expected ${expected}, got ${actual})"
    else
        info "checksum verified"
    fi
else
    info "checksums.txt not found; skipping verification"
fi

# --- extract & install -----------------------------------------------------

tar -xzf "${tmp}/${asset}" -C "$tmp"
[ -f "${tmp}/${BINARY}" ] || err "archive did not contain '${BINARY}'"

mkdir -p "$INSTALL_DIR"
install -m 755 "${tmp}/${BINARY}" "${INSTALL_DIR}/${BINARY}"

# Strip the macOS quarantine flag in case it was set.
if [ "$os" = "Darwin" ]; then
    xattr -d com.apple.quarantine "${INSTALL_DIR}/${BINARY}" 2>/dev/null || true
fi

info "installed ${BINARY} to ${INSTALL_DIR}/${BINARY}"

# --- PATH hint -------------------------------------------------------------

case ":${PATH}:" in
    *":${INSTALL_DIR}:"*) ;;
    *)
        info "note: ${INSTALL_DIR} is not on your PATH"
        info "add this to your shell profile:"
        info "  export PATH=\"${INSTALL_DIR}:\$PATH\""
        ;;
esac

"${INSTALL_DIR}/${BINARY}" --version 2>/dev/null || true
