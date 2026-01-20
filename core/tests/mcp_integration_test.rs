use dmn_core::{DmnConfig, DmnMcpServer, Orchestrator, ServiceConfig};
use serde_json::{json, Value};
use std::collections::HashMap;
use std::io::{BufRead, BufReader, Write};
use std::process::{Command, Stdio};
use std::sync::Arc;
use tokio::sync::Mutex;

/// Helper function to create a test configuration
fn create_test_config() -> DmnConfig {
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
    services.insert(
        "database".to_string(),
        ServiceConfig {
            command: "postgres".to_string(),
            depends_on: vec![],
            ready_when: None,
            env_file: None,
        },
    );

    DmnConfig {
        version: "1.0".to_string(),
        services,
    }
}

/// Helper function to send an MCP request and get response
fn send_mcp_request(
    stdin: &mut std::process::ChildStdin,
    stdout: &mut BufReader<std::process::ChildStdout>,
    request: Value,
) -> Result<Value, Box<dyn std::error::Error>> {
    // Send request
    let request_str = serde_json::to_string(&request)?;
    writeln!(stdin, "{}", request_str)?;
    stdin.flush()?;

    // Read response
    let mut response_line = String::new();
    stdout.read_line(&mut response_line)?;
    
    let response: Value = serde_json::from_str(&response_line)?;
    Ok(response)
}

#[test]
#[ignore] // This test requires the binary to be built
fn test_mcp_server_tools_list() {
    // This test spawns the actual dmn binary in MCP mode
    // and tests the tools/list endpoint
    
    // Create a temporary config file
    let temp_dir = std::env::temp_dir();
    let config_path = temp_dir.join("test_dmn.json");
    let config = create_test_config();
    std::fs::write(&config_path, serde_json::to_string_pretty(&config).unwrap()).unwrap();

    // Spawn the dmn binary in MCP mode
    let mut child = Command::new("cargo")
        .args(&["run", "--", "mcp", "-c", config_path.to_str().unwrap()])
        .stdin(Stdio::piped())
        .stdout(Stdio::piped())
        .stderr(Stdio::piped())
        .spawn()
        .expect("Failed to spawn dmn process");

    let mut stdin = child.stdin.take().expect("Failed to open stdin");
    let stdout = child.stdout.take().expect("Failed to open stdout");
    let mut stdout_reader = BufReader::new(stdout);

    // Send tools/list request
    let request = json!({
        "jsonrpc": "2.0",
        "id": 1,
        "method": "tools/list",
        "params": null
    });

    let response = send_mcp_request(&mut stdin, &mut stdout_reader, request)
        .expect("Failed to get response");

    // Verify response
    assert_eq!(response["jsonrpc"], "2.0");
    assert_eq!(response["id"], 1);
    assert!(response["result"].is_object());
    
    let tools = response["result"]["tools"].as_array().expect("tools should be an array");
    assert_eq!(tools.len(), 3);
    
    let tool_names: Vec<String> = tools
        .iter()
        .map(|t| t["name"].as_str().unwrap().to_string())
        .collect();
    
    assert!(tool_names.contains(&"read_logs".to_string()));
    assert!(tool_names.contains(&"get_service_status".to_string()));
    assert!(tool_names.contains(&"list_services".to_string()));

    // Clean up
    child.kill().expect("Failed to kill child process");
    std::fs::remove_file(config_path).ok();
}

#[test]
#[ignore] // This test requires the binary to be built
fn test_mcp_server_list_services() {
    // Create a temporary config file
    let temp_dir = std::env::temp_dir();
    let config_path = temp_dir.join("test_dmn_list.json");
    let config = create_test_config();
    std::fs::write(&config_path, serde_json::to_string_pretty(&config).unwrap()).unwrap();

    // Spawn the dmn binary in MCP mode
    let mut child = Command::new("cargo")
        .args(&["run", "--", "mcp", "-c", config_path.to_str().unwrap()])
        .stdin(Stdio::piped())
        .stdout(Stdio::piped())
        .stderr(Stdio::piped())
        .spawn()
        .expect("Failed to spawn dmn process");

    let mut stdin = child.stdin.take().expect("Failed to open stdin");
    let stdout = child.stdout.take().expect("Failed to open stdout");
    let mut stdout_reader = BufReader::new(stdout);

    // Send tools/call request for list_services
    let request = json!({
        "jsonrpc": "2.0",
        "id": 2,
        "method": "tools/call",
        "params": {
            "name": "list_services",
            "arguments": {}
        }
    });

    let response = send_mcp_request(&mut stdin, &mut stdout_reader, request)
        .expect("Failed to get response");

    // Verify response
    assert_eq!(response["jsonrpc"], "2.0");
    assert_eq!(response["id"], 2);
    assert!(response["result"].is_object());
    
    let result = &response["result"];
    assert_eq!(result["type"], "success");
    
    let content = result["content"].as_array().expect("content should be an array");
    assert_eq!(content.len(), 1);
    assert_eq!(content[0]["type"], "text");
    
    let text = content[0]["text"].as_str().unwrap();
    assert!(text.contains("test_service"));
    assert!(text.contains("database"));

    // Clean up
    child.kill().expect("Failed to kill child process");
    std::fs::remove_file(config_path).ok();
}

#[test]
#[ignore] // This test requires the binary to be built
fn test_mcp_server_get_service_status() {
    // Create a temporary config file
    let temp_dir = std::env::temp_dir();
    let config_path = temp_dir.join("test_dmn_status.json");
    let config = create_test_config();
    std::fs::write(&config_path, serde_json::to_string_pretty(&config).unwrap()).unwrap();

    // Spawn the dmn binary in MCP mode
    let mut child = Command::new("cargo")
        .args(&["run", "--", "mcp", "-c", config_path.to_str().unwrap()])
        .stdin(Stdio::piped())
        .stdout(Stdio::piped())
        .stderr(Stdio::piped())
        .spawn()
        .expect("Failed to spawn dmn process");

    let mut stdin = child.stdin.take().expect("Failed to open stdin");
    let stdout = child.stdout.take().expect("Failed to open stdout");
    let mut stdout_reader = BufReader::new(stdout);

    // Send tools/call request for get_service_status
    let request = json!({
        "jsonrpc": "2.0",
        "id": 3,
        "method": "tools/call",
        "params": {
            "name": "get_service_status",
            "arguments": {}
        }
    });

    let response = send_mcp_request(&mut stdin, &mut stdout_reader, request)
        .expect("Failed to get response");

    // Verify response
    assert_eq!(response["jsonrpc"], "2.0");
    assert_eq!(response["id"], 3);
    assert!(response["result"].is_object());
    
    let result = &response["result"];
    assert_eq!(result["type"], "success");
    
    let content = result["content"].as_array().expect("content should be an array");
    assert_eq!(content.len(), 1);
    assert_eq!(content[0]["type"], "text");
    
    let text = content[0]["text"].as_str().unwrap();
    assert!(text.contains("test_service"));
    assert!(text.contains("database"));
    assert!(text.contains("NotStarted"));

    // Clean up
    child.kill().expect("Failed to kill child process");
    std::fs::remove_file(config_path).ok();
}

#[test]
#[ignore] // This test requires the binary to be built
fn test_mcp_server_read_logs() {
    // Create a temporary config file
    let temp_dir = std::env::temp_dir();
    let config_path = temp_dir.join("test_dmn_logs.json");
    let config = create_test_config();
    std::fs::write(&config_path, serde_json::to_string_pretty(&config).unwrap()).unwrap();

    // Spawn the dmn binary in MCP mode
    let mut child = Command::new("cargo")
        .args(&["run", "--", "mcp", "-c", config_path.to_str().unwrap()])
        .stdin(Stdio::piped())
        .stdout(Stdio::piped())
        .stderr(Stdio::piped())
        .spawn()
        .expect("Failed to spawn dmn process");

    let mut stdin = child.stdin.take().expect("Failed to open stdin");
    let stdout = child.stdout.take().expect("Failed to open stdout");
    let mut stdout_reader = BufReader::new(stdout);

    // Send tools/call request for read_logs
    let request = json!({
        "jsonrpc": "2.0",
        "id": 4,
        "method": "tools/call",
        "params": {
            "name": "read_logs",
            "arguments": {
                "service": "test_service",
                "lines": 10
            }
        }
    });

    let response = send_mcp_request(&mut stdin, &mut stdout_reader, request)
        .expect("Failed to get response");

    // Verify response
    assert_eq!(response["jsonrpc"], "2.0");
    assert_eq!(response["id"], 4);
    assert!(response["result"].is_object());
    
    let result = &response["result"];
    assert_eq!(result["type"], "success");
    
    let content = result["content"].as_array().expect("content should be an array");
    assert_eq!(content.len(), 1);
    assert_eq!(content[0]["type"], "text");

    // Clean up
    child.kill().expect("Failed to kill child process");
    std::fs::remove_file(config_path).ok();
}

#[test]
#[ignore] // This test requires the binary to be built
fn test_mcp_server_invalid_service() {
    // Create a temporary config file
    let temp_dir = std::env::temp_dir();
    let config_path = temp_dir.join("test_dmn_invalid.json");
    let config = create_test_config();
    std::fs::write(&config_path, serde_json::to_string_pretty(&config).unwrap()).unwrap();

    // Spawn the dmn binary in MCP mode
    let mut child = Command::new("cargo")
        .args(&["run", "--", "mcp", "-c", config_path.to_str().unwrap()])
        .stdin(Stdio::piped())
        .stdout(Stdio::piped())
        .stderr(Stdio::piped())
        .spawn()
        .expect("Failed to spawn dmn process");

    let mut stdin = child.stdin.take().expect("Failed to open stdin");
    let stdout = child.stdout.take().expect("Failed to open stdout");
    let mut stdout_reader = BufReader::new(stdout);

    // Send tools/call request for read_logs with invalid service
    let request = json!({
        "jsonrpc": "2.0",
        "id": 5,
        "method": "tools/call",
        "params": {
            "name": "read_logs",
            "arguments": {
                "service": "nonexistent_service",
                "lines": 10
            }
        }
    });

    let response = send_mcp_request(&mut stdin, &mut stdout_reader, request)
        .expect("Failed to get response");

    // Verify response contains error
    assert_eq!(response["jsonrpc"], "2.0");
    assert_eq!(response["id"], 5);
    
    let result = &response["result"];
    assert_eq!(result["type"], "error");
    
    let error = result["error"].as_str().unwrap();
    assert!(error.contains("Service not found"));

    // Clean up
    child.kill().expect("Failed to kill child process");
    std::fs::remove_file(config_path).ok();
}

#[test]
#[ignore] // This test requires the binary to be built
fn test_mcp_server_unknown_method() {
    // Create a temporary config file
    let temp_dir = std::env::temp_dir();
    let config_path = temp_dir.join("test_dmn_unknown.json");
    let config = create_test_config();
    std::fs::write(&config_path, serde_json::to_string_pretty(&config).unwrap()).unwrap();

    // Spawn the dmn binary in MCP mode
    let mut child = Command::new("cargo")
        .args(&["run", "--", "mcp", "-c", config_path.to_str().unwrap()])
        .stdin(Stdio::piped())
        .stdout(Stdio::piped())
        .stderr(Stdio::piped())
        .spawn()
        .expect("Failed to spawn dmn process");

    let mut stdin = child.stdin.take().expect("Failed to open stdin");
    let stdout = child.stdout.take().expect("Failed to open stdout");
    let mut stdout_reader = BufReader::new(stdout);

    // Send request with unknown method
    let request = json!({
        "jsonrpc": "2.0",
        "id": 6,
        "method": "unknown/method",
        "params": null
    });

    let response = send_mcp_request(&mut stdin, &mut stdout_reader, request)
        .expect("Failed to get response");

    // Verify response contains error
    assert_eq!(response["jsonrpc"], "2.0");
    assert_eq!(response["id"], 6);
    assert!(response["error"].is_object());
    
    let error = &response["error"];
    assert_eq!(error["code"], -32601);
    assert!(error["message"].as_str().unwrap().contains("Method not found"));

    // Clean up
    child.kill().expect("Failed to kill child process");
    std::fs::remove_file(config_path).ok();
}

#[test]
#[ignore] // This test requires the binary to be built
fn test_mcp_server_malformed_json() {
    // Create a temporary config file
    let temp_dir = std::env::temp_dir();
    let config_path = temp_dir.join("test_dmn_malformed.json");
    let config = create_test_config();
    std::fs::write(&config_path, serde_json::to_string_pretty(&config).unwrap()).unwrap();

    // Spawn the dmn binary in MCP mode
    let mut child = Command::new("cargo")
        .args(&["run", "--", "mcp", "-c", config_path.to_str().unwrap()])
        .stdin(Stdio::piped())
        .stdout(Stdio::piped())
        .stderr(Stdio::piped())
        .spawn()
        .expect("Failed to spawn dmn process");

    let mut stdin = child.stdin.take().expect("Failed to open stdin");
    let stdout = child.stdout.take().expect("Failed to open stdout");
    let mut stdout_reader = BufReader::new(stdout);

    // Send malformed JSON
    writeln!(stdin, "{{invalid json}}").unwrap();
    stdin.flush().unwrap();

    // Read response
    let mut response_line = String::new();
    stdout_reader.read_line(&mut response_line).unwrap();
    
    let response: Value = serde_json::from_str(&response_line).unwrap();

    // Verify response contains parse error
    assert_eq!(response["jsonrpc"], "2.0");
    assert!(response["error"].is_object());
    
    let error = &response["error"];
    assert_eq!(error["code"], -32700);
    assert!(error["message"].as_str().unwrap().contains("Parse error"));

    // Clean up
    child.kill().expect("Failed to kill child process");
    std::fs::remove_file(config_path).ok();
}

// Unit tests that don't require spawning the binary

#[tokio::test]
async fn test_mcp_server_stdio_mock() {
    // This test uses the MCP server directly without spawning a process
    let config = create_test_config();
    let orchestrator = Orchestrator::new(config).unwrap();
    let orch = Arc::new(Mutex::new(orchestrator));
    let server = DmnMcpServer::new_authenticated(orch);

    // Test tools/list
    let request = dmn_core::mcp_server::McpRequest {
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
    let tools = result["tools"].as_array().unwrap();
    assert_eq!(tools.len(), 3);
}

#[tokio::test]
async fn test_mcp_server_multiple_requests() {
    // Test handling multiple sequential requests
    let config = create_test_config();
    let orchestrator = Orchestrator::new(config).unwrap();
    let orch = Arc::new(Mutex::new(orchestrator));
    let server = DmnMcpServer::new_authenticated(orch);

    // Request 1: tools/list
    let request1 = dmn_core::mcp_server::McpRequest {
        jsonrpc: "2.0".to_string(),
        id: Some(json!(1)),
        method: "tools/list".to_string(),
        params: None,
    };
    let response1 = server.handle_request(request1).await;
    assert_eq!(response1.id, Some(json!(1)));
    assert!(response1.result.is_some());

    // Request 2: list_services
    let request2 = dmn_core::mcp_server::McpRequest {
        jsonrpc: "2.0".to_string(),
        id: Some(json!(2)),
        method: "tools/call".to_string(),
        params: Some(json!({
            "name": "list_services",
            "arguments": {}
        })),
    };
    let response2 = server.handle_request(request2).await;
    assert_eq!(response2.id, Some(json!(2)));
    assert!(response2.result.is_some());

    // Request 3: get_service_status
    let request3 = dmn_core::mcp_server::McpRequest {
        jsonrpc: "2.0".to_string(),
        id: Some(json!(3)),
        method: "tools/call".to_string(),
        params: Some(json!({
            "name": "get_service_status",
            "arguments": {}
        })),
    };
    let response3 = server.handle_request(request3).await;
    assert_eq!(response3.id, Some(json!(3)));
    assert!(response3.result.is_some());
}
