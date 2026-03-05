# OpenDaemon CLI Guide

The `dmn` CLI manages services defined in `dmn.json`.

Runtime behavior:

- If an extension daemon is running for the same config, CLI commands route to it.
- Otherwise, commands use the local CLI supervisor runtime.

## Command Reference

| Command | Purpose |
| --- | --- |
| `dmn --help` | Show command help |
| `dmn --version` | Show CLI version |
| `dmn start [service]` | Start all services, or one service with dependencies |
| `dmn stop [service]` | Stop all services, or one service |
| `dmn restart <service>` | Restart one service |
| `dmn status [service]` | Show status for all services, or one service |
| `dmn daemon` | Run daemon mode (extension RPC server) |
| `dmn mcp` | Run MCP server mode |
| `dmn mcp --check` | Validate MCP setup and config, then exit |

All service commands support `-c, --config <path>` and default to `dmn.json`.

## Common Usage

```bash
# Start everything
dmn start

# Start one service + dependencies
dmn start frontend

# Check all statuses
dmn status

# Check one service status
dmn status frontend

# Restart one service
dmn restart frontend

# Stop everything
dmn stop
```

## Status Values

- `Not Started`
- `Starting`
- `Running`
- `Stopped`
- `Failed (exit code: N)`

## Config Path Best Practice

Use an absolute config path when commands run outside your workspace shell:

```bash
dmn start --config /absolute/path/to/dmn.json
```

## Troubleshooting

- `No dmn.json found`: pass `--config` or run in the correct folder.
- Command not routing to extension daemon: confirm extension is open on the same workspace/config.
- Service stuck in `Starting`: verify `ready_when` pattern/URL and timeout values.
- On Windows, `stop` now force-terminates process trees to prevent wrapper child processes from lingering and keeping ports busy.

## Related Docs

- [Project README](README.md)
- [Extension Guide](extension/README.md)
- [MCP Guide](MCP.md)
