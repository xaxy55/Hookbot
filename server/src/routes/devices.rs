use axum::extract::{Path, State};
use axum::Json;
use serde_json::json;

use crate::db::DbPool;
use crate::error::AppError;
use crate::models::*;
use crate::services::proxy;

// --- Config export/import types ---

#[derive(Debug, serde::Serialize, serde::Deserialize)]
pub struct ConfigExport {
    pub metadata: ExportMetadata,
    pub device_info: ExportDeviceInfo,
    pub device_config: DeviceConfig,
    pub servo_config: Option<serde_json::Value>,
    pub sensor_configs: Vec<SensorConfig>,
    pub automation_rules: Vec<ExportRule>,
}

#[derive(Debug, serde::Serialize, serde::Deserialize)]
pub struct ExportMetadata {
    pub export_date: String,
    pub firmware_version: Option<String>,
}

#[derive(Debug, serde::Serialize, serde::Deserialize)]
pub struct ExportDeviceInfo {
    pub name: String,
    pub hostname: String,
    pub purpose: Option<String>,
    pub personality: Option<String>,
    pub device_type: Option<String>,
}

#[derive(Debug, serde::Serialize, serde::Deserialize)]
pub struct ExportRule {
    pub name: String,
    pub enabled: bool,
    pub trigger_type: String,
    pub trigger_config: serde_json::Value,
    pub action_type: String,
    pub action_config: serde_json::Value,
    pub cooldown_secs: i64,
}

pub async fn list_devices(State(db): State<DbPool>) -> Result<Json<Vec<DeviceWithStatus>>, AppError> {
    let conn = db.lock().unwrap();
    let mut stmt = conn.prepare(
        "SELECT d.id, d.name, d.hostname, d.ip_address, d.purpose, d.personality,
                d.created_at, d.updated_at, d.device_type,
                s.state, s.uptime_ms, s.free_heap, s.recorded_at
         FROM devices d
         LEFT JOIN (
             SELECT device_id, state, uptime_ms, free_heap, recorded_at,
                    ROW_NUMBER() OVER (PARTITION BY device_id ORDER BY recorded_at DESC) as rn
             FROM status_log
         ) s ON s.device_id = d.id AND s.rn = 1
         ORDER BY d.name",
    )?;

    let devices = stmt.query_map([], |row| {
        let state: Option<String> = row.get(9)?;
        let latest_status = state.map(|s| StatusSnapshot {
            state: s,
            uptime_ms: row.get(10).unwrap_or(0),
            free_heap: row.get(11).unwrap_or(0),
            recorded_at: row.get(12).unwrap_or_default(),
        });

        let online = latest_status.as_ref().map_or(false, |s| {
            chrono::NaiveDateTime::parse_from_str(&s.recorded_at, "%Y-%m-%d %H:%M:%S")
                .map(|dt| {
                    let now = chrono::Utc::now().naive_utc();
                    (now - dt).num_seconds() < 30
                })
                .unwrap_or(false)
        });

        Ok(DeviceWithStatus {
            device: Device {
                id: row.get(0)?,
                name: row.get(1)?,
                hostname: row.get(2)?,
                ip_address: row.get(3)?,
                purpose: row.get(4)?,
                personality: row.get(5)?,
                device_type: row.get(8)?,
                created_at: row.get(6)?,
                updated_at: row.get(7)?,
            },
            latest_status,
            online,
        })
    })?
    .collect::<Result<Vec<_>, _>>()?;

    Ok(Json(devices))
}

pub async fn create_device(
    State(db): State<DbPool>,
    Json(input): Json<CreateDevice>,
) -> Result<Json<Device>, AppError> {
    let id = uuid::Uuid::new_v4().to_string();
    let conn = db.lock().unwrap();

    conn.execute(
        "INSERT INTO devices (id, name, hostname, ip_address, purpose, personality, device_type)
         VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7)",
        rusqlite::params![id, input.name, input.hostname, input.ip_address, input.purpose, input.personality, input.device_type],
    )?;

    conn.execute(
        "INSERT INTO device_config (device_id) VALUES (?1)",
        [&id],
    )?;

    let device = conn.query_row(
        "SELECT id, name, hostname, ip_address, purpose, personality, created_at, updated_at, device_type
         FROM devices WHERE id = ?1",
        [&id],
        |row| {
            Ok(Device {
                id: row.get(0)?,
                name: row.get(1)?,
                hostname: row.get(2)?,
                ip_address: row.get(3)?,
                purpose: row.get(4)?,
                personality: row.get(5)?,
                device_type: row.get(8)?,
                created_at: row.get(6)?,
                updated_at: row.get(7)?,
            })
        },
    )?;

    Ok(Json(device))
}

fn query_device(conn: &rusqlite::Connection, id: &str) -> Result<Device, AppError> {
    conn.query_row(
        "SELECT id, name, hostname, ip_address, purpose, personality, created_at, updated_at, device_type
         FROM devices WHERE id = ?1",
        [id],
        |row| {
            Ok(Device {
                id: row.get(0)?,
                name: row.get(1)?,
                hostname: row.get(2)?,
                ip_address: row.get(3)?,
                purpose: row.get(4)?,
                personality: row.get(5)?,
                device_type: row.get(8)?,
                created_at: row.get(6)?,
                updated_at: row.get(7)?,
            })
        },
    ).map_err(|_| AppError::NotFound(format!("Device {id} not found")))
}

fn query_latest_status(conn: &rusqlite::Connection, id: &str) -> Option<StatusSnapshot> {
    conn.query_row(
        "SELECT state, uptime_ms, free_heap, recorded_at FROM status_log
         WHERE device_id = ?1 ORDER BY recorded_at DESC LIMIT 1",
        [id],
        |row| {
            Ok(StatusSnapshot {
                state: row.get(0)?,
                uptime_ms: row.get(1)?,
                free_heap: row.get(2)?,
                recorded_at: row.get(3)?,
            })
        },
    ).ok()
}

fn is_online(status: &Option<StatusSnapshot>) -> bool {
    status.as_ref().map_or(false, |s| {
        chrono::NaiveDateTime::parse_from_str(&s.recorded_at, "%Y-%m-%d %H:%M:%S")
            .map(|dt| (chrono::Utc::now().naive_utc() - dt).num_seconds() < 30)
            .unwrap_or(false)
    })
}

fn query_device_ip(conn: &rusqlite::Connection, id: &str) -> Result<String, AppError> {
    conn.query_row(
        "SELECT ip_address FROM devices WHERE id = ?1", [id],
        |row| row.get(0),
    ).map_err(|_| AppError::NotFound(format!("Device {id} not found")))
}

pub async fn get_device(
    State(db): State<DbPool>,
    Path(id): Path<String>,
) -> Result<Json<DeviceWithStatus>, AppError> {
    let conn = db.lock().unwrap();
    let device = query_device(&conn, &id)?;
    let latest_status = query_latest_status(&conn, &id);
    let online = is_online(&latest_status);
    Ok(Json(DeviceWithStatus { device, latest_status, online }))
}

pub async fn update_device(
    State(db): State<DbPool>,
    Path(id): Path<String>,
    Json(input): Json<UpdateDevice>,
) -> Result<Json<Device>, AppError> {
    let conn = db.lock().unwrap();

    let exists: bool = conn.query_row(
        "SELECT COUNT(*) FROM devices WHERE id = ?1", [&id], |row| row.get::<_, i32>(0),
    ).map(|c| c > 0)?;
    if !exists {
        return Err(AppError::NotFound(format!("Device {id} not found")));
    }

    if let Some(name) = &input.name {
        conn.execute("UPDATE devices SET name = ?1, updated_at = datetime('now') WHERE id = ?2", rusqlite::params![name, id])?;
    }
    if let Some(hostname) = &input.hostname {
        conn.execute("UPDATE devices SET hostname = ?1, updated_at = datetime('now') WHERE id = ?2", rusqlite::params![hostname, id])?;
    }
    if let Some(ip) = &input.ip_address {
        conn.execute("UPDATE devices SET ip_address = ?1, updated_at = datetime('now') WHERE id = ?2", rusqlite::params![ip, id])?;
    }
    if let Some(purpose) = &input.purpose {
        conn.execute("UPDATE devices SET purpose = ?1, updated_at = datetime('now') WHERE id = ?2", rusqlite::params![purpose, id])?;
    }
    if let Some(personality) = &input.personality {
        conn.execute("UPDATE devices SET personality = ?1, updated_at = datetime('now') WHERE id = ?2", rusqlite::params![personality, id])?;
    }

    let device = query_device(&conn, &id)?;
    Ok(Json(device))
}

pub async fn delete_device(
    State(db): State<DbPool>,
    Path(id): Path<String>,
) -> Result<Json<serde_json::Value>, AppError> {
    let conn = db.lock().unwrap();
    let rows = conn.execute("DELETE FROM devices WHERE id = ?1", [&id])?;
    if rows == 0 {
        return Err(AppError::NotFound(format!("Device {id} not found")));
    }
    Ok(Json(json!({ "ok": true })))
}

pub async fn forward_state(
    State(db): State<DbPool>,
    Path(id): Path<String>,
    Json(input): Json<StateChange>,
) -> Result<Json<serde_json::Value>, AppError> {
    let ip = {
        let conn = db.lock().unwrap();
        query_device_ip(&conn, &id)?
    };

    let device_url = format!("http://{}/state", ip);
    let body = json!({
        "state": input.state,
        "tool": input.tool.unwrap_or_default(),
        "detail": input.detail.unwrap_or_default(),
    });

    proxy::forward_json(&device_url, &body).await?;
    Ok(Json(json!({ "ok": true, "state": input.state })))
}

pub async fn forward_servos(
    State(db): State<DbPool>,
    Path(id): Path<String>,
    Json(body): Json<serde_json::Value>,
) -> Result<Json<serde_json::Value>, AppError> {
    let ip = {
        let conn = db.lock().unwrap();
        query_device_ip(&conn, &id)?
    };
    proxy::forward_json(&format!("http://{}/servos", ip), &body).await?;
    Ok(Json(json!({ "ok": true })))
}

pub async fn forward_servo_config(
    State(db): State<DbPool>,
    Path(id): Path<String>,
    Json(body): Json<serde_json::Value>,
) -> Result<Json<serde_json::Value>, AppError> {
    let ip = {
        let conn = db.lock().unwrap();
        query_device_ip(&conn, &id)?
    };
    proxy::forward_json(&format!("http://{}/servos/config", ip), &body).await?;
    Ok(Json(json!({ "ok": true })))
}

pub async fn get_servos(
    State(db): State<DbPool>,
    Path(id): Path<String>,
) -> Result<Json<serde_json::Value>, AppError> {
    let ip = {
        let conn = db.lock().unwrap();
        query_device_ip(&conn, &id)?
    };
    let result = proxy::get_json(&format!("http://{}/servos", ip)).await?;
    Ok(Json(result))
}

pub async fn forward_tasks(
    State(db): State<DbPool>,
    Path(id): Path<String>,
    Json(body): Json<serde_json::Value>,
) -> Result<Json<serde_json::Value>, AppError> {
    let ip = {
        let conn = db.lock().unwrap();
        query_device_ip(&conn, &id)?
    };

    proxy::forward_json(&format!("http://{}/tasks", ip), &body).await?;
    Ok(Json(json!({ "ok": true })))
}

pub async fn get_device_status(
    State(db): State<DbPool>,
    Path(id): Path<String>,
) -> Result<Json<serde_json::Value>, AppError> {
    let ip = {
        let conn = db.lock().unwrap();
        query_device_ip(&conn, &id)?
    };

    let result = proxy::get_json(&format!("http://{}/status", ip)).await?;
    Ok(Json(result))
}

pub async fn get_device_history(
    State(db): State<DbPool>,
    Path(id): Path<String>,
) -> Result<Json<Vec<StatusSnapshot>>, AppError> {
    let conn = db.lock().unwrap();
    let mut stmt = conn.prepare(
        "SELECT state, uptime_ms, free_heap, recorded_at FROM status_log
         WHERE device_id = ?1 ORDER BY recorded_at DESC LIMIT 100",
    )?;

    let history = stmt.query_map([&id], |row| {
        Ok(StatusSnapshot {
            state: row.get(0)?,
            uptime_ms: row.get(1)?,
            free_heap: row.get(2)?,
            recorded_at: row.get(3)?,
        })
    })?.collect::<Result<Vec<_>, _>>()?;

    Ok(Json(history))
}

fn query_config(conn: &rusqlite::Connection, id: &str) -> Result<DeviceConfig, AppError> {
    conn.query_row(
        "SELECT device_id, led_brightness, led_colors, sound_enabled, sound_volume, avatar_preset, custom_data, sound_pack
         FROM device_config WHERE device_id = ?1",
        [id],
        |row| {
            let led_colors_str: Option<String> = row.get(2)?;
            let avatar_preset_str: Option<String> = row.get(5)?;
            let custom_data_str: Option<String> = row.get(6)?;
            Ok(DeviceConfig {
                device_id: row.get(0)?,
                led_brightness: row.get(1)?,
                led_colors: led_colors_str.and_then(|s| serde_json::from_str(&s).ok()),
                sound_enabled: row.get::<_, i32>(3)? != 0,
                sound_volume: row.get(4)?,
                avatar_preset: avatar_preset_str.and_then(|s| serde_json::from_str(&s).ok()),
                custom_data: custom_data_str.and_then(|s| serde_json::from_str(&s).ok()),
                sound_pack: row.get(7)?,
            })
        },
    ).map_err(|_| AppError::NotFound(format!("Config for device {id} not found")))
}

pub async fn get_config(
    State(db): State<DbPool>,
    Path(id): Path<String>,
) -> Result<Json<DeviceConfig>, AppError> {
    let conn = db.lock().unwrap();
    let config = query_config(&conn, &id)?;
    Ok(Json(config))
}

pub async fn update_config(
    State(db): State<DbPool>,
    Path(id): Path<String>,
    Json(input): Json<UpdateConfig>,
) -> Result<Json<DeviceConfig>, AppError> {
    let config = {
        let conn = db.lock().unwrap();

        if let Some(b) = input.led_brightness {
            conn.execute("UPDATE device_config SET led_brightness = ?1 WHERE device_id = ?2", rusqlite::params![b, id])?;
        }
        if let Some(ref c) = input.led_colors {
            let s = serde_json::to_string(c).unwrap();
            conn.execute("UPDATE device_config SET led_colors = ?1 WHERE device_id = ?2", rusqlite::params![s, id])?;
        }
        if let Some(e) = input.sound_enabled {
            conn.execute("UPDATE device_config SET sound_enabled = ?1 WHERE device_id = ?2", rusqlite::params![e as i32, id])?;
        }
        if let Some(v) = input.sound_volume {
            conn.execute("UPDATE device_config SET sound_volume = ?1 WHERE device_id = ?2", rusqlite::params![v, id])?;
        }
        if let Some(ref a) = input.avatar_preset {
            let s = serde_json::to_string(a).unwrap();
            conn.execute("UPDATE device_config SET avatar_preset = ?1 WHERE device_id = ?2", rusqlite::params![s, id])?;
        }
        if let Some(ref d) = input.custom_data {
            let s = serde_json::to_string(d).unwrap();
            conn.execute("UPDATE device_config SET custom_data = ?1 WHERE device_id = ?2", rusqlite::params![s, id])?;
        }
        if let Some(ref sp) = input.sound_pack {
            conn.execute("UPDATE device_config SET sound_pack = ?1 WHERE device_id = ?2", rusqlite::params![sp, id])?;
        }

        query_config(&conn, &id)?
    };

    Ok(Json(config))
}

/// Build the melody payload for a given sound pack name.
/// Returns None for "default" pack (device uses hardcoded melodies).
fn build_sound_pack_payload(pack: &str) -> Option<serde_json::Value> {
    match pack {
        "default" => None,
        "retro" => Some(json!({
            "pack": "retro",
            "melodies": {
                "0": [{"freq": 200, "dur": 80}],
                "1": [{"freq": 300, "dur": 60}, {"freq": 400, "dur": 60}, {"freq": 500, "dur": 60}],
                "2": [{"freq": 400, "dur": 80}, {"freq": 300, "dur": 80}],
                "3": [{"freq": 988, "dur": 80}, {"freq": 1319, "dur": 120}],
                "4": [{"freq": 660, "dur": 80}, {"freq": 880, "dur": 100}],
                "5": [{"freq": 440, "dur": 80}, {"freq": 330, "dur": 80}, {"freq": 220, "dur": 120}]
            }
        })),
        "minimal" => Some(json!({
            "pack": "minimal",
            "melodies": {
                "0": [{"freq": 1000, "dur": 20}],
                "1": [{"freq": 800, "dur": 15}, {"freq": 800, "dur": 15}],
                "2": [{"freq": 200, "dur": 100}],
                "3": [{"freq": 880, "dur": 40}, {"freq": 1100, "dur": 40}],
                "4": [{"freq": 660, "dur": 30}],
                "5": [{"freq": 150, "dur": 100}]
            }
        })),
        "musical" => Some(json!({
            "pack": "musical",
            "melodies": {
                "0": [{"freq": 262, "dur": 100}],
                "1": [{"freq": 262, "dur": 80}, {"freq": 330, "dur": 80}, {"freq": 392, "dur": 80}],
                "2": [{"freq": 262, "dur": 100}, {"freq": 277, "dur": 100}],
                "3": [{"freq": 262, "dur": 80}, {"freq": 330, "dur": 80}, {"freq": 392, "dur": 80}, {"freq": 523, "dur": 120}],
                "4": [{"freq": 392, "dur": 80}, {"freq": 523, "dur": 100}],
                "5": [{"freq": 262, "dur": 80}, {"freq": 311, "dur": 80}, {"freq": 370, "dur": 120}]
            }
        })),
        _ => None,
    }
}

pub async fn push_config(
    State(db): State<DbPool>,
    Path(id): Path<String>,
) -> Result<Json<serde_json::Value>, AppError> {
    let (ip, config, sound_pack) = {
        let conn = db.lock().unwrap();

        let ip = query_device_ip(&conn, &id)?;

        let (config, sound_pack) = conn.query_row(
            "SELECT led_brightness, led_colors, sound_enabled, sound_volume, avatar_preset, custom_data, sound_pack
             FROM device_config WHERE device_id = ?1",
            [&id],
            |row| {
                let led_colors_str: Option<String> = row.get(1)?;
                let avatar_preset_str: Option<String> = row.get(4)?;
                let custom_data_str: Option<String> = row.get(5)?;
                let sound_pack: Option<String> = row.get(6)?;
                Ok((json!({
                    "led_brightness": row.get::<_, i32>(0)?,
                    "led_colors": led_colors_str.and_then(|s| serde_json::from_str::<serde_json::Value>(&s).ok()),
                    "sound_enabled": row.get::<_, i32>(2)? != 0,
                    "sound_volume": row.get::<_, i32>(3)?,
                    "avatar_preset": avatar_preset_str.and_then(|s| serde_json::from_str::<serde_json::Value>(&s).ok()),
                    "custom_data": custom_data_str.and_then(|s| serde_json::from_str::<serde_json::Value>(&s).ok()),
                }), sound_pack))
            },
        ).map_err(|_| AppError::NotFound(format!("Config for device {id} not found")))?;

        (ip, config, sound_pack)
    };

    proxy::forward_json(&format!("http://{}/config", ip), &config).await?;

    // Push sound pack data to device if not default
    let pack_name = sound_pack.as_deref().unwrap_or("default");
    if let Some(sound_payload) = build_sound_pack_payload(pack_name) {
        let _ = proxy::forward_json(&format!("http://{}/sounds", ip), &sound_payload).await;
    } else {
        // "default" pack: tell device to disable custom melodies
        let _ = proxy::forward_json(&format!("http://{}/sounds", ip), &json!({"pack": "default"})).await;
    }

    Ok(Json(json!({ "ok": true })))
}

/// GET /api/devices/:id/config/export - export full device configuration as JSON backup
pub async fn export_config(
    State(db): State<DbPool>,
    Path(id): Path<String>,
) -> Result<Json<ConfigExport>, AppError> {
    let conn = db.lock().unwrap();

    // Device info
    let device = query_device(&conn, &id)?;

    // Device config
    let config = query_config(&conn, &id)?;

    // Sensor configs
    let mut sensor_stmt = conn.prepare(
        "SELECT id, device_id, channel, pin, sensor_type, label, poll_interval_ms, threshold
         FROM sensor_configs WHERE device_id = ?1 ORDER BY channel",
    )?;
    let sensor_configs = sensor_stmt.query_map([&id], |row| {
        Ok(SensorConfig {
            id: row.get(0)?,
            device_id: row.get(1)?,
            channel: row.get(2)?,
            pin: row.get(3)?,
            sensor_type: row.get(4)?,
            label: row.get(5)?,
            poll_interval_ms: row.get(6)?,
            threshold: row.get(7)?,
        })
    })?
    .collect::<Result<Vec<_>, _>>()?;

    // Automation rules (strip IDs for portability)
    let mut rules_stmt = conn.prepare(
        "SELECT name, enabled, trigger_type, trigger_config, action_type, action_config, cooldown_secs
         FROM automation_rules WHERE device_id = ?1 ORDER BY created_at",
    )?;
    let automation_rules = rules_stmt.query_map([&id], |row| {
        let tc: String = row.get(3)?;
        let ac: String = row.get(5)?;
        Ok(ExportRule {
            name: row.get(0)?,
            enabled: row.get::<_, i32>(1)? != 0,
            trigger_type: row.get(2)?,
            trigger_config: serde_json::from_str(&tc).unwrap_or_default(),
            action_type: row.get(4)?,
            action_config: serde_json::from_str(&ac).unwrap_or_default(),
            cooldown_secs: row.get(6)?,
        })
    })?
    .collect::<Result<Vec<_>, _>>()?;

    // Try to get servo config from live device
    let servo_config = {
        let ip = query_device_ip(&conn, &id).ok();
        drop(conn);
        if let Some(ip) = ip {
            proxy::get_json(&format!("http://{}/servos", ip)).await.ok()
        } else {
            None
        }
    };

    // Firmware version from latest successful OTA job
    let conn = db.lock().unwrap();
    let firmware_version: Option<String> = conn
        .query_row(
            "SELECT f.version FROM ota_jobs o
             JOIN firmware f ON f.id = o.firmware_id
             WHERE o.device_id = ?1 AND o.status = 'success'
             ORDER BY o.created_at DESC LIMIT 1",
            [&id],
            |row| row.get(0),
        )
        .ok();

    Ok(Json(ConfigExport {
        metadata: ExportMetadata {
            export_date: chrono::Utc::now().format("%Y-%m-%dT%H:%M:%SZ").to_string(),
            firmware_version,
        },
        device_info: ExportDeviceInfo {
            name: device.name,
            hostname: device.hostname,
            purpose: device.purpose,
            personality: device.personality,
            device_type: device.device_type,
        },
        device_config: config,
        servo_config,
        sensor_configs,
        automation_rules,
    }))
}

/// POST /api/devices/:id/config/import - import a previously exported config JSON
pub async fn import_config(
    State(db): State<DbPool>,
    Path(id): Path<String>,
    Json(input): Json<ConfigExport>,
) -> Result<Json<serde_json::Value>, AppError> {
    let conn = db.lock().unwrap();

    // Verify device exists
    let _device = query_device(&conn, &id)?;

    // Update device soft fields (purpose, personality, device_type) but NOT name/hostname/ip
    if let Some(purpose) = &input.device_info.purpose {
        conn.execute(
            "UPDATE devices SET purpose = ?1, updated_at = datetime('now') WHERE id = ?2",
            rusqlite::params![purpose, id],
        )?;
    }
    if let Some(personality) = &input.device_info.personality {
        conn.execute(
            "UPDATE devices SET personality = ?1, updated_at = datetime('now') WHERE id = ?2",
            rusqlite::params![personality, id],
        )?;
    }

    // Update device config
    let cfg = &input.device_config;
    conn.execute(
        "UPDATE device_config SET led_brightness = ?1, sound_enabled = ?2, sound_volume = ?3, sound_pack = ?4 WHERE device_id = ?5",
        rusqlite::params![cfg.led_brightness, cfg.sound_enabled as i32, cfg.sound_volume, cfg.sound_pack.as_deref().unwrap_or("default"), id],
    )?;
    if let Some(ref colors) = cfg.led_colors {
        let s = serde_json::to_string(colors).unwrap();
        conn.execute(
            "UPDATE device_config SET led_colors = ?1 WHERE device_id = ?2",
            rusqlite::params![s, id],
        )?;
    }
    if let Some(ref preset) = cfg.avatar_preset {
        let s = serde_json::to_string(preset).unwrap();
        conn.execute(
            "UPDATE device_config SET avatar_preset = ?1 WHERE device_id = ?2",
            rusqlite::params![s, id],
        )?;
    }
    if let Some(ref data) = cfg.custom_data {
        let s = serde_json::to_string(data).unwrap();
        conn.execute(
            "UPDATE device_config SET custom_data = ?1 WHERE device_id = ?2",
            rusqlite::params![s, id],
        )?;
    }

    // Replace sensor configs
    conn.execute("DELETE FROM sensor_configs WHERE device_id = ?1", [&id])?;
    for sc in &input.sensor_configs {
        conn.execute(
            "INSERT INTO sensor_configs (device_id, channel, pin, sensor_type, label, poll_interval_ms, threshold)
             VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7)",
            rusqlite::params![id, sc.channel, sc.pin, sc.sensor_type, sc.label, sc.poll_interval_ms, sc.threshold],
        )?;
    }

    // Replace automation rules (create new IDs)
    conn.execute("DELETE FROM automation_rules WHERE device_id = ?1", [&id])?;
    for rule in &input.automation_rules {
        let rule_id = uuid::Uuid::new_v4().to_string();
        let tc = serde_json::to_string(&rule.trigger_config).unwrap_or_else(|_| "{}".to_string());
        let ac = serde_json::to_string(&rule.action_config).unwrap_or_else(|_| "{}".to_string());
        conn.execute(
            "INSERT INTO automation_rules (id, device_id, name, enabled, trigger_type, trigger_config, action_type, action_config, cooldown_secs)
             VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, ?9)",
            rusqlite::params![rule_id, id, rule.name, rule.enabled as i32, rule.trigger_type, tc, rule.action_type, ac, rule.cooldown_secs],
        )?;
    }

    // Try to push servo config to live device
    let servo_pushed = if let Some(ref servo_config) = input.servo_config {
        let ip = query_device_ip(&conn, &id).ok();
        drop(conn);
        if let Some(ip) = ip {
            proxy::forward_json(&format!("http://{}/servos/config", ip), servo_config)
                .await
                .is_ok()
        } else {
            false
        }
    } else {
        false
    };

    Ok(Json(json!({
        "ok": true,
        "imported": {
            "device_config": true,
            "sensor_configs": input.sensor_configs.len(),
            "automation_rules": input.automation_rules.len(),
            "servo_config_pushed": servo_pushed,
        }
    })))
}

/// POST /api/devices/:id/animation - forward animation to device
pub async fn forward_animation(
    State(db): State<DbPool>,
    Path(id): Path<String>,
    Json(body): Json<serde_json::Value>,
) -> Result<Json<serde_json::Value>, AppError> {
    let ip = {
        let conn = db.lock().unwrap();
        query_device_ip(&conn, &id)?
    };
    proxy::forward_json(&format!("http://{}/animation", ip), &body).await?;
    Ok(Json(json!({ "ok": true })))
}

/// POST /api/devices/:id/animation/stop - stop animation on device
pub async fn stop_animation(
    State(db): State<DbPool>,
    Path(id): Path<String>,
) -> Result<Json<serde_json::Value>, AppError> {
    let ip = {
        let conn = db.lock().unwrap();
        query_device_ip(&conn, &id)?
    };
    proxy::forward_json(&format!("http://{}/animation/stop", ip), &json!({})).await?;
    Ok(Json(json!({ "ok": true })))
}
