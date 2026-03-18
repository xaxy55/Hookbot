use rusqlite::Connection;
use std::path::Path;
use std::sync::{Arc, Mutex};

pub type DbPool = Arc<Mutex<Connection>>;

pub fn init(path: &Path) -> DbPool {
    if let Some(parent) = path.parent() {
        std::fs::create_dir_all(parent).expect("Failed to create database directory");
    }

    let conn = Connection::open(path).expect("Failed to open database");

    conn.execute_batch("PRAGMA journal_mode=WAL; PRAGMA foreign_keys=ON;")
        .expect("Failed to set pragmas");

    conn.execute_batch(SCHEMA).expect("Failed to create schema");

    Arc::new(Mutex::new(conn))
}

const SCHEMA: &str = r#"
CREATE TABLE IF NOT EXISTS devices (
    id TEXT PRIMARY KEY,
    name TEXT NOT NULL,
    hostname TEXT NOT NULL,
    ip_address TEXT NOT NULL,
    purpose TEXT,
    personality TEXT,
    device_type TEXT,
    created_at TEXT NOT NULL DEFAULT (datetime('now')),
    updated_at TEXT NOT NULL DEFAULT (datetime('now'))
);

CREATE TABLE IF NOT EXISTS device_config (
    device_id TEXT PRIMARY KEY REFERENCES devices(id) ON DELETE CASCADE,
    led_brightness INTEGER NOT NULL DEFAULT 60,
    led_colors TEXT,
    sound_enabled INTEGER NOT NULL DEFAULT 1,
    sound_volume INTEGER NOT NULL DEFAULT 50,
    avatar_preset TEXT,
    custom_data TEXT
);

CREATE TABLE IF NOT EXISTS firmware (
    id TEXT PRIMARY KEY,
    version TEXT NOT NULL,
    filename TEXT NOT NULL,
    size_bytes INTEGER NOT NULL,
    checksum TEXT NOT NULL,
    uploaded_at TEXT NOT NULL DEFAULT (datetime('now')),
    notes TEXT,
    device_type TEXT
);

CREATE TABLE IF NOT EXISTS ota_jobs (
    id TEXT PRIMARY KEY,
    firmware_id TEXT NOT NULL REFERENCES firmware(id),
    device_id TEXT NOT NULL REFERENCES devices(id),
    status TEXT NOT NULL DEFAULT 'pending',
    created_at TEXT NOT NULL DEFAULT (datetime('now')),
    updated_at TEXT NOT NULL DEFAULT (datetime('now')),
    error_msg TEXT
);

CREATE TABLE IF NOT EXISTS status_log (
    id INTEGER PRIMARY KEY AUTOINCREMENT,
    device_id TEXT NOT NULL REFERENCES devices(id) ON DELETE CASCADE,
    state TEXT NOT NULL,
    uptime_ms INTEGER NOT NULL,
    free_heap INTEGER NOT NULL,
    recorded_at TEXT NOT NULL DEFAULT (datetime('now'))
);

CREATE INDEX IF NOT EXISTS idx_status_log_device ON status_log(device_id, recorded_at);
CREATE INDEX IF NOT EXISTS idx_ota_jobs_device ON ota_jobs(device_id);

-- Gamification: tool usage tracking
CREATE TABLE IF NOT EXISTS tool_uses (
    id INTEGER PRIMARY KEY AUTOINCREMENT,
    device_id TEXT REFERENCES devices(id) ON DELETE CASCADE,
    tool_name TEXT NOT NULL,
    event TEXT NOT NULL,
    project TEXT,
    duration_ms INTEGER,
    xp_earned INTEGER NOT NULL DEFAULT 0,
    created_at TEXT NOT NULL DEFAULT (datetime('now'))
);
CREATE INDEX IF NOT EXISTS idx_tool_uses_device ON tool_uses(device_id, created_at);
CREATE INDEX IF NOT EXISTS idx_tool_uses_tool ON tool_uses(tool_name, created_at);

-- Gamification: coding sessions
CREATE TABLE IF NOT EXISTS sessions (
    id INTEGER PRIMARY KEY AUTOINCREMENT,
    device_id TEXT REFERENCES devices(id) ON DELETE CASCADE,
    started_at TEXT NOT NULL DEFAULT (datetime('now')),
    ended_at TEXT,
    tool_count INTEGER NOT NULL DEFAULT 0,
    xp_earned INTEGER NOT NULL DEFAULT 0
);
CREATE INDEX IF NOT EXISTS idx_sessions_device ON sessions(device_id, started_at);

-- Gamification: XP ledger
CREATE TABLE IF NOT EXISTS xp_ledger (
    id INTEGER PRIMARY KEY AUTOINCREMENT,
    device_id TEXT REFERENCES devices(id) ON DELETE CASCADE,
    amount INTEGER NOT NULL,
    reason TEXT NOT NULL,
    created_at TEXT NOT NULL DEFAULT (datetime('now'))
);
CREATE INDEX IF NOT EXISTS idx_xp_ledger_device ON xp_ledger(device_id);

-- Gamification: achievements
CREATE TABLE IF NOT EXISTS achievements (
    id INTEGER PRIMARY KEY AUTOINCREMENT,
    device_id TEXT REFERENCES devices(id) ON DELETE CASCADE,
    badge_id TEXT NOT NULL,
    earned_at TEXT NOT NULL DEFAULT (datetime('now')),
    UNIQUE(device_id, badge_id)
);
CREATE INDEX IF NOT EXISTS idx_achievements_device ON achievements(device_id);

-- Gamification: streaks
CREATE TABLE IF NOT EXISTS streaks (
    device_id TEXT PRIMARY KEY REFERENCES devices(id) ON DELETE CASCADE,
    current_streak INTEGER NOT NULL DEFAULT 0,
    longest_streak INTEGER NOT NULL DEFAULT 0,
    last_active_date TEXT,
    updated_at TEXT NOT NULL DEFAULT (datetime('now'))
);

-- Server settings (key-value)
CREATE TABLE IF NOT EXISTS server_settings (
    key TEXT PRIMARY KEY,
    value TEXT NOT NULL
);

-- Store purchases
CREATE TABLE IF NOT EXISTS store_purchases (
    id INTEGER PRIMARY KEY AUTOINCREMENT,
    device_id TEXT NOT NULL REFERENCES devices(id) ON DELETE CASCADE,
    item_id TEXT NOT NULL,
    xp_cost INTEGER NOT NULL,
    purchased_at TEXT NOT NULL DEFAULT (datetime('now')),
    UNIQUE(device_id, item_id)
);
CREATE INDEX IF NOT EXISTS idx_store_purchases_device ON store_purchases(device_id);

CREATE TABLE IF NOT EXISTS notifications (
    id INTEGER PRIMARY KEY AUTOINCREMENT,
    device_id TEXT NOT NULL REFERENCES devices(id) ON DELETE CASCADE,
    source TEXT NOT NULL,
    unread INTEGER NOT NULL DEFAULT 0,
    message TEXT,
    delivered INTEGER NOT NULL DEFAULT 0,
    created_at TEXT NOT NULL DEFAULT (datetime('now')),
    delivered_at TEXT
);
CREATE INDEX IF NOT EXISTS idx_notifications_device ON notifications(device_id, created_at);

CREATE TABLE IF NOT EXISTS sensor_configs (
    id INTEGER PRIMARY KEY AUTOINCREMENT,
    device_id TEXT NOT NULL REFERENCES devices(id) ON DELETE CASCADE,
    channel INTEGER NOT NULL,
    pin INTEGER NOT NULL,
    sensor_type TEXT NOT NULL DEFAULT 'disabled',
    label TEXT,
    poll_interval_ms INTEGER NOT NULL DEFAULT 1000,
    threshold INTEGER NOT NULL DEFAULT 0,
    UNIQUE(device_id, channel)
);

CREATE TABLE IF NOT EXISTS automation_rules (
    id TEXT PRIMARY KEY,
    device_id TEXT NOT NULL REFERENCES devices(id) ON DELETE CASCADE,
    name TEXT NOT NULL,
    enabled INTEGER NOT NULL DEFAULT 1,
    trigger_type TEXT NOT NULL,
    trigger_config TEXT NOT NULL DEFAULT '{}',
    action_type TEXT NOT NULL,
    action_config TEXT NOT NULL DEFAULT '{}',
    cooldown_secs INTEGER NOT NULL DEFAULT 60,
    last_triggered_at TEXT,
    created_at TEXT NOT NULL DEFAULT (datetime('now'))
);
CREATE INDEX IF NOT EXISTS idx_rules_device ON automation_rules(device_id);
"#;
