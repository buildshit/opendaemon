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

# Install into all detected VS Code-family editors so behavior is consistent across forks.
$editors = @("code", "cursor", "antigravity", "kiro")
$installedTargets = @()
$failedTargets = @()

foreach ($editor in $editors) {
    $cmd = Get-Command $editor -ErrorAction SilentlyContinue
    if (-not $cmd) {
        continue
    }

    Write-Host "Installing via '$editor'..." -ForegroundColor Cyan
    & $editor --install-extension $vsixFile.FullName --force

    if ($LASTEXITCODE -eq 0) {
        $installedTargets += $editor
        Write-Host "Installed successfully in $editor." -ForegroundColor Green
    } else {
        $failedTargets += $editor
        Write-Host "Install failed in $editor." -ForegroundColor Red
    }
}

if ($installedTargets.Count -eq 0) {
    Write-Host "No supported editor CLI found (expected one of: $($editors -join ', '))." -ForegroundColor Red
    exit 1
}

if ($failedTargets.Count -gt 0) {
    Write-Host "Installed in: $($installedTargets -join ', ')" -ForegroundColor Yellow
    Write-Host "Failed in: $($failedTargets -join ', ')" -ForegroundColor Red
    exit 1
}

Write-Host "Extension installed successfully in: $($installedTargets -join ', ')" -ForegroundColor Green
Write-Host "Restart each editor window to activate the updated extension." -ForegroundColor Yellow
