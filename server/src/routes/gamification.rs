use axum::extract::{Query, Request, State};
use axum::Json;
use serde::Deserialize;
use serde_json::json;

use crate::auth::UserId;
use crate::db::DbPool;
use crate::error::AppError;
use crate::models::*;

// --- XP / Leveling helpers ---

/// XP required to reach a given level: 100 * level * (level + 1) / 2
/// Level 1: 100, Level 2: 300, Level 3: 600, Level 4: 1000, etc.
pub fn xp_for_level(level: i64) -> i64 {
    100 * level * (level + 1) / 2
}

pub fn level_from_xp(total_xp: i64) -> i64 {
    let mut level = 0i64;
    while xp_for_level(level + 1) <= total_xp {
        level += 1;
    }
    level
}

pub fn title_for_level(level: i64) -> &'static str {
    match level {
        0 => "Newbie",
        1..=2 => "Apprentice",
        3..=5 => "Coder",
        6..=9 => "Developer",
        10..=14 => "Engineer",
        15..=19 => "Architect",
        20..=29 => "Wizard",
        30..=49 => "Legend",
        _ => "Mythic",
    }
}

// --- Badge definitions ---

pub const BADGE_DEFS: &[(&str, &str, &str, &str)] = &[
    ("first_hook", "First Contact", "Received your first hook event", "zap"),
    ("first_ota", "First OTA", "Deployed firmware over the air", "upload"),
    ("tool_100", "Centurion", "100 tool calls tracked", "layers"),
    ("tool_500", "Power User", "500 tool calls tracked", "star"),
    ("tool_1000", "Tool Master", "1000 tool calls tracked", "crown"),
    ("uptime_24h", "Always On", "24h device uptime", "clock"),
    ("night_owl", "Night Owl", "Coding past midnight", "moon"),
    ("early_bird", "Early Bird", "Coding before 6 AM", "sunrise"),
    ("speed_demon", "Speed Demon", "10 tool calls in 5 minutes", "bolt"),
    ("streak_3", "On a Roll", "3-day coding streak", "fire"),
    ("streak_7", "Week Warrior", "7-day coding streak", "fire"),
    ("streak_30", "Monthly Master", "30-day coding streak", "fire"),
    ("xp_1000", "XP Hoarder", "Earned 1000 XP", "gem"),
    ("xp_10000", "XP Mogul", "Earned 10000 XP", "gem"),
    ("level_5", "Level 5", "Reached level 5", "trophy"),
    ("level_10", "Level 10", "Reached level 10", "trophy"),
    ("level_20", "Level 20", "Reached level 20", "trophy"),
    ("all_states", "Shape Shifter", "Used all avatar states", "palette"),
];

// --- Query params ---

#[derive(Debug, Deserialize)]
pub struct DeviceQuery {
    pub device_id: Option<String>,
}

#[derive(Debug, Deserialize)]
pub struct ActivityQuery {
    pub device_id: Option<String>,
    pub limit: Option<i64>,
}

#[derive(Debug, Deserialize)]
pub struct AnalyticsQuery {
    pub device_id: Option<String>,
    pub days: Option<i64>,
}

// --- Route handlers ---

/// Helper: build a SQL IN clause for user's device IDs, or None if legacy/no user
fn user_device_ids(conn: &rusqlite::Connection, user_id: &Option<String>) -> Option<Vec<String>> {
    let uid = user_id.as_ref()?;
    let mut stmt = conn.prepare("SELECT id FROM devices WHERE user_id = ?1").ok()?;
    let ids = stmt.query_map([uid], |r| r.get(0)).ok()?
        .collect::<Result<Vec<String>, _>>().ok()?;
    Some(ids)
}

/// GET /api/gamification/stats
pub async fn get_stats(
    State(db): State<DbPool>,
    Query(q): Query<DeviceQuery>,
    req: Request,
) -> Result<Json<GamificationStats>, AppError> {
    let user_id = req.extensions().get::<UserId>().and_then(|u| u.0.clone());
    let conn = db.lock().unwrap();

    // If device_id specified, use it; otherwise scope to user's devices
    let total_xp: i64 = if let Some(ref did) = q.device_id {
        conn.query_row(
            "SELECT COALESCE(SUM(amount), 0) FROM xp_ledger WHERE device_id = ?1",
            [did], |r| r.get(0),
        )?
    } else if let Some(ref ids) = user_device_ids(&conn, &user_id) {
        if ids.is_empty() { 0 } else {
            let placeholders: Vec<String> = ids.iter().enumerate().map(|(i, _)| format!("?{}", i + 1)).collect();
            let sql = format!("SELECT COALESCE(SUM(amount), 0) FROM xp_ledger WHERE device_id IN ({})", placeholders.join(","));
            let mut stmt = conn.prepare(&sql)?;
            let params: Vec<&dyn rusqlite::types::ToSql> = ids.iter().map(|s| s as &dyn rusqlite::types::ToSql).collect();
            stmt.query_row(params.as_slice(), |r| r.get(0))?
        }
    } else {
        conn.query_row(
            "SELECT COALESCE(SUM(amount), 0) FROM xp_ledger", [],
            |r| r.get(0),
        )?
    };

    let total_tool_uses: i64 = if let Some(ref did) = q.device_id {
        conn.query_row(
            "SELECT COUNT(*) FROM tool_uses WHERE device_id = ?1",
            [did], |r| r.get(0),
        )?
    } else if let Some(ref ids) = user_device_ids(&conn, &user_id) {
        if ids.is_empty() { 0 } else {
            let placeholders: Vec<String> = ids.iter().enumerate().map(|(i, _)| format!("?{}", i + 1)).collect();
            let sql = format!("SELECT COUNT(*) FROM tool_uses WHERE device_id IN ({})", placeholders.join(","));
            let mut stmt = conn.prepare(&sql)?;
            let params: Vec<&dyn rusqlite::types::ToSql> = ids.iter().map(|s| s as &dyn rusqlite::types::ToSql).collect();
            stmt.query_row(params.as_slice(), |r| r.get(0))?
        }
    } else {
        conn.query_row("SELECT COUNT(*) FROM tool_uses", [], |r| r.get(0))?
    };

    let (current_streak, longest_streak) = if let Some(ref did) = q.device_id {
        conn.query_row(
            "SELECT COALESCE(current_streak, 0), COALESCE(longest_streak, 0) FROM streaks WHERE device_id = ?1",
            [did], |r| Ok((r.get::<_, i64>(0)?, r.get::<_, i64>(1)?)),
        ).unwrap_or((0, 0))
    } else if let Some(ref ids) = user_device_ids(&conn, &user_id) {
        if ids.is_empty() { (0, 0) } else {
            let placeholders: Vec<String> = ids.iter().enumerate().map(|(i, _)| format!("?{}", i + 1)).collect();
            let sql = format!("SELECT COALESCE(MAX(current_streak), 0), COALESCE(MAX(longest_streak), 0) FROM streaks WHERE device_id IN ({})", placeholders.join(","));
            let mut stmt = conn.prepare(&sql)?;
            let params: Vec<&dyn rusqlite::types::ToSql> = ids.iter().map(|s| s as &dyn rusqlite::types::ToSql).collect();
            stmt.query_row(params.as_slice(), |r| Ok((r.get::<_, i64>(0)?, r.get::<_, i64>(1)?)))
                .unwrap_or((0, 0))
        }
    } else {
        conn.query_row(
            "SELECT COALESCE(MAX(current_streak), 0), COALESCE(MAX(longest_streak), 0) FROM streaks",
            [], |r| Ok((r.get::<_, i64>(0)?, r.get::<_, i64>(1)?)),
        ).unwrap_or((0, 0))
    };

    let achievements_earned: i64 = if let Some(ref did) = q.device_id {
        conn.query_row(
            "SELECT COUNT(*) FROM achievements WHERE device_id = ?1",
            [did], |r| r.get(0),
        )?
    } else if let Some(ref ids) = user_device_ids(&conn, &user_id) {
        if ids.is_empty() { 0 } else {
            let placeholders: Vec<String> = ids.iter().enumerate().map(|(i, _)| format!("?{}", i + 1)).collect();
            let sql = format!("SELECT COUNT(DISTINCT badge_id) FROM achievements WHERE device_id IN ({})", placeholders.join(","));
            let mut stmt = conn.prepare(&sql)?;
            let params: Vec<&dyn rusqlite::types::ToSql> = ids.iter().map(|s| s as &dyn rusqlite::types::ToSql).collect();
            stmt.query_row(params.as_slice(), |r| r.get(0))?
        }
    } else {
        conn.query_row("SELECT COUNT(DISTINCT badge_id) FROM achievements", [], |r| r.get(0))?
    };

    let level = level_from_xp(total_xp);
    let xp_for_current = xp_for_level(level);
    let xp_for_next = xp_for_level(level + 1);

    Ok(Json(GamificationStats {
        total_xp,
        level,
        xp_for_current_level: xp_for_current,
        xp_for_next_level: xp_for_next,
        total_tool_uses,
        current_streak,
        longest_streak,
        achievements_earned,
        title: title_for_level(level).to_string(),
    }))
}

/// GET /api/gamification/activity
pub async fn get_activity(
    State(db): State<DbPool>,
    Query(q): Query<ActivityQuery>,
) -> Result<Json<Vec<ActivityEntry>>, AppError> {
    let conn = db.lock().unwrap();
    let limit = q.limit.unwrap_or(50).min(200);

    let mut stmt = if let Some(ref did) = q.device_id {
        let mut s = conn.prepare(
            "SELECT id, tool_name, event, xp_earned, created_at, device_id FROM tool_uses WHERE device_id = ?1 ORDER BY created_at DESC LIMIT ?2"
        )?;
        let rows = s.query_map(rusqlite::params![did, limit], |row| {
            Ok(ActivityEntry {
                id: row.get(0)?,
                tool_name: row.get(1)?,
                event: row.get(2)?,
                xp_earned: row.get(3)?,
                created_at: row.get(4)?,
                device_id: row.get(5)?,
            })
        })?.collect::<Result<Vec<_>, _>>()?;
        return Ok(Json(rows));
    } else {
        conn.prepare(
            "SELECT id, tool_name, event, xp_earned, created_at, device_id FROM tool_uses ORDER BY created_at DESC LIMIT ?1"
        )?
    };

    let rows = stmt.query_map(rusqlite::params![limit], |row| {
        Ok(ActivityEntry {
            id: row.get(0)?,
            tool_name: row.get(1)?,
            event: row.get(2)?,
            xp_earned: row.get(3)?,
            created_at: row.get(4)?,
            device_id: row.get(5)?,
        })
    })?.collect::<Result<Vec<_>, _>>()?;

    Ok(Json(rows))
}

/// GET /api/gamification/analytics
pub async fn get_analytics(
    State(db): State<DbPool>,
    Query(q): Query<AnalyticsQuery>,
) -> Result<Json<AnalyticsData>, AppError> {
    let conn = db.lock().unwrap();
    let days = q.days.unwrap_or(30);
    let device_filter = q.device_id.as_deref().unwrap_or("");
    let has_device = !device_filter.is_empty();

    // Tools per day
    let tools_per_day = {
        let days_p = format!("-{days} days");
        if has_device {
            let mut stmt = conn.prepare(
                "SELECT date(created_at) as d, COUNT(*) as c FROM tool_uses WHERE device_id = ?1 AND created_at >= datetime('now', ?2) GROUP BY d ORDER BY d"
            )?;
            let rows = stmt.query_map(rusqlite::params![device_filter, days_p], |r| {
                Ok(DayCount { date: r.get(0)?, count: r.get(1)? })
            })?.collect::<Result<Vec<_>, _>>()?;
            rows
        } else {
            let mut stmt = conn.prepare(
                "SELECT date(created_at) as d, COUNT(*) as c FROM tool_uses WHERE created_at >= datetime('now', ?1) GROUP BY d ORDER BY d"
            )?;
            let rows = stmt.query_map(rusqlite::params![days_p], |r| {
                Ok(DayCount { date: r.get(0)?, count: r.get(1)? })
            })?.collect::<Result<Vec<_>, _>>()?;
            rows
        }
    };

    // Hourly activity
    let hourly_activity = {
        if has_device {
            let mut stmt = conn.prepare("SELECT CAST(strftime('%H', created_at) AS INTEGER) as h, COUNT(*) as c FROM tool_uses WHERE device_id = ?1 GROUP BY h ORDER BY h")?;
            let rows = stmt.query_map(rusqlite::params![device_filter], |r| {
                Ok(HourCount { hour: r.get(0)?, count: r.get(1)? })
            })?.collect::<Result<Vec<_>, _>>()?;
            rows
        } else {
            let mut stmt = conn.prepare("SELECT CAST(strftime('%H', created_at) AS INTEGER) as h, COUNT(*) as c FROM tool_uses GROUP BY h ORDER BY h")?;
            let rows = stmt.query_map([], |r| {
                Ok(HourCount { hour: r.get(0)?, count: r.get(1)? })
            })?.collect::<Result<Vec<_>, _>>()?;
            rows
        }
    };

    // Tool distribution (top 20)
    let tool_distribution = {
        if has_device {
            let mut stmt = conn.prepare("SELECT tool_name, COUNT(*) as c FROM tool_uses WHERE device_id = ?1 GROUP BY tool_name ORDER BY c DESC LIMIT 20")?;
            let rows = stmt.query_map(rusqlite::params![device_filter], |r| {
                Ok(ToolCount { tool_name: r.get(0)?, count: r.get(1)? })
            })?.collect::<Result<Vec<_>, _>>()?;
            rows
        } else {
            let mut stmt = conn.prepare("SELECT tool_name, COUNT(*) as c FROM tool_uses GROUP BY tool_name ORDER BY c DESC LIMIT 20")?;
            let rows = stmt.query_map([], |r| {
                Ok(ToolCount { tool_name: r.get(0)?, count: r.get(1)? })
            })?.collect::<Result<Vec<_>, _>>()?;
            rows
        }
    };

    // State distribution from status_log
    let state_distribution = {
        if has_device {
            let mut stmt = conn.prepare("SELECT state, COUNT(*) as c FROM status_log WHERE device_id = ?1 GROUP BY state ORDER BY c DESC")?;
            let rows = stmt.query_map(rusqlite::params![device_filter], |r| {
                Ok(StateCount { state: r.get(0)?, count: r.get(1)? })
            })?.collect::<Result<Vec<_>, _>>()?;
            rows
        } else {
            let mut stmt = conn.prepare("SELECT state, COUNT(*) as c FROM status_log GROUP BY state ORDER BY c DESC")?;
            let rows = stmt.query_map([], |r| {
                Ok(StateCount { state: r.get(0)?, count: r.get(1)? })
            })?.collect::<Result<Vec<_>, _>>()?;
            rows
        }
    };

    // Session lengths
    let session_lengths = {
        let days_p = format!("-{days} days");
        if has_device {
            let mut stmt = conn.prepare("SELECT date(started_at) as d, (julianday(COALESCE(ended_at, datetime('now'))) - julianday(started_at)) * 1440.0 as mins FROM sessions WHERE device_id = ?1 AND started_at >= datetime('now', ?2) ORDER BY started_at")?;
            let rows = stmt.query_map(rusqlite::params![device_filter, days_p], |r| {
                Ok(SessionLength { date: r.get(0)?, duration_minutes: r.get(1)? })
            })?.collect::<Result<Vec<_>, _>>()?;
            rows
        } else {
            let mut stmt = conn.prepare("SELECT date(started_at) as d, (julianday(COALESCE(ended_at, datetime('now'))) - julianday(started_at)) * 1440.0 as mins FROM sessions WHERE started_at >= datetime('now', ?1) ORDER BY started_at")?;
            let rows = stmt.query_map(rusqlite::params![days_p], |r| {
                Ok(SessionLength { date: r.get(0)?, duration_minutes: r.get(1)? })
            })?.collect::<Result<Vec<_>, _>>()?;
            rows
        }
    };

    // XP over time
    let xp_over_time = {
        let days_p = format!("-{days} days");
        if has_device {
            let mut stmt = conn.prepare("SELECT date(created_at) as d, SUM(amount) as x FROM xp_ledger WHERE device_id = ?1 AND created_at >= datetime('now', ?2) GROUP BY d ORDER BY d")?;
            let rows = stmt.query_map(rusqlite::params![device_filter, days_p], |r| {
                Ok(DayXp { date: r.get(0)?, xp: r.get(1)? })
            })?.collect::<Result<Vec<_>, _>>()?;
            rows
        } else {
            let mut stmt = conn.prepare("SELECT date(created_at) as d, SUM(amount) as x FROM xp_ledger WHERE created_at >= datetime('now', ?1) GROUP BY d ORDER BY d")?;
            let rows = stmt.query_map(rusqlite::params![days_p], |r| {
                Ok(DayXp { date: r.get(0)?, xp: r.get(1)? })
            })?.collect::<Result<Vec<_>, _>>()?;
            rows
        }
    };

    Ok(Json(AnalyticsData {
        tools_per_day,
        hourly_activity,
        tool_distribution,
        state_distribution,
        session_lengths,
        xp_over_time,
    }))
}

/// GET /api/gamification/achievements
pub async fn get_achievements(
    State(db): State<DbPool>,
    Query(q): Query<DeviceQuery>,
) -> Result<Json<Vec<BadgeDefinition>>, AppError> {
    let conn = db.lock().unwrap();

    let earned: Vec<(String, String)> = if let Some(ref did) = q.device_id {
        let mut stmt = conn.prepare("SELECT badge_id, earned_at FROM achievements WHERE device_id = ?1")?;
        let rows = stmt.query_map([did], |r| Ok((r.get(0)?, r.get(1)?)))?.collect::<Result<Vec<_>, _>>()?;
        rows
    } else {
        let mut stmt = conn.prepare("SELECT badge_id, MIN(earned_at) FROM achievements GROUP BY badge_id")?;
        let rows = stmt.query_map([], |r| Ok((r.get(0)?, r.get(1)?)))?.collect::<Result<Vec<_>, _>>()?;
        rows
    };

    let badges: Vec<BadgeDefinition> = BADGE_DEFS.iter().map(|(id, name, desc, icon)| {
        let found = earned.iter().find(|(bid, _)| bid == id);
        BadgeDefinition {
            id,
            name,
            description: desc,
            icon,
            earned: found.is_some(),
            earned_at: found.map(|(_, at)| at.clone()),
        }
    }).collect();

    Ok(Json(badges))
}

/// GET /api/gamification/leaderboard
pub async fn get_leaderboard(
    State(db): State<DbPool>,
    req: Request,
) -> Result<Json<Vec<LeaderboardEntry>>, AppError> {
    let user_id = req.extensions().get::<UserId>().and_then(|u| u.0.clone());
    let conn = db.lock().unwrap();

    let sql = if user_id.is_some() {
        "SELECT d.id, d.name,
                COALESCE((SELECT SUM(amount) FROM xp_ledger WHERE device_id = d.id), 0) as total_xp,
                COALESCE((SELECT current_streak FROM streaks WHERE device_id = d.id), 0) as streak,
                COALESCE((SELECT COUNT(*) FROM achievements WHERE device_id = d.id), 0) as badges
         FROM devices d
         WHERE d.user_id = ?1
         ORDER BY total_xp DESC"
    } else {
        "SELECT d.id, d.name,
                COALESCE((SELECT SUM(amount) FROM xp_ledger WHERE device_id = d.id), 0) as total_xp,
                COALESCE((SELECT current_streak FROM streaks WHERE device_id = d.id), 0) as streak,
                COALESCE((SELECT COUNT(*) FROM achievements WHERE device_id = d.id), 0) as badges
         FROM devices d
         ORDER BY total_xp DESC"
    };

    let mut stmt = conn.prepare(sql)?;

    let entries = if let Some(ref uid) = user_id {
        stmt.query_map([uid], |r| {
            let total_xp: i64 = r.get(2)?;
            Ok(LeaderboardEntry {
                device_id: r.get(0)?,
                device_name: r.get(1)?,
                total_xp,
                level: level_from_xp(total_xp),
                current_streak: r.get(3)?,
                achievements: r.get(4)?,
            })
        })?.collect::<Result<Vec<_>, _>>()?
    } else {
        stmt.query_map([], |r| {
            let total_xp: i64 = r.get(2)?;
            Ok(LeaderboardEntry {
                device_id: r.get(0)?,
                device_name: r.get(1)?,
                total_xp,
                level: level_from_xp(total_xp),
                current_streak: r.get(3)?,
                achievements: r.get(4)?,
            })
        })?.collect::<Result<Vec<_>, _>>()?
    };

    Ok(Json(entries))
}

/// GET /api/gamification/streaks
pub async fn get_streaks(
    State(db): State<DbPool>,
    Query(q): Query<DeviceQuery>,
) -> Result<Json<serde_json::Value>, AppError> {
    let conn = db.lock().unwrap();

    if let Some(ref did) = q.device_id {
        let streak = conn.query_row(
            "SELECT current_streak, longest_streak, last_active_date, updated_at FROM streaks WHERE device_id = ?1",
            [did],
            |r| Ok(json!({
                "current_streak": r.get::<_, i64>(0)?,
                "longest_streak": r.get::<_, i64>(1)?,
                "last_active_date": r.get::<_, Option<String>>(2)?,
                "updated_at": r.get::<_, String>(3)?,
            })),
        ).unwrap_or(json!({
            "current_streak": 0,
            "longest_streak": 0,
            "last_active_date": null,
            "updated_at": null,
        }));
        Ok(Json(streak))
    } else {
        let mut stmt = conn.prepare(
            "SELECT s.device_id, d.name, s.current_streak, s.longest_streak, s.last_active_date
             FROM streaks s JOIN devices d ON s.device_id = d.id ORDER BY s.current_streak DESC"
        )?;
        let rows = stmt.query_map([], |r| {
            Ok(json!({
                "device_id": r.get::<_, String>(0)?,
                "device_name": r.get::<_, String>(1)?,
                "current_streak": r.get::<_, i64>(2)?,
                "longest_streak": r.get::<_, i64>(3)?,
                "last_active_date": r.get::<_, Option<String>>(4)?,
            }))
        })?.collect::<Result<Vec<_>, _>>()?;
        Ok(Json(json!(rows)))
    }
}

// --- XP recording helper (called from hooks) ---

pub fn record_tool_use_and_xp(
    conn: &rusqlite::Connection,
    device_id: Option<&str>,
    tool_name: &str,
    event: &str,
    project: Option<&str>,
) -> Result<(i64, Vec<String>), rusqlite::Error> {
    // Determine XP based on event type
    let xp = match event {
        "PreToolUse" => 5,
        "PostToolUse" => 10,
        "UserPromptSubmit" => 3,
        "TaskCompleted" => 25,
        "Stop" => 2,
        _ => 1,
    };

    // Insert tool use
    conn.execute(
        "INSERT INTO tool_uses (device_id, tool_name, event, project, xp_earned) VALUES (?1, ?2, ?3, ?4, ?5)",
        rusqlite::params![device_id, tool_name, event, project, xp],
    )?;

    // Insert XP ledger entry
    let reason = format!("{}:{}", event, tool_name);
    conn.execute(
        "INSERT INTO xp_ledger (device_id, amount, reason) VALUES (?1, ?2, ?3)",
        rusqlite::params![device_id, xp, reason],
    )?;

    // Update or create session (within 30 min gap = same session)
    if let Some(did) = device_id {
        let session_exists: bool = conn.query_row(
            "SELECT COUNT(*) > 0 FROM sessions WHERE device_id = ?1 AND ended_at IS NULL AND started_at >= datetime('now', '-30 minutes')",
            [did], |r| r.get(0),
        ).unwrap_or(false);

        if session_exists {
            conn.execute(
                "UPDATE sessions SET tool_count = tool_count + 1, xp_earned = xp_earned + ?1, ended_at = datetime('now') WHERE device_id = ?2 AND ended_at IS NULL",
                rusqlite::params![xp, did],
            )?;
        } else {
            // Close any open sessions
            conn.execute(
                "UPDATE sessions SET ended_at = datetime('now') WHERE device_id = ?1 AND ended_at IS NULL",
                [did],
            )?;
            // Start new session
            conn.execute(
                "INSERT INTO sessions (device_id, tool_count, xp_earned) VALUES (?1, 1, ?2)",
                rusqlite::params![did, xp],
            )?;
        }

        // Update streak
        let today = chrono::Utc::now().format("%Y-%m-%d").to_string();
        let streak_row: Option<(i64, i64, Option<String>)> = conn.query_row(
            "SELECT current_streak, longest_streak, last_active_date FROM streaks WHERE device_id = ?1",
            [did],
            |r| Ok((r.get(0)?, r.get(1)?, r.get(2)?)),
        ).ok();

        if let Some((current, longest, last_date)) = streak_row {
            if last_date.as_deref() != Some(&today) {
                let yesterday = (chrono::Utc::now() - chrono::Duration::days(1)).format("%Y-%m-%d").to_string();
                let new_streak = if last_date.as_deref() == Some(yesterday.as_str()) {
                    current + 1
                } else {
                    1
                };
                let new_longest = new_streak.max(longest);
                conn.execute(
                    "UPDATE streaks SET current_streak = ?1, longest_streak = ?2, last_active_date = ?3, updated_at = datetime('now') WHERE device_id = ?4",
                    rusqlite::params![new_streak, new_longest, today, did],
                )?;
            }
        } else {
            conn.execute(
                "INSERT INTO streaks (device_id, current_streak, longest_streak, last_active_date) VALUES (?1, 1, 1, ?2)",
                rusqlite::params![did, today],
            )?;
        }
    }

    // Check achievements
    let mut new_badges = Vec::new();
    if let Some(did) = device_id {
        let total_xp: i64 = conn.query_row(
            "SELECT COALESCE(SUM(amount), 0) FROM xp_ledger WHERE device_id = ?1",
            [did], |r| r.get(0),
        )?;
        let total_tools: i64 = conn.query_row(
            "SELECT COUNT(*) FROM tool_uses WHERE device_id = ?1",
            [did], |r| r.get(0),
        )?;
        let current_streak: i64 = conn.query_row(
            "SELECT COALESCE(current_streak, 0) FROM streaks WHERE device_id = ?1",
            [did], |r| r.get(0),
        ).unwrap_or(0);
        let level = level_from_xp(total_xp);
        let hour: i64 = conn.query_row(
            "SELECT CAST(strftime('%H', 'now') AS INTEGER)", [], |r| r.get(0),
        )?;

        // Check recent speed (tools in last 5 min)
        let recent_5min: i64 = conn.query_row(
            "SELECT COUNT(*) FROM tool_uses WHERE device_id = ?1 AND created_at >= datetime('now', '-5 minutes')",
            [did], |r| r.get(0),
        )?;

        let checks: Vec<(&str, bool)> = vec![
            ("first_hook", total_tools >= 1),
            ("tool_100", total_tools >= 100),
            ("tool_500", total_tools >= 500),
            ("tool_1000", total_tools >= 1000),
            ("night_owl", hour >= 0 && hour < 4),
            ("early_bird", hour >= 4 && hour < 6),
            ("speed_demon", recent_5min >= 10),
            ("streak_3", current_streak >= 3),
            ("streak_7", current_streak >= 7),
            ("streak_30", current_streak >= 30),
            ("xp_1000", total_xp >= 1000),
            ("xp_10000", total_xp >= 10000),
            ("level_5", level >= 5),
            ("level_10", level >= 10),
            ("level_20", level >= 20),
        ];

        for (badge_id, condition) in checks {
            if condition {
                let already: bool = conn.query_row(
                    "SELECT COUNT(*) > 0 FROM achievements WHERE device_id = ?1 AND badge_id = ?2",
                    rusqlite::params![did, badge_id], |r| r.get(0),
                ).unwrap_or(true);
                if !already {
                    conn.execute(
                        "INSERT OR IGNORE INTO achievements (device_id, badge_id) VALUES (?1, ?2)",
                        rusqlite::params![did, badge_id],
                    )?;
                    new_badges.push(badge_id.to_string());
                }
            }
        }
    }

    Ok((xp, new_badges))
}
