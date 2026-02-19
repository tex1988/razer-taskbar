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
    pub battery_percentage: u8,
    pub is_charging: bool,
    pub is_connected: bool,
    pub is_selected: bool,
    #[serde(default)]
    pub category: DeviceCategory,
}

pub type DeviceMap = HashMap<String, RazerDevice>;
