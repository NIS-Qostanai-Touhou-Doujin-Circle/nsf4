use crate::models::{AppState, RTSPStream};
use tokio::net::{TcpListener, TcpStream};
use tokio::io::{AsyncReadExt, AsyncWriteExt};
use tokio::sync::mpsc;
use log::{info, error};
use std::collections::HashMap;
use std::sync::Arc;
use tokio::sync::Mutex as TokioMutex;

pub struct RTSPServer {
    app_state: AppState,
    stream_data_rx: mpsc::Receiver<(String, Vec<u8>)>, // stream_key, data
    clients: Arc<TokioMutex<HashMap<String, Vec<mpsc::Sender<Vec<u8>>>>>>, // stream_key -> [client_tx]
}

impl RTSPServer {
    pub fn new(app_state: AppState, stream_data_rx: mpsc::Receiver<(String, Vec<u8>)>) -> Self {
        Self { 
            app_state, 
            stream_data_rx,
            clients: Arc::new(TokioMutex::new(HashMap::new())),
        }
    }

    pub async fn start(&mut self) -> Result<(), Box<dyn std::error::Error>> {
        let listener = TcpListener::bind(format!("0.0.0.0:{}", self.app_state.config.rtsp_port)).await?;
        info!("RTSP server listening on port {}", self.app_state.config.rtsp_port);

        // Start handling stream data from RTMP server
        let clients_clone = self.clients.clone();
        tokio::spawn(async move {
            while let Some((stream_key, data)) = self.stream_data_rx.recv().await {
                Self::broadcast_to_rtsp_clients(&stream_key, &data, clients_clone.clone()).await;
            }
        });

        // Accept RTSP clients
        loop {
            let (socket, addr) = listener.accept().await?;
            let app_state = self.app_state.clone();
            let clients_clone = self.clients.clone();

            tokio::spawn(async move {
                if let Err(e) = handle_rtsp_client(socket, addr.to_string(), app_state, clients_clone).await {
                    error!("Error handling RTSP client {}: {}", addr, e);
                }
            });
        }
    }

    async fn broadcast_to_rtsp_clients(
        stream_key: &str, 
        data: &[u8],
        clients: Arc<TokioMutex<HashMap<String, Vec<mpsc::Sender<Vec<u8>>>>>>,
    ) {
        let mut clients = clients.lock().await;
        if let Some(client_txs) = clients.get_mut(stream_key) {
            // Keep track of which clients need to be removed because they're closed
            let mut to_remove = Vec::new();
            
            for (idx, client_tx) in client_txs.iter().enumerate() {
                if client_tx.send(data.to_vec()).await.is_err() {
                    // Client channel closed, mark for removal
                    to_remove.push(idx);
                }
            }
            
            // Remove closed clients from back to front to avoid index issues
            to_remove.sort_by(|a, b| b.cmp(a));
            for idx in to_remove {
                client_txs.remove(idx);
            }
            
            info!("Broadcasted {} bytes to {} RTSP clients for stream: {}", 
                 data.len(), client_txs.len(), stream_key);
        }
    }
}

async fn handle_rtsp_client(
    mut socket: TcpStream,
    client_addr: String,
    app_state: AppState,
    clients: Arc<TokioMutex<HashMap<String, Vec<mpsc::Sender<Vec<u8>>>>>>,
) -> Result<(), Box<dyn std::error::Error>> {
    info!("New RTSP client connected: {}", client_addr);

    let mut buffer = [0; 4096];
    let mut current_stream: Option<String> = None;
    let (client_tx, mut client_rx) = mpsc::channel::<Vec<u8>>(100);

    // Loop to handle RTSP requests
    loop {
        tokio::select! {
            result = socket.read(&mut buffer) => {
                match result {
                    Ok(0) => {
                        info!("RTSP client {} disconnected", client_addr);
                        break;
                    }
                    Ok(n) => {
                        let request = String::from_utf8_lossy(&buffer[..n]);
                        info!("RTSP request from {}: {}", client_addr, request.lines().next().unwrap_or(""));
                        
                        if let Some(stream_key) = parse_stream_key_from_request(&request) {
                            current_stream = Some(stream_key.clone());
                            
                            // Register this client to receive stream data
                            let mut clients_map = clients.lock().await;
                            clients_map.entry(stream_key)
                                .or_insert_with(Vec::new)
                                .push(client_tx.clone());
                        }

                        // Handle the RTSP request
                        handle_rtsp_request(&request, &mut socket, &app_state).await?;
                    }
                    Err(e) => {
                        error!("Error reading from RTSP client {}: {}", client_addr, e);
                        break;
                    }
                }
            }
            Some(data) = client_rx.recv() => {
                // Received video/audio data from RTMP stream, send to this RTSP client
                if let Err(e) = send_rtp_data(&mut socket, &data).await {
                    error!("Error sending RTP data to {}: {}", client_addr, e);
                    break;
                }
            }
            else => break,
        }
    }

    // Remove this client from the clients map
    if let Some(stream_key) = current_stream {
        let mut clients_map = clients.lock().await;
        if let Some(client_txs) = clients_map.get_mut(&stream_key) {
            // Find and remove this client's sender
            if let Some(pos) = client_txs.iter().position(|tx| tx == &client_tx) {
                client_txs.remove(pos);
            }
        }
    }

    Ok(())
}

fn parse_stream_key_from_request(request: &str) -> Option<String> {
    for line in request.lines() {
        if line.starts_with("SETUP") || line.starts_with("PLAY") || line.starts_with("DESCRIBE") {
            if let Some(url) = line.split_whitespace().nth(1) {
                // Parse URL like rtsp://server:port/live/streamkey
                let parts: Vec<&str> = url.split('/').collect();
                if parts.len() >= 2 {
                    return Some(parts[parts.len() - 1].to_string());
                }
            }
        }
    }
    None
}

async fn handle_rtsp_request(
    request: &str,
    socket: &mut TcpStream,
    app_state: &AppState,
) -> Result<(), Box<dyn std::error::Error>> {
    let lines: Vec<&str> = request.lines().collect();
    if lines.is_empty() {
        return Ok(());
    }

    let request_line = lines[0];
    let parts: Vec<&str> = request_line.split_whitespace().collect();
    
    if parts.len() < 3 {
        return Ok(());
    }

    let method = parts[0];
    let url = parts[1];

    let mut cseq = "1";
    for line in lines.iter() {
        if line.starts_with("CSeq:") {
            if let Some(seq) = line.split(':').nth(1) {
                cseq = seq.trim();
            }
        }
    }

    match method {
        "DESCRIBE" => {
            send_describe_response(socket, url, cseq).await?;
        }
        "SETUP" => {
            send_setup_response(socket, cseq).await?;
        }
        "PLAY" => {
            send_play_response(socket, cseq).await?;
        }
        "TEARDOWN" => {
            send_teardown_response(socket, cseq).await?;
        }
        _ => {
            send_not_implemented_response(socket, cseq).await?;
        }
    }

    Ok(())
}

async fn send_describe_response(socket: &mut TcpStream, url: &str, cseq: &str) -> Result<(), Box<dyn std::error::Error>> {
    let sdp = format!(
        "v=0\r\n\
         o=- 0 0 IN IP4 127.0.0.1\r\n\
         s=Live Stream\r\n\
         c=IN IP4 0.0.0.0\r\n\
         t=0 0\r\n\
         m=video 0 RTP/AVP 96\r\n\
         a=rtpmap:96 H264/90000\r\n\
         a=control:track1\r\n"
    );

    let response = format!(
        "RTSP/1.0 200 OK\r\n\
         CSeq: {}\r\n\
         Content-Type: application/sdp\r\n\
         Content-Length: {}\r\n\
         \r\n\
         {}",
        cseq,
        sdp.len(),
        sdp
    );

    socket.write_all(response.as_bytes()).await?;
    Ok(())
}

async fn send_setup_response(socket: &mut TcpStream, cseq: &str) -> Result<(), Box<dyn std::error::Error>> {
    let response = format!(
        "RTSP/1.0 200 OK\r\n\
         CSeq: {}\r\n\
         Transport: RTP/AVP;unicast;client_port=8000-8001;server_port=9000-9001\r\n\
         Session: 12345678\r\n\
         \r\n",
        cseq
    );

    socket.write_all(response.as_bytes()).await?;
    Ok(())
}

async fn send_play_response(socket: &mut TcpStream, cseq: &str) -> Result<(), Box<dyn std::error::Error>> {
    let response = format!(
        "RTSP/1.0 200 OK\r\n\
         CSeq: {}\r\n\
         Session: 12345678\r\n\
         \r\n",
        cseq
    );

    socket.write_all(response.as_bytes()).await?;
    Ok(())
}

async fn send_teardown_response(socket: &mut TcpStream, cseq: &str) -> Result<(), Box<dyn std::error::Error>> {
    let response = format!(
        "RTSP/1.0 200 OK\r\n\
         CSeq: {}\r\n\
         Session: 12345678\r\n\
         \r\n",
        cseq
    );

    socket.write_all(response.as_bytes()).await?;
    Ok(())
}

async fn send_not_implemented_response(socket: &mut TcpStream, cseq: &str) -> Result<(), Box<dyn std::error::Error>> {
    let response = format!(
        "RTSP/1.0 501 Not Implemented\r\n\
         CSeq: {}\r\n\
         \r\n",
        cseq
    );

    socket.write_all(response.as_bytes()).await?;
    Ok(())
}

async fn send_rtp_data(socket: &mut TcpStream, data: &[u8]) -> Result<(), Box<dyn std::error::Error>> {
    // In a real implementation, you'd need to format data as proper RTP packets
    // This is simplified for demonstration purposes
    socket.write_all(data).await?;
    Ok(())
}
