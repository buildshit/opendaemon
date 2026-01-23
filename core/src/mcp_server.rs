use crate::logs::LogLineCount;
use crate::orchestrator::Orchestrator;
use serde::{Deserialize, Serialize};
use serde_json::{json, Value};
use std::io::{self, BufRead, Write};
use std::sync::Arc;
use thiserror::Error;
use tokio::sync::Mutex;

#[derive(Debug, Error)]
pub enum McpError {
    #[error("IO error: {0}")]
    Io(#[from] io::Error),
    #[error("JSON error: {0}")]
    Json(#[from] serde_json::Error),
    #[error("Service not found: {0}")]
    ServiceNotFound(String),
    #[error("Invalid parameter: {0}")]
    InvalidParameter(String),
    #[error("Authentication required")]
    AuthenticationRequired,
}

/// MCP Tool definition
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct McpTool {
    pub name: String,
    pub description: String,
    #[serde(rename = "inputSchema")]
    pub input_schema: Value,
}

/// MCP Tool call request
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct McpToolCall {
    pub name: String,
    pub arguments: Value,
}

/// MCP Tool result
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(tag = "type", rename_all = "lowercase")]
pub enum McpToolResult {
    Success { content: Vec<McpContent> },
    Error { error: String },
}

/// MCP Content type
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(tag = "type", rename_all = "lowercase")]
pub enum McpContent {
    Text { text: String },
}

/// MCP JSON-RPC Request
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct McpRequest {
    pub jsonrpc: String,
    pub id: Option<Value>,
    pub method: String,
    pub params: Option<Value>,
}

/// MCP JSON-RPC Response
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct McpResponse {
    pub jsonrpc: String,
    pub id: Option<Value>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub result: Option<Value>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub error: Option<McpErrorResponse>,
}

/// MCP Error Response
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct McpErrorResponse {
    pub code: i32,
    pub message: String,
}

/// DmnMcpServer - MCP server for AI agent integration
pub struct DmnMcpServer {
    orchestrator: Arc<Mutex<Orchestrator>>,
    authenticated: bool,
}

impl DmnMcpServer {
    /// Create a new MCP server with an orchestrator reference
    /// By default, authentication is disabled (free tier)
    pub fn new(orchestrator: Arc<Mutex<Orchestrator>>) -> Self {
        Self {
            orchestrator,
            authenticated: false, // Default to unauthenticated (free tier)
        }
    }

    /// Create a new MCP server with authentication enabled (Pro tier)
    pub fn new_authenticated(orchestrator: Arc<Mutex<Orchestrator>>) -> Self {
        Self {
            orchestrator,
            authenticated: true,
        }
    }

    /// Set authentication status
    /// This is a placeholder for future Pro authentication implementation
    pub fn set_authenticated(&mut self, authenticated: bool) {
        self.authenticated = authenticated;
    }

    /// Check if the server is authenticated
    pub fn is_authenticated(&self) -> bool {
        self.authenticated
    }

    /// Get the list of available tools
    pub fn get_tools(&self) -> Vec<McpTool> {
        // Always return tools list - authentication is checked per tool call
        vec![
            McpTool {
                name: "read_logs".to_string(),
                description: "Read logs from a specific service".to_string(),
                input_schema: json!({
                    "type": "object",
                    "properties": {
                        "service": {
                            "type": "string",
                            "description": "Name of the service to read logs from"
                        },
                        "lines": {
                            "oneOf": [
                                {"type": "number"},
                                {"type": "string", "enum": ["all"]}
                            ],
                            "description": "Number of lines to return, or 'all' for all available lines"
                        }
                    },
                    "required": ["service", "lines"]
                }),
            },
            McpTool {
                name: "get_service_status".to_string(),
                description: "Get the current status of all services".to_string(),
                input_schema: json!({
                    "type": "object",
                    "properties": {}
                }),
            },
            McpTool {
                name: "list_services".to_string(),
                description: "List all services defined in dmn.json".to_string(),
                input_schema: json!({
                    "type": "object",
                    "properties": {}
                }),
            },
        ]
    }

    /// Handle a tool call
    pub async fn handle_tool_call(&self, call: McpToolCall) -> McpToolResult {
        // Check authentication
        if !self.authenticated {
            return McpToolResult::Error {
                error: "Authentication required".to_string(),
            };
        }

        match call.name.as_str() {
            "read_logs" => self.handle_read_logs(call.arguments).await,
            "get_service_status" => self.handle_get_service_status().await,
            "list_services" => self.handle_list_services().await,
            _ => McpToolResult::Error {
                error: format!("Unknown tool: {}", call.name),
            },
        }
    }

    /// Handle read_logs tool call
    async fn handle_read_logs(&self, arguments: Value) -> McpToolResult {
        // Parse arguments
        let service = match arguments.get("service").and_then(|v| v.as_str()) {
            Some(s) => s,
            None => {
                return McpToolResult::Error {
                    error: "Missing or invalid 'service' parameter".to_string(),
                }
            }
        };

        let lines = match arguments.get("lines") {
            Some(Value::String(s)) if s == "all" => LogLineCount::All,
            Some(Value::Number(n)) => {
                if let Some(num) = n.as_u64() {
                    LogLineCount::Last(num as usize)
                } else {
                    return McpToolResult::Error {
                        error: "Invalid 'lines' parameter: must be a positive number or 'all'".to_string(),
                    };
                }
            }
            _ => {
                return McpToolResult::Error {
                    error: "Missing or invalid 'lines' parameter: must be a positive number or 'all'".to_string(),
                }
            }
        };

        // Get logs from orchestrator
        let orch = self.orchestrator.lock().await;
        
        // Check if service exists
        if !orch.config().services.contains_key(service) {
            return McpToolResult::Error {
                error: format!("Service not found: {}", service),
            };
        }

        let log_buffer = orch.log_buffer.lock().await;
        let log_lines = log_buffer.get_lines(service, lines);
        drop(log_buffer);
        drop(orch);

        // Format logs as text
        let logs_text = log_lines
            .iter()
            .map(|line| format!("[{}] {}", line.timestamp_str(), line.content))
            .collect::<Vec<_>>()
            .join("\n");

        McpToolResult::Success {
            content: vec![McpContent::Text { text: logs_text }],
        }
    }

    /// Handle get_service_status tool call
    async fn handle_get_service_status(&self) -> McpToolResult {
        let orch = self.orchestrator.lock().await;
        
        let mut statuses = Vec::new();
        for service_name in orch.config().services.keys() {
            let status = orch
                .process_manager
                .get_status(service_name)
                .map(|s| format!("{:?}", s))
                .unwrap_or_else(|| "NotStarted".to_string());
            
            statuses.push(format!("{}: {}", service_name, status));
        }

        let status_text = statuses.join("\n");

        McpToolResult::Success {
            content: vec![McpContent::Text { text: status_text }],
        }
    }

    /// Handle list_services tool call
    async fn handle_list_services(&self) -> McpToolResult {
        let orch = self.orchestrator.lock().await;
        let services: Vec<String> = orch.config().services.keys().cloned().collect();
        let services_text = services.join("\n");

        McpToolResult::Success {
            content: vec![McpContent::Text { text: services_text }],
        }
    }

    /// Run the MCP server on stdio
    pub async fn run_stdio(&self) -> Result<(), McpError> {
        let stdin = io::stdin();
        let mut stdout = io::stdout();

        for line in stdin.lock().lines() {
            let line = line?;
            
            // Parse the JSON-RPC request
            let request: McpRequest = match serde_json::from_str(&line) {
                Ok(req) => req,
                Err(e) => {
                    // Send error response
                    let error_response = McpResponse {
                        jsonrpc: "2.0".to_string(),
                        id: None,
                        result: None,
                        error: Some(McpErrorResponse {
                            code: -32700,
                            message: format!("Parse error: {}", e),
                        }),
                    };
                    writeln!(stdout, "{}", serde_json::to_string(&error_response)?)?;
                    stdout.flush()?;
                    continue;
                }
            };

            // Handle the request
            let response = self.handle_request(request).await;
            
            // Send response
            writeln!(stdout, "{}", serde_json::to_string(&response)?)?;
            stdout.flush()?;
        }

        Ok(())
    }

    /// Handle an MCP request
    pub async fn handle_request(&self, request: McpRequest) -> McpResponse {
        match request.method.as_str() {
            "initialize" => {
                // MCP initialization handshake
                McpResponse {
                    jsonrpc: "2.0".to_string(),
                    id: request.id,
                    result: Some(json!({
                        "protocolVersion": "2024-11-05",
                        "capabilities": {
                            "tools": {}
                        },
                        "serverInfo": {
                            "name": "opendaemon",
                            "version": "1.0.0"
                        }
                    })),
                    error: None,
                }
            }
            "notifications/initialized" => {
                // Client confirms initialization - no response needed for notifications
                McpResponse {
                    jsonrpc: "2.0".to_string(),
                    id: request.id,
                    result: Some(json!({})),
                    error: None,
                }
            }
            "tools/list" => {
                let tools = self.get_tools();
                McpResponse {
                    jsonrpc: "2.0".to_string(),
                    id: request.id,
                    result: Some(json!({ "tools": tools })),
                    error: None,
                }
            }
            "tools/call" => {
                // Parse tool call from params
                let tool_call: McpToolCall = match request.params {
                    Some(params) => match serde_json::from_value(params) {
                        Ok(call) => call,
                        Err(e) => {
                            return McpResponse {
                                jsonrpc: "2.0".to_string(),
                                id: request.id,
                                result: None,
                                error: Some(McpErrorResponse {
                                    code: -32602,
                                    message: format!("Invalid params: {}", e),
                                }),
                            };
                        }
                    },
                    None => {
                        return McpResponse {
                            jsonrpc: "2.0".to_string(),
                            id: request.id,
                            result: None,
                            error: Some(McpErrorResponse {
                                code: -32602,
                                message: "Missing params".to_string(),
                            }),
                        };
                    }
                };

                // Execute tool call
                let result = self.handle_tool_call(tool_call).await;
                
                McpResponse {
                    jsonrpc: "2.0".to_string(),
                    id: request.id,
                    result: Some(serde_json::to_value(result).unwrap()),
                    error: None,
                }
            }
            _ => McpResponse {
                jsonrpc: "2.0".to_string(),
                id: request.id,
                result: None,
                error: Some(McpErrorResponse {
                    code: -32601,
                    message: format!("Method not found: {}", request.method),
                }),
            },
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::config::{DmnConfig, ServiceConfig};
    use crate::logs::LogLine;
    use std::collections::HashMap;
    use std::time::SystemTime;

    fn create_test_orchestrator() -> Arc<Mutex<Orchestrator>> {
        let mut services = HashMap::new();
        services.insert(
            "test_service".to_string(),
            ServiceConfig {
                command: "echo test".to_string(),
                depends_on: vec![],
                ready_when: None,
                env_file: None,
            },
        );

        let config = DmnConfig {
            version: "1.0".to_string(),
            services,
        };

        let orchestrator = Orchestrator::new(config).unwrap();
        Arc::new(Mutex::new(orchestrator))
    }

    #[test]
    fn test_mcp_server_new() {
        let orch = create_test_orchestrator();
        let server = DmnMcpServer::new(orch);
        assert!(!server.authenticated); // Default is unauthenticated
    }

    #[test]
    fn test_mcp_server_new_authenticated() {
        let orch = create_test_orchestrator();
        let server = DmnMcpServer::new_authenticated(orch);
        assert!(server.authenticated);
    }

    #[test]
    fn test_set_authenticated() {
        let orch = create_test_orchestrator();
        let mut server = DmnMcpServer::new(orch);
        assert!(!server.is_authenticated());
        
        server.set_authenticated(true);
        assert!(server.is_authenticated());
        
        server.set_authenticated(false);
        assert!(!server.is_authenticated());
    }

    #[test]
    fn test_get_tools() {
        let orch = create_test_orchestrator();
        let server = DmnMcpServer::new(orch);
        let tools = server.get_tools();
        
        assert_eq!(tools.len(), 3);
        assert!(tools.iter().any(|t| t.name == "read_logs"));
        assert!(tools.iter().any(|t| t.name == "get_service_status"));
        assert!(tools.iter().any(|t| t.name == "list_services"));
    }

    #[tokio::test]
    async fn test_handle_list_services() {
        let orch = create_test_orchestrator();
        let server = DmnMcpServer::new_authenticated(orch);
        
        let result = server.handle_list_services().await;
        
        match result {
            McpToolResult::Success { content } => {
                assert_eq!(content.len(), 1);
                match &content[0] {
                    McpContent::Text { text } => {
                        assert!(text.contains("test_service"));
                    }
                }
            }
            _ => panic!("Expected success result"),
        }
    }

    #[tokio::test]
    async fn test_handle_list_services_multiple() {
        let mut services = HashMap::new();
        services.insert(
            "database".to_string(),
            ServiceConfig {
                command: "postgres".to_string(),
                depends_on: vec![],
                ready_when: None,
                env_file: None,
            },
        );
        services.insert(
            "backend".to_string(),
            ServiceConfig {
                command: "cargo run".to_string(),
                depends_on: vec!["database".to_string()],
                ready_when: None,
                env_file: None,
            },
        );
        services.insert(
            "frontend".to_string(),
            ServiceConfig {
                command: "npm start".to_string(),
                depends_on: vec!["backend".to_string()],
                ready_when: None,
                env_file: None,
            },
        );

        let config = DmnConfig {
            version: "1.0".to_string(),
            services,
        };

        let orchestrator = Orchestrator::new(config).unwrap();
        let orch = Arc::new(Mutex::new(orchestrator));
        let server = DmnMcpServer::new_authenticated(orch);
        
        let result = server.handle_list_services().await;
        
        match result {
            McpToolResult::Success { content } => {
                assert_eq!(content.len(), 1);
                match &content[0] {
                    McpContent::Text { text } => {
                        assert!(text.contains("database"));
                        assert!(text.contains("backend"));
                        assert!(text.contains("frontend"));
                        // Should have 3 lines (one per service)
                        let line_count = text.lines().count();
                        assert_eq!(line_count, 3);
                    }
                }
            }
            _ => panic!("Expected success result"),
        }
    }

    #[tokio::test]
    async fn test_handle_list_services_empty() {
        let config = DmnConfig {
            version: "1.0".to_string(),
            services: HashMap::new(),
        };

        let orchestrator = Orchestrator::new(config).unwrap();
        let orch = Arc::new(Mutex::new(orchestrator));
        let server = DmnMcpServer::new_authenticated(orch);
        
        let result = server.handle_list_services().await;
        
        match result {
            McpToolResult::Success { content } => {
                assert_eq!(content.len(), 1);
                match &content[0] {
                    McpContent::Text { text } => {
                        // Should be empty
                        assert_eq!(text, "");
                    }
                }
            }
            _ => panic!("Expected success result"),
        }
    }

    #[tokio::test]
    async fn test_handle_get_service_status() {
        let orch = create_test_orchestrator();
        let server = DmnMcpServer::new_authenticated(orch);
        
        let result = server.handle_get_service_status().await;
        
        match result {
            McpToolResult::Success { content } => {
                assert_eq!(content.len(), 1);
                match &content[0] {
                    McpContent::Text { text } => {
                        assert!(text.contains("test_service"));
                        assert!(text.contains("NotStarted"));
                    }
                }
            }
            _ => panic!("Expected success result"),
        }
    }

    #[tokio::test]
    async fn test_handle_get_service_status_multiple_services() {
        let mut services = HashMap::new();
        services.insert(
            "service1".to_string(),
            ServiceConfig {
                command: "echo test1".to_string(),
                depends_on: vec![],
                ready_when: None,
                env_file: None,
            },
        );
        services.insert(
            "service2".to_string(),
            ServiceConfig {
                command: "echo test2".to_string(),
                depends_on: vec![],
                ready_when: None,
                env_file: None,
            },
        );

        let config = DmnConfig {
            version: "1.0".to_string(),
            services,
        };

        let orchestrator = Orchestrator::new(config).unwrap();
        let orch = Arc::new(Mutex::new(orchestrator));
        let server = DmnMcpServer::new_authenticated(orch);
        
        let result = server.handle_get_service_status().await;
        
        match result {
            McpToolResult::Success { content } => {
                assert_eq!(content.len(), 1);
                match &content[0] {
                    McpContent::Text { text } => {
                        assert!(text.contains("service1"));
                        assert!(text.contains("service2"));
                    }
                }
            }
            _ => panic!("Expected success result"),
        }
    }

    #[tokio::test]
    async fn test_handle_get_service_status_with_running_service() {
        let mut services = HashMap::new();
        services.insert(
            "test_service".to_string(),
            ServiceConfig {
                command: if cfg!(windows) { "timeout /t 10".to_string() } else { "sleep 10".to_string() },
                depends_on: vec![],
                ready_when: None,
                env_file: None,
            },
        );

        let config = DmnConfig {
            version: "1.0".to_string(),
            services,
        };

        let orchestrator = Orchestrator::new(config).unwrap();
        let orch = Arc::new(Mutex::new(orchestrator));
        let server = DmnMcpServer::new_authenticated(orch.clone());
        
        // Start a service
        {
            let mut orch_lock = orch.lock().await;
            let service_config = orch_lock.config().services.get("test_service").unwrap().clone();
            let _ = orch_lock.process_manager.spawn_service("test_service", &service_config).await;
            orch_lock.process_manager.update_status("test_service", crate::process::ServiceStatus::Running);
        }
        
        // Give it a moment to start
        tokio::time::sleep(std::time::Duration::from_millis(100)).await;
        
        let result = server.handle_get_service_status().await;
        
        match result {
            McpToolResult::Success { content } => {
                assert_eq!(content.len(), 1);
                match &content[0] {
                    McpContent::Text { text } => {
                        assert!(text.contains("test_service"));
                        assert!(text.contains("Running"));
                    }
                }
            }
            _ => panic!("Expected success result"),
        }
        
        // Clean up
        {
            let mut orch_lock = orch.lock().await;
            let _ = orch_lock.process_manager.stop_service("test_service").await;
        }
    }

    #[tokio::test]
    async fn test_handle_read_logs_invalid_service() {
        let orch = create_test_orchestrator();
        let server = DmnMcpServer::new_authenticated(orch);
        
        let args = json!({
            "service": "nonexistent",
            "lines": 10
        });
        
        let result = server.handle_read_logs(args).await;
        
        match result {
            McpToolResult::Error { error } => {
                assert!(error.contains("Service not found"));
            }
            _ => panic!("Expected error result"),
        }
    }

    #[tokio::test]
    async fn test_handle_read_logs_missing_params() {
        let orch = create_test_orchestrator();
        let server = DmnMcpServer::new_authenticated(orch);
        
        let args = json!({});
        
        let result = server.handle_read_logs(args).await;
        
        match result {
            McpToolResult::Error { error } => {
                assert!(error.contains("Missing"));
            }
            _ => panic!("Expected error result"),
        }
    }

    #[tokio::test]
    async fn test_handle_tool_call_unknown_tool() {
        let orch = create_test_orchestrator();
        let server = DmnMcpServer::new_authenticated(orch);
        
        let call = McpToolCall {
            name: "unknown_tool".to_string(),
            arguments: json!({}),
        };
        
        let result = server.handle_tool_call(call).await;
        
        match result {
            McpToolResult::Error { error } => {
                assert!(error.contains("Unknown tool"));
            }
            _ => panic!("Expected error result"),
        }
    }

    #[tokio::test]
    async fn test_authentication_check() {
        let orch = create_test_orchestrator();
        let server = DmnMcpServer::new(orch); // Unauthenticated by default
        
        let call = McpToolCall {
            name: "list_services".to_string(),
            arguments: json!({}),
        };
        
        let result = server.handle_tool_call(call).await;
        
        match result {
            McpToolResult::Error { error } => {
                assert!(error.contains("Authentication required"));
            }
            _ => panic!("Expected error result"),
        }
    }

    #[tokio::test]
    async fn test_authentication_check_read_logs() {
        let orch = create_test_orchestrator();
        let server = DmnMcpServer::new(orch);
        
        let call = McpToolCall {
            name: "read_logs".to_string(),
            arguments: json!({
                "service": "test_service",
                "lines": 10
            }),
        };
        
        let result = server.handle_tool_call(call).await;
        
        match result {
            McpToolResult::Error { error } => {
                assert_eq!(error, "Authentication required");
            }
            _ => panic!("Expected error result for unauthenticated read_logs"),
        }
    }

    #[tokio::test]
    async fn test_authentication_check_get_service_status() {
        let orch = create_test_orchestrator();
        let server = DmnMcpServer::new(orch);
        
        let call = McpToolCall {
            name: "get_service_status".to_string(),
            arguments: json!({}),
        };
        
        let result = server.handle_tool_call(call).await;
        
        match result {
            McpToolResult::Error { error } => {
                assert_eq!(error, "Authentication required");
            }
            _ => panic!("Expected error result for unauthenticated get_service_status"),
        }
    }

    #[tokio::test]
    async fn test_authenticated_tools_work() {
        let orch = create_test_orchestrator();
        let server = DmnMcpServer::new_authenticated(orch);
        
        // Test list_services works when authenticated
        let call = McpToolCall {
            name: "list_services".to_string(),
            arguments: json!({}),
        };
        
        let result = server.handle_tool_call(call).await;
        
        match result {
            McpToolResult::Success { .. } => {
                // Success expected
            }
            _ => panic!("Expected success result for authenticated list_services"),
        }
    }

    #[tokio::test]
    async fn test_authenticated_read_logs_works() {
        let orch = create_test_orchestrator();
        let server = DmnMcpServer::new_authenticated(orch);
        
        let call = McpToolCall {
            name: "read_logs".to_string(),
            arguments: json!({
                "service": "test_service",
                "lines": 10
            }),
        };
        
        let result = server.handle_tool_call(call).await;
        
        match result {
            McpToolResult::Success { .. } => {
                // Success expected
            }
            _ => panic!("Expected success result for authenticated read_logs"),
        }
    }

    #[tokio::test]
    async fn test_authenticated_get_service_status_works() {
        let orch = create_test_orchestrator();
        let server = DmnMcpServer::new_authenticated(orch);
        
        let call = McpToolCall {
            name: "get_service_status".to_string(),
            arguments: json!({}),
        };
        
        let result = server.handle_tool_call(call).await;
        
        match result {
            McpToolResult::Success { .. } => {
                // Success expected
            }
            _ => panic!("Expected success result for authenticated get_service_status"),
        }
    }

    #[tokio::test]
    async fn test_authentication_toggle() {
        let orch = create_test_orchestrator();
        let mut server = DmnMcpServer::new(orch);
        
        // Initially unauthenticated
        let call = McpToolCall {
            name: "list_services".to_string(),
            arguments: json!({}),
        };
        
        let result = server.handle_tool_call(call.clone()).await;
        assert!(matches!(result, McpToolResult::Error { .. }));
        
        // Authenticate
        server.set_authenticated(true);
        let result = server.handle_tool_call(call.clone()).await;
        assert!(matches!(result, McpToolResult::Success { .. }));
        
        // Deauthenticate
        server.set_authenticated(false);
        let result = server.handle_tool_call(call).await;
        assert!(matches!(result, McpToolResult::Error { .. }));
    }

    #[tokio::test]
    async fn test_handle_read_logs_with_number() {
        let orch = create_test_orchestrator();
        let server = DmnMcpServer::new_authenticated(orch.clone());
        
        // Add some logs to the buffer
        {
            let orch_lock = orch.lock().await;
            let mut log_buffer = orch_lock.log_buffer.lock().await;
            for i in 1..=10 {
                log_buffer.append(
                    "test_service",
                    LogLine {
                        timestamp: SystemTime::now(),
                        content: format!("Log line {}", i),
                        stream: crate::logs::LogStream::Stdout,
                    },
                );
            }
        }
        
        let args = json!({
            "service": "test_service",
            "lines": 5
        });
        
        let result = server.handle_read_logs(args).await;
        
        match result {
            McpToolResult::Success { content } => {
                assert_eq!(content.len(), 1);
                match &content[0] {
                    McpContent::Text { text } => {
                        // Should contain last 5 lines (6-10)
                        assert!(text.contains("Log line 6"));
                        assert!(text.contains("Log line 10"));
                        // Line 5 should be included since we're getting the last 5 lines
                        // Actually, let's just check that we have 5 lines
                        let line_count = text.lines().count();
                        assert_eq!(line_count, 5);
                    }
                }
            }
            _ => panic!("Expected success result"),
        }
    }

    #[tokio::test]
    async fn test_handle_read_logs_with_all() {
        let orch = create_test_orchestrator();
        let server = DmnMcpServer::new_authenticated(orch.clone());
        
        // Add some logs to the buffer
        {
            let orch_lock = orch.lock().await;
            let mut log_buffer = orch_lock.log_buffer.lock().await;
            for i in 1..=5 {
                log_buffer.append(
                    "test_service",
                    LogLine {
                        timestamp: SystemTime::now(),
                        content: format!("Log line {}", i),
                        stream: crate::logs::LogStream::Stdout,
                    },
                );
            }
        }
        
        let args = json!({
            "service": "test_service",
            "lines": "all"
        });
        
        let result = server.handle_read_logs(args).await;
        
        match result {
            McpToolResult::Success { content } => {
                assert_eq!(content.len(), 1);
                match &content[0] {
                    McpContent::Text { text } => {
                        // Should contain all lines
                        assert!(text.contains("Log line 1"));
                        assert!(text.contains("Log line 5"));
                    }
                }
            }
            _ => panic!("Expected success result"),
        }
    }

    #[tokio::test]
    async fn test_handle_read_logs_empty_buffer() {
        let orch = create_test_orchestrator();
        let server = DmnMcpServer::new_authenticated(orch);
        
        let args = json!({
            "service": "test_service",
            "lines": 10
        });
        
        let result = server.handle_read_logs(args).await;
        
        match result {
            McpToolResult::Success { content } => {
                assert_eq!(content.len(), 1);
                match &content[0] {
                    McpContent::Text { text } => {
                        // Should be empty
                        assert_eq!(text, "");
                    }
                }
            }
            _ => panic!("Expected success result"),
        }
    }

    #[tokio::test]
    async fn test_handle_read_logs_invalid_lines_param() {
        let orch = create_test_orchestrator();
        let server = DmnMcpServer::new_authenticated(orch);
        
        let args = json!({
            "service": "test_service",
            "lines": "invalid"
        });
        
        let result = server.handle_read_logs(args).await;
        
        match result {
            McpToolResult::Error { error } => {
                assert!(error.contains("Missing or invalid"));
            }
            _ => panic!("Expected error result"),
        }
    }

    #[tokio::test]
    async fn test_handle_read_logs_negative_lines() {
        let orch = create_test_orchestrator();
        let server = DmnMcpServer::new_authenticated(orch);
        
        let args = json!({
            "service": "test_service",
            "lines": -5
        });
        
        let result = server.handle_read_logs(args).await;
        
        match result {
            McpToolResult::Error { error } => {
                assert!(error.contains("Missing or invalid") || error.contains("must be a positive number"));
            }
            _ => panic!("Expected error result"),
        }
    }

    #[tokio::test]
    async fn test_handle_request_tools_list() {
        let orch = create_test_orchestrator();
        let server = DmnMcpServer::new_authenticated(orch);
        
        let request = McpRequest {
            jsonrpc: "2.0".to_string(),
            id: Some(json!(1)),
            method: "tools/list".to_string(),
            params: None,
        };
        
        let response = server.handle_request(request).await;
        
        assert_eq!(response.jsonrpc, "2.0");
        assert_eq!(response.id, Some(json!(1)));
        assert!(response.result.is_some());
        assert!(response.error.is_none());
        
        let result = response.result.unwrap();
        let tools = result.get("tools").unwrap().as_array().unwrap();
        assert_eq!(tools.len(), 3);
    }

    #[tokio::test]
    async fn test_handle_request_tools_call_list_services() {
        let orch = create_test_orchestrator();
        let server = DmnMcpServer::new_authenticated(orch);
        
        let request = McpRequest {
            jsonrpc: "2.0".to_string(),
            id: Some(json!(2)),
            method: "tools/call".to_string(),
            params: Some(json!({
                "name": "list_services",
                "arguments": {}
            })),
        };
        
        let response = server.handle_request(request).await;
        
        assert_eq!(response.jsonrpc, "2.0");
        assert_eq!(response.id, Some(json!(2)));
        assert!(response.result.is_some());
        assert!(response.error.is_none());
    }

    #[tokio::test]
    async fn test_handle_request_tools_call_read_logs() {
        let orch = create_test_orchestrator();
        let server = DmnMcpServer::new_authenticated(orch);
        
        let request = McpRequest {
            jsonrpc: "2.0".to_string(),
            id: Some(json!(3)),
            method: "tools/call".to_string(),
            params: Some(json!({
                "name": "read_logs",
                "arguments": {
                    "service": "test_service",
                    "lines": 10
                }
            })),
        };
        
        let response = server.handle_request(request).await;
        
        assert_eq!(response.jsonrpc, "2.0");
        assert_eq!(response.id, Some(json!(3)));
        assert!(response.result.is_some());
        assert!(response.error.is_none());
    }

    #[tokio::test]
    async fn test_handle_request_tools_call_get_service_status() {
        let orch = create_test_orchestrator();
        let server = DmnMcpServer::new_authenticated(orch);
        
        let request = McpRequest {
            jsonrpc: "2.0".to_string(),
            id: Some(json!(4)),
            method: "tools/call".to_string(),
            params: Some(json!({
                "name": "get_service_status",
                "arguments": {}
            })),
        };
        
        let response = server.handle_request(request).await;
        
        assert_eq!(response.jsonrpc, "2.0");
        assert_eq!(response.id, Some(json!(4)));
        assert!(response.result.is_some());
        assert!(response.error.is_none());
    }

    #[tokio::test]
    async fn test_handle_request_unknown_method() {
        let orch = create_test_orchestrator();
        let server = DmnMcpServer::new_authenticated(orch);
        
        let request = McpRequest {
            jsonrpc: "2.0".to_string(),
            id: Some(json!(5)),
            method: "unknown/method".to_string(),
            params: None,
        };
        
        let response = server.handle_request(request).await;
        
        assert_eq!(response.jsonrpc, "2.0");
        assert_eq!(response.id, Some(json!(5)));
        assert!(response.result.is_none());
        assert!(response.error.is_some());
        
        let error = response.error.unwrap();
        assert_eq!(error.code, -32601);
        assert!(error.message.contains("Method not found"));
    }

    #[tokio::test]
    async fn test_handle_request_tools_call_missing_params() {
        let orch = create_test_orchestrator();
        let server = DmnMcpServer::new_authenticated(orch);
        
        let request = McpRequest {
            jsonrpc: "2.0".to_string(),
            id: Some(json!(6)),
            method: "tools/call".to_string(),
            params: None,
        };
        
        let response = server.handle_request(request).await;
        
        assert_eq!(response.jsonrpc, "2.0");
        assert_eq!(response.id, Some(json!(6)));
        assert!(response.result.is_none());
        assert!(response.error.is_some());
        
        let error = response.error.unwrap();
        assert_eq!(error.code, -32602);
        assert!(error.message.contains("Missing params"));
    }

    #[tokio::test]
    async fn test_handle_request_tools_call_invalid_params() {
        let orch = create_test_orchestrator();
        let server = DmnMcpServer::new_authenticated(orch);
        
        let request = McpRequest {
            jsonrpc: "2.0".to_string(),
            id: Some(json!(7)),
            method: "tools/call".to_string(),
            params: Some(json!("invalid")),
        };
        
        let response = server.handle_request(request).await;
        
        assert_eq!(response.jsonrpc, "2.0");
        assert_eq!(response.id, Some(json!(7)));
        assert!(response.result.is_none());
        assert!(response.error.is_some());
        
        let error = response.error.unwrap();
        assert_eq!(error.code, -32602);
        assert!(error.message.contains("Invalid params"));
    }
}
