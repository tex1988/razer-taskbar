use serde::{Deserialize, Serialize};
use std::fs;
use std::path::PathBuf;
use anyhow::Result;
use super::icon_settings::IconSettings;

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
fn default_display_charging() -> bool { true }
fn default_synapse_version() -> SynapseVersion { SynapseVersion::Auto }
fn default_show_device_overlay() -> bool { false }

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
    #[serde(default = "default_display_charging")]
    pub display_charging_state: bool,
    #[serde(default)]
    pub shown_device_handle: String,
    #[serde(default = "default_synapse_version")]
    pub synapse_version: SynapseVersion,
    #[serde(default)]
    pub custom_assets_folder: Option<String>,
    #[serde(default)]
    pub icon_theme: IconTheme,
    #[serde(default = "default_show_device_overlay")]
    pub show_device_type_overlay: bool,
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
            display_charging_state: default_display_charging(),
            shown_device_handle: String::new(),
            synapse_version: default_synapse_version(),
            custom_assets_folder: None,
            icon_theme: IconTheme::default(),
            show_device_type_overlay: default_show_device_overlay(),
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

    fn settings_path() -> PathBuf {
        let data_dir = dirs::data_local_dir()
            .unwrap_or_else(|| PathBuf::from("."));
        data_dir.join("razer-taskbar").join("settings.json")
    }
}

