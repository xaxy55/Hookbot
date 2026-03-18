use axum::extract::{Path, State};
use axum::Json;
use serde_json::json;

use crate::db::DbPool;
use crate::error::AppError;
use crate::models::{AutomationRule, CreateRule, UpdateRule};

/// GET /api/devices/:id/rules - list automation rules for device
pub async fn list_rules(
    State(db): State<DbPool>,
    Path(id): Path<String>,
) -> Result<Json<Vec<AutomationRule>>, AppError> {
    let conn = db.lock().unwrap();
    let mut stmt = conn.prepare(
        "SELECT id, device_id, name, enabled, trigger_type, trigger_config,
                action_type, action_config, cooldown_secs, last_triggered_at, created_at
         FROM automation_rules WHERE device_id = ?1 ORDER BY created_at DESC",
    )?;

    let rules = stmt
        .query_map([&id], |row| {
            let trigger_config_str: String = row.get(5)?;
            let action_config_str: String = row.get(7)?;
            Ok(AutomationRule {
                id: row.get(0)?,
                device_id: row.get(1)?,
                name: row.get(2)?,
                enabled: row.get::<_, i32>(3)? != 0,
                trigger_type: row.get(4)?,
                trigger_config: serde_json::from_str(&trigger_config_str).unwrap_or_default(),
                action_type: row.get(6)?,
                action_config: serde_json::from_str(&action_config_str).unwrap_or_default(),
                cooldown_secs: row.get(8)?,
                last_triggered_at: row.get(9)?,
                created_at: row.get(10)?,
            })
        })?
        .collect::<Result<Vec<_>, _>>()?;

    Ok(Json(rules))
}

/// POST /api/devices/:id/rules - create a new automation rule
pub async fn create_rule(
    State(db): State<DbPool>,
    Path(id): Path<String>,
    Json(body): Json<CreateRule>,
) -> Result<Json<AutomationRule>, AppError> {
    let rule_id = uuid::Uuid::new_v4().to_string();
    let trigger_config_str = serde_json::to_string(&body.trigger_config).unwrap_or_else(|_| "{}".to_string());
    let action_config_str = serde_json::to_string(&body.action_config).unwrap_or_else(|_| "{}".to_string());
    let cooldown = body.cooldown_secs.unwrap_or(60);

    let conn = db.lock().unwrap();
    conn.execute(
        "INSERT INTO automation_rules (id, device_id, name, trigger_type, trigger_config, action_type, action_config, cooldown_secs)
         VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8)",
        rusqlite::params![rule_id, id, body.name, body.trigger_type, trigger_config_str, body.action_type, action_config_str, cooldown],
    )?;

    let rule = conn.query_row(
        "SELECT id, device_id, name, enabled, trigger_type, trigger_config,
                action_type, action_config, cooldown_secs, last_triggered_at, created_at
         FROM automation_rules WHERE id = ?1",
        [&rule_id],
        |row| {
            let tc: String = row.get(5)?;
            let ac: String = row.get(7)?;
            Ok(AutomationRule {
                id: row.get(0)?,
                device_id: row.get(1)?,
                name: row.get(2)?,
                enabled: row.get::<_, i32>(3)? != 0,
                trigger_type: row.get(4)?,
                trigger_config: serde_json::from_str(&tc).unwrap_or_default(),
                action_type: row.get(6)?,
                action_config: serde_json::from_str(&ac).unwrap_or_default(),
                cooldown_secs: row.get(8)?,
                last_triggered_at: row.get(9)?,
                created_at: row.get(10)?,
            })
        },
    )?;

    Ok(Json(rule))
}

/// PUT /api/devices/:id/rules/:rule_id - update rule fields
pub async fn update_rule(
    State(db): State<DbPool>,
    Path((id, rule_id)): Path<(String, String)>,
    Json(body): Json<UpdateRule>,
) -> Result<Json<AutomationRule>, AppError> {
    let conn = db.lock().unwrap();

    // Verify rule exists for this device
    let exists: bool = conn
        .query_row(
            "SELECT COUNT(*) FROM automation_rules WHERE id = ?1 AND device_id = ?2",
            rusqlite::params![rule_id, id],
            |row| row.get::<_, i32>(0),
        )
        .map(|c| c > 0)?;
    if !exists {
        return Err(AppError::NotFound(format!(
            "Rule {rule_id} not found for device {id}"
        )));
    }

    if let Some(name) = &body.name {
        conn.execute(
            "UPDATE automation_rules SET name = ?1 WHERE id = ?2",
            rusqlite::params![name, rule_id],
        )?;
    }
    if let Some(enabled) = body.enabled {
        conn.execute(
            "UPDATE automation_rules SET enabled = ?1 WHERE id = ?2",
            rusqlite::params![enabled as i32, rule_id],
        )?;
    }
    if let Some(tt) = &body.trigger_type {
        conn.execute(
            "UPDATE automation_rules SET trigger_type = ?1 WHERE id = ?2",
            rusqlite::params![tt, rule_id],
        )?;
    }
    if let Some(tc) = &body.trigger_config {
        let s = serde_json::to_string(tc).unwrap_or_else(|_| "{}".to_string());
        conn.execute(
            "UPDATE automation_rules SET trigger_config = ?1 WHERE id = ?2",
            rusqlite::params![s, rule_id],
        )?;
    }
    if let Some(at) = &body.action_type {
        conn.execute(
            "UPDATE automation_rules SET action_type = ?1 WHERE id = ?2",
            rusqlite::params![at, rule_id],
        )?;
    }
    if let Some(ac) = &body.action_config {
        let s = serde_json::to_string(ac).unwrap_or_else(|_| "{}".to_string());
        conn.execute(
            "UPDATE automation_rules SET action_config = ?1 WHERE id = ?2",
            rusqlite::params![s, rule_id],
        )?;
    }
    if let Some(cd) = body.cooldown_secs {
        conn.execute(
            "UPDATE automation_rules SET cooldown_secs = ?1 WHERE id = ?2",
            rusqlite::params![cd, rule_id],
        )?;
    }

    let rule = conn.query_row(
        "SELECT id, device_id, name, enabled, trigger_type, trigger_config,
                action_type, action_config, cooldown_secs, last_triggered_at, created_at
         FROM automation_rules WHERE id = ?1",
        [&rule_id],
        |row| {
            let tc: String = row.get(5)?;
            let ac: String = row.get(7)?;
            Ok(AutomationRule {
                id: row.get(0)?,
                device_id: row.get(1)?,
                name: row.get(2)?,
                enabled: row.get::<_, i32>(3)? != 0,
                trigger_type: row.get(4)?,
                trigger_config: serde_json::from_str(&tc).unwrap_or_default(),
                action_type: row.get(6)?,
                action_config: serde_json::from_str(&ac).unwrap_or_default(),
                cooldown_secs: row.get(8)?,
                last_triggered_at: row.get(9)?,
                created_at: row.get(10)?,
            })
        },
    )?;

    Ok(Json(rule))
}

/// DELETE /api/devices/:id/rules/:rule_id - delete rule
pub async fn delete_rule(
    State(db): State<DbPool>,
    Path((id, rule_id)): Path<(String, String)>,
) -> Result<Json<serde_json::Value>, AppError> {
    let conn = db.lock().unwrap();
    let rows = conn.execute(
        "DELETE FROM automation_rules WHERE id = ?1 AND device_id = ?2",
        rusqlite::params![rule_id, id],
    )?;
    if rows == 0 {
        return Err(AppError::NotFound(format!(
            "Rule {rule_id} not found for device {id}"
        )));
    }
    Ok(Json(json!({ "ok": true })))
}
