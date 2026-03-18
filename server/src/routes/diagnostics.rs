use axum::extract::State;
use axum::Json;
use serde::Serialize;
use std::time::Instant;

use crate::db::DbPool;
use crate::error::AppError;

#[derive(Serialize)]
pub struct DiagResult {
    pub checks: Vec<Check>,
    pub overall: String,
}

#[derive(Serialize)]
pub struct Check {
    pub name: String,
    pub status: String,       // "pass", "fail", "warn"
    pub message: String,
    pub latency_ms: Option<u64>,
}

pub async fn run_diagnostics(
    State(db): State<DbPool>,
) -> Result<Json<DiagResult>, AppError> {
    let mut checks = Vec::new();

    // 1. Database check
    checks.push(check_database(&db));

    // 2. Gather device IPs for per-device checks
    let devices: Vec<(String, String, String)> = {
        let conn = db.lock().unwrap();
        let mut stmt = conn.prepare("SELECT id, name, ip_address FROM devices").unwrap();
        stmt.query_map([], |row| Ok((row.get(0)?, row.get(1)?, row.get(2)?)))
            .ok()
            .map(|rows| rows.filter_map(|r| r.ok()).collect())
            .unwrap_or_default()
    };

    // 3. Per-device checks (ping, HTTP, DNS)
    for (_id, name, ip) in &devices {
        let device_checks = check_device(&name, &ip).await;
        checks.extend(device_checks);
    }

    // 4. If no devices, note that
    if devices.is_empty() {
        checks.push(Check {
            name: "devices".into(),
            status: "warn".into(),
            message: "No devices registered".into(),
            latency_ms: None,
        });
    }

    // 5. Filesystem check (firmware dir)
    checks.push(check_firmware_dir());

    let overall = if checks.iter().any(|c| c.status == "fail") {
        "fail"
    } else if checks.iter().any(|c| c.status == "warn") {
        "warn"
    } else {
        "pass"
    };

    Ok(Json(DiagResult {
        checks,
        overall: overall.into(),
    }))
}

fn check_database(db: &DbPool) -> Check {
    let start = Instant::now();
    let result = {
        let conn = db.lock().unwrap();
        conn.query_row("SELECT COUNT(*) FROM devices", [], |row| row.get::<_, i32>(0))
    };
    let latency = start.elapsed().as_millis() as u64;

    match result {
        Ok(count) => Check {
            name: "database".into(),
            status: "pass".into(),
            message: format!("SQLite OK - {count} device(s)"),
            latency_ms: Some(latency),
        },
        Err(e) => Check {
            name: "database".into(),
            status: "fail".into(),
            message: format!("SQLite error: {e}"),
            latency_ms: Some(latency),
        },
    }
}

async fn check_device(name: &str, ip: &str) -> Vec<Check> {
    let mut checks = Vec::new();

    // TCP connect check (port 80)
    let start = Instant::now();
    let tcp_result = tokio::time::timeout(
        std::time::Duration::from_secs(3),
        tokio::net::TcpStream::connect(format!("{ip}:80")),
    ).await;
    let latency = start.elapsed().as_millis() as u64;

    match tcp_result {
        Ok(Ok(_)) => {
            checks.push(Check {
                name: format!("{name} - tcp:80"),
                status: "pass".into(),
                message: format!("Port 80 open on {ip}"),
                latency_ms: Some(latency),
            });
        }
        Ok(Err(e)) => {
            checks.push(Check {
                name: format!("{name} - tcp:80"),
                status: "fail".into(),
                message: format!("Connection refused: {e}"),
                latency_ms: Some(latency),
            });
            return checks; // Skip HTTP check if TCP fails
        }
        Err(_) => {
            checks.push(Check {
                name: format!("{name} - tcp:80"),
                status: "fail".into(),
                message: format!("Connection timeout ({ip}:80)"),
                latency_ms: Some(latency),
            });
            return checks;
        }
    }

    // HTTP /status check
    let start = Instant::now();
    let client = reqwest::Client::builder()
        .timeout(std::time::Duration::from_secs(3))
        .build()
        .unwrap();

    match client.get(format!("http://{ip}/status")).send().await {
        Ok(resp) => {
            let latency = start.elapsed().as_millis() as u64;
            let status_code = resp.status();
            if status_code.is_success() {
                match resp.json::<serde_json::Value>().await {
                    Ok(body) => {
                        let state = body["state"].as_str().unwrap_or("?");
                        let fw = body["firmware_version"].as_str().unwrap_or("?");
                        checks.push(Check {
                            name: format!("{name} - http"),
                            status: "pass".into(),
                            message: format!("HTTP OK - state: {state}, fw: {fw}"),
                            latency_ms: Some(latency),
                        });
                    }
                    Err(_) => {
                        checks.push(Check {
                            name: format!("{name} - http"),
                            status: "warn".into(),
                            message: format!("HTTP {status_code} but invalid JSON body"),
                            latency_ms: Some(latency),
                        });
                    }
                }
            } else {
                checks.push(Check {
                    name: format!("{name} - http"),
                    status: "fail".into(),
                    message: format!("HTTP {status_code}"),
                    latency_ms: Some(latency),
                });
            }
        }
        Err(e) => {
            let latency = start.elapsed().as_millis() as u64;
            checks.push(Check {
                name: format!("{name} - http"),
                status: "fail".into(),
                message: format!("HTTP error: {e}"),
                latency_ms: Some(latency),
            });
        }
    }

    // DNS check (resolve hostname.local)
    let start = Instant::now();
    let dns_result = tokio::net::lookup_host(format!("{ip}:80")).await;
    let latency = start.elapsed().as_millis() as u64;
    match dns_result {
        Ok(mut addrs) => {
            if let Some(addr) = addrs.next() {
                checks.push(Check {
                    name: format!("{name} - dns"),
                    status: "pass".into(),
                    message: format!("Resolves to {}", addr.ip()),
                    latency_ms: Some(latency),
                });
            }
        }
        Err(e) => {
            checks.push(Check {
                name: format!("{name} - dns"),
                status: "warn".into(),
                message: format!("DNS lookup failed: {e}"),
                latency_ms: Some(latency),
            });
        }
    }

    checks
}

fn check_firmware_dir() -> Check {
    let dir = std::env::var("FIRMWARE_DIR").unwrap_or_else(|_| "data/firmware".into());
    let path = std::path::Path::new(&dir);
    if path.exists() && path.is_dir() {
        let count = std::fs::read_dir(path).map(|d| d.count()).unwrap_or(0);
        Check {
            name: "firmware_dir".into(),
            status: "pass".into(),
            message: format!("{dir} exists ({count} files)"),
            latency_ms: None,
        }
    } else {
        Check {
            name: "firmware_dir".into(),
            status: "warn".into(),
            message: format!("{dir} does not exist"),
            latency_ms: None,
        }
    }
}
