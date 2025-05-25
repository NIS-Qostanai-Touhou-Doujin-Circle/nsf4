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
