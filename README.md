# OpenDaemon (dmn)

A VS Code extension built with Rust that orchestrates local development services through a declarative `dmn.json` configuration file.

## Project Structure

This is a Cargo workspace with two crates:

- **core**: The main orchestration engine and CLI
- **pro**: Pro features (authentication, mcp server, remote services, etc.)

### Core Modules

- `config`: Configuration file parsing and validation
- `graph`: Dependency graph construction and traversal
- `process`: Process spawning and management
- `logs`: Log streaming and circular buffer storage
- `orchestrator`: Core orchestration logic coordinating all components

## Building

```bash
cargo build
```

## Running

```bash
# Run in daemon mode (for VS Code extension)
cargo run --bin dmn -- daemon

# Run in MCP server mode (for AI agents)
cargo run --bin dmn -- mcp

# Start all services
cargo run --bin dmn -- start

# Stop all services
cargo run --bin dmn -- stop

# Check service status
cargo run --bin dmn -- status
```

## Dependencies

- **tokio**: Async runtime for process management
- **serde/serde_json**: Configuration parsing
- **clap**: CLI argument parsing
- **regex**: Log pattern matching
- **petgraph**: Dependency graph operations
- **thiserror**: Error handling
- **reqwest**: HTTP client for URL polling


# Example of how this would be used

User would type `dmn start <service name>` in the terminal and it would start the service name defined in the `dmn.json` file.

Example of how the dmn.json would look:

```
{
  "version": "1.0",
  "services": {
    "db": {
      "command": "docker run -p 5432:5432 postgres",
      "ready_when": { "log_contains": "database system is ready to accept connections" }
    },
    "backend": {
      "command": "npm run dev:api",
      "depends_on": ["db"],
      "ready_when": { "log_contains": "Server listening on 3000" },
      "env_file": ".env"
    },
    "frontend": {
      "command": "npm run dev:web",
      "depends_on": ["backend"],
      "ready_when": { "url_responds": "http://localhost:8080" }
    }
  }
}
```

The idea is to make it as simple as possible for the user to get their commands in the logic they need and be able to log it all properly. The MCP server would allow the AI to see the services made in the dmn.json file and ask for the information on that service properly. Allowing the services to have logic and conditions allows the exact behaviour the user needs to run the services properly and troubleshoot properly. 

### 1. The "Native + Container" Hybrid
Unlike `docker-compose`, which forces everything into a container, or `npm` scripts, which can’t easily handle Docker, `dmn` treats them as equals. 
*   **Your DB:** A Docker command.
*   **Your Backend/Frontend:** Native `npm` commands.
*   **The Result:** You get the speed of native development with the reliability of containerized infrastructure.

### 2. "Smart Waiting" vs. "Blind Starting"
This is the biggest pain point in dev-ops today. 
*   **Standard Way:** You start the backend, it tries to connect to the DB immediately, the DB isn't ready, the backend crashes, you have to manually restart it.
*   **The `dmn` Way:** Your `ready_when` logic creates a "handshake." The backend command literally isn't fired until the DB log confirms it is ready. It eliminates "Race Conditions" in local development.

### 3. Structured Context for the AI
When a user asks an AI: *"Why is my backend failing?"*
*   **Without `dmn`:** The AI has to guess which terminal window has the error.
*   **With `dmn` (MCP):** The AI can literally ask the orchestrator: *"Which service is currently in the `failed` or `starting` state?"* The orchestrator replies: *"The DB is ready, but the Backend has been `starting` for 30 seconds and hasn't seen the 'Server listening' log."* 
*   **The Win:** The AI now has a **map** of the project's health, not just a pile of logs.
