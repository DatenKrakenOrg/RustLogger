use crate::config::Config;
use anyhow::{Context, Result};
use reqwest::Client;
use serde::{Deserialize, Serialize};
use std::sync::Arc;

#[derive(Debug, Serialize)]
pub struct LogPayload {
    pub message_type: String,
    pub csv_line: String,
}

#[derive(Debug, Deserialize)]
pub struct ApiResponse {
    pub result: String,
    pub message_type: String,
    pub index: String,
}

pub struct ApiClient {
    client: Client,
    config: Arc<Config>,
}

impl ApiClient {
    pub async fn new(config: &Config) -> Result<Self> {
        let client = Client::builder()
            .timeout(std::time::Duration::from_secs(30))
            .build()
            .context("Failed to create HTTP client")?;

        let api_client = Self {
            client,
            config: Arc::new(config.clone()),
        };

        // Don't test connection at startup, will retry on each request
        Ok(api_client)
    }

    async fn test_connection(&self) -> Result<()> {
        let url = format!("{}/whoareyou", self.config.api_url);
        let response = self
            .client
            .get(&url)
            .send()
            .await
            .context("Failed to connect to log-forwarding-api")?;

        if response.status().is_success() {
            log::info!("Successfully connected to log-forwarding-api");
            Ok(())
        } else {
            anyhow::bail!(
                "API health check failed: {}",
                response.status()
            )
        }
    }

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
            println!("Request failed {}", error_text);
        }

        Ok(())
    }


}
