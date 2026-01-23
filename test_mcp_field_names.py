#!/usr/bin/env python3
"""Test MCP server field names for Kiro compatibility"""

import json
import subprocess
import sys
import time

def test_field_names():
    print("Testing MCP Field Names for Kiro Compatibility")
    print("=" * 50)
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
    
    def send_request(request):
        """Send request and get response"""
        process.stdin.write(json.dumps(request) + "\n")
        process.stdin.flush()
        response_line = process.stdout.readline()
        return json.loads(response_line) if response_line else None
    
    # Initialize
    init_request = {
        "jsonrpc": "2.0",
        "id": 1,
        "method": "initialize",
        "params": {
            "protocolVersion": "2024-11-05",
            "capabilities": {},
            "clientInfo": {"name": "test", "version": "1.0.0"}
        }
    }
    
    init_response = send_request(init_request)
    if not init_response or 'result' not in init_response:
        print("✗ Initialization failed")
        return False
    
    print("✓ MCP server initialized")
    print()
    
    # Test tools/list with field name validation
    print("Testing tools/list response format...")
    print("-" * 40)
    
    tools_request = {
        "jsonrpc": "2.0",
        "id": 2,
        "method": "tools/list",
        "params": {}
    }
    
    tools_response = send_request(tools_request)
    if not tools_response or 'result' not in tools_response:
        print("✗ Failed to get tools")
        return False
    
    tools = tools_response['result'].get('tools', [])
    print(f"✓ Found {len(tools)} tools")
    print()
    
    # Validate each tool has correct field names
    for i, tool in enumerate(tools):
        print(f"Tool {i+1}: {tool.get('name', 'UNNAMED')}")
        print(f"  Fields: {list(tool.keys())}")
        
        # Check required fields
        required_fields = ['name', 'description', 'inputSchema']
        missing_fields = []
        
        for field in required_fields:
            if field not in tool:
                missing_fields.append(field)
        
        if missing_fields:
            print(f"  ✗ Missing fields: {missing_fields}")
            return False
        else:
            print(f"  ✓ All required fields present")
        
        # Check inputSchema is an object
        input_schema = tool.get('inputSchema')
        if not isinstance(input_schema, dict):
            print(f"  ✗ inputSchema is not an object: {type(input_schema)}")
            return False
        else:
            print(f"  ✓ inputSchema is a valid object")
        
        print()
    
    # Show the raw JSON for verification
    print("Raw tools response (formatted):")
    print("-" * 40)
    print(json.dumps(tools_response, indent=2))
    
    # Cleanup
    process.terminate()
    try:
        process.wait(timeout=2)
    except subprocess.TimeoutExpired:
        process.kill()
    
    print()
    print("✓ All field names are correct for Kiro compatibility!")
    return True

if __name__ == "__main__":
    success = test_field_names()
    sys.exit(0 if success else 1)