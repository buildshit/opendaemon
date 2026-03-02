# Test the packaged extension

$ErrorActionPreference = "Stop"

Write-Host "Testing OpenDaemon package..." -ForegroundColor Green

# Find the VSIX file
$vsixFile = Get-ChildItem dist/*.vsix | Sort-Object LastWriteTime -Descending | Select-Object -First 1

if (-not $vsixFile) {
    Write-Host "No VSIX file found. Run package-extension.ps1 first." -ForegroundColor Red
    exit 1
}

Write-Host "`nPackage: $($vsixFile.Name)" -ForegroundColor Cyan
Write-Host "Size: $([math]::Round($vsixFile.Length / 1MB, 2)) MB" -ForegroundColor Cyan

# Create temp directory
$tempDir = New-Item -ItemType Directory -Path "$env:TEMP\opendaemon-test-$(Get-Random)" -Force

try {
    # Extract VSIX (it's a ZIP file)
    Write-Host "`nExtracting package..." -ForegroundColor Yellow
    Expand-Archive -Path $vsixFile.FullName -DestinationPath $tempDir -Force
    
    # Verify binaries exist in package
    Write-Host "`nVerifying binaries in package..." -ForegroundColor Yellow
    
    $allBinaries = @(
        "dmn-win32-x64.exe",
        "dmn-linux-x64",
        "dmn-linux-arm64",
        "dmn-darwin-x64",
        "dmn-darwin-arm64",
        "dmn.exe",
        "dmn.cmd",
        "dmn"
    )
    
    $missingBinaries = @()
    foreach ($binary in $allBinaries) {
        $binaryPath = Join-Path $tempDir "extension\bin\$binary"
        
        if (Test-Path $binaryPath) {
            $binarySize = [math]::Round((Get-Item $binaryPath).Length / 1MB, 2)
            Write-Host "✓ $binary ($binarySize MB)" -ForegroundColor Green
        } else {
            Write-Host "✗ $binary (missing)" -ForegroundColor Red
            $missingBinaries += $binary
        }
    }
    
    # Test current platform binary
    Write-Host "`nTesting current platform binary..." -ForegroundColor Yellow
    $currentBinary = "dmn-win32-x64.exe"
    $binaryPath = Join-Path $tempDir "extension\bin\$currentBinary"
    
    if (Test-Path $binaryPath) {
        Write-Host "✓ Binary found in package: $currentBinary" -ForegroundColor Green
        
        # Test if binary is executable
        try {
            $version = & $binaryPath --version 2>&1
            if ($LASTEXITCODE -eq 0) {
                Write-Host "✓ Binary is executable" -ForegroundColor Green
                Write-Host "  Version: $version" -ForegroundColor Cyan
            } else {
                Write-Host "✗ Binary failed to execute" -ForegroundColor Red
            }
        } catch {
            Write-Host "✗ Binary failed to execute: $_" -ForegroundColor Red
        }
    } else {
        Write-Host "✗ Binary not found in package" -ForegroundColor Red
        Write-Host "  Expected: $binaryPath" -ForegroundColor Yellow
    }
    
    # Check for other required files
    Write-Host "`nChecking required files..." -ForegroundColor Yellow
    
    $requiredFiles = @(
        "extension\package.json",
        "extension\out\extension.js",
        "extension\out\daemon.js",
        "extension\out\rpc-client.js",
        "extension\out\tree-view.js",
        "extension\out\commands.js",
        "extension\out\logs.js",
        "extension\out\wizard.js",
        "extension\out\error-display.js"
    )
    
    $missingFiles = @()
    foreach ($file in $requiredFiles) {
        $filePath = Join-Path $tempDir $file
        if (Test-Path $filePath) {
            Write-Host "✓ $file" -ForegroundColor Green
        } else {
            Write-Host "✗ $file (missing)" -ForegroundColor Red
            $missingFiles += $file
        }
    }
    
    # Check that source files are NOT included
    Write-Host "`nVerifying source files are excluded..." -ForegroundColor Yellow
    
    $excludedFiles = @(
        "extension\src\extension.ts",
        "extension\tsconfig.json",
        "extension\node_modules"
    )
    
    $incorrectlyIncluded = @()
    foreach ($file in $excludedFiles) {
        $filePath = Join-Path $tempDir $file
        if (Test-Path $filePath) {
            Write-Host "✗ $file (should be excluded)" -ForegroundColor Red
            $incorrectlyIncluded += $file
        } else {
            Write-Host "✓ $file (correctly excluded)" -ForegroundColor Green
        }
    }
    
    # Summary
    Write-Host "`n========================================" -ForegroundColor Cyan
    Write-Host "Test Summary" -ForegroundColor Cyan
    Write-Host "========================================" -ForegroundColor Cyan
    
    $allPassed = $true
    
    if ($missingBinaries.Count -gt 0) {
        Write-Host "✗ Missing binaries: $($missingBinaries.Count)" -ForegroundColor Red
        $allPassed = $false
    } else {
        Write-Host "✓ All binaries present" -ForegroundColor Green
    }
    
    if ($missingFiles.Count -gt 0) {
        Write-Host "✗ Missing required files: $($missingFiles.Count)" -ForegroundColor Red
        $allPassed = $false
    } else {
        Write-Host "✓ All required files present" -ForegroundColor Green
    }
    
    if ($incorrectlyIncluded.Count -gt 0) {
        Write-Host "✗ Files that should be excluded: $($incorrectlyIncluded.Count)" -ForegroundColor Red
        $allPassed = $false
    } else {
        Write-Host "✓ Source files correctly excluded" -ForegroundColor Green
    }
    
    if ($allPassed) {
        Write-Host "`n✓ All tests passed!" -ForegroundColor Green
    } else {
        Write-Host "`n✗ Some tests failed" -ForegroundColor Red
    }
    
} finally {
    # Cleanup
    Remove-Item -Path $tempDir -Recurse -Force -ErrorAction SilentlyContinue
}

Write-Host "`nPackage test complete!" -ForegroundColor Green
Write-Host "`nTo install:" -ForegroundColor Yellow
Write-Host "  code --install-extension `"$($vsixFile.FullName)`" --force" -ForegroundColor White

if ($allPassed) {
    exit 0
} else {
    exit 1
}
