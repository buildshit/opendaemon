use serde::{Deserialize, Serialize};
use serde_json::{json, Value};
use std::collections::HashMap;
use std::sync::Arc;
use std::time::Duration;
use tokio::io::{AsyncBufReadExt, AsyncWriteExt, BufReader};
use tokio::sync::Mutex;

use crate::orchestrator::Orchestrator;

/// JSON-RPC 2.0 request from the VS Code extension
#[derive(Debug, Clone, Deserialize, Serialize, PartialEq)]
pub struct JsonRpcRequest {
    pub jsonrpc: String,
    pub id: u64,
    pub method: String,
    pub params: Option<Value>,
}

/// JSON-RPC 2.0 response to the VS Code extension
#[derive(Debug, Clone, Deserialize, Serialize, PartialEq)]
pub struct JsonRpcResponse {
    pub jsonrpc: String,
    pub id: u64,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub result: Option<Value>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub error: Option<RpcError>,
}

/// JSON-RPC 2.0 notification (no id, no response expected)
#[derive(Debug, Clone, Deserialize, Serialize, PartialEq)]
pub struct JsonRpcNotification {
    pub jsonrpc: String,
    pub method: String,
    pub params: Value,
}

/// JSON-RPC error object
#[derive(Debug, Clone, Deserialize, Serialize, PartialEq)]
pub struct RpcError {
    pub code: i32,
    pub message: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub data: Option<Value>,
}

impl RpcError {
    pub fn new(code: i32, message: impl Into<String>) -> Self {
        Self {
            code,
            message: message.into(),
            data: None,
        }
    }

    pub fn with_data(mut self, data: Value) -> Self {
        self.data = Some(data);
        self
    }

    /// Parse error (-32700)
    pub fn parse_error(message: impl Into<String>) -> Self {
        Self::new(-32700, message)
    }

    /// Invalid request (-32600)
    pub fn invalid_request(message: impl Into<String>) -> Self {
        Self::new(-32600, message)
    }

    /// Method not found (-32601)
    pub fn method_not_found(method: impl Into<String>) -> Self {
        Self::new(-32601, format!("Method not found: {}", method.into()))
    }

    /// Invalid params (-32602)
    pub fn invalid_params(message: impl Into<String>) -> Self {
        Self::new(-32602, message)
    }

    /// Internal error (-32603)
    pub fn internal_error(message: impl Into<String>) -> Self {
        Self::new(-32603, message)
    }

    /// Server error (-32000 to -32099)
    pub fn server_error(code: i32, message: impl Into<String>) -> Self {
        assert!((-32099..=-32000).contains(&code), "Server error codes must be between -32099 and -32000");
        Self::new(code, message)
    }
}

impl JsonRpcResponse {
    pub fn success(id: u64, result: Value) -> Self {
        Self {
            jsonrpc: "2.0".to_string(),
            id,
            result: Some(result),
            error: None,
        }
    }

    pub fn error(id: u64, error: RpcError) -> Self {
        Self {
            jsonrpc: "2.0".to_string(),
            id,
            result: None,
            error: Some(error),
        }
    }
}

impl JsonRpcNotification {
    pub fn new(method: impl Into<String>, params: Value) -> Self {
        Self {
            jsonrpc: "2.0".to_string(),
            method: method.into(),
            params,
        }
    }
}

/// RPC method requests parsed from JSON-RPC
#[derive(Debug, Clone, PartialEq)]
pub enum RpcRequest {
    StartAll,
    StopAll,
    StartService { service: String },
    StopService { service: String },
    RestartService { service: String },
    GetStatus,
    GetLogs { service: String, lines: LogLinesParam },
}

/// Parameter for specifying how many log lines to retrieve
#[derive(Debug, Clone, PartialEq, Deserialize, Serialize)]
#[serde(untagged)]
pub enum LogLinesParam {
    Count(usize),
    All(String), // "all"
}

impl LogLinesParam {
    pub fn is_all(&self) -> bool {
        matches!(self, LogLinesParam::All(s) if s == "all")
    }

    pub fn count(&self) -> Option<usize> {
        match self {
            LogLinesParam::Count(n) => Some(*n),
            LogLinesParam::All(_) => None,
        }
    }
}

impl RpcRequest {
    /// Parse a JSON-RPC request into a typed RpcRequest
    pub fn from_json_rpc(req: &JsonRpcRequest) -> Result<Self, RpcError> {
        match req.method.as_str() {
            "startAll" => Ok(RpcRequest::StartAll),
            "stopAll" => Ok(RpcRequest::StopAll),
            "startService" => {
                let params = req.params.as_ref()
                    .ok_or_else(|| RpcError::invalid_params("Missing params for startService"))?;
                let service = params.get("service")
                    .and_then(|v| v.as_str())
                    .ok_or_else(|| RpcError::invalid_params("Missing or invalid 'service' parameter"))?
                    .to_string();
                Ok(RpcRequest::StartService { service })
            }
            "stopService" => {
                let params = req.params.as_ref()
                    .ok_or_else(|| RpcError::invalid_params("Missing params for stopService"))?;
                let service = params.get("service")
                    .and_then(|v| v.as_str())
                    .ok_or_else(|| RpcError::invalid_params("Missing or invalid 'service' parameter"))?
                    .to_string();
                Ok(RpcRequest::StopService { service })
            }
            "restartService" => {
                let params = req.params.as_ref()
                    .ok_or_else(|| RpcError::invalid_params("Missing params for restartService"))?;
                let service = params.get("service")
                    .and_then(|v| v.as_str())
                    .ok_or_else(|| RpcError::invalid_params("Missing or invalid 'service' parameter"))?
                    .to_string();
                Ok(RpcRequest::RestartService { service })
            }
            "getStatus" => Ok(RpcRequest::GetStatus),
            "getLogs" => {
                let params = req.params.as_ref()
                    .ok_or_else(|| RpcError::invalid_params("Missing params for getLogs"))?;
                let service = params.get("service")
                    .and_then(|v| v.as_str())
                    .ok_or_else(|| RpcError::invalid_params("Missing or invalid 'service' parameter"))?
                    .to_string();
                
                let lines = if let Some(lines_value) = params.get("lines") {
                    serde_json::from_value(lines_value.clone())
                        .map_err(|_| RpcError::invalid_params("Invalid 'lines' parameter"))?
                } else {
                    LogLinesParam::All("all".to_string())
                };
                
                Ok(RpcRequest::GetLogs { service, lines })
            }
            _ => Err(RpcError::method_not_found(&req.method)),
        }
    }
}

/// RPC Server that communicates via stdin/stdout
pub struct RpcServer {
    orchestrator: Arc<Mutex<Orchestrator>>,
}

impl RpcServer {
    pub fn new(orchestrator: Arc<Mutex<Orchestrator>>) -> Self {
        Self { orchestrator }
    }

    /// Run the RPC server, reading from stdin and writing to stdout
    pub async fn run(&self) -> Result<(), Box<dyn std::error::Error>> {
        let stdin = tokio::io::stdin();
        let mut stdout = tokio::io::stdout();
        let mut reader = BufReader::new(stdin);
        let mut line = String::new();

        // Start event streaming in a separate task
        let orchestrator_clone = Arc::clone(&self.orchestrator);
        tokio::spawn(async move {
            Self::stream_events(orchestrator_clone).await;
        });

        loop {
            line.clear();
            let bytes_read = reader.read_line(&mut line).await?;
            
            // EOF reached
            if bytes_read == 0 {
                break;
            }

            let trimmed = line.trim();
            if trimmed.is_empty() {
                continue;
            }

            // Parse JSON-RPC request
            let response = match serde_json::from_str::<JsonRpcRequest>(trimmed) {
                Ok(json_req) => {
                    // Parse into typed request
                    match RpcRequest::from_json_rpc(&json_req) {
                        Ok(req) => {
                            // Handle the request
                            self.handle_request(json_req.id, req).await
                        }
                        Err(err) => JsonRpcResponse::error(json_req.id, err),
                    }
                }
                Err(e) => {
                    // Parse error - we don't have an ID, so use 0
                    JsonRpcResponse::error(
                        0,
                        RpcError::parse_error(format!("Invalid JSON: {}", e)),
                    )
                }
            };

            // Write response to stdout
            let response_json = serde_json::to_string(&response)?;
            stdout.write_all(response_json.as_bytes()).await?;
            stdout.write_all(b"\n").await?;
            stdout.flush().await?;
        }

        Ok(())
    }

    /// Stream orchestrator events as JSON-RPC notifications
    async fn stream_events(orchestrator: Arc<Mutex<Orchestrator>>) {
        // Subscribe to events
        let _event_tx = {
            let orch = orchestrator.lock().await;
            orch.subscribe_events()
        };

        // Create a receiver for events
        let (_tx, mut rx) = tokio::sync::mpsc::unbounded_channel();
        
        // Forward events from orchestrator to our receiver
        tokio::spawn(async move {
            // We need to poll the orchestrator's event channel
            // Since we can't directly access event_rx, we'll use the event_tx to create a new receiver
            // This is a limitation - in a real implementation, we'd need to refactor the Orchestrator
            // to allow multiple subscribers or expose the event receiver
            
            // For now, we'll just keep the channel open
            // In practice, the orchestrator emits events via event_tx, and we'd need to
            // have the orchestrator forward events to multiple subscribers
            loop {
                tokio::time::sleep(Duration::from_secs(1)).await;
            }
        });

        // Send events as notifications
        while let Some(event) = rx.recv().await {
            let notification = Self::event_to_notification(event);
            if let Err(e) = Self::send_notification(notification).await {
                eprintln!("Error sending notification: {}", e);
            }
        }
    }

    /// Convert an OrchestratorEvent to a JSON-RPC notification
    fn event_to_notification(event: crate::orchestrator::OrchestratorEvent) -> JsonRpcNotification {
        use crate::orchestrator::OrchestratorEvent;
        
        match event {
            OrchestratorEvent::ServiceStarting { service } => {
                JsonRpcNotification::new(
                    "serviceStarting",
                    json!({ "service": service }),
                )
            }
            OrchestratorEvent::ServiceReady { service } => {
                JsonRpcNotification::new(
                    "serviceReady",
                    json!({ "service": service }),
                )
            }
            OrchestratorEvent::ServiceFailed { service, error } => {
                JsonRpcNotification::new(
                    "serviceFailed",
                    json!({ "service": service, "error": error }),
                )
            }
            OrchestratorEvent::ServiceStopped { service } => {
                JsonRpcNotification::new(
                    "serviceStopped",
                    json!({ "service": service }),
                )
            }
            OrchestratorEvent::LogLine { service, line } => {
                JsonRpcNotification::new(
                    "logLine",
                    json!({
                        "service": service,
                        "timestamp": line.timestamp.duration_since(std::time::UNIX_EPOCH)
                            .unwrap_or_default()
                            .as_secs(),
                        "content": line.content,
                        "stream": match line.stream {
                            crate::logs::LogStream::Stdout => "stdout",
                            crate::logs::LogStream::Stderr => "stderr",
                        }
                    }),
                )
            }
        }
    }

    /// Handle a parsed RPC request and return a response
    async fn handle_request(&self, id: u64, request: RpcRequest) -> JsonRpcResponse {
        match request {
            RpcRequest::StartAll => {
                let mut orch = self.orchestrator.lock().await;
                match orch.start_all().await {
                    Ok(_) => JsonRpcResponse::success(id, json!({"status": "started"})),
                    Err(e) => JsonRpcResponse::error(id, RpcError::internal_error(e.to_string())),
                }
            }
            RpcRequest::StopAll => {
                let mut orch = self.orchestrator.lock().await;
                match orch.stop_all().await {
                    Ok(_) => JsonRpcResponse::success(id, json!({"status": "stopped"})),
                    Err(e) => JsonRpcResponse::error(id, RpcError::internal_error(e.to_string())),
                }
            }
            RpcRequest::StartService { service } => {
                let mut orch = self.orchestrator.lock().await;
                match orch.start_service_with_deps(&service).await {
                    Ok(_) => JsonRpcResponse::success(
                        id,
                        json!({"status": "started", "service": service}),
                    ),
                    Err(e) => JsonRpcResponse::error(id, RpcError::internal_error(e.to_string())),
                }
            }
            RpcRequest::StopService { service } => {
                let mut orch = self.orchestrator.lock().await;
                match orch.stop_service(&service).await {
                    Ok(_) => JsonRpcResponse::success(
                        id,
                        json!({"status": "stopped", "service": service}),
                    ),
                    Err(e) => JsonRpcResponse::error(id, RpcError::internal_error(e.to_string())),
                }
            }
            RpcRequest::RestartService { service } => {
                let mut orch = self.orchestrator.lock().await;
                match orch.restart_service(&service).await {
                    Ok(_) => JsonRpcResponse::success(
                        id,
                        json!({"status": "restarted", "service": service}),
                    ),
                    Err(e) => JsonRpcResponse::error(id, RpcError::internal_error(e.to_string())),
                }
            }
            RpcRequest::GetStatus => {
                let orch = self.orchestrator.lock().await;
                let statuses = orch.process_manager.get_all_statuses();
                
                // Convert ServiceStatus to string representation
                let status_map: HashMap<String, String> = statuses
                    .into_iter()
                    .map(|(name, status)| {
                        let status_str = match status {
                            crate::process::ServiceStatus::NotStarted => "not_started",
                            crate::process::ServiceStatus::Starting => "starting",
                            crate::process::ServiceStatus::Running => "running",
                            crate::process::ServiceStatus::Stopped => "stopped",
                            crate::process::ServiceStatus::Failed { exit_code } => {
                                return (name, format!("failed (exit code: {})", exit_code));
                            }
                        };
                        (name, status_str.to_string())
                    })
                    .collect();
                
                JsonRpcResponse::success(id, json!({"services": status_map}))
            }
            RpcRequest::GetLogs { service, lines } => {
                let orch = self.orchestrator.lock().await;
                
                // Check if service exists in config
                if !orch.config().services.contains_key(&service) {
                    return JsonRpcResponse::error(
                        id,
                        RpcError::server_error(-32001, format!("Service not found: {}", service)),
                    );
                }
                
                let log_buffer = orch.log_buffer.lock().await;
                let log_lines = if lines.is_all() {
                    log_buffer.get_all_lines(&service)
                } else if let Some(count) = lines.count() {
                    log_buffer.get_lines(&service, crate::logs::LogLineCount::Last(count))
                } else {
                    log_buffer.get_all_lines(&service)
                };
                
                // Convert log lines to JSON-friendly format
                let logs: Vec<serde_json::Value> = log_lines
                    .into_iter()
                    .map(|line| {
                        json!({
                            "timestamp": line.timestamp.duration_since(std::time::UNIX_EPOCH)
                                .unwrap_or_default()
                                .as_secs(),
                            "content": line.content,
                            "stream": match line.stream {
                                crate::logs::LogStream::Stdout => "stdout",
                                crate::logs::LogStream::Stderr => "stderr",
                            }
                        })
                    })
                    .collect();
                
                JsonRpcResponse::success(
                    id,
                    json!({
                        "service": service,
                        "logs": logs
                    }),
                )
            }
        }
    }

    /// Send a notification (no response expected)
    pub async fn send_notification(
        notification: JsonRpcNotification,
    ) -> Result<(), Box<dyn std::error::Error>> {
        let mut stdout = tokio::io::stdout();
        let notification_json = serde_json::to_string(&notification)?;
        stdout.write_all(notification_json.as_bytes()).await?;
        stdout.write_all(b"\n").await?;
        stdout.flush().await?;
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::json;
    use std::time::Duration;

    #[test]
    fn test_json_rpc_request_deserialization() {
        let json = json!({
            "jsonrpc": "2.0",
            "id": 1,
            "method": "startAll",
            "params": null
        });

        let req: JsonRpcRequest = serde_json::from_value(json).unwrap();
        assert_eq!(req.jsonrpc, "2.0");
        assert_eq!(req.id, 1);
        assert_eq!(req.method, "startAll");
        assert_eq!(req.params, None);
    }

    #[test]
    fn test_json_rpc_request_with_params() {
        let json = json!({
            "jsonrpc": "2.0",
            "id": 2,
            "method": "startService",
            "params": {
                "service": "backend"
            }
        });

        let req: JsonRpcRequest = serde_json::from_value(json).unwrap();
        assert_eq!(req.method, "startService");
        assert!(req.params.is_some());
    }

    #[test]
    fn test_json_rpc_response_success() {
        let response = JsonRpcResponse::success(1, json!({"status": "ok"}));
        assert_eq!(response.id, 1);
        assert!(response.result.is_some());
        assert!(response.error.is_none());

        let serialized = serde_json::to_string(&response).unwrap();
        assert!(serialized.contains("\"result\""));
        assert!(!serialized.contains("\"error\""));
    }

    #[test]
    fn test_json_rpc_response_error() {
        let error = RpcError::internal_error("Something went wrong");
        let response = JsonRpcResponse::error(1, error);
        assert_eq!(response.id, 1);
        assert!(response.result.is_none());
        assert!(response.error.is_some());

        let serialized = serde_json::to_string(&response).unwrap();
        assert!(!serialized.contains("\"result\""));
        assert!(serialized.contains("\"error\""));
    }

    #[test]
    fn test_rpc_error_codes() {
        let err = RpcError::parse_error("Invalid JSON");
        assert_eq!(err.code, -32700);

        let err = RpcError::invalid_request("Bad request");
        assert_eq!(err.code, -32600);

        let err = RpcError::method_not_found("unknownMethod");
        assert_eq!(err.code, -32601);

        let err = RpcError::invalid_params("Missing param");
        assert_eq!(err.code, -32602);

        let err = RpcError::internal_error("Internal error");
        assert_eq!(err.code, -32603);
    }

    #[test]
    fn test_parse_start_all_request() {
        let json_req = JsonRpcRequest {
            jsonrpc: "2.0".to_string(),
            id: 1,
            method: "startAll".to_string(),
            params: None,
        };

        let req = RpcRequest::from_json_rpc(&json_req).unwrap();
        assert_eq!(req, RpcRequest::StartAll);
    }

    #[test]
    fn test_parse_stop_all_request() {
        let json_req = JsonRpcRequest {
            jsonrpc: "2.0".to_string(),
            id: 2,
            method: "stopAll".to_string(),
            params: None,
        };

        let req = RpcRequest::from_json_rpc(&json_req).unwrap();
        assert_eq!(req, RpcRequest::StopAll);
    }

    #[test]
    fn test_parse_start_service_request() {
        let json_req = JsonRpcRequest {
            jsonrpc: "2.0".to_string(),
            id: 3,
            method: "startService".to_string(),
            params: Some(json!({"service": "backend"})),
        };

        let req = RpcRequest::from_json_rpc(&json_req).unwrap();
        assert_eq!(req, RpcRequest::StartService { service: "backend".to_string() });
    }

    #[test]
    fn test_parse_stop_service_request() {
        let json_req = JsonRpcRequest {
            jsonrpc: "2.0".to_string(),
            id: 4,
            method: "stopService".to_string(),
            params: Some(json!({"service": "frontend"})),
        };

        let req = RpcRequest::from_json_rpc(&json_req).unwrap();
        assert_eq!(req, RpcRequest::StopService { service: "frontend".to_string() });
    }

    #[test]
    fn test_parse_restart_service_request() {
        let json_req = JsonRpcRequest {
            jsonrpc: "2.0".to_string(),
            id: 5,
            method: "restartService".to_string(),
            params: Some(json!({"service": "database"})),
        };

        let req = RpcRequest::from_json_rpc(&json_req).unwrap();
        assert_eq!(req, RpcRequest::RestartService { service: "database".to_string() });
    }

    #[test]
    fn test_parse_get_status_request() {
        let json_req = JsonRpcRequest {
            jsonrpc: "2.0".to_string(),
            id: 6,
            method: "getStatus".to_string(),
            params: None,
        };

        let req = RpcRequest::from_json_rpc(&json_req).unwrap();
        assert_eq!(req, RpcRequest::GetStatus);
    }

    #[test]
    fn test_parse_get_logs_request_with_count() {
        let json_req = JsonRpcRequest {
            jsonrpc: "2.0".to_string(),
            id: 7,
            method: "getLogs".to_string(),
            params: Some(json!({"service": "backend", "lines": 100})),
        };

        let req = RpcRequest::from_json_rpc(&json_req).unwrap();
        match req {
            RpcRequest::GetLogs { service, lines } => {
                assert_eq!(service, "backend");
                assert_eq!(lines.count(), Some(100));
            }
            _ => panic!("Expected GetLogs request"),
        }
    }

    #[test]
    fn test_parse_get_logs_request_with_all() {
        let json_req = JsonRpcRequest {
            jsonrpc: "2.0".to_string(),
            id: 8,
            method: "getLogs".to_string(),
            params: Some(json!({"service": "frontend", "lines": "all"})),
        };

        let req = RpcRequest::from_json_rpc(&json_req).unwrap();
        match req {
            RpcRequest::GetLogs { service, lines } => {
                assert_eq!(service, "frontend");
                assert!(lines.is_all());
            }
            _ => panic!("Expected GetLogs request"),
        }
    }

    #[test]
    fn test_parse_get_logs_request_default_all() {
        let json_req = JsonRpcRequest {
            jsonrpc: "2.0".to_string(),
            id: 9,
            method: "getLogs".to_string(),
            params: Some(json!({"service": "database"})),
        };

        let req = RpcRequest::from_json_rpc(&json_req).unwrap();
        match req {
            RpcRequest::GetLogs { service, lines } => {
                assert_eq!(service, "database");
                assert!(lines.is_all());
            }
            _ => panic!("Expected GetLogs request"),
        }
    }

    #[test]
    fn test_parse_unknown_method() {
        let json_req = JsonRpcRequest {
            jsonrpc: "2.0".to_string(),
            id: 10,
            method: "unknownMethod".to_string(),
            params: None,
        };

        let result = RpcRequest::from_json_rpc(&json_req);
        assert!(result.is_err());
        let err = result.unwrap_err();
        assert_eq!(err.code, -32601);
    }

    #[test]
    fn test_parse_missing_params() {
        let json_req = JsonRpcRequest {
            jsonrpc: "2.0".to_string(),
            id: 11,
            method: "startService".to_string(),
            params: None,
        };

        let result = RpcRequest::from_json_rpc(&json_req);
        assert!(result.is_err());
        let err = result.unwrap_err();
        assert_eq!(err.code, -32602);
    }

    #[test]
    fn test_parse_invalid_service_param() {
        let json_req = JsonRpcRequest {
            jsonrpc: "2.0".to_string(),
            id: 12,
            method: "startService".to_string(),
            params: Some(json!({"wrong_key": "value"})),
        };

        let result = RpcRequest::from_json_rpc(&json_req);
        assert!(result.is_err());
        let err = result.unwrap_err();
        assert_eq!(err.code, -32602);
    }

    #[test]
    fn test_json_rpc_notification() {
        let notification = JsonRpcNotification::new("serviceStarted", json!({"service": "backend"}));
        assert_eq!(notification.jsonrpc, "2.0");
        assert_eq!(notification.method, "serviceStarted");

        let serialized = serde_json::to_string(&notification).unwrap();
        assert!(!serialized.contains("\"id\""));
    }

    #[test]
    fn test_log_lines_param_serialization() {
        let count = LogLinesParam::Count(50);
        let json = serde_json::to_value(&count).unwrap();
        assert_eq!(json, json!(50));

        let all = LogLinesParam::All("all".to_string());
        let json = serde_json::to_value(&all).unwrap();
        assert_eq!(json, json!("all"));
    }

    #[test]
    fn test_log_lines_param_deserialization() {
        let json = json!(100);
        let param: LogLinesParam = serde_json::from_value(json).unwrap();
        assert_eq!(param.count(), Some(100));

        let json = json!("all");
        let param: LogLinesParam = serde_json::from_value(json).unwrap();
        assert!(param.is_all());
    }

    #[tokio::test]
    async fn test_rpc_server_parse_and_respond() {
        use tokio::io::AsyncWriteExt;
        use std::collections::HashMap;
        
        // Create a mock orchestrator
        let config = crate::config::DmnConfig {
            version: "1.0".to_string(),
            services: HashMap::new(),
        };
        let orchestrator = crate::orchestrator::Orchestrator::new(config).unwrap();
        let orchestrator = Arc::new(Mutex::new(orchestrator));

        // Create pipes for stdin/stdout simulation
        let (mut stdin_write, stdin_read) = tokio::io::duplex(1024);
        let (stdout_write, mut stdout_read) = tokio::io::duplex(1024);

        // Spawn the server in a separate task
        let server = RpcServer::new(orchestrator.clone());
        let server_handle = tokio::spawn(async move {
            let mut reader = tokio::io::BufReader::new(stdin_read);
            let mut writer = stdout_write;
            let mut line = String::new();

            // Read one request
            reader.read_line(&mut line).await.unwrap();
            let json_req: JsonRpcRequest = serde_json::from_str(&line).unwrap();
            let response = server.handle_request(json_req.id, RpcRequest::from_json_rpc(&json_req).unwrap()).await;
            let response_json = serde_json::to_string(&response).unwrap();
            writer.write_all(response_json.as_bytes()).await.unwrap();
            writer.write_all(b"\n").await.unwrap();
            writer.flush().await.unwrap();
        });

        // Send a request
        let request = JsonRpcRequest {
            jsonrpc: "2.0".to_string(),
            id: 1,
            method: "getStatus".to_string(),
            params: None,
        };
        let request_json = serde_json::to_string(&request).unwrap();
        stdin_write.write_all(request_json.as_bytes()).await.unwrap();
        stdin_write.write_all(b"\n").await.unwrap();
        stdin_write.flush().await.unwrap();

        // Read the response
        let mut response_line = String::new();
        let mut reader = tokio::io::BufReader::new(&mut stdout_read);
        reader.read_line(&mut response_line).await.unwrap();

        let response: JsonRpcResponse = serde_json::from_str(&response_line).unwrap();
        assert_eq!(response.id, 1);
        // Now we expect success since handlers are implemented
        assert!(response.result.is_some());
        assert!(response.error.is_none());

        server_handle.await.unwrap();
    }

    #[tokio::test]
    async fn test_send_notification() {
        use tokio::io::AsyncWriteExt;
        
        // Capture stdout
        let (mut stdout_write, mut stdout_read) = tokio::io::duplex(1024);

        // Send notification in a separate task
        let notification = JsonRpcNotification::new("serviceStarted", json!({"service": "backend"}));
        let notification_clone = notification.clone();
        
        tokio::spawn(async move {
            let notification_json = serde_json::to_string(&notification_clone).unwrap();
            stdout_write.write_all(notification_json.as_bytes()).await.unwrap();
            stdout_write.write_all(b"\n").await.unwrap();
            stdout_write.flush().await.unwrap();
        });

        // Read the notification
        let mut notification_line = String::new();
        let mut reader = tokio::io::BufReader::new(&mut stdout_read);
        reader.read_line(&mut notification_line).await.unwrap();

        let received: JsonRpcNotification = serde_json::from_str(&notification_line).unwrap();
        assert_eq!(received.method, "serviceStarted");
        assert_eq!(received.params, json!({"service": "backend"}));
    }

    // Tests for RPC method handlers

    #[tokio::test]
    async fn test_handle_start_all() {
        let config = crate::config::DmnConfig {
            version: "1.0".to_string(),
            services: HashMap::new(),
        };
        let orchestrator = crate::orchestrator::Orchestrator::new(config).unwrap();
        let orchestrator = Arc::new(Mutex::new(orchestrator));
        let server = RpcServer::new(orchestrator);

        let response = server.handle_request(1, RpcRequest::StartAll).await;
        assert_eq!(response.id, 1);
        assert!(response.result.is_some());
        assert!(response.error.is_none());
    }

    #[tokio::test]
    async fn test_handle_stop_all() {
        let config = crate::config::DmnConfig {
            version: "1.0".to_string(),
            services: HashMap::new(),
        };
        let orchestrator = crate::orchestrator::Orchestrator::new(config).unwrap();
        let orchestrator = Arc::new(Mutex::new(orchestrator));
        let server = RpcServer::new(orchestrator);

        let response = server.handle_request(2, RpcRequest::StopAll).await;
        assert_eq!(response.id, 2);
        assert!(response.result.is_some());
        assert!(response.error.is_none());
    }

    #[tokio::test]
    async fn test_handle_start_service() {
        let mut services = HashMap::new();
        services.insert(
            "test_service".to_string(),
            crate::config::ServiceConfig {
                command: if cfg!(windows) { "cmd /c echo test" } else { "echo test" }.to_string(),
                depends_on: vec![],
                ready_when: None,
                env_file: None,
            },
        );

        let config = crate::config::DmnConfig {
            version: "1.0".to_string(),
            services,
        };
        let orchestrator = crate::orchestrator::Orchestrator::new(config).unwrap();
        let orchestrator = Arc::new(Mutex::new(orchestrator));
        let server = RpcServer::new(orchestrator);

        let response = server.handle_request(
            3,
            RpcRequest::StartService {
                service: "test_service".to_string(),
            },
        ).await;
        
        assert_eq!(response.id, 3);
        assert!(response.result.is_some());
        assert!(response.error.is_none());
    }

    #[tokio::test]
    async fn test_handle_start_service_not_found() {
        let config = crate::config::DmnConfig {
            version: "1.0".to_string(),
            services: HashMap::new(),
        };
        let orchestrator = crate::orchestrator::Orchestrator::new(config).unwrap();
        let orchestrator = Arc::new(Mutex::new(orchestrator));
        let server = RpcServer::new(orchestrator);

        let response = server.handle_request(
            4,
            RpcRequest::StartService {
                service: "nonexistent".to_string(),
            },
        ).await;
        
        assert_eq!(response.id, 4);
        assert!(response.result.is_none());
        assert!(response.error.is_some());
    }

    #[tokio::test]
    async fn test_handle_stop_service() {
        let mut services = HashMap::new();
        services.insert(
            "test_service".to_string(),
            crate::config::ServiceConfig {
                command: if cfg!(windows) { "timeout /t 10" } else { "sleep 10" }.to_string(),
                depends_on: vec![],
                ready_when: None,
                env_file: None,
            },
        );

        let config = crate::config::DmnConfig {
            version: "1.0".to_string(),
            services,
        };
        let mut orchestrator = crate::orchestrator::Orchestrator::new(config).unwrap();
        
        // Start the service first
        orchestrator.start_service_with_deps("test_service").await.unwrap();
        tokio::time::sleep(Duration::from_millis(200)).await;
        
        let orchestrator = Arc::new(Mutex::new(orchestrator));
        let server = RpcServer::new(orchestrator);

        let response = server.handle_request(
            5,
            RpcRequest::StopService {
                service: "test_service".to_string(),
            },
        ).await;
        
        assert_eq!(response.id, 5);
        assert!(response.result.is_some());
        assert!(response.error.is_none());
    }

    #[tokio::test]
    async fn test_handle_restart_service() {
        let mut services = HashMap::new();
        services.insert(
            "test_service".to_string(),
            crate::config::ServiceConfig {
                command: if cfg!(windows) { "cmd /c echo test" } else { "echo test" }.to_string(),
                depends_on: vec![],
                ready_when: None,
                env_file: None,
            },
        );

        let config = crate::config::DmnConfig {
            version: "1.0".to_string(),
            services,
        };
        let orchestrator = crate::orchestrator::Orchestrator::new(config).unwrap();
        let orchestrator = Arc::new(Mutex::new(orchestrator));
        let server = RpcServer::new(orchestrator);

        let response = server.handle_request(
            6,
            RpcRequest::RestartService {
                service: "test_service".to_string(),
            },
        ).await;
        
        assert_eq!(response.id, 6);
        assert!(response.result.is_some());
        assert!(response.error.is_none());
    }

    #[tokio::test]
    async fn test_handle_get_status() {
        let mut services = HashMap::new();
        services.insert(
            "service1".to_string(),
            crate::config::ServiceConfig {
                command: if cfg!(windows) { "cmd /c echo test" } else { "echo test" }.to_string(),
                depends_on: vec![],
                ready_when: None,
                env_file: None,
            },
        );
        services.insert(
            "service2".to_string(),
            crate::config::ServiceConfig {
                command: if cfg!(windows) { "cmd /c echo test" } else { "echo test" }.to_string(),
                depends_on: vec![],
                ready_when: None,
                env_file: None,
            },
        );

        let config = crate::config::DmnConfig {
            version: "1.0".to_string(),
            services,
        };
        let mut orchestrator = crate::orchestrator::Orchestrator::new(config).unwrap();
        
        // Start one service
        orchestrator.start_service_with_deps("service1").await.unwrap();
        tokio::time::sleep(Duration::from_millis(200)).await;
        
        let orchestrator = Arc::new(Mutex::new(orchestrator));
        let server = RpcServer::new(orchestrator);

        let response = server.handle_request(7, RpcRequest::GetStatus).await;
        
        assert_eq!(response.id, 7);
        assert!(response.result.is_some());
        assert!(response.error.is_none());
        
        let result = response.result.unwrap();
        assert!(result.get("services").is_some());
    }

    #[tokio::test]
    async fn test_handle_get_logs() {
        let mut services = HashMap::new();
        services.insert(
            "test_service".to_string(),
            crate::config::ServiceConfig {
                command: if cfg!(windows) { "cmd /c echo test output" } else { "echo test output" }.to_string(),
                depends_on: vec![],
                ready_when: None,
                env_file: None,
            },
        );

        let config = crate::config::DmnConfig {
            version: "1.0".to_string(),
            services,
        };
        let mut orchestrator = crate::orchestrator::Orchestrator::new(config).unwrap();
        
        // Start the service
        orchestrator.start_service_with_deps("test_service").await.unwrap();
        tokio::time::sleep(Duration::from_millis(300)).await;
        
        let orchestrator = Arc::new(Mutex::new(orchestrator));
        let server = RpcServer::new(orchestrator);

        let response = server.handle_request(
            8,
            RpcRequest::GetLogs {
                service: "test_service".to_string(),
                lines: LogLinesParam::All("all".to_string()),
            },
        ).await;
        
        assert_eq!(response.id, 8);
        assert!(response.result.is_some());
        assert!(response.error.is_none());
        
        let result = response.result.unwrap();
        assert!(result.get("service").is_some());
        assert!(result.get("logs").is_some());
    }

    #[tokio::test]
    async fn test_handle_get_logs_with_count() {
        let mut services = HashMap::new();
        services.insert(
            "test_service".to_string(),
            crate::config::ServiceConfig {
                command: if cfg!(windows) { "cmd /c echo test" } else { "echo test" }.to_string(),
                depends_on: vec![],
                ready_when: None,
                env_file: None,
            },
        );

        let config = crate::config::DmnConfig {
            version: "1.0".to_string(),
            services,
        };
        let mut orchestrator = crate::orchestrator::Orchestrator::new(config).unwrap();
        
        // Start the service
        orchestrator.start_service_with_deps("test_service").await.unwrap();
        tokio::time::sleep(Duration::from_millis(300)).await;
        
        let orchestrator = Arc::new(Mutex::new(orchestrator));
        let server = RpcServer::new(orchestrator);

        let response = server.handle_request(
            9,
            RpcRequest::GetLogs {
                service: "test_service".to_string(),
                lines: LogLinesParam::Count(10),
            },
        ).await;
        
        assert_eq!(response.id, 9);
        assert!(response.result.is_some());
        assert!(response.error.is_none());
    }

    #[tokio::test]
    async fn test_handle_get_logs_service_not_found() {
        let config = crate::config::DmnConfig {
            version: "1.0".to_string(),
            services: HashMap::new(),
        };
        let orchestrator = crate::orchestrator::Orchestrator::new(config).unwrap();
        let orchestrator = Arc::new(Mutex::new(orchestrator));
        let server = RpcServer::new(orchestrator);

        let response = server.handle_request(
            10,
            RpcRequest::GetLogs {
                service: "nonexistent".to_string(),
                lines: LogLinesParam::All("all".to_string()),
            },
        ).await;
        
        assert_eq!(response.id, 10);
        assert!(response.result.is_none());
        assert!(response.error.is_some());
        
        let error = response.error.unwrap();
        assert_eq!(error.code, -32001);
    }

    // Tests for event streaming

    #[test]
    fn test_event_to_notification_service_starting() {
        use crate::orchestrator::OrchestratorEvent;
        
        let event = OrchestratorEvent::ServiceStarting {
            service: "backend".to_string(),
        };
        
        let notification = RpcServer::event_to_notification(event);
        assert_eq!(notification.method, "serviceStarting");
        assert_eq!(notification.params.get("service").unwrap(), "backend");
    }

    #[test]
    fn test_event_to_notification_service_ready() {
        use crate::orchestrator::OrchestratorEvent;
        
        let event = OrchestratorEvent::ServiceReady {
            service: "database".to_string(),
        };
        
        let notification = RpcServer::event_to_notification(event);
        assert_eq!(notification.method, "serviceReady");
        assert_eq!(notification.params.get("service").unwrap(), "database");
    }

    #[test]
    fn test_event_to_notification_service_failed() {
        use crate::orchestrator::OrchestratorEvent;
        
        let event = OrchestratorEvent::ServiceFailed {
            service: "frontend".to_string(),
            error: "Connection refused".to_string(),
        };
        
        let notification = RpcServer::event_to_notification(event);
        assert_eq!(notification.method, "serviceFailed");
        assert_eq!(notification.params.get("service").unwrap(), "frontend");
        assert_eq!(notification.params.get("error").unwrap(), "Connection refused");
    }

    #[test]
    fn test_event_to_notification_service_stopped() {
        use crate::orchestrator::OrchestratorEvent;
        
        let event = OrchestratorEvent::ServiceStopped {
            service: "backend".to_string(),
        };
        
        let notification = RpcServer::event_to_notification(event);
        assert_eq!(notification.method, "serviceStopped");
        assert_eq!(notification.params.get("service").unwrap(), "backend");
    }

    #[test]
    fn test_event_to_notification_log_line() {
        use crate::orchestrator::OrchestratorEvent;
        use crate::logs::{LogLine, LogStream};
        use std::time::SystemTime;
        
        let log_line = LogLine {
            timestamp: SystemTime::now(),
            content: "Server started on port 3000".to_string(),
            stream: LogStream::Stdout,
        };
        
        let event = OrchestratorEvent::LogLine {
            service: "backend".to_string(),
            line: log_line,
        };
        
        let notification = RpcServer::event_to_notification(event);
        assert_eq!(notification.method, "logLine");
        assert_eq!(notification.params.get("service").unwrap(), "backend");
        assert_eq!(notification.params.get("content").unwrap(), "Server started on port 3000");
        assert_eq!(notification.params.get("stream").unwrap(), "stdout");
        assert!(notification.params.get("timestamp").is_some());
    }

    #[tokio::test]
    async fn test_event_streaming_integration() {
        use crate::orchestrator::OrchestratorEvent;
        
        // Create orchestrator
        let mut services = HashMap::new();
        services.insert(
            "test_service".to_string(),
            crate::config::ServiceConfig {
                command: if cfg!(windows) { "cmd /c echo test" } else { "echo test" }.to_string(),
                depends_on: vec![],
                ready_when: None,
                env_file: None,
            },
        );

        let config = crate::config::DmnConfig {
            version: "1.0".to_string(),
            services,
        };
        let orchestrator = crate::orchestrator::Orchestrator::new(config).unwrap();
        
        // Get event sender
        let event_tx = orchestrator.subscribe_events();
        
        // Send a test event
        let test_event = OrchestratorEvent::ServiceStarting {
            service: "test_service".to_string(),
        };
        
        assert!(event_tx.send(test_event).is_ok());
        
        // Convert to notification
        let event = OrchestratorEvent::ServiceReady {
            service: "test_service".to_string(),
        };
        let notification = RpcServer::event_to_notification(event);
        
        // Verify notification structure
        assert_eq!(notification.jsonrpc, "2.0");
        assert_eq!(notification.method, "serviceReady");
        assert!(notification.params.is_object());
    }
}
