mod config;
mod syslog_server;
mod api_client;
mod log_forwarder;
mod buffer_db;

use anyhow::Result;
use clap::Parser;
use config::Config;
use api_client::ApiClient;
use log_forwarder::LogForwarder;
use syslog_server::SyslogServer;
use std::sync::Arc;
use tokio::signal;

#[derive(Parser)]
#[command(name = "container-log-collector")]
#[command(about = "A syslog-based container log collector for Docker and Podman")]
struct Args {
    #[arg(short, long, default_value = "config.env")]
    config: String,
}

#[tokio::main]
async fn main() -> Result<()> {
    env_logger::init();
    let args = Args::parse();
    
    log::info!("Starting Container Log Collector");
    
    let config = Arc::new(Config::load(&args.config)?);
    log::info!("Configuration loaded from: {}", args.config);
    
    let api_client = Arc::new(ApiClient::new(&config).await?);
    log::info!("Connected to log-forwarding-api at: {}", config.api_url);
    
    let log_forwarder = Arc::new(LogForwarder::new(config.clone(), api_client.clone()).await?);
    
    let syslog_server = SyslogServer::new(config.clone(), log_forwarder);
    
    log::info!("Starting syslog server on {}:{}", config.bind_address, config.syslog_port);
    
    tokio::select! {
        result = syslog_server.run() => {
            if let Err(e) = result {
                log::error!("Syslog server error: {}", e);
            }
        }
        _ = signal::ctrl_c() => {
            log::info!("Received shutdown signal, stopping server...");
        }
    }
    
    log::info!("Container Log Collector stopped");
    Ok(())
}