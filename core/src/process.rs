use crate::config::ServiceConfig;
use crate::logs::{LogBuffer, LogLine, LogStream};
use std::collections::HashMap;
use std::sync::Arc;
use std::time::{Duration, SystemTime};
use thiserror::Error;
use tokio::io::{AsyncBufReadExt, BufReader};
use tokio::process::{Child, Command};
use tokio::sync::{mpsc, Mutex};
use tokio::time::timeout;

/// Event emitted by ProcessManager for log lines
#[derive(Debug, Clone)]
pub struct LogLineEvent {
    pub service: String,
    pub line: LogLine,
}

/// Event emitted when a managed service process exits naturally
#[derive(Debug, Clone, PartialEq)]
pub struct ProcessExitEvent {
    pub service: String,
    pub status: ServiceStatus,
    pub reason: String,
}

#[derive(Debug, Clone, PartialEq)]
pub enum ServiceStatus {
    NotStarted,
    Starting,
    Running,
    Stopped,
    Failed { exit_code: i32 },
}

#[derive(Debug, Error)]
pub enum ProcessError {
    #[error("IO error: {0}")]
    Io(#[from] std::io::Error),
    #[error("Service not found: {0}")]
    ServiceNotFound(String),
    #[error("Service already running: {0}")]
    AlreadyRunning(String),
    #[error("Service not running: {0}")]
    NotRunning(String),
    #[error("Failed to parse command: {0}")]
    CommandParse(String),
    #[error("Timeout waiting for process to stop: {0}")]
    StopTimeout(String),
}

pub struct ManagedProcess {
    pub service_name: String,
    pub child: Child,
    pub stdin: Option<tokio::process::ChildStdin>,
    pub status: ServiceStatus,
    pub started_at: SystemTime,
    pub env_vars: HashMap<String, String>,
}

pub struct ProcessManager {
    processes: HashMap<String, ManagedProcess>,
    log_buffer: Arc<Mutex<LogBuffer>>,
    log_event_tx: Option<mpsc::UnboundedSender<LogLineEvent>>,
}

impl ProcessManager {
    pub fn new(log_buffer: Arc<Mutex<LogBuffer>>) -> Self {
        Self {
            processes: HashMap::new(),
            log_buffer,
            log_event_tx: None,
        }
    }

    /// Create a new ProcessManager with an event sender for log line notifications
    pub fn with_log_events(
        log_buffer: Arc<Mutex<LogBuffer>>,
        log_event_tx: mpsc::UnboundedSender<LogLineEvent>,
    ) -> Self {
        Self {
            processes: HashMap::new(),
            log_buffer,
            log_event_tx: Some(log_event_tx),
        }
    }

    /// Spawn a service process with the given configuration
    pub async fn spawn_service(
        &mut self,
        service_name: &str,
        config: &ServiceConfig,
    ) -> Result<(), ProcessError> {
        // Check if service is already running
        if let Some(process) = self.processes.get(service_name) {
            if matches!(
                process.status,
                ServiceStatus::Starting | ServiceStatus::Running
            ) {
                return Err(ProcessError::AlreadyRunning(service_name.to_string()));
            }
            // If service is stopped or failed, remove it so we can spawn a new one
            if matches!(
                process.status,
                ServiceStatus::Stopped | ServiceStatus::Failed { .. }
            ) {
                self.processes.remove(service_name);
            }
        }

        // Parse command string
        let parts = Self::parse_command(&config.command)?;
        if parts.is_empty() {
            return Err(ProcessError::CommandParse("Command is empty".to_string()));
        }

        let program = &parts[0];
        let args = &parts[1..];

        // Load environment variables from env_file if specified
        let mut env_vars = HashMap::new();
        if let Some(env_file) = &config.env_file {
            let env_path = std::path::Path::new(env_file);
            env_vars = crate::config::load_env_file(env_path).map_err(|e| {
                ProcessError::Io(std::io::Error::new(
                    std::io::ErrorKind::Other,
                    e.to_string(),
                ))
            })?;
        }

        // Spawn the process
        let mut command = Command::new(program);
        command.args(args);

        // Set environment variables
        for (key, value) in &env_vars {
            command.env(key, value);
        }

        // Pipe stdout, stderr AND stdin
        command.stdin(std::process::Stdio::piped());
        command.stdout(std::process::Stdio::piped());
        command.stderr(std::process::Stdio::piped());

        let mut child = command.spawn()?;

        // Get stdin handle
        let stdin = child.stdin.take();

        // Get stdout and stderr handles
        let stdout = child.stdout.take().ok_or_else(|| {
            ProcessError::Io(std::io::Error::new(
                std::io::ErrorKind::Other,
                "Failed to capture stdout",
            ))
        })?;

        let stderr = child.stderr.take().ok_or_else(|| {
            ProcessError::Io(std::io::Error::new(
                std::io::ErrorKind::Other,
                "Failed to capture stderr",
            ))
        })?;

        // Create managed process
        let managed_process = ManagedProcess {
            service_name: service_name.to_string(),
            child,
            stdin,
            status: ServiceStatus::Starting,
            started_at: SystemTime::now(),
            env_vars: env_vars.clone(),
        };

        self.processes
            .insert(service_name.to_string(), managed_process);

        // Spawn async tasks to read stdout and stderr
        let service_name_clone = service_name.to_string();
        let log_buffer_clone = Arc::clone(&self.log_buffer);
        let log_event_tx_clone = self.log_event_tx.clone();
        tokio::spawn(async move {
            Self::stream_output(
                service_name_clone,
                stdout,
                LogStream::Stdout,
                log_buffer_clone,
                log_event_tx_clone,
            )
            .await;
        });

        let service_name_clone = service_name.to_string();
        let log_buffer_clone = Arc::clone(&self.log_buffer);
        let log_event_tx_clone = self.log_event_tx.clone();
        tokio::spawn(async move {
            Self::stream_output(
                service_name_clone,
                stderr,
                LogStream::Stderr,
                log_buffer_clone,
                log_event_tx_clone,
            )
            .await;
        });

        Ok(())
    }

    /// Stream output from stdout or stderr to the log buffer and emit events
    async fn stream_output<R>(
        service_name: String,
        reader: R,
        stream: LogStream,
        log_buffer: Arc<Mutex<LogBuffer>>,
        log_event_tx: Option<mpsc::UnboundedSender<LogLineEvent>>,
    ) where
        R: tokio::io::AsyncRead + Unpin,
    {
        let mut lines = BufReader::new(reader).lines();

        while let Ok(Some(line)) = lines.next_line().await {
            let log_line = LogLine {
                timestamp: SystemTime::now(),
                content: line,
                stream: stream.clone(),
            };

            // Store in log buffer
            {
                let mut buffer = log_buffer.lock().await;
                buffer.append(&service_name, log_line.clone());
            }

            // Emit log event if sender is available
            if let Some(ref tx) = log_event_tx {
                let event = LogLineEvent {
                    service: service_name.clone(),
                    line: log_line,
                };
                // Ignore send errors - receiver may have been dropped
                let _ = tx.send(event);
            }
        }
    }

    /// Stop a service gracefully with SIGTERM, then force kill if needed
    ///
    /// NOTE: When stop_service is called explicitly, the service is always marked as
    /// `Stopped` regardless of exit code. This is because killed processes often have
    /// non-zero exit codes (e.g., 130 for SIGINT, 137 for SIGKILL on Unix, or various
    /// codes on Windows). The `Failed` status should only be used when a process
    /// terminates unexpectedly on its own with an error.
    pub async fn stop_service(&mut self, service_name: &str) -> Result<(), ProcessError> {
        let process = self
            .processes
            .get_mut(service_name)
            .ok_or_else(|| ProcessError::ServiceNotFound(service_name.to_string()))?;

        // Check if process is actually running
        if matches!(
            process.status,
            ServiceStatus::Stopped | ServiceStatus::Failed { .. }
        ) {
            // Already stopped, just return success
            return Ok(());
        }

        // Try graceful shutdown with SIGTERM (or equivalent on Windows)
        #[cfg(unix)]
        {
            use nix::sys::signal::{kill, Signal};
            use nix::unistd::Pid;

            if let Some(pid) = process.child.id() {
                let _ = kill(Pid::from_raw(pid as i32), Signal::SIGTERM);
            }
        }

        #[cfg(windows)]
        {
            // Use taskkill /T so wrapper commands (cmd/powershell/npm) don't leave
            // descendants running in the background.
            if let Some(pid) = process.child.id() {
                if Self::kill_process_tree_windows(pid, true).await.is_err() {
                    // Fallback to direct kill if taskkill fails unexpectedly.
                    let _ = process.child.kill().await;
                }
            } else {
                let _ = process.child.kill().await;
            }
        }

        // Wait for process to exit with timeout
        let wait_result = timeout(Duration::from_secs(10), process.child.wait()).await;

        match wait_result {
            Ok(Ok(_exit_status)) => {
                // Process exited within timeout
                // Always mark as Stopped when stop_service is called explicitly,
                // regardless of exit code. Killed processes often have non-zero
                // exit codes which doesn't mean they "failed".
                process.status = ServiceStatus::Stopped;
                Ok(())
            }
            Ok(Err(e)) => {
                // Error waiting for process
                Err(ProcessError::Io(e))
            }
            Err(_) => {
                // Timeout - force kill
                #[cfg(windows)]
                {
                    if let Some(pid) = process.child.id() {
                        if Self::kill_process_tree_windows(pid, true).await.is_err() {
                            process.child.kill().await?;
                        }
                    } else {
                        process.child.kill().await?;
                    }
                }

                #[cfg(not(windows))]
                {
                    process.child.kill().await?;
                }

                process.child.wait().await?;
                process.status = ServiceStatus::Stopped;
                Ok(())
            }
        }
    }

    /// Restart a service (stop then start)
    pub async fn restart_service(
        &mut self,
        service_name: &str,
        config: &ServiceConfig,
    ) -> Result<(), ProcessError> {
        // Stop the service if it's running
        if self.processes.contains_key(service_name) {
            self.stop_service(service_name).await?;
        }

        // Start the service
        self.spawn_service(service_name, config).await?;

        Ok(())
    }

    /// Write data to a service's stdin
    pub async fn write_stdin(
        &mut self,
        service_name: &str,
        data: &str,
    ) -> Result<(), ProcessError> {
        let process = self
            .processes
            .get_mut(service_name)
            .ok_or_else(|| ProcessError::ServiceNotFound(service_name.to_string()))?;

        if let Some(stdin) = &mut process.stdin {
            use tokio::io::AsyncWriteExt;
            stdin.write_all(data.as_bytes()).await?;
            stdin.flush().await?;
            Ok(())
        } else {
            // Process might be running but stdin capture failed or wasn't set up
            Err(ProcessError::Io(std::io::Error::new(
                std::io::ErrorKind::Other,
                "Stdin not available for this service",
            )))
        }
    }

    /// Get the current status of a service
    pub fn get_status(&self, service_name: &str) -> Option<ServiceStatus> {
        self.processes.get(service_name).map(|p| p.status.clone())
    }

    /// Update the status of a service
    pub fn update_status(&mut self, service_name: &str, status: ServiceStatus) {
        if let Some(process) = self.processes.get_mut(service_name) {
            process.status = status;
        }
    }

    /// Get all service statuses
    pub fn get_all_statuses(&self) -> HashMap<String, ServiceStatus> {
        self.processes
            .iter()
            .map(|(name, process)| (name.clone(), process.status.clone()))
            .collect()
    }

    /// Check if a service is running
    pub fn is_running(&self, service_name: &str) -> bool {
        self.processes
            .get(service_name)
            .map(|p| matches!(p.status, ServiceStatus::Running))
            .unwrap_or(false)
    }

    /// Poll all managed services for natural process exits and update status.
    /// Returns exit events for services that transitioned from Starting/Running.
    pub fn poll_exited_processes(&mut self) -> Vec<ProcessExitEvent> {
        let mut events = Vec::new();

        for (service_name, process) in self.processes.iter_mut() {
            if !matches!(
                process.status,
                ServiceStatus::Starting | ServiceStatus::Running
            ) {
                continue;
            }

            match process.child.try_wait() {
                Ok(Some(exit_status)) => {
                    let (status, reason) = match exit_status.code() {
                        Some(0) => (ServiceStatus::Stopped, "Process exited cleanly".to_string()),
                        Some(code) => (
                            ServiceStatus::Failed { exit_code: code },
                            format!("Process exited with code {}", code),
                        ),
                        None => (
                            ServiceStatus::Failed { exit_code: -1 },
                            "Process terminated by signal".to_string(),
                        ),
                    };

                    process.status = status.clone();
                    process.stdin = None;
                    events.push(ProcessExitEvent {
                        service: service_name.clone(),
                        status,
                        reason,
                    });
                }
                Ok(None) => {
                    // Still running
                }
                Err(e) => {
                    let status = ServiceStatus::Failed { exit_code: -1 };
                    process.status = status.clone();
                    process.stdin = None;
                    events.push(ProcessExitEvent {
                        service: service_name.clone(),
                        status,
                        reason: format!("Failed to poll process state: {}", e),
                    });
                }
            }
        }

        events
    }

    /// Parse a command string into program and arguments
    /// Simple implementation that splits on whitespace
    /// TODO: Handle quoted arguments properly
    /// Parse a command string into program and arguments
    /// Handles quoted arguments (single and double quotes)
    fn parse_command(command: &str) -> Result<Vec<String>, ProcessError> {
        let mut args = Vec::new();
        let mut current_arg = String::new();
        let mut in_single_quote = false;
        let mut in_double_quote = false;
        let mut escaped = false;

        for c in command.chars() {
            if escaped {
                current_arg.push(c);
                escaped = false;
            } else if c == '\\' {
                if in_single_quote {
                    current_arg.push(c);
                } else {
                    escaped = true;
                }
            } else if c == '\'' {
                if in_double_quote {
                    current_arg.push(c);
                } else {
                    in_single_quote = !in_single_quote;
                }
            } else if c == '"' {
                if in_single_quote {
                    current_arg.push(c);
                } else {
                    in_double_quote = !in_double_quote;
                }
            } else if c.is_whitespace() {
                if in_single_quote || in_double_quote {
                    current_arg.push(c);
                } else if !current_arg.is_empty() {
                    args.push(current_arg);
                    current_arg = String::new();
                }
            } else {
                current_arg.push(c);
            }
        }

        // Push the last argument if exists
        if !current_arg.is_empty() {
            args.push(current_arg);
        }

        if args.is_empty() {
            return Err(ProcessError::CommandParse("Command is empty".to_string()));
        }

        // Check for unbalanced quotes
        if in_single_quote || in_double_quote {
            return Err(ProcessError::CommandParse(
                "Unbalanced quotes in command".to_string(),
            ));
        }

        Ok(args)
    }

    #[cfg(windows)]
    async fn kill_process_tree_windows(pid: u32, force: bool) -> Result<(), ProcessError> {
        let mut taskkill = Command::new("taskkill");
        taskkill.arg("/PID").arg(pid.to_string()).arg("/T");
        if force {
            taskkill.arg("/F");
        }

        let output = taskkill.output().await?;
        if output.status.success() {
            return Ok(());
        }

        // Treat already-stopped processes as success.
        let combined_output = format!(
            "{} {}",
            String::from_utf8_lossy(&output.stdout),
            String::from_utf8_lossy(&output.stderr)
        )
        .to_lowercase();
        if combined_output.contains("not found")
            || combined_output.contains("not running")
            || combined_output.contains("no running instance")
            || combined_output.contains("cannot find the process")
        {
            return Ok(());
        }

        Err(ProcessError::Io(std::io::Error::new(
            std::io::ErrorKind::Other,
            format!(
                "taskkill failed for pid {}: {}",
                pid,
                String::from_utf8_lossy(&output.stderr).trim()
            ),
        )))
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::config::ServiceConfig;

    fn create_test_log_buffer() -> Arc<Mutex<LogBuffer>> {
        Arc::new(Mutex::new(LogBuffer::new(1000)))
    }

    #[cfg(windows)]
    fn reserve_free_port() -> u16 {
        let listener = std::net::TcpListener::bind("127.0.0.1:0").unwrap();
        listener.local_addr().unwrap().port()
    }

    #[cfg(windows)]
    fn wait_for_port_open(port: u16, timeout: Duration) -> bool {
        let deadline = std::time::Instant::now() + timeout;
        while std::time::Instant::now() < deadline {
            if std::net::TcpStream::connect(("127.0.0.1", port)).is_ok() {
                return true;
            }
            std::thread::sleep(Duration::from_millis(100));
        }
        false
    }

    #[cfg(windows)]
    fn wait_for_port_free(port: u16, timeout: Duration) -> bool {
        let deadline = std::time::Instant::now() + timeout;
        while std::time::Instant::now() < deadline {
            if let Ok(listener) = std::net::TcpListener::bind(("127.0.0.1", port)) {
                drop(listener);
                return true;
            }
            std::thread::sleep(Duration::from_millis(100));
        }
        false
    }

    #[cfg(windows)]
    fn cleanup_process_using_port(port: u16) {
        let output = std::process::Command::new("netstat")
            .args(["-ano", "-p", "tcp"])
            .output();

        let output = match output {
            Ok(out) => out,
            Err(_) => return,
        };

        let port_suffix = format!(":{}", port);
        let stdout = String::from_utf8_lossy(&output.stdout);

        for line in stdout.lines() {
            let parts: Vec<&str> = line.split_whitespace().collect();
            if parts.len() < 5 {
                continue;
            }

            let local_addr = parts[1];
            let state = parts[3];
            let pid_str = parts[4];

            if !local_addr.ends_with(&port_suffix) || state != "LISTENING" {
                continue;
            }

            if let Ok(pid) = pid_str.parse::<u32>() {
                let _ = std::process::Command::new("taskkill")
                    .args(["/PID", &pid.to_string(), "/T", "/F"])
                    .output();
            }
        }
    }

    #[test]
    fn test_service_status_variants() {
        let status1 = ServiceStatus::NotStarted;
        let status2 = ServiceStatus::Starting;
        let status3 = ServiceStatus::Running;
        let status4 = ServiceStatus::Stopped;
        let status5 = ServiceStatus::Failed { exit_code: 1 };

        assert_eq!(status1, ServiceStatus::NotStarted);
        assert_eq!(status2, ServiceStatus::Starting);
        assert_eq!(status3, ServiceStatus::Running);
        assert_eq!(status4, ServiceStatus::Stopped);
        assert_eq!(status5, ServiceStatus::Failed { exit_code: 1 });
    }

    #[test]
    fn test_process_manager_new() {
        let log_buffer = create_test_log_buffer();
        let manager = ProcessManager::new(log_buffer);
        assert_eq!(manager.processes.len(), 0);
    }

    #[test]
    fn test_parse_command_simple() {
        let result = ProcessManager::parse_command("echo hello");
        assert!(result.is_ok());
        let parts = result.unwrap();
        assert_eq!(parts.len(), 2);
        assert_eq!(parts[0], "echo");
        assert_eq!(parts[1], "hello");
    }

    #[test]
    fn test_parse_command_multiple_args() {
        let result = ProcessManager::parse_command("cargo run --release --bin myapp");
        assert!(result.is_ok());
        let parts = result.unwrap();
        assert_eq!(parts.len(), 5);
        assert_eq!(parts[0], "cargo");
        assert_eq!(parts[1], "run");
        assert_eq!(parts[2], "--release");
        assert_eq!(parts[3], "--bin");
        assert_eq!(parts[4], "myapp");
    }

    #[test]
    fn test_parse_command_empty() {
        let result = ProcessManager::parse_command("");
        assert!(result.is_err());
    }

    #[test]
    fn test_parse_command_quoted() {
        let result = ProcessManager::parse_command("node -e \"console.log('hello world')\"");
        assert!(result.is_ok());
        let parts = result.unwrap();
        assert_eq!(parts.len(), 3);
        assert_eq!(parts[0], "node");
        assert_eq!(parts[1], "-e");
        assert_eq!(parts[2], "console.log('hello world')");
    }

    #[test]
    fn test_parse_command_mixed_quotes() {
        let result = ProcessManager::parse_command("echo 'hello \"world\"' \"foo 'bar'\"");
        assert!(result.is_ok());
        let parts = result.unwrap();
        assert_eq!(parts.len(), 3);
        assert_eq!(parts[0], "echo");
        assert_eq!(parts[1], "hello \"world\"");
        assert_eq!(parts[2], "foo 'bar'");
    }

    #[test]
    fn test_parse_command_whitespace_only() {
        let result = ProcessManager::parse_command("   ");
        assert!(result.is_err());
    }

    #[tokio::test]
    async fn test_spawn_service_simple_command() {
        let log_buffer = create_test_log_buffer();
        let mut manager = ProcessManager::new(log_buffer);

        #[cfg(unix)]
        let command = "echo test";
        #[cfg(windows)]
        let command = "cmd /c echo test";

        let config = ServiceConfig {
            command: command.to_string(),
            depends_on: vec![],
            ready_when: None,
            env_file: None,
        };

        let result = manager.spawn_service("test_service", &config).await;
        assert!(result.is_ok());

        // Check that process was added
        assert!(manager.processes.contains_key("test_service"));

        // Check initial status
        let status = manager.get_status("test_service");
        assert!(status.is_some());
        assert_eq!(status.unwrap(), ServiceStatus::Starting);
    }

    #[tokio::test]
    async fn test_spawn_service_already_running() {
        let log_buffer = create_test_log_buffer();
        let mut manager = ProcessManager::new(log_buffer);

        #[cfg(unix)]
        let command = "sleep 10";
        #[cfg(windows)]
        let command = "cmd /c timeout /t 10";

        let config = ServiceConfig {
            command: command.to_string(),
            depends_on: vec![],
            ready_when: None,
            env_file: None,
        };

        // Spawn first time
        let result1 = manager.spawn_service("test_service", &config).await;
        assert!(result1.is_ok());

        // Try to spawn again
        let result2 = manager.spawn_service("test_service", &config).await;
        assert!(result2.is_err());
        assert!(matches!(
            result2.unwrap_err(),
            ProcessError::AlreadyRunning(_)
        ));
    }

    #[tokio::test]
    async fn test_spawn_service_with_env_file() {
        use std::io::Write;

        let temp_dir = std::env::temp_dir();
        let env_path = temp_dir.join("test_spawn_env.env");

        // Create test env file
        let mut file = std::fs::File::create(&env_path).unwrap();
        file.write_all(b"TEST_VAR=test_value\n").unwrap();
        drop(file);

        let log_buffer = create_test_log_buffer();
        let mut manager = ProcessManager::new(log_buffer);

        #[cfg(unix)]
        let command = "echo test";
        #[cfg(windows)]
        let command = "cmd /c echo test";

        let config = ServiceConfig {
            command: command.to_string(),
            depends_on: vec![],
            ready_when: None,
            env_file: Some(env_path.to_str().unwrap().to_string()),
        };

        let result = manager.spawn_service("test_service", &config).await;
        assert!(result.is_ok());

        // Check that env vars were loaded
        let process = manager.processes.get("test_service").unwrap();
        assert_eq!(
            process.env_vars.get("TEST_VAR"),
            Some(&"test_value".to_string())
        );

        std::fs::remove_file(env_path).ok();
    }

    #[tokio::test]
    async fn test_get_status() {
        let log_buffer = create_test_log_buffer();
        let mut manager = ProcessManager::new(log_buffer);

        // Non-existent service
        assert!(manager.get_status("nonexistent").is_none());

        #[cfg(unix)]
        let command = "echo test";
        #[cfg(windows)]
        let command = "cmd /c echo test";

        // Spawn a service
        let config = ServiceConfig {
            command: command.to_string(),
            depends_on: vec![],
            ready_when: None,
            env_file: None,
        };

        manager
            .spawn_service("test_service", &config)
            .await
            .unwrap();

        let status = manager.get_status("test_service");
        assert!(status.is_some());
        assert_eq!(status.unwrap(), ServiceStatus::Starting);
    }

    #[tokio::test]
    async fn test_update_status() {
        let log_buffer = create_test_log_buffer();
        let mut manager = ProcessManager::new(log_buffer);

        #[cfg(unix)]
        let command = "echo test";
        #[cfg(windows)]
        let command = "cmd /c echo test";

        let config = ServiceConfig {
            command: command.to_string(),
            depends_on: vec![],
            ready_when: None,
            env_file: None,
        };

        manager
            .spawn_service("test_service", &config)
            .await
            .unwrap();

        // Update status
        manager.update_status("test_service", ServiceStatus::Running);

        let status = manager.get_status("test_service");
        assert_eq!(status.unwrap(), ServiceStatus::Running);
    }

    #[tokio::test]
    async fn test_get_all_statuses() {
        let log_buffer = create_test_log_buffer();
        let mut manager = ProcessManager::new(log_buffer);

        #[cfg(unix)]
        let command = "echo test";
        #[cfg(windows)]
        let command = "cmd /c echo test";

        let config = ServiceConfig {
            command: command.to_string(),
            depends_on: vec![],
            ready_when: None,
            env_file: None,
        };

        manager.spawn_service("service1", &config).await.unwrap();
        manager.spawn_service("service2", &config).await.unwrap();

        manager.update_status("service1", ServiceStatus::Running);
        manager.update_status("service2", ServiceStatus::Starting);

        let statuses = manager.get_all_statuses();
        assert_eq!(statuses.len(), 2);
        assert_eq!(statuses.get("service1"), Some(&ServiceStatus::Running));
        assert_eq!(statuses.get("service2"), Some(&ServiceStatus::Starting));
    }

    #[tokio::test]
    async fn test_is_running() {
        let log_buffer = create_test_log_buffer();
        let mut manager = ProcessManager::new(log_buffer);

        #[cfg(unix)]
        let command = "echo test";
        #[cfg(windows)]
        let command = "cmd /c echo test";

        let config = ServiceConfig {
            command: command.to_string(),
            depends_on: vec![],
            ready_when: None,
            env_file: None,
        };

        // Non-existent service
        assert!(!manager.is_running("nonexistent"));

        manager
            .spawn_service("test_service", &config)
            .await
            .unwrap();

        // Starting status - not running yet
        assert!(!manager.is_running("test_service"));

        // Update to running
        manager.update_status("test_service", ServiceStatus::Running);
        assert!(manager.is_running("test_service"));

        // Update to stopped
        manager.update_status("test_service", ServiceStatus::Stopped);
        assert!(!manager.is_running("test_service"));
    }

    #[tokio::test]
    async fn test_stop_service_not_found() {
        let log_buffer = create_test_log_buffer();
        let mut manager = ProcessManager::new(log_buffer);

        let result = manager.stop_service("nonexistent").await;
        assert!(result.is_err());
        assert!(matches!(
            result.unwrap_err(),
            ProcessError::ServiceNotFound(_)
        ));
    }

    #[tokio::test]
    async fn test_stop_service_already_stopped() {
        let log_buffer = create_test_log_buffer();
        let mut manager = ProcessManager::new(log_buffer);

        #[cfg(unix)]
        let command = "echo test";
        #[cfg(windows)]
        let command = "cmd /c echo test";

        let config = ServiceConfig {
            command: command.to_string(),
            depends_on: vec![],
            ready_when: None,
            env_file: None,
        };

        manager
            .spawn_service("test_service", &config)
            .await
            .unwrap();
        manager.update_status("test_service", ServiceStatus::Stopped);

        let result = manager.stop_service("test_service").await;
        // Should be idempotent and return Ok even if already stopped
        assert!(result.is_ok());
    }

    #[tokio::test]
    async fn test_stop_service_graceful() {
        let log_buffer = create_test_log_buffer();
        let mut manager = ProcessManager::new(log_buffer);

        // Use a command that will run for a bit
        #[cfg(unix)]
        let command = "sleep 1";
        #[cfg(windows)]
        let command = "timeout /t 1";

        let config = ServiceConfig {
            command: command.to_string(),
            depends_on: vec![],
            ready_when: None,
            env_file: None,
        };

        manager
            .spawn_service("test_service", &config)
            .await
            .unwrap();
        manager.update_status("test_service", ServiceStatus::Running);

        // Give it a moment to actually start
        tokio::time::sleep(Duration::from_millis(100)).await;

        let result = manager.stop_service("test_service").await;
        assert!(result.is_ok());

        // Check status was updated - should always be Stopped when stop_service is called
        // explicitly, regardless of exit code (killed processes have non-zero exit codes)
        let status = manager.get_status("test_service").unwrap();
        assert_eq!(status, ServiceStatus::Stopped);
    }

    #[cfg(windows)]
    #[tokio::test]
    async fn test_stop_service_kills_process_tree_and_releases_port() {
        let log_buffer = create_test_log_buffer();
        let mut manager = ProcessManager::new(log_buffer);

        let port = reserve_free_port();
        let script_path = std::env::temp_dir().join(format!("dmn_port_holder_{}.ps1", port));
        let script_content = format!(
            "$listener = [System.Net.Sockets.TcpListener]::new([System.Net.IPAddress]::Loopback, {})\n$listener.Start()\nwhile ($true) {{ Start-Sleep -Milliseconds 200 }}\n",
            port
        );
        std::fs::write(&script_path, script_content).unwrap();
        let script_path_arg = script_path.to_string_lossy().replace('\\', "/");

        // Intentionally run through cmd -> powershell to mirror common wrapper usage and
        // verify we terminate the full process tree, not just the direct child.
        let command = format!(
            "cmd /c powershell -NoProfile -ExecutionPolicy Bypass -File \"{}\"",
            script_path_arg
        );

        let config = ServiceConfig {
            command,
            depends_on: vec![],
            ready_when: None,
            env_file: None,
        };

        manager
            .spawn_service("tree_service", &config)
            .await
            .unwrap();
        manager.update_status("tree_service", ServiceStatus::Running);

        assert!(
            wait_for_port_open(port, Duration::from_secs(5)),
            "service did not open test port {} in time",
            port
        );

        let stop_result = manager.stop_service("tree_service").await;
        if stop_result.is_err() {
            cleanup_process_using_port(port);
            let _ = std::fs::remove_file(&script_path);
        }
        assert!(stop_result.is_ok(), "stop_service failed: {:?}", stop_result);

        let port_released = wait_for_port_free(port, Duration::from_secs(5));
        if !port_released {
            cleanup_process_using_port(port);
        }
        let _ = std::fs::remove_file(&script_path);

        assert!(
            port_released,
            "test port {} is still busy after stop_service",
            port
        );
    }

    #[tokio::test]
    async fn test_restart_service() {
        let log_buffer = create_test_log_buffer();
        let mut manager = ProcessManager::new(log_buffer);

        #[cfg(unix)]
        let command = "echo test";
        #[cfg(windows)]
        let command = "cmd /c echo test";

        let config = ServiceConfig {
            command: command.to_string(),
            depends_on: vec![],
            ready_when: None,
            env_file: None,
        };

        // Initial spawn
        manager
            .spawn_service("test_service", &config)
            .await
            .unwrap();
        manager.update_status("test_service", ServiceStatus::Running);

        // Restart
        let result = manager.restart_service("test_service", &config).await;
        assert!(result.is_ok());

        // Should be in Starting status again
        let status = manager.get_status("test_service").unwrap();
        assert_eq!(status, ServiceStatus::Starting);
    }

    #[tokio::test]
    async fn test_restart_service_not_running() {
        let log_buffer = create_test_log_buffer();
        let mut manager = ProcessManager::new(log_buffer);

        #[cfg(unix)]
        let command = "echo test";
        #[cfg(windows)]
        let command = "cmd /c echo test";

        let config = ServiceConfig {
            command: command.to_string(),
            depends_on: vec![],
            ready_when: None,
            env_file: None,
        };

        // Restart without spawning first
        let result = manager.restart_service("test_service", &config).await;
        assert!(result.is_ok());

        // Should be spawned now
        assert!(manager.processes.contains_key("test_service"));
    }

    #[tokio::test]
    async fn test_log_capture_stdout() {
        let log_buffer = create_test_log_buffer();
        let mut manager = ProcessManager::new(Arc::clone(&log_buffer));

        #[cfg(unix)]
        let command = "echo hello_stdout";
        #[cfg(windows)]
        let command = "cmd /c echo hello_stdout";

        let config = ServiceConfig {
            command: command.to_string(),
            depends_on: vec![],
            ready_when: None,
            env_file: None,
        };

        manager
            .spawn_service("test_service", &config)
            .await
            .unwrap();

        // Wait for process to complete and logs to be captured
        tokio::time::sleep(Duration::from_millis(500)).await;

        let buffer = log_buffer.lock().await;
        let logs = buffer.get_all_lines("test_service");

        // Should have captured the output
        assert!(!logs.is_empty());
        assert!(logs.iter().any(|l| l.content.contains("hello_stdout")));
    }

    #[tokio::test]
    async fn test_log_capture_stderr() {
        let log_buffer = create_test_log_buffer();
        let mut manager = ProcessManager::new(Arc::clone(&log_buffer));

        // Command that writes to stderr
        #[cfg(unix)]
        let command = "sh -c 'echo hello_stderr >&2'";
        #[cfg(windows)]
        let command = "cmd /c echo hello_stderr 1>&2";

        let config = ServiceConfig {
            command: command.to_string(),
            depends_on: vec![],
            ready_when: None,
            env_file: None,
        };

        manager
            .spawn_service("test_service", &config)
            .await
            .unwrap();

        // Wait for process to complete and logs to be captured
        tokio::time::sleep(Duration::from_millis(500)).await;

        let buffer = log_buffer.lock().await;
        let logs = buffer.get_all_lines("test_service");

        // Should have captured the stderr output
        assert!(!logs.is_empty());

        // Check that at least one log line is from stderr
        let has_stderr = logs.iter().any(|l| matches!(l.stream, LogStream::Stderr));
        assert!(has_stderr);
    }

    #[tokio::test]
    async fn test_multiple_services() {
        let log_buffer = create_test_log_buffer();
        let mut manager = ProcessManager::new(log_buffer);

        #[cfg(unix)]
        let command1 = "echo service1";
        #[cfg(windows)]
        let command1 = "cmd /c echo service1";

        #[cfg(unix)]
        let command2 = "echo service2";
        #[cfg(windows)]
        let command2 = "cmd /c echo service2";

        let config1 = ServiceConfig {
            command: command1.to_string(),
            depends_on: vec![],
            ready_when: None,
            env_file: None,
        };

        let config2 = ServiceConfig {
            command: command2.to_string(),
            depends_on: vec![],
            ready_when: None,
            env_file: None,
        };

        manager.spawn_service("service1", &config1).await.unwrap();
        manager.spawn_service("service2", &config2).await.unwrap();

        assert_eq!(manager.processes.len(), 2);
        assert!(manager.processes.contains_key("service1"));
        assert!(manager.processes.contains_key("service2"));
    }

    #[tokio::test]
    async fn test_process_exit_handling() {
        let log_buffer = create_test_log_buffer();
        let mut manager = ProcessManager::new(log_buffer);

        #[cfg(unix)]
        let command = "echo quick_exit";
        #[cfg(windows)]
        let command = "cmd /c echo quick_exit";

        // Command that exits immediately
        let config = ServiceConfig {
            command: command.to_string(),
            depends_on: vec![],
            ready_when: None,
            env_file: None,
        };

        manager
            .spawn_service("test_service", &config)
            .await
            .unwrap();

        // Wait for process to exit
        tokio::time::sleep(Duration::from_millis(500)).await;

        // Process should still be in the map (we don't auto-remove)
        assert!(manager.processes.contains_key("test_service"));
    }

    #[tokio::test]
    async fn test_poll_exited_processes_updates_status() {
        let log_buffer = create_test_log_buffer();
        let mut manager = ProcessManager::new(log_buffer);

        #[cfg(unix)]
        let command = "echo quick_exit";
        #[cfg(windows)]
        let command = "cmd /c echo quick_exit";

        let config = ServiceConfig {
            command: command.to_string(),
            depends_on: vec![],
            ready_when: None,
            env_file: None,
        };

        manager
            .spawn_service("test_service", &config)
            .await
            .unwrap();
        manager.update_status("test_service", ServiceStatus::Running);

        // Give process time to exit naturally.
        tokio::time::sleep(Duration::from_millis(300)).await;

        let events = manager.poll_exited_processes();
        assert!(!events.is_empty());
        assert_eq!(events[0].service, "test_service");
        assert_eq!(events[0].status, ServiceStatus::Stopped);
        assert!(events[0].reason.contains("cleanly"));
    }

    #[tokio::test]
    async fn test_invalid_command() {
        let log_buffer = create_test_log_buffer();
        let mut manager = ProcessManager::new(log_buffer);

        let config = ServiceConfig {
            command: "this_command_does_not_exist_12345".to_string(),
            depends_on: vec![],
            ready_when: None,
            env_file: None,
        };

        let result = manager.spawn_service("test_service", &config).await;
        assert!(result.is_err());
    }
}
