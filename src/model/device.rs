use serde::{Deserialize, Serialize};
use std::collections::HashMap;
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "UPPERCASE")]
pub enum DeviceCategory {
    Mouse,
    Keyboard,
    Headphones,
    Unknown,
}
impl Default for DeviceCategory {
    fn default() -> Self {
        DeviceCategory::Unknown
    }
}
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RazerDevice {
    pub name: String,
    pub handle: String,
    /// Unique serial number from Synapse logs (if available).
    #[serde(default)]
    pub serial_number: Option<String>,
    pub battery_percentage: u8,
    pub is_charging: bool,
    pub is_connected: bool,
    pub is_selected: bool,
    #[serde(default)]
    pub category: DeviceCategory,
}

impl RazerDevice {
    /// Returns the best unique identifier: serial_number if present, otherwise handle.
    pub fn unique_id(&self) -> &str {
        self.serial_number.as_deref().unwrap_or(&self.handle)
    }
}

pub type DeviceMap = HashMap<String, RazerDevice>;

/// Per-device configuration stored in settings.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DeviceConfig {
    /// The unique identifier (serial number or handle).
    pub id: String,
    /// Last known device name (for display in settings when device is offline).
    pub name: String,
    /// Whether to show a tray icon for this device.
    #[serde(default = "default_visible")]
    pub visible: bool,
    /// Whether the device is currently connected (not persisted, runtime only).
    #[serde(skip)]
    pub connected: bool,
}

/// PartialEq excludes the runtime-only `connected` field to avoid false-positive change detection.
impl PartialEq for DeviceConfig {
    fn eq(&self, other: &Self) -> bool {
        self.id == other.id && self.name == other.name && self.visible == other.visible
    }
}

fn default_visible() -> bool { true }

