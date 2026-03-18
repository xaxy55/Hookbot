use axum::extract::{Path, State};
use axum::Json;
use serde_json::json;
use uuid::Uuid;

use crate::db::DbPool;
use crate::error::AppError;
use crate::models::*;

pub async fn list_routes(
    State(db): State<DbPool>,
) -> Result<Json<Vec<ProjectRouteWithDevice>>, AppError> {
    let conn = db.lock().unwrap();
    let mut stmt = conn.prepare(
        "SELECT pr.id, pr.project_path, pr.device_id, pr.label, pr.created_at, d.name
         FROM project_routes pr
         LEFT JOIN devices d ON d.id = pr.device_id
         ORDER BY pr.created_at DESC"
    )?;

    let routes = stmt.query_map([], |row| {
        Ok(ProjectRouteWithDevice {
            id: row.get(0)?,
            project_path: row.get(1)?,
            device_id: row.get(2)?,
            label: row.get(3)?,
            created_at: row.get(4)?,
            device_name: row.get(5)?,
        })
    })?.collect::<Result<Vec<_>, _>>()?;

    Ok(Json(routes))
}

pub async fn create_route(
    State(db): State<DbPool>,
    Json(input): Json<CreateProjectRoute>,
) -> Result<Json<ProjectRoute>, AppError> {
    let id = Uuid::new_v4().to_string();
    let conn = db.lock().unwrap();

    conn.execute(
        "INSERT INTO project_routes (id, project_path, device_id, label) VALUES (?1, ?2, ?3, ?4)",
        rusqlite::params![id, input.project_path, input.device_id, input.label],
    )?;

    let route = conn.query_row(
        "SELECT id, project_path, device_id, label, created_at FROM project_routes WHERE id = ?1",
        [&id],
        |row| Ok(ProjectRoute {
            id: row.get(0)?,
            project_path: row.get(1)?,
            device_id: row.get(2)?,
            label: row.get(3)?,
            created_at: row.get(4)?,
        }),
    )?;

    Ok(Json(route))
}

pub async fn update_route(
    State(db): State<DbPool>,
    Path(id): Path<String>,
    Json(input): Json<UpdateProjectRoute>,
) -> Result<Json<ProjectRoute>, AppError> {
    let conn = db.lock().unwrap();

    if let Some(ref project_path) = input.project_path {
        conn.execute(
            "UPDATE project_routes SET project_path = ?1 WHERE id = ?2",
            rusqlite::params![project_path, id],
        )?;
    }
    if let Some(ref device_id) = input.device_id {
        conn.execute(
            "UPDATE project_routes SET device_id = ?1 WHERE id = ?2",
            rusqlite::params![device_id, id],
        )?;
    }
    if let Some(ref label) = input.label {
        conn.execute(
            "UPDATE project_routes SET label = ?1 WHERE id = ?2",
            rusqlite::params![label, id],
        )?;
    }

    let route = conn.query_row(
        "SELECT id, project_path, device_id, label, created_at FROM project_routes WHERE id = ?1",
        [&id],
        |row| Ok(ProjectRoute {
            id: row.get(0)?,
            project_path: row.get(1)?,
            device_id: row.get(2)?,
            label: row.get(3)?,
            created_at: row.get(4)?,
        }),
    )?;

    Ok(Json(route))
}

pub async fn delete_route(
    State(db): State<DbPool>,
    Path(id): Path<String>,
) -> Result<Json<serde_json::Value>, AppError> {
    let conn = db.lock().unwrap();
    conn.execute("DELETE FROM project_routes WHERE id = ?1", [&id])?;
    Ok(Json(json!({ "ok": true })))
}
