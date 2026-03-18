mod config;
mod db;
mod error;
mod models;
mod routes;
mod services;

use axum::routing::{delete, get, post};
use axum::Router;
use tower_http::cors::{Any, CorsLayer};
use tracing::info;

use config::AppConfig;

#[tokio::main]
async fn main() {
    tracing_subscriber::fmt::init();

    let config = AppConfig::from_env();
    info!("Starting hookbot server on {}", config.bind_addr);

    let pool = db::init(&config.database_url);
    info!("Database initialized at {:?}", config.database_url);

    // Start background services
    services::device_poller::start(pool.clone(), config.poll_interval_secs, config.log_retention_hours);

    let cors = CorsLayer::new()
        .allow_origin(Any)
        .allow_methods(Any)
        .allow_headers(Any);

    // Device routes use just the pool
    let device_routes = Router::new()
        .route("/api/devices", get(routes::devices::list_devices).post(routes::devices::create_device))
        .route("/api/devices/{id}", get(routes::devices::get_device).put(routes::devices::update_device).delete(routes::devices::delete_device))
        .route("/api/devices/{id}/state", post(routes::devices::forward_state))
        .route("/api/devices/{id}/tasks", post(routes::devices::forward_tasks))
        .route("/api/devices/{id}/servos", get(routes::devices::get_servos).post(routes::devices::forward_servos))
        .route("/api/devices/{id}/servos/config", post(routes::devices::forward_servo_config))
        .route("/api/devices/{id}/status", get(routes::devices::get_device_status))
        .route("/api/devices/{id}/history", get(routes::devices::get_device_history))
        .route("/api/devices/{id}/config", get(routes::devices::get_config).put(routes::devices::update_config))
        .route("/api/devices/{id}/config/push", post(routes::devices::push_config))
        .route("/api/devices/{id}/notifications", post(routes::notifications::forward_notification))
        .route("/api/notifications/webhook", post(routes::notifications::webhook_notification))
        .route("/api/discovery", get(routes::discovery::scan))
        .route("/api/diagnostics", get(routes::diagnostics::run_diagnostics))
        .route("/api/hook", post(routes::hooks::handle_hook))
        .route("/api/hook/github", post(routes::github::handle_github_hook))
        .route("/api/gamification/stats", get(routes::gamification::get_stats))
        .route("/api/gamification/activity", get(routes::gamification::get_activity))
        .route("/api/gamification/analytics", get(routes::gamification::get_analytics))
        .route("/api/gamification/achievements", get(routes::gamification::get_achievements))
        .route("/api/gamification/leaderboard", get(routes::gamification::get_leaderboard))
        .route("/api/gamification/streaks", get(routes::gamification::get_streaks))
        .with_state(pool.clone());

    // Firmware/OTA/Build routes use both pool and config
    let firmware_routes = Router::new()
        .route("/api/firmware", post(routes::firmware::upload_firmware).get(routes::firmware::list_firmware))
        .route("/api/firmware/{id}/binary", get(routes::firmware::serve_firmware_binary))
        .route("/api/firmware/build", post(routes::build::build_firmware))
        .route("/api/ota/deploy", post(routes::ota::deploy))
        .route("/api/ota/jobs", get(routes::ota::list_jobs))
        .with_state((pool.clone(), config.clone()));

    // Settings & log management routes
    let settings_routes = Router::new()
        .route("/api/settings", get(routes::settings::get_settings).put(routes::settings::update_settings))
        .route("/api/logs/stats", get(routes::settings::get_log_stats))
        .route("/api/logs/prune", delete(routes::settings::prune_logs))
        .with_state((pool.clone(), config.clone()));

    // Store routes
    let store_routes = Router::new()
        .route("/api/store", get(routes::store::list_items))
        .route("/api/store/buy", post(routes::store::buy_item))
        .route("/api/store/owned", get(routes::store::owned_items))
        .with_state(pool.clone());

    let app = Router::new()
        .route("/api/health", get(|| async {
            axum::Json(serde_json::json!({
                "status": "ok",
                "uptime_secs": std::time::SystemTime::now()
                    .duration_since(std::time::UNIX_EPOCH)
                    .unwrap()
                    .as_secs(),
            }))
        }))
        .merge(device_routes)
        .merge(firmware_routes)
        .merge(settings_routes)
        .merge(store_routes)
        .layer(cors);

    let listener = tokio::net::TcpListener::bind(&config.bind_addr).await.unwrap();
    info!("Server listening on {}", config.bind_addr);
    axum::serve(listener, app).await.unwrap();
}
