use anyhow::Result;
use image::{imageops, Rgba, RgbaImage};
use imageproc::drawing::draw_text_mut;
use lazy_static::lazy_static;
use ab_glyph::{FontRef, PxScale};
use std::collections::HashMap;
use std::path::PathBuf;
use std::sync::Mutex;
use std::sync::atomic::{AtomicBool, Ordering};
use tray_icon::Icon;
use crate::utils::parse_hex_color;
use crate::device::DeviceCategory;

// Include the auto-generated embedded assets from build.rs
include!(concat!(env!("OUT_DIR"), "/embedded_assets.rs"));

// Struct to represent a battery range mapping
#[derive(Debug, Clone)]
struct BatteryRange {
    min: u8,
    max: u8,
    filename: String,
}

lazy_static! {
    static ref EMBEDDED_ASSETS: HashMap<&'static str, &'static [u8]> = get_embedded_assets();
    static ref CUSTOM_ASSETS_FOLDER: Mutex<Option<PathBuf>> = Mutex::new(None);
    static ref BATTERY_RANGES: Mutex<Option<Vec<BatteryRange>>> = Mutex::new(None);
    static ref ICON_THEME: Mutex<String> = Mutex::new("dark".to_string());
}

/// Set when Windows broadcasts WM_SETTINGCHANGE for ImmersiveColorSet (theme changed).
static SYSTEM_THEME_CHANGED: AtomicBool = AtomicBool::new(false);

/// Create a hidden top-level window that listens for WM_SETTINGCHANGE broadcasts.
/// When the system app colour scheme changes the SYSTEM_THEME_CHANGED flag is set.
/// Must be called once from the main thread (same thread that runs the message pump).
/// Note: HWND_MESSAGE windows do NOT receive broadcast messages, so we use a real
/// top-level window that is kept invisible via WS_EX_TOOLWINDOW and no WS_VISIBLE.
pub fn create_theme_change_listener() {
    use windows::Win32::UI::WindowsAndMessaging::*;
    use windows::Win32::Foundation::{HWND, LPARAM, LRESULT, WPARAM};

    unsafe extern "system" fn wnd_proc(hwnd: HWND, msg: u32, wparam: WPARAM, lparam: LPARAM) -> LRESULT {
        // WM_SETTINGCHANGE = 0x001A
        if msg == 0x001A {
            // Read the parameter name if provided
            let param_name = if lparam.0 != 0 {
                let ptr = lparam.0 as *const u16;
                let mut len = 0usize;
                unsafe { while *ptr.add(len) != 0 { len += 1; } }
                String::from_utf16_lossy(unsafe { std::slice::from_raw_parts(ptr, len) })
            } else {
                String::new()
            };

            eprintln!("DEBUG: WM_SETTINGCHANGE param = '{}'", param_name);

            // "ImmersiveColorSet" is the canonical signal for app colour scheme changes.
            // Also trigger on empty/NULL lParam (generic settings change) by re-reading
            // the registry to check whether the actual theme value changed.
            let should_flag = param_name == "ImmersiveColorSet" || param_name.is_empty();

            if should_flag {
                eprintln!("DEBUG: Flagging system theme change");
                SYSTEM_THEME_CHANGED.store(true, Ordering::Relaxed);
            }
        }
        unsafe { DefWindowProcW(hwnd, msg, wparam, lparam) }
    }

    unsafe {
        use windows::core::w;

        let class_name = w!("RazerTaskbarThemeWatcher");
        let wc = WNDCLASSW {
            lpfnWndProc: Some(wnd_proc),
            lpszClassName: class_name,
            ..Default::default()
        };
        RegisterClassW(&wc);

        // Use a plain top-level window (None parent) — these receive broadcast messages.
        // WS_EX_TOOLWINDOW keeps it off the taskbar; no WS_VISIBLE keeps it invisible.
        let result = CreateWindowExW(
            WS_EX_TOOLWINDOW,
            class_name,
            w!(""),
            WINDOW_STYLE(0), // no WS_VISIBLE
            0, 0, 0, 0,
            None,   // NULL parent = top-level, receives broadcasts
            None, None, None,
        );
        match result {
            Ok(hwnd) => eprintln!("DEBUG: Theme watcher window created: {:?}", hwnd),
            Err(e)   => eprintln!("ERROR: Failed to create theme watcher window: {:?}", e),
        }
    }
}

/// Returns true (and clears the flag) if a system theme change was detected since last call.
pub fn consume_system_theme_changed() -> bool {
    SYSTEM_THEME_CHANGED.swap(false, Ordering::Relaxed)
}

// Embedded default icon.properties content
const DEFAULT_ICON_PROPERTIES: &str = include_str!("assets/icon.properties");

// File name constants
const UNKNOWN_BATTERY_ICON: &str = "no_device.png";
const CHARGING_OVERLAY_FILENAME: &str = "charging.png";
const ICON_PROPERTIES_FILENAME: &str = "icon.properties";

// Device type overlay filenames (keyed by DeviceCategory)
const DEVICE_OVERLAY_MOUSE: &str = "mouse.png";
const DEVICE_OVERLAY_KEYBOARD: &str = "keyboard.png";
const DEVICE_OVERLAY_HEADPHONES: &str = "headphones.png";
const DEVICE_OVERLAY_UNKNOWN: &str = "unknown.png";

// Target icon size for Windows tray icons (system tray typically uses 16x16 or 32x32)
// We use 32x32 as it looks better on high-DPI displays and Windows will scale down if needed
const TARGET_ICON_WIDTH: u32 = 32;
const TARGET_ICON_HEIGHT: u32 = 32;

/// Parse icon.properties file content into BatteryRange structs
fn parse_icon_properties(content: &str) -> Result<Vec<BatteryRange>> {
    let mut ranges = Vec::new();

    for line in content.lines() {
        let line = line.trim();

        // Skip empty lines and comments
        if line.is_empty() || line.starts_with('#') {
            continue;
        }

        // Parse format: "min-max=filename.png"
        if let Some((range_part, filename)) = line.split_once('=') {
            if let Some((min_str, max_str)) = range_part.split_once('-') {
                if let (Ok(min), Ok(max)) = (min_str.trim().parse::<u8>(), max_str.trim().parse::<u8>()) {
                    ranges.push(BatteryRange {
                        min,
                        max,
                        filename: filename.trim().to_string(),
                    });
                }
            }
        }
    }

    // Sort ranges by min value for consistent lookup
    ranges.sort_by_key(|r| r.min);

    Ok(ranges)
}

/// Load and cache battery ranges from icon.properties
fn load_battery_ranges() -> Result<Vec<BatteryRange>> {
    let mut cached_ranges = BATTERY_RANGES.lock().unwrap();

    if let Some(ref ranges) = *cached_ranges {
        return Ok(ranges.clone());
    }

    // Try to load from custom assets folder first
    let ranges = if let Some(custom_folder) = get_custom_assets_folder() {
        let properties_path = custom_folder.join(ICON_PROPERTIES_FILENAME);
        if properties_path.exists() {
            let content = std::fs::read_to_string(&properties_path)?;
            parse_icon_properties(&content)?
        } else {
            // Fallback to default if custom folder exists but no icon.properties
            parse_icon_properties(DEFAULT_ICON_PROPERTIES)?
        }
    } else {
        // Use embedded default icon.properties
        parse_icon_properties(DEFAULT_ICON_PROPERTIES)?
    };

    *cached_ranges = Some(ranges.clone());
    Ok(ranges)
}

/// Find the appropriate icon filename for a given battery percentage
fn find_icon_for_percentage(percentage: u8) -> Result<String> {
    let ranges = load_battery_ranges()?;

    for range in &ranges {
        if percentage >= range.min && percentage <= range.max {
            return Ok(range.filename.clone());
        }
    }

    // If no range matches, return the first icon as fallback
    ranges.first()
        .map(|r| r.filename.clone())
        .ok_or_else(|| anyhow::anyhow!("No battery icon ranges defined in icon.properties"))
}

/// Set the custom assets folder path and invalidate cached ranges
pub fn set_custom_assets_folder(path: Option<PathBuf>) {
    eprintln!("DEBUG: set_custom_assets_folder called with: {:?}", path);
    *CUSTOM_ASSETS_FOLDER.lock().unwrap() = path.clone();
    // Invalidate cached ranges when folder changes
    *BATTERY_RANGES.lock().unwrap() = None;
    eprintln!("DEBUG: Custom assets folder updated, cache invalidated. New value: {:?}", path);
}

/// Get the custom assets folder path
fn get_custom_assets_folder() -> Option<PathBuf> {
    CUSTOM_ASSETS_FOLDER.lock().unwrap().clone()
}

/// Detect the current Windows system theme (dark or light).
/// Reads HKCU\Software\Microsoft\Windows\CurrentVersion\Themes\Personalize → AppsUseLightTheme.
/// Returns "dark" when the system is in dark mode, "light" otherwise.
fn detect_system_theme() -> &'static str {
    use winreg::RegKey;
    use winreg::enums::*;

    let hkcu = RegKey::predef(HKEY_CURRENT_USER);
    let key = hkcu.open_subkey(r"Software\Microsoft\Windows\CurrentVersion\Themes\Personalize").ok();
    if let Some(key) = key {
        let value: Result<u32, _> = key.get_value("AppsUseLightTheme");
        if let Ok(v) = value {
            // 0 = dark mode, 1 = light mode
            return if v == 0 { "dark" } else { "light" };
        }
    }
    // Default to dark if the key cannot be read
    "dark"
}

/// Set the active icon theme ("dark", "light", or "system") and invalidate caches.
/// When "system" is passed the actual OS theme is resolved immediately.
/// Returns true if the resolved theme changed from the previously stored value.
pub fn set_icon_theme(theme: &str) -> bool {
    eprintln!("DEBUG: set_icon_theme called with: {}", theme);
    let resolved = if theme == "system" {
        detect_system_theme()
    } else {
        theme
    };
    eprintln!("DEBUG: resolved theme: {}", resolved);
    let mut lock = ICON_THEME.lock().unwrap();
    let changed = *lock != resolved;
    if changed {
        *lock = resolved.to_string();
        drop(lock);
        // Invalidate cached ranges so properties are re-read for new theme
        *BATTERY_RANGES.lock().unwrap() = None;
    }
    changed
}

/// Get the active icon theme name
fn get_icon_theme() -> String {
    ICON_THEME.lock().unwrap().clone()
}

/// Validate icon size and warn if incorrect
/// Returns the image as-is to preserve quality - no resizing
/// Icons should be exactly 32x32 pixels for best quality
fn validate_icon_size(img: RgbaImage, source: &str) -> RgbaImage {
    let (width, height) = img.dimensions();

    // If already the target size, return as-is
    if width == TARGET_ICON_WIDTH && height == TARGET_ICON_HEIGHT {
        return img;
    }

    // Warn if size is incorrect - don't resize to preserve quality
    eprintln!("WARNING: Icon '{}' is {}x{} but should be {}x{} for optimal quality. Please use 32x32 pixel assets.",
              source, width, height, TARGET_ICON_WIDTH, TARGET_ICON_HEIGHT);

    // Return as-is - Windows will handle scaling but it may look blurry
    img
}

/// Draw percentage text overlay on the icon
fn draw_percentage_overlay(
    img: &mut RgbaImage,
    percentage: u8,
    text_size: u32,
    text_color: &str,
    font_name: &str,
    text_align: &str,
    text_x: i32,
    text_y: i32,
    show_percent_symbol: bool,
) {
    eprintln!("DEBUG: draw_percentage_overlay called with percentage: {}, size: {}, color: {}, font: {}, align: {}, x: {}, y: {}, show_%: {}",
              percentage, text_size, text_color, font_name, text_align, text_x, text_y, show_percent_symbol);

    // Try to load the requested font from Windows registry
    // Windows stores font file mappings in the registry
    let font_data = load_font_from_registry(font_name)
        .or_else(|| {
            eprintln!("Warning: Could not load font '{}' from registry, trying fallback fonts", font_name);
            // Try fallback fonts
            load_font_from_registry("Arial")
                .or_else(|| load_font_from_registry("Segoe UI"))
        });

    let font_data = match font_data {
        Some(data) => {
            eprintln!("DEBUG: Font data loaded successfully, size: {} bytes", data.len());
            data
        }
        None => {
            eprintln!("ERROR: Failed to load any system font for percentage overlay");
            return;
        }
    };

    // ...existing code...

    let font = match FontRef::try_from_slice(&font_data) {
        Ok(font) => {
            eprintln!("DEBUG: Font parsed successfully");
            font
        }
        Err(e) => {
            eprintln!("ERROR: Failed to parse font data: {:?}", e);
            return;
        }
    };

    let text = if show_percent_symbol {
        format!("{}%", percentage)
    } else {
        format!("{}", percentage)
    };
    eprintln!("DEBUG: Text to draw: '{}'", text);

    // Use configured font size
    let scale = PxScale::from(text_size as f32);

    // Parse text color from hex string (RRGGBB)
    let (r, g, b) = parse_hex_color(text_color).unwrap_or((255, 255, 255)); // Default to white

    // Calculate actual text width using glyph metrics
    use ab_glyph::{Font, ScaleFont};
    let scaled_font = font.as_scaled(scale);
    let text_width: f32 = text.chars()
        .filter_map(|c| {
            let glyph_id = font.glyph_id(c);
            Some(scaled_font.h_advance(glyph_id))
        })
        .sum();

    eprintln!("DEBUG: Measured text width: {} (text: '{}', size: {})", text_width, text, text_size);

    // Calculate X position based on alignment
    // Alignment determines how the text should be positioned:
    // - "left": Text's left edge aligns with icon's left edge
    // - "center": Text's center aligns with icon's center
    // - "right": Text's right edge aligns with icon's right edge
    // Then apply offset from settings (text_x)
    eprintln!("DEBUG: Calculating X position - text_x offset: {}, text_align: '{}', text_width: {}", text_x, text_align, text_width);

    // Calculate base position based on alignment
    let base_x = match text_align {
        "left" => {
            // Left alignment: text starts at left edge of icon
            eprintln!("DEBUG: Alignment matched 'left' - text starts at left edge");
            0
        }
        "center" => {
            // Center alignment: text center aligns with icon center
            // X = (icon_width / 2) - (text_width / 2)
            let center_x = ((TARGET_ICON_WIDTH as f32 / 2.0) - (text_width / 2.0)).max(0.0) as i32;
            eprintln!("DEBUG: Alignment matched 'center' - centering text, base X: {}", center_x);
            center_x
        }
        "right" => {
            // Right alignment: text ends at right edge of icon
            // X = icon_width - text_width
            let right_x = (TARGET_ICON_WIDTH as f32 - text_width).max(0.0) as i32;
            eprintln!("DEBUG: Alignment matched 'right' - text ends at right edge, base X: {}", right_x);
            right_x
        }
        _ => {
            // Default to center
            eprintln!("DEBUG: Alignment matched default (center), text_align was: '{}'", text_align);
            ((TARGET_ICON_WIDTH as f32 / 2.0) - (text_width / 2.0)).max(0.0) as i32
        }
    };

    // Apply offset from settings (can be positive or negative)
    let x = base_x + text_x;
    eprintln!("DEBUG: Final X position: {} (base: {} + offset: {})", x, base_x, text_x);

    let y = text_y;

    eprintln!("DEBUG: Drawing text at position ({}, {}), text_width: {}, color: ({},{},{})",
              x, y, text_width, r, g, b);

    // Draw very subtle 1-pixel black outline for readability (4 directions only)
    // Using very low opacity (80) to keep it minimal
    draw_text_mut(img, Rgba([0u8, 0u8, 0u8, 80u8]), x - 1, y, scale, &font, &text);
    draw_text_mut(img, Rgba([0u8, 0u8, 0u8, 80u8]), x + 1, y, scale, &font, &text);
    draw_text_mut(img, Rgba([0u8, 0u8, 0u8, 80u8]), x, y - 1, scale, &font, &text);
    draw_text_mut(img, Rgba([0u8, 0u8, 0u8, 80u8]), x, y + 1, scale, &font, &text);

    // Draw the main text with configured color
    draw_text_mut(img, Rgba([r, g, b, 255u8]), x, y, scale, &font, &text);

    eprintln!("DEBUG: Text drawing completed with minimal outline");
}


/// Main public method to load battery icon - returns complete Icon ready for tray
pub fn load_icon(
    percentage: u8,
    is_charging: bool,
    show_percentage: bool,
    text_size: u32,
    text_color: &str,
    font_name: &str,
    text_align: &str,
    text_x: i32,
    text_y: i32,
    show_percent_symbol: bool,
    show_device_type_overlay: bool,
    device_category: DeviceCategory,
) -> Result<Icon> {
    eprintln!("DEBUG: load_icon called - percentage: {}, is_charging: {}, show_percentage: {}",
              percentage, is_charging, show_percentage);

    // Find and load the appropriate icon for this percentage
    let filename = find_icon_for_percentage(percentage)?;
    let mut base_img = load_image_from_assets(&filename)?;

    // Validate base image size (no resize - preserves quality)
    base_img = validate_icon_size(base_img, &filename);

    // Apply charging overlay if needed
    if is_charging {
        if let Ok(overlay_img) = load_image_from_assets(CHARGING_OVERLAY_FILENAME) {
            let overlay = validate_icon_size(overlay_img, CHARGING_OVERLAY_FILENAME);
            imageops::overlay(&mut base_img, &overlay, 0i64, 0i64);
        }
    }

    // Apply device type overlay if enabled
    if show_device_type_overlay {
        let overlay_filename = match device_category {
            DeviceCategory::Mouse => DEVICE_OVERLAY_MOUSE,
            DeviceCategory::Keyboard => DEVICE_OVERLAY_KEYBOARD,
            DeviceCategory::Headphones => DEVICE_OVERLAY_HEADPHONES,
            DeviceCategory::Unknown => DEVICE_OVERLAY_UNKNOWN,
        };
        eprintln!("DEBUG: Applying device type overlay: {}", overlay_filename);
        if let Ok(overlay_img) = load_image_from_assets(overlay_filename) {
            let overlay = validate_icon_size(overlay_img, overlay_filename);
            imageops::overlay(&mut base_img, &overlay, 0i64, 0i64);
        }
    }

    // Draw percentage text overlay if enabled
    if show_percentage {
        eprintln!("DEBUG: show_percentage is TRUE, calling draw_percentage_overlay");
        draw_percentage_overlay(&mut base_img, percentage, text_size, text_color, font_name, text_align, text_x, text_y, show_percent_symbol);
    } else {
        eprintln!("DEBUG: show_percentage is FALSE, skipping text overlay");
    }

    // Convert RgbaImage to Icon
    let (width, height) = base_img.dimensions();
    let rgba = base_img.into_raw();

    Ok(Icon::from_rgba(rgba, width, height)?)
}

/// Load unknown battery icon for when no devices are found
pub fn load_unknown_icon() -> Result<Icon> {
    let img = load_image_from_assets(UNKNOWN_BATTERY_ICON)?;

    // Validate size (no resize - preserves quality)
    let validated = validate_icon_size(img, UNKNOWN_BATTERY_ICON);

    let (width, height) = validated.dimensions();
    let rgba = validated.into_raw();

    Ok(Icon::from_rgba(rgba, width, height)?)
}

/// Load an image file from assets (custom folder or embedded), with theme support.
/// Resolution order:
///   1. custom_folder/<theme>/filename   (if custom folder is set and theme subfolder exists)
///   2. custom_folder/filename           (if custom folder is set but no theme subfolder)
///   3. embedded <theme>/filename        (embedded assets keyed as "dark/..." or "light/...")
fn load_image_from_assets(filename: &str) -> Result<RgbaImage> {
    let theme = get_icon_theme();

    // Try custom assets folder first
    if let Some(custom_folder) = get_custom_assets_folder() {
        eprintln!("DEBUG: Checking custom folder for {}: {:?}", filename, custom_folder);

        // Try custom_folder/<theme>/filename first
        let themed_path = custom_folder.join(&theme).join(filename);
        if themed_path.exists() {
            eprintln!("DEBUG: Loading CUSTOM themed asset: {:?}", themed_path);
            let img = image::open(&themed_path)
                .map_err(|e| anyhow::anyhow!("Failed to load custom asset {}: {}", filename, e))?
                .to_rgba8();
            return Ok(img);
        }

        // Fall back to custom_folder/filename (no theme subfolder)
        let flat_path = custom_folder.join(filename);
        if flat_path.exists() {
            eprintln!("DEBUG: Loading CUSTOM flat asset: {:?}", flat_path);
            let img = image::open(&flat_path)
                .map_err(|e| anyhow::anyhow!("Failed to load custom asset {}: {}", filename, e))?
                .to_rgba8();
            return Ok(img);
        }

        eprintln!("DEBUG: Custom asset not found (tried themed and flat), falling back to embedded");
    } else {
        eprintln!("DEBUG: No custom folder set, using embedded assets for {}", filename);
    }

    // Fallback to embedded resources (themed key first, then try opposite theme)
    eprintln!("DEBUG: Loading EMBEDDED asset: {}/{}", theme, filename);
    load_embedded_asset(filename)
}

/// Load embedded asset from compiled binary
/// Uses the auto-generated embedded assets from build.rs that parsed icon.properties
fn load_embedded_asset(filename: &str) -> Result<RgbaImage> {
    let theme = get_icon_theme();
    let themed_key = format!("{}/{}", theme, filename);

    // Try themed key first
    if let Some(bytes) = EMBEDDED_ASSETS.get(themed_key.as_str()) {
        let img = image::load_from_memory(bytes)
            .map_err(|e| anyhow::anyhow!("Failed to decode embedded image {}: {}", themed_key, e))?
            .to_rgba8();
        return Ok(img);
    }

    // Fallback: try opposite theme if the requested theme is missing
    let fallback_theme = if theme == "dark" { "light" } else { "dark" };
    let fallback_key = format!("{}/{}", fallback_theme, filename);
    eprintln!("WARNING: Embedded asset '{}' not found, trying fallback theme '{}'", themed_key, fallback_theme);

    let bytes = EMBEDDED_ASSETS
        .get(fallback_key.as_str())
        .ok_or_else(|| anyhow::anyhow!(
            "Asset '{}' not found in embedded resources for theme '{}' or fallback '{}'. Make sure it exists in src/assets/<theme>/ and is referenced in icon.properties.",
            filename, theme, fallback_theme
        ))?;

    let img = image::load_from_memory(bytes)
        .map_err(|e| anyhow::anyhow!("Failed to decode embedded image {}: {}", fallback_key, e))?
        .to_rgba8();

    Ok(img)
}

/// Load a font from Windows registry
/// Windows stores all installed fonts in the registry with their file paths
/// This allows us to support ALL installed fonts without manual mapping
fn load_font_from_registry(font_name: &str) -> Option<Vec<u8>> {
    use winreg::RegKey;
    use winreg::enums::*;

    eprintln!("DEBUG: Looking up font '{}' in Windows registry", font_name);

    // Open registry key for fonts
    // HKEY_LOCAL_MACHINE\SOFTWARE\Microsoft\Windows NT\CurrentVersion\Fonts
    let hklm = RegKey::predef(HKEY_LOCAL_MACHINE);
    let fonts_key = hklm
        .open_subkey(r"SOFTWARE\Microsoft\Windows NT\CurrentVersion\Fonts")
        .ok()?;

    // Iterate through all font entries
    for (name, _value) in fonts_key.enum_values().filter_map(|x| x.ok()) {
        // Check if this entry matches our font name
        // Registry names are like "Arial (TrueType)" or "Comic Sans MS (TrueType)"
        if name.contains(font_name) {
            // Value is the font filename - try to get as String
            let filename: String = match fonts_key.get_value(&name) {
                Ok(val) => val,
                Err(_) => continue,
            };

            // Build full path
            let full_path = if filename.contains(":\\") {
                // Absolute path
                filename
            } else {
                // Relative path - prepend Windows\Fonts
                format!("C:\\Windows\\Fonts\\{}", filename)
            };

            eprintln!("DEBUG: Registry found font file: {}", full_path);

            // Try to load the font file
            if let Ok(data) = std::fs::read(&full_path) {
                eprintln!("DEBUG: Successfully loaded font from registry path: {}", full_path);
                return Some(data);
            }
        }
    }

    eprintln!("DEBUG: Font '{}' not found in Windows registry", font_name);
    None
}

