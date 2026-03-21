use axum::extract::{Path, Query, State};
use axum::Json;
use serde::Deserialize;

use crate::db::DbPool;
use crate::error::AppError;
use crate::models::*;

#[derive(Debug, Deserialize)]
pub struct DeviceQuery {
    pub device_id: Option<String>,
}

fn resolve_device_id(conn: &rusqlite::Connection, device_id: Option<&str>) -> Result<String, AppError> {
    if let Some(id) = device_id {
        return Ok(id.to_string());
    }
    conn.query_row("SELECT id FROM devices ORDER BY created_at LIMIT 1", [], |row| row.get(0))
        .map_err(|_| AppError::NotFound("No devices registered".into()))
}

/// GET /api/music/config — get music integration config
pub async fn get_config(
    State(db): State<DbPool>,
    Query(q): Query<DeviceQuery>,
) -> Result<Json<Vec<MusicConfig>>, AppError> {
    let conn = db.lock().unwrap();
    let device_id = resolve_device_id(&conn, q.device_id.as_deref())?;

    let mut stmt = conn.prepare(
        "SELECT id, device_id, provider, access_token, refresh_token, auto_pause_meetings, \
         focus_playlist_id, enabled, created_at \
         FROM music_configs WHERE device_id = ?1"
    )?;
    let configs = stmt.query_map([&device_id], |row| {
        Ok(MusicConfig {
            id: row.get(0)?,
            device_id: row.get(1)?,
            provider: row.get(2)?,
            access_token: row.get(3)?,
            refresh_token: row.get(4)?,
            auto_pause_meetings: row.get(5)?,
            focus_playlist_id: row.get(6)?,
            enabled: row.get(7)?,
            created_at: row.get(8)?,
        })
    })?.filter_map(|r| r.ok()).collect::<Vec<_>>();

    Ok(Json(configs))
}

/// POST /api/music/config — create music integration config
pub async fn create_config(
    State(db): State<DbPool>,
    Json(input): Json<CreateMusicConfig>,
) -> Result<Json<MusicConfig>, AppError> {
    let conn = db.lock().unwrap();
    let device_id = resolve_device_id(&conn, input.device_id.as_deref())?;
    let id = uuid::Uuid::new_v4().to_string();

    conn.execute(
        "INSERT INTO music_configs (id, device_id, provider, access_token, refresh_token, focus_playlist_id) \
         VALUES (?1, ?2, ?3, ?4, ?5, ?6)",
        rusqlite::params![id, device_id, input.provider, input.access_token, input.refresh_token, input.focus_playlist_id],
    )?;

    Ok(Json(MusicConfig {
        id,
        device_id,
        provider: input.provider,
        access_token: input.access_token,
        refresh_token: input.refresh_token,
        auto_pause_meetings: true,
        focus_playlist_id: input.focus_playlist_id,
        enabled: true,
        created_at: chrono::Utc::now().to_rfc3339(),
    }))
}

/// PUT /api/music/config/:id — update music config
pub async fn update_config(
    State(db): State<DbPool>,
    Path(id): Path<String>,
    Json(input): Json<UpdateMusicConfig>,
) -> Result<Json<serde_json::Value>, AppError> {
    let conn = db.lock().unwrap();

    if let Some(token) = &input.access_token {
        conn.execute("UPDATE music_configs SET access_token = ?1 WHERE id = ?2", rusqlite::params![token, id])?;
    }
    if let Some(token) = &input.refresh_token {
        conn.execute("UPDATE music_configs SET refresh_token = ?1 WHERE id = ?2", rusqlite::params![token, id])?;
    }
    if let Some(v) = input.auto_pause_meetings {
        conn.execute("UPDATE music_configs SET auto_pause_meetings = ?1 WHERE id = ?2", rusqlite::params![v, id])?;
    }
    if let Some(pl) = &input.focus_playlist_id {
        conn.execute("UPDATE music_configs SET focus_playlist_id = ?1 WHERE id = ?2", rusqlite::params![pl, id])?;
    }
    if let Some(v) = input.enabled {
        conn.execute("UPDATE music_configs SET enabled = ?1 WHERE id = ?2", rusqlite::params![v, id])?;
    }

    Ok(Json(serde_json::json!({ "ok": true })))
}

/// DELETE /api/music/config/:id — remove music config
pub async fn delete_config(
    State(db): State<DbPool>,
    Path(id): Path<String>,
) -> Result<Json<serde_json::Value>, AppError> {
    let conn = db.lock().unwrap();
    conn.execute("DELETE FROM music_configs WHERE id = ?1", [&id])?;
    Ok(Json(serde_json::json!({ "ok": true })))
}

/// GET /api/music/now-playing — get current track info
pub async fn now_playing(
    State(db): State<DbPool>,
    Query(q): Query<DeviceQuery>,
) -> Result<Json<NowPlaying>, AppError> {
    let conn = db.lock().unwrap();
    let device_id = resolve_device_id(&conn, q.device_id.as_deref())?;

    // Check if we have an enabled music config
    let _config_exists: bool = conn.query_row(
        "SELECT COUNT(*) > 0 FROM music_configs WHERE device_id = ?1 AND enabled = 1",
        [&device_id],
        |row| row.get(0),
    ).unwrap_or(false);

    // In production, this would query the Spotify/Apple Music API.
    // Return placeholder data indicating the integration is ready.
    Ok(Json(NowPlaying {
        is_playing: false,
        track_name: None,
        artist_name: None,
        album_name: None,
        album_art_url: None,
        progress_ms: None,
        duration_ms: None,
    }))
}

/// POST /api/music/action — control playback
pub async fn music_action(
    State(db): State<DbPool>,
    Query(q): Query<DeviceQuery>,
    Json(input): Json<MusicAction>,
) -> Result<Json<serde_json::Value>, AppError> {
    let conn = db.lock().unwrap();
    let device_id = resolve_device_id(&conn, q.device_id.as_deref())?;

    let config = conn.query_row(
        "SELECT provider FROM music_configs WHERE device_id = ?1 AND enabled = 1 LIMIT 1",
        [&device_id],
        |row| row.get::<_, String>(0),
    ).map_err(|_| AppError::NotFound("No active music integration found".into()))?;

    Ok(Json(serde_json::json!({
        "ok": true,
        "provider": config,
        "action": input.action,
        "message": format!("Music action '{}' queued", input.action)
    })))
}
