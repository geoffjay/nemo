#!/bin/bash
# Generate the Homebrew formula for a release by filling in the version and
# per-target sha256 checksums from the release's checksums.txt.
#
# Usage:
#   ./scripts/gen-homebrew-formula.sh <version> [checksums.txt] > nemo.rb
#
#   version         Release version (leading 'v' optional, e.g. v0.6.0)
#   checksums.txt   Path to the release checksums file
#                   (default: downloaded from the GitHub release for <version>)
#
# The rendered formula is written to stdout so the caller decides where it
# lands (e.g. Formula/nemo.rb in the geoffjay/homebrew-tap checkout). The
# checked-in template packaging/homebrew/nemo.rb.tpl is never modified.
#
# The checksums file is the `sha256sum *.tar.gz *.zip *.deb` output produced by
# the release workflow, with lines of the form "<sha256>  <filename>".
set -euo pipefail

SCRIPT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
PROJECT_ROOT="$(dirname "$SCRIPT_DIR")"
REPO="geoffjay/nemo"
TEMPLATE="$PROJECT_ROOT/packaging/homebrew/nemo.rb.tpl"

VERSION="${1:-}"
CHECKSUMS="${2:-}"

if [[ -z "$VERSION" ]]; then
    echo "Usage: $0 <version> [checksums.txt] > nemo.rb" >&2
    exit 1
fi
VERSION="${VERSION#v}"

# Download checksums.txt from the release if not provided locally.
CLEANUP=""
if [[ -z "$CHECKSUMS" ]]; then
    CHECKSUMS="$(mktemp)"
    CLEANUP="$CHECKSUMS"
    trap 'rm -f "$CLEANUP"' EXIT
    url="https://github.com/${REPO}/releases/download/v${VERSION}/checksums.txt"
    echo "Downloading $url" >&2
    curl -fsSL "$url" -o "$CHECKSUMS"
fi

# Look up the sha256 for a given asset filename.
sum_for() {
    local name="$1"
    local sum
    sum="$(grep " ${name}\$" "$CHECKSUMS" | awk '{print $1}' | head -1)"
    if [[ -z "$sum" ]]; then
        echo "error: no checksum for $name in $CHECKSUMS" >&2
        exit 1
    fi
    echo "$sum"
}

SHA_MAC_ARM="$(sum_for "nemo-aarch64-apple-darwin.tar.gz")"
SHA_MAC_X86="$(sum_for "nemo-x86_64-apple-darwin.tar.gz")"
SHA_LNX_ARM="$(sum_for "nemo-aarch64-unknown-linux-gnu.tar.gz")"
SHA_LNX_X86="$(sum_for "nemo-x86_64-unknown-linux-gnu.tar.gz")"

# Render the formula from the checked-in template to stdout.
sed \
    -e "s/version \"[0-9][^\"]*\"/version \"${VERSION}\"/" \
    -e "s/SHA256_AARCH64_APPLE_DARWIN/${SHA_MAC_ARM}/" \
    -e "s/SHA256_X86_64_APPLE_DARWIN/${SHA_MAC_X86}/" \
    -e "s/SHA256_AARCH64_UNKNOWN_LINUX_GNU/${SHA_LNX_ARM}/" \
    -e "s/SHA256_X86_64_UNKNOWN_LINUX_GNU/${SHA_LNX_X86}/" \
    "$TEMPLATE"

echo "Rendered formula for v${VERSION}" >&2
