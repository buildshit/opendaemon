# Test MCP Server Functionality
Write-Host "Testing OpenDaemon MCP Server..." -ForegroundColor Cyan
Write-Host ""

# Test 1: Check if binary exists
Write-Host "Test 1: Checking binary..." -ForegroundColor Yellow
if (Test-Path "target\release\dmn.exe") {
    Write-Host "✓ Binary found at target\release\dmn.exe" -ForegroundColor Green
} else {
    Write-Host "✗ Binary not found" -ForegroundColor Red
    exit 1
}

# Test 2: Check if dmn.json exists
Write-Host ""
Write-Host "Test 2: Checking configuration..." -ForegroundColor Yellow
if (Test-Path "dmn.json") {
    Write-Host "✓ Configuration found at dmn.json" -ForegroundColor Green
    $config = Get-Content "dmn.json" | ConvertFrom-Json
    $serviceCount = ($config.services | Get-Member -MemberType NoteProperty).Count
    Write-Host "  Services configured: $serviceCount" -ForegroundColor Gray
    foreach ($service in $config.services.PSObject.Properties) {
        Write-Host "    - $($service.Name)" -ForegroundColor Gray
    }
} else {
    Write-Host "✗ Configuration not found" -ForegroundColor Red
    exit 1
}

# Test 3: Test MCP server initialization
Write-Host ""
Write-Host "Test 3: Testing MCP server initialization..." -ForegroundColor Yellow
Write-Host "  Starting MCP server (will run for 5 seconds)..." -ForegroundColor Gray

$job = Start-Job -ScriptBlock {
    Set-Location $using:PWD
    & "target\release\dmn.exe" mcp 2>&1
}

# Wait a bit for startup
Start-Sleep -Seconds 2

# Check if job is still running
if ($job.State -eq "Running") {
    Write-Host "✓ MCP server started successfully" -ForegroundColor Green
    
    # Let it run a bit more to see output
    Start-Sleep -Seconds 3
    
    # Get output
    $output = Receive-Job -Job $job
    
    Write-Host ""
    Write-Host "Server Output:" -ForegroundColor Cyan
    Write-Host "----------------------------------------"
    $output | ForEach-Object { Write-Host $_ -ForegroundColor Gray }
    Write-Host "----------------------------------------"
    
    # Stop the job
    Stop-Job -Job $job
    Remove-Job -Job $job
    
    Write-Host ""
    Write-Host "✓ MCP server test completed" -ForegroundColor Green
} else {
    Write-Host "✗ MCP server failed to start" -ForegroundColor Red
    $output = Receive-Job -Job $job
    Write-Host "Error output:" -ForegroundColor Red
    $output | ForEach-Object { Write-Host $_ -ForegroundColor Red }
    Remove-Job -Job $job
    exit 1
}

Write-Host ""
Write-Host "All tests passed! ✓" -ForegroundColor Green
Write-Host ""
Write-Host "To manually test the MCP server, run:" -ForegroundColor Cyan
Write-Host "  target\release\dmn.exe mcp" -ForegroundColor White
