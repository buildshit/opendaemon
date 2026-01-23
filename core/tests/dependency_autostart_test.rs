use dmn_core::config::{DmnConfig, ReadyCondition, ServiceConfig};
use dmn_core::orchestrator::Orchestrator;
use dmn_core::process::ServiceStatus;
use std::collections::HashMap;
use std::time::Duration;

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
async fn test_start_service_auto_starts_dependencies() {
    // Test that starting a service automatically starts its dependencies in order
    let mut services = HashMap::new();
    
    // Database - no dependencies
    services.insert(
        "database".to_string(),
        ServiceConfig {
            command: if cfg!(windows) {
                "cmd /c echo Database started && echo Database ready".to_string()
            } else {
                "echo 'Database started' && echo 'Database ready'".to_string()
            },
            depends_on: vec![],
            ready_when: Some(ReadyCondition::LogContains {
                pattern: "Database ready".to_string(),
                timeout_seconds: Some(10),
            }),
            env_file: None,
        },
    );
    
    // Backend - depends on database
    services.insert(
        "backend".to_string(),
        ServiceConfig {
            command: if cfg!(windows) {
                "cmd /c echo Backend started && echo Backend ready".to_string()
            } else {
                "echo 'Backend started' && echo 'Backend ready'".to_string()
            },
            depends_on: vec!["database".to_string()],
            ready_when: Some(ReadyCondition::LogContains {
                pattern: "Backend ready".to_string(),
                timeout_seconds: Some(10),
            }),
            env_file: None,
        },
    );
    
    // Frontend - depends on backend (which depends on database)
    services.insert(
        "frontend".to_string(),
        ServiceConfig {
            command: if cfg!(windows) {
                "cmd /c echo Frontend started && echo Frontend ready".to_string()
            } else {
                "echo 'Frontend started' && echo 'Frontend ready'".to_string()
            },
            depends_on: vec!["backend".to_string()],
            ready_when: Some(ReadyCondition::LogContains {
                pattern: "Frontend ready".to_string(),
                timeout_seconds: Some(10),
            }),
            env_file: None,
        },
    );
    
    let config = DmnConfig {
        version: "1.0".to_string(),
        services,
    };
    
    let mut orchestrator = Orchestrator::new(config).unwrap();
    
    // Start ONLY the frontend service
    // This should automatically start database and backend first
    let result = orchestrator.start_service_with_deps("frontend").await;
    assert!(result.is_ok(), "Failed to start frontend: {:?}", result.err());
    
    // Wait for all services to be ready
    tokio::time::sleep(Duration::from_secs(3)).await;
    
    // Verify ALL services are ready (including dependencies that were auto-started)
    assert!(wait_for_ready(&orchestrator, "database", 10).await, 
        "Database should be auto-started and ready");
    assert!(wait_for_ready(&orchestrator, "backend", 10).await, 
        "Backend should be auto-started and ready");
    assert!(wait_for_ready(&orchestrator, "frontend", 10).await, 
        "Frontend should be ready");
    
    // Verify logs show all services started
    let log_buffer = orchestrator.log_buffer.lock().await;
    
    let db_logs = log_buffer.get_all_lines("database");
    assert!(db_logs.iter().any(|l| l.content.contains("Database started")), 
        "Database should have been started");
    
    let backend_logs = log_buffer.get_all_lines("backend");
    assert!(backend_logs.iter().any(|l| l.content.contains("Backend started")), 
        "Backend should have been started");
    
    let frontend_logs = log_buffer.get_all_lines("frontend");
    assert!(frontend_logs.iter().any(|l| l.content.contains("Frontend started")), 
        "Frontend should have been started");
}

#[tokio::test]
async fn test_start_service_skips_already_running_dependencies() {
    // Test that already running dependencies are not restarted
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
    
    let config = DmnConfig {
        version: "1.0".to_string(),
        services,
    };
    
    let mut orchestrator = Orchestrator::new(config).unwrap();
    
    // First, start the database manually
    orchestrator.start_service_with_deps("database").await.unwrap();
    tokio::time::sleep(Duration::from_millis(500)).await;
    assert!(wait_for_ready(&orchestrator, "database", 5).await);
    
    // Now start backend - database should not be restarted
    let result = orchestrator.start_service_with_deps("backend").await;
    assert!(result.is_ok());
    
    // Both should be ready
    assert!(wait_for_ready(&orchestrator, "database", 5).await);
    assert!(wait_for_ready(&orchestrator, "backend", 5).await);
}

#[tokio::test]
async fn test_get_dependencies_returns_correct_list() {
    // Test that the graph returns correct dependency list
    let mut services = HashMap::new();
    
    services.insert(
        "database".to_string(),
        ServiceConfig {
            command: "echo db".to_string(),
            depends_on: vec![],
            ready_when: None,
            env_file: None,
        },
    );
    
    services.insert(
        "cache".to_string(),
        ServiceConfig {
            command: "echo cache".to_string(),
            depends_on: vec![],
            ready_when: None,
            env_file: None,
        },
    );
    
    services.insert(
        "backend".to_string(),
        ServiceConfig {
            command: "echo backend".to_string(),
            depends_on: vec!["database".to_string(), "cache".to_string()],
            ready_when: None,
            env_file: None,
        },
    );
    
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
    
    let orchestrator = Orchestrator::new(config).unwrap();
    
    // Test direct dependencies
    let backend_deps = orchestrator.graph().get_dependencies("backend").unwrap();
    assert_eq!(backend_deps.len(), 2);
    assert!(backend_deps.contains(&"database".to_string()));
    assert!(backend_deps.contains(&"cache".to_string()));
    
    // Test frontend dependencies (only direct dependency is backend)
    let frontend_deps = orchestrator.graph().get_dependencies("frontend").unwrap();
    assert_eq!(frontend_deps.len(), 1);
    assert!(frontend_deps.contains(&"backend".to_string()));
    
    // Test service with no dependencies
    let database_deps = orchestrator.graph().get_dependencies("database").unwrap();
    assert_eq!(database_deps.len(), 0);
}

#[tokio::test]
async fn test_stop_service_cascades_to_dependents() {
    // Test that stopping a service also stops all its dependents
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
    
    // Verify all are running
    assert!(wait_for_ready(&orchestrator, "database", 5).await);
    assert!(wait_for_ready(&orchestrator, "backend", 5).await);
    assert!(wait_for_ready(&orchestrator, "frontend", 5).await);
    
    // Stop the backend - should also stop frontend (which depends on backend)
    orchestrator.stop_service("backend").await.unwrap();
    tokio::time::sleep(Duration::from_secs(2)).await;
    
    // Database should still be running
    assert!(wait_for_ready(&orchestrator, "database", 1).await);
    
    // Backend and frontend should be stopped
    assert!(!orchestrator.is_service_ready("backend").await);
    assert!(!orchestrator.is_service_ready("frontend").await);
}

#[tokio::test]
async fn test_status_updates_on_lifecycle_events() {
    // Test that service status is correctly updated through lifecycle
    let mut services = HashMap::new();
    
    services.insert(
        "test_service".to_string(),
        ServiceConfig {
            command: if cfg!(windows) {
                "cmd /c echo Started && timeout /t 5".to_string()
            } else {
                "echo 'Started' && sleep 5".to_string()
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
    
    // Initially no status
    let initial_status = orchestrator.process_manager.get_status("test_service");
    assert!(initial_status.is_none());
    
    // Start service
    orchestrator.start_service_with_deps("test_service").await.unwrap();
    
    // After starting, should be Running
    tokio::time::sleep(Duration::from_millis(500)).await;
    let running_status = orchestrator.process_manager.get_status("test_service");
    assert!(matches!(running_status, Some(ServiceStatus::Running)));
    
    // Stop service
    orchestrator.stop_service("test_service").await.unwrap();
    
    // After stopping, should be Stopped
    let stopped_status = orchestrator.process_manager.get_status("test_service");
    assert!(matches!(stopped_status, Some(ServiceStatus::Stopped) | Some(ServiceStatus::Failed { .. })));
}
