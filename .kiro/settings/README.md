# Kiro MCP Configuration

This directory contains the MCP (Model Context Protocol) server configuration for Kiro.

## What is MCP?

MCP allows Kiro (this AI assistant) to interact with your OpenDaemon services by:
- Reading service logs
- Checking service status
- Listing configured services

## Configuration File

**mcp.json** - Configures the OpenDaemon MCP server

### Current Configuration

```json
{
  "mcpServers": {
    "opendaemon": {
      "command": "F:\\test apps\\opendaemon\\target\\release\\dmn.exe",
      "args": ["mcp"],
      "env": {},
      "disabled": false,
      "autoApprove": [
        "list_services",
        "get_service_status"
      ],
      "disabledTools": []
    }
  }
}
```

### Configuration Explained

- **command**: Full path to the Rust-compiled dmn.exe binary
- **args**: `["mcp"]` tells dmn to run in MCP server mode
- **disabled**: Set to `false` to enable the server
- **autoApprove**: Tools that don't require user approval each time
  - `list_services` - Safe to auto-approve (just lists service names)
  - `get_service_status` - Safe to auto-approve (just shows status)
  - `read_logs` is NOT auto-approved (requires user permission to read logs)
- **disabledTools**: List of tools to disable (empty = all tools enabled)

## How It Works

1. **Rust MCP Server**: The `dmn.exe` binary (compiled from Rust code in `core/src/mcp_server.rs`) runs as an MCP server
2. **JSON-RPC Protocol**: Communicates via JSON-RPC 2.0 over stdio
3. **Kiro Integration**: Kiro can now call the MCP tools to help you debug services

## Available Tools

The Rust MCP server exposes 3 tools:

### 1. list_services
Lists all services defined in your dmn.json

**Auto-approved**: Yes (safe, read-only)

### 2. get_service_status
Shows the current status of all services (Running, Stopped, Failed, etc.)

**Auto-approved**: Yes (safe, read-only)

### 3. read_logs
Reads log output from a specific service

**Auto-approved**: No (requires user permission)

## Testing the Configuration

After reloading Kiro, you can test by asking:

- "What services are configured in OpenDaemon?"
- "What's the status of my services?"
- "Show me the logs from the database service"

## Reconnecting the Server

If you make changes to mcp.json:
1. The server will automatically reconnect
2. Or use the MCP Server view in Kiro's feature panel to manually reconnect

## Troubleshooting

### Server Not Connecting

1. Check that dmn.exe exists at the specified path:
   ```powershell
   Test-Path "F:\test apps\opendaemon\target\release\dmn.exe"
   ```

2. Test the server manually:
   ```powershell
   & "F:\test apps\opendaemon\target\release\dmn.exe" mcp
   ```
   (Press Ctrl+C to stop)

3. Check Kiro's MCP Server view for connection status

### Changing the Path

If you move the project or want to use a relative path, update the `command` field in mcp.json.

For a relative path (if dmn.exe is in PATH):
```json
"command": "dmn"
```

For absolute path (current):
```json
"command": "F:\\test apps\\opendaemon\\target\\release\\dmn.exe"
```

## Architecture

```
┌─────────────────┐
│  Kiro (VS Code) │
│   AI Assistant  │
└────────┬────────┘
         │ MCP Protocol
         │ (JSON-RPC over stdio)
         ▼
┌─────────────────┐
│   dmn.exe       │
│  (Rust Binary)  │
│  MCP Server     │
└────────┬────────┘
         │
         ▼
┌─────────────────┐
│  Orchestrator   │
│  Service Logs   │
│  Process Mgmt   │
└─────────────────┘
```

## Source Code

The MCP server implementation is in:
- **core/src/mcp_server.rs** - Main MCP server logic (Rust)
- **core/src/main.rs** - Entry point for `dmn mcp` command (Rust)

## References

- [MCP Integration Guide](../../docs/MCP_INTEGRATION.md)
- [Model Context Protocol Spec](https://modelcontextprotocol.io/)
- [Test Results](../../MCP_SERVER_TEST_RESULTS.md)
