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

        // Try to test connection but don't fail if API is unavailable
        match api_client.test_connection().await {
            Ok(()) => {
                log::info!("Successfully connected to log-forwarding-api at: {}", config.api_url);
            }
            Err(e) => {
                log::warn!("API not available at startup ({}). Will retry on each request.", e);
            }
        }

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
        self.send_log_with_retry(raw_syslog, 3).await
    }

    pub async fn send_log_with_retry(&self, raw_syslog: &str, max_retries: u32) -> Result<()> {
        let mut last_error = None;
        
        for attempt in 1..=max_retries {
            match self.send_log_internal(raw_syslog).await {
                Ok(()) => {
                    if attempt > 1 {
                        log::debug!("API request succeeded on attempt {}", attempt);
                    }
                    return Ok(());
                }
                Err(e) => {
                    last_error = Some(e);
                    if attempt < max_retries {
                        let delay = std::time::Duration::from_millis(500 * attempt as u64);
                        log::trace!("API request failed (attempt {}), retrying in {:?}ms", attempt, delay.as_millis());
                        tokio::time::sleep(delay).await;
                    }
                }
            }
        }
        
        Err(last_error.unwrap())
    }

    async fn send_log_internal(&self, raw_syslog: &str) -> Result<()> {
        let payload = LogPayload {
            message_type: "container_logs".to_string(),
            csv_line: raw_syslog.to_string(),
        };

        let url = format!("{}/send_log", self.config.api_url);
        let response = self
            .client
            .post(&url)
            .json(&payload)
            .send()
            .await
            .context("Failed to send log to API")?;

        if !response.status().is_success() {
            let error_text = response.text().await.unwrap_or_else(|_| "Unknown error".to_string());
            anyhow::bail!("API request failed: {}", error_text);
        }

        Ok(())
    }

    pub async fn bulk_send_logs(&self, raw_syslogs: Vec<String>) -> Result<()> {
        if raw_syslogs.is_empty() {
            return Ok(());
        }

        // Send logs concurrently but limit concurrency to avoid overwhelming the API
        let semaphore = Arc::new(tokio::sync::Semaphore::new(10));
        let mut tasks = Vec::new();

        for raw_syslog in raw_syslogs {
            let client = self.client.clone();
            let api_url = self.config.api_url.clone();
            let permit = semaphore.clone().acquire_owned().await?;

            let task = tokio::spawn(async move {
                let _permit = permit;
                let payload = LogPayload {
                    message_type: "container_logs".to_string(),
                    csv_line: raw_syslog,
                };

                let url = format!("{}/send_log", api_url);
                let response = client
                    .post(&url)
                    .json(&payload)
                    .send()
                    .await
                    .context("Failed to send log to API")?;

                if !response.status().is_success() {
                    let error_text = response.text().await.unwrap_or_else(|_| "Unknown error".to_string());
                    anyhow::bail!("API request failed: {}", error_text);
                }

                Ok::<(), anyhow::Error>(())
            });

            tasks.push(task);
        }

        // Wait for all requests to complete
        let mut errors = Vec::new();
        for task in tasks {
            if let Err(e) = task.await? {
                errors.push(e);
            }
        }

        if !errors.is_empty() {
            log::warn!("Some bulk log requests failed: {} errors", errors.len());
            for error in &errors {
                log::debug!("Bulk send error: {}", error);
            }
            // Return the first error for now
            return Err(errors.into_iter().next().unwrap());
        }

        Ok(())
    }
}