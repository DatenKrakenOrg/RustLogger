use serde::{Deserialize, Serialize};
use chrono::{DateTime, Utc};

#[derive(Debug, Deserialize, Serialize)]
#[serde(rename_all = "UPPERCASE")]
enum LogLevel {
    Critical,
    Warn,
    Info
}

#[derive(Debug, Deserialize, Serialize)]
struct InnerMsg {
    device: String,
    msg: String,
    exceeded_values: Vec<bool>,
}

#[derive(Debug, Deserialize, Serialize)]
pub struct LogEntry {
    timestamp: DateTime<Utc>,
    level: LogLevel,
    temperature: f64,
    humidity: f64,
    msg: InnerMsg,
}
