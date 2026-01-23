use dmn_core::config::{DmnConfig, ReadyCondition, ServiceConfig};
use dmn_core::mcp_server::{DmnMcpServer, McpToolCall};
use dmn_core::orchestrator::Orchestrator;
use serde_json::json;
use std::collections::HashMap;
use std::sync::Arc;
use std::time::Duration;
use tokio::sync::Mutex;

fn create_test_config() -> DmnConfig {
    let mut services = HashMap::new();
    
    services.insert(
        "database".to_string(),
        ServiceConfig {
            command: if cfg!(windows) {
                "cmd /c echo Database starting && timeout /t 2 /nobreak >nul && echo Database ready".to_string()
            } else {
                "echo 'Database starting' && sleep 2 && echo 'Database ready'".to_string()
            },
            depends_on: vec![],
            ready_when: Some(ReadyCondition::LogContains {
                pattern: "ready".to_string(),
                timeout_seconds: None,
            }),
            env_file: None,
        },
    );
    
    services.insert(
        "backend".to_string(),
        ServiceConfig {
            command: if cfg!(windows) {
                "cmd /c echo Backend starting && timeout /t 1 /nobreak >nul && echo Backend listening".to_string()
            } else {
                "echo 'Backend starting' && sleep 1 && echo 'Backend listening'".to_string()
            },
            depends_on: vec!["database".to_string()],
            ready_when: Some(ReadyCondition::LogContains {
                pattern: "listening".to_string(),
                timeout_seconds: None,
            }),
            env_file: None,
        },
    );

    DmnConfig {
        version: "1.0".to_string(),
        services,
    }
}

#[tokio::test]
async fn test_mcp_tool_list_services() {
    let config = create_test_config();
    let orchestrator = Orchestrator::new(config).unwrap();
    let orchestrator = Arc::new(Mutex::new(orchestrator));
    
    let mcp_server = DmnMcpServer::new_authenticated(Arc::clone(&orchestrator));
    
    let call = McpToolCall {
        name: "list_services".to_string(),
        arguments: json!({}),
    };
    
    let result = mcp_server.handle_tool_call(call).await;
    
    match result {
        dmn_core::mcp_server::McpToolResult::Success { content } => {
            assert!(!content.is_empty(), "Should have content");
        }
        dmn_core::mcp_server::McpToolResult::Error { error } => {
            panic!("list_services failed: {}", error);
        }
    }
}

#[tokio::test]
async fn test_mcp_tool_read_logs() {
    let config = create_test_config();
    let mut orchestrator = Orchestrator::new(config).unwrap();
    
    orchestrator.start_service_with_deps("database").await.unwrap();
    tokio::time::sleep(Duration::from_secs(3)).await;
    
    let orchestrator = Arc::new(Mutex::new(orchestrator));
    let mcp_server = DmnMcpServer::new_authenticated(Arc::clone(&orchestrator));
    
    let call = McpToolCall {
        name: "read_logs".to_string(),
        arguments: json!({
            "service": "database",
            "lines": 10
        }),
    };
    
    let result = mcp_server.handle_tool_call(call).await;
    
    match result {
        dmn_core::mcp_server::McpToolResult::Success { content } => {
            assert!(!content.is_empty(), "Should have content");
        }
        dmn_core::mcp_server::McpToolResult::Error { error } => {
            panic!("read_logs failed: {}", error);
        }
    }
    
    let mut orch = orchestrator.lock().await;
    let _ = orch.stop_all().await;
}

#[tokio::test]
async fn test_mcp_tool_registration() {
    let config = create_test_config();
    let orchestrator = Orchestrator::new(config).unwrap();
    let orchestrator = Arc::new(Mutex::new(orchestrator));
    
    let mcp_server = DmnMcpServer::new_authenticated(Arc::clone(&orchestrator));
    
    let tools = mcp_server.list_tools();
    
    assert_eq!(tools.len(), 3, "Should have 3 tools");
    
    let tool_names: Vec<String> = tools.iter().map(|t| t.name.clone()).collect();
    assert!(tool_names.contains(&"read_logs".to_string()));
    assert!(tool_names.contains(&"get_service_status".to_string()));
    assert!(tool_names.contains(&"list_services".to_string()));
}

#[tokio::test]
async fn test_mcp_authentication_required() {
    let config = create_test_config();
    let orchestrator = Orchestrator::new(config).unwrap();
    let orchestrator = Arc::new(Mutex::new(orchestrator));
    
    let mcp_server = DmnMcpServer::new(Arc::clone(&orchestrator));
    
    let call = McpToolCall {
        name: "list_services".to_string(),
        arguments: json!({}),
    };
    
    let result = mcp_server.handle_tool_call(call).await;
    
    match result {
        dmn_core::mcp_server::McpToolResult::Error { error } => {
            assert!(error.contains("Authentication") || error.contains("authentication"),
                "Error should mention authentication: {}", error);
        }
        dmn_core::mcp_server::McpToolResult::Success { .. } => {
            panic!("Should have returned authentication error");
        }
    }
}
