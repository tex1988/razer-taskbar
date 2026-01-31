use serde::{Deserialize, Serialize};
use std::fs;
use std::path::PathBuf;
use anyhow::Result;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Settings {
    #[serde(default = "default_run_at_startup")]
    pub run_at_startup: bool,

    #[serde(default = "default_show_percentage")]
    pub show_percentage: bool,


    #[serde(default = "default_polling_interval")]
    pub polling_interval_minutes: u64,

    #[serde(default = "default_display_charging")]
    pub display_charging_state: bool,

    #[serde(default)]
    pub shown_device_handle: String,

    #[serde(default = "default_synapse_version")]
    pub synapse_version: SynapseVersion,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
#[serde(rename_all = "lowercase")]
pub enum SynapseVersion {
    Auto,
    V3,
    V4,
}

fn default_run_at_startup() -> bool { false }
fn default_show_percentage() -> bool { false }
fn default_polling_interval() -> u64 { 10 }
fn default_display_charging() -> bool { true }
fn default_synapse_version() -> SynapseVersion { SynapseVersion::Auto }

impl Default for Settings {
    fn default() -> Self {
        Self {
            run_at_startup: default_run_at_startup(),
            show_percentage: default_show_percentage(),
            polling_interval_minutes: default_polling_interval(),
            display_charging_state: default_display_charging(),
            shown_device_handle: String::new(),
            synapse_version: default_synapse_version(),
        }
    }
}

impl Settings {
    pub fn load() -> Result<Self> {
        let path = Self::settings_path();
        if path.exists() {
            let content = fs::read_to_string(&path)?;
            Ok(serde_json::from_str(&content)?)
        } else {
            let settings = Self::default();
            settings.save()?;
            Ok(settings)
        }
    }

    pub fn save(&self) -> Result<()> {
        let path = Self::settings_path();
        if let Some(parent) = path.parent() {
            fs::create_dir_all(parent)?;
        }
        let content = serde_json::to_string_pretty(self)?;
        fs::write(&path, content)?;
        Ok(())
    }

    fn settings_path() -> PathBuf {
        let data_dir = dirs::data_local_dir()
            .unwrap_or_else(|| PathBuf::from("."));
        data_dir.join("razer-taskbar").join("settings.json")
    }
}
