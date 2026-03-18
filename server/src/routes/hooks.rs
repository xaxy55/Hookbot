use axum::extract::State;
use axum::Json;
use serde_json::json;

use crate::db::DbPool;
use crate::error::AppError;
use crate::models::*;
use crate::services::proxy;
use super::gamification::record_tool_use_and_xp;

pub async fn handle_hook(
    State(db): State<DbPool>,
    Json(input): Json<HookEvent>,
) -> Result<Json<serde_json::Value>, AppError> {
    let state = match input.event.as_str() {
        "PreToolUse" => "thinking",
        "PostToolUse" => {
            let tool_output = input.tool_output.as_deref().unwrap_or("");
            let tool_name = input.tool_name.as_deref().unwrap_or("");
            let is_build_or_test = tool_name.to_lowercase().contains("bash")
                && (tool_output.to_lowercase().contains("passed")
                    || tool_output.to_lowercase().contains("success")
                    || tool_output.to_lowercase().contains("build succeeded"));
            if is_build_or_test { "success" } else { "idle" }
        }
        "UserPromptSubmit" => "thinking",
        "Stop" => "idle",
        "TaskCompleted" => "success",
        _ => "idle",
    };

    // Find target device IP and ID - scoped to drop MutexGuard before await
    // Priority: 1) project_routes lookup by project path, 2) device_id from request, 3) first device
    let (device_ip, device_id): (Option<String>, Option<String>) = {
        let conn = db.lock().unwrap();

        // Try project-based routing first
        let routed_device_id: Option<String> = input.project.as_ref().and_then(|project| {
            conn.query_row(
                "SELECT device_id FROM project_routes WHERE project_path = ?1",
                [project],
                |row| row.get(0),
            ).ok()
        });

        let effective_device_id = routed_device_id.or_else(|| input.device_id.clone());

        if let Some(ref did) = effective_device_id {
            let ip = conn.query_row(
                "SELECT ip_address FROM devices WHERE id = ?1", [did],
                |row| row.get(0),
            ).ok();
            (ip, Some(did.clone()))
        } else {
            let result: Option<(String, String)> = conn.query_row(
                "SELECT id, ip_address FROM devices ORDER BY created_at LIMIT 1", [],
                |row| Ok((row.get(0)?, row.get(1)?)),
            ).ok();
            match result {
                Some((id, ip)) => (Some(ip), Some(id)),
                None => (None, None),
            }
        }
    };

    // Record tool use and award XP
    let (xp_earned, new_badges) = {
        let conn = db.lock().unwrap();
        let tool_name = input.tool_name.as_deref().unwrap_or("unknown");
        record_tool_use_and_xp(
            &conn,
            device_id.as_deref(),
            tool_name,
            &input.event,
            input.project.as_deref(),
        ).unwrap_or((0, vec![]))
    };

    if let Some(ref ip) = device_ip {
        // Forward state
        let tool_name = input.tool_name.clone().unwrap_or_default();
        let body = json!({
            "state": state,
            "tool": tool_name,
            "detail": "",
        });
        let _ = proxy::forward_json(&format!("http://{}/state", ip), &body).await;

        // Forward tasks if present
        if let Some(ref tasks) = input.tasks {
            let tasks_body = json!({
                "items": tasks,
                "active": input.active_task.unwrap_or(0),
            });
            let _ = proxy::forward_json(&format!("http://{}/tasks", ip), &tasks_body).await;
        }

        // Push XP/level update to device for OLED display
        if let Some(ref did) = device_id {
            let total_xp: i64 = {
                let conn = db.lock().unwrap();
                conn.query_row(
                    "SELECT COALESCE(SUM(amount), 0) FROM xp_ledger WHERE device_id = ?1",
                    [did], |r| r.get(0),
                ).unwrap_or(0)
            };
            let level = super::gamification::level_from_xp(total_xp);
            let xp_current_level = super::gamification::xp_for_level(level);
            let xp_next_level = super::gamification::xp_for_level(level + 1);
            let progress = if xp_next_level > xp_current_level {
                ((total_xp - xp_current_level) as f64 / (xp_next_level - xp_current_level) as f64 * 100.0) as i64
            } else {
                100
            };

            let xp_body = json!({
                "level": level,
                "xp": total_xp,
                "progress": progress,
                "title": super::gamification::title_for_level(level),
            });
            let _ = proxy::forward_json(&format!("http://{}/xp", ip), &xp_body).await;
        }
    }

    Ok(Json(json!({
        "ok": true,
        "state": state,
        "xp_earned": xp_earned,
        "new_badges": new_badges,
    })))
}
