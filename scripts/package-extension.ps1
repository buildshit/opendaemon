# Package VS Code extension with bundled binaries (PowerShell)

param(
    [switch]$BuildAll
)

$ErrorActionPreference = "Stop"

Write-Host "Packaging OpenDaemon VS Code extension..." -ForegroundColor Green

if ($BuildAll) {
    Write-Host "Step 1/9: Building Rust binaries for all platforms..." -ForegroundColor Cyan
    & .\scripts\build-all.ps1
    if ($LASTEXITCODE -ne 0) {
        throw "Rust build failed"
    }
} else {
    Write-Host "Step 1/9: Skipping build-all (using existing dist binaries)." -ForegroundColor Cyan
}

# Step 2: Verify dist binaries exist
Write-Host "Step 2/9: Verifying dist binaries..." -ForegroundColor Cyan
$requiredDistBinaries = @(
    "dist/dmn-win32-x64.exe",
    "dist/dmn-linux-x64",
    "dist/dmn-linux-arm64",
    "dist/dmn-darwin-x64",
    "dist/dmn-darwin-arm64"
)

$missingDistBinaries = @()
foreach ($binary in $requiredDistBinaries) {
    if (-not (Test-Path $binary)) {
        $missingDistBinaries += $binary
        Write-Host "  [MISSING] Missing: $binary" -ForegroundColor Red
    } else {
        $size = [math]::Round((Get-Item $binary).Length / 1MB, 2)
        Write-Host "  [OK] Found: $binary ($size MB)" -ForegroundColor Green
    }
}

if ($missingDistBinaries.Count -gt 0) {
    throw @"
Missing dist binaries: $($missingDistBinaries -join ', ')
Build all binaries in CI first, or run:
  .\scripts\package-extension.ps1 -BuildAll
"@
}

# Step 3: Bundle binaries with extension
Write-Host "Step 3/9: Bundling binaries with extension..." -ForegroundColor Cyan
& .\scripts\bundle-extension.ps1
if ($LASTEXITCODE -ne 0) {
    throw "Binary bundling failed"
}

# Step 4: Verify bundled binaries and wrappers
Write-Host "Step 4/9: Verifying bundled binaries..." -ForegroundColor Cyan
$requiredBundledFiles = @(
    "extension/bin/dmn-win32-x64.exe",
    "extension/bin/dmn.exe",
    "extension/bin/dmn.cmd",
    "extension/bin/dmn-linux-x64",
    "extension/bin/dmn-linux-arm64",
    "extension/bin/dmn-darwin-x64",
    "extension/bin/dmn-darwin-arm64",
    "extension/bin/dmn"
)

$missingBundledFiles = @()
foreach ($file in $requiredBundledFiles) {
    if (-not (Test-Path $file)) {
        $missingBundledFiles += $file
        Write-Host "  [MISSING] Missing: $file" -ForegroundColor Red
    } else {
        $size = [math]::Round((Get-Item $file).Length / 1MB, 2)
        Write-Host "  [OK] Found: $file ($size MB)" -ForegroundColor Green
    }
}

if ($missingBundledFiles.Count -gt 0) {
    throw "Missing bundled files: $($missingBundledFiles -join ', ')"
}

# Step 5: Install extension dependencies
Write-Host "Step 5/9: Installing extension dependencies..." -ForegroundColor Cyan
Push-Location extension
if (-not (Test-Path "node_modules")) {
    npm install
    if ($LASTEXITCODE -ne 0) {
        Pop-Location
        throw "npm install failed"
    }
}

# Step 6: Compile TypeScript
Write-Host "Step 6/9: Compiling TypeScript..." -ForegroundColor Cyan
npm run compile
if ($LASTEXITCODE -ne 0) {
    Pop-Location
    throw "TypeScript compilation failed"
}

if (-not (Test-Path "out/extension.js")) {
    Pop-Location
    throw "TypeScript compilation did not produce expected output"
}
Write-Host "  [OK] TypeScript compiled successfully" -ForegroundColor Green

# Step 7: Verify package.json
Write-Host "Step 7/9: Verifying package.json..." -ForegroundColor Cyan
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
if (-not $packageJson.license) {
    Pop-Location
    throw "package.json missing 'license' field"
}
Write-Host "  [OK] Package: $($packageJson.name) v$($packageJson.version)" -ForegroundColor Green
Write-Host "  [OK] Publisher: $($packageJson.publisher)" -ForegroundColor Green
Write-Host "  [OK] License: $($packageJson.license)" -ForegroundColor Green

# Step 8: Package extension with vsce
Write-Host "Step 8/9: Packaging extension with vsce..." -ForegroundColor Cyan
npx @vscode/vsce package --out ../dist/ --no-dependencies
if ($LASTEXITCODE -ne 0) {
    Pop-Location
    throw "Extension packaging failed"
}

Pop-Location

# Step 9: Verify package and run package tests
Write-Host "Step 9/9: Verifying package..." -ForegroundColor Cyan
$vsixFile = Get-ChildItem dist/*.vsix | Sort-Object LastWriteTime -Descending | Select-Object -First 1
if (-not $vsixFile) {
    throw "VSIX file was not created"
}

$packageSize = [math]::Round($vsixFile.Length / 1MB, 2)
Write-Host "  [OK] Package created: $($vsixFile.Name)" -ForegroundColor Green
Write-Host "  [OK] Size: $packageSize MB" -ForegroundColor Green

& .\scripts\test-package.ps1
if ($LASTEXITCODE -ne 0) {
    Write-Host "  [WARN] Package tests failed (non-fatal)" -ForegroundColor Yellow
}

Write-Host "`n========================================" -ForegroundColor Green
Write-Host "Extension packaged successfully!" -ForegroundColor Green
Write-Host "========================================" -ForegroundColor Green
Write-Host "`nPackage: $($vsixFile.FullName)" -ForegroundColor Cyan
Write-Host "Size: $packageSize MB" -ForegroundColor Cyan
Write-Host "`nTo install:" -ForegroundColor Yellow
Write-Host "  code --install-extension `"$($vsixFile.FullName)`" --force" -ForegroundColor White
Write-Host "`nTo publish to VS Code Marketplace:" -ForegroundColor Yellow
Write-Host "  cd extension" -ForegroundColor White
Write-Host "  npx @vscode/vsce publish" -ForegroundColor White
Write-Host "`nTo publish to Open VSX:" -ForegroundColor Yellow
Write-Host "  cd extension" -ForegroundColor White
Write-Host "  npx ovsx publish ../dist/$($vsixFile.Name) -p <OVSX_PAT>" -ForegroundColor White
