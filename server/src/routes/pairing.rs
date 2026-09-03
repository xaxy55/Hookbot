//! Phone pairing over QR code.
//!
//! The dashboard (authenticated) mints a pairing token, renders it as a QR code
//! together with the server URL, and the phone POSTs the token back to redeem it
//! for a long-lived credential. Redemption is unauthenticated by necessity — the
//! phone has nothing yet — so the token is the only secret: 244 bits of entropy,
//! stored hashed, usable exactly once, and dead after two minutes.

use axum::http::{header::HeaderMap, StatusCode};
use axum::response::{IntoResponse, Response};
use axum::{Extension, Json};
use rusqlite::{Connection, OptionalExtension};
use serde::{Deserialize, Serialize};
use sha2::{Digest, Sha256};
use std::sync::Arc;

use crate::auth::{client_ip_from_headers, LoginRateLimiter, UserId};
use crate::config::AppConfig;
use crate::db::DbPool;

/// Long enough to point a phone at the screen, short enough that a QR code
/// photographed over someone's shoulder is worthless by the time it is used.
pub const PAIRING_TTL_SECS: i64 = 120;

/// Redemption is public, so it gets its own IP rate limiter. Wrapped in a
/// newtype because axum keys extensions by type and the login limiter is
/// already registered as a `LoginRateLimiter`.
#[derive(Clone)]
pub struct PairingRateLimiter(pub LoginRateLimiter);

impl PairingRateLimiter {
    pub fn new(max_attempts: u32, window_secs: u64) -> Self {
        Self(LoginRateLimiter::new(max_attempts, window_secs))
    }
}

// ── Wire types ──

#[derive(Serialize)]
pub struct PairingToken {
    /// The secret that goes into the QR code. Never stored in plaintext.
    pub token: String,
    /// UTC, "YYYY-MM-DD HH:MM:SS" (SQLite's datetime format), like the rest of the API.
    pub expires_at: String,
    /// Convenience for the countdown in the dashboard, so it never has to guess
    /// how far the browser clock has drifted from the server's.
    pub expires_in_secs: i64,
}

#[derive(Deserialize)]
pub struct RedeemRequest {
    pub token: String,
}

// ── Token lifecycle (pure DB logic, unit tested below) ──

#[derive(Debug, PartialEq, Eq)]
pub enum RedeemError {
    /// No token by that name was ever minted (or it was already cleaned up).
    Unknown,
    /// Minted, but past its expiry.
    Expired,
    /// Minted and still fresh, but a phone already redeemed it.
    AlreadyUsed,
    /// The database refused the write.
    Storage,
}

fn generate_token() -> String {
    // Two v4 UUIDs = 244 bits from the OS CSPRNG. Matches how the rest of the
    // codebase mints secrets (API keys, personal access tokens) without pulling
    // in another dependency.
    format!(
        "hbp_{}{}",
        uuid::Uuid::new_v4().simple(),
        uuid::Uuid::new_v4().simple()
    )
}

fn hash_token(token: &str) -> String {
    hex::encode(Sha256::digest(token.as_bytes()))
}

/// Mint a pairing token for `user_id` (None in single-user mode).
/// Returns the plaintext token — the caller's only chance to see it — and its expiry.
pub fn create_token(
    conn: &Connection,
    user_id: Option<&str>,
    ttl_secs: i64,
) -> rusqlite::Result<(String, String)> {
    // Opportunistic cleanup: redeemed and stale rows have no further use.
    let _ = conn.execute(
        "DELETE FROM pairing_tokens WHERE expires_at < datetime('now', '-1 hour')",
        [],
    );

    let token = generate_token();
    let token_hash = hash_token(&token);
    let modifier = format!("{:+} seconds", ttl_secs);

    conn.execute(
        "INSERT INTO pairing_tokens (token_hash, user_id, expires_at)
         VALUES (?1, ?2, datetime('now', ?3))",
        rusqlite::params![token_hash, user_id, modifier],
    )?;

    let expires_at: String = conn.query_row(
        "SELECT expires_at FROM pairing_tokens WHERE token_hash = ?1",
        [&token_hash],
        |row| row.get(0),
    )?;

    Ok((token, expires_at))
}

/// Redeem a pairing token, burning it in the process.
/// Returns the user id it was minted for (None in single-user mode).
pub fn consume_token(conn: &Connection, token: &str) -> Result<Option<String>, RedeemError> {
    let token_hash = hash_token(token);

    let row: Option<(Option<String>, bool, bool)> = conn
        .query_row(
            "SELECT user_id, used_at IS NOT NULL, expires_at <= datetime('now')
             FROM pairing_tokens WHERE token_hash = ?1",
            [&token_hash],
            |r| Ok((r.get(0)?, r.get(1)?, r.get(2)?)),
        )
        .optional()
        .map_err(|e| {
            tracing::error!("Pairing token lookup failed: {}", e);
            RedeemError::Storage
        })?;

    let (user_id, used, expired) = row.ok_or(RedeemError::Unknown)?;
    if used {
        return Err(RedeemError::AlreadyUsed);
    }
    if expired {
        return Err(RedeemError::Expired);
    }

    // Conditional update, so two phones racing for the same token cannot both win.
    let burned = conn
        .execute(
            "UPDATE pairing_tokens SET used_at = datetime('now')
             WHERE token_hash = ?1 AND used_at IS NULL",
            [&token_hash],
        )
        .map_err(|e| {
            tracing::error!("Failed to burn pairing token: {}", e);
            RedeemError::Storage
        })?;

    if burned == 0 {
        return Err(RedeemError::AlreadyUsed);
    }

    Ok(user_id)
}

// ── Handlers ──

/// POST /api/auth/pair — mint a pairing token. Requires a dashboard session.
pub async fn create_pairing_token(
    Extension(UserId(user_id)): Extension<UserId>,
    Extension(db): Extension<DbPool>,
) -> Response {
    let conn = db.lock().unwrap();

    match create_token(&conn, user_id.as_deref(), PAIRING_TTL_SECS) {
        Ok((token, expires_at)) => (
            StatusCode::OK,
            Json(PairingToken {
                token,
                expires_at,
                expires_in_secs: PAIRING_TTL_SECS,
            }),
        )
            .into_response(),
        Err(e) => {
            tracing::error!("Failed to create pairing token: {}", e);
            (
                StatusCode::INTERNAL_SERVER_ERROR,
                Json(serde_json::json!({ "error": "Failed to create pairing token" })),
            )
                .into_response()
        }
    }
}

/// POST /api/auth/pair/redeem — exchange a pairing token for a credential.
/// Public: the phone has no credential yet, so the token is the whole proof.
pub async fn redeem_pairing_token(
    Extension(_config): Extension<Arc<AppConfig>>,
    Extension(db): Extension<DbPool>,
    Extension(rate_limiter): Extension<PairingRateLimiter>,
    headers: HeaderMap,
    Json(body): Json<RedeemRequest>,
) -> Response {
    let client_ip = client_ip_from_headers(&headers).unwrap_or_else(|| "unknown".to_string());

    if let Err(retry_after) = rate_limiter.0.check_and_increment(&client_ip).await {
        tracing::warn!("Pairing redemption rate limited for IP {}", client_ip);
        return (
            StatusCode::TOO_MANY_REQUESTS,
            Json(serde_json::json!({
                "error": "Too many pairing attempts. Try again later.",
                "retry_after_secs": retry_after,
            })),
        )
            .into_response();
    }

    let conn = db.lock().unwrap();

    let user_id = match consume_token(&conn, body.token.trim()) {
        Ok(uid) => uid,
        Err(RedeemError::Expired) => {
            return (
                StatusCode::GONE,
                Json(serde_json::json!({
                    "error": "This pairing code has expired. Generate a new one in the dashboard.",
                    "reason": "expired",
                })),
            )
                .into_response();
        }
        Err(RedeemError::AlreadyUsed) => {
            return (
                StatusCode::CONFLICT,
                Json(serde_json::json!({
                    "error": "This pairing code has already been used. Generate a new one in the dashboard.",
                    "reason": "already_used",
                })),
            )
                .into_response();
        }
        Err(RedeemError::Unknown) => {
            return (
                StatusCode::NOT_FOUND,
                Json(serde_json::json!({
                    "error": "Unrecognised pairing code.",
                    "reason": "unknown",
                })),
            )
                .into_response();
        }
        Err(RedeemError::Storage) => {
            return (
                StatusCode::INTERNAL_SERVER_ERROR,
                Json(serde_json::json!({ "error": "Failed to redeem pairing code" })),
            )
                .into_response();
        }
    };

    match user_id {
        // Multi-tenant mode: hand the phone its own revocable personal access
        // token rather than the account's primary API key, so a lost phone can be
        // cut off from Account → API Tokens without rotating anything else.
        Some(uid) => {
            let id = uuid::Uuid::new_v4().to_string();
            let token = format!("hb_{}", uuid::Uuid::new_v4().to_string().replace("-", ""));
            let preview = format!("...{}", &token[token.len().saturating_sub(8)..]);
            let name = "Paired phone";

            if let Err(e) = conn.execute(
                "INSERT INTO user_api_tokens (id, user_id, token, token_preview, name)
                 VALUES (?1, ?2, ?3, ?4, ?5)",
                rusqlite::params![id, uid, token, preview, name],
            ) {
                tracing::error!("Failed to issue paired-device token: {}", e);
                return (
                    StatusCode::INTERNAL_SERVER_ERROR,
                    Json(serde_json::json!({ "error": "Failed to issue credential" })),
                )
                    .into_response();
            }

            let email: String = conn
                .query_row("SELECT email FROM users WHERE id = ?1", [&uid], |row| {
                    row.get(0)
                })
                .unwrap_or_default();

            tracing::info!("Paired a phone for user {}", uid);

            (
                StatusCode::OK,
                Json(serde_json::json!({
                    "ok": true,
                    "mode": "workos",
                    "api_key": token,
                    "user_id": uid,
                    "email": email,
                })),
            )
                .into_response()
        }
        // Single-user mode: mint a revocable personal token the same way the
        // multi-tenant branch above does, rather than handing out
        // config.api_key. Two reasons: a lost phone can be cut off from
        // Account -> API Tokens without rotating the server's own shared
        // secret, and config.api_key is operator-chosen and may contain bytes
        // an HTTP header cannot carry (see check_api_key's byte-exact compare
        // for why that used to fail silently), where a minted hb_ token never
        // can.
        None => {
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
            let name = "Paired phone";

            if let Err(e) = conn.execute(
                "INSERT INTO user_api_tokens (id, user_id, token, token_preview, name)
                 VALUES (?1, ?2, ?3, ?4, ?5)",
                rusqlite::params![id, uid, token, preview, name],
            ) {
                tracing::error!("Failed to issue paired-device token: {}", e);
                return (
                    StatusCode::INTERNAL_SERVER_ERROR,
                    Json(serde_json::json!({ "error": "Failed to issue credential" })),
                )
                    .into_response();
            }

            tracing::info!("Paired a phone in single-user mode");
            (
                StatusCode::OK,
                Json(serde_json::json!({
                    "ok": true,
                    "mode": "legacy",
                    "api_key": token,
                })),
            )
                .into_response()
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::db;

    /// Multi-tenant tokens carry a user_id, and the FK needs a row to point at.
    fn insert_user(conn: &Connection, id: &str) {
        conn.execute(
            "INSERT INTO users (id, workos_id, email, name, api_key)
             VALUES (?1, ?2, ?3, '', ?4)",
            rusqlite::params![id, format!("workos_{}", id), "dev@example.test", "key-1"],
        )
        .expect("insert user");
    }

    #[test]
    fn valid_token_redeems_and_returns_its_user() {
        let conn = db::open_memory();
        insert_user(&conn, "user-1");

        let (token, expires_at) =
            create_token(&conn, Some("user-1"), PAIRING_TTL_SECS).expect("mint");

        assert!(token.starts_with("hbp_"));
        assert!(!expires_at.is_empty());
        assert_eq!(consume_token(&conn, &token), Ok(Some("user-1".to_string())));
    }

    #[test]
    fn single_user_mode_token_redeems_without_a_user() {
        let conn = db::open_memory();
        let (token, _) = create_token(&conn, None, PAIRING_TTL_SECS).expect("mint");

        assert_eq!(consume_token(&conn, &token), Ok(None));
    }

    #[test]
    fn expired_token_is_rejected() {
        let conn = db::open_memory();
        // Minted with a TTL already in the past.
        let (token, _) = create_token(&conn, None, -10).expect("mint");

        assert_eq!(consume_token(&conn, &token), Err(RedeemError::Expired));
    }

    #[test]
    fn token_is_single_use() {
        let conn = db::open_memory();
        let (token, _) = create_token(&conn, None, PAIRING_TTL_SECS).expect("mint");

        assert_eq!(consume_token(&conn, &token), Ok(None));
        assert_eq!(consume_token(&conn, &token), Err(RedeemError::AlreadyUsed));
    }

    #[test]
    fn unknown_token_is_rejected() {
        let conn = db::open_memory();
        // Mint one so the table is non-empty — an unrelated token must still fail.
        let _ = create_token(&conn, None, PAIRING_TTL_SECS).expect("mint");

        assert_eq!(
            consume_token(&conn, "hbp_not-a-real-token"),
            Err(RedeemError::Unknown)
        );
    }

    #[test]
    fn plaintext_token_is_never_stored() {
        let conn = db::open_memory();
        let (token, _) = create_token(&conn, None, PAIRING_TTL_SECS).expect("mint");

        let stored: String = conn
            .query_row("SELECT token_hash FROM pairing_tokens", [], |r| r.get(0))
            .expect("row");

        assert_ne!(stored, token);
        assert_eq!(stored, hash_token(&token));
        // A leaked database row must not be redeemable as-is.
        assert_eq!(consume_token(&conn, &stored), Err(RedeemError::Unknown));
    }

    #[test]
    fn each_token_is_distinct() {
        let conn = db::open_memory();
        let (a, _) = create_token(&conn, None, PAIRING_TTL_SECS).expect("mint");
        let (b, _) = create_token(&conn, None, PAIRING_TTL_SECS).expect("mint");

        assert_ne!(a, b);
        // 4 chars of prefix + two 32-char hex UUIDs.
        assert_eq!(a.len(), 4 + 64);
    }
}
