# Quick package script for testing (PowerShell)

$ErrorActionPreference = "Stop"

Write-Host "Quick packaging OpenDaemon VS Code extension..." -ForegroundColor Green

# Ensure dist binary exists and is up-to-date with core sources.
# We always run bundle-extension.ps1 afterwards so dmn.exe/cmd wrappers stay in sync.
$distBinaryPath = "dist/dmn-win32-x64.exe"
$shouldBuildBinary = -not (Test-Path $distBinaryPath)

if (-not $shouldBuildBinary) {
    $binaryTimestamp = (Get-Item $distBinaryPath).LastWriteTimeUtc
    $coreInputs = @("core/src", "core/Cargo.toml", "Cargo.toml", "Cargo.lock")

    foreach ($inputPath in $coreInputs) {
        if (-not (Test-Path $inputPath)) {
            continue
        }

        $item = Get-Item $inputPath
        if ($item.PSIsContainer) {
            $newerSource = Get-ChildItem $inputPath -Recurse -File |
                Where-Object { $_.LastWriteTimeUtc -gt $binaryTimestamp } |
                Select-Object -First 1
            if ($newerSource) {
                $shouldBuildBinary = $true
                break
            }
        } elseif ($item.LastWriteTimeUtc -gt $binaryTimestamp) {
            $shouldBuildBinary = $true
            break
        }
    }
}

if ($shouldBuildBinary) {
    Write-Host "Daemon binary missing or stale, building..." -ForegroundColor Yellow
    & .\scripts\build-current.ps1
    if (-not $?) {
        throw "Binary build failed"
    }
} else {
    Write-Host "Using existing dist daemon binary (up-to-date)." -ForegroundColor DarkGray
}

Write-Host "Bundling binaries and wrappers into extension/bin..." -ForegroundColor Cyan
& .\scripts\bundle-extension.ps1
if (-not $?) {
    throw "Binary bundling failed"
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
