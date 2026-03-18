use axum::extract::{Path, State};
use axum::Json;
use serde_json::json;

use crate::db::DbPool;
use crate::error::AppError;
use crate::models::*;
use crate::services::proxy;

fn query_group(conn: &rusqlite::Connection, id: &str) -> Result<DeviceGroup, AppError> {
    let (gid, name, color, created_at) = conn
        .query_row(
            "SELECT id, name, color, created_at FROM device_groups WHERE id = ?1",
            [id],
            |row| Ok((row.get::<_, String>(0)?, row.get::<_, String>(1)?, row.get::<_, String>(2)?, row.get::<_, String>(3)?)),
        )
        .map_err(|_| AppError::NotFound(format!("Group {id} not found")))?;

    let mut stmt = conn.prepare("SELECT device_id FROM device_group_members WHERE group_id = ?1")?;
    let device_ids = stmt
        .query_map([&gid], |row| row.get::<_, String>(0))?
        .collect::<Result<Vec<_>, _>>()?;

    Ok(DeviceGroup {
        id: gid,
        name,
        color,
        created_at,
        device_ids,
    })
}

pub async fn list_groups(State(db): State<DbPool>) -> Result<Json<Vec<DeviceGroup>>, AppError> {
    let conn = db.lock().unwrap();
    let mut stmt = conn.prepare("SELECT id, name, color, created_at FROM device_groups ORDER BY name")?;

    let groups_raw: Vec<(String, String, String, String)> = stmt
        .query_map([], |row| {
            Ok((
                row.get::<_, String>(0)?,
                row.get::<_, String>(1)?,
                row.get::<_, String>(2)?,
                row.get::<_, String>(3)?,
            ))
        })?
        .collect::<Result<Vec<_>, _>>()?;

    let mut groups = Vec::new();
    for (gid, name, color, created_at) in groups_raw {
        let mut mstmt = conn.prepare("SELECT device_id FROM device_group_members WHERE group_id = ?1")?;
        let device_ids = mstmt
            .query_map([&gid], |row| row.get::<_, String>(0))?
            .collect::<Result<Vec<_>, _>>()?;

        groups.push(DeviceGroup {
            id: gid,
            name,
            color,
            created_at,
            device_ids,
        });
    }

    Ok(Json(groups))
}

pub async fn create_group(
    State(db): State<DbPool>,
    Json(input): Json<CreateGroup>,
) -> Result<Json<DeviceGroup>, AppError> {
    let id = uuid::Uuid::new_v4().to_string();
    let color = input.color.unwrap_or_else(|| "#6366f1".to_string());
    let conn = db.lock().unwrap();

    conn.execute(
        "INSERT INTO device_groups (id, name, color) VALUES (?1, ?2, ?3)",
        rusqlite::params![id, input.name, color],
    )?;

    let group = query_group(&conn, &id)?;
    Ok(Json(group))
}

pub async fn update_group(
    State(db): State<DbPool>,
    Path(id): Path<String>,
    Json(input): Json<UpdateGroup>,
) -> Result<Json<DeviceGroup>, AppError> {
    let conn = db.lock().unwrap();

    // Verify exists
    let _ = query_group(&conn, &id)?;

    if let Some(name) = &input.name {
        conn.execute(
            "UPDATE device_groups SET name = ?1 WHERE id = ?2",
            rusqlite::params![name, id],
        )?;
    }
    if let Some(color) = &input.color {
        conn.execute(
            "UPDATE device_groups SET color = ?1 WHERE id = ?2",
            rusqlite::params![color, id],
        )?;
    }

    let group = query_group(&conn, &id)?;
    Ok(Json(group))
}

pub async fn delete_group(
    State(db): State<DbPool>,
    Path(id): Path<String>,
) -> Result<Json<serde_json::Value>, AppError> {
    let conn = db.lock().unwrap();
    let rows = conn.execute("DELETE FROM device_groups WHERE id = ?1", [&id])?;
    if rows == 0 {
        return Err(AppError::NotFound(format!("Group {id} not found")));
    }
    Ok(Json(json!({ "ok": true })))
}

pub async fn add_member(
    State(db): State<DbPool>,
    Path(id): Path<String>,
    Json(input): Json<AddGroupMember>,
) -> Result<Json<DeviceGroup>, AppError> {
    let conn = db.lock().unwrap();

    // Verify group exists
    let _ = query_group(&conn, &id)?;

    // Verify device exists
    let exists: bool = conn
        .query_row(
            "SELECT COUNT(*) FROM devices WHERE id = ?1",
            [&input.device_id],
            |row| row.get::<_, i32>(0),
        )
        .map(|c| c > 0)?;
    if !exists {
        return Err(AppError::NotFound(format!(
            "Device {} not found",
            input.device_id
        )));
    }

    conn.execute(
        "INSERT OR IGNORE INTO device_group_members (group_id, device_id) VALUES (?1, ?2)",
        rusqlite::params![id, input.device_id],
    )?;

    let group = query_group(&conn, &id)?;
    Ok(Json(group))
}

pub async fn remove_member(
    State(db): State<DbPool>,
    Path((id, device_id)): Path<(String, String)>,
) -> Result<Json<serde_json::Value>, AppError> {
    let conn = db.lock().unwrap();
    conn.execute(
        "DELETE FROM device_group_members WHERE group_id = ?1 AND device_id = ?2",
        rusqlite::params![id, device_id],
    )?;
    Ok(Json(json!({ "ok": true })))
}

/// Helper: get IP addresses for all devices in a group
fn query_group_device_ips(conn: &rusqlite::Connection, group_id: &str) -> Result<Vec<(String, String)>, AppError> {
    let mut stmt = conn.prepare(
        "SELECT d.id, d.ip_address FROM devices d
         INNER JOIN device_group_members m ON m.device_id = d.id
         WHERE m.group_id = ?1",
    )?;
    let pairs = stmt
        .query_map([group_id], |row| {
            Ok((row.get::<_, String>(0)?, row.get::<_, String>(1)?))
        })?
        .collect::<Result<Vec<_>, _>>()?;
    Ok(pairs)
}

pub async fn send_group_state(
    State(db): State<DbPool>,
    Path(id): Path<String>,
    Json(input): Json<GroupStateRequest>,
) -> Result<Json<serde_json::Value>, AppError> {
    let device_ips = {
        let conn = db.lock().unwrap();
        // Verify group exists
        let _ = query_group(&conn, &id)?;
        query_group_device_ips(&conn, &id)?
    };

    let body = json!({ "state": input.state });
    let mut successes = 0;
    let mut failures = 0;

    for (_device_id, ip) in &device_ips {
        let url = format!("http://{}/state", ip);
        match proxy::forward_json(&url, &body).await {
            Ok(_) => successes += 1,
            Err(_) => failures += 1,
        }
    }

    Ok(Json(json!({
        "ok": true,
        "state": input.state,
        "devices_total": device_ips.len(),
        "successes": successes,
        "failures": failures,
    })))
}

pub async fn send_group_command(
    State(db): State<DbPool>,
    Path(id): Path<String>,
    Json(input): Json<GroupCommandRequest>,
) -> Result<Json<serde_json::Value>, AppError> {
    let device_ips = {
        let conn = db.lock().unwrap();
        // Verify group exists
        let _ = query_group(&conn, &id)?;
        query_group_device_ips(&conn, &id)?
    };

    let endpoint = input.endpoint.trim_start_matches('/');
    let mut successes = 0;
    let mut failures = 0;

    for (_device_id, ip) in &device_ips {
        let url = format!("http://{}/{}", ip, endpoint);
        match proxy::forward_json(&url, &input.body).await {
            Ok(_) => successes += 1,
            Err(_) => failures += 1,
        }
    }

    Ok(Json(json!({
        "ok": true,
        "endpoint": input.endpoint,
        "devices_total": device_ips.len(),
        "successes": successes,
        "failures": failures,
    })))
}
