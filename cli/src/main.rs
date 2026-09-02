mod security;
mod status;
mod devices;
mod config;
mod hooks;
mod tunnels;

use clap::{Parser, Subcommand};
use colored::Colorize;

#[derive(Parser)]
#[command(
    name = "hookbot",
    version,
    about = "Hookbot CLI — manage & audit your Hookbot instance",
    after_help = "Environment variables:\n  HOOKBOT_URL       API base URL\n  HOOKBOT_API_KEY   API key for authentication"
)]
struct Cli {
    /// API base URL (e.g. https://hookbot.example.com)
    #[arg(long, env = "HOOKBOT_URL", global = true)]
    url: Option<String>,

    /// API key for authentication
    #[arg(long, env = "HOOKBOT_API_KEY", global = true)]
    key: Option<String>,

    /// Output as JSON (for scripting)
    #[arg(long, global = true)]
    json: bool,

    #[command(subcommand)]
    command: Commands,
}

#[derive(Subcommand)]
enum Commands {
    /// Check server health and overview
    Status,

    /// Login to a Hookbot instance and save the server URL + API key
    Login {
        /// Server URL to login to (alias for --url)
        #[arg(long)]
        server: Option<String>,

        /// Admin password (will prompt if not provided)
        #[arg(long)]
        password: Option<String>,

        /// Save credentials to ~/.hookbot for future use
        #[arg(long, default_value = "true")]
        save: bool,
    },

    /// Set up the Claude Code hook integration
    Hooks {
        #[command(subcommand)]
        action: CliHookAction,
    },

    /// List and manage devices
    Devices {
        #[command(subcommand)]
        action: Option<DeviceAction>,
    },

    /// Ping a device or the server to check connectivity
    Ping {
        /// Device ID to ping (omit to ping the server)
        device: Option<String>,

        /// Number of pings
        #[arg(short = 'c', long, default_value = "4")]
        count: u32,
    },

    /// Run OWASP security audit against a live instance
    Security {
        /// Target URL to scan (overrides --url)
        #[arg(long)]
        target: Option<String>,

        /// Also scan the frontend URL
        #[arg(long)]
        frontend: Option<String>,

        /// Only run specific check categories (comma-separated)
        #[arg(long)]
        only: Option<String>,

        /// Skip specific check categories (comma-separated)
        #[arg(long)]
        skip: Option<String>,
    },

    /// Validate local configuration and environment
    Config {
        /// Path to .env file (default: .env)
        #[arg(long, default_value = ".env")]
        env_file: String,
    },

    /// Full diagnostic: config + security + connectivity
    Doctor {
        /// Path to .env file
        #[arg(long, default_value = ".env")]
        env_file: String,
    },

    /// View recent logs
    Logs {
        /// Number of log entries to show
        #[arg(short, long, default_value = "50")]
        limit: u32,

        /// Filter by device ID
        #[arg(long)]
        device: Option<String>,

        /// Follow mode (poll for new logs)
        #[arg(short, long)]
        follow: bool,
    },

    /// Manage OTA firmware updates
    Ota {
        #[command(subcommand)]
        action: OtaAction,
    },

    /// Manage Cloudflare Tunnels for remote access
    Tunnel {
        #[command(subcommand)]
        action: CliTunnelAction,
    },
}

#[derive(Subcommand)]
enum DeviceAction {
    /// List all devices
    List,
    /// Show device details
    Info {
        /// Device ID
        id: String,
    },
    /// Discover devices on local network via mDNS
    Discover,
    /// Reboot a device
    Reboot {
        /// Device ID
        id: String,
    },
}

#[derive(Subcommand)]
enum CliHookAction {
    /// Install the Hookbot hooks into your Claude Code settings
    Install {
        /// Install into ./.claude/settings.json instead of ~/.claude/settings.json
        #[arg(long)]
        project: bool,

        /// Settings file to write (overrides --project)
        #[arg(long)]
        settings: Option<String>,

        /// Path to the hook script (default: hooks/deskbot-hook.js in this checkout)
        #[arg(long)]
        script: Option<String>,

        /// Bind the hooks to a specific device ID
        #[arg(long)]
        device: Option<String>,
    },
}

#[derive(Subcommand)]
enum OtaAction {
    /// List available firmware versions
    List,
    /// Show OTA job status
    Status,
}

#[derive(Subcommand)]
enum CliTunnelAction {
    /// List all tunnels
    List,
    /// Start a tunnel by ID
    Start {
        /// Tunnel ID (or first 8 chars)
        id: String,
    },
    /// Stop a tunnel by ID
    Stop {
        /// Tunnel ID (or first 8 chars)
        id: String,
    },
    /// Show tunnel status and metrics
    Status {
        /// Tunnel ID (omit to show all)
        id: Option<String>,
    },
    /// Create a quick-connect tunnel (no Cloudflare account needed)
    QuickConnect,
    /// View tunnel process logs
    Logs {
        /// Tunnel ID
        id: String,
        /// Number of log lines to show
        #[arg(short, long, default_value = "50")]
        limit: u32,
    },
}

fn api_client(key: &Option<String>) -> reqwest::Client {
    let mut headers = reqwest::header::HeaderMap::new();
    if let Some(k) = key {
        if let Ok(val) = k.parse() {
            headers.insert("X-API-Key", val);
        }
    }
    reqwest::Client::builder()
        .default_headers(headers)
        .timeout(std::time::Duration::from_secs(10))
        .build()
        .expect("failed to build HTTP client")
}

/// Path to the CLI's credential file (~/.hookbot). Never inside the repo.
fn saved_config_path() -> String {
    let home = std::env::var("HOME").unwrap_or_default();
    format!("{home}/.hookbot")
}

/// Save the server URL and API key to ~/.hookbot with owner-only permissions.
fn save_config(url: &str, key: Option<&str>) -> Result<String, String> {
    let path = saved_config_path();
    let mut content = format!("url=\"{url}\"\n");
    if let Some(k) = key {
        content.push_str(&format!("api_key=\"{k}\"\n"));
    }
    std::fs::write(&path, &content).map_err(|e| format!("Failed to write {path}: {e}"))?;

    #[cfg(unix)]
    {
        use std::os::unix::fs::PermissionsExt;
        std::fs::set_permissions(&path, std::fs::Permissions::from_mode(0o600))
            .map_err(|e| e.to_string())?;
    }

    Ok(path)
}

/// Load saved credentials from ~/.hookbot
fn load_saved_config() -> (Option<String>, Option<String>) {
    let path = saved_config_path();
    let content = match std::fs::read_to_string(&path) {
        Ok(c) => c,
        Err(_) => return (None, None),
    };
    let mut url = None;
    let mut key = None;
    for line in content.lines() {
        let line = line.trim();
        if let Some((k, v)) = line.split_once('=') {
            match k.trim() {
                "url" => url = Some(v.trim().trim_matches('"').to_string()),
                "api_key" => key = Some(v.trim().trim_matches('"').to_string()),
                _ => {}
            }
        }
    }
    (url, key)
}

#[tokio::main]
async fn main() {
    let cli = Cli::parse();
    let (saved_url, saved_key) = load_saved_config();

    let base = cli.url.clone()
        .or(saved_url)
        .unwrap_or_else(|| "http://localhost:3000".into());
    let key = cli.key.clone().or(saved_key);
    let client = api_client(&key);

    let result = match cli.command {
        Commands::Status => status::run(&client, &base, cli.json).await,
        Commands::Login { server, password, save } => {
            let target = server.as_deref().unwrap_or(&base);
            login_cmd(target, password, key.as_deref(), save, cli.json).await
        }
        Commands::Hooks { action } => {
            let ha = match action {
                CliHookAction::Install { project, settings, script, device } => {
                    hooks::HookAction::Install { project, settings, script, device }
                }
            };
            hooks::run(ha, &base, key.as_deref(), cli.json).await
        }
        Commands::Devices { action } => devices::run(&client, &base, action, cli.json).await,
        Commands::Ping { device, count } => ping_cmd(&client, &base, device.as_deref(), count).await,
        Commands::Security { target, frontend, only, skip } => {
            let api_target = target.unwrap_or_else(|| base.clone());
            security::run(&api_target, frontend.as_deref(), only.as_deref(), skip.as_deref(), cli.json).await
        }
        Commands::Config { env_file } => config::run(&env_file, cli.json).await,
        Commands::Doctor { env_file } => doctor_cmd(&client, &base, &env_file, cli.json).await,
        Commands::Logs { limit, device, follow } => logs_cmd(&client, &base, limit, device, follow).await,
        Commands::Ota { action } => ota_cmd(&client, &base, action).await,
        Commands::Tunnel { action } => {
            let ta = match action {
                CliTunnelAction::List => tunnels::TunnelAction::List,
                CliTunnelAction::Start { id } => tunnels::TunnelAction::Start { id },
                CliTunnelAction::Stop { id } => tunnels::TunnelAction::Stop { id },
                CliTunnelAction::Status { id } => tunnels::TunnelAction::Status { id },
                CliTunnelAction::QuickConnect => tunnels::TunnelAction::QuickConnect,
                CliTunnelAction::Logs { id, limit } => tunnels::TunnelAction::Logs { id, limit },
            };
            tunnels::run(&client, &base, ta, cli.json).await
        }
    };

    if let Err(e) = result {
        if cli.json {
            let j = serde_json::json!({"error": e});
            println!("{}", serde_json::to_string_pretty(&j).unwrap());
        } else {
            eprintln!("{} {e}", "error:".red().bold());
        }
        std::process::exit(1);
    }
}

/// Normalise a user-supplied server URL: default to HTTPS, drop trailing slashes.
fn normalize_url(url: &str) -> String {
    let url = url.trim().trim_end_matches('/');
    if url.starts_with("http://") || url.starts_with("https://") {
        url.to_string()
    } else {
        format!("https://{url}")
    }
}

async fn login_cmd(
    server: &str,
    password: Option<String>,
    key: Option<&str>,
    save: bool,
    json: bool,
) -> Result<(), String> {
    let server = normalize_url(server);

    let client = reqwest::Client::builder()
        .timeout(std::time::Duration::from_secs(10))
        .cookie_store(true)
        .build()
        .map_err(|e: reqwest::Error| e.to_string())?;

    // 1. Verify the URL points at a live Hookbot server before storing anything.
    if !json {
        println!("{}", "=== Hookbot Login ===".bold());
        println!();
        print!("  Server:    {server} ... ");
        use std::io::Write;
        let _ = std::io::stdout().flush();
    }

    let health = client
        .get(format!("{server}/api/health"))
        .send()
        .await
        .map_err(|e: reqwest::Error| format!("Cannot reach {server}/api/health: {e}"))?;

    if !health.status().is_success() {
        return Err(format!(
            "{server}/api/health returned HTTP {} — is this a Hookbot server?",
            health.status()
        ));
    }

    // A single-page-app host answers 200 with HTML for any path, so only a
    // well-formed health payload proves this URL is really the API.
    let health_body: serde_json::Value = health
        .json()
        .await
        .map_err(|_| format!("{server}/api/health did not return JSON — is this a Hookbot server?"))?;
    if health_body.get("status").and_then(|v| v.as_str()) != Some("ok") {
        return Err(format!(
            "{server}/api/health returned an unexpected payload — is this a Hookbot server?"
        ));
    }
    if !json {
        println!("{}", "reachable".green());
    }

    // 2. Authenticate. An API key is checked as-is; otherwise exchange the admin
    //    password for a session cookie and read the API key back from /api/auth/me.
    let api_key: Option<String> = match key {
        Some(k) => {
            let resp = client
                .get(format!("{server}/api/auth/status"))
                .header("X-API-Key", k)
                .send()
                .await
                .map_err(|e: reqwest::Error| format!("Connection failed: {e}"))?;
            let body: serde_json::Value = resp.json().await.unwrap_or_default();
            if !body.get("authenticated").and_then(|v| v.as_bool()).unwrap_or(false) {
                return Err("API key rejected by the server.".into());
            }
            Some(k.to_string())
        }
        None => {
            let password = match password {
                Some(p) => p,
                None => {
                    // Never echo. Typing a password into a visible prompt puts
                    // it in scrollback and, for anyone who pastes the line,
                    // shell history.
                    rpassword::prompt_password("  Password:  ")
                        .map_err(|e| format!("Could not read password: {e}"))?
                        .trim()
                        .to_string()
                }
            };

            let resp = client
                .post(format!("{server}/api/auth/login"))
                .json(&serde_json::json!({"password": password}))
                .send()
                .await
                .map_err(|e: reqwest::Error| format!("Connection failed: {e}"))?;

            let status = resp.status();
            let body: serde_json::Value = resp.json::<serde_json::Value>().await.unwrap_or_default();

            if !status.is_success() {
                let msg = body
                    .get("error")
                    .and_then(|v: &serde_json::Value| v.as_str())
                    .unwrap_or("Login failed");
                if status.as_u16() == 429 {
                    // "Try again later" with no number is not actionable.
                    let secs = body
                        .get("retry_after_secs")
                        .and_then(|v: &serde_json::Value| v.as_u64());
                    return Err(match secs {
                        Some(s) => format!(
                            "{msg} Wait {s}s and retry — the limit is per client IP, \
                             and a wrong password counts. Use --password to avoid \
                             burning attempts on typos."
                        ),
                        None => format!("{msg} (HTTP {status})"),
                    });
                }
                return Err(format!("{msg} (HTTP {status})"));
            }

            // The session cookie is now in the jar — trade it for the API key
            // so later runs can authenticate without a password.
            let me = client
                .get(format!("{server}/api/auth/me"))
                .send()
                .await
                .map_err(|e: reqwest::Error| format!("Connection failed: {e}"))?;
            let me_body: serde_json::Value = me.json().await.unwrap_or_default();
            me_body
                .get("api_key")
                .and_then(|v| v.as_str())
                .map(|s| s.to_string())
        }
    };

    let saved_path = if save {
        Some(save_config(&server, api_key.as_deref())?)
    } else {
        None
    };

    if json {
        let report = serde_json::json!({
            "ok": true,
            "url": server,
            "health": health_body,
            "api_key_saved": api_key.is_some() && saved_path.is_some(),
            "config": saved_path,
        });
        println!("{}", serde_json::to_string_pretty(&report).unwrap());
        return Ok(());
    }

    println!("  Auth:      {}", "authenticated".green().bold());
    match (&api_key, &saved_path) {
        (Some(_), Some(path)) => {
            println!("  API key:   {} (saved)", "****".dimmed());
            println!("  Config:    {}", path.dimmed());
        }
        (Some(_), None) => {
            println!("  API key:   {}", "****".dimmed());
            println!("  {}", "Not saved (--save=false)".dimmed());
        }
        (None, Some(path)) => {
            println!("  API key:   {}", "not issued by this server".yellow());
            println!("  Config:    {} (URL only)", path.dimmed());
        }
        (None, None) => {}
    }

    println!();
    println!("  {} Logged in to {server}", "OK".green().bold());
    println!();
    println!("  Next:");
    println!("    {} hookbot status", "$".dimmed());
    println!("    {} hookbot hooks install", "$".dimmed());

    Ok(())
}

async fn ping_cmd(
    client: &reqwest::Client,
    base: &str,
    device: Option<&str>,
    count: u32,
) -> Result<(), String> {
    let target = match device {
        Some(id) => format!("device {id}"),
        None => format!("server {base}"),
    };
    println!("PING {} ({} requests)", target, count);
    println!();

    let mut times: Vec<f64> = Vec::new();
    let mut failures = 0u32;

    for i in 0..count {
        let url = match device {
            Some(id) => format!("{base}/api/devices/{id}"),
            None => format!("{base}/api/health"),
        };

        let start = std::time::Instant::now();
        match client.get(&url).send().await {
            Ok(resp) => {
                let elapsed = start.elapsed().as_secs_f64() * 1000.0;
                let status = resp.status().as_u16();

                if resp.status().is_success() {
                    times.push(elapsed);
                    println!(
                        "  {} seq={} status={} time={:.1}ms",
                        "OK".green(),
                        i + 1,
                        status,
                        elapsed
                    );
                } else {
                    failures += 1;
                    println!(
                        "  {} seq={} status={} time={:.1}ms",
                        "ERR".red(),
                        i + 1,
                        status,
                        elapsed
                    );
                }
            }
            Err(e) => {
                failures += 1;
                println!("  {} seq={} error: {}", "FAIL".red(), i + 1, e);
            }
        }

        if i + 1 < count {
            tokio::time::sleep(std::time::Duration::from_secs(1)).await;
        }
    }

    println!();
    println!("--- {} ping statistics ---", target);
    println!(
        "{} requests, {} ok, {} failed ({:.0}% loss)",
        count,
        count - failures,
        failures,
        (failures as f64 / count as f64) * 100.0,
    );

    if !times.is_empty() {
        let min = times.iter().cloned().fold(f64::INFINITY, f64::min);
        let max = times.iter().cloned().fold(f64::NEG_INFINITY, f64::max);
        let avg = times.iter().sum::<f64>() / times.len() as f64;
        println!("rtt min/avg/max = {:.1}/{:.1}/{:.1} ms", min, avg, max);
    }

    Ok(())
}

async fn doctor_cmd(
    client: &reqwest::Client,
    base: &str,
    env_file: &str,
    json: bool,
) -> Result<(), String> {
    if !json {
        println!("{}", "=== Hookbot Doctor ===".bold());
        println!("Running full diagnostic...\n");
    }

    let mut all_ok = true;

    // 1. Config check
    if !json { println!("{}", "--- Configuration ---".bold()); }
    if let Err(e) = config::run(env_file, json).await {
        if !json { eprintln!("  Config check failed: {e}"); }
        all_ok = false;
    }

    if !json { println!(); }

    // 2. Connectivity
    if !json { println!("{}", "--- Connectivity ---".bold()); }
    if let Err(e) = status::run(client, base, json).await {
        if !json { eprintln!("  Status check failed: {e}"); }
        all_ok = false;
    }

    if !json { println!(); }

    // 3. Quick security scan
    if !json { println!("{}", "--- Quick Security Scan ---".bold()); }
    if let Err(e) = security::run(base, None, None, Some("injection,paths"), json).await {
        if !json { eprintln!("  Security check failed: {e}"); }
        all_ok = false;
    }

    if !json {
        println!();
        if all_ok {
            println!("{}", "Doctor: All checks passed.".green().bold());
        } else {
            println!("{}", "Doctor: Some checks failed. See above for details.".yellow().bold());
        }
    }

    Ok(())
}

async fn logs_cmd(
    client: &reqwest::Client,
    base: &str,
    limit: u32,
    device: Option<String>,
    follow: bool,
) -> Result<(), String> {
    let fetch = |last_ts: Option<String>| {
        let url = {
            let mut u = format!("{base}/api/logs?limit={limit}");
            if let Some(ref id) = device {
                u.push_str(&format!("&device_id={id}"));
            }
            if let Some(ref ts) = last_ts {
                u.push_str(&format!("&after={ts}"));
            }
            u
        };
        let client = client.clone();
        async move {
            let resp = client.get(&url).send().await.map_err(|e| e.to_string())?;
            if !resp.status().is_success() {
                return Err(format!("HTTP {}", resp.status()));
            }
            let body: serde_json::Value = resp.json().await.map_err(|e| e.to_string())?;
            Ok(body)
        }
    };

    let print_logs = |body: &serde_json::Value| -> Option<String> {
        let mut last = None;
        if let Some(logs) = body.as_array() {
            for log in logs {
                let ts = log.get("created_at").and_then(|v| v.as_str()).unwrap_or("?");
                let dev = log.get("device_id").and_then(|v| v.as_str()).unwrap_or("?");
                let status = log.get("status").and_then(|v| v.as_str()).unwrap_or("?");

                let status_colored = match status {
                    s if s.contains("error") || s.contains("fail") => s.red().to_string(),
                    s if s.contains("online") || s.contains("ok") => s.green().to_string(),
                    s => s.to_string(),
                };

                println!(
                    "{}  {}  {}",
                    ts.dimmed(),
                    format!("[{dev}]").cyan(),
                    status_colored,
                );
                last = Some(ts.to_string());
            }
        }
        last
    };

    let body = fetch(None).await?;
    let mut last_ts = print_logs(&body);

    if follow {
        println!("{}", "--- following (Ctrl+C to stop) ---".dimmed());
        loop {
            tokio::time::sleep(std::time::Duration::from_secs(3)).await;
            match fetch(last_ts.clone()).await {
                Ok(body) => {
                    if let Some(ts) = print_logs(&body) {
                        last_ts = Some(ts);
                    }
                }
                Err(_) => {
                    eprint!(".");
                }
            }
        }
    }

    Ok(())
}

async fn ota_cmd(
    client: &reqwest::Client,
    base: &str,
    action: OtaAction,
) -> Result<(), String> {
    match action {
        OtaAction::List => {
            let resp = client.get(format!("{base}/api/firmware")).send().await.map_err(|e| e.to_string())?;
            if !resp.status().is_success() {
                return Err(format!("HTTP {}", resp.status()));
            }
            let body: serde_json::Value = resp.json().await.map_err(|e| e.to_string())?;
            if let Some(items) = body.as_array() {
                if items.is_empty() {
                    println!("No firmware versions uploaded.");
                    return Ok(());
                }
                println!("{:<8} {:<12} {:<10} Created", "ID", "Version", "Size");
                println!("{}", "-".repeat(60));
                for fw in items {
                    let id = fw.get("id").and_then(|v| v.as_i64()).unwrap_or(0);
                    let ver = fw.get("version").and_then(|v| v.as_str()).unwrap_or("?");
                    let size = fw.get("size").and_then(|v| v.as_i64()).unwrap_or(0);
                    let created = fw.get("created_at").and_then(|v| v.as_str()).unwrap_or("?");
                    println!("{:<8} {:<12} {:<10} {}", id, ver, format_bytes(size), created);
                }
                println!("\n{} firmware(s)", items.len());
            }
            Ok(())
        }
        OtaAction::Status => {
            let resp = client.get(format!("{base}/api/ota/jobs")).send().await.map_err(|e| e.to_string())?;
            if !resp.status().is_success() {
                return Err(format!("HTTP {}", resp.status()));
            }
            let body: serde_json::Value = resp.json().await.map_err(|e| e.to_string())?;
            if let Some(jobs) = body.as_array() {
                if jobs.is_empty() {
                    println!("No OTA jobs.");
                    return Ok(());
                }
                for job in jobs {
                    let id = job.get("id").and_then(|v| v.as_i64()).unwrap_or(0);
                    let status = job.get("status").and_then(|v| v.as_str()).unwrap_or("?");
                    let device = job.get("device_id").and_then(|v| v.as_str()).unwrap_or("?");
                    let created = job.get("created_at").and_then(|v| v.as_str()).unwrap_or("?");

                    let status_colored = match status {
                        "completed" => status.green(),
                        "failed" => status.red(),
                        "pending" => status.yellow(),
                        s => s.normal(),
                    };

                    println!("  #{:<4} device={:<8} status={:<12} {}", id, device, status_colored, created.dimmed());
                }
            } else {
                println!("{}", serde_json::to_string_pretty(&body).unwrap());
            }
            Ok(())
        }
    }
}

fn format_bytes(b: i64) -> String {
    if b < 1024 { return format!("{b} B"); }
    if b < 1024 * 1024 { return format!("{:.1} KB", b as f64 / 1024.0); }
    format!("{:.1} MB", b as f64 / (1024.0 * 1024.0))
}
