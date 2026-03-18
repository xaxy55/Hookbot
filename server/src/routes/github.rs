use axum::extract::{Query, State};
use axum::http::HeaderMap;
use axum::Json;
use serde_json::{json, Value};

use crate::db::DbPool;
use crate::error::AppError;
use crate::models::GitHubHookQuery;
use crate::services::proxy;
use super::gamification::record_tool_use_and_xp;

/// Map a GitHub webhook event + payload to an avatar state.
fn map_github_event(event: &str, body: &Value) -> (&'static str, String) {
    let action = body["action"].as_str().unwrap_or("");
    let repo = body["repository"]["full_name"].as_str().unwrap_or("unknown");

    let state = match event {
        "push" => "success",

        "pull_request" => match action {
            "opened" | "reopened" => "thinking",
            "closed" => {
                if body["pull_request"]["merged"].as_bool().unwrap_or(false) {
                    "success"
                } else {
                    "idle"
                }
            }
            _ => "idle",
        },

        "issues" => match action {
            "opened" => "thinking",
            "closed" => "success",
            _ => "idle",
        },

        "workflow_run" => {
            let conclusion = body["workflow_run"]["conclusion"].as_str().unwrap_or("");
            match (action, conclusion) {
                ("completed", "success") => "success",
                ("completed", "failure") | ("completed", "timed_out") => "error",
                _ => "idle",
            }
        }

        "check_run" => {
            let conclusion = body["check_run"]["conclusion"].as_str().unwrap_or("");
            match (action, conclusion) {
                ("completed", "success") => "success",
                ("completed", "failure") | ("completed", "timed_out") => "error",
                _ => "idle",
            }
        }

        "star" => match action {
            "created" => "success",
            _ => "idle",
        },

        "ping" => "success",

        _ => "idle",
    };

    // Build a descriptive tool name for activity tracking
    let tool_name = match event {
        "push" => {
            let branch = body["ref"].as_str().unwrap_or("").replace("refs/heads/", "");
            format!("github:push:{}", branch)
        }
        "pull_request" => {
            let number = body["pull_request"]["number"].as_u64().unwrap_or(0);
            format!("github:pr:{}#{}", action, number)
        }
        "issues" => {
            let number = body["issue"]["number"].as_u64().unwrap_or(0);
            format!("github:issue:{}#{}", action, number)
        }
        "workflow_run" => {
            let name = body["workflow_run"]["name"].as_str().unwrap_or("workflow");
            format!("github:ci:{}", name)
        }
        "check_run" => {
            let name = body["check_run"]["name"].as_str().unwrap_or("check");
            format!("github:check:{}", name)
        }
        "star" => format!("github:star:{}", repo),
        "ping" => "github:ping".to_string(),
        other => format!("github:{}", other),
    };

    (state, tool_name)
}

pub async fn handle_github_hook(
    State(db): State<DbPool>,
    headers: HeaderMap,
    query: Query<GitHubHookQuery>,
    Json(body): Json<Value>,
) -> Result<Json<Value>, AppError> {
    let event = headers
        .get("x-github-event")
        .and_then(|v| v.to_str().ok())
        .unwrap_or("unknown");

    let (state, tool_name) = map_github_event(event, &body);

    let repo = body["repository"]["full_name"]
        .as_str()
        .unwrap_or("unknown")
        .to_string();

    // Find target device — same logic as hooks.rs
    let (device_ip, device_id): (Option<String>, Option<String>) = {
        let conn = db.lock().unwrap();
        if let Some(ref did) = query.device_id {
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
        record_tool_use_and_xp(
            &conn,
            device_id.as_deref(),
            &tool_name,
            event,
            Some(&repo),
        ).unwrap_or((0, vec![]))
    };

    if let Some(ref ip) = device_ip {
        // Forward state to device
        let state_body = json!({
            "state": state,
            "tool": tool_name,
            "detail": format!("github:{}", event),
        });
        let _ = proxy::forward_json(&format!("http://{}/state", ip), &state_body).await;

        // Push XP/level update
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
        "event": event,
        "state": state,
        "tool": tool_name,
        "xp_earned": xp_earned,
        "new_badges": new_badges,
    })))
}
