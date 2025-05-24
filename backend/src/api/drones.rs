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
    let video = services::add_drone(state, payload.url, payload.title)
        .await
        .map_err(|e| (StatusCode::INTERNAL_SERVER_ERROR, e.to_string()))?;
    
    let response = AddDroneResponse {
        id: video.id,
        url: video.url,
        title: video.title,
        thumbnail: video.thumbnail,
        createdAt: video.createdAt,
    };
    
    Ok(Json(response))
}

pub async fn delete_drone(
    Extension(state): Extension<Arc<AppState>>,
    Path(id): Path<String>,
) -> Result<Json<DeleteDroneResponse>, (StatusCode, String)> {
    let success = services::delete_drone(state, id)
        .await
        .map_err(|e| (StatusCode::INTERNAL_SERVER_ERROR, e.to_string()))?;
    
    Ok(Json(DeleteDroneResponse { success }))
}

pub async fn get_drone_by_id(
    Extension(state): Extension<Arc<AppState>>,
    Path(id): Path<String>,
) -> Result<Json<Video>, (StatusCode, String)> {
    let drone_option = services::get_drone_by_id(state, id)
        .await
        .map_err(|e| (StatusCode::INTERNAL_SERVER_ERROR, e.to_string()))?;
    
    match drone_option {
        Some(drone) => Ok(Json( drone )),
        None => Err((StatusCode::NOT_FOUND, "Drone not found".to_string())),
    }
}
