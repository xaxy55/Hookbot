use axum::extract::{Query, State};
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

/// POST /api/mood/feedback — record user feedback on a state/animation
pub async fn record_feedback(
    State(db): State<DbPool>,
    Json(input): Json<RecordMoodFeedback>,
) -> Result<Json<serde_json::Value>, AppError> {
    let conn = db.lock().unwrap();
    let device_id = resolve_device_id(&conn, input.device_id.as_deref())?;
    let duration = input.duration_secs.unwrap_or(0);

    let anim_id_param = input.animation_id.as_deref().unwrap_or("");

    // Upsert preference record
    let existing = conn.query_row(
        "SELECT id FROM mood_preferences WHERE device_id = ?1 AND state = ?2 AND COALESCE(animation_id, '') = ?3",
        rusqlite::params![device_id, input.state, anim_id_param],
        |row| row.get::<_, i64>(0),
    );

    match existing {
        Ok(id) => {
            let (pos_col, neg_col) = match input.feedback.as_str() {
                "positive" => ("positive_responses = positive_responses + 1", "positive_responses"),
                _ => ("negative_responses = negative_responses + 1", "negative_responses"),
            };
            let _ = neg_col; // avoid unused warning
            conn.execute(
                &format!(
                    "UPDATE mood_preferences SET {pos_col}, total_duration_secs = total_duration_secs + ?1, \
                     last_shown_at = datetime('now'), updated_at = datetime('now') WHERE id = ?2"
                ),
                rusqlite::params![duration, id],
            )?;
        }
        Err(_) => {
            let (pos, neg) = match input.feedback.as_str() {
                "positive" => (1, 0),
                _ => (0, 1),
            };
            conn.execute(
                "INSERT INTO mood_preferences (device_id, state, animation_id, positive_responses, negative_responses, \
                 total_duration_secs, last_shown_at) VALUES (?1, ?2, ?3, ?4, ?5, ?6, datetime('now'))",
                rusqlite::params![device_id, input.state, input.animation_id, pos, neg, duration],
            )?;
        }
    }

    // Update mood patterns for current time slot
    let now = chrono::Local::now();
    let hour = now.hour() as i64;
    let dow = now.weekday().num_days_from_monday() as i64;

    if input.feedback == "positive" {
        conn.execute(
            "INSERT INTO mood_patterns (device_id, hour_of_day, day_of_week, preferred_state, preferred_animation, \
             confidence, sample_count) VALUES (?1, ?2, ?3, ?4, ?5, 0.5, 1) \
             ON CONFLICT(device_id, hour_of_day, day_of_week) DO UPDATE SET \
             preferred_state = CASE WHEN sample_count > 5 THEN preferred_state ELSE excluded.preferred_state END, \
             preferred_animation = CASE WHEN sample_count > 5 THEN preferred_animation ELSE excluded.preferred_animation END, \
             sample_count = sample_count + 1, \
             confidence = MIN(1.0, confidence + 0.05), \
             updated_at = datetime('now')",
            rusqlite::params![device_id, hour, dow, input.state, input.animation_id],
        )?;
    }

    Ok(Json(serde_json::json!({ "ok": true })))
}

/// GET /api/mood/preferences — get learned preferences for a device
pub async fn get_preferences(
    State(db): State<DbPool>,
    Query(q): Query<DeviceQuery>,
) -> Result<Json<Vec<MoodPreference>>, AppError> {
    let conn = db.lock().unwrap();
    let device_id = resolve_device_id(&conn, q.device_id.as_deref())?;

    let mut stmt = conn.prepare(
        "SELECT id, device_id, state, animation_id, positive_responses, negative_responses, \
         total_duration_secs, last_shown_at \
         FROM mood_preferences WHERE device_id = ?1 \
         ORDER BY (positive_responses - negative_responses) DESC"
    )?;
    let prefs = stmt.query_map([&device_id], |row| {
        let pos: i64 = row.get(4)?;
        let neg: i64 = row.get(5)?;
        let total = pos + neg;
        let score = if total > 0 { pos as f64 / total as f64 } else { 0.5 };
        Ok(MoodPreference {
            id: row.get(0)?,
            device_id: row.get(1)?,
            state: row.get(2)?,
            animation_id: row.get(3)?,
            positive_responses: pos,
            negative_responses: neg,
            total_duration_secs: row.get(6)?,
            score,
            last_shown_at: row.get(7)?,
        })
    })?.filter_map(|r| r.ok()).collect::<Vec<_>>();

    Ok(Json(prefs))
}

/// GET /api/mood/patterns — get time-based mood patterns
pub async fn get_patterns(
    State(db): State<DbPool>,
    Query(q): Query<DeviceQuery>,
) -> Result<Json<Vec<MoodPattern>>, AppError> {
    let conn = db.lock().unwrap();
    let device_id = resolve_device_id(&conn, q.device_id.as_deref())?;

    let mut stmt = conn.prepare(
        "SELECT device_id, hour_of_day, day_of_week, preferred_state, preferred_animation, \
         confidence, sample_count \
         FROM mood_patterns WHERE device_id = ?1 \
         ORDER BY day_of_week, hour_of_day"
    )?;
    let patterns = stmt.query_map([&device_id], |row| {
        Ok(MoodPattern {
            device_id: row.get(0)?,
            hour_of_day: row.get(1)?,
            day_of_week: row.get(2)?,
            preferred_state: row.get(3)?,
            preferred_animation: row.get(4)?,
            confidence: row.get(5)?,
            sample_count: row.get(6)?,
        })
    })?.filter_map(|r| r.ok()).collect::<Vec<_>>();

    Ok(Json(patterns))
}

/// GET /api/mood/suggest — get a mood suggestion for the current time
pub async fn get_suggestion(
    State(db): State<DbPool>,
    Query(q): Query<DeviceQuery>,
) -> Result<Json<MoodSuggestion>, AppError> {
    let conn = db.lock().unwrap();
    let device_id = resolve_device_id(&conn, q.device_id.as_deref())?;

    let now = chrono::Local::now();
    let hour = now.hour() as i64;
    let dow = now.weekday().num_days_from_monday() as i64;

    // Try exact time match first
    let pattern = conn.query_row(
        "SELECT preferred_state, preferred_animation, confidence, sample_count \
         FROM mood_patterns WHERE device_id = ?1 AND hour_of_day = ?2 AND day_of_week = ?3",
        rusqlite::params![device_id, hour, dow],
        |row| Ok((
            row.get::<_, Option<String>>(0)?,
            row.get::<_, Option<String>>(1)?,
            row.get::<_, f64>(2)?,
            row.get::<_, i64>(3)?,
        )),
    );

    if let Ok((state, animation, confidence, count)) = pattern {
        if confidence > 0.3 && count >= 3 {
            return Ok(Json(MoodSuggestion {
                suggested_state: state,
                suggested_animation: animation,
                confidence,
                reason: format!("Based on {} observations at this time of day", count),
            }));
        }
    }

    // Fall back to general preference (highest-scoring state)
    let best = conn.query_row(
        "SELECT state, animation_id, \
         CAST(positive_responses AS REAL) / MAX(positive_responses + negative_responses, 1) as score \
         FROM mood_preferences WHERE device_id = ?1 AND positive_responses > negative_responses \
         ORDER BY score DESC, positive_responses DESC LIMIT 1",
        [&device_id],
        |row| Ok((
            row.get::<_, String>(0)?,
            row.get::<_, Option<String>>(1)?,
            row.get::<_, f64>(2)?,
        )),
    );

    match best {
        Ok((state, animation, score)) => Ok(Json(MoodSuggestion {
            suggested_state: Some(state),
            suggested_animation: animation,
            confidence: score * 0.6, // Lower confidence for non-time-based
            reason: "Based on overall preference history".into(),
        })),
        Err(_) => Ok(Json(MoodSuggestion {
            suggested_state: None,
            suggested_animation: None,
            confidence: 0.0,
            reason: "Not enough data to make a suggestion yet".into(),
        })),
    }
}

use chrono::{Timelike, Datelike};
