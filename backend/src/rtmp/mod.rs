use rml_rtmp::sessions::{
    ServerSession,
    ServerSessionConfig,
    ServerSessionEvent,
    ServerSessionResult,
};
use tokio::net::TcpListener;
use tokio::io::{AsyncReadExt, AsyncWriteExt};
use std::net::SocketAddr;

pub async fn start_rtmp_server(addr: SocketAddr) -> Result<(), Box<dyn std::error::Error>> {
    let listener = TcpListener::bind(addr).await?;
    tracing::info!("RTMP server listening on {}", addr);

    loop {
        let (mut socket, client_addr) = listener.accept().await?;
        tracing::info!("RTMP connection from {}", client_addr);

        tokio::spawn(async move {
            let config = ServerSessionConfig::new();
            let (mut session, _) = ServerSession::new(config).unwrap();
            let mut buffer = vec![0u8; 4096];

            loop {
                let bytes_read = match socket.read(&mut buffer).await {
                    Ok(0) => break, // Connection closed
                    Ok(n) => n,
                    Err(e) => {
                        tracing::error!("Error reading from socket: {}", e);
                        break;
                    }
                };

                match session.handle_input(&buffer[..bytes_read]) {
                    Ok(results) => {
                        if !process_session_results(&mut session, &mut socket, results).await {
                            break;
                        }
                    }
                    Err(e) => {
                        tracing::error!("Error handling RTMP input: {}", e);
                        break;
                    }
                }
            }

            tracing::info!("RTMP connection closed from {}", client_addr);
        });
    }
}

async fn process_session_results(
    session: &mut ServerSession,
    socket: &mut tokio::net::TcpStream,
    results: Vec<ServerSessionResult>,
) -> bool {
    for result in results {
        match result {
            ServerSessionResult::OutboundResponse(packet) => {
                if let Err(e) = socket.write_all(&packet.bytes).await {
                    tracing::error!("Error writing to socket: {}", e);
                    return false;
                }
            }
            ServerSessionResult::RaisedEvent(event) => {
                match event {
                    // Fixed: Use proper struct pattern matching
                    ServerSessionEvent::ClientChunkSizeChanged { new_chunk_size } => {
                        tracing::debug!("Client chunk size changed to {}", new_chunk_size);
                    }
                    ServerSessionEvent::PublishStreamRequested { app_name, stream_key, .. } => {
                        tracing::info!("Stream published: app={}, stream={}", app_name, stream_key);
                    }
                    ServerSessionEvent::PlayStreamRequested { app_name, stream_key, .. } => {
                        tracing::info!("Stream play requested: app={}, stream={}", app_name, stream_key);
                    }
                    _ => {
                        tracing::debug!("Received event: {:?}", event);
                    }
                }
            }
            _ => {}
        }
    }
    
    true
}
