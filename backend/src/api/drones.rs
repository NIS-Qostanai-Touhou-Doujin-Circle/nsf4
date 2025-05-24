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
    tracing::info!(url = %payload.url, title = %payload.title, "api::drones::add_drone called");
    let video = services::add_drone(state, payload.url.clone(), payload.title.clone())
        .await
        .map_err(|e| {
            tracing::error!(error = %e, "add_drone service error");
            (StatusCode::INTERNAL_SERVER_ERROR, e.to_string())
        })?;
    
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
