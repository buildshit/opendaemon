# Extension Packaging Guide

This guide explains how to package the OpenDaemon VS Code extension for distribution.

## Overview

The OpenDaemon extension bundles platform-specific Rust binaries to provide a seamless installation experience across Windows, macOS, and Linux.

## Quick Start

### Package for All Platforms

**Windows**:
```powershell
.\scripts\package-extension.ps1
```

**Linux/macOS**:
```bash
chmod +x scripts/package-extension.sh
./scripts/package-extension.sh
```

This creates a `.vsix` file in the `dist/` directory that includes binaries for all platforms.

### Package for Current Platform Only (Development)

**Windows**:
```powershell
.\scripts\package-extension-quick.ps1
```

This is faster for development but only includes the binary for your current platform.

## Packaging Process

The packaging script performs the following steps:

1. **Build Rust Binaries**: Compiles the `dmn` binary for all target platforms
2. **Bundle Binaries**: Copies binaries to `extension/bin/`
3. **Verify Binaries**: Ensures all required binaries are present
4. **Install Dependencies**: Runs `npm install` if needed
5. **Compile TypeScript**: Compiles `.ts` files to `.js` in `out/`
6. **Verify package.json**: Checks required fields are present
7. **Package with vsce**: Creates the `.vsix` file
8. **Verify Package**: Confirms the package was created successfully
9. **Run Tests**: Validates package contents

## Testing

### Test Package Contents

```powershell
# Windows
.\scripts\test-package.ps1

# Linux/macOS
./scripts/test-package.sh
```

This verifies:
- All platform binaries are included
- Current platform binary is executable
- All required JavaScript files are present
- Source files are excluded

### Test Installation

```powershell
# Windows
.\scripts\test-install.ps1

# Linux/macOS
./scripts/test-install.sh
```

This:
- Uninstalls any existing version
- Installs the packaged extension
- Verifies it appears in VS Code's extension list

## Package Contents

The packaged `.vsix` file includes:

### Required Files
- `extension/package.json` - Extension manifest
- `extension/out/**/*.js` - Compiled TypeScript
- `extension/bin/dmn-*` - Platform-specific binaries

### Excluded Files
- `extension/src/**/*.ts` - TypeScript source files
- `extension/node_modules/` - Dependencies (bundled separately)
- `extension/tsconfig.json` - TypeScript configuration
- `extension/.vscode-test/` - Test artifacts

## Platform-Specific Binaries

The extension includes binaries for all supported platforms:

| Platform | Architecture | Binary Name | Size (approx) |
|----------|-------------|-------------|---------------|
| Windows | x64 | `dmn-win32-x64.exe` | 4-5 MB |
| Linux | x64 | `dmn-linux-x64` | 4-5 MB |
| macOS | x64 (Intel) | `dmn-darwin-x64` | 4-5 MB |
| macOS | arm64 (Apple Silicon) | `dmn-darwin-arm64` | 4-5 MB |

The extension automatically selects the correct binary at runtime based on the user's platform.

## Configuration Files

### package.json

Key fields for packaging:

```json
{
  "name": "opendaemon",
  "version": "0.1.0",
  "publisher": "opendaemon",
  "engines": {
    "vscode": "^1.85.0"
  },
  "main": "./out/extension.js",
  "scripts": {
    "vscode:prepublish": "npm run compile",
    "package": "vsce package"
  }
}
```

### .vscodeignore

Controls which files are included/excluded:

```
# Exclude source files
src/**
tsconfig.json
node_modules/**

# Include binaries
!bin/**
```

## Manual Packaging

If you need to package manually:

```bash
cd extension

# Install dependencies
npm install

# Compile TypeScript
npm run compile

# Package with vsce
npx @vscode/vsce package --out ../dist/
```

## Publishing

### To VS Code Marketplace

1. Create a publisher account at https://marketplace.visualstudio.com/
2. Get a Personal Access Token (PAT)
3. Login:
   ```bash
   cd extension
   npx @vscode/vsce login <publisher-name>
   ```
4. Publish:
   ```bash
   npx @vscode/vsce publish
   ```

### To Open VSX Registry

```bash
npx ovsx publish dist/opendaemon-*.vsix -p <token>
```

## Troubleshooting

### Missing Binaries

**Problem**: Package is missing binaries for some platforms.

**Solution**:
1. Run `./scripts/build-all.sh` (or `.ps1` on Windows)
2. Verify binaries exist in `dist/`
3. Run `./scripts/bundle-extension.sh` to copy them to `extension/bin/`

### TypeScript Compilation Errors

**Problem**: `npm run compile` fails.

**Solution**:
1. Check for syntax errors in `.ts` files
2. Run `npm install` to ensure dependencies are installed
3. Check `tsconfig.json` is valid

### Package Too Large

**Problem**: `.vsix` file is very large (>20MB).

**Solution**:
1. Ensure Rust binaries are built in release mode with stripping:
   ```toml
   [profile.release]
   strip = true
   lto = true
   ```
2. Check `.vscodeignore` excludes unnecessary files
3. Verify `node_modules/` is excluded

### Binary Not Executable

**Problem**: Binary fails to run after installation.

**Solution**:
1. On Unix systems, ensure binary has execute permissions
2. Check binary is not corrupted (compare checksums)
3. Verify correct binary for platform is being selected

### vsce Command Not Found

**Problem**: `npx @vscode/vsce` fails.

**Solution**:
```bash
cd extension
npm install --save-dev @vscode/vsce
```

## Version Management

When releasing a new version:

1. Update version in `extension/package.json`
2. Update version in `Cargo.toml` (workspace.package.version)
3. Rebuild binaries: `./scripts/build-all.sh`
4. Package extension: `./scripts/package-extension.sh`
5. Test package: `./scripts/test-package.sh`
6. Tag release: `git tag v0.1.0 && git push --tags`

## CI/CD Integration

Example GitHub Actions workflow:

```yaml
name: Package Extension

on:
  push:
    tags:
      - 'v*'

jobs:
  package:
    runs-on: ubuntu-latest
    
    steps:
      - uses: actions/checkout@v3
      
      - name: Install Rust
        uses: actions-rs/toolchain@v1
        with:
          toolchain: stable
      
      - name: Install Node.js
        uses: actions/setup-node@v3
        with:
          node-version: '18'
      
      - name: Install Rust targets
        run: |
          rustup target add x86_64-unknown-linux-gnu
          rustup target add x86_64-apple-darwin
          rustup target add aarch64-apple-darwin
          rustup target add x86_64-pc-windows-msvc
      
      - name: Package extension
        run: |
          chmod +x scripts/*.sh
          ./scripts/package-extension.sh
      
      - name: Test package
        run: ./scripts/test-package.sh
      
      - name: Upload artifact
        uses: actions/upload-artifact@v3
        with:
          name: opendaemon-vsix
          path: dist/*.vsix
```

## Best Practices

1. **Always test before publishing**: Run `test-package.sh` and `test-install.sh`
2. **Include all platforms**: Users expect cross-platform support
3. **Keep package size reasonable**: Aim for <20MB total
4. **Version consistently**: Keep extension and binary versions in sync
5. **Test on all platforms**: Verify binary selection works correctly
6. **Document changes**: Update CHANGELOG.md for each release

## Support

For issues with packaging:
1. Check the troubleshooting section above
2. Review `scripts/README.md` for detailed script documentation
3. Check the main `PACKAGING.md` in the repository root
4. Open an issue on GitHub with packaging logs
