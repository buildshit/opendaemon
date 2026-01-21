# Bundle Rust binaries with VS Code extension (PowerShell)

$ErrorActionPreference = "Stop"

Write-Host "Bundling Rust binaries with VS Code extension..." -ForegroundColor Green

# Create extension bin directory
New-Item -ItemType Directory -Force -Path extension/bin | Out-Null

# Copy all platform binaries to extension
Write-Host "Copying binaries to extension/bin..." -ForegroundColor Cyan
Copy-Item dist/dmn-linux-x64 extension/bin/
Copy-Item dist/dmn-darwin-x64 extension/bin/
Copy-Item dist/dmn-darwin-arm64 extension/bin/
Copy-Item dist/dmn-win32-x64.exe extension/bin/

Write-Host "Binaries bundled successfully!" -ForegroundColor Green
Get-ChildItem extension/bin/ | Format-Table Name, Length
