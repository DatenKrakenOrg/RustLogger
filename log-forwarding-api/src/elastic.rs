use crate::log_entry::{ElasticLogDocument, LogEntry};
use crate::query_structures::{LogQuery, SearchQuery};
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

/// Persists a document in elasticsearch based on a client and a index
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

/// Creates a log mapping. This is needed in order to create a index in elastic search. It's format matches the logs.
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
