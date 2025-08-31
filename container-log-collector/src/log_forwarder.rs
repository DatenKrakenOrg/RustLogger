use crate::api_client::ApiClient;
use crate::buffer_db::{BufferDb, BufferStats};
use crate::config::Config;
use anyhow::Result;
use std::sync::Arc;
use tokio::sync::mpsc;
use tokio::time::{Duration, interval};

pub struct LogForwarder {
    config: Arc<Config>,
    api_client: Arc<ApiClient>,
    buffer_db: Arc<BufferDb>,
    log_sender: mpsc::UnboundedSender<String>,
}

impl LogForwarder {
    pub async fn new(config: Arc<Config>, api_client: Arc<ApiClient>) -> Result<Self> {
        // Initialize SQLite buffer database
        let buffer_db = Arc::new(BufferDb::new(&config.buffer_db_path).await?);
        
        // Reset any logs stuck in 'processing' state from previous runs
        buffer_db.reset_processing_to_pending().await?;
        
        let (log_sender, log_receiver) = mpsc::unbounded_channel();
        
        let forwarder = Self {
            config: config.clone(),
            api_client: api_client.clone(),
            buffer_db: buffer_db.clone(),
            log_sender,
        };
        
        // Start background tasks
        tokio::spawn(Self::buffer_writer_task(
            buffer_db.clone(),
            log_receiver,
        ));
        
        tokio::spawn(Self::api_forwarder_task(
            config.clone(),
            api_client.clone(),
            buffer_db.clone(),
        ));
        
        tokio::spawn(Self::retry_task(
            config.clone(),
            api_client.clone(),
            buffer_db.clone(),
        ));
        
        tokio::spawn(Self::stats_task(
            config.clone(),
            buffer_db.clone(),
        ));
        
        tokio::spawn(Self::cleanup_task(
            config.clone(),
            buffer_db.clone(),
        ));
        
        Ok(forwarder)
    }

    pub async fn forward_log(&self, raw_syslog: String) -> Result<()> {
        if let Err(e) = self.log_sender.send(raw_syslog) {
            log::error!("Failed to queue log for buffering: {}", e);
            return Err(anyhow::anyhow!("Failed to queue log: {}", e));
        }
        Ok(())
    }

    // Task 1: Buffer Writer - Stores incoming logs to SQLite (always succeeds)
    async fn buffer_writer_task(
        buffer_db: Arc<BufferDb>,
        mut log_receiver: mpsc::UnboundedReceiver<String>,
    ) {
        log::info!("Started buffer writer task");
        
        while let Some(raw_syslog) = log_receiver.recv().await {
            if let Err(e) = buffer_db.store_log(&raw_syslog).await {
                log::error!("Failed to store log in buffer database: {}", e);
                // Continue processing other logs even if one fails
            }
        }
        
        log::info!("Buffer writer task stopped");
    }

    // Task 2: API Forwarder - Sends pending logs to API
    async fn api_forwarder_task(
        config: Arc<Config>,
        api_client: Arc<ApiClient>,
        buffer_db: Arc<BufferDb>,
    ) {
        log::info!("Started API forwarder task");
        let mut interval = interval(Duration::from_millis(config.batch_timeout_ms));
        
        loop {
            interval.tick().await;
            
            // Get pending logs from database
            let pending_logs = match buffer_db.get_pending_logs(config.batch_size as i32).await {
                Ok(logs) => logs,
                Err(e) => {
                    log::error!("Failed to fetch pending logs: {}", e);
                    continue;
                }
            };
            
            if pending_logs.is_empty() {
                continue;
            }
            
            log::debug!("Processing {} pending logs", pending_logs.len());
            
            // Process each log individually for better error handling
            for (log_id, raw_syslog) in pending_logs {
                // Mark as processing
                if let Err(e) = buffer_db.mark_processing(log_id).await {
                    log::warn!("Failed to mark log {} as processing: {}", log_id, e);
                    continue;
                }
                
                // Try to send to API
                match api_client.send_log(&raw_syslog).await {
                    Ok(()) => {
                        // Successfully sent, remove from database
                        if let Err(e) = buffer_db.mark_sent(log_id).await {
                            log::warn!("Failed to mark log {} as sent: {}", log_id, e);
                        }
                        log::trace!("Successfully forwarded log {}", log_id);
                    }
                    Err(e) => {
                        // Failed to send, mark as failed for retry
                        if let Err(db_e) = buffer_db.mark_failed(log_id).await {
                            log::error!("Failed to mark log {} as failed: {}", log_id, db_e);
                        } else {
                            log::debug!("Marked log {} as failed for retry: {}", log_id, e);
                        }
                    }
                }
            }
        }
    }

    // Task 3: Retry Handler - Retries failed logs after delay
    async fn retry_task(
        config: Arc<Config>,
        api_client: Arc<ApiClient>,
        buffer_db: Arc<BufferDb>,
    ) {
        log::info!("Started retry task");
        let mut interval = interval(Duration::from_secs(config.retry_delay_secs));
        interval.tick().await; // Skip first immediate tick
        
        loop {
            interval.tick().await;
            
            // Get logs eligible for retry
            let retry_logs = match buffer_db.get_retry_candidates(
                config.max_retries,
                config.retry_delay_secs as i64,
            ).await {
                Ok(logs) => logs,
                Err(e) => {
                    log::error!("Failed to fetch retry candidates: {}", e);
                    continue;
                }
            };
            
            if retry_logs.is_empty() {
                continue;
            }
            
            log::info!("Retrying {} failed logs", retry_logs.len());
            
            for (log_id, raw_syslog) in retry_logs {
                // Mark as processing
                if let Err(e) = buffer_db.mark_processing(log_id).await {
                    log::warn!("Failed to mark retry log {} as processing: {}", log_id, e);
                    continue;
                }
                
                // Try to send to API with more aggressive retry
                match api_client.send_log_with_retry(&raw_syslog, 3).await {
                    Ok(()) => {
                        // Successfully sent, remove from database
                        if let Err(e) = buffer_db.mark_sent(log_id).await {
                            log::warn!("Failed to mark retry log {} as sent: {}", log_id, e);
                        } else {
                            log::info!("Successfully retried and sent log {}", log_id);
                        }
                    }
                    Err(e) => {
                        // Failed again, mark as failed
                        if let Err(db_e) = buffer_db.mark_failed(log_id).await {
                            log::error!("Failed to mark retry log {} as failed: {}", log_id, db_e);
                        } else {
                            log::debug!("Retry failed for log {}: {}", log_id, e);
                        }
                    }
                }
            }
        }
    }

    // Task 4: Statistics Reporter - Logs buffer status periodically
    async fn stats_task(
        _config: Arc<Config>,
        buffer_db: Arc<BufferDb>,
    ) {
        log::info!("Started statistics task");
        let mut interval = interval(Duration::from_secs(60)); // Report every minute
        
        loop {
            interval.tick().await;
            
            match buffer_db.get_stats().await {
                Ok(stats) => {
                    if stats.pending_count > 0 || stats.failed_count > 0 {
                        log::info!(
                            "Buffer stats: {} pending, {} processing, {} failed, {} total retries{}",
                            stats.pending_count,
                            stats.processing_count,
                            stats.failed_count,
                            stats.total_retries,
                            if let Some(oldest) = stats.oldest_pending_timestamp {
                                format!(", oldest pending: {}s ago", 
                                    std::time::SystemTime::now()
                                        .duration_since(std::time::UNIX_EPOCH)
                                        .unwrap_or_default()
                                        .as_secs() as i64 - oldest)
                            } else {
                                String::new()
                            }
                        );
                    } else {
                        log::debug!("Buffer is empty, all logs forwarded successfully");
                    }
                }
                Err(e) => {
                    log::error!("Failed to get buffer statistics: {}", e);
                }
            }
        }
    }

    // Task 5: Cleanup - Removes old failed logs
    async fn cleanup_task(
        config: Arc<Config>,
        buffer_db: Arc<BufferDb>,
    ) {
        log::info!("Started cleanup task");
        let mut interval = interval(Duration::from_secs(3600)); // Cleanup every hour
        
        loop {
            interval.tick().await;
            
            match buffer_db.cleanup_old_failed(config.cleanup_failed_after_hours).await {
                Ok(deleted) => {
                    if deleted > 0 {
                        log::info!("Cleaned up {} old failed log entries", deleted);
                    }
                }
                Err(e) => {
                    log::error!("Failed to cleanup old failed logs: {}", e);
                }
            }
        }
    }

    pub async fn get_stats(&self) -> Result<BufferStats> {
        self.buffer_db.get_stats().await
    }
}