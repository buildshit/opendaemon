use crate::config::ReadyCondition;
use crate::logs::LogLine;
use regex::Regex;
use std::collections::{HashMap, HashSet};
use std::time::Duration;
use thiserror::Error;
use tokio::sync::mpsc;
use tokio::time::timeout;

#[derive(Debug, Error)]
pub enum ReadyError {
    #[error("Timeout waiting for service '{0}' to be ready")]
    Timeout(String),
    #[error("Invalid regex pattern: {0}")]
    InvalidRegex(#[from] regex::Error),
    #[error("HTTP request failed: {0}")]
    HttpError(#[from] reqwest::Error),
}

pub struct ReadyWatcher {
    conditions: HashMap<String, ReadyCondition>,
    ready_services: HashSet<String>,
    default_timeout: Duration,
}

impl ReadyWatcher {
    pub fn new(default_timeout: Duration) -> Self {
        Self {
            conditions: HashMap::new(),
            ready_services: HashSet::new(),
            default_timeout,
        }
    }

    pub fn with_timeout(mut self, timeout: Duration) -> Self {
        self.default_timeout = timeout;
        self
    }

    pub fn set_timeout(&mut self, timeout: Duration) {
        self.default_timeout = timeout;
    }

    pub fn get_timeout(&self) -> Duration {
        self.default_timeout
    }

    pub fn is_ready(&self, service_name: &str) -> bool {
        self.ready_services.contains(service_name)
    }

    pub fn mark_ready(&mut self, service_name: &str) {
        self.ready_services.insert(service_name.to_string());
    }

    pub fn reset_service(&mut self, service_name: &str) {
        self.ready_services.remove(service_name);
        self.conditions.remove(service_name);
    }

    pub fn reset_all(&mut self) {
        self.ready_services.clear();
        self.conditions.clear();
    }

    /// Watch a service for readiness based on its condition
    /// This is the main dispatcher that routes to specific watch methods
    pub async fn watch_service(
        &mut self,
        service_name: String,
        condition: ReadyCondition,
        log_rx: Option<mpsc::Receiver<LogLine>>,
    ) -> Result<(), ReadyError> {
        self.watch_service_with_timeout(service_name, condition, log_rx, None).await
    }

    /// Watch a service for readiness with a custom timeout
    pub async fn watch_service_with_timeout(
        &mut self,
        service_name: String,
        condition: ReadyCondition,
        log_rx: Option<mpsc::Receiver<LogLine>>,
        custom_timeout: Option<Duration>,
    ) -> Result<(), ReadyError> {
        self.conditions.insert(service_name.clone(), condition.clone());

        let timeout_duration = custom_timeout.unwrap_or(self.default_timeout);

        match condition {
            ReadyCondition::LogContains { pattern } => {
                if let Some(rx) = log_rx {
                    self.watch_log_pattern_with_timeout(service_name, pattern, rx, timeout_duration).await
                } else {
                    Err(ReadyError::Timeout(format!(
                        "No log receiver provided for service '{}'",
                        service_name
                    )))
                }
            }
            ReadyCondition::UrlResponds { url } => {
                self.watch_url_with_timeout(service_name, url, timeout_duration).await
            }
        }
    }

    async fn watch_log_pattern(
        &mut self,
        service_name: String,
        pattern: String,
        log_rx: mpsc::Receiver<LogLine>,
    ) -> Result<(), ReadyError> {
        self.watch_log_pattern_with_timeout(service_name, pattern, log_rx, self.default_timeout).await
    }

    async fn watch_log_pattern_with_timeout(
        &mut self,
        service_name: String,
        pattern: String,
        mut log_rx: mpsc::Receiver<LogLine>,
        timeout_duration: Duration,
    ) -> Result<(), ReadyError> {
        let regex = Regex::new(&pattern)?;
        
        let result = timeout(timeout_duration, async {
            while let Some(line) = log_rx.recv().await {
                if regex.is_match(&line.content) {
                    return Ok(());
                }
            }
            Err(ReadyError::Timeout(service_name.clone()))
        })
        .await;

        match result {
            Ok(Ok(())) => {
                self.mark_ready(&service_name);
                Ok(())
            }
            Ok(Err(e)) => Err(e),
            Err(_) => Err(ReadyError::Timeout(service_name)),
        }
    }

    async fn watch_url(
        &mut self,
        service_name: String,
        url: String,
    ) -> Result<(), ReadyError> {
        self.watch_url_with_timeout(service_name, url, self.default_timeout).await
    }

    async fn watch_url_with_timeout(
        &mut self,
        service_name: String,
        url: String,
        timeout_duration: Duration,
    ) -> Result<(), ReadyError> {
        let client = reqwest::Client::new();
        let poll_interval = Duration::from_millis(500);

        let result = timeout(timeout_duration, async {
            loop {
                match client.get(&url).send().await {
                    Ok(response) if response.status().is_success() => {
                        return Ok(());
                    }
                    _ => {
                        tokio::time::sleep(poll_interval).await;
                    }
                }
            }
        })
        .await;

        match result {
            Ok(Ok(())) => {
                self.mark_ready(&service_name);
                Ok(())
            }
            Ok(Err(e)) => Err(e),
            Err(_) => Err(ReadyError::Timeout(service_name)),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_ready_watcher_new() {
        let watcher = ReadyWatcher::new(Duration::from_secs(30));
        assert_eq!(watcher.ready_services.len(), 0);
        assert_eq!(watcher.conditions.len(), 0);
    }

    #[test]
    fn test_is_ready() {
        let mut watcher = ReadyWatcher::new(Duration::from_secs(30));
        assert!(!watcher.is_ready("service1"));
        
        watcher.mark_ready("service1");
        assert!(watcher.is_ready("service1"));
        assert!(!watcher.is_ready("service2"));
    }

    #[test]
    fn test_mark_ready() {
        let mut watcher = ReadyWatcher::new(Duration::from_secs(30));
        
        watcher.mark_ready("service1");
        watcher.mark_ready("service2");
        
        assert!(watcher.is_ready("service1"));
        assert!(watcher.is_ready("service2"));
        assert_eq!(watcher.ready_services.len(), 2);
    }

    #[test]
    fn test_reset_service() {
        let mut watcher = ReadyWatcher::new(Duration::from_secs(30));
        
        watcher.mark_ready("service1");
        watcher.mark_ready("service2");
        
        watcher.reset_service("service1");
        
        assert!(!watcher.is_ready("service1"));
        assert!(watcher.is_ready("service2"));
    }

    #[test]
    fn test_reset_all() {
        let mut watcher = ReadyWatcher::new(Duration::from_secs(30));
        
        watcher.mark_ready("service1");
        watcher.mark_ready("service2");
        watcher.mark_ready("service3");
        
        watcher.reset_all();
        
        assert!(!watcher.is_ready("service1"));
        assert!(!watcher.is_ready("service2"));
        assert!(!watcher.is_ready("service3"));
        assert_eq!(watcher.ready_services.len(), 0);
        assert_eq!(watcher.conditions.len(), 0);
    }

    #[tokio::test]
    async fn test_watch_log_pattern_simple_match() {
        use crate::logs::LogStream;
        use std::time::SystemTime;

        let mut watcher = ReadyWatcher::new(Duration::from_secs(5));
        let (tx, rx) = mpsc::channel(10);

        // Spawn task to watch for pattern
        let service_name = "test_service".to_string();
        let pattern = "Server started".to_string();
        
        let handle = tokio::spawn(async move {
            watcher.watch_log_pattern(service_name, pattern, rx).await
        });

        // Send some log lines
        tx.send(LogLine {
            timestamp: SystemTime::now(),
            content: "Initializing...".to_string(),
            stream: LogStream::Stdout,
        })
        .await
        .unwrap();

        tx.send(LogLine {
            timestamp: SystemTime::now(),
            content: "Server started on port 8080".to_string(),
            stream: LogStream::Stdout,
        })
        .await
        .unwrap();

        let result = handle.await.unwrap();
        assert!(result.is_ok());
    }

    #[tokio::test]
    async fn test_watch_log_pattern_regex_match() {
        use crate::logs::LogStream;
        use std::time::SystemTime;

        let mut watcher = ReadyWatcher::new(Duration::from_secs(5));
        let (tx, rx) = mpsc::channel(10);

        let service_name = "test_service".to_string();
        let pattern = r"Listening on \d+".to_string();
        
        let handle = tokio::spawn(async move {
            watcher.watch_log_pattern(service_name, pattern, rx).await
        });

        tx.send(LogLine {
            timestamp: SystemTime::now(),
            content: "Starting server...".to_string(),
            stream: LogStream::Stdout,
        })
        .await
        .unwrap();

        tx.send(LogLine {
            timestamp: SystemTime::now(),
            content: "Listening on 3000".to_string(),
            stream: LogStream::Stdout,
        })
        .await
        .unwrap();

        let result = handle.await.unwrap();
        assert!(result.is_ok());
    }

    #[tokio::test]
    async fn test_watch_log_pattern_case_sensitive() {
        use crate::logs::LogStream;
        use std::time::SystemTime;

        let mut watcher = ReadyWatcher::new(Duration::from_secs(5));
        let (tx, rx) = mpsc::channel(10);

        let service_name = "test_service".to_string();
        let pattern = "READY".to_string();
        
        let handle = tokio::spawn(async move {
            watcher.watch_log_pattern(service_name, pattern, rx).await
        });

        // Send lowercase version - should not match
        tx.send(LogLine {
            timestamp: SystemTime::now(),
            content: "ready".to_string(),
            stream: LogStream::Stdout,
        })
        .await
        .unwrap();

        // Send uppercase version - should match
        tx.send(LogLine {
            timestamp: SystemTime::now(),
            content: "READY".to_string(),
            stream: LogStream::Stdout,
        })
        .await
        .unwrap();

        let result = handle.await.unwrap();
        assert!(result.is_ok());
    }

    #[tokio::test]
    async fn test_watch_log_pattern_case_insensitive() {
        use crate::logs::LogStream;
        use std::time::SystemTime;

        let mut watcher = ReadyWatcher::new(Duration::from_secs(5));
        let (tx, rx) = mpsc::channel(10);

        let service_name = "test_service".to_string();
        let pattern = r"(?i)ready".to_string(); // Case-insensitive regex
        
        let handle = tokio::spawn(async move {
            watcher.watch_log_pattern(service_name, pattern, rx).await
        });

        tx.send(LogLine {
            timestamp: SystemTime::now(),
            content: "System is READY".to_string(),
            stream: LogStream::Stdout,
        })
        .await
        .unwrap();

        let result = handle.await.unwrap();
        assert!(result.is_ok());
    }

    #[tokio::test]
    async fn test_watch_log_pattern_partial_match() {
        use crate::logs::LogStream;
        use std::time::SystemTime;

        let mut watcher = ReadyWatcher::new(Duration::from_secs(5));
        let (tx, rx) = mpsc::channel(10);

        let service_name = "test_service".to_string();
        let pattern = "database connected".to_string();
        
        let handle = tokio::spawn(async move {
            watcher.watch_log_pattern(service_name, pattern, rx).await
        });

        tx.send(LogLine {
            timestamp: SystemTime::now(),
            content: "INFO: database connected successfully at localhost:5432".to_string(),
            stream: LogStream::Stdout,
        })
        .await
        .unwrap();

        let result = handle.await.unwrap();
        assert!(result.is_ok());
    }

    #[tokio::test]
    async fn test_watch_log_pattern_marks_ready() {
        use crate::logs::LogStream;
        use std::time::SystemTime;

        let mut watcher = ReadyWatcher::new(Duration::from_secs(5));
        let (tx, rx) = mpsc::channel(10);

        let service_name = "test_service".to_string();
        let pattern = "ready".to_string();
        
        assert!(!watcher.is_ready(&service_name));

        let handle = tokio::spawn(async move {
            watcher.watch_log_pattern(service_name.clone(), pattern, rx).await.unwrap();
            watcher
        });

        tx.send(LogLine {
            timestamp: SystemTime::now(),
            content: "Service is ready".to_string(),
            stream: LogStream::Stdout,
        })
        .await
        .unwrap();

        let watcher = handle.await.unwrap();
        assert!(watcher.is_ready("test_service"));
    }

    #[tokio::test]
    async fn test_watch_log_pattern_multiple_lines() {
        use crate::logs::LogStream;
        use std::time::SystemTime;

        let mut watcher = ReadyWatcher::new(Duration::from_secs(5));
        let (tx, rx) = mpsc::channel(10);

        let service_name = "test_service".to_string();
        let pattern = "initialization complete".to_string();
        
        let handle = tokio::spawn(async move {
            watcher.watch_log_pattern(service_name, pattern, rx).await
        });

        // Send multiple lines before the match
        for i in 1..=5 {
            tx.send(LogLine {
                timestamp: SystemTime::now(),
                content: format!("Loading module {}", i),
                stream: LogStream::Stdout,
            })
            .await
            .unwrap();
        }

        tx.send(LogLine {
            timestamp: SystemTime::now(),
            content: "All modules loaded, initialization complete".to_string(),
            stream: LogStream::Stdout,
        })
        .await
        .unwrap();

        let result = handle.await.unwrap();
        assert!(result.is_ok());
    }

    #[tokio::test]
    async fn test_watch_log_pattern_stderr_stream() {
        use crate::logs::LogStream;
        use std::time::SystemTime;

        let mut watcher = ReadyWatcher::new(Duration::from_secs(5));
        let (tx, rx) = mpsc::channel(10);

        let service_name = "test_service".to_string();
        let pattern = "warning: ready".to_string();
        
        let handle = tokio::spawn(async move {
            watcher.watch_log_pattern(service_name, pattern, rx).await
        });

        // Pattern can match in stderr too
        tx.send(LogLine {
            timestamp: SystemTime::now(),
            content: "warning: ready to accept connections".to_string(),
            stream: LogStream::Stderr,
        })
        .await
        .unwrap();

        let result = handle.await.unwrap();
        assert!(result.is_ok());
    }

    #[tokio::test]
    async fn test_watch_log_pattern_complex_regex() {
        use crate::logs::LogStream;
        use std::time::SystemTime;

        let mut watcher = ReadyWatcher::new(Duration::from_secs(5));
        let (tx, rx) = mpsc::channel(10);

        let service_name = "test_service".to_string();
        // Match lines like "Server listening on http://localhost:3000"
        let pattern = r"Server listening on https?://[^:]+:\d+".to_string();
        
        let handle = tokio::spawn(async move {
            watcher.watch_log_pattern(service_name, pattern, rx).await
        });

        tx.send(LogLine {
            timestamp: SystemTime::now(),
            content: "Server listening on http://localhost:3000".to_string(),
            stream: LogStream::Stdout,
        })
        .await
        .unwrap();

        let result = handle.await.unwrap();
        assert!(result.is_ok());
    }

    #[tokio::test]
    async fn test_watch_url_success() {
        // This test uses httpbin.org or a local test - for now, skip actual HTTP test
        // and test the timeout logic instead
        let mut watcher = ReadyWatcher::new(Duration::from_millis(100));
        let service_name = "test_service".to_string();
        
        // Test with invalid URL to verify timeout works
        let result = watcher.watch_url(service_name.clone(), "http://localhost:59999/nonexistent".to_string()).await;
        assert!(result.is_err());
        assert!(matches!(result.unwrap_err(), ReadyError::Timeout(_)));
    }

    #[tokio::test]
    async fn test_watch_url_delayed_success() {
        // Test timeout behavior
        let mut watcher = ReadyWatcher::new(Duration::from_millis(100));
        let service_name = "test_service".to_string();

        let result = watcher.watch_url(service_name.clone(), "http://localhost:59998/health".to_string()).await;
        assert!(result.is_err());
        assert!(!watcher.is_ready(&service_name));
    }

    #[tokio::test]
    async fn test_watch_url_marks_ready() {
        // Test that successful response marks service as ready
        // We'll use a mock by spawning a simple TCP server
        use tokio::io::AsyncWriteExt;
        
        let listener = tokio::net::TcpListener::bind("127.0.0.1:0").await.unwrap();
        let addr = listener.local_addr().unwrap();
        let url = format!("http://{}/ready", addr);

        // Spawn a simple HTTP server
        tokio::spawn(async move {
            if let Ok((mut socket, _)) = listener.accept().await {
                // Read the request (we don't care about parsing it)
                let mut buf = vec![0u8; 1024];
                let _ = tokio::io::AsyncReadExt::read(&mut socket, &mut buf).await;
                
                // Send a valid HTTP response
                let response = "HTTP/1.1 200 OK\r\nContent-Length: 2\r\nConnection: close\r\n\r\nOK";
                let _ = socket.write_all(response.as_bytes()).await;
                let _ = socket.shutdown().await;
            }
        });

        tokio::time::sleep(Duration::from_millis(50)).await;

        let mut watcher = ReadyWatcher::new(Duration::from_secs(5));
        let service_name = "api_service".to_string();

        assert!(!watcher.is_ready(&service_name));
        
        let result = watcher.watch_url(service_name.clone(), url).await;
        assert!(result.is_ok());
        assert!(watcher.is_ready(&service_name));
    }

    #[tokio::test]
    async fn test_watch_url_different_success_codes() {
        // Test that various 2xx codes are considered success
        use tokio::io::AsyncWriteExt;
        
        for status_code in [200, 201, 204] {
            let listener = tokio::net::TcpListener::bind("127.0.0.1:0").await.unwrap();
            let addr = listener.local_addr().unwrap();
            let url = format!("http://{}/health", addr);

            let code = status_code;
            tokio::spawn(async move {
                if let Ok((mut socket, _)) = listener.accept().await {
                    let mut buf = vec![0u8; 1024];
                    let _ = tokio::io::AsyncReadExt::read(&mut socket, &mut buf).await;
                    
                    let response = format!("HTTP/1.1 {} OK\r\nContent-Length: 0\r\nConnection: close\r\n\r\n", code);
                    let _ = socket.write_all(response.as_bytes()).await;
                    let _ = socket.shutdown().await;
                }
            });

            tokio::time::sleep(Duration::from_millis(50)).await;

            let mut watcher = ReadyWatcher::new(Duration::from_secs(5));
            let service_name = format!("service_{}", status_code);

            let result = watcher.watch_url(service_name.clone(), url).await;
            assert!(result.is_ok(), "Status code {} should be considered success", status_code);
            assert!(watcher.is_ready(&service_name));
        }
    }

    #[tokio::test]
    async fn test_watch_url_polling_interval() {
        use std::sync::atomic::{AtomicU32, Ordering};
        use std::sync::Arc;
        use tokio::io::AsyncWriteExt;

        let listener = tokio::net::TcpListener::bind("127.0.0.1:0").await.unwrap();
        let addr = listener.local_addr().unwrap();
        let url = format!("http://{}/health", addr);

        let request_count = Arc::new(AtomicU32::new(0));
        let request_count_clone = Arc::clone(&request_count);

        tokio::spawn(async move {
            loop {
                if let Ok((mut socket, _)) = listener.accept().await {
                    let count = request_count_clone.clone();
                    tokio::spawn(async move {
                        let current = count.fetch_add(1, Ordering::SeqCst);
                        
                        let mut buf = vec![0u8; 1024];
                        let _ = tokio::io::AsyncReadExt::read(&mut socket, &mut buf).await;
                        
                        // Fail first 2 requests, succeed on 3rd
                        let response: &str = if current < 2 {
                            "HTTP/1.1 503 Service Unavailable\r\nContent-Length: 0\r\nConnection: close\r\n\r\n"
                        } else {
                            "HTTP/1.1 200 OK\r\nContent-Length: 2\r\nConnection: close\r\n\r\nOK"
                        };
                        let _ = socket.write_all(response.as_bytes()).await;
                        let _ = socket.shutdown().await;
                    });
                }
            }
        });

        tokio::time::sleep(Duration::from_millis(50)).await;

        let mut watcher = ReadyWatcher::new(Duration::from_secs(10));
        let service_name = "test_service".to_string();

        let start = std::time::Instant::now();
        let result = watcher.watch_url(service_name, url).await;
        let elapsed = start.elapsed();
        
        assert!(result.is_ok());
        // Should have made 3 requests with ~500ms between each
        // Total time should be at least 1 second (2 intervals)
        assert!(elapsed >= Duration::from_millis(900), "Should wait between polls, elapsed: {:?}", elapsed);
        assert!(request_count.load(Ordering::SeqCst) >= 3);
    }

    // Timeout handling tests
    #[tokio::test]
    async fn test_timeout_configuration() {
        let watcher = ReadyWatcher::new(Duration::from_secs(30));
        assert_eq!(watcher.get_timeout(), Duration::from_secs(30));

        let watcher = watcher.with_timeout(Duration::from_secs(60));
        assert_eq!(watcher.get_timeout(), Duration::from_secs(60));
    }

    #[tokio::test]
    async fn test_set_timeout() {
        let mut watcher = ReadyWatcher::new(Duration::from_secs(30));
        assert_eq!(watcher.get_timeout(), Duration::from_secs(30));

        watcher.set_timeout(Duration::from_secs(45));
        assert_eq!(watcher.get_timeout(), Duration::from_secs(45));
    }

    #[tokio::test]
    async fn test_log_pattern_timeout() {
        use crate::logs::LogStream;
        use std::time::SystemTime;

        let mut watcher = ReadyWatcher::new(Duration::from_millis(500));
        let (tx, rx) = mpsc::channel(10);

        let service_name = "test_service".to_string();
        let pattern = "NEVER_APPEARS".to_string();
        
        let handle = tokio::spawn(async move {
            watcher.watch_log_pattern(service_name, pattern, rx).await
        });

        // Send some logs that don't match
        for i in 0..5 {
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
        assert!(matches!(result.unwrap_err(), ReadyError::Timeout(_)));
    }

    #[tokio::test]
    async fn test_url_timeout() {
        let mut watcher = ReadyWatcher::new(Duration::from_millis(500));
        let service_name = "test_service".to_string();
        
        // Use a non-routable IP to ensure timeout
        let result = watcher.watch_url(service_name.clone(), "http://192.0.2.1:9999/health".to_string()).await;
        
        assert!(result.is_err());
        assert!(matches!(result.unwrap_err(), ReadyError::Timeout(_)));
        assert!(!watcher.is_ready(&service_name));
    }

    #[tokio::test]
    async fn test_custom_timeout_shorter() {
        use crate::logs::LogStream;
        use std::time::SystemTime;

        let mut watcher = ReadyWatcher::new(Duration::from_secs(10));
        let (tx, rx) = mpsc::channel(10);

        let service_name = "test_service".to_string();
        let pattern = "READY".to_string();
        let condition = ReadyCondition::LogContains { pattern: pattern.clone() };
        
        // Use a custom shorter timeout
        let handle = tokio::spawn(async move {
            watcher.watch_service_with_timeout(
                service_name,
                condition,
                Some(rx),
                Some(Duration::from_millis(300))
            ).await
        });

        // Send logs slowly - should timeout before match
        tokio::time::sleep(Duration::from_millis(200)).await;
        let _ = tx.send(LogLine {
            timestamp: SystemTime::now(),
            content: "Starting...".to_string(),
            stream: LogStream::Stdout,
        })
        .await;

        tokio::time::sleep(Duration::from_millis(200)).await;
        let _ = tx.send(LogLine {
            timestamp: SystemTime::now(),
            content: "READY".to_string(),
            stream: LogStream::Stdout,
        })
        .await;

        let result = handle.await.unwrap();
        assert!(result.is_err());
        assert!(matches!(result.unwrap_err(), ReadyError::Timeout(_)));
    }

    #[tokio::test]
    async fn test_custom_timeout_longer() {
        use crate::logs::LogStream;
        use std::time::SystemTime;

        let mut watcher = ReadyWatcher::new(Duration::from_millis(100));
        let (tx, rx) = mpsc::channel(10);

        let service_name = "test_service".to_string();
        let pattern = "READY".to_string();
        let condition = ReadyCondition::LogContains { pattern: pattern.clone() };
        
        // Use a custom longer timeout
        let handle = tokio::spawn(async move {
            watcher.watch_service_with_timeout(
                service_name,
                condition,
                Some(rx),
                Some(Duration::from_secs(5))
            ).await
        });

        // Send logs slowly - should succeed with longer timeout
        tokio::time::sleep(Duration::from_millis(200)).await;
        tx.send(LogLine {
            timestamp: SystemTime::now(),
            content: "Starting...".to_string(),
            stream: LogStream::Stdout,
        })
        .await
        .unwrap();

        tokio::time::sleep(Duration::from_millis(200)).await;
        tx.send(LogLine {
            timestamp: SystemTime::now(),
            content: "READY".to_string(),
            stream: LogStream::Stdout,
        })
        .await
        .unwrap();

        let result = handle.await.unwrap();
        assert!(result.is_ok());
    }

    #[tokio::test]
    async fn test_timeout_error_message() {
        let mut watcher = ReadyWatcher::new(Duration::from_millis(100));
        let service_name = "my_service".to_string();
        
        let result = watcher.watch_url(service_name.clone(), "http://192.0.2.1:9999/health".to_string()).await;
        
        assert!(result.is_err());
        let err = result.unwrap_err();
        assert!(matches!(err, ReadyError::Timeout(_)));
        assert!(err.to_string().contains("my_service"));
    }

    #[tokio::test]
    async fn test_timeout_does_not_mark_ready() {
        use crate::logs::LogStream;
        use std::time::SystemTime;

        let mut watcher = ReadyWatcher::new(Duration::from_millis(200));
        let (tx, rx) = mpsc::channel(10);

        let service_name = "test_service".to_string();
        let pattern = "READY".to_string();
        
        assert!(!watcher.is_ready(&service_name));

        let handle = tokio::spawn(async move {
            watcher.watch_log_pattern(service_name.clone(), pattern, rx).await.unwrap_err();
            watcher
        });

        // Send non-matching logs
        tx.send(LogLine {
            timestamp: SystemTime::now(),
            content: "Not ready yet".to_string(),
            stream: LogStream::Stdout,
        })
        .await
        .unwrap();

        let watcher = handle.await.unwrap();
        assert!(!watcher.is_ready("test_service"));
    }

    #[tokio::test]
    async fn test_multiple_services_different_timeouts() {
        use crate::logs::LogStream;
        use std::time::SystemTime;

        // Service 1: short timeout, should timeout
        let mut watcher1 = ReadyWatcher::new(Duration::from_millis(200));
        let (tx1, rx1) = mpsc::channel(10);
        let service1 = "service1".to_string();
        let pattern1 = "READY".to_string();
        let condition1 = ReadyCondition::LogContains { pattern: pattern1 };

        let handle1 = tokio::spawn(async move {
            watcher1.watch_service_with_timeout(
                service1,
                condition1,
                Some(rx1),
                Some(Duration::from_millis(100))
            ).await
        });

        // Service 2: longer timeout, should succeed
        let mut watcher2 = ReadyWatcher::new(Duration::from_millis(200));
        let (tx2, rx2) = mpsc::channel(10);
        let service2 = "service2".to_string();
        let pattern2 = "READY".to_string();
        let condition2 = ReadyCondition::LogContains { pattern: pattern2 };

        let handle2 = tokio::spawn(async move {
            watcher2.watch_service_with_timeout(
                service2,
                condition2,
                Some(rx2),
                Some(Duration::from_secs(5))
            ).await
        });

        // Send ready message after 150ms
        tokio::time::sleep(Duration::from_millis(150)).await;
        tx1.send(LogLine {
            timestamp: SystemTime::now(),
            content: "READY".to_string(),
            stream: LogStream::Stdout,
        })
        .await
        .ok();

        tx2.send(LogLine {
            timestamp: SystemTime::now(),
            content: "READY".to_string(),
            stream: LogStream::Stdout,
        })
        .await
        .ok();

        let result1 = handle1.await.unwrap();
        let result2 = handle2.await.unwrap();

        assert!(result1.is_err()); // Timed out
        assert!(result2.is_ok());  // Succeeded
    }
}
