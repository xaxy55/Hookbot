//! Device-facing API endpoints for cloud-connected ESP32 devices.
//!
//! These endpoints are called by ESP devices (not users). They authenticate
//! via `X-Device-Token` header rather than user API keys/sessions.
//!
//! Endpoints:
//! - POST /api/device/register   — self-register on first boot
//! - POST /api/device/heartbeat  — push status (replaces server polling)
//! - GET  /api/device/commands   — long-poll for pending commands
//! - POST /api/device/commands/:id/ack — acknowledge command execution
//! - POST /api/devices/claim     — user claims device by code (user-authed)

use axum::extract::{Path, Query, Request, State};
use axum::http::StatusCode;
use axum::response::{IntoResponse, Response};
use axum::Json;
use serde::{Deserialize, Serialize};
use serde_json::json;
use tracing::info;

use crate::auth::UserId;
use crate::db::DbPool;
use crate::error::AppError;
use crate::services::command_queue::CommandQueue;

// --- Claim code generation ---

const CLAIM_CHARSET: &[u8] = b"ABCDEFGHJKMNPQRSTUVWXYZ23456789"; // excludes O/0/I/1/L

fn generate_claim_code() -> String {
    use std::time::{SystemTime, UNIX_EPOCH};
    let seed = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .unwrap()
        .as_nanos();
    // Simple pseudo-random using seed + uuid entropy
    let uuid_bytes = uuid::Uuid::new_v4();
    let bytes = uuid_bytes.as_bytes();
    (0..6)
        .map(|i| {
            let idx = (bytes[i] as usize ^ ((seed >> (i * 4)) as usize)) % CLAIM_CHARSET.len();
            CLAIM_CHARSET[idx] as char
        })
        .collect()
}

fn hash_token(token: &str) -> String {
    use sha2::{Digest, Sha256};
    let mut hasher = Sha256::new();
    hasher.update(token.as_bytes());
    hex::encode(hasher.finalize())
}

// --- Types ---

#[derive(Debug, Deserialize)]
#[allow(dead_code)]
pub struct RegisterRequest {
    pub hostname: String,
    pub mac_address: Option<String>,
    pub firmware_version: Option<String>,
    pub device_type: Option<String>,
}

#[derive(Debug, Serialize)]
pub struct RegisterResponse {
    pub device_id: String,
    pub device_token: String,
    pub claim_code: String,
    pub claimed: bool,
}

#[derive(Debug, Deserialize)]
pub struct HeartbeatRequest {
    pub state: Option<String>,
    pub uptime: Option<i64>,
    #[serde(alias = "freeHeap")]
    pub free_heap: Option<i64>,
    pub ip: Option<String>,
    pub device_type: Option<String>,
    pub sensors: Option<Vec<serde_json::Value>>,
}

#[derive(Debug, Deserialize)]
pub struct CommandsQuery {
    pub wait: Option<u64>, // long-poll timeout in seconds (max 30)
}

#[derive(Debug, Deserialize)]
#[allow(dead_code)]
pub struct AckRequest {
    pub success: Option<bool>,
    pub result: Option<serde_json::Value>,
}

#[derive(Debug, Deserialize)]
pub struct ClaimRequest {
    pub claim_code: String,
    pub name: Option<String>,
}

// --- Helpers ---

/// Extract device_id from X-Device-Token header by verifying against device_tokens table.
fn authenticate_device(req: &Request, db: &DbPool) -> Result<String, Response> {
    let token = req
        .headers()
        .get("x-device-token")
        .and_then(|v| v.to_str().ok())
        .ok_or_else(|| {
            (
                StatusCode::UNAUTHORIZED,
                Json(json!({ "error": "Missing X-Device-Token header" })),
            )
                .into_response()
        })?;

    let token_hash = hash_token(token);
    let conn = db.lock().unwrap();
    let device_id: String = conn
        .query_row(
            "SELECT device_id FROM device_tokens WHERE token_hash = ?1",
            [&token_hash],
            |row| row.get(0),
        )
        .map_err(|_| {
            (
                StatusCode::UNAUTHORIZED,
                Json(json!({ "error": "Invalid device token" })),
            )
                .into_response()
        })?;

    Ok(device_id)
}

// --- Handlers ---

/// POST /api/device/register — ESP device self-registration on first boot.
pub async fn register(
    State(db): State<DbPool>,
    Json(input): Json<RegisterRequest>,
) -> Result<Json<RegisterResponse>, AppError> {
    let conn = db.lock().unwrap();

    // Check if device already exists by hostname
    let existing: Option<(String, Option<String>)> = conn
        .query_row(
            "SELECT d.id, dt.claim_code FROM devices d
             LEFT JOIN device_tokens dt ON dt.device_id = d.id
             WHERE d.hostname = ?1 AND d.connection_mode = 'cloud'",
            [&input.hostname],
            |row| Ok((row.get(0)?, row.get(1)?)),
        )
        .ok();

    if let Some((device_id, existing_claim_code)) = existing {
        // Device already registered — check if claimed
        let claimed = existing_claim_code.is_none();

        if claimed {
            // Already claimed, generate new token for reconnection
            let new_token = uuid::Uuid::new_v4().to_string();
            let token_hash = hash_token(&new_token);
            let _ = conn.execute(
                "UPDATE device_tokens SET token_hash = ?1 WHERE device_id = ?2",
                rusqlite::params![token_hash, device_id],
            );

            return Ok(Json(RegisterResponse {
                device_id,
                device_token: new_token,
                claim_code: String::new(),
                claimed: true,
            }));
        } else {
            // Not yet claimed — refresh token, keep claim code
            let new_token = uuid::Uuid::new_v4().to_string();
            let token_hash = hash_token(&new_token);
            let claim_code = existing_claim_code.unwrap_or_else(generate_claim_code);

            let _ = conn.execute(
                "UPDATE device_tokens SET token_hash = ?1, claim_code = ?2 WHERE device_id = ?3",
                rusqlite::params![token_hash, claim_code, device_id],
            );

            return Ok(Json(RegisterResponse {
                device_id,
                device_token: new_token,
                claim_code,
                claimed: false,
            }));
        }
    }

    // New device — create it
    let device_id = uuid::Uuid::new_v4().to_string();
    let device_token = uuid::Uuid::new_v4().to_string();
    let token_hash = hash_token(&device_token);
    let claim_code = generate_claim_code();

    conn.execute(
        "INSERT INTO devices (id, name, hostname, ip_address, device_type, connection_mode, created_at, updated_at)
         VALUES (?1, ?2, ?3, '', ?4, 'cloud', datetime('now'), datetime('now'))",
        rusqlite::params![
            device_id,
            input.hostname,
            input.hostname,
            input.device_type.as_deref().unwrap_or(""),
        ],
    )
    .map_err(|e| AppError::Internal(format!("Failed to create device: {e}")))?;

    conn.execute(
        "INSERT INTO device_tokens (device_id, token_hash, claim_code, created_at)
         VALUES (?1, ?2, ?3, datetime('now'))",
        rusqlite::params![device_id, token_hash, claim_code],
    )
    .map_err(|e| AppError::Internal(format!("Failed to create device token: {e}")))?;

    info!(
        "New cloud device registered: {} (hostname: {}, claim: {})",
        device_id, input.hostname, claim_code
    );

    Ok(Json(RegisterResponse {
        device_id,
        device_token,
        claim_code,
        claimed: false,
    }))
}

/// POST /api/device/heartbeat — Device pushes its status to the server.
pub async fn heartbeat(
    State(db): State<DbPool>,
    req: Request,
) -> Response {
    let device_id = match authenticate_device(&req, &db) {
        Ok(id) => id,
        Err(resp) => return resp,
    };

    // Parse body
    let body_bytes = match axum::body::to_bytes(req.into_body(), 1024 * 64).await {
        Ok(b) => b,
        Err(_) => {
            return (
                StatusCode::BAD_REQUEST,
                Json(json!({ "error": "Invalid request body" })),
            )
                .into_response();
        }
    };

    let input: HeartbeatRequest = match serde_json::from_slice(&body_bytes) {
        Ok(b) => b,
        Err(_) => {
            return (
                StatusCode::BAD_REQUEST,
                Json(json!({ "error": "Invalid JSON" })),
            )
                .into_response();
        }
    };

    let conn = db.lock().unwrap();

    // Record status log (same as device_poller does)
    let state = input.state.as_deref().unwrap_or("unknown");
    let uptime = input.uptime.unwrap_or(0);
    let free_heap = input.free_heap.unwrap_or(0);

    let _ = conn.execute(
        "INSERT INTO status_log (device_id, state, uptime_ms, free_heap) VALUES (?1, ?2, ?3, ?4)",
        rusqlite::params![device_id, state, uptime, free_heap],
    );

    // Update device_type if reported
    if let Some(ref dt) = input.device_type {
        let _ = conn.execute(
            "UPDATE devices SET device_type = ?1, updated_at = datetime('now') WHERE id = ?2 AND (device_type IS NULL OR device_type != ?1)",
            rusqlite::params![dt, device_id],
        );
    }

    // Update IP if reported
    if let Some(ref ip) = input.ip {
        let _ = conn.execute(
            "UPDATE devices SET ip_address = ?1, updated_at = datetime('now') WHERE id = ?2 AND ip_address != ?1",
            rusqlite::params![ip, device_id],
        );
    }

    // Record sensor readings
    if let Some(ref sensors) = input.sensors {
        for sensor in sensors {
            let channel = sensor["channel"].as_i64();
            let value = sensor["value"].as_f64();
            if let (Some(ch), Some(val)) = (channel, value) {
                let _ = conn.execute(
                    "INSERT INTO sensor_readings (device_id, channel, value) VALUES (?1, ?2, ?3)",
                    rusqlite::params![device_id, ch, val],
                );
            }
        }
    }

    // Update last heartbeat
    let _ = conn.execute(
        "UPDATE device_tokens SET last_heartbeat_at = datetime('now') WHERE device_id = ?1",
        [&device_id],
    );

    // Check if device is claimed
    let claimed: bool = conn
        .query_row(
            "SELECT claim_code FROM device_tokens WHERE device_id = ?1",
            [&device_id],
            |row| row.get::<_, Option<String>>(0),
        )
        .ok()
        .flatten()
        .is_none();

    let claim_code: String = if !claimed {
        conn.query_row(
            "SELECT claim_code FROM device_tokens WHERE device_id = ?1",
            [&device_id],
            |row| row.get::<_, Option<String>>(0),
        )
        .ok()
        .flatten()
        .unwrap_or_default()
    } else {
        String::new()
    };

    (
        StatusCode::OK,
        Json(json!({
            "ok": true,
            "claimed": claimed,
            "claim_code": claim_code,
        })),
    )
        .into_response()
}

/// GET /api/device/commands — Device polls for pending commands (supports long-poll).
pub async fn get_commands(
    State((db, queue)): State<(DbPool, CommandQueue)>,
    Query(params): Query<CommandsQuery>,
    req: Request,
) -> Response {
    let device_id = match authenticate_device(&req, &db) {
        Ok(id) => id,
        Err(resp) => return resp,
    };

    // Check for pending commands immediately
    let commands = queue.get_pending(&db, &device_id);
    if !commands.is_empty() {
        return (StatusCode::OK, Json(json!({ "commands": commands }))).into_response();
    }

    // Long-poll: wait for notification or timeout
    let wait_secs = params.wait.unwrap_or(0).min(30);
    if wait_secs > 0 {
        let waiter = queue.get_waiter(&device_id).await;
        let timeout = tokio::time::Duration::from_secs(wait_secs);

        match tokio::time::timeout(timeout, waiter.notified()).await {
            Ok(_) => {
                // Notified — check for commands
                let commands = queue.get_pending(&db, &device_id);
                return (StatusCode::OK, Json(json!({ "commands": commands }))).into_response();
            }
            Err(_) => {
                // Timeout — return empty
                return (StatusCode::OK, Json(json!({ "commands": [] }))).into_response();
            }
        }
    }

    (StatusCode::OK, Json(json!({ "commands": [] }))).into_response()
}

/// POST /api/device/commands/:id/ack — Device acknowledges a command.
pub async fn ack_command(
    State((db, queue)): State<(DbPool, CommandQueue)>,
    Path(command_id): Path<String>,
    req: Request,
) -> Response {
    let _device_id = match authenticate_device(&req, &db) {
        Ok(id) => id,
        Err(resp) => return resp,
    };

    if queue.acknowledge(&db, &command_id) {
        (StatusCode::OK, Json(json!({ "ok": true }))).into_response()
    } else {
        (
            StatusCode::NOT_FOUND,
            Json(json!({ "error": "Command not found or already acknowledged" })),
        )
            .into_response()
    }
}

/// POST /api/devices/claim — User claims a device by its claim code (user-authenticated).
pub async fn claim_device(
    State(db): State<DbPool>,
    req: Request,
) -> Response {
    let user_id = match req.extensions().get::<UserId>().and_then(|u| u.0.clone()) {
        Some(uid) => uid,
        None => {
            return (
                StatusCode::UNAUTHORIZED,
                Json(json!({ "error": "Authentication required" })),
            )
                .into_response();
        }
    };

    // Parse body
    let body_bytes = match axum::body::to_bytes(req.into_body(), 1024 * 4).await {
        Ok(b) => b,
        Err(_) => {
            return (
                StatusCode::BAD_REQUEST,
                Json(json!({ "error": "Invalid request body" })),
            )
                .into_response();
        }
    };

    let input: ClaimRequest = match serde_json::from_slice(&body_bytes) {
        Ok(b) => b,
        Err(_) => {
            return (
                StatusCode::BAD_REQUEST,
                Json(json!({ "error": "Invalid JSON" })),
            )
                .into_response();
        }
    };

    let claim_code = input.claim_code.trim().to_uppercase();
    if claim_code.len() != 6 {
        return (
            StatusCode::BAD_REQUEST,
            Json(json!({ "error": "Claim code must be 6 characters" })),
        )
            .into_response();
    }

    let conn = db.lock().unwrap();

    // Find device by claim code
    let device_id: String = match conn.query_row(
        "SELECT device_id FROM device_tokens WHERE claim_code = ?1",
        [&claim_code],
        |row| row.get(0),
    ) {
        Ok(id) => id,
        Err(_) => {
            return (
                StatusCode::NOT_FOUND,
                Json(json!({ "error": "Invalid claim code" })),
            )
                .into_response();
        }
    };

    // Assign device to user
    let _ = conn.execute(
        "UPDATE devices SET user_id = ?1, updated_at = datetime('now') WHERE id = ?2",
        rusqlite::params![user_id, device_id],
    );

    // Update name if provided
    if let Some(ref name) = input.name {
        let _ = conn.execute(
            "UPDATE devices SET name = ?1 WHERE id = ?2",
            rusqlite::params![name, device_id],
        );
    }

    // Clear claim code (device is now claimed)
    let _ = conn.execute(
        "UPDATE device_tokens SET claim_code = NULL WHERE device_id = ?1",
        [&device_id],
    );

    info!("Device {} claimed by user {} with code {}", device_id, user_id, claim_code);

    // Get device info to return
    let device_name: String = conn
        .query_row("SELECT name FROM devices WHERE id = ?1", [&device_id], |row| {
            row.get(0)
        })
        .unwrap_or_default();

    (
        StatusCode::OK,
        Json(json!({
            "ok": true,
            "device_id": device_id,
            "name": device_name,
        })),
    )
        .into_response()
}
