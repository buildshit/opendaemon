# OpenDaemon (dmn)

A VS Code extension and Rust-based orchestrator for managing local development services with smart dependency ordering and AI-powered log analysis.

## Overview

OpenDaemon (`dmn`) helps you manage multiple development services (databases, backend servers, frontend dev servers, etc.) through a single declarative configuration file. It handles dependency ordering, waits for services to be ready, and provides integrated log viewing—all from within VS Code.

### Key Features

- **Declarative Configuration**: Define all your services in a single `dmn.json` file
- **Smart Dependency Management**: Services start in the correct order based on dependencies
- **Ready Detection**: Wait for services to be truly ready before starting dependents
- **Integrated Logs**: View real-time logs from all services in VS Code
- **AI Integration**: MCP server allows AI agents to read service logs for debugging
- **Cross-Platform**: Works on Windows, macOS, and Linux

## Quick Start

### Installation

1. Install the OpenDaemon extension from the VS Code marketplace
2. Open a workspace/folder in VS Code
3. Create a `dmn.json` file in your workspace root (or use the wizard)

### Creating Your First Configuration

Create a `dmn.json` file in your workspace root:

```json
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
      },
      "env_file": ".env.local"
    },
    "frontend": {
      "command": "npm run dev --prefix ./frontend",
      "depends_on": ["backend"],
      "ready_when": {
        "log_contains": "Local:.*http://localhost:5173"
      }
    }
  }
}
```

### Basic Usage

#### Using the VS Code Extension

1. Open the OpenDaemon sidebar (look for the daemon icon)
2. Click "Start All" to start all services in dependency order
3. Click on any service to view its logs
4. Right-click a service for options (Start, Stop, Restart)
5. Click "Stop All" to gracefully shut down all services

#### Using the CLI

```bash
# Start the daemon (usually done automatically by the extension)
dmn daemon

# Start in MCP mode for AI agent integration
dmn mcp
```

## Configuration Reference

### Basic Structure

```json
{
  "version": "1.0",
  "services": {
    "service-name": {
      "command": "command to run",
      "depends_on": ["other-service"],
      "ready_when": { /* readiness condition */ },
      "env_file": ".env"
    }
  }
}
```

### Service Configuration Options

#### `command` (required)
The command to execute for this service.

```json
{
  "command": "npm run dev"
}
```

#### `depends_on` (optional)
Array of service names that must be ready before this service starts.

```json
{
  "depends_on": ["database", "redis"]
}
```

#### `ready_when` (optional)
Condition to determine when the service is ready. If omitted, the service is considered ready immediately after starting.

**Log Pattern Matching:**
```json
{
  "ready_when": {
    "log_contains": "Server listening on port 3000"
  }
}
```

Supports regex patterns:
```json
{
  "ready_when": {
    "log_contains": "Listening on.*:\\d+"
  }
}
```

**URL Health Check:**
```json
{
  "ready_when": {
    "url_responds": "http://localhost:3000/health"
  }
}
```

**Custom Timeout:**

For services that take longer to start, you can specify a custom timeout (in seconds):

```json
{
  "ready_when": {
    "log_contains": "Server listening on port 3000",
    "timeout_seconds": 120
  }
}
```

Or with URL health checks:

```json
{
  "ready_when": {
    "url_responds": "http://localhost:3000/health",
    "timeout_seconds": 90
  }
}
```

The default timeout is 60 seconds if not specified.

#### `env_file` (optional)
Path to an environment file to load for this service.

```json
{
  "env_file": ".env.local"
}
```

## Common Scenarios

### Database + Backend + Frontend

```json
{
  "version": "1.0",
  "services": {
    "postgres": {
      "command": "docker run --rm -p 5432:5432 -e POSTGRES_PASSWORD=dev postgres:15",
      "ready_when": {
        "log_contains": "database system is ready to accept connections"
      }
    },
    "api": {
      "command": "cargo run --bin api-server",
      "depends_on": ["postgres"],
      "ready_when": {
        "log_contains": "Listening on 0.0.0.0:8080"
      },
      "env_file": ".env"
    },
    "web": {
      "command": "npm run dev",
      "depends_on": ["api"],
      "ready_when": {
        "url_responds": "http://localhost:5173"
      }
    }
  }
}
```

### Microservices with Shared Dependencies

```json
{
  "version": "1.0",
  "services": {
    "redis": {
      "command": "redis-server",
      "ready_when": {
        "log_contains": "Ready to accept connections"
      }
    },
    "auth-service": {
      "command": "node services/auth/index.js",
      "depends_on": ["redis"],
      "ready_when": {
        "url_responds": "http://localhost:3001/health"
      }
    },
    "user-service": {
      "command": "node services/users/index.js",
      "depends_on": ["redis", "auth-service"],
      "ready_when": {
        "url_responds": "http://localhost:3002/health"
      }
    },
    "api-gateway": {
      "command": "node gateway/index.js",
      "depends_on": ["auth-service", "user-service"],
      "ready_when": {
        "log_contains": "Gateway listening on port 8080"
      }
    }
  }
}
```

### Docker Compose Integration

```json
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
```

## AI Integration (MCP)

OpenDaemon includes an MCP (Model Context Protocol) server that allows AI coding assistants to read service logs and help debug issues.

### Quick Setup

1. **Test the MCP server:**
   ```bash
   dmn mcp
   ```

2. **Configure your AI assistant** (Kiro, Cursor, Claude Desktop)

3. **Ask your AI:** "What services are configured in OpenDaemon?"

**📚 Guides:**
- **[MCP Quick Start](docs/MCP_QUICK_START.md)** - Get started in 5 minutes
- **[MCP Integration Guide](docs/MCP_INTEGRATION.md)** - Complete technical reference

### What Your AI Can Do

With MCP integration, your AI assistant can:
- **Debug issues** by reading actual service logs
- **Check service status** and dependencies  
- **Analyze error patterns** across multiple services
- **Suggest fixes** based on real runtime data

Example conversation:
```
You: "My backend is failing, can you help?"
AI: [Checks service status and reads logs]
    "The backend logs show a database connection error. 
     Let me check if the database service is running..."
```

## Troubleshooting

### No Services Found

If you see "No services found" when trying to start services:

1. **Check if dmn.json exists**: Ensure you have a `dmn.json` file in your workspace root
   - Use the command palette: "OpenDaemon: Create Configuration" to create one
   - Or manually create the file with at least one service defined

2. **Verify services are defined**: Open your `dmn.json` and ensure the `services` object is not empty:
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

3. **Check JSON syntax**: Ensure your `dmn.json` is valid JSON (no trailing commas, proper quotes)

4. **Reload the window**: Try reloading VS Code with "Developer: Reload Window" from the command palette

5. **Check the Output panel**: Look for error messages in the "OpenDaemon" output channel

### Service Timeout Errors

If services are timing out before they're ready:

1. **Increase the timeout**: Add a `timeout_seconds` field to your ready condition:
   ```json
   {
     "ready_when": {
       "log_contains": "Server ready",
       "timeout_seconds": 120
     }
   }
   ```

2. **Verify the ready condition**: Check that your log pattern or URL is correct
   - View the service logs in the Output panel to see what's actually being logged
   - Test your regex pattern at regex101.com
   - For URL checks, verify the URL is accessible from your machine

3. **Check service logs**: When a timeout occurs, the error message includes the last few log lines
   - Look for error messages or unexpected output
   - Ensure the service is actually starting (not failing immediately)

4. **Simplify the pattern**: Try a simpler ready condition first:
   ```json
   {
     "ready_when": {
       "log_contains": "ready"
     }
   }
   ```

5. **Common timeout values**:
   - Fast services (Redis, simple scripts): 30-60 seconds (default)
   - Medium services (Node.js apps, Python servers): 60-90 seconds
   - Slow services (databases, Docker containers): 90-180 seconds
   - Very slow services (large builds, complex initialization): 180+ seconds

### Services Won't Start

1. Check the logs in the output panel
2. Verify your `dmn.json` syntax is correct
3. Ensure commands are executable from your workspace directory
4. Check for circular dependencies

### Ready Condition Never Triggers

1. Verify the log pattern or URL is correct
2. Check the service logs to see what's actually being output
3. Try using a simpler pattern first
4. Increase timeout using `timeout_seconds` if needed

### Extension Not Detecting dmn.json

1. Ensure the file is in your workspace root
2. Try reloading the VS Code window
3. Check the file is named exactly `dmn.json`

## Configuration Schema

For detailed schema documentation including all available options and validation rules, see [docs/DMN_JSON_SCHEMA.md](docs/DMN_JSON_SCHEMA.md).

## Contributing

Contributions are welcome! Please see [CONTRIBUTING.md](CONTRIBUTING.md) for guidelines.

## License

MIT License - see [LICENSE](LICENSE) for details.

## Support

- GitHub Issues: [Report bugs or request features](https://github.com/opendaemon/dmn/issues)
- Documentation: [Full documentation](https://opendaemon.com/docs)
- Community: [Discord server](https://discord.gg/opendaemon)
