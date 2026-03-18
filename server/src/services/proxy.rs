use crate::error::AppError;
use serde_json::json;

pub async fn forward_json(url: &str, body: &serde_json::Value) -> Result<serde_json::Value, AppError> {
    let client = reqwest::Client::builder()
        .timeout(std::time::Duration::from_secs(5))
        .build()
        .map_err(|e| AppError::Internal(format!("HTTP client error: {e}")))?;

    let resp = client.post(url)
        .json(body)
        .send()
        .await
        .map_err(|e| AppError::Internal(format!("Failed to reach device at {url}: {e}")))?;

    let status = resp.status();
    let text = resp.text().await.unwrap_or_default();

    match serde_json::from_str(&text) {
        Ok(v) => Ok(v),
        Err(_) => Ok(json!({ "ok": status.is_success(), "raw": text })),
    }
}

pub async fn get_json(url: &str) -> Result<serde_json::Value, AppError> {
    let client = reqwest::Client::builder()
        .timeout(std::time::Duration::from_secs(5))
        .build()
        .map_err(|e| AppError::Internal(format!("HTTP client error: {e}")))?;

    let resp = client.get(url)
        .send()
        .await
        .map_err(|e| AppError::Internal(format!("Failed to reach device at {url}: {e}")))?;

    let status = resp.status();
    let text = resp.text().await.unwrap_or_default();

    match serde_json::from_str(&text) {
        Ok(v) => Ok(v),
        Err(_) => Ok(json!({ "ok": status.is_success(), "raw": text })),
    }
}
