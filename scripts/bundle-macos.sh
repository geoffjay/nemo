#!/bin/bash
set -euo pipefail

# Bundle a macOS .app from the built binary
#
# Usage: ./scripts/bundle-macos.sh <target> [version]
#   target:  The Rust target triple (e.g., aarch64-apple-darwin)
#   version: The version string (default: extracted from Cargo.toml)

SCRIPT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
PROJECT_ROOT="$(dirname "$SCRIPT_DIR")"

TARGET="${1:-}"
VERSION="${2:-}"

if [[ -z "$TARGET" ]]; then
    echo "Usage: $0 <target> [version]"
    echo "  target:  aarch64-apple-darwin or x86_64-apple-darwin"
    echo "  version: Version string (default: from Cargo.toml)"
    exit 1
fi

# Extract version from Cargo.toml if not provided
if [[ -z "$VERSION" ]]; then
    VERSION=$(grep '^version' "$PROJECT_ROOT/Cargo.toml" | head -1 | sed 's/.*"\(.*\)".*/\1/')
fi

BINARY_NAME="nemo"
APP_NAME="Nemo"
BUNDLE_NAME="${APP_NAME}.app"
BINARY_PATH="$PROJECT_ROOT/target/$TARGET/release/$BINARY_NAME"
BUNDLE_DIR="$PROJECT_ROOT/target/$TARGET/release/$BUNDLE_NAME"
ASSETS_DIR="$PROJECT_ROOT/assets"

echo "Creating macOS app bundle..."
echo "  Target:  $TARGET"
echo "  Version: $VERSION"
echo "  Binary:  $BINARY_PATH"
echo "  Bundle:  $BUNDLE_DIR"

# Verify binary exists
if [[ ! -f "$BINARY_PATH" ]]; then
    echo "Error: Binary not found at $BINARY_PATH"
    echo "Run 'cargo build --release --target $TARGET' first."
    exit 1
fi

# Clean up any existing bundle
rm -rf "$BUNDLE_DIR"

# Create bundle directory structure
mkdir -p "$BUNDLE_DIR/Contents/MacOS"
mkdir -p "$BUNDLE_DIR/Contents/Resources"

# Copy binary
cp "$BINARY_PATH" "$BUNDLE_DIR/Contents/MacOS/$BINARY_NAME"

# Generate Info.plist from template
sed "s/{{VERSION}}/$VERSION/g" "$ASSETS_DIR/Info.plist.template" > "$BUNDLE_DIR/Contents/Info.plist"

# Copy icon if it exists
if [[ -f "$ASSETS_DIR/AppIcon.icns" ]]; then
    cp "$ASSETS_DIR/AppIcon.icns" "$BUNDLE_DIR/Contents/Resources/"
else
    echo "Warning: No AppIcon.icns found in $ASSETS_DIR"
    echo "  The app will use a generic icon."
fi

# Copy entitlements for reference (useful for code signing)
cp "$ASSETS_DIR/entitlements.plist" "$BUNDLE_DIR/Contents/Resources/"

# Copy data files (nemos and .opencode config) into Resources
echo "Copying data files to Resources..."

# Copy examples directory
if [[ -d "$PROJECT_ROOT/examples" ]]; then
    cp -r "$PROJECT_ROOT/examples" "$BUNDLE_DIR/Contents/Resources/"
    echo "  Copied examples directory"
else
    echo "Warning: No examples directory found"
fi

# Create PkgInfo
echo -n "APPL????" > "$BUNDLE_DIR/Contents/PkgInfo"

echo "Bundle created successfully: $BUNDLE_DIR"

# Output the bundle path for CI usage
echo "BUNDLE_PATH=$BUNDLE_DIR" >> "${GITHUB_OUTPUT:-/dev/null}" 2>/dev/null || true
