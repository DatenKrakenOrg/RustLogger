use elasticsearch::{
    auth::Credentials,
    http::transport::{SingleNodeConnectionPool, TransportBuilder},
    indices::{IndicesCreateParts, IndicesExistsParts},
    Elasticsearch, IndexParts,
};
use serde_json::{json, Value};
use std::env;
use url::Url;
use anyhow::{Result, Context};
use crate::serializable_objects::LogEntry;

pub const INDEX_NAME: &str = "log-test";

pub fn create_client() -> Result<Elasticsearch> {
    let username = env::var("ELASTIC_USERNAME").context("ELASTIC_USERNAME not set")?;
    let password = env::var("ELASTIC_PASSWORD").context("ELASTIC_PASSWORD not set")?;

    let url = Url::parse("https://localhost:9200").context("Invalid ES URL")?;
    let pool = SingleNodeConnectionPool::new(url);

    let transport = TransportBuilder::new(pool)
        .auth(Credentials::Basic(username, password))
        .disable_proxy()
        .cert_validation(elasticsearch::cert::CertificateValidation::None)
        .build()
        .context("Failed to build transport")?;

    Ok(Elasticsearch::new(transport))
}

pub async fn create_logs_index(
    index_name: &str,
    connector: &Elasticsearch,
) -> Result<String> {
    let mapping = create_log_mapping();

    let exists = connector
        .indices()
        .exists(IndicesExistsParts::Index(&[index_name]))
        .send()
        .await
        .context("Index fetch attempt failed")?;

    if exists.status_code().is_success() {
        return Ok(format!("Index '{}' already exists", index_name));
    }

    let response = connector
        .indices()
        .create(IndicesCreateParts::Index(index_name))
        .body(json!({
            "mappings": mapping
        }))
        .send()
        .await
        .context("Index creation attempt failed")?;

    response.error_for_status_code().context("Failed to insert log entry")?;

    Ok(format!("Index '{}' created successfully", index_name))
}


pub async fn send_document(
    index_name: &str,
    client: &Elasticsearch,
    log_entry: &LogEntry
) -> Result<String> {
    let response = client
        .index(IndexParts::Index(index_name))
        .body(log_entry)
        .send()
        .await
        .context("Log entry request failed")?;

    response.error_for_status_code().context("Failed to insert log entry")?;

    Ok(format!(
        "Log entry inserted: {}",
        serde_json::to_string_pretty(log_entry)?
    ))
}



fn create_log_mapping() -> Value {
    json!({
        "properties": {
            "timestamp": {
                "type": "date",
                // RFC3339/ISO-8601 format
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
