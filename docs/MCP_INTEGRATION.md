# MCP Integration Guide

OpenDaemon includes a Model Context Protocol (MCP) server that allows AI coding assistants to read service logs and status information. This enables AI agents to help debug issues by analyzing runtime output from your development services.

## 📚 Documentation

**Choose your path:**

- **🚀 [Quick Start Guide](MCP_QUICK_START.md)** - Get started in 5 minutes
- **✅ [Setup Checklist](MCP_SETUP_CHECKLIST.md)** - Step-by-step checklist  
- **🔧 [Troubleshooting Guide](MCP_TROUBLESHOOTING.md)** - Fix common issues
- **📖 This document** - Complete technical reference

## Table of Contents

- [Overview](#overview)
- [Quick Start](#quick-start)
- [Available Tools](#available-tools)
- [Configuration](#configuration)
- [Usage Examples](#usage-examples)
- [Authentication](#authentication)
- [Troubleshooting](#troubleshooting)

## Overview

The MCP server exposes seven tools that AI agents can call:

1. **list_services** *(read-only)* - List all services in `dmn.json`
2. **get_service_status** *(read-only)* - Get status for all services
3. **read_logs** *(read-only)* - Read buffered logs from a service
4. **watch_logs** *(read-only)* - Watch live logs with duration/pattern filters
5. **start_service** - Start one service (and dependencies)
6. **stop_service** - Stop one service
7. **restart_service** - Restart one service

This combination lets AI assistants both **observe** and **act**:
- Observe: inspect current state and targeted logs without pulling unnecessary output
- Act: start/stop/restart services directly from tool calls
- Diagnose: watch for specific patterns and stop automatically once matched

## Quick Start

### 1. Start the MCP Server

```bash
dmn mcp
```

This starts OpenDaemon in MCP mode, which:
- Reads your `dmn.json` configuration
- Exposes MCP tools on stdio for AI agent communication
- Waits for MCP tool requests
- Reuses the active OpenDaemon extension daemon (when present for the same config) so MCP and the extension UI stay in sync

> `dmn mcp` does **not** auto-start services.  
> Use `start_service` from MCP (or CLI/extension actions) to launch services.

### 2. Configure Your AI Assistant

#### Manual MCP config (all IDEs)

OpenDaemon now relies on manual MCP client configuration.
Use absolute paths and always include `--config` to avoid working-directory issues.

#### Cursor / Kiro / Antigravity (`mcpServers`)

Add to your Cursor settings (`.cursor/mcp.json`):

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

#### Claude Desktop

Add to `~/Library/Application Support/Claude/claude_desktop_config.json` (macOS):

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

#### VS Code with Kiro

Add to `.kiro/settings/mcp.json`:

```json
{
  "mcpServers": {
    "opendaemon": {
      "command": "dmn",
      "args": ["mcp", "--config", "/absolute/path/to/dmn.json"],
      "disabled": false,
      "autoApprove": ["list_services", "get_service_status"],
      "disabledTools": []
    }
  }
}
```

### 3. Test the Integration

Ask your AI assistant:
- "What services are running?"
- "Show me the last 50 lines of logs from the backend service"
- "Start backend-api and watch logs until it prints 'Server ready'"
- "Restart frontend and monitor stderr for 10 seconds"

## Available Tools

All tool calls return MCP `content` + `isError` responses.

### list_services

List all services defined in the `dmn.json` configuration.

**Parameters:**

```typescript
{} // No parameters required
```

**Example request:**

```json
{
  "name": "list_services",
  "arguments": {}
}
```

### get_service_status

Get the current status of all configured services.

**Parameters:**

```typescript
{} // No parameters required
```

**Status values:**
- `not_started`
- `starting`
- `running`
- `stopped`
- `failed (exit code: X)`

### read_logs

Read buffered log output from a specific service.

**Parameters:**

```typescript
{
  service: string;                // Service name (required)
  lines: number | "all";          // Required
  contains?: string;              // Optional substring filter
  caseSensitive?: boolean;        // Default false
  stream?: "stdout" | "stderr" | "both"; // Default "both"
}
```

### watch_logs

Watch live logs with bounded runtime and token-aware filters.

```typescript
{
  service: string;                 // Required
  durationSeconds?: number;        // Required if untilPattern not set
  untilPattern?: string;           // Regex. Required if durationSeconds not set
  timeoutSeconds?: number;         // Optional hard timeout (0 disables)
  pollIntervalMs?: number;         // Default 250, minimum 50
  maxLines?: number;               // Default 200
  includeExisting?: boolean;       // Default false
  includePatterns?: string[];      // Regex include filters (any match)
  excludePatterns?: string[];      // Regex exclude filters
  caseSensitive?: boolean;         // Default false
  stream?: "stdout" | "stderr" | "both"; // Default "both"
}
```

**Example request (watch for readiness text):**

```json
{
  "name": "watch_logs",
  "arguments": {
    "service": "backend",
    "untilPattern": "Server ready",
    "timeoutSeconds": 30,
    "stream": "stdout"
  }
}
```

### start_service

Start a service by name (including dependencies).

```typescript
{
  service: string;
}
```

### stop_service

Stop a running service by name.

```typescript
{
  service: string;
}
```

### restart_service

Restart a service by name.

```typescript
{
  service: string;
}
```

## Configuration

### MCP Server Options

The MCP server can be configured via environment variables:

```bash
# Set log buffer size (default: 1000 lines per service)
export DMN_LOG_BUFFER_SIZE=5000

# Set ready timeout (default: 60 seconds)
export DMN_READY_TIMEOUT=120

# Start MCP server
dmn mcp --config /absolute/path/to/dmn.json
```

### Workspace Configuration

Ensure your `dmn.json` is properly configured:

```json
{
  "version": "1.0",
  "services": {
    "backend": {
      "command": "npm run dev",
      "ready_when": {
        "log_contains": "Server listening"
      }
    }
  }
}
```

The MCP server will:
1. Load this configuration
2. Expose tools for AI agents to query and control services
3. Wait for tool calls (services start only when explicitly requested)

## Usage Examples

### Example 1: Debugging a Failed Service

**User to AI:** "The backend service is failing, can you help?"

**AI uses tools:**
1. `get_service_status()` - Confirms backend is "Failed"
2. `read_logs(service: "backend", lines: 50)` - Reads recent logs
3. Analyzes error messages
4. Suggests fix based on actual error

**AI response:** "The backend is failing because it can't connect to the database. The logs show 'ECONNREFUSED localhost:5432'. Let me check if the database is running..."

### Example 2: Understanding Startup Order

**User to AI:** "Why isn't my frontend starting?"

**AI uses tools:**
1. `list_services()` - Gets all services
2. `get_service_status()` - Checks status of all services
3. `read_logs(service: "backend", lines: 20)` - Checks backend logs
4. Identifies that backend hasn't reached ready state

**AI response:** "Your frontend depends on the backend, which is still starting. The backend logs show it's waiting for database migrations to complete. Once that finishes, the frontend will start automatically."

### Example 3: Analyzing Error Patterns

**User to AI:** "I'm seeing intermittent errors, can you analyze the logs?"

**AI uses tools:**
1. `read_logs(service: "backend", lines: "all")` - Gets all backend logs
2. `read_logs(service: "database", lines: "all")` - Gets all database logs
3. Correlates timestamps and error patterns
4. Identifies connection pool exhaustion

**AI response:** "I found the issue. Your backend is exhausting the database connection pool. The logs show 'too many clients' errors from PostgreSQL starting at 10:15:23, which correlates with the backend errors. You should increase the connection pool size."

### Example 4: Service Discovery

**User to AI:** "What services do I have running?"

**AI uses tools:**
1. `list_services()` - Gets service names
2. `get_service_status()` - Gets their statuses

**AI response:** "You have 5 services configured:
- database: Running
- redis: Running  
- backend: Running
- frontend: Running
- worker: Stopped

All critical services are running. The worker service is stopped but that appears intentional."

### Example 5: Start and Restart Frontend (Validated Workflow)

**User to AI:** "Please use the OpenDaemon MCP tool to start the frontend service"

**AI uses tools:**
1. `list_services()` - Verifies `frontend` exists
2. `start_service(service: "frontend")` - Requests startup (with dependencies)
3. `get_service_status()` - Confirms `frontend` is `starting` or `running`
4. `read_logs(service: "frontend", lines: 20)` - Verifies runtime output

**User to AI:** "Great it works, please restart the frontend"

**AI uses tools:**
1. `restart_service(service: "frontend")`
2. `get_service_status()` - Confirms transition back to `running`

**Expected response pattern:** "frontend restarted successfully and is running; dependency services (for example `backend-api` and `database`) remain running."

This workflow is a practical end-to-end check that MCP mutating tools are wired correctly in your IDE.

## Authentication

### Current Status

In the current version (1.0), the MCP server runs without authentication. This is suitable for local development environments.

### Future: Pro Authentication

Future versions will include authentication for Pro features:

```bash
# Login to OpenDaemon Pro
dmn login

# Start MCP server with authentication
dmn mcp --auth
```

**Pro Features (Coming Soon):**
- Remote service management
- Team collaboration
- Enhanced security
- Cloud log storage
- Advanced analytics

### Security Considerations

**Current (Local Development):**
- MCP server only accessible via stdio (not network)
- Runs with your user permissions
- Logs stored in memory only
- No data leaves your machine

**Best Practices:**
- Don't commit sensitive data to logs
- Use environment files for secrets
- Review AI agent permissions in your IDE

## Troubleshooting

### MCP Server Won't Start

**Error:** `Failed to load dmn.json`

**Solution:** Ensure `dmn.json` exists in your workspace root:
```bash
ls dmn.json
```

**Error:** `Service 'X' not found`

**Solution:** Check your service names in `dmn.json`:
```bash
cat dmn.json | grep -A 5 services
```

### AI Agent Can't Connect

**Issue:** AI says "MCP server not available"

**Solutions:**

1. Verify MCP server is running:
```bash
ps aux | grep "dmn mcp"
```

2. Check your AI assistant's MCP configuration
3. Restart your AI assistant
4. Check for error messages in the MCP server output

### No Logs Returned

**Issue:** `read_logs` returns empty array

**Possible Causes:**

1. **Service hasn't started yet**
   - Check status with `get_service_status`
   - Wait for service to start

2. **Service hasn't produced output**
   - Verify the service is actually running
   - Check if the command is correct

3. **Logs rotated out of buffer**
   - Increase buffer size: `DMN_LOG_BUFFER_SIZE=5000 dmn mcp`
   - Request logs sooner after events

### Tool Call Errors

**Error:** `Service 'backend' not found`

**Solution:** Verify service name matches exactly:
```bash
# List services
dmn mcp
# Then ask AI: "List all services"
```

**Error:** `Invalid lines parameter`

**Solution:** Use a number or "all":
```json
// ✅ Valid
{"service": "backend", "lines": 100}
{"service": "backend", "lines": "all"}

// ❌ Invalid
{"service": "backend", "lines": "last"}
{"service": "backend", "lines": -1}
```

## Advanced Usage

### Custom MCP Client

You can build custom MCP clients that interact with OpenDaemon:

```typescript
import { MCPClient } from '@modelcontextprotocol/sdk';

const client = new MCPClient({
  command: 'dmn',
  args: ['mcp', '--config', '/absolute/path/to/dmn.json']
});

await client.connect();

// Read logs
const logs = await client.callTool('read_logs', {
  service: 'backend',
  lines: 100
});

console.log(logs);
```

### Programmatic Access

For automation or testing:

```bash
# Start MCP server in background
dmn mcp --config /absolute/path/to/dmn.json &

# Use MCP client to query
echo '{"jsonrpc":"2.0","id":1,"method":"tools/call","params":{"name":"get_service_status","arguments":{}}}' | dmn mcp --config /absolute/path/to/dmn.json
```

### Integration with CI/CD

Use MCP tools in CI pipelines:

```yaml
# .github/workflows/test.yml
- name: Start services
  run: dmn mcp --config /absolute/path/to/dmn.json &
  
- name: Wait for services
  run: |
    while ! dmn-cli status | grep -q "All services running"; do
      sleep 1
    done
    
- name: Run tests
  run: npm test
```

## Best Practices

### 1. Use Descriptive Service Names

✅ Good:
```json
{
  "services": {
    "postgres-db": { ... },
    "auth-api": { ... },
    "frontend-dev": { ... }
  }
}
```

❌ Avoid:
```json
{
  "services": {
    "service1": { ... },
    "s2": { ... }
  }
}
```

### 2. Request Appropriate Log Amounts

```typescript
// ✅ For recent errors
read_logs({ service: "backend", lines: 50 })

// ✅ For full analysis
read_logs({ service: "backend", lines: "all" })

// ❌ Too many for quick checks
read_logs({ service: "backend", lines: 10000 })
```

### 3. Check Status Before Reading Logs

```typescript
// ✅ Good practice
const status = await get_service_status();
if (status.services.backend === "running") {
  const logs = await read_logs({ service: "backend", lines: 100 });
}

// ❌ May fail if service not started
const logs = await read_logs({ service: "backend", lines: 100 });
```

### 4. Use Auto-Approve for Safe Tools

In `.kiro/settings/mcp.json`:

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

This allows AI to inspect status and run read-only log watches without prompting, while still requiring approval for mutating service-control tools.

## See Also

- [README.md](../README.md) - Quick start guide
- [DMN_JSON_SCHEMA.md](DMN_JSON_SCHEMA.md) - Configuration reference
- [Model Context Protocol Specification](https://modelcontextprotocol.io/docs)
- [CONTRIBUTING.md](../CONTRIBUTING.md) - Contributing guidelines

## Support

- **Issues:** [GitHub Issues](https://github.com/opendaemon/dmn/issues)
- **Discussions:** [GitHub Discussions](https://github.com/opendaemon/dmn/discussions)
- **Discord:** [Join our community](https://discord.gg/opendaemon)
- **Documentation:** [Full docs](https://opendaemon.com/docs)
