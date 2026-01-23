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

Choose your AI assistant and follow the setup:

### Option A: Kiro (VS Code)

1. **Create the config directory:**
   ```bash
   mkdir -p .kiro/settings
   ```

2. **Create `.kiro/settings/mcp.json`:**
   ```json
   {
     "mcpServers": {
       "opendaemon": {
         "command": "dmn",
         "args": ["mcp"],
         "disabled": false,
         "autoApprove": [
           "list_services",
           "get_service_status"
         ]
       }
     }
   }
   ```

3. **Reload VS Code:**
   - Press `Ctrl+Shift+P`
   - Type "Developer: Reload Window"
   - Press Enter

### Option B: Cursor

1. **Create `.cursor/mcp.json` in your project:**
   ```json
   {
     "mcpServers": {
       "opendaemon": {
         "command": "dmn",
         "args": ["mcp"],
         "env": {}
       }
     }
   }
   ```

2. **Restart Cursor**

### Option C: Claude Desktop

1. **Find your config file:**
   - **macOS:** `~/Library/Application Support/Claude/claude_desktop_config.json`
   - **Windows:** `%APPDATA%/Claude/claude_desktop_config.json`

2. **Add OpenDaemon to the config:**
   ```json
   {
     "mcpServers": {
       "opendaemon": {
         "command": "dmn",
         "args": ["mcp"]
       }
     }
   }
   ```

3. **Restart Claude Desktop**

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

**Expected response:** Your AI should show service statuses (NotStarted, Running, Failed, etc.)

### Log Reading (requires permission)
```
"Show me the logs from the database service"
```

**Expected response:** Your AI should ask for permission, then show logs (or say no logs if services aren't started)

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
         "args": ["mcp"]
       }
     }
   }
   ```

3. **Check your workspace:**
   - Make sure you're in a directory with `dmn.json`
   - The MCP server needs this file to work

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
      "args": ["mcp"],
      "autoApprove": [
        "list_services",
        "get_service_status"
      ]
    }
  }
}
```

**Safe to auto-approve:**
- `list_services` - Just lists service names
- `get_service_status` - Just shows status

**Requires permission:**
- `read_logs` - Reads actual log data

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