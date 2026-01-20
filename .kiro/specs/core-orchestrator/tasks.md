# Implementation Plan

- [x] 1. Set up Rust project structure and core dependencies
  - Create Cargo workspace with `core` and `pro` crates
  - Add dependencies: tokio, serde, serde_json, clap, regex, petgraph, thiserror, reqwest
  - Configure Cargo.toml with feature flags for pro features
  - Set up basic module structure (config, graph, process, logs, orchestrator)
  - _Requirements: 1.1, 1.2, 1.3_

- [x] 2. Implement configuration parser
  - [x] 2.1 Create data structures for DmnConfig, ServiceConfig, and ReadyCondition
    - Define Rust structs with serde derive macros
    - Implement Display and Debug traits
    - Write unit tests for struct serialization/deserialization
    - _Requirements: 1.1, 1.2_

  - [x] 2.2 Implement config file parsing and validation
    - Write `parse_config()` function to read and parse JSON
    - Implement validation logic for required fields
    - Add error handling with descriptive messages
    - Write unit tests for valid and invalid configurations
    - _Requirements: 1.1, 1.2, 1.3, 1.4_

  - [x] 2.3 Implement environment file loading
    - Write `load_env_file()` function to parse .env files
    - Handle missing files gracefully
    - Write unit tests with sample .env files
    - _Requirements: 1.5_

- [x] 3. Implement dependency graph
  - [x] 3.1 Create ServiceGraph struct with petgraph
    - Initialize DiGraph with service names as nodes
    - Build edges from depends_on relationships
    - Write unit tests for graph construction
    - _Requirements: 2.1, 2.2_

  - [x] 3.2 Implement cycle detection
    - Use petgraph's is_cyclic_directed algorithm
    - Generate error messages listing services in cycle
    - Write unit tests with cyclic and acyclic graphs
    - _Requirements: 2.2_

  - [x] 3.3 Implement topological sorting for start order
    - Use petgraph's toposort algorithm
    - Handle missing dependencies with clear errors
    - Write unit tests verifying correct ordering
    - _Requirements: 2.3, 2.4_

  - [x] 3.4 Implement dependent service lookup
    - Write `get_dependents()` to find services that depend on a given service
    - Write unit tests for various dependency scenarios
    - _Requirements: 2.3_

- [x] 4. Implement log buffer
  - [x] 4.1 Create LogLine and CircularBuffer structs
    - Define LogLine with timestamp, content, and stream type
    - Implement CircularBuffer using VecDeque
    - Write unit tests for buffer operations
    - _Requirements: 6.1, 6.2_

  - [x] 4.2 Implement LogBuffer with per-service buffers
    - Create HashMap of service name to CircularBuffer
    - Implement `append()` method with automatic eviction
    - Implement `get_lines()` and `get_all_lines()` methods
    - Write unit tests for concurrent access and buffer limits
    - _Requirements: 6.2, 6.3, 6.4, 6.5_

- [x] 5. Implement process manager
  - [x] 5.1 Create ManagedProcess and ProcessManager structs
    - Define ServiceStatus enum
    - Create ManagedProcess with Child handle and metadata
    - Initialize ProcessManager with empty HashMap
    - _Requirements: 3.1, 3.2, 5.1, 5.2_

  - [x] 5.2 Implement process spawning
    - Write `spawn_service()` using tokio::process::Command
    - Parse command string and set up environment variables
    - Pipe stdout and stderr to separate async tasks
    - Update service status to Starting
    - Write unit tests spawning simple commands
    - _Requirements: 3.1, 3.2, 3.5, 5.2_

  - [x] 5.3 Implement stdout/stderr streaming to log buffer
    - Create async tasks to read from stdout and stderr
    - Parse lines and append to LogBuffer
    - Handle process exit and update status
    - Write unit tests verifying log capture
    - _Requirements: 3.2, 3.3, 6.1_

  - [x] 5.4 Implement graceful process shutdown
    - Write `stop_service()` with SIGTERM signal
    - Implement timeout and force kill with SIGKILL
    - Update service status appropriately
    - Write unit tests for graceful and forced shutdown
    - _Requirements: 3.3, 3.4, 7.1, 7.2, 7.3_

  - [x] 5.5 Implement service restart
    - Write `restart_service()` that stops then starts
    - Handle errors during restart
    - Write unit tests for restart scenarios
    - _Requirements: 10.4_

  - [x] 5.6 Implement status tracking and queries
    - Write `get_status()` to return current ServiceStatus
    - Implement status updates on process events
    - Write unit tests for status transitions
    - _Requirements: 5.1, 5.2, 5.3, 5.4, 5.5, 5.6_

- [x] 6. Implement ready watcher
  - [x] 6.1 Create ReadyWatcher struct and watch methods
    - Initialize with HashMap of conditions and ready set
    - Implement `watch_service()` dispatcher
    - _Requirements: 4.1, 4.2, 4.4_

  - [x] 6.2 Implement log pattern matching
    - Write `watch_log_pattern()` with regex matching
    - Subscribe to log stream for service
    - Mark service ready when pattern matches
    - Write unit tests with various regex patterns
    - _Requirements: 4.1, 4.2, 4.5_

  - [x] 6.3 Implement URL polling
    - Write `watch_url()` with reqwest HTTP client
    - Poll URL every 500ms until success response
    - Mark service ready on successful response
    - Write unit tests with mock HTTP server
    - _Requirements: 4.3_

  - [x] 6.4 Implement timeout handling
    - Add configurable timeout for ready conditions
    - Return error if timeout exceeded
    - Write unit tests for timeout scenarios
    - _Requirements: 4.6_

- [x] 7. Implement core orchestrator
  - [x] 7.1 Create Orchestrator struct integrating all components
    - Initialize with config, graph, process manager, log buffer, ready watcher
    - Set up event channel for orchestrator events
    - _Requirements: 1.1, 2.1, 3.1, 5.1, 6.1_

  - [x] 7.2 Implement start_all with dependency ordering
    - Get start order from dependency graph
    - Iterate through services and start each with dependencies
    - Emit events for service lifecycle changes
    - Write integration tests for full stack startup
    - _Requirements: 2.3, 3.1, 4.1, 4.2, 9.3_

  - [x] 7.3 Implement start_service_with_deps
    - Check if dependencies are ready
    - Wait for dependencies if not ready
    - Spawn service process
    - Set up ready watching based on configuration
    - Write integration tests with various dependency scenarios

    - _Requirements: 2.3, 3.1, 4.1, 4.2, 4.4_

  - [x] 7.4 Implement stop_all with reverse ordering
    - Get reverse start order from dependency graph
    - Stop each service gracefully
    - Emit events for service stops
    - Write integration tests for full stack shutdown
    - _Requirements: 7.1, 7.2, 7.4_

  - [x] 7.5 Implement stop_service with dependent cascade
    - Get list of dependent services
    - Stop dependents first
    - Stop the target service
    - Write integration tests for cascade stops
    - _Requirements: 7.2, 10.3_

  - [x] 7.6 Implement restart_service
    - Call stop_service then start_service_with_deps
    - Handle errors appropriately
    - Write integration tests for restart scenarios
    - _Requirements: 10.4_

- [x] 8. Implement JSON-RPC server for extension communication
  - [x] 8.1 Create RPC request and response types

    - Define RpcRequest enum with all methods
    - Define RpcResponse struct with result/error
    - Implement serialization/deserialization
    - Write unit tests for message parsing
    - _Requirements: 9.1, 9.3_

  - [x] 8.2 Implement RPC server with stdio communication
    - Create RpcServer struct with orchestrator reference
    - Read JSON-RPC messages from stdin
    - Parse and dispatch to handler methods
    - Write responses to stdout
    - Write integration tests with mock stdin/stdout
    - _Requirements: 9.1, 9.3, 9.5_

  - [x] 8.3 Implement RPC method handlers
    - Implement StartAll, StopAll, StartService, StopService, RestartService
    - Implement GetStatus to return all service statuses
    - Implement GetLogs with service name and line count parameters
    - Handle errors and return appropriate error responses
    - Write unit tests for each method
    - _Requirements: 9.3, 9.4, 9.5, 10.1, 10.2, 10.4_

  - [x] 8.4 Implement event streaming to extension
    - Stream OrchestratorEvents as JSON-RPC notifications
    - Include log lines, status changes, and errors
    - Write integration tests for event streaming
    - _Requirements: 9.4, 9.5_

- [ ] 9. Implement MCP server for AI agent integration
  - [ ] 9.1 Create MCP server structure
    - Create DmnMcpServer struct with orchestrator reference
    - Set up MCP SDK integration (or implement protocol manually)
    - _Requirements: 11.1_
  - [ ] 9.2 Implement read_logs tool
    - Register tool with MCP server
    - Parse service name and lines parameter (number or "all")
    - Query log buffer and return formatted logs
    - Handle non-existent services with error
    - Write unit tests for various parameter combinations
    - _Requirements: 11.2, 11.3, 11.6_
  - [ ] 9.3 Implement get_service_status tool
    - Register tool with MCP server
    - Query process manager for all service statuses
    - Return formatted status map
    - Write unit tests for status queries
    - _Requirements: 11.4_
  - [ ] 9.4 Implement list_services tool
    - Register tool with MCP server
    - Return list of all service names from config
    - Write unit tests for service listing
    - _Requirements: 11.5_
  - [ ] 9.5 Implement MCP server stdio listener
    - Set up stdio communication for MCP protocol
    - Handle tool calls and return results
    - Write integration tests with mock MCP client
    - _Requirements: 11.1_
  - [ ] 9.6 Add authentication placeholder for Pro features
    - Add auth check before executing tools
    - Return error if not authenticated
    - Write unit tests for auth gating
    - _Requirements: 11.6, 11.7_

- [ ] 10. Create CLI entry point
  - [ ] 10.1 Implement main.rs with clap argument parsing
    - Define CLI commands: daemon, mcp, start, stop, status
    - Parse arguments and dispatch to appropriate mode
    - Write integration tests for CLI invocation
    - _Requirements: 9.1, 11.1_
  - [ ] 10.2 Implement daemon mode
    - Load dmn.json configuration
    - Initialize orchestrator
    - Start JSON-RPC server on stdio
    - Write integration tests for daemon mode
    - _Requirements: 9.1, 9.3_
  - [ ] 10.3 Implement MCP mode
    - Load dmn.json configuration
    - Initialize orchestrator
    - Start MCP server on stdio
    - Write integration tests for MCP mode
    - _Requirements: 11.1_

- [ ] 11. Create VS Code extension
  - [ ] 11.1 Set up TypeScript extension project
    - Initialize with yo code generator
    - Configure package.json with extension metadata
    - Set up build scripts and dependencies
    - _Requirements: 9.1_
  - [ ] 11.2 Implement extension activation and dmn.json detection
    - Activate on workspace open
    - Scan for dmn.json file in workspace root
    - Show notification if dmn.json found
    - Write tests for file detection
    - _Requirements: 9.1, 12.1_
  - [ ] 11.3 Implement daemon process spawning
    - Bundle Rust binary with extension
    - Spawn dmn binary in daemon mode
    - Set up stdin/stdout communication
    - Handle process crashes and restart
    - Write tests for process lifecycle
    - _Requirements: 9.1, 9.6_
  - [ ] 11.4 Implement JSON-RPC client
    - Create RPC client class for sending requests
    - Implement request/response matching with IDs
    - Handle RPC errors
    - Write unit tests for RPC communication
    - _Requirements: 9.3_
  - [ ] 11.5 Implement service tree view
    - Create ServiceTreeDataProvider class
    - Register tree view in package.json
    - Display services with status icons
    - Update tree on status changes
    - Write tests for tree view updates
    - _Requirements: 9.2, 9.4_
  - [ ] 11.6 Implement tree view commands
    - Register commands: startAll, stopAll, startService, stopService, restartService
    - Add context menu items to tree view
    - Send RPC requests to daemon process
    - Write tests for command execution
    - _Requirements: 9.3, 10.1, 10.2, 10.4_
  - [ ] 11.7 Implement log output panel
    - Create output channel for logs
    - Implement showLogs command
    - Request logs via RPC and display in output panel
    - Stream real-time logs from daemon events
    - Write tests for log display
    - _Requirements: 9.5, 6.3, 6.4_
  - [ ] 11.8 Implement dmn.json creation wizard
    - Detect projects without dmn.json
    - Show notification offering to create config
    - Scan for package.json scripts and docker-compose.yml
    - Generate initial dmn.json with suggested services
    - Write tests for config generation
    - _Requirements: 12.1, 12.2, 12.3_
  - [ ] 11.9 Implement file watcher for dmn.json changes
    - Watch dmn.json for modifications
    - Reload configuration and update tree view
    - Handle file deletion
    - Write tests for file watching
    - _Requirements: 9.7, 12.4, 12.5_

- [ ] 12. Implement error handling and reporting
  - [ ] 12.1 Create comprehensive error types
    - Define DmnError enum with all error variants
    - Implement Display and Error traits
    - Add context to errors using thiserror
    - Write unit tests for error formatting
    - _Requirements: 8.1, 8.2, 8.3_
  - [ ] 12.2 Implement error propagation in orchestrator
    - Use Result types throughout orchestrator
    - Convert errors to user-friendly messages
    - Emit error events to extension
    - Write integration tests for error scenarios
    - _Requirements: 8.1, 8.2, 8.3, 8.4, 8.5_
  - [ ] 12.3 Implement error display in VS Code extension
    - Show error notifications for critical failures
    - Display error details in output panel
    - Provide actionable error messages
    - Write tests for error display
    - _Requirements: 8.1, 8.2, 8.3_

- [ ] 13. Write integration tests
  - [ ] 13.1 Create test fixtures with sample dmn.json files
    - Simple linear dependencies
    - Complex multi-level dependencies
    - Services with ready conditions
    - Services with env files
    - _Requirements: 1.1, 1.2, 1.5, 2.1, 4.1, 4.3_
  - [ ] 13.2 Write end-to-end orchestration tests
    - Test full startup sequence
    - Test dependency waiting
    - Test graceful shutdown
    - Test service failures and cascades
    - _Requirements: 2.3, 3.1, 4.1, 4.2, 7.1, 7.2_
  - [ ] 13.3 Write extension integration tests
    - Test RPC communication
    - Test tree view updates
    - Test log streaming
    - Test command execution
    - _Requirements: 9.3, 9.4, 9.5_
  - [ ] 13.4 Write MCP integration tests
    - Test tool registration
    - Test read_logs with various parameters
    - Test get_service_status
    - Test list_services
    - _Requirements: 11.1, 11.2, 11.3, 11.4, 11.5_

- [ ] 14. Create build and packaging scripts
  - [ ] 14.1 Set up cross-compilation for Rust binary
    - Configure cargo for Windows, macOS, Linux targets
    - Create build script for all platforms
    - Test binaries on each platform
    - _Requirements: 9.6_
  - [ ] 14.2 Bundle Rust binary with VS Code extension
    - Copy platform-specific binaries to extension
    - Update extension to select correct binary
    - Test extension on all platforms
    - _Requirements: 9.6_
  - [ ] 14.3 Create extension packaging script
    - Use vsce to package extension
    - Include all necessary files
    - Test packaged extension installation
    - _Requirements: 9.1, 9.6_

- [ ] 15. Write documentation
  - [ ] 15.1 Create README with quick start guide
    - Installation instructions
    - Basic usage examples
    - Configuration reference
    - _Requirements: 12.1, 12.2, 12.3_
  - [ ] 15.2 Create dmn.json schema documentation
    - Document all configuration options
    - Provide examples for common scenarios
    - Document ready_when conditions
    - _Requirements: 1.1, 1.2, 1.5, 4.1, 4.3_
  - [ ] 15.3 Create MCP integration guide
    - Document available MCP tools
    - Provide examples for AI agents
    - Document authentication setup
    - _Requirements: 11.1, 11.2, 11.3, 11.4, 11.5_
