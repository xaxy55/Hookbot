use axum::extract::{Path, State};
use axum::Json;
use serde_json::json;

use crate::db::DbPool;
use crate::error::AppError;
use crate::models::*;

fn query_tunnel(conn: &rusqlite::Connection, id: &str) -> Result<TunnelConfig, AppError> {
    conn.query_row(
        "SELECT id, name, tunnel_type, hostname, port, status, last_connected_at, config, created_at \
         FROM tunnel_configs WHERE id = ?1",
        [id],
        |row| Ok(TunnelConfig {
            id: row.get(0)?,
            name: row.get(1)?,
            tunnel_type: row.get(2)?,
            hostname: row.get(3)?,
            port: row.get(4)?,
            status: row.get(5)?,
            last_connected_at: row.get(6)?,
            config: serde_json::from_str(&row.get::<_, String>(7)?).unwrap_or(json!({})),
            created_at: row.get(8)?,
        }),
    ).map_err(|_| AppError::NotFound(format!("Tunnel '{id}' not found")))
}

/// GET /api/tunnels — list tunnel configurations
pub async fn list_tunnels(
    State(db): State<DbPool>,
) -> Result<Json<Vec<TunnelConfig>>, AppError> {
    let conn = db.lock().unwrap();
    let mut stmt = conn.prepare(
        "SELECT id, name, tunnel_type, hostname, port, status, last_connected_at, config, created_at \
         FROM tunnel_configs ORDER BY created_at DESC"
    )?;
    let tunnels = stmt.query_map([], |row| {
        Ok(TunnelConfig {
            id: row.get(0)?,
            name: row.get(1)?,
            tunnel_type: row.get(2)?,
            hostname: row.get(3)?,
            port: row.get(4)?,
            status: row.get(5)?,
            last_connected_at: row.get(6)?,
            config: serde_json::from_str(&row.get::<_, String>(7)?).unwrap_or(json!({})),
            created_at: row.get(8)?,
        })
    })?.filter_map(|r| r.ok()).collect::<Vec<_>>();

    Ok(Json(tunnels))
}

/// POST /api/tunnels — create a tunnel config
pub async fn create_tunnel(
    State(db): State<DbPool>,
    Json(input): Json<CreateTunnel>,
) -> Result<Json<TunnelConfig>, AppError> {
    let conn = db.lock().unwrap();
    let id = uuid::Uuid::new_v4().to_string();
    let tunnel_type = input.tunnel_type.unwrap_or_else(|| "cloudflare".to_string());
    let port = input.port.unwrap_or(3000);
    let config = serde_json::to_string(&input.config.unwrap_or(json!({}))).unwrap();

    conn.execute(
        "INSERT INTO tunnel_configs (id, name, tunnel_type, hostname, port, auth_token, config) \
         VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7)",
        rusqlite::params![id, input.name, tunnel_type, input.hostname, port, input.auth_token, config],
    )?;

    let tunnel = query_tunnel(&conn, &id)?;
    Ok(Json(tunnel))
}

/// GET /api/tunnels/:id — get a specific tunnel
pub async fn get_tunnel(
    State(db): State<DbPool>,
    Path(id): Path<String>,
) -> Result<Json<TunnelConfig>, AppError> {
    let conn = db.lock().unwrap();
    let tunnel = query_tunnel(&conn, &id)?;
    Ok(Json(tunnel))
}

/// PUT /api/tunnels/:id — update a tunnel config
pub async fn update_tunnel(
    State(db): State<DbPool>,
    Path(id): Path<String>,
    Json(input): Json<UpdateTunnel>,
) -> Result<Json<TunnelConfig>, AppError> {
    let conn = db.lock().unwrap();
    let _ = query_tunnel(&conn, &id)?;

    if let Some(ref name) = input.name {
        conn.execute("UPDATE tunnel_configs SET name = ?1 WHERE id = ?2", rusqlite::params![name, id])?;
    }
    if let Some(ref hostname) = input.hostname {
        conn.execute("UPDATE tunnel_configs SET hostname = ?1 WHERE id = ?2", rusqlite::params![hostname, id])?;
    }
    if let Some(port) = input.port {
        conn.execute("UPDATE tunnel_configs SET port = ?1 WHERE id = ?2", rusqlite::params![port, id])?;
    }
    if let Some(ref token) = input.auth_token {
        conn.execute("UPDATE tunnel_configs SET auth_token = ?1 WHERE id = ?2", rusqlite::params![token, id])?;
    }
    if let Some(ref cfg) = input.config {
        let json = serde_json::to_string(cfg).unwrap();
        conn.execute("UPDATE tunnel_configs SET config = ?1 WHERE id = ?2", rusqlite::params![json, id])?;
    }

    let tunnel = query_tunnel(&conn, &id)?;
    Ok(Json(tunnel))
}

/// DELETE /api/tunnels/:id — delete a tunnel config
pub async fn delete_tunnel(
    State(db): State<DbPool>,
    Path(id): Path<String>,
) -> Result<Json<serde_json::Value>, AppError> {
    let conn = db.lock().unwrap();
    let rows = conn.execute("DELETE FROM tunnel_configs WHERE id = ?1", [&id])?;
    if rows == 0 {
        return Err(AppError::NotFound(format!("Tunnel '{id}' not found")));
    }
    Ok(Json(json!({ "ok": true })))
}

/// POST /api/tunnels/:id/start — start a tunnel
pub async fn start_tunnel(
    State(db): State<DbPool>,
    Path(id): Path<String>,
) -> Result<Json<serde_json::Value>, AppError> {
    let conn = db.lock().unwrap();
    let tunnel = query_tunnel(&conn, &id)?;

    if tunnel.status == "running" {
        return Err(AppError::BadRequest("Tunnel is already running".into()));
    }

    conn.execute(
        "UPDATE tunnel_configs SET status = 'running', last_connected_at = datetime('now') WHERE id = ?1",
        [&id],
    )?;

    Ok(Json(json!({
        "ok": true,
        "status": "running",
        "message": format!("Tunnel '{}' started on port {}", tunnel.name, tunnel.port),
        "hostname": tunnel.hostname,
    })))
}

/// POST /api/tunnels/:id/stop — stop a tunnel
pub async fn stop_tunnel(
    State(db): State<DbPool>,
    Path(id): Path<String>,
) -> Result<Json<serde_json::Value>, AppError> {
    let conn = db.lock().unwrap();
    let _ = query_tunnel(&conn, &id)?;

    conn.execute(
        "UPDATE tunnel_configs SET status = 'stopped' WHERE id = ?1",
        [&id],
    )?;

    Ok(Json(json!({ "ok": true, "status": "stopped" })))
}
