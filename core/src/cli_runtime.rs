use crate::config::{parse_config, DmnConfig};
use crate::orchestrator::{Orchestrator, OrchestratorEvent};
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::path::{Path, PathBuf};
use std::sync::Arc;
use std::time::{Duration, SystemTime, UNIX_EPOCH};
use tokio::sync::Mutex;
use tokio::time::Instant;

const HEARTBEAT_INTERVAL: Duration = Duration::from_secs(1);
const HEARTBEAT_STALE_AFTER: Duration = Duration::from_secs(5);
const CONTROL_POLL_INTERVAL: Duration = Duration::from_millis(250);
const STOP_WAIT_TIMEOUT: Duration = Duration::from_secs(20);
const CONTROL_ACTION_TIMEOUT: Duration = Duration::from_secs(20);

const STATUS_NOT_STARTED: &str = "not_started";
const STATUS_STARTING: &str = "starting";
const STATUS_RUNNING: &str = "running";
const STATUS_STOPPED: &str = "stopped";
const STATUS_FAILED_PREFIX: &str = "failed:";

const ACTION_STOP: &str = "stop";
const ACTION_START_SERVICE: &str = "start_service";
const ACTION_STOP_SERVICE: &str = "stop_service";
const ACTION_RESTART_SERVICE: &str = "restart_service";

#[derive(Debug, Clone, Serialize, Deserialize)]
struct RuntimeState {
    config_path: String,
    started_at_unix: u64,
    heartbeat_unix: u64,
    services: HashMap<String, String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
struct ControlRequest {
    action: String,
    #[serde(default)]
    service: Option<String>,
    requested_at_unix: u64,
}

#[derive(Debug, Clone)]
struct RuntimePaths {
    runtime_dir: PathBuf,
    state_file: PathBuf,
    control_file: PathBuf,
}

impl RuntimePaths {
    fn for_config(config_path: &Path) -> Self {
        let config_dir = config_path.parent().unwrap_or_else(|| Path::new("."));
        let runtime_dir = config_dir.join(".dmn");
        Self {
            runtime_dir: runtime_dir.clone(),
            state_file: runtime_dir.join("runtime-state.json"),
            control_file: runtime_dir.join("runtime-control.json"),
        }
    }
}

pub async fn run_start_command(config_path: PathBuf, service: Option<String>) -> i32 {
    let resolved_config_path = resolve_config_path(&config_path);
    let config_id = config_identifier(&resolved_config_path);
    let paths = RuntimePaths::for_config(&resolved_config_path);

    match load_runtime_state(&paths) {
        Ok(Some(existing)) if is_state_active(&existing) => {
            if existing.config_path != config_id {
                eprintln!(
                    "Another OpenDaemon supervisor is already running for config: {}",
                    existing.config_path
                );
                return 1;
            }

            if let Some(service_name) = service.as_deref() {
                if let Err(e) = send_control_action_and_wait(
                    &paths,
                    &config_id,
                    ACTION_START_SERVICE,
                    Some(service_name),
                    CONTROL_ACTION_TIMEOUT,
                )
                .await
                {
                    eprintln!("Failed to request service start: {}", e);
                    return 1;
                }
                eprintln!("Start request sent for service '{}'.", service_name);
                return 0;
            }

            eprintln!(
                "OpenDaemon supervisor is already running for config: {}",
                existing.config_path
            );
            eprintln!(
                "Use `dmn stop --config {}` first.",
                resolved_config_path.display()
            );
            return 1;
        }
        Ok(Some(_)) => {
            cleanup_runtime_files(&paths);
        }
        Ok(None) => {}
        Err(e) => {
            eprintln!("Failed to read runtime state: {}", e);
            return 1;
        }
    }

    let dmn_config = match parse_config(&resolved_config_path) {
        Ok(cfg) => cfg,
        Err(e) => {
            eprintln!("Failed to load configuration: {}", e);
            return 1;
        }
    };

    if let Some(service_name) = service.as_deref() {
        if !dmn_config.services.contains_key(service_name) {
            eprintln!("Service '{}' not found in configuration.", service_name);
            return 1;
        }
    }

    let initial_state = RuntimeState {
        config_path: config_id.clone(),
        started_at_unix: now_unix_secs(),
        heartbeat_unix: now_unix_secs(),
        services: default_service_statuses(&dmn_config),
    };
    if let Err(e) = persist_runtime_state(&paths, &initial_state) {
        eprintln!("Failed to initialize runtime state: {}", e);
        return 1;
    }
    let _ = remove_file_if_exists(&paths.control_file);

    let orchestrator = match Orchestrator::new(dmn_config) {
        Ok(orch) => Arc::new(Mutex::new(orch)),
        Err(e) => {
            eprintln!("Failed to create orchestrator: {}", e);
            cleanup_runtime_files(&paths);
            return 1;
        }
    };
    let state = Arc::new(Mutex::new(initial_state));

    // Keep runtime state updated as services transition through their lifecycle.
    let mut event_rx = {
        let orch = orchestrator.lock().await;
        orch.subscribe_events()
    };
    let event_paths = paths.clone();
    let event_state = Arc::clone(&state);
    tokio::spawn(async move {
        while let Ok(event) = event_rx.recv().await {
            let mut should_persist = false;
            {
                let mut state_guard = event_state.lock().await;
                match event {
                    OrchestratorEvent::ServiceStarting { service } => {
                        state_guard
                            .services
                            .insert(service.clone(), STATUS_STARTING.to_string());
                        eprintln!("Starting {}...", service);
                        should_persist = true;
                    }
                    OrchestratorEvent::ServiceReady { service } => {
                        state_guard
                            .services
                            .insert(service.clone(), STATUS_RUNNING.to_string());
                        eprintln!("{} is ready", service);
                        should_persist = true;
                    }
                    OrchestratorEvent::ServiceFailed { service, error } => {
                        state_guard.services.insert(
                            service.clone(),
                            format!("{} {}", STATUS_FAILED_PREFIX, compact_error(&error)),
                        );
                        eprintln!("{} failed: {}", service, compact_error(&error));
                        should_persist = true;
                    }
                    OrchestratorEvent::ServiceStopped { service } => {
                        state_guard
                            .services
                            .insert(service.clone(), STATUS_STOPPED.to_string());
                        eprintln!("Stopped {}", service);
                        should_persist = true;
                    }
                    OrchestratorEvent::Error { message, category } => {
                        eprintln!("[{}] {}", category, message);
                    }
                    OrchestratorEvent::LogLine { .. } => {}
                }
                if should_persist {
                    state_guard.heartbeat_unix = now_unix_secs();
                }
            }

            if should_persist {
                if let Err(e) = persist_state_from_mutex(&event_paths, &event_state).await {
                    eprintln!("Warning: failed to persist runtime state: {}", e);
                }
            }
        }
    });

    // Start either all services or a specific service with dependencies.
    let startup_result = {
        let mut orch = orchestrator.lock().await;
        match service.as_deref() {
            Some(service_name) => orch.start_service_with_deps(service_name).await,
            None => orch.start_all().await,
        }
    };
    if let Err(e) = startup_result {
        eprintln!("Failed to start services: {}", e);
        cleanup_runtime_files(&paths);
        return 1;
    }

    if let Some(service_name) = service.as_deref() {
        eprintln!(
            "Service '{}' launched. Press Ctrl+C to stop all running services.",
            service_name
        );
    } else {
        eprintln!("Services launched. Press Ctrl+C to stop.");
    }
    eprintln!("You can also run `dmn stop` from another terminal.");

    let mut ctrl_c = std::pin::pin!(tokio::signal::ctrl_c());
    let mut heartbeat_ticker = tokio::time::interval(HEARTBEAT_INTERVAL);
    let mut reconcile_ticker = tokio::time::interval(CONTROL_POLL_INTERVAL);

    loop {
        tokio::select! {
            _ = &mut ctrl_c => {
                eprintln!("Interrupt received. Stopping services...");
                break;
            }
            _ = heartbeat_ticker.tick() => {
                if let Err(e) = touch_heartbeat(&paths, &state).await {
                    eprintln!("Warning: failed to update heartbeat: {}", e);
                }
            }
            _ = reconcile_ticker.tick() => {
                {
                    let mut orch = orchestrator.lock().await;
                    orch.reconcile_exited_processes().await;
                }

                match read_control_request(&paths) {
                    Ok(Some(control)) => {
                        if control.action == ACTION_STOP {
                            eprintln!("Stop request received. Shutting down...");
                            let _ = remove_file_if_exists(&paths.control_file);
                            break;
                        }

                        let action_result: Result<(), String> = {
                            let mut orch = orchestrator.lock().await;
                            match control.action.as_str() {
                                ACTION_START_SERVICE => match control.service.as_deref() {
                                    Some(service_name) => orch
                                        .start_service_with_deps(service_name)
                                        .await
                                        .map_err(|e| e.to_string()),
                                    None => Err("Missing service name for start_service request".to_string()),
                                },
                                ACTION_STOP_SERVICE => match control.service.as_deref() {
                                    Some(service_name) => orch
                                        .stop_service(service_name)
                                        .await
                                        .map_err(|e| e.to_string()),
                                    None => Err("Missing service name for stop_service request".to_string()),
                                },
                                ACTION_RESTART_SERVICE => match control.service.as_deref() {
                                    Some(service_name) => orch
                                        .restart_service(service_name)
                                        .await
                                        .map_err(|e| e.to_string()),
                                    None => Err("Missing service name for restart_service request".to_string()),
                                },
                                other => Err(format!("Unknown control action '{}'", other)),
                            }
                        };

                        match action_result {
                            Ok(_) => {
                                if let Some(service_name) = control.service.as_deref() {
                                    eprintln!("Processed '{}' for service '{}'.", control.action, service_name);
                                } else {
                                    eprintln!("Processed control action '{}'.", control.action);
                                }
                            }
                            Err(error) => {
                                if let Some(service_name) = control.service.as_deref() {
                                    {
                                        let mut state_guard = state.lock().await;
                                        state_guard.services.insert(
                                            service_name.to_string(),
                                            format!("{} {}", STATUS_FAILED_PREFIX, compact_error(&error)),
                                        );
                                        state_guard.heartbeat_unix = now_unix_secs();
                                    }
                                    let _ = persist_state_from_mutex(&paths, &state).await;
                                }
                                eprintln!("Control action failed: {}", error);
                            }
                        }

                        let _ = remove_file_if_exists(&paths.control_file);
                    }
                    Ok(None) => {}
                    Err(e) => {
                        eprintln!("Warning: failed to read control request: {}", e);
                    }
                }
            }
        }
    }

    let stop_result = {
        let mut orch = orchestrator.lock().await;
        orch.stop_all().await
    };

    {
        let mut state_guard = state.lock().await;
        for service_status in state_guard.services.values_mut() {
            *service_status = STATUS_STOPPED.to_string();
        }
        state_guard.heartbeat_unix = now_unix_secs();
    }
    if let Err(e) = persist_state_from_mutex(&paths, &state).await {
        eprintln!("Warning: failed to persist final runtime state: {}", e);
    }
    cleanup_runtime_files(&paths);

    match stop_result {
        Ok(_) => {
            eprintln!("All services stopped.");
            0
        }
        Err(e) => {
            eprintln!("Failed to stop services cleanly: {}", e);
            1
        }
    }
}

pub async fn run_stop_command(config_path: PathBuf, service: Option<String>) -> i32 {
    let resolved_config_path = resolve_config_path(&config_path);
    let config_id = config_identifier(&resolved_config_path);
    let paths = RuntimePaths::for_config(&resolved_config_path);

    let state = match load_runtime_state(&paths) {
        Ok(Some(state)) => state,
        Ok(None) => {
            eprintln!("No running OpenDaemon supervisor found.");
            return 0;
        }
        Err(e) => {
            eprintln!("Failed to read runtime state: {}", e);
            return 1;
        }
    };

    if state.config_path != config_id {
        eprintln!(
            "A supervisor is running for a different config: {}",
            state.config_path
        );
        eprintln!("Use the matching `--config` path to control that supervisor.");
        return 1;
    }

    if !is_state_active(&state) {
        eprintln!("Found stale runtime state. Cleaning it up.");
        cleanup_runtime_files(&paths);
        return 0;
    }

    if let Some(service_name) = service.as_deref() {
        if let Err(e) = send_control_action_and_wait(
            &paths,
            &config_id,
            ACTION_STOP_SERVICE,
            Some(service_name),
            CONTROL_ACTION_TIMEOUT,
        )
        .await
        {
            eprintln!("Failed to request service stop: {}", e);
            return 1;
        }
        eprintln!("Stop request sent for service '{}'.", service_name);
        return 0;
    }

    if let Err(e) = write_control_request(&paths, ACTION_STOP, None) {
        eprintln!("Failed to send stop request: {}", e);
        return 1;
    }

    let deadline = Instant::now() + STOP_WAIT_TIMEOUT;
    loop {
        if Instant::now() >= deadline {
            eprintln!("Timed out waiting for supervisor shutdown.");
            return 1;
        }

        tokio::time::sleep(Duration::from_millis(250)).await;
        match load_runtime_state(&paths) {
            Ok(None) => {
                eprintln!("All services stopped.");
                return 0;
            }
            Ok(Some(current_state)) => {
                if !is_state_active(&current_state) {
                    eprintln!("Supervisor became unresponsive. Cleaning stale runtime state.");
                    cleanup_runtime_files(&paths);
                    return 0;
                }
            }
            Err(e) => {
                eprintln!("Failed to monitor shutdown progress: {}", e);
                return 1;
            }
        }
    }
}

pub async fn run_restart_command(config_path: PathBuf, service: String) -> i32 {
    let resolved_config_path = resolve_config_path(&config_path);
    let config_id = config_identifier(&resolved_config_path);
    let paths = RuntimePaths::for_config(&resolved_config_path);

    let state = match load_runtime_state(&paths) {
        Ok(Some(state)) => state,
        Ok(None) => {
            eprintln!("No running OpenDaemon supervisor found.");
            eprintln!("Run `dmn start` first.");
            return 1;
        }
        Err(e) => {
            eprintln!("Failed to read runtime state: {}", e);
            return 1;
        }
    };

    if state.config_path != config_id {
        eprintln!(
            "A supervisor is running for a different config: {}",
            state.config_path
        );
        return 1;
    }
    if !is_state_active(&state) {
        eprintln!("Supervisor state is stale. Run `dmn start` again.");
        cleanup_runtime_files(&paths);
        return 1;
    }

    if let Err(e) = send_control_action_and_wait(
        &paths,
        &config_id,
        ACTION_RESTART_SERVICE,
        Some(&service),
        CONTROL_ACTION_TIMEOUT,
    )
    .await
    {
        eprintln!("Failed to request service restart: {}", e);
        return 1;
    }

    eprintln!("Restart request sent for service '{}'.", service);
    0
}

pub async fn run_status_command(config_path: PathBuf, service_filter: Option<String>) -> i32 {
    let resolved_config_path = resolve_config_path(&config_path);
    let config_id = config_identifier(&resolved_config_path);
    let paths = RuntimePaths::for_config(&resolved_config_path);

    let dmn_config = match parse_config(&resolved_config_path) {
        Ok(cfg) => cfg,
        Err(e) => {
            eprintln!("Failed to load configuration: {}", e);
            return 1;
        }
    };

    if let Some(service_name) = service_filter.as_deref() {
        if !dmn_config.services.contains_key(service_name) {
            eprintln!("Service '{}' not found in configuration.", service_name);
            return 1;
        }
    }

    let state = match load_runtime_state(&paths) {
        Ok(state) => state,
        Err(e) => {
            eprintln!("Warning: failed to read runtime state: {}", e);
            None
        }
    };

    let runtime_is_active = state
        .as_ref()
        .map(|s| s.config_path == config_id && is_state_active(s))
        .unwrap_or(false);
    let stale_state_for_config = state
        .as_ref()
        .map(|s| s.config_path == config_id && !is_state_active(s))
        .unwrap_or(false);

    let mut service_names: Vec<_> = dmn_config.services.keys().cloned().collect();
    service_names.sort();
    if let Some(service_name) = service_filter {
        service_names.retain(|name| name == &service_name);
    }

    eprintln!("\nService Status:");
    eprintln!("{:-<60}", "");
    eprintln!(
        "Supervisor: {}",
        if runtime_is_active {
            "running"
        } else {
            "not running"
        }
    );

    for service_name in service_names {
        let raw_status = if runtime_is_active {
            state
                .as_ref()
                .and_then(|s| s.services.get(&service_name))
                .map(|s| s.as_str())
                .unwrap_or(STATUS_NOT_STARTED)
        } else {
            STATUS_NOT_STARTED
        };
        eprintln!("{:<30} {}", service_name, display_status(raw_status));
    }

    if stale_state_for_config {
        eprintln!("\nNote: Found stale runtime state from a previous session.");
    }

    0
}

fn resolve_config_path(config_path: &Path) -> PathBuf {
    if config_path.is_absolute() {
        config_path.to_path_buf()
    } else {
        std::env::current_dir()
            .unwrap_or_else(|_| PathBuf::from("."))
            .join(config_path)
    }
}

fn config_identifier(config_path: &Path) -> String {
    std::fs::canonicalize(config_path)
        .unwrap_or_else(|_| config_path.to_path_buf())
        .to_string_lossy()
        .to_string()
}

fn now_unix_secs() -> u64 {
    SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .unwrap_or_default()
        .as_secs()
}

fn default_service_statuses(config: &DmnConfig) -> HashMap<String, String> {
    config
        .services
        .keys()
        .map(|name| (name.clone(), STATUS_NOT_STARTED.to_string()))
        .collect()
}

fn is_state_active(state: &RuntimeState) -> bool {
    let now = now_unix_secs();
    now.saturating_sub(state.heartbeat_unix) <= HEARTBEAT_STALE_AFTER.as_secs()
}

fn compact_error(error: &str) -> String {
    let first_line = error.lines().next().unwrap_or(error).trim();
    if first_line.len() > 120 {
        format!("{}...", &first_line[..117])
    } else {
        first_line.to_string()
    }
}

fn display_status(raw: &str) -> String {
    match raw {
        STATUS_NOT_STARTED => "Not Started".to_string(),
        STATUS_STARTING => "Starting".to_string(),
        STATUS_RUNNING => "Running".to_string(),
        STATUS_STOPPED => "Stopped".to_string(),
        _ if raw.starts_with(STATUS_FAILED_PREFIX) => {
            let details = raw
                .trim_start_matches(STATUS_FAILED_PREFIX)
                .trim()
                .to_string();
            if details.is_empty() {
                "Failed".to_string()
            } else {
                format!("Failed ({})", details)
            }
        }
        _ => raw.to_string(),
    }
}

fn load_runtime_state(paths: &RuntimePaths) -> Result<Option<RuntimeState>, String> {
    if !paths.state_file.exists() {
        return Ok(None);
    }

    let contents = std::fs::read_to_string(&paths.state_file).map_err(|e| {
        format!(
            "failed reading runtime state '{}': {}",
            paths.state_file.display(),
            e
        )
    })?;

    let state = serde_json::from_str::<RuntimeState>(&contents).map_err(|e| {
        format!(
            "failed parsing runtime state '{}': {}",
            paths.state_file.display(),
            e
        )
    })?;

    Ok(Some(state))
}

fn persist_runtime_state(paths: &RuntimePaths, state: &RuntimeState) -> Result<(), String> {
    std::fs::create_dir_all(&paths.runtime_dir).map_err(|e| {
        format!(
            "failed creating runtime directory '{}': {}",
            paths.runtime_dir.display(),
            e
        )
    })?;

    let payload = serde_json::to_string_pretty(state)
        .map_err(|e| format!("failed serializing runtime state: {}", e))?;

    std::fs::write(&paths.state_file, payload).map_err(|e| {
        format!(
            "failed writing runtime state '{}': {}",
            paths.state_file.display(),
            e
        )
    })?;

    Ok(())
}

async fn persist_state_from_mutex(
    paths: &RuntimePaths,
    state: &Arc<Mutex<RuntimeState>>,
) -> Result<(), String> {
    let snapshot = { state.lock().await.clone() };
    persist_runtime_state(paths, &snapshot)
}

async fn touch_heartbeat(
    paths: &RuntimePaths,
    state: &Arc<Mutex<RuntimeState>>,
) -> Result<(), String> {
    {
        let mut guard = state.lock().await;
        guard.heartbeat_unix = now_unix_secs();
    }
    persist_state_from_mutex(paths, state).await
}

async fn send_control_action_and_wait(
    paths: &RuntimePaths,
    config_id: &str,
    action: &str,
    service: Option<&str>,
    timeout_duration: Duration,
) -> Result<(), String> {
    write_control_request(paths, action, service)?;

    let deadline = Instant::now() + timeout_duration;
    loop {
        if !paths.control_file.exists() {
            return Ok(());
        }
        if Instant::now() >= deadline {
            return Err("timed out waiting for supervisor to process control action".to_string());
        }

        tokio::time::sleep(Duration::from_millis(100)).await;
        match load_runtime_state(paths)? {
            Some(state) => {
                if state.config_path != config_id {
                    return Err("runtime state changed to a different configuration".to_string());
                }
                if !is_state_active(&state) {
                    return Err("supervisor is unresponsive".to_string());
                }
            }
            None => {
                return Err("supervisor is no longer running".to_string());
            }
        }
    }
}

fn write_control_request(
    paths: &RuntimePaths,
    action: &str,
    service: Option<&str>,
) -> Result<(), String> {
    std::fs::create_dir_all(&paths.runtime_dir).map_err(|e| {
        format!(
            "failed creating runtime directory '{}': {}",
            paths.runtime_dir.display(),
            e
        )
    })?;

    let request = ControlRequest {
        action: action.to_string(),
        service: service.map(|s| s.to_string()),
        requested_at_unix: now_unix_secs(),
    };
    let payload = serde_json::to_string_pretty(&request)
        .map_err(|e| format!("failed serializing control request: {}", e))?;
    std::fs::write(&paths.control_file, payload).map_err(|e| {
        format!(
            "failed writing control request '{}': {}",
            paths.control_file.display(),
            e
        )
    })
}

fn read_control_request(paths: &RuntimePaths) -> Result<Option<ControlRequest>, String> {
    if !paths.control_file.exists() {
        return Ok(None);
    }

    let contents = std::fs::read_to_string(&paths.control_file).map_err(|e| {
        format!(
            "failed reading control request '{}': {}",
            paths.control_file.display(),
            e
        )
    })?;

    let request = serde_json::from_str::<ControlRequest>(&contents).map_err(|e| {
        format!(
            "failed parsing control request '{}': {}",
            paths.control_file.display(),
            e
        )
    })?;
    Ok(Some(request))
}

fn remove_file_if_exists(path: &Path) -> Result<(), String> {
    if path.exists() {
        std::fs::remove_file(path)
            .map_err(|e| format!("failed removing file '{}': {}", path.display(), e))?;
    }
    Ok(())
}

fn cleanup_runtime_files(paths: &RuntimePaths) {
    let _ = remove_file_if_exists(&paths.control_file);
    let _ = remove_file_if_exists(&paths.state_file);
    let _ = std::fs::remove_dir(&paths.runtime_dir);
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_display_status_known_values() {
        assert_eq!(display_status("not_started"), "Not Started");
        assert_eq!(display_status("starting"), "Starting");
        assert_eq!(display_status("running"), "Running");
        assert_eq!(display_status("stopped"), "Stopped");
    }

    #[test]
    fn test_display_status_failed_value() {
        assert_eq!(
            display_status("failed: Process exited with code 1"),
            "Failed (Process exited with code 1)"
        );
    }

    #[test]
    fn test_state_activity_window() {
        let now = now_unix_secs();
        let active = RuntimeState {
            config_path: "test".to_string(),
            started_at_unix: now.saturating_sub(2),
            heartbeat_unix: now.saturating_sub(2),
            services: HashMap::new(),
        };
        assert!(is_state_active(&active));

        let stale = RuntimeState {
            config_path: "test".to_string(),
            started_at_unix: now.saturating_sub(30),
            heartbeat_unix: now.saturating_sub(30),
            services: HashMap::new(),
        };
        assert!(!is_state_active(&stale));
    }

    #[test]
    fn test_control_request_serialization_round_trip() {
        let request = ControlRequest {
            action: ACTION_START_SERVICE.to_string(),
            service: Some("api".to_string()),
            requested_at_unix: 123,
        };
        let serialized = serde_json::to_string(&request).unwrap();
        let deserialized: ControlRequest = serde_json::from_str(&serialized).unwrap();
        assert_eq!(deserialized.action, ACTION_START_SERVICE);
        assert_eq!(deserialized.service.as_deref(), Some("api"));
        assert_eq!(deserialized.requested_at_unix, 123);
    }
}
