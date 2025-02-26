use crate::core::error::ConfigError;
use serde::{Deserialize, Serialize};
use serde_json::json;
use tauri::AppHandle;
use tauri_plugin_store::StoreExt;

const SETTINGS_FILE: &str = "settings.json";

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Settings {
    pub shortcuts: ShortcutConfig,
    pub audio: AudioConfig,
    pub window: WindowConfig,
}

impl Default for Settings {
    fn default() -> Self {
        Self {
            shortcuts: ShortcutConfig {
                record_key: Some("Space".to_string()),
                record_modifier: Some("CONTROL".to_string()),
            },
            audio: AudioConfig {
                default_device: None,
            },
            window: WindowConfig {
                width: 400.0,
                height: 80.0,
            },
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ShortcutConfig {
    #[serde(default = "default_record_modifier")]
    pub record_modifier: Option<String>,
    #[serde(default = "default_record_key")]
    pub record_key: Option<String>,
}

fn default_record_modifier() -> Option<String> {
    Some("CONTROL".to_string())
}

fn default_record_key() -> Option<String> {
    Some("Space".to_string())
}

impl Default for ShortcutConfig {
    fn default() -> Self {
        Self {
            record_modifier: Some("CONTROL".to_string()),
            record_key: Some("Space".to_string()),
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AudioConfig {
    pub default_device: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct WindowConfig {
    pub width: f64,
    pub height: f64,
}

impl Settings {
    pub fn load(app_handle: &AppHandle) -> Result<Self, ConfigError> {
        let store = app_handle
            .store(SETTINGS_FILE)
            .map_err(|e| ConfigError::Loading(e.to_string()))?;

        println!("Loading settings from store...");

        if let Some(settings) = store.get("settings") {
            println!("Found existing settings: {:?}", settings);

            let settings: Settings = serde_json::from_value(settings)
                .map_err(|e| ConfigError::Loading(format!("Failed to parse settings: {}", e)))?;
            return Ok(settings);
        }

        println!("No existing settings found, creating defaults...");
        let default_settings = Self::default();
        println!("Default settings: {:?}", default_settings);

        store.set("settings", json!(default_settings.clone()));

        store
            .save()
            .map_err(|e| ConfigError::Loading(format!("Failed to persist settings: {}", e)))?;

        Ok(default_settings)
    }

    pub fn save(&self, app_handle: &AppHandle) -> Result<(), ConfigError> {
        let store = app_handle
            .store(SETTINGS_FILE)
            .map_err(|e| ConfigError::Loading(e.to_string()))?;

        store.set("settings", json!(self));

        store
            .save()
            .map_err(|e| ConfigError::Loading(e.to_string()))
    }

    pub fn update_shortcuts(
        &mut self,
        app_handle: &AppHandle,
        key: String,
        modifier: String,
    ) -> Result<(), ConfigError> {
        self.shortcuts.record_key = Some(key);
        self.shortcuts.record_modifier = Some(modifier);
        self.save(app_handle)
    }
}
