use anyhow::Result;
use image::RgbaImage;
use lazy_static::lazy_static;
use std::collections::HashMap;
use std::path::PathBuf;
use std::sync::Mutex;

include!(concat!(env!("OUT_DIR"), "/embedded_assets.rs"));

#[derive(Debug, Clone)]
pub(crate) struct BatteryRange {
    pub min: u8,
    pub max: u8,
    pub filename: String,
}

lazy_static! {
    static ref EMBEDDED_ASSETS: HashMap<&'static str, &'static [u8]> = get_embedded_assets();
    /// The resolved folder containing theme assets (e.g. `themes_root/theme_name`).
    /// None = use built-in embedded assets.
    pub(crate) static ref CUSTOM_ASSETS_FOLDER: Mutex<Option<PathBuf>> = Mutex::new(None);
    pub(crate) static ref BATTERY_RANGES: Mutex<Option<Vec<BatteryRange>>> = Mutex::new(None);
    /// Root folder that holds theme subfolders (set by the user in settings).
    static ref THEMES_FOLDER: Mutex<Option<PathBuf>> = Mutex::new(None);
}

pub(crate) const DEFAULT_ICON_PROPERTIES: &str = include_str!("../../assets/icon.properties");
pub(crate) const ICON_PROPERTIES_FILENAME: &str = "icon.properties";

// ── Public API ─────────────────────────────────────────────────

/// Set the resolved theme asset folder directly (themes_root + "/" + theme_name).
/// Pass `None` to revert to built-in embedded assets.
pub fn set_custom_assets_folder(path: Option<PathBuf>) {
    *CUSTOM_ASSETS_FOLDER.lock().unwrap() = path;
    *BATTERY_RANGES.lock().unwrap() = None;
}

/// Configure the themes root folder and active theme together.
/// This derives the resolved CUSTOM_ASSETS_FOLDER from them.
pub fn set_themes_config(themes_root: Option<PathBuf>, active_theme: &str) {
    let resolved = if active_theme == "Default" {
        None
    } else {
        themes_root.as_ref().map(|root| root.join(active_theme))
    };
    set_custom_assets_folder(resolved);
    *THEMES_FOLDER.lock().unwrap() = themes_root;
}

/// Scan the given themes root folder for valid theme subfolders (those that
/// contain either a `dark/` or `light/` subdirectory or an `icon.properties`).
/// Always prepends "Default" to the result.
pub fn scan_themes(themes_root: &PathBuf) -> Vec<String> {
    let mut themes = vec!["Default".to_string()];
    if let Ok(entries) = std::fs::read_dir(themes_root) {
        let mut names: Vec<String> = entries
            .filter_map(|e| e.ok())
            .filter(|e| e.path().is_dir())
            .filter(|e| {
                let p = e.path();
                p.join("dark").is_dir()
                    || p.join("light").is_dir()
                    || p.join(ICON_PROPERTIES_FILENAME).exists()
            })
            .filter_map(|e| e.file_name().into_string().ok())
            .collect();
        names.sort();
        themes.extend(names);
    }
    themes
}

/// Return the default themes root: a `themes/` folder next to the current executable.
pub fn default_themes_root() -> Option<PathBuf> {
    std::env::current_exe().ok().and_then(|p| p.parent().map(|d| d.join("themes")))
}

pub(crate) fn get_custom_assets_folder() -> Option<PathBuf> {
    CUSTOM_ASSETS_FOLDER.lock().unwrap().clone()
}

// ── Battery ranges ─────────────────────────────────────────────

pub(crate) fn find_icon_for_percentage(pct: u8) -> Result<String> {
    let ranges = load_battery_ranges()?;
    for r in &ranges {
        if pct >= r.min && pct <= r.max {
            return Ok(r.filename.clone());
        }
    }
    ranges.first().map(|r| r.filename.clone())
        .ok_or_else(|| anyhow::anyhow!("No battery icon ranges defined"))
}

fn load_battery_ranges() -> Result<Vec<BatteryRange>> {
    let mut cached = BATTERY_RANGES.lock().unwrap();
    if let Some(ref ranges) = *cached {
        return Ok(ranges.clone());
    }
    let content = match get_custom_assets_folder() {
        Some(folder) => {
            let path = folder.join(ICON_PROPERTIES_FILENAME);
            if path.exists() { std::fs::read_to_string(&path)? }
            else { DEFAULT_ICON_PROPERTIES.to_string() }
        }
        None => DEFAULT_ICON_PROPERTIES.to_string(),
    };
    let ranges = parse_icon_properties(&content)?;
    *cached = Some(ranges.clone());
    Ok(ranges)
}

fn parse_icon_properties(content: &str) -> Result<Vec<BatteryRange>> {
    let mut ranges = Vec::new();
    for line in content.lines() {
        let line = line.trim();
        if line.is_empty() || line.starts_with('#') { continue; }
        if let Some((range_part, filename)) = line.split_once('=') {
            if let Some((min_s, max_s)) = range_part.split_once('-') {
                let min = min_s.trim().parse::<u8>();
                let max = max_s.trim().parse::<u8>();
                if let (Ok(min), Ok(max)) = (min, max) {
                    ranges.push(BatteryRange { min, max, filename: filename.trim().to_string() });
                }
            }
        }
    }
    ranges.sort_by_key(|r| r.min);
    Ok(ranges)
}

// ── Image loading ──────────────────────────────────────────────

pub(crate) fn load_image_from_assets(filename: &str) -> Result<RgbaImage> {
    let theme = super::theme::get_icon_theme();
    if let Some(folder) = get_custom_assets_folder() {
        if let Some(img) = try_load_custom(&folder, &theme, filename) {
            return Ok(img);
        }
    }
    load_embedded_asset(filename)
}

fn try_load_custom(folder: &PathBuf, theme: &str, name: &str) -> Option<RgbaImage> {
    let themed = folder.join(theme).join(name);
    if themed.exists() { return image::open(&themed).ok().map(|i| i.to_rgba8()); }
    let flat = folder.join(name);
    if flat.exists() { return image::open(&flat).ok().map(|i| i.to_rgba8()); }
    None
}

fn load_embedded_asset(filename: &str) -> Result<RgbaImage> {
    let theme = super::theme::get_icon_theme();
    let key = format!("{}/{}", theme, filename);
    if let Some(bytes) = EMBEDDED_ASSETS.get(key.as_str()) {
        return Ok(image::load_from_memory(bytes)?.to_rgba8());
    }
    let fb_theme = if theme == "dark" { "light" } else { "dark" };
    let fb_key = format!("{}/{}", fb_theme, filename);
    let bytes = EMBEDDED_ASSETS.get(fb_key.as_str())
        .ok_or_else(|| anyhow::anyhow!("Asset '{}' not found", filename))?;
    Ok(image::load_from_memory(bytes)?.to_rgba8())
}

