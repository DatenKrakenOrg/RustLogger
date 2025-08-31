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
use regex::Regex;
use std::sync::Arc;

struct AppState {
    client: Elasticsearch,
    host_id: Uuid,
    message_types: HashMap<String, MessageTypeConfig>,
    regex_cache: HashMap<String, Arc<Regex>>,
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

    // Parse CSV line into JSON object using regex pattern
    let regex = match data.regex_cache.get(&payload.message_type) {
        Some(regex) => regex,
        None => {
            return Ok(HttpResponse::InternalServerError().json(serde_json::json!({
                "error": format!("Regex pattern not compiled for message type: {}", payload.message_type)
            })));
        }
    };

    let log_document = match parse_csv_with_regex(&payload.csv_line, message_type_config, regex) {
        Ok(doc) => doc,
        Err(e) => {
            return Ok(HttpResponse::BadRequest().json(serde_json::json!({
                "error": format!("Failed to parse CSV with regex: {}", e)
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
    let (message_types, regex_cache) = load_message_types(&config_path)
        .context("Failed to load message types configuration")?;

    // Create indices for all message types at startup
    println!("Creating indices for all message types...");
    for message_type in message_types.values() {
        match create_logs_index(&message_type.index_name, &client, &message_type.fields).await {
            Ok(result) => println!("Index creation result for '{}': {}", message_type.name, result),
            Err(e) => eprintln!("Failed to create index for '{}': {}", message_type.name, e),
        }
    }

    let state = web::Data::new(AppState {
        client: client.clone(),
        host_id: Uuid::new_v4(),
        message_types,
        regex_cache,
    });

    env_logger::init_from_env(env_logger::Env::new().default_filter_or("warn"));
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

fn load_message_types(config_path: &str) -> Result<(HashMap<String, MessageTypeConfig>, HashMap<String, Arc<Regex>>)> {
    let content = fs::read_to_string(config_path)?;
    let config_file: ConfigFile = toml::from_str(&content)?;
    
    let mut types = HashMap::new();
    let mut regex_cache = HashMap::new();
    
    for message_type in config_file.message_types {
        let regex = Regex::new(&message_type.regex_pattern)
            .context(format!("Failed to compile regex for message type '{}': {}", message_type.name, message_type.regex_pattern))?;
        
        regex_cache.insert(message_type.name.clone(), Arc::new(regex));
        types.insert(message_type.name.clone(), message_type);
    }
    
    Ok((types, regex_cache))
}

fn parse_csv_with_regex(csv_line: &str, config: &MessageTypeConfig, regex: &Regex) -> Result<serde_json::Value> {
    let captures = regex.captures(csv_line.trim())
        .ok_or_else(|| {
            println!("ERROR: Regex match failed for message type '{}' with line: '{}'", config.name, csv_line);
            println!("ERROR: Regex pattern was: '{}'", config.regex_pattern);
            anyhow::anyhow!("CSV line doesn't match regex pattern for message type '{}': {}", config.name, csv_line)
        })?;

    let mut json_obj = serde_json::Map::new();
    
    for (field_name, field_config) in &config.fields {
        if let Some(captured_value) = captures.name(field_name) {
            match parse_field_value(captured_value.as_str(), field_config) {
                Ok(field_value) => {
                    json_obj.insert(field_name.clone(), field_value);
                },
                Err(e) => {
                    eprintln!("ERROR: Failed to parse field '{}' with value '{}': {}", field_name, captured_value.as_str(), e);
                    return Err(e);
                }
            }
        } else {
            eprintln!("ERROR: Field '{}' not found in regex captures for message type '{}'", field_name, config.name);
            return Err(anyhow::anyhow!("Field '{}' not found in regex captures", field_name));
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
