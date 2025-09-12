use crate::api_client::ApiClient;
use crate::config::Config;
use anyhow::Result;
use std::net::SocketAddr;
use std::sync::Arc;
use tokio::net::UdpSocket;

/// Simple UDP syslog server that forwards messages directly to HTTP API
/// Receives syslog messages via UDP and immediately forwards them to the log forwarding API
pub struct SyslogServer {
    /// Application configuration containing bind address and port
    config: Arc<Config>,
    /// HTTP client for forwarding logs to API
    api_client: Arc<ApiClient>,
}

impl SyslogServer {
    /// Creates a new syslog server with direct API forwarding
    /// 
    /// # Arguments
    /// * `config` - Application configuration for server binding
    /// * `api_client` - HTTP client for forwarding logs to API
    /// 
    /// # Returns
    /// * `Self` - New syslog server instance
    pub fn new(config: Arc<Config>, api_client: Arc<ApiClient>) -> Self {
        Self {
            config,
            api_client,
        }
    }

    /// Starts the UDP syslog server and runs the main message processing loop
    /// 
    /// # Returns
    /// * `Result<()>` - Never returns normally, only on error
    /// 
    /// # Behavior
    /// - Binds UDP socket to configured address and port
    /// - Runs infinite loop receiving UDP messages
    /// - Forwards each message immediately to HTTP API
    /// - Logs errors but continues processing other messages
    /// - Uses 8KB buffer for incoming syslog messages
    pub async fn run(&self) -> Result<()> {
        let bind_addr = format!("{}:{}", self.config.bind_address, self.config.syslog_port);
        log::debug!("Binding UDP socket to {}", bind_addr);
        
        let socket = UdpSocket::bind(&bind_addr).await?;
        log::info!("Syslog server listening on {}", bind_addr);

        let mut buf = vec![0u8; 8192]; // 8KB buffer for syslog messages

        loop {
            log::trace!("Waiting for UDP message...");
            match socket.recv_from(&mut buf).await {
                Ok((len, addr)) => {
                    let message = &buf[..len];
                    if let Err(e) = self.handle_syslog_message(message, addr).await {
                        log::error!("Error handling syslog message from {}: {}", addr, e);
                    }
                }
                Err(e) => {
                    log::error!("Error receiving UDP message: {}", e);
                }
            }
        }
    }

    /// Handles a single incoming syslog message by forwarding it to the API
    /// 
    /// # Arguments
    /// * `raw_message` - Raw UDP message bytes received from sender
    /// * `addr` - Source address of the UDP message
    /// 
    /// # Returns
    /// * `Result<()>` - Success or error if message processing/forwarding fails
    /// 
    /// # Behavior
    /// - Converts raw bytes to UTF-8 string (lossy conversion for invalid UTF-8)
    /// - Logs the received message at debug level
    /// - Immediately forwards to API client without buffering
    /// - Returns error if API forwarding fails (logged by caller)
    async fn handle_syslog_message(&self, raw_message: &[u8], addr: SocketAddr) -> Result<()> {
        let message_str = String::from_utf8_lossy(raw_message).to_string();
        log::debug!("Received syslog message from {}: {}", addr, message_str.trim());
        
        // Forward the raw syslog message directly to the API
        self.api_client.send_log(&message_str).await?;

        Ok(())
    }
}