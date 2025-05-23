use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use chrono::{DateTime, Utc};

// Enhanced metadata for a stream
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct StreamMetadata {
    pub title: String,
    pub description: String,
    pub created_at: DateTime<Utc>,
    pub updated_at: DateTime<Utc>,
    pub tags: Vec<String>,
    pub thumbnail: Option<Vec<u8>>,
    pub duration: Option<u64>, // in seconds
    pub language: Option<String>,
    pub category: Option<String>,
}

impl StreamMetadata {
    pub fn clone(&self) -> Self {
        Self {
            title: self.title.clone(),
            description: self.description.clone(),
            created_at: self.created_at,
            updated_at: self.updated_at,
            tags: self.tags.clone(),
            thumbnail: self.thumbnail.clone(),
            duration: self.duration,
            language: self.language.clone(),
            category: self.category.clone(),
        }
    }
}

// Enhanced status of a stream
#[derive(Debug, Clone, Serialize, Deserialize)]
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

impl StreamStatus {
    pub fn clone(&self) -> Self {
        Self {
            is_live: self.is_live,
            bitrate: self.bitrate,
            resolution: self.resolution.clone(),
            fps: self.fps,
            codec: self.codec.clone(),
            viewers: self.viewers,
            started_at: self.started_at,
            last_frame_at: self.last_frame_at,
        }
    }
}

// Enhanced input RTMP stream
#[derive(Debug, Clone, Serialize, Deserialize)]
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

impl RTMPStream {
    pub fn clone(&self) -> Self {
        Self {
            id: self.id.clone(),
            name: self.name.clone(),
            url: self.url.clone(),
            stream_key: self.stream_key.clone(),
            status: self.status.clone(),
            metadata: self.metadata.clone(),
            publisher_ip: self.publisher_ip.clone(),
            auth_token: self.auth_token.clone(),
        }
    }
}

// Enhanced output RTSP stream
#[derive(Debug, Clone, Serialize, Deserialize)]
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

// Stream configuration
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct StreamConfig {
    pub max_viewers: u32,
    pub auto_record: bool,
    pub record_path: Option<String>,
    pub transcode_profiles: Vec<TranscodeProfile>,
    pub auth_required: bool,
}

// Transcoding profile
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TranscodeProfile {
    pub name: String,
    pub video_bitrate: u32,
    pub audio_bitrate: u32,
    pub resolution: String,
    pub fps: f32,
    pub codec: String,
}

// Stream statistics
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct StreamStats {
    pub stream_id: String,
    pub bytes_sent: u64,
    pub bytes_received: u64,
    pub packets_sent: u64,
    pub packets_received: u64,
    pub dropped_frames: u32,
    pub uptime: u64, // in seconds
}

// Enhanced stream manager
#[derive(Debug)]
pub struct StreamManager {
    pub rtmp_streams: HashMap<String, RTMPStream>,
    pub rtsp_streams: HashMap<String, RTSPStream>,
    pub configs: HashMap<String, StreamConfig>,
    pub stats: HashMap<String, StreamStats>,
}

impl StreamManager {
    pub fn new() -> Self {
        Self {
            rtmp_streams: HashMap::new(),
            rtsp_streams: HashMap::new(),
            configs: HashMap::new(),
            stats: HashMap::new(),
        }
    }

    pub fn add_rtmp_stream(&mut self, stream: RTMPStream) {
        self.rtmp_streams.insert(stream.id.clone(), stream);
    }

    pub fn add_rtsp_stream(&mut self, stream: RTSPStream) {
        self.rtsp_streams.insert(stream.id.clone(), stream);
    }

    pub fn get_live_streams(&self) -> Vec<&RTMPStream> {
        self.rtmp_streams
            .values()
            .filter(|stream| stream.status.is_live)
            .collect()
    }

    pub fn update_stream_stats(&mut self, stream_id: &str, stats: StreamStats) {
        self.stats.insert(stream_id.to_string(), stats);
    }
}

// Server configuration
#[derive(Debug, Clone, Deserialize)]
pub struct ServerConfig {
    pub http_port: u16,
    pub rtmp_port: u16,
    pub rtsp_port: u16,
    pub max_connections: usize,
    pub auth_enabled: bool,
    pub recording_enabled: bool,
    pub recording_path: String,
}

impl Default for ServerConfig {
    fn default() -> Self {
        Self {
            http_port: 8080,
            rtmp_port: 1935,
            rtsp_port: 554,
            max_connections: 1000,
            auth_enabled: false,
            recording_enabled: false,
            recording_path: "./recordings".to_string(),
        }
    }
}

// Application state for Actix Web
#[derive(Debug)]
pub struct AppState {
    pub stream_manager: std::sync::Arc<std::sync::Mutex<StreamManager>>,
    pub config: ServerConfig,
}