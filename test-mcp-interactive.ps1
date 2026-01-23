# Interactive MCP Server Test
Write-Host "Testing OpenDaemon MCP Server with actual requests..." -ForegroundColor Cyan
Write-Host ""

# Start the MCP server as a background job
Write-Host "Starting MCP server..." -ForegroundColor Yellow
$mcpProcess = Start-Process -FilePath "target\release\dmn.exe" -ArgumentList "mcp" -NoNewWindow -PassThru -RedirectStandardInput "mcp-input.txt" -RedirectStandardOutput "mcp-output.txt" -RedirectStandardError "mcp-error.txt"

# Wait for server to initialize
Start-Sleep -Seconds 2

if ($mcpProcess.HasExited) {
    Write-Host "✗ MCP server exited unexpectedly" -ForegroundColor Red
    Write-Host ""
    Write-Host "Error output:" -ForegroundColor Red
    Get-Content "mcp-error.txt" | ForEach-Object { Write-Host $_ -ForegroundColor Red }
    exit 1
}

Write-Host "✓ MCP server is running (PID: $($mcpProcess.Id))" -ForegroundColor Green
Write-Host ""

# Test 1: List tools
Write-Host "Test 1: Listing available tools..." -ForegroundColor Yellow
$listToolsRequest = @{
    jsonrpc = "2.0"
    id = 1
    method = "tools/list"
    params = @{}
} | ConvertTo-Json -Compress

$listToolsRequest | Out-File -FilePath "mcp-input.txt" -Encoding utf8
Start-Sleep -Seconds 1

if (Test-Path "mcp-output.txt") {
    $output = Get-Content "mcp-output.txt" -Raw
    if ($output) {
        Write-Host "✓ Received response" -ForegroundColor Green
        Write-Host "Response:" -ForegroundColor Cyan
        $output | ConvertFrom-Json | ConvertTo-Json -Depth 10 | Write-Host -ForegroundColor Gray
    } else {
        Write-Host "✗ No response received" -ForegroundColor Red
    }
} else {
    Write-Host "✗ No output file created" -ForegroundColor Red
}

Write-Host ""

# Test 2: List services
Write-Host "Test 2: Listing services..." -ForegroundColor Yellow
$listServicesRequest = @{
    jsonrpc = "2.0"
    id = 2
    method = "tools/call"
    params = @{
        name = "list_services"
        arguments = @{}
    }
} | ConvertTo-Json -Compress

# Clear previous output
Remove-Item "mcp-output.txt" -ErrorAction SilentlyContinue
$listServicesRequest | Out-File -FilePath "mcp-input.txt" -Encoding utf8 -Append
Start-Sleep -Seconds 1

if (Test-Path "mcp-output.txt") {
    $output = Get-Content "mcp-output.txt" -Raw
    if ($output) {
        Write-Host "✓ Received response" -ForegroundColor Green
        Write-Host "Response:" -ForegroundColor Cyan
        $output | ConvertFrom-Json | ConvertTo-Json -Depth 10 | Write-Host -ForegroundColor Gray
    } else {
        Write-Host "✗ No response received" -ForegroundColor Red
    }
}

Write-Host ""

# Cleanup
Write-Host "Cleaning up..." -ForegroundColor Yellow
Stop-Process -Id $mcpProcess.Id -Force -ErrorAction SilentlyContinue
Remove-Item "mcp-input.txt" -ErrorAction SilentlyContinue
Remove-Item "mcp-output.txt" -ErrorAction SilentlyContinue
Remove-Item "mcp-error.txt" -ErrorAction SilentlyContinue

Write-Host "✓ Test completed" -ForegroundColor Green
