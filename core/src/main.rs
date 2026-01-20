use clap::{Parser, Subcommand};
use std::path::PathBuf;

#[derive(Parser)]
#[command(name = "dmn")]
#[command(about = "OpenDaemon - Local development service orchestrator", long_about = None)]
struct Cli {
    #[command(subcommand)]
    command: Commands,
}

#[derive(Subcommand)]
enum Commands {
    /// Run in daemon mode for VS Code extension
    Daemon {
        #[arg(short, long, default_value = "dmn.json")]
        config: PathBuf,
    },
    /// Run in MCP server mode for AI agents
    Mcp {
        #[arg(short, long, default_value = "dmn.json")]
        config: PathBuf,
    },
    /// Start all services
    Start {
        #[arg(short, long, default_value = "dmn.json")]
        config: PathBuf,
    },
    /// Stop all services
    Stop {
        #[arg(short, long, default_value = "dmn.json")]
        config: PathBuf,
    },
    /// Show service status
    Status {
        #[arg(short, long, default_value = "dmn.json")]
        config: PathBuf,
    },
}

#[tokio::main]
async fn main() {
    let cli = Cli::parse();

    match cli.command {
        Commands::Daemon { config } => {
            println!("Starting daemon mode with config: {:?}", config);
            // TODO: Implement daemon mode
        }
        Commands::Mcp { config } => {
            println!("Starting MCP server mode with config: {:?}", config);
            // TODO: Implement MCP mode
        }
        Commands::Start { config } => {
            println!("Starting services with config: {:?}", config);
            // TODO: Implement start command
        }
        Commands::Stop { config } => {
            println!("Stopping services with config: {:?}", config);
            // TODO: Implement stop command
        }
        Commands::Status { config } => {
            println!("Checking status with config: {:?}", config);
            // TODO: Implement status command
        }
    }
}
