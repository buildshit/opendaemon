use dmn_core::{DmnConfig, Orchestrator, ServiceConfig};
use std::collections::HashMap;

/// Helper to create a simple test config
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

    DmnConfig {
        version: "1.0".to_string(),
        services,
    }
}

#[tokio::test]
async fn test_error_on_missing_service() {
    let config = create_test_config();
    let mut orchestrator = Orchestrator::new(config).unwrap();

    // Try to start a non-existent service
    let result = orchestrator.start_service_with_deps("nonexistent").await;
    assert!(result.is_err());

    let err = result.unwrap_err();
    assert!(err.to_string().contains("nonexistent"));
}

#[tokio::test]
async fn test_error_on_stop_missing_service() {
    let config = create_test_config();
    let mut orchestrator = Orchestrator::new(config).unwrap();

    // Try to stop a non-existent service
    let result = orchestrator.stop_service("nonexistent").await;
    assert!(result.is_err());

    let err = result.unwrap_err();
    assert!(err.to_string().contains("nonexistent"));
}

#[tokio::test]
async fn test_cyclic_dependency_error() {
    let mut services = HashMap::new();
    services.insert(
        "service_a".to_string(),
        ServiceConfig {
            command: "echo a".to_string(),
            depends_on: vec!["service_b".to_string()],
            ready_when: None,
            env_file: None,
        },
    );
    services.insert(
        "service_b".to_string(),
        ServiceConfig {
            command: "echo b".to_string(),
            depends_on: vec!["service_a".to_string()],
            ready_when: None,
            env_file: None,
        },
    );

    let config = DmnConfig {
        version: "1.0".to_string(),
        services,
    };

    // Creating orchestrator should fail due to cyclic dependency
    let result = Orchestrator::new(config);
    assert!(result.is_err());

    let err = result.unwrap_err();
    let err_msg = err.to_string();
    assert!(err_msg.contains("Cyclic") || err_msg.contains("cycle") || err_msg.contains("cyclic"));
}

#[tokio::test]
async fn test_missing_dependency_error() {
    let mut services = HashMap::new();
    services.insert(
        "frontend".to_string(),
        ServiceConfig {
            command: "echo frontend".to_string(),
            depends_on: vec!["backend".to_string()],
            ready_when: None,
            env_file: None,
        },
    );

    let config = DmnConfig {
        version: "1.0".to_string(),
        services,
    };

    // Creating orchestrator should fail due to missing dependency
    let result = Orchestrator::new(config);
    assert!(result.is_err());

    let err = result.unwrap_err();
    let err_msg = err.to_string();
    assert!(err_msg.contains("backend") || err_msg.contains("not found"));
}

#[tokio::test]
async fn test_error_propagation_in_start_all() {
    let mut services = HashMap::new();
    services.insert(
        "service1".to_string(),
        ServiceConfig {
            command: "invalid_command_that_does_not_exist_12345".to_string(),
            depends_on: vec![],
            ready_when: None,
            env_file: None,
        },
    );

    let config = DmnConfig {
        version: "1.0".to_string(),
        services,
    };

    let mut orchestrator = Orchestrator::new(config).unwrap();

    // start_all should fail when a service fails to spawn
    let result = orchestrator.start_all().await;
    assert!(result.is_err());
}

#[tokio::test]
async fn test_error_message_format() {
    use dmn_core::OrchestratorError;

    let orch_err = OrchestratorError::ServiceNotFound {
        service: "test".to_string(),
    };

    let message = orch_err.to_string();
    assert!(message.contains("test"));
    assert!(message.contains("not found") || message.contains("Service"));
}

#[tokio::test]
async fn test_restart_nonexistent_service_error() {
    let config = create_test_config();
    let mut orchestrator = Orchestrator::new(config).unwrap();

    // Try to restart a non-existent service
    let result = orchestrator.restart_service("nonexistent").await;
    assert!(result.is_err());

    let err = result.unwrap_err();
    assert!(err.to_string().contains("nonexistent"));
}
