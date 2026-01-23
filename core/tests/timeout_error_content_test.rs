use dmn_core::ready::ReadyWatcher;
use dmn_core::config::ReadyCondition;
use dmn_core::logs::{LogLine, LogStream};
use std::time::{Duration, SystemTime};
use tokio::sync::mpsc;

#[tokio::test]
async fn test_log_timeout_error_message_contains_all_details() {
    let mut watcher = ReadyWatcher::new(Duration::from_millis(500));
    let (tx, rx) = mpsc::channel(10);

    let service_name = "test_service".to_string();
    let pattern = "READY_SIGNAL".to_string();
    let condition = ReadyCondition::LogContains {
        pattern: pattern.clone(),
        timeout_seconds: None,
    };
    
    let handle = tokio::spawn(async move {
        watcher.watch_service_with_timeout(
            service_name,
            condition,
            Some(rx),
            Some(Duration::from_millis(500))
        ).await
    });

    // Send some log lines that don't match
    for i in 1..=3 {
        tx.send(LogLine {
            timestamp: SystemTime::now(),
            content: format!("Log line {}", i),
            stream: LogStream::Stdout,
        })
        .await
        .unwrap();
        tokio::time::sleep(Duration::from_millis(100)).await;
    }

    let result = handle.await.unwrap();
    assert!(result.is_err());
    
    let err = result.unwrap_err();
    let err_str = err.to_string();
    
    // Verify error message contains all required information
    println!("Error message:\n{}", err_str);
    
    // Check service name
    assert!(err_str.contains("test_service"), "Error should contain service name");
    
    // Check timeout duration
    assert!(err_str.contains("0 seconds"), "Error should contain timeout duration");
    
    // Check condition details
    assert!(err_str.contains("log_contains"), "Error should contain condition type");
    assert!(err_str.contains("READY_SIGNAL"), "Error should contain pattern");
    
    // Check that it includes log lines
    assert!(err_str.contains("Last") && err_str.contains("log lines"), "Error should mention log lines");
    assert!(err_str.contains("Log line"), "Error should include actual log content");
    
    // Check troubleshooting section
    assert!(err_str.contains("Troubleshooting"), "Error should contain troubleshooting section");
    assert!(err_str.contains("timeout_seconds"), "Error should suggest timeout configuration");
}

#[tokio::test]
async fn test_url_timeout_error_message_contains_all_details() {
    let mut watcher = ReadyWatcher::new(Duration::from_millis(500));
    let service_name = "api_service".to_string();
    let url = "http://192.0.2.1:9999/health".to_string();
    let condition = ReadyCondition::UrlResponds {
        url: url.clone(),
        timeout_seconds: None,
    };
    
    let result = watcher.watch_service_with_timeout(
        service_name,
        condition,
        None,
        Some(Duration::from_millis(500))
    ).await;
    
    assert!(result.is_err());
    
    let err = result.unwrap_err();
    let err_str = err.to_string();
    
    // Verify error message contains all required information
    println!("Error message:\n{}", err_str);
    
    // Check service name
    assert!(err_str.contains("api_service"), "Error should contain service name");
    
    // Check timeout duration
    assert!(err_str.contains("0 seconds"), "Error should contain timeout duration");
    
    // Check condition details
    assert!(err_str.contains("url_responds"), "Error should contain condition type");
    assert!(err_str.contains(&url), "Error should contain URL");
    
    // Check that it includes attempt details
    assert!(err_str.contains("attempts"), "Error should mention connection attempts");
    
    // Check troubleshooting section
    assert!(err_str.contains("Troubleshooting"), "Error should contain troubleshooting section");
    assert!(err_str.contains("timeout_seconds"), "Error should suggest timeout configuration");
    assert!(err_str.contains("listening"), "Error should suggest checking if service is listening");
}

#[tokio::test]
async fn test_log_timeout_with_no_logs_shows_appropriate_message() {
    let mut watcher = ReadyWatcher::new(Duration::from_millis(300));
    let (_tx, rx) = mpsc::channel(10);

    let service_name = "silent_service".to_string();
    let pattern = "READY".to_string();
    let condition = ReadyCondition::LogContains {
        pattern,
        timeout_seconds: None,
    };
    
    let handle = tokio::spawn(async move {
        watcher.watch_service_with_timeout(
            service_name,
            condition,
            Some(rx),
            Some(Duration::from_millis(300))
        ).await
    });

    // Don't send any logs
    let result = handle.await.unwrap();
    assert!(result.is_err());
    
    let err = result.unwrap_err();
    let err_str = err.to_string();
    
    println!("Error message:\n{}", err_str);
    
    // Should indicate no logs were received
    assert!(err_str.contains("No log output received"), "Error should indicate no logs received");
}

#[tokio::test]
async fn test_log_timeout_captures_last_10_lines_only() {
    let mut watcher = ReadyWatcher::new(Duration::from_millis(800));
    let (tx, rx) = mpsc::channel(20);

    let service_name = "verbose_service".to_string();
    let pattern = "NEVER_MATCHES".to_string();
    let condition = ReadyCondition::LogContains {
        pattern,
        timeout_seconds: None,
    };
    
    let handle = tokio::spawn(async move {
        watcher.watch_service_with_timeout(
            service_name,
            condition,
            Some(rx),
            Some(Duration::from_millis(800))
        ).await
    });

    // Send 15 log lines
    for i in 1..=15 {
        tx.send(LogLine {
            timestamp: SystemTime::now(),
            content: format!("Log line {}", i),
            stream: LogStream::Stdout,
        })
        .await
        .unwrap();
        tokio::time::sleep(Duration::from_millis(40)).await;
    }

    let result = handle.await.unwrap();
    assert!(result.is_err());
    
    let err = result.unwrap_err();
    let err_str = err.to_string();
    
    println!("Error message:\n{}", err_str);
    
    // Should only show last 10 lines
    assert!(err_str.contains("Last 10 log lines"), "Error should indicate 10 log lines");
    
    // Should contain lines 6-15 (last 10)
    assert!(err_str.contains("Log line 15"), "Error should contain last line");
    assert!(err_str.contains("Log line 6"), "Error should contain 10th from last line");
    
    // Should NOT contain early lines (check for exact match with leading spaces)
    let lines: Vec<&str> = err_str.lines().collect();
    let has_line_1 = lines.iter().any(|l| l.trim() == "Log line 1");
    let has_line_5 = lines.iter().any(|l| l.trim() == "Log line 5");
    assert!(!has_line_1, "Error should not contain 'Log line 1' as a separate line");
    assert!(!has_line_5, "Error should not contain 'Log line 5' as a separate line");
}
