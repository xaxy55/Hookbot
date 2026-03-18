use axum::extract::State;
use axum::Json;
use tracing::info;

use crate::db::DbPool;
use crate::error::AppError;
use crate::models::DiscoveredDevice;

pub async fn scan(
    State(db): State<DbPool>,
) -> Result<Json<Vec<DiscoveredDevice>>, AppError> {
    let mdns = mdns_sd::ServiceDaemon::new()
        .map_err(|e| AppError::Internal(format!("mDNS init failed: {e}")))?;

    let receiver = mdns.browse("_http._tcp.local.")
        .map_err(|e| AppError::Internal(format!("mDNS browse failed: {e}")))?;

    let mut discovered = Vec::new();

    // Collect results for up to 3 seconds
    let deadline = std::time::Instant::now() + std::time::Duration::from_secs(3);
    while std::time::Instant::now() < deadline {
        match receiver.recv_timeout(std::time::Duration::from_millis(500)) {
            Ok(event) => {
                if let mdns_sd::ServiceEvent::ServiceResolved(info) = event {
                    let hostname = info.get_hostname().trim_end_matches('.').to_string();
                    if hostname.starts_with("hookbot") {
                        let ip = info.get_addresses().iter().next()
                            .map(|a| a.to_string())
                            .unwrap_or_default();
                        let port = info.get_port();

                        // Check if already registered
                        let conn = db.lock().unwrap();
                        let already_registered = conn.query_row(
                            "SELECT COUNT(*) FROM devices WHERE hostname = ?1 OR ip_address = ?2",
                            rusqlite::params![hostname, ip],
                            |row| row.get::<_, i32>(0),
                        ).map(|c| c > 0).unwrap_or(false);

                        discovered.push(DiscoveredDevice {
                            hostname,
                            ip_address: ip,
                            port,
                            already_registered,
                        });
                    }
                }
            }
            Err(_) => continue, // timeout or disconnected
        }
    }

    // Stop browsing before shutdown to avoid channel errors
    let _ = mdns.stop_browse("_http._tcp.local.");
    std::thread::sleep(std::time::Duration::from_millis(100));
    let _ = mdns.shutdown();
    Ok(Json(discovered))
}

/// Reusable auto-discovery: scan mDNS for devices and register any new ones found.
pub async fn discover_and_register(db: &DbPool, prefix: &str) {
    info!("Starting auto-discovery scan (prefix: {prefix})...");

    let mdns = match mdns_sd::ServiceDaemon::new() {
        Ok(m) => m,
        Err(e) => {
            info!("Auto-discovery: mDNS init failed: {e}");
            return;
        }
    };

    let receiver = match mdns.browse("_http._tcp.local.") {
        Ok(r) => r,
        Err(e) => {
            info!("Auto-discovery: mDNS browse failed: {e}");
            return;
        }
    };

    let mut registered = 0u32;
    let deadline = std::time::Instant::now() + std::time::Duration::from_secs(3);

    while std::time::Instant::now() < deadline {
        match receiver.recv_timeout(std::time::Duration::from_millis(500)) {
            Ok(event) => {
                if let mdns_sd::ServiceEvent::ServiceResolved(svc) = event {
                    let hostname = svc.get_hostname().trim_end_matches('.').to_string();
                    if !hostname.starts_with(prefix) {
                        continue;
                    }

                    let ip = match svc.get_addresses().iter().next() {
                        Some(a) => a.to_string(),
                        None => continue,
                    };

                    let conn = db.lock().unwrap();
                    let already: bool = conn
                        .query_row(
                            "SELECT COUNT(*) FROM devices WHERE hostname = ?1 OR ip_address = ?2",
                            rusqlite::params![hostname, ip],
                            |row| row.get::<_, i32>(0),
                        )
                        .map(|c| c > 0)
                        .unwrap_or(false);

                    if !already {
                        let id = uuid::Uuid::new_v4().to_string();
                        let name = hostname.clone();
                        let _ = conn.execute(
                            "INSERT INTO devices (id, name, hostname, ip_address) VALUES (?1, ?2, ?3, ?4)",
                            rusqlite::params![id, name, hostname, ip],
                        );
                        let _ = conn.execute(
                            "INSERT INTO device_config (device_id) VALUES (?1)",
                            [&id],
                        );
                        info!("Auto-discovery: registered new device {hostname} at {ip}");
                        registered += 1;
                    }
                }
            }
            Err(_) => continue,
        }
    }

    let _ = mdns.stop_browse("_http._tcp.local.");
    std::thread::sleep(std::time::Duration::from_millis(100));
    let _ = mdns.shutdown();

    info!("Auto-discovery complete: {registered} new device(s) registered");
}
