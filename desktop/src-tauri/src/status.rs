use serde::{Deserialize, Serialize};

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct DeviceStatus {
    pub id: String,
    pub name: String,
    pub hostname: String,
    pub ip_address: String,
    pub online: bool,
    pub device_type: Option<String>,
    pub latest_status: Option<StatusSnapshot>,
}

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct StatusSnapshot {
    pub state: String,
    pub uptime_ms: u64,
    pub free_heap: u64,
    pub recorded_at: String,
}

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct XpInfo {
    pub total_xp: u64,
    pub level: u32,
    pub title: String,
    pub current_streak: u32,
    pub xp_for_current_level: u64,
    pub xp_for_next_level: u64,
}

pub struct HookbotClient {
    base_url: String,
    client: reqwest::Client,
}

impl HookbotClient {
    pub fn new(base_url: &str, api_key: Option<&str>) -> Self {
        let mut headers = reqwest::header::HeaderMap::new();
        if let Some(key) = api_key {
            headers.insert(
                reqwest::header::AUTHORIZATION,
                reqwest::header::HeaderValue::from_str(&format!("Bearer {key}"))
                    .unwrap_or_else(|_| reqwest::header::HeaderValue::from_static("")),
            );
        }
        let client = reqwest::Client::builder()
            .default_headers(headers)
            .timeout(std::time::Duration::from_secs(5))
            .build()
            .expect("Failed to build HTTP client");

        Self {
            base_url: base_url.trim_end_matches('/').to_string(),
            client,
        }
    }

    pub fn set_base_url(&mut self, url: &str) {
        self.base_url = url.trim_end_matches('/').to_string();
    }

    pub fn set_api_key(&mut self, key: &str) {
        let mut headers = reqwest::header::HeaderMap::new();
        headers.insert(
            reqwest::header::AUTHORIZATION,
            reqwest::header::HeaderValue::from_str(&format!("Bearer {key}"))
                .unwrap_or_else(|_| reqwest::header::HeaderValue::from_static("")),
        );
        self.client = reqwest::Client::builder()
            .default_headers(headers)
            .timeout(std::time::Duration::from_secs(5))
            .build()
            .expect("Failed to rebuild HTTP client");
    }

    pub async fn get_devices(&self) -> Result<Vec<DeviceStatus>, String> {
        let url = format!("{}/api/devices", self.base_url);
        self.client
            .get(&url)
            .send()
            .await
            .map_err(|e| e.to_string())?
            .json::<Vec<DeviceStatus>>()
            .await
            .map_err(|e| e.to_string())
    }

    pub async fn get_xp(&self) -> Result<XpInfo, String> {
        let url = format!("{}/api/gamification/stats", self.base_url);
        self.client
            .get(&url)
            .send()
            .await
            .map_err(|e| e.to_string())?
            .json::<XpInfo>()
            .await
            .map_err(|e| e.to_string())
    }
}
