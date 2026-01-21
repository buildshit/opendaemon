# Quick package script for testing (PowerShell)

$ErrorActionPreference = "Stop"

Write-Host "Quick packaging OpenDaemon VS Code extension..." -ForegroundColor Green

# Ensure binary exists
if (-not (Test-Path "extension/bin/dmn-win32-x64.exe")) {
    Write-Host "Binary not found, building..." -ForegroundColor Yellow
    & .\scripts\build-current.ps1
    New-Item -ItemType Directory -Force -Path extension/bin | Out-Null
    Copy-Item dist/dmn-win32-x64.exe extension/bin/
}

# Compile TypeScript
Write-Host "Compiling TypeScript..." -ForegroundColor Cyan
Push-Location extension
npm run compile
if ($LASTEXITCODE -ne 0) {
    Pop-Location
    throw "TypeScript compilation failed"
}

# Package extension with vsce
Write-Host "Packaging extension..." -ForegroundColor Cyan
npx @vscode/vsce package --out ../dist/ --no-dependencies
if ($LASTEXITCODE -ne 0) {
    Pop-Location
    throw "Extension packaging failed"
}

Pop-Location

Write-Host "Extension packaged successfully!" -ForegroundColor Green
Get-ChildItem dist/*.vsix | Format-Table Name, Length
