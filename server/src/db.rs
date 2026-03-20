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

    run_migrations(&conn);

    Arc::new(Mutex::new(conn))
}

fn run_migrations(conn: &Connection) {
    let migrations: &[&str] = &[
        "ALTER TABLE devices ADD COLUMN device_type TEXT",
        "ALTER TABLE device_config ADD COLUMN sound_pack TEXT DEFAULT 'default'",
        "ALTER TABLE community_plugins ADD COLUMN verified BOOLEAN DEFAULT 0",
        "ALTER TABLE shared_assets ADD COLUMN verified BOOLEAN DEFAULT 0",
        // Pet state
        "CREATE TABLE IF NOT EXISTS pet_state (
            device_id TEXT PRIMARY KEY REFERENCES devices(id) ON DELETE CASCADE,
            hunger INTEGER NOT NULL DEFAULT 50,
            happiness INTEGER NOT NULL DEFAULT 50,
            last_fed_at TEXT,
            last_pet_at TEXT,
            total_feeds INTEGER NOT NULL DEFAULT 0,
            total_pets INTEGER NOT NULL DEFAULT 0
        )",
        // Token usage tracking
        "CREATE TABLE IF NOT EXISTS token_usage (
            id INTEGER PRIMARY KEY AUTOINCREMENT,
            device_id TEXT NOT NULL REFERENCES devices(id) ON DELETE CASCADE,
            input_tokens INTEGER NOT NULL DEFAULT 0,
            output_tokens INTEGER NOT NULL DEFAULT 0,
            model TEXT NOT NULL DEFAULT 'unknown',
            recorded_at TEXT NOT NULL DEFAULT (datetime('now'))
        )",
        "CREATE INDEX IF NOT EXISTS idx_token_usage_device ON token_usage(device_id, recorded_at)",
        // Mood journal
        "CREATE TABLE IF NOT EXISTS mood_journal (
            id INTEGER PRIMARY KEY AUTOINCREMENT,
            device_id TEXT NOT NULL REFERENCES devices(id) ON DELETE CASCADE,
            mood TEXT NOT NULL,
            note TEXT,
            energy INTEGER NOT NULL DEFAULT 3,
            created_at TEXT NOT NULL DEFAULT (datetime('now'))
        )",
        "CREATE INDEX IF NOT EXISTS idx_mood_journal_device ON mood_journal(device_id, created_at)",
        // Plugin sandboxing
        "CREATE TABLE IF NOT EXISTS plugin_sandboxes (
            id INTEGER PRIMARY KEY AUTOINCREMENT,
            plugin_id TEXT NOT NULL REFERENCES community_plugins(id) ON DELETE CASCADE,
            device_id TEXT NOT NULL REFERENCES devices(id) ON DELETE CASCADE,
            allowed_apis TEXT NOT NULL DEFAULT '[]',
            blocked_apis TEXT NOT NULL DEFAULT '[]',
            max_calls_per_minute INTEGER NOT NULL DEFAULT 60,
            can_access_network INTEGER NOT NULL DEFAULT 0,
            can_modify_state INTEGER NOT NULL DEFAULT 0,
            can_send_notifications INTEGER NOT NULL DEFAULT 0,
            can_access_sensors INTEGER NOT NULL DEFAULT 0,
            enabled INTEGER NOT NULL DEFAULT 1,
            created_at TEXT NOT NULL DEFAULT (datetime('now')),
            UNIQUE(plugin_id, device_id)
        )",
        // Device-to-device communication
        "CREATE TABLE IF NOT EXISTS device_links (
            id TEXT PRIMARY KEY,
            source_device_id TEXT NOT NULL REFERENCES devices(id) ON DELETE CASCADE,
            target_device_id TEXT NOT NULL REFERENCES devices(id) ON DELETE CASCADE,
            trigger_type TEXT NOT NULL,
            trigger_config TEXT NOT NULL DEFAULT '{}',
            action_type TEXT NOT NULL,
            action_config TEXT NOT NULL DEFAULT '{}',
            enabled INTEGER NOT NULL DEFAULT 1,
            cooldown_secs INTEGER NOT NULL DEFAULT 30,
            last_triggered_at TEXT,
            created_at TEXT NOT NULL DEFAULT (datetime('now'))
        )",
        "CREATE INDEX IF NOT EXISTS idx_device_links_source ON device_links(source_device_id)",
        "CREATE INDEX IF NOT EXISTS idx_device_links_target ON device_links(target_device_id)",
        // Multi-user support
        "CREATE TABLE IF NOT EXISTS users (
            id TEXT PRIMARY KEY,
            username TEXT NOT NULL UNIQUE,
            display_name TEXT NOT NULL,
            password_hash TEXT NOT NULL,
            role TEXT NOT NULL DEFAULT 'user',
            api_token TEXT UNIQUE,
            last_login_at TEXT,
            created_at TEXT NOT NULL DEFAULT (datetime('now'))
        )",
        "CREATE TABLE IF NOT EXISTS user_device_assignments (
            user_id TEXT NOT NULL REFERENCES users(id) ON DELETE CASCADE,
            device_id TEXT NOT NULL REFERENCES devices(id) ON DELETE CASCADE,
            permissions TEXT NOT NULL DEFAULT 'full',
            assigned_at TEXT NOT NULL DEFAULT (datetime('now')),
            PRIMARY KEY (user_id, device_id)
        )",
        // Remote access / tunnels
        "CREATE TABLE IF NOT EXISTS tunnel_configs (
            id TEXT PRIMARY KEY,
            name TEXT NOT NULL,
            tunnel_type TEXT NOT NULL DEFAULT 'cloudflare',
            hostname TEXT,
            port INTEGER NOT NULL DEFAULT 3000,
            auth_token TEXT,
            status TEXT NOT NULL DEFAULT 'stopped',
            last_connected_at TEXT,
            config TEXT NOT NULL DEFAULT '{}',
            created_at TEXT NOT NULL DEFAULT (datetime('now'))
        )",
        // Mood learning
        "CREATE TABLE IF NOT EXISTS mood_preferences (
            id INTEGER PRIMARY KEY AUTOINCREMENT,
            device_id TEXT NOT NULL REFERENCES devices(id) ON DELETE CASCADE,
            state TEXT NOT NULL,
            animation_id TEXT,
            positive_responses INTEGER NOT NULL DEFAULT 0,
            negative_responses INTEGER NOT NULL DEFAULT 0,
            total_duration_secs INTEGER NOT NULL DEFAULT 0,
            last_shown_at TEXT,
            updated_at TEXT NOT NULL DEFAULT (datetime('now'))
        )",
        "CREATE UNIQUE INDEX IF NOT EXISTS idx_mood_prefs_device_state ON mood_preferences(device_id, state, animation_id)",
        "CREATE TABLE IF NOT EXISTS mood_patterns (
            id INTEGER PRIMARY KEY AUTOINCREMENT,
            device_id TEXT NOT NULL REFERENCES devices(id) ON DELETE CASCADE,
            hour_of_day INTEGER NOT NULL,
            day_of_week INTEGER NOT NULL,
            preferred_state TEXT,
            preferred_animation TEXT,
            confidence REAL NOT NULL DEFAULT 0.0,
            sample_count INTEGER NOT NULL DEFAULT 0,
            updated_at TEXT NOT NULL DEFAULT (datetime('now')),
            UNIQUE(device_id, hour_of_day, day_of_week)
        )",
    ];

    for sql in migrations {
        // Ignore errors (column already exists)
        let _ = conn.execute_batch(sql);
    }
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

-- Community plugins
CREATE TABLE IF NOT EXISTS community_plugins (
    id TEXT PRIMARY KEY,
    name TEXT NOT NULL,
    description TEXT NOT NULL DEFAULT '',
    author TEXT NOT NULL DEFAULT 'anonymous',
    version TEXT NOT NULL DEFAULT '1.0.0',
    category TEXT NOT NULL DEFAULT 'utility',
    tags TEXT NOT NULL DEFAULT '[]',
    payload TEXT NOT NULL DEFAULT '{}',
    downloads INTEGER NOT NULL DEFAULT 0,
    rating_sum INTEGER NOT NULL DEFAULT 0,
    rating_count INTEGER NOT NULL DEFAULT 0,
    created_at TEXT NOT NULL DEFAULT (datetime('now')),
    updated_at TEXT NOT NULL DEFAULT (datetime('now'))
);

CREATE TABLE IF NOT EXISTS community_plugin_installs (
    id INTEGER PRIMARY KEY AUTOINCREMENT,
    plugin_id TEXT NOT NULL REFERENCES community_plugins(id) ON DELETE CASCADE,
    device_id TEXT NOT NULL REFERENCES devices(id) ON DELETE CASCADE,
    installed_at TEXT NOT NULL DEFAULT (datetime('now')),
    UNIQUE(plugin_id, device_id)
);
CREATE INDEX IF NOT EXISTS idx_plugin_installs_device ON community_plugin_installs(device_id);
CREATE INDEX IF NOT EXISTS idx_plugin_installs_plugin ON community_plugin_installs(plugin_id);

CREATE TABLE IF NOT EXISTS community_plugin_ratings (
    id INTEGER PRIMARY KEY AUTOINCREMENT,
    plugin_id TEXT NOT NULL REFERENCES community_plugins(id) ON DELETE CASCADE,
    device_id TEXT NOT NULL REFERENCES devices(id) ON DELETE CASCADE,
    stars INTEGER NOT NULL CHECK(stars >= 1 AND stars <= 5),
    rated_at TEXT NOT NULL DEFAULT (datetime('now')),
    UNIQUE(plugin_id, device_id)
);

-- Shared assets (avatars, animations, screensavers)
CREATE TABLE IF NOT EXISTS shared_assets (
    id TEXT PRIMARY KEY,
    name TEXT NOT NULL,
    description TEXT NOT NULL DEFAULT '',
    author TEXT NOT NULL DEFAULT 'anonymous',
    asset_type TEXT NOT NULL,
    payload TEXT NOT NULL DEFAULT '{}',
    downloads INTEGER NOT NULL DEFAULT 0,
    rating_sum INTEGER NOT NULL DEFAULT 0,
    rating_count INTEGER NOT NULL DEFAULT 0,
    created_at TEXT NOT NULL DEFAULT (datetime('now'))
);
CREATE INDEX IF NOT EXISTS idx_shared_assets_type ON shared_assets(asset_type);

CREATE TABLE IF NOT EXISTS shared_asset_installs (
    id INTEGER PRIMARY KEY AUTOINCREMENT,
    asset_id TEXT NOT NULL REFERENCES shared_assets(id) ON DELETE CASCADE,
    device_id TEXT NOT NULL REFERENCES devices(id) ON DELETE CASCADE,
    installed_at TEXT NOT NULL DEFAULT (datetime('now')),
    UNIQUE(asset_id, device_id)
);
CREATE INDEX IF NOT EXISTS idx_asset_installs_device ON shared_asset_installs(device_id);

CREATE TABLE IF NOT EXISTS shared_asset_ratings (
    id INTEGER PRIMARY KEY AUTOINCREMENT,
    asset_id TEXT NOT NULL REFERENCES shared_assets(id) ON DELETE CASCADE,
    device_id TEXT NOT NULL REFERENCES devices(id) ON DELETE CASCADE,
    stars INTEGER NOT NULL CHECK(stars >= 1 AND stars <= 5),
    rated_at TEXT NOT NULL DEFAULT (datetime('now')),
    UNIQUE(asset_id, device_id)
);

CREATE TABLE IF NOT EXISTS sensor_readings (
    id INTEGER PRIMARY KEY AUTOINCREMENT,
    device_id TEXT NOT NULL,
    channel INTEGER NOT NULL,
    value REAL NOT NULL,
    recorded_at TEXT NOT NULL DEFAULT (datetime('now')),
    FOREIGN KEY (device_id) REFERENCES devices(id)
);
CREATE INDEX IF NOT EXISTS idx_sensor_readings_device_time ON sensor_readings(device_id, recorded_at);

-- Device groups
CREATE TABLE IF NOT EXISTS device_groups (
    id TEXT PRIMARY KEY,
    name TEXT NOT NULL UNIQUE,
    color TEXT DEFAULT '#6366f1',
    created_at TEXT NOT NULL DEFAULT (datetime('now'))
);

CREATE TABLE IF NOT EXISTS device_group_members (
    group_id TEXT NOT NULL,
    device_id TEXT NOT NULL,
    PRIMARY KEY (group_id, device_id),
    FOREIGN KEY (group_id) REFERENCES device_groups(id) ON DELETE CASCADE,
    FOREIGN KEY (device_id) REFERENCES devices(id) ON DELETE CASCADE
);

-- Verified publishers
CREATE TABLE IF NOT EXISTS verified_publishers (
    id TEXT PRIMARY KEY,
    name TEXT NOT NULL UNIQUE,
    display_name TEXT NOT NULL,
    badge_type TEXT NOT NULL DEFAULT 'verified',
    verified_at TEXT NOT NULL DEFAULT (datetime('now')),
    verified_by TEXT DEFAULT 'system'
);

-- Per-project device routing
CREATE TABLE IF NOT EXISTS project_routes (
    id TEXT PRIMARY KEY,
    project_path TEXT NOT NULL UNIQUE,
    device_id TEXT NOT NULL,
    label TEXT,
    created_at TEXT NOT NULL DEFAULT (datetime('now')),
    FOREIGN KEY (device_id) REFERENCES devices(id)
);
"#;
