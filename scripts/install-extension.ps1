# Install the packaged extension for testing

$ErrorActionPreference = "Stop"

Write-Host "Installing OpenDaemon extension..." -ForegroundColor Green

# Find the VSIX file
$vsixFile = Get-ChildItem dist/*.vsix | Sort-Object LastWriteTime -Descending | Select-Object -First 1

if (-not $vsixFile) {
    Write-Host "No VSIX file found in dist/. Run package-extension.ps1 first." -ForegroundColor Red
    exit 1
}

Write-Host "Installing: $($vsixFile.Name)" -ForegroundColor Cyan

# Install the extension
code --install-extension $vsixFile.FullName --force

if ($LASTEXITCODE -eq 0) {
    Write-Host "Extension installed successfully!" -ForegroundColor Green
    Write-Host "Restart VS Code to activate the extension." -ForegroundColor Yellow
} else {
    Write-Host "Extension installation failed." -ForegroundColor Red
    exit 1
}
