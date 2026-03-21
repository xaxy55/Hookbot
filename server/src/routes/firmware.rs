use axum::extract::{Multipart, Path, State};
use axum::body::Body;
use axum::Json;
use sha2::{Digest, Sha256};
use tokio::fs;

use crate::config::AppConfig;
use crate::db::DbPool;
use crate::error::AppError;
use crate::models::Firmware;

pub async fn upload_firmware(
    State((db, config)): State<(DbPool, AppConfig)>,
    mut multipart: Multipart,
) -> Result<Json<Firmware>, AppError> {
    let mut version = String::new();
    let mut notes: Option<String> = None;
    let mut device_type: Option<String> = None;
    let mut file_data: Option<(String, Vec<u8>)> = None;

    while let Some(field) = multipart.next_field().await.map_err(|e| AppError::BadRequest(e.to_string()))? {
        let name = field.name().unwrap_or("").to_string();
        match name.as_str() {
            "version" => {
                version = field.text().await.map_err(|e| AppError::BadRequest(e.to_string()))?;
            }
            "notes" => {
                notes = Some(field.text().await.map_err(|e| AppError::BadRequest(e.to_string()))?);
            }
            "device_type" => {
                device_type = Some(field.text().await.map_err(|e| AppError::BadRequest(e.to_string()))?);
            }
            "file" => {
                let filename = field.file_name().unwrap_or("firmware.bin").to_string();
                let data = field.bytes().await.map_err(|e| AppError::BadRequest(e.to_string()))?;
                file_data = Some((filename, data.to_vec()));
            }
            _ => {}
        }
    }

    let (filename, data) = file_data.ok_or_else(|| AppError::BadRequest("No file uploaded".into()))?;
    if version.is_empty() {
        return Err(AppError::BadRequest("Version is required".into()));
    }

    let mut hasher = Sha256::new();
    hasher.update(&data);
    let checksum = hex::encode(hasher.finalize());

    let id = uuid::Uuid::new_v4().to_string();
    let size_bytes = data.len() as i64;

    // Save file
    fs::create_dir_all(&config.firmware_dir).await
        .map_err(|e| AppError::Internal(format!("Failed to create firmware dir: {e}")))?;
    let file_path = config.firmware_dir.join(&id);
    // Ensure the resolved path stays within firmware_dir (path traversal guard)
    let canonical_dir = config.firmware_dir.canonicalize()
        .map_err(|e| AppError::Internal(format!("Failed to resolve firmware dir: {e}")))?;
    let canonical_file = file_path.canonicalize().unwrap_or_else(|_| canonical_dir.join(&id));
    if !canonical_file.starts_with(&canonical_dir) {
        return Err(AppError::BadRequest("Invalid firmware path".into()));
    }
    fs::write(&file_path, &data).await
        .map_err(|e| AppError::Internal(format!("Failed to save firmware: {e}")))?;

    let conn = db.lock().unwrap();
    conn.execute(
        "INSERT INTO firmware (id, version, filename, size_bytes, checksum, notes, device_type) VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7)",
        rusqlite::params![id, version, filename, size_bytes, checksum, notes, device_type],
    )?;
    drop(conn);

    let firmware = Firmware {
        id,
        version,
        filename,
        size_bytes,
        checksum,
        uploaded_at: chrono::Utc::now().format("%Y-%m-%d %H:%M:%S").to_string(),
        notes,
        device_type,
    };

    Ok(Json(firmware))
}

pub async fn list_firmware(
    State((db, _config)): State<(DbPool, AppConfig)>,
) -> Result<Json<Vec<Firmware>>, AppError> {
    let conn = db.lock().unwrap();
    let mut stmt = conn.prepare(
        "SELECT id, version, filename, size_bytes, checksum, uploaded_at, notes, device_type FROM firmware ORDER BY uploaded_at DESC",
    )?;

    let firmwares = stmt.query_map([], |row| {
        Ok(Firmware {
            id: row.get(0)?,
            version: row.get(1)?,
            filename: row.get(2)?,
            size_bytes: row.get(3)?,
            checksum: row.get(4)?,
            uploaded_at: row.get(5)?,
            notes: row.get(6)?,
            device_type: row.get(7)?,
        })
    })?.collect::<Result<Vec<_>, _>>()?;

    Ok(Json(firmwares))
}

pub async fn serve_firmware_binary(
    State((_db, config)): State<(DbPool, AppConfig)>,
    Path(id): Path<String>,
) -> Result<Body, AppError> {
    // Validate id to prevent path traversal
    if id.contains('/') || id.contains('\\') || id.contains("..") {
        return Err(AppError::BadRequest("Invalid firmware id".into()));
    }
    let file_path = config.firmware_dir.join(&id);
    // Ensure the resolved path stays within firmware_dir
    let canonical_dir = config.firmware_dir.canonicalize()
        .map_err(|_| AppError::Internal("Failed to resolve firmware dir".into()))?;
    let canonical_file = file_path.canonicalize()
        .map_err(|_| AppError::NotFound(format!("Firmware binary {id} not found")))?;
    if !canonical_file.starts_with(&canonical_dir) {
        return Err(AppError::BadRequest("Invalid firmware id".into()));
    }
    let data = fs::read(&canonical_file).await
        .map_err(|_| AppError::NotFound(format!("Firmware binary {id} not found")))?;

    Ok(Body::from(data))
}
