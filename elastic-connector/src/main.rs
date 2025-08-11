use elasticsearch::{
    Elasticsearch, Error,
    auth::Credentials,
    http::transport::{SingleNodeConnectionPool, TransportBuilder},
    indices::{IndicesCreateParts, IndicesExistsParts},
};
use serde_json::{Value, json};
use url::Url;

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    let credentials = Credentials::Basic("elastic".into(), "secretPassword".into());

    let url = Url::parse("https://localhost:9200")?;
    let conn_pool = SingleNodeConnectionPool::new(url);
    let transport = TransportBuilder::new(conn_pool)
        .auth(credentials)
        .disable_proxy()
        .cert_validation(elasticsearch::cert::CertificateValidation::None)
        .build()?;

    let client = Elasticsearch::new(transport);

    create_logs_index("log-test", &client).await?;

    Ok(())
}

async fn create_logs_index(
    index_name: &str,
    connector: &Elasticsearch,
) -> Result<(), Box<dyn std::error::Error>> {
    let mapping = create_log_mapping();

    let exists = connector
        .indices()
        .exists(IndicesExistsParts::Index(&[index_name]))
        .send()
        .await?;

    if exists.status_code().is_success() {
        println!("Index '{}' already exists", index_name);
        return Ok(());
    } else {
        println!("Index will be created");
    }

    let response = connector
        .indices()
        .create(IndicesCreateParts::Index(index_name))
        .body(json!({
            "mappings": mapping
        }))
        .send()
        .await?;

    match response.error_for_status_code() {
        Ok(_) => println!("Index '{}' created successfully", index_name),
        Err(e) => println!("Failed to create index: {}", e),
    }

    Ok(())
}

fn create_log_mapping() -> Value {
    json!({
        "properties": {
            "timestamp": {
                "type": "date",
                "format": "yyyy-MM-dd HH:mm:ss"
            },
            "level": {
                "type": "keyword"
            },
            "temperatur": {
                "type": "float"
            },
            "humidity": {
                "type": "float"
            },
            "msg": {
                "properties": {
                    "device": {
                        "type": "keyword"
                    },
                    "msg": {
                        "type": "text",
                        "analyzer": "standard"
                    },
                    "exceeded_values": {
                        "type": "boolean"
                    }
                }
            }
        }
    })
}
