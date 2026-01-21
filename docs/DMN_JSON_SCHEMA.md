# dmn.json Configuration Schema

Complete reference for the `dmn.json` configuration file format.

## Table of Contents

- [Overview](#overview)
- [Root Schema](#root-schema)
- [Service Configuration](#service-configuration)
- [Ready Conditions](#ready-conditions)
- [Environment Files](#environment-files)
- [Examples](#examples)
- [Validation Rules](#validation-rules)

## Overview

The `dmn.json` file is a declarative configuration that defines all services in your development environment. OpenDaemon reads this file to understand what processes to run, in what order, and how to determine when they're ready.

## Root Schema

```json
{
  "version": "1.0",
  "services": {
    "service-name": { /* ServiceConfig */ }
  }
}
```

### Root Properties

#### `version` (required)
- **Type**: `string`
- **Pattern**: `^\d+\.\d+$`
- **Description**: Schema version for the configuration file
- **Current Version**: `"1.0"`

```json
{
  "version": "1.0"
}
```

#### `services` (required)
- **Type**: `object`
- **Description**: Map of service names to service configurations
- **Key Pattern**: `^[a-zA-Z0-9_-]+$` (alphanumeric, underscore, hyphen)
- **Value Type**: `ServiceConfig` object

```json
{
  "services": {
    "my-service": { /* config */ },
    "another_service": { /* config */ }
  }
}
```

## Service Configuration

Each service in the `services` object has the following schema:

```typescript
interface ServiceConfig {
  command: string;
  depends_on?: string[];
  ready_when?: ReadyCondition;
  env_file?: string;
}
```

### `command` (required)

The shell command to execute for this service.

- **Type**: `string`
- **Min Length**: 1
- **Description**: Command string executed in a shell environment

**Examples:**

```json
{
  "command": "npm run dev"
}
```

```json
{
  "command": "cargo run --bin server"
}
```

```json
{
  "command": "docker run --rm -p 5432:5432 postgres:15"
}
```

**Notes:**
- Commands are executed in the workspace root directory
- Shell features like pipes, redirects, and environment variable expansion are supported
- Use absolute paths or ensure executables are in PATH

### `depends_on` (optional)

Array of service names that must be ready before this service starts.

- **Type**: `array` of `string`
- **Default**: `[]` (no dependencies)
- **Description**: List of service names this service depends on

**Examples:**

Single dependency:
```json
{
  "depends_on": ["database"]
}
```

Multiple dependencies:
```json
{
  "depends_on": ["database", "redis", "auth-service"]
}
```

**Behavior:**
- OpenDaemon will start dependencies first
- This service won't start until all dependencies are "ready"
- Circular dependencies are detected and reported as errors
- Missing dependencies (services not defined) cause validation errors

### `ready_when` (optional)

Condition that determines when the service is considered "ready".

- **Type**: `ReadyCondition` object (see below)
- **Default**: Service is ready immediately after spawning
- **Description**: Defines how to detect service readiness

If omitted, the service is considered ready as soon as the process starts.

### `env_file` (optional)

Path to an environment file to load for this service.

- **Type**: `string`
- **Description**: Relative path to a `.env` file
- **Format**: Standard `.env` format (`KEY=value`)

**Example:**

```json
{
  "env_file": ".env.local"
}
```

**Environment File Format:**
```
DATABASE_URL=postgresql://localhost:5432/mydb
API_KEY=secret123
DEBUG=true
```

**Notes:**
- Path is relative to workspace root
- Variables are merged with system environment
- Service-specific variables override system variables
- Missing files are handled gracefully (warning logged)

## Ready Conditions

Ready conditions define how OpenDaemon determines when a service is ready to accept traffic or be used by dependent services.

### ReadyCondition Types

```typescript
type ReadyCondition = 
  | { log_contains: string }
  | { url_responds: string };
```

### Log Pattern Matching

Wait for a specific pattern to appear in the service's stdout or stderr.

```json
{
  "ready_when": {
    "log_contains": "pattern"
  }
}
```

#### Properties

- **`log_contains`**: `string`
  - Regex pattern to match against log output
  - Supports full Rust regex syntax
  - Case-sensitive by default
  - Matches against each line of output

#### Examples

**Simple string match:**
```json
{
  "ready_when": {
    "log_contains": "Server started on port 3000"
  }
}
```

**Regex pattern:**
```json
{
  "ready_when": {
    "log_contains": "Listening on.*:\\d+"
  }
}
```

**Case-insensitive match:**
```json
{
  "ready_when": {
    "log_contains": "(?i)ready"
  }
}
```

**Multiple conditions (match any):**
```json
{
  "ready_when": {
    "log_contains": "(Server listening|Application started)"
  }
}
```

#### Common Patterns

**Node.js/Express:**
```json
{
  "log_contains": "Server listening on port \\d+"
}
```

**Vite:**
```json
{
  "log_contains": "Local:.*http://localhost:\\d+"
}
```

**PostgreSQL:**
```json
{
  "log_contains": "database system is ready to accept connections"
}
```

**Redis:**
```json
{
  "log_contains": "Ready to accept connections"
}
```

**Django:**
```json
{
  "log_contains": "Starting development server at"
}
```

**Rails:**
```json
{
  "log_contains": "Listening on"
}
```

### URL Health Check

Poll a URL until it returns a successful HTTP response.

```json
{
  "ready_when": {
    "url_responds": "http://localhost:3000/health"
  }
}
```

#### Properties

- **`url_responds`**: `string`
  - Full URL to poll (must include protocol)
  - Polls every 500ms
  - Considers 2xx and 3xx status codes as success
  - Timeout after reasonable period (configurable in future)

#### Examples

**Health endpoint:**
```json
{
  "ready_when": {
    "url_responds": "http://localhost:3000/health"
  }
}
```

**Root endpoint:**
```json
{
  "ready_when": {
    "url_responds": "http://localhost:8080/"
  }
}
```

**HTTPS endpoint:**
```json
{
  "ready_when": {
    "url_responds": "https://localhost:3443/api/status"
  }
}
```

#### Behavior

- Polls the URL every 500ms
- Accepts any 2xx or 3xx HTTP status code
- Ignores connection errors (keeps retrying)
- Times out after a reasonable period (future: configurable)
- Does not follow redirects by default

## Examples

### Minimal Configuration

```json
{
  "version": "1.0",
  "services": {
    "app": {
      "command": "npm start"
    }
  }
}
```

### Simple Dependency Chain

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
      }
    }
  }
}
```

### Complex Microservices Setup

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
    "redis": {
      "command": "docker run --rm -p 6379:6379 redis:7",
      "ready_when": {
        "log_contains": "Ready to accept connections"
      }
    },
    "auth-service": {
      "command": "node services/auth/index.js",
      "depends_on": ["postgres", "redis"],
      "ready_when": {
        "url_responds": "http://localhost:3001/health"
      },
      "env_file": "services/auth/.env"
    },
    "user-service": {
      "command": "node services/users/index.js",
      "depends_on": ["postgres", "redis"],
      "ready_when": {
        "url_responds": "http://localhost:3002/health"
      },
      "env_file": "services/users/.env"
    },
    "api-gateway": {
      "command": "node gateway/index.js",
      "depends_on": ["auth-service", "user-service"],
      "ready_when": {
        "log_contains": "Gateway listening on port 8080"
      },
      "env_file": "gateway/.env"
    },
    "frontend": {
      "command": "npm run dev --prefix ./frontend",
      "depends_on": ["api-gateway"],
      "ready_when": {
        "log_contains": "Local:.*http://localhost:5173"
      }
    }
  }
}
```

### Full-Stack with Multiple Databases

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
    "mongodb": {
      "command": "docker run --rm -p 27017:27017 mongo:7",
      "ready_when": {
        "log_contains": "Waiting for connections"
      }
    },
    "redis": {
      "command": "docker run --rm -p 6379:6379 redis:7",
      "ready_when": {
        "log_contains": "Ready to accept connections"
      }
    },
    "backend": {
      "command": "cargo run --bin api-server",
      "depends_on": ["postgres", "mongodb", "redis"],
      "ready_when": {
        "log_contains": "Server listening on 0.0.0.0:8080"
      },
      "env_file": ".env.local"
    },
    "worker": {
      "command": "cargo run --bin worker",
      "depends_on": ["postgres", "redis"],
      "ready_when": {
        "log_contains": "Worker started"
      },
      "env_file": ".env.local"
    },
    "frontend": {
      "command": "npm run dev",
      "depends_on": ["backend"],
      "ready_when": {
        "url_responds": "http://localhost:5173"
      }
    }
  }
}
```

## Validation Rules

OpenDaemon validates your configuration and reports errors before starting services.

### Service Name Validation

- Must match pattern: `^[a-zA-Z0-9_-]+$`
- Only alphanumeric characters, underscores, and hyphens
- No spaces or special characters

**Valid:**
- `my-service`
- `service_1`
- `API-Gateway`

**Invalid:**
- `my service` (space)
- `service@1` (special char)
- `service.name` (dot)

### Dependency Validation

**Circular Dependencies:**
```json
{
  "services": {
    "a": { "command": "...", "depends_on": ["b"] },
    "b": { "command": "...", "depends_on": ["a"] }
  }
}
```
❌ Error: Circular dependency detected: a → b → a

**Missing Dependencies:**
```json
{
  "services": {
    "app": { "command": "...", "depends_on": ["database"] }
  }
}
```
❌ Error: Service 'app' depends on 'database' which is not defined

### Command Validation

- Must not be empty
- Must be a valid string

```json
{
  "command": ""
}
```
❌ Error: Command cannot be empty

### Ready Condition Validation

**Invalid regex pattern:**
```json
{
  "ready_when": {
    "log_contains": "[invalid(regex"
  }
}
```
❌ Error: Invalid regex pattern in ready_when condition

**Invalid URL:**
```json
{
  "ready_when": {
    "url_responds": "not-a-url"
  }
}
```
❌ Error: Invalid URL in ready_when condition

### Environment File Validation

- Path must be relative (not absolute)
- File should exist (warning if missing, not error)

```json
{
  "env_file": "/absolute/path/.env"
}
```
⚠️ Warning: Absolute paths not recommended

## JSON Schema

For IDE integration and validation, use this JSON Schema:

```json
{
  "$schema": "http://json-schema.org/draft-07/schema#",
  "title": "OpenDaemon Configuration",
  "type": "object",
  "required": ["version", "services"],
  "properties": {
    "version": {
      "type": "string",
      "pattern": "^\\d+\\.\\d+$",
      "description": "Configuration schema version"
    },
    "services": {
      "type": "object",
      "description": "Map of service names to configurations",
      "patternProperties": {
        "^[a-zA-Z0-9_-]+$": {
          "type": "object",
          "required": ["command"],
          "properties": {
            "command": {
              "type": "string",
              "minLength": 1,
              "description": "Shell command to execute"
            },
            "depends_on": {
              "type": "array",
              "items": {
                "type": "string"
              },
              "description": "Services that must be ready before starting"
            },
            "ready_when": {
              "oneOf": [
                {
                  "type": "object",
                  "required": ["log_contains"],
                  "properties": {
                    "log_contains": {
                      "type": "string",
                      "description": "Regex pattern to match in logs"
                    }
                  },
                  "additionalProperties": false
                },
                {
                  "type": "object",
                  "required": ["url_responds"],
                  "properties": {
                    "url_responds": {
                      "type": "string",
                      "format": "uri",
                      "description": "URL to poll for readiness"
                    }
                  },
                  "additionalProperties": false
                }
              ],
              "description": "Condition to determine service readiness"
            },
            "env_file": {
              "type": "string",
              "description": "Path to environment file"
            }
          },
          "additionalProperties": false
        }
      },
      "additionalProperties": false
    }
  },
  "additionalProperties": false
}
```

## Best Practices

### 1. Use Specific Ready Conditions

❌ Too generic:
```json
{
  "ready_when": {
    "log_contains": "started"
  }
}
```

✅ Specific:
```json
{
  "ready_when": {
    "log_contains": "Server listening on port 3000"
  }
}
```

### 2. Prefer URL Health Checks for HTTP Services

✅ Better:
```json
{
  "ready_when": {
    "url_responds": "http://localhost:3000/health"
  }
}
```

### 3. Keep Service Names Descriptive

✅ Good names:
- `postgres-db`
- `auth-service`
- `frontend-dev`

❌ Avoid:
- `service1`
- `s1`
- `temp`

### 4. Use Environment Files for Secrets

✅ Recommended:
```json
{
  "command": "npm start",
  "env_file": ".env.local"
}
```

❌ Avoid hardcoding:
```json
{
  "command": "API_KEY=secret123 npm start"
}
```

### 5. Document Complex Regex Patterns

Add comments in your configuration (if using JSON5 or JSONC):
```jsonc
{
  "ready_when": {
    // Matches: "Listening on 0.0.0.0:8080" or "Listening on localhost:8080"
    "log_contains": "Listening on (0\\.0\\.0\\.0|localhost):\\d+"
  }
}
```

## Troubleshooting

### Service Never Becomes Ready

1. Check the actual log output in VS Code
2. Test your regex pattern at regex101.com
3. Try a simpler pattern first
4. Verify the URL is correct and accessible

### Circular Dependency Errors

Use the error message to identify the cycle:
```
Error: Circular dependency detected: a → b → c → a
```

Break the cycle by removing one dependency or restructuring services.

### Command Not Found

Ensure:
- The executable is in PATH
- You're using the correct command syntax for your shell
- The workspace directory is correct

## See Also

- [README.md](../README.md) - Quick start guide
- [MCP_INTEGRATION.md](MCP_INTEGRATION.md) - AI agent integration
- [CONTRIBUTING.md](../CONTRIBUTING.md) - Contributing guidelines
