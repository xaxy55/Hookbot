use axum::extract::{Path, State};
use axum::Json;
use serde::Deserialize;
use serde_json::json;

use crate::db::DbPool;
use crate::error::AppError;
use crate::services::proxy;

#[derive(Debug, Deserialize)]
pub struct NotificationInput {
    pub source: String,
    pub unread: i32,
    pub active: Option<bool>,
}

/// POST /api/devices/:id/notifications - forward notification data to device
pub async fn forward_notification(
    State(db): State<DbPool>,
    Path(id): Path<String>,
    Json(input): Json<NotificationInput>,
) -> Result<Json<serde_json::Value>, AppError> {
    let ip = {
        let conn = db.lock().unwrap();
        conn.query_row(
            "SELECT ip_address FROM devices WHERE id = ?1",
            [&id],
            |row| row.get::<_, String>(0),
        )
        .map_err(|_| AppError::NotFound(format!("Device {id} not found")))?
    };

    let active = input.active.unwrap_or(input.unread > 0);
    let body = json!({
        "source": input.source,
        "unread": input.unread,
        "active": active,
    });

    proxy::forward_json(&format!("http://{}/notifications", ip), &body).await?;

    Ok(Json(json!({
        "ok": true,
        "source": input.source,
        "unread": input.unread,
    })))
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
