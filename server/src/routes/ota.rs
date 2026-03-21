use axum::extract::State;
use axum::Json;
use serde_json::json;

use axum::Extension;

use crate::config::AppConfig;
use crate::db::DbPool;
use crate::error::AppError;
use crate::models::*;
use crate::services::command_queue::CommandQueue;
use crate::services::proxy;

pub async fn deploy(
    State((db, config)): State<(DbPool, AppConfig)>,
    Extension(queue): Extension<CommandQueue>,
    Json(input): Json<OtaDeploy>,
) -> Result<Json<Vec<OtaJob>>, AppError> {
    let conn = db.lock().unwrap();

    // Verify firmware exists and get its device_type
    let fw_device_type: Option<String> = conn.query_row(
        "SELECT device_type FROM firmware WHERE id = ?1", [&input.firmware_id],
        |row| row.get(0),
    ).map_err(|_| AppError::NotFound("Firmware not found".into()))?;

    // Validate device types match for each target device
    if let Some(ref fw_type) = fw_device_type {
        for device_id in &input.device_ids {
            let dev_type: Option<String> = conn.query_row(
                "SELECT device_type FROM devices WHERE id = ?1", [device_id],
                |row| row.get(0),
            ).ok().flatten();

            if let Some(ref dt) = dev_type {
                if dt != fw_type {
                    return Err(AppError::BadRequest(format!(
                        "Device type mismatch: firmware is for '{}' but device {} is '{}'",
                        fw_type, &device_id[..8.min(device_id.len())], dt
                    )));
                }
            }
        }
    }

    let mut jobs = Vec::new();
    for device_id in &input.device_ids {
        let job_id = uuid::Uuid::new_v4().to_string();
        conn.execute(
            "INSERT INTO ota_jobs (id, firmware_id, device_id, status) VALUES (?1, ?2, ?3, 'pending')",
            rusqlite::params![job_id, input.firmware_id, device_id],
        )?;
        jobs.push(OtaJob {
            id: job_id,
            firmware_id: input.firmware_id.clone(),
            device_id: device_id.clone(),
            status: "pending".into(),
            created_at: chrono::Utc::now().format("%Y-%m-%d %H:%M:%S").to_string(),
            updated_at: chrono::Utc::now().format("%Y-%m-%d %H:%M:%S").to_string(),
            error_msg: None,
        });
    }
    drop(conn);

    // Spawn OTA tasks — check connection mode for each device
    for job in &jobs {
        let db = db.clone();
        let config = config.clone();
        let job_id = job.id.clone();
        let device_id = job.device_id.clone();
        let firmware_id = job.firmware_id.clone();

        // Check if device is cloud-connected
        let connection_mode: String = {
            let conn = db.lock().unwrap();
            conn.query_row(
                "SELECT COALESCE(connection_mode, 'lan') FROM devices WHERE id = ?1",
                [&device_id],
                |row| row.get(0),
            )
            .unwrap_or_else(|_| "lan".to_string())
        };

        if connection_mode == "cloud" {
            // Cloud device: enqueue OTA command with public firmware URL
            let frontend_url = config.frontend_url.as_deref().unwrap_or("");
            let base_url = if !frontend_url.is_empty() {
                // Derive API URL from frontend URL (hookbot.mr-ai.no -> bot.mr-ai.no)
                format!("https://{}", config.bind_addr)
            } else {
                format!("http://{}", config.bind_addr)
            };
            let firmware_url = format!("{}/api/firmware/{}/binary", base_url, firmware_id);
            let payload = serde_json::json!({ "url": firmware_url });
            let _ = queue.enqueue(&db, &device_id, "ota", &payload);
            let queue_clone = queue.clone();
            let device_id_clone = device_id.clone();
            tokio::spawn(async move {
                queue_clone.notify_device(&device_id_clone).await;
            });
            // Mark job as in_progress (device will pull it)
            let conn = db.lock().unwrap();
            let _ = conn.execute(
                "UPDATE ota_jobs SET status = 'in_progress', updated_at = datetime('now') WHERE id = ?1",
                [&job_id],
            );
        } else {
            // LAN device: direct OTA push
            tokio::spawn(async move {
                execute_ota(db, config, job_id, device_id, firmware_id).await;
            });
        }
    }

    Ok(Json(jobs))
}

async fn execute_ota(db: DbPool, config: AppConfig, job_id: String, device_id: String, firmware_id: String) {
    // Update status to in_progress
    {
        let conn = db.lock().unwrap();
        let _ = conn.execute(
            "UPDATE ota_jobs SET status = 'in_progress', updated_at = datetime('now') WHERE id = ?1",
            [&job_id],
        );
    }

    // Get device IP
    let ip = {
        let conn = db.lock().unwrap();
        conn.query_row("SELECT ip_address FROM devices WHERE id = ?1", [&device_id], |row| row.get::<_, String>(0))
            .unwrap_or_default()
    };

    if ip.is_empty() {
        update_job_status(&db, &job_id, "failed", Some("Device not found"));
        return;
    }

    // Build firmware URL - use the bind address to construct
    let firmware_url = format!("http://{}/api/firmware/{}/binary", config.bind_addr, firmware_id);

    let body = json!({ "url": firmware_url });
    match proxy::forward_json(&format!("http://{}/ota", ip), &body).await {
        Ok(_) => {
            // Wait and check if device comes back with new firmware
            tokio::time::sleep(tokio::time::Duration::from_secs(30)).await;

            match proxy::get_json(&format!("http://{}/info", ip)).await {
                Ok(_info) => {
                    update_job_status(&db, &job_id, "success", None);
                }
                Err(_) => {
                    update_job_status(&db, &job_id, "failed", Some("Device did not respond after OTA"));
                }
            }
        }
        Err(e) => {
            update_job_status(&db, &job_id, "failed", Some(&format!("Failed to send OTA command: {e}")));
        }
    }
}

fn update_job_status(db: &DbPool, job_id: &str, status: &str, error: Option<&str>) {
    let conn = db.lock().unwrap();
    let _ = conn.execute(
        "UPDATE ota_jobs SET status = ?1, error_msg = ?2, updated_at = datetime('now') WHERE id = ?3",
        rusqlite::params![status, error, job_id],
    );
}

pub async fn list_jobs(
    State((db, _config)): State<(DbPool, AppConfig)>,
) -> Result<Json<Vec<OtaJob>>, AppError> {
    let conn = db.lock().unwrap();
    let mut stmt = conn.prepare(
        "SELECT id, firmware_id, device_id, status, created_at, updated_at, error_msg
         FROM ota_jobs ORDER BY created_at DESC LIMIT 50",
    )?;

    let jobs = stmt.query_map([], |row| {
        Ok(OtaJob {
            id: row.get(0)?,
            firmware_id: row.get(1)?,
            device_id: row.get(2)?,
            status: row.get(3)?,
            created_at: row.get(4)?,
            updated_at: row.get(5)?,
            error_msg: row.get(6)?,
        })
    })?.collect::<Result<Vec<_>, _>>()?;

    Ok(Json(jobs))
}
