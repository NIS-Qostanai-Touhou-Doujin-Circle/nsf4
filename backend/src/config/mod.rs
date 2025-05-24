use serde::Deserialize;
use std::env;

#[derive(Clone, Debug, Deserialize)]
pub struct Config {
    pub database_url: String,
    pub port: u16,
    pub media_server_url: String,
}

impl Config {
    pub fn from_env() -> Result<Self, env::VarError> {
        Ok(Config {
            database_url: "mysql://cos1nus:Random_Sh1t@localhost:3306/nsf".to_string(),
            port: 5123,
            media_server_url: "rtmp://167.99.129.124:1935".to_string(),
        })
    }
}
