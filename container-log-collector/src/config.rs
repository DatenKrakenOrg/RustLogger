use anyhow::Result;
use dotenvy::dotenv;
use std::env;

/// Configuration for the container log collector
/// Loads settings from environment variables with sensible defaults
#[derive(Debug, Clone)]
pub struct Config {
    /// Address to bind the syslog UDP server to (default: "0.0.0.0")
    pub bind_address: String,
    /// UDP port for the syslog server (default: 514)
    pub syslog_port: u16,
    /// HTTP URL of the log forwarding API (default: "http://localhost:8080")
    pub api_url: String,
    /// Secret API key for authentication
    pub secret: String,
}

impl Config {
    /// Loads configuration from environment variables
    /// 
    /// # Arguments
    /// * `config_path` - Path to .env file to load (falls back to default .env)
    /// 
    /// # Returns
    /// * `Result<Self>` - Configuration struct or error if loading fails
    /// 
    /// # Environment Variables
    /// * `BIND_ADDRESS` - Server bind address (default: "0.0.0.0")
    /// * `SYSLOG_PORT` - UDP port for syslog server (default: 514)
    /// * `API_URL` - HTTP URL of log forwarding API (default: "http://localhost:8080")
    /// * `SECRET_API_KEY` - API authentication key (default: "123456")
    pub fn load(config_path: &str) -> Result<Self> {
        // Load the specified config file
        if std::path::Path::new(config_path).exists() {
            dotenvy::from_filename(config_path).ok();
        } else if env::var("DEPLOYMENT").unwrap_or_default() != "PROD" {
            // Fallback to default .env if config file doesn't exist
            dotenv().ok();
        }
        
        Ok(Self {
            bind_address: env::var("BIND_ADDRESS").expect("BIND_ADDRESS must be set"),
            syslog_port: env::var("SYSLOG_PORT").unwrap().parse().expect("SYSLOG_PORT must be set and a number"),
            api_url: env::var("API_URL").expect("API_URL must be set"),
            secret: env::var("SECRET_API_KEY").expect("SECRET_API_KEY must be set")
        })
    }
}