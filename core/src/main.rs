use clap::{Parser, Subcommand};
use serde_json::{json, Value};
use std::path::PathBuf;
use std::sync::Arc;
use tokio::sync::Mutex;

mod cli_daemon_client;
mod cli_runtime;
mod config;
mod graph;
mod logs;
mod mcp_server;
mod orchestrator;
mod process;
mod ready;
mod rpc;

use config::parse_config;
use mcp_server::DmnMcpServer;
use orchestrator::Orchestrator;
use rpc::RpcServer;

#[derive(Parser)]
#[command(name = "dmn")]
#[command(about = "OpenDaemon - Local development service orchestrator", long_about = None)]
#[command(version)]
struct Cli {
    #[command(subcommand)]
    command: Commands,
}

#[derive(Subcommand)]
enum Commands {
    /// Run in daemon mode for VS Code extension (JSON-RPC over stdio)
    Daemon {
        /// Path to dmn.json configuration file
        #[arg(short, long, default_value = "dmn.json")]
        config: PathBuf,
    },
    /// Run in MCP server mode for AI agents (Model Context Protocol over stdio)
    Mcp {
        /// Path to dmn.json configuration file
        #[arg(short, long, default_value = "dmn.json")]
        config: PathBuf,
        /// Validate MCP setup and exit
        #[arg(long)]
        check: bool,
    },
    /// Start the local supervisor and launch services
    Start {
        /// Path to dmn.json configuration file
        #[arg(short, long, default_value = "dmn.json")]
        config: PathBuf,
        /// Optional service name to start (with dependencies)
        service: Option<String>,
    },
    /// Stop services managed by a running `dmn start`
    Stop {
        /// Path to dmn.json configuration file
        #[arg(short, long, default_value = "dmn.json")]
        config: PathBuf,
        /// Optional service name to stop
        service: Option<String>,
    },
    /// Restart a running service
    Restart {
        /// Path to dmn.json configuration file
        #[arg(short, long, default_value = "dmn.json")]
        config: PathBuf,
        /// Service name to restart
        service: String,
    },
    /// Show service status from the local CLI supervisor
    Status {
        /// Path to dmn.json configuration file
        #[arg(short, long, default_value = "dmn.json")]
        config: PathBuf,
        /// Optional service name to inspect
        service: Option<String>,
    },
}

#[tokio::main]
async fn main() {
    let cli = Cli::parse();

    let exit_code = match cli.command {
        Commands::Daemon { config } => run_daemon_mode(config).await,
        Commands::Mcp { config, check } => run_mcp_mode(config, check).await,
        Commands::Start { config, service } => run_start_command(config, service).await,
        Commands::Stop { config, service } => run_stop_command(config, service).await,
        Commands::Restart { config, service } => run_restart_command(config, service).await,
        Commands::Status { config, service } => run_status_command(config, service).await,
    };

    std::process::exit(exit_code);
}

/// Run in daemon mode - JSON-RPC server for VS Code extension
async fn run_daemon_mode(config_path: PathBuf) -> i32 {
    eprintln!("Starting daemon mode with config: {:?}", config_path);

    // Load configuration
    let dmn_config = match parse_config(&config_path) {
        Ok(cfg) => cfg,
        Err(e) => {
            eprintln!("Failed to load configuration: {}", e);
            return 1;
        }
    };

    // Create orchestrator
    let orchestrator = match Orchestrator::new(dmn_config) {
        Ok(orch) => Arc::new(Mutex::new(orch)),
        Err(e) => {
            eprintln!("Failed to create orchestrator: {}", e);
            return 1;
        }
    };

    // Create and run RPC server
    let rpc_server = RpcServer::with_cli_ipc(orchestrator, config_path);
    if let Err(e) = rpc_server.run().await {
        eprintln!("RPC server error: {}", e);
        return 1;
    }

    0
}

/// Run in MCP mode - Model Context Protocol server for AI agents
async fn run_mcp_mode(config_path: PathBuf, check_only: bool) -> i32 {
    eprintln!("Starting MCP server mode with config: {:?}", config_path);

    // Load configuration
    let dmn_config = match parse_config(&config_path) {
        Ok(cfg) => cfg,
        Err(e) => {
            eprintln!("Failed to load configuration: {}", e);
            return 1;
        }
    };

    if check_only {
        eprintln!(
            "MCP check passed: configuration loaded with {} service(s).",
            dmn_config.services.len()
        );
        return 0;
    }

    // Create orchestrator
    let orchestrator = match Orchestrator::new(dmn_config) {
        Ok(orch) => Arc::new(Mutex::new(orch)),
        Err(e) => {
            eprintln!("Failed to create orchestrator: {}", e);
            return 1;
        }
    };

    // Create and run MCP server
    // TODO: Check for Pro authentication and use new_authenticated() if authenticated
    // For now, using authenticated version as placeholder for Pro features
    let mcp_server = DmnMcpServer::new_authenticated(orchestrator);
    if let Err(e) = mcp_server.run_stdio().await {
        eprintln!("MCP server error: {}", e);
        return 1;
    }

    0
}

/// Start services under the local CLI supervisor
async fn run_start_command(config_path: PathBuf, service: Option<String>) -> i32 {
    let method = if service.is_some() {
        "startService"
    } else {
        "startAll"
    };
    let params = service
        .as_ref()
        .map(|service_name| json!({ "service": service_name }));

    match cli_daemon_client::request_via_daemon(&config_path, method, params).await {
        Ok(_) => {
            if let Some(service_name) = service {
                eprintln!(
                    "Service '{}' start requested via extension daemon.",
                    service_name
                );
            } else {
                eprintln!("Start requested via extension daemon.");
            }
            0
        }
        Err(cli_daemon_client::DaemonClientError::NotAvailable) => {
            cli_runtime::run_start_command(config_path, service).await
        }
        Err(e) => {
            eprintln!("Failed to start services via extension daemon: {}", e);
            1
        }
    }
}

/// Stop services managed by the local CLI supervisor
async fn run_stop_command(config_path: PathBuf, service: Option<String>) -> i32 {
    let method = if service.is_some() {
        "stopService"
    } else {
        "stopAll"
    };
    let params = service
        .as_ref()
        .map(|service_name| json!({ "service": service_name }));

    match cli_daemon_client::request_via_daemon(&config_path, method, params).await {
        Ok(_) => {
            if let Some(service_name) = service {
                eprintln!(
                    "Service '{}' stop requested via extension daemon.",
                    service_name
                );
            } else {
                eprintln!("Stop requested via extension daemon.");
            }
            0
        }
        Err(cli_daemon_client::DaemonClientError::NotAvailable) => {
            cli_runtime::run_stop_command(config_path, service).await
        }
        Err(e) => {
            eprintln!("Failed to stop services via extension daemon: {}", e);
            1
        }
    }
}

/// Restart a service managed by the local CLI supervisor
async fn run_restart_command(config_path: PathBuf, service: String) -> i32 {
    match cli_daemon_client::request_via_daemon(
        &config_path,
        "restartService",
        Some(json!({ "service": service.clone() })),
    )
    .await
    {
        Ok(_) => {
            eprintln!("Restart requested via extension daemon.");
            0
        }
        Err(cli_daemon_client::DaemonClientError::NotAvailable) => {
            cli_runtime::run_restart_command(config_path, service).await
        }
        Err(e) => {
            eprintln!("Failed to restart service via extension daemon: {}", e);
            1
        }
    }
}

/// Show service status from local runtime state
async fn run_status_command(config_path: PathBuf, service: Option<String>) -> i32 {
    match cli_daemon_client::request_via_daemon(&config_path, "getStatus", None).await {
        Ok(result) => render_daemon_status(result, service),
        Err(cli_daemon_client::DaemonClientError::NotAvailable) => {
            cli_runtime::run_status_command(config_path, service).await
        }
        Err(e) => {
            eprintln!("Failed to get status via extension daemon: {}", e);
            1
        }
    }
}

fn render_daemon_status(result: Value, service_filter: Option<String>) -> i32 {
    let services_obj = match result.get("services").and_then(|v| v.as_object()) {
        Some(obj) => obj,
        None => {
            eprintln!("Invalid status response from extension daemon.");
            return 1;
        }
    };

    let mut service_names: Vec<String> = services_obj.keys().cloned().collect();
    service_names.sort();
    if let Some(service_name) = service_filter {
        service_names.retain(|name| name == &service_name);
        if service_names.is_empty() {
            eprintln!("Service '{}' not found in configuration.", service_name);
            return 1;
        }
    }

    eprintln!("\nService Status:");
    eprintln!("{:-<60}", "");
    eprintln!("Controller: extension-daemon");

    for service_name in service_names {
        let status = services_obj
            .get(&service_name)
            .and_then(|v| v.as_str())
            .unwrap_or("unknown");
        eprintln!("{:<30} {}", service_name, display_status(status));
    }

    0
}

fn display_status(status: &str) -> String {
    match status {
        "not_started" => "Not Started".to_string(),
        "starting" => "Starting".to_string(),
        "running" => "Running".to_string(),
        "stopped" => "Stopped".to_string(),
        s if s.starts_with("failed") => {
            let mut chars = s.chars();
            match chars.next() {
                Some(first) => first.to_uppercase().to_string() + chars.as_str(),
                None => "Failed".to_string(),
            }
        }
        other => other.to_string(),
    }
}
