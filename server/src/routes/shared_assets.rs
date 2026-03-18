use axum::extract::{Path, Query, State};
use axum::Json;
use serde::{Deserialize, Serialize};

use crate::db::DbPool;
use crate::error::AppError;

// ── Types ─────────────────────────────────────────────────────────

#[derive(Debug, Serialize)]
pub struct SharedAsset {
    pub id: String,
    pub name: String,
    pub description: String,
    pub author: String,
    pub asset_type: String,
    pub payload: serde_json::Value,
    pub downloads: i64,
    pub rating_avg: f64,
    pub rating_count: i64,
    pub installed: bool,
    pub verified: bool,
    pub created_at: String,
}

#[derive(Debug, Deserialize)]
pub struct ListQuery {
    pub device_id: Option<String>,
    pub asset_type: Option<String>,
    pub search: Option<String>,
    pub sort: Option<String>,
}

#[derive(Debug, Deserialize)]
pub struct PublishRequest {
    pub name: String,
    pub description: Option<String>,
    pub author: Option<String>,
    pub asset_type: String, // "avatar", "animation", "screensaver"
    pub payload: serde_json::Value,
}

#[derive(Debug, Deserialize)]
pub struct InstallRequest {
    pub device_id: Option<String>,
}

#[derive(Debug, Deserialize)]
pub struct RateRequest {
    pub device_id: Option<String>,
    pub stars: i32,
}

// ── Helpers ───────────────────────────────────────────────────────

fn resolve_device_id(conn: &rusqlite::Connection, device_id: Option<&str>) -> Result<String, AppError> {
    if let Some(id) = device_id {
        return Ok(id.to_string());
    }
    conn.query_row("SELECT id FROM devices ORDER BY created_at LIMIT 1", [], |row| row.get(0))
        .map_err(|_| AppError::NotFound("No devices registered".into()))
}

fn is_verified_publisher(conn: &rusqlite::Connection, author: &str) -> bool {
    conn.query_row(
        "SELECT COUNT(*) FROM verified_publishers WHERE name = ?1",
        [author],
        |row| row.get::<_, i32>(0),
    ).map(|c| c > 0).unwrap_or(false)
}

// ── Handlers ──────────────────────────────────────────────────────

/// GET /api/community/assets — list shared assets
pub async fn list_assets(
    State(db): State<DbPool>,
    Query(q): Query<ListQuery>,
) -> Result<Json<Vec<SharedAsset>>, AppError> {
    let conn = db.lock().unwrap();
    let device_id = resolve_device_id(&conn, q.device_id.as_deref()).ok();

    let order = match q.sort.as_deref() {
        Some("popular") => "a.downloads DESC",
        Some("rating") => "(CASE WHEN a.rating_count = 0 THEN 0 ELSE CAST(a.rating_sum AS REAL) / a.rating_count END) DESC",
        Some("verified") => "verified DESC, a.created_at DESC",
        Some("newest") | _ => "a.created_at DESC",
    };

    let sql = format!(
        "SELECT a.id, a.name, a.description, a.author, a.asset_type, a.payload, \
         a.downloads, a.rating_sum, a.rating_count, a.created_at, \
         CASE WHEN vp.id IS NOT NULL THEN 1 ELSE a.verified END AS verified \
         FROM shared_assets a \
         LEFT JOIN verified_publishers vp ON a.author = vp.name \
         ORDER BY {order}"
    );

    let mut stmt = conn.prepare(&sql)?;
    let rows = stmt.query_map([], |row| {
        Ok((
            row.get::<_, String>(0)?,
            row.get::<_, String>(1)?,
            row.get::<_, String>(2)?,
            row.get::<_, String>(3)?,
            row.get::<_, String>(4)?,
            row.get::<_, String>(5)?,
            row.get::<_, i64>(6)?,
            row.get::<_, i64>(7)?,
            row.get::<_, i64>(8)?,
            row.get::<_, String>(9)?,
            row.get::<_, bool>(10)?,
        ))
    })?;

    let mut assets = Vec::new();
    for row in rows {
        let (id, name, description, author, asset_type, payload_json,
             downloads, rating_sum, rating_count, created_at, verified) = row?;

        // Filter by type
        if let Some(ref at) = q.asset_type {
            if at != "all" && &asset_type != at {
                continue;
            }
        }

        // Filter by search
        if let Some(ref search) = q.search {
            let s = search.to_lowercase();
            if !name.to_lowercase().contains(&s)
                && !description.to_lowercase().contains(&s)
                && !author.to_lowercase().contains(&s)
            {
                continue;
            }
        }

        let installed = if let Some(ref did) = device_id {
            conn.query_row(
                "SELECT COUNT(*) FROM shared_asset_installs WHERE asset_id = ?1 AND device_id = ?2",
                rusqlite::params![id, did],
                |row| row.get::<_, i32>(0),
            ).map(|c| c > 0).unwrap_or(false)
        } else {
            false
        };

        let payload: serde_json::Value = serde_json::from_str(&payload_json)
            .unwrap_or(serde_json::Value::Object(Default::default()));
        let rating_avg = if rating_count > 0 { rating_sum as f64 / rating_count as f64 } else { 0.0 };

        assets.push(SharedAsset {
            id, name, description, author, asset_type, payload,
            downloads, rating_avg, rating_count, installed, verified, created_at,
        });
    }

    Ok(Json(assets))
}

/// POST /api/community/assets — publish a shared asset
pub async fn publish_asset(
    State(db): State<DbPool>,
    Json(input): Json<PublishRequest>,
) -> Result<Json<SharedAsset>, AppError> {
    let conn = db.lock().unwrap();

    let valid_types = ["avatar", "animation", "screensaver"];
    if !valid_types.contains(&input.asset_type.as_str()) {
        return Err(AppError::BadRequest(format!(
            "asset_type must be one of: {}", valid_types.join(", ")
        )));
    }

    let id = uuid::Uuid::new_v4().to_string();
    let description = input.description.unwrap_or_default();
    let author = input.author.unwrap_or_else(|| "anonymous".to_string());
    let payload_json = serde_json::to_string(&input.payload).unwrap_or_else(|_| "{}".to_string());

    let verified = is_verified_publisher(&conn, &author);

    conn.execute(
        "INSERT INTO shared_assets (id, name, description, author, asset_type, payload, verified) \
         VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7)",
        rusqlite::params![id, input.name, description, author, input.asset_type, payload_json, verified],
    )?;

    Ok(Json(SharedAsset {
        id,
        name: input.name,
        description,
        author,
        asset_type: input.asset_type,
        payload: input.payload,
        downloads: 0,
        rating_avg: 0.0,
        rating_count: 0,
        installed: false,
        verified,
        created_at: "just now".into(),
    }))
}

/// POST /api/community/assets/:id/install — install a shared asset
pub async fn install_asset(
    State(db): State<DbPool>,
    Path(asset_id): Path<String>,
    Json(input): Json<InstallRequest>,
) -> Result<Json<serde_json::Value>, AppError> {
    let conn = db.lock().unwrap();
    let device_id = resolve_device_id(&conn, input.device_id.as_deref())?;

    // Verify asset exists
    conn.query_row(
        "SELECT id FROM shared_assets WHERE id = ?1",
        [&asset_id],
        |row| row.get::<_, String>(0),
    ).map_err(|_| AppError::NotFound(format!("Asset '{}' not found", asset_id)))?;

    conn.execute(
        "INSERT OR IGNORE INTO shared_asset_installs (asset_id, device_id) VALUES (?1, ?2)",
        rusqlite::params![asset_id, device_id],
    )?;

    conn.execute(
        "UPDATE shared_assets SET downloads = downloads + 1 WHERE id = ?1",
        [&asset_id],
    )?;

    Ok(Json(serde_json::json!({ "ok": true, "asset_id": asset_id })))
}

/// DELETE /api/community/assets/:id/install — uninstall a shared asset
pub async fn uninstall_asset(
    State(db): State<DbPool>,
    Path(asset_id): Path<String>,
    Query(q): Query<InstallRequest>,
) -> Result<Json<serde_json::Value>, AppError> {
    let conn = db.lock().unwrap();
    let device_id = resolve_device_id(&conn, q.device_id.as_deref())?;

    conn.execute(
        "DELETE FROM shared_asset_installs WHERE asset_id = ?1 AND device_id = ?2",
        rusqlite::params![asset_id, device_id],
    )?;

    Ok(Json(serde_json::json!({ "ok": true })))
}

/// POST /api/community/assets/:id/rate — rate a shared asset
pub async fn rate_asset(
    State(db): State<DbPool>,
    Path(asset_id): Path<String>,
    Json(input): Json<RateRequest>,
) -> Result<Json<serde_json::Value>, AppError> {
    let conn = db.lock().unwrap();
    let device_id = resolve_device_id(&conn, input.device_id.as_deref())?;

    if input.stars < 1 || input.stars > 5 {
        return Err(AppError::BadRequest("Stars must be 1-5".into()));
    }

    let existing: Option<i32> = conn.query_row(
        "SELECT stars FROM shared_asset_ratings WHERE asset_id = ?1 AND device_id = ?2",
        rusqlite::params![asset_id, device_id],
        |row| row.get(0),
    ).ok();

    if let Some(old_stars) = existing {
        conn.execute(
            "UPDATE shared_asset_ratings SET stars = ?1, rated_at = datetime('now') WHERE asset_id = ?2 AND device_id = ?3",
            rusqlite::params![input.stars, asset_id, device_id],
        )?;
        conn.execute(
            "UPDATE shared_assets SET rating_sum = rating_sum - ?1 + ?2 WHERE id = ?3",
            rusqlite::params![old_stars, input.stars, asset_id],
        )?;
    } else {
        conn.execute(
            "INSERT INTO shared_asset_ratings (asset_id, device_id, stars) VALUES (?1, ?2, ?3)",
            rusqlite::params![asset_id, device_id, input.stars],
        )?;
        conn.execute(
            "UPDATE shared_assets SET rating_sum = rating_sum + ?1, rating_count = rating_count + 1 WHERE id = ?2",
            rusqlite::params![input.stars, asset_id],
        )?;
    }

    Ok(Json(serde_json::json!({ "ok": true, "stars": input.stars })))
}
