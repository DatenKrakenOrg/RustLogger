use anyhow::Result;
use chrono::{DateTime, Utc};
use reqwest::Client;
use serde::{Deserialize, Serialize};

#[derive(Debug, Deserialize, Serialize, Clone)]
#[serde(rename_all = "UPPERCASE")]
pub enum LogLevel {
    Critical,
    Warn,
    Info,
}

#[derive(Debug, Deserialize, Serialize, Clone)]
pub struct InnerMsg {
    pub device: String,
    pub msg: String,
    pub exceeded_values: Vec<bool>,
}

#[derive(Debug, Deserialize, Serialize, Clone)]
pub struct LogEntry {
    pub timestamp: DateTime<Utc>,
    pub level: LogLevel,
    pub temperature: f64,
    pub humidity: f64,
    pub msg: InnerMsg,
}

#[derive(Debug, Deserialize, Serialize, Clone)]
pub struct ContainerLogEntry {
    pub timestamp: DateTime<Utc>,
    pub container_name: String,
    pub log_message: String,
}

#[derive(Debug, Deserialize)]
pub struct LogsResponse {
    pub logs: Vec<LogEntry>,
}

#[derive(Debug, Deserialize)]
pub struct ContainerLogsResponse {
    pub logs: Vec<ContainerLogEntry>,
}

pub struct ApiClient {
    client: Client,
    base_url: String,
    api_key: Option<String>,
}

impl ApiClient {
    /// Creates a new API client with the specified base URL.
    ///
    /// Initializes a new HTTP client instance configured to communicate with
    /// the log forwarding API. The client starts without authentication and
    /// requires an API key to be set before making requests.
    ///
    /// # Arguments
    ///
    /// * `base_url` - Base URL of the log forwarding API (e.g., "http://localhost:8080")
    ///
    /// # Returns
    ///
    /// A new `ApiClient` instance ready for configuration and use
    ///
    /// # Example
    ///
    /// ```rust
    /// let client = ApiClient::new("http://localhost:8080".to_string());
    /// ```
    pub fn new(base_url: String) -> Self {
        Self {
            client: Client::new(),
            base_url,
            api_key: None,
        }
    }

    /// Sets or clears the API authentication key.
    ///
    /// Configures the API key used for authenticating requests to the log
    /// forwarding API. The key is sent as an `X-API-Key` header with each request.
    ///
    /// # Arguments
    ///
    /// * `api_key` - Optional API key string. Pass `None` to clear authentication.
    ///
    /// # Example
    ///
    /// ```rust
    /// // Set an API key
    /// client.set_api_key(Some("your-api-key".to_string()));
    /// 
    /// // Clear the API key
    /// client.set_api_key(None);
    /// ```
    pub fn set_api_key(&mut self, api_key: Option<String>) {
        self.api_key = api_key;
    }

    /// Retrieves sensor logs from the API with optional filtering and pagination.
    ///
    /// Fetches log entries from the `/logs` endpoint with support for various
    /// filtering options including log level, device name, date ranges, and
    /// pagination parameters. All parameters are optional and can be combined.
    ///
    /// # Arguments
    ///
    /// * `limit` - Maximum number of logs to retrieve (default: server-defined)
    /// * `offset` - Number of logs to skip for pagination (default: 0)
    /// * `level` - Filter by log level ("CRITICAL", "WARN", "INFO")
    /// * `device` - Filter by device name (URL-encoded automatically)
    /// * `from` - Start of date range filter (RFC3339 format)
    /// * `to` - End of date range filter (RFC3339 format)
    ///
    /// # Returns
    ///
    /// `Ok(Vec<LogEntry>)` on success, containing the filtered log entries.
    /// Returns an error if the request fails or authentication is invalid.
    ///
    /// # Example
    ///
    /// ```rust
    /// // Fetch latest 50 critical logs
    /// let logs = client.fetch_logs(
    ///     Some(50),
    ///     Some(0),
    ///     Some("CRITICAL"),
    ///     None,
    ///     None,
    ///     None
    /// ).await?;
    /// ```
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

    /// Performs full-text search on sensor logs.
    ///
    /// Searches through sensor log content using the `/logs/search` endpoint.
    /// The search operates on message content, device names, and log levels
    /// with fuzzy matching capabilities provided by Elasticsearch.
    ///
    /// # Arguments
    ///
    /// * `query` - Search query string (URL-encoded automatically)
    /// * `limit` - Maximum number of results to return (default: server-defined)
    /// * `offset` - Number of results to skip for pagination (default: 0)
    ///
    /// # Returns
    ///
    /// `Ok(Vec<LogEntry>)` containing matching log entries sorted by relevance.
    /// Returns an error if the request fails or authentication is invalid.
    ///
    /// # Example
    ///
    /// ```rust
    /// // Search for temperature-related logs
    /// let logs = client.search_logs("temperature sensor", Some(100), Some(0)).await?;
    /// ```
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

    /// Performs full-text search on container logs.
    ///
    /// Searches through container log content using the `/container-logs/search` endpoint.
    /// The search operates on log messages and container names with fuzzy matching
    /// capabilities provided by Elasticsearch.
    ///
    /// # Arguments
    ///
    /// * `query` - Search query string (URL-encoded automatically)
    /// * `limit` - Maximum number of results to return (default: server-defined)
    /// * `offset` - Number of results to skip for pagination (default: 0)
    ///
    /// # Returns
    ///
    /// `Ok(Vec<ContainerLogEntry>)` containing matching container log entries
    /// sorted by relevance. Returns an error if the request fails or authentication is invalid.
    ///
    /// # Example
    ///
    /// ```rust
    /// // Search for error logs from web containers
    /// let logs = client.search_container_logs("error web", Some(50), Some(0)).await?;
    /// ```
    pub async fn search_container_logs(
        &self,
        query: &str,
        limit: Option<usize>,
        offset: Option<usize>,
    ) -> Result<Vec<ContainerLogEntry>> {
        let mut url = format!("{}/container-logs/search", self.base_url);
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
        let logs_response: ContainerLogsResponse = response.json().await?;
        Ok(logs_response.logs)
    }

    /// Retrieves container logs from the API with optional filtering and pagination.
    ///
    /// Fetches container log entries from the `/container-logs` endpoint with support
    /// for filtering by container name, date ranges, and pagination parameters.
    /// All parameters are optional and can be combined.
    ///
    /// # Arguments
    ///
    /// * `limit` - Maximum number of logs to retrieve (default: server-defined)
    /// * `offset` - Number of logs to skip for pagination (default: 0)
    /// * `container_name` - Filter by specific container name (URL-encoded automatically)
    /// * `from` - Start of date range filter (RFC3339 format)
    /// * `to` - End of date range filter (RFC3339 format)
    ///
    /// # Returns
    ///
    /// `Ok(Vec<ContainerLogEntry>)` on success, containing the filtered container log entries.
    /// Returns an error if the request fails or authentication is invalid.
    ///
    /// # Filtering Options
    ///
    /// - **Container name**: Exact match filtering for specific containers
    /// - **Date range**: Timestamp-based filtering with RFC3339 dates
    /// - **Pagination**: Limit and offset for handling large datasets
    ///
    /// # Example
    ///
    /// ```rust
    /// // Fetch latest 100 logs from "web-server" container
    /// let logs = client.fetch_container_logs(
    ///     Some(100),
    ///     Some(0),
    ///     Some("web-server"),
    ///     None,
    ///     None
    /// ).await?;
    /// ```
    pub async fn fetch_container_logs(
        &self,
        limit: Option<usize>,
        offset: Option<usize>,
        container_name: Option<&str>,
        from: Option<DateTime<Utc>>,
        to: Option<DateTime<Utc>>,
    ) -> Result<Vec<ContainerLogEntry>> {
        let mut url = format!("{}/container-logs", self.base_url);
        let mut params = Vec::new();

        if let Some(limit) = limit {
            params.push(format!("limit={}", limit));
        }
        if let Some(offset) = offset {
            params.push(format!("offset={}", offset));
        }
        if let Some(container_name) = container_name {
            params.push(format!("container_name={}", urlencoding::encode(container_name)));
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
        let logs_response: ContainerLogsResponse = response.json().await?;
        Ok(logs_response.logs)
    }
}
