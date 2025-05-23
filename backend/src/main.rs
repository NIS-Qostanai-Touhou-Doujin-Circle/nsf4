mod models;
mod reciever;
mod sender;

use actix_web::{web, App, HttpServer, HttpResponse, Result, middleware::Logger as ActixLogger}; // Renamed to avoid conflict
use actix_cors::Cors;
use serde::{Serialize};
use std::sync::{Arc, Mutex};
use models::{AppState, StreamManager, ServerConfig};
use reciever::RTMPServer;
use sender::RTSPServer;
use log::{info, error}; // Added

#[derive(Serialize)]
pub struct ResponseData {
    pub status: String,
    pub message: String,
}

async fn health_check() -> Result<HttpResponse> {
    Ok(HttpResponse::Ok().json(ResponseData {
        status: "ok".to_string(),
        message: "Server is running".to_string(),
    }))
}

fn setup_logger() -> std::result::Result<(), fern::InitError> {
    fern::Dispatch::new()
        .format(|out, message, record| {
            out.finish(format_args!(
                "{} [{}] [{}] {}",
                chrono::Local::now().format("%Y-%m-%d %H:%M:%S"),
                record.level(),
                record.target(),
                message
            ))
        })
        .level(log::LevelFilter::Info) // Set default log level
        .chain(std::io::stdout())
        .chain(fern::log_file("app.log")?)
        .apply()?;
    Ok(())
}

#[actix_web::main]
pub async fn main() -> std::io::Result<()> {
    if let Err(e) = setup_logger() {
        eprintln!("Error setting up logger: {}", e); // Fallback if logger setup fails
    }

    let server_config = ServerConfig::default();
    let stream_manager = Arc::new(Mutex::new(StreamManager::new()));

    let app_state = AppState {
        stream_manager: stream_manager.clone(),
        config: server_config.clone(),
    };

    info!("HTTP server listening on port {}", server_config.http_port);
    
    // Initialize RTMP and RTSP servers
    let rtmp_server = RTMPServer::new(app_state.clone());
    let rtsp_server = RTSPServer::new(app_state.clone());

    // Start RTMP and RTSP server
    tokio::spawn(async move {
        if let Err(e) = rtmp_server.start().await {
            error!("RTMP server error: {}", e);
        }
    });

    // Start RTSP server
    tokio::spawn(async move {
        if let Err(e) = rtsp_server.start().await {
            error!("RTSP server error: {}", e);
        }
    });

    // Start HTTP server
    HttpServer::new(move || {
        let cors = Cors::default()
            .allow_any_origin()
            .allow_any_method()
            .allow_any_header()
            .supports_credentials();

        App::new()
            .app_data(web::Data::new(app_state.clone()))
            .wrap(cors)
            .wrap(ActixLogger::default())
            .route("/health", web::get().to(health_check))
    })
    .bind(format!("127.0.0.1:{}", server_config.http_port))?
    .run()
    .await
}