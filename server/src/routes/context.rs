use axum::extract::State;
use axum::Json;

use crate::db::DbPool;
use crate::error::AppError;
use crate::models::DeviceContext;

/// GET /api/context - detect current developer context from recent tool usage
pub async fn get_context(
    State(db): State<DbPool>,
) -> Result<Json<DeviceContext>, AppError> {
    let conn = db.lock().unwrap();
    let mut stmt = conn.prepare(
        "SELECT tool_name FROM tool_uses ORDER BY created_at DESC LIMIT 20",
    )?;

    let recent_tools: Vec<String> = stmt
        .query_map([], |row| row.get::<_, String>(0))?
        .filter_map(|r| r.ok())
        .collect();

    let (context, confidence) = detect_context(&recent_tools);

    Ok(Json(DeviceContext {
        context,
        confidence,
        recent_tools,
    }))
}

fn detect_context(tools: &[String]) -> (String, f64) {
    if tools.is_empty() {
        return ("idle".to_string(), 1.0);
    }

    let total = tools.len() as f64;

    let search_count = tools.iter().filter(|t| {
        let tl = t.to_lowercase();
        tl.contains("search") || tl.contains("grep") || tl.contains("find") || tl.contains("glob")
    }).count() as f64;

    let code_count = tools.iter().filter(|t| {
        let tl = t.to_lowercase();
        tl.contains("edit") || tl.contains("write") || tl.contains("create") || tl.contains("insert")
    }).count() as f64;

    let test_count = tools.iter().filter(|t| {
        let tl = t.to_lowercase();
        tl.contains("test") || tl.contains("run") || tl.contains("exec") || tl.contains("bash")
    }).count() as f64;

    let debug_count = tools.iter().filter(|t| {
        let tl = t.to_lowercase();
        tl.contains("debug") || tl.contains("log") || tl.contains("inspect") || tl.contains("console")
    }).count() as f64;

    let review_count = tools.iter().filter(|t| {
        let tl = t.to_lowercase();
        tl.contains("read") || tl.contains("diff") || tl.contains("review") || tl.contains("git")
    }).count() as f64;

    let categories = vec![
        ("searching", search_count),
        ("coding", code_count),
        ("testing", test_count),
        ("debugging", debug_count),
        ("reviewing", review_count),
    ];

    let (best_context, best_count) = categories
        .into_iter()
        .max_by(|a, b| a.1.partial_cmp(&b.1).unwrap_or(std::cmp::Ordering::Equal))
        .unwrap_or(("idle", 0.0));

    if best_count == 0.0 {
        return ("idle".to_string(), 0.5);
    }

    let confidence = (best_count / total).min(1.0);
    (best_context.to_string(), confidence)
}
