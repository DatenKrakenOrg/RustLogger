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
use serializable_objects::LogEntry;
use std::env;
use uuid::Uuid;

struct AppState {
    client: Elasticsearch,
    host_id: Uuid,
    index_name: String,
}

/// Endpoint used to send logs towards the es cluster.
#[post("/send_log")]
async fn send_log(
    data: web::Data<AppState>,
    log_message: web::Json<LogEntry>,
) -> ActixResult<HttpResponse> {
    // Map_err needed since send_document doesnt return a actix error.
    let return_val = send_document(&data.index_name, &data.client, &log_message)
        .await
        .map_err(ErrorInternalServerError)?;

    Ok(HttpResponse::Ok().json(serde_json::json!({ "result": return_val })))
}

/// Endpoint that returns the container name OR if not available a uuid generated on startup within crate::main.
#[get("/whoareyou")]
async fn who_are_you(data: web::Data<AppState>) -> ActixResult<HttpResponse> {
    println!("Fuck you");
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
    let index_name: String = env::var("INDEX_NAME").context("INDEX_NAME not set")?;

    // Creates a index if missing, otherwise returns
    create_logs_index(&index_name, &client)
        .await
        .context("Failed to call create_logs_index function")?;

    let state = web::Data::new(AppState {
        client: client.clone(),
        host_id: Uuid::new_v4(),
        index_name: index_name,
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
