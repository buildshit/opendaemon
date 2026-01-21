#!/bin/bash
# Test extension installation

set -e

GREEN='\033[0;32m'
CYAN='\033[0;36m'
YELLOW='\033[1;33m'
RED='\033[0;31m'
NC='\033[0m' # No Color

echo -e "${GREEN}Testing OpenDaemon extension installation...${NC}"

# Find the VSIX file
VSIX_FILE=$(ls -t dist/*.vsix 2>/dev/null | head -n1)

if [ -z "$VSIX_FILE" ]; then
    echo -e "${RED}No VSIX file found. Run package-extension.sh first.${NC}"
    exit 1
fi

echo -e "\n${CYAN}Package: $(basename "$VSIX_FILE")${NC}"

# Check if VS Code is installed
if ! command -v code &> /dev/null; then
    echo -e "${RED}VS Code 'code' command not found in PATH.${NC}"
    echo -e "${YELLOW}Please ensure VS Code is installed and added to PATH.${NC}"
    exit 1
fi

CODE_VERSION=$(code --version | head -n1)
echo -e "${GREEN}✓ VS Code found: $CODE_VERSION${NC}"

# Uninstall existing version (if any)
echo -e "\n${YELLOW}Uninstalling existing version (if any)...${NC}"
code --uninstall-extension opendaemon.opendaemon 2>&1 || true

# Install the extension
echo -e "\n${CYAN}Installing extension...${NC}"
if code --install-extension "$VSIX_FILE" --force; then
    echo -e "${GREEN}✓ Extension installed successfully${NC}"
else
    echo -e "${RED}✗ Extension installation failed${NC}"
    exit 1
fi

# Verify installation
echo -e "\n${YELLOW}Verifying installation...${NC}"
if code --list-extensions | grep -q "opendaemon"; then
    echo -e "${GREEN}✓ Extension is installed${NC}"
else
    echo -e "${RED}✗ Extension not found in installed extensions${NC}"
    exit 1
fi

echo -e "\n${GREEN}========================================"
echo -e "Installation test complete!"
echo -e "========================================${NC}"
echo -e "\n${YELLOW}Next steps:${NC}"
echo -e "1. Restart VS Code"
echo -e "2. Open a workspace with a dmn.json file"
echo -e "3. Check the 'OpenDaemon Services' view in the Explorer"
echo -e "4. Try starting/stopping services"
