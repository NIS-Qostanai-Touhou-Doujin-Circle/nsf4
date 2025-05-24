use axum::{
    extract::{ws::{Message, Utf8Bytes, WebSocket}, Extension, WebSocketUpgrade},
    response::IntoResponse,
};
use futures::{sink::SinkExt, stream::StreamExt};
use std::sync::Arc;

use crate::services::AppState;

pub async fn handler(
    ws: WebSocketUpgrade,
    Extension(state): Extension<Arc<AppState>>,
) -> impl IntoResponse {
    tracing::info!("WebSocket upgrade requested");
    ws.on_upgrade(|socket| handle_socket(socket, state))
}

async fn handle_socket(socket: WebSocket, state: Arc<AppState>) {
    tracing::info!("WebSocket connection established");
    let (mut sender, mut receiver) = socket.split();
    
    // Simple echo handler
    while let Some(result) = receiver.next().await {
        match result {
            Ok(Message::Text(text)) => {
                tracing::debug!(message = %text, "WebSocket message received");                
                // Echo the message back
                if let Err(e) = sender.send(Message::Text(format!("Echo: {}", text).into())).await {
                    tracing::error!(error = ?e, "Failed to send WebSocket message");
                    break;
                }
            }
            Ok(Message::Close(frame)) => {
                tracing::info!(?frame, "WebSocket close message received");
                break;
            }
            Ok(other) => {
                tracing::debug!(message = ?other, "Non-text WebSocket message received");
            }
            Err(e) => {
                tracing::error!(error = ?e, "WebSocket receive error");
                break;
            }
        }
    }
    
    tracing::info!("WebSocket connection closed");
}
