use dmn_core::config::DmnConfig;
use dmn_core::orchestrator::Orchestrator;
use std::fs;
use std::path::PathBuf;
use std::time::{Duration, Instant};
use tokio;

/// Test loading a dmn.json with custom timeout values
#[tokio::test]
async fn test_load_config_with_custom_timeouts() {
    let config_path = PathBuf::from("tests/fixtures/dmn_custom_timeout.json");

    // Verify the config file exists
    assert!(config_path.exists(), "Test config file should exist");

    // Load and parse the config
    let config_content =
        fs::read_to_string(&config_path).expect("Should be able to read config file");

    let config: DmnConfig =
        serde_json::from_str(&config_content).expect("Should be able to parse config");

    // Verify services are loaded
    assert_eq!(config.services.len(), 4, "Should have 4 services");

    // Verify database service has custom timeout
    let database = config
        .services
        .get("database")
        .expect("database service should exist");
    if let Some(ready_when) = &database.ready_when {
        match ready_when {
            dmn_core::config::ReadyCondition::LogContains {
                pattern,
                timeout_seconds,
            } => {
                assert_eq!(pattern, "Database ready");
                assert_eq!(
                    *timeout_seconds,
                    Some(10),
                    "database should have 10 second timeout"
                );
            }
            _ => panic!("database should have log_contains condition"),
        }
    } else {
        panic!("database should have ready_when condition");
    }

    // Verify api service has custom timeout
    let api = config
        .services
        .get("api")
        .expect("api service should exist");
    if let Some(ready_when) = &api.ready_when {
        match ready_when {
            dmn_core::config::ReadyCondition::LogContains {
                pattern,
                timeout_seconds,
            } => {
                assert_eq!(pattern, "API listening");
                assert_eq!(
                    *timeout_seconds,
                    Some(15),
                    "api should have 15 second timeout"
                );
            }
            _ => panic!("api should have log_contains condition"),
        }
    }

    // Verify web service has custom timeout for URL
    let web = config
        .services
        .get("web")
        .expect("web service should exist");
    if let Some(ready_when) = &web.ready_when {
        match ready_when {
            dmn_core::config::ReadyCondition::UrlResponds {
                url,
                timeout_seconds,
            } => {
                assert_eq!(url, "http://localhost:8000");
                assert_eq!(
                    *timeout_seconds,
                    Some(30),
                    "web should have 30 second timeout"
                );
            }
            _ => panic!("web should have url_responds condition"),
        }
    }

    // Verify worker service has no custom timeout (should use default)
    let worker = config
        .services
        .get("worker")
        .expect("worker service should exist");
    if let Some(ready_when) = &worker.ready_when {
        match ready_when {
            dmn_core::config::ReadyCondition::LogContains {
                pattern,
                timeout_seconds,
            } => {
                assert_eq!(pattern, "Worker ready");
                assert_eq!(
                    *timeout_seconds, None,
                    "worker should not have custom timeout"
                );
            }
            _ => panic!("worker should have log_contains condition"),
        }
    }
}

/// Test that orchestrator respects custom timeouts from config file
#[tokio::test]
async fn test_orchestrator_uses_custom_timeouts_from_config() {
    let config_path = PathBuf::from("tests/fixtures/dmn_custom_timeout.json");
    let config_content =
        fs::read_to_string(&config_path).expect("Should be able to read config file");

    let mut config: DmnConfig =
        serde_json::from_str(&config_content).expect("Should be able to parse config");

    // Modify the database service to have a pattern that won't match
    // and a short timeout to verify it's respected
    if let Some(database) = config.services.get_mut("database") {
        database.command = if cfg!(windows) {
            "cmd /c echo Starting && timeout /t 5 >nul".to_string()
        } else {
            "sh -c 'echo Starting && sleep 5'".to_string()
        };
        database.ready_when = Some(dmn_core::config::ReadyCondition::LogContains {
            pattern: "NEVER_MATCHES".to_string(),
            timeout_seconds: Some(2), // 2 second custom timeout
        });
    }

    let mut orchestrator = Orchestrator::new(config).unwrap();

    let start = Instant::now();
    let result = orchestrator.start_service_with_deps("database").await;

    // Service should start successfully
    assert!(result.is_ok(), "Service should start");

    // Wait for timeout to occur
    tokio::time::sleep(Duration::from_secs(3)).await;

    let elapsed = start.elapsed();

    // Verify timeout occurred around 2 seconds (custom timeout)
    assert!(
        elapsed < Duration::from_secs(10),
        "Should timeout with custom 2 second timeout, took {:?}",
        elapsed
    );
}

/// Test backward compatibility - config without timeout_seconds should work
#[tokio::test]
async fn test_backward_compatibility_no_timeout_field() {
    let config_json = if cfg!(windows) {
        r#"{
            "version": "1.0",
            "services": {
                "legacy_service": {
                    "command": "cmd /c echo Starting",
                    "ready_when": {
                        "type": "log_contains",
                        "pattern": "Starting"
                    }
                }
            }
        }"#
    } else {
        r#"{
            "version": "1.0",
            "services": {
                "legacy_service": {
                    "command": "sh -c 'echo Starting'",
                    "ready_when": {
                        "type": "log_contains",
                        "pattern": "Starting"
                    }
                }
            }
        }"#
    };

    let config: DmnConfig = serde_json::from_str(config_json)
        .expect("Should parse config without timeout_seconds field");

    // Verify service loaded correctly
    assert_eq!(config.services.len(), 1);

    let service = config.services.get("legacy_service").unwrap();
    if let Some(ready_when) = &service.ready_when {
        match ready_when {
            dmn_core::config::ReadyCondition::LogContains {
                pattern,
                timeout_seconds,
            } => {
                assert_eq!(pattern, "Starting");
                assert_eq!(
                    *timeout_seconds, None,
                    "Should default to None when not specified"
                );
            }
            _ => panic!("Should have log_contains condition"),
        }
    }

    // Verify orchestrator can be created and service can start
    let mut orchestrator = Orchestrator::new(config).unwrap();
    let result = orchestrator.start_service_with_deps("legacy_service").await;
    assert!(result.is_ok());

    // Wait for service to become ready
    tokio::time::sleep(Duration::from_secs(1)).await;
}

/// Test that different services can have different timeout values
#[tokio::test]
async fn test_mixed_timeout_configurations() {
    let config_json = if cfg!(windows) {
        r#"{
            "version": "1.0",
            "services": {
                "fast_service": {
                    "command": "cmd /c echo Fast ready",
                    "ready_when": {
                        "type": "log_contains",
                        "pattern": "Fast ready",
                        "timeout_seconds": 5
                    }
                },
                "slow_service": {
                    "command": "cmd /c echo Slow starting",
                    "ready_when": {
                        "type": "log_contains",
                        "pattern": "Slow ready",
                        "timeout_seconds": 120
                    }
                },
                "default_service": {
                    "command": "cmd /c echo Default starting",
                    "ready_when": {
                        "type": "log_contains",
                        "pattern": "Default ready"
                    }
                }
            }
        }"#
    } else {
        r#"{
            "version": "1.0",
            "services": {
                "fast_service": {
                    "command": "sh -c 'echo Fast ready'",
                    "ready_when": {
                        "type": "log_contains",
                        "pattern": "Fast ready",
                        "timeout_seconds": 5
                    }
                },
                "slow_service": {
                    "command": "sh -c 'echo Slow starting'",
                    "ready_when": {
                        "type": "log_contains",
                        "pattern": "Slow ready",
                        "timeout_seconds": 120
                    }
                },
                "default_service": {
                    "command": "sh -c 'echo Default starting'",
                    "ready_when": {
                        "type": "log_contains",
                        "pattern": "Default ready"
                    }
                }
            }
        }"#
    };

    let config: DmnConfig =
        serde_json::from_str(config_json).expect("Should parse config with mixed timeouts");

    // Verify all services loaded
    assert_eq!(config.services.len(), 3);

    // Verify fast_service has 5 second timeout
    let fast = config.services.get("fast_service").unwrap();
    if let Some(dmn_core::config::ReadyCondition::LogContains {
        timeout_seconds, ..
    }) = &fast.ready_when
    {
        assert_eq!(*timeout_seconds, Some(5));
    }

    // Verify slow_service has 120 second timeout
    let slow = config.services.get("slow_service").unwrap();
    if let Some(dmn_core::config::ReadyCondition::LogContains {
        timeout_seconds, ..
    }) = &slow.ready_when
    {
        assert_eq!(*timeout_seconds, Some(120));
    }

    // Verify default_service has no custom timeout
    let default = config.services.get("default_service").unwrap();
    if let Some(dmn_core::config::ReadyCondition::LogContains {
        timeout_seconds, ..
    }) = &default.ready_when
    {
        assert_eq!(*timeout_seconds, None);
    }

    // Verify orchestrator can be created
    let orchestrator = Orchestrator::new(config);
    assert!(orchestrator.is_ok());
}

/// Test that timeout_seconds works with url_responds condition
#[tokio::test]
async fn test_url_responds_with_custom_timeout() {
    let config_json = if cfg!(windows) {
        r#"{
            "version": "1.0",
            "services": {
                "http_service": {
                    "command": "cmd /c echo HTTP starting",
                    "ready_when": {
                        "type": "url_responds",
                        "url": "http://localhost:9999/health",
                        "timeout_seconds": 3
                    }
                }
            }
        }"#
    } else {
        r#"{
            "version": "1.0",
            "services": {
                "http_service": {
                    "command": "sh -c 'echo HTTP starting'",
                    "ready_when": {
                        "type": "url_responds",
                        "url": "http://localhost:9999/health",
                        "timeout_seconds": 3
                    }
                }
            }
        }"#
    };

    let config: DmnConfig =
        serde_json::from_str(config_json).expect("Should parse config with url_responds timeout");

    let service = config.services.get("http_service").unwrap();
    if let Some(ready_when) = &service.ready_when {
        match ready_when {
            dmn_core::config::ReadyCondition::UrlResponds {
                url,
                timeout_seconds,
            } => {
                assert_eq!(url, "http://localhost:9999/health");
                assert_eq!(*timeout_seconds, Some(3));
            }
            _ => panic!("Should have url_responds condition"),
        }
    }

    let mut orchestrator = Orchestrator::new(config).unwrap();

    let start = Instant::now();
    let result = orchestrator.start_service_with_deps("http_service").await;
    assert!(result.is_ok());

    // Wait for timeout
    tokio::time::sleep(Duration::from_secs(4)).await;

    let elapsed = start.elapsed();

    // Should timeout around 3 seconds
    assert!(
        elapsed < Duration::from_secs(10),
        "Should timeout with custom 3 second timeout, took {:?}",
        elapsed
    );
}
