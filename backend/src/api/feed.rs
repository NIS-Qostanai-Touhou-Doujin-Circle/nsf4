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
    println!("Fetching feed...");
    let feed = services::get_feed(state)
        .await
        .map_err(|e| (StatusCode::INTERNAL_SERVER_ERROR, e.to_string()))?;
    
    Ok(Json(feed))
}
