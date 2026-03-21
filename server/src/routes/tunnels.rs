use axum::extract::{Path, Query, State};
use axum::{Extension, Json};
use serde::Deserialize;
use serde_json::json;

use crate::db::DbPool;
use crate::error::AppError;
use crate::models::*;
use crate::services::tunnel_manager::TunnelManager;

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

fn get_tunnel_token(conn: &rusqlite::Connection, id: &str) -> Option<String> {
    conn.query_row(
        "SELECT auth_token FROM tunnel_configs WHERE id = ?1",
        [id],
        |row| row.get::<_, Option<String>>(0),
    ).ok().flatten()
}

/// GET /api/tunnels — list tunnel configurations
pub async fn list_tunnels(
    State(db): State<DbPool>,
    Extension(tm): Extension<TunnelManager>,
) -> Result<Json<Vec<serde_json::Value>>, AppError> {
    let tunnels = {
        let conn = db.lock().unwrap();
        let mut stmt = conn.prepare(
            "SELECT id, name, tunnel_type, hostname, port, status, last_connected_at, config, created_at \
             FROM tunnel_configs ORDER BY created_at DESC"
        )?;
        let results = stmt.query_map([], |row| {
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
        results
    };

    // Enrich with live process info
    let mut result = Vec::new();
    for tunnel in tunnels {
        let process_info = tm.get_info(&tunnel.id).await;
        let mut val = serde_json::to_value(&tunnel).unwrap();
        if let Some(info) = process_info {
            val["process"] = json!({
                "pid": info.pid,
                "started_at": info.started_at,
                "restart_count": info.restart_count,
                "assigned_url": info.assigned_url,
                "connected": info.connected,
            });
        }
        result.push(val);
    }

    Ok(Json(result))
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

/// DELETE /api/tunnels/:id — delete a tunnel config (stops process first)
pub async fn delete_tunnel(
    State(db): State<DbPool>,
    Path(id): Path<String>,
    Extension(tm): Extension<TunnelManager>,
) -> Result<Json<serde_json::Value>, AppError> {
    // Stop process if running
    let _ = tm.stop_tunnel_process(&id).await;

    let conn = db.lock().unwrap();
    let rows = conn.execute("DELETE FROM tunnel_configs WHERE id = ?1", [&id])?;
    if rows == 0 {
        return Err(AppError::NotFound(format!("Tunnel '{id}' not found")));
    }
    Ok(Json(json!({ "ok": true })))
}

/// POST /api/tunnels/:id/start — start a tunnel (spawns cloudflared process)
pub async fn start_tunnel(
    State(db): State<DbPool>,
    Path(id): Path<String>,
    Extension(tm): Extension<TunnelManager>,
) -> Result<Json<serde_json::Value>, AppError> {
    let (tunnel, token) = {
        let conn = db.lock().unwrap();
        let tunnel = query_tunnel(&conn, &id)?;
        let token = get_tunnel_token(&conn, &id);
        (tunnel, token)
    };

    if tm.is_running(&id).await {
        return Err(AppError::BadRequest("Tunnel process is already running".into()));
    }

    tm.start_tunnel_process(&id, token.as_deref(), tunnel.port as u16)
        .await
        .map_err(|e| AppError::BadRequest(e))?;

    // Update DB status
    {
        let conn = db.lock().unwrap();
        conn.execute(
            "UPDATE tunnel_configs SET status = 'running', last_connected_at = datetime('now') WHERE id = ?1",
            [&id],
        )?;
    }

    Ok(Json(json!({
        "ok": true,
        "status": "running",
        "message": format!("Tunnel '{}' started on port {}", tunnel.name, tunnel.port),
        "hostname": tunnel.hostname,
    })))
}

/// POST /api/tunnels/:id/stop — stop a tunnel (kills cloudflared process)
pub async fn stop_tunnel(
    State(db): State<DbPool>,
    Path(id): Path<String>,
    Extension(tm): Extension<TunnelManager>,
) -> Result<Json<serde_json::Value>, AppError> {
    {
        let conn = db.lock().unwrap();
        let _ = query_tunnel(&conn, &id)?;
    }

    tm.stop_tunnel_process(&id)
        .await
        .map_err(|e| AppError::BadRequest(e))?;

    {
        let conn = db.lock().unwrap();
        conn.execute(
            "UPDATE tunnel_configs SET status = 'stopped' WHERE id = ?1",
            [&id],
        )?;
    }

    Ok(Json(json!({ "ok": true, "status": "stopped" })))
}

/// POST /api/tunnels/quick-connect — one-click TryCloudflare tunnel (no account needed)
pub async fn quick_connect(
    State(db): State<DbPool>,
    Extension(tm): Extension<TunnelManager>,
) -> Result<Json<serde_json::Value>, AppError> {
    let id = uuid::Uuid::new_v4().to_string();
    let name = format!("quick-{}", &id[..8]);

    {
        let conn = db.lock().unwrap();
        conn.execute(
            "INSERT INTO tunnel_configs (id, name, tunnel_type, hostname, port, config, status) \
             VALUES (?1, ?2, 'cloudflare', NULL, 3000, '{}', 'running')",
            rusqlite::params![id, name],
        )?;
    }

    tm.start_tunnel_process(&id, None, 3000)
        .await
        .map_err(|e| {
            // Clean up the DB entry on failure
            let conn = db.lock().unwrap();
            let _ = conn.execute("DELETE FROM tunnel_configs WHERE id = ?1", [&id]);
            AppError::BadRequest(e)
        })?;

    // Wait briefly for cloudflared to print the URL
    tokio::time::sleep(std::time::Duration::from_secs(3)).await;

    let info = tm.get_info(&id).await;
    let assigned_url = info.as_ref().and_then(|i| i.assigned_url.clone());

    // Store the assigned URL as hostname
    if let Some(ref url) = assigned_url {
        let conn = db.lock().unwrap();
        let _ = conn.execute(
            "UPDATE tunnel_configs SET hostname = ?1 WHERE id = ?2",
            rusqlite::params![url, id],
        );
    }

    Ok(Json(json!({
        "ok": true,
        "id": id,
        "name": name,
        "status": "running",
        "assigned_url": assigned_url,
        "message": "Quick-connect tunnel started. URL may take a few seconds to appear.",
    })))
}

#[derive(Deserialize)]
pub struct LogsQuery {
    pub limit: Option<usize>,
}

/// GET /api/tunnels/:id/logs — get tunnel process logs
pub async fn get_tunnel_logs(
    Path(id): Path<String>,
    Query(q): Query<LogsQuery>,
    Extension(tm): Extension<TunnelManager>,
) -> Result<Json<Vec<TunnelLogEntry>>, AppError> {
    let limit = q.limit.unwrap_or(100);
    let logs = tm.get_logs(&id, limit).await;
    Ok(Json(logs.into_iter().map(|l| TunnelLogEntry {
        timestamp: l.timestamp,
        level: l.level,
        message: l.message,
    }).collect()))
}

/// GET /api/tunnels/:id/metrics — get tunnel process metrics
pub async fn get_tunnel_metrics(
    State(db): State<DbPool>,
    Path(id): Path<String>,
    Extension(tm): Extension<TunnelManager>,
) -> Result<Json<TunnelMetrics>, AppError> {
    let db_status = {
        let conn = db.lock().unwrap();
        let tunnel = query_tunnel(&conn, &id)?;
        tunnel.status
    };

    let info = tm.get_info(&id).await;

    let metrics = match info {
        Some(info) => {
            let started = chrono::DateTime::parse_from_rfc3339(&info.started_at).ok();
            let uptime = started.map(|s| (chrono::Utc::now() - s.with_timezone(&chrono::Utc)).num_seconds());
            TunnelMetrics {
                tunnel_id: id,
                pid: info.pid,
                started_at: Some(info.started_at),
                uptime_secs: uptime,
                restart_count: info.restart_count,
                assigned_url: info.assigned_url,
                connected: info.connected,
                status: "running".to_string(),
            }
        }
        None => TunnelMetrics {
            tunnel_id: id,
            pid: None,
            started_at: None,
            uptime_secs: None,
            restart_count: 0,
            assigned_url: None,
            connected: false,
            status: db_status,
        },
    };

    Ok(Json(metrics))
}
