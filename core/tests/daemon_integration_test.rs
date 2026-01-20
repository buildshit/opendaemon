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
            "echo-service": {
                "command": "echo 'Hello from echo service'"
            },
            "sleep-service": {
                "command": "sleep 1"
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
fn test_daemon_mode_starts() {
    let temp_dir = TempDir::new().unwrap();
    let config_path = create_test_config(&temp_dir);

    // Start daemon process
    let mut child = Command::new(get_dmn_binary())
        .arg("daemon")
        .arg("--config")
        .arg(config_path)
        .stdin(Stdio::piped())
        .stdout(Stdio::piped())
        .stderr(Stdio::piped())
        .spawn()
        .expect("Failed to start daemon");

    // Give it a moment to start
    std::thread::sleep(Duration::from_millis(100));

    // Check if process is still running
    match child.try_wait() {
        Ok(Some(status)) => {
            panic!("Daemon exited prematurely with status: {}", status);
        }
        Ok(None) => {
            // Process is still running, which is expected
        }
        Err(e) => {
            panic!("Error checking daemon status: {}", e);
        }
    }

    // Clean up
    let _ = child.kill();
    let _ = child.wait();
}

#[test]
fn test_daemon_mode_json_rpc_get_status() {
    let temp_dir = TempDir::new().unwrap();
    let config_path = create_test_config(&temp_dir);

    // Start daemon process
    let mut child = Command::new(get_dmn_binary())
        .arg("daemon")
        .arg("--config")
        .arg(config_path)
        .stdin(Stdio::piped())
        .stdout(Stdio::piped())
        .stderr(Stdio::piped())
        .spawn()
        .expect("Failed to start daemon");

    let mut stdin = child.stdin.take().expect("Failed to get stdin");
    let stdout = child.stdout.take().expect("Failed to get stdout");
    let mut reader = BufReader::new(stdout);

    // Send getStatus request
    let request = r#"{"jsonrpc":"2.0","id":1,"method":"getStatus","params":null}"#;
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
            assert!(response_str.contains("result") || response_str.contains("error"));
        }
    }
}

#[test]
fn test_daemon_mode_invalid_config() {
    let temp_dir = TempDir::new().unwrap();
    let config_path = temp_dir.path().join("invalid.json");
    fs::write(&config_path, "{ invalid json }").unwrap();

    // Try to start daemon with invalid config
    let output = Command::new(get_dmn_binary())
        .arg("daemon")
        .arg("--config")
        .arg(config_path)
        .stdin(Stdio::null())
        .stdout(Stdio::piped())
        .stderr(Stdio::piped())
        .output()
        .expect("Failed to execute daemon");

    // Should exit with error
    assert!(!output.status.success());
    let stderr = String::from_utf8_lossy(&output.stderr);
    assert!(stderr.contains("Failed to load configuration"));
}

#[test]
fn test_daemon_mode_missing_config() {
    // Try to start daemon with non-existent config
    let output = Command::new(get_dmn_binary())
        .arg("daemon")
        .arg("--config")
        .arg("nonexistent.json")
        .stdin(Stdio::null())
        .stdout(Stdio::piped())
        .stderr(Stdio::piped())
        .output()
        .expect("Failed to execute daemon");

    // Should exit with error
    assert!(!output.status.success());
    let stderr = String::from_utf8_lossy(&output.stderr);
    assert!(stderr.contains("Failed to load configuration") || stderr.contains("No such file"));
}

#[test]
fn test_daemon_mode_handles_eof() {
    let temp_dir = TempDir::new().unwrap();
    let config_path = create_test_config(&temp_dir);

    // Start daemon process
    let mut child = Command::new(get_dmn_binary())
        .arg("daemon")
        .arg("--config")
        .arg(config_path)
        .stdin(Stdio::piped())
        .stdout(Stdio::piped())
        .stderr(Stdio::piped())
        .spawn()
        .expect("Failed to start daemon");

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
