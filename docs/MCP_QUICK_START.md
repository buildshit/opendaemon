# MCP Quick Start Guide

**Get your AI assistant talking to OpenDaemon in 5 minutes!**

## What is MCP?

MCP (Model Context Protocol) lets AI assistants like Kiro, Cursor, or Claude Desktop read your service logs and status. This means your AI can help debug issues by looking at actual runtime data from your development services.

## Prerequisites

- ✅ OpenDaemon installed and working
- ✅ A `dmn.json` file with your services configured
- ✅ An AI assistant that supports MCP (Kiro, Cursor, Claude Desktop, etc.)

## Step 1: Test Your OpenDaemon Setup

First, make sure OpenDaemon works normally:

```bash
# Check that dmn.exe exists
dmn --help

# Test with your services
dmn start
```

If this doesn't work, set up OpenDaemon first using the main [README.md](../README.md).

## Step 2: Test the MCP Server

Test that the MCP server starts correctly:

```bash
# Start the MCP server (it will wait for input)
dmn mcp
```

You should see:
```
Starting MCP server mode with config: "dmn.json"
```

The server is now waiting for MCP requests. Press `Ctrl+C` to stop it.

✅ **If this works, your MCP server is ready!**

## Step 3: Configure Your AI Assistant

OpenDaemon now uses **manual MCP configuration only**.  
Choose the file your client reads (`.cursor/mcp.json`, `.kiro/settings/mcp.json`, `.antigravity/mcp.json`, etc.) and paste one of these snippets.

> Always include `--config` with an absolute `dmn.json` path.  
> This avoids failures when IDEs launch MCP servers from a non-workspace working directory.
> If you build from source and have multiple binaries, point to your newest build output.
> With matching config paths, MCP calls share the active OpenDaemon extension daemon runtime.

### Option A: Cursor / Kiro / Antigravity (`mcpServers`)

#### Windows

```json
{
  "mcpServers": {
    "opendaemon": {
      "command": "C:\\path\\to\\dmn.exe",
      "args": [
        "mcp",
        "--config",
        "C:\\path\\to\\dmn.json"
      ],
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

#### macOS

```json
{
  "mcpServers": {
    "opendaemon": {
      "command": "/absolute/path/to/dmn",
      "args": [
        "mcp",
        "--config",
        "/absolute/path/to/dmn.json"
      ],
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

#### Linux

```json
{
  "mcpServers": {
    "opendaemon": {
      "command": "/absolute/path/to/dmn",
      "args": [
        "mcp",
        "--config",
        "/absolute/path/to/dmn.json"
      ],
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

### Option B: VS Code `mcp.json` format (`servers`)

Use this in `.vscode/mcp.json` when your VS Code build expects `servers` format:

```json
{
  "servers": {
    "opendaemon": {
      "type": "stdio",
      "command": "C:\\path\\to\\dmn.exe",
      "args": [
        "mcp",
        "--config",
        "C:\\path\\to\\dmn.json"
      ],
      "env": {}
    }
  }
}
```

### Option C: Claude Desktop

1. Edit your Claude Desktop config file:
   - **macOS:** `~/Library/Application Support/Claude/claude_desktop_config.json`
   - **Windows:** `%APPDATA%/Claude/claude_desktop_config.json`
2. Paste the same `mcpServers` snippet from **Option A**.
3. Restart Claude Desktop.

## Step 4: Test the Integration

Ask your AI assistant these questions:

### Basic Test
```
"What services are configured in OpenDaemon?"
```

**Expected response:** Your AI should list your services (database, backend-api, frontend, etc.)

### Status Check
```
"What's the status of my services?"
```

**Expected response:** Your AI should show service statuses (`not_started`, `running`, `failed (...)`, etc.)

### Log Reading (requires permission)
```
"Show me the logs from the database service"
```

**Expected response:** Your AI should ask for permission, then show logs (or say no logs if services aren't started)

### Service Control Verification (Start + Restart)
Use this exact workflow to confirm mutating tools are wired correctly:

1. Ask your AI:
   ```
   "Please use the OpenDaemon MCP tool to start the frontend service"
   ```
2. Expected MCP behavior:
   - `list_services` includes `frontend`
   - `start_service` returns "Start requested for 'frontend' (dependencies included)."
   - `get_service_status` shows `frontend` as `starting` or `running`
   - `read_logs` for `frontend` shows recent startup/runtime output
3. Ask your AI:
   ```
   "Great it works, please restart the frontend"
   ```
4. Expected MCP behavior:
   - `restart_service` is called with `{ "service": "frontend" }`
   - `get_service_status` returns `frontend` as `starting`, then `running`
   - Dependent services (for example `backend-api`, `database`) remain healthy

If this sequence succeeds, your MCP client is controlling the active OpenDaemon runtime correctly.

## Step 5: Start Using It!

Now you can ask your AI assistant to help with development tasks:

### Debugging
```
"My backend service is failing, can you help debug it?"
```

### Service Management
```
"Are all my services running properly?"
```

```
"Start the backend-api service, then watch logs until it prints 'running'"
```

### Log Analysis
```
"Check the frontend logs for any errors in the last 50 lines"
```

### Environment Overview
```
"Give me an overview of my development environment"
```

## Troubleshooting

### "MCP server not found" or "Connection failed"

1. **Check the command path:**
   ```bash
   # Make sure this works
   dmn --version
   ```

2. **Use full path if needed:**
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
3. **Prefer explicit config path:**
   - Always pass `--config` with an absolute path to `dmn.json`
   - This avoids working-directory issues in IDE MCP runners

### "No services found"

1. **Verify dmn.json exists:**
   ```bash
   ls dmn.json
   ```

2. **Check the file content:**
   ```bash
   cat dmn.json
   ```

3. **Make sure it has services defined:**
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

### AI asks for permission every time

This is normal for the `read_logs` tool. To auto-approve safe tools, add them to `autoApprove`:

```json
{
  "mcpServers": {
    "opendaemon": {
      "command": "dmn",
      "args": ["mcp", "--config", "/absolute/path/to/dmn.json"],
      "autoApprove": [
        "list_services",
        "get_service_status",
        "watch_logs"
      ]
    }
  }
}
```

**Safe to auto-approve:**
- `list_services` - Just lists service names
- `get_service_status` - Just shows status
- `watch_logs` - Read-only live log watch with filters

**Requires permission:**
- `read_logs` - Reads actual log data
- `start_service` / `stop_service` / `restart_service` - Mutates runtime service state

## What Can Your AI Do Now?

With MCP integration, your AI assistant can:

### 🔍 **Debug Issues**
- Read error logs from failing services
- Correlate errors across multiple services
- Suggest fixes based on actual error messages

### 📊 **Monitor Status**
- Check which services are running
- Identify failed or stuck services
- Understand service dependencies

### 🛠️ **Control Services**
- Start a service with dependencies (`start_service`)
- Stop a running service (`stop_service`)
- Restart unhealthy services (`restart_service`)
- Watch live logs with stop conditions (`watch_logs`)

### 🚀 **Development Workflow**
- Help you understand startup issues
- Analyze performance problems in logs
- Guide you through service configuration

### 💡 **Smart Suggestions**
- Recommend configuration changes
- Help optimize service startup order
- Suggest debugging strategies

## Example Conversation

**You:** "My app isn't working, can you help?"

**AI:** *[Calls get_service_status automatically]*  
"I can see your services. The database and backend-api are running, but the frontend shows as Failed. Let me check the frontend logs."

*[Asks permission to read logs]*

**You:** "Yes, check the logs"

**AI:** *[Calls read_logs for frontend]*  
"I found the issue! The frontend logs show 'ECONNREFUSED localhost:8080' - it's trying to connect to the backend on port 8080, but your backend-api is running on port 3000. You need to update your frontend configuration to point to the correct backend URL."

## Next Steps

- **Read the full guide:** [MCP_INTEGRATION.md](MCP_INTEGRATION.md) for advanced features
- **Customize auto-approval:** Add more tools to `autoApprove` as needed
- **Try different questions:** Experiment with various debugging scenarios
- **Share feedback:** Let us know how MCP integration helps your workflow!

## Need Help?

- **Issues:** [GitHub Issues](https://github.com/opendaemon/dmn/issues)
- **Discussions:** [GitHub Discussions](https://github.com/opendaemon/dmn/discussions)
- **Discord:** [Join our community](https://discord.gg/opendaemon)

---

**🎉 Congratulations!** Your AI assistant can now help debug your development services using real runtime data. Happy coding!