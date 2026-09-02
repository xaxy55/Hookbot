use axum::extract::{Path, Query, State};
use axum::{Extension, Json};
use serde::Deserialize;

use std::sync::Arc;

use crate::config::AppConfig;
use crate::crypto;
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

/// GET /api/desk-lights — list desk light configurations
pub async fn list_lights(
    State(db): State<DbPool>,
    Query(q): Query<DeviceQuery>,
) -> Result<Json<Vec<DeskLightConfig>>, AppError> {
    let conn = db.lock().unwrap();
    let device_id = resolve_device_id(&conn, q.device_id.as_deref())?;

    let mut stmt = conn.prepare(
        "SELECT id, device_id, provider, name, bridge_ip, \
         COALESCE(api_key_enc, api_key) IS NOT NULL AND COALESCE(api_key_enc, api_key) != '', \
         light_ids, state_colors, enabled, created_at \
         FROM desk_lights WHERE device_id = ?1 ORDER BY created_at"
    )?;
    let lights = stmt.query_map([&device_id], |row| {
        let light_ids_str: String = row.get(6)?;
        let state_colors_str: String = row.get(7)?;
        Ok(DeskLightConfig {
            id: row.get(0)?,
            device_id: row.get(1)?,
            provider: row.get(2)?,
            name: row.get(3)?,
            bridge_ip: row.get(4)?,
            has_api_key: row.get(5)?,
            light_ids: serde_json::from_str(&light_ids_str).unwrap_or_default(),
            state_colors: serde_json::from_str(&state_colors_str).unwrap_or_default(),
            enabled: row.get(8)?,
            created_at: row.get(9)?,
        })
    })?.filter_map(|r| r.ok()).collect::<Vec<_>>();

    Ok(Json(lights))
}

/// POST /api/desk-lights — create a desk light configuration
pub async fn create_light(
    State(db): State<DbPool>,
    Extension(config): Extension<Arc<AppConfig>>,
    Json(input): Json<CreateDeskLight>,
) -> Result<Json<DeskLightConfig>, AppError> {
    let conn = db.lock().unwrap();
    let device_id = resolve_device_id(&conn, input.device_id.as_deref())?;
    let id = uuid::Uuid::new_v4().to_string();
    let light_ids_vec = input.light_ids.unwrap_or_default();
    let light_ids = serde_json::to_string(&light_ids_vec).unwrap();
    let default_colors = serde_json::json!({
        "idle": "#4488ff",
        "working": "#44ff44",
        "error": "#ff4444",
        "testing": "#ffaa00",
        "focus": "#8844ff"
    });
    let state_colors_val = input.state_colors.unwrap_or(default_colors);
    let state_colors = serde_json::to_string(&state_colors_val).unwrap();

    // Encrypt before it ever reaches the database.
    let enc_key = match input.api_key.as_deref().filter(|k| !k.is_empty()) {
        Some(k) => Some(
            crypto::encrypt(&config.session_secret, k)
                .map_err(|e| AppError::Internal(format!("Could not encrypt credential: {e}")))?,
        ),
        None => None,
    };

    conn.execute(
        "INSERT INTO desk_lights (id, device_id, provider, name, bridge_ip, api_key_enc, light_ids, state_colors) \
         VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8)",
        rusqlite::params![id, device_id, input.provider, input.name, input.bridge_ip, enc_key, light_ids, state_colors],
    )?;

    Ok(Json(DeskLightConfig {
        id,
        device_id,
        provider: input.provider,
        name: input.name,
        bridge_ip: input.bridge_ip,
        has_api_key: enc_key.is_some(),
        light_ids: light_ids_vec,
        state_colors: state_colors_val,
        enabled: true,
        created_at: chrono::Utc::now().to_rfc3339(),
    }))
}

/// PUT /api/desk-lights/:id — update a desk light configuration
pub async fn update_light(
    State(db): State<DbPool>,
    Extension(config): Extension<Arc<AppConfig>>,
    Path(id): Path<String>,
    Json(input): Json<UpdateDeskLight>,
) -> Result<Json<serde_json::Value>, AppError> {
    let conn = db.lock().unwrap();

    if let Some(name) = &input.name {
        conn.execute("UPDATE desk_lights SET name = ?1 WHERE id = ?2", rusqlite::params![name, id])?;
    }
    if let Some(ip) = &input.bridge_ip {
        conn.execute("UPDATE desk_lights SET bridge_ip = ?1 WHERE id = ?2", rusqlite::params![ip, id])?;
    }
    if let Some(key) = &input.api_key {
        let enc = crypto::encrypt(&config.session_secret, key)
            .map_err(|e| AppError::Internal(format!("Could not encrypt credential: {e}")))?;
        // Write the encrypted column and clear any legacy plaintext.
        conn.execute(
            "UPDATE desk_lights SET api_key_enc = ?1, api_key = NULL WHERE id = ?2",
            rusqlite::params![enc, id],
        )?;
    }
    if let Some(ids) = &input.light_ids {
        let json = serde_json::to_string(ids).unwrap();
        conn.execute("UPDATE desk_lights SET light_ids = ?1 WHERE id = ?2", rusqlite::params![json, id])?;
    }
    if let Some(colors) = &input.state_colors {
        let json = serde_json::to_string(colors).unwrap();
        conn.execute("UPDATE desk_lights SET state_colors = ?1 WHERE id = ?2", rusqlite::params![json, id])?;
    }
    if let Some(enabled) = input.enabled {
        conn.execute("UPDATE desk_lights SET enabled = ?1 WHERE id = ?2", rusqlite::params![enabled, id])?;
    }

    Ok(Json(serde_json::json!({ "ok": true })))
}

/// DELETE /api/desk-lights/:id — delete a desk light configuration
pub async fn delete_light(
    State(db): State<DbPool>,
    Path(id): Path<String>,
) -> Result<Json<serde_json::Value>, AppError> {
    let conn = db.lock().unwrap();
    conn.execute("DELETE FROM desk_lights WHERE id = ?1", [&id])?;
    Ok(Json(serde_json::json!({ "ok": true })))
}

/// POST /api/desk-lights/:id/action — trigger a light action (set color, effect, etc.)
pub async fn trigger_action(
    State(db): State<DbPool>,
    Extension(config): Extension<Arc<AppConfig>>,
    Path(id): Path<String>,
    Json(input): Json<DeskLightAction>,
) -> Result<Json<serde_json::Value>, AppError> {
    // Scope the guard: it is not Send, so it must not be alive across the
    // awaits below.
    let (provider, bridge_ip, enc_key, light_ids) = {
        let conn = db.lock().unwrap();
        let light: Result<(String, String, Option<String>, String), _> = conn.query_row(
            "SELECT provider, COALESCE(bridge_ip, ''), COALESCE(api_key_enc, ''), light_ids \
             FROM desk_lights WHERE id = ?1 AND enabled = 1",
            [&id],
            |row| Ok((row.get(0)?, row.get(1)?, row.get(2)?, row.get(3)?)),
        );
        let (p, b, e, ids_json) =
            light.map_err(|_| AppError::NotFound("Light config not found or disabled".into()))?;
        let ids: Vec<String> = serde_json::from_str(&ids_json).unwrap_or_default();
        (p, b, e, ids)
    };

    if provider != "hue" {
        return Err(AppError::BadRequest(format!(
            "Provider '{provider}' is not supported yet"
        )));
    }
    let Some(enc_key) = enc_key.filter(|k: &String| !k.is_empty()) else {
        return Err(AppError::BadRequest(
            "This bridge is not paired yet. Run the pairing flow first.".into(),
        ));
    };
    let token = crypto::decrypt(&config.session_secret, &enc_key)
        .map_err(|e| AppError::Internal(format!("Stored credential unreadable: {e}")))?;

    // Hue takes hue/sat rather than hex, so translate what the UI sends.
    let mut body = serde_json::Map::new();
    body.insert("on".into(), serde_json::json!(true));
    if let Some(b) = input.brightness {
        body.insert("bri".into(), serde_json::json!(b.clamp(0, 254)));
    }
    if let Some(hex) = input.color.as_deref() {
        if let Some((h, s)) = hex_to_hue_sat(hex) {
            body.insert("hue".into(), serde_json::json!(h));
            body.insert("sat".into(), serde_json::json!(s));
        }
    }
    let payload = serde_json::Value::Object(body);

    let client = bridge_client()?;

    let mut applied = 0usize;
    for lid in &light_ids {
        let mut r = client
            .put(format!("https://{bridge_ip}/api/{token}/lights/{lid}/state"))
            .json(&payload)
            .send()
            .await;
        if r.is_err() {
            r = client
                .put(format!("http://{bridge_ip}/api/{token}/lights/{lid}/state"))
                .json(&payload)
                .send()
                .await;
        }
        match r {
            Ok(r) if r.status().is_success() => applied += 1,
            Ok(r) => tracing::warn!("Hue light {lid} returned {}", r.status()),
            Err(e) => tracing::warn!("Hue light {lid} unreachable: {e}"),
        }
    }

    Ok(Json(serde_json::json!({
        "ok": applied > 0,
        "provider": provider,
        "lights_applied": applied,
        "lights_total": light_ids.len(),
    })))
}


/// Hue bridges serve their local API over HTTPS with a self-signed
/// certificate, and newer firmware 301s plain HTTP to it. Verification is
/// therefore disabled *for bridge requests only* — this client is never used
/// for anything else, and the bridge is a device on the user's own network
/// reached by address, not a public endpoint.
fn bridge_client() -> Result<reqwest::Client, AppError> {
    reqwest::Client::builder()
        .danger_accept_invalid_certs(true)
        .timeout(std::time::Duration::from_secs(5))
        .build()
        .map_err(|e| AppError::Internal(e.to_string()))
}

/// Hue wants hue/saturation, the UI stores hex.
fn hex_to_hue_sat(hex: &str) -> Option<(u16, u8)> {
    let hex = hex.trim_start_matches('#');
    if hex.len() != 6 {
        return None;
    }
    let r = u8::from_str_radix(&hex[0..2], 16).ok()? as f32 / 255.0;
    let g = u8::from_str_radix(&hex[2..4], 16).ok()? as f32 / 255.0;
    let b = u8::from_str_radix(&hex[4..6], 16).ok()? as f32 / 255.0;
    let max = r.max(g).max(b);
    let min = r.min(g).min(b);
    let delta = max - min;

    let mut h = if delta == 0.0 {
        0.0
    } else if max == r {
        60.0 * (((g - b) / delta) % 6.0)
    } else if max == g {
        60.0 * (((b - r) / delta) + 2.0)
    } else {
        60.0 * (((r - g) / delta) + 4.0)
    };
    if h < 0.0 {
        h += 360.0;
    }
    let s = if max == 0.0 { 0.0 } else { delta / max };
    Some(((h / 360.0 * 65535.0) as u16, (s * 254.0) as u8))
}

/// POST /api/desk-lights/hue/pair — capture a bridge credential.
///
/// The Hue bridge only issues a username while its physical link button has
/// been pressed in the last ~30 seconds, so this polls for it. The resulting
/// token is encrypted before storage and is never returned to the caller.
pub async fn hue_pair(
    State(db): State<DbPool>,
    Extension(config): Extension<Arc<AppConfig>>,
    Json(input): Json<HuePairRequest>,
) -> Result<Json<serde_json::Value>, AppError> {
    let device_id = {
        let conn = db.lock().unwrap();
        resolve_device_id(&conn, input.device_id.as_deref())?
    };

    // Reject anything that is not a bare host: this string is interpolated
    // into a URL, so a value containing a path or scheme could redirect the
    // request somewhere else entirely.
    let host = input.bridge_ip.trim().trim_end_matches('/');
    if host.is_empty()
        || !host
            .chars()
            .all(|c| c.is_ascii_alphanumeric() || matches!(c, '.' | '-' | ':'))
    {
        return Err(AppError::BadRequest(
            "Bridge address must be a bare hostname or IP, e.g. 192.168.1.42".into(),
        ));
    }

    let client = bridge_client()?;
    let body = serde_json::json!({ "devicetype": "hookbot#server" });

    // ~30s of polling, matching the bridge's own link-button window.
    let mut last_error = "no response from bridge".to_string();
    for _ in 0..30 {
        // Try HTTPS first; older bridges only speak plain HTTP.
        let mut res = client
            .post(format!("https://{host}/api"))
            .json(&body)
            .send()
            .await;
        if res.is_err() {
            res = client.post(format!("http://{host}/api")).json(&body).send().await;
        }
        match res {
            Ok(res) => {
                let parsed: serde_json::Value = res.json().await.unwrap_or(serde_json::Value::Null);
                if let Some(username) = parsed[0]["success"]["username"].as_str() {
                    let enc = crypto::encrypt(&config.session_secret, username).map_err(|e| {
                        AppError::Internal(format!("Could not encrypt credential: {e}"))
                    })?;
                    let name = input.name.unwrap_or_else(|| "Hue Bridge".to_string());
                    let id = uuid::Uuid::new_v4().to_string();
                    {
                        let conn = db.lock().unwrap();
                        conn.execute(
                            "INSERT INTO desk_lights (id, device_id, provider, name, bridge_ip, api_key_enc, light_ids, state_colors) \
                             VALUES (?1, ?2, 'hue', ?3, ?4, ?5, '[]', '{}')",
                            rusqlite::params![id, device_id, name, host, enc],
                        )?;
                    }
                    tracing::info!("Hue bridge paired at {host} (credential stored encrypted)");
                    // Deliberately no token in the response.
                    return Ok(Json(serde_json::json!({
                        "ok": true,
                        "id": id,
                        "bridge_ip": host,
                        "message": "Paired. The bridge credential is stored encrypted and cannot be read back."
                    })));
                }
                if let Some(desc) = parsed[0]["error"]["description"].as_str() {
                    last_error = desc.to_string();
                }
            }
            Err(e) => last_error = e.to_string(),
        }
        tokio::time::sleep(std::time::Duration::from_secs(1)).await;
    }

    Err(AppError::BadRequest(format!(
        "Pairing timed out: {last_error}. Press the round button on the bridge, then try again."
    )))
}
