use axum::extract::{Path, Query, State};
use axum::http::StatusCode;
use axum::response::{IntoResponse, Redirect, Response};
use axum::{Extension, Json};
use base64::engine::general_purpose::URL_SAFE_NO_PAD;
use base64::Engine as _;
use hmac::{Hmac, Mac};
use serde::Deserialize;
use sha2::{Digest, Sha256};
use std::sync::Arc;

use crate::config::AppConfig;
use crate::db::DbPool;
use crate::error::AppError;
use crate::models::*;

const SPOTIFY_SCOPES: &str =
    "user-read-playback-state user-modify-playback-state user-read-currently-playing";

/// Percent-encode for query strings (same helper style as the WorkOS flow).
fn urlencode(s: &str) -> String {
    s.bytes()
        .map(|b| match b {
            b'A'..=b'Z' | b'a'..=b'z' | b'0'..=b'9' | b'-' | b'_' | b'.' | b'~' => {
                (b as char).to_string()
            }
            _ => format!("%{:02X}", b),
        })
        .collect()
}

fn hmac_bytes(secret: &[u8; 32], msg: &str) -> [u8; 32] {
    let mut mac = <Hmac<Sha256>>::new_from_slice(secret).expect("HMAC accepts any key length");
    mac.update(msg.as_bytes());
    mac.finalize().into_bytes().into()
}

/// The OAuth `state` is signed rather than stored: it carries the device id
/// through the round trip, the random nonce makes it unguessable, and the HMAC
/// proves we issued it — so a stray or forged callback cannot bind someone
/// else's Spotify account to a device.
fn sign_state(secret: &[u8; 32], device_id: &str, issued_at: i64, nonce: &str) -> String {
    let payload = format!("{}:{}:{}", device_id, issued_at, nonce);
    let sig = hex::encode(hmac_bytes(secret, &payload));
    format!("{}:{}", payload, sig)
}

/// Returns the payload (device_id, issued_at, nonce) only if the signature is
/// ours and still fresh.
fn verify_state(secret: &[u8; 32], state: &str) -> Option<(String, String)> {
    let (payload, sig) = state.rsplit_once(':')?;
    let mut parts = payload.splitn(3, ':');
    let device_id = parts.next()?;
    let issued_at: i64 = parts.next()?.parse().ok()?;
    let nonce = parts.next()?;

    // 10 minutes is plenty for a consent screen and keeps a leaked link short-lived.
    if (chrono::Utc::now().timestamp() - issued_at).abs() > 600 {
        return None;
    }

    let expected = sign_state(secret, device_id, issued_at, nonce);
    let expected_sig = expected.rsplit_once(':')?.1;
    if sig.len() != expected_sig.len() {
        return None;
    }
    // Constant-time compare so a mismatch cannot be probed byte by byte.
    let equal = sig
        .bytes()
        .zip(expected_sig.bytes())
        .fold(0u8, |acc, (a, b)| acc | (a ^ b))
        == 0;
    equal.then(|| (device_id.to_string(), payload.to_string()))
}

/// PKCE verifier, derived rather than stored. It is an HMAC of the state
/// payload under session_secret, so only this server can compute it, it is
/// unique per authorization attempt (the payload carries a random nonce), and
/// it is never put in a URL, a cookie, or the database. base64url of 32 bytes
/// is 43 characters, which is exactly RFC 7636's minimum length and uses only
/// unreserved characters.
fn pkce_verifier(secret: &[u8; 32], state_payload: &str) -> String {
    URL_SAFE_NO_PAD.encode(hmac_bytes(secret, &format!("spotify-pkce:{}", state_payload)))
}

fn pkce_challenge(verifier: &str) -> String {
    URL_SAFE_NO_PAD.encode(Sha256::digest(verifier.as_bytes()))
}

#[derive(Debug, Deserialize)]
struct SpotifyTokens {
    access_token: String,
    refresh_token: Option<String>,
    expires_in: i64,
}

#[derive(Debug, Deserialize)]
pub struct CallbackQuery {
    pub code: Option<String>,
    pub state: Option<String>,
    pub error: Option<String>,
}

/// GET /api/music/spotify/authorize — send the browser to Spotify's consent screen.
pub async fn spotify_authorize(
    State(db): State<DbPool>,
    Extension(config): Extension<Arc<AppConfig>>,
    Query(q): Query<DeviceQuery>,
) -> Result<Redirect, AppError> {
    let (client_id, redirect_uri) = spotify_credentials(&config)?;

    let device_id = {
        let conn = db.lock().unwrap();
        resolve_device_id(&conn, q.device_id.as_deref())?
    };

    // Random nonce so the state cannot be guessed or replayed.
    let nonce = uuid::Uuid::new_v4().simple().to_string();
    let state = sign_state(
        &config.session_secret,
        &device_id,
        chrono::Utc::now().timestamp(),
        &nonce,
    );
    let payload = state.rsplit_once(':').map(|(p, _)| p).unwrap_or_default();
    let challenge = pkce_challenge(&pkce_verifier(&config.session_secret, payload));

    let url = format!(
        "https://accounts.spotify.com/authorize?client_id={}&response_type=code&redirect_uri={}\
         &scope={}&state={}&code_challenge_method=S256&code_challenge={}",
        urlencode(client_id),
        urlencode(redirect_uri),
        urlencode(SPOTIFY_SCOPES),
        urlencode(&state),
        urlencode(&challenge),
    );
    Ok(Redirect::to(&url))
}

/// GET /api/music/spotify/callback — exchange the code and store the tokens.
pub async fn spotify_callback(
    State(db): State<DbPool>,
    Extension(config): Extension<Arc<AppConfig>>,
    Query(q): Query<CallbackQuery>,
) -> Response {
    let frontend = config.frontend_url.clone().unwrap_or_default();
    let fail = |reason: &str| -> Response {
        if frontend.is_empty() {
            (StatusCode::BAD_REQUEST, Json(serde_json::json!({ "error": reason }))).into_response()
        } else {
            Redirect::to(&format!("{}/music?spotify_error={}", frontend, urlencode(reason)))
                .into_response()
        }
    };

    if let Some(err) = q.error {
        // Reflect only a conservative subset: this value comes from the query
        // string and ends up in a redirect URL the frontend renders.
        let safe: String = err
            .chars()
            .filter(|c| c.is_ascii_alphanumeric() || *c == '_' || *c == '-')
            .take(64)
            .collect();
        return fail(if safe.is_empty() { "denied" } else { &safe });
    }
    let (Some(code), Some(state)) = (q.code, q.state) else {
        return fail("missing_code");
    };
    let Some((device_id, state_payload)) = verify_state(&config.session_secret, &state) else {
        return fail("invalid_state");
    };
    let (client_id, redirect_uri) = match spotify_credentials(&config) {
        Ok(v) => v,
        Err(_) => return fail("not_configured"),
    };
    let verifier = pkce_verifier(&config.session_secret, &state_payload);

    let tokens = match exchange_code(client_id, redirect_uri, &code, &verifier).await {
        Ok(t) => t,
        Err(e) => {
            tracing::warn!("Spotify token exchange failed: {}", e);
            return fail("token_exchange_failed");
        }
    };

    let expires_at = (chrono::Utc::now() + chrono::Duration::seconds(tokens.expires_in)).to_rfc3339();
    {
        let conn = db.lock().unwrap();
        // Re-connecting an existing device replaces its tokens rather than
        // creating a second row (device_id+provider is UNIQUE).
        if let Err(e) = conn.execute(
            "INSERT INTO music_configs (id, device_id, provider, access_token, refresh_token, token_expires_at, scopes, enabled)
             VALUES (?1, ?2, 'spotify', ?3, ?4, ?5, ?6, 1)
             ON CONFLICT(device_id, provider) DO UPDATE SET
                 access_token = excluded.access_token,
                 refresh_token = COALESCE(excluded.refresh_token, music_configs.refresh_token),
                 token_expires_at = excluded.token_expires_at,
                 scopes = excluded.scopes,
                 enabled = 1",
            rusqlite::params![
                uuid::Uuid::new_v4().to_string(),
                device_id,
                tokens.access_token,
                tokens.refresh_token,
                expires_at,
                SPOTIFY_SCOPES,
            ],
        ) {
            tracing::error!("Failed to persist Spotify tokens: {}", e);
            return fail("persist_failed");
        }
    }

    tracing::info!("Spotify connected for device {}", device_id);
    if frontend.is_empty() {
        Json(serde_json::json!({ "ok": true, "provider": "spotify" })).into_response()
    } else {
        Redirect::to(&format!("{}/music?spotify=connected", frontend)).into_response()
    }
}

fn spotify_credentials(config: &AppConfig) -> Result<(&str, &str), AppError> {
    match (
        config.spotify_client_id.as_deref(),
        config.spotify_redirect_uri.as_deref(),
    ) {
        (Some(id), Some(uri)) => Ok((id, uri)),
        _ => Err(AppError::BadRequest(
            "Spotify is not configured. Set SPOTIFY_CLIENT_ID and SPOTIFY_REDIRECT_URI.".into(),
        )),
    }
}

async fn exchange_code(
    client_id: &str,
    redirect_uri: &str,
    code: &str,
    code_verifier: &str,
) -> Result<SpotifyTokens, String> {
    // PKCE: the verifier replaces the client secret, so nothing confidential
    // has to be deployed alongside the server.
    let res = reqwest::Client::new()
        .post("https://accounts.spotify.com/api/token")
        .form(&[
            ("grant_type", "authorization_code"),
            ("code", code),
            ("redirect_uri", redirect_uri),
            ("client_id", client_id),
            ("code_verifier", code_verifier),
        ])
        .send()
        .await
        .map_err(|e| e.to_string())?;

    if !res.status().is_success() {
        return Err(format!("HTTP {}", res.status()));
    }
    res.json::<SpotifyTokens>().await.map_err(|e| e.to_string())
}

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
        let access_token: Option<String> = row.get(3)?;
        let refresh_token: Option<String> = row.get(4)?;
        Ok(MusicConfig {
            id: row.get(0)?,
            device_id: row.get(1)?,
            provider: row.get(2)?,
            connected: access_token.as_deref().is_some_and(|t| !t.is_empty()),
            access_token,
            refresh_token,
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
        connected: input.access_token.as_deref().is_some_and(|t| !t.is_empty()),
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
    Extension(config): Extension<Arc<AppConfig>>,
    Query(q): Query<DeviceQuery>,
) -> Result<Json<NowPlaying>, AppError> {
    let device_id = {
        let conn = db.lock().unwrap();
        resolve_device_id(&conn, q.device_id.as_deref())?
    };
    Ok(Json(current_track(&db, &config, &device_id).await?))
}

fn nothing_playing() -> NowPlaying {
    NowPlaying {
        is_playing: false,
        track_name: None,
        artist_name: None,
        album_name: None,
        album_art_url: None,
        progress_ms: None,
        duration_ms: None,
    }
}

/// Shared by the HTTP route and the background pusher that mirrors playback
/// onto the device display.
pub(crate) async fn current_track(
    db: &DbPool,
    config: &AppConfig,
    device_id: &str,
) -> Result<NowPlaying, AppError> {
    // Nothing connected yet is a normal state, not an error — the UI renders
    // an empty player and a Connect button.
    let Some(token) = access_token(db, config, device_id).await? else {
        return Ok(NowPlaying {
            is_playing: false,
            track_name: None,
            artist_name: None,
            album_name: None,
            album_art_url: None,
            progress_ms: None,
            duration_ms: None,
        });
    };

    let res = reqwest::Client::new()
        .get("https://api.spotify.com/v1/me/player/currently-playing")
        .bearer_auth(&token)
        .send()
        .await
        .map_err(|e| AppError::Internal(format!("Spotify request failed: {e}")))?;

    // 204 means "nothing is playing right now", which is not a failure.
    if res.status() == StatusCode::NO_CONTENT {
        return Ok(nothing_playing());
    }
    if !res.status().is_success() {
        return Err(AppError::Internal(format!("Spotify returned {}", res.status())));
    }

    let body: serde_json::Value = res
        .json()
        .await
        .map_err(|e| AppError::Internal(format!("Spotify sent invalid JSON: {e}")))?;
    let item = &body["item"];

    Ok(NowPlaying {
        is_playing: body["is_playing"].as_bool().unwrap_or(false),
        track_name: item["name"].as_str().map(str::to_string),
        artist_name: item["artists"][0]["name"].as_str().map(str::to_string),
        album_name: item["album"]["name"].as_str().map(str::to_string),
        album_art_url: item["album"]["images"][0]["url"].as_str().map(str::to_string),
        progress_ms: body["progress_ms"].as_i64(),
        duration_ms: item["duration_ms"].as_i64(),
    })
}

/// Return a usable Spotify access token for the device, refreshing it first if
/// it is expired or about to be. `None` means no account is connected.
async fn access_token(
    db: &DbPool,
    config: &AppConfig,
    device_id: &str,
) -> Result<Option<String>, AppError> {
    let row: Option<(String, Option<String>, Option<String>, Option<String>)> = {
        let conn = db.lock().unwrap();
        conn.query_row(
            "SELECT id, access_token, refresh_token, token_expires_at \
             FROM music_configs WHERE device_id = ?1 AND provider = 'spotify' AND enabled = 1 LIMIT 1",
            [device_id],
            |r| Ok((r.get(0)?, r.get(1)?, r.get(2)?, r.get(3)?)),
        )
        .ok()
    };
    let Some((config_id, access, refresh, expires_at)) = row else {
        return Ok(None);
    };
    let Some(access) = access else { return Ok(None) };

    // Refresh a minute early so a token cannot expire mid-request.
    let expired = expires_at
        .and_then(|t| chrono::DateTime::parse_from_rfc3339(&t).ok())
        .map(|t| chrono::Utc::now() + chrono::Duration::seconds(60) >= t)
        .unwrap_or(false);
    if !expired {
        return Ok(Some(access));
    }

    let (Some(refresh), Some(client_id)) = (refresh, config.spotify_client_id.as_deref()) else {
        // Expired with no way to refresh — the user has to reconnect.
        return Ok(Some(access));
    };

    let res = reqwest::Client::new()
        .post("https://accounts.spotify.com/api/token")
        .form(&[
            ("grant_type", "refresh_token"),
            ("refresh_token", &refresh),
            ("client_id", client_id),
        ])
        .send()
        .await
        .map_err(|e| AppError::Internal(format!("Spotify refresh failed: {e}")))?;

    if !res.status().is_success() {
        return Err(AppError::Internal(format!(
            "Spotify refresh returned {}",
            res.status()
        )));
    }
    let tokens: SpotifyTokens = res
        .json()
        .await
        .map_err(|e| AppError::Internal(format!("Spotify sent invalid JSON: {e}")))?;
    let new_expiry =
        (chrono::Utc::now() + chrono::Duration::seconds(tokens.expires_in)).to_rfc3339();

    {
        let conn = db.lock().unwrap();
        // Spotify only returns a new refresh token sometimes; keep the old one otherwise.
        conn.execute(
            "UPDATE music_configs SET access_token = ?1, token_expires_at = ?2, \
             refresh_token = COALESCE(?3, refresh_token) WHERE id = ?4",
            rusqlite::params![tokens.access_token, new_expiry, tokens.refresh_token, config_id],
        )?;
    }
    Ok(Some(tokens.access_token))
}

/// POST /api/music/action — control playback
pub async fn music_action(
    State(db): State<DbPool>,
    Extension(config): Extension<Arc<AppConfig>>,
    Query(q): Query<DeviceQuery>,
    Json(input): Json<MusicAction>,
) -> Result<Json<serde_json::Value>, AppError> {
    let device_id = {
        let conn = db.lock().unwrap();
        resolve_device_id(&conn, q.device_id.as_deref())?
    };

    let token = access_token(&db, &config, &device_id)
        .await?
        .ok_or_else(|| AppError::NotFound("No active music integration found".into()))?;

    // Spotify uses different verbs per action, and returns 204 on success.
    let client = reqwest::Client::new();
    let req = match input.action.as_str() {
        "play" => client.put("https://api.spotify.com/v1/me/player/play"),
        "pause" => client.put("https://api.spotify.com/v1/me/player/pause"),
        "next" => client.post("https://api.spotify.com/v1/me/player/next"),
        "previous" => client.post("https://api.spotify.com/v1/me/player/previous"),
        other => {
            return Err(AppError::BadRequest(format!(
                "Unsupported action '{other}'. Use play, pause, next or previous."
            )))
        }
    };

    let res = req
        .bearer_auth(&token)
        .header("Content-Length", "0")
        .send()
        .await
        .map_err(|e| AppError::Internal(format!("Spotify request failed: {e}")))?;

    // 404 here means no active Spotify device, which is worth saying plainly.
    if res.status() == StatusCode::NOT_FOUND {
        return Err(AppError::BadRequest(
            "No active Spotify player. Start playback on a device first.".into(),
        ));
    }
    if !res.status().is_success() {
        return Err(AppError::Internal(format!("Spotify returned {}", res.status())));
    }

    Ok(Json(serde_json::json!({
        "ok": true,
        "provider": "spotify",
        "action": input.action,
    })))
}

#[cfg(test)]
mod tests {
    use super::*;

    const SECRET: [u8; 32] = [7u8; 32];

    /// The browser must never receive Spotify credentials. A refresh token is
    /// long-lived, so leaking it would undo the point of the PKCE flow.
    #[test]
    fn serialized_config_never_carries_oauth_tokens() {
        let cfg = MusicConfig {
            id: "cfg-1".into(),
            device_id: "dev-1".into(),
            provider: "spotify".into(),
            connected: true,
            access_token: Some("BQAsecret-access".into()),
            refresh_token: Some("AQAsecret-refresh".into()),
            auto_pause_meetings: true,
            focus_playlist_id: None,
            enabled: true,
            created_at: "2026-01-01".into(),
        };

        let json = serde_json::to_string(&cfg).expect("serializes");
        assert!(!json.contains("secret-access"), "access token leaked: {json}");
        assert!(!json.contains("secret-refresh"), "refresh token leaked: {json}");
        assert!(!json.contains("access_token"), "field name still present: {json}");
        assert!(!json.contains("refresh_token"), "field name still present: {json}");
        assert!(json.contains("\"connected\":true"), "connected flag missing: {json}");
    }

    fn now() -> i64 {
        chrono::Utc::now().timestamp()
    }

    #[test]
    fn round_trips_a_valid_state() {
        let state = sign_state(&SECRET, "dev-1", now(), "nonce-a");
        let (device_id, _) = verify_state(&SECRET, &state).expect("valid state accepted");
        assert_eq!(device_id, "dev-1");
    }

    #[test]
    fn rejects_a_tampered_device_id() {
        let state = sign_state(&SECRET, "dev-1", now(), "nonce-a");
        let forged = state.replace("dev-1", "dev-2");
        assert!(verify_state(&SECRET, &forged).is_none());
    }

    #[test]
    fn rejects_a_forged_signature() {
        let payload = format!("dev-1:{}:nonce-a", now());
        let forged = format!("{}:{}", payload, "0".repeat(64));
        assert!(verify_state(&SECRET, &forged).is_none());
    }

    #[test]
    fn rejects_a_state_signed_with_another_key() {
        let state = sign_state(&[9u8; 32], "dev-1", now(), "nonce-a");
        assert!(verify_state(&SECRET, &state).is_none());
    }

    #[test]
    fn rejects_an_expired_state() {
        let state = sign_state(&SECRET, "dev-1", now() - 601, "nonce-a");
        assert!(verify_state(&SECRET, &state).is_none());
    }

    #[test]
    fn verifier_is_unique_per_nonce_and_never_guessable_without_the_key() {
        let p1 = format!("dev-1:{}:nonce-a", now());
        let p2 = format!("dev-1:{}:nonce-b", now());
        let v1 = pkce_verifier(&SECRET, &p1);
        let v2 = pkce_verifier(&SECRET, &p2);
        assert_ne!(v1, v2, "a fresh nonce must yield a fresh verifier");
        assert_ne!(
            v1,
            pkce_verifier(&[9u8; 32], &p1),
            "another key must not reproduce the verifier"
        );
        assert_eq!(v1, pkce_verifier(&SECRET, &p1), "same input is stable");
    }

    #[test]
    fn verifier_meets_rfc7636_length_and_charset() {
        let v = pkce_verifier(&SECRET, "dev-1:123:nonce");
        assert!((43..=128).contains(&v.len()), "len was {}", v.len());
        assert!(v
            .chars()
            .all(|c| c.is_ascii_alphanumeric() || matches!(c, '-' | '.' | '_' | '~')));
    }

    #[test]
    fn challenge_is_s256_of_the_verifier() {
        let v = pkce_verifier(&SECRET, "dev-1:123:nonce");
        let expected = URL_SAFE_NO_PAD.encode(Sha256::digest(v.as_bytes()));
        assert_eq!(pkce_challenge(&v), expected);
        assert_ne!(pkce_challenge(&v), v, "the verifier must not be sent as-is");
    }
}
