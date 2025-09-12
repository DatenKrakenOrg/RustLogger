use crate::config::Config;
use anyhow::{Context, Result};
use reqwest::Client;
use std::sync::Arc;
use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use syslog_loose::{parse_message,Variant};

/// JSON payload for sending a single log to the API
#[derive(Debug, Serialize)]
pub struct LogPayload {
    timestamp: DateTime<Utc>,
    container_name: String,
    log_message: String,
}


/// Simple HTTP client for forwarding syslog messages to the log forwarding API
/// Provides direct, synchronous forwarding without batching or retry logic
pub struct ApiClient {
    /// HTTP client for making requests
    client: Client,
    /// Application configuration containing API URL and credentials
    config: Arc<Config>,
}

impl ApiClient {
    /// Creates a new API client with HTTP timeout configured
    /// 
    /// # Arguments
    /// * `config` - Application configuration containing API URL and secret
    /// 
    /// # Returns
    /// * `Result<Self>` - New API client or error if HTTP client creation fails
    pub async fn new(config: &Config) -> Result<Self> {
        let client = Client::builder()
            .timeout(std::time::Duration::from_secs(30))
            .build()
            .context("Failed to create HTTP client")?;

        Ok(Self {
            client,
            config: Arc::new(config.clone()),
        })
    }

    /// Sends a single syslog message directly to the log forwarding API
    /// 
    /// # Arguments
    /// * `raw_syslog` - Raw syslog message string as received from UDP
    /// 
    /// # Returns
    /// * `Result<()>` - Success or error if HTTP request fails
    /// 
    /// # Behavior
    /// - Wraps syslog message in JSON payload 
    /// - Sends POST request to {api_url}/send_container_log endpoint
    /// - Includes X-Api-Key header for authentication
    /// - Logs errors but doesn't retry failed requests
    pub async fn send_log(&self, raw_syslog: &str) -> Result<()> {
        let syslog = parse_message(raw_syslog,Variant::RFC3164);
        let payload = LogPayload {
            timestamp :syslog.timestamp.unwrap().to_utc(),
            container_name: syslog.appname.expect("no hostname found").to_string(),
            log_message: syslog.msg.to_string(),
        };
        

        let url = format!("{}/send_container_log", self.config.api_url);
        let response = self
            .client
            .post(&url)
            .header("X-Api-Key", self.config.secret.clone())
            .json(&payload)
            .send()
            .await
            .context("Failed to send log to API")?;

        if !response.status().is_success() {
            let error_text = response.text().await.unwrap_or_else(|_| "Unknown error".to_string());
            log::error!("API request failed {}",error_text);
        } else {
            log::debug!("Successfully sent log to API");
        }

        Ok(())
    }
}
