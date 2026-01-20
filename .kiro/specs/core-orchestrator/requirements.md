# Requirements Document

## Introduction

OpenDaemon (`dmn`) is a VS Code extension built with Rust that orchestrates local development services through a declarative `dmn.json` configuration file. Users define multiple services (daemons) in a single configuration file, where each service represents a process to run (like a database, backend server, frontend dev server, etc.). The orchestrator provides fine-grained control over when and how each service starts, including dependency ordering and smart waiting for readiness. An integrated MCP (Model Context Protocol) server enables AI agents to read logs from any running service by simply specifying the service name and number of lines. This requirements document focuses on the foundational orchestration engine that reads the configuration, manages service lifecycles, implements smart waiting logic, and exposes service logs to AI agents.

## Requirements

### Requirement 1: Configuration File Parsing

**User Story:** As a developer, I want to define multiple services in a single `dmn.json` file, so that I can declaratively specify all the processes I need to run and their relationships.

#### Acceptance Criteria

1. WHEN the system reads a `dmn.json` file THEN it SHALL parse the JSON structure into an internal service configuration model
2. WHEN a configuration file contains a "services" object THEN the system SHALL extract each service definition with its command, dependencies, and ready conditions
3. WHEN a user adds a new service to the "services" object THEN the system SHALL recognize and manage it alongside existing services
4. IF a configuration file is malformed or missing required fields THEN the system SHALL return a descriptive error message indicating the specific issue
5. WHEN a service definition includes an "env_file" property THEN the system SHALL load environment variables from the specified file path
6. WHEN a service definition includes a "depends_on" array THEN the system SHALL record the dependency relationships for execution ordering
2. WHEN a configuration file contains a "services" object THEN the system SHALL extract each service definition with its command, dependencies, and ready conditions
3. IF a configuration file is malformed or missing required fields THEN the system SHALL return a descriptive error message indicating the specific issue
4. WHEN a service definition includes an "env_file" property THEN the system SHALL load environment variables from the specified file path
5. WHEN a service definition includes a "depends_on" array THEN the system SHALL record the dependency relationships for execution ordering

### Requirement 2: Dependency Graph Construction

**User Story:** As a developer, I want services to start in the correct order based on their dependencies, so that dependent services don't fail because their dependencies aren't ready.

#### Acceptance Criteria

1. WHEN the system processes service configurations THEN it SHALL construct a directed acyclic graph (DAG) representing service dependencies
2. IF the dependency graph contains a cycle THEN the system SHALL detect it and return an error listing the services involved in the cycle
3. WHEN determining start order THEN the system SHALL use topological sorting to ensure dependencies start before dependents
4. IF a service lists a non-existent service in "depends_on" THEN the system SHALL return an error identifying the missing dependency

### Requirement 3: Process Spawning and Management

**User Story:** As a developer, I want the orchestrator to spawn and manage service processes, so that I don't have to manually start each service in separate terminals.

#### Acceptance Criteria

1. WHEN a service is ready to start THEN the system SHALL spawn a new process using the configured command
2. WHEN a process is spawned THEN the system SHALL capture both stdout and stderr streams
3. WHEN a process exits THEN the system SHALL record the exit code and timestamp
4. WHEN a process crashes THEN the system SHALL update the service status to reflect the failure
5. IF a service command cannot be executed THEN the system SHALL return an error with details about why the command failed

### Requirement 4: Smart Waiting with Log Pattern Matching

**User Story:** As a developer, I want the orchestrator to wait for services to be truly ready before starting dependents, so that I don't get connection errors from services starting too early.

#### Acceptance Criteria

1. WHEN a service has a "ready_when.log_contains" condition THEN the system SHALL monitor the process output for the specified string
2. WHEN the specified log pattern is matched THEN the system SHALL mark the service as ready and trigger dependent services
3. WHEN a service has a "ready_when.url_responds" condition THEN the system SHALL poll the specified URL until it returns a successful response
4. IF a service has no "ready_when" condition THEN the system SHALL consider it ready immediately after spawning
5. WHEN monitoring logs THEN the system SHALL use regex pattern matching to support flexible log detection
6. IF a ready condition is not met within a reasonable timeout THEN the system SHALL report a timeout error

### Requirement 5: Service Status Tracking

**User Story:** As a developer, I want to see the current status of all services, so that I can understand what's running, what's starting, and what has failed.

#### Acceptance Criteria

1. WHEN a service is first defined THEN the system SHALL initialize its status as "not_started"
2. WHEN a service process is spawned THEN the system SHALL update its status to "starting"
3. WHEN a service meets its ready condition THEN the system SHALL update its status to "running"
4. WHEN a service process exits with code 0 THEN the system SHALL update its status to "stopped"
5. WHEN a service process exits with a non-zero code THEN the system SHALL update its status to "failed" and record the exit code
6. WHEN queried THEN the system SHALL provide the current status of all configured services

### Requirement 6: Log Streaming and Storage

**User Story:** As a developer, I want to see real-time logs from all services, so that I can debug issues and understand what's happening in my stack.

#### Acceptance Criteria

1. WHEN a service outputs to stdout or stderr THEN the system SHALL capture each line with a timestamp
2. WHEN logs are captured THEN the system SHALL store them in a circular buffer per service to prevent unbounded memory growth
3. WHEN the system is running THEN it SHALL provide a way to stream logs in real-time
4. WHEN requested THEN the system SHALL provide access to historical logs from the circular buffer
5. WHEN a log line is captured THEN the system SHALL prefix it with the service name for identification

### Requirement 7: Graceful Shutdown

**User Story:** As a developer, I want to cleanly stop all services, so that resources are properly released and I don't have orphaned processes.

#### Acceptance Criteria

1. WHEN a shutdown is requested THEN the system SHALL send termination signals to all running processes
2. WHEN shutting down THEN the system SHALL stop services in reverse dependency order (dependents before dependencies)
3. WHEN a process doesn't terminate within a grace period THEN the system SHALL forcefully kill the process
4. WHEN all processes have stopped THEN the system SHALL exit cleanly
5. IF a shutdown is interrupted THEN the system SHALL ensure all spawned processes are terminated before exiting

### Requirement 8: Error Handling and Reporting

**User Story:** As a developer, I want clear error messages when something goes wrong, so that I can quickly identify and fix configuration or runtime issues.

#### Acceptance Criteria

1. WHEN any error occurs THEN the system SHALL provide a descriptive error message indicating what went wrong
2. WHEN a service fails to start THEN the system SHALL include the service name and the reason for failure
3. WHEN a configuration error is detected THEN the system SHALL indicate the specific field or service that is misconfigured
4. WHEN a process crashes THEN the system SHALL capture and display the last lines of output before the crash
5. IF multiple services fail THEN the system SHALL report all failures, not just the first one

### Requirement 9: VS Code Extension Integration

**User Story:** As a developer, I want to manage my services directly from VS Code, so that I can start, stop, and monitor all my processes without leaving my editor.

#### Acceptance Criteria

1. WHEN the extension is activated THEN it SHALL scan the workspace for a `dmn.json` file
2. WHEN a `dmn.json` file is found THEN the system SHALL display all defined services in a VS Code sidebar tree view
3. WHEN a user clicks the start button THEN the system SHALL start all services according to their dependency order
4. WHEN services are running THEN the system SHALL display the status of each service in the tree view (starting, running, failed, stopped)
5. WHEN a user clicks on a service THEN the system SHALL show that service's logs in the VS Code output panel
6. WHEN the Rust binary is bundled with the extension THEN it SHALL be executable on Windows, macOS, and Linux
7. WHEN a user modifies the `dmn.json` file THEN the system SHALL reload the configuration and update the tree view

### Requirement 10: Individual Service Control

**User Story:** As a developer, I want to start, stop, and restart individual services, so that I can test specific scenarios without restarting my entire stack.

#### Acceptance Criteria

1. WHEN a user right-clicks on a service in the tree view THEN the system SHALL provide options to start, stop, or restart that specific service
2. WHEN starting an individual service THEN the system SHALL first ensure all its dependencies are running
3. WHEN stopping an individual service THEN the system SHALL also stop any services that depend on it
4. WHEN restarting a service THEN the system SHALL stop it gracefully and then start it again
5. WHEN a service is manually stopped THEN the system SHALL not automatically restart it unless explicitly requested

### Requirement 11: MCP Server for AI Agent Log Access

**User Story:** As a developer using AI coding assistants, I want my AI agent to read service logs, so that it can help me debug issues by analyzing runtime output.

#### Acceptance Criteria

1. WHEN the MCP server is enabled THEN it SHALL expose a "read_logs" tool that AI agents can call via the Model Context Protocol
2. WHEN an AI agent calls "read_logs" with parameters like `{"service": "backend", "lines": 100}` THEN the system SHALL return the last 100 log lines from the backend service
3. WHEN an AI agent calls "read_logs" with parameters like `{"service": "frontend", "lines": "all"}` THEN the system SHALL return all available log lines from the frontend service's circular buffer
4. WHEN an AI agent calls "get_service_status" THEN the system SHALL return the current status of all services (running, stopped, failed, etc.)
5. WHEN an AI agent calls "list_services" THEN the system SHALL return the names of all services defined in the `dmn.json` file
6. WHEN the MCP server receives a request for a non-existent service THEN it SHALL return an error indicating the service was not found
7. IF the MCP server is not authenticated THEN it SHALL return an error indicating authentication is required

### Requirement 12: Configuration Discovery and Initialization

**User Story:** As a developer, I want the extension to help me create a `dmn.json` file, so that I can quickly set up orchestration for my project.

#### Acceptance Criteria

1. WHEN the extension detects a project with no `dmn.json` file THEN it SHALL offer to create an initial configuration
2. WHEN creating an initial configuration THEN the system SHALL scan for common patterns (package.json scripts, docker-compose.yml) to suggest services
3. WHEN a user creates a new `dmn.json` file THEN the system SHALL provide a template with the correct JSON structure and example services
4. WHEN a user saves the `dmn.json` file THEN the system SHALL automatically detect it and populate the sidebar tree view
5. IF the `dmn.json` file is deleted THEN the system SHALL clear the tree view and stop any running services

#### Requirement 13: Authentication & Device Flow
**User Story:** As a user, I want to log in via the CLI/Extension to verify my Pro license without managing complex API keys.
**Acceptance Criteria:**
1.  WHEN the "Login" command is triggered, the system SHALL generate a unique device code and open `https://opendaemon.com/activate?code={code}`.
2.  The CLI SHALL poll the authentication backend for confirmation of that specific device code.
3.  UPON success, the system SHALL securely store the returned authentication token.
4.  The system SHALL support a `dmn refresh` command to re-check license status without re-logging in (e.g., after upgrading from Free to Pro).

#### Requirement 14: JSON Schema & IntelliSense
**User Story:** As a developer editing `dmn.json`, I want autocomplete and validation, so I don't make syntax errors.
**Acceptance Criteria:**
1.  The extension SHALL contribute a JSON Validation Provider for files matching `dmn.json` or `.dmn/config.json`.
2.  The schema SHALL enforce valid types (e.g., ensuring `depends_on` is an array of strings).
3.  The schema SHALL provide hover descriptions explaining what `log_contains` and `url_responds` do.