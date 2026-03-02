#!/bin/bash
# Build script for cross-compiling dmn binary for all platforms

set -e

echo "Building dmn for all platforms..."

# Create output directory
mkdir -p dist

# Build for Linux x86_64
echo "Building for Linux x86_64..."
cargo build --release --target x86_64-unknown-linux-gnu --package dmn-core
cp target/x86_64-unknown-linux-gnu/release/dmn dist/dmn-linux-x64

# Build for Linux ARM64
echo "Building for Linux ARM64..."
cargo build --release --target aarch64-unknown-linux-gnu --package dmn-core
cp target/aarch64-unknown-linux-gnu/release/dmn dist/dmn-linux-arm64

# Build for macOS x86_64
echo "Building for macOS x86_64..."
cargo build --release --target x86_64-apple-darwin --package dmn-core
cp target/x86_64-apple-darwin/release/dmn dist/dmn-darwin-x64

# Build for macOS ARM64
echo "Building for macOS ARM64..."
cargo build --release --target aarch64-apple-darwin --package dmn-core
cp target/aarch64-apple-darwin/release/dmn dist/dmn-darwin-arm64

# Build for Windows x86_64
echo "Building for Windows x86_64..."
cargo build --release --target x86_64-pc-windows-msvc --package dmn-core
cp target/x86_64-pc-windows-msvc/release/dmn.exe dist/dmn-win32-x64.exe

echo "Build complete! Binaries are in the dist/ directory:"
ls -lh dist/
