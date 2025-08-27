// Remove the import since we're now using serde_json::Value
use anyhow::{Context, Ok, Result};
use elasticsearch::{
    Elasticsearch, IndexParts,
    auth::Credentials,
    http::transport::{
        SingleNodeConnectionPool, TransportBuilder,
    },
    indices::{IndicesCreateParts, IndicesExistsParts},
};
//use env_logger::builder;
use serde_json::{Value, json};
use std::env;
use url::Url;
use std::result::Result::Ok as ResultOk;

/// Creates a elastic search client
///
/// # Examples
/// ```
/// let client: Elasticsearch = create_client().context("Failed to create elasticsearch client")?;
/// ```
pub fn create_client() -> Result<Elasticsearch> {
    let username: String = env::var("ELASTIC_USERNAME").context("ELASTIC_USERNAME not set")?;
    let password: String = env::var("ELASTIC_PASSWORD").context("ELASTIC_PASSWORD not set")?;
    let str_url: String = env::var("ELASTIC_URL").context("ELASTIC_URL not set")?;

    // Parse URL with proper scheme detection
    let url: Url = Url::parse(&str_url).context("Invalid ES URL")?;


    let pool: SingleNodeConnectionPool = SingleNodeConnectionPool::new(url);

    //Since of a local project we disable cert and only use basic authentication
    let transport = TransportBuilder::new(pool)
        .auth(Credentials::Basic(username, password))
        .disable_proxy()
        .cert_validation(elasticsearch::cert::CertificateValidation::None)
        .build()
        .context("Failed to build transport")?;

    Ok(Elasticsearch::new(transport))
}

/// Creates an index in elastic search based on the cluster on the client passed
///
/// # Examples:
/// ```
///     let client: Elasticsearch = create_client().context("Failed to create elasticsearch client")?;
///    let index_name: String = env::var("INDEX_NAME").context("INDEX_NAME not set")?;
///
///    // Creates a index if missing, otherwise returns
///    create_logs_index(
///        &index_name,
///        &client,
///    )
///    .await
///    .context("Failed to call create_logs_index function")?;
/// ```
pub async fn create_logs_index(index_name: &str, connector: &Elasticsearch) -> Result<String> {
    let mapping = create_log_mapping();

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
        .context("Index fetch attempt failed")?;

    if exists.status_code().is_success() {
        return Ok(format!("Index '{}' already exists", index_name));
    }

    //If not create one with a mapping matching the log
    let response = connector
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
        .context("Index creation attempt failed")?;

    response
        .error_for_status_code()
        .context("Failed to insert log entry")?;

    Ok(format!("Index '{}' created successfully", index_name))
}

/// Persists a document in elasticsearch based on a client and a index
pub async fn send_document(
    index_name: &str,
    client: &Elasticsearch,
    log_document: &serde_json::Value,
) -> Result<String> {
    let response = client
        .index(IndexParts::Index(index_name))
        .body(log_document)
        .send()
        .await
        .context("Log entry request failed")?;

    response
        .error_for_status_code()
        .context("Failed to insert log entry")?;

    Ok(format!(
        "Log entry inserted: {}",
        serde_json::to_string_pretty(log_document)?
    ))
}

pub async fn get_nodes(client: &Elasticsearch) -> Result<String> {
    let result = client
        .nodes()
        .info(elasticsearch::nodes::NodesInfoParts::None)
        .send()
        .await
        .context("Node Info request failed");

    match result {
        ResultOk(r) => match r.text().await {
            ResultOk(s) => Ok(s),
            Err(_) => Err(anyhow::Error::msg("Node Info could not be retrieved")),
        },
        Err(_) => Err(anyhow::Error::msg("Node Info could not be retrieved")),
    }
}

/// Creates a log mapping. This is needed in order to create a index in elastic search. It's format matches the logs.
fn create_log_mapping() -> Value {
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
