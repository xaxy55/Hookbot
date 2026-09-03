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
use rusqlite::OptionalExtension;

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
    pub(crate) async fn check_and_increment(&self, ip: &str) -> Result<(), u64> {
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
    // Allow CORS preflight requests through without auth
    if req.method() == axum::http::Method::OPTIONS {
        return next.run(req).await;
    }

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

        // Check a token minted via Account -> API Tokens (or phone pairing,
        // which now issues the same kind of token). Without this, every
        // token "Create API Token" produces is unusable outside WorkOS mode —
        // it gets written to user_api_tokens, but nothing here ever reads
        // that table for a single-admin deployment.
        if check_local_api_token(&req, &db) {
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
/// Checks both the user's default api_key and any active personal access tokens.
fn check_user_api_key(req: &Request<Body>, db: &DbPool) -> Option<String> {
    let key = extract_api_key_from_headers(req)?;
    let conn = db.lock().unwrap();

    // Check default user api_key first
    if let Ok(id) = conn.query_row(
        "SELECT id FROM users WHERE api_key = ?1",
        [&key],
        |row| row.get::<_, String>(0),
    ) {
        return Some(id);
    }

    // Check personal access tokens (user_api_tokens)
    if let Ok(user_id) = conn.query_row(
        "SELECT user_id FROM user_api_tokens WHERE token = ?1 AND revoked_at IS NULL",
        [&key],
        |row| row.get::<_, String>(0),
    ) {
        // Update last_used_at
        let _ = conn.execute(
            "UPDATE user_api_tokens SET last_used_at = datetime('now') WHERE token = ?1",
            [&key],
        );
        return Some(user_id);
    }

    None
}

/// Legacy-mode counterpart to check_user_api_key: same user_api_tokens table,
/// but without requiring a matching user_id, since single-admin mode has only
/// the one (synthetic) user.
fn check_local_api_token(req: &Request<Body>, db: &DbPool) -> bool {
    let Some(key) = extract_api_key_from_headers(req) else { return false };
    let conn = db.lock().unwrap();
    let found: Option<String> = conn
        .query_row(
            "SELECT id FROM user_api_tokens WHERE token = ?1 AND revoked_at IS NULL",
            [&key],
            |row| row.get(0),
        )
        .optional()
        .unwrap_or(None);
    let Some(token_id) = found else { return false };
    let _ = conn.execute(
        "UPDATE user_api_tokens SET last_used_at = datetime('now') WHERE id = ?1",
        [&token_id],
    );
    true
}

/// Extract API key from Authorization: Bearer or X-API-Key header.
fn extract_api_key_from_headers(req: &Request<Body>) -> Option<String> {
    if let Some(auth) = req.headers().get(header::AUTHORIZATION) {
        let bytes = trim_ascii_ws(auth.as_bytes());
        if let Some(token) = bytes.strip_prefix(b"Bearer ") {
            // Lossy on purpose: an API key with a non-ASCII byte still needs a
            // String here to look up in the (UTF-8) database, and this path is
            // only used to *find* a token, never to prove equality with one —
            // check_api_key below compares the original bytes exactly.
            return Some(String::from_utf8_lossy(trim_ascii_ws(token)).into_owned());
        }
    }
    if let Some(key) = req.headers().get("x-api-key") {
        return Some(String::from_utf8_lossy(trim_ascii_ws(key.as_bytes())).into_owned());
    }
    None
}

fn trim_ascii_ws(b: &[u8]) -> &[u8] {
    let b = match b.iter().position(|c| !c.is_ascii_whitespace()) {
        Some(i) => &b[i..],
        None => return &[],
    };
    match b.iter().rposition(|c| !c.is_ascii_whitespace()) {
        Some(i) => &b[..=i],
        None => &[],
    }
}

fn check_api_key(req: &Request<Body>, expected_key: &str) -> bool {
    let expected = expected_key.as_bytes();

    // Check Authorization: Bearer <key>
    if let Some(auth) = req.headers().get(header::AUTHORIZATION) {
        let bytes = trim_ascii_ws(auth.as_bytes());
        if let Some(token) = bytes.strip_prefix(b"Bearer ") {
            return constant_time_eq_bytes(trim_ascii_ws(token), expected);
        }
    }

    // Check X-API-Key header
    if let Some(key) = req.headers().get("x-api-key") {
        return constant_time_eq_bytes(trim_ascii_ws(key.as_bytes()), expected);
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

fn constant_time_eq_bytes(a: &[u8], b: &[u8]) -> bool {
    if a.len() != b.len() {
        return false;
    }
    a.iter().zip(b.iter()).fold(0u8, |acc, (x, y)| acc | (x ^ y)) == 0
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

/// Extract client IP from the proxy headers, if any are present.
pub(crate) fn client_ip_from_headers(headers: &header::HeaderMap) -> Option<String> {
    // Check X-Forwarded-For first (for reverse proxy / Cloudflare)
    if let Some(forwarded) = headers.get("x-forwarded-for") {
        if let Ok(val) = forwarded.to_str() {
            if let Some(first_ip) = val.split(',').next() {
                return Some(first_ip.trim().to_string());
            }
        }
    }

    // Fall back to CF-Connecting-IP (Cloudflare)
    if let Some(cf_ip) = headers.get("cf-connecting-ip") {
        if let Ok(val) = cf_ip.to_str() {
            return Some(val.trim().to_string());
        }
    }

    None
}

/// Extract client IP from ConnectInfo or X-Forwarded-For header.
fn extract_client_ip(req: &Request<Body>) -> String {
    if let Some(ip) = client_ip_from_headers(req.headers()) {
        return ip;
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
        // Must accept the same credentials require_auth does, including tokens
        // from Account -> API Tokens. Otherwise a client holding a working
        // token is told it is not signed in, while every other endpoint
        // accepts it.
        check_api_key(&req, &config.api_key)
            || check_local_api_token(&req, &db)
            || check_session_cookie(&req, &config.session_secret)
    };

    // Always report the mode. Returning null for single-admin left clients
    // unable to tell "this server has no WorkOS" from "this server is too old
    // to say", so the iOS app led with a WorkOS button that 404s here.
    Json(AuthStatusResponse {
        authenticated,
        workos_enabled: Some(workos_enabled),
    })
}

/// Rotate the API key — generates a new key and saves it to disk.
/// Requires the current API key for authorization (already enforced by auth middleware).
pub async fn rotate_api_key(
    Extension(config): Extension<Arc<AppConfig>>,
) -> Response {
    let new_key = uuid::Uuid::new_v4().to_string();

    // Save to disk — resolve path to prevent traversal
    let base_dir = config.firmware_dir.parent().unwrap_or(&config.firmware_dir);
    let key_file = base_dir.join("api_key");
    let canonical_base = match base_dir.canonicalize() {
        Ok(p) => p,
        Err(e) => {
            tracing::error!("Failed to resolve base dir: {}", e);
            return (
                StatusCode::INTERNAL_SERVER_ERROR,
                Json(serde_json::json!({ "error": "Failed to resolve storage directory" })),
            ).into_response();
        }
    };
    let canonical_key_file = key_file.canonicalize().unwrap_or_else(|_| canonical_base.join("api_key"));
    if !canonical_key_file.starts_with(&canonical_base) {
        return (
            StatusCode::INTERNAL_SERVER_ERROR,
            Json(serde_json::json!({ "error": "Invalid key file path" })),
        ).into_response();
    }
    match std::fs::write(&canonical_key_file, &new_key) {
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
    pub state: Option<String>,
}

#[derive(Deserialize)]
pub struct LoginQuery {
    pub mobile_redirect: Option<String>,
}

/// GET /auth/login - redirect to WorkOS AuthKit
pub async fn workos_login(
    Extension(config): Extension<Arc<AppConfig>>,
    Query(params): Query<LoginQuery>,
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

    // Pass mobile_redirect via state parameter so it survives the OAuth round-trip
    let state = params
        .mobile_redirect
        .as_deref()
        .map(|r| format!("mobile:{}", r))
        .unwrap_or_default();

    let mut url = format!(
        "https://api.workos.com/user_management/authorize?client_id={}&redirect_uri={}&response_type=code&provider=authkit",
        urlencoding(client_id),
        urlencoding(redirect_uri),
    );

    if !state.is_empty() {
        url.push_str(&format!("&state={}", urlencoding(&state)));
    }

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
        .json(&serde_json::json!({
            "client_id": client_id,
            "client_secret": api_key,
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
                StatusCode::SERVICE_UNAVAILABLE,
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
            StatusCode::UNAUTHORIZED,
            Json(serde_json::json!({ "error": format!("WorkOS error: {}", status) })),
        )
            .into_response();
    }

    let body: serde_json::Value = match resp.json().await {
        Ok(v) => v,
        Err(e) => {
            tracing::error!("WorkOS response parse error: {}", e);
            return (
                StatusCode::INTERNAL_SERVER_ERROR,
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
            StatusCode::INTERNAL_SERVER_ERROR,
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

    // Check if this is a mobile OAuth flow via the state parameter
    let mobile_redirect = params
        .state
        .as_deref()
        .and_then(|s| s.strip_prefix("mobile:"))
        .map(|s| s.to_string());

    if let Some(mobile_url) = mobile_redirect {
        // Mobile flow: redirect to app's custom URL scheme with the user's API key
        let user_api_key = {
            let conn = db.lock().unwrap();
            conn.query_row(
                "SELECT api_key FROM users WHERE id = ?1",
                [&user_id],
                |row| row.get::<_, String>(0),
            )
            .unwrap_or_default()
        };

        let redirect = format!(
            "{}?api_key={}&email={}",
            mobile_url,
            urlencoding(&user_api_key),
            urlencoding(&email),
        );

        return (StatusCode::FOUND, [(header::LOCATION, redirect)])
            .into_response();
    }

    // Web flow: set session cookie and redirect to frontend
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
        // Legacy mode: mint a revocable personal token rather than handing
        // back config.api_key. Same reasoning as phone pairing: config.api_key
        // is operator-chosen and may contain a byte an HTTP header cannot
        // carry as visible ASCII, which silently made both this endpoint's
        // output and the reqwest client that has to send it back unusable
        // together — the CLI's `hookbot login` would save a key it could
        // never actually present.
        let conn = db.lock().unwrap();
        let uid = match crate::routes::account::local_admin_id(&conn) {
            Ok(id) => id,
            Err(e) => {
                tracing::error!("Failed to resolve local admin: {}", e);
                return (
                    StatusCode::INTERNAL_SERVER_ERROR,
                    Json(serde_json::json!({ "error": "Failed to issue credential" })),
                )
                    .into_response();
            }
        };
        let id = uuid::Uuid::new_v4().to_string();
        let token = format!("hb_{}", uuid::Uuid::new_v4().to_string().replace("-", ""));
        let preview = format!("...{}", &token[token.len().saturating_sub(8)..]);

        if let Err(e) = conn.execute(
            "INSERT INTO user_api_tokens (id, user_id, token, token_preview, name)
             VALUES (?1, ?2, ?3, ?4, 'CLI login')",
            rusqlite::params![id, uid, token, preview],
        ) {
            tracing::error!("Failed to issue CLI login token: {}", e);
            return (
                StatusCode::INTERNAL_SERVER_ERROR,
                Json(serde_json::json!({ "error": "Failed to issue credential" })),
            )
                .into_response();
        }

        (
            StatusCode::OK,
            Json(serde_json::json!({
                "mode": "legacy",
                "api_key": token,
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

#[cfg(test)]
mod header_key_tests {
    use super::*;

    fn request_with_header(name: &str, value: &[u8]) -> Request<Body> {
        Request::builder()
            .header(name, value)
            .body(Body::empty())
            .unwrap()
    }

    #[test]
    fn matches_a_non_ascii_key_byte_for_byte() {
        // A key an operator's password manager generated, containing a byte
        // that is valid in an HTTP header but not visible ASCII. This is what
        // silently broke every header-based login before: HeaderValue::to_str()
        // rejects such bytes, so the old code never even reached the compare.
        let key = "correct-horseø-battery-staple";
        let req = request_with_header("x-api-key", key.as_bytes());
        assert!(check_api_key(&req, key));
    }

    #[test]
    fn rejects_a_wrong_key_of_the_same_shape() {
        let req = request_with_header("x-api-key", "correct-horseø-battery-staple".as_bytes());
        assert!(!check_api_key(&req, "correct-horseø-battery-staaple"));
    }

    #[test]
    fn trims_surrounding_whitespace_on_bearer_and_header_forms() {
        let key = "plain-ascii-key";
        let bearer = request_with_header("authorization", format!("Bearer  {key}  ").as_bytes());
        assert!(check_api_key(&bearer, key));

        let header = request_with_header("x-api-key", format!("  {key}\t").as_bytes());
        assert!(check_api_key(&header, key));
    }

    #[test]
    fn extract_recovers_a_non_ascii_key_for_lookup() {
        let key = "tökén-with-accénts";
        let req = request_with_header("x-api-key", key.as_bytes());
        assert_eq!(extract_api_key_from_headers(&req).as_deref(), Some(key));
    }

    #[test]
    fn check_local_api_token_accepts_a_dashboard_minted_token_and_rejects_a_revoked_one() {
        let conn = crate::db::open_memory();
        conn.execute(
            "INSERT INTO users (id, workos_id, email, name, api_key) VALUES ('u1','u1','a@b','A','k1')",
            [],
        )
        .unwrap();
        conn.execute(
            "INSERT INTO user_api_tokens (id, user_id, token, token_preview, name) \
             VALUES ('t1','u1','hb_live','...live','Paired phone')",
            [],
        )
        .unwrap();
        conn.execute(
            "INSERT INTO user_api_tokens (id, user_id, token, token_preview, name, revoked_at) \
             VALUES ('t2','u1','hb_dead','...dead','Old phone', datetime('now'))",
            [],
        )
        .unwrap();
        let db: DbPool = std::sync::Arc::new(std::sync::Mutex::new(conn));

        let live = request_with_header("x-api-key", b"hb_live");
        assert!(check_local_api_token(&live, &db));

        let dead = request_with_header("x-api-key", b"hb_dead");
        assert!(!check_local_api_token(&dead, &db));

        let unknown = request_with_header("x-api-key", b"hb_never_issued");
        assert!(!check_local_api_token(&unknown, &db));
    }
}
