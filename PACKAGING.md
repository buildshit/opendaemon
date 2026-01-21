# OpenDaemon Packaging Guide

This document describes how to build and package the OpenDaemon VS Code extension with the Rust binary.

## Overview

OpenDaemon consists of two main components:
1. **Rust Binary (`dmn`)**: The core orchestration engine
2. **VS Code Extension**: The UI and VS Code integration

The extension bundles platform-specific binaries to provide a seamless installation experience.

## Prerequisites

### For Building Rust Binary

- Rust toolchain (install from https://rustup.rs/)
- Platform-specific build tools:
  - **Windows**: Visual Studio Build Tools with C++ support
  - **macOS**: Xcode Command Line Tools
  - **Linux**: GCC and development libraries

### For Building Extension

- Node.js 18+ and npm
- VS Code Extension Manager (`@vscode/vsce`)

## Quick Start

### Build and Package for Current Platform

**Windows**:
```powershell
.\scripts\package-extension-quick.ps1
```

**Linux/macOS**:
```bash
chmod +x scripts/*.sh
./scripts/package-extension.sh
```

This will:
1. Build the Rust binary for your current platform
2. Bundle it with the extension
3. Compile TypeScript
4. Create a `.vsix` package in `dist/`

### Install the Extension

**Windows**:
```powershell
.\scripts\install-extension.ps1
```

**Linux/macOS**:
```bash
code --install-extension dist/opendaemon-*.vsix --force
```

## Detailed Build Process

### Step 1: Build Rust Binary

#### Current Platform Only

**Windows**:
```powershell
.\scripts\build-current.ps1
```

**Linux/macOS**:
```bash
./scripts/build-current.sh
```

Output: `dist/dmn-<platform>-<arch>[.exe]`

#### All Platforms (Cross-Compilation)

**Requirements**:
- Install Rust targets:
  ```bash
  rustup target add x86_64-unknown-linux-gnu
  rustup target add x86_64-apple-darwin
  rustup target add aarch64-apple-darwin
  rustup target add x86_64-pc-windows-msvc
  ```

**Windows**:
```powershell
.\scripts\build-all.ps1
```

**Linux/macOS**:
```bash
./scripts/build-all.sh
```

Output:
- `dist/dmn-linux-x64`
- `dist/dmn-darwin-x64`
- `dist/dmn-darwin-arm64`
- `dist/dmn-win32-x64.exe`

### Step 2: Bundle Binaries with Extension

**Windows**:
```powershell
.\scripts\bundle-extension.ps1
```

**Linux/macOS**:
```bash
./scripts/bundle-extension.sh
```

This copies all platform binaries to `extension/bin/`.

### Step 3: Compile TypeScript

```bash
cd extension
npm install
npm run compile
```

### Step 4: Package Extension

```bash
cd extension
npx @vscode/vsce package --out ../dist/
```

Or use the all-in-one script:

**Windows**:
```powershell
.\scripts\package-extension.ps1
```

**Linux/macOS**:
```bash
./scripts/package-extension.sh
```

## Platform-Specific Binary Selection

The extension automatically selects the correct binary based on the platform:

| Platform | Architecture | Binary Name |
|----------|-------------|-------------|
| Windows | x64 | `dmn-win32-x64.exe` |
| macOS | x64 (Intel) | `dmn-darwin-x64` |
| macOS | arm64 (Apple Silicon) | `dmn-darwin-arm64` |
| Linux | x64 | `dmn-linux-x64` |

The selection logic is in `extension/src/daemon.ts`:

```typescript
private getBinaryPath(): string {
    const platform = process.platform;
    const arch = process.arch;
    let binaryName: string;
    
    if (platform === 'win32') {
        binaryName = 'dmn-win32-x64.exe';
    } else if (platform === 'darwin') {
        binaryName = arch === 'arm64' ? 'dmn-darwin-arm64' : 'dmn-darwin-x64';
    } else if (platform === 'linux') {
        binaryName = 'dmn-linux-x64';
    }
    
    return path.join(this.context.extensionPath, 'bin', binaryName);
}
```

## Testing the Package

### 1. Verify Binary Exists

```bash
# Windows
Test-Path extension/bin/dmn-win32-x64.exe

# Linux/macOS
ls -lh extension/bin/
```

### 2. Test Binary Selection Logic

```bash
node scripts/test-binary-selection.js
```

### 3. Install and Test Extension

```bash
# Install
code --install-extension dist/opendaemon-*.vsix --force

# Restart VS Code and test:
# 1. Open a workspace with dmn.json
# 2. Check if OpenDaemon Services view appears
# 3. Try starting/stopping services
```

## CI/CD Integration

### GitHub Actions Example

```yaml
name: Build and Package

on: [push, pull_request]

jobs:
  build:
    strategy:
      matrix:
        os: [ubuntu-latest, macos-latest, windows-latest]
    
    runs-on: ${{ matrix.os }}
    
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
      
      - name: Build Rust binary
        run: cargo build --release --package dmn-core
      
      - name: Install extension dependencies
        run: |
          cd extension
          npm install
      
      - name: Package extension
        shell: bash
        run: |
          if [ "$RUNNER_OS" == "Windows" ]; then
            pwsh scripts/package-extension.ps1
          else
            chmod +x scripts/*.sh
            ./scripts/package-extension.sh
          fi
      
      - name: Upload artifact
        uses: actions/upload-artifact@v3
        with:
          name: opendaemon-${{ matrix.os }}
          path: dist/*.vsix
```

## Troubleshooting

### Binary Not Found Error

**Symptom**: Extension fails to start with "Binary not found" error.

**Solution**:
1. Verify binary exists: `ls extension/bin/`
2. Check binary name matches platform
3. Ensure binary is executable (Unix): `chmod +x extension/bin/dmn-*`

### Cross-Compilation Errors

**Symptom**: Build fails for non-native targets.

**Solution**:
1. Install target: `rustup target add <target-triple>`
2. Install cross-compilation toolchain (Linux):
   ```bash
   sudo apt-get install gcc-x86-64-linux-gnu gcc-aarch64-linux-gnu
   ```

### Extension Packaging Fails

**Symptom**: `vsce package` fails with errors.

**Solution**:
1. Ensure TypeScript compiles: `npm run compile`
2. Check package.json is valid
3. Add missing fields (repository, license)
4. Use `--no-dependencies` flag if needed

### Large Package Size

**Symptom**: VSIX file is very large (>10MB).

**Solution**:
1. Ensure Rust binary is built in release mode with stripping:
   ```toml
   [profile.release]
   strip = true
   lto = true
   ```
2. Exclude unnecessary files in `.vscodeignore`
3. Consider using UPX to compress binaries (optional)

## File Structure

```
opendaemon/
├── dist/                          # Build output
│   ├── dmn-linux-x64
│   ├── dmn-darwin-x64
│   ├── dmn-darwin-arm64
│   ├── dmn-win32-x64.exe
│   └── opendaemon-0.1.0.vsix
├── extension/
│   ├── bin/                       # Bundled binaries (gitignored)
│   │   ├── dmn-linux-x64
│   │   ├── dmn-darwin-x64
│   │   ├── dmn-darwin-arm64
│   │   └── dmn-win32-x64.exe
│   ├── src/                       # TypeScript source
│   ├── out/                       # Compiled JavaScript
│   └── package.json
├── core/                          # Rust source
│   ├── src/
│   └── Cargo.toml
└── scripts/                       # Build scripts
    ├── build-all.sh
    ├── build-all.ps1
    ├── build-current.sh
    ├── build-current.ps1
    ├── bundle-extension.sh
    ├── bundle-extension.ps1
    ├── package-extension.sh
    ├── package-extension.ps1
    └── install-extension.ps1
```

## Publishing

### To VS Code Marketplace

1. Create a publisher account at https://marketplace.visualstudio.com/
2. Get a Personal Access Token (PAT)
3. Login with vsce:
   ```bash
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

## Version Management

Update version in both:
1. `extension/package.json`
2. `Cargo.toml` (workspace.package.version)

Then rebuild and package.

## License

Ensure LICENSE file exists in the root directory before publishing.
