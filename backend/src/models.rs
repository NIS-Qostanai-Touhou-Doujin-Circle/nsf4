use chrono::{DateTime, Utc};
use serde::{Serialize, Deserialize};
use std::collections::HashMap;
use std::sync::{Arc, Mutex};

// Configuration for the application
#[derive(Clone, Debug)]
pub struct Config {
    pub rtmp_port: u16,
    pub rtsp_port: u16,
    pub http_port: u16,
    pub log_level: String,
}

impl Default for Config {
    fn default() -> Self {
        Self {
            rtmp_port: 1935,
            rtsp_port: 8554,
            http_port: 8080,
            log_level: "info".to_string(),
        }
    }
}

// Stream status information
#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct StreamStatus {
    pub is_live: bool,
    pub bitrate: u32,
    pub resolution: String,
    pub fps: Option<f32>,
    pub codec: Option<String>,
    pub viewers: u32,
    pub started_at: Option<DateTime<Utc>>,
    pub last_frame_at: Option<DateTime<Utc>>,
}

// Stream metadata
#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct StreamMetadata {
    pub title: String,
    pub description: String,
    pub created_at: DateTime<Utc>,
    pub updated_at: DateTime<Utc>,
    pub tags: Vec<String>,
    pub thumbnail: Option<String>,
    pub duration: Option<i64>,
    pub language: Option<String>,
    pub category: Option<String>,
}

// RTMP Stream
#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct RTMPStream {
    pub id: String,
    pub name: String,
    pub url: String,
    pub stream_key: String,
    pub status: StreamStatus,
    pub metadata: Option<StreamMetadata>,
    pub publisher_ip: Option<String>,
    pub auth_token: Option<String>,
}

// RTSP Stream
#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct RTSPStream {
    pub id: String,
    pub name: String,
    pub url: String,
    pub status: StreamStatus,
    pub input_stream_id: String,
    pub metadata: Option<StreamMetadata>,
    pub mount_point: String,
    pub allowed_ips: Vec<String>,
}

// Stream Manager for keeping track of active streams
#[derive(Clone)]
pub struct StreamManager {
    pub rtmp_streams: HashMap<String, RTMPStream>,
    pub rtsp_streams: HashMap<String, RTSPStream>,
}

impl StreamManager {
    pub fn new() -> Self {
        Self {
            rtmp_streams: HashMap::new(),
            rtsp_streams: HashMap::new(),
        }
    }

    pub fn add_rtmp_stream(&mut self, stream: RTMPStream) {
        self.rtmp_streams.insert(stream.id.clone(), stream);
    }

    pub fn add_rtsp_stream(&mut self, stream: RTSPStream) {
        self.rtsp_streams.insert(stream.id.clone(), stream);
    }
}

// Application state shared across components
#[derive(Clone)]
pub struct AppState {
    pub config: Config,
    pub stream_manager: Arc<Mutex<StreamManager>>,
}

impl AppState {
    pub fn new(config: Config) -> Self {
        Self {
            config,
            stream_manager: Arc::new(Mutex::new(StreamManager::new())),
        }
    }
}
