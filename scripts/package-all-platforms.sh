#!/bin/bash
# Package VS Code extension with binaries for all platforms

set -e

echo "Packaging OpenDaemon VS Code extension for all platforms..."

# Step 1: Build Rust binaries for all platforms
echo "Step 1: Building Rust binaries for all platforms..."
./scripts/build-all.sh

# Step 2: Bundle binaries with extension
echo "Step 2: Bundling binaries with extension..."
./scripts/bundle-extension.sh

# Step 3: Compile TypeScript
echo "Step 3: Compiling TypeScript..."
cd extension
npm run compile

# Step 4: Package extension with vsce
echo "Step 4: Packaging extension..."
npx @vscode/vsce package --out ../dist/

cd ..

echo "Extension packaged successfully!"
echo "Output: dist/opendaemon-*.vsix"
ls -lh dist/*.vsix

echo ""
echo "To install the extension:"
echo "  code --install-extension dist/opendaemon-*.vsix"
