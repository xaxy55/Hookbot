use std::collections::HashMap;
use std::sync::Arc;

use tokio::sync::Mutex;
use tokio::time::Instant;
use tracing::{info, debug};

use crate::db::DbPool;

struct FailureState {
    consecutive_failures: u32,
    next_retry_at: Instant,
}

type FailureMap = Arc<Mutex<HashMap<String, FailureState>>>;

pub fn start(db: DbPool, interval_secs: u64, default_retention_hours: u64) {
    let failures: FailureMap = Arc::new(Mutex::new(HashMap::new()));

    tokio::spawn(async move {
        info!("Device poller started (interval: {interval_secs}s, default retention: {default_retention_hours}h)");
        let mut interval = tokio::time::interval(tokio::time::Duration::from_secs(interval_secs));

        loop {
            interval.tick().await;
            poll_all_devices(&db, default_retention_hours, &failures).await;
        }
    });
}

fn get_retention_hours(db: &DbPool, default: u64) -> u64 {
    let conn = db.lock().unwrap();
    conn.query_row(
        "SELECT value FROM server_settings WHERE key = 'log_retention_hours'",
        [],
        |row| row.get::<_, String>(0),
    )
    .ok()
    .and_then(|v| v.parse().ok())
    .unwrap_or(default)
}

async fn poll_all_devices(db: &DbPool, default_retention_hours: u64, failures: &FailureMap) {
    let retention_hours = get_retention_hours(db, default_retention_hours);

    let devices: Vec<(String, String)> = {
        let conn = db.lock().unwrap();
        let mut stmt = match conn.prepare("SELECT id, ip_address FROM devices") {
            Ok(s) => s,
            Err(_) => return,
        };
        stmt.query_map([], |row| Ok((row.get(0)?, row.get(1)?)))
            .ok()
            .map(|rows| rows.filter_map(|r| r.ok()).collect())
            .unwrap_or_default()
    };

    for (device_id, ip) in devices {
        let db = db.clone();
        let failures = failures.clone();
        tokio::spawn(async move {
            // Check if device is in backoff
            {
                let map = failures.lock().await;
                if let Some(state) = map.get(&device_id) {
                    if Instant::now() < state.next_retry_at {
                        debug!("Skipping {device_id}: in backoff until {:?}", state.next_retry_at);
                        return;
                    }
                }
            }

            let result = poll_device(&db, &device_id, &ip, retention_hours).await;

            match result {
                true => {
                    // Success: remove from failure map
                    let mut map = failures.lock().await;
                    map.remove(&device_id);
                }
                false => {
                    // Failure: increment and set backoff
                    let mut map = failures.lock().await;
                    let entry = map.entry(device_id.clone()).or_insert(FailureState {
                        consecutive_failures: 0,
                        next_retry_at: Instant::now(),
                    });
                    entry.consecutive_failures += 1;
                    let backoff_secs = std::cmp::min(
                        2u64.saturating_pow(entry.consecutive_failures),
                        300,
                    );
                    entry.next_retry_at = Instant::now() + std::time::Duration::from_secs(backoff_secs);
                    debug!(
                        "Device {device_id} poll failed ({} consecutive), next retry in {backoff_secs}s",
                        entry.consecutive_failures
                    );
                }
            }
        });
    }
}

async fn poll_device(db: &DbPool, device_id: &str, ip: &str, retention_hours: u64) -> bool {
    let client = reqwest::Client::builder()
        .timeout(std::time::Duration::from_secs(3))
        .build()
        .unwrap();

    match client.get(format!("http://{ip}/status")).send().await {
        Ok(resp) => {
            if let Ok(json) = resp.json::<serde_json::Value>().await {
                let state = json["state"].as_str().unwrap_or("unknown");
                let uptime = json["uptime"].as_i64().unwrap_or(0);
                let free_heap = json["freeHeap"].as_i64().unwrap_or(0);

                // Update device_type if reported
                let device_type = json["device_type"].as_str();

                let conn = db.lock().unwrap();
                let _ = conn.execute(
                    "INSERT INTO status_log (device_id, state, uptime_ms, free_heap) VALUES (?1, ?2, ?3, ?4)",
                    rusqlite::params![device_id, state, uptime, free_heap],
                );

                // Auto-update device_type from firmware report
                if let Some(dt) = device_type {
                    let _ = conn.execute(
                        "UPDATE devices SET device_type = ?1, updated_at = datetime('now') WHERE id = ?2 AND (device_type IS NULL OR device_type != ?1)",
                        rusqlite::params![dt, device_id],
                    );
                }

                // Update IP address if it changed
                if let Some(ip_from_device) = json["ip"].as_str() {
                    let _ = conn.execute(
                        "UPDATE devices SET ip_address = ?1, updated_at = datetime('now') WHERE id = ?2 AND ip_address != ?1",
                        rusqlite::params![ip_from_device, device_id],
                    );
                }

                // Record sensor readings if present in status response
                if let Some(sensors) = json.get("sensors") {
                    if let Some(arr) = sensors.as_array() {
                        for sensor in arr {
                            let channel = sensor["channel"].as_i64();
                            let value = sensor["value"].as_f64();
                            if let (Some(ch), Some(val)) = (channel, value) {
                                let _ = conn.execute(
                                    "INSERT INTO sensor_readings (device_id, channel, value) VALUES (?1, ?2, ?3)",
                                    rusqlite::params![device_id, ch, val],
                                );
                            }
                        }
                    }
                }

                // Prune entries older than retention period
                let _ = conn.execute(
                    "DELETE FROM status_log WHERE device_id = ?1 AND recorded_at < datetime('now', ?2)",
                    rusqlite::params![device_id, format!("-{} hours", retention_hours)],
                );

                // Prune old sensor readings (same retention)
                let _ = conn.execute(
                    "DELETE FROM sensor_readings WHERE device_id = ?1 AND recorded_at < datetime('now', ?2)",
                    rusqlite::params![device_id, format!("-{} hours", retention_hours)],
                );

                true
            } else {
                false
            }
        }
        Err(e) => {
            debug!("Poll failed for {device_id} at {ip}: {e}");
            false
        }
    }
}

/// Manually prune all status logs older than the configured retention period.
/// Returns the number of deleted rows.
pub fn prune_logs(db: &DbPool, default_retention_hours: u64) -> usize {
    let retention_hours = get_retention_hours(db, default_retention_hours);
    let conn = db.lock().unwrap();
    conn.execute(
        "DELETE FROM status_log WHERE recorded_at < datetime('now', ?1)",
        [format!("-{} hours", retention_hours)],
    )
    .unwrap_or(0)
}
