#!/bin/bash
# Bundle Rust binaries with VS Code extension

set -e

echo "Bundling Rust binaries with VS Code extension..."

# Create extension bin directory
mkdir -p extension/bin

# Copy all platform binaries to extension
echo "Copying binaries to extension/bin..."
cp dist/dmn-linux-x64 extension/bin/
cp dist/dmn-darwin-x64 extension/bin/
cp dist/dmn-darwin-arm64 extension/bin/
cp dist/dmn-win32-x64.exe extension/bin/

# Make binaries executable on Unix platforms
chmod +x extension/bin/dmn-linux-x64
chmod +x extension/bin/dmn-darwin-x64
chmod +x extension/bin/dmn-darwin-arm64

echo "Binaries bundled successfully!"
ls -lh extension/bin/
