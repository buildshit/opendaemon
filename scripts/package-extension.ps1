# Package VS Code extension with bundled binaries (PowerShell)

$ErrorActionPreference = "Stop"

Write-Host "Packaging OpenDaemon VS Code extension..." -ForegroundColor Green

# Step 1: Build Rust binaries for all platforms
Write-Host "Step 1: Building Rust binaries for all platforms..." -ForegroundColor Cyan
& .\scripts\build-all.ps1
if ($LASTEXITCODE -ne 0) {
    throw "Rust build failed"
}

# Step 2: Bundle binaries with extension
Write-Host "Step 2: Bundling binaries with extension..." -ForegroundColor Cyan
& .\scripts\bundle-extension.ps1
if ($LASTEXITCODE -ne 0) {
    throw "Binary bundling failed"
}

# Step 3: Verify binaries exist
Write-Host "Step 3: Verifying bundled binaries..." -ForegroundColor Cyan
$requiredBinaries = @(
    "extension/bin/dmn-win32-x64.exe",
    "extension/bin/dmn-linux-x64",
    "extension/bin/dmn-darwin-x64",
    "extension/bin/dmn-darwin-arm64"
)

$missingBinaries = @()
foreach ($binary in $requiredBinaries) {
    if (-not (Test-Path $binary)) {
        $missingBinaries += $binary
        Write-Host "  ✗ Missing: $binary" -ForegroundColor Red
    } else {
        $size = [math]::Round((Get-Item $binary).Length / 1MB, 2)
        Write-Host "  ✓ Found: $binary ($size MB)" -ForegroundColor Green
    }
}

if ($missingBinaries.Count -gt 0) {
    throw "Missing binaries: $($missingBinaries -join ', ')"
}

# Step 4: Install extension dependencies
Write-Host "Step 4: Installing extension dependencies..." -ForegroundColor Cyan
Push-Location extension
if (-not (Test-Path "node_modules")) {
    npm install
    if ($LASTEXITCODE -ne 0) {
        Pop-Location
        throw "npm install failed"
    }
}

# Step 5: Compile TypeScript
Write-Host "Step 5: Compiling TypeScript..." -ForegroundColor Cyan
npm run compile
if ($LASTEXITCODE -ne 0) {
    Pop-Location
    throw "TypeScript compilation failed"
}

# Verify compiled output
if (-not (Test-Path "out/extension.js")) {
    Pop-Location
    throw "TypeScript compilation did not produce expected output"
}
Write-Host "  ✓ TypeScript compiled successfully" -ForegroundColor Green

# Step 6: Verify package.json
Write-Host "Step 6: Verifying package.json..." -ForegroundColor Cyan
$packageJson = Get-Content "package.json" | ConvertFrom-Json
if (-not $packageJson.name) {
    Pop-Location
    throw "package.json missing 'name' field"
}
if (-not $packageJson.version) {
    Pop-Location
    throw "package.json missing 'version' field"
}
if (-not $packageJson.publisher) {
    Pop-Location
    throw "package.json missing 'publisher' field"
}
Write-Host "  ✓ Package: $($packageJson.name) v$($packageJson.version)" -ForegroundColor Green
Write-Host "  ✓ Publisher: $($packageJson.publisher)" -ForegroundColor Green

# Step 7: Package extension with vsce
Write-Host "Step 7: Packaging extension with vsce..." -ForegroundColor Cyan
npx @vscode/vsce package --out ../dist/
if ($LASTEXITCODE -ne 0) {
    Pop-Location
    throw "Extension packaging failed"
}

Pop-Location

# Step 8: Verify package was created
Write-Host "Step 8: Verifying package..." -ForegroundColor Cyan
$vsixFile = Get-ChildItem dist/*.vsix | Sort-Object LastWriteTime -Descending | Select-Object -First 1

if (-not $vsixFile) {
    throw "VSIX file was not created"
}

$packageSize = [math]::Round($vsixFile.Length / 1MB, 2)
Write-Host "  ✓ Package created: $($vsixFile.Name)" -ForegroundColor Green
Write-Host "  ✓ Size: $packageSize MB" -ForegroundColor Green

# Step 9: Run package tests
Write-Host "Step 9: Running package tests..." -ForegroundColor Cyan
& .\scripts\test-package.ps1
if ($LASTEXITCODE -ne 0) {
    Write-Host "  ⚠ Package tests failed (non-fatal)" -ForegroundColor Yellow
}

Write-Host "`n========================================" -ForegroundColor Green
Write-Host "Extension packaged successfully!" -ForegroundColor Green
Write-Host "========================================" -ForegroundColor Green
Write-Host "`nPackage: $($vsixFile.FullName)" -ForegroundColor Cyan
Write-Host "Size: $packageSize MB" -ForegroundColor Cyan
Write-Host "`nTo install:" -ForegroundColor Yellow
Write-Host "  code --install-extension `"$($vsixFile.FullName)`" --force" -ForegroundColor White
Write-Host "`nTo publish:" -ForegroundColor Yellow
Write-Host "  cd extension" -ForegroundColor White
Write-Host "  npx @vscode/vsce publish" -ForegroundColor White
