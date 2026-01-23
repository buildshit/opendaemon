#!/usr/bin/env python3
"""Test script for OpenDaemon MCP Server"""

import json
import subprocess
import sys
import time

def test_mcp_server():
    print("Testing OpenDaemon MCP Server...")
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
        print("✗ dmn.exe not found. Please build the project first.")
        return False
    
    # Give it a moment to start
    time.sleep(0.5)
    
    if process.poll() is not None:
        print("✗ MCP server exited unexpectedly")
        stderr = process.stderr.read()
        print(f"Error: {stderr}")
        return False
    
    print("✓ MCP server started")
    print()
    
    # Test 1: List tools
    print("Test 1: Listing available tools...")
    request = {
        "jsonrpc": "2.0",
        "id": 1,
        "method": "tools/list",
        "params": {}
    }
    
    try:
        process.stdin.write(json.dumps(request) + "\n")
        process.stdin.flush()
        
        # Read response
        response_line = process.stdout.readline()
        if response_line:
            response = json.loads(response_line)
            print("✓ Received response")
            print(f"Tools available: {len(response.get('result', {}).get('tools', []))}")
            for tool in response.get('result', {}).get('tools', []):
                print(f"  - {tool['name']}: {tool['description']}")
        else:
            print("✗ No response received")
            return False
    except Exception as e:
        print(f"✗ Error: {e}")
        return False
    
    print()
    
    # Test 2: List services
    print("Test 2: Listing services...")
    request = {
        "jsonrpc": "2.0",
        "id": 2,
        "method": "tools/call",
        "params": {
            "name": "list_services",
            "arguments": {}
        }
    }
    
    try:
        process.stdin.write(json.dumps(request) + "\n")
        process.stdin.flush()
        
        # Read response
        response_line = process.stdout.readline()
        if response_line:
            response = json.loads(response_line)
            print("✓ Received response")
            result = response.get('result', {})
            if 'content' in result:
                services_text = result['content'][0].get('text', '')
                services = services_text.split('\n') if services_text else []
                print(f"Services found: {len(services)}")
                for service in services:
                    if service:
                        print(f"  - {service}")
            else:
                print(f"Response: {json.dumps(response, indent=2)}")
        else:
            print("✗ No response received")
            return False
    except Exception as e:
        print(f"✗ Error: {e}")
        return False
    
    print()
    
    # Test 3: Get service status
    print("Test 3: Getting service status...")
    request = {
        "jsonrpc": "2.0",
        "id": 3,
        "method": "tools/call",
        "params": {
            "name": "get_service_status",
            "arguments": {}
        }
    }
    
    try:
        process.stdin.write(json.dumps(request) + "\n")
        process.stdin.flush()
        
        # Read response
        response_line = process.stdout.readline()
        if response_line:
            response = json.loads(response_line)
            print("✓ Received response")
            result = response.get('result', {})
            if 'content' in result:
                status_text = result['content'][0].get('text', '')
                statuses = status_text.split('\n') if status_text else []
                print(f"Service statuses:")
                for status in statuses:
                    if status:
                        print(f"  {status}")
            else:
                print(f"Response: {json.dumps(response, indent=2)}")
        else:
            print("✗ No response received")
            return False
    except Exception as e:
        print(f"✗ Error: {e}")
        return False
    
    print()
    
    # Cleanup
    print("Cleaning up...")
    process.terminate()
    try:
        process.wait(timeout=2)
    except subprocess.TimeoutExpired:
        process.kill()
    
    print("✓ All tests passed!")
    return True

if __name__ == "__main__":
    success = test_mcp_server()
    sys.exit(0 if success else 1)
