use axum::extract::{Path, Query, State};
use axum::Json;
use serde_json::json;

use crate::db::DbPool;
use crate::error::AppError;
use crate::models::*;
use crate::routes::gamification::level_from_xp;

// ========================
// Buddy System
// ========================

/// GET /api/social/buddies — list buddy pairs
pub async fn list_buddies(
    State(db): State<DbPool>,
) -> Result<Json<Vec<Buddy>>, AppError> {
    let conn = db.lock().unwrap();
    let mut stmt = conn.prepare(
        "SELECT b.id, b.device_id, b.buddy_device_id, b.status, b.mirror_mood, \
         b.created_at, b.accepted_at, d1.name, d2.name \
         FROM buddies b \
         LEFT JOIN devices d1 ON d1.id = b.device_id \
         LEFT JOIN devices d2 ON d2.id = b.buddy_device_id \
         ORDER BY b.created_at DESC"
    )?;
    let rows = stmt.query_map([], |row| {
        Ok(Buddy {
            id: row.get(0)?,
            device_id: row.get(1)?,
            buddy_device_id: row.get(2)?,
            status: row.get(3)?,
            mirror_mood: row.get::<_, i32>(4)? != 0,
            created_at: row.get(5)?,
            accepted_at: row.get(6)?,
            device_name: row.get(7)?,
            buddy_device_name: row.get(8)?,
        })
    })?.filter_map(|r| r.ok()).collect();
    Ok(Json(rows))
}

/// POST /api/social/buddies — create a buddy pairing
pub async fn create_buddy(
    State(db): State<DbPool>,
    Json(input): Json<CreateBuddy>,
) -> Result<Json<Buddy>, AppError> {
    if input.device_id == input.buddy_device_id {
        return Err(AppError::BadRequest("Cannot buddy with yourself".into()));
    }
    let conn = db.lock().unwrap();
    let id = uuid::Uuid::new_v4().to_string();
    let mirror = if input.mirror_mood.unwrap_or(true) { 1 } else { 0 };

    conn.execute(
        "INSERT INTO buddies (id, device_id, buddy_device_id, mirror_mood) VALUES (?1, ?2, ?3, ?4)",
        rusqlite::params![id, input.device_id, input.buddy_device_id, mirror],
    )?;

    let buddy = conn.query_row(
        "SELECT b.id, b.device_id, b.buddy_device_id, b.status, b.mirror_mood, \
         b.created_at, b.accepted_at, d1.name, d2.name \
         FROM buddies b \
         LEFT JOIN devices d1 ON d1.id = b.device_id \
         LEFT JOIN devices d2 ON d2.id = b.buddy_device_id \
         WHERE b.id = ?1",
        [&id],
        |row| Ok(Buddy {
            id: row.get(0)?,
            device_id: row.get(1)?,
            buddy_device_id: row.get(2)?,
            status: row.get(3)?,
            mirror_mood: row.get::<_, i32>(4)? != 0,
            created_at: row.get(5)?,
            accepted_at: row.get(6)?,
            device_name: row.get(7)?,
            buddy_device_name: row.get(8)?,
        }),
    )?;
    Ok(Json(buddy))
}

/// POST /api/social/buddies/{id}/accept — accept a buddy request
pub async fn accept_buddy(
    State(db): State<DbPool>,
    Path(id): Path<String>,
) -> Result<Json<serde_json::Value>, AppError> {
    let conn = db.lock().unwrap();
    let changed = conn.execute(
        "UPDATE buddies SET status = 'accepted', accepted_at = datetime('now') WHERE id = ?1 AND status = 'pending'",
        [&id],
    )?;
    if changed == 0 {
        return Err(AppError::NotFound(format!("Buddy request {id} not found or already accepted")));
    }
    Ok(Json(json!({ "ok": true })))
}

/// DELETE /api/social/buddies/{id} — remove a buddy pairing
pub async fn delete_buddy(
    State(db): State<DbPool>,
    Path(id): Path<String>,
) -> Result<Json<serde_json::Value>, AppError> {
    let conn = db.lock().unwrap();
    conn.execute("DELETE FROM buddies WHERE id = ?1", [&id])?;
    Ok(Json(json!({ "ok": true })))
}

// ========================
// Raid Mode
// ========================

/// GET /api/social/raids — list raids
pub async fn list_raids(
    State(db): State<DbPool>,
) -> Result<Json<Vec<Raid>>, AppError> {
    let conn = db.lock().unwrap();
    let mut stmt = conn.prepare(
        "SELECT r.id, r.from_device_id, r.to_device_id, r.message, r.avatar_state, \
         r.duration_secs, r.status, r.created_at, r.expires_at, d1.name, d2.name \
         FROM raids r \
         LEFT JOIN devices d1 ON d1.id = r.from_device_id \
         LEFT JOIN devices d2 ON d2.id = r.to_device_id \
         ORDER BY r.created_at DESC LIMIT 50"
    )?;
    let rows = stmt.query_map([], |row| {
        Ok(Raid {
            id: row.get(0)?,
            from_device_id: row.get(1)?,
            to_device_id: row.get(2)?,
            message: row.get(3)?,
            avatar_state: row.get(4)?,
            duration_secs: row.get(5)?,
            status: row.get(6)?,
            created_at: row.get(7)?,
            expires_at: row.get(8)?,
            from_device_name: row.get(9)?,
            to_device_name: row.get(10)?,
        })
    })?.filter_map(|r| r.ok()).collect();
    Ok(Json(rows))
}

/// POST /api/social/raids — send a raid
pub async fn create_raid(
    State(db): State<DbPool>,
    Json(input): Json<CreateRaid>,
) -> Result<Json<Raid>, AppError> {
    let conn = db.lock().unwrap();
    let id = uuid::Uuid::new_v4().to_string();
    let message = input.message.unwrap_or_default();
    let avatar_state = input.avatar_state.unwrap_or_else(|| "happy".to_string());
    let duration = input.duration_secs.unwrap_or(30);

    conn.execute(
        "INSERT INTO raids (id, from_device_id, to_device_id, message, avatar_state, duration_secs, expires_at) \
         VALUES (?1, ?2, ?3, ?4, ?5, ?6, datetime('now', '+' || ?6 || ' seconds'))",
        rusqlite::params![id, input.from_device_id, input.to_device_id, message, avatar_state, duration],
    )?;

    let raid = conn.query_row(
        "SELECT r.id, r.from_device_id, r.to_device_id, r.message, r.avatar_state, \
         r.duration_secs, r.status, r.created_at, r.expires_at, d1.name, d2.name \
         FROM raids r \
         LEFT JOIN devices d1 ON d1.id = r.from_device_id \
         LEFT JOIN devices d2 ON d2.id = r.to_device_id \
         WHERE r.id = ?1",
        [&id],
        |row| Ok(Raid {
            id: row.get(0)?,
            from_device_id: row.get(1)?,
            to_device_id: row.get(2)?,
            message: row.get(3)?,
            avatar_state: row.get(4)?,
            duration_secs: row.get(5)?,
            status: row.get(6)?,
            created_at: row.get(7)?,
            expires_at: row.get(8)?,
            from_device_name: row.get(9)?,
            to_device_name: row.get(10)?,
        }),
    )?;
    Ok(Json(raid))
}

// ========================
// Shared Streaks
// ========================

/// GET /api/social/shared-streaks — list shared streak challenges
pub async fn list_shared_streaks(
    State(db): State<DbPool>,
) -> Result<Json<Vec<SharedStreak>>, AppError> {
    let conn = db.lock().unwrap();
    let mut stmt = conn.prepare(
        "SELECT id, name, device_ids, current_streak, longest_streak, last_active_date, created_at, updated_at \
         FROM shared_streaks ORDER BY created_at DESC"
    )?;
    let rows = stmt.query_map([], |row| {
        let ids_json: String = row.get(2)?;
        let device_ids: Vec<String> = serde_json::from_str(&ids_json).unwrap_or_default();
        Ok(SharedStreak {
            id: row.get(0)?,
            name: row.get(1)?,
            device_ids,
            current_streak: row.get(3)?,
            longest_streak: row.get(4)?,
            last_active_date: row.get(5)?,
            created_at: row.get(6)?,
            updated_at: row.get(7)?,
        })
    })?.filter_map(|r| r.ok()).collect();
    Ok(Json(rows))
}

/// POST /api/social/shared-streaks — create a shared streak challenge
pub async fn create_shared_streak(
    State(db): State<DbPool>,
    Json(input): Json<CreateSharedStreak>,
) -> Result<Json<SharedStreak>, AppError> {
    if input.device_ids.len() < 2 {
        return Err(AppError::BadRequest("Need at least 2 devices for a shared streak".into()));
    }
    let conn = db.lock().unwrap();
    let id = uuid::Uuid::new_v4().to_string();
    let ids_json = serde_json::to_string(&input.device_ids).unwrap_or_else(|_| "[]".to_string());

    conn.execute(
        "INSERT INTO shared_streaks (id, name, device_ids) VALUES (?1, ?2, ?3)",
        rusqlite::params![id, input.name, ids_json],
    )?;

    let streak = conn.query_row(
        "SELECT id, name, device_ids, current_streak, longest_streak, last_active_date, created_at, updated_at \
         FROM shared_streaks WHERE id = ?1",
        [&id],
        |row| {
            let ids_json: String = row.get(2)?;
            let device_ids: Vec<String> = serde_json::from_str(&ids_json).unwrap_or_default();
            Ok(SharedStreak {
                id: row.get(0)?,
                name: row.get(1)?,
                device_ids,
                current_streak: row.get(3)?,
                longest_streak: row.get(4)?,
                last_active_date: row.get(5)?,
                created_at: row.get(6)?,
                updated_at: row.get(7)?,
            })
        },
    )?;
    Ok(Json(streak))
}

/// DELETE /api/social/shared-streaks/{id} — delete a shared streak
pub async fn delete_shared_streak(
    State(db): State<DbPool>,
    Path(id): Path<String>,
) -> Result<Json<serde_json::Value>, AppError> {
    let conn = db.lock().unwrap();
    conn.execute("DELETE FROM shared_streaks WHERE id = ?1", [&id])?;
    Ok(Json(json!({ "ok": true })))
}

// ========================
// Live Coding Indicator / Presence
// ========================

/// GET /api/social/presence — list all device coding presence
pub async fn list_presence(
    State(db): State<DbPool>,
) -> Result<Json<Vec<CodingPresence>>, AppError> {
    let conn = db.lock().unwrap();
    let mut stmt = conn.prepare(
        "SELECT p.device_id, p.is_coding, p.last_activity_at, p.current_state, p.updated_at, d.name \
         FROM coding_presence p \
         LEFT JOIN devices d ON d.id = p.device_id \
         ORDER BY p.is_coding DESC, p.last_activity_at DESC"
    )?;
    let rows = stmt.query_map([], |row| {
        Ok(CodingPresence {
            device_id: row.get(0)?,
            is_coding: row.get::<_, i32>(1)? != 0,
            last_activity_at: row.get(2)?,
            current_state: row.get(3)?,
            updated_at: row.get(4)?,
            device_name: row.get(5)?,
        })
    })?.filter_map(|r| r.ok()).collect();
    Ok(Json(rows))
}

/// POST /api/social/presence — update coding presence
pub async fn update_presence(
    State(db): State<DbPool>,
    Json(input): Json<UpdatePresence>,
) -> Result<Json<serde_json::Value>, AppError> {
    let conn = db.lock().unwrap();
    let is_coding = if input.is_coding { 1 } else { 0 };
    let state = input.current_state.unwrap_or_else(|| "coding".to_string());

    conn.execute(
        "INSERT INTO coding_presence (device_id, is_coding, last_activity_at, current_state, updated_at) \
         VALUES (?1, ?2, datetime('now'), ?3, datetime('now')) \
         ON CONFLICT(device_id) DO UPDATE SET \
         is_coding = ?2, last_activity_at = datetime('now'), current_state = ?3, updated_at = datetime('now')",
        rusqlite::params![input.device_id, is_coding, state],
    )?;
    Ok(Json(json!({ "ok": true })))
}

// ========================
// Team Dashboard
// ========================

/// GET /api/social/team — team dashboard showing all hookbots
pub async fn get_team_dashboard(
    State(db): State<DbPool>,
) -> Result<Json<TeamDashboard>, AppError> {
    let conn = db.lock().unwrap();
    let mut stmt = conn.prepare(
        "SELECT d.id, d.name, \
         COALESCE(p.is_coding, 0), COALESCE(p.current_state, 'idle'), p.last_activity_at, \
         COALESCE((SELECT SUM(amount) FROM xp_ledger WHERE device_id = d.id), 0), \
         COALESCE(s.current_streak, 0) \
         FROM devices d \
         LEFT JOIN coding_presence p ON p.device_id = d.id \
         LEFT JOIN streaks s ON s.device_id = d.id \
         ORDER BY p.is_coding DESC, d.name"
    )?;
    let members: Vec<TeamMember> = stmt.query_map([], |row| {
        let total_xp: i64 = row.get(5)?;
        let level = level_from_xp(total_xp);
        Ok(TeamMember {
            device_id: row.get(0)?,
            device_name: row.get(1)?,
            is_coding: row.get::<_, i32>(2)? != 0,
            current_state: row.get(3)?,
            last_activity_at: row.get(4)?,
            level,
            total_xp,
            current_streak: row.get(6)?,
        })
    })?.filter_map(|r| r.ok()).collect();

    let active_count = members.iter().filter(|m| m.is_coding).count() as i64;
    let total_count = members.len() as i64;

    Ok(Json(TeamDashboard { members, active_count, total_count }))
}

// ========================
// Hookbot Reactions
// ========================

/// GET /api/social/reactions — list reactions for a device
pub async fn list_reactions(
    State(db): State<DbPool>,
    Query(q): Query<GlobalEventQuery>,
) -> Result<Json<Vec<Reaction>>, AppError> {
    let conn = db.lock().unwrap();
    let limit = q.limit.unwrap_or(50);
    let mut stmt = conn.prepare(
        "SELECT r.id, r.from_device_id, r.to_device_id, r.reaction, r.message, r.delivered, \
         r.created_at, d.name \
         FROM reactions r \
         LEFT JOIN devices d ON d.id = r.from_device_id \
         ORDER BY r.created_at DESC LIMIT ?1"
    )?;
    let rows = stmt.query_map([limit], |row| {
        Ok(Reaction {
            id: row.get(0)?,
            from_device_id: row.get(1)?,
            to_device_id: row.get(2)?,
            reaction: row.get(3)?,
            message: row.get(4)?,
            delivered: row.get::<_, i32>(5)? != 0,
            created_at: row.get(6)?,
            from_device_name: row.get(7)?,
        })
    })?.filter_map(|r| r.ok()).collect();
    Ok(Json(rows))
}

/// POST /api/social/reactions — send a reaction to a hookbot
pub async fn send_reaction(
    State(db): State<DbPool>,
    Json(input): Json<SendReaction>,
) -> Result<Json<Reaction>, AppError> {
    let valid_reactions = [
        "fireworks", "skull", "heart", "fire", "rocket", "party",
        "thumbsup", "clap", "eyes", "100", "bug", "ship",
    ];
    if !valid_reactions.contains(&input.reaction.as_str()) {
        return Err(AppError::BadRequest(format!(
            "Invalid reaction '{}'. Valid: {:?}", input.reaction, valid_reactions
        )));
    }

    let conn = db.lock().unwrap();
    conn.execute(
        "INSERT INTO reactions (from_device_id, to_device_id, reaction, message) VALUES (?1, ?2, ?3, ?4)",
        rusqlite::params![input.from_device_id, input.to_device_id, input.reaction, input.message],
    )?;
    let id = conn.last_insert_rowid();

    let reaction = conn.query_row(
        "SELECT r.id, r.from_device_id, r.to_device_id, r.reaction, r.message, r.delivered, \
         r.created_at, d.name \
         FROM reactions r \
         LEFT JOIN devices d ON d.id = r.from_device_id \
         WHERE r.id = ?1",
        [id],
        |row| Ok(Reaction {
            id: row.get(0)?,
            from_device_id: row.get(1)?,
            to_device_id: row.get(2)?,
            reaction: row.get(3)?,
            message: row.get(4)?,
            delivered: row.get::<_, i32>(5)? != 0,
            created_at: row.get(6)?,
            from_device_name: row.get(7)?,
        }),
    )?;
    Ok(Json(reaction))
}

// ========================
// Global Event Wall
// ========================

/// GET /api/social/events — list global events
pub async fn list_global_events(
    State(db): State<DbPool>,
    Query(q): Query<GlobalEventQuery>,
) -> Result<Json<Vec<GlobalEvent>>, AppError> {
    let conn = db.lock().unwrap();
    let limit = q.limit.unwrap_or(50);

    let (sql, params): (String, Vec<Box<dyn rusqlite::types::ToSql>>) = if let Some(ref et) = q.event_type {
        (
            "SELECT id, device_id, event_type, message, anonymous, device_name, created_at \
             FROM global_events WHERE event_type = ?1 ORDER BY created_at DESC LIMIT ?2".into(),
            vec![Box::new(et.clone()) as Box<dyn rusqlite::types::ToSql>, Box::new(limit)],
        )
    } else {
        (
            "SELECT id, device_id, event_type, message, anonymous, device_name, created_at \
             FROM global_events ORDER BY created_at DESC LIMIT ?1".into(),
            vec![Box::new(limit) as Box<dyn rusqlite::types::ToSql>],
        )
    };

    let param_refs: Vec<&dyn rusqlite::types::ToSql> = params.iter().map(|p| p.as_ref()).collect();
    let mut stmt = conn.prepare(&sql)?;
    let rows = stmt.query_map(param_refs.as_slice(), |row| {
        Ok(GlobalEvent {
            id: row.get(0)?,
            device_id: row.get(1)?,
            event_type: row.get(2)?,
            message: row.get(3)?,
            anonymous: row.get::<_, i32>(4)? != 0,
            device_name: row.get(5)?,
            created_at: row.get(6)?,
        })
    })?.filter_map(|r| r.ok()).collect();
    Ok(Json(rows))
}

/// POST /api/social/events — publish a global event
pub async fn create_global_event(
    State(db): State<DbPool>,
    Json(input): Json<CreateGlobalEvent>,
) -> Result<Json<GlobalEvent>, AppError> {
    let conn = db.lock().unwrap();
    let anonymous = if input.anonymous.unwrap_or(true) { 1 } else { 0 };

    let device_name: Option<String> = if let Some(ref did) = input.device_id {
        conn.query_row(
            "SELECT name FROM devices WHERE id = ?1",
            [did],
            |row| row.get(0),
        ).ok()
    } else {
        None
    };

    conn.execute(
        "INSERT INTO global_events (device_id, event_type, message, anonymous, device_name) \
         VALUES (?1, ?2, ?3, ?4, ?5)",
        rusqlite::params![input.device_id, input.event_type, input.message, anonymous, device_name],
    )?;
    let id = conn.last_insert_rowid();

    let event = conn.query_row(
        "SELECT id, device_id, event_type, message, anonymous, device_name, created_at \
         FROM global_events WHERE id = ?1",
        [id],
        |row| Ok(GlobalEvent {
            id: row.get(0)?,
            device_id: row.get(1)?,
            event_type: row.get(2)?,
            message: row.get(3)?,
            anonymous: row.get::<_, i32>(4)? != 0,
            device_name: row.get(5)?,
            created_at: row.get(6)?,
        }),
    )?;
    Ok(Json(event))
}
