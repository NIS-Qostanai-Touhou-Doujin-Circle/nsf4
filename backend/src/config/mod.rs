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
    pub redis_url: String,
    pub gps_data_ttl_seconds: u64,
}

impl Config {
    pub fn from_env() -> Result<Self, env::VarError> {
        // Load configuration from environment variables with fallbacks to defaults
        let database_url = env::var("DATABASE_URL")
            .unwrap_or_else(|_| "mysql://root:root@localhost:3306/nsf".to_string());
        
        let port = env::var("PORT")
            .ok()
            .and_then(|p| p.parse::<u16>().ok())
            .unwrap_or(5123);
            
        let media_server_url = env::var("MEDIA_SERVER_URL")
            .unwrap_or_else(|_| "rtmp://167.99.129.124:1935".to_string());
            
        let screenshot_interval_seconds = env::var("SCREENSHOT_INTERVAL_SECONDS")
            .ok()
            .and_then(|s| s.parse::<u64>().ok())
            .unwrap_or(10);
              let screenshot_quality = env::var("SCREENSHOT_QUALITY")
            .ok()
            .and_then(|s| s.parse::<u32>().ok())
            .unwrap_or(80);
            
        let redis_username = env::var("REDIS_USERNAME")
            .unwrap_or_else(|_| "".to_string());
            
        let redis_password = env::var("REDIS_PASSWORD")
            .unwrap_or_else(|_| "".to_string());

        let redis_url = env::var("REDIS_URL")
            .unwrap_or_else(|_| {
                if redis_username.is_empty() && redis_password.is_empty() {
                    "redis://localhost:6379".to_string()
                } else {    
                    format!("redis://{}:{}@localhost:6379", redis_username, redis_password)
                }
            });
            
        let gps_data_ttl_seconds = env::var("GPS_DATA_TTL_SECONDS")
            .ok()
            .and_then(|s| s.parse::<u64>().ok())
            .unwrap_or(3600); // 1 hour default
        
        let cfg = Config {
            database_url,
            port,
            media_server_url,
            screenshot_interval_seconds,
            screenshot_quality,
            redis_url,
            gps_data_ttl_seconds,
        };        info!(
            database_url = %cfg.database_url, 
            port = cfg.port, 
            media_server_url = %cfg.media_server_url,
            screenshot_interval = cfg.screenshot_interval_seconds,
            screenshot_quality = cfg.screenshot_quality,
            redis_url = %cfg.redis_url,
            gps_data_ttl = cfg.gps_data_ttl_seconds,
            "Configuration loaded from environment"
        );
        Ok(cfg)
    }
}
