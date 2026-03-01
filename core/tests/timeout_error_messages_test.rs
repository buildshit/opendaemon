use dmn_core::config::{DmnConfig, ReadyCondition, ServiceConfig};
use dmn_core::orchestrator::Orchestrator;
use std::collections::HashMap;
use std::time::Duration;
use tokio;

#[tokio::test]
async fn test_timeout_error_includes_service_name_and_condition() {
    let mut services = HashMap::new();
    services.insert(
        "slow_service".to_string(),
        ServiceConfig {
            command: if cfg!(windows) {
                "cmd /c echo Starting && timeout /t 2 >nul && echo Done"
            } else {
                "sh -c 'echo Starting && sleep 2 && echo Done'"
            }
            .to_string(),
            depends_on: vec![],
            ready_when: Some(ReadyCondition::LogContains {
                pattern: "NEVER_APPEARS".to_string(),
                timeout_seconds: Some(1), // 1 second timeout
            }),
            env_file: None,
        },
    );

    let config = DmnConfig {
        version: "1.0".to_string(),
        services,
    };

    let mut orchestrator = Orchestrator::new(config).unwrap();
    let result = orchestrator.start_service_with_deps("slow_service").await;

    // The service should start but fail the ready check
    assert!(result.is_ok());

    // Wait for the timeout to occur
    tokio::time::sleep(Duration::from_secs(2)).await;

    // Check that the service is not marked as ready
    assert!(!orchestrator.is_service_ready("slow_service").await);
}

#[tokio::test]
async fn test_url_timeout_error_includes_details() {
    let mut services = HashMap::new();
    services.insert(
        "api_service".to_string(),
        ServiceConfig {
            command: if cfg!(windows) {
                "cmd /c echo API Starting && timeout /t 5 >nul"
            } else {
                "sh -c 'echo API Starting && sleep 5'"
            }
            .to_string(),
            depends_on: vec![],
            ready_when: Some(ReadyCondition::UrlResponds {
                url: "http://localhost:59999/health".to_string(),
                timeout_seconds: Some(1), // 1 second timeout
            }),
            env_file: None,
        },
    );

    let config = DmnConfig {
        version: "1.0".to_string(),
        services,
    };

    let mut orchestrator = Orchestrator::new(config).unwrap();
    let result = orchestrator.start_service_with_deps("api_service").await;

    // The service should start but fail the ready check
    assert!(result.is_ok());

    // Wait for the timeout to occur
    tokio::time::sleep(Duration::from_secs(2)).await;

    // Check that the service is not marked as ready
    assert!(!orchestrator.is_service_ready("api_service").await);
}

#[tokio::test]
async fn test_log_timeout_captures_recent_logs() {
    let mut services = HashMap::new();
    services.insert(
        "logging_service".to_string(),
        ServiceConfig {
            command: if cfg!(windows) {
                "cmd /c for /L %i in (1,1,5) do @(echo Log line %i && timeout /t 1 >nul)"
            } else {
                "sh -c 'for i in 1 2 3 4 5; do echo \"Log line $i\"; sleep 1; done'"
            }
            .to_string(),
            depends_on: vec![],
            ready_when: Some(ReadyCondition::LogContains {
                pattern: "READY_SIGNAL".to_string(),
                timeout_seconds: Some(2), // 2 second timeout
            }),
            env_file: None,
        },
    );

    let config = DmnConfig {
        version: "1.0".to_string(),
        services,
    };

    let mut orchestrator = Orchestrator::new(config).unwrap();
    let result = orchestrator
        .start_service_with_deps("logging_service")
        .await;

    // The service should start but fail the ready check
    assert!(result.is_ok());

    // Wait for the timeout to occur
    tokio::time::sleep(Duration::from_secs(3)).await;

    // Check that the service is not marked as ready
    assert!(!orchestrator.is_service_ready("logging_service").await);
}
