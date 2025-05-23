use crate::models::{RTMPStream, StreamStatus, StreamMetadata, AppState};
use chrono::Utc;
use tokio::net::{TcpListener, TcpStream};
use tokio::io::{AsyncReadExt, AsyncWriteExt};
use uuid::Uuid;
use log::{info, error, warn}; // Added

pub struct RTMPServer {
    app_state: AppState,
}

impl RTMPServer {
    pub fn new(app_state: AppState) -> Self {
        Self { app_state }
    }

    pub async fn start(&self) -> Result<(), Box<dyn std::error::Error>> {
        let listener = TcpListener::bind(format!("0.0.0.0:{}", self.app_state.config.rtmp_port)).await?;
        info!("RTMP server listening on port {}", self.app_state.config.rtmp_port);

        loop {
            let (socket, addr) = listener.accept().await?;
            let app_state = self.app_state.clone();
            
            tokio::spawn(async move {
                if let Err(e) = handle_rtmp_connection(socket, addr.to_string(), app_state).await {
                    error!("Error handling RTMP connection from {}: {}", addr, e);
                }
            });
        }
    }
}

async fn handle_rtmp_connection(
    mut socket: TcpStream,
    client_ip: String,
    app_state: AppState,
) -> Result<(), Box<dyn std::error::Error>> {
    info!("New RTMP connection from: {}", client_ip);
    let mut buffer = [0; 1024];
    
    // RTMP handshake
    perform_handshake(&mut socket).await?;
    
    // Parse RTMP messages
    loop {
        let n = socket.read(&mut buffer).await?;
        if n == 0 {
            info!("RTMP client {} disconnected.", client_ip);
            break;
        }
        
        // Parse RTMP message and handle accordingly
        if let Some(rtmp_message) = parse_rtmp_message(&buffer[..n]) {
            info!("RTMP message from {}: {:?}", client_ip, rtmp_message);
            handle_rtmp_message(rtmp_message, &client_ip, &app_state, &mut socket).await?;
        } else {
            warn!("Failed to parse RTMP message from {} (data length: {})", client_ip, n);
        }
    }
    
    Ok(())
}

async fn perform_handshake(socket: &mut TcpStream) -> Result<(), Box<dyn std::error::Error>> {
    // Simplified RTMP handshake
    let mut c0c1 = [0u8; 1537];
    socket.read_exact(&mut c0c1).await?;
    
    // Send S0, S1, S2
    let s0 = [3u8]; // RTMP version 3
    let mut s1 = [0u8; 1536];
    let mut s2 = [0u8; 1536];
    
    // Fill S1 with timestamp and random data
    let timestamp = Utc::now().timestamp() as u32;
    s1[0..4].copy_from_slice(&timestamp.to_be_bytes());
    s1[4..8].copy_from_slice(&[0, 0, 0, 0]); // Zero field
    
    // S2 echoes C1
    s2.copy_from_slice(&c0c1[1..]);
    
    socket.write_all(&s0).await?;
    socket.write_all(&s1).await?;
    socket.write_all(&s2).await?;
    
    // Read C2
    let mut c2 = [0u8; 1536];
    socket.read_exact(&mut c2).await?;
    
    info!("RTMP handshake completed with a client.");
    Ok(())
}

#[derive(Debug)]
enum RTMPMessage {
    Connect { app_name: String },
    Publish { stream_key: String },
    Play { stream_name: String },
    DeleteStream { stream_id: String },
}

fn parse_rtmp_message(data: &[u8]) -> Option<RTMPMessage> {
    // Simplified RTMP message parsing
    // In a real implementation, you'd need a proper RTMP parser
    if data.len() < 12 {
        return None;
    }
    
    let message_type = data[11];
    
    match message_type {
        20 => { // AMF0 Command
            if let Ok(command) = String::from_utf8(data[12..].to_vec()) {
                if command.contains("connect") {
                    return Some(RTMPMessage::Connect { app_name: "live".to_string() });
                } else if command.contains("publish") {
                    return Some(RTMPMessage::Publish { stream_key: "test".to_string() });
                }
            }
        }
        _ => {}
    }
    
    None
}

async fn handle_rtmp_message(
    message: RTMPMessage,
    client_ip: &str,
    app_state: &AppState,
    socket: &mut TcpStream,
) -> Result<(), Box<dyn std::error::Error>> {
    match message {
        RTMPMessage::Connect { app_name } => {
            info!("RTMP Connect from {} to app: {}", client_ip, app_name);
            send_connect_result(socket, true).await?;
        }
        RTMPMessage::Publish { stream_key } => {
            info!("RTMP Publish from {} with key: {}", client_ip, stream_key);
            
            let stream_id = Uuid::new_v4().to_string();
            let rtmp_stream = RTMPStream {
                id: stream_id.clone(),
                name: format!("Stream_{}", stream_key),
                url: format!("rtmp://127.0.0.1:1935/live/{}", stream_key),
                stream_key: stream_key.clone(),
                status: StreamStatus {
                    is_live: true,
                    bitrate: 0,
                    resolution: "1920x1080".to_string(),
                    fps: Some(30.0),
                    codec: Some("H264".to_string()),
                    viewers: 0,
                    started_at: Some(Utc::now()),
                    last_frame_at: Some(Utc::now()),
                },
                metadata: Some(StreamMetadata {
                    title: format!("Live Stream {}", stream_key),
                    description: "RTMP Live Stream".to_string(),
                    created_at: Utc::now(),
                    updated_at: Utc::now(),
                    tags: vec!["live".to_string(), "rtmp".to_string()],
                    thumbnail: None,
                    duration: None,
                    language: Some("en".to_string()),
                    category: Some("live".to_string()),
                }),
                publisher_ip: Some(client_ip.to_string()),
                auth_token: None,
            };
            
            // Add stream to manager
            if let Ok(mut manager) = app_state.stream_manager.lock() {
                manager.add_rtmp_stream(rtmp_stream);
                
                // Create corresponding RTSP stream
                let rtsp_stream_id = Uuid::new_v4().to_string();
                let rtsp_stream = crate::models::RTSPStream {
                    id: rtsp_stream_id,
                    name: format!("RTSP_{}", stream_key),
                    url: format!("rtsp://127.0.0.1:{}/live/{}", app_state.config.rtsp_port, stream_key),
                    status: crate::models::StreamStatus {
                        is_live: true,
                        bitrate: 0,
                        resolution: "1920x1080".to_string(),
                        fps: Some(30.0),
                        codec: Some("H264".to_string()),
                        viewers: 0,
                        started_at: Some(Utc::now()),
                        last_frame_at: Some(Utc::now()),
                    },
                    input_stream_id: stream_id.clone(),
                    metadata: None,
                    mount_point: format!("/live/{}", stream_key),
                    allowed_ips: vec![],
                };
                
                manager.add_rtsp_stream(rtsp_stream);
                info!("Created RTSP stream for RTMP key: {}", stream_key);
            }
            
            send_publish_result(socket, true).await?;
        }
        RTMPMessage::Play { stream_name } => {
            info!("RTMP Play request from {} for stream: {}", client_ip, stream_name);
            send_play_result(socket, true).await?;
        }
        RTMPMessage::DeleteStream { stream_id } => {
            info!("RTMP Delete stream: {} requested by {}", stream_id, client_ip);
            
            // Remove stream from manager
            if let Ok(mut manager) = app_state.stream_manager.lock() {
                manager.rtmp_streams.remove(&stream_id);
            }
        }
    }
    
    Ok(())
}

async fn send_connect_result(socket: &mut TcpStream, success: bool) -> Result<(), Box<dyn std::error::Error>> {
    // Simplified RTMP response
    let response: &'static [u8] = if success {
        b"_result\x00\x3f\xf0\x00\x00\x00\x00\x00\x00"
    } else {
        b"_error\x00\x3f\xf0\x00\x00\x00\x00\x00\x00"
    };
    
    socket.write_all(response).await?;
    Ok(())
}

async fn send_publish_result(socket: &mut TcpStream, success: bool) -> Result<(), Box<dyn std::error::Error>> {
    let response = if success {
        b"onStatus\x00\x00\x00\x00\x00\x00\x00\x00\x00"
    } else {
        b"onStatus\x00\x00\x00\x00\x00\x00\x00\x00\x01"
    };
    
    socket.write_all(response).await?;
    Ok(())
}

async fn send_play_result(socket: &mut TcpStream, success: bool) -> Result<(), Box<dyn std::error::Error>> {
    let response = if success {
        b"onStatus\x00\x00\x00\x00\x00\x00\x00\x00\x00"
    } else {
        b"onStatus\x00\x00\x00\x00\x00\x00\x00\x00\x01"
    };
    
    socket.write_all(response).await?;
    Ok(())
}
