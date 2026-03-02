# Build script for cross-compiling dmn binary for all platforms (PowerShell)

$ErrorActionPreference = "Stop"

Write-Host "Building dmn for all platforms..." -ForegroundColor Green

# Create output directory
New-Item -ItemType Directory -Force -Path dist | Out-Null

# Build for Linux x86_64
Write-Host "Building for Linux x86_64..." -ForegroundColor Cyan
cargo build --release --target x86_64-unknown-linux-gnu --package dmn-core
Copy-Item target/x86_64-unknown-linux-gnu/release/dmn dist/dmn-linux-x64

# Build for Linux ARM64
Write-Host "Building for Linux ARM64..." -ForegroundColor Cyan
cargo build --release --target aarch64-unknown-linux-gnu --package dmn-core
Copy-Item target/aarch64-unknown-linux-gnu/release/dmn dist/dmn-linux-arm64

# Build for macOS x86_64
Write-Host "Building for macOS x86_64..." -ForegroundColor Cyan
cargo build --release --target x86_64-apple-darwin --package dmn-core
Copy-Item target/x86_64-apple-darwin/release/dmn dist/dmn-darwin-x64

# Build for macOS ARM64
Write-Host "Building for macOS ARM64..." -ForegroundColor Cyan
cargo build --release --target aarch64-apple-darwin --package dmn-core
Copy-Item target/aarch64-apple-darwin/release/dmn dist/dmn-darwin-arm64

# Build for Windows x86_64
Write-Host "Building for Windows x86_64..." -ForegroundColor Cyan
cargo build --release --target x86_64-pc-windows-msvc --package dmn-core
Copy-Item target/x86_64-pc-windows-msvc/release/dmn.exe dist/dmn-win32-x64.exe

Write-Host "Build complete! Binaries are in the dist/ directory:" -ForegroundColor Green
Get-ChildItem dist/ | Format-Table Name, Length
