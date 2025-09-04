mod config;
mod syslog_server;
mod api_client;

use anyhow::Result;
use clap::Parser;
use config::Config;
use api_client::ApiClient;
use syslog_server::SyslogServer;
use std::sync::Arc;
use tokio::signal;

/// Command-line arguments for the container log collector
#[derive(Parser)]
#[command(name = "container-log-collector")]
#[command(about = "A simple syslog-to-HTTP forwarder for container logs")]
struct Args {
    /// Path to configuration file (.env format)
    #[arg(short, long, default_value = "config.env")]
    config: String,
}

/// Main entry point for the container log collector
/// 
/// # Behavior
/// - Initializes logging with env_logger
/// - Loads configuration from specified file or environment
/// - Creates HTTP client for API communication
/// - Starts UDP syslog server
/// - Runs until SIGINT/SIGTERM received
/// - Provides clean shutdown handling
#[tokio::main]
async fn main() -> Result<()> {
    env_logger::init();
    let args = Args::parse();
    
    log::info!("Starting Container Log Collector");
    
    // Load configuration from file or environment variables
    let config = Arc::new(Config::load(&args.config)?);
    log::info!("Configuration loaded from: {}", args.config);
    
    // Create HTTP client for API communication
    let api_client = Arc::new(ApiClient::new(&config).await?);
    log::info!("API client created for: {}", config.api_url);
    
    // Create and start the syslog server
    let syslog_server = SyslogServer::new(config.clone(), api_client);
    log::info!("Starting syslog server on {}:{}", config.bind_address, config.syslog_port);
    
    // Run server until shutdown signal received
    tokio::select! {
        result = syslog_server.run() => {
            if let Err(e) = result {
                log::error!("Syslog server error: {}", e);
                return Err(e);
            }
        }
        _ = signal::ctrl_c() => {
            log::info!("Received shutdown signal, stopping server...");
        }
    }
    
    log::info!("Container Log Collector stopped");
    Ok(())
}