use std::fs;
use std::path::PathBuf;
use std::process::Command;
use tempfile::TempDir;

/// Helper to create a temporary dmn.json config file
fn create_test_config(dir: &TempDir) -> PathBuf {
    let config_path = dir.path().join("dmn.json");
    let config_content = r#"{
        "version": "1.0",
        "services": {
            "test-service": {
                "command": "echo 'Hello from test service'"
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
fn test_cli_help() {
    let output = Command::new(get_dmn_binary())
        .arg("--help")
        .output()
        .expect("Failed to execute dmn --help");

    assert!(output.status.success());
    let stdout = String::from_utf8_lossy(&output.stdout);
    assert!(stdout.contains("OpenDaemon"));
    assert!(stdout.contains("daemon"));
    assert!(stdout.contains("mcp"));
    assert!(stdout.contains("start"));
    assert!(stdout.contains("stop"));
    assert!(stdout.contains("status"));
}

#[test]
fn test_cli_version() {
    let output = Command::new(get_dmn_binary())
        .arg("--version")
        .output()
        .expect("Failed to execute dmn --version");

    assert!(output.status.success());
    let stdout = String::from_utf8_lossy(&output.stdout);
    assert!(stdout.contains("dmn"));
}

#[test]
fn test_daemon_command_help() {
    let output = Command::new(get_dmn_binary())
        .arg("daemon")
        .arg("--help")
        .output()
        .expect("Failed to execute dmn daemon --help");

    assert!(output.status.success());
    let stdout = String::from_utf8_lossy(&output.stdout);
    assert!(stdout.contains("daemon"));
    assert!(stdout.contains("config"));
}

#[test]
fn test_mcp_command_help() {
    let output = Command::new(get_dmn_binary())
        .arg("mcp")
        .arg("--help")
        .output()
        .expect("Failed to execute dmn mcp --help");

    assert!(output.status.success());
    let stdout = String::from_utf8_lossy(&output.stdout);
    assert!(stdout.contains("mcp"));
    assert!(stdout.contains("config"));
}

#[test]
fn test_start_command_help() {
    let output = Command::new(get_dmn_binary())
        .arg("start")
        .arg("--help")
        .output()
        .expect("Failed to execute dmn start --help");

    assert!(output.status.success());
    let stdout = String::from_utf8_lossy(&output.stdout);
    assert!(stdout.contains("start"));
    assert!(stdout.contains("config"));
}

#[test]
fn test_stop_command_help() {
    let output = Command::new(get_dmn_binary())
        .arg("stop")
        .arg("--help")
        .output()
        .expect("Failed to execute dmn stop --help");

    assert!(output.status.success());
    let stdout = String::from_utf8_lossy(&output.stdout);
    assert!(stdout.contains("stop"));
    assert!(stdout.contains("config"));
}

#[test]
fn test_status_command_help() {
    let output = Command::new(get_dmn_binary())
        .arg("status")
        .arg("--help")
        .output()
        .expect("Failed to execute dmn status --help");

    assert!(output.status.success());
    let stdout = String::from_utf8_lossy(&output.stdout);
    assert!(stdout.contains("status"));
    assert!(stdout.contains("config"));
}

#[test]
fn test_status_command_with_config() {
    let temp_dir = TempDir::new().unwrap();
    let config_path = create_test_config(&temp_dir);

    let output = Command::new(get_dmn_binary())
        .arg("status")
        .arg("--config")
        .arg(config_path)
        .output()
        .expect("Failed to execute dmn status");

    // Status command should succeed even with no running services
    assert!(output.status.success());
    let stderr = String::from_utf8_lossy(&output.stderr);
    // The status command outputs to stderr
    // It should either show the service status or indicate no services are running
    assert!(
        stderr.contains("test-service") || stderr.contains("No services"),
        "Expected status output, got: {}",
        stderr
    );
}

#[test]
fn test_invalid_config_path() {
    let output = Command::new(get_dmn_binary())
        .arg("status")
        .arg("--config")
        .arg("nonexistent.json")
        .output()
        .expect("Failed to execute dmn status");

    // Should fail with non-existent config
    assert!(!output.status.success());
    let stderr = String::from_utf8_lossy(&output.stderr);
    assert!(stderr.contains("Failed to load configuration") || stderr.contains("No such file"));
}

#[test]
fn test_malformed_config() {
    let temp_dir = TempDir::new().unwrap();
    let config_path = temp_dir.path().join("bad.json");
    fs::write(&config_path, "{ invalid json }").unwrap();

    let output = Command::new(get_dmn_binary())
        .arg("status")
        .arg("--config")
        .arg(config_path)
        .output()
        .expect("Failed to execute dmn status");

    // Should fail with malformed config
    assert!(!output.status.success());
    let stderr = String::from_utf8_lossy(&output.stderr);
    assert!(stderr.contains("Failed to load configuration"));
}
