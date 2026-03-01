# DMN Terminal Commands Reference

Complete reference for all terminal commands available in the OpenDaemon (dmn) system.

## Table of Contents

- [Overview](#overview)
- [Installation & Setup](#installation--setup)
- [Core Commands](#core-commands)
- [Command Options](#command-options)
- [Usage Examples](#usage-examples)
- [Environment Variables](#environment-variables)
- [Exit Codes](#exit-codes)
- [Troubleshooting](#troubleshooting)

## Overview

OpenDaemon (`dmn`) is a command-line tool for orchestrating local development services. It provides several modes of operation:

- **Daemon Mode**: JSON-RPC server for VS Code extension integration
- **MCP Mode**: Model Context Protocol server for AI agent integration  
- **Direct Commands**: Start/stop/restart/status with shared daemon routing
- **Interactive Mode**: Via VS Code extension UI

## Confirmed Working Snapshot (2026-03-01)

Validated from a real workspace terminal session:

- `dmn start frontend` → `Service 'frontend' start requested via extension daemon.`
- `dmn stop` → `Stop requested via extension daemon.`
- `dmn start` → `Start requested via extension daemon.`
- `dmn start database` → `Service 'database' start requested via extension daemon.`
- `dmn restart database` → `Restart requested via extension daemon.`
- `dmn status` → header shows `Controller: extension-daemon` and live service states

## Installation & Setup

### Prerequisites

- Rust toolchain (for building from source)
- VS Code (for extension integration)
- Services defined in `dmn.json` configuration file

### Installation

```bash
# Install from source (if building locally)
cargo install --path .

# Or use the pre-built binary from releases
# Download dmn.exe (Windows) or dmn (Linux/macOS)
```

### Verify Installation

```bash
dmn --version
```

## Core Commands

### `dmn daemon`

Run in daemon mode for VS Code extension communication.

```bash
dmn daemon [OPTIONS]
```

**Purpose**: 
- Starts a JSON-RPC server over stdio
- Used by the VS Code extension for service management
- Handles start/stop/status requests from the extension UI

**Options**:
- `-c, --config <PATH>` - Path to dmn.json configuration file (default: "dmn.json")
- `--check` - Validate MCP configuration and exit without starting stdio server

**Example**:
```bash
# Start daemon with default config
dmn daemon

# Start daemon with custom config
dmn daemon --config ./config/services.json
```

**When to use**:
- Automatically started by VS Code extension
- Manual use for debugging extension communication
- Integration with other tools that need JSON-RPC interface

---

### `dmn mcp`

Run in MCP (Model Context Protocol) server mode for AI agent integration.

```bash
dmn mcp [OPTIONS]
```

**Purpose**:
- Exposes service logs and status to AI coding assistants
- Enables AI agents to debug issues by reading actual runtime data
- Provides tools: `read_logs`, `get_service_status`, `list_services`

**Options**:
- `-c, --config <PATH>` - Path to dmn.json configuration file (default: "dmn.json")

**Example**:
```bash
# Start MCP server with default config
dmn mcp

# Start MCP server with custom config  
dmn mcp --config ./dev/dmn.json

# Validate MCP configuration only
dmn mcp --check
```

**When to use**:
- AI assistant integration (Kiro, Cursor, Claude Desktop)
- Automated debugging and log analysis
- Development workflow automation with AI

**AI Integration Setup**:
```json
// .kiro/settings/mcp.json
{
  "mcpServers": {
    "opendaemon": {
      "command": "dmn",
      "args": ["mcp"],
      "disabled": false
    }
  }
}
```

---

### `dmn start`

Start all services or one service (with dependencies).

```bash
dmn start [SERVICE] [OPTIONS]
```

**Purpose**:
- Prefer controlling the active extension daemon (same runtime as UI/Command Palette)
- Start all services or one target service + dependencies
- Fall back to local foreground supervisor mode when no daemon is available

**Options**:
- `-c, --config <PATH>` - Path to dmn.json configuration file (default: "dmn.json")

**Example**:
```bash
# Start all services
dmn start

# Start only frontend (auto-starts dependencies)
dmn start frontend

# Start all with custom config
dmn start --config ./config/dev.json
```

**Behavior**:
- With an active extension daemon:
  - Sends `startAll` / `startService` RPC to the daemon
  - Returns immediately after request acceptance
- Without an extension daemon:
  - Starts local supervisor in current terminal
  - Keeps running in foreground until `dmn stop`/Ctrl+C

**Exit Codes**:
- `0` - Start request accepted or supervisor started
- `1` - Configuration error, invalid service, or control request failure

---

### `dmn stop`

Stop one service or stop all services.

```bash
dmn stop [SERVICE] [OPTIONS]
```

**Purpose**:
- Stop a single managed service by name
- Or stop all running services

**Options**:
- `-c, --config <PATH>` - Path to dmn.json configuration file (default: "dmn.json")

**Example**:
```bash
# Stop one service
dmn stop frontend

# Stop all services
dmn stop

# Stop with custom config
dmn stop --config ./dev/dmn.json
```

**Behavior**:
- With an active extension daemon:
  - Sends `stopService` / `stopAll` RPC to the daemon
- Without an extension daemon:
  - Uses local supervisor control path (`runtime-control.json`)

**Exit Codes**:
- `0` - Stop request processed successfully
- `1` - Stop request failed or no matching active controller/config

---

### `dmn restart`

Restart a single managed service.

```bash
dmn restart <SERVICE> [OPTIONS]
```

**Purpose**:
- Stop and re-start one managed service
- Preserve dependency-aware behavior from orchestrator logic

**Options**:
- `-c, --config <PATH>` - Path to dmn.json configuration file (default: "dmn.json")

**Example**:
```bash
dmn restart backend-api
dmn restart frontend --config ./config/dev.json
```

**Exit Codes**:
- `0` - Restart request accepted
- `1` - No active controller, invalid service, or request failure

---

### `dmn status`

Show the current status of all services.

```bash
dmn status [SERVICE] [OPTIONS]
```

**Purpose**:
- Display current state from the active service controller
- Show either all services or one specific service
- Useful for debugging and monitoring

**Options**:
- `-c, --config <PATH>` - Path to dmn.json configuration file (default: "dmn.json")

**Example**:
```bash
# Check status of all services
dmn status

# Check one service
dmn status backend-api

# Check status with custom config
dmn status --config ./staging/dmn.json
```

**Output Format**:
```
Service Status:
Controller: extension-daemon
database                       Running
backend-api                    Running  
frontend                       Failed (exit code: 1)
worker                         Not Started
redis                          Stopped
```

When no extension daemon is active, status falls back to the local supervisor output (`Supervisor: running/not running`).

**Status Values**:
- `Not Started` - Service hasn't been started yet
- `Starting` - Service is starting but not ready
- `Running` - Service is running and ready
- `Stopped` - Service was stopped gracefully
- `Failed (exit code: N)` - Service crashed with exit code N

**Exit Codes**:
- `0` - Status retrieved successfully
- `1` - Configuration error

---

### `dmn --help`

Display help information for all commands.

```bash
dmn --help
dmn <COMMAND> --help
```

**Examples**:
```bash
# General help
dmn --help

# Help for specific command
dmn daemon --help
dmn mcp --help
dmn start --help
```

---

### `dmn --version`

Display version information.

```bash
dmn --version
```

**Alias:** `dmn -V`  
**Note:** lowercase `dmn -v` is not a valid flag.

**Output**:
```
dmn 1.0.0
```

## Command Options

### Global Options

These options are available for all commands:

#### `--config` / `-c`

Specify path to configuration file.

- **Type**: Path
- **Default**: `dmn.json`
- **Description**: Path to the dmn.json configuration file

**Examples**:
```bash
dmn start --config ./config/development.json
dmn daemon -c /path/to/services.json
dmn mcp --config ../shared/dmn.json
```

**Path Resolution**:
- Relative paths are resolved from current working directory
- Absolute paths are used as-is
- File must exist and be readable
- Must be valid JSON matching dmn.json schema

### Command-Specific Behavior

#### Configuration Loading

All commands load and validate the configuration file before executing:

1. **File Discovery**: Look for config file at specified path
2. **JSON Parsing**: Parse and validate JSON syntax
3. **Schema Validation**: Validate against dmn.json schema
4. **Dependency Analysis**: Build service dependency graph
5. **Error Reporting**: Report any configuration issues

#### Error Handling

Commands provide detailed error messages for common issues:

```bash
# Missing config file
$ dmn start --config missing.json
Failed to load configuration: No such file or directory (os error 2)

# Invalid JSON
$ dmn start --config invalid.json  
Failed to load configuration: expected `,` or `}` at line 5 column 10

# Circular dependency
$ dmn start
Failed to create orchestrator: Circular dependency detected: api → database → api
```

## Usage Examples

### Basic Development Workflow

```bash
# 1. Create configuration
cat > dmn.json << EOF
{
  "version": "1.0",
  "services": {
    "database": {
      "command": "docker run --rm -p 5432:5432 -e POSTGRES_PASSWORD=dev postgres:15",
      "ready_when": {
        "log_contains": "database system is ready to accept connections"
      }
    },
    "backend": {
      "command": "npm run dev",
      "depends_on": ["database"],
      "ready_when": {
        "url_responds": "http://localhost:3000/health"
      }
    }
  }
}
EOF

# 2. Start all services
dmn start

# 3. Check status
dmn status

# 4. Stop all services
dmn stop
```

### Multi-Environment Setup

```bash
# Development environment
dmn start --config ./config/dev.json

# Staging environment  
dmn start --config ./config/staging.json

# Production-like environment
dmn start --config ./config/prod-local.json
```

### AI Integration Workflow

```bash
# 1. Start MCP server for AI integration
dmn mcp &

# 2. Configure AI assistant (Kiro example)
mkdir -p .kiro/settings
cat > .kiro/settings/mcp.json << EOF
{
  "mcpServers": {
    "opendaemon": {
      "command": "dmn",
      "args": ["mcp"],
      "disabled": false,
      "autoApprove": ["list_services", "get_service_status"]
    }
  }
}
EOF

# 3. Now AI can help debug:
# "What services are running?"
# "Show me the backend logs"
# "Why is the frontend failing?"
```

### Debugging Service Issues

```bash
# 1. Check overall status
dmn status

# 2. If using VS Code extension, check logs in Output panel
# 3. For command-line debugging, use MCP mode:
dmn mcp

# Then use MCP client or AI to read logs:
# read_logs(service="backend", lines=50)
```

### CI/CD Integration

```bash
#!/bin/bash
# ci-test.sh

set -e

# Start services for testing
echo "Starting test services..."
dmn start --config ./test/dmn.json

# Wait for services to be ready (status command returns 0 when ready)
echo "Waiting for services..."
timeout 120 bash -c 'until dmn status --config ./test/dmn.json | grep -q "Running"; do sleep 1; done'

# Run tests
echo "Running tests..."
npm test

# Cleanup
echo "Stopping services..."
dmn stop --config ./test/dmn.json
```

### Docker Integration

```bash
# Using dmn with Docker Compose
cat > dmn.json << EOF
{
  "version": "1.0", 
  "services": {
    "infrastructure": {
      "command": "docker-compose up",
      "ready_when": {
        "log_contains": "Started"
      }
    },
    "app": {
      "command": "npm start",
      "depends_on": ["infrastructure"],
      "ready_when": {
        "url_responds": "http://localhost:3000"
      }
    }
  }
}
EOF

dmn start
```

## Environment Variables

### Configuration

#### `DMN_LOG_BUFFER_SIZE`

Set the log buffer size for each service (MCP mode).

- **Type**: Integer
- **Default**: 1000
- **Description**: Number of log lines to keep in memory per service

```bash
export DMN_LOG_BUFFER_SIZE=5000
dmn mcp
```

#### `DMN_READY_TIMEOUT`

Set the default ready timeout for services.

- **Type**: Integer (seconds)
- **Default**: 60
- **Description**: Default timeout for ready conditions

```bash
export DMN_READY_TIMEOUT=120
dmn start
```

### Runtime Environment

Services inherit the environment from the dmn process, plus any variables from `env_file` configurations.

**Example**:
```bash
# Set environment for all services
export DATABASE_URL=postgresql://localhost:5432/dev
export DEBUG=true

# Start services (they inherit these variables)
dmn start
```

**Service-specific environment**:
```json
{
  "services": {
    "backend": {
      "command": "npm start",
      "env_file": ".env.backend"
    }
  }
}
```

## Exit Codes

All dmn commands use standard exit codes:

### Success Codes

- **0** - Command completed successfully

### Error Codes

- **1** - General error (configuration, startup, or runtime failure)

### Specific Error Scenarios

#### Configuration Errors (Exit Code 1)

```bash
# Missing config file
$ dmn start --config missing.json
# Exit code: 1

# Invalid JSON syntax
$ dmn start --config invalid.json  
# Exit code: 1

# Schema validation failure
$ dmn start --config bad-schema.json
# Exit code: 1
```

#### Service Errors (Exit Code 1)

```bash
# Service startup failure
$ dmn start
# Exit code: 1 (if any service fails to start)

# Timeout waiting for ready condition
$ dmn start
# Exit code: 1 (if service doesn't become ready in time)

# Circular dependency
$ dmn start
# Exit code: 1 (if dependency cycle detected)
```

#### Usage in Scripts

```bash
#!/bin/bash

# Start services and check for success
if dmn start; then
    echo "Services started successfully"
    # Run tests or other commands
    npm test
    dmn stop
else
    echo "Failed to start services" >&2
    exit 1
fi
```

## Troubleshooting

### Common Issues

#### "No dmn.json file found"

**Problem**: Configuration file not found.

**Solutions**:
```bash
# Check current directory
ls dmn.json

# Create minimal config
cat > dmn.json << EOF
{
  "version": "1.0",
  "services": {
    "app": {
      "command": "npm start"
    }
  }
}
EOF

# Use custom path
dmn start --config ./config/services.json
```

#### "Command not found: dmn"

**Problem**: dmn binary not in PATH.

**Solutions**:
```bash
# Check if binary exists
which dmn

# Add to PATH (if installed locally)
export PATH="$PATH:/path/to/dmn/binary"

# Use full path
/full/path/to/dmn start

# Install from source
cargo install --path .
```

#### "Service timeout" errors

**Problem**: Service doesn't become ready within timeout.

**Solutions**:
```bash
# Check service logs (via VS Code extension or MCP)
dmn mcp
# Then use AI or MCP client to read logs

# Increase timeout in config
{
  "ready_when": {
    "log_contains": "ready",
    "timeout_seconds": 120
  }
}

# Simplify ready condition
{
  "ready_when": {
    "log_contains": "started"
  }
}
```

#### "Circular dependency detected"

**Problem**: Services depend on each other in a loop.

**Solution**:
```bash
# Error shows the cycle:
# "Circular dependency detected: a → b → c → a"

# Fix by removing one dependency or restructuring
# Review your depends_on fields in dmn.json
```

### Debug Mode

For detailed debugging, check the stderr output:

```bash
# Daemon mode shows debug info on stderr
dmn daemon 2> debug.log

# MCP mode shows startup info
dmn mcp 2> mcp-debug.log

# Direct commands show progress
dmn start 2> start-debug.log
```

### Getting Help

1. **Check command help**: `dmn <command> --help`
2. **Validate configuration**: Use JSON schema validation
3. **Test services manually**: Run service commands directly
4. **Check VS Code Output panel**: Look for detailed error messages
5. **File issues**: Report bugs with configuration and error messages

### Performance Tips

#### Optimize Service Startup

```json
{
  "services": {
    "fast-service": {
      "command": "redis-server",
      "ready_when": {
        "log_contains": "Ready to accept connections",
        "timeout_seconds": 30
      }
    },
    "slow-service": {
      "command": "docker run postgres",
      "ready_when": {
        "log_contains": "database system is ready",
        "timeout_seconds": 180
      }
    }
  }
}
```

#### Parallel Startup

Services without dependencies start in parallel automatically:

```json
{
  "services": {
    "redis": {
      "command": "redis-server"
    },
    "postgres": {
      "command": "postgres"
    },
    "app": {
      "command": "npm start",
      "depends_on": ["redis", "postgres"]
    }
  }
}
```

Redis and Postgres start simultaneously, then app starts when both are ready.

## See Also

- [README.md](README.md) - Quick start guide and overview
- [docs/DMN_JSON_SCHEMA.md](docs/DMN_JSON_SCHEMA.md) - Configuration file reference
- [docs/MCP_INTEGRATION.md](docs/MCP_INTEGRATION.md) - AI agent integration guide
- [docs/MCP_QUICK_START.md](docs/MCP_QUICK_START.md) - 5-minute MCP setup
- [docs/MCP_TROUBLESHOOTING.md](docs/MCP_TROUBLESHOOTING.md) - MCP-specific issues

---

**OpenDaemon (dmn)** - Local development service orchestrator  
For support, visit: [GitHub Issues](https://github.com/opendaemon/dmn/issues)