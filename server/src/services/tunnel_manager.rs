use std::collections::{HashMap, VecDeque};
use std::sync::Arc;

use tokio::io::{AsyncBufReadExt, BufReader};
use tokio::process::{Child, Command};
use tokio::sync::Mutex;
use tracing::{info, warn, error};

use crate::db::DbPool;

const MAX_LOG_LINES: usize = 500;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TunnelProcessInfo {
    pub tunnel_id: String,
    pub pid: Option<u32>,
    pub started_at: String,
    pub restart_count: u32,
    pub assigned_url: Option<String>,
    pub connected: bool,
}

use serde::{Serialize, Deserialize};

#[derive(Clone)]
pub struct TunnelManager {
    processes: Arc<Mutex<HashMap<String, TunnelHandle>>>,
    db: DbPool,
    cloudflared_path: String,
    auto_restart: bool,
}

struct TunnelHandle {
    _child: Child,
    pid: Option<u32>,
    started_at: chrono::DateTime<chrono::Utc>,
    restart_count: u32,
    log_buffer: Arc<Mutex<VecDeque<LogLine>>>,
    assigned_url: Arc<Mutex<Option<String>>>,
    connected: Arc<Mutex<bool>>,
    shutdown_tx: tokio::sync::watch::Sender<bool>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct LogLine {
    pub timestamp: String,
    pub level: String,
    pub message: String,
}

impl TunnelManager {
    pub fn new(db: DbPool, cloudflared_path: String, auto_restart: bool) -> Self {
        Self {
            processes: Arc::new(Mutex::new(HashMap::new())),
            db,
            cloudflared_path,
            auto_restart,
        }
    }

    /// Recover tunnels that were marked as "running" before server restart
    pub async fn recover_running_tunnels(&self) {
        let tunnels: Vec<(String, Option<String>, i64, Option<String>)> = {
            let conn = self.db.lock().unwrap();
            let mut stmt = conn.prepare(
                "SELECT id, auth_token, port, hostname FROM tunnel_configs WHERE status = 'running'"
            ).unwrap();
            stmt.query_map([], |row| {
                Ok((
                    row.get::<_, String>(0)?,
                    row.get::<_, Option<String>>(1)?,
                    row.get::<_, i64>(2)?,
                    row.get::<_, Option<String>>(3)?,
                ))
            }).unwrap().filter_map(|r| r.ok()).collect()
        };

        for (id, token, port, _hostname) in tunnels {
            info!("Recovering tunnel {id}");
            if let Err(e) = self.start_tunnel_process(&id, token.as_deref(), port as u16).await {
                warn!("Failed to recover tunnel {id}: {e}");
                // Mark as stopped since we couldn't restart it
                let conn = self.db.lock().unwrap();
                let _ = conn.execute(
                    "UPDATE tunnel_configs SET status = 'error', config = json_set(config, '$.error_message', ?1) WHERE id = ?2",
                    rusqlite::params![e, id],
                );
            }
        }
    }

    /// Start a tunnel process (quick-connect mode if no token, named tunnel if token provided)
    pub async fn start_tunnel_process(
        &self,
        tunnel_id: &str,
        auth_token: Option<&str>,
        local_port: u16,
    ) -> Result<(), String> {
        let mut procs = self.processes.lock().await;
        if procs.contains_key(tunnel_id) {
            return Err("Tunnel process already running".into());
        }

        let mut cmd = Command::new(&self.cloudflared_path);
        cmd.stdout(std::process::Stdio::piped())
            .stderr(std::process::Stdio::piped())
            .kill_on_drop(true);

        if let Some(token) = auth_token {
            // Named tunnel mode: use token-based auth
            cmd.args(["tunnel", "--no-autoupdate", "run", "--token", token]);
        } else {
            // Quick-connect mode (TryCloudflare): no account needed
            cmd.args([
                "tunnel",
                "--no-autoupdate",
                "--url",
                &format!("http://localhost:{local_port}"),
            ]);
        }

        let mut child = cmd.spawn().map_err(|e| {
            format!("Failed to spawn cloudflared: {e}. Is cloudflared installed?")
        })?;

        let pid = child.id();
        let log_buffer: Arc<Mutex<VecDeque<LogLine>>> =
            Arc::new(Mutex::new(VecDeque::with_capacity(MAX_LOG_LINES)));
        let assigned_url: Arc<Mutex<Option<String>>> = Arc::new(Mutex::new(None));
        let connected: Arc<Mutex<bool>> = Arc::new(Mutex::new(false));

        let (shutdown_tx, shutdown_rx) = tokio::sync::watch::channel(false);

        // Capture stderr (cloudflared logs to stderr)
        if let Some(stderr) = child.stderr.take() {
            let buf = log_buffer.clone();
            let url_ref = assigned_url.clone();
            let conn_ref = connected.clone();
            tokio::spawn(async move {
                let reader = BufReader::new(stderr);
                let mut lines = reader.lines();
                while let Ok(Some(line)) = lines.next_line().await {
                    // Parse cloudflared output for the assigned URL
                    if line.contains(".trycloudflare.com") || line.contains("cfargotunnel.com") {
                        // Extract URL from log line
                        if let Some(url) = extract_url(&line) {
                            *url_ref.lock().await = Some(url);
                        }
                    }
                    if line.contains("Connection") && line.contains("registered") {
                        *conn_ref.lock().await = true;
                    }
                    if line.contains("Unregistered tunnel connection") {
                        *conn_ref.lock().await = false;
                    }

                    let log_line = parse_log_line(&line);
                    let mut b = buf.lock().await;
                    if b.len() >= MAX_LOG_LINES {
                        b.pop_front();
                    }
                    b.push_back(log_line);
                }
            });
        }

        // Also capture stdout
        if let Some(stdout) = child.stdout.take() {
            let buf = log_buffer.clone();
            let url_ref = assigned_url.clone();
            tokio::spawn(async move {
                let reader = BufReader::new(stdout);
                let mut lines = reader.lines();
                while let Ok(Some(line)) = lines.next_line().await {
                    if line.contains(".trycloudflare.com") || line.contains("cfargotunnel.com") {
                        if let Some(url) = extract_url(&line) {
                            *url_ref.lock().await = Some(url);
                        }
                    }

                    let log_line = parse_log_line(&line);
                    let mut b = buf.lock().await;
                    if b.len() >= MAX_LOG_LINES {
                        b.pop_front();
                    }
                    b.push_back(log_line);
                }
            });
        }

        // Spawn watchdog for auto-restart
        if self.auto_restart {
            let tunnel_id_owned = tunnel_id.to_string();
            let manager = self.clone();
            let mut shutdown_rx_clone = shutdown_rx.clone();
            tokio::spawn(async move {
                loop {
                    tokio::select! {
                        _ = shutdown_rx_clone.changed() => {
                            if *shutdown_rx_clone.borrow() {
                                break;
                            }
                        }
                        _ = tokio::time::sleep(std::time::Duration::from_secs(15)) => {
                            let is_running = {
                                let procs = manager.processes.lock().await;
                                procs.contains_key(&tunnel_id_owned)
                            };
                            if !is_running {
                                break;
                            }
                        }
                    }
                }
            });
        }

        let handle = TunnelHandle {
            _child: child,
            pid,
            started_at: chrono::Utc::now(),
            restart_count: 0,
            log_buffer,
            assigned_url,
            connected,
            shutdown_tx,
        };

        procs.insert(tunnel_id.to_string(), handle);
        info!("Started tunnel process for {tunnel_id} (pid: {pid:?})");
        Ok(())
    }

    /// Stop a running tunnel
    pub async fn stop_tunnel_process(&self, tunnel_id: &str) -> Result<(), String> {
        let mut procs = self.processes.lock().await;
        if let Some(mut handle) = procs.remove(tunnel_id) {
            // Signal the watchdog to stop
            let _ = handle.shutdown_tx.send(true);
            // Kill the child process
            if let Err(e) = handle._child.kill().await {
                warn!("Failed to kill tunnel {tunnel_id}: {e}");
            }
            info!("Stopped tunnel process for {tunnel_id}");
            Ok(())
        } else {
            Err("Tunnel process not found".into())
        }
    }

    /// Get process info for a tunnel
    pub async fn get_info(&self, tunnel_id: &str) -> Option<TunnelProcessInfo> {
        let procs = self.processes.lock().await;
        procs.get(tunnel_id).map(|h| {
            let url = h.assigned_url.try_lock().ok().and_then(|u| u.clone());
            let connected = h.connected.try_lock().ok().map(|c| *c).unwrap_or(false);
            TunnelProcessInfo {
                tunnel_id: tunnel_id.to_string(),
                pid: h.pid,
                started_at: h.started_at.to_rfc3339(),
                restart_count: h.restart_count,
                assigned_url: url,
                connected,
            }
        })
    }

    /// Get logs for a tunnel
    pub async fn get_logs(&self, tunnel_id: &str, limit: usize) -> Vec<LogLine> {
        let procs = self.processes.lock().await;
        if let Some(handle) = procs.get(tunnel_id) {
            let buf = handle.log_buffer.lock().await;
            let skip = if buf.len() > limit { buf.len() - limit } else { 0 };
            buf.iter().skip(skip).cloned().collect()
        } else {
            vec![]
        }
    }

    /// Check if a tunnel process is running
    pub async fn is_running(&self, tunnel_id: &str) -> bool {
        let procs = self.processes.lock().await;
        procs.contains_key(tunnel_id)
    }

    /// Get info for all running tunnels
    pub async fn list_running(&self) -> Vec<TunnelProcessInfo> {
        let procs = self.processes.lock().await;
        let mut result = Vec::new();
        for (id, h) in procs.iter() {
            let url = h.assigned_url.try_lock().ok().and_then(|u| u.clone());
            let connected = h.connected.try_lock().ok().map(|c| *c).unwrap_or(false);
            result.push(TunnelProcessInfo {
                tunnel_id: id.clone(),
                pid: h.pid,
                started_at: h.started_at.to_rfc3339(),
                restart_count: h.restart_count,
                assigned_url: url,
                connected,
            });
        }
        result
    }

    /// Shutdown all running tunnels (called on server shutdown)
    pub async fn shutdown_all(&self) {
        let mut procs = self.processes.lock().await;
        for (id, mut handle) in procs.drain() {
            let _ = handle.shutdown_tx.send(true);
            if let Err(e) = handle._child.kill().await {
                error!("Failed to kill tunnel {id}: {e}");
            } else {
                info!("Shutdown tunnel {id}");
            }
        }
    }
}

fn extract_url(line: &str) -> Option<String> {
    // cloudflared prints URLs like https://xxx.trycloudflare.com
    for word in line.split_whitespace() {
        let word = word.trim_matches(|c: char| !c.is_alphanumeric() && c != ':' && c != '/' && c != '.' && c != '-');
        if word.starts_with("https://") && (word.contains(".trycloudflare.com") || word.contains(".cfargotunnel.com")) {
            return Some(word.to_string());
        }
    }
    None
}

fn parse_log_line(line: &str) -> LogLine {
    let now = chrono::Utc::now().to_rfc3339();

    // cloudflared JSON log format: {"level":"info","msg":"...","time":"..."}
    if let Ok(val) = serde_json::from_str::<serde_json::Value>(line) {
        return LogLine {
            timestamp: val.get("time")
                .and_then(|v| v.as_str())
                .unwrap_or(&now)
                .to_string(),
            level: val.get("level")
                .and_then(|v| v.as_str())
                .unwrap_or("info")
                .to_string(),
            message: val.get("msg")
                .or_else(|| val.get("message"))
                .and_then(|v| v.as_str())
                .unwrap_or(line)
                .to_string(),
        };
    }

    // Fallback: plain text
    let level = if line.contains("ERR") || line.contains("error") {
        "error"
    } else if line.contains("WARN") || line.contains("warn") {
        "warn"
    } else {
        "info"
    };

    LogLine {
        timestamp: now,
        level: level.to_string(),
        message: line.to_string(),
    }
}
