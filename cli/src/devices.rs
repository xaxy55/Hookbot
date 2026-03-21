use colored::Colorize;
use crate::DeviceAction;

pub async fn run(
    client: &reqwest::Client,
    base: &str,
    action: Option<DeviceAction>,
    json: bool,
) -> Result<(), String> {
    match action.unwrap_or(DeviceAction::List) {
        DeviceAction::List => list(client, base, json).await,
        DeviceAction::Info { id } => info(client, base, &id, json).await,
        DeviceAction::Discover => discover().await,
        DeviceAction::Reboot { id } => reboot(client, base, &id).await,
    }
}

async fn list(client: &reqwest::Client, base: &str, json: bool) -> Result<(), String> {
    let resp = client
        .get(&format!("{base}/api/devices"))
        .send()
        .await
        .map_err(|e| e.to_string())?;

    if resp.status().as_u16() == 401 {
        return Err("Authentication required. Pass --key <API_KEY> or set HOOKBOT_API_KEY.".into());
    }
    if !resp.status().is_success() {
        return Err(format!("HTTP {}", resp.status()));
    }

    let body: serde_json::Value = resp.json().await.map_err(|e| e.to_string())?;

    if json {
        println!("{}", serde_json::to_string_pretty(&body).unwrap());
        return Ok(());
    }

    let devices = body.as_array().ok_or("unexpected response format")?;

    if devices.is_empty() {
        println!("No devices registered.");
        return Ok(());
    }

    println!(
        "  {:<6} {:<20} {:<16} {:<8} {:<12} {}",
        "ID", "Name", "IP", "Online", "Firmware", "Last Seen"
    );
    println!("  {}", "-".repeat(80));

    for dev in devices {
        let id = dev.get("id").and_then(|v| v.as_i64()).unwrap_or(0);
        let name = dev.get("name").and_then(|v| v.as_str()).unwrap_or("?");
        let ip = dev.get("ip").and_then(|v| v.as_str()).unwrap_or("?");
        let online = dev.get("online").and_then(|v| v.as_bool()).unwrap_or(false);
        let fw = dev.get("firmware_version").and_then(|v| v.as_str()).unwrap_or("?");
        let last_seen = dev.get("last_seen").and_then(|v| v.as_str()).unwrap_or("?");

        let online_str = if online {
            "yes".green().to_string()
        } else {
            "no".red().to_string()
        };

        let name_display = if name.len() > 18 {
            format!("{}...", &name[..15])
        } else {
            name.to_string()
        };

        println!("  {:<6} {:<20} {:<16} {:<8} {:<12} {}",
            id, name_display, ip, online_str, fw, last_seen.dimmed()
        );
    }

    println!("\n  {} device(s)", devices.len());
    Ok(())
}

async fn info(client: &reqwest::Client, base: &str, id: &str, json: bool) -> Result<(), String> {
    let resp = client
        .get(&format!("{base}/api/devices/{id}"))
        .send()
        .await
        .map_err(|e| e.to_string())?;

    if resp.status().as_u16() == 401 {
        return Err("Authentication required.".into());
    }
    if resp.status().as_u16() == 404 {
        return Err(format!("Device '{id}' not found."));
    }
    if !resp.status().is_success() {
        return Err(format!("HTTP {}", resp.status()));
    }

    let body: serde_json::Value = resp.json().await.map_err(|e| e.to_string())?;

    if json {
        println!("{}", serde_json::to_string_pretty(&body).unwrap());
        return Ok(());
    }

    // Pretty print device info
    println!("{}", "=== Device Info ===".bold());
    println!();

    let fields = [
        ("ID", "id"), ("Name", "name"), ("IP", "ip"), ("MAC", "mac"),
        ("Online", "online"), ("Firmware", "firmware_version"),
        ("Type", "device_type"), ("Last Seen", "last_seen"),
        ("Created", "created_at"),
    ];

    for (label, key) in &fields {
        if let Some(val) = body.get(key) {
            let display = match val {
                serde_json::Value::Bool(true) => "yes".green().to_string(),
                serde_json::Value::Bool(false) => "no".red().to_string(),
                serde_json::Value::String(s) => s.clone(),
                v => v.to_string(),
            };
            println!("  {:<14} {}", format!("{label}:").dimmed(), display);
        }
    }

    // Config if present
    if let Some(config) = body.get("config") {
        println!();
        println!("  {}", "Config:".dimmed());
        if let Some(obj) = config.as_object() {
            for (k, v) in obj {
                println!("    {:<12} {}", k.dimmed(), v);
            }
        }
    }

    Ok(())
}

async fn discover() -> Result<(), String> {
    println!("{}", "Scanning for Hookbot devices on local network...".dimmed());
    println!("(mDNS discovery — looking for _hookbot._tcp services)\n");

    let output = tokio::process::Command::new("dns-sd")
        .args(["-B", "_http._tcp", "local."])
        .stdout(std::process::Stdio::piped())
        .stderr(std::process::Stdio::piped())
        .spawn();

    match output {
        Ok(mut child) => {
            tokio::time::sleep(std::time::Duration::from_secs(3)).await;
            let _ = child.kill().await;

            if let Some(stdout) = child.stdout.take() {
                use tokio::io::AsyncReadExt;
                let mut buf = String::new();
                let mut reader = stdout;
                let _ = reader.read_to_string(&mut buf).await;

                let hookbot_lines: Vec<&str> = buf
                    .lines()
                    .filter(|l| l.to_lowercase().contains("hookbot"))
                    .collect();

                if hookbot_lines.is_empty() {
                    println!("No Hookbot devices found on local network.");
                    println!("{}", "Tip: devices advertise as 'Hookbot-XXYY' via BLE at first boot.".dimmed());
                } else {
                    println!("Found {} device(s):", hookbot_lines.len());
                    for line in hookbot_lines {
                        println!("  {}", line);
                    }
                }
            }
        }
        Err(_) => {
            println!("{}", "dns-sd not available. Try: avahi-browse -r _http._tcp".yellow());
        }
    }

    Ok(())
}

async fn reboot(client: &reqwest::Client, base: &str, id: &str) -> Result<(), String> {
    print!("Rebooting device {id}... ");

    let resp = client
        .post(&format!("{base}/api/devices/{id}/reboot"))
        .send()
        .await
        .map_err(|e| e.to_string())?;

    if resp.status().as_u16() == 401 {
        return Err("Authentication required.".into());
    }
    if resp.status().as_u16() == 404 {
        return Err(format!("Device '{id}' not found."));
    }
    if !resp.status().is_success() {
        return Err(format!("HTTP {}", resp.status()));
    }

    println!("{}", "OK".green());
    println!("{}", "Device will restart shortly.".dimmed());
    Ok(())
}
