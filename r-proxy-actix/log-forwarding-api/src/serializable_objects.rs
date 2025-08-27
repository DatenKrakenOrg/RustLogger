use serde::{Deserialize, Serialize};
use std::collections::HashMap;

/// Payload received from log_sender containing message type and CSV data
#[derive(Debug, Deserialize, Serialize)]
pub struct LogPayload {
    pub message_type: String,
    pub csv_line: String,
}

/// Configuration for message types (same as in rust-logger)
#[derive(Debug, Deserialize, Clone)]
pub struct MessageTypeConfig {
    pub name: String,
    pub index_name: String,
    pub description: String,
    pub fields: HashMap<String, toml::Value>,
    pub logic: Option<HashMap<String, toml::Value>>,
}

#[derive(Debug, Deserialize)]
pub struct ConfigFile {
    pub message_types: Vec<MessageTypeConfig>,
}
