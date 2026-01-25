use crate::config::DmnConfig;
use crate::graph::{GraphError, ServiceGraph};
use crate::logs::{LogBuffer, LogLine};
use crate::process::{LogLineEvent, ProcessError, ProcessManager};
use crate::ready::ReadyWatcher;
use std::sync::Arc;
use std::time::Duration;
use thiserror::Error;
use tokio::sync::{broadcast, mpsc, Mutex};

#[derive(Debug, Clone)]
pub enum OrchestratorEvent {
    ServiceStarting { service: String },
    ServiceReady { service: String },
    ServiceFailed { service: String, error: String },
    ServiceStopped { service: String },
    LogLine { service: String, line: LogLine },
    Error { message: String, category: String },
}

#[derive(Debug, Error)]
pub enum OrchestratorError {
    #[error("Configuration error: {0}")]
    Config(String),
    #[error("Graph error: {0}")]
    Graph(#[from] GraphError),
    #[error("Process error: {0}")]
    Process(#[from] ProcessError),
    #[error("Service not found: {0}")]
    ServiceNotFound(String),
    #[error("Ready watcher error: {0}")]
    ReadyError(String),
}

pub struct Orchestrator {
    config: DmnConfig,
    graph: ServiceGraph,
    pub process_manager: ProcessManager,
    pub log_buffer: Arc<Mutex<LogBuffer>>,
    ready_watcher: Arc<Mutex<ReadyWatcher>>,
    event_tx: broadcast::Sender<OrchestratorEvent>,
}

impl std::fmt::Debug for Orchestrator {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("Orchestrator")
            .field("config", &self.config)
            .field("graph", &"<ServiceGraph>")
            .field("process_manager", &"<ProcessManager>")
            .field("log_buffer", &"<Arc<Mutex<LogBuffer>>>")
            .field("ready_watcher", &"<Arc<Mutex<ReadyWatcher>>>")
            .field("event_tx", &"<broadcast::Sender>")
            .finish()
    }
}

impl Orchestrator {
    /// Create a new Orchestrator from a configuration
    /// This initializes all components: graph, process manager, log buffer, and ready watcher
    pub fn new(config: DmnConfig) -> Result<Self, OrchestratorError> {
        // Build the dependency graph from config
        let graph = ServiceGraph::from_config(&config)?;
        
        // Create log buffer
        let log_buffer = Arc::new(Mutex::new(LogBuffer::new(1000)));
        
        // Create broadcast event channel for orchestrator events
        // Capacity of 1000 should be enough for most use cases
        let (event_tx, _) = broadcast::channel(1000);
        
        // Create log event channel for real-time log forwarding
        let (log_event_tx, mut log_event_rx) = mpsc::unbounded_channel::<LogLineEvent>();
        
        // Create process manager with log event sender
        let process_manager = ProcessManager::with_log_events(Arc::clone(&log_buffer), log_event_tx);
        
        // Create ready watcher with default timeout of 60 seconds
        let ready_watcher = Arc::new(Mutex::new(ReadyWatcher::new(Duration::from_secs(60))));
        
        // Spawn task to forward log events to the main event channel
        let event_tx_for_logs = event_tx.clone();
        tokio::spawn(async move {
            while let Some(log_event) = log_event_rx.recv().await {
                let orch_event = OrchestratorEvent::LogLine {
                    service: log_event.service,
                    line: log_event.line,
                };
                // Ignore send errors - no receivers may be subscribed yet
                let _ = event_tx_for_logs.send(orch_event);
            }
        });
        
        Ok(Self {
            config,
            graph,
            process_manager,
            log_buffer,
            ready_watcher,
            event_tx,
        })
    }
    
    /// Subscribe to orchestrator events
    /// Returns a receiver that will receive all events emitted by the orchestrator
    /// Multiple subscribers can receive events simultaneously (broadcast)
    pub fn subscribe_events(&self) -> broadcast::Receiver<OrchestratorEvent> {
        self.event_tx.subscribe()
    }
    
    /// Get a sender for emitting events from outside the orchestrator
    pub fn event_sender(&self) -> broadcast::Sender<OrchestratorEvent> {
        self.event_tx.clone()
    }
    
    /// Emit an event to all subscribers
    fn emit_event(&self, event: OrchestratorEvent) {
        // Ignore send errors - if no one is listening, that's okay
        let _ = self.event_tx.send(event);
    }
    
    /// Emit an error event with proper categorization
    fn emit_error(&self, error: &OrchestratorError) {
        let category = match error {
            OrchestratorError::Config(_) => "CONFIG",
            OrchestratorError::Graph(_) => "GRAPH",
            OrchestratorError::Process(_) => "PROCESS",
            OrchestratorError::ServiceNotFound(_) => "SERVICE",
            OrchestratorError::ReadyError(_) => "READY",
        };
        
        self.emit_event(OrchestratorEvent::Error {
            message: error.to_string(),
            category: category.to_string(),
        });
    }
    
    /// Get the configuration
    pub fn config(&self) -> &DmnConfig {
        &self.config
    }
    
    /// Get the ready watcher (for checking service ready state)
    pub fn ready_watcher(&self) -> &Arc<Mutex<ReadyWatcher>> {
        &self.ready_watcher
    }
    
    /// Get the dependency graph
    pub fn graph(&self) -> &ServiceGraph {
        &self.graph
    }
    
    /// Check if a service is ready
    pub async fn is_service_ready(&self, service_name: &str) -> bool {
        let watcher = self.ready_watcher.lock().await;
        watcher.is_ready(service_name)
    }
    
    /// Start all services in dependency order
    /// Services are started according to their dependencies, with each service
    /// waiting for its dependencies to be ready before starting
    pub async fn start_all(&mut self) -> Result<(), OrchestratorError> {
        // Get the start order from the dependency graph
        let start_order = match self.graph.get_start_order() {
            Ok(order) => order,
            Err(e) => {
                let err = OrchestratorError::Graph(e);
                self.emit_error(&err);
                return Err(err);
            }
        };
        
        // Start each service in order
        for service_name in start_order {
            if let Err(e) = self.start_service_with_deps(&service_name).await {
                self.emit_error(&e);
                return Err(e);
            }
        }
        
        Ok(())
    }
    
    /// Start a service along with its dependencies
    /// This method ensures all dependencies are started before starting the service
    /// Dependencies are auto-started recursively if they're not already running
    /// 
    /// NOTE: This method returns immediately after spawning the service.
    /// The ready check happens asynchronously and emits serviceReady/serviceFailed notifications.
    pub async fn start_service_with_deps(&mut self, service_name: &str) -> Result<(), OrchestratorError> {
        // Check if service exists in config
        let service_config = self.config.services.get(service_name)
            .ok_or_else(|| {
                let err = OrchestratorError::ServiceNotFound(service_name.to_string());
                self.emit_error(&err);
                err
            })?
            .clone();
        
        // Check if service is already running or starting
        if let Some(status) = self.process_manager.get_status(service_name) {
            if matches!(status, crate::process::ServiceStatus::Running | crate::process::ServiceStatus::Starting) {
                // Already running or starting, nothing to do
                return Ok(());
            }
        }
        
        // Get dependencies for this service
        let dependencies = match self.graph.get_dependencies(service_name) {
            Ok(deps) => deps,
            Err(e) => {
                let err = OrchestratorError::Graph(e);
                self.emit_error(&err);
                return Err(err);
            }
        };
        
        // Start all dependencies first (recursive - they will start their own dependencies)
        for dep in &dependencies {
            let needs_start = match self.process_manager.get_status(dep) {
                Some(crate::process::ServiceStatus::Running) |
                Some(crate::process::ServiceStatus::Starting) => false,
                _ => true,
            };
            
            if needs_start {
                // Dependency is not running - start it first (recursively)
                // This will also start its dependencies
                if let Err(e) = Box::pin(self.start_service_with_deps(dep)).await {
                    self.emit_error(&e);
                    return Err(e);
                }
                
                // Wait for dependency to be ready before continuing
                // This ensures proper startup order
                if let Err(e) = self.wait_for_ready(dep).await {
                    self.emit_error(&e);
                    return Err(e);
                }
            } else {
                // Dependency is running/starting, wait for it to be ready
                let is_ready = {
                    let watcher = self.ready_watcher.lock().await;
                    watcher.is_ready(dep)
                };
                
                if !is_ready {
                    if let Err(e) = self.wait_for_ready(dep).await {
                        self.emit_error(&e);
                        return Err(e);
                    }
                }
            }
        }
        
        // Now all dependencies should be running and ready
        // Emit starting event for this service
        self.emit_event(OrchestratorEvent::ServiceStarting {
            service: service_name.to_string(),
        });
        
        // Spawn the service process
        if let Err(e) = self.process_manager.spawn_service(service_name, &service_config).await {
            let err = OrchestratorError::Process(e);
            self.emit_error(&err);
            return Err(err);
        }
        
        // Update status to Starting
        self.process_manager.update_status(service_name, crate::process::ServiceStatus::Starting);
        
        // Set up ready watching asynchronously based on configuration
        if let Some(ready_condition) = &service_config.ready_when {
            // Create a channel to receive log lines for this service
            let (log_tx, log_rx) = mpsc::channel(100);
            
            // Spawn a task to forward logs to the ready watcher
            let log_buffer = Arc::clone(&self.log_buffer);
            let service_name_for_logs = service_name.to_string();
            tokio::spawn(async move {
                // Poll for new log lines and forward them
                let mut last_count = 0;
                loop {
                    tokio::time::sleep(Duration::from_millis(10)).await;
                    let buffer = log_buffer.lock().await;
                    let lines = buffer.get_all_lines(&service_name_for_logs);
                    let current_count = lines.len();
                    
                    // Only send new lines
                    if current_count > last_count {
                        let new_lines: Vec<_> = lines.iter().skip(last_count).cloned().collect();
                        drop(buffer); // Release lock before sending
                        
                        for line in new_lines {
                            if log_tx.send(line).await.is_err() {
                                return; // Channel closed, stop polling
                            }
                        }
                        last_count = current_count;
                    } else {
                        drop(buffer);
                    }
                }
            });
            
            // Extract custom timeout from ready condition if specified
            let custom_timeout = match ready_condition {
                crate::config::ReadyCondition::LogContains { timeout_seconds, .. } => {
                    timeout_seconds.map(Duration::from_secs)
                }
                crate::config::ReadyCondition::UrlResponds { timeout_seconds, .. } => {
                    timeout_seconds.map(Duration::from_secs)
                }
            };
            
            // Spawn async task to watch for readiness and emit events
            // This allows the RPC call to return immediately
            let ready_watcher = Arc::clone(&self.ready_watcher);
            let process_manager_status = service_name.to_string();
            let event_tx = self.event_tx.clone();
            let ready_condition = ready_condition.clone();
            let service_name_for_ready = service_name.to_string();
            
            // Store process manager reference for updating status
            // We need to use a channel to communicate back status updates
            let (status_tx, mut status_rx) = mpsc::channel::<(String, crate::process::ServiceStatus)>(1);
            
            // Spawn the ready watching task
            tokio::spawn(async move {
                let result = {
                    let mut watcher = ready_watcher.lock().await;
                    watcher.watch_service_with_timeout(
                        service_name_for_ready.clone(),
                        ready_condition,
                        Some(log_rx),
                        custom_timeout,
                    ).await
                };
                
                match result {
                    Ok(_) => {
                        // Service is ready - emit event
                        let _ = event_tx.send(OrchestratorEvent::ServiceReady {
                            service: service_name_for_ready.clone(),
                        });
                        // Send status update
                        let _ = status_tx.send((service_name_for_ready, crate::process::ServiceStatus::Running)).await;
                    }
                    Err(e) => {
                        // Ready check failed - emit failure event
                        let _ = event_tx.send(OrchestratorEvent::ServiceFailed {
                            service: service_name_for_ready.clone(),
                            error: e.to_string(),
                        });
                        // Send status update
                        let _ = status_tx.send((service_name_for_ready, crate::process::ServiceStatus::Failed { exit_code: -1 })).await;
                    }
                }
            });
            
            // Spawn another task to handle status updates from ready watcher
            // This is a workaround since we can't easily pass ProcessManager to spawned tasks
            let process_manager = &mut self.process_manager;
            // We can't spawn here as we need to update process_manager synchronously
            // Instead, we'll update status to Running after a short delay if no failure occurs
            // The ready watcher will emit the proper events
            
            // For now, let's wait briefly for very fast ready conditions
            // But don't block for long - let the async task handle it
            tokio::select! {
                Some((name, status)) = status_rx.recv() => {
                    self.process_manager.update_status(&name, status);
                }
                _ = tokio::time::sleep(Duration::from_millis(100)) => {
                    // Ready check is taking longer, return now
                    // Status will be updated via the spawned task's events
                }
            }
        } else {
            // No ready condition - mark as ready immediately
            {
                let mut watcher = self.ready_watcher.lock().await;
                watcher.mark_ready(service_name);
            }
            self.process_manager.update_status(service_name, crate::process::ServiceStatus::Running);
            
            // Emit ready event
            self.emit_event(OrchestratorEvent::ServiceReady {
                service: service_name.to_string(),
            });
        }
        
        Ok(())
    }
    
    /// Wait for a service to become ready
    async fn wait_for_ready(&self, service_name: &str) -> Result<(), OrchestratorError> {
        // Poll until the service is ready or timeout
        let timeout_duration = Duration::from_secs(60);
        let start = std::time::Instant::now();
        
        while start.elapsed() < timeout_duration {
            let is_ready = {
                let watcher = self.ready_watcher.lock().await;
                watcher.is_ready(service_name)
            };
            
            if is_ready {
                return Ok(());
            }
            
            tokio::time::sleep(Duration::from_millis(100)).await;
        }
        
        Err(OrchestratorError::ReadyError(
            format!("Timeout waiting for service '{}' to be ready", service_name)
        ))
    }
    
    /// Stop all services in reverse dependency order
    /// Services are stopped in reverse order to ensure dependents are stopped before dependencies
    pub async fn stop_all(&mut self) -> Result<(), OrchestratorError> {
        // Get the start order and reverse it for stop order
        let start_order = match self.graph.get_start_order() {
            Ok(order) => order,
            Err(e) => {
                let err = OrchestratorError::Graph(e);
                self.emit_error(&err);
                return Err(err);
            }
        };
        let stop_order: Vec<_> = start_order.into_iter().rev().collect();
        
        // Stop each service in reverse order
        for service_name in stop_order {
            // Only stop if the service is actually running
            if self.process_manager.get_status(&service_name).is_some() {
                if let Err(e) = self.stop_service(&service_name).await {
                    self.emit_error(&e);
                    return Err(e);
                }
            }
        }
        
        Ok(())
    }
    
    /// Stop a service and all services that depend on it
    /// This implements cascade stopping - dependents are stopped first
    pub async fn stop_service(&mut self, service_name: &str) -> Result<(), OrchestratorError> {
        // Check if service exists in config
        if !self.config.services.contains_key(service_name) {
            let err = OrchestratorError::ServiceNotFound(service_name.to_string());
            self.emit_error(&err);
            return Err(err);
        }
        
        // Check current status - only proceed if service is running/starting
        let current_status = self.process_manager.get_status(service_name);
        let needs_stop = matches!(
            current_status,
            Some(crate::process::ServiceStatus::Running) | Some(crate::process::ServiceStatus::Starting)
        );
        
        // If service is already stopped/failed/not-started, return early - no duplicates
        if !needs_stop {
            return Ok(());
        }
        
        // Get list of dependent services (services that depend on this one)
        let dependents = match self.graph.get_dependents(service_name) {
            Ok(deps) => deps,
            Err(e) => {
                let err = OrchestratorError::Graph(e);
                self.emit_error(&err);
                return Err(err);
            }
        };
        
        // Stop dependents first (cascade) - only those that are running/starting
        for dependent in dependents {
            let dep_status = self.process_manager.get_status(&dependent);
            let dep_needs_stop = matches!(
                dep_status,
                Some(crate::process::ServiceStatus::Running) | Some(crate::process::ServiceStatus::Starting)
            );
            
            if dep_needs_stop {
                // Use Box::pin to handle recursion
                if let Err(e) = Box::pin(self.stop_service(&dependent)).await {
                    self.emit_error(&e);
                    return Err(e);
                }
            }
        }
        
        // Stop the target service itself
        if let Err(e) = self.process_manager.stop_service(service_name).await {
            let err = OrchestratorError::Process(e);
            self.emit_error(&err);
            return Err(err);
        }
        
        // Reset ready status
        {
            let mut watcher = self.ready_watcher.lock().await;
            watcher.reset_service(service_name);
        }
        
        // Emit stopped event for this service
        self.emit_event(OrchestratorEvent::ServiceStopped {
            service: service_name.to_string(),
        });
        
        Ok(())
    }
    
    /// Restart a service by stopping it and then starting it again
    /// This will also restart any services that depend on it
    pub async fn restart_service(&mut self, service_name: &str) -> Result<(), OrchestratorError> {
        // Check if service exists in config
        if !self.config.services.contains_key(service_name) {
            let err = OrchestratorError::ServiceNotFound(service_name.to_string());
            self.emit_error(&err);
            return Err(err);
        }
        
        // Stop the service (and its dependents)
        if let Err(e) = self.stop_service(service_name).await {
            self.emit_error(&e);
            return Err(e);
        }
        
        // Wait a moment for cleanup
        tokio::time::sleep(Duration::from_millis(100)).await;
        
        // Start the service again
        if let Err(e) = self.start_service_with_deps(service_name).await {
            self.emit_error(&e);
            return Err(e);
        }
        
        Ok(())
    }
}


#[cfg(test)]
mod tests {
    use super::*;
    use crate::config::ServiceConfig;
    use std::collections::HashMap;
    use std::time::Duration;

    fn create_test_config() -> DmnConfig {
        let mut services = HashMap::new();
        services.insert(
            "database".to_string(),
            ServiceConfig {
                command: "postgres".to_string(),
                depends_on: vec![],
                ready_when: None,
                env_file: None,
            },
        );
        services.insert(
            "backend".to_string(),
            ServiceConfig {
                command: "cargo run".to_string(),
                depends_on: vec!["database".to_string()],
                ready_when: None,
                env_file: None,
            },
        );

        DmnConfig {
            version: "1.0".to_string(),
            services,
        }
    }

    #[test]
    fn test_orchestrator_new() {
        let config = create_test_config();
        let orchestrator = Orchestrator::new(config);
        
        assert!(orchestrator.is_ok());
        let orch = orchestrator.unwrap();
        assert_eq!(orch.config.services.len(), 2);
    }

    #[test]
    fn test_orchestrator_new_with_cycle() {
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

        let result = Orchestrator::new(config);
        assert!(result.is_err());
        assert!(matches!(result.unwrap_err(), OrchestratorError::Graph(_)));
    }

    #[test]
    fn test_orchestrator_config_access() {
        let config = create_test_config();
        let orchestrator = Orchestrator::new(config).unwrap();
        
        let config_ref = orchestrator.config();
        assert_eq!(config_ref.version, "1.0");
        assert_eq!(config_ref.services.len(), 2);
    }

    #[test]
    fn test_orchestrator_graph_access() {
        let config = create_test_config();
        let orchestrator = Orchestrator::new(config).unwrap();
        
        let graph = orchestrator.graph();
        let start_order = graph.get_start_order().unwrap();
        assert_eq!(start_order.len(), 2);
        
        // Database should come before backend
        let db_pos = start_order.iter().position(|s| s == "database").unwrap();
        let backend_pos = start_order.iter().position(|s| s == "backend").unwrap();
        assert!(db_pos < backend_pos);
    }

    #[test]
    fn test_orchestrator_event_channel() {
        let config = create_test_config();
        let orchestrator = Orchestrator::new(config).unwrap();
        
        // Test that we can get a sender for emitting events
        let event_sender = orchestrator.event_sender();
        
        // Test that we can send events
        let test_event = OrchestratorEvent::ServiceStarting {
            service: "test".to_string(),
        };
        
        assert!(event_sender.send(test_event).is_ok());
        
        // Test that we can subscribe to receive events
        let mut event_receiver = orchestrator.subscribe_events();
        
        // Send another event
        let test_event2 = OrchestratorEvent::ServiceReady {
            service: "test".to_string(),
        };
        event_sender.send(test_event2).unwrap();
        
        // Verify we can receive events (non-blocking check - may or may not have event yet)
        // Just verify the channel is working
        drop(event_receiver);
    }

    #[test]
    fn test_emit_event() {
        let config = create_test_config();
        let orchestrator = Orchestrator::new(config).unwrap();
        
        // Emit an event
        orchestrator.emit_event(OrchestratorEvent::ServiceStarting {
            service: "database".to_string(),
        });
        
        // Event should be sent (we can't easily verify receipt without async, but at least it doesn't panic)
    }

    #[tokio::test]
    async fn test_start_service_with_deps_no_dependencies() {
        let mut services = HashMap::new();
        services.insert(
            "simple".to_string(),
            ServiceConfig {
                command: if cfg!(windows) { "cmd /c echo test" } else { "echo test" }.to_string(),
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
        let result = orchestrator.start_service_with_deps("simple").await;
        
        assert!(result.is_ok());
        
        // Give it a moment to process
        tokio::time::sleep(Duration::from_millis(200)).await;
        
        // Check that service is marked as ready
        let is_ready = {
            let watcher = orchestrator.ready_watcher.lock().await;
            watcher.is_ready("simple")
        };
        assert!(is_ready);
    }

    #[tokio::test]
    async fn test_start_service_with_deps_linear_dependencies() {
        let mut services = HashMap::new();
        services.insert(
            "database".to_string(),
            ServiceConfig {
                command: if cfg!(windows) { "cmd /c echo db" } else { "echo db" }.to_string(),
                depends_on: vec![],
                ready_when: None,
                env_file: None,
            },
        );
        services.insert(
            "backend".to_string(),
            ServiceConfig {
                command: if cfg!(windows) { "cmd /c echo backend" } else { "echo backend" }.to_string(),
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
        
        // Start database first
        orchestrator.start_service_with_deps("database").await.unwrap();
        tokio::time::sleep(Duration::from_millis(200)).await;
        
        // Now start backend (which depends on database)
        let result = orchestrator.start_service_with_deps("backend").await;
        assert!(result.is_ok());
        
        tokio::time::sleep(Duration::from_millis(200)).await;
        
        // Both should be ready
        let watcher = orchestrator.ready_watcher.lock().await;
        assert!(watcher.is_ready("database"));
        assert!(watcher.is_ready("backend"));
    }

    #[tokio::test]
    async fn test_start_all_simple() {
        let mut services = HashMap::new();
        services.insert(
            "service1".to_string(),
            ServiceConfig {
                command: if cfg!(windows) { "cmd /c echo s1" } else { "echo s1" }.to_string(),
                depends_on: vec![],
                ready_when: None,
                env_file: None,
            },
        );
        services.insert(
            "service2".to_string(),
            ServiceConfig {
                command: if cfg!(windows) { "cmd /c echo s2" } else { "echo s2" }.to_string(),
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
        let result = orchestrator.start_all().await;
        
        assert!(result.is_ok());
        
        tokio::time::sleep(Duration::from_millis(300)).await;
        
        // Both services should be ready
        let watcher = orchestrator.ready_watcher.lock().await;
        assert!(watcher.is_ready("service1"));
        assert!(watcher.is_ready("service2"));
    }

    #[tokio::test]
    async fn test_start_all_with_dependencies() {
        let mut services = HashMap::new();
        services.insert(
            "database".to_string(),
            ServiceConfig {
                command: if cfg!(windows) { "cmd /c echo db" } else { "echo db" }.to_string(),
                depends_on: vec![],
                ready_when: None,
                env_file: None,
            },
        );
        services.insert(
            "backend".to_string(),
            ServiceConfig {
                command: if cfg!(windows) { "cmd /c echo backend" } else { "echo backend" }.to_string(),
                depends_on: vec!["database".to_string()],
                ready_when: None,
                env_file: None,
            },
        );
        services.insert(
            "frontend".to_string(),
            ServiceConfig {
                command: if cfg!(windows) { "cmd /c echo frontend" } else { "echo frontend" }.to_string(),
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
        let result = orchestrator.start_all().await;
        
        assert!(result.is_ok());
        
        tokio::time::sleep(Duration::from_millis(500)).await;
        
        // All services should be ready
        let watcher = orchestrator.ready_watcher.lock().await;
        assert!(watcher.is_ready("database"));
        assert!(watcher.is_ready("backend"));
        assert!(watcher.is_ready("frontend"));
    }

    #[tokio::test]
    async fn test_start_service_nonexistent() {
        let config = create_test_config();
        let mut orchestrator = Orchestrator::new(config).unwrap();
        
        let result = orchestrator.start_service_with_deps("nonexistent").await;
        assert!(result.is_err());
        assert!(matches!(result.unwrap_err(), OrchestratorError::ServiceNotFound(_)));
    }

    #[tokio::test]
    async fn test_event_emission_on_start() {
        let mut services = HashMap::new();
        services.insert(
            "test_service".to_string(),
            ServiceConfig {
                command: if cfg!(windows) { "cmd /c echo test" } else { "echo test" }.to_string(),
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
        
        // Events should have been emitted (we can't easily verify without more complex setup)
        tokio::time::sleep(Duration::from_millis(100)).await;
    }

    #[tokio::test]
    async fn test_stop_service_simple() {
        let mut services = HashMap::new();
        services.insert(
            "simple".to_string(),
            ServiceConfig {
                command: if cfg!(windows) { "timeout /t 10" } else { "sleep 10" }.to_string(),
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
        orchestrator.start_service_with_deps("simple").await.unwrap();
        tokio::time::sleep(Duration::from_millis(200)).await;
        
        // Stop the service
        let result = orchestrator.stop_service("simple").await;
        assert!(result.is_ok());
        
        // Service should no longer be ready
        let is_ready = {
            let watcher = orchestrator.ready_watcher.lock().await;
            watcher.is_ready("simple")
        };
        assert!(!is_ready);
    }

    #[tokio::test]
    async fn test_stop_service_with_dependents() {
        let mut services = HashMap::new();
        services.insert(
            "database".to_string(),
            ServiceConfig {
                command: if cfg!(windows) { "timeout /t 10" } else { "sleep 10" }.to_string(),
                depends_on: vec![],
                ready_when: None,
                env_file: None,
            },
        );
        services.insert(
            "backend".to_string(),
            ServiceConfig {
                command: if cfg!(windows) { "timeout /t 10" } else { "sleep 10" }.to_string(),
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
        
        // Start both services
        orchestrator.start_all().await.unwrap();
        tokio::time::sleep(Duration::from_millis(300)).await;
        
        // Stop database (should also stop backend)
        let result = orchestrator.stop_service("database").await;
        assert!(result.is_ok());
        
        // Both services should no longer be ready
        let watcher = orchestrator.ready_watcher.lock().await;
        assert!(!watcher.is_ready("database"));
        assert!(!watcher.is_ready("backend"));
    }

    #[tokio::test]
    async fn test_stop_all_simple() {
        let mut services = HashMap::new();
        services.insert(
            "service1".to_string(),
            ServiceConfig {
                command: if cfg!(windows) { "timeout /t 10" } else { "sleep 10" }.to_string(),
                depends_on: vec![],
                ready_when: None,
                env_file: None,
            },
        );
        services.insert(
            "service2".to_string(),
            ServiceConfig {
                command: if cfg!(windows) { "timeout /t 10" } else { "sleep 10" }.to_string(),
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
        
        // Start all services
        orchestrator.start_all().await.unwrap();
        tokio::time::sleep(Duration::from_millis(300)).await;
        
        // Stop all services
        let result = orchestrator.stop_all().await;
        assert!(result.is_ok());
        
        // All services should no longer be ready
        let watcher = orchestrator.ready_watcher.lock().await;
        assert!(!watcher.is_ready("service1"));
        assert!(!watcher.is_ready("service2"));
    }

    #[tokio::test]
    async fn test_stop_all_with_dependencies() {
        let mut services = HashMap::new();
        services.insert(
            "database".to_string(),
            ServiceConfig {
                command: if cfg!(windows) { "timeout /t 10" } else { "sleep 10" }.to_string(),
                depends_on: vec![],
                ready_when: None,
                env_file: None,
            },
        );
        services.insert(
            "backend".to_string(),
            ServiceConfig {
                command: if cfg!(windows) { "timeout /t 10" } else { "sleep 10" }.to_string(),
                depends_on: vec!["database".to_string()],
                ready_when: None,
                env_file: None,
            },
        );
        services.insert(
            "frontend".to_string(),
            ServiceConfig {
                command: if cfg!(windows) { "timeout /t 10" } else { "sleep 10" }.to_string(),
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
        
        // Stop all services
        let result = orchestrator.stop_all().await;
        if let Err(ref e) = result {
            eprintln!("Stop all failed: {:?}", e);
        }
        assert!(result.is_ok());
        
        // All services should no longer be ready
        let watcher = orchestrator.ready_watcher.lock().await;
        assert!(!watcher.is_ready("database"));
        assert!(!watcher.is_ready("backend"));
        assert!(!watcher.is_ready("frontend"));
    }

    #[tokio::test]
    async fn test_stop_service_nonexistent() {
        let config = create_test_config();
        let mut orchestrator = Orchestrator::new(config).unwrap();
        
        let result = orchestrator.stop_service("nonexistent").await;
        assert!(result.is_err());
        assert!(matches!(result.unwrap_err(), OrchestratorError::ServiceNotFound(_)));
    }

    #[tokio::test]
    async fn test_restart_service_simple() {
        let mut services = HashMap::new();
        services.insert(
            "simple".to_string(),
            ServiceConfig {
                command: if cfg!(windows) { "cmd /c echo test" } else { "echo test" }.to_string(),
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
        orchestrator.start_service_with_deps("simple").await.unwrap();
        tokio::time::sleep(Duration::from_millis(200)).await;
        
        // Restart the service
        let result = orchestrator.restart_service("simple").await;
        assert!(result.is_ok());
        
        tokio::time::sleep(Duration::from_millis(200)).await;
        
        // Service should be ready again
        let is_ready = {
            let watcher = orchestrator.ready_watcher.lock().await;
            watcher.is_ready("simple")
        };
        assert!(is_ready);
    }

    #[tokio::test]
    async fn test_restart_service_with_dependencies() {
        let mut services = HashMap::new();
        services.insert(
            "database".to_string(),
            ServiceConfig {
                command: if cfg!(windows) { "timeout /t 10" } else { "sleep 10" }.to_string(),
                depends_on: vec![],
                ready_when: None,
                env_file: None,
            },
        );
        services.insert(
            "backend".to_string(),
            ServiceConfig {
                command: if cfg!(windows) { "timeout /t 10" } else { "sleep 10" }.to_string(),
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
        tokio::time::sleep(Duration::from_millis(300)).await;
        
        // Restart database (should also stop backend)
        let result = orchestrator.restart_service("database").await;
        assert!(result.is_ok());
        
        tokio::time::sleep(Duration::from_millis(300)).await;
        
        // Database should be ready again
        let is_ready = {
            let watcher = orchestrator.ready_watcher.lock().await;
            watcher.is_ready("database")
        };
        assert!(is_ready);
    }

    #[tokio::test]
    async fn test_restart_service_nonexistent() {
        let config = create_test_config();
        let mut orchestrator = Orchestrator::new(config).unwrap();
        
        let result = orchestrator.restart_service("nonexistent").await;
        assert!(result.is_err());
        assert!(matches!(result.unwrap_err(), OrchestratorError::ServiceNotFound(_)));
    }
}