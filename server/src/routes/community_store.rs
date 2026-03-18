use axum::extract::{Path, Query, State};
use axum::Json;
use serde::{Deserialize, Serialize};

use crate::db::DbPool;
use crate::error::AppError;
use crate::models::{VerifiedPublisher, CreateVerifiedPublisher};

// ── Types ─────────────────────────────────────────────────────────

#[derive(Debug, Serialize)]
pub struct CommunityPlugin {
    pub id: String,
    pub name: String,
    pub description: String,
    pub author: String,
    pub version: String,
    pub category: String,
    pub tags: Vec<String>,
    pub payload: serde_json::Value,
    pub downloads: i64,
    pub rating_avg: f64,
    pub rating_count: i64,
    pub installed: bool,
    pub verified: bool,
    pub created_at: String,
    pub updated_at: String,
}

#[derive(Debug, Deserialize)]
pub struct ListQuery {
    pub device_id: Option<String>,
    pub category: Option<String>,
    pub search: Option<String>,
    pub sort: Option<String>, // "popular", "newest", "rating"
}

#[derive(Debug, Deserialize)]
pub struct PublishRequest {
    pub name: String,
    pub description: String,
    pub author: Option<String>,
    pub version: Option<String>,
    pub category: Option<String>,
    pub tags: Option<Vec<String>>,
    pub payload: Option<serde_json::Value>,
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

/// GET /api/community/plugins — list community plugins
pub async fn list_plugins(
    State(db): State<DbPool>,
    Query(q): Query<ListQuery>,
) -> Result<Json<Vec<CommunityPlugin>>, AppError> {
    let conn = db.lock().unwrap();
    let device_id = resolve_device_id(&conn, q.device_id.as_deref()).ok();

    let order = match q.sort.as_deref() {
        Some("popular") => "p.downloads DESC",
        Some("rating") => "(CASE WHEN p.rating_count = 0 THEN 0 ELSE CAST(p.rating_sum AS REAL) / p.rating_count END) DESC",
        Some("verified") => "verified DESC, p.created_at DESC",
        Some("newest") | _ => "p.created_at DESC",
    };

    let sql = format!(
        "SELECT p.id, p.name, p.description, p.author, p.version, p.category, p.tags, \
         p.payload, p.downloads, p.rating_sum, p.rating_count, p.created_at, p.updated_at, \
         CASE WHEN vp.id IS NOT NULL THEN 1 ELSE p.verified END AS verified \
         FROM community_plugins p \
         LEFT JOIN verified_publishers vp ON p.author = vp.name \
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
            row.get::<_, String>(6)?,
            row.get::<_, String>(7)?,
            row.get::<_, i64>(8)?,
            row.get::<_, i64>(9)?,
            row.get::<_, i64>(10)?,
            row.get::<_, String>(11)?,
            row.get::<_, String>(12)?,
            row.get::<_, bool>(13)?,
        ))
    })?;

    let mut plugins = Vec::new();
    for row in rows {
        let (id, name, description, author, version, category, tags_json, payload_json,
             downloads, rating_sum, rating_count, created_at, updated_at, verified) = row?;

        // Filter by category
        if let Some(ref cat) = q.category {
            if cat != "all" && &category != cat {
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
                "SELECT COUNT(*) FROM community_plugin_installs WHERE plugin_id = ?1 AND device_id = ?2",
                rusqlite::params![id, did],
                |row| row.get::<_, i32>(0),
            ).map(|c| c > 0).unwrap_or(false)
        } else {
            false
        };

        let tags: Vec<String> = serde_json::from_str(&tags_json).unwrap_or_default();
        let payload: serde_json::Value = serde_json::from_str(&payload_json).unwrap_or(serde_json::Value::Object(Default::default()));
        let rating_avg = if rating_count > 0 { rating_sum as f64 / rating_count as f64 } else { 0.0 };

        plugins.push(CommunityPlugin {
            id, name, description, author, version, category, tags, payload,
            downloads, rating_avg, rating_count, installed, verified, created_at, updated_at,
        });
    }

    Ok(Json(plugins))
}

/// POST /api/community/plugins — publish a new plugin
pub async fn publish_plugin(
    State(db): State<DbPool>,
    Json(input): Json<PublishRequest>,
) -> Result<Json<CommunityPlugin>, AppError> {
    let conn = db.lock().unwrap();
    let id = uuid::Uuid::new_v4().to_string();
    let author = input.author.unwrap_or_else(|| "anonymous".to_string());
    let version = input.version.unwrap_or_else(|| "1.0.0".to_string());
    let category = input.category.unwrap_or_else(|| "utility".to_string());
    let tags = input.tags.unwrap_or_default();
    let tags_json = serde_json::to_string(&tags).unwrap_or_else(|_| "[]".to_string());
    let payload = input.payload.unwrap_or(serde_json::Value::Object(Default::default()));
    let payload_json = serde_json::to_string(&payload).unwrap_or_else(|_| "{}".to_string());

    let verified = is_verified_publisher(&conn, &author);

    conn.execute(
        "INSERT INTO community_plugins (id, name, description, author, version, category, tags, payload, verified) \
         VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, ?9)",
        rusqlite::params![id, input.name, input.description, author, version, category, tags_json, payload_json, verified],
    )?;

    Ok(Json(CommunityPlugin {
        id,
        name: input.name,
        description: input.description,
        author,
        version,
        category,
        tags,
        payload,
        downloads: 0,
        rating_avg: 0.0,
        rating_count: 0,
        installed: false,
        verified,
        created_at: "just now".into(),
        updated_at: "just now".into(),
    }))
}

/// POST /api/community/plugins/:id/install — install a plugin
pub async fn install_plugin(
    State(db): State<DbPool>,
    Path(plugin_id): Path<String>,
    Json(input): Json<InstallRequest>,
) -> Result<Json<serde_json::Value>, AppError> {
    let conn = db.lock().unwrap();
    let device_id = resolve_device_id(&conn, input.device_id.as_deref())?;

    // Verify plugin exists
    conn.query_row(
        "SELECT id FROM community_plugins WHERE id = ?1",
        [&plugin_id],
        |row| row.get::<_, String>(0),
    ).map_err(|_| AppError::NotFound(format!("Plugin '{}' not found", plugin_id)))?;

    conn.execute(
        "INSERT OR IGNORE INTO community_plugin_installs (plugin_id, device_id) VALUES (?1, ?2)",
        rusqlite::params![plugin_id, device_id],
    )?;

    conn.execute(
        "UPDATE community_plugins SET downloads = downloads + 1 WHERE id = ?1",
        [&plugin_id],
    )?;

    Ok(Json(serde_json::json!({ "ok": true, "plugin_id": plugin_id })))
}

/// DELETE /api/community/plugins/:id/install — uninstall a plugin
pub async fn uninstall_plugin(
    State(db): State<DbPool>,
    Path(plugin_id): Path<String>,
    Query(q): Query<InstallRequest>,
) -> Result<Json<serde_json::Value>, AppError> {
    let conn = db.lock().unwrap();
    let device_id = resolve_device_id(&conn, q.device_id.as_deref())?;

    conn.execute(
        "DELETE FROM community_plugin_installs WHERE plugin_id = ?1 AND device_id = ?2",
        rusqlite::params![plugin_id, device_id],
    )?;

    Ok(Json(serde_json::json!({ "ok": true })))
}

/// POST /api/community/plugins/:id/rate — rate a plugin
pub async fn rate_plugin(
    State(db): State<DbPool>,
    Path(plugin_id): Path<String>,
    Json(input): Json<RateRequest>,
) -> Result<Json<serde_json::Value>, AppError> {
    let conn = db.lock().unwrap();
    let device_id = resolve_device_id(&conn, input.device_id.as_deref())?;

    if input.stars < 1 || input.stars > 5 {
        return Err(AppError::BadRequest("Stars must be 1-5".into()));
    }

    // Check for existing rating
    let existing: Option<i32> = conn.query_row(
        "SELECT stars FROM community_plugin_ratings WHERE plugin_id = ?1 AND device_id = ?2",
        rusqlite::params![plugin_id, device_id],
        |row| row.get(0),
    ).ok();

    if let Some(old_stars) = existing {
        // Update existing rating
        conn.execute(
            "UPDATE community_plugin_ratings SET stars = ?1, rated_at = datetime('now') WHERE plugin_id = ?2 AND device_id = ?3",
            rusqlite::params![input.stars, plugin_id, device_id],
        )?;
        conn.execute(
            "UPDATE community_plugins SET rating_sum = rating_sum - ?1 + ?2 WHERE id = ?3",
            rusqlite::params![old_stars, input.stars, plugin_id],
        )?;
    } else {
        // New rating
        conn.execute(
            "INSERT INTO community_plugin_ratings (plugin_id, device_id, stars) VALUES (?1, ?2, ?3)",
            rusqlite::params![plugin_id, device_id, input.stars],
        )?;
        conn.execute(
            "UPDATE community_plugins SET rating_sum = rating_sum + ?1, rating_count = rating_count + 1 WHERE id = ?2",
            rusqlite::params![input.stars, plugin_id],
        )?;
    }

    Ok(Json(serde_json::json!({ "ok": true, "stars": input.stars })))
}

// ── Verified Publisher Handlers ───────────────────────────────────

/// GET /api/community/publishers — list verified publishers
pub async fn list_publishers(
    State(db): State<DbPool>,
) -> Result<Json<Vec<VerifiedPublisher>>, AppError> {
    let conn = db.lock().unwrap();
    let mut stmt = conn.prepare(
        "SELECT id, name, display_name, badge_type, verified_at, verified_by FROM verified_publishers ORDER BY verified_at DESC"
    )?;
    let rows = stmt.query_map([], |row| {
        Ok(VerifiedPublisher {
            id: row.get(0)?,
            name: row.get(1)?,
            display_name: row.get(2)?,
            badge_type: row.get(3)?,
            verified_at: row.get(4)?,
            verified_by: row.get(5)?,
        })
    })?;

    let publishers: Vec<VerifiedPublisher> = rows.filter_map(|r| r.ok()).collect();
    Ok(Json(publishers))
}

/// POST /api/community/publishers — add a verified publisher
pub async fn add_publisher(
    State(db): State<DbPool>,
    Json(input): Json<CreateVerifiedPublisher>,
) -> Result<Json<VerifiedPublisher>, AppError> {
    let conn = db.lock().unwrap();
    let id = uuid::Uuid::new_v4().to_string();
    let badge_type = input.badge_type.unwrap_or_else(|| "verified".to_string());

    conn.execute(
        "INSERT INTO verified_publishers (id, name, display_name, badge_type) VALUES (?1, ?2, ?3, ?4)",
        rusqlite::params![id, input.name, input.display_name, badge_type],
    )?;

    // Update existing plugins/assets by this author to verified
    conn.execute(
        "UPDATE community_plugins SET verified = 1 WHERE author = ?1",
        [&input.name],
    )?;
    conn.execute(
        "UPDATE shared_assets SET verified = 1 WHERE author = ?1",
        [&input.name],
    )?;

    let publisher = conn.query_row(
        "SELECT id, name, display_name, badge_type, verified_at, verified_by FROM verified_publishers WHERE id = ?1",
        [&id],
        |row| Ok(VerifiedPublisher {
            id: row.get(0)?,
            name: row.get(1)?,
            display_name: row.get(2)?,
            badge_type: row.get(3)?,
            verified_at: row.get(4)?,
            verified_by: row.get(5)?,
        }),
    )?;

    Ok(Json(publisher))
}

/// DELETE /api/community/publishers/:id — remove a verified publisher
pub async fn remove_publisher(
    State(db): State<DbPool>,
    Path(publisher_id): Path<String>,
) -> Result<Json<serde_json::Value>, AppError> {
    let conn = db.lock().unwrap();

    // Get the publisher name before deleting so we can un-verify their content
    let name: String = conn.query_row(
        "SELECT name FROM verified_publishers WHERE id = ?1",
        [&publisher_id],
        |row| row.get(0),
    ).map_err(|_| AppError::NotFound(format!("Publisher '{}' not found", publisher_id)))?;

    conn.execute(
        "DELETE FROM verified_publishers WHERE id = ?1",
        [&publisher_id],
    )?;

    // Un-verify their plugins and assets
    conn.execute(
        "UPDATE community_plugins SET verified = 0 WHERE author = ?1",
        [&name],
    )?;
    conn.execute(
        "UPDATE shared_assets SET verified = 0 WHERE author = ?1",
        [&name],
    )?;

    Ok(Json(serde_json::json!({ "ok": true })))
}
