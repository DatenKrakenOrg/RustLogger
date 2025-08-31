use crate::config::Config;
use crate::log_forwarder::LogForwarder;
use anyhow::Result;
use std::net::SocketAddr;
use std::sync::Arc;
use tokio::net::UdpSocket;

pub struct SyslogServer {
    config: Arc<Config>,
    log_forwarder: Arc<LogForwarder>,
}

impl SyslogServer {
    pub fn new(config: Arc<Config>, log_forwarder: Arc<LogForwarder>) -> Self {
        Self {
            config,
            log_forwarder,
        }
    }

    pub async fn run(&self) -> Result<()> {
        let bind_addr = format!("{}:{}", self.config.bind_address, self.config.syslog_port);
        println!("Socket Binding ...");
        let socket = UdpSocket::bind(&bind_addr).await?;
        println!("Syslog server listening on {}", bind_addr);

        let mut buf = vec![0u8; 8192]; // 8KB buffer for syslog messages

        loop {
            println!("Waiting for UDP message...");
            match socket.recv_from(&mut buf).await {
                Ok((len, addr)) => {
                    let message = &buf[..len];
                    if let Err(e) = self.handle_syslog_message(message, addr).await {
                        println!("Error handling syslog message from {}: {}", addr, e);
                    }
                }
                Err(e) => {
                    println!("Error receiving UDP message: {}", e);
                }
            }
        }
    }

    async fn handle_syslog_message(&self, raw_message: &[u8], addr: SocketAddr) -> Result<()> {
        let message_str = String::from_utf8_lossy(raw_message).to_string();

        println!("Received syslog message from {}: {}", addr, message_str.trim());
        
        // Forward the raw syslog message directly to the API
        self.log_forwarder.forward_log(message_str).await?;

        Ok(())
    }
}
