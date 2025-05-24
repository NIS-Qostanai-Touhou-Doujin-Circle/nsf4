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
    ws.on_upgrade(|socket| handle_socket(socket, state))
}

async fn handle_socket(socket: WebSocket, state: Arc<AppState>) {
    let (mut sender, mut receiver) = socket.split();
    
    // Simple echo handler
    while let Some(Ok(msg)) = receiver.next().await {
        if let Message::Text(text) = msg {
            tracing::debug!("Received message: {}", text);
            
            // Echo the message back
            if sender.send(Message::Text(format!("Echo: {}", text).into())).await.is_err() {
                break;
            }
        }
    }
    
    tracing::debug!("WebSocket connection closed");
}
