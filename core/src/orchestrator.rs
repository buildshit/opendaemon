use crate::config::DmnConfig;
use crate::graph::{GraphError, ServiceGraph};
use crate::logs::{LogBuffer, LogLine};
use crate::process::{ProcessError, ProcessManager};
use crate::ready::ReadyWatcher;
use std::sync::Arc;
use std::time::Duration;
use thiserror::Error;
use tokio::sync::{mpsc, Mutex};

#[derive(Debug, Clone)]
pub enum OrchestratorEvent {
    ServiceStarting { service: String },
    ServiceReady { service: String },
    ServiceFailed { service: String, error: String },
    ServiceStopped { service: String },
    LogLine { service: String, line: LogLine },
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
    event_tx: mpsc::UnboundedSender<OrchestratorEvent>,
    event_rx: Arc<Mutex<mpsc::UnboundedReceiver<OrchestratorEvent>>>,
}

impl std::fmt::Debug for Orchestrator {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("Orchestrator")
            .field("config", &self.config)
            .field("graph", &"<ServiceGraph>")
            .field("process_manager", &"<ProcessManager>")
            .field("log_buffer", &"<Arc<Mutex<LogBuffer>>>")
            .field("ready_watcher", &"<Arc<Mutex<ReadyWatcher>>>")
            .field("event_tx", &"<mpsc::UnboundedSender>")
            .field("event_rx", &"<Arc<Mutex<mpsc::UnboundedReceiver>>>")
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
        
        // Create process manager
        let process_manager = ProcessManager::new(Arc::clone(&log_buffer));
        
        // Create ready watcher with default timeout of 30 seconds
        let ready_watcher = Arc::new(Mutex::new(ReadyWatcher::new(Duration::from_secs(30))));
        
        // Create event channel for orchestrator events
        let (event_tx, event_rx) = mpsc::unbounded_channel();
        
        Ok(Self {
            config,
            graph,
            process_manager,
            log_buffer,
            ready_watcher,
            event_tx,
            event_rx: Arc::new(Mutex::new(event_rx)),
        })
    }
    
    /// Get a receiver for orchestrator events
    /// This allows external consumers to listen to events
    pub fn subscribe_events(&self) -> mpsc::UnboundedSender<OrchestratorEvent> {
        self.event_tx.clone()
    }
    
    /// Emit an event to all subscribers
    fn emit_event(&self, event: OrchestratorEvent) {
        // Ignore send errors - if no one is listening, that's okay
        let _ = self.event_tx.send(event);
    }
    
    /// Get the configuration
    pub fn config(&self) -> &DmnConfig {
        &self.config
    }
    
    /// Get the dependency graph
    pub fn graph(&self) -> &ServiceGraph {
        &self.graph
    }
    
    /// Start all services in dependency order
    /// Services are started according to their dependencies, with each service
    /// waiting for its dependencies to be ready before starting
    pub async fn start_all(&mut self) -> Result<(), OrchestratorError> {
        // Get the start order from the dependency graph
        let start_order = self.graph.get_start_order()?;
        
        // Start each service in order
        for service_name in start_order {
            self.start_service_with_deps(&service_name).await?;
        }
        
        Ok(())
    }
    
    /// Start a service along with its dependencies
    /// This method ensures all dependencies are ready before starting the service
    pub async fn start_service_with_deps(&mut self, service_name: &str) -> Result<(), OrchestratorError> {
        // Check if service exists in config
        let service_config = self.config.services.get(service_name)
            .ok_or_else(|| OrchestratorError::ServiceNotFound(service_name.to_string()))?
            .clone();
        
        // Emit starting event
        self.emit_event(OrchestratorEvent::ServiceStarting {
            service: service_name.to_string(),
        });
        
        // Get dependencies for this service
        let dependencies = self.graph.get_dependencies(service_name)?;
        
        // Wait for all dependencies to be ready
        for dep in dependencies {
            let is_ready = {
                let watcher = self.ready_watcher.lock().await;
                watcher.is_ready(&dep)
            };
            
            if !is_ready {
                // Wait for dependency to become ready
                self.wait_for_ready(&dep).await?;
            }
        }
        
        // Spawn the service process
        self.process_manager.spawn_service(service_name, &service_config).await?;
        
        // Update status to Starting
        self.process_manager.update_status(service_name, crate::process::ServiceStatus::Starting);
        
        // Set up ready watching based on configuration
        if let Some(ready_condition) = &service_config.ready_when {
            // Create a channel to receive log lines for this service
            let (log_tx, log_rx) = mpsc::channel(100);
            
            // Spawn a task to forward logs to the ready watcher
            let log_buffer = Arc::clone(&self.log_buffer);
            let service_name_clone = service_name.to_string();
            tokio::spawn(async move {
                // Poll for new log lines and forward them
                let mut last_count = 0;
                loop {
                    tokio::time::sleep(Duration::from_millis(50)).await;
                    let buffer = log_buffer.lock().await;
                    let lines = buffer.get_all_lines(&service_name_clone);
                    let current_count = lines.len();
                    drop(buffer);
                    
                    // Only send new lines
                    if current_count > last_count {
                        for line in lines.iter().skip(last_count) {
                            if log_tx.send(line.clone()).await.is_err() {
                                return;
                            }
                        }
                        last_count = current_count;
                    }
                }
            });
            
            // Watch for readiness in a separate task
            let service_name_clone = service_name.to_string();
            let ready_condition_clone = ready_condition.clone();
            let ready_watcher = Arc::clone(&self.ready_watcher);
            let event_tx = self.event_tx.clone();
            
            tokio::spawn(async move {
                let result = {
                    let mut watcher = ready_watcher.lock().await;
                    watcher.watch_service(
                        service_name_clone.clone(),
                        ready_condition_clone,
                        Some(log_rx),
                    ).await
                };
                
                match result {
                    Ok(_) => {
                        // Emit ready event
                        let _ = event_tx.send(OrchestratorEvent::ServiceReady {
                            service: service_name_clone,
                        });
                    }
                    Err(e) => {
                        // Emit failed event
                        let _ = event_tx.send(OrchestratorEvent::ServiceFailed {
                            service: service_name_clone,
                            error: e.to_string(),
                        });
                    }
                }
            });
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
        let start_order = self.graph.get_start_order()?;
        let stop_order: Vec<_> = start_order.into_iter().rev().collect();
        
        // Stop each service in reverse order
        for service_name in stop_order {
            // Only stop if the service is actually running
            if self.process_manager.get_status(&service_name).is_some() {
                self.stop_service(&service_name).await?;
            }
        }
        
        Ok(())
    }
    
    /// Stop a service and all services that depend on it
    /// This implements cascade stopping - dependents are stopped first
    pub async fn stop_service(&mut self, service_name: &str) -> Result<(), OrchestratorError> {
        // Check if service exists in config
        if !self.config.services.contains_key(service_name) {
            return Err(OrchestratorError::ServiceNotFound(service_name.to_string()));
        }
        
        // Check if service is already stopped or not running
        if let Some(status) = self.process_manager.get_status(service_name) {
            if matches!(status, crate::process::ServiceStatus::Stopped | crate::process::ServiceStatus::Failed { .. }) {
                // Already stopped, nothing to do
                return Ok(());
            }
        } else {
            // Service not found in process manager, nothing to stop
            return Ok(());
        }
        
        // Get list of dependent services (services that depend on this one)
        let dependents = self.graph.get_dependents(service_name)?;
        
        // Stop dependents first (cascade)
        for dependent in dependents {
            // Only stop if the service is actually running
            if self.process_manager.get_status(&dependent).is_some() {
                // Use Box::pin to handle recursion
                Box::pin(self.stop_service(&dependent)).await?;
            }
        }
        
        // Now stop the target service itself
        self.process_manager.stop_service(service_name).await?;
        
        // Reset ready status
        {
            let mut watcher = self.ready_watcher.lock().await;
            watcher.reset_service(service_name);
        }
        
        // Emit stopped event
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
            return Err(OrchestratorError::ServiceNotFound(service_name.to_string()));
        }
        
        // Stop the service (and its dependents)
        self.stop_service(service_name).await?;
        
        // Wait a moment for cleanup
        tokio::time::sleep(Duration::from_millis(100)).await;
        
        // Start the service again
        self.start_service_with_deps(service_name).await?;
        
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
        
        let event_sender = orchestrator.subscribe_events();
        
        // Test that we can send events
        let test_event = OrchestratorEvent::ServiceStarting {
            service: "test".to_string(),
        };
        
        assert!(event_sender.send(test_event).is_ok());
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