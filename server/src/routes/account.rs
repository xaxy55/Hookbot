use axum::extract::State;
use axum::http::StatusCode;
use axum::response::{IntoResponse, Response};
use axum::{Extension, Json};
use serde::{Deserialize, Serialize};

use crate::auth::UserId;
use crate::db::DbPool;


/// Single-admin deployments (ADMIN_PASSWORD, no WorkOS) have no `users` row, so
/// UserId is None even though require_auth has already authenticated the
/// request. Bind those tokens to a singleton local admin instead of refusing —
/// otherwise API tokens are unusable outside multi-tenant mode.
///
/// The row carries an empty password hash on purpose: it exists only to own
/// tokens and can never be used to log in.
const LOCAL_ADMIN_ID: &str = "local-admin";

fn local_admin_id(conn: &rusqlite::Connection) -> Result<String, rusqlite::Error> {
    conn.execute(
        "INSERT OR IGNORE INTO users (id, username, display_name, password_hash, role) \
         VALUES (?1, 'admin', 'Administrator', '', 'admin')",
        [LOCAL_ADMIN_ID],
    )?;
    Ok(LOCAL_ADMIN_ID.to_string())
}

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
    let conn = db.lock().unwrap();
    let uid = match user_id {
        Some(id) => id,
        None => match local_admin_id(&conn) {
            Ok(id) => id,
            Err(_) => return (StatusCode::OK, Json(serde_json::json!([]))).into_response(),
        },
    };
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
    let uid = match user_id {
        Some(id) => id,
        None => match local_admin_id(&conn) {
            Ok(id) => id,
            Err(e) => {
                return (
                    StatusCode::INTERNAL_SERVER_ERROR,
                    Json(serde_json::json!({ "error": format!("Could not resolve account: {e}") })),
                )
                    .into_response()
            }
        },
    };
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
    let conn = db.lock().unwrap();
    let uid = match user_id {
        Some(id) => id,
        None => match local_admin_id(&conn) {
            Ok(id) => id,
            Err(_) => {
                return (
                    StatusCode::NOT_FOUND,
                    Json(serde_json::json!({ "error": "Token not found" })),
                )
                    .into_response()
            }
        },
    };
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
