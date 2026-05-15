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

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn is_charging_true_for_charging_lowercase() {
        assert!(ChargingStatus::Text("charging".into()).is_charging());
    }

    #[test]
    fn is_charging_true_for_charging_mixed_case() {
        assert!(ChargingStatus::Text("Charging".into()).is_charging());
        assert!(ChargingStatus::Text("CHARGING".into()).is_charging());
    }

    #[test]
    fn is_charging_true_for_charge_variant() {
        assert!(ChargingStatus::Text("charge".into()).is_charging());
        assert!(ChargingStatus::Text("Charge".into()).is_charging());
    }

    #[test]
    fn is_charging_false_for_discharging() {
        assert!(!ChargingStatus::Text("discharging".into()).is_charging());
        assert!(!ChargingStatus::Text("Discharging".into()).is_charging());
    }

    #[test]
    fn is_charging_false_for_empty_or_other_values() {
        assert!(!ChargingStatus::Text("".into()).is_charging());
        assert!(!ChargingStatus::Text("full".into()).is_charging());
        assert!(!ChargingStatus::Text("unknown".into()).is_charging());
    }
}

