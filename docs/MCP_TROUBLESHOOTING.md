# MCP Troubleshooting Guide

Common issues and solutions when setting up MCP integration with OpenDaemon.

## Quick Diagnostics

Run these commands to check your setup:

```bash
# 1. Check OpenDaemon is installed
dmn --version

# 2. Check dmn.json exists
ls dmn.json

# 3. Test MCP server starts
dmn mcp --config /absolute/path/to/dmn.json
# Should show: "Starting MCP server mode with config: "/absolute/path/to/dmn.json""
# Press Ctrl+C to stop
```

## Common Issues

### 1. "Command not found: dmn"

**Problem:** OpenDaemon binary not in PATH or not installed.

**Solutions:**

**Option A: Use full path**
```json
{
  "mcpServers": {
    "opendaemon": {
      "command": "/full/path/to/dmn",
      "args": ["mcp", "--config", "/full/path/to/dmn.json"]
    }
  }
}
```

**Option B: Add to PATH**
```bash
# Add the directory containing dmn to your PATH
export PATH="/path/to/opendaemon:$PATH"
```

**Option C: Check installation**
- Make sure OpenDaemon is properly installed
- Verify the binary exists where you expect it

### 2. "Failed to load dmn.json"

**Problem:** MCP server can't find or read your configuration file.

**Solutions:**

1. **Check file exists:**
   ```bash
   ls dmn.json
   ```

2. **Use explicit config path:**
   - Always pass `--config` with an absolute path to `dmn.json`
   - This avoids IDE working-directory differences

3. **Check file syntax:**
   ```bash
   # Validate JSON syntax
   cat dmn.json | python -m json.tool
   ```

4. **Minimum valid dmn.json:**
   ```json
   {
     "version": "1.0",
     "services": {
       "test": {
         "command": "echo hello"
       }
     }
   }
   ```

### 3. "MCP server not available" or "Connection failed"

**Problem:** AI assistant can't connect to the MCP server.

**Solutions:**

1. **Test server manually:**
   ```bash
   dmn mcp --config /absolute/path/to/dmn.json
   # Should start without errors
   ```

2. **Use explicit `--config`:**
   - Do not rely on current working directory
   - Pass an absolute `dmn.json` path in your MCP args

3. **Restart AI assistant:**
   - Kiro: Reload VS Code window
   - Cursor: Restart application
   - Claude Desktop: Restart application

4. **Check configuration syntax:**
   ```json
   {
     "mcpServers": {
       "opendaemon": {
         "command": "dmn",
         "args": ["mcp", "--config", "/absolute/path/to/dmn.json"],
         "disabled": false
       }
     }
   }
   ```

5. **Verify you are using a current binary build:**
   ```bash
   /path/to/dmn mcp --help
   ```
   - Recent builds show a `--check` option.
   - If your binary does not, update the binary path in your MCP config (for local source builds, prefer the newest build output).
   - If `target/build-current/release/dmn.exe` is locked and cannot be rebuilt, point MCP to `dist/dmn-win32-x64.exe` or close the locking IDE process and rebuild.

6. **Ensure MCP and extension daemon target the same `dmn.json`:**
   - MCP now reuses the active extension daemon when config paths match.
   - If MCP output and extension UI status disagree, confirm the MCP `--config` path is the same file used by the extension workspace.

### 4. "No services found" or Empty Results

**Problem:** MCP server starts but returns no services.

**Solutions:**

1. **Check dmn.json has services:**
   ```json
   {
     "version": "1.0",
     "services": {
       "my-service": {
         "command": "npm start"
       }
     }
   }
   ```

2. **Verify services section is not empty:**
   ```bash
   # Should show your services
   cat dmn.json | grep -A 10 services
   ```

3. **Test with minimal config:**
   ```json
   {
     "version": "1.0",
     "services": {
       "test": {
         "command": "echo test"
       }
     }
   }
   ```

### 5. "Method not found: initialize"

**Problem:** MCP server doesn't support proper MCP protocol.

**Solutions:**

1. **Update OpenDaemon:**
   - Make sure you have the latest version
   - The MCP server was added in recent versions

2. **Rebuild if using source:**
   ```bash
   cargo build --release
   ```

3. **Check server response:**
   ```bash
   # Test with a simple MCP request
   echo '{"jsonrpc":"2.0","id":1,"method":"initialize","params":{}}' | dmn mcp --config /absolute/path/to/dmn.json
   ```

### 6. "Invalid input: expected object, received undefined"

**Problem:** Tool schema validation errors.

**Solutions:**

1. **Update to latest version:**
   - This was fixed in recent versions
   - Make sure you have the latest OpenDaemon build

2. **Check field names:**
   - Older versions had incorrect field naming
   - Should be `inputSchema` not `input_schema`

### 7. AI Asks Permission for Every Tool Call

**Problem:** No tools are auto-approved.

**Solution:** Add safe tools to auto-approve list:

```json
{
  "mcpServers": {
    "opendaemon": {
      "command": "dmn",
      "args": ["mcp", "--config", "/absolute/path/to/dmn.json"],
      "autoApprove": [
        "list_services",
        "get_service_status"
      ]
    }
  }
}
```

**Safe to auto-approve:**
- `list_services` - Only lists service names
- `get_service_status` - Only shows service status
- `watch_logs` - Read-only live log watching/filtering

**Requires permission:**
- `read_logs` - Reads actual log content
- `start_service`, `stop_service`, `restart_service` - Changes service runtime state

### 8. "Service 'X' not found"

**Problem:** Trying to read logs from a service that doesn't exist.

**Solutions:**

1. **Check service names:**
   ```bash
   # List actual service names
   cat dmn.json | grep -A 1 '".*":'
   ```

2. **Use exact names:**
   - Service names are case-sensitive
   - Must match exactly what's in dmn.json

3. **Ask AI to list services first:**
   ```
   "What services are configured?"
   ```

### 9. Need Detailed MCP Tool Call Tracing

**Problem:** You need to inspect MCP request/tool-call flow end-to-end.

**Solutions:**

1. **Server-side runtime trace:**
   - Start MCP manually with stderr visible:
     ```bash
     dmn mcp --config /absolute/path/to/dmn.json 2> mcp-debug.log
     ```
   - Recent builds emit structured stderr events for:
     - request dispatch (`initialize`, `tools/list`, `tools/call`)
     - tool execution start/finish, argument summaries, elapsed time
     - invalid params / unknown method errors

2. **Correlate with client logs:**
   - Use your IDE/client MCP output panel for startup errors
   - Use MCP server stderr for request/tool-call execution traces

### 10. `start_service` says "Start requested" but nothing changes

**Problem:** MCP reports a successful start request, but service state appears unchanged.

**What this usually means:**
- The service is already running, so status remains `running`
- You are reading old log lines (no new startup event yet)
- MCP and extension daemon are pointed at different `dmn.json` files

**Checks:**

1. **Confirm current state first:**
   - Run `get_service_status`
   - If `frontend` is already `running`, `start_service` is effectively idempotent

2. **Force a visible transition:**
   - Call `restart_service` for the same service
   - Verify status moves through `starting` to `running`

3. **Verify fresh logs after restart:**
   - Call `read_logs(service: "frontend", lines: 20)`
   - Confirm recent timestamps and startup lines

4. **Confirm config-path alignment:**
   - Ensure MCP args include `--config /absolute/path/to/dmn.json`
   - Ensure this is the same workspace config used by the extension daemon

## Platform-Specific Issues

### Windows

1. **Path separators:**
   ```json
   {
     "command": "C:\\path\\to\\dmn.exe"
   }
   ```

2. **PowerShell execution policy:**
   ```powershell
   Set-ExecutionPolicy -ExecutionPolicy RemoteSigned -Scope CurrentUser
   ```

### macOS/Linux

1. **Permissions:**
   ```bash
   chmod +x /path/to/dmn
   ```

2. **Shell environment:**
   - Make sure PATH is set correctly
   - Check shell profile (.bashrc, .zshrc)

## Testing Your Setup

### Manual MCP Test

Create a test script to verify MCP communication:

```python
#!/usr/bin/env python3
import json
import subprocess
import sys

def test_mcp():
    # Start MCP server
    process = subprocess.Popen(
        ["dmn", "mcp", "--config", "/absolute/path/to/dmn.json"],
        stdin=subprocess.PIPE,
        stdout=subprocess.PIPE,
        stderr=subprocess.PIPE,
        text=True
    )
    
    # Send initialize request
    request = {
        "jsonrpc": "2.0",
        "id": 1,
        "method": "initialize",
        "params": {}
    }
    
    process.stdin.write(json.dumps(request) + "\n")
    process.stdin.flush()
    
    # Read response
    response = process.stdout.readline()
    print("Response:", response)
    
    process.terminate()

if __name__ == "__main__":
    test_mcp()
```

### Expected Output

When working correctly, you should see:

1. **MCP server starts:**
   ```
   Starting MCP server mode with config: "dmn.json"
   ```

2. **Initialize response:**
   ```json
   {
     "jsonrpc": "2.0",
     "id": 1,
     "result": {
       "protocolVersion": "2025-06-18",
       "serverInfo": {
         "name": "opendaemon",
         "version": "0.1.0"
       }
     }
   }
   ```

3. **Tools list:**
   ```json
   {
     "tools": [
       {"name": "list_services"},
       {"name": "get_service_status"},
       {"name": "read_logs"},
       {"name": "watch_logs"},
       {"name": "start_service"},
       {"name": "stop_service"},
       {"name": "restart_service"}
     ]
   }
   ```

## Getting Help

If you're still having issues:

1. **Check the logs:**
   - Look for error messages in your AI assistant's output
   - Check VS Code's Output panel:
     - `OpenDaemon CLI` (runtime daemon/CLI diagnostics)

2. **Create a minimal test case:**
   - Use the simplest possible dmn.json
   - Test with just one service

3. **Report the issue:**
   - Include your dmn.json (remove sensitive data)
   - Include error messages
   - Specify your OS and AI assistant

## Support Channels

- **GitHub Issues:** [Report bugs](https://github.com/opendaemon/dmn/issues)
- **Discussions:** [Ask questions](https://github.com/opendaemon/dmn/discussions)
- **Discord:** [Community chat](https://discord.gg/opendaemon)

## Quick Reference

### Working MCP Configuration Examples

**Kiro (.kiro/settings/mcp.json):**
```json
{
  "mcpServers": {
    "opendaemon": {
      "command": "dmn",
      "args": ["mcp", "--config", "/absolute/path/to/dmn.json"],
      "disabled": false,
      "autoApprove": ["list_services", "get_service_status", "watch_logs"]
    }
  }
}
```

**Cursor (.cursor/mcp.json):**
```json
{
  "mcpServers": {
    "opendaemon": {
      "command": "dmn",
      "args": ["mcp", "--config", "/absolute/path/to/dmn.json"],
      "env": {}
    }
  }
}
```

**Claude Desktop:**
```json
{
  "mcpServers": {
    "opendaemon": {
      "command": "dmn",
      "args": ["mcp", "--config", "/absolute/path/to/dmn.json"]
    }
  }
}
```

### Test Commands

```bash
# Basic functionality
dmn --version
dmn mcp --config /absolute/path/to/dmn.json

# Service listing
echo '{"jsonrpc":"2.0","id":1,"method":"tools/call","params":{"name":"list_services","arguments":{}}}' | dmn mcp --config /absolute/path/to/dmn.json

# Status check
echo '{"jsonrpc":"2.0","id":2,"method":"tools/call","params":{"name":"get_service_status","arguments":{}}}' | dmn mcp --config /absolute/path/to/dmn.json
```