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
    pub(crate) static ref CUSTOM_ASSETS_FOLDER: Mutex<Option<PathBuf>> = Mutex::new(None);
    pub(crate) static ref BATTERY_RANGES: Mutex<Option<Vec<BatteryRange>>> = Mutex::new(None);
}

pub(crate) const DEFAULT_ICON_PROPERTIES: &str = include_str!("../../assets/icon.properties");
pub(crate) const ICON_PROPERTIES_FILENAME: &str = "icon.properties";

// ── Public API ─────────────────────────────────────────────────

pub fn set_custom_assets_folder(path: Option<PathBuf>) {
    *CUSTOM_ASSETS_FOLDER.lock().unwrap() = path;
    *BATTERY_RANGES.lock().unwrap() = None;
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

