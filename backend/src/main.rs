use std::net::SocketAddr;
use std::sync::Arc;

use axum::{
    extract::Extension,
    routing::{get, post},
    Router,
};
use sqlx::mysql::MySqlPoolOptions; // Changed from postgres to mysql
use tower_http::cors::{Any, CorsLayer};
use tracing_subscriber::{layer::SubscriberExt, util::SubscriberInitExt};

mod api;
mod config;
mod database;
mod models;
mod services;
mod websocket;
mod rtmp;

use api::{feed, drones};

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    // Initialize tracing
    tracing_subscriber::registry()
        .with(tracing_subscriber::EnvFilter::new(
            std::env::var("RUST_LOG").unwrap_or_else(|_| "info".into()),
        ))
        .with(tracing_subscriber::fmt::layer())
        .init();
    
    // Load configuration from environment variables
    let config = config::Config::from_env()?;
    // Log loaded configuration for debugging
    tracing::info!(config = ?config, "Configuration loaded");

    // Connect to database - changed to MySQL
    tracing::info!(database_url = %config.database_url, "Connecting to database");
    let db_pool = MySqlPoolOptions::new()
        .max_connections(10)
        .connect(&config.database_url)
        .await?;
    
    // Run migrations
    tracing::info!("Running database migrations");
    sqlx::migrate!("./migrations")
        .run(&db_pool)
        .await?;
    tracing::info!("Database migrations completed");
    
    // Make sure migrations table exists before running migrations
    sqlx::query("CREATE TABLE IF NOT EXISTS _sqlx_migrations (
        version BIGINT PRIMARY KEY,
        description TEXT NOT NULL,
        installed_on TIMESTAMP NOT NULL DEFAULT CURRENT_TIMESTAMP,
        success BOOLEAN NOT NULL,
        checksum BLOB NOT NULL,
        execution_time BIGINT NOT NULL
    )")
    .execute(&db_pool)
    .await?;
    // Create shared application state
    let app_state = Arc::new(services::AppState {
        db: db_pool,
        config: config.clone(),
    });
    // Initialize RTMP relays for existing drones
    tracing::info!("Fetching existing drones to initialize RTMP relays");
    let videos = database::get_videos(&app_state.db).await?;
    tracing::info!(count = videos.len(), "Existing drones found");
    for video in videos {
        let destination = format!("{}/{}", app_state.config.media_server_url, video.id);
        let added = rtmp::add_rtmp_relay(video.id.clone(), video.url.clone(), destination.clone());
        tracing::info!(video_id = %video.id, added = %added, destination = %destination, "Initialized RTMP relay for drone");
    }
    
    // Set up CORS
    let cors = CorsLayer::new()
        .allow_origin(Any)
        .allow_methods(Any)
        .allow_headers(Any);    // Build application router
    let app = Router::new()
        .route("/api/feed", get(feed::get_feed))
        .route("/api/drones", post(drones::add_drone))
        .route("/api/drones/{id}", 
            get(drones::get_drone_by_id)
            .delete(drones::delete_drone)
        )
        .route("/ws", get(websocket::handler))
        .layer(Extension(app_state))
        .layer(cors);
      // Start HTTP server and RTMP server
    let http_addr = SocketAddr::from(([0, 0, 0, 0], config.port));
    
    tracing::info!("HTTP server listening on {}", http_addr);
    
    // Start RTMP server in background
    let rtmp_addr = SocketAddr::from(([0, 0, 0, 0], config.port + 1));
    tracing::info!("RTMP server listening on {}", rtmp_addr);
    tokio::spawn(async move {
        if let Err(e) = rtmp::start_rtmp_server(rtmp_addr).await {
            tracing::error!(error = %e, "RTMP server error");
        }
    });
    // Start HTTP server
    axum_server::bind(http_addr)
        .serve(app.into_make_service())
        .await?;
    
    Ok(())
}
