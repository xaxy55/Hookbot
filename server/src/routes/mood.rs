use axum::extract::{Query, State};
use axum::Json;
use serde::{Deserialize, Serialize};

use crate::db::DbPool;
use crate::error::AppError;

fn resolve_device_id(conn: &rusqlite::Connection, device_id: Option<&str>) -> Result<String, AppError> {
    if let Some(id) = device_id {
        return Ok(id.to_string());
    }
    conn.query_row("SELECT id FROM devices ORDER BY created_at LIMIT 1", [], |row| row.get(0))
        .map_err(|_| AppError::NotFound("No devices registered".into()))
}

// ── Types ────────────────────────────────────────────────────────

#[derive(Debug, Serialize)]
pub struct MoodEntry {
    pub id: i64,
    pub device_id: String,
    pub mood: String,
    pub note: Option<String>,
    pub energy: i64,
    pub created_at: String,
}

#[derive(Debug, Serialize)]
pub struct MoodStats {
    pub total_entries: i64,
    pub this_week: i64,
    pub avg_energy: f64,
    pub most_common_mood: Option<String>,
    pub mood_distribution: Vec<MoodCount>,
}

#[derive(Debug, Serialize)]
pub struct MoodCount {
    pub mood: String,
    pub count: i64,
}

// ── Query params ─────────────────────────────────────────────────

#[derive(Debug, Deserialize)]
pub struct MoodQuery {
    pub device_id: Option<String>,
    pub days: Option<i64>,
}

#[derive(Debug, Deserialize)]
pub struct DeviceQuery {
    pub device_id: Option<String>,
}

#[derive(Debug, Deserialize)]
pub struct CreateMoodRequest {
    pub device_id: Option<String>,
    pub mood: String,
    pub note: Option<String>,
    pub energy: Option<i64>,
}

// ── Handlers ─────────────────────────────────────────────────────

/// GET /api/mood — list mood journal entries
pub async fn get_entries(
    State(db): State<DbPool>,
    Query(q): Query<MoodQuery>,
) -> Result<Json<Vec<MoodEntry>>, AppError> {
    let conn = db.lock().unwrap();
    let device_id = resolve_device_id(&conn, q.device_id.as_deref())?;
    let days = q.days.unwrap_or(30);
    let days_param = format!("-{} days", days);

    let entries = {
        let mut stmt = conn.prepare(
            "SELECT id, device_id, mood, note, energy, created_at \
             FROM mood_journal \
             WHERE device_id = ?1 AND created_at >= datetime('now', ?2) \
             ORDER BY created_at DESC",
        )?;
        let rows = stmt.query_map(rusqlite::params![device_id, days_param], |row| {
            Ok(MoodEntry {
                id: row.get(0)?,
                device_id: row.get(1)?,
                mood: row.get(2)?,
                note: row.get(3)?,
                energy: row.get(4)?,
                created_at: row.get(5)?,
            })
        })?;
        rows.filter_map(|r| r.ok()).collect::<Vec<_>>()
    };

    Ok(Json(entries))
}

/// POST /api/mood — create a mood journal entry
pub async fn create_entry(
    State(db): State<DbPool>,
    Json(input): Json<CreateMoodRequest>,
) -> Result<Json<MoodEntry>, AppError> {
    let conn = db.lock().unwrap();
    let device_id = resolve_device_id(&conn, input.device_id.as_deref())?;
    let energy = input.energy.unwrap_or(3).max(1).min(5);

    conn.execute(
        "INSERT INTO mood_journal (device_id, mood, note, energy) VALUES (?1, ?2, ?3, ?4)",
        rusqlite::params![device_id, input.mood, input.note, energy],
    )?;

    let id = conn.last_insert_rowid();

    let entry = conn.query_row(
        "SELECT id, device_id, mood, note, energy, created_at FROM mood_journal WHERE id = ?1",
        [id],
        |row| {
            Ok(MoodEntry {
                id: row.get(0)?,
                device_id: row.get(1)?,
                mood: row.get(2)?,
                note: row.get(3)?,
                energy: row.get(4)?,
                created_at: row.get(5)?,
            })
        },
    )?;

    Ok(Json(entry))
}

/// GET /api/mood/stats — mood statistics
pub async fn get_stats(
    State(db): State<DbPool>,
    Query(q): Query<DeviceQuery>,
) -> Result<Json<MoodStats>, AppError> {
    let conn = db.lock().unwrap();
    let device_id = resolve_device_id(&conn, q.device_id.as_deref())?;

    let total_entries: i64 = conn.query_row(
        "SELECT COUNT(*) FROM mood_journal WHERE device_id = ?1",
        [&device_id],
        |row| row.get(0),
    )?;

    let this_week: i64 = conn.query_row(
        "SELECT COUNT(*) FROM mood_journal WHERE device_id = ?1 AND created_at >= datetime('now', '-7 days')",
        [&device_id],
        |row| row.get(0),
    )?;

    let avg_energy: f64 = conn.query_row(
        "SELECT COALESCE(AVG(energy), 0.0) FROM mood_journal WHERE device_id = ?1",
        [&device_id],
        |row| row.get(0),
    )?;

    let most_common_mood: Option<String> = conn.query_row(
        "SELECT mood FROM mood_journal WHERE device_id = ?1 GROUP BY mood ORDER BY COUNT(*) DESC LIMIT 1",
        [&device_id],
        |row| row.get(0),
    ).ok();

    let mood_distribution = {
        let mut stmt = conn.prepare(
            "SELECT mood, COUNT(*) as cnt FROM mood_journal WHERE device_id = ?1 GROUP BY mood ORDER BY cnt DESC",
        )?;
        let rows = stmt.query_map([&device_id], |row| {
            Ok(MoodCount {
                mood: row.get(0)?,
                count: row.get(1)?,
            })
        })?;
        rows.filter_map(|r| r.ok()).collect::<Vec<_>>()
    };

    Ok(Json(MoodStats {
        total_entries,
        this_week,
        avg_energy,
        most_common_mood,
        mood_distribution,
    }))
}
