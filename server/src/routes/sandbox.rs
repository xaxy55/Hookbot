use axum::extract::{Path, Query, State};
use axum::Json;
use serde::Deserialize;

use crate::db::DbPool;
use crate::error::AppError;
use crate::models::*;

#[derive(Debug, Deserialize)]
pub struct SandboxQuery {
    pub device_id: Option<String>,
    pub plugin_id: Option<String>,
}

fn resolve_device_id(conn: &rusqlite::Connection, device_id: Option<&str>) -> Result<String, AppError> {
    if let Some(id) = device_id {
        return Ok(id.to_string());
    }
    conn.query_row("SELECT id FROM devices ORDER BY created_at LIMIT 1", [], |row| row.get(0))
        .map_err(|_| AppError::NotFound("No devices registered".into()))
}

/// GET /api/community/sandboxes — list sandbox configs
pub async fn list_sandboxes(
    State(db): State<DbPool>,
    Query(q): Query<SandboxQuery>,
) -> Result<Json<Vec<PluginSandbox>>, AppError> {
    let conn = db.lock().unwrap();

    let mut sql = String::from(
        "SELECT s.id, s.plugin_id, s.device_id, s.allowed_apis, s.blocked_apis, \
         s.max_calls_per_minute, s.can_access_network, s.can_modify_state, \
         s.can_send_notifications, s.can_access_sensors, s.enabled, s.created_at \
         FROM plugin_sandboxes s WHERE 1=1"
    );
    let mut params: Vec<Box<dyn rusqlite::types::ToSql>> = Vec::new();
    let mut idx = 1;

    if let Some(ref did) = q.device_id {
        sql.push_str(&format!(" AND s.device_id = ?{idx}"));
        params.push(Box::new(did.clone()));
        idx += 1;
    }
    if let Some(ref pid) = q.plugin_id {
        sql.push_str(&format!(" AND s.plugin_id = ?{idx}"));
        params.push(Box::new(pid.clone()));
        let _ = idx;
    }
    sql.push_str(" ORDER BY s.created_at DESC");

    let param_refs: Vec<&dyn rusqlite::types::ToSql> = params.iter().map(|p| p.as_ref()).collect();
    let mut stmt = conn.prepare(&sql)?;
    let rows = stmt.query_map(param_refs.as_slice(), |row| {
        Ok((
            row.get::<_, i64>(0)?,
            row.get::<_, String>(1)?,
            row.get::<_, String>(2)?,
            row.get::<_, String>(3)?,
            row.get::<_, String>(4)?,
            row.get::<_, i64>(5)?,
            row.get::<_, bool>(6)?,
            row.get::<_, bool>(7)?,
            row.get::<_, bool>(8)?,
            row.get::<_, bool>(9)?,
            row.get::<_, bool>(10)?,
            row.get::<_, String>(11)?,
        ))
    })?;

    let mut sandboxes = Vec::new();
    for row in rows {
        let (id, plugin_id, device_id, allowed_json, blocked_json,
             max_calls, net, state, notif, sensors, enabled, created_at) = row?;

        sandboxes.push(PluginSandbox {
            id,
            plugin_id,
            device_id,
            allowed_apis: serde_json::from_str(&allowed_json).unwrap_or_default(),
            blocked_apis: serde_json::from_str(&blocked_json).unwrap_or_default(),
            max_calls_per_minute: max_calls,
            can_access_network: net,
            can_modify_state: state,
            can_send_notifications: notif,
            can_access_sensors: sensors,
            enabled,
            created_at,
        });
    }

    Ok(Json(sandboxes))
}

/// POST /api/community/sandboxes — create sandbox config for a plugin install
pub async fn create_sandbox(
    State(db): State<DbPool>,
    Json(input): Json<CreatePluginSandbox>,
) -> Result<Json<PluginSandbox>, AppError> {
    let conn = db.lock().unwrap();
    let device_id = resolve_device_id(&conn, input.device_id.as_deref())?;

    // Verify plugin exists
    conn.query_row(
        "SELECT id FROM community_plugins WHERE id = ?1",
        [&input.plugin_id],
        |row| row.get::<_, String>(0),
    ).map_err(|_| AppError::NotFound(format!("Plugin '{}' not found", input.plugin_id)))?;

    let allowed = serde_json::to_string(&input.allowed_apis.unwrap_or_default()).unwrap();
    let blocked = serde_json::to_string(&input.blocked_apis.unwrap_or_default()).unwrap();
    let max_calls = input.max_calls_per_minute.unwrap_or(60);
    let net = input.can_access_network.unwrap_or(false);
    let state = input.can_modify_state.unwrap_or(false);
    let notif = input.can_send_notifications.unwrap_or(false);
    let sensors = input.can_access_sensors.unwrap_or(false);

    conn.execute(
        "INSERT OR REPLACE INTO plugin_sandboxes \
         (plugin_id, device_id, allowed_apis, blocked_apis, max_calls_per_minute, \
          can_access_network, can_modify_state, can_send_notifications, can_access_sensors) \
         VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, ?9)",
        rusqlite::params![input.plugin_id, device_id, allowed, blocked, max_calls, net, state, notif, sensors],
    )?;

    let id = conn.last_insert_rowid();

    Ok(Json(PluginSandbox {
        id,
        plugin_id: input.plugin_id,
        device_id,
        allowed_apis: serde_json::from_str(&allowed).unwrap_or_default(),
        blocked_apis: serde_json::from_str(&blocked).unwrap_or_default(),
        max_calls_per_minute: max_calls,
        can_access_network: net,
        can_modify_state: state,
        can_send_notifications: notif,
        can_access_sensors: sensors,
        enabled: true,
        created_at: "just now".into(),
    }))
}

/// PUT /api/community/sandboxes/:id — update sandbox config
pub async fn update_sandbox(
    State(db): State<DbPool>,
    Path(id): Path<i64>,
    Json(input): Json<UpdatePluginSandbox>,
) -> Result<Json<serde_json::Value>, AppError> {
    let conn = db.lock().unwrap();

    // Verify exists
    conn.query_row(
        "SELECT id FROM plugin_sandboxes WHERE id = ?1", [id],
        |row| row.get::<_, i64>(0),
    ).map_err(|_| AppError::NotFound(format!("Sandbox config {id} not found")))?;

    if let Some(apis) = &input.allowed_apis {
        let json = serde_json::to_string(apis).unwrap();
        conn.execute("UPDATE plugin_sandboxes SET allowed_apis = ?1 WHERE id = ?2", rusqlite::params![json, id])?;
    }
    if let Some(apis) = &input.blocked_apis {
        let json = serde_json::to_string(apis).unwrap();
        conn.execute("UPDATE plugin_sandboxes SET blocked_apis = ?1 WHERE id = ?2", rusqlite::params![json, id])?;
    }
    if let Some(v) = input.max_calls_per_minute {
        conn.execute("UPDATE plugin_sandboxes SET max_calls_per_minute = ?1 WHERE id = ?2", rusqlite::params![v, id])?;
    }
    if let Some(v) = input.can_access_network {
        conn.execute("UPDATE plugin_sandboxes SET can_access_network = ?1 WHERE id = ?2", rusqlite::params![v, id])?;
    }
    if let Some(v) = input.can_modify_state {
        conn.execute("UPDATE plugin_sandboxes SET can_modify_state = ?1 WHERE id = ?2", rusqlite::params![v, id])?;
    }
    if let Some(v) = input.can_send_notifications {
        conn.execute("UPDATE plugin_sandboxes SET can_send_notifications = ?1 WHERE id = ?2", rusqlite::params![v, id])?;
    }
    if let Some(v) = input.can_access_sensors {
        conn.execute("UPDATE plugin_sandboxes SET can_access_sensors = ?1 WHERE id = ?2", rusqlite::params![v, id])?;
    }
    if let Some(v) = input.enabled {
        conn.execute("UPDATE plugin_sandboxes SET enabled = ?1 WHERE id = ?2", rusqlite::params![v, id])?;
    }

    Ok(Json(serde_json::json!({ "ok": true, "id": id })))
}

/// DELETE /api/community/sandboxes/:id — delete sandbox config
pub async fn delete_sandbox(
    State(db): State<DbPool>,
    Path(id): Path<i64>,
) -> Result<Json<serde_json::Value>, AppError> {
    let conn = db.lock().unwrap();
    let rows = conn.execute("DELETE FROM plugin_sandboxes WHERE id = ?1", [id])?;
    if rows == 0 {
        return Err(AppError::NotFound(format!("Sandbox config {id} not found")));
    }
    Ok(Json(serde_json::json!({ "ok": true })))
}

/// POST /api/community/sandboxes/check — check if a plugin action is allowed
pub async fn check_permission(
    State(db): State<DbPool>,
    Json(input): Json<CheckPermissionRequest>,
) -> Result<Json<CheckPermissionResponse>, AppError> {
    let conn = db.lock().unwrap();
    let device_id = resolve_device_id(&conn, input.device_id.as_deref())?;

    let result = conn.query_row(
        "SELECT allowed_apis, blocked_apis, can_access_network, can_modify_state, \
         can_send_notifications, can_access_sensors, enabled \
         FROM plugin_sandboxes WHERE plugin_id = ?1 AND device_id = ?2",
        rusqlite::params![input.plugin_id, device_id],
        |row| Ok((
            row.get::<_, String>(0)?,
            row.get::<_, String>(1)?,
            row.get::<_, bool>(2)?,
            row.get::<_, bool>(3)?,
            row.get::<_, bool>(4)?,
            row.get::<_, bool>(5)?,
            row.get::<_, bool>(6)?,
        )),
    );

    match result {
        Ok((allowed_json, blocked_json, net, state, notif, sensors, enabled)) => {
            if !enabled {
                return Ok(Json(CheckPermissionResponse { allowed: false, reason: "Plugin sandbox is disabled".into() }));
            }

            let allowed: Vec<String> = serde_json::from_str(&allowed_json).unwrap_or_default();
            let blocked: Vec<String> = serde_json::from_str(&blocked_json).unwrap_or_default();

            // Check blocked list first
            if blocked.iter().any(|a| a == &input.api) {
                return Ok(Json(CheckPermissionResponse { allowed: false, reason: format!("API '{}' is blocked", input.api) }));
            }

            // Check capability-based permissions
            let cap_allowed = match input.api.as_str() {
                a if a.starts_with("network") => net,
                a if a.starts_with("state") => state,
                a if a.starts_with("notification") => notif,
                a if a.starts_with("sensor") => sensors,
                _ => true,
            };

            if !cap_allowed {
                return Ok(Json(CheckPermissionResponse { allowed: false, reason: format!("Capability for '{}' not granted", input.api) }));
            }

            // If allowed list is non-empty, check whitelist
            if !allowed.is_empty() && !allowed.iter().any(|a| a == &input.api) {
                return Ok(Json(CheckPermissionResponse { allowed: false, reason: format!("API '{}' not in allowed list", input.api) }));
            }

            Ok(Json(CheckPermissionResponse { allowed: true, reason: "Permitted".into() }))
        }
        Err(_) => {
            // No sandbox config = unrestricted (for backwards compatibility)
            Ok(Json(CheckPermissionResponse { allowed: true, reason: "No sandbox configured (unrestricted)".into() }))
        }
    }
}

#[derive(Debug, Deserialize)]
pub struct CheckPermissionRequest {
    pub plugin_id: String,
    pub device_id: Option<String>,
    pub api: String,
}

#[derive(Debug, serde::Serialize)]
pub struct CheckPermissionResponse {
    pub allowed: bool,
    pub reason: String,
}
