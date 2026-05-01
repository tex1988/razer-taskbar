mod assets;
mod text_overlay;
mod theme;

use anyhow::Result;
use image::{imageops, RgbaImage};
use tray_icon::Icon;
use crate::model::{DeviceCategory, IconSettings, TextOverlayConfig};

// Re-export public API
pub use assets::{set_themes_config, scan_themes, default_themes_root};
pub use theme::{create_theme_change_listener, consume_system_theme_changed, set_icon_theme};

const UNKNOWN_BATTERY_ICON: &str = "no_device.png";
const CHARGING_OVERLAY: &str = "charging.png";
const DEVICE_OVERLAY_MOUSE: &str = "mouse.png";
const DEVICE_OVERLAY_KEYBOARD: &str = "keyboard.png";
const DEVICE_OVERLAY_HEADPHONES: &str = "headphones.png";
const DEVICE_OVERLAY_UNKNOWN: &str = "unknown.png";

// ── LoadIconParams ─────────────────────────────────────────────

pub struct LoadIconParams {
    pub percentage: u8,
    pub is_charging: bool,
    pub icon_settings: IconSettings,
    pub device_category: DeviceCategory,
}

// ── Public API ─────────────────────────────────────────────────

pub fn load_icon(params: &LoadIconParams) -> Result<Icon> {
    let filename = assets::find_icon_for_percentage(params.percentage)?;
    let mut img = assets::load_image_from_assets(&filename)?;

    if params.is_charging {
        apply_overlay(&mut img, CHARGING_OVERLAY);
    }
    if params.icon_settings.show_device_type_overlay {
        apply_device_overlay(&mut img, params.device_category);
    }
    if params.icon_settings.show_percentage {
        let config = TextOverlayConfig::from(&params.icon_settings);
        text_overlay::draw_percentage_overlay(&mut img, params.percentage, &config);
    }
    image_to_icon(img)
}

pub fn load_unknown_icon() -> Result<Icon> {
    let img = assets::load_image_from_assets(UNKNOWN_BATTERY_ICON)?;
    image_to_icon(img)
}

// ── Helpers ────────────────────────────────────────────────────

fn image_to_icon(img: RgbaImage) -> Result<Icon> {
    let (w, h) = img.dimensions();
    Ok(Icon::from_rgba(img.into_raw(), w, h)?)
}

fn apply_overlay(img: &mut RgbaImage, filename: &str) {
    if let Ok(overlay) = assets::load_image_from_assets(filename) {
        imageops::overlay(img, &overlay, 0, 0);
    }
}

fn apply_device_overlay(img: &mut RgbaImage, category: DeviceCategory) {
    let name = match category {
        DeviceCategory::Mouse => DEVICE_OVERLAY_MOUSE,
        DeviceCategory::Keyboard => DEVICE_OVERLAY_KEYBOARD,
        DeviceCategory::Headphones => DEVICE_OVERLAY_HEADPHONES,
        DeviceCategory::Unknown => DEVICE_OVERLAY_UNKNOWN,
    };
    apply_overlay(img, name);
}

