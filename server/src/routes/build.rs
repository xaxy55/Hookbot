use axum::extract::State;
use axum::Json;
use serde::{Deserialize, Serialize};
use sha2::{Digest, Sha256};
use tokio::fs;
use tracing::info;

use crate::config::AppConfig;
use crate::db::DbPool;
use crate::error::AppError;
use crate::models::Firmware;

#[derive(Debug, Deserialize)]
pub struct BuildRequest {
    /// PlatformIO environment name: "esp32" or "esp32-4848s040c"
    pub environment: String,
    /// Optional version string for the built firmware
    pub version: Option<String>,
    /// Optional notes
    pub notes: Option<String>,
}

#[derive(Debug, Serialize)]
pub struct BuildStatus {
    pub status: String,
    pub message: String,
    pub firmware: Option<Firmware>,
    pub build_log: Option<String>,
}

fn env_to_device_type(env: &str) -> &str {
    match env {
        "esp32-4848s040c" => "esp32_4848s040c_lcd",
        "esp32" => "esp32_oled",
        _ => "unknown",
    }
}

pub async fn build_firmware(
    State((db, config)): State<(DbPool, AppConfig)>,
    Json(input): Json<BuildRequest>,
) -> Result<Json<BuildStatus>, AppError> {
    let env = &input.environment;

    // Validate environment name
    if env != "esp32" && env != "esp32-4848s040c" {
        return Err(AppError::BadRequest(format!(
            "Invalid environment '{}'. Must be 'esp32' or 'esp32-4848s040c'",
            env
        )));
    }

    // Find the firmware directory (relative to server working dir)
    let firmware_dir = std::env::var("FIRMWARE_DIR_SRC")
        .unwrap_or_else(|_| "../firmware".to_string());

    let firmware_path = std::path::PathBuf::from(&firmware_dir);
    if !firmware_path.exists() {
        return Err(AppError::BadRequest(format!(
            "Firmware source directory not found at '{}'. Set FIRMWARE_DIR_SRC env var.",
            firmware_dir
        )));
    }

    info!("Building firmware for environment: {}", env);

    // Run PlatformIO build
    let output = tokio::process::Command::new("pio")
        .args(["run", "-e", env])
        .current_dir(&firmware_path)
        .output()
        .await
        .map_err(|e| AppError::Internal(format!(
            "Failed to run PlatformIO: {}. Is 'pio' installed and in PATH?", e
        )))?;

    let stdout = String::from_utf8_lossy(&output.stdout).to_string();
    let stderr = String::from_utf8_lossy(&output.stderr).to_string();
    let build_log = format!("{}\n{}", stdout, stderr);

    if !output.status.success() {
        return Ok(Json(BuildStatus {
            status: "failed".into(),
            message: "Build failed".into(),
            firmware: None,
            build_log: Some(build_log),
        }));
    }

    // Find the built binary
    let bin_path = firmware_path
        .join(".pio")
        .join("build")
        .join(env)
        .join("firmware.bin");

    if !bin_path.exists() {
        return Ok(Json(BuildStatus {
            status: "failed".into(),
            message: format!("Build succeeded but binary not found at {:?}", bin_path),
            firmware: None,
            build_log: Some(build_log),
        }));
    }

    let data = fs::read(&bin_path).await
        .map_err(|e| AppError::Internal(format!("Failed to read built firmware: {e}")))?;

    // Compute checksum
    let mut hasher = Sha256::new();
    hasher.update(&data);
    let checksum = hex::encode(hasher.finalize());

    let id = uuid::Uuid::new_v4().to_string();
    let size_bytes = data.len() as i64;
    let version = input.version.unwrap_or_else(|| {
        chrono::Utc::now().format("build-%Y%m%d-%H%M%S").to_string()
    });
    let device_type = env_to_device_type(env).to_string();
    let filename = format!("firmware-{}-{}.bin", env, version);

    // Save binary to firmware storage
    fs::create_dir_all(&config.firmware_dir).await
        .map_err(|e| AppError::Internal(format!("Failed to create firmware dir: {e}")))?;
    // Ensure the resolved path stays within firmware_dir (path traversal guard)
    let canonical_dir = config.firmware_dir.canonicalize()
        .map_err(|e| AppError::Internal(format!("Failed to resolve firmware dir: {e}")))?;
    let safe_name = std::path::Path::new(&id).file_name()
        .ok_or_else(|| AppError::Internal("Invalid firmware ID".into()))?;
    let fw_path = canonical_dir.join(safe_name);
    if !fw_path.starts_with(&canonical_dir) {
        return Err(AppError::Internal("Invalid firmware storage path".into()));
    }
    fs::write(&fw_path, &data).await
        .map_err(|e| AppError::Internal(format!("Failed to save firmware: {e}")))?;

    // Store in database
    {
        let conn = db.lock().unwrap();
        conn.execute(
            "INSERT INTO firmware (id, version, filename, size_bytes, checksum, notes, device_type) VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7)",
            rusqlite::params![id, version, filename, size_bytes, checksum, input.notes, device_type],
        )?;
    }

    let firmware = Firmware {
        id,
        version,
        filename,
        size_bytes,
        checksum,
        uploaded_at: chrono::Utc::now().format("%Y-%m-%d %H:%M:%S").to_string(),
        notes: input.notes,
        device_type: Some(device_type),
    };

    info!("Firmware built successfully: {} bytes", size_bytes);

    Ok(Json(BuildStatus {
        status: "success".into(),
        message: format!("Built {} firmware ({} bytes)", env, size_bytes),
        firmware: Some(firmware),
        build_log: Some(build_log),
    }))
}
