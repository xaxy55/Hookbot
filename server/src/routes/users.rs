use axum::extract::{Path, State};
use axum::Json;
use serde_json::json;

use crate::db::DbPool;
use crate::error::AppError;
use crate::models::*;

fn query_user_with_devices(conn: &rusqlite::Connection, id: &str) -> Result<UserWithDevices, AppError> {
    let user = conn.query_row(
        "SELECT id, username, display_name, role, last_login_at, created_at FROM users WHERE id = ?1",
        [id],
        |row| Ok(LocalUser {
            id: row.get(0)?,
            username: row.get(1)?,
            display_name: row.get(2)?,
            role: row.get(3)?,
            last_login_at: row.get(4)?,
            created_at: row.get(5)?,
        }),
    ).map_err(|_| AppError::NotFound(format!("User '{id}' not found")))?;

    let mut stmt = conn.prepare(
        "SELECT device_id FROM user_device_assignments WHERE user_id = ?1"
    )?;
    let device_ids = stmt.query_map([id], |row| row.get::<_, String>(0))?
        .filter_map(|r| r.ok())
        .collect();

    Ok(UserWithDevices { user, device_ids })
}

/// GET /api/users — list all users
pub async fn list_users(
    State(db): State<DbPool>,
) -> Result<Json<Vec<UserWithDevices>>, AppError> {
    let conn = db.lock().unwrap();
    let mut stmt = conn.prepare(
        "SELECT id, username, display_name, role, last_login_at, created_at FROM users ORDER BY created_at"
    )?;
    let users: Vec<LocalUser> = stmt.query_map([], |row| {
        Ok(LocalUser {
            id: row.get(0)?,
            username: row.get(1)?,
            display_name: row.get(2)?,
            role: row.get(3)?,
            last_login_at: row.get(4)?,
            created_at: row.get(5)?,
        })
    })?.filter_map(|r| r.ok()).collect();

    let mut result = Vec::new();
    for user in users {
        let mut stmt = conn.prepare(
            "SELECT device_id FROM user_device_assignments WHERE user_id = ?1"
        )?;
        let device_ids = stmt.query_map([&user.id], |row| row.get::<_, String>(0))?
            .filter_map(|r| r.ok())
            .collect();
        result.push(UserWithDevices { user, device_ids });
    }

    Ok(Json(result))
}

/// POST /api/users — create a new user
pub async fn create_user(
    State(db): State<DbPool>,
    Json(input): Json<CreateUser>,
) -> Result<Json<UserWithDevices>, AppError> {
    let conn = db.lock().unwrap();
    let id = uuid::Uuid::new_v4().to_string();
    let role = input.role.unwrap_or_else(|| "user".to_string());

    if !["admin", "user", "viewer"].contains(&role.as_str()) {
        return Err(AppError::BadRequest("Role must be 'admin', 'user', or 'viewer'".into()));
    }

    let password_hash = bcrypt::hash(&input.password, bcrypt::DEFAULT_COST)
        .map_err(|e| AppError::Internal(format!("Failed to hash password: {e}")))?;

    let api_token = uuid::Uuid::new_v4().to_string();

    conn.execute(
        "INSERT INTO users (id, username, display_name, password_hash, role, api_token) \
         VALUES (?1, ?2, ?3, ?4, ?5, ?6)",
        rusqlite::params![id, input.username, input.display_name, password_hash, role, api_token],
    ).map_err(|e| {
        if e.to_string().contains("UNIQUE") {
            AppError::BadRequest(format!("Username '{}' already exists", input.username))
        } else {
            AppError::Db(e)
        }
    })?;

    let user = query_user_with_devices(&conn, &id)?;
    Ok(Json(user))
}

/// GET /api/users/:id — get a specific user
pub async fn get_user(
    State(db): State<DbPool>,
    Path(id): Path<String>,
) -> Result<Json<UserWithDevices>, AppError> {
    let conn = db.lock().unwrap();
    let user = query_user_with_devices(&conn, &id)?;
    Ok(Json(user))
}

/// PUT /api/users/:id — update a user
pub async fn update_user(
    State(db): State<DbPool>,
    Path(id): Path<String>,
    Json(input): Json<UpdateUser>,
) -> Result<Json<UserWithDevices>, AppError> {
    let conn = db.lock().unwrap();
    let _ = query_user_with_devices(&conn, &id)?;

    if let Some(ref name) = input.display_name {
        conn.execute("UPDATE users SET display_name = ?1 WHERE id = ?2", rusqlite::params![name, id])?;
    }
    if let Some(ref role) = input.role {
        if !["admin", "user", "viewer"].contains(&role.as_str()) {
            return Err(AppError::BadRequest("Role must be 'admin', 'user', or 'viewer'".into()));
        }
        conn.execute("UPDATE users SET role = ?1 WHERE id = ?2", rusqlite::params![role, id])?;
    }
    if let Some(ref password) = input.password {
        let hash = bcrypt::hash(password, bcrypt::DEFAULT_COST)
            .map_err(|e| AppError::Internal(format!("Failed to hash password: {e}")))?;
        conn.execute("UPDATE users SET password_hash = ?1 WHERE id = ?2", rusqlite::params![hash, id])?;
    }

    let user = query_user_with_devices(&conn, &id)?;
    Ok(Json(user))
}

/// DELETE /api/users/:id — delete a user
pub async fn delete_user(
    State(db): State<DbPool>,
    Path(id): Path<String>,
) -> Result<Json<serde_json::Value>, AppError> {
    let conn = db.lock().unwrap();
    let rows = conn.execute("DELETE FROM users WHERE id = ?1", [&id])?;
    if rows == 0 {
        return Err(AppError::NotFound(format!("User '{id}' not found")));
    }
    Ok(Json(json!({ "ok": true })))
}

/// POST /api/users/:id/devices — assign a device to a user
pub async fn assign_device(
    State(db): State<DbPool>,
    Path(id): Path<String>,
    Json(input): Json<AssignDevice>,
) -> Result<Json<UserWithDevices>, AppError> {
    let conn = db.lock().unwrap();
    let _ = query_user_with_devices(&conn, &id)?;

    // Verify device exists
    conn.query_row("SELECT id FROM devices WHERE id = ?1", [&input.device_id], |row| row.get::<_, String>(0))
        .map_err(|_| AppError::NotFound(format!("Device '{}' not found", input.device_id)))?;

    let perms = input.permissions.unwrap_or_else(|| "full".to_string());
    conn.execute(
        "INSERT OR REPLACE INTO user_device_assignments (user_id, device_id, permissions) VALUES (?1, ?2, ?3)",
        rusqlite::params![id, input.device_id, perms],
    )?;

    let user = query_user_with_devices(&conn, &id)?;
    Ok(Json(user))
}

/// DELETE /api/users/:id/devices/:device_id — unassign a device
pub async fn unassign_device(
    State(db): State<DbPool>,
    Path((id, device_id)): Path<(String, String)>,
) -> Result<Json<serde_json::Value>, AppError> {
    let conn = db.lock().unwrap();
    conn.execute(
        "DELETE FROM user_device_assignments WHERE user_id = ?1 AND device_id = ?2",
        rusqlite::params![id, device_id],
    )?;
    Ok(Json(json!({ "ok": true })))
}
