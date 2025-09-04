use crate::config::Config;
use anyhow::{Context, Result};
use reqwest::Client;
use serde::Serialize;
use std::sync::Arc;

/// JSON payload for sending a single log to the API
#[derive(Debug, Serialize)]
pub struct LogPayload {
    /// Type of message being sent (always "container_logs" for this collector)
    pub message_type: String,
    /// Raw syslog message as received from UDP
    pub csv_line: String,
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
    /// - Wraps syslog message in JSON payload with message_type "container_logs"
    /// - Sends POST request to {api_url}/send_log endpoint
    /// - Includes X-Api-Key header for authentication
    /// - Logs errors but doesn't retry failed requests
    pub async fn send_log(&self, raw_syslog: &str) -> Result<()> {
        let payload = LogPayload {
            message_type: "container_logs".to_string(),
            csv_line: raw_syslog.to_string(),
        };

        let url = format!("{}/send_log", self.config.api_url);
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
