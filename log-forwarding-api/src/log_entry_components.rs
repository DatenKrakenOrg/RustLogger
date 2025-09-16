use serde::{Deserialize, Serialize};
#[derive(Debug, Deserialize, Serialize)]
#[serde(rename_all = "UPPERCASE")]
pub enum LogLevel {
    Critical,
    Warn,
    Info,
}

#[derive(Debug, Deserialize, Serialize)]
pub struct InnerMsg {
    device: String,
    msg: String,
    exceeded_values: Vec<bool>,
}
