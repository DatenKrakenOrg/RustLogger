use crate::api_client::ApiClient;
use crate::config::Config;
use anyhow::Result;
use std::sync::Arc;
use tokio::sync::mpsc;
use tokio::time::{interval, Duration};

pub struct LogForwarder {
    config: Arc<Config>,
    api_client: Arc<ApiClient>,
    log_sender: mpsc::UnboundedSender<String>,
}

impl LogForwarder {
    pub async fn new(config: Arc<Config>, api_client: Arc<ApiClient>) -> Result<Self> {
        let (log_sender, mut log_receiver) = mpsc::unbounded_channel();
        
        let forwarder = Self {
            config: config.clone(),
            api_client: api_client.clone(),
            log_sender,
        };
        
        // Start background task to process queued logs
        let api_client_clone = api_client.clone();
        let config_clone = config.clone();
        tokio::spawn(async move {
            let mut batch = Vec::new();
            let mut batch_timer = interval(Duration::from_millis(config_clone.batch_timeout_ms));
            
            loop {
                tokio::select! {
                    // Receive new log messages
                    msg = log_receiver.recv() => {
                        match msg {
                            Some(log) => {
                                log::debug!("Queued log message for processing");
                                batch.push(log);
                                
                                // Send immediately if batch is full
                                if batch.len() >= config_clone.batch_size {
                                    Self::send_batch(&api_client_clone, &mut batch).await;
                                }
                            }
                            None => {
                                log::info!("Log receiver channel closed, shutting down forwarder");
                                break;
                            }
                        }
                    }
                    
                    // Send batch on timeout
                    _ = batch_timer.tick() => {
                        if !batch.is_empty() {
                            log::debug!("Sending batch of {} logs due to timeout", batch.len());
                            Self::send_batch(&api_client_clone, &mut batch).await;
                        }
                    }
                }
            }
        });
        
        Ok(forwarder)
    }
    
    async fn send_batch(api_client: &ApiClient, batch: &mut Vec<String>) {
        if batch.is_empty() {
            return;
        }
        
        for log in batch.drain(..) {
            if let Err(e) = api_client.send_log(&log).await {
                log::error!("Failed to send log to API: {}", e);
            }
        }
    }

    pub async fn forward_log(&self, raw_syslog: String) -> Result<()> {
        if let Err(e) = self.log_sender.send(raw_syslog) {
            log::error!("Failed to queue log for processing: {}", e);
            return Err(anyhow::anyhow!("Failed to queue log: {}", e));
        }
        Ok(())
    }
}
