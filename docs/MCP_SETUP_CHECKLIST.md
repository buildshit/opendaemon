# MCP Setup Checklist

**✅ Follow this checklist to get MCP working with your AI assistant**

Print this page or keep it open while setting up MCP integration.

## Prerequisites

- [ ] OpenDaemon is installed and working
- [ ] You have a `dmn.json` file with services configured
- [ ] You're using Kiro, Cursor, or Claude Desktop

## Step 1: Test OpenDaemon

- [ ] **Test basic command:**
  ```bash
  dmn --version
  ```
  ✅ Should show version number

- [ ] **Test MCP server:**
  ```bash
  dmn mcp
  ```
  ✅ Should show: `Starting MCP server mode with config: "dmn.json"`
  
  Press `Ctrl+C` to stop

- [ ] **Check dmn.json exists:**
  ```bash
  ls dmn.json
  ```
  ✅ File should exist in your project root

## Step 2: Configure Your AI Assistant

### Option A: Manual MCP config (recommended for all IDEs)

- [ ] **Choose your MCP config file** (`.cursor/mcp.json`, `.kiro/settings/mcp.json`, `.antigravity/mcp.json`, etc.)
- [ ] **Copy a full snippet from** [MCP_QUICK_START.md](MCP_QUICK_START.md)
- [ ] **Use `--config` with an absolute `dmn.json` path**
- [ ] **Restart or reload your IDE after saving**

### Option B: Kiro (VS Code)

- [ ] **Create config directory:**
  ```bash
  mkdir -p .kiro/settings
  ```

- [ ] **Create `.kiro/settings/mcp.json`:**
  ```json
  {
    "mcpServers": {
      "opendaemon": {
        "command": "dmn",
        "args": ["mcp", "--config", "/absolute/path/to/dmn.json"],
        "disabled": false,
        "autoApprove": [
          "list_services",
          "get_service_status",
          "watch_logs"
        ]
      }
    }
  }
  ```

- [ ] **Reload VS Code:**
  - Press `Ctrl+Shift+P`
  - Type "Developer: Reload Window"
  - Press Enter

### Option C: Cursor

- [ ] **Create `.cursor/mcp.json` in your project:**
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

- [ ] **Restart Cursor**

### Option D: Claude Desktop

- [ ] **Find config file location:**
  - **macOS:** `~/Library/Application Support/Claude/claude_desktop_config.json`
  - **Windows:** `%APPDATA%/Claude/claude_desktop_config.json`

- [ ] **Add to config file:**
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

- [ ] **Restart Claude Desktop**

## Step 3: Test Integration

- [ ] **Basic test - Ask your AI:**
  ```
  "What services are configured in OpenDaemon?"
  ```
  ✅ Should list your services (database, backend-api, etc.)

- [ ] **Status test - Ask your AI:**
  ```
  "What's the status of my services?"
  ```
  ✅ Should show service statuses (`not_started`, `running`, `failed (...)`, etc.)

- [ ] **Log test - Ask your AI:**
  ```
  "Show me logs from the database service"
  ```
  ✅ Should ask for permission, then show logs or "no logs available"

- [ ] **Start test - Ask your AI:**
  ```
  "Please use the OpenDaemon MCP tool to start the frontend service"
  ```
  ✅ Should call `start_service` for `frontend` and then show `frontend` as `starting` or `running`

- [ ] **Restart test - Ask your AI:**
  ```
  "Great it works, please restart the frontend"
  ```
  ✅ Should call `restart_service` for `frontend`, then show `starting` and finally `running`

## Step 4: Verify Everything Works

- [ ] **AI can list services** ✅
- [ ] **AI can check service status** ✅  
- [ ] **AI can read logs (with permission)** ✅
- [ ] **AI can start/stop/restart services** ✅
- [ ] **AI can start and restart `frontend` on demand** ✅
- [ ] **No error messages in AI assistant** ✅

## Troubleshooting

If something doesn't work, check these common issues:

### ❌ "Command not found: dmn"

**Fix:** Use full path in config:
```json
{
  "command": "/full/path/to/dmn"
}
```

### ❌ "Failed to load dmn.json"

**Fix:** Use an absolute config path in args:
```json
{
  "args": ["mcp", "--config", "/absolute/path/to/dmn.json"]
}
```

### ❌ "MCP server not available"

**Fix:** 
1. Test `dmn mcp` manually
2. Restart your AI assistant
3. Check config file syntax

### ❌ "No services found"

**Fix:** Check your dmn.json has services defined:
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

### ❌ AI asks permission for every tool

**Fix:** Add auto-approve to your config:
```json
{
  "autoApprove": [
    "list_services",
    "get_service_status",
    "watch_logs"
  ]
}
```

## Success! 🎉

When everything works, you can ask your AI:

- **"Help debug my failing backend service"**
- **"What's wrong with my development environment?"**
- **"Check all service logs for errors"**
- **"Why isn't my frontend connecting to the backend?"**

Your AI can now read real service logs and help debug issues!

## Need More Help?

- **📖 Detailed Guide:** [MCP_QUICK_START.md](MCP_QUICK_START.md)
- **🔧 Troubleshooting:** [MCP_TROUBLESHOOTING.md](MCP_TROUBLESHOOTING.md)
- **📚 Full Reference:** [MCP_INTEGRATION.md](MCP_INTEGRATION.md)
- **🐛 Report Issues:** [GitHub Issues](https://github.com/opendaemon/dmn/issues)

---

**Print this checklist and check off each step as you complete it!**