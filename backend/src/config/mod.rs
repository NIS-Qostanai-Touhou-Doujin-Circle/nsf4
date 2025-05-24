use serde::Deserialize;
use std::env;
use tracing::info;

#[derive(Clone, Debug, Deserialize)]
pub struct Config {
    pub database_url: String,
    pub port: u16,
    pub media_server_url: String,
    pub screenshot_interval_seconds: u64,
    pub screenshot_quality: u32,
}

impl Config {
    pub fn from_env() -> Result<Self, env::VarError> {
        // Load configuration values (currently hardcoded defaults)
        let cfg = Config {
            database_url: "mysql://cos1nus:Random_Sh1t@localhost:3306/nsf".to_string(),
            port: 5123,
            media_server_url: "rtmp://167.99.129.124:1935".to_string(),
            screenshot_interval_seconds: 10, // Default to 10 seconds
            screenshot_quality: 80,          // Default to 80% quality
        };        // Log loaded configuration
        info!(
            database_url = %cfg.database_url, 
            port = cfg.port, 
            media_server_url = %cfg.media_server_url,
            screenshot_interval = cfg.screenshot_interval_seconds,
            screenshot_quality = cfg.screenshot_quality,
            "Configuration loaded from environment"
        );
        Ok(cfg)
    }
}
