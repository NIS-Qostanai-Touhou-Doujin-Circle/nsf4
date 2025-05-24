use axum::{
    extract::{Extension, Path, Json as JsonExtractor},
    http::StatusCode,
    response::Json,
};
use std::sync::Arc;

use crate::models::{AddDroneRequest, AddDroneResponse, DeleteDroneResponse};
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
