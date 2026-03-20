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

/// GET /api/desk-occupancy/config — get occupancy config
pub async fn get_config(
    State(db): State<DbPool>,
    Query(q): Query<DeviceQuery>,
) -> Result<Json<DeskOccupancyConfig>, AppError> {
    let conn = db.lock().unwrap();
    let device_id = resolve_device_id(&conn, q.device_id.as_deref())?;

    let config = conn.query_row(
        "SELECT id, device_id, break_remind_minutes, enabled, created_at \
         FROM desk_occupancy_config WHERE device_id = ?1",
        [&device_id],
        |row| Ok(DeskOccupancyConfig {
            id: row.get(0)?,
            device_id: row.get(1)?,
            break_remind_minutes: row.get(2)?,
            enabled: row.get(3)?,
            created_at: row.get(4)?,
        }),
    );

    match config {
        Ok(c) => Ok(Json(c)),
        Err(_) => {
            let id = uuid::Uuid::new_v4().to_string();
            conn.execute(
                "INSERT INTO desk_occupancy_config (id, device_id) VALUES (?1, ?2)",
                rusqlite::params![id, device_id],
            )?;
            Ok(Json(DeskOccupancyConfig {
                id,
                device_id,
                break_remind_minutes: 60,
                enabled: true,
                created_at: chrono::Utc::now().to_rfc3339(),
            }))
        }
    }
}

/// PUT /api/desk-occupancy/config — update occupancy config
pub async fn update_config(
    State(db): State<DbPool>,
    Query(q): Query<DeviceQuery>,
    Json(input): Json<UpdateDeskOccupancyConfig>,
) -> Result<Json<serde_json::Value>, AppError> {
    let conn = db.lock().unwrap();
    let device_id = resolve_device_id(&conn, q.device_id.as_deref())?;

    if let Some(v) = input.break_remind_minutes {
        conn.execute("UPDATE desk_occupancy_config SET break_remind_minutes = ?1 WHERE device_id = ?2",
            rusqlite::params![v, device_id])?;
    }
    if let Some(v) = input.enabled {
        conn.execute("UPDATE desk_occupancy_config SET enabled = ?1 WHERE device_id = ?2",
            rusqlite::params![v, device_id])?;
    }

    Ok(Json(serde_json::json!({ "ok": true })))
}

/// POST /api/desk-occupancy/events — record an occupancy event
pub async fn record_event(
    State(db): State<DbPool>,
    Json(input): Json<RecordOccupancyEvent>,
) -> Result<Json<OccupancyEvent>, AppError> {
    let conn = db.lock().unwrap();
    let device_id = resolve_device_id(&conn, input.device_id.as_deref())?;

    let valid_types = ["occupied", "vacant", "break_start", "break_end"];
    if !valid_types.contains(&input.event_type.as_str()) {
        return Err(AppError::BadRequest(format!(
            "event_type must be one of: {}", valid_types.join(", ")
        )));
    }

    conn.execute(
        "INSERT INTO desk_occupancy_events (device_id, event_type) VALUES (?1, ?2)",
        rusqlite::params![device_id, input.event_type],
    )?;

    let id = conn.last_insert_rowid();

    Ok(Json(OccupancyEvent {
        id,
        device_id,
        event_type: input.event_type,
        created_at: chrono::Utc::now().to_rfc3339(),
    }))
}

/// GET /api/desk-occupancy/events — get recent occupancy events
pub async fn get_events(
    State(db): State<DbPool>,
    Query(q): Query<DeviceQuery>,
) -> Result<Json<Vec<OccupancyEvent>>, AppError> {
    let conn = db.lock().unwrap();
    let device_id = resolve_device_id(&conn, q.device_id.as_deref())?;

    let mut stmt = conn.prepare(
        "SELECT id, device_id, event_type, created_at FROM desk_occupancy_events \
         WHERE device_id = ?1 ORDER BY created_at DESC LIMIT 100"
    )?;
    let events = stmt.query_map([&device_id], |row| {
        Ok(OccupancyEvent {
            id: row.get(0)?,
            device_id: row.get(1)?,
            event_type: row.get(2)?,
            created_at: row.get(3)?,
        })
    })?.filter_map(|r| r.ok()).collect::<Vec<_>>();

    Ok(Json(events))
}

/// GET /api/desk-occupancy/report — get weekly desk health report
pub async fn get_report(
    State(db): State<DbPool>,
    Query(q): Query<DeviceQuery>,
) -> Result<Json<OccupancyReport>, AppError> {
    let conn = db.lock().unwrap();
    let device_id = resolve_device_id(&conn, q.device_id.as_deref())?;

    // Calculate total desk hours and breaks from events
    let mut stmt = conn.prepare(
        "SELECT event_type, created_at FROM desk_occupancy_events \
         WHERE device_id = ?1 AND created_at >= datetime('now', '-7 days') \
         ORDER BY created_at"
    )?;
    let events: Vec<(String, String)> = stmt.query_map([&device_id], |row| {
        Ok((row.get(0)?, row.get(1)?))
    })?.filter_map(|r| r.ok()).collect();

    let mut total_desk_minutes: f64 = 0.0;
    let mut total_break_minutes: f64 = 0.0;
    let mut breaks_taken: i64 = 0;
    let mut session_count: i64 = 0;
    let mut last_occupied_at: Option<chrono::DateTime<chrono::Utc>> = None;
    let mut last_break_at: Option<chrono::DateTime<chrono::Utc>> = None;

    for (etype, ts) in &events {
        if let Ok(t) = chrono::DateTime::parse_from_rfc3339(ts) {
            let t = t.with_timezone(&chrono::Utc);
            match etype.as_str() {
                "occupied" => {
                    session_count += 1;
                    last_occupied_at = Some(t);
                }
                "vacant" => {
                    if let Some(start) = last_occupied_at {
                        total_desk_minutes += (t - start).num_minutes() as f64;
                    }
                    last_occupied_at = None;
                }
                "break_start" => {
                    breaks_taken += 1;
                    last_break_at = Some(t);
                }
                "break_end" => {
                    if let Some(start) = last_break_at {
                        total_break_minutes += (t - start).num_minutes() as f64;
                    }
                    last_break_at = None;
                }
                _ => {}
            }
        }
    }

    let avg_session = if session_count > 0 { total_desk_minutes / session_count as f64 } else { 0.0 };

    let suggestion = if breaks_taken == 0 && total_desk_minutes > 120.0 {
        "You should take more breaks! Try a 5-minute break every hour.".to_string()
    } else if avg_session > 90.0 {
        "Your sessions are quite long. Consider shorter, focused sessions with breaks.".to_string()
    } else {
        "Good balance! Keep taking regular breaks.".to_string()
    };

    // Daily stats
    let mut daily_stmt = conn.prepare(
        "SELECT DATE(created_at) as d, \
         SUM(CASE WHEN event_type = 'occupied' THEN 1 ELSE 0 END) as desk_sessions, \
         SUM(CASE WHEN event_type = 'break_start' THEN 1 ELSE 0 END) as break_count \
         FROM desk_occupancy_events WHERE device_id = ?1 AND created_at >= datetime('now', '-7 days') \
         GROUP BY d ORDER BY d DESC"
    )?;
    let daily = daily_stmt.query_map([&device_id], |row| {
        Ok(OccupancyDayStats {
            date: row.get(0)?,
            desk_hours: row.get::<_, i64>(1)? as f64, // approximate from session count
            break_count: row.get(2)?,
            longest_session_minutes: 0.0, // simplified
        })
    })?.filter_map(|r| r.ok()).collect::<Vec<_>>();

    Ok(Json(OccupancyReport {
        total_desk_hours: total_desk_minutes / 60.0,
        total_break_hours: total_break_minutes / 60.0,
        avg_session_minutes: avg_session,
        breaks_taken,
        optimal_break_suggestion: suggestion,
        daily_stats: daily,
    }))
}
