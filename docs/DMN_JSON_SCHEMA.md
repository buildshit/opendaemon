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
  | { log_contains: string; timeout_seconds?: number }
  | { url_responds: string; timeout_seconds?: number };
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

With custom timeout:

```json
{
  "ready_when": {
    "log_contains": "pattern",
    "timeout_seconds": 120
  }
}
```

#### Properties

- **`log_contains`**: `string`
  - Regex pattern to match against log output
  - Supports full Rust regex syntax
  - Case-sensitive by default
  - Matches against each line of output

- **`timeout_seconds`**: `number` (optional)
  - Maximum time to wait for the ready condition (in seconds)
  - Default: 60 seconds
  - Minimum: 1 second
  - Use higher values for slow-starting services

#### Examples

**Simple string match:**
```json
{
  "ready_when": {
    "log_contains": "Server started on port 3000"
  }
}
```

**With custom timeout:**
```json
{
  "ready_when": {
    "log_contains": "Server started on port 3000",
    "timeout_seconds": 90
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

With custom timeout:

```json
{
  "ready_when": {
    "url_responds": "http://localhost:3000/health",
    "timeout_seconds": 120
  }
}
```

#### Properties

- **`url_responds`**: `string`
  - Full URL to poll (must include protocol)
  - Polls every 500ms
  - Considers 2xx and 3xx status codes as success
  - Default timeout: 60 seconds

- **`timeout_seconds`**: `number` (optional)
  - Maximum time to wait for the URL to respond (in seconds)
  - Default: 60 seconds
  - Minimum: 1 second
  - Use higher values for services with slow startup

#### Examples

**Health endpoint:**
```json
{
  "ready_when": {
    "url_responds": "http://localhost:3000/health"
  }
}
```

**With custom timeout:**
```json
{
  "ready_when": {
    "url_responds": "http://localhost:3000/health",
    "timeout_seconds": 90
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
- Default timeout: 60 seconds (configurable via `timeout_seconds`)
- Does not follow redirects by default

## Timeout Configuration

Both `log_contains` and `url_responds` ready conditions support an optional `timeout_seconds` field to customize how long OpenDaemon waits for a service to become ready.

### Default Timeout

If `timeout_seconds` is not specified, the default timeout is **60 seconds**.

### Custom Timeout Examples

**Slow-starting database:**
```json
{
  "services": {
    "postgres": {
      "command": "docker run --rm -p 5432:5432 postgres:15",
      "ready_when": {
        "log_contains": "database system is ready",
        "timeout_seconds": 120
      }
    }
  }
}
```

**Service with long initialization:**
```json
{
  "services": {
    "ml-service": {
      "command": "python train.py",
      "ready_when": {
        "log_contains": "Model loaded",
        "timeout_seconds": 300
      }
    }
  }
}
```

**Fast service with short timeout:**
```json
{
  "services": {
    "redis": {
      "command": "redis-server",
      "ready_when": {
        "log_contains": "Ready to accept connections",
        "timeout_seconds": 30
      }
    }
  }
}
```

### Recommended Timeout Values

- **Fast services** (Redis, simple scripts): 30-60 seconds
- **Medium services** (Node.js apps, Python servers): 60-90 seconds  
- **Slow services** (databases, Docker containers): 90-180 seconds
- **Very slow services** (large builds, ML models): 180+ seconds

### Timeout Error Messages

When a service times out, OpenDaemon provides detailed error information:

- Service name that timed out
- The ready condition that was being waited for
- The last few log lines from the service (for `log_contains` conditions)
- Troubleshooting suggestions

Example timeout error:
```
Service 'backend' timed out after 60 seconds waiting for ready condition.
Condition: log_contains "Server listening"
Last log lines:
  [INFO] Loading configuration...
  [INFO] Connecting to database...
  [ERROR] Connection refused

Troubleshooting:
- Check if the ready condition pattern is correct
- Increase timeout_seconds if the service needs more time
- Review the service logs for errors
```

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
                    },
                    "timeout_seconds": {
                      "type": "number",
                      "minimum": 1,
                      "description": "Maximum time to wait for ready condition (default: 60)"
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
                    },
                    "timeout_seconds": {
                      "type": "number",
                      "minimum": 1,
                      "description": "Maximum time to wait for ready condition (default: 60)"
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

## Troubleshooting

### "No Services Found" Error

This error occurs when OpenDaemon cannot find or load services from your configuration.

#### Possible Causes and Solutions

**1. Missing dmn.json file**

Error: `No dmn.json file found in workspace`

Solution:
- Create a `dmn.json` file in your workspace root
- Use the command palette: "OpenDaemon: Create Configuration"
- Ensure the file is named exactly `dmn.json` (not `dmn.json.txt` or similar)

**2. Empty services object**

Your `dmn.json` exists but has no services defined:

```json
{
  "version": "1.0",
  "services": {}
}
```

Solution: Add at least one service:
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

**3. Invalid JSON syntax**

Common JSON errors:
- Trailing commas
- Missing quotes around keys
- Unclosed brackets or braces

Solution: Validate your JSON using:
- VS Code's built-in JSON validation
- Online JSON validators
- The error message in the OpenDaemon output panel

**4. Tree view not initialized**

The extension's tree view may not be properly initialized.

Solution:
- Reload the VS Code window: "Developer: Reload Window"
- Check the OpenDaemon output panel for initialization errors
- Ensure the extension is activated (check Extensions panel)

**5. File in wrong location**

The `dmn.json` file must be in the workspace root, not in a subdirectory.

Solution:
```
✅ Correct:
/my-project/dmn.json

❌ Incorrect:
/my-project/config/dmn.json
/my-project/src/dmn.json
```

### Timeout Errors

Services timing out before becoming ready is a common issue, especially for slow-starting services.

#### Understanding Timeout Errors

When a service times out, you'll see an error like:

```
Service 'database' timed out after 60 seconds waiting for ready condition.
Condition: log_contains "ready to accept connections"
```

#### Common Causes and Solutions

**1. Service needs more time to start**

Some services (databases, Docker containers, ML models) take longer than the default 60 seconds.

Solution: Increase the timeout:
```json
{
  "ready_when": {
    "log_contains": "ready to accept connections",
    "timeout_seconds": 120
  }
}
```

**2. Incorrect ready condition pattern**

The log pattern or URL doesn't match what the service actually outputs.

Solution:
- Check the service logs in the Output panel
- Copy the exact text from the logs
- Test regex patterns at regex101.com
- Start with a simple substring match before using regex

Example - Too specific:
```json
{
  "ready_when": {
    "log_contains": "Server listening on port 3000"
  }
}
```

Better - More flexible:
```json
{
  "ready_when": {
    "log_contains": "Server listening"
  }
}
```

**3. Service is failing to start**

The service may be crashing or encountering errors before it becomes ready.

Solution:
- Check the last few log lines in the timeout error message
- View full logs in the Output panel
- Look for error messages or stack traces
- Verify the command is correct
- Check environment variables and dependencies

**4. URL not accessible**

For `url_responds` conditions, the URL may not be reachable.

Solution:
- Verify the URL is correct (including protocol: `http://` or `https://`)
- Check the port number matches your service
- Ensure the service is binding to the correct interface (0.0.0.0 vs localhost)
- Test the URL manually with curl or a browser

**5. Service outputs to stderr instead of stdout**

Some services log to stderr, which OpenDaemon monitors, but the pattern might not match.

Solution:
- Check both stdout and stderr in the logs
- Adjust your pattern to match the actual output
- Consider using a URL health check instead

#### Timeout Troubleshooting Checklist

When debugging timeout issues:

1. ✅ Check the service logs in the Output panel
2. ✅ Verify the ready condition matches actual output
3. ✅ Test the service command manually in a terminal
4. ✅ Increase timeout_seconds if needed
5. ✅ Simplify the ready condition pattern
6. ✅ Check for service errors or crashes
7. ✅ Verify dependencies are running
8. ✅ Test URLs manually with curl

#### Timeout Best Practices

**Use appropriate timeout values:**

```json
{
  "services": {
    "redis": {
      "command": "redis-server",
      "ready_when": {
        "log_contains": "Ready to accept connections",
        "timeout_seconds": 30
      }
    },
    "postgres": {
      "command": "docker run --rm postgres:15",
      "ready_when": {
        "log_contains": "database system is ready",
        "timeout_seconds": 120
      }
    },
    "ml-model": {
      "command": "python load_model.py",
      "ready_when": {
        "log_contains": "Model loaded successfully",
        "timeout_seconds": 300
      }
    }
  }
}
```

**Prefer URL health checks for HTTP services:**

```json
{
  "ready_when": {
    "url_responds": "http://localhost:3000/health",
    "timeout_seconds": 90
  }
}
```

This is more reliable than log pattern matching for web services.

### Service Won't Start

If a service fails to start at all (not a timeout issue):

**1. Command not found**

Error: `command not found` or similar

Solution:
- Ensure the executable is in your PATH
- Use absolute paths if needed
- Verify the command works in a terminal
- Check for typos in the command

**2. Permission denied**

Error: `Permission denied`

Solution:
- Make scripts executable: `chmod +x script.sh`
- Check file permissions
- Run VS Code with appropriate permissions

**3. Working directory issues**

The command may expect to run from a specific directory.

Solution:
- Commands run from the workspace root
- Use `cd` in your command if needed: `cd subdir && npm start`
- Use relative paths correctly

**4. Missing dependencies**

The service may depend on other services that aren't running.

Solution:
- Check the `depends_on` field
- Ensure dependencies are defined and starting correctly
- Look for circular dependencies

**5. Environment variables missing**

The service may need specific environment variables.

Solution:
- Use the `env_file` field:
```json
{
  "command": "npm start",
  "env_file": ".env.local"
}
```

- Verify the env file exists and has correct format
- Check for required variables

### Circular Dependency Errors

Error: `Circular dependency detected: a → b → c → a`

This means services depend on each other in a loop.

Solution:
- Review the dependency chain in the error message
- Remove one dependency to break the cycle
- Restructure services to avoid circular dependencies
- Consider if all dependencies are truly necessary

Example problem:
```json
{
  "services": {
    "service-a": {
      "command": "...",
      "depends_on": ["service-b"]
    },
    "service-b": {
      "command": "...",
      "depends_on": ["service-a"]
    }
  }
}
```

Solution: Remove one dependency or add an intermediate service.

### Configuration Validation Errors

**Invalid service name**

Error: `Invalid service name: 'my service'`

Solution: Use only alphanumeric characters, hyphens, and underscores:
```json
{
  "services": {
    "my-service": { /* ... */ },
    "my_service": { /* ... */ },
    "myService123": { /* ... */ }
  }
}
```

**Invalid regex pattern**

Error: `Invalid regex pattern in ready_when condition`

Solution:
- Test your regex at regex101.com
- Escape special characters: `\\.`, `\\d`, `\\[`, etc.
- Use raw strings or double-escape in JSON

**Missing required fields**

Error: `Missing required field: command`

Solution: Ensure all required fields are present:
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

### Getting Help

If you're still experiencing issues:

1. **Check the Output panel**: Look for detailed error messages in the "OpenDaemon" output channel
2. **Enable debug logging**: Check the extension logs for more details
3. **Test manually**: Try running the service command directly in a terminal
4. **Simplify**: Start with a minimal configuration and add complexity gradually
5. **Report issues**: File a bug report with:
   - Your `dmn.json` configuration
   - Error messages from the Output panel
   - Steps to reproduce the issue
   - Your OS and VS Code version

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
