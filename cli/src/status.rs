use colored::Colorize;
use std::time::Instant;

pub async fn run(client: &reqwest::Client, base: &str, json: bool) -> Result<(), String> {
    let mut report = serde_json::Map::new();

    // Health check
    let start = Instant::now();
    let health = client.get(&format!("{base}/api/health")).send().await;
    let health_ms = start.elapsed().as_secs_f64() * 1000.0;

    if !json {
        println!("{}", "=== Hookbot Status ===".bold());
        println!();
    }

    match health {
        Ok(resp) => {
            let status = resp.status();
            if json {
                report.insert("server".into(), serde_json::json!({
                    "status": if status.is_success() { "online" } else { "error" },
                    "http_status": status.as_u16(),
                    "response_ms": health_ms,
                }));
            } else {
                if status.is_success() {
                    let body: serde_json::Value = resp.json().await.unwrap_or_default();
                    println!("  Server:    {} {} {}",
                        "ONLINE".green().bold(),
                        format!("({})", status).dimmed(),
                        format!("{:.0}ms", health_ms).dimmed(),
                    );
                    if let Some(obj) = body.as_object() {
                        for (k, v) in obj {
                            if k != "status" {
                                println!("             {}: {}", k.dimmed(), v);
                            }
                        }
                    }
                } else {
                    println!("  Server:    {} (HTTP {}, {:.0}ms)", "ERROR".red().bold(), status, health_ms);
                }
            }
        }
        Err(e) => {
            if json {
                report.insert("server".into(), serde_json::json!({
                    "status": "offline",
                    "error": e.to_string(),
                }));
            } else {
                println!("  Server:    {} — {}", "OFFLINE".red().bold(), e);
                println!();
                return Ok(());
            }
        }
    }

    // Auth status
    match client.get(&format!("{base}/api/auth/status")).send().await {
        Ok(resp) => {
            let body: serde_json::Value = resp.json().await.unwrap_or_default();
            let authed = body.get("authenticated").and_then(|v| v.as_bool()).unwrap_or(false);
            let workos = body.get("workos_enabled").and_then(|v| v.as_bool()).unwrap_or(false);

            if json {
                report.insert("auth".into(), serde_json::json!({
                    "authenticated": authed,
                    "workos_enabled": workos,
                }));
            } else {
                print!("  Auth:      ");
                if authed {
                    print!("{}", "Authenticated".green());
                } else {
                    print!("{}", "Not authenticated".yellow());
                }
                if workos {
                    print!("  (WorkOS {})", "enabled".green());
                }
                println!();
            }
        }
        Err(_) => {
            if !json { println!("  Auth:      {}", "unavailable".dimmed()); }
        }
    }

    // Devices
    match client.get(&format!("{base}/api/devices")).send().await {
        Ok(resp) => {
            if resp.status().as_u16() == 401 {
                if json {
                    report.insert("devices".into(), serde_json::json!({"error": "auth_required"}));
                } else {
                    println!("  Devices:   {}", "auth required (pass --key)".yellow());
                }
            } else if resp.status().is_success() {
                let body: serde_json::Value = resp.json().await.unwrap_or_default();
                if let Some(devices) = body.as_array() {
                    let online = devices.iter().filter(|d| {
                        d.get("online").and_then(|v| v.as_bool()).unwrap_or(false)
                    }).count();

                    if json {
                        report.insert("devices".into(), serde_json::json!({
                            "total": devices.len(),
                            "online": online,
                        }));
                    } else {
                        println!("  Devices:   {} total, {} online",
                            devices.len(),
                            online.to_string().green(),
                        );
                    }
                }
            }
        }
        Err(_) => {
            if !json { println!("  Devices:   {}", "unavailable".dimmed()); }
        }
    }

    // Firmware
    match client.get(&format!("{base}/api/firmware")).send().await {
        Ok(resp) => {
            if resp.status().as_u16() == 401 {
                if !json { println!("  Firmware:  {}", "auth required".dimmed()); }
            } else if resp.status().is_success() {
                let body: serde_json::Value = resp.json().await.unwrap_or_default();
                if let Some(fw) = body.as_array() {
                    let latest = fw.first()
                        .and_then(|f| f.get("version"))
                        .and_then(|v| v.as_str())
                        .unwrap_or("?");

                    if json {
                        report.insert("firmware".into(), serde_json::json!({
                            "count": fw.len(),
                            "latest": latest,
                        }));
                    } else {
                        if fw.is_empty() {
                            println!("  Firmware:  none uploaded");
                        } else {
                            println!("  Firmware:  {} versions (latest: {})", fw.len(), latest.cyan());
                        }
                    }
                }
            }
        }
        Err(_) => {
            if !json { println!("  Firmware:  {}", "unavailable".dimmed()); }
        }
    }

    // OTA jobs
    match client.get(&format!("{base}/api/ota/jobs")).send().await {
        Ok(resp) if resp.status().is_success() => {
            let body: serde_json::Value = resp.json().await.unwrap_or_default();
            if let Some(jobs) = body.as_array() {
                let pending = jobs.iter().filter(|j| {
                    j.get("status").and_then(|v| v.as_str()) == Some("pending")
                }).count();

                if json {
                    report.insert("ota".into(), serde_json::json!({
                        "total_jobs": jobs.len(),
                        "pending": pending,
                    }));
                } else if pending > 0 {
                    println!("  OTA:       {} pending job(s)", pending.to_string().yellow());
                }
            }
        }
        _ => {}
    }

    if json {
        println!("{}", serde_json::to_string_pretty(&serde_json::Value::Object(report)).unwrap());
    } else {
        println!();
    }

    Ok(())
}
