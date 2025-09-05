use anyhow::Result;
use chrono::{DateTime, Utc};
use reqwest::Client;
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
    pub device: String,
    pub msg: String,
    pub exceeded_values: Vec<bool>,
}

#[derive(Debug, Deserialize, Serialize)]
pub struct LogEntry {
    pub timestamp: DateTime<Utc>,
    pub level: LogLevel,
    pub temperature: f64,
    pub humidity: f64,
    pub msg: InnerMsg,
}

#[derive(Debug, Deserialize)]
pub struct LogsResponse {
    pub logs: Vec<LogEntry>,
}

pub struct ApiClient {
    client: Client,
    base_url: String,
    api_key: Option<String>,
}

impl ApiClient {
    pub fn new(base_url: String) -> Self {
        Self {
            client: Client::new(),
            base_url,
            api_key: None,
        }
    }

    pub fn set_api_key(&mut self, api_key: Option<String>) {
        self.api_key = api_key;
    }

    pub async fn fetch_logs(
        &self,
        limit: Option<usize>,
        offset: Option<usize>,
        level: Option<&str>,
        device: Option<&str>,
        from: Option<DateTime<Utc>>,
        to: Option<DateTime<Utc>>,
    ) -> Result<Vec<LogEntry>> {
        let mut url = format!("{}/logs", self.base_url);
        let mut params = Vec::new();

        if let Some(limit) = limit {
            params.push(format!("limit={}", limit));
        }
        if let Some(offset) = offset {
            params.push(format!("offset={}", offset));
        }
        if let Some(level) = level {
            params.push(format!("level={}", level));
        }
        if let Some(device) = device {
            params.push(format!("device={}", urlencoding::encode(device)));
        }
        if let Some(from) = from {
            params.push(format!("from={}", from.to_rfc3339()));
        }
        if let Some(to) = to {
            params.push(format!("to={}", to.to_rfc3339()));
        }

        if !params.is_empty() {
        url.push('?');
        url.push_str(&params.join("&"));
    }

    let mut request = self.client.get(&url);
    
    if let Some(ref api_key) = self.api_key {
        request = request.header("X-API-Key", api_key);
    }
    
    let response = request.send().await?;
    let logs_response: LogsResponse = response.json().await?;
    Ok(logs_response.logs)
    }

    pub async fn search_logs(
        &self,
        query: &str,
        limit: Option<usize>,
        offset: Option<usize>,
    ) -> Result<Vec<LogEntry>> {
        let mut url = format!("{}/logs/search", self.base_url);
        let mut params = vec![format!("query={}", urlencoding::encode(query))];

        if let Some(limit) = limit {
            params.push(format!("limit={}", limit));
        }
        if let Some(offset) = offset {
            params.push(format!("offset={}", offset));
        }

        url.push('?');
        url.push_str(&params.join("&"));

        let mut request = self.client.get(&url);
        
        if let Some(ref api_key) = self.api_key {
            request = request.header("X-API-Key", api_key);
        }
        
        let response = request.send().await?;
        let logs_response: LogsResponse = response.json().await?;
        Ok(logs_response.logs)
    }
}
