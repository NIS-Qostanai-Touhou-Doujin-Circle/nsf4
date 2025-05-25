// Импорты для работы с веб-фреймворком Axum
use axum::{
    extract::Extension,
    http::StatusCode,
    response::Json,
};
use std::sync::Arc;

// Импорты моделей данных и сервисов приложения
use crate::models::Feed;
use crate::services::{self, AppState};

/// Получает ленту всех видео/дронов
/// 
/// Возвращает полный список всех видео с информацией о дронах
/// Используется для отображения главной страницы с лентой
pub async fn get_feed(
    Extension(state): Extension<Arc<AppState>>,
) -> Result<Json<Feed>, (StatusCode, String)> {
    tracing::info!("api::feed::get_feed called");
    
    // Получаем данные ленты через сервисный слой
    let feed = services::get_feed(state)
        .await
        .map_err(|e| (StatusCode::INTERNAL_SERVER_ERROR, e.to_string()))?;
        
    // Логируем количество видео в ленте для отладки
    tracing::info!(count = %feed.videos.len(), "Feed fetched");
    Ok(Json(feed))
}

/// Получает количество элементов в ленте
/// 
/// Возвращает общее количество видео/дронов в системе
/// Используется для отображения статистики или пагинации
pub async fn get_feed_count(
    Extension(state): Extension<Arc<AppState>>,
) -> Result<Json<usize>, (StatusCode, String)> {
    tracing::info!("api::feed::get_feed_count called");
    
    // Получаем количество элементов в ленте
    let count = services::get_feed_count(state)
        .await
        .map_err(|e| (StatusCode::INTERNAL_SERVER_ERROR, e.to_string()))?;
        
    // Логируем полученное количество
    tracing::info!(count = %count, "Feed count fetched");
    Ok(Json(count))
}