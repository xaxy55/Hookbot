use axum::extract::State;
use axum::http::StatusCode;
use axum::response::{IntoResponse, Response};
use axum::{Extension, Json};
use serde::{Deserialize, Serialize};

use crate::auth::UserId;
use crate::db::DbPool;

// ── API Token types ──

#[derive(Serialize)]
pub struct ApiToken {
    pub id: String,
    pub name: String,
    pub token_preview: String, // last 8 chars
    pub created_at: String,
    pub last_used_at: Option<String>,
}

#[derive(Deserialize)]
pub struct CreateTokenRequest {
    pub name: String,
}

#[derive(Serialize)]
pub struct CreateTokenResponse {
    pub id: String,
    pub name: String,
    pub token: String, // full token, only shown once
}

// ── QR Login types ──

#[derive(Serialize)]
pub struct QrLoginCode {
    pub code: String,
    pub expires_at: String,
}

#[derive(Deserialize)]
pub struct QrLoginExchange {
    pub code: String,
}

// ── Account info ──

#[derive(Deserialize)]
pub struct UpdateAccountRequest {
    pub name: Option<String>,
}

// ── Handlers ──

/// GET /api/account/tokens - list user's API tokens
pub async fn list_tokens(
    Extension(UserId(user_id)): Extension<UserId>,
    State(db): State<DbPool>,
) -> Response {
    let uid = match user_id {
        Some(id) => id,
        None => return (StatusCode::OK, Json(serde_json::json!([]))).into_response(),
    };

    let conn = db.lock().unwrap();
    let mut stmt = conn
        .prepare(
            "SELECT id, name, token_preview, created_at, last_used_at
             FROM user_api_tokens
             WHERE user_id = ?1 AND revoked_at IS NULL
             ORDER BY created_at DESC",
        )
        .unwrap();

    let tokens: Vec<ApiToken> = stmt
        .query_map([&uid], |row| {
            Ok(ApiToken {
                id: row.get(0)?,
                name: row.get(1)?,
                token_preview: row.get(2)?,
                created_at: row.get(3)?,
                last_used_at: row.get(4)?,
            })
        })
        .unwrap()
        .filter_map(|r| r.ok())
        .collect();

    (StatusCode::OK, Json(serde_json::json!(tokens))).into_response()
}

/// POST /api/account/tokens - create a new API token
pub async fn create_token(
    Extension(UserId(user_id)): Extension<UserId>,
    State(db): State<DbPool>,
    Json(body): Json<CreateTokenRequest>,
) -> Response {
    let uid = match user_id {
        Some(id) => id,
        None => {
            return (
                StatusCode::UNAUTHORIZED,
                Json(serde_json::json!({ "error": "Multi-tenant auth required" })),
            )
                .into_response()
        }
    };

    let name = body.name.trim().to_string();
    if name.is_empty() || name.len() > 64 {
        return (
            StatusCode::BAD_REQUEST,
            Json(serde_json::json!({ "error": "Token name must be 1-64 characters" })),
        )
            .into_response();
    }

    let id = uuid::Uuid::new_v4().to_string();
    let token = format!("hb_{}", uuid::Uuid::new_v4().to_string().replace("-", ""));
    let preview = format!("...{}", &token[token.len().saturating_sub(8)..]);

    let conn = db.lock().unwrap();
    match conn.execute(
        "INSERT INTO user_api_tokens (id, user_id, token, token_preview, name) VALUES (?1, ?2, ?3, ?4, ?5)",
        rusqlite::params![id, uid, token, preview, name],
    ) {
        Ok(_) => (
            StatusCode::CREATED,
            Json(serde_json::json!(CreateTokenResponse { id, name, token })),
        )
            .into_response(),
        Err(e) => {
            tracing::error!("Failed to create API token: {}", e);
            (
                StatusCode::INTERNAL_SERVER_ERROR,
                Json(serde_json::json!({ "error": "Failed to create token" })),
            )
                .into_response()
        }
    }
}

/// DELETE /api/account/tokens/:id - revoke an API token
pub async fn revoke_token(
    Extension(UserId(user_id)): Extension<UserId>,
    State(db): State<DbPool>,
    axum::extract::Path(token_id): axum::extract::Path<String>,
) -> Response {
    let uid = match user_id {
        Some(id) => id,
        None => {
            return (
                StatusCode::UNAUTHORIZED,
                Json(serde_json::json!({ "error": "Multi-tenant auth required" })),
            )
                .into_response()
        }
    };

    let conn = db.lock().unwrap();
    let affected = conn
        .execute(
            "UPDATE user_api_tokens SET revoked_at = datetime('now') WHERE id = ?1 AND user_id = ?2 AND revoked_at IS NULL",
            rusqlite::params![token_id, uid],
        )
        .unwrap_or(0);

    if affected == 0 {
        return (
            StatusCode::NOT_FOUND,
            Json(serde_json::json!({ "error": "Token not found" })),
        )
            .into_response();
    }

    (StatusCode::OK, Json(serde_json::json!({ "ok": true }))).into_response()
}

/// POST /api/account/qr-login - generate a temporary QR login code
pub async fn generate_qr_login(
    Extension(UserId(user_id)): Extension<UserId>,
    State(db): State<DbPool>,
) -> Response {
    let uid = match user_id {
        Some(id) => id,
        None => {
            return (
                StatusCode::UNAUTHORIZED,
                Json(serde_json::json!({ "error": "Multi-tenant auth required" })),
            )
                .into_response()
        }
    };

    // Generate a random 32-char code
    let code = format!("qr_{}", uuid::Uuid::new_v4().to_string().replace("-", ""));

    let conn = db.lock().unwrap();
    // Clean up expired codes first
    let _ = conn.execute(
        "DELETE FROM qr_login_codes WHERE expires_at < datetime('now')",
        [],
    );

    match conn.execute(
        "INSERT INTO qr_login_codes (code, user_id, expires_at) VALUES (?1, ?2, datetime('now', '+5 minutes'))",
        rusqlite::params![code, uid],
    ) {
        Ok(_) => {
            let expires_at: String = conn
                .query_row(
                    "SELECT expires_at FROM qr_login_codes WHERE code = ?1",
                    [&code],
                    |row| row.get(0),
                )
                .unwrap_or_default();

            (
                StatusCode::OK,
                Json(serde_json::json!(QrLoginCode { code, expires_at })),
            )
                .into_response()
        }
        Err(e) => {
            tracing::error!("Failed to create QR login code: {}", e);
            (
                StatusCode::INTERNAL_SERVER_ERROR,
                Json(serde_json::json!({ "error": "Failed to generate code" })),
            )
                .into_response()
        }
    }
}

/// POST /api/auth/qr-exchange - exchange QR code for API key (public, called by mobile app)
pub async fn exchange_qr_login(
    State(db): State<DbPool>,
    Json(body): Json<QrLoginExchange>,
) -> Response {
    let conn = db.lock().unwrap();

    // Find valid, unused code
    let result: Result<(String, String), _> = conn.query_row(
        "SELECT qr.user_id, u.api_key
         FROM qr_login_codes qr
         JOIN users u ON u.id = qr.user_id
         WHERE qr.code = ?1 AND qr.expires_at > datetime('now') AND qr.used = 0",
        [&body.code],
        |row| Ok((row.get(0)?, row.get(1)?)),
    );

    match result {
        Ok((user_id, api_key)) => {
            // Mark code as used
            let _ = conn.execute(
                "UPDATE qr_login_codes SET used = 1 WHERE code = ?1",
                [&body.code],
            );

            // Get user info
            let email: String = conn
                .query_row("SELECT email FROM users WHERE id = ?1", [&user_id], |row| {
                    row.get(0)
                })
                .unwrap_or_default();

            (
                StatusCode::OK,
                Json(serde_json::json!({
                    "ok": true,
                    "api_key": api_key,
                    "user_id": user_id,
                    "email": email,
                })),
            )
                .into_response()
        }
        Err(_) => (
            StatusCode::NOT_FOUND,
            Json(serde_json::json!({ "error": "Invalid or expired code" })),
        )
            .into_response(),
    }
}

/// PUT /api/account - update account info
pub async fn update_account(
    Extension(UserId(user_id)): Extension<UserId>,
    State(db): State<DbPool>,
    Json(body): Json<UpdateAccountRequest>,
) -> Response {
    let uid = match user_id {
        Some(id) => id,
        None => {
            return (
                StatusCode::UNAUTHORIZED,
                Json(serde_json::json!({ "error": "Multi-tenant auth required" })),
            )
                .into_response()
        }
    };

    let conn = db.lock().unwrap();

    if let Some(name) = &body.name {
        let name = name.trim();
        if name.is_empty() {
            return (
                StatusCode::BAD_REQUEST,
                Json(serde_json::json!({ "error": "Name cannot be empty" })),
            )
                .into_response();
        }
        let _ = conn.execute(
            "UPDATE users SET name = ?1 WHERE id = ?2",
            rusqlite::params![name, uid],
        );
    }

    // Return updated user info
    match conn.query_row(
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
    ) {
        Ok(user) => (StatusCode::OK, Json(user)).into_response(),
        Err(_) => (
            StatusCode::NOT_FOUND,
            Json(serde_json::json!({ "error": "User not found" })),
        )
            .into_response(),
    }
}
