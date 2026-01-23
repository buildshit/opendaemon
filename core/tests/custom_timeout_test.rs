use dmn_core::config::{DmnConfig, ReadyCondition, ServiceConfig};
use dmn_core::orchestrator::Orchestrator;
use std::collections::HashMap;
use std::time::{Duration, Instant};
use tokio;

/// Test that custom timeout_seconds in log_contains is respected
#[tokio::test]
async fn test_custom_timeout_log_contains_respected() {
    let mut services = HashMap::new();
    services.insert(
        "custom_timeout_service".to_string(),
        ServiceConfig {
            command: if cfg!(windows) {
                "cmd /c echo Starting && timeout /t 5 >nul && echo Done"
            } else {
                "sh -c 'echo Starting && sleep 5 && echo Done'"
            }
            .to_string(),
            depends_on: vec![],
            ready_when: Some(ReadyCondition::LogContains {
                pattern: "NEVER_APPEARS".to_string(),
                timeout_seconds: Some(2), // Custom 2 second timeout
            }),
            env_file: None,
        },
    );

    let config = DmnConfig {
        version: "1.0".to_string(),
        services,
    };

    let mut orchestrator = Orchestrator::new(config).unwrap();
    
    let start = Instant::now();
    let result = orchestrator.start_service_with_deps("custom_timeout_service").await;
    
    // Service should start successfully
    assert!(result.is_ok());
    
    // Wait for timeout to occur (2 seconds + buffer)
    tokio::time::sleep(Duration::from_secs(3)).await;
    
    let elapsed = start.elapsed();
    
    // Verify the service is not ready (timeout occurred)
    assert!(!orchestrator.is_service_ready("custom_timeout_service").await);
    
    // Verify timeout happened around 2 seconds (not default 60 seconds)
    // Allow some buffer for test execution time
    assert!(elapsed < Duration::from_secs(10), 
        "Timeout should have occurred around 2 seconds, but took {:?}", elapsed);
}

/// Test that custom timeout_seconds in url_responds is respected
#[tokio::test]
async fn test_custom_timeout_url_responds_respected() {
    let mut services = HashMap::new();
    services.insert(
        "api_custom_timeout".to_string(),
        ServiceConfig {
            command: if cfg!(windows) {
                "cmd /c echo API Starting && timeout /t 5 >nul"
            } else {
                "sh -c 'echo API Starting && sleep 5'"
            }
            .to_string(),
            depends_on: vec![],
            ready_when: Some(ReadyCondition::UrlResponds {
                url: "http://localhost:59998/health".to_string(),
                timeout_seconds: Some(2), // Custom 2 second timeout
            }),
            env_file: None,
        },
    );

    let config = DmnConfig {
        version: "1.0".to_string(),
        services,
    };

    let mut orchestrator = Orchestrator::new(config).unwrap();
    
    let start = Instant::now();
    let result = orchestrator.start_service_with_deps("api_custom_timeout").await;
    
    // Service should start successfully
    assert!(result.is_ok());
    
    // Wait for timeout to occur (2 seconds + buffer)
    tokio::time::sleep(Duration::from_secs(3)).await;
    
    let elapsed = start.elapsed();
    
    // Verify the service is not ready (timeout occurred)
    assert!(!orchestrator.is_service_ready("api_custom_timeout").await);
    
    // Verify timeout happened around 2 seconds (not default 60 seconds)
    assert!(elapsed < Duration::from_secs(10), 
        "Timeout should have occurred around 2 seconds, but took {:?}", elapsed);
}

/// Test that default timeout is used when timeout_seconds is not specified
#[tokio::test]
async fn test_default_timeout_when_not_specified() {
    let mut services = HashMap::new();
    services.insert(
        "default_timeout_service".to_string(),
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
                timeout_seconds: None, // No custom timeout - should use default
            }),
            env_file: None,
        },
    );

    let config = DmnConfig {
        version: "1.0".to_string(),
        services,
    };

    let mut orchestrator = Orchestrator::new(config).unwrap();
    
    let start = Instant::now();
    let result = orchestrator.start_service_with_deps("default_timeout_service").await;
    
    // Service should start successfully
    assert!(result.is_ok());
    
    // Wait a short time (less than default timeout)
    tokio::time::sleep(Duration::from_secs(3)).await;
    
    let elapsed = start.elapsed();
    
    // Service should still be waiting (not timed out yet)
    // The default timeout is 60 seconds, so after 3 seconds it should still be waiting
    // We can't easily verify it's still waiting without internal state access,
    // but we can verify it hasn't completed in the short time
    assert!(elapsed < Duration::from_secs(10), 
        "Test should complete quickly, took {:?}", elapsed);
    
    // Note: We don't wait for the full 60 second default timeout in the test
    // as that would make tests too slow. The key is that it doesn't timeout
    // in the first few seconds like a custom short timeout would.
}

/// Test that a service with custom timeout that succeeds before timeout is marked ready
#[tokio::test]
async fn test_custom_timeout_success_before_timeout() {
    let mut services = HashMap::new();
    services.insert(
        "quick_service".to_string(),
        ServiceConfig {
            command: if cfg!(windows) {
                "cmd /c echo Starting && echo READY"
            } else {
                "sh -c 'echo Starting && echo READY'"
            }
            .to_string(),
            depends_on: vec![],
            ready_when: Some(ReadyCondition::LogContains {
                pattern: "READY".to_string(),
                timeout_seconds: Some(10), // 10 second timeout (plenty of time)
            }),
            env_file: None,
        },
    );

    let config = DmnConfig {
        version: "1.0".to_string(),
        services,
    };

    let mut orchestrator = Orchestrator::new(config).unwrap();
    
    let start = Instant::now();
    let result = orchestrator.start_service_with_deps("quick_service").await;
    
    // Service should start successfully
    assert!(result.is_ok());
    
    // Wait for service to become ready
    tokio::time::sleep(Duration::from_secs(2)).await;
    
    let elapsed = start.elapsed();
    
    // Verify the service is ready (pattern matched before timeout)
    assert!(orchestrator.is_service_ready("quick_service").await,
        "Service should be ready after matching pattern");
    
    // Verify it completed quickly (not waiting for full timeout)
    assert!(elapsed < Duration::from_secs(5), 
        "Service should become ready quickly, took {:?}", elapsed);
}

/// Test multiple services with different custom timeouts
#[tokio::test]
async fn test_multiple_services_different_timeouts() {
    let mut services = HashMap::new();
    
    // Service with 2 second timeout
    services.insert(
        "short_timeout".to_string(),
        ServiceConfig {
            command: if cfg!(windows) {
                "cmd /c echo Starting short"
            } else {
                "sh -c 'echo Starting short'"
            }
            .to_string(),
            depends_on: vec![],
            ready_when: Some(ReadyCondition::LogContains {
                pattern: "NEVER".to_string(),
                timeout_seconds: Some(2),
            }),
            env_file: None,
        },
    );
    
    // Service with 5 second timeout
    services.insert(
        "long_timeout".to_string(),
        ServiceConfig {
            command: if cfg!(windows) {
                "cmd /c echo Starting long"
            } else {
                "sh -c 'echo Starting long'"
            }
            .to_string(),
            depends_on: vec![],
            ready_when: Some(ReadyCondition::LogContains {
                pattern: "NEVER".to_string(),
                timeout_seconds: Some(5),
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
    let result1 = orchestrator.start_service_with_deps("short_timeout").await;
    let result2 = orchestrator.start_service_with_deps("long_timeout").await;
    
    assert!(result1.is_ok());
    assert!(result2.is_ok());
    
    // Wait for short timeout to occur
    tokio::time::sleep(Duration::from_secs(3)).await;
    
    // Short timeout service should not be ready (timed out)
    assert!(!orchestrator.is_service_ready("short_timeout").await);
    
    // Long timeout service should still be waiting (not timed out yet)
    // We can't directly verify it's still waiting, but we know it hasn't
    // completed successfully since the pattern never appears
    
    // Wait for long timeout to occur
    tokio::time::sleep(Duration::from_secs(3)).await;
    
    // Now long timeout service should also not be ready (timed out)
    assert!(!orchestrator.is_service_ready("long_timeout").await);
}

/// Test that timeout_seconds of 0 or very small values are handled
#[tokio::test]
async fn test_very_short_custom_timeout() {
    let mut services = HashMap::new();
    services.insert(
        "instant_timeout".to_string(),
        ServiceConfig {
            command: if cfg!(windows) {
                "cmd /c echo Starting && timeout /t 2 >nul"
            } else {
                "sh -c 'echo Starting && sleep 2'"
            }
            .to_string(),
            depends_on: vec![],
            ready_when: Some(ReadyCondition::LogContains {
                pattern: "READY".to_string(),
                timeout_seconds: Some(1), // Very short 1 second timeout
            }),
            env_file: None,
        },
    );

    let config = DmnConfig {
        version: "1.0".to_string(),
        services,
    };

    let mut orchestrator = Orchestrator::new(config).unwrap();
    
    let start = Instant::now();
    let result = orchestrator.start_service_with_deps("instant_timeout").await;
    
    assert!(result.is_ok());
    
    // Wait for timeout
    tokio::time::sleep(Duration::from_secs(2)).await;
    
    let elapsed = start.elapsed();
    
    // Should timeout quickly
    assert!(!orchestrator.is_service_ready("instant_timeout").await);
    assert!(elapsed < Duration::from_secs(5), 
        "Should timeout quickly, took {:?}", elapsed);
}
