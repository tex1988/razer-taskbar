use serde::{Deserialize, Serialize};
use std::collections::HashMap;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RazerDevice {
    pub name: String,
    pub handle: String,
    pub battery_percentage: u8,
    pub is_charging: bool,
    pub is_connected: bool,
    pub is_selected: bool,
}

pub type DeviceMap = HashMap<String, RazerDevice>;
