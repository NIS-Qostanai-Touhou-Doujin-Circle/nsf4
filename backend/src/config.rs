use crate::models::Config;
use std::env;

pub fn load_config() -> Config {
    // Load config from environment or use defaults
    let rtmp_port = env::var("RTMP_PORT")
        .ok()
        .and_then(|p| p.parse::<u16>().ok())
        .unwrap_or(1935);
    
    let rtsp_port = env::var("RTSP_PORT")
        .ok()
        .and_then(|p| p.parse::<u16>().ok())
        .unwrap_or(8554);
    
    let http_port = env::var("HTTP_PORT")
        .ok()
        .and_then(|p| p.parse::<u16>().ok())
        .unwrap_or(8080);
    
    let log_level = env::var("LOG_LEVEL").unwrap_or_else(|_| "info".to_string());
    
    Config {
        rtmp_port,
        rtsp_port,
        http_port,
        log_level,
    }
}
