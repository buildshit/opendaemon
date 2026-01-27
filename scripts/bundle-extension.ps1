# Bundle Rust binaries with VS Code extension (PowerShell)

$ErrorActionPreference = "Stop"

Write-Host "Bundling Rust binaries with VS Code extension..." -ForegroundColor Green

# Create extension bin directory
New-Item -ItemType Directory -Force -Path extension/bin | Out-Null

# Copy all platform binaries to extension
Write-Host "Copying binaries to extension/bin..." -ForegroundColor Cyan
if (Test-Path dist/dmn-linux-x64) { Copy-Item dist/dmn-linux-x64 extension/bin/ }
if (Test-Path dist/dmn-darwin-x64) { Copy-Item dist/dmn-darwin-x64 extension/bin/ }
if (Test-Path dist/dmn-darwin-arm64) { Copy-Item dist/dmn-darwin-arm64 extension/bin/ }
if (Test-Path dist/dmn-win32-x64.exe) { Copy-Item dist/dmn-win32-x64.exe extension/bin/ }

# IMPORTANT: Create dmn.exe copy for Windows
# Windows command lookup prioritizes .exe files, making this more reliable than .cmd wrappers
# This ensures 'dmn' command works in PowerShell without needing PATH to resolve .cmd files
Write-Host "Creating dmn.exe for Windows..." -ForegroundColor Cyan
if (Test-Path dist/dmn-win32-x64.exe) {
    Copy-Item dist/dmn-win32-x64.exe extension/bin/dmn.exe
    Write-Host "  Created dmn.exe (copy of dmn-win32-x64.exe)" -ForegroundColor Green
}

# Create wrapper scripts for CLI access (as backup for .cmd scenarios)
Write-Host "Creating wrapper scripts..." -ForegroundColor Cyan

# Windows wrapper (dmn.cmd) - kept as backup
$windowsWrapper = @"
@echo off
REM OpenDaemon CLI wrapper for Windows (backup for dmn.exe)
REM Users should primarily use dmn.exe, but this provides .cmd compatibility
"%~dp0dmn-win32-x64.exe" %*
"@
Set-Content -Path "extension/bin/dmn.cmd" -Value $windowsWrapper -NoNewline

# Unix wrapper (dmn) - shell script
$unixWrapper = @"
#!/bin/sh
# OpenDaemon CLI wrapper for Unix systems
# This script allows users to type 'dmn' instead of the platform-specific binary name

DIR="`$(dirname "`$0")"

# Detect platform and architecture
OS="`$(uname -s)"
ARCH="`$(uname -m)"

case "`$OS-`$ARCH" in
    Darwin-arm64)
        exec "`$DIR/dmn-darwin-arm64" "`$@"
        ;;
    Darwin-x86_64)
        exec "`$DIR/dmn-darwin-x64" "`$@"
        ;;
    Linux-x86_64)
        exec "`$DIR/dmn-linux-x64" "`$@"
        ;;
    Linux-aarch64)
        exec "`$DIR/dmn-linux-arm64" "`$@"
        ;;
    *)
        echo "Unsupported platform: `$OS-`$ARCH" >&2
        echo "Supported platforms: macOS (Intel/Apple Silicon), Linux (x64/arm64)" >&2
        exit 1
        ;;
esac
"@
Set-Content -Path "extension/bin/dmn" -Value $unixWrapper -NoNewline

Write-Host "Binaries and wrapper scripts bundled successfully!" -ForegroundColor Green
Get-ChildItem extension/bin/ | Format-Table Name, Length
