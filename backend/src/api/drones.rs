use axum::{
    extract::{Extension, Path, Json as JsonExtractor},
    http::StatusCode,
    response::Json,
};
use std::sync::Arc;

use crate::models::{AddDroneRequest, AddDroneResponse, DeleteDroneResponse, Video};
use crate::services::{self, AppState};

pub async fn add_drone(
    Extension(state): Extension<Arc<AppState>>,
    JsonExtractor(payload): JsonExtractor<AddDroneRequest>,
) -> Result<Json<AddDroneResponse>, (StatusCode, String)> {
    tracing::info!(url = %payload.rtmp_url, title = %payload.title, "api::drones::add_drone called");
    let video = services::add_drone(
        state.clone(), 
        payload.title.clone(),
        payload.rtmp_url.clone(),
        payload.ws_url.clone(),
        None, // Pass None for drone_id_override for API-added drones
    )
        .await
        .map_err(|e| {
            tracing::error!(error = %e, "add_drone service error");
            (StatusCode::INTERNAL_SERVER_ERROR, e.to_string())
        })?;      // После добавления дрона инициируем WebSocket-соединение, если URL предоставлен
    let drone_id = video.id.clone();
    let ws_url = payload.ws_url.clone();
    
    // Запускаем соединение в отдельной задаче только если есть WebSocket URL
    if let Some(ws_url) = ws_url.as_ref().filter(|url| !url.trim().is_empty()) {
        let state_clone = state.clone();
        let ws_url = ws_url.clone();
        tokio::spawn(async move {
            tracing::info!(drone_id = %drone_id, url = %ws_url, "Starting WebSocket connection to new drone");
            match services::drone_client::connect_to_drone(state_clone, drone_id.clone(), ws_url).await {
                Ok(_) => tracing::info!(drone_id = %drone_id, "Drone client connection finished"),
                Err(e) => tracing::error!(drone_id = %drone_id, error = %e, "Failed to connect to drone"),
            }
        });
    } else {
        tracing::info!(drone_id = %drone_id, "No WebSocket URL provided, skipping WebSocket connection");
    }let response = AddDroneResponse {
        id: video.id,
        url: video.url,
        title: video.title,
        thumbnail: video.thumbnail,
        created_at: video.created_at,
        rtmp_url: video.rtmp_url,
        ws_url: video.ws_url,
        video_source_name: video.video_source_name,
    };
    
    Ok(Json(response))
}

pub async fn delete_drone(
    Extension(state): Extension<Arc<AppState>>,
    Path(id): Path<String>,
) -> Result<Json<DeleteDroneResponse>, (StatusCode, String)> {
    tracing::info!(drone_id = %id, "api::drones::delete_drone called");
    let success = services::delete_drone(state, id.clone())
        .await
        .map_err(|e| {
            tracing::error!(error = %e, "delete_drone service error");
            (StatusCode::INTERNAL_SERVER_ERROR, e.to_string())
        })?;
    
    Ok(Json(DeleteDroneResponse { success }))
}

pub async fn get_drone_by_id(
    Extension(state): Extension<Arc<AppState>>,
    Path(id): Path<String>,
) -> Result<Json<Video>, (StatusCode, String)> {
    tracing::info!(drone_id = %id, "api::drones::get_drone_by_id called");
    let drone_option = services::get_drone_by_id(state, id.clone())
        .await
        .map_err(|e| {
            tracing::error!(error = %e, "get_drone_by_id service error");
            (StatusCode::INTERNAL_SERVER_ERROR, e.to_string())
        })?;
    
    match drone_option {
        Some(drone) => Ok(Json( drone )),
        None => Err((StatusCode::NOT_FOUND, "Drone not found".to_string())),
    }
}

pub async fn revive_drone_connection(
    Extension(state): Extension<Arc<AppState>>,
    Path(id): Path<String>,
) -> Result<Json<serde_json::Value>, (StatusCode, String)> {
    tracing::info!(drone_id = %id, "api::drones::revive_drone_connection called");
    
    match services::revive_drone_connection(state, id.clone()).await {
        Ok(_) => {
            let response = serde_json::json!({
                "success": true,
                "message": format!("Connection revival initiated for drone {}", id),
                "drone_id": id
            });
            Ok(Json(response))
        },
        Err(e) => {
            tracing::error!(drone_id = %id, error = %e, "revive_drone_connection service error");
            let response = serde_json::json!({
                "success": false,
                "error": e.to_string(),
                "drone_id": id
            });
            Err((StatusCode::INTERNAL_SERVER_ERROR, response.to_string()))
        }
    }
}

pub async fn get_connection_status(
    Extension(_state): Extension<Arc<AppState>>,
    Path(id): Path<String>,
) -> Result<Json<serde_json::Value>, (StatusCode, String)> {
    tracing::info!(drone_id = %id, "api::drones::get_connection_status called");
    
    let is_connected = services::get_drone_connection_status(&id);
    let active_connections = services::get_active_drone_connections();
    
    let response = serde_json::json!({
        "drone_id": id,
        "is_connected": is_connected,
        "active_connections": active_connections.len(),
        "all_active_connections": active_connections
    });
    
    Ok(Json(response))
}
