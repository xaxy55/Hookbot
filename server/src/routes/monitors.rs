use axum::extract::{Query, State};
use axum::Json;
use serde::Deserialize;

use crate::db::DbPool;
use crate::error::AppError;
use crate::models::*;

#[derive(Debug, Deserialize)]
pub struct DeviceQuery {
    pub device_id: Option<String>,
}

fn resolve_device_id(conn: &rusqlite::Connection, device_id: Option<&str>) -> Result<String, AppError> {
    if let Some(id) = device_id {
        return Ok(id.to_string());
    }
    conn.query_row("SELECT id FROM devices ORDER BY created_at LIMIT 1", [], |row| row.get(0))
        .map_err(|_| AppError::NotFound("No devices registered".into()))
}

/// GET /api/monitors — get multi-monitor config
pub async fn get_config(
    State(db): State<DbPool>,
    Query(q): Query<DeviceQuery>,
) -> Result<Json<MonitorConfig>, AppError> {
    let conn = db.lock().unwrap();
    let device_id = resolve_device_id(&conn, q.device_id.as_deref())?;

    let config = conn.query_row(
        "SELECT id, device_id, monitor_count, servo_pin, angle_map, detection_method, enabled, active_monitor, created_at \
         FROM monitor_configs WHERE device_id = ?1",
        [&device_id],
        |row| {
            let angle_str: String = row.get(4)?;
            Ok(MonitorConfig {
                id: row.get(0)?,
                device_id: row.get(1)?,
                monitor_count: row.get(2)?,
                servo_pin: row.get(3)?,
                angle_map: serde_json::from_str(&angle_str).unwrap_or_default(),
                detection_method: row.get(5)?,
                enabled: row.get(6)?,
                active_monitor: row.get(7)?,
                created_at: row.get(8)?,
            })
        },
    );

    match config {
        Ok(c) => Ok(Json(c)),
        Err(_) => {
            let id = uuid::Uuid::new_v4().to_string();
            let default_angles = serde_json::json!({"0": 45, "1": 90, "2": 135});
            conn.execute(
                "INSERT INTO monitor_configs (id, device_id, angle_map) VALUES (?1, ?2, ?3)",
                rusqlite::params![id, device_id, serde_json::to_string(&default_angles).unwrap()],
            )?;
            Ok(Json(MonitorConfig {
                id,
                device_id,
                monitor_count: 2,
                servo_pin: None,
                angle_map: default_angles,
                detection_method: "manual".into(),
                enabled: true,
                active_monitor: 0,
                created_at: chrono::Utc::now().to_rfc3339(),
            }))
        }
    }
}

/// PUT /api/monitors — update monitor config
pub async fn update_config(
    State(db): State<DbPool>,
    Query(q): Query<DeviceQuery>,
    Json(input): Json<UpdateMonitorConfig>,
) -> Result<Json<serde_json::Value>, AppError> {
    let conn = db.lock().unwrap();
    let device_id = resolve_device_id(&conn, q.device_id.as_deref())?;

    if let Some(v) = input.monitor_count {
        conn.execute("UPDATE monitor_configs SET monitor_count = ?1 WHERE device_id = ?2",
            rusqlite::params![v, device_id])?;
    }
    if let Some(v) = input.servo_pin {
        conn.execute("UPDATE monitor_configs SET servo_pin = ?1 WHERE device_id = ?2",
            rusqlite::params![v, device_id])?;
    }
    if let Some(v) = &input.angle_map {
        let json = serde_json::to_string(v).unwrap();
        conn.execute("UPDATE monitor_configs SET angle_map = ?1 WHERE device_id = ?2",
            rusqlite::params![json, device_id])?;
    }
    if let Some(v) = &input.detection_method {
        conn.execute("UPDATE monitor_configs SET detection_method = ?1 WHERE device_id = ?2",
            rusqlite::params![v, device_id])?;
    }
    if let Some(v) = input.enabled {
        conn.execute("UPDATE monitor_configs SET enabled = ?1 WHERE device_id = ?2",
            rusqlite::params![v, device_id])?;
    }

    Ok(Json(serde_json::json!({ "ok": true })))
}

/// POST /api/monitors/active — set the active monitor (triggers servo movement)
pub async fn set_active_monitor(
    State(db): State<DbPool>,
    Query(q): Query<DeviceQuery>,
    Json(input): Json<SetActiveMonitor>,
) -> Result<Json<serde_json::Value>, AppError> {
    let conn = db.lock().unwrap();
    let device_id = resolve_device_id(&conn, q.device_id.as_deref())?;

    let (count, servo_pin, angle_map_str): (i32, Option<i32>, String) = conn.query_row(
        "SELECT monitor_count, servo_pin, angle_map FROM monitor_configs WHERE device_id = ?1 AND enabled = 1",
        [&device_id],
        |row| Ok((row.get(0)?, row.get(1)?, row.get(2)?)),
    ).map_err(|_| AppError::NotFound("Monitor config not found or disabled".into()))?;

    if input.monitor >= count {
        return Err(AppError::BadRequest(format!(
            "Monitor index {} out of range (0-{})", input.monitor, count - 1
        )));
    }

    conn.execute(
        "UPDATE monitor_configs SET active_monitor = ?1 WHERE device_id = ?2",
        rusqlite::params![input.monitor, device_id],
    )?;

    let angle_map: serde_json::Value = serde_json::from_str(&angle_map_str).unwrap_or_default();
    let target_angle = angle_map
        .get(&input.monitor.to_string())
        .and_then(|v| v.as_i64())
        .unwrap_or(90);

    Ok(Json(serde_json::json!({
        "ok": true,
        "active_monitor": input.monitor,
        "target_angle": target_angle,
        "servo_pin": servo_pin,
        "message": format!("Hookbot now looking at monitor {}", input.monitor)
    })))
}
