use super::device::DeviceCategory;
use serde::Deserialize;

#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct LoggedDeviceInfo {
    pub serial_number: Option<String>,
    #[serde(default)]
    pub has_battery: bool,
    pub device_container_id: String,
    #[serde(default)]
    pub power_status: Option<PowerStatus>,
    pub name: NameMap,
    #[allow(dead_code)]
    pub product_name: NameMap,
    #[serde(default)]
    pub category: DeviceCategory,
}

#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct PowerStatus {
    pub charging_status: ChargingStatus,
    pub level: u8,
}

#[derive(Debug, Deserialize)]
#[serde(untagged)]
pub enum ChargingStatus {
    Text(String),
}

impl ChargingStatus {
    pub fn is_charging(&self) -> bool {
        match self {
            ChargingStatus::Text(s) => {
                let lower = s.to_lowercase();
                lower == "charging" || lower == "charge"
            }
        }
    }
}

#[derive(Debug, Deserialize)]
pub struct NameMap {
    pub en: String,
}
