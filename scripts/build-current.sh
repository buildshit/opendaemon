#!/bin/bash
# Build script for current platform only

set -e

echo "Building dmn for current platform..."

# Create output directory
mkdir -p dist

# Build for current platform
cargo build --release --package dmn-core

# Detect platform and copy binary
if [[ "$OSTYPE" == "linux-gnu"* ]]; then
    cp target/release/dmn dist/dmn-linux-x64
    echo "Built for Linux x86_64"
elif [[ "$OSTYPE" == "darwin"* ]]; then
    if [[ $(uname -m) == "arm64" ]]; then
        cp target/release/dmn dist/dmn-darwin-arm64
        echo "Built for macOS ARM64"
    else
        cp target/release/dmn dist/dmn-darwin-x64
        echo "Built for macOS x86_64"
    fi
elif [[ "$OSTYPE" == "msys" ]] || [[ "$OSTYPE" == "win32" ]]; then
    cp target/release/dmn.exe dist/dmn-win32-x64.exe
    echo "Built for Windows x86_64"
else
    echo "Unknown platform: $OSTYPE"
    exit 1
fi

echo "Build complete! Binary is in the dist/ directory:"
ls -lh dist/
