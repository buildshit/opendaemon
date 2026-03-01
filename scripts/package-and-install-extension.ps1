$ErrorActionPreference = "Stop"

$repoRoot = Resolve-Path (Join-Path $PSScriptRoot "..")

Write-Host "Running OpenDaemon extension package + install workflow..." -ForegroundColor Green
Push-Location $repoRoot

try {
    Write-Host "Step 1/2: Quick package extension" -ForegroundColor Cyan
    & "$PSScriptRoot\package-extension-quick.ps1"
    if ($LASTEXITCODE -ne 0) {
        throw "package-extension-quick.ps1 failed with exit code $LASTEXITCODE"
    }

    Write-Host "Step 2/2: Install latest VSIX into supported editors" -ForegroundColor Cyan
    & "$PSScriptRoot\install-extension.ps1"
    if ($LASTEXITCODE -ne 0) {
        throw "install-extension.ps1 failed with exit code $LASTEXITCODE"
    }

    Write-Host "Workflow complete. Reload Cursor/VS Code windows to activate updates." -ForegroundColor Yellow
} finally {
    Pop-Location
}
