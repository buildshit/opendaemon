#!/bin/bash
# Test the packaged extension

set -e

GREEN='\033[0;32m'
CYAN='\033[0;36m'
YELLOW='\033[1;33m'
RED='\033[0;31m'
NC='\033[0m' # No Color

echo -e "${GREEN}Testing OpenDaemon package...${NC}"

# Find the VSIX file
VSIX_FILE=$(ls -t dist/*.vsix 2>/dev/null | head -n1)

if [ -z "$VSIX_FILE" ]; then
    echo -e "${RED}No VSIX file found. Run package-extension.sh first.${NC}"
    exit 1
fi

echo -e "\n${CYAN}Package: $(basename "$VSIX_FILE")${NC}"
PACKAGE_SIZE=$(du -h "$VSIX_FILE" | cut -f1)
echo -e "${CYAN}Size: $PACKAGE_SIZE${NC}"

# Create temp directory
TEMP_DIR=$(mktemp -d)
trap "rm -rf $TEMP_DIR" EXIT

# Extract VSIX (it's a ZIP file)
echo -e "\n${YELLOW}Extracting package...${NC}"
unzip -q "$VSIX_FILE" -d "$TEMP_DIR"

# Verify binary exists in package
echo -e "\n${YELLOW}Verifying binaries in package...${NC}"

# Determine which binary to test based on platform
PLATFORM=$(uname -s)
ARCH=$(uname -m)

if [ "$PLATFORM" = "Darwin" ]; then
    if [ "$ARCH" = "arm64" ]; then
        BINARY_NAME="dmn-darwin-arm64"
    else
        BINARY_NAME="dmn-darwin-x64"
    fi
elif [ "$PLATFORM" = "Linux" ]; then
    BINARY_NAME="dmn-linux-x64"
else
    # Windows (Git Bash/WSL)
    BINARY_NAME="dmn-win32-x64.exe"
fi

BINARY_PATH="$TEMP_DIR/extension/bin/$BINARY_NAME"

if [ -f "$BINARY_PATH" ]; then
    echo -e "${GREEN}✓ Binary found in package: $BINARY_NAME${NC}"
    
    BINARY_SIZE=$(du -h "$BINARY_PATH" | cut -f1)
    echo -e "  ${CYAN}Binary size: $BINARY_SIZE${NC}"
    
    # Test if binary is executable
    chmod +x "$BINARY_PATH"
    if VERSION=$("$BINARY_PATH" --version 2>&1); then
        echo -e "${GREEN}✓ Binary is executable${NC}"
        echo -e "  ${CYAN}Version: $VERSION${NC}"
    else
        echo -e "${RED}✗ Binary failed to execute${NC}"
    fi
else
    echo -e "${RED}✗ Binary not found in package${NC}"
    echo -e "  ${YELLOW}Expected: $BINARY_PATH${NC}"
fi

# Check for all platform binaries
echo -e "\n${YELLOW}Checking all platform binaries...${NC}"
ALL_BINARIES=(
    "dmn-win32-x64.exe"
    "dmn-linux-x64"
    "dmn-darwin-x64"
    "dmn-darwin-arm64"
)

for binary in "${ALL_BINARIES[@]}"; do
    if [ -f "$TEMP_DIR/extension/bin/$binary" ]; then
        size=$(du -h "$TEMP_DIR/extension/bin/$binary" | cut -f1)
        echo -e "${GREEN}✓ $binary ($size)${NC}"
    else
        echo -e "${RED}✗ $binary (missing)${NC}"
    fi
done

# Check for other required files
echo -e "\n${YELLOW}Checking required files...${NC}"

REQUIRED_FILES=(
    "extension/package.json"
    "extension/out/extension.js"
    "extension/out/daemon.js"
    "extension/out/rpc-client.js"
    "extension/out/tree-view.js"
    "extension/out/commands.js"
    "extension/out/logs.js"
)

for file in "${REQUIRED_FILES[@]}"; do
    if [ -f "$TEMP_DIR/$file" ]; then
        echo -e "${GREEN}✓ $file${NC}"
    else
        echo -e "${RED}✗ $file (missing)${NC}"
    fi
done

# Check that source files are NOT included
echo -e "\n${YELLOW}Verifying source files are excluded...${NC}"

EXCLUDED_FILES=(
    "extension/src/extension.ts"
    "extension/tsconfig.json"
    "extension/node_modules"
)

for file in "${EXCLUDED_FILES[@]}"; do
    if [ -e "$TEMP_DIR/$file" ]; then
        echo -e "${RED}✗ $file (should be excluded)${NC}"
    else
        echo -e "${GREEN}✓ $file (correctly excluded)${NC}"
    fi
done

echo -e "\n${GREEN}Package test complete!${NC}"
echo -e "\n${YELLOW}To install:${NC}"
echo -e "  code --install-extension \"$VSIX_FILE\" --force"
