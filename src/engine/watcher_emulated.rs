use super::event_loop::Watcher;
use super::watcher_common::log_devices;
use crate::model::{DeviceCategory, DeviceMap, IconSettings, RazerDevice};
use crate::model::Settings;
use crate::ui::TrayManager;
use crate::util::log;
use anyhow::Result;

/// Emulated devices for testing multi-icon tray behavior without real hardware.
const EMULATED_DEVICES: &[EmulatedDeviceDef] = &[
    EmulatedDeviceDef {
        name: "Razer Viper Ultimate",
        handle: "EMU-VIPER-001",
        serial: "SN-VIPER-001",
        battery: 25,
        charging: false,
        category: DeviceCategory::Mouse,
    },
    EmulatedDeviceDef {
        name: "Razer BlackWidow V4",
        handle: "EMU-BLACKWIDOW-002",
        serial: "SN-BLACKWIDOW-002",
        battery: 80,
        charging: true,
        category: DeviceCategory::Keyboard,
    },
    EmulatedDeviceDef {
        name: "Razer Kraken V3",
        handle: "EMU-KRAKEN-003",
        serial: "SN-KRAKEN-003",
        battery: 52,
        charging: false,
        category: DeviceCategory::Headphones,
    },
    EmulatedDeviceDef {
        name: "Razer DeathAdder V3 Pro",
        handle: "EMU-DEATHADDER-004",
        serial: "SN-DEATHADDER-004",
        battery: 5,
        charging: false,
        category: DeviceCategory::Unknown,
    },
    EmulatedDeviceDef {
        name: "Razer BlackWidow V4",
        handle: "EMU-BLACKWIDOW-012",
        serial: "SN-BLACKWIDOW-012",
        battery: 80,
        charging: true,
        category: DeviceCategory::Keyboard,
    },
    EmulatedDeviceDef {
        name: "Razer Kraken V3",
        handle: "EMU-KRAKEN-013",
        serial: "SN-KRAKEN-013",
        battery: 52,
        charging: false,
        category: DeviceCategory::Headphones,
    },
    EmulatedDeviceDef {
        name: "Razer DeathAdder V3 Pro",
        handle: "EMU-DEATHADDER-014",
        serial: "SN-DEATHADDER-014",
        battery: 5,
        charging: false,
        category: DeviceCategory::Unknown,
    },
];

struct EmulatedDeviceDef {
    name: &'static str,
    handle: &'static str,
    serial: &'static str,
    battery: u8,
    charging: bool,
    category: DeviceCategory,
}

pub struct EmulationWatcher {
    tick: u32,
    devices: DeviceMap,
}

impl EmulationWatcher {
    pub fn new() -> Self {
        let devices = build_emulated_devices();
        Self { tick: 0, devices }
    }
}

fn build_emulated_devices() -> DeviceMap {
    let mut map = DeviceMap::new();
    for def in EMULATED_DEVICES {
        map.insert(
            def.serial.to_string(),
            RazerDevice {
                name: def.name.to_string(),
                handle: def.handle.to_string(),
                serial_number: Some(def.serial.to_string()),
                battery_percentage: def.battery,
                is_charging: def.charging,
                is_connected: true,
                is_selected: true,
                category: def.category,
            },
        );
    }
    map
}

impl Watcher for EmulationWatcher {
    fn parse_and_update(
        &mut self,
        tray: &mut TrayManager,
        icon_settings: &IconSettings,
        debug: bool,
    ) -> Result<()> {
        self.tick += 1;

        // Simulate battery drain / charge every cycle
        for def in EMULATED_DEVICES {
            if let Some(dev) = self.devices.get_mut(def.serial) {
                if dev.is_charging {
                    dev.battery_percentage = (dev.battery_percentage + 1).min(100);
                    if dev.battery_percentage >= 100 {
                        dev.is_charging = false;
                    }
                } else {
                    dev.battery_percentage = dev.battery_percentage.saturating_sub(1);
                    if dev.battery_percentage == 0 {
                        dev.is_charging = true;
                    }
                }
            }
        }

        log(
            &format!("[Emulation] tick={}, devices:", self.tick),
            debug,
        );
        log_devices(&self.devices, debug);
        tray.update_devices(self.devices.clone(), icon_settings)
    }

    fn parse_and_update_with_settings(
        &mut self, tray: &mut TrayManager,
        settings: &mut Settings, debug: bool,
    ) -> Result<()> {
        self.tick += 1;

        for def in EMULATED_DEVICES {
            if let Some(dev) = self.devices.get_mut(def.serial) {
                if dev.is_charging {
                    dev.battery_percentage = (dev.battery_percentage + 1).min(100);
                    if dev.battery_percentage >= 100 { dev.is_charging = false; }
                } else {
                    dev.battery_percentage = dev.battery_percentage.saturating_sub(1);
                    if dev.battery_percentage == 0 { dev.is_charging = true; }
                }
                // Apply visibility from device_configs
                let uid = dev.unique_id().to_string();
                dev.is_selected = settings.is_device_visible(&uid);
            }
        }

        log(&format!("[Emulation] tick={}, devices:", self.tick), debug);
        log_devices(&self.devices, debug);
        let icon_settings = settings.to_icon_settings();
        tray.update_devices(self.devices.clone(), &icon_settings)
    }

    fn last_devices(&self) -> DeviceMap {
        self.devices.clone()
    }

    fn persists_devices(&self) -> bool { true }

    fn check_log_rotation(&mut self, _debug: bool) {
        // No log files in emulation mode
    }
}

