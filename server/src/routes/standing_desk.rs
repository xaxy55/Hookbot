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

/// GET /api/standing-desk — get standing desk config and status
pub async fn get_config(
    State(db): State<DbPool>,
    Query(q): Query<DeviceQuery>,
) -> Result<Json<StandingDeskConfig>, AppError> {
    let conn = db.lock().unwrap();
    let device_id = resolve_device_id(&conn, q.device_id.as_deref())?;

    let config = conn.query_row(
        "SELECT id, device_id, sit_remind_minutes, stand_remind_minutes, enabled, current_position, \
         total_stand_minutes, total_sit_minutes, transitions_today, last_transition_at, created_at \
         FROM standing_desk WHERE device_id = ?1",
        [&device_id],
        |row| Ok(StandingDeskConfig {
            id: row.get(0)?,
            device_id: row.get(1)?,
            sit_remind_minutes: row.get(2)?,
            stand_remind_minutes: row.get(3)?,
            enabled: row.get(4)?,
            current_position: row.get(5)?,
            total_stand_minutes: row.get(6)?,
            total_sit_minutes: row.get(7)?,
            transitions_today: row.get(8)?,
            last_transition_at: row.get(9)?,
            created_at: row.get(10)?,
        }),
    );

    match config {
        Ok(c) => Ok(Json(c)),
        Err(_) => {
            // Auto-create config
            let id = uuid::Uuid::new_v4().to_string();
            conn.execute(
                "INSERT INTO standing_desk (id, device_id) VALUES (?1, ?2)",
                rusqlite::params![id, device_id],
            )?;
            Ok(Json(StandingDeskConfig {
                id,
                device_id,
                sit_remind_minutes: 45,
                stand_remind_minutes: 15,
                enabled: true,
                current_position: "sitting".into(),
                total_stand_minutes: 0,
                total_sit_minutes: 0,
                transitions_today: 0,
                last_transition_at: None,
                created_at: chrono::Utc::now().to_rfc3339(),
            }))
        }
    }
}

/// PUT /api/standing-desk — update standing desk config
pub async fn update_config(
    State(db): State<DbPool>,
    Query(q): Query<DeviceQuery>,
    Json(input): Json<UpdateStandingDeskConfig>,
) -> Result<Json<serde_json::Value>, AppError> {
    let conn = db.lock().unwrap();
    let device_id = resolve_device_id(&conn, q.device_id.as_deref())?;

    if let Some(v) = input.sit_remind_minutes {
        conn.execute("UPDATE standing_desk SET sit_remind_minutes = ?1 WHERE device_id = ?2", rusqlite::params![v, device_id])?;
    }
    if let Some(v) = input.stand_remind_minutes {
        conn.execute("UPDATE standing_desk SET stand_remind_minutes = ?1 WHERE device_id = ?2", rusqlite::params![v, device_id])?;
    }
    if let Some(v) = input.enabled {
        conn.execute("UPDATE standing_desk SET enabled = ?1 WHERE device_id = ?2", rusqlite::params![v, device_id])?;
    }

    Ok(Json(serde_json::json!({ "ok": true })))
}

/// POST /api/standing-desk/position — record a position change (sit/stand)
pub async fn change_position(
    State(db): State<DbPool>,
    Query(q): Query<DeviceQuery>,
    Json(input): Json<DeskPositionChange>,
) -> Result<Json<serde_json::Value>, AppError> {
    let conn = db.lock().unwrap();
    let device_id = resolve_device_id(&conn, q.device_id.as_deref())?;

    if input.position != "sitting" && input.position != "standing" {
        return Err(AppError::BadRequest("Position must be 'sitting' or 'standing'".into()));
    }

    // Calculate duration of previous position
    let prev = conn.query_row(
        "SELECT current_position, last_transition_at FROM standing_desk WHERE device_id = ?1",
        [&device_id],
        |row| Ok((row.get::<_, String>(0)?, row.get::<_, Option<String>>(1)?)),
    );

    if let Ok((prev_pos, last_at)) = prev {
        if let Some(last_str) = last_at {
            if let Ok(last_time) = chrono::DateTime::parse_from_rfc3339(&last_str) {
                let duration = chrono::Utc::now().signed_duration_since(last_time.with_timezone(&chrono::Utc));
                let mins = duration.num_minutes();

                // Log history
                conn.execute(
                    "INSERT INTO standing_desk_history (device_id, position, duration_minutes) VALUES (?1, ?2, ?3)",
                    rusqlite::params![device_id, prev_pos, mins],
                )?;

                // Update totals
                let col = if prev_pos == "standing" { "total_stand_minutes" } else { "total_sit_minutes" };
                conn.execute(
                    &format!("UPDATE standing_desk SET {col} = {col} + ?1 WHERE device_id = ?2"),
                    rusqlite::params![mins, device_id],
                )?;
            }
        }
    }

    // Update current position
    conn.execute(
        "UPDATE standing_desk SET current_position = ?1, last_transition_at = datetime('now'), \
         transitions_today = transitions_today + 1 WHERE device_id = ?2",
        rusqlite::params![input.position, device_id],
    )?;

    let celebration = input.position == "standing";

    Ok(Json(serde_json::json!({
        "ok": true,
        "position": input.position,
        "celebration": celebration,
        "message": if celebration { "Great job standing up! Your hookbot is celebrating!" } else { "Taking a seat. Remember to stand again soon!" }
    })))
}

/// GET /api/standing-desk/report — get desk health report
pub async fn get_report(
    State(db): State<DbPool>,
    Query(q): Query<DeviceQuery>,
) -> Result<Json<DeskHealthReport>, AppError> {
    let conn = db.lock().unwrap();
    let device_id = resolve_device_id(&conn, q.device_id.as_deref())?;

    let (total_stand, total_sit, transitions) = conn.query_row(
        "SELECT total_stand_minutes, total_sit_minutes, transitions_today FROM standing_desk WHERE device_id = ?1",
        [&device_id],
        |row| Ok((row.get::<_, i64>(0)?, row.get::<_, i64>(1)?, row.get::<_, i64>(2)?)),
    ).unwrap_or((0, 0, 0));

    let total = total_stand + total_sit;
    let ratio = if total > 0 { total_stand as f64 / total as f64 } else { 0.0 };

    // Get daily history
    let mut stmt = conn.prepare(
        "SELECT DATE(recorded_at) as d, \
         SUM(CASE WHEN position = 'standing' THEN duration_minutes ELSE 0 END), \
         SUM(CASE WHEN position = 'sitting' THEN duration_minutes ELSE 0 END), \
         COUNT(*) \
         FROM standing_desk_history WHERE device_id = ?1 \
         GROUP BY d ORDER BY d DESC LIMIT 7"
    )?;
    let daily = stmt.query_map([&device_id], |row| {
        Ok(DeskDayStats {
            date: row.get(0)?,
            stand_minutes: row.get(1)?,
            sit_minutes: row.get(2)?,
            transitions: row.get(3)?,
        })
    })?.filter_map(|r| r.ok()).collect::<Vec<_>>();

    Ok(Json(DeskHealthReport {
        total_stand_minutes: total_stand,
        total_sit_minutes: total_sit,
        stand_ratio: ratio,
        transitions_today: transitions,
        daily_history: daily,
    }))
}
