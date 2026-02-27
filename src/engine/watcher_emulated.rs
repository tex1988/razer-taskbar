use super::event_loop::Watcher;
use super::watcher_common::log_devices;
use crate::model::{DeviceCategory, DeviceMap, IconSettings, RazerDevice};
use crate::ui::TrayManager;
use crate::util::log;
use anyhow::Result;

/// Emulated devices for testing multi-icon tray behavior without real hardware.
const EMULATED_DEVICES: &[EmulatedDeviceDef] = &[
    EmulatedDeviceDef {
        name: "Razer Viper Ultimate",
        handle: "EMU-VIPER-001",
        battery: 25,
        charging: false,
        category: DeviceCategory::Mouse,
    },
    EmulatedDeviceDef {
        name: "Razer BlackWidow V4",
        handle: "EMU-BLACKWIDOW-002",
        battery: 80,
        charging: true,
        category: DeviceCategory::Keyboard,
    },
    EmulatedDeviceDef {
        name: "Razer Kraken V3",
        handle: "EMU-KRAKEN-003",
        battery: 52,
        charging: false,
        category: DeviceCategory::Headphones,
    },
    EmulatedDeviceDef {
        name: "Razer DeathAdder V3 Pro",
        handle: "EMU-DEATHADDER-004",
        battery: 5,
        charging: false,
        category: DeviceCategory::Mouse,
    },
];

struct EmulatedDeviceDef {
    name: &'static str,
    handle: &'static str,
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
            def.handle.to_string(),
            RazerDevice {
                name: def.name.to_string(),
                handle: def.handle.to_string(),
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
            if let Some(dev) = self.devices.get_mut(def.handle) {
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

    fn check_log_rotation(&mut self, _debug: bool) {
        // No log files in emulation mode
    }
}

