# OpenDaemon (`dmn`)

OpenDaemon orchestrates local development services from a single `dmn.json` file. It provides one shared runtime across the CLI, VS Code extension, and MCP tools so service state and logs stay in sync.

## Documentation

- [CLI Guide](CLI.md)
- [Extension Guide](extension/README.md)
- [MCP Guide](MCP.md)
- [Packaging & Publishing](PACKAGING.md)

## Core Capabilities

- Dependency-aware service startup and shutdown
- Readiness checks with log patterns or URL probes
- Optional per-service `.env` loading via `env_file`
- Real-time status and logs from CLI, extension, and MCP
- Extension-daemon first routing, with local supervisor fallback

## Quick Start

1. Create a `dmn.json` file in your workspace root.
2. Start services with `dmn start`.
3. Check runtime state with `dmn status`.
4. Stop services with `dmn stop`.

### Example `dmn.json`

```json
{
  "version": "1.0",
  "services": {
    "database": {
      "command": "docker run --rm -p 5432:5432 -e POSTGRES_PASSWORD=dev postgres:15",
      "ready_when": {
        "type": "log_contains",
        "pattern": "database system is ready to accept connections",
        "timeout_seconds": 120
      }
    },
    "backend": {
      "command": "npm run dev",
      "depends_on": ["database"],
      "ready_when": {
        "type": "url_responds",
        "url": "http://localhost:3000/health"
      },
      "env_file": ".env.local"
    }
  }
}
```

`ready_when` also supports legacy keys (`log_contains`, `url_responds`) for backward compatibility.

## Build from Source

### Core binary

```bash
cd core
cargo build --release
```

### VS Code extension

```bash
cd extension
npm install
npm run compile
```

## Notes

- Use `--config` with an absolute path when running `dmn` from scripts or MCP clients.
- If a VS Code extension daemon is running for the same config, CLI and MCP calls route to it.
- If no extension daemon is available, `dmn start` runs in local supervisor mode.

## License

OpenDaemon is licensed under GNU AGPL v3 (`AGPL-3.0-only`). See `LICENSE`.
