use std::net::SocketAddr;
use std::sync::Arc;

use axum::{
    extract::Extension,
    routing::{get, post},
    Router,
};
use sqlx::mysql::MySqlPoolOptions;
use tower_http::cors::{Any, CorsLayer};
use tracing_subscriber::{layer::SubscriberExt, util::SubscriberInitExt};

mod api;
mod config;
mod database;
mod models;
mod services;
mod websocket;
mod rtmp;
mod redis;

use api::{feed, drones};

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    // Инициализация трейсинга
    // Настройка файла для логирования
    let file_appender = tracing_appender::rolling::daily("logs", "/app/data/logs/application.log");
    let (non_blocking, _guard) = tracing_appender::non_blocking(file_appender);
    
    // Инициализация трейсинга с выводом в консоль и файл
    tracing_subscriber::registry()
        .with(tracing_subscriber::EnvFilter::new(
            std::env::var("RUST_LOG").unwrap_or_else(|_| "info".into()),
        ))
        .with(tracing_subscriber::fmt::layer()
            .with_writer(std::io::stdout)
            .with_ansi(true))
        .with(tracing_subscriber::fmt::layer()
            .with_writer(non_blocking.clone())
            .with_ansi(false)
            .with_writer(non_blocking)
            .event_format(
                tracing_subscriber::fmt::format()
                    .with_target(true)
                    .compact()
            ))
        .init();
    
    // Сохраняем _guard в состоянии приложения или держим в статической переменной
    // для предотвращения преждевременной очистки
    let _tracing_guard = _guard;    // Загрузка конфигурации из переменных окружения
    let config = config::Config::from_env()?;
    // Логирование загруженной конфигурации для отладки
    tracing::info!(config = ?config, "Конфигурация загружена");

    // Подключение к базе данных - изменено на MySQL
    tracing::info!(database_url = %config.database_url, "Подключение к базе данных");
    let db_pool = MySqlPoolOptions::new()
        .max_connections(10)
        .connect(&config.database_url)
        .await?;
    
    // Запуск миграций
    tracing::info!("Запуск миграций базы данных");
    sqlx::migrate!("./migrations")
        .run(&db_pool)
        .await?;
    tracing::info!("Миграции базы данных завершены");    
    // Подключение к Redis
    tracing::info!(redis_url = %config.redis_url, "Подключение к Redis");
    let redis_client = redis::RedisClient::new(&config.redis_url, config.gps_data_ttl_seconds)
        .map_err(|e| format!("Не удалось подключиться к Redis: {}", e))?;
    
    // Тестирование подключения к Redis
    match redis_client.ping().await {
        Ok(_) => tracing::info!("Подключение к Redis успешно"),
        Err(e) => tracing::error!(error = %e, "Подключение к Redis не удалось"),
    }
    
    // Убеждаемся, что таблица миграций существует перед запуском миграций
    sqlx::query("CREATE TABLE IF NOT EXISTS _sqlx_migrations (
        version BIGINT PRIMARY KEY,
        description TEXT NOT NULL,
        installed_on TIMESTAMP NOT NULL DEFAULT CURRENT_TIMESTAMP,
        success BOOLEAN NOT NULL,
        checksum BLOB NOT NULL,
        execution_time BIGINT NOT NULL
    )")
    .execute(&db_pool)
    .await?;      // Создание общего состояния приложения
    let app_state = Arc::new(services::AppState {
        db: db_pool,
        config: config.clone(),
        redis: redis_client,
    });
      
    // Инициализация RTMP-релеев для существующих дронов
    tracing::info!("Получение существующих дронов для инициализации RTMP-релеев");
    let videos = database::get_videos(&app_state.db).await?;
    tracing::info!(count = videos.len(), "Найдены существующие дроны");
    
    for video in videos {
        let destination = format!("{}/{}", app_state.config.media_server_url, video.id);
        let added = rtmp::add_rtmp_relay(video.id.clone(), video.rtmp_url.clone(), destination.clone(), app_state.db.clone());
        tracing::info!(video_id = %video.id, added = %added, destination = %destination, rtmp_url = %video.rtmp_url, "Инициализирован RTMP-релей для дрона");
    }      // Инициализируем WebSocket подключения к дронам
    tracing::info!("Запуск WebSocket подключений к дронам");
    let app_state_for_clients = app_state.clone();
    tokio::spawn(async move {
        // Запускаем с небольшой задержкой для уверенности, что сервер полностью поднят
        tokio::time::sleep(std::time::Duration::from_secs(2)).await;

        match services::drone_client::start_drone_clients(app_state_for_clients).await {
            Ok(_) => tracing::info!("Инициализация клиентов дронов завершена"),
            Err(e) => tracing::error!(error = %e, "Не удалось инициализировать клиенты дронов"),
        }
    });
    
    // Настройка CORS
    let cors = CorsLayer::new()
        .allow_origin(Any)
        .allow_methods(Any)
        .allow_headers(Any);    // Построение роутера приложения
    let app = Router::new()
        .route("/api/feed", get(feed::get_feed))
        .route("/api/drones", post(drones::add_drone))
        .route("/api/drones/{id}", 
            get(drones::get_drone_by_id)
            .delete(drones::delete_drone)
        )
        .route("/api/rtmp-count", get(feed::get_feed_count))
        .route("/api/ws-count", get(websocket::get_ws_count))
        .route("/api/drones/{id}/revive", post(drones::revive_drone_connection))
        .route("/api/drones/{id}/status", get(drones::get_connection_status))
        .route("/api/analytics/{id}", get(drones::get_analytics_by_id))
        .route("/api/debug/connections", get(drones::get_connection_debug_info))
        .merge(websocket::router()) // Используем новый WebSocket роутер
        .layer(Extension(app_state.clone()))
        .layer(cors);
      
    // Запуск HTTP сервера и RTMP сервера
    let http_addr = SocketAddr::from(([0, 0, 0, 0], config.port));
    
    tracing::info!("HTTP сервер слушает на {}", http_addr);
    
    // Запуск RTMP сервера в фоновом режиме
    let rtmp_addr = SocketAddr::from(([0, 0, 0, 0], config.port + 1));
    tracing::info!("RTMP сервер слушает на {}", rtmp_addr);
    tokio::spawn(async move {
        if let Err(e) = rtmp::start_rtmp_server(rtmp_addr).await {
            tracing::error!(error = %e, "Ошибка RTMP сервера");
        }
    });
    
    // Запуск HTTP сервера
    axum_server::bind(http_addr)
        .serve(app.into_make_service())
        .await?;
    
    Ok(())
}
