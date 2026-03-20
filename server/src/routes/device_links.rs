use axum::extract::{Path, Query, State};
use axum::Json;
use serde::Deserialize;
use serde_json::json;

use crate::db::DbPool;
use crate::error::AppError;
use crate::models::*;
use crate::services::proxy;

#[derive(Debug, Deserialize)]
pub struct LinkQuery {
    pub device_id: Option<String>,
}

fn query_link(conn: &rusqlite::Connection, id: &str) -> Result<DeviceLink, AppError> {
    conn.query_row(
        "SELECT l.id, l.source_device_id, l.target_device_id, l.trigger_type, l.trigger_config, \
         l.action_type, l.action_config, l.enabled, l.cooldown_secs, l.last_triggered_at, l.created_at, \
         sd.name, td.name \
         FROM device_links l \
         LEFT JOIN devices sd ON sd.id = l.source_device_id \
         LEFT JOIN devices td ON td.id = l.target_device_id \
         WHERE l.id = ?1",
        [id],
        |row| Ok(DeviceLink {
            id: row.get(0)?,
            source_device_id: row.get(1)?,
            target_device_id: row.get(2)?,
            trigger_type: row.get(3)?,
            trigger_config: serde_json::from_str(&row.get::<_, String>(4)?).unwrap_or(json!({})),
            action_type: row.get(5)?,
            action_config: serde_json::from_str(&row.get::<_, String>(6)?).unwrap_or(json!({})),
            enabled: row.get(7)?,
            cooldown_secs: row.get(8)?,
            last_triggered_at: row.get(9)?,
            created_at: row.get(10)?,
            source_device_name: row.get(11)?,
            target_device_name: row.get(12)?,
        }),
    ).map_err(|_| AppError::NotFound(format!("Device link {id} not found")))
}

/// GET /api/device-links — list device-to-device links
pub async fn list_links(
    State(db): State<DbPool>,
    Query(q): Query<LinkQuery>,
) -> Result<Json<Vec<DeviceLink>>, AppError> {
    let conn = db.lock().unwrap();

    let (sql, params): (String, Vec<String>) = if let Some(ref did) = q.device_id {
        (
            "SELECT l.id, l.source_device_id, l.target_device_id, l.trigger_type, l.trigger_config, \
             l.action_type, l.action_config, l.enabled, l.cooldown_secs, l.last_triggered_at, l.created_at, \
             sd.name, td.name \
             FROM device_links l \
             LEFT JOIN devices sd ON sd.id = l.source_device_id \
             LEFT JOIN devices td ON td.id = l.target_device_id \
             WHERE l.source_device_id = ?1 OR l.target_device_id = ?1 \
             ORDER BY l.created_at DESC".into(),
            vec![did.clone()],
        )
    } else {
        (
            "SELECT l.id, l.source_device_id, l.target_device_id, l.trigger_type, l.trigger_config, \
             l.action_type, l.action_config, l.enabled, l.cooldown_secs, l.last_triggered_at, l.created_at, \
             sd.name, td.name \
             FROM device_links l \
             LEFT JOIN devices sd ON sd.id = l.source_device_id \
             LEFT JOIN devices td ON td.id = l.target_device_id \
             ORDER BY l.created_at DESC".into(),
            vec![],
        )
    };

    let param_refs: Vec<&dyn rusqlite::types::ToSql> = params.iter().map(|p| p as &dyn rusqlite::types::ToSql).collect();
    let mut stmt = conn.prepare(&sql)?;
    let rows = stmt.query_map(param_refs.as_slice(), |row| {
        Ok(DeviceLink {
            id: row.get(0)?,
            source_device_id: row.get(1)?,
            target_device_id: row.get(2)?,
            trigger_type: row.get(3)?,
            trigger_config: serde_json::from_str(&row.get::<_, String>(4)?).unwrap_or(json!({})),
            action_type: row.get(5)?,
            action_config: serde_json::from_str(&row.get::<_, String>(6)?).unwrap_or(json!({})),
            enabled: row.get(7)?,
            cooldown_secs: row.get(8)?,
            last_triggered_at: row.get(9)?,
            created_at: row.get(10)?,
            source_device_name: row.get(11)?,
            target_device_name: row.get(12)?,
        })
    })?;

    let links: Vec<DeviceLink> = rows.filter_map(|r| r.ok()).collect();
    Ok(Json(links))
}

/// POST /api/device-links — create a device link
pub async fn create_link(
    State(db): State<DbPool>,
    Json(input): Json<CreateDeviceLink>,
) -> Result<Json<DeviceLink>, AppError> {
    let conn = db.lock().unwrap();
    let id = uuid::Uuid::new_v4().to_string();

    // Verify both devices exist
    for did in [&input.source_device_id, &input.target_device_id] {
        conn.query_row("SELECT id FROM devices WHERE id = ?1", [did], |row| row.get::<_, String>(0))
            .map_err(|_| AppError::NotFound(format!("Device '{did}' not found")))?;
    }

    let trigger_config = serde_json::to_string(&input.trigger_config.unwrap_or(json!({}))).unwrap();
    let action_config = serde_json::to_string(&input.action_config.unwrap_or(json!({}))).unwrap();
    let cooldown = input.cooldown_secs.unwrap_or(30);

    conn.execute(
        "INSERT INTO device_links (id, source_device_id, target_device_id, trigger_type, trigger_config, \
         action_type, action_config, cooldown_secs) VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8)",
        rusqlite::params![id, input.source_device_id, input.target_device_id,
                          input.trigger_type, trigger_config, input.action_type, action_config, cooldown],
    )?;

    let link = query_link(&conn, &id)?;
    Ok(Json(link))
}

/// PUT /api/device-links/:id — update a device link
pub async fn update_link(
    State(db): State<DbPool>,
    Path(id): Path<String>,
    Json(input): Json<UpdateDeviceLink>,
) -> Result<Json<DeviceLink>, AppError> {
    let conn = db.lock().unwrap();
    let _ = query_link(&conn, &id)?;

    if let Some(v) = &input.trigger_type {
        conn.execute("UPDATE device_links SET trigger_type = ?1 WHERE id = ?2", rusqlite::params![v, id])?;
    }
    if let Some(v) = &input.trigger_config {
        let json = serde_json::to_string(v).unwrap();
        conn.execute("UPDATE device_links SET trigger_config = ?1 WHERE id = ?2", rusqlite::params![json, id])?;
    }
    if let Some(v) = &input.action_type {
        conn.execute("UPDATE device_links SET action_type = ?1 WHERE id = ?2", rusqlite::params![v, id])?;
    }
    if let Some(v) = &input.action_config {
        let json = serde_json::to_string(v).unwrap();
        conn.execute("UPDATE device_links SET action_config = ?1 WHERE id = ?2", rusqlite::params![json, id])?;
    }
    if let Some(v) = input.enabled {
        conn.execute("UPDATE device_links SET enabled = ?1 WHERE id = ?2", rusqlite::params![v, id])?;
    }
    if let Some(v) = input.cooldown_secs {
        conn.execute("UPDATE device_links SET cooldown_secs = ?1 WHERE id = ?2", rusqlite::params![v, id])?;
    }

    let link = query_link(&conn, &id)?;
    Ok(Json(link))
}

/// DELETE /api/device-links/:id — delete a device link
pub async fn delete_link(
    State(db): State<DbPool>,
    Path(id): Path<String>,
) -> Result<Json<serde_json::Value>, AppError> {
    let conn = db.lock().unwrap();
    let rows = conn.execute("DELETE FROM device_links WHERE id = ?1", [&id])?;
    if rows == 0 {
        return Err(AppError::NotFound(format!("Device link {id} not found")));
    }
    Ok(Json(json!({ "ok": true })))
}

/// Evaluate device links when a source device changes state.
/// Called from the hook handler after state changes.
pub async fn evaluate_device_links(db: &DbPool, source_device_id: &str, new_state: &str) {
    let links: Vec<(String, String, String, String, serde_json::Value, i64, Option<String>)> = {
        let conn = db.lock().unwrap();
        let mut stmt = match conn.prepare(
            "SELECT l.id, l.target_device_id, l.action_type, l.action_config, d.ip_address, \
             l.cooldown_secs, l.last_triggered_at \
             FROM device_links l \
             JOIN devices d ON d.id = l.target_device_id \
             WHERE l.source_device_id = ?1 AND l.enabled = 1"
        ) {
            Ok(s) => s,
            Err(_) => return,
        };

        let collected: Vec<_> = match stmt.query_map([source_device_id], |row| {
            Ok((
                row.get::<_, String>(0)?,
                row.get::<_, String>(1)?,
                row.get::<_, String>(2)?,
                row.get::<_, String>(3)?,
                row.get::<_, String>(4)?,
                row.get::<_, i64>(5)?,
                row.get::<_, Option<String>>(6)?,
            ))
        }) {
            Ok(rows) => rows.collect(),
            Err(_) => return,
        };
        collected.into_iter().filter_map(|r| {
            let (id, tid, atype, aconfig_str, ip, cooldown, last) = r.ok()?;
            let aconfig: serde_json::Value = serde_json::from_str(&aconfig_str).unwrap_or(json!({}));
            Some((id, tid, atype, ip, aconfig, cooldown, last))
        }).collect()
    };

    let now = chrono::Utc::now();

    for (link_id, _target_id, action_type, target_ip, action_config, cooldown, last_triggered) in links {
        // Check cooldown
        if let Some(ref last) = last_triggered {
            if let Ok(last_time) = chrono::NaiveDateTime::parse_from_str(last, "%Y-%m-%d %H:%M:%S") {
                let elapsed = now.naive_utc().signed_duration_since(last_time).num_seconds();
                if elapsed < cooldown {
                    continue;
                }
            }
        }

        // Check trigger matches (state_change triggers match any state change)
        // Execute the action on target device
        let result = match action_type.as_str() {
            "set_state" => {
                let target_state = action_config.get("state")
                    .and_then(|s| s.as_str())
                    .unwrap_or(new_state);
                let body = json!({ "state": target_state });
                proxy::forward_json(&format!("http://{}/state", target_ip), &body).await
            }
            "play_animation" => {
                proxy::forward_json(&format!("http://{}/animation", target_ip), &action_config).await
            }
            "send_notification" => {
                let msg = action_config.get("message")
                    .and_then(|s| s.as_str())
                    .unwrap_or("Device link triggered");
                let body = json!({ "source": "device_link", "unread": 1, "message": msg });
                proxy::forward_json(&format!("http://{}/notification", target_ip), &body).await
            }
            "relay_state" => {
                let body = json!({ "state": new_state });
                proxy::forward_json(&format!("http://{}/state", target_ip), &body).await
            }
            _ => continue,
        };

        if result.is_ok() {
            // Update last_triggered_at
            let conn = db.lock().unwrap();
            let _ = conn.execute(
                "UPDATE device_links SET last_triggered_at = datetime('now') WHERE id = ?1",
                [&link_id],
            );
        }
    }
}
