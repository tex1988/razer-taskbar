use crate::model::DeviceMap;
use crate::util::log;

pub fn log_devices(devices: &DeviceMap, debug: bool) {
    if devices.is_empty() { return; }
    log(&format!("Found {} devices:", devices.len()), debug);
    for device in devices.values() {
        log(
            &format!(
                "  - {} ({}%{}){}",
                device.name,
                device.battery_percentage,
                if device.is_charging { " charging" } else { "" },
                if !device.is_connected { " [disconnected]" } else { "" }
            ),
            debug,
        );
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::model::{DeviceCategory, RazerDevice};

    fn make_device(name: &str, battery: u8, charging: bool, connected: bool) -> RazerDevice {
        RazerDevice {
            name: name.into(),
            handle: "handle".into(),
            serial_number: None,
            battery_percentage: battery,
            is_charging: charging,
            is_connected: connected,
            is_selected: true,
            category: DeviceCategory::Unknown,
        }
    }

    #[test]
    fn log_devices_does_not_panic_on_empty_map() {
        let devices = DeviceMap::new();
        // Should return early without panicking
        log_devices(&devices, false);
    }

    #[test]
    fn log_devices_does_not_panic_with_single_device() {
        let mut devices = DeviceMap::new();
        devices.insert("h1".into(), make_device("Mouse", 75, false, true));
        log_devices(&devices, false);
    }

    #[test]
    fn log_devices_handles_charging_device() {
        let mut devices = DeviceMap::new();
        devices.insert("h1".into(), make_device("Headset", 50, true, true));
        log_devices(&devices, false);
    }

    #[test]
    fn log_devices_handles_disconnected_device() {
        let mut devices = DeviceMap::new();
        devices.insert("h1".into(), make_device("Keyboard", 20, false, false));
        log_devices(&devices, false);
    }
}

