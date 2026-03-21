use axum::extract::{Query, State};
use axum::Json;
use serde::Deserialize;
use serde_json::json;

use crate::db::DbPool;
use crate::error::AppError;

#[derive(Debug, Deserialize)]
pub struct InsightsQuery {
    pub device_id: Option<String>,
    pub days: Option<i64>,
}

/// GET /api/insights/flow-state
/// Detect flow state sessions from tool usage patterns.
/// A flow state is defined as >= 10 tool uses within a session with < 2 min gaps.
pub async fn get_flow_states(
    State(db): State<DbPool>,
    Query(q): Query<InsightsQuery>,
) -> Result<Json<serde_json::Value>, AppError> {
    let conn = db.lock().unwrap();
    let days = q.days.unwrap_or(30);
    let days_p = format!("-{days} days");

    // Identify sessions that look like flow states (high-intensity coding)
    let (flow_sessions, total_sessions, avg_flow_duration) = if let Some(ref did) = q.device_id {
        let flow: i64 = conn.query_row(
            "SELECT COUNT(*) FROM sessions WHERE device_id = ?1 AND tool_count >= 10
             AND started_at >= datetime('now', ?2) AND ended_at IS NOT NULL",
            rusqlite::params![did, days_p], |r| r.get(0),
        )?;
        let total: i64 = conn.query_row(
            "SELECT COUNT(*) FROM sessions WHERE device_id = ?1
             AND started_at >= datetime('now', ?2) AND ended_at IS NOT NULL",
            rusqlite::params![did, days_p], |r| r.get(0),
        )?;
        let avg_dur: f64 = conn.query_row(
            "SELECT COALESCE(AVG((julianday(ended_at) - julianday(started_at)) * 1440.0), 0)
             FROM sessions WHERE device_id = ?1 AND tool_count >= 10
             AND started_at >= datetime('now', ?2) AND ended_at IS NOT NULL",
            rusqlite::params![did, days_p], |r| r.get(0),
        )?;
        (flow, total, avg_dur)
    } else {
        let flow: i64 = conn.query_row(
            "SELECT COUNT(*) FROM sessions WHERE tool_count >= 10
             AND started_at >= datetime('now', ?1) AND ended_at IS NOT NULL",
            rusqlite::params![days_p], |r| r.get(0),
        )?;
        let total: i64 = conn.query_row(
            "SELECT COUNT(*) FROM sessions WHERE started_at >= datetime('now', ?1) AND ended_at IS NOT NULL",
            rusqlite::params![days_p], |r| r.get(0),
        )?;
        let avg_dur: f64 = conn.query_row(
            "SELECT COALESCE(AVG((julianday(ended_at) - julianday(started_at)) * 1440.0), 0)
             FROM sessions WHERE tool_count >= 10
             AND started_at >= datetime('now', ?1) AND ended_at IS NOT NULL",
            rusqlite::params![days_p], |r| r.get(0),
        )?;
        (flow, total, avg_dur)
    };

    // Peak flow hours
    let peak_flow_hours = {
        let sql = if q.device_id.is_some() {
            "SELECT CAST(strftime('%H', started_at) AS INTEGER) as h, COUNT(*) as c
             FROM sessions WHERE device_id = ?1 AND tool_count >= 10
             AND started_at >= datetime('now', ?2) AND ended_at IS NOT NULL
             GROUP BY h ORDER BY c DESC LIMIT 5"
        } else {
            "SELECT CAST(strftime('%H', started_at) AS INTEGER) as h, COUNT(*) as c
             FROM sessions WHERE tool_count >= 10
             AND started_at >= datetime('now', ?1) AND ended_at IS NOT NULL
             GROUP BY h ORDER BY c DESC LIMIT 5"
        };
        let mut stmt = conn.prepare(sql)?;
        let rows: Vec<serde_json::Value> = if let Some(ref did) = q.device_id {
            stmt.query_map(rusqlite::params![did, days_p], |r| {
                Ok(json!({"hour": r.get::<_, i64>(0)?, "count": r.get::<_, i64>(1)?}))
            })?.collect::<Result<Vec<_>, _>>()?
        } else {
            stmt.query_map(rusqlite::params![days_p], |r| {
                Ok(json!({"hour": r.get::<_, i64>(0)?, "count": r.get::<_, i64>(1)?}))
            })?.collect::<Result<Vec<_>, _>>()?
        };
        rows
    };

    let flow_rate = if total_sessions > 0 {
        (flow_sessions as f64 / total_sessions as f64) * 100.0
    } else {
        0.0
    };

    Ok(Json(json!({
        "flow_sessions": flow_sessions,
        "total_sessions": total_sessions,
        "flow_rate_pct": (flow_rate * 10.0).round() / 10.0,
        "avg_flow_duration_min": (avg_flow_duration * 10.0).round() / 10.0,
        "peak_flow_hours": peak_flow_hours,
    })))
}

/// GET /api/insights/code-quality
/// Track coding quality correlation with time of day, day of week, and session length.
pub async fn get_code_quality(
    State(db): State<DbPool>,
    Query(q): Query<InsightsQuery>,
) -> Result<Json<serde_json::Value>, AppError> {
    let conn = db.lock().unwrap();
    let days = q.days.unwrap_or(30);
    let days_p = format!("-{days} days");

    // XP per hour (proxy for productivity/quality)
    let xp_by_hour = {
        let sql = if q.device_id.is_some() {
            "SELECT CAST(strftime('%H', created_at) AS INTEGER) as h,
                    COALESCE(AVG(xp_earned), 0) as avg_xp,
                    COUNT(*) as count
             FROM tool_uses WHERE device_id = ?1 AND created_at >= datetime('now', ?2)
             GROUP BY h ORDER BY h"
        } else {
            "SELECT CAST(strftime('%H', created_at) AS INTEGER) as h,
                    COALESCE(AVG(xp_earned), 0) as avg_xp,
                    COUNT(*) as count
             FROM tool_uses WHERE created_at >= datetime('now', ?1)
             GROUP BY h ORDER BY h"
        };
        let mut stmt = conn.prepare(sql)?;
        let rows: Vec<serde_json::Value> = if let Some(ref did) = q.device_id {
            stmt.query_map(rusqlite::params![did, days_p], |r| {
                Ok(json!({
                    "hour": r.get::<_, i64>(0)?,
                    "avg_xp": r.get::<_, f64>(1)?,
                    "count": r.get::<_, i64>(2)?,
                }))
            })?.collect::<Result<Vec<_>, _>>()?
        } else {
            stmt.query_map(rusqlite::params![days_p], |r| {
                Ok(json!({
                    "hour": r.get::<_, i64>(0)?,
                    "avg_xp": r.get::<_, f64>(1)?,
                    "count": r.get::<_, i64>(2)?,
                }))
            })?.collect::<Result<Vec<_>, _>>()?
        };
        rows
    };

    // Productivity by day of week
    let by_day_of_week = {
        let sql = if q.device_id.is_some() {
            "SELECT CAST(strftime('%w', created_at) AS INTEGER) as dow,
                    COUNT(*) as count, COALESCE(SUM(xp_earned), 0) as total_xp
             FROM tool_uses WHERE device_id = ?1 AND created_at >= datetime('now', ?2)
             GROUP BY dow ORDER BY dow"
        } else {
            "SELECT CAST(strftime('%w', created_at) AS INTEGER) as dow,
                    COUNT(*) as count, COALESCE(SUM(xp_earned), 0) as total_xp
             FROM tool_uses WHERE created_at >= datetime('now', ?1)
             GROUP BY dow ORDER BY dow"
        };
        let dow_names = ["Sun", "Mon", "Tue", "Wed", "Thu", "Fri", "Sat"];
        let mut stmt = conn.prepare(sql)?;
        let rows: Vec<serde_json::Value> = if let Some(ref did) = q.device_id {
            stmt.query_map(rusqlite::params![did, days_p], |r| {
                let dow: usize = r.get::<_, i64>(0)? as usize;
                Ok(json!({
                    "day": dow_names.get(dow).unwrap_or(&"?"),
                    "day_num": dow,
                    "tool_uses": r.get::<_, i64>(1)?,
                    "total_xp": r.get::<_, i64>(2)?,
                }))
            })?.collect::<Result<Vec<_>, _>>()?
        } else {
            stmt.query_map(rusqlite::params![days_p], |r| {
                let dow: usize = r.get::<_, i64>(0)? as usize;
                Ok(json!({
                    "day": dow_names.get(dow).unwrap_or(&"?"),
                    "day_num": dow,
                    "tool_uses": r.get::<_, i64>(1)?,
                    "total_xp": r.get::<_, i64>(2)?,
                }))
            })?.collect::<Result<Vec<_>, _>>()?
        };
        rows
    };

    // Best and worst hours
    let best_hour: Option<i64> = xp_by_hour.iter()
        .filter_map(|v| {
            let count = v["count"].as_i64().unwrap_or(0);
            if count >= 5 { Some((v["avg_xp"].as_f64().unwrap_or(0.0), v["hour"].as_i64().unwrap_or(0))) } else { None }
        })
        .max_by(|a, b| a.0.partial_cmp(&b.0).unwrap_or(std::cmp::Ordering::Equal))
        .map(|(_, h)| h);

    Ok(Json(json!({
        "xp_by_hour": xp_by_hour,
        "by_day_of_week": by_day_of_week,
        "optimal_hour": best_hour,
    })))
}

/// GET /api/insights/weekly-digest
/// Generate a weekly summary of activity, patterns, and tips.
pub async fn get_weekly_digest(
    State(db): State<DbPool>,
    Query(q): Query<InsightsQuery>,
) -> Result<Json<serde_json::Value>, AppError> {
    let conn = db.lock().unwrap();

    let device_filter = q.device_id.as_deref();

    // This week's stats
    let (week_tools, week_xp, week_sessions) = if let Some(did) = device_filter {
        let tools: i64 = conn.query_row(
            "SELECT COUNT(*) FROM tool_uses WHERE device_id = ?1 AND created_at >= datetime('now', '-7 days')",
            [did], |r| r.get(0),
        )?;
        let xp: i64 = conn.query_row(
            "SELECT COALESCE(SUM(amount), 0) FROM xp_ledger WHERE device_id = ?1 AND created_at >= datetime('now', '-7 days')",
            [did], |r| r.get(0),
        )?;
        let sessions: i64 = conn.query_row(
            "SELECT COUNT(*) FROM sessions WHERE device_id = ?1 AND started_at >= datetime('now', '-7 days')",
            [did], |r| r.get(0),
        )?;
        (tools, xp, sessions)
    } else {
        let tools: i64 = conn.query_row(
            "SELECT COUNT(*) FROM tool_uses WHERE created_at >= datetime('now', '-7 days')", [], |r| r.get(0),
        )?;
        let xp: i64 = conn.query_row(
            "SELECT COALESCE(SUM(amount), 0) FROM xp_ledger WHERE created_at >= datetime('now', '-7 days')", [], |r| r.get(0),
        )?;
        let sessions: i64 = conn.query_row(
            "SELECT COUNT(*) FROM sessions WHERE started_at >= datetime('now', '-7 days')", [], |r| r.get(0),
        )?;
        (tools, xp, sessions)
    };

    // Previous week's stats for comparison
    let (prev_tools, prev_xp) = if let Some(did) = device_filter {
        let tools: i64 = conn.query_row(
            "SELECT COUNT(*) FROM tool_uses WHERE device_id = ?1
             AND created_at >= datetime('now', '-14 days') AND created_at < datetime('now', '-7 days')",
            [did], |r| r.get(0),
        )?;
        let xp: i64 = conn.query_row(
            "SELECT COALESCE(SUM(amount), 0) FROM xp_ledger WHERE device_id = ?1
             AND created_at >= datetime('now', '-14 days') AND created_at < datetime('now', '-7 days')",
            [did], |r| r.get(0),
        )?;
        (tools, xp)
    } else {
        let tools: i64 = conn.query_row(
            "SELECT COUNT(*) FROM tool_uses
             WHERE created_at >= datetime('now', '-14 days') AND created_at < datetime('now', '-7 days')", [], |r| r.get(0),
        )?;
        let xp: i64 = conn.query_row(
            "SELECT COALESCE(SUM(amount), 0) FROM xp_ledger
             WHERE created_at >= datetime('now', '-14 days') AND created_at < datetime('now', '-7 days')", [], |r| r.get(0),
        )?;
        (tools, xp)
    };

    // Top tools this week
    let top_tools = {
        let sql = if device_filter.is_some() {
            "SELECT tool_name, COUNT(*) as c FROM tool_uses WHERE device_id = ?1
             AND created_at >= datetime('now', '-7 days') GROUP BY tool_name ORDER BY c DESC LIMIT 5"
        } else {
            "SELECT tool_name, COUNT(*) as c FROM tool_uses
             WHERE created_at >= datetime('now', '-7 days') GROUP BY tool_name ORDER BY c DESC LIMIT 5"
        };
        let mut stmt = conn.prepare(sql)?;
        let rows: Vec<serde_json::Value> = if let Some(did) = device_filter {
            stmt.query_map([did], |r| {
                Ok(json!({"tool": r.get::<_, String>(0)?, "count": r.get::<_, i64>(1)?}))
            })?.collect::<Result<Vec<_>, _>>()?
        } else {
            stmt.query_map([], |r| {
                Ok(json!({"tool": r.get::<_, String>(0)?, "count": r.get::<_, i64>(1)?}))
            })?.collect::<Result<Vec<_>, _>>()?
        };
        rows
    };

    // Most active day this week
    let most_active_day = {
        let sql = if device_filter.is_some() {
            "SELECT date(created_at) as d, COUNT(*) as c FROM tool_uses WHERE device_id = ?1
             AND created_at >= datetime('now', '-7 days') GROUP BY d ORDER BY c DESC LIMIT 1"
        } else {
            "SELECT date(created_at) as d, COUNT(*) as c FROM tool_uses
             WHERE created_at >= datetime('now', '-7 days') GROUP BY d ORDER BY c DESC LIMIT 1"
        };
        let mut stmt = conn.prepare(sql)?;
        let row: Option<serde_json::Value> = if let Some(did) = device_filter {
            stmt.query_map([did], |r| {
                Ok(json!({"date": r.get::<_, String>(0)?, "count": r.get::<_, i64>(1)?}))
            })?.collect::<Result<Vec<_>, _>>()?.into_iter().next()
        } else {
            stmt.query_map([], |r| {
                Ok(json!({"date": r.get::<_, String>(0)?, "count": r.get::<_, i64>(1)?}))
            })?.collect::<Result<Vec<_>, _>>()?.into_iter().next()
        };
        row
    };

    // Generate tips based on patterns
    let mut tips: Vec<String> = Vec::new();
    let tools_change = if prev_tools > 0 {
        ((week_tools as f64 - prev_tools as f64) / prev_tools as f64 * 100.0).round()
    } else {
        0.0
    };

    if tools_change > 20.0 {
        tips.push(format!("Your activity is up {tools_change}% from last week. Great momentum!"));
    } else if tools_change < -20.0 {
        tips.push(format!("Activity dropped {:.0}% from last week. Consider setting daily coding goals.", tools_change.abs()));
    }

    if week_sessions > 0 {
        let avg_tools_per_session = week_tools as f64 / week_sessions as f64;
        if avg_tools_per_session < 5.0 {
            tips.push("Your sessions are short. Try blocking longer focus periods.".to_string());
        } else if avg_tools_per_session > 30.0 {
            tips.push("Long intense sessions detected. Remember to take breaks every 90 minutes.".to_string());
        }
    }

    if tips.is_empty() {
        tips.push("Keep up the consistent work! Steady progress is key.".to_string());
    }

    Ok(Json(json!({
        "period": "last_7_days",
        "this_week": {
            "tool_uses": week_tools,
            "xp_earned": week_xp,
            "sessions": week_sessions,
        },
        "previous_week": {
            "tool_uses": prev_tools,
            "xp_earned": prev_xp,
        },
        "change_pct": {
            "tool_uses": tools_change,
            "xp": if prev_xp > 0 { ((week_xp as f64 - prev_xp as f64) / prev_xp as f64 * 100.0).round() } else { 0.0 },
        },
        "top_tools": top_tools,
        "most_active_day": most_active_day,
        "tips": tips,
    })))
}

/// GET /api/insights/burnout
/// Detect potential burnout patterns: late nights, no breaks, declining quality.
pub async fn get_burnout_check(
    State(db): State<DbPool>,
    Query(q): Query<InsightsQuery>,
) -> Result<Json<serde_json::Value>, AppError> {
    let conn = db.lock().unwrap();
    let days = q.days.unwrap_or(14);
    let days_p = format!("-{days} days");

    // Late night sessions (after 10 PM)
    let late_nights: i64 = if let Some(ref did) = q.device_id {
        conn.query_row(
            "SELECT COUNT(DISTINCT date(created_at)) FROM tool_uses WHERE device_id = ?1
             AND created_at >= datetime('now', ?2)
             AND CAST(strftime('%H', created_at) AS INTEGER) >= 22",
            rusqlite::params![did, days_p], |r| r.get(0),
        )?
    } else {
        conn.query_row(
            "SELECT COUNT(DISTINCT date(created_at)) FROM tool_uses
             WHERE created_at >= datetime('now', ?1)
             AND CAST(strftime('%H', created_at) AS INTEGER) >= 22",
            rusqlite::params![days_p], |r| r.get(0),
        )?
    };

    // Weekend work days
    let weekend_days: i64 = if let Some(ref did) = q.device_id {
        conn.query_row(
            "SELECT COUNT(DISTINCT date(created_at)) FROM tool_uses WHERE device_id = ?1
             AND created_at >= datetime('now', ?2)
             AND CAST(strftime('%w', created_at) AS INTEGER) IN (0, 6)",
            rusqlite::params![did, days_p], |r| r.get(0),
        )?
    } else {
        conn.query_row(
            "SELECT COUNT(DISTINCT date(created_at)) FROM tool_uses
             WHERE created_at >= datetime('now', ?1)
             AND CAST(strftime('%w', created_at) AS INTEGER) IN (0, 6)",
            rusqlite::params![days_p], |r| r.get(0),
        )?
    };

    // Average daily tool uses (declining = possible burnout)
    let daily_trend = {
        let sql = if q.device_id.is_some() {
            "SELECT date(created_at) as d, COUNT(*) as c FROM tool_uses WHERE device_id = ?1
             AND created_at >= datetime('now', ?2) GROUP BY d ORDER BY d"
        } else {
            "SELECT date(created_at) as d, COUNT(*) as c FROM tool_uses
             WHERE created_at >= datetime('now', ?1) GROUP BY d ORDER BY d"
        };
        let mut stmt = conn.prepare(sql)?;
        let rows: Vec<(String, i64)> = if let Some(ref did) = q.device_id {
            stmt.query_map(rusqlite::params![did, days_p], |r| {
                Ok((r.get::<_, String>(0)?, r.get::<_, i64>(1)?))
            })?.collect::<Result<Vec<_>, _>>()?
        } else {
            stmt.query_map(rusqlite::params![days_p], |r| {
                Ok((r.get::<_, String>(0)?, r.get::<_, i64>(1)?))
            })?.collect::<Result<Vec<_>, _>>()?
        };
        rows
    };

    // Calculate trend (simple: compare first half vs second half average)
    let total_days = daily_trend.len();
    let (first_half_avg, second_half_avg) = if total_days >= 4 {
        let mid = total_days / 2;
        let first: f64 = daily_trend[..mid].iter().map(|(_, c)| *c as f64).sum::<f64>() / mid as f64;
        let second: f64 = daily_trend[mid..].iter().map(|(_, c)| *c as f64).sum::<f64>() / (total_days - mid) as f64;
        (first, second)
    } else {
        (0.0, 0.0)
    };

    let trend_direction = if first_half_avg > 0.0 {
        ((second_half_avg - first_half_avg) / first_half_avg * 100.0).round()
    } else {
        0.0
    };

    // Longest consecutive work days without a break
    let active_dates: Vec<String> = daily_trend.iter().map(|(d, _)| d.clone()).collect();
    let mut max_consecutive = 0i64;
    let mut current_consecutive = 1i64;
    for i in 1..active_dates.len() {
        // Simple: if dates are consecutive
        let prev = &active_dates[i - 1];
        let curr = &active_dates[i];
        if let (Ok(p), Ok(c)) = (
            chrono::NaiveDate::parse_from_str(prev, "%Y-%m-%d"),
            chrono::NaiveDate::parse_from_str(curr, "%Y-%m-%d"),
        ) {
            if c - p == chrono::Duration::days(1) {
                current_consecutive += 1;
            } else {
                max_consecutive = max_consecutive.max(current_consecutive);
                current_consecutive = 1;
            }
        }
    }
    max_consecutive = max_consecutive.max(current_consecutive);

    // Risk score (0-100)
    let mut risk_score = 0.0f64;
    let late_night_ratio = late_nights as f64 / days as f64;
    risk_score += late_night_ratio * 30.0; // up to 30 points
    risk_score += (weekend_days as f64 / (days as f64 / 7.0 * 2.0).max(1.0)).min(1.0) * 20.0; // up to 20 points
    if trend_direction < -20.0 { risk_score += 20.0; } // declining output
    if max_consecutive > 10 { risk_score += 15.0; }
    if max_consecutive > 20 { risk_score += 15.0; }

    let risk_level = match risk_score as i64 {
        0..=25 => "low",
        26..=50 => "moderate",
        51..=75 => "elevated",
        _ => "high",
    };

    let mut warnings: Vec<String> = Vec::new();
    if late_nights > days / 3 {
        warnings.push(format!("Late night coding on {late_nights} of {days} days. Try to wrap up by 10 PM."));
    }
    if weekend_days > 2 {
        warnings.push(format!("Worked {weekend_days} weekend days in the period. Consider taking weekends off."));
    }
    if trend_direction < -30.0 {
        warnings.push(format!("Activity declining {:.0}% — this could indicate fatigue.", trend_direction.abs()));
    }
    if max_consecutive > 10 {
        warnings.push(format!("{max_consecutive} consecutive work days without a break. Rest is important!"));
    }
    if warnings.is_empty() {
        warnings.push("Your work pattern looks healthy. Keep it up!".to_string());
    }

    Ok(Json(json!({
        "risk_score": (risk_score.min(100.0) * 10.0).round() / 10.0,
        "risk_level": risk_level,
        "late_night_days": late_nights,
        "weekend_work_days": weekend_days,
        "max_consecutive_days": max_consecutive,
        "activity_trend_pct": trend_direction,
        "warnings": warnings,
    })))
}

/// GET /api/insights/project-time
/// Automatic per-project time tracking from hook data.
pub async fn get_project_time(
    State(db): State<DbPool>,
    Query(q): Query<InsightsQuery>,
) -> Result<Json<serde_json::Value>, AppError> {
    let conn = db.lock().unwrap();
    let days = q.days.unwrap_or(30);
    let days_p = format!("-{days} days");

    let projects = {
        let sql = if q.device_id.is_some() {
            "SELECT COALESCE(project, 'unknown') as p,
                    COUNT(*) as tool_count,
                    COALESCE(SUM(xp_earned), 0) as total_xp,
                    MIN(created_at) as first_active,
                    MAX(created_at) as last_active,
                    COUNT(DISTINCT date(created_at)) as active_days
             FROM tool_uses WHERE device_id = ?1 AND created_at >= datetime('now', ?2)
             GROUP BY p ORDER BY tool_count DESC"
        } else {
            "SELECT COALESCE(project, 'unknown') as p,
                    COUNT(*) as tool_count,
                    COALESCE(SUM(xp_earned), 0) as total_xp,
                    MIN(created_at) as first_active,
                    MAX(created_at) as last_active,
                    COUNT(DISTINCT date(created_at)) as active_days
             FROM tool_uses WHERE created_at >= datetime('now', ?1)
             GROUP BY p ORDER BY tool_count DESC"
        };
        let mut stmt = conn.prepare(sql)?;
        let rows: Vec<serde_json::Value> = if let Some(ref did) = q.device_id {
            stmt.query_map(rusqlite::params![did, days_p], |r| {
                let tool_count: i64 = r.get(1)?;
                // Estimate minutes: ~1 min per 2 tool uses (rough proxy)
                let est_minutes = (tool_count as f64 * 0.5).round();
                Ok(json!({
                    "project": r.get::<_, String>(0)?,
                    "tool_uses": tool_count,
                    "total_xp": r.get::<_, i64>(2)?,
                    "estimated_minutes": est_minutes,
                    "first_active": r.get::<_, String>(3)?,
                    "last_active": r.get::<_, String>(4)?,
                    "active_days": r.get::<_, i64>(5)?,
                }))
            })?.collect::<Result<Vec<_>, _>>()?
        } else {
            stmt.query_map(rusqlite::params![days_p], |r| {
                let tool_count: i64 = r.get(1)?;
                let est_minutes = (tool_count as f64 * 0.5).round();
                Ok(json!({
                    "project": r.get::<_, String>(0)?,
                    "tool_uses": tool_count,
                    "total_xp": r.get::<_, i64>(2)?,
                    "estimated_minutes": est_minutes,
                    "first_active": r.get::<_, String>(3)?,
                    "last_active": r.get::<_, String>(4)?,
                    "active_days": r.get::<_, i64>(5)?,
                }))
            })?.collect::<Result<Vec<_>, _>>()?
        };
        rows
    };

    let total_estimated_hours: f64 = projects.iter()
        .map(|p| p["estimated_minutes"].as_f64().unwrap_or(0.0))
        .sum::<f64>() / 60.0;

    Ok(Json(json!({
        "projects": projects,
        "total_estimated_hours": (total_estimated_hours * 10.0).round() / 10.0,
        "period_days": days,
    })))
}

/// GET /api/insights/pair-programming
/// Detect pairing sessions based on rapid tool alternation patterns and multi-device activity.
pub async fn get_pair_programming(
    State(db): State<DbPool>,
    Query(q): Query<InsightsQuery>,
) -> Result<Json<serde_json::Value>, AppError> {
    let conn = db.lock().unwrap();
    let days = q.days.unwrap_or(30);
    let days_p = format!("-{days} days");

    // Detect sessions with very high tool density (pair sessions tend to have more interactions)
    let high_density_sessions: i64 = if let Some(ref did) = q.device_id {
        conn.query_row(
            "SELECT COUNT(*) FROM sessions WHERE device_id = ?1 AND tool_count >= 20
             AND started_at >= datetime('now', ?2) AND ended_at IS NOT NULL
             AND (julianday(ended_at) - julianday(started_at)) * 1440.0 > 10",
            rusqlite::params![did, days_p], |r| r.get(0),
        )?
    } else {
        conn.query_row(
            "SELECT COUNT(*) FROM sessions WHERE tool_count >= 20
             AND started_at >= datetime('now', ?1) AND ended_at IS NOT NULL
             AND (julianday(ended_at) - julianday(started_at)) * 1440.0 > 10",
            rusqlite::params![days_p], |r| r.get(0),
        )?
    };

    // Total sessions for comparison
    let total_sessions: i64 = if let Some(ref did) = q.device_id {
        conn.query_row(
            "SELECT COUNT(*) FROM sessions WHERE device_id = ?1
             AND started_at >= datetime('now', ?2) AND ended_at IS NOT NULL",
            rusqlite::params![did, days_p], |r| r.get(0),
        )?
    } else {
        conn.query_row(
            "SELECT COUNT(*) FROM sessions WHERE started_at >= datetime('now', ?1) AND ended_at IS NOT NULL",
            rusqlite::params![days_p], |r| r.get(0),
        )?
    };

    // Concurrent device sessions (true pairing indicator)
    let concurrent_device_sessions: i64 = conn.query_row(
        "SELECT COUNT(*) FROM sessions s1
         JOIN sessions s2 ON s1.device_id != s2.device_id
         AND s1.started_at < COALESCE(s2.ended_at, datetime('now'))
         AND s2.started_at < COALESCE(s1.ended_at, datetime('now'))
         AND s1.started_at >= datetime('now', ?1)
         AND s1.id < s2.id",
        rusqlite::params![days_p], |r| r.get(0),
    ).unwrap_or(0);

    Ok(Json(json!({
        "high_density_sessions": high_density_sessions,
        "total_sessions": total_sessions,
        "concurrent_device_sessions": concurrent_device_sessions,
        "pair_rate_pct": if total_sessions > 0 {
            ((high_density_sessions as f64 / total_sessions as f64) * 100.0 * 10.0).round() / 10.0
        } else { 0.0 },
    })))
}

/// GET /api/insights/retrospective
/// Auto-generate sprint retro talking points from hookbot data.
pub async fn get_retrospective(
    State(db): State<DbPool>,
    Query(q): Query<InsightsQuery>,
) -> Result<Json<serde_json::Value>, AppError> {
    let conn = db.lock().unwrap();
    let days = q.days.unwrap_or(14); // default 2-week sprint
    let days_p = format!("-{days} days");

    let device_filter = q.device_id.as_deref();

    // Total activity
    let (total_tools, total_xp, total_sessions) = if let Some(did) = device_filter {
        let t: i64 = conn.query_row(
            "SELECT COUNT(*) FROM tool_uses WHERE device_id = ?1 AND created_at >= datetime('now', ?2)",
            rusqlite::params![did, days_p], |r| r.get(0))?;
        let x: i64 = conn.query_row(
            "SELECT COALESCE(SUM(amount), 0) FROM xp_ledger WHERE device_id = ?1 AND created_at >= datetime('now', ?2)",
            rusqlite::params![did, days_p], |r| r.get(0))?;
        let s: i64 = conn.query_row(
            "SELECT COUNT(*) FROM sessions WHERE device_id = ?1 AND started_at >= datetime('now', ?2)",
            rusqlite::params![did, days_p], |r| r.get(0))?;
        (t, x, s)
    } else {
        let t: i64 = conn.query_row(
            "SELECT COUNT(*) FROM tool_uses WHERE created_at >= datetime('now', ?1)",
            rusqlite::params![days_p], |r| r.get(0))?;
        let x: i64 = conn.query_row(
            "SELECT COALESCE(SUM(amount), 0) FROM xp_ledger WHERE created_at >= datetime('now', ?1)",
            rusqlite::params![days_p], |r| r.get(0))?;
        let s: i64 = conn.query_row(
            "SELECT COUNT(*) FROM sessions WHERE started_at >= datetime('now', ?1)",
            rusqlite::params![days_p], |r| r.get(0))?;
        (t, x, s)
    };

    // Achievements earned this sprint
    let new_achievements: Vec<String> = {
        let sql = if device_filter.is_some() {
            "SELECT badge_id FROM achievements WHERE device_id = ?1 AND earned_at >= datetime('now', ?2)"
        } else {
            "SELECT DISTINCT badge_id FROM achievements WHERE earned_at >= datetime('now', ?1)"
        };
        let mut stmt = conn.prepare(sql)?;
        if let Some(did) = device_filter {
            stmt.query_map(rusqlite::params![did, days_p], |r| r.get(0))?
                .collect::<Result<Vec<String>, _>>()?
        } else {
            stmt.query_map(rusqlite::params![days_p], |r| r.get(0))?
                .collect::<Result<Vec<String>, _>>()?
        }
    };

    // Active days count
    let active_days: i64 = if let Some(did) = device_filter {
        conn.query_row(
            "SELECT COUNT(DISTINCT date(created_at)) FROM tool_uses WHERE device_id = ?1 AND created_at >= datetime('now', ?2)",
            rusqlite::params![did, days_p], |r| r.get(0))?
    } else {
        conn.query_row(
            "SELECT COUNT(DISTINCT date(created_at)) FROM tool_uses WHERE created_at >= datetime('now', ?1)",
            rusqlite::params![days_p], |r| r.get(0))?
    };

    // Top used tools
    let top_tools = {
        let sql = if device_filter.is_some() {
            "SELECT tool_name, COUNT(*) as c FROM tool_uses WHERE device_id = ?1
             AND created_at >= datetime('now', ?2) GROUP BY tool_name ORDER BY c DESC LIMIT 5"
        } else {
            "SELECT tool_name, COUNT(*) as c FROM tool_uses
             WHERE created_at >= datetime('now', ?1) GROUP BY tool_name ORDER BY c DESC LIMIT 5"
        };
        let mut stmt = conn.prepare(sql)?;
        let rows: Vec<serde_json::Value> = if let Some(did) = device_filter {
            stmt.query_map(rusqlite::params![did, days_p], |r| {
                Ok(json!({"tool": r.get::<_, String>(0)?, "count": r.get::<_, i64>(1)?}))
            })?.collect::<Result<Vec<_>, _>>()?
        } else {
            stmt.query_map(rusqlite::params![days_p], |r| {
                Ok(json!({"tool": r.get::<_, String>(0)?, "count": r.get::<_, i64>(1)?}))
            })?.collect::<Result<Vec<_>, _>>()?
        };
        rows
    };

    // Generate talking points
    let mut went_well: Vec<String> = Vec::new();
    let mut to_improve: Vec<String> = Vec::new();

    if active_days as f64 / days as f64 > 0.7 {
        went_well.push(format!("Consistent activity: coded on {active_days} of {days} days"));
    } else if active_days > 0 {
        to_improve.push(format!("Only active {active_days} of {days} days. Try to code more consistently."));
    }

    if !new_achievements.is_empty() {
        went_well.push(format!("Earned {} new achievement(s): {}", new_achievements.len(), new_achievements.join(", ")));
    }

    if total_sessions > 0 {
        let avg_tools = total_tools as f64 / total_sessions as f64;
        if avg_tools > 15.0 {
            went_well.push(format!("Deep focus sessions averaging {:.0} tool uses each", avg_tools));
        }
    }

    if went_well.is_empty() {
        went_well.push("Keep building momentum. Every session counts.".to_string());
    }

    Ok(Json(json!({
        "sprint_days": days,
        "summary": {
            "total_tool_uses": total_tools,
            "total_xp": total_xp,
            "total_sessions": total_sessions,
            "active_days": active_days,
            "new_achievements": new_achievements,
        },
        "top_tools": top_tools,
        "talking_points": {
            "went_well": went_well,
            "to_improve": to_improve,
        },
    })))
}
