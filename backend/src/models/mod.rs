use serde::{Deserialize, Serialize};
use sqlx::FromRow;

#[derive(Debug, Serialize, Deserialize, FromRow)]
pub struct Video {
    pub id: String,
    pub url: String,
    pub title: String,
    pub thumbnail: String,
    #[sqlx(rename = "created_at")]
    #[serde(rename = "createdAt")]
    pub created_at: String,
    pub rtmp_url: String,
    pub ws_url: Option<String>,
    pub video_source_name: String,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct Feed {
    pub videos: Vec<Video>,
}

#[allow(dead_code)]
#[derive(Debug, Deserialize)]
pub struct AddDroneRequest {
    pub title: String,
    pub rtmp_url: String,
    pub ws_url: Option<String>,
}

#[derive(Debug, Serialize)]
pub struct AddDroneResponse {
    pub id: String,
    pub url: String,
    pub title: String,
    pub thumbnail: String,
    #[serde(rename = "createdAt")]
    pub created_at: String,
    pub rtmp_url: String,
    pub ws_url: Option<String>,
    pub video_source_name: String,
}

// Remove unused DeleteDroneRequest struct since it's not used
// #[derive(Debug, Deserialize)]
// pub struct DeleteDroneRequest {
//     pub id: String,
// }

#[derive(Debug, Serialize)]
pub struct DeleteDroneResponse {
    pub success: bool,
}

#[derive(Debug, Serialize, Deserialize, FromRow, Clone)]
pub struct DroneGpsData {
    pub id: String,
    pub video_id: String,
    pub created_at: String,
    pub latitude: f64,
    pub longitude: f64,
    pub altitude: f64,
}

#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct DroneGpsDataWithTitle {
    pub id: String,
    pub video_id: String,
    pub created_at: String,
    pub latitude: f64,
    pub longitude: f64,
    pub altitude: f64,
    pub title: String,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct DroneGpsUpdate {
    pub drone_id: String,
    pub latitude: f64,
    pub longitude: f64,
    pub altitude: f64,
    pub timestamp: Option<String>,
    pub title: Option<String>,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct WebSocketMessage {
    pub message_type: String,
    pub data: serde_json::Value,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct WebSocketError {
    pub error: String,
}

// Типы сообщений для WebSocket
pub mod ws_message_types {
    pub const GPS_UPDATE: &str = "gps_update";
    pub const GPS_DATA: &str = "gps_data";
    pub const ERROR: &str = "error";
}