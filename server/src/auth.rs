use axum::body::Body;
use axum::extract::{ConnectInfo, Request};
use axum::http::{header, StatusCode};
use axum::middleware::Next;
use axum::response::{IntoResponse, Response};
use axum::Extension;
use axum::Json;
use hmac::{Hmac, Mac};
use serde::{Deserialize, Serialize};
use sha2::Sha256;
use std::collections::HashMap;
use std::net::SocketAddr;
use std::sync::Arc;
use std::time::{SystemTime, UNIX_EPOCH};
use tokio::sync::Mutex;
use tracing::warn;

use crate::config::AppConfig;

type HmacSha256 = Hmac<Sha256>;

const SESSION_COOKIE_NAME: &str = "hookbot_session";
const SESSION_MAX_AGE_SECS: u64 = 7 * 24 * 60 * 60; // 7 days

/// Per-IP login attempt tracking for rate limiting.
#[derive(Clone)]
pub struct LoginRateLimiter {
    /// Map of IP -> (attempt_count, window_start_epoch_secs)
    attempts: Arc<Mutex<HashMap<String, (u32, u64)>>>,
    max_attempts: u32,
    window_secs: u64,
}

impl LoginRateLimiter {
    pub fn new(max_attempts: u32, window_secs: u64) -> Self {
        Self {
            attempts: Arc::new(Mutex::new(HashMap::new())),
            max_attempts,
            window_secs,
        }
    }

    /// Returns Ok(()) if the request is allowed, Err(secs_until_reset) if rate limited.
    async fn check_and_increment(&self, ip: &str) -> Result<(), u64> {
        let now = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .unwrap()
            .as_secs();

        let mut attempts = self.attempts.lock().await;

        let entry = attempts.entry(ip.to_string()).or_insert((0, now));

        // Reset window if expired
        if now >= entry.1 + self.window_secs {
            *entry = (0, now);
        }

        if entry.0 >= self.max_attempts {
            let retry_after = (entry.1 + self.window_secs).saturating_sub(now);
            return Err(retry_after);
        }

        entry.0 += 1;
        Ok(())
    }

    /// Clear attempts for an IP after successful login.
    async fn clear(&self, ip: &str) {
        self.attempts.lock().await.remove(ip);
    }
}

/// Middleware that requires either a valid API key or session cookie.
pub async fn require_auth(
    Extension(config): Extension<Arc<AppConfig>>,
    req: Request<Body>,
    next: Next,
) -> Response {
    // Check API key in headers
    if check_api_key(&req, &config.api_key) {
        return next.run(req).await;
    }

    // Check session cookie
    if check_session_cookie(&req, &config.session_secret) {
        return next.run(req).await;
    }

    (
        StatusCode::UNAUTHORIZED,
        Json(serde_json::json!({ "error": "Authentication required" })),
    )
        .into_response()
}

fn check_api_key(req: &Request<Body>, expected_key: &str) -> bool {
    // Check Authorization: Bearer <key>
    if let Some(auth) = req.headers().get(header::AUTHORIZATION) {
        if let Ok(auth_str) = auth.to_str() {
            if let Some(token) = auth_str.strip_prefix("Bearer ") {
                return constant_time_eq(token.trim(), expected_key);
            }
        }
    }

    // Check X-API-Key header
    if let Some(key) = req.headers().get("x-api-key") {
        if let Ok(key_str) = key.to_str() {
            return constant_time_eq(key_str.trim(), expected_key);
        }
    }

    false
}

fn check_session_cookie(req: &Request<Body>, secret: &[u8; 32]) -> bool {
    let cookie_header = match req.headers().get(header::COOKIE) {
        Some(c) => match c.to_str() {
            Ok(s) => s,
            Err(_) => return false,
        },
        None => return false,
    };

    // Parse cookies to find our session cookie
    for cookie in cookie_header.split(';') {
        let cookie = cookie.trim();
        if let Some(value) = cookie.strip_prefix(&format!("{}=", SESSION_COOKIE_NAME)) {
            return validate_session_token(value, secret);
        }
    }

    false
}

fn validate_session_token(token: &str, secret: &[u8; 32]) -> bool {
    // Format: timestamp:hmac_hex
    let parts: Vec<&str> = token.splitn(2, ':').collect();
    if parts.len() != 2 {
        return false;
    }

    let timestamp: u64 = match parts[0].parse() {
        Ok(t) => t,
        Err(_) => return false,
    };

    let now = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .unwrap()
        .as_secs();

    // Check expiry
    if now > timestamp + SESSION_MAX_AGE_SECS {
        return false;
    }

    // Verify HMAC
    let expected_mac = compute_session_hmac(timestamp, secret);
    constant_time_eq(parts[1], &expected_mac)
}

fn create_session_token(secret: &[u8; 32]) -> String {
    let timestamp = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .unwrap()
        .as_secs();
    let hmac = compute_session_hmac(timestamp, secret);
    format!("{}:{}", timestamp, hmac)
}

fn compute_session_hmac(timestamp: u64, secret: &[u8; 32]) -> String {
    let mut mac = HmacSha256::new_from_slice(secret).expect("HMAC can take key of any size");
    mac.update(timestamp.to_be_bytes().as_ref());
    hex::encode(mac.finalize().into_bytes())
}

fn constant_time_eq(a: &str, b: &str) -> bool {
    if a.len() != b.len() {
        return false;
    }
    a.as_bytes()
        .iter()
        .zip(b.as_bytes().iter())
        .fold(0u8, |acc, (x, y)| acc | (x ^ y))
        == 0
}

/// Build the session cookie string, adding Secure flag when TLS is enabled.
fn build_session_cookie(token: &str, max_age: u64, tls_enabled: bool) -> String {
    let secure = if tls_enabled { "; Secure" } else { "" };
    format!(
        "{}={}; HttpOnly; SameSite=Lax; Path=/; Max-Age={}{}",
        SESSION_COOKIE_NAME, token, max_age, secure
    )
}

/// Extract client IP from ConnectInfo or X-Forwarded-For header.
fn extract_client_ip(req: &Request<Body>) -> String {
    // Check X-Forwarded-For first (for reverse proxy / Cloudflare)
    if let Some(forwarded) = req.headers().get("x-forwarded-for") {
        if let Ok(val) = forwarded.to_str() {
            if let Some(first_ip) = val.split(',').next() {
                return first_ip.trim().to_string();
            }
        }
    }

    // Fall back to CF-Connecting-IP (Cloudflare)
    if let Some(cf_ip) = req.headers().get("cf-connecting-ip") {
        if let Ok(val) = cf_ip.to_str() {
            return val.trim().to_string();
        }
    }

    // Fall back to peer address from extensions
    if let Some(connect_info) = req.extensions().get::<ConnectInfo<SocketAddr>>() {
        return connect_info.0.ip().to_string();
    }

    "unknown".to_string()
}

// --- Handlers ---

#[derive(Deserialize)]
pub struct LoginRequest {
    pub password: String,
}

#[derive(Serialize)]
pub struct LoginResponse {
    pub ok: bool,
}

#[derive(Serialize)]
pub struct AuthStatusResponse {
    pub authenticated: bool,
}

pub async fn login(
    Extension(config): Extension<Arc<AppConfig>>,
    Extension(rate_limiter): Extension<LoginRateLimiter>,
    req: Request<Body>,
) -> Response {
    let client_ip = extract_client_ip(&req);

    // Check rate limit
    if let Err(retry_after) = rate_limiter.check_and_increment(&client_ip).await {
        warn!("Login rate limited for IP {}", client_ip);
        return (
            StatusCode::TOO_MANY_REQUESTS,
            [(header::HeaderName::from_static("retry-after"), retry_after.to_string())],
            Json(serde_json::json!({
                "error": "Too many login attempts. Try again later.",
                "retry_after_secs": retry_after,
            })),
        )
            .into_response();
    }

    // Parse body manually since we already consumed req for IP extraction
    let body_bytes = match axum::body::to_bytes(req.into_body(), 1024 * 16).await {
        Ok(b) => b,
        Err(_) => {
            return (
                StatusCode::BAD_REQUEST,
                Json(serde_json::json!({ "error": "Invalid request body" })),
            )
                .into_response();
        }
    };
    let body: LoginRequest = match serde_json::from_slice(&body_bytes) {
        Ok(b) => b,
        Err(_) => {
            return (
                StatusCode::BAD_REQUEST,
                Json(serde_json::json!({ "error": "Invalid JSON" })),
            )
                .into_response();
        }
    };

    match bcrypt::verify(&body.password, &config.admin_password_hash) {
        Ok(true) => {
            // Clear rate limit on successful login
            rate_limiter.clear(&client_ip).await;

            let tls_enabled = config.tls_cert_path.is_some();
            let token = create_session_token(&config.session_secret);
            let cookie = build_session_cookie(&token, SESSION_MAX_AGE_SECS, tls_enabled);

            (
                StatusCode::OK,
                [(header::SET_COOKIE, cookie)],
                Json(LoginResponse { ok: true }),
            )
                .into_response()
        }
        _ => {
            warn!("Failed login attempt from IP {}", client_ip);
            (
                StatusCode::UNAUTHORIZED,
                Json(serde_json::json!({ "error": "Invalid password" })),
            )
                .into_response()
        }
    }
}

pub async fn logout(
    Extension(config): Extension<Arc<AppConfig>>,
) -> Response {
    let tls_enabled = config.tls_cert_path.is_some();
    let cookie = build_session_cookie("", 0, tls_enabled);

    (
        StatusCode::OK,
        [(header::SET_COOKIE, cookie)],
        Json(LoginResponse { ok: true }),
    )
        .into_response()
}

pub async fn auth_status(
    Extension(config): Extension<Arc<AppConfig>>,
    req: Request<Body>,
) -> Json<AuthStatusResponse> {
    let authenticated = check_api_key(&req, &config.api_key)
        || check_session_cookie(&req, &config.session_secret);
    Json(AuthStatusResponse { authenticated })
}

/// Rotate the API key — generates a new key and saves it to disk.
/// Requires the current API key for authorization (already enforced by auth middleware).
pub async fn rotate_api_key(
    Extension(config): Extension<Arc<AppConfig>>,
) -> Response {
    let new_key = uuid::Uuid::new_v4().to_string();

    // Save to disk
    let key_file = config.firmware_dir.parent().unwrap_or(&config.firmware_dir).join("api_key");
    match std::fs::write(&key_file, &new_key) {
        Ok(_) => {
            tracing::info!("API key rotated, saved to {:?}", key_file);
            (
                StatusCode::OK,
                Json(serde_json::json!({
                    "ok": true,
                    "api_key": new_key,
                    "message": "API key rotated. Update all clients with the new key. Server restart required for the new key to take effect.",
                })),
            )
                .into_response()
        }
        Err(e) => {
            tracing::error!("Failed to save rotated API key: {}", e);
            (
                StatusCode::INTERNAL_SERVER_ERROR,
                Json(serde_json::json!({ "error": format!("Failed to save new key: {}", e) })),
            )
                .into_response()
        }
    }
}
