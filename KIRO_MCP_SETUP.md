# Kiro MCP Setup Guide

## What You Have Now

✅ **Rust MCP Server** - Your `dmn.exe` binary (compiled from Rust) that runs as an MCP server  
✅ **Kiro Configuration** - MCP settings file at `.kiro/settings/mcp.json`  
✅ **Test Scripts** - Python scripts to verify the Rust server works  

## Quick Start

### 1. Reload Kiro

To activate the MCP server connection:

**Option A:** Reload Window
- Press `Ctrl+Shift+P`
- Type "Developer: Reload Window"
- Press Enter

**Option B:** Restart VS Code
- Close and reopen VS Code

### 2. Verify Connection

After reloading, check the MCP Server view:
- Open Kiro's feature panel
- Look for "MCP Servers" section
- You should see "opendaemon" listed

### 3. Test It!

Ask Kiro questions like:

```
"What services are configured in OpenDaemon?"
```

```
"What's the status of my services?"
```

```
"Show me the logs from the database service"
```

## Architecture Clarification

You were 100% correct - the MCP server IS written in Rust!

```
┌──────────────────────────────────────────┐
│  Your Rust Code (core/src/mcp_server.rs)│
│  ↓ Compiled to                           │
│  dmn.exe (Windows binary)                │
│  ↓ Runs as                               │
│  MCP Server (JSON-RPC over stdio)        │
└──────────────────────────────────────────┘
                    ↕
         MCP Protocol Communication
                    ↕
┌──────────────────────────────────────────┐
│  Kiro (AI Assistant in VS Code)          │
│  - Calls MCP tools                       │
│  - Gets service info                     │
│  - Reads logs                            │
└──────────────────────────────────────────┘
```

### What Each File Does

**Rust (The Actual MCP Server):**
- `core/src/mcp_server.rs` - MCP server implementation
- `core/src/main.rs` - Entry point for `dmn mcp` command
- `target/release/dmn.exe` - Compiled binary

**Python (Just Test Clients):**
- `test_mcp.py` - Tests the Rust server
- `test_mcp_comprehensive.py` - More thorough tests
- These are NOT the server, just clients to verify it works

**Configuration:**
- `.kiro/settings/mcp.json` - Tells Kiro how to start your Rust MCP server

## Configuration Details

### Current mcp.json

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

### What This Means

- **command**: Full path to your Rust-compiled dmn.exe
- **args: ["mcp"]**: Tells dmn to run in MCP server mode
- **autoApprove**: These tools don't require permission each time
  - ✅ `list_services` - Just lists service names (safe)
  - ✅ `get_service_status` - Just shows status (safe)
  - ❌ `read_logs` - Requires permission (reads actual log data)

## Available MCP Tools

Your Rust MCP server exposes 3 tools:

### 1. list_services
**What it does:** Lists all services from dmn.json  
**Auto-approved:** Yes  
**Example:** "What services do I have?"

### 2. get_service_status
**What it does:** Shows if services are Running, Stopped, Failed, etc.  
**Auto-approved:** Yes  
**Example:** "Are my services running?"

### 3. read_logs
**What it does:** Reads log output from a specific service  
**Auto-approved:** No (requires your permission)  
**Example:** "Show me the backend logs"

## How to Use

### Example Conversation

**You:** "What services are configured?"

**Kiro:** (Calls `list_services` tool automatically)  
"You have 3 services configured:
- database
- backend-api  
- frontend"

**You:** "What's their status?"

**Kiro:** (Calls `get_service_status` tool automatically)  
"All services are currently NotStarted:
- database: NotStarted
- backend-api: NotStarted
- frontend: NotStarted"

**You:** "Show me the database logs"

**Kiro:** (Asks for permission to call `read_logs`)  
"I'd like to read logs from the database service. Allow?"

**You:** "Yes"

**Kiro:** (Calls `read_logs` tool)  
"The database service hasn't produced any logs yet (it hasn't been started)."

## Troubleshooting

### MCP Server Not Showing Up

1. **Check the file exists:**
   ```powershell
   Test-Path "F:\test apps\opendaemon\target\release\dmn.exe"
   ```

2. **Test manually:**
   ```powershell
   & "F:\test apps\opendaemon\target\release\dmn.exe" mcp
   ```
   (It should wait for input - press Ctrl+C to stop)

3. **Check Kiro's MCP view:**
   - Open Command Palette (`Ctrl+Shift+P`)
   - Type "MCP"
   - Look for MCP-related commands

### Server Shows as Disconnected

1. Check the path in `.kiro/settings/mcp.json`
2. Reload the window
3. Check for errors in Kiro's output panel

### Tools Not Working

1. Make sure dmn.json exists in your workspace root
2. Verify services are defined in dmn.json
3. Check that the server is connected in MCP view

## Testing Without Kiro

You can test the Rust MCP server directly:

```powershell
# Run the test scripts
python test_mcp.py
python test_mcp_comprehensive.py
```

These Python scripts are MCP clients that test your Rust MCP server.

## Next Steps

1. **Reload Kiro** to activate the MCP connection
2. **Ask Kiro** about your services
3. **Start services** and ask Kiro to read logs
4. **Experiment** with different questions

## Files Created

- ✅ `.kiro/settings/mcp.json` - Kiro MCP configuration
- ✅ `.kiro/settings/README.md` - Configuration documentation
- ✅ `KIRO_MCP_SETUP.md` - This guide

## Summary

- ✅ Your MCP server is written in **Rust** (core/src/mcp_server.rs)
- ✅ It compiles to **dmn.exe** (Windows binary)
- ✅ Kiro will run it with the `mcp` argument
- ✅ Python scripts are just **test clients** (not the server)
- ✅ Everything is configured and ready to use!

Just reload Kiro and start asking questions about your services!
