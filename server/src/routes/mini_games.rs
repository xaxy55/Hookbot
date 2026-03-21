use axum::extract::{Path, Query, State};
use axum::Json;
use serde::{Deserialize, Serialize};
use serde_json::json;

use crate::db::DbPool;
use crate::error::AppError;

// ── OLED Mini-Games ─────────────────────────────────────────────
// Snake, Pong, Tetris — playable via web UI or physical buttons.
// Game state is stored server-side and can be pushed to device OLED.

#[derive(Debug, Serialize)]
pub struct GameScore {
    pub id: i64,
    pub device_id: String,
    pub game: String,
    pub score: i64,
    pub duration_secs: i64,
    pub played_at: String,
}

#[derive(Debug, Serialize)]
pub struct GameLeaderboard {
    pub game: String,
    pub scores: Vec<GameScore>,
    pub personal_best: Option<i64>,
}

#[derive(Debug, Deserialize)]
pub struct GameQuery {
    pub device_id: Option<String>,
    pub game: Option<String>,
    pub limit: Option<i64>,
}

fn resolve_device_id(conn: &rusqlite::Connection, device_id: Option<&str>) -> Result<String, AppError> {
    if let Some(id) = device_id {
        return Ok(id.to_string());
    }
    conn.query_row("SELECT id FROM devices ORDER BY created_at LIMIT 1", [], |row| row.get(0))
        .map_err(|_| AppError::NotFound("No devices registered".into()))
}

/// GET /api/games/scores — get game scores/leaderboard
pub async fn get_scores(
    State(db): State<DbPool>,
    Query(q): Query<GameQuery>,
) -> Result<Json<GameLeaderboard>, AppError> {
    let conn = db.lock().unwrap();
    let device_id = resolve_device_id(&conn, q.device_id.as_deref())?;
    let game = q.game.as_deref().unwrap_or("snake");
    let limit = q.limit.unwrap_or(20).min(100);

    let mut stmt = conn.prepare(
        "SELECT id, device_id, game, score, duration_secs, played_at FROM game_scores WHERE game = ?1 ORDER BY score DESC LIMIT ?2"
    )?;
    let scores = stmt.query_map(rusqlite::params![game, limit], |row| {
        Ok(GameScore {
            id: row.get(0)?,
            device_id: row.get(1)?,
            game: row.get(2)?,
            score: row.get(3)?,
            duration_secs: row.get(4)?,
            played_at: row.get(5)?,
        })
    })?.collect::<Result<Vec<_>, _>>()?;

    let personal_best: Option<i64> = conn.query_row(
        "SELECT MAX(score) FROM game_scores WHERE device_id = ?1 AND game = ?2",
        rusqlite::params![device_id, game], |r| r.get(0),
    ).ok().flatten();

    Ok(Json(GameLeaderboard { game: game.to_string(), scores, personal_best }))
}

#[derive(Debug, Deserialize)]
pub struct SubmitScoreRequest {
    pub device_id: Option<String>,
    pub game: String,
    pub score: i64,
    pub duration_secs: Option<i64>,
}

#[derive(Debug, Serialize)]
pub struct SubmitScoreResponse {
    pub ok: bool,
    pub new_high_score: bool,
    pub xp_earned: i64,
    pub rank: i64,
}

/// POST /api/games/scores — submit a game score
pub async fn submit_score(
    State(db): State<DbPool>,
    Json(input): Json<SubmitScoreRequest>,
) -> Result<Json<SubmitScoreResponse>, AppError> {
    let conn = db.lock().unwrap();
    let device_id = resolve_device_id(&conn, input.device_id.as_deref())?;
    let duration = input.duration_secs.unwrap_or(0);

    // Validate game name
    let valid_games = ["snake", "pong", "tetris"];
    if !valid_games.contains(&input.game.as_str()) {
        return Err(AppError::BadRequest(format!("Invalid game: {}. Valid: {:?}", input.game, valid_games)));
    }

    let prev_best: Option<i64> = conn.query_row(
        "SELECT MAX(score) FROM game_scores WHERE device_id = ?1 AND game = ?2",
        rusqlite::params![device_id, input.game], |r| r.get(0),
    ).ok().flatten();

    let new_high_score = prev_best.map_or(true, |b| input.score > b);

    conn.execute(
        "INSERT INTO game_scores (device_id, game, score, duration_secs) VALUES (?1, ?2, ?3, ?4)",
        rusqlite::params![device_id, input.game, input.score, duration],
    )?;

    // Award XP: base 5 + bonus for high scores
    let xp = if new_high_score { 15 } else { 5 };
    conn.execute(
        "INSERT INTO xp_ledger (device_id, amount, reason) VALUES (?1, ?2, ?3)",
        rusqlite::params![device_id, xp, format!("minigame:{}", input.game)],
    )?;

    // Calculate rank
    let rank: i64 = conn.query_row(
        "SELECT COUNT(*) + 1 FROM (SELECT DISTINCT device_id, MAX(score) as best FROM game_scores WHERE game = ?1 GROUP BY device_id HAVING best > ?2)",
        rusqlite::params![input.game, input.score], |r| r.get(0),
    )?;

    Ok(Json(SubmitScoreResponse { ok: true, new_high_score, xp_earned: xp, rank }))
}

/// GET /api/games — list available games with stats
pub async fn list_games(
    State(db): State<DbPool>,
    Query(q): Query<GameQuery>,
) -> Result<Json<Vec<serde_json::Value>>, AppError> {
    let conn = db.lock().unwrap();
    let device_id = resolve_device_id(&conn, q.device_id.as_deref()).ok();

    let games = vec!["snake", "pong", "tetris"];
    let mut result = Vec::new();

    for game in games {
        let total_plays: i64 = conn.query_row(
            "SELECT COUNT(*) FROM game_scores WHERE game = ?1", [game], |r| r.get(0),
        ).unwrap_or(0);

        let global_best: Option<i64> = conn.query_row(
            "SELECT MAX(score) FROM game_scores WHERE game = ?1", [game], |r| r.get(0),
        ).ok().flatten();

        let personal_best: Option<i64> = if let Some(ref did) = device_id {
            conn.query_row(
                "SELECT MAX(score) FROM game_scores WHERE device_id = ?1 AND game = ?2",
                rusqlite::params![did, game], |r| r.get(0),
            ).ok().flatten()
        } else { None };

        let personal_plays: i64 = if let Some(ref did) = device_id {
            conn.query_row(
                "SELECT COUNT(*) FROM game_scores WHERE device_id = ?1 AND game = ?2",
                rusqlite::params![did, game], |r| r.get(0),
            ).unwrap_or(0)
        } else { 0 };

        result.push(json!({
            "game": game,
            "total_plays": total_plays,
            "global_best": global_best,
            "personal_best": personal_best,
            "personal_plays": personal_plays,
        }));
    }

    Ok(Json(result))
}
