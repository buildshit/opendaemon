# Test extension installation

$ErrorActionPreference = "Stop"

Write-Host "Testing OpenDaemon extension installation..." -ForegroundColor Green

# Find the VSIX file
$vsixFile = Get-ChildItem dist/*.vsix | Sort-Object LastWriteTime -Descending | Select-Object -First 1

if (-not $vsixFile) {
    Write-Host "No VSIX file found. Run package-extension.ps1 first." -ForegroundColor Red
    exit 1
}

Write-Host "`nPackage: $($vsixFile.Name)" -ForegroundColor Cyan

# Check if VS Code is installed
$codeCmd = Get-Command code -ErrorAction SilentlyContinue

if (-not $codeCmd) {
    Write-Host "VS Code 'code' command not found in PATH." -ForegroundColor Red
    Write-Host "Please ensure VS Code is installed and added to PATH." -ForegroundColor Yellow
    exit 1
}

Write-Host "✓ VS Code found: $($codeCmd.Source)" -ForegroundColor Green

# Uninstall existing version (if any)
Write-Host "`nUninstalling existing version (if any)..." -ForegroundColor Yellow
$uninstallOutput = code --uninstall-extension opendaemon.opendaemon 2>&1
Write-Host $uninstallOutput

# Install the extension
Write-Host "`nInstalling extension..." -ForegroundColor Cyan
$installOutput = code --install-extension $vsixFile.FullName --force 2>&1

if ($LASTEXITCODE -eq 0) {
    Write-Host "✓ Extension installed successfully" -ForegroundColor Green
    Write-Host $installOutput
} else {
    Write-Host "✗ Extension installation failed" -ForegroundColor Red
    Write-Host $installOutput
    exit 1
}

# Verify installation
Write-Host "`nVerifying installation..." -ForegroundColor Yellow
$extensions = code --list-extensions 2>&1

if ($extensions -match "opendaemon") {
    Write-Host "✓ Extension is installed" -ForegroundColor Green
} else {
    Write-Host "✗ Extension not found in installed extensions" -ForegroundColor Red
    exit 1
}

Write-Host "`n========================================" -ForegroundColor Green
Write-Host "Installation test complete!" -ForegroundColor Green
Write-Host "========================================" -ForegroundColor Green
Write-Host "`nNext steps:" -ForegroundColor Yellow
Write-Host "1. Restart VS Code" -ForegroundColor White
Write-Host "2. Open a workspace with a dmn.json file" -ForegroundColor White
Write-Host "3. Check the 'OpenDaemon Services' view in the Explorer" -ForegroundColor White
Write-Host "4. Try starting/stopping services" -ForegroundColor White
