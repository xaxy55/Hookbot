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

/// GET /api/streamdeck/buttons — list all button configs
pub async fn list_buttons(
    State(db): State<DbPool>,
    Query(q): Query<DeviceQuery>,
) -> Result<Json<Vec<StreamDeckButton>>, AppError> {
    let conn = db.lock().unwrap();
    let device_id = resolve_device_id(&conn, q.device_id.as_deref())?;

    let mut stmt = conn.prepare(
        "SELECT id, device_id, position, label, icon, action_type, action_config, enabled, created_at \
         FROM streamdeck_buttons WHERE device_id = ?1 ORDER BY position"
    )?;
    let buttons = stmt.query_map([&device_id], |row| {
        let config_str: String = row.get(6)?;
        Ok(StreamDeckButton {
            id: row.get(0)?,
            device_id: row.get(1)?,
            position: row.get(2)?,
            label: row.get(3)?,
            icon: row.get(4)?,
            action_type: row.get(5)?,
            action_config: serde_json::from_str(&config_str).unwrap_or_default(),
            enabled: row.get(7)?,
            created_at: row.get(8)?,
        })
    })?.filter_map(|r| r.ok()).collect::<Vec<_>>();

    Ok(Json(buttons))
}

/// POST /api/streamdeck/buttons — create a button config
pub async fn create_button(
    State(db): State<DbPool>,
    Json(input): Json<CreateStreamDeckButton>,
) -> Result<Json<StreamDeckButton>, AppError> {
    let conn = db.lock().unwrap();
    let device_id = resolve_device_id(&conn, input.device_id.as_deref())?;
    let id = uuid::Uuid::new_v4().to_string();
    let action_config_val = input.action_config.unwrap_or(serde_json::json!({}));
    let action_config = serde_json::to_string(&action_config_val).unwrap();

    conn.execute(
        "INSERT INTO streamdeck_buttons (id, device_id, position, label, icon, action_type, action_config) \
         VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7)",
        rusqlite::params![id, device_id, input.position, input.label, input.icon, input.action_type, action_config],
    )?;

    Ok(Json(StreamDeckButton {
        id,
        device_id,
        position: input.position,
        label: input.label,
        icon: input.icon,
        action_type: input.action_type,
        action_config: action_config_val,
        enabled: true,
        created_at: chrono::Utc::now().to_rfc3339(),
    }))
}

/// PUT /api/streamdeck/buttons/:id — update a button
pub async fn update_button(
    State(db): State<DbPool>,
    Path(id): Path<String>,
    Json(input): Json<UpdateStreamDeckButton>,
) -> Result<Json<serde_json::Value>, AppError> {
    let conn = db.lock().unwrap();

    if let Some(v) = &input.label {
        conn.execute("UPDATE streamdeck_buttons SET label = ?1 WHERE id = ?2", rusqlite::params![v, id])?;
    }
    if let Some(v) = &input.icon {
        conn.execute("UPDATE streamdeck_buttons SET icon = ?1 WHERE id = ?2", rusqlite::params![v, id])?;
    }
    if let Some(v) = &input.action_type {
        conn.execute("UPDATE streamdeck_buttons SET action_type = ?1 WHERE id = ?2", rusqlite::params![v, id])?;
    }
    if let Some(v) = &input.action_config {
        let json = serde_json::to_string(v).unwrap();
        conn.execute("UPDATE streamdeck_buttons SET action_config = ?1 WHERE id = ?2", rusqlite::params![json, id])?;
    }
    if let Some(v) = input.enabled {
        conn.execute("UPDATE streamdeck_buttons SET enabled = ?1 WHERE id = ?2", rusqlite::params![v, id])?;
    }

    Ok(Json(serde_json::json!({ "ok": true })))
}

/// DELETE /api/streamdeck/buttons/:id — delete a button
pub async fn delete_button(
    State(db): State<DbPool>,
    Path(id): Path<String>,
) -> Result<Json<serde_json::Value>, AppError> {
    let conn = db.lock().unwrap();
    conn.execute("DELETE FROM streamdeck_buttons WHERE id = ?1", [&id])?;
    Ok(Json(serde_json::json!({ "ok": true })))
}

/// POST /api/streamdeck/trigger — trigger a button action
pub async fn trigger_button(
    State(db): State<DbPool>,
    Json(input): Json<TriggerStreamDeckButton>,
) -> Result<Json<serde_json::Value>, AppError> {
    let conn = db.lock().unwrap();

    let button = conn.query_row(
        "SELECT action_type, action_config, device_id FROM streamdeck_buttons WHERE id = ?1 AND enabled = 1",
        [&input.button_id],
        |row| Ok((row.get::<_, String>(0)?, row.get::<_, String>(1)?, row.get::<_, String>(2)?)),
    ).map_err(|_| AppError::NotFound("Button not found or disabled".into()))?;

    let (action_type, action_config, device_id) = button;

    Ok(Json(serde_json::json!({
        "ok": true,
        "device_id": device_id,
        "action_type": action_type,
        "action_config": serde_json::from_str::<serde_json::Value>(&action_config).unwrap_or_default(),
        "message": format!("Button action '{}' triggered", action_type)
    })))
}
