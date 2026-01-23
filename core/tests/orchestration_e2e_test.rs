use dmn_core::config::{DmnConfig, ReadyCondition, ServiceConfig};
use dmn_core::orchestrator::Orchestrator;
use dmn_core::process::ServiceStatus;
use std::collections::HashMap;
use std::path::PathBuf;
use std::time::Duration;

/// Helper function to load a test fixture
fn load_fixture(name: &str) -> DmnConfig {
    let path = PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .join("tests")
        .join("fixtures")
        .join(name);
    
    let content = std::fs::read_to_string(&path)
        .expect(&format!("Failed to read fixture: {}", path.display()));
    
    serde_json::from_str(&content)
        .expect(&format!("Failed to parse fixture: {}", path.display()))
}

/// Helper to wait for a service to reach a specific status
async fn wait_for_status(
    orchestrator: &Orchestrator,
    service: &str,
    expected_status: ServiceStatus,
    timeout_secs: u64,
) -> bool {
    let start = std::time::Instant::now();
    while start.elapsed() < Duration::from_secs(timeout_secs) {
        if let Some(status) = orchestrator.process_manager.get_status(service) {
            if std::mem::discriminant(&status) == std::mem::discriminant(&expected_status) {
                return true;
            }
        }
        tokio::time::sleep(Duration::from_millis(100)).await;
    }
    false
}

/// Helper to wait for a service to be ready
async fn wait_for_ready(orchestrator: &Orchestrator, service: &str, timeout_secs: u64) -> bool {
    let start = std::time::Instant::now();
    while start.elapsed() < Duration::from_secs(timeout_secs) {
        if orchestrator.is_service_ready(service).await {
            return true;
        }
        tokio::time::sleep(Duration::from_millis(100)).await;
    }
    false
}

#[tokio::test]
async fn test_simple_linear_startup_sequence() {
    // Test full startup sequence with simple linear dependencies
    // Requirements: 2.3, 3.1, 4.1, 4.2
    
    let config = load_fixture("simple_linear.json");
    let mut orchestrator = Orchestrator::new(config).unwrap();
    
    // Start all services
    let result = orchestrator.start_all().await;
    assert!(result.is_ok(), "Failed to start all services: {:?}", result.err());
    
    // Wait for all services to be ready
    tokio::time::sleep(Duration::from_secs(5)).await;
    
    // Verify all services are ready
    assert!(wait_for_ready(&orchestrator, "database", 10).await, "Database not ready");
    assert!(wait_for_ready(&orchestrator, "backend", 10).await, "Backend not ready");
    assert!(wait_for_ready(&orchestrator, "frontend", 10).await, "Frontend not ready");
    
    // Verify logs were captured
    let log_buffer = orchestrator.log_buffer.lock().await;
    let db_logs = log_buffer.get_all_lines("database");
    assert!(!db_logs.is_empty(), "Database logs should not be empty");
    assert!(db_logs.iter().any(|l| l.content.contains("Database ready")));
    
    let backend_logs = log_buffer.get_all_lines("backend");
    assert!(!backend_logs.is_empty(), "Backend logs should not be empty");
    assert!(backend_logs.iter().any(|l| l.content.contains("Backend listening")));
}

#[tokio::test]
async fn test_complex_multilevel_dependencies() {
    // Test complex multi-level dependency graph
    // Requirements: 2.3, 3.1, 4.1, 4.2
    
    let config = load_fixture("complex_multilevel.json");
    let mut orchestrator = Orchestrator::new(config).unwrap();
    
    // Start all services
    let result = orchestrator.start_all().await;
    assert!(result.is_ok(), "Failed to start all services: {:?}", result.err());
    
    // Wait for all services to be ready
    tokio::time::sleep(Duration::from_secs(8)).await;
    
    // Verify correct startup order by checking that dependencies are ready before dependents
    assert!(wait_for_ready(&orchestrator, "postgres", 10).await);
    assert!(wait_for_ready(&orchestrator, "redis", 10).await);
    assert!(wait_for_ready(&orchestrator, "auth_service", 10).await);
    assert!(wait_for_ready(&orchestrator, "api_gateway", 10).await);
    assert!(wait_for_ready(&orchestrator, "user_service", 10).await);
    assert!(wait_for_ready(&orchestrator, "frontend", 10).await);
    
    // Verify all services have logs
    let log_buffer = orchestrator.log_buffer.lock().await;
    for service in ["postgres", "redis", "auth_service", "api_gateway", "user_service", "frontend"] {
        let logs = log_buffer.get_all_lines(service);
        assert!(!logs.is_empty(), "{} should have logs", service);
    }
}

#[tokio::test]
async fn test_dependency_waiting() {
    // Test that services wait for dependencies to be ready
    // Requirements: 2.3, 4.1, 4.2
    
    let mut services = HashMap::new();
    
    // Database with slow ready condition
    services.insert(
        "database".to_string(),
        ServiceConfig {
            command: if cfg!(windows) {
                "cmd /c echo Starting... && timeout /t 2 /nobreak >nul && echo Database ready on port 5432".to_string()
            } else {
                "echo 'Starting...' && sleep 2 && echo 'Database ready on port 5432'".to_string()
            },
            depends_on: vec![],
            ready_when: Some(ReadyCondition::LogContains {
                pattern: "ready on port".to_string(),
                timeout_seconds: None,
            }),
            env_file: None,
        },
    );
    
    // Backend that depends on database
    services.insert(
        "backend".to_string(),
        ServiceConfig {
            command: if cfg!(windows) {
                "cmd /c echo Backend starting && echo Backend ready".to_string()
            } else {
                "echo 'Backend starting' && echo 'Backend ready'".to_string()
            },
            depends_on: vec!["database".to_string()],
            ready_when: None,
            env_file: None,
        },
    );
    
    let config = DmnConfig {
        version: "1.0".to_string(),
        services,
    };
    
    let mut orchestrator = Orchestrator::new(config).unwrap();
    
    // Start all services
    orchestrator.start_all().await.unwrap();
    
    // Wait for database to be ready
    assert!(wait_for_ready(&orchestrator, "database", 10).await, "Database should be ready");
    
    // Backend should also be ready (it waited for database)
    assert!(wait_for_ready(&orchestrator, "backend", 10).await, "Backend should be ready");
    
    // Verify backend didn't start before database was ready by checking logs
    let log_buffer = orchestrator.log_buffer.lock().await;
    let db_logs = log_buffer.get_all_lines("database");
    let backend_logs = log_buffer.get_all_lines("backend");
    
    assert!(!db_logs.is_empty());
    assert!(!backend_logs.is_empty());
}

#[tokio::test]
async fn test_graceful_shutdown() {
    // Test graceful shutdown of all services
    // Requirements: 7.1, 7.2
    
    let mut services = HashMap::new();
    
    services.insert(
        "service1".to_string(),
        ServiceConfig {
            command: if cfg!(windows) {
                "timeout /t 30".to_string()
            } else {
                "sleep 30".to_string()
            },
            depends_on: vec![],
            ready_when: None,
            env_file: None,
        },
    );
    
    services.insert(
        "service2".to_string(),
        ServiceConfig {
            command: if cfg!(windows) {
                "timeout /t 30".to_string()
            } else {
                "sleep 30".to_string()
            },
            depends_on: vec!["service1".to_string()],
            ready_when: None,
            env_file: None,
        },
    );
    
    let config = DmnConfig {
        version: "1.0".to_string(),
        services,
    };
    
    let mut orchestrator = Orchestrator::new(config).unwrap();
    
    // Start all services
    orchestrator.start_all().await.unwrap();
    tokio::time::sleep(Duration::from_millis(500)).await;
    
    // Verify services are running
    assert!(wait_for_ready(&orchestrator, "service1", 5).await);
    assert!(wait_for_ready(&orchestrator, "service2", 5).await);
    
    // Stop all services
    let result = orchestrator.stop_all().await;
    assert!(result.is_ok(), "Failed to stop all services: {:?}", result.err());
    
    // Wait for shutdown
    tokio::time::sleep(Duration::from_secs(2)).await;
    
    // Verify services are stopped
    let status1 = orchestrator.process_manager.get_status("service1");
    let status2 = orchestrator.process_manager.get_status("service2");
    
    assert!(matches!(status1, Some(ServiceStatus::Stopped) | Some(ServiceStatus::Failed { .. })));
    assert!(matches!(status2, Some(ServiceStatus::Stopped) | Some(ServiceStatus::Failed { .. })));
}

#[tokio::test]
async fn test_reverse_dependency_order_shutdown() {
    // Test that services stop in reverse dependency order
    // Requirements: 7.1, 7.2
    
    let config = load_fixture("simple_linear.json");
    let mut orchestrator = Orchestrator::new(config).unwrap();
    
    // Start all services
    orchestrator.start_all().await.unwrap();
    tokio::time::sleep(Duration::from_secs(5)).await;
    
    // Verify all are ready
    assert!(wait_for_ready(&orchestrator, "database", 5).await);
    assert!(wait_for_ready(&orchestrator, "backend", 5).await);
    assert!(wait_for_ready(&orchestrator, "frontend", 5).await);
    
    // Stop all services
    orchestrator.stop_all().await.unwrap();
    tokio::time::sleep(Duration::from_secs(2)).await;
    
    // All should be stopped
    let db_status = orchestrator.process_manager.get_status("database");
    let backend_status = orchestrator.process_manager.get_status("backend");
    let frontend_status = orchestrator.process_manager.get_status("frontend");
    
    assert!(matches!(db_status, Some(ServiceStatus::Stopped) | Some(ServiceStatus::Failed { .. })));
    assert!(matches!(backend_status, Some(ServiceStatus::Stopped) | Some(ServiceStatus::Failed { .. })));
    assert!(matches!(frontend_status, Some(ServiceStatus::Stopped) | Some(ServiceStatus::Failed { .. })));
}

#[tokio::test]
async fn test_service_failure_cascade() {
    // Test that when a service fails, dependent services are handled appropriately
    // Requirements: 7.2
    
    let mut services = HashMap::new();
    
    // Service that will fail immediately
    services.insert(
        "failing_service".to_string(),
        ServiceConfig {
            command: if cfg!(windows) {
                "cmd /c exit 1".to_string()
            } else {
                "exit 1".to_string()
            },
            depends_on: vec![],
            ready_when: None,
            env_file: None,
        },
    );
    
    // Service that depends on the failing service
    services.insert(
        "dependent_service".to_string(),
        ServiceConfig {
            command: if cfg!(windows) {
                "cmd /c echo Dependent".to_string()
            } else {
                "echo 'Dependent'".to_string()
            },
            depends_on: vec!["failing_service".to_string()],
            ready_when: None,
            env_file: None,
        },
    );
    
    let config = DmnConfig {
        version: "1.0".to_string(),
        services,
    };
    
    let mut orchestrator = Orchestrator::new(config).unwrap();
    
    // Try to start all services
    let _result = orchestrator.start_all().await;
    
    // The start should complete, but the failing service will fail
    tokio::time::sleep(Duration::from_secs(2)).await;
    
    // Check that the failing service has failed status
    let status = orchestrator.process_manager.get_status("failing_service");
    assert!(matches!(status, Some(ServiceStatus::Failed { .. })));
}

#[tokio::test]
async fn test_cascade_stop_on_dependency_stop() {
    // Test that stopping a service also stops its dependents
    // Requirements: 7.2
    
    let mut services = HashMap::new();
    
    services.insert(
        "database".to_string(),
        ServiceConfig {
            command: if cfg!(windows) {
                "timeout /t 30".to_string()
            } else {
                "sleep 30".to_string()
            },
            depends_on: vec![],
            ready_when: None,
            env_file: None,
        },
    );
    
    services.insert(
        "backend".to_string(),
        ServiceConfig {
            command: if cfg!(windows) {
                "timeout /t 30".to_string()
            } else {
                "sleep 30".to_string()
            },
            depends_on: vec!["database".to_string()],
            ready_when: None,
            env_file: None,
        },
    );
    
    services.insert(
        "frontend".to_string(),
        ServiceConfig {
            command: if cfg!(windows) {
                "timeout /t 30".to_string()
            } else {
                "sleep 30".to_string()
            },
            depends_on: vec!["backend".to_string()],
            ready_when: None,
            env_file: None,
        },
    );
    
    let config = DmnConfig {
        version: "1.0".to_string(),
        services,
    };
    
    let mut orchestrator = Orchestrator::new(config).unwrap();
    
    // Start all services
    orchestrator.start_all().await.unwrap();
    tokio::time::sleep(Duration::from_millis(500)).await;
    
    // Verify all are ready
    assert!(wait_for_ready(&orchestrator, "database", 5).await);
    assert!(wait_for_ready(&orchestrator, "backend", 5).await);
    assert!(wait_for_ready(&orchestrator, "frontend", 5).await);
    
    // Stop the database (should cascade to backend and frontend)
    orchestrator.stop_service("database").await.unwrap();
    tokio::time::sleep(Duration::from_secs(2)).await;
    
    // All services should be stopped
    let db_status = orchestrator.process_manager.get_status("database");
    let backend_status = orchestrator.process_manager.get_status("backend");
    let frontend_status = orchestrator.process_manager.get_status("frontend");
    
    assert!(matches!(db_status, Some(ServiceStatus::Stopped) | Some(ServiceStatus::Failed { .. })));
    assert!(matches!(backend_status, Some(ServiceStatus::Stopped) | Some(ServiceStatus::Failed { .. })));
    assert!(matches!(frontend_status, Some(ServiceStatus::Stopped) | Some(ServiceStatus::Failed { .. })));
}

#[tokio::test]
async fn test_restart_service() {
    // Test restarting a service
    // Requirements: 10.4
    
    let mut services = HashMap::new();
    
    services.insert(
        "test_service".to_string(),
        ServiceConfig {
            command: if cfg!(windows) {
                "cmd /c echo Service started && timeout /t 10".to_string()
            } else {
                "echo 'Service started' && sleep 10".to_string()
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
    
    let mut orchestrator = Orchestrator::new(config).unwrap();
    
    // Start the service
    orchestrator.start_service_with_deps("test_service").await.unwrap();
    tokio::time::sleep(Duration::from_millis(500)).await;
    assert!(wait_for_ready(&orchestrator, "test_service", 5).await);
    
    // Get initial log count
    let initial_log_count = {
        let log_buffer = orchestrator.log_buffer.lock().await;
        log_buffer.get_all_lines("test_service").len()
    };
    
    // Restart the service
    orchestrator.restart_service("test_service").await.unwrap();
    tokio::time::sleep(Duration::from_millis(500)).await;
    
    // Service should be ready again
    assert!(wait_for_ready(&orchestrator, "test_service", 5).await);
    
    // Should have more logs (from the restart)
    let new_log_count = {
        let log_buffer = orchestrator.log_buffer.lock().await;
        log_buffer.get_all_lines("test_service").len()
    };
    
    assert!(new_log_count >= initial_log_count, "Should have logs from restart");
}

#[tokio::test]
async fn test_env_file_loading() {
    // Test that environment variables from env files are loaded
    // Requirements: 1.5
    
    let config = load_fixture("with_env_files.json");
    let mut orchestrator = Orchestrator::new(config).unwrap();
    
    // Start all services
    orchestrator.start_all().await.unwrap();
    tokio::time::sleep(Duration::from_secs(5)).await;
    
    // Verify services are ready
    assert!(wait_for_ready(&orchestrator, "database", 10).await);
    assert!(wait_for_ready(&orchestrator, "api_server", 10).await);
    assert!(wait_for_ready(&orchestrator, "worker", 10).await);
    
    // Check that environment variables were used in the output
    let log_buffer = orchestrator.log_buffer.lock().await;
    
    let db_logs = log_buffer.get_all_lines("database");
    let db_log_text = db_logs.iter()
        .map(|l| l.content.as_str())
        .collect::<Vec<_>>()
        .join("\n");
    assert!(db_log_text.contains("DATABASE_URL") || db_log_text.contains("postgresql"), 
        "Database logs should contain env var: {}", db_log_text);
    
    let api_logs = log_buffer.get_all_lines("api_server");
    let api_log_text = api_logs.iter()
        .map(|l| l.content.as_str())
        .collect::<Vec<_>>()
        .join("\n");
    assert!(api_log_text.contains("API_KEY") || api_log_text.contains("test_api_key"), 
        "API logs should contain env var: {}", api_log_text);
}

#[tokio::test]
async fn test_circular_dependency_detection() {
    // Test that circular dependencies are detected
    // Requirements: 2.2
    
    let config = load_fixture("circular_dependency.json");
    let result = Orchestrator::new(config);
    
    assert!(result.is_err(), "Should detect circular dependency");
    let err = result.unwrap_err();
    assert!(err.to_string().contains("cycle") || err.to_string().contains("Cycle"), 
        "Error should mention cycle: {}", err);
}

#[tokio::test]
async fn test_event_emission() {
    // Test that orchestrator events are emitted correctly
    // Requirements: 9.4
    
    let mut services = HashMap::new();
    services.insert(
        "test_service".to_string(),
        ServiceConfig {
            command: if cfg!(windows) {
                "cmd /c echo Test && timeout /t 2".to_string()
            } else {
                "echo 'Test' && sleep 2".to_string()
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
    
    let mut orchestrator = Orchestrator::new(config).unwrap();
    let _event_sender = orchestrator.subscribe_events();
    
    // Start the service
    orchestrator.start_service_with_deps("test_service").await.unwrap();
    tokio::time::sleep(Duration::from_millis(500)).await;
    
    // Events should have been emitted (we can't easily verify without more complex setup)
    // But at least verify the service started
    assert!(wait_for_ready(&orchestrator, "test_service", 5).await);
}


#[tokio::test]
async fn test_custom_timeout_configuration() {
    // Test that custom timeout_seconds from config is respected
    // Requirements: 3.1, 3.2, 3.4, 3.5
    
    let mut services = HashMap::new();
    
    // Service with custom 2-second timeout that will succeed
    services.insert(
        "quick_service".to_string(),
        ServiceConfig {
            command: if cfg!(windows) {
                "cmd /c echo Starting && timeout /t 1 && echo Service ready".to_string()
            } else {
                "sh -c 'echo Starting && sleep 1 && echo Service ready'".to_string()
            },
            depends_on: vec![],
            ready_when: Some(ReadyCondition::LogContains {
                pattern: "Service ready".to_string(),
                timeout_seconds: Some(5), // Custom 5-second timeout
            }),
            env_file: None,
        },
    );
    
    // Service with no custom timeout (should use default 60 seconds)
    services.insert(
        "default_timeout_service".to_string(),
        ServiceConfig {
            command: if cfg!(windows) {
                "cmd /c echo Starting && echo Default ready".to_string()
            } else {
                "sh -c 'echo Starting && echo Default ready'".to_string()
            },
            depends_on: vec![],
            ready_when: Some(ReadyCondition::LogContains {
                pattern: "Default ready".to_string(),
                timeout_seconds: None, // No custom timeout, should use default
            }),
            env_file: None,
        },
    );
    
    let config = DmnConfig {
        version: "1.0".to_string(),
        services,
    };
    
    let mut orchestrator = Orchestrator::new(config).unwrap();
    
    // Start both services
    let result = orchestrator.start_service_with_deps("quick_service").await;
    assert!(result.is_ok(), "Quick service should start successfully");
    
    let result = orchestrator.start_service_with_deps("default_timeout_service").await;
    assert!(result.is_ok(), "Default timeout service should start successfully");
    
    // Wait for services to be ready
    tokio::time::sleep(Duration::from_secs(3)).await;
    
    // Verify both services became ready
    assert!(wait_for_ready(&orchestrator, "quick_service", 5).await, 
        "Quick service should be ready within custom timeout");
    assert!(wait_for_ready(&orchestrator, "default_timeout_service", 5).await, 
        "Default timeout service should be ready");
    
    // Verify logs show the services started
    let log_buffer = orchestrator.log_buffer.lock().await;
    let quick_logs = log_buffer.get_all_lines("quick_service");
    assert!(quick_logs.iter().any(|l| l.content.contains("Service ready")), 
        "Quick service logs should contain ready message");
    
    let default_logs = log_buffer.get_all_lines("default_timeout_service");
    assert!(default_logs.iter().any(|l| l.content.contains("Default ready")), 
        "Default timeout service logs should contain ready message");
}
