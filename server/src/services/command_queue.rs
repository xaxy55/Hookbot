use std::collections::HashMap;
use std::sync::Arc;

use tokio::sync::Notify;
use tokio::sync::Mutex;
use tracing::{debug, warn};

use crate::db::DbPool;

/// Manages the device command queue with long-poll notification support.
#[derive(Clone)]
pub struct CommandQueue {
    /// Per-device notifiers for waking long-poll connections.
    waiters: Arc<Mutex<HashMap<String, Arc<Notify>>>>,
}

impl CommandQueue {
    pub fn new() -> Self {
        Self {
            waiters: Arc::new(Mutex::new(HashMap::new())),
        }
    }

    /// Enqueue a command for a cloud-connected device.
    pub fn enqueue(
        &self,
        db: &DbPool,
        device_id: &str,
        command_type: &str,
        payload: &serde_json::Value,
    ) -> Result<String, String> {
        let id = uuid::Uuid::new_v4().to_string();
        let payload_str = serde_json::to_string(payload).unwrap_or_else(|_| "{}".to_string());

        let conn = db.lock().unwrap();
        conn.execute(
            "INSERT INTO device_commands (id, device_id, command_type, payload, status, created_at, expires_at)
             VALUES (?1, ?2, ?3, ?4, 'pending', datetime('now'), datetime('now', '+5 minutes'))",
            rusqlite::params![id, device_id, command_type, payload_str],
        )
        .map_err(|e| format!("Failed to enqueue command: {e}"))?;

        debug!("Enqueued command {id} ({command_type}) for device {device_id}");
        Ok(id)
    }

    /// Notify any long-polling connection for this device that a new command is available.
    pub async fn notify_device(&self, device_id: &str) {
        let waiters = self.waiters.lock().await;
        if let Some(notify) = waiters.get(device_id) {
            notify.notify_waiters();
        }
    }

    /// Get or create a Notify handle for a device (used by long-poll endpoint).
    pub async fn get_waiter(&self, device_id: &str) -> Arc<Notify> {
        let mut waiters = self.waiters.lock().await;
        waiters
            .entry(device_id.to_string())
            .or_insert_with(|| Arc::new(Notify::new()))
            .clone()
    }

    /// Fetch pending commands for a device and mark them as delivered.
    pub fn get_pending(&self, db: &DbPool, device_id: &str) -> Vec<serde_json::Value> {
        let conn = db.lock().unwrap();

        // Fetch pending commands that haven't expired
        let mut stmt = match conn.prepare(
            "SELECT id, command_type, payload, created_at
             FROM device_commands
             WHERE device_id = ?1 AND status = 'pending'
               AND (expires_at IS NULL OR expires_at > datetime('now'))
             ORDER BY created_at ASC",
        ) {
            Ok(s) => s,
            Err(e) => {
                warn!("Failed to query pending commands: {e}");
                return vec![];
            }
        };

        let commands: Vec<serde_json::Value> = stmt
            .query_map([device_id], |row| {
                let id: String = row.get(0)?;
                let cmd_type: String = row.get(1)?;
                let payload_str: String = row.get(2)?;
                let created_at: String = row.get(3)?;

                let payload: serde_json::Value =
                    serde_json::from_str(&payload_str).unwrap_or(serde_json::json!({}));

                Ok(serde_json::json!({
                    "id": id,
                    "type": cmd_type,
                    "payload": payload,
                    "created_at": created_at,
                }))
            })
            .ok()
            .map(|rows| rows.filter_map(|r| r.ok()).collect())
            .unwrap_or_default();

        // Mark as delivered
        if !commands.is_empty() {
            let ids: Vec<String> = commands
                .iter()
                .filter_map(|c| c["id"].as_str().map(|s| format!("'{}'", s)))
                .collect();
            let id_list = ids.join(",");
            let _ = conn.execute_batch(&format!(
                "UPDATE device_commands SET status = 'delivered', delivered_at = datetime('now')
                 WHERE id IN ({id_list})"
            ));
        }

        commands
    }

    /// Mark a command as acknowledged.
    pub fn acknowledge(&self, db: &DbPool, command_id: &str) -> bool {
        let conn = db.lock().unwrap();
        let rows = conn
            .execute(
                "UPDATE device_commands SET status = 'acknowledged', acknowledged_at = datetime('now')
                 WHERE id = ?1 AND status = 'delivered'",
                [command_id],
            )
            .unwrap_or(0);
        rows > 0
    }

    /// Clean up expired and old commands.
    pub fn expire_old(&self, db: &DbPool) -> usize {
        let conn = db.lock().unwrap();
        conn.execute(
            "UPDATE device_commands SET status = 'expired'
             WHERE status = 'pending' AND expires_at IS NOT NULL AND expires_at < datetime('now')",
            [],
        )
        .unwrap_or(0)
    }

    /// Mark cloud devices as offline if no heartbeat received within the timeout.
    /// Returns the number of devices marked offline.
    pub fn check_heartbeat_timeouts(&self, db: &DbPool, timeout_secs: u64) -> usize {
        let conn = db.lock().unwrap();

        // Find cloud devices with stale heartbeats and update their status
        let stale_count = conn
            .execute(
                "UPDATE devices SET updated_at = datetime('now')
                 WHERE connection_mode = 'cloud'
                   AND id IN (
                       SELECT dt.device_id FROM device_tokens dt
                       WHERE dt.last_heartbeat_at IS NOT NULL
                         AND dt.last_heartbeat_at < datetime('now', '-' || ?1 || ' seconds')
                   )
                   AND id NOT IN (
                       SELECT device_id FROM status_log
                       WHERE state = 'offline'
                         AND timestamp > datetime('now', '-' || ?1 || ' seconds')
                   )",
                rusqlite::params![timeout_secs],
            )
            .unwrap_or(0);

        // Insert 'offline' status log entries for stale devices
        if stale_count > 0 {
            let _ = conn.execute_batch(&format!(
                "INSERT INTO status_log (device_id, state, uptime_ms, free_heap)
                 SELECT d.id, 'offline', 0, 0
                 FROM devices d
                 JOIN device_tokens dt ON dt.device_id = d.id
                 WHERE d.connection_mode = 'cloud'
                   AND dt.last_heartbeat_at IS NOT NULL
                   AND dt.last_heartbeat_at < datetime('now', '-{timeout_secs} seconds')
                   AND d.id NOT IN (
                       SELECT device_id FROM status_log
                       WHERE state = 'offline'
                         AND timestamp > datetime('now', '-{timeout_secs} seconds')
                   )"
            ));
        }

        stale_count
    }

    /// Start background task to periodically expire old commands and check heartbeat timeouts.
    pub fn start_expiry_task(&self, db: DbPool) {
        let queue = self.clone();
        tokio::spawn(async move {
            let mut interval =
                tokio::time::interval(tokio::time::Duration::from_secs(30));
            loop {
                interval.tick().await;

                // Expire old commands
                let expired = queue.expire_old(&db);
                if expired > 0 {
                    debug!("Expired {expired} old device commands");
                }

                // Check heartbeat timeouts (60 second threshold)
                let timed_out = queue.check_heartbeat_timeouts(&db, 60);
                if timed_out > 0 {
                    debug!("{timed_out} cloud device(s) marked offline (heartbeat timeout)");
                }
            }
        });
    }
}
