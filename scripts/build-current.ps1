# Build script for current platform only (PowerShell)

$ErrorActionPreference = "Stop"

Write-Host "Building dmn for current platform..." -ForegroundColor Green

# Create output directory
New-Item -ItemType Directory -Force -Path dist | Out-Null

# Build for current platform
cargo build --release --package dmn-core

# Copy binary for Windows
Copy-Item target/release/dmn.exe dist/dmn-win32-x64.exe
Write-Host "Built for Windows x86_64" -ForegroundColor Cyan

Write-Host "Build complete! Binary is in the dist/ directory:" -ForegroundColor Green
Get-ChildItem dist/ | Format-Table Name, Length
