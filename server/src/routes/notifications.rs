use axum::extract::{Path, State};
use axum::Json;
use serde::Deserialize;
use serde_json::json;

use crate::db::DbPool;
use crate::error::AppError;
use crate::models::{CreateNotification, Notification};
use crate::services::proxy;

/// POST /api/devices/:id/notifications - persist + forward notification data to device
pub async fn forward_notification(
    State(db): State<DbPool>,
    Path(id): Path<String>,
    Json(input): Json<CreateNotification>,
) -> Result<Json<serde_json::Value>, AppError> {
    let unread = input.unread.unwrap_or(0);

    // Insert notification into DB
    let notif_id = {
        let conn = db.lock().unwrap();
        conn.execute(
            "INSERT INTO notifications (device_id, source, unread, message) VALUES (?1, ?2, ?3, ?4)",
            rusqlite::params![id, input.source, unread, input.message],
        )?;
        conn.last_insert_rowid()
    };

    // Look up device IP and forward
    let ip = {
        let conn = db.lock().unwrap();
        conn.query_row(
            "SELECT ip_address FROM devices WHERE id = ?1",
            [&id],
            |row| row.get::<_, String>(0),
        )
        .map_err(|_| AppError::NotFound(format!("Device {id} not found")))?
    };

    let active = input.active.unwrap_or(unread > 0);
    let body = json!({
        "source": input.source,
        "unread": unread,
        "active": active,
    });

    let delivered = proxy::forward_json(&format!("http://{}/notifications", ip), &body)
        .await
        .is_ok();

    // Update delivered status
    if delivered {
        let conn = db.lock().unwrap();
        let _ = conn.execute(
            "UPDATE notifications SET delivered = 1, delivered_at = datetime('now') WHERE id = ?1",
            rusqlite::params![notif_id],
        );
    }

    Ok(Json(json!({
        "ok": true,
        "id": notif_id,
        "source": input.source,
        "unread": unread,
        "delivered": delivered,
    })))
}

/// GET /api/devices/:id/notifications - get last 50 notifications for device
pub async fn get_notifications(
    State(db): State<DbPool>,
    Path(id): Path<String>,
) -> Result<Json<Vec<Notification>>, AppError> {
    let conn = db.lock().unwrap();
    let mut stmt = conn.prepare(
        "SELECT id, device_id, source, unread, message, delivered, created_at, delivered_at
         FROM notifications WHERE device_id = ?1 ORDER BY created_at DESC LIMIT 50",
    )?;

    let notifications = stmt
        .query_map([&id], |row| {
            Ok(Notification {
                id: row.get(0)?,
                device_id: row.get(1)?,
                source: row.get(2)?,
                unread: row.get(3)?,
                message: row.get(4)?,
                delivered: row.get::<_, i32>(5)? != 0,
                created_at: row.get(6)?,
                delivered_at: row.get(7)?,
            })
        })?
        .collect::<Result<Vec<_>, _>>()?;

    Ok(Json(notifications))
}

/// DELETE /api/devices/:id/notifications/:nid - delete a notification by id
pub async fn delete_notification(
    State(db): State<DbPool>,
    Path((id, nid)): Path<(String, i64)>,
) -> Result<Json<serde_json::Value>, AppError> {
    let conn = db.lock().unwrap();
    let rows = conn.execute(
        "DELETE FROM notifications WHERE id = ?1 AND device_id = ?2",
        rusqlite::params![nid, id],
    )?;
    if rows == 0 {
        return Err(AppError::NotFound(format!(
            "Notification {nid} not found for device {id}"
        )));
    }
    Ok(Json(json!({ "ok": true })))
}

/// POST /api/notifications/webhook - incoming webhook from Teams/Slack/etc.
/// Broadcasts to all online devices or a specific device_id
pub async fn webhook_notification(
    State(db): State<DbPool>,
    Json(input): Json<WebhookInput>,
) -> Result<Json<serde_json::Value>, AppError> {
    let source = input.source.unwrap_or_else(|| "teams".to_string());
    let unread = input.unread.unwrap_or(1);
    let active = input.active.unwrap_or(unread > 0);

    let body = json!({
        "source": source,
        "unread": unread,
        "active": active,
    });

    // Find target device(s)
    let ips: Vec<String> = {
        let conn = db.lock().unwrap();
        if let Some(ref device_id) = input.device_id {
            conn.query_row(
                "SELECT ip_address FROM devices WHERE id = ?1",
                [device_id],
                |row| row.get::<_, String>(0),
            )
            .map(|ip| vec![ip])
            .unwrap_or_default()
        } else {
            // Broadcast to all devices
            let mut stmt = conn
                .prepare("SELECT ip_address FROM devices")
                .unwrap();
            stmt.query_map([], |row| row.get::<_, String>(0))
                .unwrap()
                .filter_map(|r| r.ok())
                .collect()
        }
    };

    let mut sent = 0;
    for ip in &ips {
        if proxy::forward_json(&format!("http://{}/notifications", ip), &body)
            .await
            .is_ok()
        {
            sent += 1;
        }
    }

    Ok(Json(json!({
        "ok": true,
        "source": source,
        "unread": unread,
        "devices_notified": sent,
    })))
}

#[derive(Debug, Deserialize)]
pub struct WebhookInput {
    pub source: Option<String>,
    pub unread: Option<i32>,
    pub active: Option<bool>,
    pub device_id: Option<String>,
}
