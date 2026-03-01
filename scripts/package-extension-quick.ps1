# Quick package script for testing (PowerShell)

$ErrorActionPreference = "Stop"

Write-Host "Quick packaging OpenDaemon VS Code extension..." -ForegroundColor Green

# Ensure binary exists and is up-to-date with core sources
$binaryPath = "extension/bin/dmn-win32-x64.exe"
$shouldBuildBinary = -not (Test-Path $binaryPath)

if (-not $shouldBuildBinary) {
    $binaryTimestamp = (Get-Item $binaryPath).LastWriteTimeUtc
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
    New-Item -ItemType Directory -Force -Path extension/bin | Out-Null
    Copy-Item dist/dmn-win32-x64.exe extension/bin/ -Force
} else {
    Write-Host "Using existing daemon binary (up-to-date)." -ForegroundColor DarkGray
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
