use axum::extract::{Query, State};
use axum::Json;
use serde::{Deserialize, Serialize};
use serde_json::json;

use crate::db::DbPool;
use crate::error::AppError;

// ── Easter Eggs: Konami Code, Loot Drops, Seasonal Events ──────

// ── Konami Code ────────────────────────────────────────────────

#[derive(Debug, Deserialize)]
pub struct KonamiRequest {
    pub device_id: Option<String>,
    pub sequence: Vec<String>, // ["up","up","down","down","left","right","left","right","b","a"]
}

const KONAMI_SEQUENCE: &[&str] = &["up", "up", "down", "down", "left", "right", "left", "right", "b", "a"];

#[derive(Debug, Serialize)]
pub struct KonamiResponse {
    pub ok: bool,
    pub valid: bool,
    pub message: String,
    pub achievement_unlocked: bool,
    pub secret_animation: Option<String>,
}

fn resolve_device_id(conn: &rusqlite::Connection, device_id: Option<&str>) -> Result<String, AppError> {
    if let Some(id) = device_id {
        return Ok(id.to_string());
    }
    conn.query_row("SELECT id FROM devices ORDER BY created_at LIMIT 1", [], |row| row.get(0))
        .map_err(|_| AppError::NotFound("No devices registered".into()))
}

/// POST /api/easter-eggs/konami — validate konami code
pub async fn konami_code(
    State(db): State<DbPool>,
    Json(input): Json<KonamiRequest>,
) -> Result<Json<KonamiResponse>, AppError> {
    let conn = db.lock().unwrap();
    let device_id = resolve_device_id(&conn, input.device_id.as_deref())?;

    let valid = input.sequence.len() == KONAMI_SEQUENCE.len()
        && input.sequence.iter().zip(KONAMI_SEQUENCE.iter()).all(|(a, b)| a == b);

    if !valid {
        return Ok(Json(KonamiResponse {
            ok: true, valid: false,
            message: "Not the right sequence...".to_string(),
            achievement_unlocked: false,
            secret_animation: None,
        }));
    }

    // Check if already unlocked
    let already: bool = conn.query_row(
        "SELECT COUNT(*) > 0 FROM achievements WHERE device_id = ?1 AND badge_id = 'konami_master'",
        [&device_id], |r| r.get(0),
    ).unwrap_or(false);

    let achievement_unlocked = if !already {
        conn.execute(
            "INSERT OR IGNORE INTO achievements (device_id, badge_id) VALUES (?1, 'konami_master')",
            [&device_id],
        )?;
        conn.execute(
            "INSERT INTO xp_ledger (device_id, amount, reason) VALUES (?1, 100, 'easter_egg:konami')",
            [&device_id],
        )?;
        true
    } else { false };

    let secret_animation = serde_json::to_string(&json!({
        "name": "konami_secret",
        "frames": [
            {"state": "happy", "duration_ms": 200, "accessories": ["party_hat", "sunglasses"]},
            {"state": "excited", "duration_ms": 200, "accessories": ["crown", "cape"]},
            {"state": "celebrating", "duration_ms": 300, "accessories": ["fireworks"]},
            {"state": "happy", "duration_ms": 500, "accessories": ["rainbow"]},
        ],
        "loop_count": 3
    })).ok();

    Ok(Json(KonamiResponse {
        ok: true, valid: true,
        message: if achievement_unlocked {
            "KONAMI CODE ACTIVATED! Secret achievement unlocked! +100 XP".to_string()
        } else {
            "KONAMI CODE ACTIVATED! (Achievement already unlocked)".to_string()
        },
        achievement_unlocked,
        secret_animation,
    }))
}

// ── Avatar Evolution ────────────────────────────────────────────

#[derive(Debug, Serialize)]
pub struct EvolutionState {
    pub device_id: String,
    pub current_form: String,
    pub current_level: i64,
    pub forms: Vec<EvolutionForm>,
    pub next_evolution_level: Option<i64>,
    pub xp_to_next: i64,
}

#[derive(Debug, Serialize)]
pub struct EvolutionForm {
    pub name: String,
    pub level_required: i64,
    pub unlocked: bool,
    pub description: String,
    pub avatar_config: serde_json::Value,
}

const EVOLUTION_FORMS: &[(&str, i64, &str)] = &[
    ("egg", 0, "A mysterious digital egg, full of potential"),
    ("blob", 10, "A cheerful blob that bounces with every commit"),
    ("robot", 20, "A determined robot, precise and efficient"),
    ("mech", 30, "A powerful mech suit, ready for any challenge"),
    ("cosmic", 40, "A cosmic entity that transcends mere code"),
];

#[derive(Debug, Deserialize)]
pub struct DeviceQuery {
    pub device_id: Option<String>,
}

/// GET /api/evolution — get avatar evolution state
pub async fn get_evolution(
    State(db): State<DbPool>,
    Query(q): Query<DeviceQuery>,
) -> Result<Json<EvolutionState>, AppError> {
    let conn = db.lock().unwrap();
    let device_id = resolve_device_id(&conn, q.device_id.as_deref())?;

    let total_xp: i64 = conn.query_row(
        "SELECT COALESCE(SUM(amount), 0) FROM xp_ledger WHERE device_id = ?1",
        [&device_id], |r| r.get(0),
    ).unwrap_or(0);

    let level = crate::routes::gamification::level_from_xp(total_xp);

    let forms: Vec<EvolutionForm> = EVOLUTION_FORMS.iter().map(|(name, req_level, desc)| {
        EvolutionForm {
            name: name.to_string(),
            level_required: *req_level,
            unlocked: level >= *req_level,
            description: desc.to_string(),
            avatar_config: json!({
                "form": name,
                "eye_style": match *name {
                    "egg" => "dots",
                    "blob" => "round",
                    "robot" => "square",
                    "mech" => "visor",
                    "cosmic" => "stars",
                    _ => "round",
                },
                "body_scale": match *name {
                    "egg" => 0.6,
                    "blob" => 0.8,
                    "robot" => 1.0,
                    "mech" => 1.2,
                    "cosmic" => 1.4,
                    _ => 1.0,
                }
            }),
        }
    }).collect();

    let current_form = EVOLUTION_FORMS.iter().rev()
        .find(|(_, req, _)| level >= *req)
        .map(|(name, _, _)| name.to_string())
        .unwrap_or_else(|| "egg".to_string());

    let next_evolution_level = EVOLUTION_FORMS.iter()
        .find(|(_, req, _)| *req > level)
        .map(|(_, req, _)| *req);

    let xp_to_next = if let Some(next_level) = next_evolution_level {
        crate::routes::gamification::xp_for_level(next_level) - total_xp
    } else { 0 };

    Ok(Json(EvolutionState {
        device_id,
        current_form,
        current_level: level,
        forms,
        next_evolution_level,
        xp_to_next: xp_to_next.max(0),
    }))
}

// ── Loot Drops ──────────────────────────────────────────────────

#[derive(Debug, Serialize)]
pub struct LootDrop {
    pub id: i64,
    pub device_id: String,
    pub item_type: String,     // "accessory", "animation", "title", "color"
    pub item_id: String,
    pub item_name: String,
    pub rarity: String,        // "common", "uncommon", "rare", "epic", "legendary"
    pub dropped_at: String,
    pub claimed: bool,
}

const LOOT_TABLE: &[(&str, &str, &str, &str, f64)] = &[
    // (item_id, item_name, item_type, rarity, drop_weight)
    ("tiny_hat", "Tiny Hat", "accessory", "common", 20.0),
    ("pixel_shades", "Pixel Shades", "accessory", "common", 18.0),
    ("bow_tie", "Bow Tie", "accessory", "common", 18.0),
    ("party_hat", "Party Hat", "accessory", "uncommon", 10.0),
    ("monocle", "Monocle", "accessory", "uncommon", 10.0),
    ("headphones", "Headphones", "accessory", "uncommon", 9.0),
    ("wizard_hat", "Wizard Hat", "accessory", "rare", 5.0),
    ("ninja_mask", "Ninja Mask", "accessory", "rare", 4.0),
    ("halo", "Halo", "accessory", "rare", 4.0),
    ("crown_of_commits", "Crown of Commits", "accessory", "epic", 1.5),
    ("flame_aura", "Flame Aura", "animation", "epic", 1.0),
    ("rainbow_trail", "Rainbow Trail", "animation", "epic", 1.0),
    ("cosmic_glow", "Cosmic Glow", "animation", "legendary", 0.3),
    ("golden_keyboard", "Golden Keyboard", "accessory", "legendary", 0.2),
    ("matrix_rain", "Matrix Rain", "animation", "legendary", 0.2),
];

/// GET /api/loot — get loot drop history
pub async fn get_loot(
    State(db): State<DbPool>,
    Query(q): Query<DeviceQuery>,
) -> Result<Json<Vec<LootDrop>>, AppError> {
    let conn = db.lock().unwrap();
    let device_id = resolve_device_id(&conn, q.device_id.as_deref())?;

    let mut stmt = conn.prepare(
        "SELECT id, device_id, item_type, item_id, item_name, rarity, dropped_at, claimed FROM loot_drops WHERE device_id = ?1 ORDER BY dropped_at DESC LIMIT 50"
    )?;
    let drops = stmt.query_map([&device_id], |r| {
        Ok(LootDrop {
            id: r.get(0)?,
            device_id: r.get(1)?,
            item_type: r.get(2)?,
            item_id: r.get(3)?,
            item_name: r.get(4)?,
            rarity: r.get(5)?,
            dropped_at: r.get(6)?,
            claimed: r.get(7)?,
        })
    })?.collect::<Result<Vec<_>, _>>()?;

    Ok(Json(drops))
}

/// POST /api/loot/claim — claim a loot drop
pub async fn claim_loot(
    State(db): State<DbPool>,
    Json(input): Json<serde_json::Value>,
) -> Result<Json<serde_json::Value>, AppError> {
    let conn = db.lock().unwrap();
    let loot_id = input["loot_id"].as_i64().ok_or_else(|| AppError::BadRequest("Missing loot_id".into()))?;

    let updated = conn.execute(
        "UPDATE loot_drops SET claimed = 1 WHERE id = ?1 AND claimed = 0",
        [loot_id],
    )?;

    if updated == 0 {
        return Err(AppError::NotFound("Loot drop not found or already claimed".into()));
    }

    Ok(Json(json!({ "ok": true })))
}

/// Called from hook handler — chance to drop loot after coding activity
pub fn maybe_drop_loot(conn: &rusqlite::Connection, device_id: &str) -> Result<Option<(String, String, String)>, rusqlite::Error> {
    // 5% chance of loot drop per activity
    let roll: f64 = rand_simple(conn);
    if roll > 0.05 { return Ok(None); }

    // Weighted random from loot table
    let total_weight: f64 = LOOT_TABLE.iter().map(|(_, _, _, _, w)| w).sum();
    let mut pick = rand_simple(conn) * total_weight;

    for (item_id, item_name, item_type, rarity, weight) in LOOT_TABLE {
        pick -= weight;
        if pick <= 0.0 {
            conn.execute(
                "INSERT INTO loot_drops (device_id, item_type, item_id, item_name, rarity) VALUES (?1, ?2, ?3, ?4, ?5)",
                rusqlite::params![device_id, item_type, item_id, item_name, rarity],
            )?;
            return Ok(Some((item_id.to_string(), item_name.to_string(), rarity.to_string())));
        }
    }

    Ok(None)
}

/// Simple pseudo-random using SQLite's random()
fn rand_simple(conn: &rusqlite::Connection) -> f64 {
    let r: i64 = conn.query_row("SELECT ABS(random()) % 10000", [], |r| r.get(0)).unwrap_or(5000);
    r as f64 / 10000.0
}

// ── Seasonal Events ─────────────────────────────────────────────

#[derive(Debug, Serialize)]
pub struct SeasonalEvent {
    pub id: String,
    pub name: String,
    pub description: String,
    pub theme: String,
    pub active: bool,
    pub start_date: String,
    pub end_date: String,
    pub special_items: Vec<serde_json::Value>,
    pub bonus_xp_multiplier: f64,
}

fn get_seasonal_events() -> Vec<SeasonalEvent> {
    let now = chrono::Utc::now();
    let month = now.format("%m").to_string().parse::<u32>().unwrap_or(1);
    let day = now.format("%d").to_string().parse::<u32>().unwrap_or(1);
    let year = now.format("%Y").to_string();

    let mut events = vec![
        SeasonalEvent {
            id: "halloween".to_string(),
            name: "Spooky Season".to_string(),
            description: "Ghosts, ghouls, and buggy code! Spooky avatars and themed animations.".to_string(),
            theme: "halloween".to_string(),
            active: month == 10 && day >= 20,
            start_date: format!("{}-10-20", year),
            end_date: format!("{}-11-01", year),
            special_items: vec![
                json!({"id": "ghost_hat", "name": "Ghost Hat", "type": "accessory"}),
                json!({"id": "pumpkin_face", "name": "Pumpkin Face", "type": "avatar"}),
                json!({"id": "spooky_dance", "name": "Spooky Dance", "type": "animation"}),
            ],
            bonus_xp_multiplier: 1.5,
        },
        SeasonalEvent {
            id: "winter_holidays".to_string(),
            name: "Winter Wonderland".to_string(),
            description: "Holiday cheer for your hookbot! Snowflakes, presents, and warm vibes.".to_string(),
            theme: "winter".to_string(),
            active: month == 12 && day >= 15,
            start_date: format!("{}-12-15", year),
            end_date: format!("{}-01-05", (year.parse::<i32>().unwrap_or(2026) + 1)),
            special_items: vec![
                json!({"id": "santa_hat", "name": "Santa Hat", "type": "accessory"}),
                json!({"id": "snowflake_aura", "name": "Snowflake Aura", "type": "animation"}),
                json!({"id": "reindeer_antlers", "name": "Reindeer Antlers", "type": "accessory"}),
            ],
            bonus_xp_multiplier: 2.0,
        },
        SeasonalEvent {
            id: "april_fools".to_string(),
            name: "April Fools Chaos".to_string(),
            description: "Everything is upside down! Inverted controls, silly animations, and pranks.".to_string(),
            theme: "chaos".to_string(),
            active: month == 4 && day == 1,
            start_date: format!("{}-04-01", year),
            end_date: format!("{}-04-02", year),
            special_items: vec![
                json!({"id": "clown_nose", "name": "Clown Nose", "type": "accessory"}),
                json!({"id": "upside_down", "name": "Upside Down Mode", "type": "animation"}),
                json!({"id": "rubber_duck", "name": "Rubber Duck", "type": "accessory"}),
            ],
            bonus_xp_multiplier: 3.0,
        },
        SeasonalEvent {
            id: "new_year".to_string(),
            name: "New Year Celebration".to_string(),
            description: "Ring in the new year with fireworks and fresh coding goals!".to_string(),
            theme: "celebration".to_string(),
            active: (month == 12 && day >= 30) || (month == 1 && day <= 2),
            start_date: format!("{}-12-30", year),
            end_date: format!("{}-01-02", (year.parse::<i32>().unwrap_or(2026) + 1)),
            special_items: vec![
                json!({"id": "party_popper", "name": "Party Popper", "type": "animation"}),
                json!({"id": "year_badge", "name": "2026 Badge", "type": "accessory"}),
            ],
            bonus_xp_multiplier: 2.0,
        },
    ];

    // Sort active events first
    events.sort_by_key(|e| !e.active);
    events
}

/// GET /api/seasonal — get seasonal events
pub async fn get_events(
) -> Result<Json<Vec<SeasonalEvent>>, AppError> {
    Ok(Json(get_seasonal_events()))
}

/// GET /api/seasonal/active — get currently active event (if any)
pub async fn get_active_event(
) -> Result<Json<serde_json::Value>, AppError> {
    let events = get_seasonal_events();
    let active = events.into_iter().find(|e| e.active);

    if let Some(event) = active {
        Ok(Json(json!({
            "active": true,
            "event": {
                "id": event.id,
                "name": event.name,
                "theme": event.theme,
                "bonus_xp_multiplier": event.bonus_xp_multiplier,
                "special_items": event.special_items,
            }
        })))
    } else {
        Ok(Json(json!({ "active": false })))
    }
}

// ── Typing Speed Mini-Game ──────────────────────────────────────

#[derive(Debug, Deserialize)]
pub struct TypingSpeedSubmit {
    pub device_id: Option<String>,
    pub wpm: i64,
    pub accuracy: f64,      // 0.0 - 1.0
    pub duration_secs: i64,
}

#[derive(Debug, Serialize)]
pub struct TypingSpeedResult {
    pub ok: bool,
    pub wpm: i64,
    pub accuracy: f64,
    pub xp_earned: i64,
    pub personal_best_wpm: i64,
    pub is_new_record: bool,
    pub rank: String,
}

/// POST /api/games/typing — submit typing speed result
pub async fn submit_typing_speed(
    State(db): State<DbPool>,
    Json(input): Json<TypingSpeedSubmit>,
) -> Result<Json<TypingSpeedResult>, AppError> {
    let conn = db.lock().unwrap();
    let device_id = resolve_device_id(&conn, input.device_id.as_deref())?;

    let prev_best: i64 = conn.query_row(
        "SELECT COALESCE(MAX(score), 0) FROM game_scores WHERE device_id = ?1 AND game = 'typing'",
        [&device_id], |r| r.get(0),
    ).unwrap_or(0);

    let is_new_record = input.wpm > prev_best;

    conn.execute(
        "INSERT INTO game_scores (device_id, game, score, duration_secs) VALUES (?1, 'typing', ?2, ?3)",
        rusqlite::params![device_id, input.wpm, input.duration_secs],
    )?;

    // XP based on WPM tiers
    let base_xp = match input.wpm {
        0..=30 => 5,
        31..=60 => 10,
        61..=90 => 20,
        91..=120 => 35,
        _ => 50,
    };
    let accuracy_bonus = (base_xp as f64 * input.accuracy * 0.5) as i64;
    let xp = base_xp + accuracy_bonus;

    conn.execute(
        "INSERT INTO xp_ledger (device_id, amount, reason) VALUES (?1, ?2, 'minigame:typing')",
        rusqlite::params![device_id, xp],
    )?;

    let rank = match input.wpm {
        0..=20 => "Pecking",
        21..=40 => "Hunt & Peck",
        41..=60 => "Average",
        61..=80 => "Proficient",
        81..=100 => "Fast",
        101..=120 => "Speed Demon",
        _ => "Legendary",
    };

    // Check for speed_demon achievement variant
    if input.wpm >= 100 {
        let already: bool = conn.query_row(
            "SELECT COUNT(*) > 0 FROM achievements WHERE device_id = ?1 AND badge_id = 'typing_speed_demon'",
            [&device_id], |r| r.get(0),
        ).unwrap_or(true);
        if !already {
            conn.execute(
                "INSERT OR IGNORE INTO achievements (device_id, badge_id) VALUES (?1, 'typing_speed_demon')",
                [&device_id],
            )?;
        }
    }

    Ok(Json(TypingSpeedResult {
        ok: true,
        wpm: input.wpm,
        accuracy: input.accuracy,
        xp_earned: xp,
        personal_best_wpm: if is_new_record { input.wpm } else { prev_best },
        is_new_record,
        rank: rank.to_string(),
    }))
}

// ── Idle Animations ─────────────────────────────────────────────

#[derive(Debug, Serialize)]
#[allow(dead_code)]
pub struct IdleAnimation {
    pub stage: i64,        // 0-based, increases with AFK time
    pub name: String,
    pub description: String,
    pub min_idle_minutes: i64,
    pub animation: serde_json::Value,
}

const IDLE_STAGES: &[(&str, &str, i64)] = &[
    ("yawn", "Your hookbot yawns and stretches", 5),
    ("doze", "Your hookbot starts to doze off", 15),
    ("nap", "Your hookbot is napping peacefully", 30),
    ("dream_bubbles", "Dream bubbles float above your hookbot", 45),
    ("sleep_juggle", "Your hookbot juggles in its sleep", 60),
    ("pillow_fort", "Your hookbot builds a tiny pillow fort", 90),
    ("tiny_house", "Your hookbot constructs a miniature house", 120),
    ("campfire", "Your hookbot roasts marshmallows by a campfire", 180),
    ("garden", "Your hookbot tends a tiny pixel garden", 240),
    ("existential", "Your hookbot contemplates the meaning of code", 360),
];

/// GET /api/idle-animations — get idle animation for current idle duration
pub async fn get_idle_animation(
    State(db): State<DbPool>,
    Query(q): Query<DeviceQuery>,
) -> Result<Json<serde_json::Value>, AppError> {
    let conn = db.lock().unwrap();
    let device_id = resolve_device_id(&conn, q.device_id.as_deref())?;

    // Get minutes since last activity
    let idle_minutes: i64 = conn.query_row(
        "SELECT COALESCE(CAST((julianday('now') - julianday(MAX(created_at))) * 24 * 60 AS INTEGER), 999) FROM tool_uses WHERE device_id = ?1",
        [&device_id], |r| r.get(0),
    ).unwrap_or(999);

    // Find the highest matching idle stage
    let stage = IDLE_STAGES.iter().enumerate().rev()
        .find(|(_, (_, _, min))| idle_minutes >= *min);

    if let Some((idx, (name, desc, min_mins))) = stage {
        Ok(Json(json!({
            "idle": true,
            "idle_minutes": idle_minutes,
            "stage": idx,
            "animation": {
                "name": name,
                "description": desc,
                "min_idle_minutes": min_mins,
            },
            "all_stages": IDLE_STAGES.iter().enumerate().map(|(i, (n, d, m))| {
                json!({"stage": i, "name": n, "description": d, "min_idle_minutes": m, "unlocked": idle_minutes >= *m})
            }).collect::<Vec<_>>(),
        })))
    } else {
        Ok(Json(json!({
            "idle": false,
            "idle_minutes": idle_minutes,
            "message": "Your hookbot is alert and ready!",
        })))
    }
}
