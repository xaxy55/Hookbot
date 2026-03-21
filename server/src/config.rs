use std::env;
use std::fs;
use std::path::PathBuf;

use hmac::{Hmac, Mac};
use sha2::Sha256;
use tracing::{info, warn};

type HmacSha256 = Hmac<Sha256>;

#[derive(Debug, Clone)]
pub struct AppConfig {
    pub database_url: PathBuf,
    pub firmware_dir: PathBuf,
    pub bind_addr: String,
    pub poll_interval_secs: u64,
    pub log_retention_hours: u64,
    #[allow(dead_code)]
    pub mdns_prefix: String,
    pub tls_cert_path: Option<String>,
    pub tls_key_path: Option<String>,
    pub anthropic_api_key: Option<String>,
    pub openai_api_key: Option<String>,
    #[allow(dead_code)]
    pub calendar_url: Option<String>,
    pub api_key: String,
    pub admin_password_hash: String,
    pub session_secret: [u8; 32],
    pub allowed_origins: Vec<String>,
    pub login_rate_limit_max: u32,
    pub login_rate_limit_window_secs: u64,
    pub workos_client_id: Option<String>,
    pub workos_api_key: Option<String>,
    pub workos_redirect_uri: Option<String>,
    pub cookie_domain: Option<String>,
    pub frontend_url: Option<String>,
    pub cloudflared_path: String,
    pub tunnel_auto_restart: bool,
}

impl AppConfig {
    pub fn from_env() -> Self {
        let database_url = env::var("DATABASE_URL")
            .unwrap_or_else(|_| "data/deskbot.db".to_string())
            .into();

        let firmware_dir: PathBuf = env::var("FIRMWARE_DIR")
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

        let tls_cert_path = env::var("TLS_CERT_PATH").ok();
        let tls_key_path = env::var("TLS_KEY_PATH").ok();
        let anthropic_api_key = env::var("ANTHROPIC_API_KEY").ok();
        let openai_api_key = env::var("OPENAI_API_KEY").ok();
        let calendar_url = env::var("CALENDAR_URL").ok();

        // Auth: API key
        let api_key = Self::resolve_api_key(&firmware_dir);

        // Auth: Admin password
        let admin_password_hash = Self::resolve_admin_password();

        // Session signing secret derived from API key
        let mut mac = HmacSha256::new_from_slice(api_key.as_bytes())
            .expect("HMAC can take key of any size");
        mac.update(b"hookbot-session");
        let result = mac.finalize();
        let mut session_secret = [0u8; 32];
        session_secret.copy_from_slice(&result.into_bytes());

        // CORS: allowed origins (comma-separated), empty = mirror request (permissive)
        let allowed_origins: Vec<String> = env::var("ALLOWED_ORIGINS")
            .unwrap_or_default()
            .split(',')
            .map(|s| s.trim().to_string())
            .filter(|s| !s.is_empty())
            .collect();
        if allowed_origins.is_empty() {
            warn!("ALLOWED_ORIGINS not set — CORS will allow any origin. Set ALLOWED_ORIGINS for production.");
        } else {
            info!("CORS allowed origins: {:?}", allowed_origins);
        }

        // Rate limiting for login
        let login_rate_limit_max = env::var("LOGIN_RATE_LIMIT_MAX")
            .ok()
            .and_then(|v| v.parse().ok())
            .unwrap_or(5);
        let login_rate_limit_window_secs = env::var("LOGIN_RATE_LIMIT_WINDOW_SECS")
            .ok()
            .and_then(|v| v.parse().ok())
            .unwrap_or(300); // 5 minutes

        // WorkOS multi-tenant auth
        let workos_client_id = env::var("WORKOS_CLIENT_ID").ok().filter(|s| !s.is_empty());
        let workos_api_key = env::var("WORKOS_API_KEY").ok().filter(|s| !s.is_empty());
        let workos_redirect_uri = env::var("WORKOS_REDIRECT_URI").ok().filter(|s| !s.is_empty());
        let cookie_domain = env::var("COOKIE_DOMAIN").ok().filter(|s| !s.is_empty());
        let frontend_url = env::var("FRONTEND_URL").ok().filter(|s| !s.is_empty());
        if workos_client_id.is_some() {
            info!("WorkOS multi-tenant mode enabled");
        }

        // Cloudflare Tunnel settings
        let cloudflared_path = env::var("CLOUDFLARED_PATH")
            .unwrap_or_else(|_| "cloudflared".to_string());
        let tunnel_auto_restart = env::var("TUNNEL_AUTO_RESTART")
            .ok()
            .map(|v| v != "false" && v != "0")
            .unwrap_or(true);

        Self {
            database_url,
            firmware_dir,
            bind_addr,
            poll_interval_secs,
            log_retention_hours,
            mdns_prefix,
            tls_cert_path,
            tls_key_path,
            anthropic_api_key,
            openai_api_key,
            calendar_url,
            api_key,
            admin_password_hash,
            session_secret,
            allowed_origins,
            login_rate_limit_max,
            login_rate_limit_window_secs,
            workos_client_id,
            workos_api_key,
            workos_redirect_uri,
            cookie_domain,
            frontend_url,
            cloudflared_path,
            tunnel_auto_restart,
        }
    }

    fn resolve_api_key(data_dir: &PathBuf) -> String {
        // 1. Check env var
        if let Ok(key) = env::var("API_KEY") {
            if !key.is_empty() {
                info!("Using API key from API_KEY env var");
                return key;
            }
        }

        // 2. Check data/api_key file
        let key_file = data_dir.parent().unwrap_or(data_dir).join("api_key");
        if let Ok(key) = fs::read_to_string(&key_file) {
            let key = key.trim().to_string();
            if !key.is_empty() {
                info!("Using API key from {:?}", key_file);
                return key;
            }
        }

        // 3. Auto-generate
        let key = uuid::Uuid::new_v4().to_string();
        if let Some(parent) = key_file.parent() {
            let _ = fs::create_dir_all(parent);
        }
        match fs::write(&key_file, &key) {
            Ok(_) => info!("Generated new API key, saved to {:?}", key_file),
            Err(e) => warn!("Could not save API key to {:?}: {}", key_file, e),
        }
        info!("=== AUTO-GENERATED API KEY: {} ===", key);
        key
    }

    fn resolve_admin_password() -> String {
        // 1. Check pre-hashed password
        if let Ok(hash) = env::var("ADMIN_PASSWORD_HASH") {
            if !hash.is_empty() {
                info!("Using admin password hash from ADMIN_PASSWORD_HASH env var");
                return hash;
            }
        }

        // 2. Check plaintext password, hash it
        if let Ok(password) = env::var("ADMIN_PASSWORD") {
            if !password.is_empty() {
                info!("Hashing admin password from ADMIN_PASSWORD env var");
                return bcrypt::hash(&password, bcrypt::DEFAULT_COST)
                    .expect("Failed to hash admin password");
            }
        }

        // 3. Auto-generate
        let password = uuid::Uuid::new_v4().to_string().split('-').next().unwrap().to_string();
        let hash = bcrypt::hash(&password, bcrypt::DEFAULT_COST)
            .expect("Failed to hash auto-generated password");
        info!("=== AUTO-GENERATED ADMIN PASSWORD: {} ===", password);
        hash
    }
}
