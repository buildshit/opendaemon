use clap::{Parser, Subcommand};
use std::path::PathBuf;
use std::sync::Arc;
use tokio::sync::Mutex;

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
    },
    /// Start all services defined in the configuration
    Start {
        /// Path to dmn.json configuration file
        #[arg(short, long, default_value = "dmn.json")]
        config: PathBuf,
    },
    /// Stop all running services
    Stop {
        /// Path to dmn.json configuration file
        #[arg(short, long, default_value = "dmn.json")]
        config: PathBuf,
    },
    /// Show the status of all services
    Status {
        /// Path to dmn.json configuration file
        #[arg(short, long, default_value = "dmn.json")]
        config: PathBuf,
    },
}

#[tokio::main]
async fn main() {
    let cli = Cli::parse();

    let exit_code = match cli.command {
        Commands::Daemon { config } => run_daemon_mode(config).await,
        Commands::Mcp { config } => run_mcp_mode(config).await,
        Commands::Start { config } => run_start_command(config).await,
        Commands::Stop { config } => run_stop_command(config).await,
        Commands::Status { config } => run_status_command(config).await,
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
    let rpc_server = RpcServer::new(orchestrator);
    if let Err(e) = rpc_server.run().await {
        eprintln!("RPC server error: {}", e);
        return 1;
    }
    
    0
}

/// Run in MCP mode - Model Context Protocol server for AI agents
async fn run_mcp_mode(config_path: PathBuf) -> i32 {
    eprintln!("Starting MCP server mode with config: {:?}", config_path);
    
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

/// Start all services
async fn run_start_command(config_path: PathBuf) -> i32 {
    eprintln!("Starting services with config: {:?}", config_path);
    
    // Load configuration
    let dmn_config = match parse_config(&config_path) {
        Ok(cfg) => cfg,
        Err(e) => {
            eprintln!("Failed to load configuration: {}", e);
            return 1;
        }
    };
    
    // Create orchestrator
    let mut orchestrator = match Orchestrator::new(dmn_config) {
        Ok(orch) => orch,
        Err(e) => {
            eprintln!("Failed to create orchestrator: {}", e);
            return 1;
        }
    };
    
    // Start all services
    match orchestrator.start_all().await {
        Ok(_) => {
            eprintln!("All services started successfully");
            0
        }
        Err(e) => {
            eprintln!("Failed to start services: {}", e);
            1
        }
    }
}

/// Stop all services
async fn run_stop_command(config_path: PathBuf) -> i32 {
    eprintln!("Stopping services with config: {:?}", config_path);
    
    // Load configuration
    let dmn_config = match parse_config(&config_path) {
        Ok(cfg) => cfg,
        Err(e) => {
            eprintln!("Failed to load configuration: {}", e);
            return 1;
        }
    };
    
    // Create orchestrator
    let mut orchestrator = match Orchestrator::new(dmn_config) {
        Ok(orch) => orch,
        Err(e) => {
            eprintln!("Failed to create orchestrator: {}", e);
            return 1;
        }
    };
    
    // Stop all services
    match orchestrator.stop_all().await {
        Ok(_) => {
            eprintln!("All services stopped successfully");
            0
        }
        Err(e) => {
            eprintln!("Failed to stop services: {}", e);
            1
        }
    }
}

/// Show service status
async fn run_status_command(config_path: PathBuf) -> i32 {
    eprintln!("Checking status with config: {:?}", config_path);
    
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
        Ok(orch) => orch,
        Err(e) => {
            eprintln!("Failed to create orchestrator: {}", e);
            return 1;
        }
    };
    
    // Get and display status
    let statuses = orchestrator.process_manager.get_all_statuses();
    
    if statuses.is_empty() {
        eprintln!("No services defined in configuration");
        return 0;
    }
    
    eprintln!("\nService Status:");
    eprintln!("{:-<50}", "");
    
    for (service_name, status) in statuses {
        let status_str = match status {
            process::ServiceStatus::NotStarted => "Not Started",
            process::ServiceStatus::Starting => "Starting",
            process::ServiceStatus::Running => "Running",
            process::ServiceStatus::Stopped => "Stopped",
            process::ServiceStatus::Failed { exit_code } => {
                eprintln!("{:<30} Failed (exit code: {})", service_name, exit_code);
                continue;
            }
        };
        eprintln!("{:<30} {}", service_name, status_str);
    }
    
    0
}
