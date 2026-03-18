use axum::extract::State;
use axum::Json;
use serde::{Deserialize, Serialize};

use crate::config::AppConfig;
use crate::db::DbPool;
use crate::error::AppError;
use crate::services::device_poller;

#[derive(Debug, Serialize)]
pub struct ServerSettings {
    pub log_retention_hours: u64,
    pub poll_interval_secs: u64,
}

#[derive(Debug, Deserialize)]
pub struct UpdateSettings {
    pub log_retention_hours: Option<u64>,
}

pub async fn get_settings(
    State((db, config)): State<(DbPool, AppConfig)>,
) -> Result<Json<ServerSettings>, AppError> {
    let log_retention_hours = {
        let conn = db.lock().unwrap();
        conn.query_row(
            "SELECT value FROM server_settings WHERE key = 'log_retention_hours'",
            [],
            |row| row.get::<_, String>(0),
        )
        .ok()
        .and_then(|v| v.parse().ok())
        .unwrap_or(config.log_retention_hours)
    };

    Ok(Json(ServerSettings {
        log_retention_hours,
        poll_interval_secs: config.poll_interval_secs,
    }))
}

pub async fn update_settings(
    State((db, _config)): State<(DbPool, AppConfig)>,
    Json(input): Json<UpdateSettings>,
) -> Result<Json<serde_json::Value>, AppError> {
    let conn = db.lock().unwrap();

    if let Some(hours) = input.log_retention_hours {
        if hours == 0 {
            return Err(AppError::BadRequest("Retention must be at least 1 hour".into()));
        }
        conn.execute(
            "INSERT INTO server_settings (key, value) VALUES ('log_retention_hours', ?1)
             ON CONFLICT(key) DO UPDATE SET value = ?1",
            [hours.to_string()],
        )?;
    }

    Ok(Json(serde_json::json!({ "ok": true })))
}

pub async fn prune_logs(
    State((db, config)): State<(DbPool, AppConfig)>,
) -> Result<Json<serde_json::Value>, AppError> {
    let deleted = device_poller::prune_logs(&db, config.log_retention_hours);
    Ok(Json(serde_json::json!({ "ok": true, "deleted": deleted })))
}

pub async fn get_log_stats(
    State((db, config)): State<(DbPool, AppConfig)>,
) -> Result<Json<serde_json::Value>, AppError> {
    let conn = db.lock().unwrap();

    let total_count: i64 = conn.query_row(
        "SELECT COUNT(*) FROM status_log", [], |row| row.get(0),
    ).unwrap_or(0);

    let oldest: Option<String> = conn.query_row(
        "SELECT MIN(recorded_at) FROM status_log", [], |row| row.get(0),
    ).ok().flatten();

    let retention_hours: u64 = conn.query_row(
        "SELECT value FROM server_settings WHERE key = 'log_retention_hours'",
        [],
        |row| row.get::<_, String>(0),
    )
    .ok()
    .and_then(|v| v.parse().ok())
    .unwrap_or(config.log_retention_hours);

    let expired_count: i64 = conn.query_row(
        "SELECT COUNT(*) FROM status_log WHERE recorded_at < datetime('now', ?1)",
        [format!("-{} hours", retention_hours)],
        |row| row.get(0),
    ).unwrap_or(0);

    Ok(Json(serde_json::json!({
        "total_entries": total_count,
        "expired_entries": expired_count,
        "oldest_entry": oldest,
        "retention_hours": retention_hours,
    })))
}
