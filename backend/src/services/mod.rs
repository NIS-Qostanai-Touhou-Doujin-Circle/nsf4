use sqlx::{Pool, MySql}; // Changed from Postgres to MySql
use std::sync::Arc;

use crate::config::Config;
use crate::models::{Video, Feed};
use crate::database;

pub struct AppState {
    pub db: Pool<MySql>, // Changed from Postgres to MySql
    pub config: Config,
}

pub async fn get_feed(state: Arc<AppState>) -> Result<Feed, sqlx::Error> {
    let videos = database::get_videos(&state.db).await?;
    Ok(Feed { videos })
}

pub async fn add_drone(
    state: Arc<AppState>,
    url: String,
    title: String,
) -> Result<Video, sqlx::Error> {
    database::add_video(&state.db, url, title).await
}

pub async fn delete_drone(
    state: Arc<AppState>,
    id: String,
) -> Result<bool, sqlx::Error> {
    database::delete_video(&state.db, id).await
}

pub async fn get_drone_by_id(
    state: Arc<AppState>,
    id: String,
) -> Result<Option<Video>, sqlx::Error> {
    database::get_video_by_id(&state.db, id).await
}
