use axum::{
    extract::Extension,
    http::StatusCode,
    response::Json,
};
use std::sync::Arc;

use crate::models::Feed;
use crate::services::{self, AppState};

pub async fn get_feed(
    Extension(state): Extension<Arc<AppState>>,
) -> Result<Json<Feed>, (StatusCode, String)> {
    tracing::info!("api::feed::get_feed called");
    let feed = services::get_feed(state)
        .await
        .map_err(|e| (StatusCode::INTERNAL_SERVER_ERROR, e.to_string()))?;
    // Log feed details
    tracing::info!(count = %feed.videos.len(), "Feed fetched");
    Ok(Json(feed))
}
