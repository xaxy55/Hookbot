use axum::extract::{Query, State};
use axum::Json;
use serde::{Deserialize, Serialize};
use serde_json::json;

use crate::db::DbPool;
use crate::error::AppError;

// ── Tamagotchi mode ─────────────────────────────────────────────
// The hookbot is a virtual pet that evolves based on your coding style.
// It gets fed automatically when you code, gets sad if ignored, and
// evolves into different forms based on patterns in your tool usage.

#[derive(Debug, Serialize)]
pub struct TamagotchiState {
    pub device_id: String,
    pub name: String,
    pub hunger: i64,
    pub happiness: i64,
    pub energy: i64,
    pub form: String,          // egg, blob, robot, mech, cosmic
    pub coding_style: String,  // explorer, builder, debugger, refactorer
    pub personality_traits: Vec<String>,
    pub age_hours: f64,
    pub total_interactions: i64,
    pub last_coding_at: Option<String>,
    pub last_decay_at: Option<String>,
    pub evolved_at: Option<String>,
    pub mood: String,
}

fn form_for_level(level: i64) -> &'static str {
    match level {
        0..=9 => "egg",
        10..=19 => "blob",
        20..=29 => "robot",
        30..=39 => "mech",
        _ => "cosmic",
    }
}

fn coding_style_from(conn: &rusqlite::Connection, device_id: &str) -> String {
    // Determine coding style from recent tool usage patterns
    let grep_count: i64 = conn.query_row(
        "SELECT COUNT(*) FROM tool_uses WHERE device_id = ?1 AND tool_name IN ('Grep', 'Glob', 'Read') AND created_at >= datetime('now', '-7 days')",
        [device_id], |r| r.get(0),
    ).unwrap_or(0);

    let edit_count: i64 = conn.query_row(
        "SELECT COUNT(*) FROM tool_uses WHERE device_id = ?1 AND tool_name IN ('Edit', 'Write', 'NotebookEdit') AND created_at >= datetime('now', '-7 days')",
        [device_id], |r| r.get(0),
    ).unwrap_or(0);

    let bash_count: i64 = conn.query_row(
        "SELECT COUNT(*) FROM tool_uses WHERE device_id = ?1 AND tool_name = 'Bash' AND created_at >= datetime('now', '-7 days')",
        [device_id], |r| r.get(0),
    ).unwrap_or(0);

    let total = grep_count + edit_count + bash_count;
    if total == 0 {
        return "explorer".to_string();
    }

    if grep_count as f64 / total as f64 > 0.5 {
        "explorer".to_string()
    } else if edit_count as f64 / total as f64 > 0.5 {
        "builder".to_string()
    } else if bash_count as f64 / total as f64 > 0.4 {
        "debugger".to_string()
    } else {
        "refactorer".to_string()
    }
}

fn personality_traits(conn: &rusqlite::Connection, device_id: &str) -> Vec<String> {
    let mut traits = Vec::new();

    // Night owl?
    let late_count: i64 = conn.query_row(
        "SELECT COUNT(*) FROM tool_uses WHERE device_id = ?1 AND CAST(strftime('%H', created_at) AS INTEGER) >= 22",
        [device_id], |r| r.get(0),
    ).unwrap_or(0);
    if late_count > 10 { traits.push("night_owl".to_string()); }

    // Early bird?
    let early_count: i64 = conn.query_row(
        "SELECT COUNT(*) FROM tool_uses WHERE device_id = ?1 AND CAST(strftime('%H', created_at) AS INTEGER) < 7",
        [device_id], |r| r.get(0),
    ).unwrap_or(0);
    if early_count > 10 { traits.push("early_bird".to_string()); }

    // Prolific coder?
    let total: i64 = conn.query_row(
        "SELECT COUNT(*) FROM tool_uses WHERE device_id = ?1",
        [device_id], |r| r.get(0),
    ).unwrap_or(0);
    if total > 500 { traits.push("prolific".to_string()); }
    if total > 1000 { traits.push("tireless".to_string()); }

    // Streak keeper?
    let streak: i64 = conn.query_row(
        "SELECT COALESCE(current_streak, 0) FROM streaks WHERE device_id = ?1",
        [device_id], |r| r.get(0),
    ).unwrap_or(0);
    if streak >= 7 { traits.push("consistent".to_string()); }
    if streak >= 30 { traits.push("devoted".to_string()); }

    traits
}

fn tamagotchi_mood(hunger: i64, happiness: i64, energy: i64) -> &'static str {
    let avg = (hunger + happiness + energy) / 3;
    match avg {
        80..=100 => "thriving",
        60..=79 => "happy",
        40..=59 => "okay",
        20..=39 => "neglected",
        _ => "desperate",
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

/// GET /api/tamagotchi — get tamagotchi state
pub async fn get_state(
    State(db): State<DbPool>,
    Query(q): Query<DeviceQuery>,
) -> Result<Json<TamagotchiState>, AppError> {
    let conn = db.lock().unwrap();
    let device_id = resolve_device_id(&conn, q.device_id.as_deref())?;

    // Ensure row exists
    conn.execute(
        "INSERT OR IGNORE INTO tamagotchi (device_id) VALUES (?1)",
        [&device_id],
    )?;

    let (name, hunger, happiness, energy, form, last_coding_at, last_decay_at, evolved_at, total_interactions, created_at): (
        String, i64, i64, i64, String, Option<String>, Option<String>, Option<String>, i64, String,
    ) = conn.query_row(
        "SELECT name, hunger, happiness, energy, form, last_coding_at, last_decay_at, evolved_at, total_interactions, created_at FROM tamagotchi WHERE device_id = ?1",
        [&device_id],
        |row| Ok((row.get(0)?, row.get(1)?, row.get(2)?, row.get(3)?, row.get(4)?,
                   row.get(5)?, row.get(6)?, row.get(7)?, row.get(8)?, row.get(9)?)),
    )?;

    // Decay stats based on inactivity
    let decayed_hunger = if let Some(ref ts) = last_coding_at {
        let mins: i64 = conn.query_row(
            "SELECT CAST((julianday('now') - julianday(?1)) * 24 * 60 AS INTEGER)", [ts], |r| r.get(0),
        ).unwrap_or(0);
        (hunger - mins / 15).max(0)  // lose 1 per 15 min of no coding
    } else { 20 };

    let decayed_happiness = if let Some(ref ts) = last_coding_at {
        let mins: i64 = conn.query_row(
            "SELECT CAST((julianday('now') - julianday(?1)) * 24 * 60 AS INTEGER)", [ts], |r| r.get(0),
        ).unwrap_or(0);
        (happiness - mins / 30).max(0)  // lose 1 per 30 min
    } else { 30 };

    let decayed_energy = if let Some(ref ts) = last_coding_at {
        let mins: i64 = conn.query_row(
            "SELECT CAST((julianday('now') - julianday(?1)) * 24 * 60 AS INTEGER)", [ts], |r| r.get(0),
        ).unwrap_or(0);
        // Energy recovers when idle (opposite of hunger/happiness)
        (energy + mins / 60).min(100)
    } else { 80 };

    // Age in hours
    let age_hours: f64 = conn.query_row(
        "SELECT (julianday('now') - julianday(?1)) * 24", [&created_at], |r| r.get(0),
    ).unwrap_or(0.0);

    // Check if form should evolve based on XP level
    let total_xp: i64 = conn.query_row(
        "SELECT COALESCE(SUM(amount), 0) FROM xp_ledger WHERE device_id = ?1",
        [&device_id], |r| r.get(0),
    ).unwrap_or(0);
    let level = crate::routes::gamification::level_from_xp(total_xp);
    let expected_form = form_for_level(level).to_string();
    if expected_form != form {
        conn.execute(
            "UPDATE tamagotchi SET form = ?1, evolved_at = datetime('now') WHERE device_id = ?2",
            rusqlite::params![expected_form, device_id],
        )?;
    }

    let coding_style = coding_style_from(&conn, &device_id);
    let traits = personality_traits(&conn, &device_id);
    let mood = tamagotchi_mood(decayed_hunger, decayed_happiness, decayed_energy).to_string();

    let current_form = if expected_form != form { expected_form } else { form };

    Ok(Json(TamagotchiState {
        device_id,
        name,
        hunger: decayed_hunger,
        happiness: decayed_happiness,
        energy: decayed_energy,
        form: current_form,
        coding_style,
        personality_traits: traits,
        age_hours,
        total_interactions,
        last_coding_at,
        last_decay_at,
        evolved_at,
        mood,
    }))
}

#[derive(Debug, Deserialize)]
pub struct InteractRequest {
    pub device_id: Option<String>,
    pub action: String, // "play", "rest", "treat"
}

#[derive(Debug, Serialize)]
pub struct InteractResponse {
    pub ok: bool,
    pub message: String,
    pub hunger: i64,
    pub happiness: i64,
    pub energy: i64,
    pub mood: String,
}

/// POST /api/tamagotchi/interact — interact with your tamagotchi
pub async fn interact(
    State(db): State<DbPool>,
    Json(input): Json<InteractRequest>,
) -> Result<Json<InteractResponse>, AppError> {
    let conn = db.lock().unwrap();
    let device_id = resolve_device_id(&conn, input.device_id.as_deref())?;

    conn.execute("INSERT OR IGNORE INTO tamagotchi (device_id) VALUES (?1)", [&device_id])?;

    let (message, hunger_delta, happiness_delta, energy_delta) = match input.action.as_str() {
        "play" => ("Your hookbot does a happy dance!", 0i64, 25i64, -10i64),
        "rest" => ("Your hookbot takes a cozy nap...", -5, 10, 30),
        "treat" => ("Yum! Your hookbot gobbles up the treat!", 30, 15, 5),
        _ => ("Your hookbot looks at you curiously.", 0, 5, 0),
    };

    conn.execute(
        "UPDATE tamagotchi SET hunger = MIN(100, MAX(0, hunger + ?1)), happiness = MIN(100, MAX(0, happiness + ?2)), energy = MIN(100, MAX(0, energy + ?3)), total_interactions = total_interactions + 1 WHERE device_id = ?4",
        rusqlite::params![hunger_delta, happiness_delta, energy_delta, device_id],
    )?;

    let (hunger, happiness, energy): (i64, i64, i64) = conn.query_row(
        "SELECT hunger, happiness, energy FROM tamagotchi WHERE device_id = ?1",
        [&device_id], |r| Ok((r.get(0)?, r.get(1)?, r.get(2)?)),
    )?;

    let mood = tamagotchi_mood(hunger, happiness, energy).to_string();

    Ok(Json(InteractResponse { ok: true, message: message.to_string(), hunger, happiness, energy, mood }))
}

#[derive(Debug, Deserialize)]
pub struct RenameRequest {
    pub device_id: Option<String>,
    pub name: String,
}

/// POST /api/tamagotchi/rename — rename your tamagotchi
pub async fn rename(
    State(db): State<DbPool>,
    Json(input): Json<RenameRequest>,
) -> Result<Json<serde_json::Value>, AppError> {
    let conn = db.lock().unwrap();
    let device_id = resolve_device_id(&conn, input.device_id.as_deref())?;

    let name = input.name.chars().take(32).collect::<String>();
    conn.execute(
        "UPDATE tamagotchi SET name = ?1 WHERE device_id = ?2",
        rusqlite::params![name, device_id],
    )?;

    Ok(Json(json!({ "ok": true, "name": name })))
}

/// Called from hook handler when coding activity is detected
pub fn on_coding_activity(conn: &rusqlite::Connection, device_id: &str) -> Result<(), rusqlite::Error> {
    conn.execute("INSERT OR IGNORE INTO tamagotchi (device_id) VALUES (?1)", [device_id])?;

    // Coding feeds the tamagotchi and makes it happy
    conn.execute(
        "UPDATE tamagotchi SET hunger = MIN(100, hunger + 3), happiness = MIN(100, happiness + 2), energy = MAX(0, energy - 1), last_coding_at = datetime('now'), total_interactions = total_interactions + 1 WHERE device_id = ?1",
        [device_id],
    )?;

    Ok(())
}
