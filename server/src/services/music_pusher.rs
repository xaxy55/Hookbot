//! Mirrors Spotify playback onto each device's display.
//!
//! The device cannot talk to Spotify itself — it has no tokens and no room for
//! a TLS stack alongside everything else — so the server polls now-playing and
//! pushes a compact summary to `POST /music` on the device.

use std::sync::Arc;

use tracing::{debug, warn};

use crate::config::AppConfig;
use crate::db::DbPool;
use crate::routes::music;
use crate::services::proxy;

pub fn start(db: DbPool, config: Arc<AppConfig>, interval_secs: u64) {
    if config.spotify_client_id.is_none() {
        return; // Nothing to push without an integration configured.
    }

    tokio::spawn(async move {
        tracing::info!("Music pusher started (interval: {interval_secs}s)");
        let mut interval = tokio::time::interval(tokio::time::Duration::from_secs(interval_secs));
        loop {
            interval.tick().await;
            if let Err(e) = push_once(&db, &config).await {
                debug!("Music push skipped: {e}");
            }
        }
    });
}

async fn push_once(db: &DbPool, config: &AppConfig) -> Result<(), String> {
    // Only devices we can reach directly and that have Spotify connected.
    let targets: Vec<(String, String)> = {
        let conn = db.lock().map_err(|e| e.to_string())?;
        let mut stmt = conn
            .prepare(
                "SELECT d.id, d.ip_address FROM devices d \
                 JOIN music_configs m ON m.device_id = d.id \
                 WHERE m.provider = 'spotify' AND m.enabled = 1 \
                   AND d.ip_address IS NOT NULL AND d.ip_address != ''",
            )
            .map_err(|e| e.to_string())?;
        let rows = stmt
            .query_map([], |r| Ok((r.get(0)?, r.get(1)?)))
            .map_err(|e| e.to_string())?;
        rows.filter_map(Result::ok).collect()
    };

    for (device_id, ip) in targets {
        let track = match music::current_track(db, config, &device_id).await {
            Ok(t) => t,
            Err(e) => {
                warn!("Now-playing lookup failed for {device_id}: {e:?}");
                continue;
            }
        };

        // The display only has room for a short line, and the firmware
        // truncates anyway — send the two fields it renders.
        let payload = serde_json::json!({
            "playing": track.is_playing,
            "track": track.track_name.unwrap_or_default(),
            "artist": track.artist_name.unwrap_or_default(),
        });

        // A device that is asleep or rebooting is expected; log and move on.
        if let Err(e) = proxy::forward_json(&format!("http://{ip}/music"), &payload).await {
            debug!("Could not push now-playing to {ip}: {e:?}");
        }
    }
    Ok(())
}
