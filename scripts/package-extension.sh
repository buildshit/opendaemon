#!/bin/bash
# Package VS Code extension with bundled binaries

set -e

GREEN='\033[0;32m'
CYAN='\033[0;36m'
YELLOW='\033[1;33m'
RED='\033[0;31m'
NC='\033[0m' # No Color

echo -e "${GREEN}Packaging OpenDaemon VS Code extension...${NC}"

# Step 1: Build Rust binaries for all platforms
echo -e "${CYAN}Step 1: Building Rust binaries for all platforms...${NC}"
./scripts/build-all.sh

# Step 2: Bundle binaries with extension
echo -e "${CYAN}Step 2: Bundling binaries with extension...${NC}"
./scripts/bundle-extension.sh

# Step 3: Verify binaries exist
echo -e "${CYAN}Step 3: Verifying bundled binaries...${NC}"
REQUIRED_BINARIES=(
    "extension/bin/dmn-win32-x64.exe"
    "extension/bin/dmn-linux-x64"
    "extension/bin/dmn-darwin-x64"
    "extension/bin/dmn-darwin-arm64"
)

MISSING_BINARIES=()
for binary in "${REQUIRED_BINARIES[@]}"; do
    if [ ! -f "$binary" ]; then
        MISSING_BINARIES+=("$binary")
        echo -e "  ${RED}✗ Missing: $binary${NC}"
    else
        size=$(du -h "$binary" | cut -f1)
        echo -e "  ${GREEN}✓ Found: $binary ($size)${NC}"
    fi
done

if [ ${#MISSING_BINARIES[@]} -ne 0 ]; then
    echo -e "${RED}Missing binaries: ${MISSING_BINARIES[*]}${NC}"
    exit 1
fi

# Step 4: Install extension dependencies
echo -e "${CYAN}Step 4: Installing extension dependencies...${NC}"
cd extension
if [ ! -d "node_modules" ]; then
    npm install
fi

# Step 5: Compile TypeScript
echo -e "${CYAN}Step 5: Compiling TypeScript...${NC}"
npm run compile

# Verify compiled output
if [ ! -f "out/extension.js" ]; then
    echo -e "${RED}TypeScript compilation did not produce expected output${NC}"
    exit 1
fi
echo -e "  ${GREEN}✓ TypeScript compiled successfully${NC}"

# Step 6: Verify package.json
echo -e "${CYAN}Step 6: Verifying package.json...${NC}"
PACKAGE_NAME=$(node -p "require('./package.json').name")
PACKAGE_VERSION=$(node -p "require('./package.json').version")
PACKAGE_PUBLISHER=$(node -p "require('./package.json').publisher")

if [ -z "$PACKAGE_NAME" ] || [ -z "$PACKAGE_VERSION" ] || [ -z "$PACKAGE_PUBLISHER" ]; then
    echo -e "${RED}package.json missing required fields${NC}"
    exit 1
fi

echo -e "  ${GREEN}✓ Package: $PACKAGE_NAME v$PACKAGE_VERSION${NC}"
echo -e "  ${GREEN}✓ Publisher: $PACKAGE_PUBLISHER${NC}"

# Step 7: Package extension with vsce
echo -e "${CYAN}Step 7: Packaging extension with vsce...${NC}"
npx @vscode/vsce package --out ../dist/

cd ..

# Step 8: Verify package was created
echo -e "${CYAN}Step 8: Verifying package...${NC}"
VSIX_FILE=$(ls -t dist/*.vsix 2>/dev/null | head -n1)

if [ -z "$VSIX_FILE" ]; then
    echo -e "${RED}VSIX file was not created${NC}"
    exit 1
fi

PACKAGE_SIZE=$(du -h "$VSIX_FILE" | cut -f1)
echo -e "  ${GREEN}✓ Package created: $(basename "$VSIX_FILE")${NC}"
echo -e "  ${GREEN}✓ Size: $PACKAGE_SIZE${NC}"

echo -e "\n${GREEN}========================================"
echo -e "Extension packaged successfully!"
echo -e "========================================${NC}"
echo -e "\n${CYAN}Package: $VSIX_FILE${NC}"
echo -e "${CYAN}Size: $PACKAGE_SIZE${NC}"
echo -e "\n${YELLOW}To install:${NC}"
echo -e "  code --install-extension \"$VSIX_FILE\" --force"
echo -e "\n${YELLOW}To publish:${NC}"
echo -e "  cd extension"
echo -e "  npx @vscode/vsce publish"
