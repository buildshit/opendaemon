use thiserror::Error;

/// Comprehensive error type for the dmn orchestrator
/// This consolidates all error types from different modules
#[derive(Debug, Error)]
pub enum DmnError {
    // Configuration errors
    #[error("Configuration error: {0}")]
    Config(#[from] ConfigError),
    
    // Dependency graph errors
    #[error("Dependency graph error: {0}")]
    Graph(#[from] GraphError),
    
    // Process management errors
    #[error("Process error: {0}")]
    Process(#[from] ProcessError),
    
    // Ready watcher errors
    #[error("Ready watcher error: {0}")]
    Ready(#[from] ReadyError),
    
    // Orchestrator errors
    #[error("Orchestrator error: {0}")]
    Orchestrator(#[from] OrchestratorError),
    
    // MCP server errors
    #[error("MCP server error: {0}")]
    Mcp(#[from] McpError),
    
    // RPC errors
    #[error("RPC error: {0}")]
    Rpc(#[from] RpcError),
    
    // Generic IO errors
    #[error("IO error: {0}")]
    Io(#[from] std::io::Error),
    
    // JSON errors
    #[error("JSON error: {0}")]
    Json(#[from] serde_json::Error),
}

/// Configuration-related errors
#[derive(Debug, Error)]
pub enum ConfigError {
    #[error("Failed to read configuration file: {0}")]
    ReadError(String),
    
    #[error("Failed to parse configuration: {0}")]
    ParseError(String),
    
    #[error("Configuration validation failed: {0}")]
    ValidationError(String),
    
    #[error("Missing required field: {0}")]
    MissingField(String),
    
    #[error("Invalid field value for '{field}': {reason}")]
    InvalidField { field: String, reason: String },
    
    #[error("Environment file not found: {0}")]
    EnvFileNotFound(String),
    
    #[error("Failed to parse environment file: {0}")]
    EnvFileParseError(String),
    
    #[error("IO error: {0}")]
    Io(#[from] std::io::Error),
    
    #[error("JSON parse error: {0}")]
    Json(#[from] serde_json::Error),
}

/// Dependency graph errors
#[derive(Debug, Error)]
pub enum GraphError {
    #[error("Cyclic dependency detected: {cycle}")]
    CyclicDependency { cycle: String },
    
    #[error("Service '{service}' not found in configuration")]
    ServiceNotFound { service: String },
    
    #[error("Service '{service}' depends on non-existent service '{dependency}'")]
    MissingDependency { service: String, dependency: String },
    
    #[error("Failed to compute start order: {0}")]
    TopologicalSortError(String),
}

/// Process management errors
#[derive(Debug, Error)]
pub enum ProcessError {
    #[error("Failed to spawn service '{service}': {reason}")]
    SpawnError { service: String, reason: String },
    
    #[error("Service '{service}' is already running")]
    AlreadyRunning { service: String },
    
    #[error("Service '{service}' is not running")]
    NotRunning { service: String },
    
    #[error("Service '{service}' not found")]
    ServiceNotFound { service: String },
    
    #[error("Failed to parse command '{command}': {reason}")]
    CommandParseError { command: String, reason: String },
    
    #[error("Service '{service}' failed to stop within timeout")]
    StopTimeout { service: String },
    
    #[error("Service '{service}' exited with code {exit_code}")]
    ServiceFailed { service: String, exit_code: i32 },
    
    #[error("Failed to read process output: {0}")]
    OutputReadError(String),
    
    #[error("IO error: {0}")]
    Io(#[from] std::io::Error),
}

/// Ready watcher errors
#[derive(Debug, Error)]
pub enum ReadyError {
    #[error("Timeout waiting for service '{service}' to be ready after {timeout_secs} seconds.\nCondition: {condition}\n{details}{troubleshooting}")]
    Timeout {
        service: String,
        timeout_secs: u64,
        condition: String,
        details: String,
        troubleshooting: String,
    },
    
    #[error("Invalid regex pattern for service '{service}': {pattern}")]
    InvalidRegex { service: String, pattern: String },
    
    #[error("HTTP request failed for service '{service}' at URL '{url}': {reason}")]
    HttpError { service: String, url: String, reason: String },
    
    #[error("No log receiver provided for service '{0}'")]
    NoLogReceiver(String),
    
    #[error("Regex error: {0}")]
    RegexError(#[from] regex::Error),
    
    #[error("HTTP client error: {0}")]
    ReqwestError(#[from] reqwest::Error),
}

/// Orchestrator errors
#[derive(Debug, Error)]
pub enum OrchestratorError {
    #[error("Failed to start service '{service}': {reason}")]
    StartError { service: String, reason: String },
    
    #[error("Failed to stop service '{service}': {reason}")]
    StopError { service: String, reason: String },
    
    #[error("Failed to restart service '{service}': {reason}")]
    RestartError { service: String, reason: String },
    
    #[error("Service '{service}' not found in configuration")]
    ServiceNotFound { service: String },
    
    #[error("Failed to start all services: {0}")]
    StartAllError(String),
    
    #[error("Failed to stop all services: {0}")]
    StopAllError(String),
    
    #[error("Dependency error: {0}")]
    DependencyError(String),
    
    #[error("Configuration error: {0}")]
    ConfigError(String),
    
    #[error("Graph error: {0}")]
    GraphError(#[from] GraphError),
    
    #[error("Process error: {0}")]
    ProcessError(#[from] ProcessError),
    
    #[error("Ready error: {0}")]
    ReadyError(String),
}

/// MCP server errors
#[derive(Debug, Error)]
pub enum McpError {
    #[error("Service '{service}' not found")]
    ServiceNotFound { service: String },
    
    #[error("Invalid parameter '{parameter}': {reason}")]
    InvalidParameter { parameter: String, reason: String },
    
    #[error("Authentication required for this operation")]
    AuthenticationRequired,
    
    #[error("Tool '{tool}' not found")]
    ToolNotFound { tool: String },
    
    #[error("Failed to execute tool '{tool}': {reason}")]
    ToolExecutionError { tool: String, reason: String },
    
    #[error("IO error: {0}")]
    Io(#[from] std::io::Error),
    
    #[error("JSON error: {0}")]
    Json(#[from] serde_json::Error),
}

/// RPC server errors
#[derive(Debug, Error)]
pub enum RpcError {
    #[error("Invalid RPC request: {0}")]
    InvalidRequest(String),
    
    #[error("Method '{method}' not found")]
    MethodNotFound { method: String },
    
    #[error("Invalid parameters for method '{method}': {reason}")]
    InvalidParams { method: String, reason: String },
    
    #[error("Internal RPC error: {0}")]
    InternalError(String),
    
    #[error("IO error: {0}")]
    Io(#[from] std::io::Error),
    
    #[error("JSON error: {0}")]
    Json(#[from] serde_json::Error),
}

impl DmnError {
    /// Convert error to a user-friendly message
    pub fn user_message(&self) -> String {
        match self {
            DmnError::Config(e) => format!("Configuration Error: {}", e),
            DmnError::Graph(e) => format!("Dependency Error: {}", e),
            DmnError::Process(e) => format!("Process Error: {}", e),
            DmnError::Ready(e) => format!("Ready Check Error: {}", e),
            DmnError::Orchestrator(e) => format!("Orchestration Error: {}", e),
            DmnError::Mcp(e) => format!("MCP Error: {}", e),
            DmnError::Rpc(e) => format!("RPC Error: {}", e),
            DmnError::Io(e) => format!("IO Error: {}", e),
            DmnError::Json(e) => format!("JSON Error: {}", e),
        }
    }
    
    /// Get a short error category for logging
    pub fn category(&self) -> &'static str {
        match self {
            DmnError::Config(_) => "CONFIG",
            DmnError::Graph(_) => "GRAPH",
            DmnError::Process(_) => "PROCESS",
            DmnError::Ready(_) => "READY",
            DmnError::Orchestrator(_) => "ORCHESTRATOR",
            DmnError::Mcp(_) => "MCP",
            DmnError::Rpc(_) => "RPC",
            DmnError::Io(_) => "IO",
            DmnError::Json(_) => "JSON",
        }
    }
    
    /// Check if this is a recoverable error
    pub fn is_recoverable(&self) -> bool {
        match self {
            DmnError::Config(_) => false, // Config errors require user intervention
            DmnError::Graph(_) => false,  // Graph errors require config changes
            DmnError::Process(ProcessError::AlreadyRunning { .. }) => true,
            DmnError::Process(ProcessError::NotRunning { .. }) => true,
            DmnError::Process(ProcessError::StopTimeout { .. }) => true,
            DmnError::Process(_) => false,
            DmnError::Ready(ReadyError::Timeout { .. }) => true, // Can retry
            DmnError::Ready(_) => false,
            DmnError::Orchestrator(_) => false,
            DmnError::Mcp(_) => true, // MCP errors are usually transient
            DmnError::Rpc(_) => true, // RPC errors are usually transient
            DmnError::Io(_) => false,
            DmnError::Json(_) => false,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_config_error_display() {
        let err = ConfigError::MissingField("version".to_string());
        assert_eq!(err.to_string(), "Missing required field: version");
    }

    #[test]
    fn test_graph_error_display() {
        let err = GraphError::CyclicDependency {
            cycle: "service1 -> service2 -> service1".to_string(),
        };
        assert_eq!(
            err.to_string(),
            "Cyclic dependency detected: service1 -> service2 -> service1"
        );
    }

    #[test]
    fn test_process_error_display() {
        let err = ProcessError::ServiceFailed {
            service: "backend".to_string(),
            exit_code: 1,
        };
        assert_eq!(err.to_string(), "Service 'backend' exited with code 1");
    }

    #[test]
    fn test_ready_error_display() {
        let err = ReadyError::Timeout {
            service: "database".to_string(),
            timeout_secs: 30,
            condition: "log_contains pattern: 'ready'".to_string(),
            details: "Last 2 log lines:\n  Starting database\n  Loading config\n".to_string(),
            troubleshooting: "Troubleshooting:\n- Check the pattern\n- Increase timeout".to_string(),
        };
        let err_str = err.to_string();
        assert!(err_str.contains("database"));
        assert!(err_str.contains("30 seconds"));
        assert!(err_str.contains("log_contains"));
    }

    #[test]
    fn test_dmn_error_user_message() {
        let err = DmnError::Config(ConfigError::MissingField("version".to_string()));
        assert!(err.user_message().contains("Configuration Error"));
        assert!(err.user_message().contains("version"));
    }

    #[test]
    fn test_dmn_error_category() {
        let err = DmnError::Process(ProcessError::ServiceNotFound {
            service: "test".to_string(),
        });
        assert_eq!(err.category(), "PROCESS");
    }

    #[test]
    fn test_dmn_error_is_recoverable() {
        let recoverable = DmnError::Process(ProcessError::AlreadyRunning {
            service: "test".to_string(),
        });
        assert!(recoverable.is_recoverable());

        let not_recoverable = DmnError::Config(ConfigError::MissingField("version".to_string()));
        assert!(!not_recoverable.is_recoverable());
    }

    #[test]
    fn test_error_from_conversions() {
        let io_err = std::io::Error::new(std::io::ErrorKind::NotFound, "file not found");
        let dmn_err: DmnError = io_err.into();
        assert!(matches!(dmn_err, DmnError::Io(_)));
    }
}
