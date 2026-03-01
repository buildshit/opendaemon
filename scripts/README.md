# Build Scripts

This directory contains scripts for building the `dmn` binary and packaging the VS Code extension.

## Prerequisites

### For Cross-Compilation

To build for all platforms, you need to install the appropriate Rust targets:

```bash
# Linux
rustup target add x86_64-unknown-linux-gnu

# macOS
rustup target add x86_64-apple-darwin
rustup target add aarch64-apple-darwin

# Windows
rustup target add x86_64-pc-windows-msvc
```

### Platform-Specific Requirements

**Linux**: Install cross-compilation toolchains:
```bash
# Ubuntu/Debian
sudo apt-get install gcc-x86-64-linux-gnu gcc-aarch64-linux-gnu
```

**macOS**: Install Xcode Command Line Tools:
```bash
xcode-select --install
```

**Windows**: Install Visual Studio Build Tools with C++ support.

### For Extension Packaging

- Node.js 18+ and npm
- VS Code Extension Manager (`@vscode/vsce`) - installed automatically via npm

## Building Rust Binaries

### Build for Current Platform Only

**Linux/macOS**:
```bash
chmod +x scripts/build-current.sh
./scripts/build-current.sh
```

**Windows**:
```powershell
.\scripts\build-current.ps1
```

### Build for All Platforms

**Linux/macOS**:
```bash
chmod +x scripts/build-all.sh
./scripts/build-all.sh
```

**Windows**:
```powershell
.\scripts\build-all.ps1
```

## Packaging the Extension

### Full Package (All Platforms)

This builds binaries for all platforms and packages the extension:

**Linux/macOS**:
```bash
chmod +x scripts/package-extension.sh
./scripts/package-extension.sh
```

**Windows**:
```powershell
.\scripts\package-extension.ps1
```

This will:
1. Build Rust binaries for all platforms
2. Bundle binaries with the extension
3. Verify all binaries are present
4. Install extension dependencies
5. Compile TypeScript
6. Verify package.json
7. Package extension with vsce
8. Verify package was created
9. Run package tests

### Quick Package (Current Platform Only)

For faster iteration during development:

**Windows**:
```powershell
.\scripts\package-extension-quick.ps1
```

This only builds for the current platform and skips some verification steps.

### Package + Install Workflow (Maintainer Fast Loop)

Run both quick packaging and extension installation in one step:

**Windows**:
```powershell
.\scripts\package-and-install-extension.ps1
```

This script runs:
1. `.\scripts\package-extension-quick.ps1`
2. `.\scripts\install-extension.ps1`

OpenDaemon service automation is also available:

```bash
dmn start extension-package-install
```

The install step uses `--force`, so this workflow is normally non-interactive.
After `Workflow complete...` appears in local supervisor mode, stop the session with `Ctrl+C` (or `dmn stop`).

## Testing

### Test Package Contents

Verify the packaged extension contains all required files:

**Linux/macOS**:
```bash
chmod +x scripts/test-package.sh
./scripts/test-package.sh
```

**Windows**:
```powershell
.\scripts\test-package.ps1
```

This will:
- Extract the VSIX package
- Verify all platform binaries are included
- Test that the current platform binary is executable
- Check that all required JavaScript files are present
- Verify source files are excluded

### Test Installation

Install the extension and verify it works:

**Linux/macOS**:
```bash
chmod +x scripts/test-install.sh
./scripts/test-install.sh
```

**Windows**:
```powershell
.\scripts\test-install.ps1
```

This will:
- Uninstall any existing version
- Install the packaged extension
- Verify the extension appears in VS Code's extension list

## Output

### Binaries

All binaries are placed in the `dist/` directory:
- `dmn-linux-x64` - Linux x86_64
- `dmn-darwin-x64` - macOS x86_64 (Intel)
- `dmn-darwin-arm64` - macOS ARM64 (Apple Silicon)
- `dmn-win32-x64.exe` - Windows x86_64

### Extension Package

The packaged extension is placed in `dist/`:
- `opendaemon-<version>.vsix` - VS Code extension package

## Manual Testing

After installing the extension:

1. Restart VS Code
2. Open a workspace with a `dmn.json` file
3. Check the "OpenDaemon Services" view in the Explorer sidebar
4. Try starting/stopping services
5. View logs in the Output panel

## CI/CD Integration

These scripts can be integrated into CI/CD pipelines:

```yaml
# GitHub Actions example
- name: Build and Package
  run: |
    chmod +x scripts/*.sh
    ./scripts/package-extension.sh

- name: Test Package
  run: ./scripts/test-package.sh

- name: Upload Artifact
  uses: actions/upload-artifact@v3
  with:
    name: opendaemon-vsix
    path: dist/*.vsix
```

## Publishing

### To VS Code Marketplace

```bash
cd extension
npx @vscode/vsce login <publisher-name>
npx @vscode/vsce publish
```

### To Open VSX Registry

```bash
npx ovsx publish dist/opendaemon-*.vsix -p <token>
```

## Troubleshooting

### Missing Linker

If you get linker errors, ensure you have the appropriate cross-compilation toolchain installed.

### Permission Denied

On Linux/macOS, make scripts executable:
```bash
chmod +x scripts/*.sh
```

### Target Not Installed

Install missing targets:
```bash
rustup target add <target-triple>
```

### Extension Packaging Fails

1. Ensure TypeScript compiles: `cd extension && npm run compile`
2. Check package.json is valid
3. Verify all binaries exist in `extension/bin/`

### Binary Not Found in Package

1. Run `./scripts/bundle-extension.sh` (or `.ps1` on Windows)
2. Verify binaries exist in `extension/bin/`
3. Check `.vscodeignore` doesn't exclude `bin/**`

## Script Reference

| Script | Purpose | Platforms |
|--------|---------|-----------|
| `build-current.sh/ps1` | Build binary for current platform | All |
| `build-all.sh/ps1` | Build binaries for all platforms | All |
| `bundle-extension.sh/ps1` | Copy binaries to extension/bin | All |
| `package-extension.sh/ps1` | Full packaging with all platforms | All |
| `package-extension-quick.ps1` | Quick packaging for current platform | Windows |
| `test-package.sh/ps1` | Test package contents | All |
| `test-install.sh/ps1` | Test extension installation | All |
| `install-extension.ps1` | Install the packaged extension | Windows |

