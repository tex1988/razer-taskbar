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

#[cfg(test)]
mod tests {
    use super::*;

    fn make_device(handle: &str, serial: Option<&str>) -> RazerDevice {
        RazerDevice {
            name: "Test Device".into(),
            handle: handle.into(),
            serial_number: serial.map(|s| s.into()),
            battery_percentage: 75,
            is_charging: false,
            is_connected: true,
            is_selected: true,
            category: DeviceCategory::Mouse,
        }
    }

    // ── RazerDevice::unique_id ─────────────────────────────────

    #[test]
    fn unique_id_prefers_serial_number_when_present() {
        let d = make_device("handle-123", Some("SN-ABC"));
        assert_eq!(d.unique_id(), "SN-ABC");
    }

    #[test]
    fn unique_id_falls_back_to_handle_when_no_serial() {
        let d = make_device("handle-123", None);
        assert_eq!(d.unique_id(), "handle-123");
    }

    // ── DeviceConfig PartialEq ─────────────────────────────────

    #[test]
    fn device_config_eq_ignores_connected_field() {
        let a = DeviceConfig { id: "id1".into(), name: "Mouse".into(), visible: true,  connected: false };
        let b = DeviceConfig { id: "id1".into(), name: "Mouse".into(), visible: true,  connected: true  };
        assert_eq!(a, b);
    }

    #[test]
    fn device_config_ne_when_visible_differs() {
        let a = DeviceConfig { id: "id1".into(), name: "Mouse".into(), visible: true,  connected: false };
        let b = DeviceConfig { id: "id1".into(), name: "Mouse".into(), visible: false, connected: false };
        assert_ne!(a, b);
    }

    #[test]
    fn device_config_ne_when_name_differs() {
        let a = DeviceConfig { id: "id1".into(), name: "Mouse".into(),    visible: true, connected: false };
        let b = DeviceConfig { id: "id1".into(), name: "Keyboard".into(), visible: true, connected: false };
        assert_ne!(a, b);
    }

    // ── DeviceCategory default ─────────────────────────────────

    #[test]
    fn device_category_default_is_unknown() {
        assert_eq!(DeviceCategory::default(), DeviceCategory::Unknown);
    }
}

