use axum::body::Body;
use axum::extract::Request;
use axum::http::{header, StatusCode};
use axum::middleware::Next;
use axum::response::{IntoResponse, Response};
use axum::Extension;
use axum::Json;
use hmac::{Hmac, Mac};
use serde::{Deserialize, Serialize};
use sha2::Sha256;
use std::sync::Arc;
use std::time::{SystemTime, UNIX_EPOCH};

use crate::config::AppConfig;

type HmacSha256 = Hmac<Sha256>;

const SESSION_COOKIE_NAME: &str = "hookbot_session";
const SESSION_MAX_AGE_SECS: u64 = 7 * 24 * 60 * 60; // 7 days

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
    Json(body): Json<LoginRequest>,
) -> Response {
    match bcrypt::verify(&body.password, &config.admin_password_hash) {
        Ok(true) => {
            let token = create_session_token(&config.session_secret);
            let cookie = format!(
                "{}={}; HttpOnly; SameSite=Lax; Path=/; Max-Age={}",
                SESSION_COOKIE_NAME, token, SESSION_MAX_AGE_SECS
            );

            (
                StatusCode::OK,
                [(header::SET_COOKIE, cookie)],
                Json(LoginResponse { ok: true }),
            )
                .into_response()
        }
        _ => (
            StatusCode::UNAUTHORIZED,
            Json(serde_json::json!({ "error": "Invalid password" })),
        )
            .into_response(),
    }
}

pub async fn logout() -> Response {
    let cookie = format!(
        "{}=; HttpOnly; SameSite=Lax; Path=/; Max-Age=0",
        SESSION_COOKIE_NAME
    );

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
