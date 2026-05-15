use crate::model::{DeviceCategory, DeviceConfig, DeviceMap, RazerDevice};
use crate::model::{IconSettings, Settings};
use super::event_loop::Watcher;
use super::watcher_common::log_devices;
use crate::ui::TrayManager;
use anyhow::Result;
use lazy_static::lazy_static;
use regex::Regex;
use std::collections::HashMap;
use std::env;
use std::fs;
use std::path::PathBuf;

lazy_static! {
    // Regex patterns for Synapse V3 log parsing
    static ref BATTERY_STATE_REGEX: Regex = Regex::new(
        r"(?m)^(?P<dateTime>.+?) INFO.+?_OnBatteryLevelChanged[\s\S]*?Name: (?P<name>.*)[\s\S]*?Handle: (?P<handle>\d+)[\s\S]*?level (?P<level>\d+) state (?P<isCharging>\d+)"
    ).unwrap();

    static ref DEVICE_LOADED_REGEX: Regex = Regex::new(
        r"(?m)^(?P<dateTime>.+?) INFO.+?_OnDeviceLoaded[\s\S]*?Name: (?P<name>.*)[\s\S]*?Handle: (?P<handle>\d+)"
    ).unwrap();

    static ref DEVICE_REMOVED_REGEX: Regex = Regex::new(
        r"(?m)^(?P<dateTime>.+?) INFO.+?_OnDeviceRemoved[\s\S]*?Name: (?P<name>.*)[\s\S]*?Handle: (?P<handle>\d+)"
    ).unwrap();
}

pub struct SynapseV3Watcher {
    log_path: PathBuf,
    cached_devices: DeviceMap,
}

impl SynapseV3Watcher {
    pub fn new() -> Option<Self> {
        let local_appdata = env::var("LOCALAPPDATA").ok()?;
        let log_path = PathBuf::from(local_appdata)
            .join("Razer")
            .join("Synapse3")
            .join("Log")
            .join("Razer Synapse 3.log");

        if log_path.exists() {
            Some(Self { log_path, cached_devices: DeviceMap::new() })
        } else {
            None
        }
    }

    pub fn parse_devices(&self, configs: &[DeviceConfig]) -> Result<DeviceMap> {
        let log_content = fs::read_to_string(&self.log_path)?;
        parse_log_content(&log_content, configs)
    }
}

/// Parse devices from raw Synapse V3 log content.
/// Extracted from file I/O so it can be tested with any string input.
pub fn parse_log_content(content: &str, configs: &[DeviceConfig]) -> Result<DeviceMap> {
    let mut devices = DeviceMap::new();

    let battery_matches = get_last_match_by_handle(&BATTERY_STATE_REGEX, content);
    for (handle, caps) in battery_matches {
        let name = caps.name("name").unwrap().as_str().trim().to_string();
        let level = caps.name("level").unwrap().as_str().parse::<u8>().unwrap_or(0);
        let is_charging = caps.name("isCharging").unwrap().as_str() != "0";

        let is_visible = configs.iter()
            .find(|c| c.id == handle)
            .map(|c| c.visible)
            .unwrap_or(true);

        devices.insert(
            handle.clone(),
            RazerDevice {
                name,
                handle: handle.clone(),
                serial_number: None,
                battery_percentage: level,
                is_charging,
                is_connected: false,
                is_selected: is_visible,
                category: DeviceCategory::Unknown,
            },
        );
    }

    let loaded_matches = get_last_match_by_handle_with_index(&DEVICE_LOADED_REGEX, content);
    let removed_matches = get_last_match_by_handle_with_index(&DEVICE_REMOVED_REGEX, content);

    for handle in loaded_matches.keys().chain(removed_matches.keys()) {
        if let Some(device) = devices.get_mut(handle) {
            let loaded_idx = loaded_matches.get(handle).map(|(idx, _)| *idx).unwrap_or(0);
            let removed_idx = removed_matches.get(handle).map(|(idx, _)| *idx).unwrap_or(0);
            device.is_connected = loaded_idx > removed_idx;
        }
    }

    Ok(devices)
}

#[cfg(test)]
mod tests {
    use super::*;

    // Minimal log fragments matching the V3 regex patterns.
    // Each multiline block must start at column 0 for the (?m) anchor.
    const BATTERY_ONLY: &str = "\
2024-01-15 10:30:00 INFO [App] _OnBatteryLevelChanged
  Name: Razer Basilisk V3
  Handle: 12345
  Battery: level 75 state 0";

    const CHARGING_LOG: &str = "\
2024-01-15 10:30:00 INFO [App] _OnBatteryLevelChanged
  Name: Razer Viper V2
  Handle: 99999
  Battery: level 42 state 1";

    // Two loaded events for the same handle (idx 0 and 1 in the loaded iterator)
    // → loaded_idx=1 > removed_idx=0 (default) → connected
    const CONNECTED_LOG: &str = "\
2024-01-15 09:00:00 INFO [App] _OnDeviceLoaded
  Name: Razer Basilisk V3
  Handle: 12345
2024-01-15 09:30:00 INFO [App] _OnDeviceLoaded
  Name: Razer Basilisk V3
  Handle: 12345
2024-01-15 10:30:00 INFO [App] _OnBatteryLevelChanged
  Name: Razer Basilisk V3
  Handle: 12345
  Battery: level 80 state 0";

    #[test]
    fn returns_empty_map_for_empty_log() {
        let devices = parse_log_content("", &[]).unwrap();
        assert!(devices.is_empty());
    }

    #[test]
    fn parses_battery_level_and_name() {
        let devices = parse_log_content(BATTERY_ONLY, &[]).unwrap();
        let dev = devices.get("12345").expect("device 12345 not found");
        assert_eq!(dev.battery_percentage, 75);
        assert_eq!(dev.name, "Razer Basilisk V3");
        assert_eq!(dev.handle, "12345");
    }

    #[test]
    fn device_not_charging_when_state_is_zero() {
        let devices = parse_log_content(BATTERY_ONLY, &[]).unwrap();
        assert!(!devices["12345"].is_charging);
    }

    #[test]
    fn device_is_charging_when_state_is_nonzero() {
        let devices = parse_log_content(CHARGING_LOG, &[]).unwrap();
        assert!(devices["99999"].is_charging);
    }

    #[test]
    fn device_not_connected_without_loaded_events() {
        let devices = parse_log_content(BATTERY_ONLY, &[]).unwrap();
        assert!(!devices["12345"].is_connected);
    }

    #[test]
    fn device_connected_when_loaded_idx_exceeds_removed_idx() {
        let devices = parse_log_content(CONNECTED_LOG, &[]).unwrap();
        assert!(devices["12345"].is_connected);
    }

    #[test]
    fn only_last_battery_event_per_handle_is_kept() {
        // Two battery events for the same handle — second value wins
        let log = "\
2024-01-15 10:00:00 INFO [App] _OnBatteryLevelChanged
  Name: Razer Basilisk V3
  Handle: 12345
  Battery: level 30 state 0
2024-01-15 11:00:00 INFO [App] _OnBatteryLevelChanged
  Name: Razer Basilisk V3
  Handle: 12345
  Battery: level 90 state 0";
        let devices = parse_log_content(log, &[]).unwrap();
        assert_eq!(devices["12345"].battery_percentage, 90);
    }

    #[test]
    fn parses_multiple_different_devices() {
        let log = "\
2024-01-15 10:00:00 INFO [App] _OnBatteryLevelChanged
  Name: Mouse
  Handle: 11111
  Battery: level 80 state 0
2024-01-15 10:01:00 INFO [App] _OnBatteryLevelChanged
  Name: Keyboard
  Handle: 22222
  Battery: level 60 state 0";
        let devices = parse_log_content(log, &[]).unwrap();
        assert_eq!(devices.len(), 2);
        assert_eq!(devices["11111"].battery_percentage, 80);
        assert_eq!(devices["22222"].battery_percentage, 60);
    }

    #[test]
    fn device_hidden_when_config_says_not_visible() {
        use crate::model::DeviceConfig;
        let configs = vec![DeviceConfig { id: "12345".into(), name: "Mouse".into(), visible: false, connected: false }];
        let devices = parse_log_content(BATTERY_ONLY, &configs).unwrap();
        assert!(!devices["12345"].is_selected);
    }

    #[test]
    fn device_visible_when_no_config_entry_exists() {
        let devices = parse_log_content(BATTERY_ONLY, &[]).unwrap();
        assert!(devices["12345"].is_selected);
    }
}

fn get_last_match_by_handle<'a>(
    regex: &Regex,
    text: &'a str,
) -> HashMap<String, regex::Captures<'a>> {
    let mut map = HashMap::new();
    for caps in regex.captures_iter(text) {
        if let Some(handle) = caps.name("handle") {
            map.insert(handle.as_str().to_string(), caps);
        }
    }
    map
}

fn get_last_match_by_handle_with_index<'a>(
    regex: &Regex,
    text: &'a str,
) -> HashMap<String, (usize, regex::Captures<'a>)> {
    let mut map = HashMap::new();
    for (idx, caps) in regex.captures_iter(text).enumerate() {
        if let Some(handle) = caps.name("handle") {
            map.insert(handle.as_str().to_string(), (idx, caps));
        }
    }
    map
}

impl Watcher for SynapseV3Watcher {
    fn parse_and_update(
        &mut self,
        tray: &mut TrayManager,
        icon_settings: &IconSettings,
        debug: bool,
    ) -> Result<()> {
        let devices = self.parse_devices(&[])?;
        self.cached_devices = devices.clone();
        log_devices(&devices, debug);
        tray.update_devices(devices, icon_settings)
    }

    fn parse_and_update_with_settings(
        &mut self, tray: &mut TrayManager,
        settings: &mut Settings, debug: bool,
    ) -> Result<()> {
        let devices = self.parse_devices(&settings.device_configs)?;
        self.cached_devices = devices.clone();
        log_devices(&devices, debug);
        let icon_settings = settings.to_icon_settings();
        tray.update_devices(devices, &icon_settings)
    }

    fn last_devices(&self) -> DeviceMap {
        self.cached_devices.clone()
    }

    fn check_log_rotation(&mut self, _debug: bool) {
        // V3 uses a single log file, no rotation needed
    }
}


