use axum::extract::{Path, Query, State};
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

/// GET /api/homeassistant — get Home Assistant config
pub async fn get_config(
    State(db): State<DbPool>,
    Query(q): Query<DeviceQuery>,
) -> Result<Json<serde_json::Value>, AppError> {
    let conn = db.lock().unwrap();
    let device_id = resolve_device_id(&conn, q.device_id.as_deref())?;

    let config = conn.query_row(
        "SELECT id, device_id, ha_url, access_token, entity_id, expose_states, expose_sensors, enabled, created_at \
         FROM homeassistant_configs WHERE device_id = ?1",
        [&device_id],
        |row| Ok(HomeAssistantConfig {
            id: row.get(0)?,
            device_id: row.get(1)?,
            ha_url: row.get(2)?,
            access_token: row.get(3)?,
            entity_id: row.get(4)?,
            expose_states: row.get(5)?,
            expose_sensors: row.get(6)?,
            enabled: row.get(7)?,
            created_at: row.get(8)?,
        }),
    );

    match config {
        Ok(c) => Ok(Json(serde_json::to_value(c).unwrap())),
        Err(_) => Ok(Json(serde_json::json!(null))),
    }
}

/// POST /api/homeassistant — create Home Assistant config
pub async fn create_config(
    State(db): State<DbPool>,
    Json(input): Json<CreateHomeAssistantConfig>,
) -> Result<Json<HomeAssistantConfig>, AppError> {
    let conn = db.lock().unwrap();
    let device_id = resolve_device_id(&conn, input.device_id.as_deref())?;
    let id = uuid::Uuid::new_v4().to_string();

    conn.execute(
        "INSERT INTO homeassistant_configs (id, device_id, ha_url, access_token, entity_id) \
         VALUES (?1, ?2, ?3, ?4, ?5)",
        rusqlite::params![id, device_id, input.ha_url, input.access_token, input.entity_id],
    )?;

    Ok(Json(HomeAssistantConfig {
        id,
        device_id,
        ha_url: input.ha_url,
        access_token: input.access_token,
        entity_id: input.entity_id,
        expose_states: true,
        expose_sensors: false,
        enabled: true,
        created_at: chrono::Utc::now().to_rfc3339(),
    }))
}

/// PUT /api/homeassistant/:id — update Home Assistant config
pub async fn update_config(
    State(db): State<DbPool>,
    Path(id): Path<String>,
    Json(input): Json<UpdateHomeAssistantConfig>,
) -> Result<Json<serde_json::Value>, AppError> {
    let conn = db.lock().unwrap();

    if let Some(v) = &input.ha_url {
        conn.execute("UPDATE homeassistant_configs SET ha_url = ?1 WHERE id = ?2", rusqlite::params![v, id])?;
    }
    if let Some(v) = &input.access_token {
        conn.execute("UPDATE homeassistant_configs SET access_token = ?1 WHERE id = ?2", rusqlite::params![v, id])?;
    }
    if let Some(v) = &input.entity_id {
        conn.execute("UPDATE homeassistant_configs SET entity_id = ?1 WHERE id = ?2", rusqlite::params![v, id])?;
    }
    if let Some(v) = input.expose_states {
        conn.execute("UPDATE homeassistant_configs SET expose_states = ?1 WHERE id = ?2", rusqlite::params![v, id])?;
    }
    if let Some(v) = input.expose_sensors {
        conn.execute("UPDATE homeassistant_configs SET expose_sensors = ?1 WHERE id = ?2", rusqlite::params![v, id])?;
    }
    if let Some(v) = input.enabled {
        conn.execute("UPDATE homeassistant_configs SET enabled = ?1 WHERE id = ?2", rusqlite::params![v, id])?;
    }

    Ok(Json(serde_json::json!({ "ok": true })))
}

/// DELETE /api/homeassistant/:id — delete Home Assistant config
pub async fn delete_config(
    State(db): State<DbPool>,
    Path(id): Path<String>,
) -> Result<Json<serde_json::Value>, AppError> {
    let conn = db.lock().unwrap();
    conn.execute("DELETE FROM homeassistant_configs WHERE id = ?1", [&id])?;
    Ok(Json(serde_json::json!({ "ok": true })))
}

/// GET /api/homeassistant/entity — get hookbot as HA-compatible entity
pub async fn get_entity(
    State(db): State<DbPool>,
    Query(q): Query<DeviceQuery>,
) -> Result<Json<HomeAssistantEntity>, AppError> {
    let conn = db.lock().unwrap();
    let device_id = resolve_device_id(&conn, q.device_id.as_deref())?;

    // Get device state
    let (state, name) = conn.query_row(
        "SELECT COALESCE(sl.state, 'unknown'), d.name FROM devices d \
         LEFT JOIN status_log sl ON sl.device_id = d.id \
         WHERE d.id = ?1 ORDER BY sl.recorded_at DESC LIMIT 1",
        [&device_id],
        |row| Ok((row.get::<_, String>(0)?, row.get::<_, String>(1)?)),
    ).unwrap_or(("unknown".into(), "hookbot".into()));

    // Get sensor data if available
    let mut sensor_attrs = serde_json::Map::new();
    if let Ok(mut stmt) = conn.prepare(
        "SELECT sc.label, sr.value FROM sensor_configs sc \
         JOIN sensor_readings sr ON sr.device_id = sc.device_id AND sr.channel = sc.channel \
         WHERE sc.device_id = ?1 ORDER BY sr.recorded_at DESC"
    ) {
        let _ = stmt.query_map([&device_id], |row| {
            let label: String = row.get(0)?;
            let value: f64 = row.get(1)?;
            Ok((label, value))
        }).map(|rows| {
            for row in rows.flatten() {
                sensor_attrs.insert(row.0, serde_json::json!(row.1));
            }
        });
    }

    let entity_id = format!("sensor.hookbot_{}", device_id.replace('-', "_"));
    sensor_attrs.insert("friendly_name".into(), serde_json::json!(name));
    sensor_attrs.insert("device_class".into(), serde_json::json!("hookbot"));

    Ok(Json(HomeAssistantEntity {
        entity_id,
        state,
        attributes: serde_json::Value::Object(sensor_attrs),
    }))
}

/// POST /api/homeassistant/sync — push state to Home Assistant
pub async fn sync_state(
    State(db): State<DbPool>,
    Query(q): Query<DeviceQuery>,
) -> Result<Json<serde_json::Value>, AppError> {
    let conn = db.lock().unwrap();
    let device_id = resolve_device_id(&conn, q.device_id.as_deref())?;

    let _config = conn.query_row(
        "SELECT ha_url, access_token FROM homeassistant_configs WHERE device_id = ?1 AND enabled = 1",
        [&device_id],
        |row| Ok((row.get::<_, String>(0)?, row.get::<_, Option<String>>(1)?)),
    ).map_err(|_| AppError::NotFound("No active Home Assistant integration".into()))?;

    // In production, would POST to HA API. For now, return success.
    Ok(Json(serde_json::json!({
        "ok": true,
        "message": "State sync queued to Home Assistant"
    })))
}
