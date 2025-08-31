use anyhow::Result;
use dotenvy::dotenv;
use std::env;

#[derive(Debug, Clone)]
pub struct Config {
    pub bind_address: String,
    pub syslog_port: u16,
    pub api_url: String,
    pub log_level: String,
    pub batch_size: usize,
    pub batch_timeout_ms: u64,
    pub buffer_db_path: String,
    pub max_retries: i32,
    pub retry_delay_secs: u64,
    pub cleanup_failed_after_hours: i64,
    pub secret: String,
}



impl Config {
    pub fn load(config_path: &str) -> Result<Self> {
        // Load the specified config file
        if std::path::Path::new(config_path).exists() {
            dotenvy::from_filename(config_path).ok();
        } else if env::var("DEPLOYMENT").unwrap_or_default() != "PROD" {
            // Fallback to default .env if config file doesn't exist
            dotenv().ok();
        }
        
        Ok(Self {
            bind_address: env::var("BIND_ADDRESS").unwrap_or_else(|_| "0.0.0.0".to_string()),
            syslog_port: env::var("SYSLOG_PORT")
                .unwrap_or_else(|_| "514".to_string())
                .parse()
                .unwrap_or(514),
            api_url: env::var("API_URL")
                .unwrap_or_else(|_| "http://localhost:8080".to_string()),
            log_level: env::var("LOG_LEVEL")
                .unwrap_or_else(|_| "info".to_string()),
            batch_size: env::var("BATCH_SIZE")
                .unwrap_or_else(|_| "100".to_string())
                .parse()
                .unwrap_or(100),
            batch_timeout_ms: env::var("BATCH_TIMEOUT_MS")
                .unwrap_or_else(|_| "5000".to_string())
                .parse()
                .unwrap_or(5000),
            buffer_db_path: env::var("BUFFER_DB_PATH")
                .unwrap_or_else(|_| "/var/lib/container-collector/buffer.db".to_string()),
            max_retries: env::var("MAX_RETRIES")
                .unwrap_or_else(|_| "5".to_string())
                .parse()
                .unwrap_or(5),
            retry_delay_secs: env::var("RETRY_DELAY_SECS")
                .unwrap_or_else(|_| "30".to_string())
                .parse()
                .unwrap_or(30),
            cleanup_failed_after_hours: env::var("CLEANUP_FAILED_AFTER_HOURS")
                .unwrap_or_else(|_| "24".to_string())
                .parse()
                .unwrap_or(24),
            secret: env::var("SECRET_API_KEY")
                .unwrap_or_else(|_| "123456".to_string()),
        })
    }
}
