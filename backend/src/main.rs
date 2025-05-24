use std::net::SocketAddr;
use std::sync::Arc;

use axum::{
    extract::Extension,
    routing::{get, post, delete},
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

    // Connect to database - changed to MySQL
    let db_pool = MySqlPoolOptions::new()
        .max_connections(10)
        .connect(&config.database_url)
        .await?;
    
    // Run migrations
    sqlx::migrate!("./migrations")
        .run(&db_pool)
        .await?;
    
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
    
    // Set up CORS
    let cors = CorsLayer::new()
        .allow_origin(Any)
        .allow_methods(Any)
        .allow_headers(Any);
    
    // Build application router
    let app = Router::new()
        .route("/api/feed", get(feed::get_feed))
        .route("/api/drones", post(drones::add_drone))
        .route("/api/drones/{id}", delete(drones::delete_drone))  // Updated from :id to {id}
        .route("/ws", get(websocket::handler))
        .layer(Extension(app_state))
        .layer(cors);
    
    // Start server - fix the bind call by removing the reference
    let addr = SocketAddr::from(([0, 0, 0, 0], config.port));
    tracing::info!("Listening on {}", addr);
    axum_server::bind(addr)  // Removed the & reference
        .serve(app.into_make_service())
        .await?;
    
    Ok(())
}
