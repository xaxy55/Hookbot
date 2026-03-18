use axum::extract::{Path, State};
use axum::Json;
use serde_json::json;

use crate::db::DbPool;
use crate::error::AppError;
use crate::models::{SensorConfig, UpdateSensorConfig};
use crate::services::proxy;

/// GET /api/devices/:id/sensors - query sensor configs for device
pub async fn get_sensors(
    State(db): State<DbPool>,
    Path(id): Path<String>,
) -> Result<Json<serde_json::Value>, AppError> {
    let (configs, ip) = {
        let conn = db.lock().unwrap();
        let mut stmt = conn.prepare(
            "SELECT id, device_id, channel, pin, sensor_type, label, poll_interval_ms, threshold
             FROM sensor_configs WHERE device_id = ?1 ORDER BY channel",
        )?;

        let configs: Vec<SensorConfig> = stmt
            .query_map([&id], |row| {
                Ok(SensorConfig {
                    id: row.get(0)?,
                    device_id: row.get(1)?,
                    channel: row.get(2)?,
                    pin: row.get(3)?,
                    sensor_type: row.get(4)?,
                    label: row.get(5)?,
                    poll_interval_ms: row.get(6)?,
                    threshold: row.get(7)?,
                })
            })?
            .collect::<Result<Vec<_>, _>>()?;

        let ip: Option<String> = conn
            .query_row(
                "SELECT ip_address FROM devices WHERE id = ?1",
                [&id],
                |row| row.get::<_, String>(0),
            )
            .ok();

        (configs, ip)
    };

    // Optionally try to get live readings from device
    let live_readings = if let Some(ip) = ip {
        proxy::get_json(&format!("http://{}/sensors", ip)).await.ok()
    } else {
        None
    };

    Ok(Json(json!({
        "configs": configs,
        "live_readings": live_readings,
    })))
}

/// PUT /api/devices/:id/sensors - upsert sensor configs, push to device
pub async fn update_sensors(
    State(db): State<DbPool>,
    Path(id): Path<String>,
    Json(body): Json<UpdateSensorConfig>,
) -> Result<Json<serde_json::Value>, AppError> {
    let ip = {
        let conn = db.lock().unwrap();
        for ch in &body.channels {
            conn.execute(
                "INSERT INTO sensor_configs (device_id, channel, pin, sensor_type, label, poll_interval_ms, threshold)
                 VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7)
                 ON CONFLICT(device_id, channel) DO UPDATE SET
                     pin = excluded.pin,
                     sensor_type = excluded.sensor_type,
                     label = excluded.label,
                     poll_interval_ms = COALESCE(excluded.poll_interval_ms, sensor_configs.poll_interval_ms),
                     threshold = COALESCE(excluded.threshold, sensor_configs.threshold)",
                rusqlite::params![
                    id,
                    ch.channel,
                    ch.pin,
                    ch.sensor_type,
                    ch.label,
                    ch.poll_interval_ms.unwrap_or(1000),
                    ch.threshold.unwrap_or(0),
                ],
            )?;
        }

        conn.query_row(
            "SELECT ip_address FROM devices WHERE id = ?1",
            [&id],
            |row| row.get::<_, String>(0),
        )
        .ok()
    };

    // Try to push config to device
    let pushed = if let Some(ip) = ip {
        let payload = json!({ "channels": body.channels });
        proxy::forward_json(&format!("http://{}/sensors/config", ip), &payload)
            .await
            .is_ok()
    } else {
        false
    };

    Ok(Json(json!({
        "ok": true,
        "pushed_to_device": pushed,
    })))
}
