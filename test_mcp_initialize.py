#!/usr/bin/env python3
"""Test MCP server initialization handshake"""

import json
import subprocess
import sys
import time

def test_mcp_initialization():
    print("Testing MCP Server Initialization...")
    print()
    
    # Start the MCP server
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
    
    time.sleep(0.5)
    
    if process.poll() is not None:
        print("✗ MCP server failed to start")
        return False
    
    print("✓ MCP server started")
    print()
    
    # Test 1: Initialize
    print("Test 1: MCP Initialize handshake")
    print("-" * 40)
    
    init_request = {
        "jsonrpc": "2.0",
        "id": 1,
        "method": "initialize",
        "params": {
            "protocolVersion": "2024-11-05",
            "capabilities": {},
            "clientInfo": {
                "name": "test-client",
                "version": "1.0.0"
            }
        }
    }
    
    try:
        process.stdin.write(json.dumps(init_request) + "\n")
        process.stdin.flush()
        
        response_line = process.stdout.readline()
        if response_line:
            response = json.loads(response_line)
            print("✓ Initialize response received")
            print(f"Protocol Version: {response.get('result', {}).get('protocolVersion')}")
            print(f"Server Name: {response.get('result', {}).get('serverInfo', {}).get('name')}")
            print(f"Server Version: {response.get('result', {}).get('serverInfo', {}).get('version')}")
            print()
        else:
            print("✗ No initialize response")
            return False
    except Exception as e:
        print(f"✗ Initialize error: {e}")
        return False
    
    # Test 2: Initialized notification
    print("Test 2: Send initialized notification")
    print("-" * 40)
    
    initialized_request = {
        "jsonrpc": "2.0",
        "method": "notifications/initialized",
        "params": {}
    }
    
    try:
        process.stdin.write(json.dumps(initialized_request) + "\n")
        process.stdin.flush()
        print("✓ Initialized notification sent")
        print()
    except Exception as e:
        print(f"✗ Initialized notification error: {e}")
        return False
    
    # Test 3: List tools (should work after initialization)
    print("Test 3: List tools after initialization")
    print("-" * 40)
    
    tools_request = {
        "jsonrpc": "2.0",
        "id": 2,
        "method": "tools/list",
        "params": {}
    }
    
    try:
        process.stdin.write(json.dumps(tools_request) + "\n")
        process.stdin.flush()
        
        response_line = process.stdout.readline()
        if response_line:
            response = json.loads(response_line)
            tools = response.get('result', {}).get('tools', [])
            print(f"✓ Found {len(tools)} tools:")
            for tool in tools:
                print(f"  • {tool['name']}")
            print()
        else:
            print("✗ No tools response")
            return False
    except Exception as e:
        print(f"✗ Tools error: {e}")
        return False
    
    # Cleanup
    process.terminate()
    try:
        process.wait(timeout=2)
    except subprocess.TimeoutExpired:
        process.kill()
    
    print("✓ MCP initialization test completed successfully!")
    return True

if __name__ == "__main__":
    success = test_mcp_initialization()
    sys.exit(0 if success else 1)