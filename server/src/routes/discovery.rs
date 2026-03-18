use axum::extract::State;
use axum::Json;

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
