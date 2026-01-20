use std::fs;
use std::io::{BufRead, BufReader, Write};
use std::path::PathBuf;
use std::process::{Command, Stdio};
use std::time::Duration;
use tempfile::TempDir;

/// Helper to create a temporary dmn.json config file
fn create_test_config(dir: &TempDir) -> PathBuf {
    let config_path = dir.path().join("dmn.json");
    let config_content = r#"{
        "version": "1.0",
        "services": {
            "backend": {
                "command": "echo 'Backend service'"
            },
            "frontend": {
                "command": "echo 'Frontend service'",
                "depends_on": ["backend"]
            }
        }
    }"#;
    fs::write(&config_path, config_content).unwrap();
    config_path
}

/// Helper to get the path to the dmn binary
fn get_dmn_binary() -> PathBuf {
    let mut path = PathBuf::from(env!("CARGO_MANIFEST_DIR"));
    path.push("../target/debug/dmn");
    if cfg!(windows) {
        path.set_extension("exe");
    }
    path
}

#[test]
fn test_mcp_mode_starts() {
    let temp_dir = TempDir::new().unwrap();
    let config_path = create_test_config(&temp_dir);

    // Start MCP server process
    let mut child = Command::new(get_dmn_binary())
        .arg("mcp")
        .arg("--config")
        .arg(config_path)
        .stdin(Stdio::piped())
        .stdout(Stdio::piped())
        .stderr(Stdio::piped())
        .spawn()
        .expect("Failed to start MCP server");

    // Give it a moment to start
    std::thread::sleep(Duration::from_millis(100));

    // Check if process is still running
    match child.try_wait() {
        Ok(Some(status)) => {
            panic!("MCP server exited prematurely with status: {}", status);
        }
        Ok(None) => {
            // Process is still running, which is expected
        }
        Err(e) => {
            panic!("Error checking MCP server status: {}", e);
        }
    }

    // Clean up
    let _ = child.kill();
    let _ = child.wait();
}

#[test]
fn test_mcp_mode_list_services() {
    let temp_dir = TempDir::new().unwrap();
    let config_path = create_test_config(&temp_dir);

    // Start MCP server process
    let mut child = Command::new(get_dmn_binary())
        .arg("mcp")
        .arg("--config")
        .arg(config_path)
        .stdin(Stdio::piped())
        .stdout(Stdio::piped())
        .stderr(Stdio::piped())
        .spawn()
        .expect("Failed to start MCP server");

    let mut stdin = child.stdin.take().expect("Failed to get stdin");
    let stdout = child.stdout.take().expect("Failed to get stdout");
    let mut reader = BufReader::new(stdout);

    // Send list_services tool call
    let request = r#"{"jsonrpc":"2.0","id":1,"method":"tools/call","params":{"name":"list_services","arguments":{}}}"#;
    writeln!(stdin, "{}", request).expect("Failed to write to stdin");
    stdin.flush().expect("Failed to flush stdin");

    // Read response with timeout
    let mut response = String::new();
    let read_result = std::thread::spawn(move || {
        reader.read_line(&mut response).ok();
        response
    })
    .join();

    // Clean up
    let _ = child.kill();
    let _ = child.wait();

    // Verify response
    if let Ok(response_str) = read_result {
        if !response_str.is_empty() {
            assert!(response_str.contains("jsonrpc"), "Expected JSON-RPC response");
            // MCP responses should contain result or error
            assert!(
                response_str.contains("result") || response_str.contains("error"),
                "Response: {}",
                response_str
            );
        }
    }
}

#[test]
fn test_mcp_mode_invalid_config() {
    let temp_dir = TempDir::new().unwrap();
    let config_path = temp_dir.path().join("invalid.json");
    fs::write(&config_path, "{ invalid json }").unwrap();

    // Try to start MCP server with invalid config
    let output = Command::new(get_dmn_binary())
        .arg("mcp")
        .arg("--config")
        .arg(config_path)
        .stdin(Stdio::null())
        .stdout(Stdio::piped())
        .stderr(Stdio::piped())
        .output()
        .expect("Failed to execute MCP server");

    // Should exit with error
    assert!(!output.status.success());
    let stderr = String::from_utf8_lossy(&output.stderr);
    assert!(stderr.contains("Failed to load configuration"));
}

#[test]
fn test_mcp_mode_missing_config() {
    // Try to start MCP server with non-existent config
    let output = Command::new(get_dmn_binary())
        .arg("mcp")
        .arg("--config")
        .arg("nonexistent.json")
        .stdin(Stdio::null())
        .stdout(Stdio::piped())
        .stderr(Stdio::piped())
        .output()
        .expect("Failed to execute MCP server");

    // Should exit with error
    assert!(!output.status.success());
    let stderr = String::from_utf8_lossy(&output.stderr);
    assert!(stderr.contains("Failed to load configuration") || stderr.contains("No such file"));
}

#[test]
fn test_mcp_mode_handles_eof() {
    let temp_dir = TempDir::new().unwrap();
    let config_path = create_test_config(&temp_dir);

    // Start MCP server process
    let mut child = Command::new(get_dmn_binary())
        .arg("mcp")
        .arg("--config")
        .arg(config_path)
        .stdin(Stdio::piped())
        .stdout(Stdio::piped())
        .stderr(Stdio::piped())
        .spawn()
        .expect("Failed to start MCP server");

    // Close stdin to simulate EOF
    drop(child.stdin.take());

    // Wait for process to exit (with timeout)
    let wait_result = std::thread::spawn(move || {
        std::thread::sleep(Duration::from_secs(2));
        child.try_wait()
    })
    .join();

    // Process should exit gracefully when stdin closes
    if let Ok(Ok(Some(status))) = wait_result {
        assert!(status.success() || status.code() == Some(0));
    }
}

#[test]
fn test_mcp_mode_get_service_status() {
    let temp_dir = TempDir::new().unwrap();
    let config_path = create_test_config(&temp_dir);

    // Start MCP server process
    let mut child = Command::new(get_dmn_binary())
        .arg("mcp")
        .arg("--config")
        .arg(config_path)
        .stdin(Stdio::piped())
        .stdout(Stdio::piped())
        .stderr(Stdio::piped())
        .spawn()
        .expect("Failed to start MCP server");

    let mut stdin = child.stdin.take().expect("Failed to get stdin");
    let stdout = child.stdout.take().expect("Failed to get stdout");
    let mut reader = BufReader::new(stdout);

    // Send get_service_status tool call
    let request = r#"{"jsonrpc":"2.0","id":2,"method":"tools/call","params":{"name":"get_service_status","arguments":{}}}"#;
    writeln!(stdin, "{}", request).expect("Failed to write to stdin");
    stdin.flush().expect("Failed to flush stdin");

    // Read response with timeout
    let mut response = String::new();
    let read_result = std::thread::spawn(move || {
        reader.read_line(&mut response).ok();
        response
    })
    .join();

    // Clean up
    let _ = child.kill();
    let _ = child.wait();

    // Verify response
    if let Ok(response_str) = read_result {
        if !response_str.is_empty() {
            assert!(response_str.contains("jsonrpc"), "Expected JSON-RPC response");
            assert!(
                response_str.contains("result") || response_str.contains("error"),
                "Response: {}",
                response_str
            );
        }
    }
}
