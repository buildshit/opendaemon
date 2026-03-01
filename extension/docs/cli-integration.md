# CLI Integration

OpenDaemon automatically makes the `dmn` command available in VS Code's integrated terminal, allowing you to manage services directly from the command line without any manual setup.

## Overview

When you install the OpenDaemon extension, the CLI is automatically configured for use in all VS Code terminals. You can immediately start using `dmn` commands without adding anything to your system PATH or running any installation scripts.

### Key Features

- **Zero Configuration**: Works immediately after extension installation
- **Automatic PATH Injection**: CLI is available in all new VS Code terminals
- **Cross-Platform**: Works on Windows, macOS, and Linux
- **No System Changes**: Only affects VS Code terminals, not your system PATH
- **Optional Global Install**: Can be installed system-wide if desired

## Quick Start

1. **Install the OpenDaemon extension** from the VS Code marketplace
2. **Open a new terminal** in VS Code (Terminal → New Terminal)
3. **Start using dmn commands** immediately:

```bash
# Check if CLI is available
dmn --version

# View help
dmn --help

# Check service status
dmn status

# Start all services
dmn start
```

That's it! No additional setup required.

## Available Commands

The `dmn` CLI provides several commands for managing your services:

### `dmn --version`

Display the current version of OpenDaemon.

```bash
dmn --version
```

**Output:**
```
dmn 1.0.0
```

---

### `dmn --help`

Display help information for all available commands.

```bash
# General help
dmn --help

# Help for specific command
dmn daemon --help
dmn mcp --help
dmn start --help
```

---

### `dmn status`

Show the current status of all configured services.

```bash
dmn status [service]
```

**Example Output:**
```
Service Status:
--------------------------------------------------
Controller: extension-daemon
database                       Running
backend-api                    Running  
frontend                       Running
worker                         Not Started
```

If no extension daemon is running, CLI falls back to local supervisor mode and the header shows `Supervisor: running/not running`.

**Status Values:**
- `Not Started` - Service hasn't been started yet
- `Starting` - Service is starting but not ready
- `Running` - Service is running and ready
- `Stopped` - Service was stopped gracefully
- `Failed (exit code: N)` - Service crashed with exit code N

---

### `dmn start`

Start services from your `dmn.json` configuration.

```bash
dmn start [service]
```

**Behavior:**
- Reads `dmn.json` from the current directory
- Routes to the active extension daemon first (same runtime as UI/Command Palette)
- Starts all services (or a single service + dependencies)
- Falls back to local supervisor mode only when no extension daemon is available

**Example:**
```bash
$ dmn start
Start requested via extension daemon.

$ dmn start frontend
Service 'frontend' start requested via extension daemon.
```

---

### `dmn stop`

Stop all running services, or stop a single service.

```bash
dmn stop [service]
```

**Behavior:**
- `dmn stop <service>` sends a targeted stop request to the active controller
- `dmn stop` stops all running services
- Controller preference: extension daemon first, local supervisor fallback

**Example:**
```bash
$ dmn stop frontend
Service 'frontend' stop requested via extension daemon.

$ dmn stop
Stop requested via extension daemon.
```

---

### `dmn restart`

Restart a single managed service.

```bash
dmn restart <service>
```

**Behavior:**
- Stops and starts the target service via the active controller
- Preserves dependency-aware orchestration behavior

---

### `dmn daemon`

Run OpenDaemon in daemon mode for VS Code extension communication.

```bash
dmn daemon
```

**Purpose:**
- Starts a JSON-RPC server over stdio
- Used by the VS Code extension for service management
- Handles start/stop/status requests from the extension UI

**Note:** This command is typically started automatically by the VS Code extension. You don't need to run it manually unless you're debugging extension communication.

---

### `dmn mcp`

Run OpenDaemon in MCP (Model Context Protocol) server mode for AI agent integration.

```bash
dmn mcp [--check]
```

**Purpose:**
- Exposes service logs and status to AI coding assistants
- Enables AI agents to debug issues by reading runtime data
- Provides tools: `read_logs`, `get_service_status`, `list_services`
- Supports `dmn mcp --check` to validate MCP config and exit

**When to use:**
- AI assistant integration (Kiro, Cursor, Claude Desktop)
- Automated debugging and log analysis
- Development workflow automation with AI

**See also:** [MCP Integration Guide](../../docs/MCP_INTEGRATION.md)

---

### Command Options

All commands support the `--config` option to specify a custom configuration file:

```bash
# Use custom config file
dmn start --config ./config/development.json
dmn start frontend --config ./config/development.json
dmn status --config /path/to/services.json
dmn stop -c ../shared/dmn.json
dmn restart backend-api -c ../shared/dmn.json
```

**Default:** If not specified, `dmn.json` in the current directory is used.

## Usage Examples

### Basic Development Workflow

```bash
# 1. Navigate to your project
cd /path/to/your/project

# 2. Check current service status
dmn status

# 3. Start all services
dmn start

# 4. Work on your project...

# 5. Stop all services when done
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

### Quick Status Check

```bash
# Check if services are running
dmn status

# Start services if needed
dmn start

# View logs in VS Code extension UI
```

### CI/CD Integration

```bash
#!/bin/bash
# test-script.sh

set -e

# Start services for testing
echo "Starting test services..."
dmn start --config ./test/dmn.json

# Wait for services to be ready
sleep 5

# Run tests
echo "Running tests..."
npm test

# Cleanup
echo "Stopping services..."
dmn stop --config ./test/dmn.json
```

## How It Works

### Automatic PATH Injection

When you install the OpenDaemon extension, it automatically configures PATH injection for all terminals:

1. **Detects your platform** (Windows, macOS, or Linux)
2. **Locates the correct binary** for your system
3. **Modifies workspace terminal settings** (`terminal.integrated.env.*`)
4. **Adds the binary directory** to the PATH environment variable

This configuration affects **all new terminals** in the workspace automatically. The extension uses VS Code's official `terminal.integrated.env.*` settings API, which is the recommended way to inject environment variables into terminals.

**What happens in your workspace:**
- The extension adds a PATH entry to `.vscode/settings.json`
- New terminals automatically inherit this PATH modification
- The original PATH is preserved using variable substitution (`${env:PATH}`)
- When the extension deactivates, the setting is restored

You don't need to do anything—it just works!

### Platform-Specific Binaries

The extension includes pre-built binaries for all supported platforms:

| Platform | Binary Name | Location |
|----------|-------------|----------|
| Windows x64 | `dmn-win32-x64.exe` | `extension/bin/` |
| macOS ARM64 | `dmn-darwin-arm64` | `extension/bin/` |
| macOS x64 | `dmn-darwin-x64` | `extension/bin/` |
| Linux x64 | `dmn-linux-x64` | `extension/bin/` |

The extension automatically selects the correct binary for your system.

### VS Code Terminal Only

**Important:** The automatic CLI integration only works in VS Code's integrated terminal. If you open a terminal outside of VS Code (like Terminal.app on macOS or PowerShell on Windows), the `dmn` command won't be available unless you install it globally.

## Global Installation (Optional)

If you want to use the `dmn` command in terminals outside of VS Code, you can install it globally on your system.

### Why Install Globally?

- Use `dmn` commands in any terminal application
- Run scripts that use `dmn` outside of VS Code
- Integrate with other development tools
- Use in CI/CD pipelines

### How to Install Globally

#### Option 1: Using the Command Palette

1. Open the Command Palette (`Ctrl+Shift+P` or `Cmd+Shift+P`)
2. Type "OpenDaemon: Install CLI Globally"
3. Follow the platform-specific instructions shown

#### Option 2: Manual Installation

**On Windows:**

1. Find the binary location:
   ```powershell
   # The extension shows this path in the notification
   # Example: C:\Users\YourName\.vscode\extensions\opendaemon-x.x.x\bin
   ```

2. Add to system PATH:
   - Open "System Properties" → "Environment Variables"
   - Edit the "Path" variable under "User variables"
   - Add the bin directory path
   - Click OK and restart your terminal

**On macOS/Linux:**

1. Copy the binary to a system location:
   ```bash
   # Find the extension path (shown in the notification)
   # Example: ~/.vscode/extensions/opendaemon-x.x.x/bin
   
   # Copy to /usr/local/bin
   sudo cp ~/.vscode/extensions/opendaemon-x.x.x/bin/dmn-darwin-arm64 /usr/local/bin/dmn
   
   # Make executable
   sudo chmod +x /usr/local/bin/dmn
   ```

2. Or add to your shell profile:
   ```bash
   # Add to ~/.bashrc, ~/.zshrc, or ~/.profile
   export PATH="$PATH:$HOME/.vscode/extensions/opendaemon-x.x.x/bin"
   ```

3. Reload your shell:
   ```bash
   source ~/.bashrc  # or ~/.zshrc
   ```

### Verify Global Installation

```bash
# Open a new terminal (outside VS Code)
dmn --version

# Should display version number
```

## Troubleshooting

### "Command not found: dmn"

**In VS Code Terminal:**

If you see this error in a VS Code terminal:

1. **Open a new terminal**: The CLI is only available in terminals created after the extension activates
   - Close the current terminal
   - Open a new one (Terminal → New Terminal)

2. **Check extension is installed**: Look for "OpenDaemon" in the Extensions panel

3. **Reload VS Code**: Use "Developer: Reload Window" from the Command Palette

4. **Check the Output panel**: Look for errors in the "OpenDaemon" output channel

**In External Terminal:**

If you see this error in a terminal outside VS Code:

- This is expected! The automatic CLI integration only works in VS Code terminals
- To use `dmn` outside VS Code, follow the [Global Installation](#global-installation-optional) instructions

---

### Binary Permission Errors (macOS/Linux)

If you see permission errors like "Permission denied" when running `dmn`:

**In VS Code Terminal:**

The extension should automatically fix permissions. If it doesn't:

1. Check the Output panel for error messages
2. Manually fix permissions:
   ```bash
   chmod +x ~/.vscode/extensions/opendaemon-*/bin/dmn-*
   ```

**After Global Installation:**

```bash
# Make the binary executable
sudo chmod +x /usr/local/bin/dmn
```

---

### Wrong Binary for Platform

If you see errors like "cannot execute binary file" or "bad CPU type":

1. **Check your platform**:
   ```bash
   # On macOS/Linux
   uname -m
   
   # On Windows
   echo %PROCESSOR_ARCHITECTURE%
   ```

2. **Verify the correct binary is being used**:
   - The extension should automatically select the right binary
   - Check the Output panel for platform detection messages

3. **Report the issue**: If the wrong binary is selected, please file a bug report with your platform details

---

### CLI Works in VS Code but Not Globally

This is expected behavior! The automatic CLI integration only affects VS Code terminals.

**Solution:** Follow the [Global Installation](#global-installation-optional) instructions to make `dmn` available system-wide.

---

### "No dmn.json file found"

If you see this error when running commands:

1. **Check current directory**: Ensure you're in a directory with a `dmn.json` file
   ```bash
   ls dmn.json
   ```

2. **Create a configuration**: Use the VS Code extension to create one
   - Command Palette → "OpenDaemon: Create Configuration"

3. **Use custom config path**:
   ```bash
   dmn start --config /path/to/dmn.json
   ```

---

### Service Commands Not Working

If `dmn start` or `dmn stop` don't work as expected:

1. **Check service status first**:
   ```bash
   dmn status
   ```

2. **Verify dmn.json syntax**: Ensure your configuration file is valid JSON

3. **Check service logs**: Use the VS Code extension to view detailed logs

4. **Try daemon mode**: The extension UI provides more detailed error messages
   - Use the OpenDaemon sidebar instead of CLI commands

---

### Extension Updates Break CLI

If the CLI stops working after updating the extension:

1. **Reload VS Code**: Use "Developer: Reload Window"

2. **Open a new terminal**: Close old terminals and open new ones

3. **Update global installation**: If you installed globally, update the binary:
   ```bash
   # Re-run the global installation steps with the new extension version
   ```

## Differences: VS Code Integration vs Global Installation

| Feature | VS Code Terminal | Global Installation |
|---------|------------------|---------------------|
| **Setup** | Automatic | Manual |
| **Availability** | VS Code terminals only | All terminals |
| **Updates** | Automatic with extension | Manual |
| **PATH Changes** | None (terminal-only) | System PATH modified |
| **Use Cases** | Development in VS Code | Scripts, CI/CD, other tools |

### When to Use Each

**Use VS Code Integration (default):**
- You primarily work in VS Code
- You want zero-configuration setup
- You don't need CLI access outside VS Code

**Use Global Installation:**
- You use multiple terminal applications
- You have scripts that call `dmn` commands
- You need CLI access in CI/CD pipelines
- You want to use `dmn` with other development tools

## Best Practices

### 1. Use VS Code Terminals for Development

For day-to-day development, use VS Code's integrated terminal:
- Automatic CLI availability
- Integrated with extension features
- Better log viewing and debugging

### 2. Use Extension UI for Complex Operations

For starting/stopping services and viewing logs, the extension UI is often more convenient:
- Visual service status
- Real-time log streaming
- Right-click context menus
- Dependency visualization

### 3. Use CLI for Scripting

For automation and scripting, use CLI commands:
- Easier to script and automate
- Better for CI/CD integration
- Can be used in shell scripts

### 4. Keep Configuration in Version Control

Always commit your `dmn.json` to version control:
```bash
git add dmn.json
git commit -m "Add OpenDaemon configuration"
```

This ensures all team members have the same service setup.

### 5. Use Custom Configs for Different Environments

Create separate config files for different environments:
```
project/
├── dmn.json              # Default (development)
├── dmn.staging.json      # Staging environment
└── dmn.production.json   # Production-like local setup
```

Then use the `--config` flag to switch between them.

## See Also

- **[DMN Terminal Commands Reference](../../DMN_TERMINAL_COMMANDS.md)** - Complete CLI command reference
- **[Configuration Schema](../../docs/DMN_JSON_SCHEMA.md)** - dmn.json configuration guide
- **[MCP Integration](../../docs/MCP_INTEGRATION.md)** - AI assistant integration
- **[Terminal Integration](./terminal-integration.md)** - Real-time log streaming in terminals
- **[README](../../README.md)** - Quick start and overview

## Support

If you encounter issues with CLI integration:

1. **Check the Output panel**: Look for errors in "OpenDaemon" output channel
2. **Check extension logs**: Enable verbose logging in settings
3. **File an issue**: [GitHub Issues](https://github.com/opendaemon/dmn/issues)
4. **Ask for help**: [Discord community](https://discord.gg/opendaemon)

---

**OpenDaemon** - Making local development service management effortless  
For more information, visit: [opendaemon.com](https://opendaemon.com)
