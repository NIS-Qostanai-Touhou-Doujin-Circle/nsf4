mod models;
mod rtmp_server;
mod rtsp_server;
mod config;
mod http_api;

use models::AppState;
use rtmp_server::RTMPServer;
use rtsp_server::RTSPServer;
use tokio::sync::mpsc;
use log::{info, error, LevelFilter};
use env_logger::Env;

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    // Initialize logging
    let config = config::load_config();
    let log_level = match config.log_level.to_lowercase().as_str() {
        "debug" => LevelFilter::Debug,
        "trace" => LevelFilter::Trace,
        "warn" => LevelFilter::Warn,
        "error" => LevelFilter::Error,
        _ => LevelFilter::Info,
    };
    
    env_logger::Builder::from_env(Env::default())
        .filter_level(log_level)
        .init();
    
    info!("Starting Not So Far v4 streaming server");
    info!("RTMP port: {}, RTSP port: {}, HTTP port: {}", 
           config.rtmp_port, config.rtsp_port, config.http_port);
    
    // Initialize application state
    let app_state = AppState::new(config);
    
    // Create channel for RTMP -> RTSP data
    let (stream_data_tx, stream_data_rx) = mpsc::channel::<(String, Vec<u8>)>(1000);
    
    // Initialize servers
    let rtmp_server = RTMPServer::new(app_state.clone(), stream_data_tx);
    let mut rtsp_server = RTSPServer::new(app_state.clone(), stream_data_rx);
    
    // Start HTTP API in a separate task
    let http_state = app_state.clone();
    tokio::spawn(async move {
        if let Err(e) = http_api::start_http_server(http_state).await {
            error!("HTTP API server error: {}", e);
        }
    });
    
    // Start both streaming servers
    info!("Starting RTMP and RTSP servers...");
    tokio::select! {
        result = rtmp_server.start() => {
            if let Err(e) = result {
                error!("RTMP server error: {}", e);
            }
        }
        result = rtsp_server.start() => {
            if let Err(e) = result {
                error!("RTSP server error: {}", e);
            }
        }
    }
    
    Ok(())
}
