mod elastic;
mod log_entry;
mod log_entry_components;
mod query_structures;
mod server_error;

use crate::server_error::ServerError;
use actix_web::{
    App, HttpResponse, HttpServer, Result as ActixResult, error::ErrorInternalServerError, get,
    http::StatusCode, middleware::Logger, post, web,
};
use dotenvy::dotenv;
use elastic::{
    create_client, create_container_log_mapping, create_log_mapping, create_logs_index, get_nodes,
    query_logs, search_logs, send_document, query_container_logs, search_container_logs,
};
use elasticsearch::Elasticsearch;
use log_entry::{ContainerLogEntry, LogEntry};
use query_structures::{LogQuery, SearchQuery, ContainerLogQuery, ContainerSearchQuery};
use std::env;
use uuid::Uuid;

struct AppState {
    client: Elasticsearch,
    host_id: Uuid,
    index_name: String,
    container_logs_index_name: String,
}

/// Endpoint used to send logsender logs towards the es cluster.
#[post("/send_log")]
async fn send_log(
    data: web::Data<AppState>,
    log_message: web::Json<LogEntry>,
) -> ActixResult<HttpResponse> {
    let log_entry = log_message.into_inner();
    // Map_err needed since send_document doesnt return a actix error.
    let return_val = send_document(&data.index_name, &data.client, &log_entry)
        .await
        .map_err(ErrorInternalServerError)?;

    Ok(HttpResponse::Ok().json(serde_json::json!({ "result": return_val })))
}

/// Endpoint used to send logsender logs towards the es cluster.
#[post("/send_container_log")]
async fn send_container_log(
    data: web::Data<AppState>,
    log_message: web::Json<ContainerLogEntry>,
) -> ActixResult<HttpResponse> {
    let log_entry = log_message.into_inner();
    // Map_err needed since send_document doesnt return a actix error.
    let return_val = send_document(&data.container_logs_index_name, &data.client, &log_entry)
        .await
        .map_err(ErrorInternalServerError)?;

    Ok(HttpResponse::Ok().json(serde_json::json!({ "result": return_val })))
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

#[get("/logs")]
async fn get_logs(
    data: web::Data<AppState>,
    query: web::Query<LogQuery>,
) -> ActixResult<HttpResponse> {
    let logs = query_logs(&data.index_name, &data.client, &query)
        .await
        .map_err(ErrorInternalServerError)?;

    Ok(HttpResponse::Ok().json(serde_json::json!({ "logs": logs })))
}

#[get("/logs/search")]
async fn search_logs_endpoint(
    data: web::Data<AppState>,
    query: web::Query<SearchQuery>,
) -> ActixResult<HttpResponse> {
    let logs = search_logs(&data.index_name, &data.client, &query)
        .await
        .map_err(ErrorInternalServerError)?;

    Ok(HttpResponse::Ok().json(serde_json::json!({ "logs": logs })))
}

#[get("/container-logs")]
async fn get_container_logs(
    data: web::Data<AppState>,
    query: web::Query<ContainerLogQuery>,
) -> ActixResult<HttpResponse> {
    let logs = query_container_logs(&data.container_logs_index_name, &data.client, &query)
        .await
        .map_err(ErrorInternalServerError)?;

    Ok(HttpResponse::Ok().json(serde_json::json!({ "logs": logs })))
}

#[get("/container-logs/search")]
async fn search_container_logs_endpoint(
    data: web::Data<AppState>,
    query: web::Query<ContainerSearchQuery>,
) -> ActixResult<HttpResponse> {
    let logs = search_container_logs(&data.container_logs_index_name, &data.client, &query)
        .await
        .map_err(ErrorInternalServerError)?;

    Ok(HttpResponse::Ok().json(serde_json::json!({ "logs": logs })))
}

#[actix_web::main]
async fn main() -> std::io::Result<()> {
    // Set DEPLOYMENT=PROD in docker compose!
    if env::var("DEPLOYMENT").unwrap_or_default() != "PROD" {
        dotenv().ok();
    }
    let client: Elasticsearch = create_client().unwrap();
    let index_name: String = env::var("INDEX_NAME")
        .map_err(|_| ServerError {
            code: StatusCode::INTERNAL_SERVER_ERROR,
            message: String::from("INDEX_NAME not set during startup"),
            additional_information: String::from("Set INDEX_NAME in .env / env variables!"),
        })
        .unwrap();

    let container_logs_index_name: String = env::var("CONTAINER_INDEX_NAME")
        .map_err(|_| ServerError {
            code: StatusCode::INTERNAL_SERVER_ERROR,
            message: String::from("CONTAINER_INDEX_NAME not set during startup"),
            additional_information: String::from(
                "Set CONTAINER_INDEX_NAME in .env / env variables!",
            ),
        })
        .unwrap();

    // Creates a index if missing, otherwise returns
    create_logs_index(&index_name, &client, create_log_mapping())
        .await
        .unwrap();

    create_logs_index(
        &container_logs_index_name,
        &client,
        create_container_log_mapping(),
    )
    .await
    .unwrap();

    let state = web::Data::new(AppState {
        client: client.clone(),
        host_id: Uuid::new_v4(),
        index_name,
        container_logs_index_name,
    });

    env_logger::init_from_env(env_logger::Env::new().default_filter_or("info"));
    HttpServer::new(move || {
        App::new()
            .app_data(state.clone())
            .service(send_log)
            .service(who_are_you)
            .service(elastic_node_info)
            .service(send_container_log)
            .service(get_logs)
            .service(search_logs_endpoint)
            .service(get_container_logs)
            .service(search_container_logs_endpoint)
            .wrap(Logger::default())
    })
    .bind(("0.0.0.0", 8080))?
    .run()
    .await?;

    Ok(())
}
