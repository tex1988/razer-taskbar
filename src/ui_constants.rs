/// UI Constants for Razer Taskbar
/// Contains all colors, sizes, fonts, paddings, and other visual properties

use image::Rgba;

// ============================================================================
// TRAY ICON SETTINGS
// ============================================================================

/// Size of the tray icon (width and height in pixels)
#[allow(dead_code)]
pub const TRAY_ICON_SIZE: u32 = 32;

/// Battery level thresholds (percentage)
#[allow(dead_code)]
pub const BATTERY_LEVEL_STEP: f32 = 20.0;

// ============================================================================
// BATTERY ICON COLORS
// ============================================================================

/// Color for charging state (green) - RGBA format
#[allow(dead_code)]
pub const COLOR_CHARGING: Rgba<u8> = Rgba([50, 200, 50, 255]);

/// Color for critical battery level (0-25%) - Red
#[allow(dead_code)]
pub const COLOR_BATTERY_CRITICAL: Rgba<u8> = Rgba([200, 50, 50, 255]);

/// Color for low battery level (26-50%) - Orange
#[allow(dead_code)]
pub const COLOR_BATTERY_LOW: Rgba<u8> = Rgba([200, 150, 50, 255]);

/// Color for medium battery level (51-75%) - Yellow
#[allow(dead_code)]
pub const COLOR_BATTERY_MEDIUM: Rgba<u8> = Rgba([200, 200, 50, 255]);

/// Color for high battery level (76-100%) - Green
#[allow(dead_code)]
pub const COLOR_BATTERY_HIGH: Rgba<u8> = Rgba([50, 200, 50, 255]);

/// Background color for empty battery area - Dark gray
#[allow(dead_code)]
pub const COLOR_BATTERY_BACKGROUND: Rgba<u8> = Rgba([50, 50, 50, 255]);

// ============================================================================
// MENU SETTINGS
// ============================================================================

/// Menu text - "Quit"
pub const MENU_TEXT_QUIT: &str = "Quit";

/// Menu text - "Settings" (for future use)
pub const MENU_TEXT_SETTINGS: &str = "Settings";

// ============================================================================
// TOOLTIP SETTINGS
// ============================================================================

/// Default tooltip when no devices are found
pub const TOOLTIP_NO_DEVICES: &str = "No devices found";

/// Tooltip prefix for device name
pub const TOOLTIP_DEFAULT_PREFIX: &str = "Razer Taskbar";

/// Text suffix for charging status
pub const TOOLTIP_CHARGING_SUFFIX: &str = " (charging)";

// ============================================================================
// MENU APPEARANCE (for future dark theme implementation)
// ============================================================================

/// Menu background color (RGB) - Dark gray for dark theme
#[allow(dead_code)]
pub const MENU_BACKGROUND_COLOR: u32 = 0x001E1E1E; // RGB(30, 30, 30)

/// Menu text color (RGB) - White
#[allow(dead_code)]
pub const MENU_TEXT_COLOR: u32 = 0x00FFFFFF; // RGB(255, 255, 255)

/// Menu separator color (RGB) - Medium gray
#[allow(dead_code)]
pub const MENU_SEPARATOR_COLOR: u32 = 0x00404040; // RGB(64, 64, 64)

/// Menu item height in pixels
#[allow(dead_code)]
pub const MENU_ITEM_HEIGHT: i32 = 30;

/// Menu width in pixels
#[allow(dead_code)]
pub const MENU_WIDTH: i32 = 200;

/// Menu corner radius for rounded corners (pixels)
#[allow(dead_code)]
pub const MENU_CORNER_RADIUS: i32 = 12;

/// Menu padding (left/right) - Smaller padding for compact menu
#[allow(dead_code)]
pub const MENU_PADDING_HORIZONTAL: i32 = 8;

/// Menu padding (top/bottom)
#[allow(dead_code)]
pub const MENU_PADDING_VERTICAL: i32 = 4;

/// Menu item text offset from top
#[allow(dead_code)]
pub const MENU_TEXT_OFFSET_Y: i32 = 20;

/// Menu separator offset from top
#[allow(dead_code)]
pub const MENU_SEPARATOR_OFFSET_Y: i32 = 48;

/// Menu separator thickness
#[allow(dead_code)]
pub const MENU_SEPARATOR_THICKNESS: i32 = 1;

/// Menu opacity (0-255, where 255 is fully opaque)
#[allow(dead_code)]
pub const MENU_OPACITY: u8 = 250;

// ============================================================================
// TEXT ALIGNMENT & POSITIONING
// ============================================================================

/// Text alignment options for menu items
#[allow(dead_code)]
#[derive(Debug, Clone, Copy, PartialEq)]
pub enum TextAlignment {
    Left,
    #[allow(dead_code)]
    Center,
    #[allow(dead_code)]
    Right,
}

/// Default text alignment for menu items (Left-aligned)
#[allow(dead_code)]
pub const MENU_TEXT_ALIGNMENT: TextAlignment = TextAlignment::Left;

/// Text alignment for menu title/header
#[allow(dead_code)]
pub const MENU_TITLE_ALIGNMENT: TextAlignment = TextAlignment::Left;

// ============================================================================
// TIMING & BEHAVIOR
// ============================================================================

/// Menu auto-close timeout in seconds
#[allow(dead_code)]
pub const MENU_AUTO_CLOSE_TIMEOUT_SECS: u64 = 10;

/// Polling interval for message pump (milliseconds)
#[allow(dead_code)]
pub const MESSAGE_PUMP_INTERVAL_MS: u64 = 100;

// ============================================================================
// BATTERY LEVEL RANGES
// ============================================================================

/// Battery level considered critical (%)
#[allow(dead_code)]
pub const BATTERY_CRITICAL_THRESHOLD: u8 = 25;

/// Battery level considered low (%)
#[allow(dead_code)]
pub const BATTERY_LOW_THRESHOLD: u8 = 50;

/// Battery level considered medium (%)
#[allow(dead_code)]
pub const BATTERY_MEDIUM_THRESHOLD: u8 = 75;
