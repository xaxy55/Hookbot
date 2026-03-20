use axum::extract::{Query, State};
use axum::Json;
use axum::body::Bytes;
use axum::http::HeaderMap;
use serde::{Deserialize, Serialize};

use crate::config::AppConfig;
use crate::db::DbPool;
use crate::error::AppError;
use crate::models::*;

type VoiceState = (DbPool, AppConfig);

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

// ─── WAV encoding ───────────────────────────────────────────────

/// Wrap raw PCM data in a WAV header (needed for Whisper API)
fn pcm_to_wav(pcm: &[u8], sample_rate: u32, bits_per_sample: u16, channels: u16) -> Vec<u8> {
    let data_size = pcm.len() as u32;
    let byte_rate = sample_rate * channels as u32 * bits_per_sample as u32 / 8;
    let block_align = channels * bits_per_sample / 8;
    let file_size = 36 + data_size;

    let mut wav = Vec::with_capacity(44 + pcm.len());
    wav.extend_from_slice(b"RIFF");
    wav.extend_from_slice(&file_size.to_le_bytes());
    wav.extend_from_slice(b"WAVE");
    wav.extend_from_slice(b"fmt ");
    wav.extend_from_slice(&16u32.to_le_bytes()); // chunk size
    wav.extend_from_slice(&1u16.to_le_bytes());  // PCM format
    wav.extend_from_slice(&channels.to_le_bytes());
    wav.extend_from_slice(&sample_rate.to_le_bytes());
    wav.extend_from_slice(&byte_rate.to_le_bytes());
    wav.extend_from_slice(&block_align.to_le_bytes());
    wav.extend_from_slice(&bits_per_sample.to_le_bytes());
    wav.extend_from_slice(b"data");
    wav.extend_from_slice(&data_size.to_le_bytes());
    wav.extend_from_slice(pcm);
    wav
}

// ─── OpenAI Whisper STT ─────────────────────────────────────────

async fn whisper_transcribe(audio_wav: Vec<u8>, api_key: &str, language: &str) -> Result<String, String> {
    let client = reqwest::Client::new();

    let file_part = reqwest::multipart::Part::bytes(audio_wav)
        .file_name("audio.wav")
        .mime_str("audio/wav")
        .map_err(|e| format!("Failed to create multipart: {e}"))?;

    let form = reqwest::multipart::Form::new()
        .text("model", "whisper-1")
        .text("language", language.to_string())
        .text("response_format", "json")
        .part("file", file_part);

    let resp = client
        .post("https://api.openai.com/v1/audio/transcriptions")
        .header("Authorization", format!("Bearer {api_key}"))
        .multipart(form)
        .send()
        .await
        .map_err(|e| format!("Whisper request failed: {e}"))?;

    if !resp.status().is_success() {
        let status = resp.status();
        let body = resp.text().await.unwrap_or_default();
        return Err(format!("Whisper API error {status}: {body}"));
    }

    #[derive(Deserialize)]
    struct WhisperResponse {
        text: String,
    }

    let result: WhisperResponse = resp.json().await
        .map_err(|e| format!("Failed to parse Whisper response: {e}"))?;

    Ok(result.text)
}

// ─── Claude Command Interpreter ─────────────────────────────────

#[derive(Debug, Serialize, Deserialize)]
struct ClaudeCommand {
    action: String,          // "state_change", "query", "animation", "unknown"
    state: Option<String>,   // for state_change: "idle", "thinking", etc.
    response: String,        // spoken response text
}

async fn claude_interpret_command(transcript: &str, api_key: &str) -> Result<ClaudeCommand, String> {
    let client = reqwest::Client::new();

    let body = serde_json::json!({
        "model": "claude-sonnet-4-20250514",
        "max_tokens": 200,
        "system": "You are a voice assistant for a desk robot called Hookbot. Parse voice commands and return JSON.\n\nAvailable states: idle, thinking, waiting, success, taskcheck, error\n\nRespond with ONLY valid JSON in this format:\n{\"action\": \"state_change\"|\"query\"|\"unknown\", \"state\": \"idle\"|\"thinking\"|\"waiting\"|\"success\"|\"taskcheck\"|\"error\"|null, \"response\": \"short spoken response\"}\n\nExamples:\n- \"go to sleep\" → {\"action\":\"state_change\",\"state\":\"idle\",\"response\":\"Going idle. Rest well.\"}\n- \"I'm working now\" → {\"action\":\"state_change\",\"state\":\"thinking\",\"response\":\"Focus mode activated.\"}\n- \"how's it going\" → {\"action\":\"query\",\"state\":null,\"response\":\"All systems nominal. Ready to assist.\"}\n- \"what time is it\" → {\"action\":\"query\",\"state\":null,\"response\":\"Check your clock, boss.\"}\n\nKeep responses short (under 15 words), personality: slightly sarcastic desk companion.",
        "messages": [
            {"role": "user", "content": transcript}
        ]
    });

    let resp = client
        .post("https://api.anthropic.com/v1/messages")
        .header("x-api-key", api_key)
        .header("anthropic-version", "2023-06-01")
        .header("content-type", "application/json")
        .json(&body)
        .send()
        .await
        .map_err(|e| format!("Claude request failed: {e}"))?;

    if !resp.status().is_success() {
        let status = resp.status();
        let body = resp.text().await.unwrap_or_default();
        return Err(format!("Claude API error {status}: {body}"));
    }

    #[derive(Deserialize)]
    struct ClaudeContent {
        text: Option<String>,
    }
    #[derive(Deserialize)]
    struct ClaudeResponse {
        content: Vec<ClaudeContent>,
    }

    let result: ClaudeResponse = resp.json().await
        .map_err(|e| format!("Failed to parse Claude response: {e}"))?;

    let text = result.content.first()
        .and_then(|c| c.text.as_ref())
        .ok_or("Empty Claude response")?;

    // Extract JSON from response (Claude may wrap it in markdown)
    let json_str = if let Some(start) = text.find('{') {
        if let Some(end) = text.rfind('}') {
            &text[start..=end]
        } else {
            text.as_str()
        }
    } else {
        text.as_str()
    };

    serde_json::from_str::<ClaudeCommand>(json_str)
        .map_err(|e| format!("Failed to parse Claude JSON '{json_str}': {e}"))
}

// ─── OpenAI TTS ─────────────────────────────────────────────────

/// Generate speech audio from text using OpenAI TTS API
/// Returns raw PCM 16-bit 16kHz mono audio data
async fn openai_tts(text: &str, api_key: &str, voice: &str) -> Result<Vec<u8>, String> {
    let client = reqwest::Client::new();

    let voice_name = match voice {
        "alloy" | "echo" | "fable" | "onyx" | "nova" | "shimmer" | "ash" | "coral" | "sage" => voice,
        _ => "onyx", // default: deep authoritative voice for the CEO
    };

    let body = serde_json::json!({
        "model": "tts-1",
        "input": text,
        "voice": voice_name,
        "response_format": "pcm",  // raw PCM 24kHz 16-bit mono
        "speed": 1.0
    });

    let resp = client
        .post("https://api.openai.com/v1/audio/speech")
        .header("Authorization", format!("Bearer {api_key}"))
        .header("Content-Type", "application/json")
        .json(&body)
        .send()
        .await
        .map_err(|e| format!("TTS request failed: {e}"))?;

    if !resp.status().is_success() {
        let status = resp.status();
        let body = resp.text().await.unwrap_or_default();
        return Err(format!("TTS API error {status}: {body}"));
    }

    let audio_bytes = resp.bytes().await
        .map_err(|e| format!("Failed to read TTS audio: {e}"))?;

    // OpenAI TTS pcm format returns 24kHz 16-bit mono
    // Downsample to 16kHz for the ESP32 I2S driver
    let samples_24k: Vec<i16> = audio_bytes
        .chunks_exact(2)
        .map(|c| i16::from_le_bytes([c[0], c[1]]))
        .collect();

    // Simple downsample 24kHz → 16kHz (take 2 out of every 3 samples)
    let mut samples_16k = Vec::with_capacity(samples_24k.len() * 2 / 3);
    for i in (0..samples_24k.len()).step_by(3) {
        samples_16k.push(samples_24k[i]);
        if i + 1 < samples_24k.len() {
            samples_16k.push(samples_24k[i + 1]);
        }
    }

    let mut pcm_16k = Vec::with_capacity(samples_16k.len() * 2);
    for s in &samples_16k {
        pcm_16k.extend_from_slice(&s.to_le_bytes());
    }

    Ok(pcm_16k)
}

// ─── Fallback command parser (no API keys needed) ───────────────

fn parse_voice_command_local(text: &str) -> ClaudeCommand {
    let lower = text.to_lowercase();

    if lower.contains("idle") || lower.contains("relax") || lower.contains("rest") || lower.contains("sleep") {
        return ClaudeCommand { action: "state_change".into(), state: Some("idle".into()), response: "Going idle.".into() };
    }
    if lower.contains("think") || lower.contains("focus") || lower.contains("work") || lower.contains("code") {
        return ClaudeCommand { action: "state_change".into(), state: Some("thinking".into()), response: "Focus mode.".into() };
    }
    if lower.contains("wait") || lower.contains("pause") || lower.contains("hold") {
        return ClaudeCommand { action: "state_change".into(), state: Some("waiting".into()), response: "Waiting.".into() };
    }
    if lower.contains("success") || lower.contains("done") || lower.contains("finish") || lower.contains("complete") || lower.contains("ship") {
        return ClaudeCommand { action: "state_change".into(), state: Some("success".into()), response: "Victory!".into() };
    }
    if lower.contains("check") || lower.contains("review") || lower.contains("approve") || lower.contains("task") {
        return ClaudeCommand { action: "state_change".into(), state: Some("taskcheck".into()), response: "Checking.".into() };
    }
    if lower.contains("error") || lower.contains("fail") || lower.contains("bug") || lower.contains("angry") || lower.contains("destroy") {
        return ClaudeCommand { action: "state_change".into(), state: Some("error".into()), response: "RAGE MODE.".into() };
    }
    if lower.contains("status") || lower.contains("how are you") || lower.contains("hello") || lower.contains("hi") {
        return ClaudeCommand { action: "query".into(), state: None, response: "All systems operational.".into() };
    }
    if lower.contains("time") || lower.contains("clock") {
        let now = chrono::Local::now();
        return ClaudeCommand { action: "query".into(), state: None, response: format!("It's {}.", now.format("%H:%M")) };
    }

    ClaudeCommand { action: "unknown".into(), state: None, response: "Didn't catch that.".into() }
}

// ─── Forward state to device ────────────────────────────────────

async fn forward_state_to_device(ip: &str, state: &str) {
    let client = reqwest::Client::builder()
        .timeout(std::time::Duration::from_secs(3))
        .build()
        .unwrap_or_default();
    let url = format!("http://{}:80/state", ip);
    let _ = client.post(&url)
        .json(&serde_json::json!({ "state": state }))
        .send()
        .await;
}

/// Send TTS audio to device for playback
async fn send_tts_to_device(ip: &str, audio_data: &[u8]) {
    let client = reqwest::Client::builder()
        .timeout(std::time::Duration::from_secs(10))
        .build()
        .unwrap_or_default();
    let url = format!("http://{}:80/audio/play", ip);
    let _ = client.post(&url)
        .header("Content-Type", "application/octet-stream")
        .body(audio_data.to_vec())
        .send()
        .await;
}

// ─── Route handlers ─────────────────────────────────────────────

/// POST /api/voice/transcribe — receive raw PCM from device, full STT→Claude→TTS pipeline
pub async fn transcribe(
    State((db, config)): State<VoiceState>,
    headers: HeaderMap,
    body: Bytes,
) -> Result<Json<VoiceResponse>, AppError> {
    let device_hostname = headers
        .get("x-device-id")
        .and_then(|v| v.to_str().ok())
        .unwrap_or("unknown")
        .to_string();

    let sample_rate: u32 = headers
        .get("x-audio-rate")
        .and_then(|v| v.to_str().ok())
        .and_then(|v| v.parse().ok())
        .unwrap_or(16000);

    let audio_size = body.len();
    let duration_secs = audio_size as f64 / (sample_rate as f64 * 2.0);

    tracing::info!(
        "[Voice] Received {audio_size} bytes from {device_hostname} ({duration_secs:.1}s at {sample_rate}Hz)"
    );

    // Resolve device
    let (device_id, device_ip, voice_cfg) = {
        let conn = db.lock().unwrap();
        let did = conn.query_row(
            "SELECT id FROM devices WHERE hostname = ?1 OR name = ?1 OR id = ?1",
            [&device_hostname],
            |row| row.get::<_, String>(0),
        ).unwrap_or_else(|_| device_hostname.clone());

        let dip = conn.query_row(
            "SELECT ip_address FROM devices WHERE id = ?1",
            [&did],
            |row| row.get::<_, String>(0),
        ).ok();

        let vcfg = conn.query_row(
            "SELECT tts_enabled, tts_voice, language FROM voice_config WHERE device_id = ?1",
            [&did],
            |row| Ok((row.get::<_, bool>(0)?, row.get::<_, String>(1)?, row.get::<_, String>(2)?)),
        ).unwrap_or((true, "onyx".to_string(), "en".to_string()));

        (did, dip, vcfg)
    };

    let (tts_enabled, tts_voice, language) = voice_cfg;

    // Store initial record
    let cmd_id = {
        let conn = db.lock().unwrap();
        conn.execute(
            "INSERT INTO voice_commands (device_id, audio_size, duration_secs, transcript, response, status)
             VALUES (?1, ?2, ?3, '', '', 'received')",
            rusqlite::params![device_id, audio_size as i64, duration_secs],
        )?;
        conn.last_insert_rowid()
    };

    // Step 1: Speech-to-Text via Whisper
    let transcript = if let Some(ref oai_key) = config.openai_api_key {
        let wav = pcm_to_wav(&body, sample_rate, 16, 1);
        match whisper_transcribe(wav, oai_key, &language).await {
            Ok(text) => {
                tracing::info!("[Voice] Whisper transcript: {text}");
                text
            }
            Err(e) => {
                tracing::warn!("[Voice] Whisper failed: {e}, using empty transcript");
                String::new()
            }
        }
    } else {
        tracing::warn!("[Voice] No OPENAI_API_KEY — STT disabled");
        String::new()
    };

    if transcript.is_empty() {
        let conn = db.lock().unwrap();
        conn.execute(
            "UPDATE voice_commands SET status = 'stt_failed' WHERE id = ?1",
            [cmd_id],
        ).ok();
        return Ok(Json(VoiceResponse {
            ok: false,
            transcript: String::new(),
            response: "Could not transcribe audio. Check OPENAI_API_KEY.".into(),
            state: None,
            tts_url: None,
        }));
    }

    // Step 2: Interpret command via Claude (or local fallback)
    let command = if let Some(ref anthropic_key) = config.anthropic_api_key {
        match claude_interpret_command(&transcript, anthropic_key).await {
            Ok(cmd) => {
                tracing::info!("[Voice] Claude: action={}, state={:?}, response={}", cmd.action, cmd.state, cmd.response);
                cmd
            }
            Err(e) => {
                tracing::warn!("[Voice] Claude failed: {e}, using local parser");
                parse_voice_command_local(&transcript)
            }
        }
    } else {
        tracing::info!("[Voice] No ANTHROPIC_API_KEY — using local parser");
        parse_voice_command_local(&transcript)
    };

    // Update DB record
    {
        let conn = db.lock().unwrap();
        conn.execute(
            "UPDATE voice_commands SET transcript = ?1, response = ?2, status = 'processed' WHERE id = ?3",
            rusqlite::params![transcript, command.response, cmd_id],
        ).ok();
    }

    // Step 3: Forward state change to device
    if let Some(ref state_str) = command.state {
        if let Some(ref ip) = device_ip {
            forward_state_to_device(ip, state_str).await;
        }
    }

    // Step 4: TTS — speak the response back through the device
    if tts_enabled && !command.response.is_empty() {
        if let Some(ref oai_key) = config.openai_api_key {
            match openai_tts(&command.response, oai_key, &tts_voice).await {
                Ok(audio) => {
                    tracing::info!("[Voice] TTS generated {} bytes", audio.len());
                    if let Some(ref ip) = device_ip {
                        send_tts_to_device(ip, &audio).await;
                    }
                }
                Err(e) => {
                    tracing::warn!("[Voice] TTS failed: {e}");
                }
            }
        }
    }

    Ok(Json(VoiceResponse {
        ok: true,
        transcript,
        response: command.response,
        state: command.state,
        tts_url: None,
    }))
}

/// POST /api/voice/tts — convert text to speech and send to device
pub async fn text_to_speech(
    State((db, config)): State<VoiceState>,
    Json(input): Json<TtsRequest>,
) -> Result<Json<TtsResponse>, AppError> {
    let (device_id, device_ip) = {
        let conn = db.lock().unwrap();
        let did = resolve_device_id(&conn, input.device_id.as_deref())?;
        let dip = conn.query_row(
            "SELECT ip_address FROM devices WHERE id = ?1",
            [&did],
            |row| row.get::<_, String>(0),
        ).ok();
        (did, dip)
    };

    tracing::info!("[Voice] TTS for device {device_id}: {:?}", input.text);

    let oai_key = config.openai_api_key.as_ref()
        .ok_or_else(|| AppError::Internal("OPENAI_API_KEY not configured".into()))?;

    let voice = input.voice.as_deref().unwrap_or("onyx");
    let audio = openai_tts(&input.text, oai_key, voice).await
        .map_err(|e| AppError::Internal(format!("TTS failed: {e}")))?;

    let audio_size = audio.len();
    let duration = audio_size as f64 / (16000.0 * 2.0);

    // Store in history
    {
        let conn = db.lock().unwrap();
        conn.execute(
            "INSERT INTO voice_commands (device_id, audio_size, duration_secs, transcript, response, status)
             VALUES (?1, ?2, ?3, '', ?4, 'tts_sent')",
            rusqlite::params![device_id, audio_size as i64, duration, input.text],
        )?;
    }

    // Send audio to device
    if let Some(ref ip) = device_ip {
        send_tts_to_device(ip, &audio).await;
    }

    Ok(Json(TtsResponse {
        ok: true,
        text: input.text,
        audio_url: None,
        duration_secs: Some(duration),
        format: "pcm_16bit_16khz".to_string(),
    }))
}

/// POST /api/voice/command — text command (browser speech or typed)
pub async fn voice_command(
    State((db, config)): State<VoiceState>,
    Json(input): Json<VoiceCommandRequest>,
) -> Result<Json<VoiceResponse>, AppError> {
    // Interpret via Claude or fallback
    let command = if let Some(ref anthropic_key) = config.anthropic_api_key {
        claude_interpret_command(&input.text, anthropic_key).await
            .unwrap_or_else(|_| parse_voice_command_local(&input.text))
    } else {
        parse_voice_command_local(&input.text)
    };

    let ip = {
        let conn = db.lock().unwrap();
        let device_id = resolve_device_id(&conn, input.device_id.as_deref())?;

        conn.execute(
            "INSERT INTO voice_commands (device_id, audio_size, duration_secs, transcript, response, status)
             VALUES (?1, 0, 0, ?2, ?3, 'processed')",
            rusqlite::params![device_id, input.text, command.response],
        )?;

        if command.state.is_some() {
            conn.query_row(
                "SELECT ip_address FROM devices WHERE id = ?1",
                [&device_id],
                |row| row.get::<_, String>(0),
            ).ok()
        } else {
            None
        }
    };

    // Forward state to device
    if let Some(ref state_str) = command.state {
        if let Some(ref ip) = ip {
            forward_state_to_device(ip, state_str).await;
        }
    }

    // TTS for typed commands too (if device has speaker)
    if !command.response.is_empty() {
        if let (Some(ref oai_key), Some(ref ip)) = (&config.openai_api_key, &ip.clone().or_else(|| {
            let conn = db.lock().unwrap();
            let did = resolve_device_id(&conn, input.device_id.as_deref()).ok()?;
            conn.query_row("SELECT ip_address FROM devices WHERE id = ?1", [&did], |row| row.get::<_, String>(0)).ok()
        })) {
            if let Ok(audio) = openai_tts(&command.response, oai_key, "onyx").await {
                send_tts_to_device(ip, &audio).await;
            }
        }
    }

    Ok(Json(VoiceResponse {
        ok: true,
        transcript: input.text,
        response: command.response,
        state: command.state,
        tts_url: None,
    }))
}

/// GET /api/voice/history — recent voice command history
pub async fn get_history(
    State((db, _config)): State<VoiceState>,
    Query(q): Query<VoiceHistoryQuery>,
) -> Result<Json<Vec<VoiceCommand>>, AppError> {
    let conn = db.lock().unwrap();
    let device_id = resolve_device_id(&conn, q.device_id.as_deref())?;
    let limit = q.limit.unwrap_or(20);

    let mut stmt = conn.prepare(
        "SELECT id, device_id, audio_size, duration_secs, transcript, response, status, created_at
         FROM voice_commands WHERE device_id = ?1
         ORDER BY created_at DESC LIMIT ?2"
    )?;

    let commands = stmt.query_map(rusqlite::params![device_id, limit], |row| {
        Ok(VoiceCommand {
            id: row.get(0)?,
            device_id: row.get(1)?,
            audio_size: row.get(2)?,
            duration_secs: row.get(3)?,
            transcript: row.get(4)?,
            response: row.get(5)?,
            status: row.get(6)?,
            created_at: row.get(7)?,
        })
    })?.filter_map(|r| r.ok()).collect::<Vec<_>>();

    Ok(Json(commands))
}

/// GET /api/voice/config — get voice settings
pub async fn get_config(
    State((db, _config)): State<VoiceState>,
    Query(q): Query<DeviceQuery>,
) -> Result<Json<VoiceConfig>, AppError> {
    let conn = db.lock().unwrap();
    let device_id = resolve_device_id(&conn, q.device_id.as_deref())?;

    let config = conn.query_row(
        "SELECT wake_word_enabled, tts_enabled, tts_voice, volume, language
         FROM voice_config WHERE device_id = ?1",
        [&device_id],
        |row| Ok(VoiceConfig {
            device_id: device_id.clone(),
            wake_word_enabled: row.get(0)?,
            tts_enabled: row.get(1)?,
            tts_voice: row.get(2)?,
            volume: row.get(3)?,
            language: row.get(4)?,
        }),
    ).unwrap_or(VoiceConfig {
        device_id: device_id.clone(),
        wake_word_enabled: true,
        tts_enabled: true,
        tts_voice: "onyx".to_string(),
        volume: 80,
        language: "en".to_string(),
    });

    Ok(Json(config))
}

/// PUT /api/voice/config — update voice settings
pub async fn update_config(
    State((db, _config)): State<VoiceState>,
    Json(input): Json<UpdateVoiceConfig>,
) -> Result<Json<serde_json::Value>, AppError> {
    let conn = db.lock().unwrap();
    let device_id = resolve_device_id(&conn, input.device_id.as_deref())?;

    conn.execute(
        "INSERT INTO voice_config (device_id, wake_word_enabled, tts_enabled, tts_voice, volume, language)
         VALUES (?1, ?2, ?3, ?4, ?5, ?6)
         ON CONFLICT(device_id) DO UPDATE SET
         wake_word_enabled = ?2,
         tts_enabled = ?3,
         tts_voice = ?4,
         volume = ?5,
         language = ?6,
         updated_at = datetime('now')",
        rusqlite::params![
            device_id,
            input.wake_word_enabled.unwrap_or(true),
            input.tts_enabled.unwrap_or(true),
            input.tts_voice.as_deref().unwrap_or("onyx"),
            input.volume.unwrap_or(80),
            input.language.as_deref().unwrap_or("en"),
        ],
    )?;

    Ok(Json(serde_json::json!({ "ok": true })))
}

