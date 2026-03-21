use axum::extract::{Query, State};
use axum::Json;
use serde::{Deserialize, Serialize};
use serde_json::json;

use crate::db::DbPool;
use crate::error::AppError;

// ── Boss Battle ─────────────────────────────────────────────────
// Weekly "boss bug" challenge: a puzzle that earns bonus XP when solved.
// Bosses rotate weekly and have difficulty tiers.

const BOSS_CHALLENGES: &[(&str, &str, &str, i64)] = &[
    ("regex_wraith", "Regex Wraith", "Write a regex that matches all valid IPv4 addresses", 50),
    ("null_pointer_phantom", "Null Pointer Phantom", "Find and fix the null reference in the code snippet", 40),
    ("infinite_loop_hydra", "Infinite Loop Hydra", "Identify the termination condition bug", 45),
    ("off_by_one_ogre", "Off-By-One Ogre", "Fix the array boundary error", 35),
    ("race_condition_dragon", "Race Condition Dragon", "Spot the concurrency bug in the async code", 60),
    ("memory_leak_leviathan", "Memory Leak Leviathan", "Find where the resource isn't freed", 55),
    ("sql_injection_serpent", "SQL Injection Serpent", "Secure the vulnerable query", 45),
    ("deadlock_demon", "Deadlock Demon", "Reorder the lock acquisitions to prevent deadlock", 50),
];

fn current_boss_index() -> usize {
    let now = chrono::Utc::now();
    let week = now.format("%U").to_string().parse::<usize>().unwrap_or(0);
    week % BOSS_CHALLENGES.len()
}

#[derive(Debug, Serialize)]
pub struct BossState {
    pub boss_id: String,
    pub boss_name: String,
    pub challenge: String,
    pub xp_reward: i64,
    pub week_number: i64,
    pub defeated: bool,
    pub defeated_at: Option<String>,
    pub attempts: i64,
    pub global_defeats: i64,
    pub hint: Option<String>,
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

/// GET /api/boss — get current weekly boss
pub async fn get_current_boss(
    State(db): State<DbPool>,
    Query(q): Query<DeviceQuery>,
) -> Result<Json<BossState>, AppError> {
    let conn = db.lock().unwrap();
    let device_id = resolve_device_id(&conn, q.device_id.as_deref())?;

    let idx = current_boss_index();
    let (boss_id, boss_name, challenge, xp_reward) = BOSS_CHALLENGES[idx];
    let week_number = chrono::Utc::now().format("%U").to_string().parse::<i64>().unwrap_or(0);

    let defeated_at: Option<String> = conn.query_row(
        "SELECT defeated_at FROM boss_battles WHERE device_id = ?1 AND boss_id = ?2 AND week_number = ?3 AND defeated = 1",
        rusqlite::params![device_id, boss_id, week_number], |r| r.get(0),
    ).ok();
    let defeated = defeated_at.is_some();

    let attempts: i64 = conn.query_row(
        "SELECT COALESCE(attempts, 0) FROM boss_battles WHERE device_id = ?1 AND boss_id = ?2 AND week_number = ?3",
        rusqlite::params![device_id, boss_id, week_number], |r| r.get(0),
    ).unwrap_or(0);

    let global_defeats: i64 = conn.query_row(
        "SELECT COUNT(*) FROM boss_battles WHERE boss_id = ?1 AND week_number = ?2 AND defeated = 1",
        rusqlite::params![boss_id, week_number], |r| r.get(0),
    ).unwrap_or(0);

    let hint = if attempts >= 3 && !defeated {
        Some(format!("Hint: Focus on the edge cases in the {} challenge!", boss_name))
    } else { None };

    Ok(Json(BossState {
        boss_id: boss_id.to_string(),
        boss_name: boss_name.to_string(),
        challenge: challenge.to_string(),
        xp_reward,
        week_number,
        defeated,
        defeated_at,
        attempts,
        global_defeats,
        hint,
    }))
}

#[derive(Debug, Deserialize)]
pub struct AttemptRequest {
    pub device_id: Option<String>,
    pub answer: String,
}

#[derive(Debug, Serialize)]
pub struct AttemptResponse {
    pub ok: bool,
    pub correct: bool,
    pub xp_earned: i64,
    pub message: String,
    pub attempts: i64,
}

/// POST /api/boss/attempt — attempt to defeat the boss
pub async fn attempt_boss(
    State(db): State<DbPool>,
    Json(input): Json<AttemptRequest>,
) -> Result<Json<AttemptResponse>, AppError> {
    let conn = db.lock().unwrap();
    let device_id = resolve_device_id(&conn, input.device_id.as_deref())?;

    let idx = current_boss_index();
    let (boss_id, boss_name, _, xp_reward) = BOSS_CHALLENGES[idx];
    let week_number = chrono::Utc::now().format("%U").to_string().parse::<i64>().unwrap_or(0);

    // Check if already defeated this week
    let already_defeated: bool = conn.query_row(
        "SELECT COUNT(*) > 0 FROM boss_battles WHERE device_id = ?1 AND boss_id = ?2 AND week_number = ?3 AND defeated = 1",
        rusqlite::params![device_id, boss_id, week_number], |r| r.get(0),
    ).unwrap_or(false);

    if already_defeated {
        return Ok(Json(AttemptResponse {
            ok: true, correct: false, xp_earned: 0,
            message: format!("You already defeated {} this week!", boss_name),
            attempts: 0,
        }));
    }

    // Upsert attempt
    conn.execute(
        "INSERT INTO boss_battles (device_id, boss_id, week_number, attempts) VALUES (?1, ?2, ?3, 1)
         ON CONFLICT(device_id, boss_id, week_number) DO UPDATE SET attempts = attempts + 1",
        rusqlite::params![device_id, boss_id, week_number],
    )?;

    // Simple validation: answer must be non-empty and at least 10 chars (a real attempt)
    let correct = !input.answer.trim().is_empty() && input.answer.trim().len() >= 10;

    let attempts: i64 = conn.query_row(
        "SELECT attempts FROM boss_battles WHERE device_id = ?1 AND boss_id = ?2 AND week_number = ?3",
        rusqlite::params![device_id, boss_id, week_number], |r| r.get(0),
    ).unwrap_or(1);

    let (xp_earned, message) = if correct {
        conn.execute(
            "UPDATE boss_battles SET defeated = 1, defeated_at = datetime('now') WHERE device_id = ?1 AND boss_id = ?2 AND week_number = ?3",
            rusqlite::params![device_id, boss_id, week_number],
        )?;

        // Bonus XP for fewer attempts
        let bonus = match attempts {
            1 => xp_reward / 2,
            2 => xp_reward / 4,
            _ => 0,
        };
        let total_xp = xp_reward + bonus;

        conn.execute(
            "INSERT INTO xp_ledger (device_id, amount, reason) VALUES (?1, ?2, ?3)",
            rusqlite::params![device_id, total_xp, format!("boss_defeat:{}", boss_id)],
        )?;

        (total_xp, format!("You defeated {}! +{} XP{}", boss_name, total_xp,
            if bonus > 0 { format!(" (includes {} bonus for quick solve!)", bonus) } else { String::new() }))
    } else {
        (0, format!("Not quite! {} still stands. Try again! (Attempt #{})", boss_name, attempts))
    };

    Ok(Json(AttemptResponse { ok: true, correct, xp_earned, message, attempts }))
}

/// GET /api/boss/history — get past boss battle results
pub async fn get_history(
    State(db): State<DbPool>,
    Query(q): Query<DeviceQuery>,
) -> Result<Json<Vec<serde_json::Value>>, AppError> {
    let conn = db.lock().unwrap();
    let device_id = resolve_device_id(&conn, q.device_id.as_deref())?;

    let mut stmt = conn.prepare(
        "SELECT boss_id, week_number, defeated, attempts, defeated_at FROM boss_battles WHERE device_id = ?1 ORDER BY week_number DESC LIMIT 20"
    )?;
    let rows = stmt.query_map([&device_id], |r| {
        let boss_id: String = r.get(0)?;
        let name = BOSS_CHALLENGES.iter().find(|(id, _, _, _)| *id == boss_id).map(|(_, n, _, _)| *n).unwrap_or("Unknown");
        Ok(json!({
            "boss_id": boss_id,
            "boss_name": name,
            "week_number": r.get::<_, i64>(1)?,
            "defeated": r.get::<_, bool>(2)?,
            "attempts": r.get::<_, i64>(3)?,
            "defeated_at": r.get::<_, Option<String>>(4)?,
        }))
    })?.collect::<Result<Vec<_>, _>>()?;

    Ok(Json(rows))
}
