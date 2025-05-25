// Импорты для работы с веб-фреймворком Axum
use axum::{
    extract::{Extension, Path, Json as JsonExtractor},
    http::StatusCode,
    response::Json,
};
use std::sync::Arc;

// Импорты моделей данных и сервисов приложения
use crate::{models::{AddDroneRequest, AddDroneResponse, DeleteDroneResponse, Video}, rtmp};
use crate::services::{self, AppState};

/// Добавляет новый дрон в систему
/// 
/// Принимает JSON с данными дрона (название, RTMP URL, WebSocket URL)
/// Возвращает информацию о созданном дроне
pub async fn add_drone(
    Extension(state): Extension<Arc<AppState>>,
    JsonExtractor(payload): JsonExtractor<AddDroneRequest>,
) -> Result<Json<AddDroneResponse>, (StatusCode, String)> {
    tracing::info!(url = %payload.rtmp_url, title = %payload.title, "api::drones::add_drone вызван");
    
    // Вызываем сервис для добавления дрона в базу данных
    let video = services::add_drone(
        state.clone(), 
        payload.title.clone(),
        payload.rtmp_url.clone(),
        payload.ws_url.clone(),
        None,
    )
    .await
    .map_err(|e| {
        tracing::error!(error = %e, "Ошибка сервиса add_drone");
        (StatusCode::INTERNAL_SERVER_ERROR, e.to_string())
    })?;      // После добавления дрона инициируем WebSocket-соединение, если URL предоставлен
    let drone_id = video.id.clone();
    let ws_url = payload.ws_url.clone();
    
    // Запускаем соединение в отдельной задаче только если есть WebSocket URL
    if let Some(ws_url) = ws_url.as_ref().filter(|url| !url.trim().is_empty()) {
        let state_clone = state.clone();
        let ws_url = ws_url.clone();
        tokio::spawn(async move {
            tracing::info!(drone_id = %drone_id, url = %ws_url, "Запуск WebSocket подключения к новому дрону");
            match services::drone_client::connect_to_drone(state_clone, drone_id.clone(), ws_url).await {
                Ok(_) => tracing::info!(drone_id = %drone_id, "Подключение клиента дрона завершено"),
                Err(e) => tracing::error!(drone_id = %drone_id, error = %e, "Не удалось подключиться к дрону"),
            }
        });
    } else {
        tracing::info!(drone_id = %drone_id, "WebSocket URL не предоставлен, пропускаем WebSocket подключение");
    }
    
    // Формируем ответ с данными созданного дрона
    let response = AddDroneResponse {
        id: video.id,
        title: video.title,
        thumbnail: video.thumbnail,
        created_at: video.created_at,
        rtmp_url: video.rtmp_url,
        ws_url: video.ws_url,
    };
    
    Ok(Json(response))
}

/// Удаляет дрон из системы по его ID
/// 
/// Принимает ID дрона как параметр пути
/// Возвращает результат операции удаления
pub async fn delete_drone(
    Extension(state): Extension<Arc<AppState>>,
    Path(id): Path<String>,
) -> Result<Json<DeleteDroneResponse>, (StatusCode, String)> {    tracing::info!(drone_id = %id, "api::drones::delete_drone вызван");
    
    // Вызываем сервис для удаления дрона из базы данных
    let success = services::delete_drone(state, id.clone())
        .await
        .map_err(|e| {
            tracing::error!(error = %e, "Ошибка сервиса delete_drone");
            (StatusCode::INTERNAL_SERVER_ERROR, e.to_string())
        })?;
    
    Ok(Json(DeleteDroneResponse { success }))
}

/// Получает информацию о дроне по его ID
/// 
/// Принимает ID дрона как параметр пути
/// Возвращает данные дрона или ошибку 404, если дрон не найден
pub async fn get_drone_by_id(
    Extension(state): Extension<Arc<AppState>>,
    Path(id): Path<String>,
) -> Result<Json<Video>, (StatusCode, String)> {    tracing::info!(drone_id = %id, "api::drones::get_drone_by_id вызван");
    
    // Получаем данные дрона из базы данных
    let drone_option = services::get_drone_by_id(state, id.clone())
        .await
        .map_err(|e| {
            tracing::error!(error = %e, "Ошибка сервиса get_drone_by_id");
            (StatusCode::INTERNAL_SERVER_ERROR, e.to_string())        })?;
    
    // Проверяем, был ли найден дрон
    match drone_option {
        Some(drone) => Ok(Json(drone)),
        None => Err((StatusCode::NOT_FOUND, "Дрон не найден".to_string())),
    }
}

/// Восстанавливает соединение с дроном
/// 
/// Принимает ID дрона как параметр пути
/// Инициирует повторное подключение к WebSocket дрона
pub async fn revive_drone_connection(
    Extension(state): Extension<Arc<AppState>>,
    Path(id): Path<String>,
) -> Result<Json<serde_json::Value>, (StatusCode, String)> {    tracing::info!(drone_id = %id, "api::drones::revive_drone_connection вызван");
    
    // Пытаемся восстановить соединение с дроном
    match services::revive_drone_connection(state, id.clone()).await {        Ok(_) => {
            // Формируем ответ об успешном восстановлении
            let response = serde_json::json!({
                "success": true,
                "message": format!("Инициировано восстановление соединения для дрона {}", id),
                "drone_id": id
            });
            Ok(Json(response))        },
        Err(e) => {
            // Логируем ошибку и формируем ответ об ошибке
            tracing::error!(drone_id = %id, error = %e, "Ошибка сервиса revive_drone_connection");
            let response = serde_json::json!({
                "success": false,
                "error": e.to_string(),
                "drone_id": id
            });
            Err((StatusCode::INTERNAL_SERVER_ERROR, response.to_string()))
        }
    }
}

/// Получает аналитические данные дрона по его ID
/// 
/// Принимает ID дрона как параметр пути
/// Возвращает статистику и метрики производительности дрона
pub async fn get_analytics_by_id(
    Extension(state): Extension<Arc<AppState>>,
    Path(id): Path<String>,
) -> Result<Json<serde_json::Value>, (StatusCode, String)> {    tracing::info!(drone_id = %id, "api::drones::get_analytics_by_id вызван");
    
    // Получаем аналитические данные дрона из RTMP модуля
    match rtmp::get_drone_analytics_by_id(id.as_str(), &state.db).await {
        Ok(analytics) => Ok(Json(analytics.into())),
        Err(e) => {
            tracing::error!(drone_id = %id, error = %e, "Ошибка сервиса get_analytics_by_id");
            Err((StatusCode::INTERNAL_SERVER_ERROR, e.to_string()))
        }
    }
}

/// Получает статус подключения дрона
/// 
/// Принимает ID дрона как параметр пути
/// Возвращает информацию о состоянии соединения и активных подключениях
pub async fn get_connection_status(
    Extension(_state): Extension<Arc<AppState>>,
    Path(id): Path<String>,
) -> Result<Json<serde_json::Value>, (StatusCode, String)> {    tracing::info!(drone_id = %id, "api::drones::get_connection_status вызван");
    
    // Проверяем статус подключения конкретного дрона
    let is_connected = services::get_drone_connection_status(&id);
    // Получаем список всех активных подключений
    let active_connections = services::get_active_drone_connections();
    
    // Формируем ответ с информацией о подключении
    let response = serde_json::json!({
        "drone_id": id,
        "is_connected": is_connected,
        "active_connections": active_connections.len(),
        "all_active_connections": active_connections
    });
    
    Ok(Json(response))
}

/// Получает отладочную информацию о всех соединениях дронов
/// 
/// Возвращает детальную информацию о всех дронах в системе,
/// их статусах подключения, GPS данных и других метриках
pub async fn get_connection_debug_info(
    Extension(state): Extension<Arc<AppState>>,
) -> Result<Json<serde_json::Value>, (StatusCode, String)> {
    tracing::info!("api::drones::get_connection_debug_info вызван");
    
    // Получаем всех дронов из базы данных
    let drones = match crate::database::get_videos(&state.db).await {
        Ok(drones) => drones,
        Err(e) => {
            tracing::error!(error = %e, "Не удалось получить дронов из базы данных");
            return Err((StatusCode::INTERNAL_SERVER_ERROR, e.to_string()));
        }
    };
    
    let active_connections = services::get_active_drone_connections();
    
    // Создаем детальную информацию о соединении для каждого дрона
    let mut drone_info = Vec::new();
    for drone in drones {
        let is_connected = services::get_drone_connection_status(&drone.id);
        let has_ws_url = drone.ws_url.as_ref().map(|url| !url.trim().is_empty()).unwrap_or(false);
        
        // Получаем последние GPS данные
        let latest_gps = services::get_drone_gps_data(state.clone(), drone.id.clone()).await.ok().flatten();
        
        drone_info.push(serde_json::json!({
            "drone_id": drone.id,
            "title": drone.title,
            "ws_url": drone.ws_url,
            "has_ws_url": has_ws_url,
            "is_connected": is_connected,
            "latest_gps": latest_gps,
            "created_at": drone.created_at
        }));
    }
    
    let response = serde_json::json!({
        "total_drones": drone_info.len(),
        "active_connections_count": active_connections.len(),
        "active_connection_ids": active_connections,
        "drones": drone_info,
        "timestamp": chrono::Utc::now().to_rfc3339()
    });
    
    Ok(Json(response))
}
