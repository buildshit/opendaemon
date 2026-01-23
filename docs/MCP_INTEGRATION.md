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

The MCP server exposes three tools that AI agents can call:

1. **read_logs** - Read log output from any service
2. **get_service_status** - Get the current status of all services
3. **list_services** - List all services defined in dmn.json

These tools allow AI assistants like Cursor, GitHub Copilot, or Claude to:
- Analyze error messages in service logs
- Understand the state of your development environment
- Suggest fixes based on actual runtime behavior
- Help debug issues across multiple services

## Quick Start

### 1. Start the MCP Server

```bash
dmn mcp
```

This starts OpenDaemon in MCP mode, which:
- Reads your `dmn.json` configuration
- Starts all services according to dependency order
- Exposes MCP tools on stdio for AI agent communication

### 2. Configure Your AI Assistant

#### Cursor

Add to your Cursor settings (`.cursor/mcp.json`):

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

#### Claude Desktop

Add to `~/Library/Application Support/Claude/claude_desktop_config.json` (macOS):

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

#### VS Code with Kiro

Add to `.kiro/settings/mcp.json`:

```json
{
  "mcpServers": {
    "opendaemon": {
      "command": "dmn",
      "args": ["mcp"],
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
- "Why is the frontend service failing?"

## Available Tools

### read_logs

Read log output from a specific service.

**Parameters:**

```typescript
{
  service: string;      // Name of the service (must exist in dmn.json)
  lines: number | "all"; // Number of lines to return, or "all"
}
```

**Returns:**

```typescript
{
  service: string;
  logs: string[];  // Array of log lines
}
```

**Example Request:**

```json
{
  "name": "read_logs",
  "arguments": {
    "service": "backend",
    "lines": 100
  }
}
```

**Example Response:**

```json
{
  "service": "backend",
  "logs": [
    "[2026-01-20 10:15:23] Server starting...",
    "[2026-01-20 10:15:24] Connected to database",
    "[2026-01-20 10:15:25] Listening on port 3000",
    "..."
  ]
}
```

**Use Cases:**
- Debugging errors: "Show me the backend logs"
- Analyzing startup: "What happened when the database started?"
- Finding patterns: "Are there any errors in the frontend logs?"

### get_service_status

Get the current status of all services.

**Parameters:**

```typescript
{} // No parameters required
```

**Returns:**

```typescript
{
  services: {
    [serviceName: string]: "NotStarted" | "Starting" | "Running" | "Stopped" | "Failed"
  }
}
```

**Example Request:**

```json
{
  "name": "get_service_status",
  "arguments": {}
}
```

**Example Response:**

```json
{
  "services": {
    "database": "Running",
    "backend": "Running",
    "frontend": "Failed",
    "worker": "Starting"
  }
}
```

**Use Cases:**
- Environment overview: "What's the status of my services?"
- Dependency checking: "Is the database running?"
- Failure detection: "Which services have failed?"

### list_services

List all services defined in the dmn.json configuration.

**Parameters:**

```typescript
{} // No parameters required
```

**Returns:**

```typescript
{
  services: string[]  // Array of service names
}
```

**Example Request:**

```json
{
  "name": "list_services",
  "arguments": {}
}
```

**Example Response:**

```json
{
  "services": [
    "database",
    "redis",
    "backend",
    "frontend",
    "worker"
  ]
}
```

**Use Cases:**
- Discovery: "What services are configured?"
- Validation: "Is there a service called 'api'?"
- Overview: "List all my services"

## Configuration

### MCP Server Options

The MCP server can be configured via environment variables:

```bash
# Set log buffer size (default: 1000 lines per service)
export DMN_LOG_BUFFER_SIZE=5000

# Set ready timeout (default: 60 seconds)
export DMN_READY_TIMEOUT=120

# Start MCP server
dmn mcp
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
2. Start all services
3. Expose tools for AI agents to query

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
  args: ['mcp']
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
dmn mcp &

# Use MCP client to query
echo '{"jsonrpc":"2.0","id":1,"method":"tools/call","params":{"name":"get_service_status","arguments":{}}}' | dmn mcp
```

### Integration with CI/CD

Use MCP tools in CI pipelines:

```yaml
# .github/workflows/test.yml
- name: Start services
  run: dmn mcp &
  
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
if (status.services.backend === "Running") {
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
      "args": ["mcp"],
      "autoApprove": [
        "list_services",
        "get_service_status"
      ]
    }
  }
}
```

This allows AI to check status without prompting, while still requiring approval for reading logs.

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
