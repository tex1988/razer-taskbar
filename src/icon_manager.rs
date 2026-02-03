use anyhow::Result;
use image::{imageops, RgbaImage};
use lazy_static::lazy_static;
use std::collections::HashMap;
use std::path::PathBuf;
use std::sync::Mutex;
use tray_icon::Icon;

lazy_static! {
    static ref ASSET_CACHE: HashMap<&'static str, &'static [u8]> = {
        let mut m = HashMap::new();

        // Regular battery icons
        m.insert("battery0.png", include_bytes!("assets/regular-icon/battery0.png") as &[u8]);
        m.insert("battery25.png", include_bytes!("assets/regular-icon/battery25.png") as &[u8]);
        m.insert("battery50.png", include_bytes!("assets/regular-icon/battery50.png") as &[u8]);
        m.insert("battery75.png", include_bytes!("assets/regular-icon/battery75.png") as &[u8]);
        m.insert("battery100.png", include_bytes!("assets/regular-icon/battery100.png") as &[u8]);

        // Charging overlays for each icon type
        m.insert("regular-icon/chrg_overlay.png", include_bytes!("assets/regular-icon/chrg_overlay.png") as &[u8]);
        m.insert("numeric-icon/chrg_overlay.png", include_bytes!("assets/numeric-icon/chrg_overlay.png") as &[u8]);

        // Unknown battery icon
        m.insert("battery_unknown.png", include_bytes!("assets/battery_unknown.png") as &[u8]);

        m
    };

    static ref CUSTOM_ASSETS_FOLDER: Mutex<Option<PathBuf>> = Mutex::new(None);
}

// Macro to generate all 101 numeric icons (0-100) at compile time
macro_rules! generate_icon_array {
    ($($num:expr),* $(,)?) => {
        [$(
            include_bytes!(concat!("assets/numeric-icon/battery", stringify!($num), ".png")) as &[u8]
        ),*]
    };
}

// Static array with all numeric icons (0-100) embedded at compile time
static NUMERIC_ICONS: &[&[u8]; 101] = &generate_icon_array![
    000, 001, 002, 003, 004, 005, 006, 007, 008, 009, 010, 011, 012, 013, 014, 015, 016, 017, 018,
    019, 020, 021, 022, 023, 024, 025, 026, 027, 028, 029, 030, 031, 032, 033, 034, 035, 036, 037,
    038, 039, 040, 041, 042, 043, 044, 045, 046, 047, 048, 049, 050, 051, 052, 053, 054, 055, 056,
    057, 058, 059, 060, 061, 062, 063, 064, 065, 066, 067, 068, 069, 070, 071, 072, 073, 074, 075,
    076, 077, 078, 079, 080, 081, 082, 083, 084, 085, 086, 087, 088, 089, 090, 091, 092, 093, 094,
    095, 096, 097, 098, 099, 100,
];

// File name constants
const UNKNOWN_BATTERY_ICON: &str = "battery_unknown.png";
const CHARGING_OVERLAY_FILENAME: &str = "chrg_overlay.png";
const BATTERY_ICON_PREFIX: &str = "battery";
const BATTERY_ICON_EXTENSION: &str = ".png";

// Folder name constants
const REGULAR_ICON_FOLDER: &str = "regular-icon";
const NUMERIC_ICON_FOLDER: &str = "numeric-icon";

/// Helper to build regular battery icon filename (e.g., "battery25.png")
fn build_regular_icon_filename(level: u8) -> String {
    format!("{}{}{}", BATTERY_ICON_PREFIX, level, BATTERY_ICON_EXTENSION)
}

/// Helper to build numeric battery icon filename (e.g., "battery042.png")
fn build_numeric_icon_filename(percentage: u8) -> String {
    format!("{}{:03}{}", BATTERY_ICON_PREFIX, percentage, BATTERY_ICON_EXTENSION)
}

/// Set the custom assets folder path
pub fn set_custom_assets_folder(path: Option<PathBuf>) {
    *CUSTOM_ASSETS_FOLDER.lock().unwrap() = path;
}

/// Get the custom assets folder path
fn get_custom_assets_folder() -> Option<PathBuf> {
    CUSTOM_ASSETS_FOLDER.lock().unwrap().clone()
}

/// Main public method to load battery icon - returns complete Icon ready for tray
/// All icon loading, overlay application, and conversion logic is encapsulated here
pub fn load_icon(percentage: u8, is_charging: bool, is_numeric: bool) -> Result<Icon> {
    // Load base image based on icon type
    let base_img = if is_numeric {
        // Use numeric/percentage icons
        load_numeric_icon(percentage)?
    } else {
        // Use regular battery level icons (0, 25, 50, 75, 100)
        load_regular_icon(percentage)?
    };

    // Apply charging overlay if needed
    let img = if is_charging {
        let icon_type = if is_numeric {
            NUMERIC_ICON_FOLDER
        } else {
            REGULAR_ICON_FOLDER
        };
        let overlay = load_overlay(icon_type)?;
        apply_overlay(base_img, &[overlay])
    } else {
        base_img
    };

    // Convert RgbaImage to Icon
    let (width, height) = img.dimensions();
    let rgba = img.into_raw();

    Ok(Icon::from_rgba(rgba, width, height)?)
}

/// Load unknown battery icon for when no devices are found
pub fn load_unknown_icon() -> Result<Icon> {
    let img = load_image(UNKNOWN_BATTERY_ICON)?;
    let (width, height) = img.dimensions();
    let rgba = img.into_raw();

    Ok(Icon::from_rgba(rgba, width, height)?)
}

/// Load regular battery icon (0, 25, 50, 75, 100) - base image only
fn load_regular_icon(percentage: u8) -> Result<RgbaImage> {
    // Map percentage to nearest level
    let level = match percentage {
        0..=12 => 0,
        13..=37 => 25,
        38..=62 => 50,
        63..=87 => 75,
        88..=100 => 100,
        _ => 100,
    };

    // Build the asset filename using helper function
    let filename = build_regular_icon_filename(level);

    // Load the base icon from custom assets or embedded resources
    load_image(&filename)
}

/// Load numeric/percentage icon - base image only
fn load_numeric_icon(percentage: u8) -> Result<RgbaImage> {
    let filename = build_numeric_icon_filename(percentage);
    if let Some(img) = try_load_custom_asset(&filename, &[NUMERIC_ICON_FOLDER]) {
        return Ok(img);
    }
    load_embedded_numeric_icon(percentage)
}

/// Load embedded numeric icon from compiled resources (without overlay)
fn load_embedded_numeric_icon(percentage: u8) -> Result<RgbaImage> {
    // Clamp percentage to 0-100
    let percentage = percentage.min(100);

    // Get the icon bytes by indexing into the array
    let bytes = NUMERIC_ICONS[percentage as usize];

    // Decode the PNG image
    let img = image::load_from_memory(bytes)
        .map_err(|e| anyhow::anyhow!("Failed to decode numeric icon for {}%: {}", percentage, e))?
        .to_rgba8();
    Ok(img)
}

/// Load image for regular battery icons - tries custom assets first, then embedded resources
fn load_image(filename: &str) -> Result<RgbaImage> {
    // Try custom assets folder first
    let subfolders = if filename.starts_with(BATTERY_ICON_PREFIX) && filename != UNKNOWN_BATTERY_ICON {
        &[REGULAR_ICON_FOLDER][..]
    } else {
        &[][..]
    };

    if let Some(img) = try_load_custom_asset(filename, subfolders) {
        return Ok(img);
    }

    // Fallback to embedded resources
    if let Some(bytes) = ASSET_CACHE.get(filename) {
        let img = image::load_from_memory(bytes)
            .map_err(|e| anyhow::anyhow!("Failed to decode embedded image {}: {}", filename, e))?
            .to_rgba8();
        Ok(img)
    } else {
        Err(anyhow::anyhow!(
            "Asset {} not found in embedded resources",
            filename
        ))
    }
}

/// Generic function to apply one or more overlay images on top of a base image
/// Overlays are composited at coordinate (0, 0) without resizing
/// Overlays are applied in the order they appear in the slice
fn apply_overlay(mut base_img: RgbaImage, overlays: &[RgbaImage]) -> RgbaImage {
    for overlay in overlays {
        // Composite the overlay at (0, 0) coordinate
        imageops::overlay(&mut base_img, overlay, 0i64, 0i64);
    }
    base_img
}

/// Load overlay image for a given icon type (tries custom assets first, then embedded)
/// icon_type should be "regular-icon" or "numeric-icon"
fn load_overlay(icon_type: &str) -> Result<RgbaImage> {
    // Try to load from custom assets folder first
    if let Some(img) = try_load_custom_asset(CHARGING_OVERLAY_FILENAME, &[icon_type]) {
        return Ok(img);
    }

    // Fallback to embedded overlay
    load_embedded_overlay(icon_type)
}

/// Helper function to load embedded charging overlay
fn load_embedded_overlay(icon_type: &str) -> Result<RgbaImage> {
    let overlay_path = format!("{}/{}", icon_type, CHARGING_OVERLAY_FILENAME);

    if let Some(bytes) = ASSET_CACHE.get(overlay_path.as_str()) {
        let img = image::load_from_memory(bytes)
            .map_err(|e| {
                anyhow::anyhow!("Failed to decode embedded overlay {}: {}", overlay_path, e)
            })?
            .to_rgba8();
        Ok(img)
    } else {
        Err(anyhow::anyhow!(
            "Overlay {} not found in embedded resources",
            overlay_path
        ))
    }
}

/// Helper function to try loading an image from custom assets folder
fn try_load_custom_asset(filename: &str, subfolders: &[&str]) -> Option<RgbaImage> {
    let custom_folder = get_custom_assets_folder()?;

    // Build paths to try based on subfolders
    let mut paths_to_try = Vec::new();
    for subfolder in subfolders {
        paths_to_try.push(custom_folder.join(subfolder).join(filename));
    }
    // Also try root folder
    paths_to_try.push(custom_folder.join(filename));

    for path in paths_to_try {
        if path.exists() {
            match image::open(&path) {
                Ok(img) => return Some(img.to_rgba8()),
                Err(e) => eprintln!("Failed to load custom asset {}: {}", path.display(), e),
            }
        }
    }

    None
}
