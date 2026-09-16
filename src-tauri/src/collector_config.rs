use serde::{Deserialize, Serialize};
use std::fs;
use std::path::PathBuf;
use tauri::{AppHandle, Manager};

use crate::dish::default_dish_address;
use crate::router::default_router_address;

const CONFIG_FILE: &str = "collector-config.json";

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct CollectorConfig {
    pub dish_address: String,
    pub dish_enabled: bool,
    pub router_address: String,
    pub router_enabled: bool,
    pub poll_interval_s: u32,
    pub retention_days: u32,
}

impl Default for CollectorConfig {
    fn default() -> Self {
        Self {
            dish_address: default_dish_address(),
            dish_enabled: true,
            router_address: default_router_address(),
            router_enabled: false,
            poll_interval_s: 10,
            retention_days: 3,
        }
    }
}

fn config_path(app: &AppHandle) -> Result<PathBuf, String> {
    let dir = app
        .path()
        .app_data_dir()
        .map_err(|e| format!("failed to resolve app data dir: {e}"))?;
    fs::create_dir_all(&dir).map_err(|e| format!("failed to create app data dir: {e}"))?;
    Ok(dir.join(CONFIG_FILE))
}

pub fn load(app: &AppHandle) -> CollectorConfig {
    let Ok(path) = config_path(app) else {
        return CollectorConfig::default();
    };
    let Ok(raw) = fs::read_to_string(&path) else {
        return CollectorConfig::default();
    };
    serde_json::from_str(&raw).unwrap_or_default()
}

pub fn save(app: &AppHandle, config: &CollectorConfig) -> Result<(), String> {
    let path = config_path(app)?;
    let raw = serde_json::to_string_pretty(config)
        .map_err(|e| format!("failed to serialize collector config: {e}"))?;
    fs::write(&path, raw).map_err(|e| format!("failed to write collector config: {e}"))
}

#[tauri::command]
pub fn get_collector_config(app: AppHandle) -> CollectorConfig {
    load(&app)
}

#[tauri::command]
pub fn save_collector_config(app: AppHandle, config: CollectorConfig) -> Result<(), String> {
    save(&app, &config)
}
