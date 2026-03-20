use axum::extract::{Query, State};
use axum::Json;
use serde::{Deserialize, Serialize};
use serde_json::json;
use crate::config::AppConfig;
use crate::db::DbPool;
use crate::error::AppError;
use crate::services::proxy;

// ── Types ────────────────────────────────────────────────────────

#[derive(Debug, Deserialize)]
pub struct VoiceCommandRequest {
    pub transcript: String,
    pub device_id: Option<String>,
    pub confidence: Option<f64>,
}

#[derive(Debug, Serialize)]
pub struct VoiceCommandResponse {
    pub ok: bool,
    pub action: String,
    pub detail: String,
    pub executed: bool,
}

#[derive(Debug, Deserialize)]
pub struct TtsRequest {
    pub text: String,
    pub device_id: Option<String>,
    pub voice: Option<String>,
}

#[derive(Debug, Serialize)]
pub struct TtsResponse {
    pub ok: bool,
    pub text: String,
    pub sent_to_device: bool,
}

#[derive(Debug, Deserialize)]
pub struct DeviceQuery {
    pub device_id: Option<String>,
}

#[derive(Debug, Serialize)]
pub struct VoiceStatus {
    pub enabled: bool,
    pub wake_word: String,
    pub last_command: Option<String>,
    pub last_command_at: Option<String>,
    pub total_commands: i64,
}

fn resolve_device_id(conn: &rusqlite::Connection, device_id: Option<&str>) -> Result<String, AppError> {
    if let Some(id) = device_id {
        return Ok(id.to_string());
    }
    conn.query_row("SELECT id FROM devices ORDER BY created_at LIMIT 1", [], |row| row.get(0))
        .map_err(|_| AppError::NotFound("No devices registered".into()))
}

// ── Voice Command Handlers ──────────────────────────────────────

/// POST /api/voice/command — process a voice command transcript
pub async fn process_command(
    State((db, _config)): State<(DbPool, AppConfig)>,
    Json(input): Json<VoiceCommandRequest>,
) -> Result<Json<VoiceCommandResponse>, AppError> {
    let (device_ip, transcript_text) = {
        let conn = db.lock().unwrap();
        let device_id = resolve_device_id(&conn, input.device_id.as_deref())?;

        conn.execute(
            "INSERT INTO tool_uses (device_id, tool_name, event, project, xp_earned) \
             VALUES (?1, 'voice_command', ?2, NULL, 5)",
            rusqlite::params![device_id, input.transcript],
        )?;

        let device_ip: Option<String> = conn.query_row(
            "SELECT ip_address FROM devices WHERE id = ?1",
            [&device_id],
            |row| row.get(0),
        ).ok();

        (device_ip, input.transcript.to_lowercase())
    };

    let (action, detail, state_change) = parse_voice_command(&transcript_text);

    let mut executed = false;
    if let (Some(ref ip), Some(ref new_state)) = (&device_ip, &state_change) {
        let body = json!({ "state": new_state });
        if proxy::forward_json(&format!("http://{}/state", ip), &body).await.is_ok() {
            executed = true;
        }
    }

    Ok(Json(VoiceCommandResponse {
        ok: true,
        action,
        detail,
        executed,
    }))
}

/// Parse a voice transcript into an action
fn parse_voice_command(transcript: &str) -> (String, String, Option<String>) {
    if transcript.contains("idle") || transcript.contains("relax") || transcript.contains("rest") {
        return ("set_state".into(), "Setting state to idle".into(), Some("idle".into()));
    }
    if transcript.contains("think") || transcript.contains("working") {
        return ("set_state".into(), "Setting state to thinking".into(), Some("thinking".into()));
    }
    if transcript.contains("error") || transcript.contains("fail") || transcript.contains("broke") {
        return ("set_state".into(), "Setting state to error".into(), Some("error".into()));
    }
    if transcript.contains("success") || transcript.contains("done") || transcript.contains("complete") || transcript.contains("finish") {
        return ("set_state".into(), "Setting state to success".into(), Some("success".into()));
    }
    if transcript.contains("wait") || transcript.contains("hold") || transcript.contains("pause") {
        return ("set_state".into(), "Setting state to waiting".into(), Some("waiting".into()));
    }
    if transcript.contains("check") || transcript.contains("task") || transcript.contains("review") {
        return ("set_state".into(), "Setting state to taskcheck".into(), Some("taskcheck".into()));
    }

    if transcript.contains("status") || transcript.contains("how are you") {
        return ("query_status".into(), "Checking device status".into(), None);
    }
    if transcript.contains("level") || transcript.contains("xp") || transcript.contains("experience") {
        return ("query_xp".into(), "Checking XP and level".into(), None);
    }
    if transcript.contains("streak") {
        return ("query_streak".into(), "Checking current streak".into(), None);
    }

    ("unknown".into(), format!("Unrecognized command: {transcript}"), None)
}

/// GET /api/voice/status — get voice control status for a device
pub async fn get_voice_status(
    State((db, _config)): State<(DbPool, AppConfig)>,
    Query(q): Query<DeviceQuery>,
) -> Result<Json<VoiceStatus>, AppError> {
    let conn = db.lock().unwrap();
    let device_id = resolve_device_id(&conn, q.device_id.as_deref())?;

    let total_commands: i64 = conn.query_row(
        "SELECT COUNT(*) FROM tool_uses WHERE device_id = ?1 AND tool_name = 'voice_command'",
        [&device_id],
        |row| row.get(0),
    ).unwrap_or(0);

    let last = conn.query_row(
        "SELECT event, created_at FROM tool_uses WHERE device_id = ?1 AND tool_name = 'voice_command' \
         ORDER BY created_at DESC LIMIT 1",
        [&device_id],
        |row| Ok((row.get::<_, String>(0)?, row.get::<_, String>(1)?)),
    ).ok();

    let (last_command, last_command_at) = match last {
        Some((cmd, at)) => (Some(cmd), Some(at)),
        None => (None, None),
    };

    let enabled = conn.query_row(
        "SELECT COALESCE(json_extract(custom_data, '$.voice_enabled'), 0) FROM device_config WHERE device_id = ?1",
        [&device_id],
        |row| row.get::<_, i64>(0),
    ).unwrap_or(0) != 0;

    Ok(Json(VoiceStatus {
        enabled,
        wake_word: "hey hookbot".into(),
        last_command,
        last_command_at,
        total_commands,
    }))
}

// ── Text-to-Speech Handlers ─────────────────────────────────────

/// POST /api/voice/speak — send text to device for TTS playback
pub async fn speak(
    State((db, _config)): State<(DbPool, AppConfig)>,
    Json(input): Json<TtsRequest>,
) -> Result<Json<TtsResponse>, AppError> {
    let device_ip = {
        let conn = db.lock().unwrap();
        let device_id = resolve_device_id(&conn, input.device_id.as_deref())?;
        conn.query_row(
            "SELECT ip_address FROM devices WHERE id = ?1",
            [&device_id],
            |row| row.get::<_, String>(0),
        ).ok()
    };

    let mut sent = false;
    if let Some(ref ip) = device_ip {
        let body = json!({
            "text": input.text,
            "voice": input.voice.as_deref().unwrap_or("default"),
        });
        if proxy::forward_json(&format!("http://{}/tts", ip), &body).await.is_ok() {
            sent = true;
        }

        let oled_body = json!({
            "source": "tts",
            "message": input.text,
            "unread": 0,
        });
        let _ = proxy::forward_json(&format!("http://{}/notification", ip), &oled_body).await;
    }

    Ok(Json(TtsResponse {
        ok: true,
        text: input.text,
        sent_to_device: sent,
    }))
}

/// POST /api/voice/announce — generate a context-aware announcement
pub async fn announce(
    State((db, _config)): State<(DbPool, AppConfig)>,
    Json(input): Json<AnnounceRequest>,
) -> Result<Json<TtsResponse>, AppError> {
    let (device_ip, recent_tools) = {
        let conn = db.lock().unwrap();
        let device_id = resolve_device_id(&conn, input.device_id.as_deref())?;

        let mut stmt = conn.prepare(
            "SELECT tool_name FROM tool_uses WHERE device_id = ?1 ORDER BY created_at DESC LIMIT 10"
        )?;
        let tools: Vec<String> = stmt.query_map([&device_id], |row| row.get::<_, String>(0))?
            .filter_map(|r| r.ok())
            .collect();

        let ip: Option<String> = conn.query_row(
            "SELECT ip_address FROM devices WHERE id = ?1",
            [&device_id],
            |row| row.get(0),
        ).ok();

        (ip, tools)
    };

    let text = match input.announcement_type.as_deref() {
        Some("status") => {
            let context = if recent_tools.is_empty() { "idle" } else { "active" };
            format!("Current status: {}. Recent tools: {}", context, recent_tools.join(", "))
        }
        Some("greeting") => {
            use chrono::Timelike;
            let hour = chrono::Local::now().hour();
            let greeting = match hour {
                0..=11 => "Good morning",
                12..=17 => "Good afternoon",
                _ => "Good evening",
            };
            format!("{}, developer! Ready to code.", greeting)
        }
        Some("summary") => {
            format!("Session summary: {} tool uses recorded.", recent_tools.len())
        }
        _ => input.text.clone().unwrap_or_else(|| "Hello from Hookbot!".into()),
    };

    let mut sent = false;
    if let Some(ref ip) = device_ip {
        let body = json!({
            "text": text,
            "voice": "default",
        });
        if proxy::forward_json(&format!("http://{}/tts", ip), &body).await.is_ok() {
            sent = true;
        }
    }

    Ok(Json(TtsResponse {
        ok: true,
        text,
        sent_to_device: sent,
    }))
}

#[derive(Debug, Deserialize)]
pub struct AnnounceRequest {
    pub device_id: Option<String>,
    pub announcement_type: Option<String>,
    pub text: Option<String>,
}
