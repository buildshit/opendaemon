#!/bin/bash
# Bundle Rust binaries with VS Code extension

set -e

echo "Bundling Rust binaries with VS Code extension..."

# Create extension bin directory
mkdir -p extension/bin

# Copy all platform binaries to extension
echo "Copying binaries to extension/bin..."
cp dist/dmn-linux-x64 extension/bin/
cp dist/dmn-linux-arm64 extension/bin/
cp dist/dmn-darwin-x64 extension/bin/
cp dist/dmn-darwin-arm64 extension/bin/
cp dist/dmn-win32-x64.exe extension/bin/

# Create Windows command shims.
cp dist/dmn-win32-x64.exe extension/bin/dmn.exe
cat > extension/bin/dmn.cmd <<'EOF'
@echo off
REM OpenDaemon CLI wrapper for Windows (backup for dmn.exe)
"%~dp0dmn-win32-x64.exe" %*
EOF

# Create Unix wrapper for "dmn" command.
cat > extension/bin/dmn <<'EOF'
#!/bin/sh
# OpenDaemon CLI wrapper for Unix systems

DIR="$(dirname "$0")"
OS="$(uname -s)"
ARCH="$(uname -m)"

case "$OS-$ARCH" in
    Darwin-arm64)
        exec "$DIR/dmn-darwin-arm64" "$@"
        ;;
    Darwin-x86_64)
        exec "$DIR/dmn-darwin-x64" "$@"
        ;;
    Linux-x86_64)
        exec "$DIR/dmn-linux-x64" "$@"
        ;;
    Linux-aarch64)
        exec "$DIR/dmn-linux-arm64" "$@"
        ;;
    *)
        echo "Unsupported platform: $OS-$ARCH" >&2
        echo "Supported platforms: Windows (x64), macOS (Intel/Apple Silicon), Linux (x64/arm64)" >&2
        exit 1
        ;;
esac
EOF

# Make Unix artifacts executable.
chmod +x extension/bin/dmn
chmod +x extension/bin/dmn-linux-x64
chmod +x extension/bin/dmn-linux-arm64
chmod +x extension/bin/dmn-darwin-x64
chmod +x extension/bin/dmn-darwin-arm64

echo "Binaries bundled successfully!"
ls -lh extension/bin/
