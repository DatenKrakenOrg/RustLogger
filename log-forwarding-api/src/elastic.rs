use crate::log_entry::{ElasticLogDocument, LogEntry, ContainerLogEntry};
use crate::query_structures::{LogQuery, SearchQuery, ContainerLogQuery, ContainerSearchQuery};
use crate::server_error::ServerError;
use actix_web::http::StatusCode;
use elasticsearch::{
    Elasticsearch, IndexParts, SearchParts,
    auth::Credentials,
    http::transport::{SingleNodeConnectionPool, TransportBuilder},
    indices::{IndicesCreateParts, IndicesExistsParts},
};
//use env_logger::builder;
use serde::Serialize;
use serde_json::{Value, json};
use std::env;
use std::result::Result::Ok;
use url::Url;

/// Creates a elastic search client
///
/// # Examples
/// ```
/// let client: Elasticsearch = create_client()?;
/// ```
pub fn create_client() -> Result<Elasticsearch, ServerError> {
    let username: String = env::var("ELASTIC_USERNAME").map_err(|_| ServerError {
        code: StatusCode::INTERNAL_SERVER_ERROR,
        message: String::from("Username for elastic search authentication not set"),
        additional_information: String::from("Set ELASTIC_USERNAME in .env / env variables!"),
    })?;
    let password: String = env::var("ELASTIC_PASSWORD").map_err(|_| ServerError {
        code: StatusCode::INTERNAL_SERVER_ERROR,
        message: String::from("Password for elastic search authentication not set"),
        additional_information: String::from("Set ELASTIC_PASSWORD in .env / env variables!"),
    })?;
    let str_url: String = env::var("ELASTIC_URL").map_err(|_| ServerError {
        code: StatusCode::INTERNAL_SERVER_ERROR,
        message: String::from("URL for elastic search authentication not set"),
        additional_information: String::from("Set ELASTIC_URL in .env / env variables!"),
    })?;

    // Parse URL with proper scheme detection
    let url: Url = Url::parse(&str_url).map_err(|e| ServerError {
        code: StatusCode::INTERNAL_SERVER_ERROR,
        message: String::from("Error while parsing URL via Url Crate!"),
        additional_information: e.to_string(),
    })?;

    let pool: SingleNodeConnectionPool = SingleNodeConnectionPool::new(url);

    //Since of a local project we disable cert and only use basic authentication
    let transport = TransportBuilder::new(pool)
        .auth(Credentials::Basic(username, password))
        .disable_proxy()
        .cert_validation(elasticsearch::cert::CertificateValidation::None)
        .build()
        .map_err(|e| ServerError {
            code: StatusCode::GATEWAY_TIMEOUT,
            message: String::from("Error while creating elastic search client!"),
            additional_information: e.to_string(),
        })?;

    Ok(Elasticsearch::new(transport))
}

/// Creates the index used for the common log gen logs in elastic search based on the cluster on the client passed
///
/// # Examples:
/// ```
///     let client: Elasticsearch = create_client()?;
///    let index_name: String = env::var("INDEX_NAME")?;
///
///    // Creates a index if missing, otherwise returns
///    create_logs_index(
///        &index_name,
///        &client,
///    )
///    .await?;
/// ```
pub async fn create_logs_index(
    index_name: &str,
    connector: &Elasticsearch,
    mapping: Value,
) -> Result<String, ServerError> {
    // Get index settings from environment variables with defaults
    let replicas: u32 = env::var("ELASTIC_INDEX_REPLICAS")
        .unwrap_or_else(|_| "1".to_string())
        .parse()
        .unwrap_or(1);

    let shards: u32 = env::var("ELASTIC_INDEX_SHARDS")
        .unwrap_or_else(|_| "1".to_string())
        .parse()
        .unwrap_or(1);

    // Check if index exists
    let exists = connector
        .indices()
        .exists(IndicesExistsParts::Index(&[index_name]))
        .send()
        .await
        .map_err(|e| ServerError {
            code: StatusCode::GATEWAY_TIMEOUT,
            message: String::from("Index existance check failed!"),
            additional_information: e.to_string(),
        })?;

    if exists.status_code().is_success() {
        return Ok(format!("Index '{}' already exists", index_name));
    }

    //If not create one with a mapping matching the log
    connector
        .indices()
        .create(IndicesCreateParts::Index(index_name))
        .body(json!({
                "settings": {
                    "number_of_replicas": replicas,
                    "number_of_shards": shards
                },
                "mappings": mapping
        }))
        .send()
        .await
        .map_err(|e| ServerError {
            code: StatusCode::GATEWAY_TIMEOUT,
            message: String::from("Index creation failed!"),
            additional_information: e.to_string(),
        })?;

    Ok(format!("Index '{}' created successfully", index_name))
}

/// Persists a document in Elasticsearch for any log type that implements the required traits.
///
/// This function is generic over log types and handles the serialization and indexing
/// process. It converts the log entry to a JSON document and sends it to the specified
/// Elasticsearch index.
///
/// # Parameters
/// * `index_name` - The name of the Elasticsearch index to store the document in
/// * `client` - Reference to the configured Elasticsearch client
/// * `log_entry` - The log entry to persist
///
/// # Returns
/// * `Ok(String)` - Success message with the inserted log entry in JSON format
/// * `Err(ServerError)` - Error if serialization, network communication, or indexing fails
///
/// # Examples
/// ```rust
/// let client = create_client()?;
/// let log = LogEntry::new(/* ... */);
/// let result = send_document("sensor_logs", &client, &log).await?;
/// println!("{}", result); // "Log entry inserted: {...}"
/// ```
pub async fn send_document<T>(
    index_name: &str,
    client: &Elasticsearch,
    log_entry: &T,
) -> Result<String, ServerError>
where
    T: ElasticLogDocument + Serialize,
{
    let json_value = log_entry.to_document_json().map_err(|e| ServerError {
        code: StatusCode::INTERNAL_SERVER_ERROR,
        message: String::from("Error while serializing log entry to JSON"),
        additional_information: e.to_string(),
    })?;

    let response = client
        .index(IndexParts::Index(index_name))
        .body(json_value)
        .send()
        .await
        .map_err(|e| ServerError {
            code: StatusCode::GATEWAY_TIMEOUT,
            message: String::from("Index creation failed!"),
            additional_information: e.to_string(),
        })?;

    response.error_for_status_code().map_err(|e| ServerError {
        code: StatusCode::INTERNAL_SERVER_ERROR,
        message: String::from("Index creation failed!"),
        additional_information: e.to_string(),
    })?;

    Ok(format!(
        "Log entry inserted: {}",
        serde_json::to_string_pretty(log_entry).map_err(|e| ServerError {
            code: StatusCode::INTERNAL_SERVER_ERROR,
            message: String::from("Error while parsing log entry into json!"),
            additional_information: e.to_string(),
        })?
    ))
}

/// Retrieves information about all nodes in the Elasticsearch cluster.
///
/// This function queries the Elasticsearch cluster for detailed information about
/// all active nodes, including their roles, versions, and operational status.
/// The response contains comprehensive cluster topology information.
///
/// # Parameters
/// * `client` - Reference to the configured Elasticsearch client
///
/// # Returns
/// * `Ok(String)` - JSON string containing detailed node information for all cluster nodes
/// * `Err(ServerError)` - Error if the request fails or response parsing fails
///
/// # Examples
/// ```rust
/// let client = create_client()?;
/// let nodes_info = get_nodes(&client).await?;
/// // Returns detailed JSON with node IDs, names, roles, versions, etc.
/// ```
pub async fn get_nodes(client: &Elasticsearch) -> Result<String, ServerError> {
    let result = client
        .nodes()
        .info(elasticsearch::nodes::NodesInfoParts::None)
        .send()
        .await
        .map_err(|e| ServerError {
            code: StatusCode::GATEWAY_TIMEOUT,
            message: String::from("Fetching Node Information failed!"),
            additional_information: e.to_string(),
        })?
        .text()
        .await
        .map_err(|e| ServerError {
            code: StatusCode::INTERNAL_SERVER_ERROR,
            message: String::from("Error while parsing node information!"),
            additional_information: e.to_string(),
        })?;

    Ok(result)
}

/// Creates the Elasticsearch mapping schema for sensor log entries.
///
/// This function defines the field mappings and data types for sensor logs in Elasticsearch.
///
/// # Mapping Structure
/// * `timestamp` - Date field with RFC3339/ISO-8601 format support
/// * `level` - Keyword field for log levels (INFO, ERROR, WARN, etc.)
/// * `temperature` - Float field for temperature sensor readings
/// * `humidity` - Float field for humidity sensor readings  
/// * `msg.device` - Keyword field for device identification
/// * `msg.msg` - Text field with standard analyzer for message content
/// * `msg.exceeded_values` - Boolean field indicating threshold violations
///
/// # Returns
/// * `Value` - JSON object containing the complete mapping definition
///
/// # Examples
/// ```rust
/// let mapping = create_log_mapping();
/// create_logs_index("sensor_logs", &client, mapping).await?;
/// ```
pub fn create_log_mapping() -> Value {
    json!({
        "properties": {
            "timestamp": {
                "type": "date",
                // RFC3339/ISO-8601 format => Parseable by chrono
                "format": "strict_date_optional_time||epoch_millis"
            },
            "level": { "type": "keyword" },
            "temperature": { "type": "float" },
            "humidity": { "type": "float" },
            "msg": {
                "properties": {
                    "device": { "type": "keyword" },
                    "msg": { "type": "text", "analyzer": "standard" },
                    "exceeded_values": { "type": "boolean" }
                }
            }
        }
    })
}

/// Creates the Elasticsearch mapping schema for container log entries.
///
/// This function defines the field mappings and data types for container logs in Elasticsearch.
///
/// # Mapping Structure
/// * `timestamp` - Date field with RFC3339/ISO-8601 format support for temporal queries
/// * `container_name` - Keyword field for exact container name matching and filtering
/// * `log_message` - Text field with standard analyzer for full-text search capabilities
///
/// # Returns
/// * `Value` - JSON object containing the complete mapping definition for container logs
///
/// # Examples
/// ```rust
/// let mapping = create_container_log_mapping();
/// create_logs_index("container_logs", &client, mapping).await?;
/// ```
pub fn create_container_log_mapping() -> Value {
    json!({
        "properties" : {
            "timestamp": {
                "type": "date",
                // RFC3339/ISO-8601 format => Parseable by chrono
                "format": "strict_date_optional_time||epoch_millis"
            },
            "container_name": { "type": "keyword" },
            "log_message": { "type": "text", "analyzer": "standard"  },
        }
    })
}

/// Queries container logs from Elasticsearch with filtering capabilities.
///
/// This function performs structured queries on container logs with support for filtering
/// by container name and time range. Results are sorted by timestamp in descending order
/// (newest first) and support pagination.
///
/// # Parameters
/// * `index_name` - The name of the Elasticsearch index containing container logs
/// * `client` - Reference to the configured Elasticsearch client
/// * `query` - Container log query parameters including filters and pagination
///
/// # Query Filters
/// * `container_name` - Filter logs by specific container name (exact match)
/// * `from`/`to` - Time range filter using DateTime<Utc> boundaries
/// * `limit` - Maximum number of results to return (default: 100)
/// * `offset` - Number of results to skip for pagination (default: 0)
///
/// # Returns
/// * `Ok(Vec<ContainerLogEntry>)` - List of matching container log entries
/// * `Err(ServerError)` - Error if query execution or response parsing fails
///
/// # Examples
/// ```rust
/// let query = ContainerLogQuery {
///     container_name: Some("web-server".to_string()),
///     from: Some(yesterday),
///     to: Some(now),
///     limit: Some(50),
///     offset: Some(0),
/// };
/// let logs = query_container_logs("container_logs", &client, &query).await?;
/// ```
pub async fn query_container_logs(
    index_name: &str,
    client: &Elasticsearch,
    query: &ContainerLogQuery,
) -> Result<Vec<ContainerLogEntry>, ServerError> {
    let mut must_clauses = Vec::new();
    
    if let Some(container_name) = &query.container_name {
        must_clauses.push(json!({
            "term": { "container_name": container_name }
        }));
    }
    
    if query.from.is_some() || query.to.is_some() {
        let mut range_query = json!({ "range": { "timestamp": {} } });
        if let Some(from) = query.from {
            range_query["range"]["timestamp"]["gte"] = json!(from.to_rfc3339());
        }
        if let Some(to) = query.to {
            range_query["range"]["timestamp"]["lte"] = json!(to.to_rfc3339());
        }
        must_clauses.push(range_query);
    }
    
    let search_body = if must_clauses.is_empty() {
        json!({
            "query": { "match_all": {} },
            "sort": [{ "timestamp": { "order": "desc" } }],
            "size": query.limit.unwrap_or(100),
            "from": query.offset.unwrap_or(0)
        })
    } else {
        json!({
            "query": { "bool": { "must": must_clauses } },
            "sort": [{ "timestamp": { "order": "desc" } }],
            "size": query.limit.unwrap_or(100),
            "from": query.offset.unwrap_or(0)
        })
    };
    
    let response = client
        .search(SearchParts::Index(&[index_name]))
        .body(search_body)
        .send()
        .await
        .map_err(|e| ServerError {
            code: StatusCode::GATEWAY_TIMEOUT,
            message: String::from("Search request failed"),
            additional_information: e.to_string(),
        })?;
        
    let response_body: Value = response
        .json()
        .await
        .map_err(|e| ServerError {
            code: StatusCode::INTERNAL_SERVER_ERROR,
            message: String::from("Failed to parse search response"),
            additional_information: e.to_string(),
        })?;
        
    let hits = response_body["hits"]["hits"]
        .as_array()
        .ok_or_else(|| ServerError {
            code: StatusCode::INTERNAL_SERVER_ERROR,
            message: String::from("Invalid search response format"),
            additional_information: String::from("Expected hits array in response"),
        })?;
        
    let mut logs = Vec::new();
    for hit in hits {
        if let Some(source) = hit["_source"].as_object() {
            let log_entry: ContainerLogEntry = serde_json::from_value(json!(source))
                .map_err(|e| ServerError {
                    code: StatusCode::INTERNAL_SERVER_ERROR,
                    message: String::from("Failed to deserialize container log entry"),
                    additional_information: e.to_string(),
                })?;
            logs.push(log_entry);
        }
    }
    
    Ok(logs)
}

/// Performs full-text search on container logs using multi-field matching.
///
/// This function executes fuzzy full-text search across container log fields with
/// automatic relevance scoring. It searches both log message content and container
/// names, providing flexible search capabilities with automatic typo tolerance.
///
/// # Parameters
/// * `index_name` - The name of the Elasticsearch index containing container logs
/// * `client` - Reference to the configured Elasticsearch client  
/// * `search` - Container search query parameters including search terms and pagination
///
/// # Returns
/// * `Ok(Vec<ContainerLogEntry>)` - List of matching container log entries ordered by relevance and timestamp
/// * `Err(ServerError)` - Error if search execution or response parsing fails
///
/// # Examples
/// ```rust
/// let search = ContainerSearchQuery {
///     query: "error database connection".to_string(),
///     limit: Some(25),
///     offset: Some(0),
/// };
/// let logs = search_container_logs("container_logs", &client, &search).await?;
/// ```
pub async fn search_container_logs(
    index_name: &str,
    client: &Elasticsearch,
    search: &ContainerSearchQuery,
) -> Result<Vec<ContainerLogEntry>, ServerError> {
    let search_body = json!({
        "query": {
            "multi_match": {
                "query": search.query,
                "fields": ["log_message", "container_name"],
                "type": "best_fields",
                "fuzziness": "AUTO"
            }
        },
        "sort": [{ "timestamp": { "order": "desc" } }],
        "size": search.limit.unwrap_or(100),
        "from": search.offset.unwrap_or(0)
    });
    
    let response = client
        .search(SearchParts::Index(&[index_name]))
        .body(search_body)
        .send()
        .await
        .map_err(|e| ServerError {
            code: StatusCode::GATEWAY_TIMEOUT,
            message: String::from("Search request failed"),
            additional_information: e.to_string(),
        })?;
        
    let response_body: Value = response
        .json()
        .await
        .map_err(|e| ServerError {
            code: StatusCode::INTERNAL_SERVER_ERROR,
            message: String::from("Failed to parse search response"),
            additional_information: e.to_string(),
        })?;
        
    let hits = response_body["hits"]["hits"]
        .as_array()
        .ok_or_else(|| ServerError {
            code: StatusCode::INTERNAL_SERVER_ERROR,
            message: String::from("Invalid search response format"),
            additional_information: String::from("Expected hits array in response"),
        })?;
        
    let mut logs = Vec::new();
    for hit in hits {
        if let Some(source) = hit["_source"].as_object() {
            let log_entry: ContainerLogEntry = serde_json::from_value(json!(source))
                .map_err(|e| ServerError {
                    code: StatusCode::INTERNAL_SERVER_ERROR,
                    message: String::from("Failed to deserialize container log entry"),
                    additional_information: e.to_string(),
                })?;
            logs.push(log_entry);
        }
    }
    
    Ok(logs)
}

/// Queries sensor logs from Elasticsearch with comprehensive filtering capabilities.
///
/// This function performs structured queries on sensor logs with support for filtering
/// by log level, device name, and time range. It's designed for querying structured
/// sensor data with temperature and humidity readings.
///
/// # Parameters
/// * `index_name` - The name of the Elasticsearch index containing sensor logs
/// * `client` - Reference to the configured Elasticsearch client
/// * `query` - Sensor log query parameters including filters and pagination
///
/// # Query Filters
/// * `level` - Filter by log level (INFO, ERROR, WARN, etc.) - case insensitive, stored as uppercase
/// * `device` - Filter logs by specific device identifier (exact match)
/// * `from`/`to` - Time range filter using DateTime<Utc> boundaries
/// * `limit` - Maximum number of results to return (default: 100)
/// * `offset` - Number of results to skip for pagination (default: 0)
///
/// # Returns
/// * `Ok(Vec<LogEntry>)` - List of matching sensor log entries sorted by timestamp (newest first)
/// * `Err(ServerError)` - Error if query execution or response parsing fails
///
/// # Examples
/// ```rust
/// let query = LogQuery {
///     level: Some("error".to_string()),
///     device: Some("sensor-01".to_string()),
///     from: Some(yesterday),
///     to: Some(now),
///     limit: Some(100),
///     offset: Some(0),
/// };
/// let logs = query_logs("sensor_logs", &client, &query).await?;
/// ```
pub async fn query_logs(
    index_name: &str,
    client: &Elasticsearch,
    query: &LogQuery,
) -> Result<Vec<LogEntry>, ServerError> {
    let mut must_clauses = Vec::new();
    
    if let Some(level) = &query.level {
        must_clauses.push(json!({
            "term": { "level": level.to_uppercase() }
        }));
    }
    
    if let Some(device) = &query.device {
        must_clauses.push(json!({
            "term": { "msg.device": device }
        }));
    }
    
    if query.from.is_some() || query.to.is_some() {
        let mut range_query = json!({ "range": { "timestamp": {} } });
        if let Some(from) = query.from {
            range_query["range"]["timestamp"]["gte"] = json!(from.to_rfc3339());
        }
        if let Some(to) = query.to {
            range_query["range"]["timestamp"]["lte"] = json!(to.to_rfc3339());
        }
        must_clauses.push(range_query);
    }
    
    let search_body = if must_clauses.is_empty() {
        json!({
            "query": { "match_all": {} },
            "sort": [{ "timestamp": { "order": "desc" } }],
            "size": query.limit.unwrap_or(100),
            "from": query.offset.unwrap_or(0)
        })
    } else {
        json!({
            "query": { "bool": { "must": must_clauses } },
            "sort": [{ "timestamp": { "order": "desc" } }],
            "size": query.limit.unwrap_or(100),
            "from": query.offset.unwrap_or(0)
        })
    };
    
    let response = client
        .search(SearchParts::Index(&[index_name]))
        .body(search_body)
        .send()
        .await
        .map_err(|e| ServerError {
            code: StatusCode::GATEWAY_TIMEOUT,
            message: String::from("Search request failed"),
            additional_information: e.to_string(),
        })?;
        
    let response_body: Value = response
        .json()
        .await
        .map_err(|e| ServerError {
            code: StatusCode::INTERNAL_SERVER_ERROR,
            message: String::from("Failed to parse search response"),
            additional_information: e.to_string(),
        })?;
        
    let hits = response_body["hits"]["hits"]
        .as_array()
        .ok_or_else(|| ServerError {
            code: StatusCode::INTERNAL_SERVER_ERROR,
            message: String::from("Invalid search response format"),
            additional_information: String::from("Expected hits array in response"),
        })?;
        
    let mut logs = Vec::new();
    for hit in hits {
        if let Some(source) = hit["_source"].as_object() {
            let log_entry: LogEntry = serde_json::from_value(json!(source))
                .map_err(|e| ServerError {
                    code: StatusCode::INTERNAL_SERVER_ERROR,
                    message: String::from("Failed to deserialize log entry"),
                    additional_information: e.to_string(),
                })?;
            logs.push(log_entry);
        }
    }
    
    Ok(logs)
}

/// Performs full-text search on sensor logs using multi-field matching with fuzzy capabilities.
///
/// This function executes fuzzy full-text search across sensor log fields including
/// message content, device names, and log levels. It provides comprehensive search
/// capabilities for sensor data with automatic typo tolerance and relevance scoring.
///
/// # Parameters
/// * `index_name` - The name of the Elasticsearch index containing sensor logs
/// * `client` - Reference to the configured Elasticsearch client
/// * `search` - Sensor search query parameters including search terms and pagination
///
/// # Search Features
/// * Multi-field search across `msg.msg`, `msg.device`, and `level` fields
/// * Fuzzy matching with automatic fuzziness adjustment for typo tolerance
/// * Best fields matching strategy for optimal relevance scoring
/// * Results sorted by timestamp in descending order (newest first)
/// * Pagination support with configurable limit and offset
///
/// # Returns
/// * `Ok(Vec<LogEntry>)` - List of matching sensor log entries ordered by relevance and timestamp
/// * `Err(ServerError)` - Error if search execution or response parsing fails
///
/// # Examples
/// ```rust
/// let search = SearchQuery {
///     query: "temperature exceeded threshold".to_string(),
///     limit: Some(50),
///     offset: Some(0),
/// };
/// let logs = search_logs("sensor_logs", &client, &search).await?;
/// ```
pub async fn search_logs(
    index_name: &str,
    client: &Elasticsearch,
    search: &SearchQuery,
) -> Result<Vec<LogEntry>, ServerError> {
    let search_body = json!({
        "query": {
            "multi_match": {
                "query": search.query,
                "fields": ["msg.msg", "msg.device", "level"],
                "type": "best_fields",
                "fuzziness": "AUTO"
            }
        },
        "sort": [{ "timestamp": { "order": "desc" } }],
        "size": search.limit.unwrap_or(100),
        "from": search.offset.unwrap_or(0)
    });
    
    let response = client
        .search(SearchParts::Index(&[index_name]))
        .body(search_body)
        .send()
        .await
        .map_err(|e| ServerError {
            code: StatusCode::GATEWAY_TIMEOUT,
            message: String::from("Search request failed"),
            additional_information: e.to_string(),
        })?;
        
    let response_body: Value = response
        .json()
        .await
        .map_err(|e| ServerError {
            code: StatusCode::INTERNAL_SERVER_ERROR,
            message: String::from("Failed to parse search response"),
            additional_information: e.to_string(),
        })?;
        
    let hits = response_body["hits"]["hits"]
        .as_array()
        .ok_or_else(|| ServerError {
            code: StatusCode::INTERNAL_SERVER_ERROR,
            message: String::from("Invalid search response format"),
            additional_information: String::from("Expected hits array in response"),
        })?;
        
    let mut logs = Vec::new();
    for hit in hits {
        if let Some(source) = hit["_source"].as_object() {
            let log_entry: LogEntry = serde_json::from_value(json!(source))
                .map_err(|e| ServerError {
                    code: StatusCode::INTERNAL_SERVER_ERROR,
                    message: String::from("Failed to deserialize log entry"),
                    additional_information: e.to_string(),
                })?;
            logs.push(log_entry);
        }
    }
    
    Ok(logs)
}
