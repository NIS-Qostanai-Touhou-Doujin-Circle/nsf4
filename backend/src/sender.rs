use crate::models::{RTSPStream, StreamStatus, StreamMetadata, AppState};
use chrono::Utc;
use tokio::net::{TcpListener, TcpStream};
use tokio::io::{AsyncReadExt, AsyncWriteExt};
use uuid::Uuid;
use std::collections::HashMap;
use std::sync::Arc;
use tokio::sync::RwLock;
use log::{info, error, warn}; // Added warn for potential future use

pub struct RTSPServer {
    app_state: AppState,
    sessions: Arc<RwLock<HashMap<String, RTSPSession>>>,
}

#[derive(Debug, Clone)]
struct RTSPSession {
    id: String,
    client_ip: String,
    stream_id: Option<String>,
    transport: Option<String>,
    rtp_port: Option<u16>,
    rtcp_port: Option<u16>,
}

impl RTSPServer {
    pub fn new(app_state: AppState) -> Self {
        Self {
            app_state,
            sessions: Arc::new(RwLock::new(HashMap::new())),
        }
    }

    pub async fn start(&self) -> Result<(), Box<dyn std::error::Error>> {
        let listener = TcpListener::bind(format!("0.0.0.0:{}", self.app_state.config.rtsp_port)).await?;
        info!("RTSP server listening on port {}", self.app_state.config.rtsp_port);
        
        loop {
            let (socket, addr) = listener.accept().await?;
            let app_state = self.app_state.clone();
            let sessions = self.sessions.clone();
            
            tokio::spawn(async move {
                if let Err(e) = handle_rtsp_connection(socket, addr.to_string(), app_state, sessions).await {
                    error!("Error handling RTSP connection from {}: {}", addr, e);
                }
            });
        }
    }

    pub async fn create_rtsp_stream_for_rtmp(&self, rtmp_stream_id: &str, rtmp_stream_key: &str) -> Result<String, Box<dyn std::error::Error>> {
        let rtsp_stream_id = Uuid::new_v4().to_string();
        let mount_point = format!("/live/{}", rtmp_stream_key);
        
        let rtsp_stream = RTSPStream {
            id: rtsp_stream_id.clone(),
            name: format!("RTSP_{}", rtmp_stream_key),
            url: format!("rtsp://127.0.0.1:{}{}", self.app_state.config.rtsp_port, mount_point),
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
            input_stream_id: rtmp_stream_id.to_string(),
            metadata: Some(StreamMetadata {
                title: format!("RTSP Stream from {}", rtmp_stream_key),
                description: "Converted from RTMP".to_string(),
                created_at: Utc::now(),
                updated_at: Utc::now(),
                tags: vec!["rtsp".to_string(), "converted".to_string()],
                thumbnail: None,
                duration: None,
                language: Some("en".to_string()),
                category: Some("live".to_string()),
            }),
            mount_point,
            allowed_ips: vec![], // Allow all IPs
        };

        if let Ok(mut manager) = self.app_state.stream_manager.lock() {
            manager.add_rtsp_stream(rtsp_stream);
        }
        info!("Created RTSP stream {} for RTMP stream {}", rtsp_stream_id, rtmp_stream_id);
        Ok(rtsp_stream_id)
    }
}

async fn handle_rtsp_connection(
    mut socket: TcpStream,
    client_ip: String,
    app_state: AppState,
    sessions: Arc<RwLock<HashMap<String, RTSPSession>>>,
) -> Result<(), Box<dyn std::error::Error>> {
    let session_id = Uuid::new_v4().to_string();
    let mut buffer = [0; 4096];
    
    // Create session
    {
        let mut sessions_guard = sessions.write().await;
        sessions_guard.insert(session_id.clone(), RTSPSession {
            id: session_id.clone(),
            client_ip: client_ip.clone(),
            stream_id: None,
            transport: None,
            rtp_port: None,
            rtcp_port: None,
        });
    }

    loop {
        let n = socket.read(&mut buffer).await?;
        if n == 0 {
            info!("RTSP client {} disconnected (session {}).", client_ip, session_id);
            break;
        }

        let request = String::from_utf8_lossy(&buffer[..n]);
        info!("RTSP request from {}: {}", client_ip, request.lines().next().unwrap_or_default());
        let response = handle_rtsp_request(&request, &session_id, &app_state, &sessions).await?;
        
        socket.write_all(response.as_bytes()).await?;
    }

    // Cleanup session
    {
        let mut sessions_guard = sessions.write().await;
        sessions_guard.remove(&session_id);
        info!("RTSP session {} cleaned up for client {}", session_id, client_ip);
    }

    Ok(())
}

async fn handle_rtsp_request(
    request: &str,
    session_id: &str,
    app_state: &AppState,
    sessions: &Arc<RwLock<HashMap<String, RTSPSession>>>,
) -> Result<String, Box<dyn std::error::Error>> {
    let lines: Vec<&str> = request.lines().collect();
    if lines.is_empty() {
        return Ok(create_rtsp_response(400, "Bad Request", None, None));
    }

    let request_line = lines[0];
    info!("Handling RTSP request: {}", request_line);
    let parts: Vec<&str> = request_line.split_whitespace().collect();
    
    if parts.len() < 3 {
        return Ok(create_rtsp_response(400, "Bad Request", None, None));
    }

    let method = parts[0];
    let url = parts[1];
    let cseq = extract_header_value(&lines, "CSeq").unwrap_or("0");

    match method {
        "OPTIONS" => {
            Ok(create_rtsp_response(200, "OK", Some(cseq), Some("OPTIONS, DESCRIBE, SETUP, PLAY, TEARDOWN")))
        }
        "DESCRIBE" => {
            info!("RTSP DESCRIBE for URL: {}", url);
            handle_describe(url, cseq, app_state).await
        }
        "SETUP" => {
            info!("RTSP SETUP for URL: {}", url);
            handle_setup(url, cseq, &lines, session_id, sessions).await
        }
        "PLAY" => {
            info!("RTSP PLAY for session: {}", session_id);
            handle_play(cseq, session_id, sessions).await
        }
        "TEARDOWN" => {
            info!("RTSP TEARDOWN for session: {}", session_id);
            handle_teardown(cseq, session_id, sessions).await
        }
        _ => {
            warn!("RTSP method not implemented: {}", method);
            Ok(create_rtsp_response(501, "Not Implemented", Some(cseq), None))
        }
    }
}

fn create_rtsp_response(code: u16, reason: &str, cseq: Option<&str>, extra_headers: Option<&str>) -> String {
    let mut response = format!("RTSP/1.0 {} {}\r\n", code, reason);
    
    if let Some(seq) = cseq {
        response.push_str(&format!("CSeq: {}\r\n", seq));
    }
    
    if code == 200 && extra_headers.is_some() {
        if let Some(headers) = extra_headers {
            response.push_str(&format!("Public: {}\r\n", headers));
        }
    }
    
    response.push_str("Server: RustRTSP/1.0\r\n");
    response.push_str("\r\n");
    
    response
}

async fn handle_describe(url: &str, cseq: &str, app_state: &AppState) -> Result<String, Box<dyn std::error::Error>> {
    // Extract stream key from URL (e.g., rtsp://host/live/streamkey)
    let path_parts: Vec<&str> = url.split('/').collect();
    if path_parts.len() < 3 {
        return Ok(create_rtsp_response(404, "Not Found", Some(cseq), None));
    }

    let stream_key = path_parts[path_parts.len() - 1];
    
    // Check if corresponding RTMP stream exists
    let stream_exists = {
        if let Ok(manager) = app_state.stream_manager.lock() {
            manager.rtmp_streams.values().any(|stream| 
                stream.stream_key == stream_key && stream.status.is_live
            )
        } else {
            false
        }
    };

    if !stream_exists {
        return Ok(create_rtsp_response(404, "Stream Not Found", Some(cseq), None));
    }

    // Create SDP description
    let sdp = create_sdp_description(stream_key);
    
    let mut response = format!("RTSP/1.0 200 OK\r\n");
    response.push_str(&format!("CSeq: {}\r\n", cseq));
    response.push_str("Content-Type: application/sdp\r\n");
    response.push_str(&format!("Content-Length: {}\r\n", sdp.len()));
    response.push_str("Server: RustRTSP/1.0\r\n");
    response.push_str("\r\n");
    response.push_str(&sdp);
    
    Ok(response)
}

async fn handle_setup(
    url: &str,
    cseq: &str,
    lines: &[&str],
    session_id: &str,
    sessions: &Arc<RwLock<HashMap<String, RTSPSession>>>,
) -> Result<String, Box<dyn std::error::Error>> {
    let transport = extract_header_value(lines, "Transport").unwrap_or("");
    
    // Parse RTP/UDP ports from Transport header
    let (rtp_port, rtcp_port) = parse_transport_ports(transport);
    
    // Update session
    {
        let mut sessions_guard = sessions.write().await;
        if let Some(session) = sessions_guard.get_mut(session_id) {
            session.transport = Some(transport.to_string());
            session.rtp_port = rtp_port;
            session.rtcp_port = rtcp_port;
        }
    }

    let mut response = format!("RTSP/1.0 200 OK\r\n");
    response.push_str(&format!("CSeq: {}\r\n", cseq));
    response.push_str(&format!("Session: {}\r\n", session_id));
    response.push_str(&format!("Transport: {}\r\n", transport));
    response.push_str("Server: RustRTSP/1.0\r\n");
    response.push_str("\r\n");
    
    Ok(response)
}

async fn handle_play(
    cseq: &str,
    session_id: &str,
    sessions: &Arc<RwLock<HashMap<String, RTSPSession>>>,
) -> Result<String, Box<dyn std::error::Error>> {
    // Start streaming to client (simplified)
    info!("Starting RTSP stream for session: {}", session_id);
    
    let mut response = format!("RTSP/1.0 200 OK\r\n");
    response.push_str(&format!("CSeq: {}\r\n", cseq));
    response.push_str(&format!("Session: {}\r\n", session_id));
    response.push_str("Range: npt=0-\r\n");
    response.push_str("Server: RustRTSP/1.0\r\n");
    response.push_str("\r\n");
    
    Ok(response)
}

async fn handle_teardown(
    cseq: &str,
    session_id: &str,
    sessions: &Arc<RwLock<HashMap<String, RTSPSession>>>,
) -> Result<String, Box<dyn std::error::Error>> {
    info!("Tearing down RTSP session: {}", session_id);
    
    let mut response = format!("RTSP/1.0 200 OK\r\n");
    response.push_str(&format!("CSeq: {}\r\n", cseq));
    response.push_str("Server: RustRTSP/1.0\r\n");
    response.push_str("\r\n");
    
    Ok(response)
}

fn extract_header_value<'a>(lines: &'a [&'a str], header_name: &'a str) -> Option<&'a str> {
    for line in lines {
        if line.starts_with(header_name) {
            if let Some(pos) = line.find(':') {
                return Some(line[pos + 1..].trim());
            }
        }
    }
    None
}

fn parse_transport_ports(transport: &str) -> (Option<u16>, Option<u16>) {
    // Parse client_port=8000-8001 from Transport header
    if let Some(client_port_start) = transport.find("client_port=") {
        let port_part = &transport[client_port_start + 12..];
        if let Some(dash_pos) = port_part.find('-') {
            let rtp_str = &port_part[..dash_pos];
            let rtcp_str = &port_part[dash_pos + 1..].split(';').next().unwrap_or("");
            
            let rtp_port = rtp_str.parse().ok();
            let rtcp_port = rtcp_str.parse().ok();
            
            return (rtp_port, rtcp_port);
        }
    }
    (None, None)
}

fn create_sdp_description(stream_key: &str) -> String {
    format!(
        "v=0\r\n\
         o=- 123456789 123456789 IN IP4 127.0.0.1\r\n\
         s=Stream {}\r\n\
         c=IN IP4 127.0.0.1\r\n\
         t=0 0\r\n\
         m=video 0 RTP/AVP 96\r\n\
         a=rtpmap:96 H264/90000\r\n\
         a=control:track1\r\n\
         m=audio 0 RTP/AVP 97\r\n\
         a=rtpmap:97 MPEG4-GENERIC/44100\r\n\
         a=control:track2\r\n",
        stream_key
    )
}