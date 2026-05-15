use serde::{Deserialize, Serialize};
use std::fs;
use std::path::PathBuf;
use anyhow::Result;
use super::icon_settings::IconSettings;
use super::device::DeviceConfig;

// ── LogFontData ────────────────────────────────────────────────

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct LogFontData {
    pub lf_height: i32,
    pub lf_width: i32,
    pub lf_escapement: i32,
    pub lf_orientation: i32,
    pub lf_weight: i32,
    pub lf_italic: u8,
    pub lf_underline: u8,
    pub lf_strike_out: u8,
    pub lf_char_set: u8,
    pub lf_out_precision: u8,
    pub lf_clip_precision: u8,
    pub lf_quality: u8,
    pub lf_pitch_and_family: u8,
    pub lf_face_name: String,
}

impl Default for LogFontData {
    fn default() -> Self {
        Self {
            lf_height: -20,
            lf_width: 0,
            lf_escapement: 0,
            lf_orientation: 0,
            lf_weight: 400,
            lf_italic: 0,
            lf_underline: 0,
            lf_strike_out: 0,
            lf_char_set: 0,
            lf_out_precision: 0,
            lf_clip_precision: 0,
            lf_quality: 0,
            lf_pitch_and_family: 0,
            lf_face_name: "Arial".to_string(),
        }
    }
}

// ── Enums ──────────────────────────────────────────────────────

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
#[serde(rename_all = "lowercase")]
pub enum TextAlignment { Left, Center, Right }

impl TextAlignment {
    pub fn as_str(&self) -> &'static str {
        match self {
            TextAlignment::Left => "left",
            TextAlignment::Center => "center",
            TextAlignment::Right => "right",
        }
    }
}

impl Default for TextAlignment {
    fn default() -> Self { TextAlignment::Center }
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
#[serde(rename_all = "lowercase")]
pub enum SynapseVersion { Auto, V3, V4 }

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
#[serde(rename_all = "lowercase")]
pub enum IconTheme { Dark, Light, System }

impl IconTheme {
    pub fn as_str(&self) -> &'static str {
        match self {
            IconTheme::Dark => "dark",
            IconTheme::Light => "light",
            IconTheme::System => "system",
        }
    }
}

impl Default for IconTheme {
    fn default() -> Self { IconTheme::System }
}

// ── Default helpers ────────────────────────────────────────────

fn default_run_at_startup() -> bool { false }
fn default_show_percentage() -> bool { false }
fn default_text_size() -> u32 { 20 }
fn default_text_color() -> String { "FFFFFF".to_string() }
fn default_text_font() -> String { "Arial".to_string() }
fn default_text_align() -> TextAlignment { TextAlignment::Center }
fn default_text_x() -> i32 { 0 }
fn default_text_y() -> i32 { 7 }
fn default_show_percent_symbol() -> bool { false }
fn default_polling_interval() -> u64 { 10 }
fn default_polling_interval_seconds() -> u64 { 0 }
fn default_display_charging() -> bool { true }
fn default_synapse_version() -> SynapseVersion { SynapseVersion::Auto }
fn default_show_device_overlay() -> bool { false }
fn default_active_theme() -> String { "Default".to_string() }

// ── Settings ───────────────────────────────────────────────────

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Settings {
    #[serde(default = "default_run_at_startup")]
    pub run_at_startup: bool,
    #[serde(default = "default_show_percentage")]
    pub show_percentage: bool,
    #[serde(default = "default_text_size")]
    pub percentage_text_size: u32,
    #[serde(default = "default_text_color")]
    pub percentage_text_color: String,
    #[serde(default = "default_text_font")]
    pub percentage_text_font: String,
    #[serde(default)]
    pub logfont_data: LogFontData,
    #[serde(default = "default_text_align")]
    pub percentage_text_align: TextAlignment,
    #[serde(default = "default_text_x")]
    pub percentage_text_x: i32,
    #[serde(default = "default_text_y")]
    pub percentage_text_y: i32,
    #[serde(default = "default_show_percent_symbol")]
    pub show_percent_symbol: bool,
    #[serde(default = "default_polling_interval")]
    pub polling_interval_minutes: u64,
    #[serde(default = "default_polling_interval_seconds")]
    pub polling_interval_seconds: u64,
    #[serde(default = "default_display_charging")]
    pub display_charging_state: bool,
    #[serde(default)]
    pub shown_device_handle: String,
    /// Per-device configurations (visibility, display order). Auto-discovered.
    #[serde(default)]
    pub device_configs: Vec<DeviceConfig>,
    #[serde(default)]
    pub hidden_device_handles: Vec<String>,
    #[serde(default = "default_synapse_version")]
    pub synapse_version: SynapseVersion,
    #[serde(default)]
    pub custom_assets_folder: Option<String>,
    #[serde(default)]
    pub icon_theme: IconTheme,
    #[serde(default = "default_show_device_overlay")]
    pub show_device_type_overlay: bool,
    /// Root folder that contains named theme subfolders. None = use folder next to exe.
    #[serde(default)]
    pub themes_folder: Option<String>,
    /// Name of the active custom theme ("Default" = built-in embedded assets).
    #[serde(default = "default_active_theme")]
    pub active_theme: String,
}

impl Default for Settings {
    fn default() -> Self {
        Self {
            run_at_startup: default_run_at_startup(),
            show_percentage: default_show_percentage(),
            percentage_text_size: default_text_size(),
            percentage_text_color: default_text_color(),
            percentage_text_font: default_text_font(),
            logfont_data: LogFontData::default(),
            percentage_text_align: default_text_align(),
            percentage_text_x: default_text_x(),
            percentage_text_y: default_text_y(),
            show_percent_symbol: default_show_percent_symbol(),
            polling_interval_minutes: default_polling_interval(),
            polling_interval_seconds: default_polling_interval_seconds(),
            display_charging_state: default_display_charging(),
            shown_device_handle: String::new(),
            device_configs: Vec::new(),
            hidden_device_handles: Vec::new(),
            synapse_version: default_synapse_version(),
            custom_assets_folder: None,
            icon_theme: IconTheme::default(),
            show_device_type_overlay: default_show_device_overlay(),
            themes_folder: None,
            active_theme: default_active_theme(),
        }
    }
}

impl Settings {
    pub fn to_icon_settings(&self) -> IconSettings {
        IconSettings {
            show_percentage: self.show_percentage,
            text_size: self.percentage_text_size,
            text_color: self.percentage_text_color.clone(),
            font_name: self.percentage_text_font.clone(),
            text_align: self.percentage_text_align.as_str().to_string(),
            text_x: self.percentage_text_x,
            text_y: self.percentage_text_y,
            show_percent_symbol: self.show_percent_symbol,
            show_device_type_overlay: self.show_device_type_overlay,
        }
    }

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

    /// Load settings from an explicit path. Used in tests and for portable configs.
    pub fn load_from_path(path: impl AsRef<std::path::Path>) -> Result<Self> {
        let content = fs::read_to_string(path.as_ref())?;
        Ok(serde_json::from_str(&content)?)
    }

    /// Save settings to an explicit path. Used in tests and for portable configs.
    pub fn save_to_path(&self, path: impl AsRef<std::path::Path>) -> Result<()> {
        let path = path.as_ref();
        if let Some(parent) = path.parent() {
            fs::create_dir_all(parent)?;
        }
        let content = serde_json::to_string_pretty(self)?;
        fs::write(path, content)?;
        Ok(())
    }

    fn settings_path() -> PathBuf {
        let data_dir = dirs::data_local_dir()
            .unwrap_or_else(|| PathBuf::from("."));
        data_dir.join("razer-taskbar").join("settings.json")
    }

    /// Calculate total polling interval in seconds (minutes * 60 + seconds).
    /// Returns at least 1 second.
    pub fn polling_interval_total_seconds(&self) -> u64 {
        let total = self.polling_interval_minutes * 60 + self.polling_interval_seconds;
        total.max(1)
    }

    // ── Device config helpers ──────────────────────────────────

    /// Look up a device config by unique id.
    #[allow(dead_code)]
    pub fn get_device_config(&self, unique_id: &str) -> Option<&DeviceConfig> {
        self.device_configs.iter().find(|c| c.id == unique_id)
    }

    /// Returns true if a device should be visible in the tray.
    /// Devices not yet in device_configs are visible by default.
    #[allow(dead_code)]
    pub fn is_device_visible(&self, unique_id: &str) -> bool {
        self.get_device_config(unique_id)
            .map(|c| c.visible)
            .unwrap_or(true)
    }

    /// Sync device_configs with the currently discovered devices.
    /// Adds new devices, marks disconnected devices, updates names.
    /// Returns true if anything changed.
    pub fn sync_device_configs(&mut self, devices: &[(String, String)]) -> bool {
        let mut changed = false;
        let discovered_ids: Vec<&str> = devices.iter().map(|(id, _)| id.as_str()).collect();

        // Mark disconnected devices as not connected
        for cfg in &mut self.device_configs {
            let is_connected = discovered_ids.contains(&cfg.id.as_str());
            if cfg.connected != is_connected {
                cfg.connected = is_connected;
                changed = true;
            }
        }

        // Add or update discovered devices
        for (unique_id, name) in devices {
            if let Some(existing) = self.device_configs.iter_mut().find(|c| c.id == *unique_id) {
                if existing.name != *name {
                    existing.name = name.clone();
                    changed = true;
                }
            } else {
                self.device_configs.push(DeviceConfig {
                    id: unique_id.clone(),
                    name: name.clone(),
                    visible: true,
                    connected: true,
                });
                changed = true;
            }
        }
        changed
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    // ── polling_interval_total_seconds ─────────────────────────

    #[test]
    fn polling_interval_combines_minutes_and_seconds() {
        let mut s = Settings::default();
        s.polling_interval_minutes = 1;
        s.polling_interval_seconds = 30;
        assert_eq!(s.polling_interval_total_seconds(), 90);
    }

    #[test]
    fn polling_interval_enforces_minimum_of_one_second() {
        let mut s = Settings::default();
        s.polling_interval_minutes = 0;
        s.polling_interval_seconds = 0;
        assert_eq!(s.polling_interval_total_seconds(), 1);
    }

    #[test]
    fn polling_interval_minutes_only() {
        let mut s = Settings::default();
        s.polling_interval_minutes = 5;
        s.polling_interval_seconds = 0;
        assert_eq!(s.polling_interval_total_seconds(), 300);
    }

    #[test]
    fn polling_interval_seconds_only() {
        let mut s = Settings::default();
        s.polling_interval_minutes = 0;
        s.polling_interval_seconds = 45;
        assert_eq!(s.polling_interval_total_seconds(), 45);
    }

    // ── is_device_visible ─────────────────────────────────────

    #[test]
    fn is_device_visible_returns_true_for_unknown_device() {
        let s = Settings::default();
        assert!(s.is_device_visible("unknown-id"));
    }

    #[test]
    fn is_device_visible_returns_true_when_config_visible() {
        let mut s = Settings::default();
        s.device_configs.push(DeviceConfig { id: "dev1".into(), name: "Mouse".into(), visible: true, connected: true });
        assert!(s.is_device_visible("dev1"));
    }

    #[test]
    fn is_device_visible_returns_false_when_config_hidden() {
        let mut s = Settings::default();
        s.device_configs.push(DeviceConfig { id: "dev1".into(), name: "Mouse".into(), visible: false, connected: false });
        assert!(!s.is_device_visible("dev1"));
    }

    // ── sync_device_configs ───────────────────────────────────

    #[test]
    fn sync_adds_new_device_and_reports_changed() {
        let mut s = Settings::default();
        let changed = s.sync_device_configs(&[("id1".into(), "Mouse".into())]);
        assert!(changed);
        assert_eq!(s.device_configs.len(), 1);
        assert_eq!(s.device_configs[0].id, "id1");
        assert!(s.device_configs[0].connected);
    }

    #[test]
    fn sync_marks_absent_device_as_disconnected() {
        let mut s = Settings::default();
        s.device_configs.push(DeviceConfig { id: "id1".into(), name: "Mouse".into(), visible: true, connected: true });
        let changed = s.sync_device_configs(&[]);   // no devices present
        assert!(changed);
        assert!(!s.device_configs[0].connected);
    }

    #[test]
    fn sync_updates_device_name_and_reports_changed() {
        let mut s = Settings::default();
        s.device_configs.push(DeviceConfig { id: "id1".into(), name: "Old Name".into(), visible: true, connected: false });
        let changed = s.sync_device_configs(&[("id1".into(), "New Name".into())]);
        assert!(changed);
        assert_eq!(s.device_configs[0].name, "New Name");
    }

    #[test]
    fn sync_reports_no_change_when_device_already_connected_with_same_name() {
        let mut s = Settings::default();
        s.device_configs.push(DeviceConfig { id: "id1".into(), name: "Mouse".into(), visible: true, connected: true });
        let changed = s.sync_device_configs(&[("id1".into(), "Mouse".into())]);
        assert!(!changed);
    }

    #[test]
    fn sync_handles_multiple_devices() {
        let mut s = Settings::default();
        let changed = s.sync_device_configs(&[
            ("id1".into(), "Mouse".into()),
            ("id2".into(), "Keyboard".into()),
        ]);
        assert!(changed);
        assert_eq!(s.device_configs.len(), 2);
    }

    // ── TextAlignment::as_str ─────────────────────────────────

    #[test]
    fn text_alignment_as_str_returns_lowercase_name() {
        assert_eq!(TextAlignment::Left.as_str(),   "left");
        assert_eq!(TextAlignment::Center.as_str(), "center");
        assert_eq!(TextAlignment::Right.as_str(),  "right");
    }

    // ── IconTheme::as_str ─────────────────────────────────────

    #[test]
    fn icon_theme_as_str_returns_lowercase_name() {
        assert_eq!(IconTheme::Dark.as_str(),   "dark");
        assert_eq!(IconTheme::Light.as_str(),  "light");
        assert_eq!(IconTheme::System.as_str(), "system");
    }

    // ── Serde roundtrip ───────────────────────────────────────

    #[test]
    fn settings_serialise_then_deserialise_preserves_values() {
        let mut s = Settings::default();
        s.polling_interval_minutes = 3;
        s.show_percentage = true;
        s.percentage_text_color = "AABBCC".into();
        s.active_theme = "MyTheme".into();

        let json = serde_json::to_string(&s).unwrap();
        let loaded: Settings = serde_json::from_str(&json).unwrap();

        assert_eq!(loaded.polling_interval_minutes, 3);
        assert_eq!(loaded.show_percentage, true);
        assert_eq!(loaded.percentage_text_color, "AABBCC");
        assert_eq!(loaded.active_theme, "MyTheme");
    }

    #[test]
    fn settings_deserialise_applies_defaults_for_missing_fields() {
        // Minimal JSON — all fields absent should get defaults
        let json = "{}";
        let s: Settings = serde_json::from_str(json).unwrap();
        assert_eq!(s.polling_interval_minutes, 10);    // default_polling_interval()
        assert_eq!(s.active_theme, "Default");
        assert_eq!(s.show_percentage, false);
    }
}

