use axum::extract::{Query, State};
use axum::Json;
use serde::{Deserialize, Serialize};

use crate::db::DbPool;
use crate::error::AppError;

// ── Pet state ─────────────────────────────────────────────────────

#[derive(Debug, Serialize)]
pub struct PetState {
    pub device_id: String,
    pub hunger: i64,        // 0=starving, 100=full
    pub happiness: i64,     // 0=miserable, 100=ecstatic
    pub last_fed_at: Option<String>,
    pub last_pet_at: Option<String>,
    pub total_feeds: i64,
    pub total_pets: i64,
    pub mood: String,       // derived from hunger+happiness
}

fn mood_from(hunger: i64, happiness: i64) -> &'static str {
    let avg = (hunger + happiness) / 2;
    match avg {
        90..=100 => "ecstatic",
        70..=89 => "happy",
        50..=69 => "content",
        30..=49 => "grumpy",
        10..=29 => "sad",
        _ => "miserable",
    }
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

/// GET /api/pet — get pet state (hunger decays over time)
pub async fn get_pet_state(
    State(db): State<DbPool>,
    Query(q): Query<DeviceQuery>,
) -> Result<Json<PetState>, AppError> {
    let conn = db.lock().unwrap();
    let device_id = resolve_device_id(&conn, q.device_id.as_deref())?;

    // Ensure row exists
    conn.execute(
        "INSERT OR IGNORE INTO pet_state (device_id) VALUES (?1)",
        [&device_id],
    )?;

    let (hunger, happiness, last_fed_at, last_pet_at, total_feeds, total_pets): (i64, i64, Option<String>, Option<String>, i64, i64) =
        conn.query_row(
            "SELECT hunger, happiness, last_fed_at, last_pet_at, total_feeds, total_pets FROM pet_state WHERE device_id = ?1",
            [&device_id],
            |row| Ok((row.get(0)?, row.get(1)?, row.get(2)?, row.get(3)?, row.get(4)?, row.get(5)?)),
        )?;

    // Decay hunger based on time since last fed (lose 1 point per 10 minutes)
    let decayed_hunger = if let Some(ref fed_at) = last_fed_at {
        let mins_since: i64 = conn.query_row(
            "SELECT CAST((julianday('now') - julianday(?1)) * 24 * 60 AS INTEGER)",
            [fed_at],
            |row| row.get(0),
        ).unwrap_or(0);
        (hunger - mins_since / 10).max(0)
    } else {
        0
    };

    // Decay happiness (lose 1 point per 30 minutes since last pet)
    let decayed_happiness = if let Some(ref pet_at) = last_pet_at {
        let mins_since: i64 = conn.query_row(
            "SELECT CAST((julianday('now') - julianday(?1)) * 24 * 60 AS INTEGER)",
            [pet_at],
            |row| row.get(0),
        ).unwrap_or(0);
        (happiness - mins_since / 30).max(0)
    } else {
        50 // Default happiness
    };

    let mood = mood_from(decayed_hunger, decayed_happiness).to_string();

    Ok(Json(PetState {
        device_id,
        hunger: decayed_hunger,
        happiness: decayed_happiness,
        last_fed_at,
        last_pet_at,
        total_feeds,
        total_pets,
        mood,
    }))
}

#[derive(Debug, Deserialize)]
pub struct FeedRequest {
    pub device_id: Option<String>,
    pub food_type: Option<String>, // "snack" (+15), "meal" (+35), "feast" (+60)
}

#[derive(Debug, Serialize)]
pub struct FeedResponse {
    pub ok: bool,
    pub hunger: i64,
    pub happiness: i64,
    pub mood: String,
    pub message: String,
}

/// POST /api/pet/feed — feed the bot
pub async fn feed_pet(
    State(db): State<DbPool>,
    Json(input): Json<FeedRequest>,
) -> Result<Json<FeedResponse>, AppError> {
    let conn = db.lock().unwrap();
    let device_id = resolve_device_id(&conn, input.device_id.as_deref())?;

    conn.execute(
        "INSERT OR IGNORE INTO pet_state (device_id) VALUES (?1)",
        [&device_id],
    )?;

    let amount = match input.food_type.as_deref() {
        Some("feast") => 60,
        Some("meal") => 35,
        _ => 15, // snack
    };

    let happiness_boost = amount / 3;
    let food_name = input.food_type.as_deref().unwrap_or("snack");

    conn.execute(
        "UPDATE pet_state SET hunger = MIN(100, hunger + ?1), happiness = MIN(100, happiness + ?2), last_fed_at = datetime('now'), total_feeds = total_feeds + 1 WHERE device_id = ?3",
        rusqlite::params![amount, happiness_boost, device_id],
    )?;

    let (hunger, happiness): (i64, i64) = conn.query_row(
        "SELECT hunger, happiness FROM pet_state WHERE device_id = ?1",
        [&device_id],
        |row| Ok((row.get(0)?, row.get(1)?)),
    )?;

    let mood = mood_from(hunger, happiness).to_string();
    let message = match food_name {
        "feast" => "Om nom nom! That was amazing!".to_string(),
        "meal" => "Mmm, delicious! Thanks!".to_string(),
        _ => "Tasty snack!".to_string(),
    };

    Ok(Json(FeedResponse { ok: true, hunger, happiness, mood, message }))
}

#[derive(Debug, Deserialize)]
pub struct PetPetRequest {
    pub device_id: Option<String>,
}

/// POST /api/pet/pet — pet the bot (increases happiness)
pub async fn pet_pet(
    State(db): State<DbPool>,
    Json(input): Json<PetPetRequest>,
) -> Result<Json<FeedResponse>, AppError> {
    let conn = db.lock().unwrap();
    let device_id = resolve_device_id(&conn, input.device_id.as_deref())?;

    conn.execute(
        "INSERT OR IGNORE INTO pet_state (device_id) VALUES (?1)",
        [&device_id],
    )?;

    conn.execute(
        "UPDATE pet_state SET happiness = MIN(100, happiness + 20), last_pet_at = datetime('now'), total_pets = total_pets + 1 WHERE device_id = ?1",
        [&device_id],
    )?;

    let (hunger, happiness): (i64, i64) = conn.query_row(
        "SELECT hunger, happiness FROM pet_state WHERE device_id = ?1",
        [&device_id],
        |row| Ok((row.get(0)?, row.get(1)?)),
    )?;

    let mood = mood_from(hunger, happiness).to_string();

    Ok(Json(FeedResponse { ok: true, hunger, happiness, mood, message: "Purrs happily!".to_string() }))
}

// ── Token usage tracking ──────────────────────────────────────────

#[derive(Debug, Serialize)]
pub struct TokenUsageEntry {
    pub id: i64,
    pub device_id: String,
    pub input_tokens: i64,
    pub output_tokens: i64,
    pub model: String,
    pub recorded_at: String,
}

#[derive(Debug, Serialize)]
pub struct TokenUsageSummary {
    pub total_input: i64,
    pub total_output: i64,
    pub total_tokens: i64,
    pub today_input: i64,
    pub today_output: i64,
    pub today_total: i64,
    pub entries_count: i64,
    pub recent: Vec<TokenUsageEntry>,
    pub daily: Vec<DailyTokenUsage>,
}

#[derive(Debug, Serialize)]
pub struct DailyTokenUsage {
    pub date: String,
    pub input_tokens: i64,
    pub output_tokens: i64,
}

#[derive(Debug, Deserialize)]
pub struct TokenQuery {
    pub device_id: Option<String>,
    pub days: Option<i64>,
}

/// GET /api/pet/tokens — get token usage summary
pub async fn get_token_usage(
    State(db): State<DbPool>,
    Query(q): Query<TokenQuery>,
) -> Result<Json<TokenUsageSummary>, AppError> {
    let conn = db.lock().unwrap();
    let device_id = resolve_device_id(&conn, q.device_id.as_deref())?;
    let days = q.days.unwrap_or(30);

    let (total_input, total_output): (i64, i64) = conn.query_row(
        "SELECT COALESCE(SUM(input_tokens), 0), COALESCE(SUM(output_tokens), 0) FROM token_usage WHERE device_id = ?1",
        [&device_id],
        |row| Ok((row.get(0)?, row.get(1)?)),
    )?;

    let (today_input, today_output): (i64, i64) = conn.query_row(
        "SELECT COALESCE(SUM(input_tokens), 0), COALESCE(SUM(output_tokens), 0) FROM token_usage WHERE device_id = ?1 AND date(recorded_at) = date('now')",
        [&device_id],
        |row| Ok((row.get(0)?, row.get(1)?)),
    )?;

    let entries_count: i64 = conn.query_row(
        "SELECT COUNT(*) FROM token_usage WHERE device_id = ?1",
        [&device_id],
        |row| row.get(0),
    )?;

    // Recent entries
    let recent = {
        let mut stmt = conn.prepare(
            "SELECT id, device_id, input_tokens, output_tokens, model, recorded_at FROM token_usage WHERE device_id = ?1 ORDER BY recorded_at DESC LIMIT 20",
        )?;
        let rows = stmt.query_map([&device_id], |row| {
            Ok(TokenUsageEntry {
                id: row.get(0)?,
                device_id: row.get(1)?,
                input_tokens: row.get(2)?,
                output_tokens: row.get(3)?,
                model: row.get(4)?,
                recorded_at: row.get(5)?,
            })
        })?;
        rows.filter_map(|r| r.ok()).collect()
    };

    // Daily aggregation
    let daily = {
        let mut stmt = conn.prepare(
            "SELECT date(recorded_at) as d, SUM(input_tokens), SUM(output_tokens) FROM token_usage WHERE device_id = ?1 AND recorded_at >= datetime('now', ?2) GROUP BY d ORDER BY d",
        )?;
        let days_param = format!("-{} days", days);
        let rows = stmt.query_map(rusqlite::params![device_id, days_param], |row| {
            Ok(DailyTokenUsage {
                date: row.get(0)?,
                input_tokens: row.get(1)?,
                output_tokens: row.get(2)?,
            })
        })?;
        rows.filter_map(|r| r.ok()).collect()
    };

    Ok(Json(TokenUsageSummary {
        total_input,
        total_output,
        total_tokens: total_input + total_output,
        today_input,
        today_output,
        today_total: today_input + today_output,
        entries_count,
        recent,
        daily,
    }))
}

#[derive(Debug, Deserialize)]
pub struct RecordTokensRequest {
    pub device_id: Option<String>,
    pub input_tokens: i64,
    pub output_tokens: i64,
    pub model: Option<String>,
}

/// POST /api/pet/tokens — record token usage
pub async fn record_token_usage(
    State(db): State<DbPool>,
    Json(input): Json<RecordTokensRequest>,
) -> Result<Json<serde_json::Value>, AppError> {
    let conn = db.lock().unwrap();
    let device_id = resolve_device_id(&conn, input.device_id.as_deref())?;
    let model = input.model.as_deref().unwrap_or("unknown");

    conn.execute(
        "INSERT INTO token_usage (device_id, input_tokens, output_tokens, model) VALUES (?1, ?2, ?3, ?4)",
        rusqlite::params![device_id, input.input_tokens, input.output_tokens, model],
    )?;

    // Feeding the bot with tokens also increases hunger slightly (it eats tokens!)
    conn.execute(
        "INSERT OR IGNORE INTO pet_state (device_id) VALUES (?1)",
        [&device_id],
    )?;
    let token_food = ((input.input_tokens + input.output_tokens) / 500).min(10) as i64;
    if token_food > 0 {
        conn.execute(
            "UPDATE pet_state SET hunger = MIN(100, hunger + ?1) WHERE device_id = ?2",
            rusqlite::params![token_food, device_id],
        )?;
    }

    Ok(Json(serde_json::json!({ "ok": true })))
}
