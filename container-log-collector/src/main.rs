mod config;
mod syslog_server;
mod api_client;
mod log_forwarder;

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
    
    println!("Starting Container Log Collector");
    
    let config = Arc::new(Config::load(&args.config)?);
    println!("Configuration loaded from: {}", args.config);
    
    // Try to create API client but don't fail if it's unavailable at startup
    let api_client = ApiClient::new(&config).await?;
    
    let log_forwarder = Arc::new(LogForwarder::new(config.clone(), Arc::new(api_client).clone()).await?);
    
    let syslog_server = SyslogServer::new(config.clone(), log_forwarder);
    
    println!("Starting syslog server on {}:{}", config.bind_address, config.syslog_port);
    
    tokio::select! {
        result = syslog_server.run() => {
            if let Err(e) = result {
                println!("Syslog server error: {}", e);
            }
        }
        _ = signal::ctrl_c() => {
            println!("Received shutdown signal, stopping server...");
        }
    }
    
    println!("Container Log Collector stopped");
    Ok(())
}
