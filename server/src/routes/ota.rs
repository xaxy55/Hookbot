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
            // Same rule as the LAN path: the device does the fetching, so this
            // must be an address the device can reach. bind_addr never is.
            let Some(base_url) = config.public_url.clone() else {
                let conn = db.lock().unwrap();
                let _ = conn.execute(
                    "UPDATE ota_jobs SET status = 'failed', \
                     error_msg = 'PUBLIC_URL (or FRONTEND_URL) is not set, so the device cannot be told where to download the firmware', \
                     updated_at = datetime('now') WHERE id = ?1",
                    [&job_id],
                );
                continue;
            };
            let expires_at = chrono::Utc::now().timestamp() + 900;
            let sig = crate::routes::firmware::sign_download(
                &config.session_secret, &firmware_id, expires_at);
            let firmware_url =
                format!("{base_url}/api/firmware/{firmware_id}/binary?exp={expires_at}&sig={sig}");
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

    // The device fetches the image itself, so this URL has to be reachable
    // *from the device*. bind_addr is a listen socket — it was producing
    // http://0.0.0.0:3000/..., which no device can resolve, so every download
    // failed silently while the job still reported success.
    let Some(base) = config.public_url.clone() else {
        update_job_status(
            &db,
            &job_id,
            "failed",
            Some("PUBLIC_URL (or FRONTEND_URL) is not set, so the device cannot be told where to download the firmware"),
        );
        return;
    };
    let expires_at = chrono::Utc::now().timestamp() + 900; // 15 min to download
    let sig = crate::routes::firmware::sign_download(&config.session_secret, &firmware_id, expires_at);
    let firmware_url =
        format!("{base}/api/firmware/{firmware_id}/binary?exp={expires_at}&sig={sig}");

    // The version we expect to see afterwards. Without this there is nothing
    // to check success against.
    let expected_version: Option<String> = {
        let conn = db.lock().unwrap();
        conn.query_row(
            "SELECT version FROM firmware WHERE id = ?1",
            [&firmware_id],
            |row| row.get(0),
        )
        .ok()
    };

    let body = json!({ "url": firmware_url });
    if let Err(e) = proxy::forward_json(&format!("http://{}/ota", ip), &body).await {
        update_job_status(&db, &job_id, "failed", Some(&format!("Failed to send OTA command: {e}")));
        return;
    }

    // Poll for the device to come back on the new version. The old code slept
    // 30s and treated *any* response as success — a device that never updated
    // answers just fine, which is exactly how a failed OTA got reported green.
    let deadline = tokio::time::Instant::now() + tokio::time::Duration::from_secs(180);
    let mut last_seen: Option<String> = None;
    loop {
        tokio::time::sleep(tokio::time::Duration::from_secs(10)).await;

        if let Ok(status) = proxy::get_json(&format!("http://{}/status", ip)).await {
            let reported = status
                .get("firmware_version")
                .and_then(|v| v.as_str())
                .map(|s| s.to_string());

            match (&expected_version, &reported) {
                // Confirmed: the device is running what we sent.
                (Some(want), Some(got)) if want == got => {
                    update_job_status(&db, &job_id, "success", None);
                    return;
                }
                _ => last_seen = reported,
            }
        }

        if tokio::time::Instant::now() >= deadline {
            let detail = match (&expected_version, &last_seen) {
                (Some(want), Some(got)) => format!(
                    "Device is still reporting {got} after the update; expected {want}. \
                     It usually means the device could not download {firmware_url}"
                ),
                _ => "Device did not come back after the update".to_string(),
            };
            update_job_status(&db, &job_id, "failed", Some(&detail));
            return;
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
