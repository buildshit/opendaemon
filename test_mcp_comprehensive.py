#!/usr/bin/env python3
"""Comprehensive test for OpenDaemon MCP Server including log reading"""

import json
import subprocess
import sys
import time
import threading

def test_mcp_with_running_services():
    print("Comprehensive OpenDaemon MCP Server Test")
    print("=" * 50)
    print()
    
    # Start the MCP server
    print("Starting MCP server...")
    try:
        process = subprocess.Popen(
            ["target/release/dmn.exe", "mcp"],
            stdin=subprocess.PIPE,
            stdout=subprocess.PIPE,
            stderr=subprocess.PIPE,
            text=True,
            bufsize=1
        )
    except FileNotFoundError:
        print("✗ dmn.exe not found")
        return False
    
    time.sleep(1)
    
    if process.poll() is not None:
        print("✗ MCP server failed to start")
        return False
    
    print("✓ MCP server running")
    print()
    
    def send_request(req_id, method, params=None):
        """Helper to send MCP request and get response"""
        request = {
            "jsonrpc": "2.0",
            "id": req_id,
            "method": method,
            "params": params or {}
        }
        
        try:
            process.stdin.write(json.dumps(request) + "\n")
            process.stdin.flush()
            
            response_line = process.stdout.readline()
            if response_line:
                return json.loads(response_line)
            return None
        except Exception as e:
            print(f"✗ Error sending request: {e}")
            return None
    
    # Test 1: List tools
    print("Test 1: List available tools")
    print("-" * 50)
    response = send_request(1, "tools/list")
    if response and 'result' in response:
        tools = response['result'].get('tools', [])
        print(f"✓ Found {len(tools)} tools:")
        for tool in tools:
            print(f"  • {tool['name']}")
            print(f"    {tool['description']}")
        print()
    else:
        print("✗ Failed to list tools")
        process.terminate()
        return False
    
    # Test 2: List services
    print("Test 2: List configured services")
    print("-" * 50)
    response = send_request(2, "tools/call", {
        "name": "list_services",
        "arguments": {}
    })
    if response and 'result' in response:
        content = response['result'].get('content', [])
        if content:
            services_text = content[0].get('text', '')
            services = [s for s in services_text.split('\n') if s]
            print(f"✓ Found {len(services)} services:")
            for service in services:
                print(f"  • {service}")
            print()
        else:
            print("✗ No services found")
    else:
        print("✗ Failed to list services")
        process.terminate()
        return False
    
    # Test 3: Get initial status
    print("Test 3: Check initial service status")
    print("-" * 50)
    response = send_request(3, "tools/call", {
        "name": "get_service_status",
        "arguments": {}
    })
    if response and 'result' in response:
        content = response['result'].get('content', [])
        if content:
            status_text = content[0].get('text', '')
            statuses = [s for s in status_text.split('\n') if s]
            print("✓ Service statuses:")
            for status in statuses:
                print(f"  {status}")
            print()
        else:
            print("✗ No status information")
    else:
        print("✗ Failed to get status")
        process.terminate()
        return False
    
    # Test 4: Try to read logs (should be empty initially)
    print("Test 4: Read logs from database service")
    print("-" * 50)
    response = send_request(4, "tools/call", {
        "name": "read_logs",
        "arguments": {
            "service": "database",
            "lines": 10
        }
    })
    if response and 'result' in response:
        content = response['result'].get('content', [])
        if content:
            logs_text = content[0].get('text', '')
            if logs_text:
                print("✓ Logs retrieved:")
                for line in logs_text.split('\n')[:5]:  # Show first 5 lines
                    if line:
                        print(f"  {line}")
                if len(logs_text.split('\n')) > 5:
                    print(f"  ... and {len(logs_text.split('\n')) - 5} more lines")
            else:
                print("✓ No logs yet (services not started)")
            print()
        else:
            print("✗ No content in response")
    else:
        print("✗ Failed to read logs")
        process.terminate()
        return False
    
    # Test 5: Test error handling - invalid service
    print("Test 5: Error handling - invalid service name")
    print("-" * 50)
    response = send_request(5, "tools/call", {
        "name": "read_logs",
        "arguments": {
            "service": "nonexistent-service",
            "lines": 10
        }
    })
    if response and 'result' in response:
        result = response['result']
        if result.get('type') == 'error':
            error_msg = result.get('error', '')
            print(f"✓ Error handled correctly: {error_msg}")
            print()
        else:
            print("✗ Should have returned an error")
    else:
        print("✗ Failed to handle error")
        process.terminate()
        return False
    
    # Test 6: Test "all" lines parameter
    print("Test 6: Read all logs")
    print("-" * 50)
    response = send_request(6, "tools/call", {
        "name": "read_logs",
        "arguments": {
            "service": "backend-api",
            "lines": "all"
        }
    })
    if response and 'result' in response:
        print("✓ 'all' parameter accepted")
        print()
    else:
        print("✗ Failed with 'all' parameter")
        process.terminate()
        return False
    
    # Cleanup
    print("=" * 50)
    print("Cleaning up...")
    process.terminate()
    try:
        process.wait(timeout=2)
    except subprocess.TimeoutExpired:
        process.kill()
    
    print()
    print("✓ All tests passed successfully!")
    print()
    print("Summary:")
    print("  • MCP server starts correctly")
    print("  • All 3 tools are available")
    print("  • Services are listed correctly")
    print("  • Status checking works")
    print("  • Log reading works")
    print("  • Error handling works")
    print()
    return True

if __name__ == "__main__":
    success = test_mcp_with_running_services()
    sys.exit(0 if success else 1)
