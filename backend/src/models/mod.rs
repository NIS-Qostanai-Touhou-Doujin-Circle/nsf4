use serde::{Deserialize, Serialize};
use sqlx::FromRow;

#[derive(Debug, Serialize, Deserialize, FromRow)]
pub struct Video {
    pub id: String,
    pub url: String,
    pub title: String,
    pub thumbnail: String,
    #[sqlx(rename = "createdAt")]
    pub createdAt: String,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct Feed {
    pub videos: Vec<Video>,
}

#[derive(Debug, Deserialize)]
pub struct AddDroneRequest {
    pub url: String,
    pub title: String,
}

#[derive(Debug, Serialize)]
pub struct AddDroneResponse {
    pub id: String,
    pub url: String,
    pub title: String,
    pub thumbnail: String,
    pub createdAt: String,
}

#[derive(Debug, Deserialize)]
pub struct DeleteDroneRequest {
    pub id: String,
}

#[derive(Debug, Serialize)]
pub struct DeleteDroneResponse {
    pub success: bool,
}
