#!/bin/bash
# Package VS Code extension with bundled binaries

set -e

GREEN='\033[0;32m'
CYAN='\033[0;36m'
YELLOW='\033[1;33m'
RED='\033[0;31m'
NC='\033[0m' # No Color

echo -e "${GREEN}Packaging OpenDaemon VS Code extension...${NC}"

if [ "${1:-}" = "--build-all" ]; then
    echo -e "${CYAN}Step 1/9: Building Rust binaries for all platforms...${NC}"
    bash ./scripts/build-all.sh
else
    echo -e "${CYAN}Step 1/9: Skipping build-all (using existing dist binaries).${NC}"
fi

# Step 2: Verify dist binaries
echo -e "${CYAN}Step 2/9: Verifying dist binaries...${NC}"
REQUIRED_DIST_BINARIES=(
    "dist/dmn-win32-x64.exe"
    "dist/dmn-linux-x64"
    "dist/dmn-linux-arm64"
    "dist/dmn-darwin-x64"
    "dist/dmn-darwin-arm64"
)

MISSING_DIST_BINARIES=()
for binary in "${REQUIRED_DIST_BINARIES[@]}"; do
    if [ ! -f "$binary" ]; then
        MISSING_DIST_BINARIES+=("$binary")
        echo -e "  ${RED}✗ Missing: $binary${NC}"
    else
        size=$(du -h "$binary" | cut -f1)
        echo -e "  ${GREEN}✓ Found: $binary ($size)${NC}"
    fi
done

if [ ${#MISSING_DIST_BINARIES[@]} -ne 0 ]; then
    echo -e "${RED}Missing dist binaries: ${MISSING_DIST_BINARIES[*]}${NC}"
    echo -e "${YELLOW}Build all binaries in CI first, or run ./scripts/package-extension.sh --build-all${NC}"
    exit 1
fi

# Step 3: Bundle binaries with extension
echo -e "${CYAN}Step 3/9: Bundling binaries with extension...${NC}"
bash ./scripts/bundle-extension.sh

# Step 4: Verify bundled binaries and wrappers
echo -e "${CYAN}Step 4/9: Verifying bundled binaries...${NC}"
REQUIRED_BUNDLED_FILES=(
    "extension/bin/dmn-win32-x64.exe"
    "extension/bin/dmn.exe"
    "extension/bin/dmn.cmd"
    "extension/bin/dmn-linux-x64"
    "extension/bin/dmn-linux-arm64"
    "extension/bin/dmn-darwin-x64"
    "extension/bin/dmn-darwin-arm64"
    "extension/bin/dmn"
)

MISSING_BUNDLED_FILES=()
for file in "${REQUIRED_BUNDLED_FILES[@]}"; do
    if [ ! -f "$file" ]; then
        MISSING_BUNDLED_FILES+=("$file")
        echo -e "  ${RED}✗ Missing: $file${NC}"
    else
        size=$(du -h "$file" | cut -f1)
        echo -e "  ${GREEN}✓ Found: $file ($size)${NC}"
    fi
done

if [ ${#MISSING_BUNDLED_FILES[@]} -ne 0 ]; then
    echo -e "${RED}Missing bundled files: ${MISSING_BUNDLED_FILES[*]}${NC}"
    exit 1
fi

# Step 5: Install extension dependencies
echo -e "${CYAN}Step 5/9: Installing extension dependencies...${NC}"
cd extension
if [ ! -d "node_modules" ]; then
    npm install
fi

# Step 6: Compile TypeScript
echo -e "${CYAN}Step 6/9: Compiling TypeScript...${NC}"
npm run compile

# Verify compiled output
if [ ! -f "out/extension.js" ]; then
    echo -e "${RED}TypeScript compilation did not produce expected output${NC}"
    exit 1
fi
echo -e "  ${GREEN}✓ TypeScript compiled successfully${NC}"

# Step 7: Verify package.json
echo -e "${CYAN}Step 7/9: Verifying package.json...${NC}"
PACKAGE_NAME=$(node -p "require('./package.json').name")
PACKAGE_VERSION=$(node -p "require('./package.json').version")
PACKAGE_PUBLISHER=$(node -p "require('./package.json').publisher")
PACKAGE_LICENSE=$(node -p "require('./package.json').license || ''")

if [ -z "$PACKAGE_NAME" ] || [ -z "$PACKAGE_VERSION" ] || [ -z "$PACKAGE_PUBLISHER" ] || [ -z "$PACKAGE_LICENSE" ]; then
    echo -e "${RED}package.json missing required fields${NC}"
    exit 1
fi

echo -e "  ${GREEN}✓ Package: $PACKAGE_NAME v$PACKAGE_VERSION${NC}"
echo -e "  ${GREEN}✓ Publisher: $PACKAGE_PUBLISHER${NC}"
echo -e "  ${GREEN}✓ License: $PACKAGE_LICENSE${NC}"

# Step 8: Package extension with vsce
echo -e "${CYAN}Step 8/9: Packaging extension with vsce...${NC}"
npx @vscode/vsce package --out ../dist/ --no-dependencies

cd ..

# Step 9: Verify package and run package tests
echo -e "${CYAN}Step 9/9: Verifying package...${NC}"
VSIX_FILE=$(ls -t dist/*.vsix 2>/dev/null | head -n1)

if [ -z "$VSIX_FILE" ]; then
    echo -e "${RED}VSIX file was not created${NC}"
    exit 1
fi

PACKAGE_SIZE=$(du -h "$VSIX_FILE" | cut -f1)
echo -e "  ${GREEN}✓ Package created: $(basename "$VSIX_FILE")${NC}"
echo -e "  ${GREEN}✓ Size: $PACKAGE_SIZE${NC}"

bash ./scripts/test-package.sh || echo -e "${YELLOW}Warning: package tests reported issues${NC}"

echo -e "\n${GREEN}========================================"
echo -e "Extension packaged successfully!"
echo -e "========================================${NC}"
echo -e "\n${CYAN}Package: $VSIX_FILE${NC}"
echo -e "${CYAN}Size: $PACKAGE_SIZE${NC}"
echo -e "\n${YELLOW}To install:${NC}"
echo -e "  code --install-extension \"$VSIX_FILE\" --force"
echo -e "\n${YELLOW}To publish to VS Code Marketplace:${NC}"
echo -e "  cd extension"
echo -e "  npx @vscode/vsce publish"
echo -e "\n${YELLOW}To publish to Open VSX:${NC}"
echo -e "  cd extension"
echo -e "  npx ovsx publish ../dist/$(basename "$VSIX_FILE") -p <OVSX_PAT>"
