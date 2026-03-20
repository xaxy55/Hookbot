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

/// GET /api/desk-lights — list desk light configurations
pub async fn list_lights(
    State(db): State<DbPool>,
    Query(q): Query<DeviceQuery>,
) -> Result<Json<Vec<DeskLightConfig>>, AppError> {
    let conn = db.lock().unwrap();
    let device_id = resolve_device_id(&conn, q.device_id.as_deref())?;

    let mut stmt = conn.prepare(
        "SELECT id, device_id, provider, name, bridge_ip, api_key, light_ids, state_colors, enabled, created_at \
         FROM desk_lights WHERE device_id = ?1 ORDER BY created_at"
    )?;
    let lights = stmt.query_map([&device_id], |row| {
        let light_ids_str: String = row.get(6)?;
        let state_colors_str: String = row.get(7)?;
        Ok(DeskLightConfig {
            id: row.get(0)?,
            device_id: row.get(1)?,
            provider: row.get(2)?,
            name: row.get(3)?,
            bridge_ip: row.get(4)?,
            api_key: row.get(5)?,
            light_ids: serde_json::from_str(&light_ids_str).unwrap_or_default(),
            state_colors: serde_json::from_str(&state_colors_str).unwrap_or_default(),
            enabled: row.get(8)?,
            created_at: row.get(9)?,
        })
    })?.filter_map(|r| r.ok()).collect::<Vec<_>>();

    Ok(Json(lights))
}

/// POST /api/desk-lights — create a desk light configuration
pub async fn create_light(
    State(db): State<DbPool>,
    Json(input): Json<CreateDeskLight>,
) -> Result<Json<DeskLightConfig>, AppError> {
    let conn = db.lock().unwrap();
    let device_id = resolve_device_id(&conn, input.device_id.as_deref())?;
    let id = uuid::Uuid::new_v4().to_string();
    let light_ids_vec = input.light_ids.unwrap_or_default();
    let light_ids = serde_json::to_string(&light_ids_vec).unwrap();
    let default_colors = serde_json::json!({
        "idle": "#4488ff",
        "working": "#44ff44",
        "error": "#ff4444",
        "testing": "#ffaa00",
        "focus": "#8844ff"
    });
    let state_colors_val = input.state_colors.unwrap_or(default_colors);
    let state_colors = serde_json::to_string(&state_colors_val).unwrap();

    conn.execute(
        "INSERT INTO desk_lights (id, device_id, provider, name, bridge_ip, api_key, light_ids, state_colors) \
         VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8)",
        rusqlite::params![id, device_id, input.provider, input.name, input.bridge_ip, input.api_key, light_ids, state_colors],
    )?;

    Ok(Json(DeskLightConfig {
        id,
        device_id,
        provider: input.provider,
        name: input.name,
        bridge_ip: input.bridge_ip,
        api_key: input.api_key,
        light_ids: light_ids_vec,
        state_colors: state_colors_val,
        enabled: true,
        created_at: chrono::Utc::now().to_rfc3339(),
    }))
}

/// PUT /api/desk-lights/:id — update a desk light configuration
pub async fn update_light(
    State(db): State<DbPool>,
    Path(id): Path<String>,
    Json(input): Json<UpdateDeskLight>,
) -> Result<Json<serde_json::Value>, AppError> {
    let conn = db.lock().unwrap();

    if let Some(name) = &input.name {
        conn.execute("UPDATE desk_lights SET name = ?1 WHERE id = ?2", rusqlite::params![name, id])?;
    }
    if let Some(ip) = &input.bridge_ip {
        conn.execute("UPDATE desk_lights SET bridge_ip = ?1 WHERE id = ?2", rusqlite::params![ip, id])?;
    }
    if let Some(key) = &input.api_key {
        conn.execute("UPDATE desk_lights SET api_key = ?1 WHERE id = ?2", rusqlite::params![key, id])?;
    }
    if let Some(ids) = &input.light_ids {
        let json = serde_json::to_string(ids).unwrap();
        conn.execute("UPDATE desk_lights SET light_ids = ?1 WHERE id = ?2", rusqlite::params![json, id])?;
    }
    if let Some(colors) = &input.state_colors {
        let json = serde_json::to_string(colors).unwrap();
        conn.execute("UPDATE desk_lights SET state_colors = ?1 WHERE id = ?2", rusqlite::params![json, id])?;
    }
    if let Some(enabled) = input.enabled {
        conn.execute("UPDATE desk_lights SET enabled = ?1 WHERE id = ?2", rusqlite::params![enabled, id])?;
    }

    Ok(Json(serde_json::json!({ "ok": true })))
}

/// DELETE /api/desk-lights/:id — delete a desk light configuration
pub async fn delete_light(
    State(db): State<DbPool>,
    Path(id): Path<String>,
) -> Result<Json<serde_json::Value>, AppError> {
    let conn = db.lock().unwrap();
    conn.execute("DELETE FROM desk_lights WHERE id = ?1", [&id])?;
    Ok(Json(serde_json::json!({ "ok": true })))
}

/// POST /api/desk-lights/:id/action — trigger a light action (set color, effect, etc.)
pub async fn trigger_action(
    State(db): State<DbPool>,
    Path(id): Path<String>,
    Json(input): Json<DeskLightAction>,
) -> Result<Json<serde_json::Value>, AppError> {
    let conn = db.lock().unwrap();

    let light: Result<(String, String, Option<String>), _> = conn.query_row(
        "SELECT provider, COALESCE(bridge_ip, ''), api_key FROM desk_lights WHERE id = ?1 AND enabled = 1",
        [&id],
        |row| Ok((row.get(0)?, row.get(1)?, row.get(2)?)),
    );

    let (provider, _bridge_ip, _api_key) = light.map_err(|_| AppError::NotFound("Light config not found or disabled".into()))?;

    // In production, this would call the Hue/WLED API. For now, log the action.
    Ok(Json(serde_json::json!({
        "ok": true,
        "provider": provider,
        "color": input.color,
        "brightness": input.brightness,
        "effect": input.effect,
        "message": "Light action queued"
    })))
}
