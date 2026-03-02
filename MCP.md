# OpenDaemon MCP Guide

OpenDaemon can run as an MCP stdio server with `dmn mcp`, allowing AI clients to inspect logs/status and control services.

## Quick Setup

1. Validate your config:

```bash
dmn mcp --check --config /absolute/path/to/dmn.json
```

2. Add OpenDaemon to your MCP client config.
3. Restart your IDE/client.
4. Ask the AI to list services or check status.

## Cursor/Kiro Config Example

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

Use an absolute `--config` path to avoid working-directory issues.

## Available MCP Tools

| Tool | Access | Purpose |
| --- | --- | --- |
| `list_services` | Read | List configured services |
| `get_service_status` | Read | Get current service statuses |
| `read_logs` | Read | Read buffered logs for one service |
| `watch_logs` | Read | Stream logs with duration/pattern stop conditions |
| `start_service` | Write | Start one service (+ dependencies) |
| `stop_service` | Write | Stop one running service |
| `restart_service` | Write | Restart one service |

### Key Parameters

- `read_logs`: `service`, `lines` (`number` or `"all"`), optional `contains`, `caseSensitive`, `stream` (`stdout|stderr|both`)
- `watch_logs`: `service`, plus either `durationSeconds` or `untilPattern`; optional `timeoutSeconds`, `pollIntervalMs`, `maxLines`, `includeExisting`, `includePatterns`, `excludePatterns`, `caseSensitive`, `stream`
- `start_service` / `stop_service` / `restart_service`: `service`

## Runtime Routing Behavior

- If an extension daemon is active for the same config path, MCP tool calls reuse that runtime.
- Otherwise, MCP uses its own orchestrator runtime for the loaded config.

## Practical Validation Flow

Ask your AI:

1. "List my OpenDaemon services."
2. "Start the frontend service."
3. "Check status until frontend is running."
4. "Restart frontend and show recent frontend logs."

## Troubleshooting

- `dmn` not found: use an absolute binary path in MCP config.
- No services returned: verify `--config` points to the right `dmn.json`.
- Service action appears to do nothing: check if service is already running, then test with `restart_service`.
- Extension UI and MCP disagree: confirm both target the same absolute config path.

## Related Docs

- [Project README](README.md)
- [CLI Guide](CLI.md)
- [Extension Guide](extension/README.md)
