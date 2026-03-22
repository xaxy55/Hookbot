// Prevents additional console window on Windows in release
#![cfg_attr(not(debug_assertions), windows_subsystem = "windows")]

mod status;

use serde::Serialize;
use status::HookbotClient;
use std::sync::Arc;
use tauri::{
    tray::TrayIconEvent, AppHandle, Emitter, Manager, WindowEvent,
};
use tokio::sync::Mutex;

#[derive(Clone, Serialize)]
struct StatusPayload {
    devices: Vec<status::DeviceStatus>,
    online_count: usize,
    total_count: usize,
    xp: Option<status::XpInfo>,
    server_online: bool,
}

#[tauri::command]
fn get_config() -> ServerConfig {
    ServerConfig::from_env()
}

#[tauri::command]
async fn set_server_url(
    url: String,
    state: tauri::State<'_, Arc<Mutex<HookbotClient>>>,
) -> Result<(), String> {
    let mut client = state.lock().await;
    client.set_base_url(&url);
    Ok(())
}

#[tauri::command]
async fn set_api_key(
    key: String,
    state: tauri::State<'_, Arc<Mutex<HookbotClient>>>,
) -> Result<(), String> {
    let mut client = state.lock().await;
    client.set_api_key(&key);
    Ok(())
}

#[tauri::command]
async fn fetch_status(
    state: tauri::State<'_, Arc<Mutex<HookbotClient>>>,
) -> Result<StatusPayload, String> {
    let client = state.lock().await;
    let (devices, xp) = tokio::join!(client.get_devices(), client.get_xp());

    let devices = devices.unwrap_or_default();
    let online_count = devices.iter().filter(|d| d.online).count();
    let total_count = devices.len();

    Ok(StatusPayload {
        devices,
        online_count,
        total_count,
        xp: xp.ok(),
        server_online: total_count > 0 || xp.is_ok(),
    })
}

#[derive(Clone, Serialize, serde::Deserialize)]
struct ServerConfig {
    url: String,
    api_key: Option<String>,
    poll_interval_secs: u64,
}

impl ServerConfig {
    fn from_env() -> Self {
        Self {
            url: std::env::var("HOOKBOT_URL")
                .unwrap_or_else(|_| "http://localhost:3001".to_string()),
            api_key: std::env::var("HOOKBOT_API_KEY").ok(),
            poll_interval_secs: std::env::var("HOOKBOT_POLL_INTERVAL")
                .ok()
                .and_then(|v| v.parse().ok())
                .unwrap_or(5),
        }
    }
}

fn setup_tray_events(app: &AppHandle) {
    let app_handle = app.clone();
    if let Some(tray) = app.tray_by_id("main") {
        tray.on_tray_icon_event(move |_tray, event| {
            if let TrayIconEvent::Click { .. } = event {
                if let Some(window) = app_handle.get_webview_window("main") {
                    if window.is_visible().unwrap_or(false) {
                        let _ = window.hide();
                    } else {
                        let _ = window.show();
                        let _ = window.set_focus();
                    }
                }
            }
        });
    }
}

fn start_polling(app: &AppHandle) {
    let app_handle = app.clone();
    let config = ServerConfig::from_env();
    let client = Arc::new(Mutex::new(HookbotClient::new(&config.url, config.api_key.as_deref())));

    app.manage(client.clone());

    tauri::async_runtime::spawn(async move {
        let interval = tokio::time::Duration::from_secs(config.poll_interval_secs);
        loop {
            let client = client.lock().await;
            let (devices, xp) = tokio::join!(client.get_devices(), client.get_xp());
            drop(client);

            let devices = devices.unwrap_or_default();
            let online_count = devices.iter().filter(|d| d.online).count();
            let total_count = devices.len();

            let payload = StatusPayload {
                devices,
                online_count,
                total_count,
                xp: xp.ok(),
                server_online: total_count > 0,
            };

            let _ = app_handle.emit("hookbot-status", &payload);

            // Update tray tooltip
            if let Some(tray) = app_handle.tray_by_id("main") {
                let tooltip = format!("Hookbot: {}/{} online", online_count, total_count);
                let _ = tray.set_tooltip(Some(&tooltip));
            }

            tokio::time::sleep(interval).await;
        }
    });
}

fn main() {
    tauri::Builder::default()
        .plugin(tauri_plugin_positioner::init())
        .invoke_handler(tauri::generate_handler![
            get_config,
            set_server_url,
            set_api_key,
            fetch_status,
        ])
        .setup(|app| {
            setup_tray_events(app.handle());
            start_polling(app.handle());

            // Hide window when it loses focus (widget behavior)
            if let Some(window) = app.get_webview_window("main") {
                let w = window.clone();
                window.on_window_event(move |event| {
                    if let WindowEvent::Focused(false) = event {
                        let _ = w.hide();
                    }
                });
            }

            Ok(())
        })
        .run(tauri::generate_context!())
        .expect("error while running hookbot desktop widget");
}
