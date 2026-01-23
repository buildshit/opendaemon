#!/usr/bin/env python3
"""Debug MCP tools response"""

import json
import subprocess
import sys
import time

def debug_tools_response():
    print("Debugging MCP Tools Response...")
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
    
    # Initialize first
    init_request = {
        "jsonrpc": "2.0",
        "id": 1,
        "method": "initialize",
        "params": {
            "protocolVersion": "2024-11-05",
            "capabilities": {},
            "clientInfo": {"name": "debug-client", "version": "1.0.0"}
        }
    }
    
    process.stdin.write(json.dumps(init_request) + "\n")
    process.stdin.flush()
    
    init_response = process.stdout.readline()
    print("Initialize response:")
    print(init_response)
    print()
    
    # Now request tools
    tools_request = {
        "jsonrpc": "2.0",
        "id": 2,
        "method": "tools/list",
        "params": {}
    }
    
    print("Sending tools/list request:")
    print(json.dumps(tools_request, indent=2))
    print()
    
    process.stdin.write(json.dumps(tools_request) + "\n")
    process.stdin.flush()
    
    tools_response = process.stdout.readline()
    print("Raw tools response:")
    print(repr(tools_response))
    print()
    
    if tools_response:
        try:
            parsed = json.loads(tools_response)
            print("Parsed tools response:")
            print(json.dumps(parsed, indent=2))
            print()
            
            result = parsed.get('result', {})
            tools = result.get('tools', [])
            print(f"Tools array length: {len(tools)}")
            print(f"Tools array: {tools}")
            
        except json.JSONDecodeError as e:
            print(f"JSON decode error: {e}")
    
    # Cleanup
    process.terminate()
    return True

if __name__ == "__main__":
    debug_tools_response()