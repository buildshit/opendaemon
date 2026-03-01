# OpenDaemon VS Code Extension

Orchestrate local development services with declarative configuration.

## Features

- **Service Management**: Manage multiple services from a single `dmn.json` configuration
- **Dependency-Aware Startup**: Services start in the correct order based on dependencies
- **Real-Time Log Streaming**: View logs from any service in the output panel
- **Service Status Monitoring**: Visual tree view showing service status with icons
- **AI Agent Integration**: Works with MCP clients through manual `mcp.json` configuration
- **Configuration Wizard**: Automatically detect and suggest services from package.json and docker-compose.yml
- **File Watching**: Automatically reload when dmn.json changes

## Project Structure

```
extension/
├── src/
│   ├── extension.ts       # Main extension entry point
│   ├── daemon.ts          # Daemon process manager
│   ├── rpc-client.ts      # JSON-RPC client for daemon communication
│   ├── tree-view.ts       # Service tree view provider
│   ├── commands.ts        # Command handlers (start/stop/restart)
│   ├── logs.ts            # Log output panel manager
│   ├── wizard.ts          # Configuration creation wizard
│   ├── file-watcher.ts    # dmn.json file watcher
│   └── test/              # Test suites
├── package.json           # Extension manifest
├── tsconfig.json          # TypeScript configuration
└── README.md              # This file
```

## Usage

1. Create a `dmn.json` file in your workspace root (or use the wizard)
2. Define your services with commands and dependencies
3. Use the OpenDaemon sidebar to start/stop services
4. View logs in the output panel
5. Right-click services for context menu actions
6. Configure MCP manually using the snippets in `../docs/MCP_QUICK_START.md`
7. Validate MCP control flow by asking your AI to start and restart `frontend`

## Configuration Example

```json
{
  "version": "1.0",
  "services": {
    "database": {
      "command": "docker-compose up postgres",
      "depends_on": [],
      "ready_when": {
        "log_contains": "database system is ready"
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
```

## Development

### Build

```bash
npm install
npm run compile
```

### Test

```bash
npm test
```

### Watch Mode

```bash
npm run watch
```

## Commands

- `OpenDaemon: Start All Services` - Start all services in dependency order
- `OpenDaemon: Stop All Services` - Stop all services
- `OpenDaemon: Start Service` - Start a specific service
- `OpenDaemon: Stop Service` - Stop a specific service
- `OpenDaemon: Restart Service` - Restart a specific service
- `OpenDaemon: Show Logs` - Show logs for a service
- `OpenDaemon: Show CLI Logs` - Open CLI integration/debug output

## Requirements

- VS Code 1.85.0 or higher
- OpenDaemon Rust binary (bundled with extension)

## See Also

- Main OpenDaemon documentation for `dmn.json` schema details
- MCP integration guide for AI agent usage

