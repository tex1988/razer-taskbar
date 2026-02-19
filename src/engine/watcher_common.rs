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

