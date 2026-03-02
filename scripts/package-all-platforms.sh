#!/bin/bash
# Package VS Code extension with binaries for all platforms.
# This script is a convenience alias for the full packaging workflow.

set -e

echo "Packaging OpenDaemon VS Code extension for all platforms..."
bash ./scripts/package-extension.sh --build-all
