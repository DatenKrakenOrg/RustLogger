mod elastic;
mod serializable_objects;
use actix_web::{
    App, HttpResponse, HttpServer, Result as ActixResult, error::ErrorInternalServerError, get,
    middleware::Logger, post, web,
};
use anyhow::{Context, Result};
use dotenvy::dotenv;
use elastic::{create_client, create_logs_index, send_document, get_nodes};
use elasticsearch::Elasticsearch;
use serializable_objects::{LogPayload, MessageTypeConfig, ConfigFile};
use std::{env, fs, collections::HashMap};
use uuid::Uuid;

struct AppState {
    client: Elasticsearch,
    host_id: Uuid,
    message_types: HashMap<String, MessageTypeConfig>,
}

/// Endpoint used to send logs towards the es cluster.
#[post("/send_log")]
async fn send_log(
    data: web::Data<AppState>,
    payload: web::Json<LogPayload>,
) -> ActixResult<HttpResponse> {
    // Get message type configuration
    let message_type_config = match data.message_types.get(&payload.message_type) {
        Some(config) => config,
        None => {
            return Ok(HttpResponse::BadRequest().json(serde_json::json!({
                "error": format!("Unknown message type: {}", payload.message_type)
            })));
        }
    };

    // Parse CSV line into JSON object based on message type configuration
    let log_document = match parse_csv_to_json(&payload.csv_line, message_type_config) {
        Ok(doc) => doc,
        Err(e) => {
            return Ok(HttpResponse::BadRequest().json(serde_json::json!({
                "error": format!("Failed to parse CSV: {}", e)
            })));
        }
    };

    // Send to Elasticsearch
    let return_val = send_document(&message_type_config.index_name, &data.client, &log_document)
        .await
        .map_err(ErrorInternalServerError)?;

    Ok(HttpResponse::Ok().json(serde_json::json!({ 
        "result": return_val,
        "message_type": payload.message_type,
        "index": message_type_config.index_name
    })))
}

/// Endpoint that returns the container name OR if not available a uuid generated on startup within crate::main.
#[get("/whoareyou")]
async fn who_are_you(data: web::Data<AppState>) -> ActixResult<HttpResponse> {
    Ok(HttpResponse::Ok().json(serde_json::json!(
        {
            "instance_id": env::var("HOSTNAME").unwrap_or_else(|_| data.host_id.to_string())
        }
    )))
}

#[get("/elasticnodeinfo")]
async fn elastic_node_info(data: web::Data<AppState>) -> ActixResult<HttpResponse> {
    let return_val = get_nodes(&data.client)
        .await
        .map_err(ErrorInternalServerError)?;

    Ok(HttpResponse::Ok().json(serde_json::json!({ "result": return_val })))
}

#[actix_web::main]
async fn main() -> Result<()> {
    // Set DEPLOYMENT=PROD in docker compose!
    if env::var("DEPLOYMENT").unwrap_or_default() != "PROD" {
        dotenv().ok();
    }
    let client: Elasticsearch = create_client().context("Failed to create elasticsearch client")?;
    
    // Load message types configuration
    let config_path = env::var("CONFIG_PATH").unwrap_or_else(|_| "message_types.toml".to_string());
    let message_types = load_message_types(&config_path)
        .context("Failed to load message types configuration")?;

    let state = web::Data::new(AppState {
        client: client.clone(),
        host_id: Uuid::new_v4(),
        message_types,
    });

    env_logger::init_from_env(env_logger::Env::new().default_filter_or("info"));
    HttpServer::new(move || {
        App::new()
            .app_data(state.clone())
            .service(send_log)
            .service(who_are_you)
            .service(elastic_node_info)
            .wrap(Logger::default())
    })
    .bind(("0.0.0.0", 8080))?
    .run()
    .await?;

    Ok(())
}

fn load_message_types(config_path: &str) -> Result<HashMap<String, MessageTypeConfig>> {
    let content = fs::read_to_string(config_path)?;
    let config_file: ConfigFile = toml::from_str(&content)?;
    
    let mut types = HashMap::new();
    for message_type in config_file.message_types {
        types.insert(message_type.name.clone(), message_type);
    }
    
    Ok(types)
}

fn parse_csv_to_json(csv_line: &str, config: &MessageTypeConfig) -> Result<serde_json::Value> {
    let fields: Vec<&str> = csv_line.split(',').collect();
    let field_names: Vec<String> = config.fields.keys().cloned().collect();
    
    if fields.len() != field_names.len() {
        return Err(anyhow::anyhow!(
            "CSV field count ({}) doesn't match configuration field count ({})", 
            fields.len(), 
            field_names.len()
        ));
    }

    let mut json_obj = serde_json::Map::new();
    
    for (i, field_name) in field_names.iter().enumerate() {
        if let Some(field_config) = config.fields.get(field_name) {
            let field_value = parse_field_value(fields[i], field_config)?;
            json_obj.insert(field_name.clone(), field_value);
        }
    }
    
    Ok(serde_json::Value::Object(json_obj))
}

fn parse_field_value(value: &str, field_config: &toml::Value) -> Result<serde_json::Value> {
    if let Some(field_type) = field_config.get("type").and_then(|v| v.as_str()) {
        match field_type {
            "datetime" => Ok(serde_json::Value::String(value.to_string())),
            "string" | "enum" | "uuid" => Ok(serde_json::Value::String(value.to_string())),
            "float" => {
                let parsed: f64 = value.parse()
                    .map_err(|_| anyhow::anyhow!("Failed to parse float: {}", value))?;
                Ok(serde_json::Value::Number(serde_json::Number::from_f64(parsed).unwrap()))
            },
            "integer" => {
                let parsed: i64 = value.parse()
                    .map_err(|_| anyhow::anyhow!("Failed to parse integer: {}", value))?;
                Ok(serde_json::Value::Number(serde_json::Number::from(parsed)))
            },
            _ => Ok(serde_json::Value::String(value.to_string())),
        }
    } else {
        Ok(serde_json::Value::String(value.to_string()))
    }
}
