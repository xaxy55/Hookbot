use std::env;
use std::path::PathBuf;

#[derive(Debug, Clone)]
pub struct AppConfig {
    pub database_url: PathBuf,
    pub firmware_dir: PathBuf,
    pub bind_addr: String,
    pub poll_interval_secs: u64,
    pub log_retention_hours: u64,
    #[allow(dead_code)]
    pub mdns_prefix: String,
    #[allow(dead_code)]
    pub anthropic_api_key: Option<String>,
    #[allow(dead_code)]
    pub calendar_url: Option<String>,
}

impl AppConfig {
    pub fn from_env() -> Self {
        let database_url = env::var("DATABASE_URL")
            .unwrap_or_else(|_| "data/deskbot.db".to_string())
            .into();

        let firmware_dir = env::var("FIRMWARE_DIR")
            .unwrap_or_else(|_| "data/firmware".to_string())
            .into();

        let bind_addr = env::var("BIND_ADDR")
            .unwrap_or_else(|_| "0.0.0.0:3000".to_string());

        let poll_interval_secs = env::var("POLL_INTERVAL")
            .ok()
            .and_then(|v| v.parse().ok())
            .unwrap_or(10);

        let log_retention_hours = env::var("LOG_RETENTION_HOURS")
            .ok()
            .and_then(|v| v.parse().ok())
            .unwrap_or(24);

        let mdns_prefix = env::var("MDNS_PREFIX")
            .unwrap_or_else(|_| "hookbot".to_string());

        let anthropic_api_key = env::var("ANTHROPIC_API_KEY").ok();
        let calendar_url = env::var("CALENDAR_URL").ok();

        Self {
            database_url,
            firmware_dir,
            bind_addr,
            poll_interval_secs,
            log_retention_hours,
            mdns_prefix,
            anthropic_api_key,
            calendar_url,
        }
    }
}
