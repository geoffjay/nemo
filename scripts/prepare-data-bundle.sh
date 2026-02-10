#!/bin/bash
# Prepare data files for bundling in releases
# Creates a data directory with examples

set -e

SCRIPT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
PROJECT_DIR="$(dirname "$SCRIPT_DIR")"
OUTPUT_DIR="${1:-$PROJECT_DIR/target/data-bundle}"

echo "Preparing data bundle in $OUTPUT_DIR"

# Clean and create output directory
rm -rf "$OUTPUT_DIR"
mkdir -p "$OUTPUT_DIR"

# Copy examples directory
if [ -d "$PROJECT_DIR/examples" ]; then
    echo "Copying examples..."
    cp -r "$PROJECT_DIR/examples" "$OUTPUT_DIR/"
else
    echo "Warning: examples directory not found"
fi

echo "Data bundle prepared successfully:"
find "$OUTPUT_DIR" -type f | head -20
