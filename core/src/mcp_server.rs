use crate::logs::{LogLine, LogLineCount, LogStream};
use crate::orchestrator::{Orchestrator, OrchestratorEvent};
use crate::rpc::{JsonRpcRequest, JsonRpcResponse};
use regex::{Regex, RegexBuilder};
use serde::ser::SerializeStruct;
use serde::{Deserialize, Serialize, Serializer};
use serde_json::{json, Value};
use std::io::{self, BufRead, Write};
use std::path::{Path, PathBuf};
use std::sync::Arc;
use thiserror::Error;
use tokio::io::{AsyncBufReadExt, AsyncWriteExt, BufReader};
use tokio::net::TcpStream;
use tokio::sync::{broadcast, Mutex};
use tokio::time::{timeout, Duration, Instant};

const DAEMON_CONNECT_TIMEOUT: Duration = Duration::from_millis(1200);
const DAEMON_RESPONSE_TIMEOUT: Duration = Duration::from_secs(20);
const DAEMON_IPC_FILENAME: &str = "daemon-ipc.json";

#[derive(Debug, Clone, Deserialize)]
struct DaemonIpcInfo {
    config_path: String,
    address: String,
}

#[derive(Debug, Error)]
enum DaemonBridgeError {
    #[error("daemon IPC not available")]
    NotAvailable,
    #[error("daemon RPC error: {0}")]
    Rpc(String),
}

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
    #[serde(skip_serializing_if = "Option::is_none")]
    pub annotations: Option<Value>,
}

/// MCP Tool call request
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct McpToolCall {
    pub name: String,
    pub arguments: Value,
}

/// MCP Tool result
#[derive(Debug, Clone)]
pub enum McpToolResult {
    Success { content: Vec<McpContent> },
    Error { error: String },
}

impl McpToolResult {
    fn success_text(text: String) -> Self {
        Self::Success {
            content: vec![McpContent::Text { text }],
        }
    }

    fn error_text(error: impl Into<String>) -> Self {
        Self::Error {
            error: error.into(),
        }
    }
}

impl Serialize for McpToolResult {
    fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
    where
        S: Serializer,
    {
        let mut state = serializer.serialize_struct("McpToolResult", 2)?;
        match self {
            McpToolResult::Success { content } => {
                state.serialize_field("content", content)?;
                state.serialize_field("isError", &false)?;
            }
            McpToolResult::Error { error } => {
                let content = vec![McpContent::Text {
                    text: error.to_string(),
                }];
                state.serialize_field("content", &content)?;
                state.serialize_field("isError", &true)?;
            }
        }
        state.end()
    }
}

/// MCP Content type
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(tag = "type", rename_all = "lowercase")]
pub enum McpContent {
    Text { text: String },
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum StreamFilter {
    Stdout,
    Stderr,
    Both,
}

impl StreamFilter {
    fn from_value(value: Option<&Value>) -> Result<Self, McpToolResult> {
        match value.and_then(|v| v.as_str()) {
            None | Some("both") => Ok(StreamFilter::Both),
            Some("stdout") => Ok(StreamFilter::Stdout),
            Some("stderr") => Ok(StreamFilter::Stderr),
            Some(other) => Err(McpToolResult::error_text(format!(
                "Invalid 'stream' value '{}': expected 'stdout', 'stderr', or 'both'",
                other
            ))),
        }
    }

    fn matches(self, stream: &LogStream) -> bool {
        match self {
            StreamFilter::Both => true,
            StreamFilter::Stdout => matches!(stream, LogStream::Stdout),
            StreamFilter::Stderr => matches!(stream, LogStream::Stderr),
        }
    }
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
    config_path: Option<PathBuf>,
    authenticated: bool,
}

impl DmnMcpServer {
    /// Create a new MCP server with an orchestrator reference
    /// By default, authentication is disabled (free tier)
    pub fn new(orchestrator: Arc<Mutex<Orchestrator>>) -> Self {
        Self {
            orchestrator,
            config_path: None,
            authenticated: false, // Default to unauthenticated (free tier)
        }
    }

    /// Create a new MCP server with authentication enabled (Pro tier)
    pub fn new_authenticated(orchestrator: Arc<Mutex<Orchestrator>>) -> Self {
        Self {
            orchestrator,
            config_path: None,
            authenticated: true,
        }
    }

    /// Attach a config path so MCP can bridge to an active extension daemon
    /// for the same workspace/runtime when available.
    pub fn with_config_path(mut self, config_path: impl Into<PathBuf>) -> Self {
        self.config_path = Some(config_path.into());
        self
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
        // Always return tools list - authentication is checked per tool call.
        vec![
            McpTool {
                name: "read_logs".to_string(),
                description: "Read buffered logs from a specific service".to_string(),
                input_schema: json!({
                    "type": "object",
                    "properties": {
                        "service": {
                            "type": "string",
                            "description": "Name of the service to read logs from"
                        },
                        "lines": {
                            "oneOf": [
                                {"type": "integer", "minimum": 1},
                                {"type": "string", "enum": ["all"]}
                            ],
                            "description": "Number of lines to return, or 'all' for all buffered lines"
                        },
                        "contains": {
                            "type": "string",
                            "description": "Optional text filter. Only log lines containing this text are returned"
                        },
                        "caseSensitive": {
                            "type": "boolean",
                            "description": "Whether the 'contains' filter should be case-sensitive (default: false)"
                        },
                        "stream": {
                            "type": "string",
                            "enum": ["stdout", "stderr", "both"],
                            "description": "Optional stream filter (default: both)"
                        }
                    },
                    "required": ["service", "lines"]
                }),
                annotations: Some(json!({ "readOnlyHint": true })),
            },
            McpTool {
                name: "watch_logs".to_string(),
                description: "Watch live logs with optional duration, pattern matching, and filtering".to_string(),
                input_schema: json!({
                    "type": "object",
                    "properties": {
                        "service": {
                            "type": "string",
                            "description": "Name of the service to watch"
                        },
                        "durationSeconds": {
                            "type": "integer",
                            "minimum": 1,
                            "description": "Watch duration in seconds"
                        },
                        "untilPattern": {
                            "type": "string",
                            "description": "Stop when this regex pattern is seen in a matching log line"
                        },
                        "timeoutSeconds": {
                            "type": "integer",
                            "minimum": 0,
                            "description": "Optional absolute timeout (0 disables timeout)"
                        },
                        "pollIntervalMs": {
                            "type": "integer",
                            "minimum": 50,
                            "description": "Polling interval for idle periods (default: 250)"
                        },
                        "maxLines": {
                            "type": "integer",
                            "minimum": 1,
                            "description": "Maximum number of matching lines to return (default: 200)"
                        },
                        "includeExisting": {
                            "type": "boolean",
                            "description": "Include buffered logs that already exist before watching (default: false)"
                        },
                        "includePatterns": {
                            "type": "array",
                            "items": { "type": "string" },
                            "description": "Optional regex patterns; at least one must match each returned line"
                        },
                        "excludePatterns": {
                            "type": "array",
                            "items": { "type": "string" },
                            "description": "Optional regex patterns; matching lines are excluded"
                        },
                        "caseSensitive": {
                            "type": "boolean",
                            "description": "Whether regex matching is case-sensitive (default: false)"
                        },
                        "stream": {
                            "type": "string",
                            "enum": ["stdout", "stderr", "both"],
                            "description": "Optional stream filter (default: both)"
                        }
                    },
                    "required": ["service"],
                    "allOf": [
                        {
                            "anyOf": [
                                { "required": ["durationSeconds"] },
                                { "required": ["untilPattern"] }
                            ]
                        }
                    ]
                }),
                annotations: Some(json!({ "readOnlyHint": true })),
            },
            McpTool {
                name: "get_service_status".to_string(),
                description: "Get the current status of all services".to_string(),
                input_schema: json!({
                    "type": "object",
                    "properties": {}
                }),
                annotations: Some(json!({ "readOnlyHint": true })),
            },
            McpTool {
                name: "list_services".to_string(),
                description: "List all services defined in dmn.json".to_string(),
                input_schema: json!({
                    "type": "object",
                    "properties": {}
                }),
                annotations: Some(json!({ "readOnlyHint": true })),
            },
            McpTool {
                name: "start_service".to_string(),
                description: "Start a service and its dependencies".to_string(),
                input_schema: json!({
                    "type": "object",
                    "properties": {
                        "service": {
                            "type": "string",
                            "description": "Name of the service to start"
                        }
                    },
                    "required": ["service"]
                }),
                annotations: None,
            },
            McpTool {
                name: "stop_service".to_string(),
                description: "Stop a running service".to_string(),
                input_schema: json!({
                    "type": "object",
                    "properties": {
                        "service": {
                            "type": "string",
                            "description": "Name of the service to stop"
                        }
                    },
                    "required": ["service"]
                }),
                annotations: None,
            },
            McpTool {
                name: "restart_service".to_string(),
                description: "Restart a service and any affected dependencies".to_string(),
                input_schema: json!({
                    "type": "object",
                    "properties": {
                        "service": {
                            "type": "string",
                            "description": "Name of the service to restart"
                        }
                    },
                    "required": ["service"]
                }),
                annotations: None,
            },
        ]
    }

    /// Handle a tool call
    pub async fn handle_tool_call(&self, call: McpToolCall) -> McpToolResult {
        // Check authentication
        if !self.authenticated {
            Self::log_mcp_event("WARN", "Rejected tool call: authentication required".to_string());
            return McpToolResult::Error {
                error: "Authentication required".to_string(),
            };
        }

        let tool_name = call.name.clone();
        let argument_summary = Self::summarize_tool_arguments(&call.arguments);
        let started_at = Instant::now();
        Self::log_mcp_event(
            "INFO",
            format!("Tool call started: {} ({})", tool_name, argument_summary),
        );

        let result = match call.name.as_str() {
            "read_logs" => self.handle_read_logs(call.arguments).await,
            "watch_logs" => self.handle_watch_logs(call.arguments).await,
            "get_service_status" => self.handle_get_service_status().await,
            "list_services" => self.handle_list_services().await,
            "start_service" => self.handle_start_service(call.arguments).await,
            "stop_service" => self.handle_stop_service(call.arguments).await,
            "restart_service" => self.handle_restart_service(call.arguments).await,
            _ => McpToolResult::error_text(format!("Unknown tool: {}", call.name)),
        };

        let elapsed_ms = started_at.elapsed().as_millis();
        Self::log_mcp_event(
            "INFO",
            format!(
                "Tool call finished: {} -> {} ({}ms)",
                tool_name,
                Self::summarize_tool_result(&result),
                elapsed_ms
            ),
        );

        result
    }

    fn log_mcp_event(level: &str, message: String) {
        let epoch_ms = std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .map(|duration| duration.as_millis())
            .unwrap_or(0);
        eprintln!("[OpenDaemon MCP {}] [{}] {}", epoch_ms, level, message);
    }

    fn summarize_tool_arguments(arguments: &Value) -> String {
        let Some(object) = arguments.as_object() else {
            return format!("non-object args: {}", Self::summarize_json_value(arguments));
        };

        if object.is_empty() {
            return "no arguments".to_string();
        }

        let mut keys: Vec<&String> = object.keys().collect();
        keys.sort_unstable();

        keys.into_iter()
            .map(|key| {
                let summary = object
                    .get(key)
                    .map(Self::summarize_json_value)
                    .unwrap_or_else(|| "null".to_string());
                format!("{}={}", key, summary)
            })
            .collect::<Vec<_>>()
            .join(", ")
    }

    fn summarize_json_value(value: &Value) -> String {
        match value {
            Value::Null => "null".to_string(),
            Value::Bool(boolean) => boolean.to_string(),
            Value::Number(number) => number.to_string(),
            Value::String(text) => {
                let max_chars = 80;
                let char_count = text.chars().count();
                if char_count <= max_chars {
                    format!("{:?}", text)
                } else {
                    let prefix: String = text.chars().take(max_chars).collect();
                    format!("{:?}...({} chars)", prefix, char_count)
                }
            }
            Value::Array(items) => format!("[{} item(s)]", items.len()),
            Value::Object(map) => format!("{{{} key(s)}}", map.len()),
        }
    }

    fn summarize_tool_result(result: &McpToolResult) -> String {
        match result {
            McpToolResult::Success { content } => {
                format!("success ({} content item(s))", content.len())
            }
            McpToolResult::Error { error } => format!("error ({})", error),
        }
    }

    fn resolved_config_path(&self) -> Option<PathBuf> {
        let configured = self.config_path.as_ref()?;
        if configured.is_absolute() {
            Some(configured.clone())
        } else {
            std::env::current_dir().ok().map(|cwd| cwd.join(configured))
        }
    }

    fn daemon_info_path(config_path: &Path) -> PathBuf {
        let config_dir = config_path.parent().unwrap_or_else(|| Path::new("."));
        config_dir.join(".dmn").join(DAEMON_IPC_FILENAME)
    }

    fn config_identity(config_path: &Path) -> String {
        std::fs::canonicalize(config_path)
            .unwrap_or_else(|_| config_path.to_path_buf())
            .to_string_lossy()
            .to_string()
    }

    fn format_daemon_timestamp(seconds: u64) -> String {
        format!("{seconds}.000")
    }

    async fn request_via_daemon(
        &self,
        method: &str,
        params: Option<Value>,
    ) -> Result<Value, DaemonBridgeError> {
        let Some(config_path) = self.resolved_config_path() else {
            return Err(DaemonBridgeError::NotAvailable);
        };
        let info_path = Self::daemon_info_path(&config_path);
        if !info_path.exists() {
            return Err(DaemonBridgeError::NotAvailable);
        }

        let info_contents =
            std::fs::read_to_string(&info_path).map_err(|_| DaemonBridgeError::NotAvailable)?;
        let info = serde_json::from_str::<DaemonIpcInfo>(&info_contents)
            .map_err(|_| DaemonBridgeError::NotAvailable)?;
        if info.config_path != Self::config_identity(&config_path) {
            return Err(DaemonBridgeError::NotAvailable);
        }

        let stream = match timeout(DAEMON_CONNECT_TIMEOUT, TcpStream::connect(&info.address)).await
        {
            Ok(Ok(stream)) => stream,
            _ => {
                let _ = std::fs::remove_file(&info_path);
                return Err(DaemonBridgeError::NotAvailable);
            }
        };

        let request = JsonRpcRequest {
            jsonrpc: "2.0".to_string(),
            id: 1,
            method: method.to_string(),
            params,
        };
        let payload = serde_json::to_string(&request).map_err(|_| DaemonBridgeError::NotAvailable)?;

        let (read_half, mut write_half) = stream.into_split();
        write_half
            .write_all(payload.as_bytes())
            .await
            .map_err(|_| DaemonBridgeError::NotAvailable)?;
        write_half
            .write_all(b"\n")
            .await
            .map_err(|_| DaemonBridgeError::NotAvailable)?;
        write_half
            .flush()
            .await
            .map_err(|_| DaemonBridgeError::NotAvailable)?;

        let mut reader = BufReader::new(read_half);
        let mut line = String::new();
        let bytes_read = timeout(DAEMON_RESPONSE_TIMEOUT, reader.read_line(&mut line))
            .await
            .map_err(|_| DaemonBridgeError::NotAvailable)?
            .map_err(|_| DaemonBridgeError::NotAvailable)?;
        if bytes_read == 0 {
            return Err(DaemonBridgeError::NotAvailable);
        }

        let response: JsonRpcResponse =
            serde_json::from_str(line.trim()).map_err(|_| DaemonBridgeError::NotAvailable)?;
        if let Some(error) = response.error {
            return Err(DaemonBridgeError::Rpc(error.message));
        }

        Ok(response.result.unwrap_or(Value::Null))
    }

    fn parse_service_name<'a>(arguments: &'a Value) -> Result<&'a str, McpToolResult> {
        arguments
            .get("service")
            .and_then(|v| v.as_str())
            .ok_or_else(|| McpToolResult::error_text("Missing or invalid 'service' parameter"))
    }

    fn parse_optional_u64(arguments: &Value, key: &str) -> Result<Option<u64>, McpToolResult> {
        match arguments.get(key) {
            None => Ok(None),
            Some(Value::Number(number)) => number.as_u64().map(Some).ok_or_else(|| {
                McpToolResult::error_text(format!(
                    "Invalid '{}' parameter: expected a non-negative integer",
                    key
                ))
            }),
            _ => Err(McpToolResult::error_text(format!(
                "Invalid '{}' parameter: expected a non-negative integer",
                key
            ))),
        }
    }

    fn parse_regex_list(
        arguments: &Value,
        key: &str,
        case_sensitive: bool,
    ) -> Result<Vec<Regex>, McpToolResult> {
        let Some(pattern_value) = arguments.get(key) else {
            return Ok(Vec::new());
        };
        let Some(patterns) = pattern_value.as_array() else {
            return Err(McpToolResult::error_text(format!(
                "Invalid '{}' parameter: expected an array of regex patterns",
                key
            )));
        };

        let mut compiled_patterns = Vec::with_capacity(patterns.len());
        for pattern_value in patterns {
            let Some(pattern) = pattern_value.as_str() else {
                return Err(McpToolResult::error_text(format!(
                    "Invalid '{}' parameter: all patterns must be strings",
                    key
                )));
            };
            let compiled = RegexBuilder::new(pattern)
                .case_insensitive(!case_sensitive)
                .build()
                .map_err(|e| {
                    McpToolResult::error_text(format!(
                        "Invalid regex in '{}': '{}' ({})",
                        key, pattern, e
                    ))
                })?;
            compiled_patterns.push(compiled);
        }

        Ok(compiled_patterns)
    }

    fn line_matches_filters(
        line: &LogLine,
        stream_filter: StreamFilter,
        include_patterns: &[Regex],
        exclude_patterns: &[Regex],
    ) -> bool {
        if !stream_filter.matches(&line.stream) {
            return false;
        }

        if !include_patterns.is_empty()
            && !include_patterns.iter().any(|pattern| pattern.is_match(&line.content))
        {
            return false;
        }

        !exclude_patterns
            .iter()
            .any(|pattern| pattern.is_match(&line.content))
    }

    fn stream_name(stream: &LogStream) -> &'static str {
        match stream {
            LogStream::Stdout => "stdout",
            LogStream::Stderr => "stderr",
        }
    }

    fn format_log_line(line: &LogLine) -> String {
        format!(
            "[{}] [{}] {}",
            line.timestamp_str(),
            Self::stream_name(&line.stream),
            line.content
        )
    }

    fn format_watch_output(
        service: &str,
        stop_reason: &str,
        matched_until_pattern: bool,
        lagged_events: u64,
        lines: &[String],
    ) -> String {
        let mut output = String::new();
        output.push_str(&format!("service: {}\n", service));
        output.push_str(&format!("stop_reason: {}\n", stop_reason));
        output.push_str(&format!(
            "matched_until_pattern: {}\n",
            matched_until_pattern
        ));
        output.push_str(&format!("lines_captured: {}\n", lines.len()));
        output.push_str(&format!("dropped_events: {}\n", lagged_events));
        output.push('\n');
        output.push_str("logs:\n");

        if lines.is_empty() {
            output.push_str("(no matching log lines captured)");
        } else {
            output.push_str(&lines.join("\n"));
        }

        output
    }

    fn next_wait_duration(
        start: Instant,
        duration_limit: Option<Duration>,
        timeout_limit: Option<Duration>,
        poll_interval: Duration,
    ) -> Duration {
        let mut wait_duration = poll_interval;

        for limit in [duration_limit, timeout_limit].into_iter().flatten() {
            let elapsed = start.elapsed();
            if elapsed >= limit {
                return Duration::from_millis(1);
            }
            let remaining = limit - elapsed;
            if remaining < wait_duration {
                wait_duration = remaining;
            }
        }

        if wait_duration.is_zero() {
            Duration::from_millis(1)
        } else {
            wait_duration
        }
    }

    fn service_status_text(status: Option<crate::process::ServiceStatus>, is_ready: bool) -> String {
        match status {
            Some(crate::process::ServiceStatus::NotStarted) | None => "not_started".to_string(),
            Some(crate::process::ServiceStatus::Starting) => {
                if is_ready {
                    "running".to_string()
                } else {
                    "starting".to_string()
                }
            }
            Some(crate::process::ServiceStatus::Running) => "running".to_string(),
            Some(crate::process::ServiceStatus::Stopped) => "stopped".to_string(),
            Some(crate::process::ServiceStatus::Failed { exit_code }) => {
                format!("failed (exit code: {})", exit_code)
            }
        }
    }

    /// Handle read_logs tool call
    async fn handle_read_logs(&self, arguments: Value) -> McpToolResult {
        let service = match Self::parse_service_name(&arguments) {
            Ok(service) => service,
            Err(error) => return error,
        };

        let lines = match arguments.get("lines") {
            Some(Value::String(s)) if s == "all" => LogLineCount::All,
            Some(Value::Number(n)) => {
                if let Some(num) = n.as_u64() {
                    if num == 0 {
                        return McpToolResult::error_text(
                            "Invalid 'lines' parameter: must be >= 1 or 'all'",
                        );
                    }
                    LogLineCount::Last(num as usize)
                } else {
                    return McpToolResult::error_text(
                        "Invalid 'lines' parameter: must be >= 1 or 'all'",
                    );
                }
            }
            _ => {
                return McpToolResult::error_text(
                    "Missing or invalid 'lines' parameter: must be >= 1 or 'all'",
                );
            }
        };

        let case_sensitive = arguments
            .get("caseSensitive")
            .and_then(|v| v.as_bool())
            .unwrap_or(false);
        let contains = arguments
            .get("contains")
            .and_then(|v| v.as_str())
            .map(|s| s.to_string());
        let stream_filter = match StreamFilter::from_value(arguments.get("stream")) {
            Ok(filter) => filter,
            Err(error) => return error,
        };

        let daemon_lines_param = match lines {
            LogLineCount::All => json!("all"),
            LogLineCount::Last(count) => json!(count),
        };

        match self
            .request_via_daemon(
                "getLogs",
                Some(json!({
                    "service": service,
                    "lines": daemon_lines_param
                })),
            )
            .await
        {
            Ok(result) => {
                let Some(log_items) = result.get("logs").and_then(|value| value.as_array()) else {
                    return McpToolResult::error_text(
                        "Failed to read logs from daemon: invalid response payload",
                    );
                };

                let filtered_lines = log_items
                    .iter()
                    .filter_map(|item| {
                        let content = item.get("content")?.as_str()?.to_string();
                        let stream_name = item.get("stream").and_then(|v| v.as_str()).unwrap_or("stdout");
                        let stream = match stream_name {
                            "stdout" => LogStream::Stdout,
                            "stderr" => LogStream::Stderr,
                            _ => LogStream::Stdout,
                        };
                        if !stream_filter.matches(&stream) {
                            return None;
                        }
                        if let Some(contains_text) = &contains {
                            let contains_match = if case_sensitive {
                                content.contains(contains_text)
                            } else {
                                content
                                    .to_lowercase()
                                    .contains(&contains_text.to_lowercase())
                            };
                            if !contains_match {
                                return None;
                            }
                        }

                        let timestamp = item
                            .get("timestamp")
                            .and_then(|v| v.as_u64())
                            .map(Self::format_daemon_timestamp)
                            .unwrap_or_else(|| "0.000".to_string());
                        Some(format!("[{}] [{}] {}", timestamp, stream_name, content))
                    })
                    .collect::<Vec<_>>();

                return McpToolResult::success_text(filtered_lines.join("\n"));
            }
            Err(DaemonBridgeError::NotAvailable) => {}
            Err(DaemonBridgeError::Rpc(error)) => {
                return McpToolResult::error_text(format!(
                    "Failed to read logs for '{}': {}",
                    service, error
                ));
            }
        }

        // Get logs from orchestrator.
        let orch = self.orchestrator.lock().await;

        // Check if service exists.
        if !orch.config().services.contains_key(service) {
            return McpToolResult::error_text(format!("Service not found: {}", service));
        }

        let log_buffer = orch.log_buffer.lock().await;
        let log_lines = log_buffer.get_lines(service, lines);
        drop(log_buffer);
        drop(orch);

        let filtered_lines = log_lines
            .into_iter()
            .filter(|line| stream_filter.matches(&line.stream))
            .filter(|line| {
                if let Some(contains_text) = &contains {
                    if case_sensitive {
                        line.content.contains(contains_text)
                    } else {
                        line.content
                            .to_lowercase()
                            .contains(&contains_text.to_lowercase())
                    }
                } else {
                    true
                }
            })
            .map(|line| Self::format_log_line(&line))
            .collect::<Vec<_>>();

        McpToolResult::success_text(filtered_lines.join("\n"))
    }

    /// Handle watch_logs tool call
    async fn handle_watch_logs(&self, arguments: Value) -> McpToolResult {
        let service = match Self::parse_service_name(&arguments) {
            Ok(service) => service.to_string(),
            Err(error) => return error,
        };

        let duration_seconds = match Self::parse_optional_u64(&arguments, "durationSeconds") {
            Ok(value) => value,
            Err(error) => return error,
        };
        let timeout_seconds = match Self::parse_optional_u64(&arguments, "timeoutSeconds") {
            Ok(Some(0)) => None,
            Ok(value) => value,
            Err(error) => return error,
        };

        let case_sensitive = arguments
            .get("caseSensitive")
            .and_then(|v| v.as_bool())
            .unwrap_or(false);
        let stream_filter = match StreamFilter::from_value(arguments.get("stream")) {
            Ok(filter) => filter,
            Err(error) => return error,
        };

        let include_patterns = match Self::parse_regex_list(&arguments, "includePatterns", case_sensitive)
        {
            Ok(patterns) => patterns,
            Err(error) => return error,
        };
        let exclude_patterns = match Self::parse_regex_list(&arguments, "excludePatterns", case_sensitive)
        {
            Ok(patterns) => patterns,
            Err(error) => return error,
        };

        let until_pattern = match arguments.get("untilPattern").and_then(|v| v.as_str()) {
            Some(pattern) => {
                let compiled = RegexBuilder::new(pattern)
                    .case_insensitive(!case_sensitive)
                    .build()
                    .map_err(|e| {
                        McpToolResult::error_text(format!(
                            "Invalid 'untilPattern' regex '{}': {}",
                            pattern, e
                        ))
                    });
                match compiled {
                    Ok(regex) => Some(regex),
                    Err(error) => return error,
                }
            }
            None => None,
        };

        if duration_seconds.is_none() && until_pattern.is_none() {
            return McpToolResult::error_text(
                "watch_logs requires either 'durationSeconds' or 'untilPattern'",
            );
        }

        let poll_interval_ms = match Self::parse_optional_u64(&arguments, "pollIntervalMs") {
            Ok(Some(value)) => {
                if value < 50 {
                    return McpToolResult::error_text(
                        "Invalid 'pollIntervalMs': must be >= 50 milliseconds",
                    );
                }
                value
            }
            Ok(None) => 250,
            Err(error) => return error,
        };
        let max_lines = match Self::parse_optional_u64(&arguments, "maxLines") {
            Ok(Some(value)) => {
                if value == 0 {
                    return McpToolResult::error_text("Invalid 'maxLines': must be >= 1");
                }
                value as usize
            }
            Ok(None) => 200,
            Err(error) => return error,
        };
        let include_existing = arguments
            .get("includeExisting")
            .and_then(|v| v.as_bool())
            .unwrap_or(false);

        let mut log_receiver = {
            let orch = self.orchestrator.lock().await;
            if !orch.config().services.contains_key(&service) {
                return McpToolResult::error_text(format!("Service not found: {}", service));
            }
            orch.subscribe_events()
        };

        let mut captured_lines: Vec<String> = Vec::new();
        let mut matched_until_pattern = false;
        let mut stop_reason = String::new();
        let mut lagged_events: u64 = 0;

        if include_existing {
            let existing_lines = {
                let orch = self.orchestrator.lock().await;
                let log_buffer = orch.log_buffer.lock().await;
                log_buffer.get_all_lines(&service)
            };

            for line in existing_lines {
                if !Self::line_matches_filters(
                    &line,
                    stream_filter,
                    &include_patterns,
                    &exclude_patterns,
                ) {
                    continue;
                }

                if let Some(until_regex) = &until_pattern {
                    if until_regex.is_match(&line.content) {
                        matched_until_pattern = true;
                    }
                }

                captured_lines.push(Self::format_log_line(&line));

                if captured_lines.len() >= max_lines {
                    stop_reason = "max_lines_reached".to_string();
                    break;
                }
                if matched_until_pattern {
                    stop_reason = "until_pattern_matched".to_string();
                    break;
                }
            }
        }

        if stop_reason.is_empty() {
            let duration_limit = duration_seconds.map(Duration::from_secs);
            let timeout_limit = timeout_seconds.map(Duration::from_secs);
            let poll_interval = Duration::from_millis(poll_interval_ms);
            let started_at = Instant::now();

            loop {
                if captured_lines.len() >= max_lines {
                    stop_reason = "max_lines_reached".to_string();
                    break;
                }
                if matched_until_pattern {
                    stop_reason = "until_pattern_matched".to_string();
                    break;
                }
                if let Some(limit) = duration_limit {
                    if started_at.elapsed() >= limit {
                        stop_reason = "duration_elapsed".to_string();
                        break;
                    }
                }
                if let Some(limit) = timeout_limit {
                    if started_at.elapsed() >= limit {
                        stop_reason = "timeout_elapsed".to_string();
                        break;
                    }
                }

                let wait_duration = Self::next_wait_duration(
                    started_at,
                    duration_limit,
                    timeout_limit,
                    poll_interval,
                );

                match tokio::time::timeout(wait_duration, log_receiver.recv()).await {
                    Ok(Ok(OrchestratorEvent::LogLine {
                        service: event_service,
                        line,
                    })) => {
                        if event_service != service {
                            continue;
                        }

                        if !Self::line_matches_filters(
                            &line,
                            stream_filter,
                            &include_patterns,
                            &exclude_patterns,
                        ) {
                            continue;
                        }

                        if let Some(until_regex) = &until_pattern {
                            if until_regex.is_match(&line.content) {
                                matched_until_pattern = true;
                            }
                        }

                        captured_lines.push(Self::format_log_line(&line));
                    }
                    Ok(Ok(_)) => {}
                    Ok(Err(broadcast::error::RecvError::Lagged(skipped))) => {
                        lagged_events += skipped as u64;
                    }
                    Ok(Err(broadcast::error::RecvError::Closed)) => {
                        stop_reason = "event_stream_closed".to_string();
                        break;
                    }
                    Err(_) => {
                        // Poll timeout. Continue checking termination conditions.
                    }
                }
            }
        }

        if stop_reason.is_empty() {
            stop_reason = "completed".to_string();
        }

        McpToolResult::success_text(Self::format_watch_output(
            &service,
            &stop_reason,
            matched_until_pattern,
            lagged_events,
            &captured_lines,
        ))
    }

    /// Handle get_service_status tool call
    async fn handle_get_service_status(&self) -> McpToolResult {
        match self.request_via_daemon("getStatus", None).await {
            Ok(result) => {
                let Some(services) = result.get("services").and_then(|value| value.as_object()) else {
                    return McpToolResult::error_text(
                        "Failed to get service status from daemon: invalid response payload",
                    );
                };

                let mut service_names: Vec<String> = services.keys().cloned().collect();
                service_names.sort();
                let statuses = service_names
                    .iter()
                    .filter_map(|service_name| {
                        services
                            .get(service_name)
                            .and_then(|status| status.as_str())
                            .map(|status| format!("{}: {}", service_name, status))
                    })
                    .collect::<Vec<_>>();

                return McpToolResult::success_text(statuses.join("\n"));
            }
            Err(DaemonBridgeError::NotAvailable) => {}
            Err(DaemonBridgeError::Rpc(error)) => {
                return McpToolResult::error_text(format!(
                    "Failed to get service status via daemon: {}",
                    error
                ));
            }
        }

        let orch = self.orchestrator.lock().await;

        let mut service_names: Vec<String> = orch.config().services.keys().cloned().collect();
        service_names.sort();

        let ready_watcher = orch.ready_watcher().lock().await;
        let statuses = service_names
            .iter()
            .map(|service_name| {
                let status = Self::service_status_text(
                    orch.process_manager.get_status(service_name),
                    ready_watcher.is_ready(service_name),
                );
                format!("{}: {}", service_name, status)
            })
            .collect::<Vec<_>>();
        drop(ready_watcher);
        drop(orch);

        McpToolResult::success_text(statuses.join("\n"))
    }

    /// Handle list_services tool call
    async fn handle_list_services(&self) -> McpToolResult {
        let orch = self.orchestrator.lock().await;
        let mut services: Vec<String> = orch.config().services.keys().cloned().collect();
        services.sort();
        drop(orch);

        McpToolResult::success_text(services.join("\n"))
    }

    /// Handle start_service tool call
    async fn handle_start_service(&self, arguments: Value) -> McpToolResult {
        let service = match Self::parse_service_name(&arguments) {
            Ok(service) => service.to_string(),
            Err(error) => return error,
        };

        let orch = self.orchestrator.lock().await;
        if !orch.config().services.contains_key(&service) {
            return McpToolResult::error_text(format!("Service not found: {}", service));
        }
        drop(orch);

        match self
            .request_via_daemon(
                "startService",
                Some(json!({ "service": service.clone() })),
            )
            .await
        {
            Ok(_) => {
                return McpToolResult::success_text(format!(
                    "Start requested for '{}' (dependencies included).",
                    service
                ));
            }
            Err(DaemonBridgeError::NotAvailable) => {}
            Err(DaemonBridgeError::Rpc(error)) => {
                return McpToolResult::error_text(format!(
                    "Failed to start '{}' via daemon: {}",
                    service, error
                ));
            }
        }

        let mut orch = self.orchestrator.lock().await;

        match orch.start_service_with_deps(&service).await {
            Ok(_) => McpToolResult::success_text(format!(
                "Start requested for '{}' (dependencies included).",
                service
            )),
            Err(error) => {
                McpToolResult::error_text(format!("Failed to start '{}': {}", service, error))
            }
        }
    }

    /// Handle stop_service tool call
    async fn handle_stop_service(&self, arguments: Value) -> McpToolResult {
        let service = match Self::parse_service_name(&arguments) {
            Ok(service) => service.to_string(),
            Err(error) => return error,
        };

        let orch = self.orchestrator.lock().await;
        if !orch.config().services.contains_key(&service) {
            return McpToolResult::error_text(format!("Service not found: {}", service));
        }
        drop(orch);

        match self
            .request_via_daemon(
                "stopService",
                Some(json!({ "service": service.clone() })),
            )
            .await
        {
            Ok(_) => return McpToolResult::success_text(format!("Stop requested for '{}'.", service)),
            Err(DaemonBridgeError::NotAvailable) => {}
            Err(DaemonBridgeError::Rpc(error)) => {
                return McpToolResult::error_text(format!(
                    "Failed to stop '{}' via daemon: {}",
                    service, error
                ));
            }
        }

        let mut orch = self.orchestrator.lock().await;

        match orch.stop_service(&service).await {
            Ok(_) => McpToolResult::success_text(format!("Stop requested for '{}'.", service)),
            Err(error) => {
                McpToolResult::error_text(format!("Failed to stop '{}': {}", service, error))
            }
        }
    }

    /// Handle restart_service tool call
    async fn handle_restart_service(&self, arguments: Value) -> McpToolResult {
        let service = match Self::parse_service_name(&arguments) {
            Ok(service) => service.to_string(),
            Err(error) => return error,
        };

        let orch = self.orchestrator.lock().await;
        if !orch.config().services.contains_key(&service) {
            return McpToolResult::error_text(format!("Service not found: {}", service));
        }
        drop(orch);

        match self
            .request_via_daemon(
                "restartService",
                Some(json!({ "service": service.clone() })),
            )
            .await
        {
            Ok(_) => {
                return McpToolResult::success_text(format!("Restart requested for '{}'.", service));
            }
            Err(DaemonBridgeError::NotAvailable) => {}
            Err(DaemonBridgeError::Rpc(error)) => {
                return McpToolResult::error_text(format!(
                    "Failed to restart '{}' via daemon: {}",
                    service, error
                ));
            }
        }

        let mut orch = self.orchestrator.lock().await;

        match orch.restart_service(&service).await {
            Ok(_) => McpToolResult::success_text(format!("Restart requested for '{}'.", service)),
            Err(error) => {
                McpToolResult::error_text(format!("Failed to restart '{}': {}", service, error))
            }
        }
    }

    /// Run the MCP server on stdio
    pub async fn run_stdio(&self) -> Result<(), McpError> {
        let stdin = io::stdin();
        let mut stdout = io::stdout();
        Self::log_mcp_event("INFO", "Starting stdio MCP server loop".to_string());

        for line in stdin.lock().lines() {
            let line = line?;

            // Parse the JSON-RPC request
            let request: McpRequest = match serde_json::from_str(&line) {
                Ok(req) => req,
                Err(e) => {
                    Self::log_mcp_event("WARN", format!("Invalid JSON-RPC payload: {}", e));
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

            let request_id = request
                .id
                .as_ref()
                .map(|id| id.to_string())
                .unwrap_or_else(|| "notification".to_string());
            let request_method = request.method.clone();
            Self::log_mcp_event(
                "DEBUG",
                format!("Received request: method={} id={}", request_method, request_id),
            );

            // Handle the request
            let should_respond = request.id.is_some();
            let response = self.handle_request(request).await;
            if !should_respond {
                Self::log_mcp_event(
                    "DEBUG",
                    format!("Handled notification without response: {}", request_method),
                );
                continue;
            }

            if let Some(error) = &response.error {
                Self::log_mcp_event(
                    "WARN",
                    format!(
                        "Responding with error for method {}: {} ({})",
                        request_method, error.message, error.code
                    ),
                );
            }

            // Send response
            writeln!(stdout, "{}", serde_json::to_string(&response)?)?;
            stdout.flush()?;
        }

        Self::log_mcp_event("INFO", "MCP stdio loop exited".to_string());
        Ok(())
    }

    /// Handle an MCP request
    pub async fn handle_request(&self, request: McpRequest) -> McpResponse {
        let method = request.method.clone();
        match request.method.as_str() {
            "initialize" => {
                Self::log_mcp_event("INFO", "Handling initialize request".to_string());
                // MCP initialization handshake
                McpResponse {
                    jsonrpc: "2.0".to_string(),
                    id: request.id,
                    result: Some(json!({
                        "protocolVersion": "2025-06-18",
                        "capabilities": {
                            "tools": {
                                "listChanged": false
                            }
                        },
                        "serverInfo": {
                            "name": "opendaemon",
                            "version": env!("CARGO_PKG_VERSION")
                        }
                    })),
                    error: None,
                }
            }
            "ping" => {
                Self::log_mcp_event("DEBUG", "Handling ping request".to_string());
                McpResponse {
                    jsonrpc: "2.0".to_string(),
                    id: request.id,
                    result: Some(json!({})),
                    error: None,
                }
            }
            "notifications/initialized" => {
                Self::log_mcp_event("DEBUG", "Received notifications/initialized".to_string());
                // Client confirms initialization - no response needed for notifications
                McpResponse {
                    jsonrpc: "2.0".to_string(),
                    id: request.id,
                    result: Some(json!({})),
                    error: None,
                }
            }
            "notifications/cancelled" => {
                Self::log_mcp_event("DEBUG", "Received notifications/cancelled".to_string());
                McpResponse {
                    jsonrpc: "2.0".to_string(),
                    id: request.id,
                    result: Some(json!({})),
                    error: None,
                }
            }
            "tools/list" => {
                let tools = self.get_tools();
                Self::log_mcp_event(
                    "INFO",
                    format!("Handling tools/list request ({} tools)", tools.len()),
                );
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
                            Self::log_mcp_event(
                                "WARN",
                                format!("tools/call rejected: invalid params ({})", e),
                            );
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
                        Self::log_mcp_event(
                            "WARN",
                            "tools/call rejected: missing params".to_string(),
                        );
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

                Self::log_mcp_event(
                    "INFO",
                    format!("Handling tools/call request for '{}'", tool_call.name),
                );

                // Execute tool call
                let result = self.handle_tool_call(tool_call).await;

                McpResponse {
                    jsonrpc: "2.0".to_string(),
                    id: request.id,
                    result: Some(serde_json::to_value(result).unwrap()),
                    error: None,
                }
            }
            _ => {
                Self::log_mcp_event("WARN", format!("Unknown method requested: {}", method));
                McpResponse {
                    jsonrpc: "2.0".to_string(),
                    id: request.id,
                    result: None,
                    error: Some(McpErrorResponse {
                        code: -32601,
                        message: format!("Method not found: {}", request.method),
                    }),
                }
            }
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

        assert_eq!(tools.len(), 7);
        assert!(tools.iter().any(|t| t.name == "read_logs"));
        assert!(tools.iter().any(|t| t.name == "watch_logs"));
        assert!(tools.iter().any(|t| t.name == "get_service_status"));
        assert!(tools.iter().any(|t| t.name == "list_services"));
        assert!(tools.iter().any(|t| t.name == "start_service"));
        assert!(tools.iter().any(|t| t.name == "stop_service"));
        assert!(tools.iter().any(|t| t.name == "restart_service"));
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
                        assert!(text.contains("not_started"));
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
                command: if cfg!(windows) {
                    "timeout /t 10".to_string()
                } else {
                    "sleep 10".to_string()
                },
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
            let service_config = orch_lock
                .config()
                .services
                .get("test_service")
                .unwrap()
                .clone();
            let _ = orch_lock
                .process_manager
                .spawn_service("test_service", &service_config)
                .await;
            orch_lock
                .process_manager
                .update_status("test_service", crate::process::ServiceStatus::Running);
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
                        assert!(text.contains("running"));
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
                assert!(
                    error.contains("Missing or invalid")
                        || error.contains("must be a positive number")
                        || error.contains("must be >= 1")
                );
            }
            _ => panic!("Expected error result"),
        }
    }

    #[tokio::test]
    async fn test_handle_start_stop_restart_service_tools() {
        let mut services = HashMap::new();
        services.insert(
            "runtime_service".to_string(),
            ServiceConfig {
                command: if cfg!(windows) {
                    "timeout /t 10".to_string()
                } else {
                    "sleep 10".to_string()
                },
                depends_on: vec![],
                ready_when: None,
                env_file: None,
            },
        );

        let config = DmnConfig {
            version: "1.0".to_string(),
            services,
        };

        let orchestrator = Arc::new(Mutex::new(Orchestrator::new(config).unwrap()));
        let server = DmnMcpServer::new_authenticated(orchestrator.clone());

        let start_result = server
            .handle_start_service(json!({ "service": "runtime_service" }))
            .await;
        assert!(matches!(start_result, McpToolResult::Success { .. }));

        tokio::time::sleep(std::time::Duration::from_millis(150)).await;

        let stop_result = server
            .handle_stop_service(json!({ "service": "runtime_service" }))
            .await;
        assert!(matches!(stop_result, McpToolResult::Success { .. }));

        let restart_result = server
            .handle_restart_service(json!({ "service": "runtime_service" }))
            .await;
        assert!(matches!(restart_result, McpToolResult::Success { .. }));

        // Cleanup
        {
            let mut orch = orchestrator.lock().await;
            let _ = orch.stop_all().await;
        }
    }

    #[tokio::test]
    async fn test_handle_watch_logs_with_existing_match() {
        let orch = create_test_orchestrator();
        let server = DmnMcpServer::new_authenticated(orch.clone());

        {
            let orch_lock = orch.lock().await;
            let mut log_buffer = orch_lock.log_buffer.lock().await;
            log_buffer.append(
                "test_service",
                LogLine {
                    timestamp: SystemTime::now(),
                    content: "service booting".to_string(),
                    stream: crate::logs::LogStream::Stdout,
                },
            );
            log_buffer.append(
                "test_service",
                LogLine {
                    timestamp: SystemTime::now(),
                    content: "service ready".to_string(),
                    stream: crate::logs::LogStream::Stdout,
                },
            );
        }

        let result = server
            .handle_watch_logs(json!({
                "service": "test_service",
                "untilPattern": "ready",
                "includeExisting": true,
                "timeoutSeconds": 2
            }))
            .await;

        match result {
            McpToolResult::Success { content } => {
                assert_eq!(content.len(), 1);
                match &content[0] {
                    McpContent::Text { text } => {
                        assert!(text.contains("stop_reason: until_pattern_matched"));
                        assert!(text.contains("service ready"));
                    }
                }
            }
            _ => panic!("Expected success result"),
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
        assert_eq!(tools.len(), 7);
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
