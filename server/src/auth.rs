use axum::body::Body;
use axum::extract::{ConnectInfo, Query, Request};
use axum::http::{header, StatusCode};
use axum::middleware::Next;
use axum::response::{IntoResponse, Redirect, Response};
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
use crate::db::DbPool;

type HmacSha256 = Hmac<Sha256>;

const SESSION_COOKIE_NAME: &str = "hookbot_session";
const SESSION_MAX_AGE_SECS: u64 = 7 * 24 * 60 * 60; // 7 days

/// Represents the authenticated user's ID (Some in multi-tenant mode, None in legacy mode).
#[derive(Debug, Clone)]
pub struct UserId(pub Option<String>);

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
/// In WorkOS multi-tenant mode, also checks per-user API keys and WorkOS session cookies.
pub async fn require_auth(
    Extension(config): Extension<Arc<AppConfig>>,
    Extension(db): Extension<DbPool>,
    mut req: Request<Body>,
    next: Next,
) -> Response {
    if config.workos_client_id.is_some() {
        // WorkOS multi-tenant mode

        // Check per-user API key in headers
        if let Some(user_id) = check_user_api_key(&req, &db) {
            req.extensions_mut().insert(UserId(Some(user_id)));
            return next.run(req).await;
        }

        // Check WorkOS session cookie (new format: user_id:timestamp:hmac)
        if let Some(user_id) = check_workos_session_cookie(&req, &config.session_secret) {
            req.extensions_mut().insert(UserId(Some(user_id)));
            return next.run(req).await;
        }

        (
            StatusCode::UNAUTHORIZED,
            Json(serde_json::json!({ "error": "Authentication required" })),
        )
            .into_response()
    } else {
        // Legacy single-user mode

        // Check global API key in headers
        if check_api_key(&req, &config.api_key) {
            req.extensions_mut().insert(UserId(None));
            return next.run(req).await;
        }

        // Check legacy session cookie (timestamp:hmac)
        if check_session_cookie(&req, &config.session_secret) {
            req.extensions_mut().insert(UserId(None));
            return next.run(req).await;
        }

        (
            StatusCode::UNAUTHORIZED,
            Json(serde_json::json!({ "error": "Authentication required" })),
        )
            .into_response()
    }
}

/// Look up a user by their per-user API key from the request headers.
fn check_user_api_key(req: &Request<Body>, db: &DbPool) -> Option<String> {
    let key = extract_api_key_from_headers(req)?;
    let conn = db.lock().unwrap();
    conn.query_row(
        "SELECT id FROM users WHERE api_key = ?1",
        [&key],
        |row| row.get(0),
    )
    .ok()
}

/// Extract API key from Authorization: Bearer or X-API-Key header.
fn extract_api_key_from_headers(req: &Request<Body>) -> Option<String> {
    if let Some(auth) = req.headers().get(header::AUTHORIZATION) {
        if let Ok(auth_str) = auth.to_str() {
            if let Some(token) = auth_str.strip_prefix("Bearer ") {
                return Some(token.trim().to_string());
            }
        }
    }
    if let Some(key) = req.headers().get("x-api-key") {
        if let Ok(key_str) = key.to_str() {
            return Some(key_str.trim().to_string());
        }
    }
    None
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

/// Check for WorkOS session cookie with format: user_id:timestamp:hmac
fn check_workos_session_cookie(req: &Request<Body>, secret: &[u8; 32]) -> Option<String> {
    let cookie_header = req.headers().get(header::COOKIE)?.to_str().ok()?;

    for cookie in cookie_header.split(';') {
        let cookie = cookie.trim();
        if let Some(value) = cookie.strip_prefix(&format!("{}=", SESSION_COOKIE_NAME)) {
            return validate_workos_session_token(value, secret);
        }
    }

    None
}

/// Validate old-format session token: timestamp:hmac
fn validate_session_token(token: &str, secret: &[u8; 32]) -> bool {
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

/// Validate new-format WorkOS session token: user_id:timestamp:hmac
/// Returns Some(user_id) on success.
fn validate_workos_session_token(token: &str, secret: &[u8; 32]) -> Option<String> {
    let parts: Vec<&str> = token.splitn(3, ':').collect();
    if parts.len() != 3 {
        return None;
    }

    let user_id = parts[0];
    let timestamp: u64 = parts[1].parse().ok()?;

    let now = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .unwrap()
        .as_secs();

    if now > timestamp + SESSION_MAX_AGE_SECS {
        return None;
    }

    let expected_mac = compute_workos_session_hmac(user_id, timestamp, secret);
    if constant_time_eq(parts[2], &expected_mac) {
        Some(user_id.to_string())
    } else {
        None
    }
}

fn create_session_token(secret: &[u8; 32]) -> String {
    let timestamp = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .unwrap()
        .as_secs();
    let hmac = compute_session_hmac(timestamp, secret);
    format!("{}:{}", timestamp, hmac)
}

fn create_workos_session_token(user_id: &str, secret: &[u8; 32]) -> String {
    let timestamp = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .unwrap()
        .as_secs();
    let hmac = compute_workos_session_hmac(user_id, timestamp, secret);
    format!("{}:{}:{}", user_id, timestamp, hmac)
}

fn compute_session_hmac(timestamp: u64, secret: &[u8; 32]) -> String {
    let mut mac = HmacSha256::new_from_slice(secret).expect("HMAC can take key of any size");
    mac.update(timestamp.to_be_bytes().as_ref());
    hex::encode(mac.finalize().into_bytes())
}

fn compute_workos_session_hmac(user_id: &str, timestamp: u64, secret: &[u8; 32]) -> String {
    let mut mac = HmacSha256::new_from_slice(secret).expect("HMAC can take key of any size");
    mac.update(user_id.as_bytes());
    mac.update(b":");
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
fn build_session_cookie(token: &str, max_age: u64, tls_enabled: bool, cookie_domain: Option<&str>) -> String {
    let (secure, same_site) = if cookie_domain.is_some() {
        // Cross-subdomain: need SameSite=None + Secure for cross-origin fetch
        ("; Secure", "None")
    } else if tls_enabled {
        ("; Secure", "Lax")
    } else {
        ("", "Lax")
    };
    let domain = cookie_domain.map(|d| format!("; Domain={}", d)).unwrap_or_default();
    format!(
        "{}={}; HttpOnly; SameSite={}; Path=/; Max-Age={}{}{}",
        SESSION_COOKIE_NAME, token, same_site, max_age, secure, domain
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
    #[serde(skip_serializing_if = "Option::is_none")]
    pub workos_enabled: Option<bool>,
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
            let cookie = build_session_cookie(&token, SESSION_MAX_AGE_SECS, tls_enabled, config.cookie_domain.as_deref());

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
    let cookie = build_session_cookie("", 0, tls_enabled, config.cookie_domain.as_deref());

    (
        StatusCode::OK,
        [(header::SET_COOKIE, cookie)],
        Json(LoginResponse { ok: true }),
    )
        .into_response()
}

pub async fn auth_status(
    Extension(config): Extension<Arc<AppConfig>>,
    Extension(db): Extension<DbPool>,
    req: Request<Body>,
) -> Json<AuthStatusResponse> {
    let workos_enabled = config.workos_client_id.is_some();

    let authenticated = if workos_enabled {
        check_user_api_key(&req, &db).is_some()
            || check_workos_session_cookie(&req, &config.session_secret).is_some()
    } else {
        check_api_key(&req, &config.api_key)
            || check_session_cookie(&req, &config.session_secret)
    };

    Json(AuthStatusResponse {
        authenticated,
        workos_enabled: if workos_enabled { Some(true) } else { None },
    })
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

// --- WorkOS OAuth handlers ---

#[derive(Deserialize)]
pub struct CallbackQuery {
    pub code: String,
}

/// GET /auth/login - redirect to WorkOS AuthKit
pub async fn workos_login(
    Extension(config): Extension<Arc<AppConfig>>,
) -> Response {
    let client_id = match &config.workos_client_id {
        Some(id) => id,
        None => {
            return (
                StatusCode::NOT_FOUND,
                Json(serde_json::json!({ "error": "WorkOS not configured" })),
            )
                .into_response();
        }
    };

    let redirect_uri = config
        .workos_redirect_uri
        .as_deref()
        .unwrap_or("http://localhost:3000/auth/callback");

    let url = format!(
        "https://api.workos.com/user_management/authorize?client_id={}&redirect_uri={}&response_type=code&provider=authkit",
        urlencoding(client_id),
        urlencoding(redirect_uri),
    );

    Redirect::temporary(&url).into_response()
}

/// GET /auth/callback?code=... - exchange code for user, create session
pub async fn workos_callback(
    Extension(config): Extension<Arc<AppConfig>>,
    Extension(db): Extension<DbPool>,
    Query(params): Query<CallbackQuery>,
) -> Response {
    let client_id = match &config.workos_client_id {
        Some(id) => id.clone(),
        None => {
            return (
                StatusCode::NOT_FOUND,
                Json(serde_json::json!({ "error": "WorkOS not configured" })),
            )
                .into_response();
        }
    };

    let api_key = match &config.workos_api_key {
        Some(k) => k.clone(),
        None => {
            return (
                StatusCode::INTERNAL_SERVER_ERROR,
                Json(serde_json::json!({ "error": "WorkOS API key not configured" })),
            )
                .into_response();
        }
    };

    // Exchange code for user info
    let client = reqwest::Client::new();
    let resp = client
        .post("https://api.workos.com/user_management/authenticate")
        .header("Authorization", format!("Bearer {}", api_key))
        .json(&serde_json::json!({
            "client_id": client_id,
            "code": params.code,
            "grant_type": "authorization_code",
        }))
        .send()
        .await;

    let resp = match resp {
        Ok(r) => r,
        Err(e) => {
            tracing::error!("WorkOS authenticate request failed: {}", e);
            return (
                StatusCode::BAD_GATEWAY,
                Json(serde_json::json!({ "error": "Failed to contact WorkOS" })),
            )
                .into_response();
        }
    };

    if !resp.status().is_success() {
        let status = resp.status();
        let body = resp.text().await.unwrap_or_default();
        tracing::error!("WorkOS authenticate error {}: {}", status, body);
        return (
            StatusCode::BAD_GATEWAY,
            Json(serde_json::json!({ "error": format!("WorkOS error: {}", status) })),
        )
            .into_response();
    }

    let body: serde_json::Value = match resp.json().await {
        Ok(v) => v,
        Err(e) => {
            tracing::error!("WorkOS response parse error: {}", e);
            return (
                StatusCode::BAD_GATEWAY,
                Json(serde_json::json!({ "error": "Invalid WorkOS response" })),
            )
                .into_response();
        }
    };

    // Extract user info from response
    let workos_user_id = body["user"]["id"].as_str().unwrap_or("").to_string();
    let email = body["user"]["email"].as_str().unwrap_or("").to_string();
    let first_name = body["user"]["first_name"].as_str().unwrap_or("");
    let last_name = body["user"]["last_name"].as_str().unwrap_or("");
    let name = format!("{} {}", first_name, last_name).trim().to_string();

    if workos_user_id.is_empty() || email.is_empty() {
        return (
            StatusCode::BAD_GATEWAY,
            Json(serde_json::json!({ "error": "WorkOS returned incomplete user data" })),
        )
            .into_response();
    }

    // Upsert user in DB
    let user_id = {
        let conn = db.lock().unwrap();

        // Try to find existing user
        let existing: Option<String> = conn
            .query_row(
                "SELECT id FROM users WHERE workos_id = ?1",
                [&workos_user_id],
                |row| row.get(0),
            )
            .ok();

        if let Some(id) = existing {
            // Update name/email in case they changed
            let _ = conn.execute(
                "UPDATE users SET email = ?1, name = ?2 WHERE id = ?3",
                rusqlite::params![email, name, id],
            );
            id
        } else {
            // Create new user with generated API key
            let id = uuid::Uuid::new_v4().to_string();
            let user_api_key = uuid::Uuid::new_v4().to_string();
            conn.execute(
                "INSERT INTO users (id, workos_id, email, name, api_key) VALUES (?1, ?2, ?3, ?4, ?5)",
                rusqlite::params![id, workos_user_id, email, name, user_api_key],
            )
            .unwrap_or_else(|e| {
                tracing::error!("Failed to insert user: {}", e);
                0
            });
            tracing::info!("Created new user {} for WorkOS user {}", id, workos_user_id);
            id
        }
    };

    // Create session cookie with user_id
    let tls_enabled = config.tls_cert_path.is_some();
    let token = create_workos_session_token(&user_id, &config.session_secret);
    let cookie = build_session_cookie(&token, SESSION_MAX_AGE_SECS, tls_enabled, config.cookie_domain.as_deref());

    let redirect_url = config.frontend_url.clone().unwrap_or_else(|| "/".to_string());

    (
        StatusCode::FOUND,
        [
            (header::SET_COOKIE, cookie),
            (header::LOCATION, redirect_url),
        ],
    )
        .into_response()
}

/// GET /api/auth/me - return current user info and API key
pub async fn get_me(
    Extension(config): Extension<Arc<AppConfig>>,
    Extension(db): Extension<DbPool>,
    req: Request<Body>,
) -> Response {
    // In WorkOS mode, return user info
    if config.workos_client_id.is_some() {
        // Try to get user_id from extension (set by middleware)
        let user_id = req.extensions().get::<UserId>().and_then(|u| u.0.clone());

        if let Some(uid) = user_id {
            let conn = db.lock().unwrap();
            let user = conn.query_row(
                "SELECT id, email, name, api_key, created_at FROM users WHERE id = ?1",
                [&uid],
                |row| {
                    Ok(serde_json::json!({
                        "id": row.get::<_, String>(0)?,
                        "email": row.get::<_, String>(1)?,
                        "name": row.get::<_, String>(2)?,
                        "api_key": row.get::<_, String>(3)?,
                        "created_at": row.get::<_, String>(4)?,
                    }))
                },
            );

            match user {
                Ok(u) => (StatusCode::OK, Json(u)).into_response(),
                Err(_) => (
                    StatusCode::NOT_FOUND,
                    Json(serde_json::json!({ "error": "User not found" })),
                )
                    .into_response(),
            }
        } else {
            (
                StatusCode::UNAUTHORIZED,
                Json(serde_json::json!({ "error": "Not authenticated" })),
            )
                .into_response()
        }
    } else {
        // Legacy mode - return global API key
        (
            StatusCode::OK,
            Json(serde_json::json!({
                "mode": "legacy",
                "api_key": config.api_key,
            })),
        )
            .into_response()
    }
}

/// Simple URL encoding for query parameters
fn urlencoding(s: &str) -> String {
    s.chars()
        .map(|c| match c {
            'A'..='Z' | 'a'..='z' | '0'..='9' | '-' | '_' | '.' | '~' => c.to_string(),
            _ => format!("%{:02X}", c as u8),
        })
        .collect()
}
