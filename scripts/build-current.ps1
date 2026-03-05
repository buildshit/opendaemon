# Build script for current platform only (PowerShell)

$ErrorActionPreference = "Stop"

Write-Host "Building dmn for current platform..." -ForegroundColor Green

# Create output directory
New-Item -ItemType Directory -Force -Path dist | Out-Null

# If a stale process is running from dist/dmn-win32-x64.exe, Copy-Item will fail.
# Stop only that exact executable so local packaging stays repeatable.
$distBinaryPath = (Resolve-Path "dist/dmn-win32-x64.exe" -ErrorAction SilentlyContinue).Path
if ($distBinaryPath) {
    $lockingProcesses = Get-CimInstance Win32_Process |
        Where-Object { $_.ExecutablePath -eq $distBinaryPath }

    if ($lockingProcesses) {
        Write-Host "Stopping process(es) locking dist/dmn-win32-x64.exe..." -ForegroundColor Yellow
        $lockingProcesses | Select-Object ProcessId, Name, CommandLine | Format-Table -AutoSize
        foreach ($proc in $lockingProcesses) {
            Stop-Process -Id $proc.ProcessId -ErrorAction Stop
        }
        Start-Sleep -Milliseconds 300
    }
}

# Build for current platform
$primaryTargetDir = "target/build-current"
$fallbackTargetDir = "target/build-current-fallback-$([DateTime]::UtcNow.ToString('yyyyMMddHHmmssfff'))"
$buildTargetDir = $primaryTargetDir

function Invoke-CargoBuild([string]$targetDir) {
    Write-Host "Running cargo build (target-dir: $targetDir)..." -ForegroundColor DarkGray
    cargo build --release --package dmn-core --target-dir $targetDir
    return ($LASTEXITCODE -eq 0)
}

$buildSucceeded = Invoke-CargoBuild $buildTargetDir
if (-not $buildSucceeded) {
    Write-Host "Primary target dir build failed (often caused by locked artifacts)." -ForegroundColor Yellow
    Write-Host "Retrying with isolated fallback target dir..." -ForegroundColor Yellow
    $buildTargetDir = $fallbackTargetDir
    $buildSucceeded = Invoke-CargoBuild $buildTargetDir
}

if (-not $buildSucceeded) {
    throw @"
Cargo build failed.
If you see file lock errors, ensure no process is locking artifacts and retry.
"@
}

# Copy binary for Windows
Copy-Item "$buildTargetDir/release/dmn.exe" dist/dmn-win32-x64.exe -Force
Write-Host "Built for Windows x86_64" -ForegroundColor Cyan

Write-Host "Build complete! Binary is in the dist/ directory:" -ForegroundColor Green
Get-ChildItem dist/ | Format-Table Name, Length
