use axum::extract::{Query, State};
use axum::Json;
use serde::{Deserialize, Serialize};

use crate::db::DbPool;
use crate::error::AppError;

// ── Item catalog ───────────────────────────────────────────────────

#[derive(Debug, Clone, Serialize)]
pub struct StoreItem {
    pub id: &'static str,
    pub name: &'static str,
    pub description: &'static str,
    pub category: &'static str,
    pub price: i64,
    pub icon: &'static str,
    pub rarity: &'static str,
}

const CATALOG: &[StoreItem] = &[
    // ── Accessories ────────────────────────────────────────────
    StoreItem { id: "acc_tophat",  name: "Top Hat",     description: "A distinguished tall hat with a band. For the classy bot.", category: "accessory", price: 0,    icon: "🎩", rarity: "starter" },
    StoreItem { id: "acc_glasses", name: "Glasses",     description: "Round spectacles. Intellectual vibes.",                     category: "accessory", price: 150,  icon: "👓", rarity: "common" },
    StoreItem { id: "acc_bowtie",  name: "Bow Tie",     description: "A snappy red bow tie. Instant charm.",                     category: "accessory", price: 200,  icon: "🎀", rarity: "common" },
    StoreItem { id: "acc_cigar",   name: "Cigar",       description: "Smoldering cigar with animated smoke. Boss energy.",       category: "accessory", price: 350,  icon: "🚬", rarity: "uncommon" },
    StoreItem { id: "acc_horns",   name: "Devil Horns", description: "Red curved horns. Embrace the chaos.",                     category: "accessory", price: 500,  icon: "😈", rarity: "uncommon" },
    StoreItem { id: "acc_monocle", name: "Monocle",     description: "Gold monocle with chain. Distinguished villainy.",          category: "accessory", price: 750,  icon: "🧐", rarity: "rare" },
    StoreItem { id: "acc_crown",   name: "Crown",       description: "Golden crown with jewels. Sparkles with animated glow.",   category: "accessory", price: 1500, icon: "👑", rarity: "epic" },
    StoreItem { id: "acc_halo",    name: "Halo",        description: "Floating golden halo with ethereal glow. Angelic.",        category: "accessory", price: 2500, icon: "😇", rarity: "legendary" },

    // ── Titles ─────────────────────────────────────────────────
    StoreItem { id: "title_overlord",    name: "Overlord",       description: "Custom title: Overlord. Command respect.",              category: "title", price: 500,  icon: "⚔️",  rarity: "uncommon" },
    StoreItem { id: "title_shadow",      name: "Shadow Coder",   description: "Custom title: Shadow Coder. Work from the darkness.",   category: "title", price: 1000, icon: "🌑", rarity: "rare" },
    StoreItem { id: "title_pixel",       name: "Pixel Wizard",   description: "Custom title: Pixel Wizard. Master of the screen.",     category: "title", price: 1500, icon: "🧙", rarity: "rare" },
    StoreItem { id: "title_chaos",       name: "Chaos Agent",    description: "Custom title: Chaos Agent. Embrace the entropy.",       category: "title", price: 2000, icon: "🌀", rarity: "epic" },
    StoreItem { id: "title_singularity", name: "Singularity",    description: "Custom title: Singularity. Beyond comprehension.",      category: "title", price: 5000, icon: "🕳️", rarity: "legendary" },

    // ── Animations ─────────────────────────────────────────────
    StoreItem { id: "anim_laugh",     name: "Laugh",       description: "Unlock the maniacal laugh animation.",           category: "animation", price: 300,  icon: "😂", rarity: "common" },
    StoreItem { id: "anim_rage",      name: "Rage",        description: "Unlock the full rage tantrum animation.",         category: "animation", price: 600,  icon: "🤬", rarity: "uncommon" },
    StoreItem { id: "anim_sleep",     name: "Sleep",       description: "Unlock the peaceful sleeping animation.",         category: "animation", price: 400,  icon: "😴", rarity: "common" },
    StoreItem { id: "anim_lookaround",name: "Look Around", description: "Unlock the suspicious look around animation.",    category: "animation", price: 500,  icon: "👀", rarity: "uncommon" },

    // ── Screensavers ───────────────────────────────────────────
    StoreItem { id: "ss_matrix",  name: "Matrix Rain",  description: "Digital rain screensaver. Very hacker.",            category: "screensaver", price: 800,  icon: "🟩", rarity: "rare" },
    StoreItem { id: "ss_stars",   name: "Starfield",    description: "Flying through space screensaver.",                  category: "screensaver", price: 600,  icon: "✨", rarity: "uncommon" },
    StoreItem { id: "ss_bounce",  name: "DVD Bounce",   description: "The classic bouncing logo. Will it hit the corner?", category: "screensaver", price: 400,  icon: "📀", rarity: "common" },
];

fn find_item(id: &str) -> Option<&'static StoreItem> {
    CATALOG.iter().find(|i| i.id == id)
}

// ── XP balance helper ──────────────────────────────────────────────

fn xp_balance(conn: &rusqlite::Connection, device_id: &str) -> i64 {
    let earned: i64 = conn.query_row(
        "SELECT COALESCE(SUM(amount), 0) FROM xp_ledger WHERE device_id = ?1",
        [device_id], |row| row.get(0),
    ).unwrap_or(0);

    let spent: i64 = conn.query_row(
        "SELECT COALESCE(SUM(xp_cost), 0) FROM store_purchases WHERE device_id = ?1",
        [device_id], |row| row.get(0),
    ).unwrap_or(0);

    earned - spent
}

// ── Handlers ───────────────────────────────────────────────────────

#[derive(Debug, Deserialize)]
pub struct DeviceQuery {
    pub device_id: Option<String>,
}

#[derive(Debug, Serialize)]
pub struct StoreItemResponse {
    #[serde(flatten)]
    pub item: &'static StoreItem,
    pub owned: bool,
    pub can_afford: bool,
}

#[derive(Debug, Serialize)]
pub struct StoreResponse {
    pub items: Vec<StoreItemResponse>,
    pub balance: i64,
}

/// GET /api/store — list all items with ownership + affordability
pub async fn list_items(
    State(db): State<DbPool>,
    Query(q): Query<DeviceQuery>,
) -> Result<Json<StoreResponse>, AppError> {
    let conn = db.lock().unwrap();

    let device_id = resolve_device_id(&conn, q.device_id.as_deref())?;
    let balance = xp_balance(&conn, &device_id);

    let owned_ids: Vec<String> = {
        let mut stmt = conn.prepare(
            "SELECT item_id FROM store_purchases WHERE device_id = ?1",
        )?;
        let ids = stmt.query_map([&device_id], |row| row.get(0))?
            .filter_map(|r| r.ok())
            .collect();
        ids
    };

    let items = CATALOG.iter().map(|item| {
        let owned = owned_ids.iter().any(|id| id == item.id);
        StoreItemResponse {
            item,
            owned,
            can_afford: !owned && balance >= item.price,
        }
    }).collect();

    Ok(Json(StoreResponse { items, balance }))
}

#[derive(Debug, Deserialize)]
pub struct BuyRequest {
    pub item_id: String,
    pub device_id: Option<String>,
}

#[derive(Debug, Serialize)]
pub struct BuyResponse {
    pub ok: bool,
    pub item_id: String,
    pub xp_spent: i64,
    pub new_balance: i64,
}

/// POST /api/store/buy — purchase an item
pub async fn buy_item(
    State(db): State<DbPool>,
    Json(input): Json<BuyRequest>,
) -> Result<Json<BuyResponse>, AppError> {
    let conn = db.lock().unwrap();
    let device_id = resolve_device_id(&conn, input.device_id.as_deref())?;

    let item = find_item(&input.item_id)
        .ok_or_else(|| AppError::NotFound(format!("Item '{}' not found", input.item_id)))?;

    // Check not already owned
    let already_owned: bool = conn.query_row(
        "SELECT COUNT(*) FROM store_purchases WHERE device_id = ?1 AND item_id = ?2",
        rusqlite::params![device_id, item.id],
        |row| row.get::<_, i32>(0),
    ).map(|c| c > 0)?;

    if already_owned {
        return Err(AppError::BadRequest("Item already owned".into()));
    }

    // Check balance
    let balance = xp_balance(&conn, &device_id);
    if balance < item.price {
        return Err(AppError::BadRequest(format!(
            "Not enough XP. Need {} but have {}", item.price, balance
        )));
    }

    // Record purchase
    conn.execute(
        "INSERT INTO store_purchases (device_id, item_id, xp_cost) VALUES (?1, ?2, ?3)",
        rusqlite::params![device_id, item.id, item.price],
    )?;

    let new_balance = xp_balance(&conn, &device_id);

    Ok(Json(BuyResponse {
        ok: true,
        item_id: input.item_id,
        xp_spent: item.price,
        new_balance,
    }))
}

/// GET /api/store/owned — list owned item IDs for a device
pub async fn owned_items(
    State(db): State<DbPool>,
    Query(q): Query<DeviceQuery>,
) -> Result<Json<Vec<String>>, AppError> {
    let conn = db.lock().unwrap();
    let device_id = resolve_device_id(&conn, q.device_id.as_deref())?;

    let mut stmt = conn.prepare(
        "SELECT item_id FROM store_purchases WHERE device_id = ?1 ORDER BY purchased_at",
    )?;
    let ids: Vec<String> = stmt.query_map([&device_id], |row| row.get(0))?
        .filter_map(|r| r.ok())
        .collect();

    Ok(Json(ids))
}

/// Resolve device_id — use provided or fall back to first registered device
fn resolve_device_id(conn: &rusqlite::Connection, device_id: Option<&str>) -> Result<String, AppError> {
    if let Some(id) = device_id {
        return Ok(id.to_string());
    }
    conn.query_row("SELECT id FROM devices ORDER BY created_at LIMIT 1", [], |row| row.get(0))
        .map_err(|_| AppError::NotFound("No devices registered".into()))
}
