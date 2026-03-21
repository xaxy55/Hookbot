use colored::Colorize;

pub enum TunnelAction {
    List,
    Start { id: String },
    Stop { id: String },
    Status { id: Option<String> },
    QuickConnect,
    Logs { id: String, limit: u32 },
}

pub async fn run(
    client: &reqwest::Client,
    base: &str,
    action: TunnelAction,
    json_output: bool,
) -> Result<(), String> {
    match action {
        TunnelAction::List => list(client, base, json_output).await,
        TunnelAction::Start { id } => start(client, base, &id).await,
        TunnelAction::Stop { id } => stop(client, base, &id).await,
        TunnelAction::Status { id } => status(client, base, id.as_deref(), json_output).await,
        TunnelAction::QuickConnect => quick_connect(client, base).await,
        TunnelAction::Logs { id, limit } => logs(client, base, &id, limit, json_output).await,
    }
}

async fn list(client: &reqwest::Client, base: &str, json_output: bool) -> Result<(), String> {
    let resp = client.get(&format!("{base}/api/tunnels"))
        .send().await.map_err(|e| e.to_string())?;
    if !resp.status().is_success() {
        return Err(format!("HTTP {}", resp.status()));
    }
    let body: serde_json::Value = resp.json().await.map_err(|e| e.to_string())?;

    if json_output {
        println!("{}", serde_json::to_string_pretty(&body).unwrap());
        return Ok(());
    }

    if let Some(tunnels) = body.as_array() {
        if tunnels.is_empty() {
            println!("No tunnels configured.");
            println!("  Use {} to create one instantly.", "hookbot tunnel quick-connect".cyan());
            return Ok(());
        }
        println!("{:<12} {:<20} {:<10} {:<8} {}", "ID", "NAME", "STATUS", "PORT", "URL");
        println!("{}", "-".repeat(75));
        for t in tunnels {
            let id = t.get("id").and_then(|v| v.as_str()).unwrap_or("?");
            let name = t.get("name").and_then(|v| v.as_str()).unwrap_or("?");
            let status = t.get("status").and_then(|v| v.as_str()).unwrap_or("stopped");
            let port = t.get("port").and_then(|v| v.as_i64()).unwrap_or(0);
            let url = t.get("process")
                .and_then(|p| p.get("assigned_url"))
                .and_then(|v| v.as_str())
                .or_else(|| t.get("hostname").and_then(|v| v.as_str()))
                .unwrap_or("-");

            let short_id = if id.len() > 8 { &id[..8] } else { id };

            let status_str = match status {
                "running" => status.green().to_string(),
                "error" => status.red().to_string(),
                _ => status.yellow().to_string(),
            };

            println!("{:<12} {:<20} {:<10} {:<8} {}", short_id, name, status_str, port, url);
        }
        println!("\n{} tunnel(s)", tunnels.len());
    }
    Ok(())
}

async fn start(client: &reqwest::Client, base: &str, id: &str) -> Result<(), String> {
    let resp = client.post(&format!("{base}/api/tunnels/{id}/start"))
        .send().await.map_err(|e| e.to_string())?;
    let status = resp.status();
    let body: serde_json::Value = resp.json().await.map_err(|e| e.to_string())?;

    if !status.is_success() {
        let msg = body.get("error").and_then(|v| v.as_str()).unwrap_or("Failed to start tunnel");
        return Err(msg.to_string());
    }

    let message = body.get("message").and_then(|v| v.as_str()).unwrap_or("Tunnel started");
    println!("{} {message}", "OK".green().bold());
    Ok(())
}

async fn stop(client: &reqwest::Client, base: &str, id: &str) -> Result<(), String> {
    let resp = client.post(&format!("{base}/api/tunnels/{id}/stop"))
        .send().await.map_err(|e| e.to_string())?;
    let status = resp.status();
    let body: serde_json::Value = resp.json().await.map_err(|e| e.to_string())?;

    if !status.is_success() {
        let msg = body.get("error").and_then(|v| v.as_str()).unwrap_or("Failed to stop tunnel");
        return Err(msg.to_string());
    }

    println!("{} Tunnel stopped", "OK".green().bold());
    Ok(())
}

async fn status(client: &reqwest::Client, base: &str, id: Option<&str>, json_output: bool) -> Result<(), String> {
    if let Some(id) = id {
        let resp = client.get(&format!("{base}/api/tunnels/{id}/metrics"))
            .send().await.map_err(|e| e.to_string())?;
        if !resp.status().is_success() {
            return Err(format!("HTTP {}", resp.status()));
        }
        let body: serde_json::Value = resp.json().await.map_err(|e| e.to_string())?;

        if json_output {
            println!("{}", serde_json::to_string_pretty(&body).unwrap());
            return Ok(());
        }

        let status = body.get("status").and_then(|v| v.as_str()).unwrap_or("unknown");
        let pid = body.get("pid").and_then(|v| v.as_u64());
        let uptime = body.get("uptime_secs").and_then(|v| v.as_i64());
        let connected = body.get("connected").and_then(|v| v.as_bool()).unwrap_or(false);
        let url = body.get("assigned_url").and_then(|v| v.as_str());
        let restarts = body.get("restart_count").and_then(|v| v.as_u64()).unwrap_or(0);

        let status_str = match status {
            "running" => "running".green().bold().to_string(),
            "error" => "error".red().bold().to_string(),
            s => s.yellow().to_string(),
        };

        println!("Tunnel {id}");
        println!("  Status:     {status_str}");
        println!("  Connected:  {}", if connected { "yes".green() } else { "no".red() });
        if let Some(pid) = pid { println!("  PID:        {pid}"); }
        if let Some(url) = url { println!("  URL:        {}", url.cyan()); }
        if let Some(secs) = uptime { println!("  Uptime:     {}", format_duration(secs)); }
        println!("  Restarts:   {restarts}");
    } else {
        // Show all tunnels status
        list(client, base, json_output).await?;
    }
    Ok(())
}

async fn quick_connect(client: &reqwest::Client, base: &str) -> Result<(), String> {
    println!("Starting quick-connect tunnel (TryCloudflare)...");

    let resp = client.post(&format!("{base}/api/tunnels/quick-connect"))
        .send().await.map_err(|e| e.to_string())?;
    let status = resp.status();
    let body: serde_json::Value = resp.json().await.map_err(|e| e.to_string())?;

    if !status.is_success() {
        let msg = body.get("error").and_then(|v| v.as_str()).unwrap_or("Failed to start tunnel");
        return Err(msg.to_string());
    }

    let id = body.get("id").and_then(|v| v.as_str()).unwrap_or("?");
    let url = body.get("assigned_url").and_then(|v| v.as_str());

    println!("{} Quick-connect tunnel started", "OK".green().bold());
    println!("  ID:   {}", &id[..8.min(id.len())]);

    if let Some(url) = url {
        println!("  URL:  {}", url.cyan().bold());
    } else {
        println!("  URL:  {} (run {} to check)", "pending...".yellow(), format!("hookbot tunnel status {}", &id[..8.min(id.len())]).cyan());
    }

    println!();
    println!("  Your hookbot is now accessible from the internet!");
    println!("  Stop with: {}", format!("hookbot tunnel stop {}", &id[..8.min(id.len())]).dimmed());

    Ok(())
}

async fn logs(client: &reqwest::Client, base: &str, id: &str, limit: u32, json_output: bool) -> Result<(), String> {
    let resp = client.get(&format!("{base}/api/tunnels/{id}/logs?limit={limit}"))
        .send().await.map_err(|e| e.to_string())?;
    if !resp.status().is_success() {
        return Err(format!("HTTP {}", resp.status()));
    }
    let body: serde_json::Value = resp.json().await.map_err(|e| e.to_string())?;

    if json_output {
        println!("{}", serde_json::to_string_pretty(&body).unwrap());
        return Ok(());
    }

    if let Some(logs) = body.as_array() {
        if logs.is_empty() {
            println!("No logs yet for tunnel {id}");
            return Ok(());
        }
        for log in logs {
            let ts = log.get("timestamp").and_then(|v| v.as_str()).unwrap_or("?");
            let level = log.get("level").and_then(|v| v.as_str()).unwrap_or("info");
            let msg = log.get("message").and_then(|v| v.as_str()).unwrap_or("");

            let level_str = match level {
                "error" => level.red().to_string(),
                "warn" => level.yellow().to_string(),
                _ => level.dimmed().to_string(),
            };

            println!("{} [{}] {}", ts.dimmed(), level_str, msg);
        }
    }
    Ok(())
}

fn format_duration(secs: i64) -> String {
    if secs < 60 { return format!("{secs}s"); }
    if secs < 3600 { return format!("{}m {}s", secs / 60, secs % 60); }
    if secs < 86400 { return format!("{}h {}m", secs / 3600, (secs % 3600) / 60); }
    format!("{}d {}h", secs / 86400, (secs % 86400) / 3600)
}
