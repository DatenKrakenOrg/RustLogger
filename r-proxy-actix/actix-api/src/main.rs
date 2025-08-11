mod serializable_objects;
mod elastic;
use actix_web::{post, web, App, HttpResponse, HttpServer, error::ErrorInternalServerError, Result as ActixResult};
use elasticsearch::Elasticsearch;
use serializable_objects::LogEntry;
use elastic::{create_client, create_logs_index, send_document, INDEX_NAME};
use dotenvy::dotenv;
use std::env;
use anyhow::{Result, Context};

struct AppState{
    client: Elasticsearch
}

#[post("/send_log")]
async fn send_log(
    data: web::Data<AppState>,
    log_message: web::Json<LogEntry>,
) -> ActixResult<HttpResponse> {
    let return_val = send_document(&INDEX_NAME, &data.client, &log_message)
        .await
        .map_err(ErrorInternalServerError)?; // 500 on failure (pick another mapper if you want 4xx)

    Ok(HttpResponse::Ok().json(serde_json::json!({ "result": return_val })))
}

#[actix_web::main]
async fn main() -> Result<()> {
    if env::var("DEPLOYMENT").unwrap_or_default() != "PROD" {
        dotenv().ok();
    }
    let client: Elasticsearch = create_client().context("Failed to create elasticsearch client")?;

    create_logs_index(&INDEX_NAME, &client).await.context("Failed to call create_logs_index function")?;

    let state = web::Data::new(AppState { client: client.clone() });

    HttpServer::new(move || {
        App::new()
            .app_data(state.clone())
            .service(send_log)
    })
    .bind(("127.0.0.1", 8080))?
    .run()
    .await?;

    Ok(())
}

