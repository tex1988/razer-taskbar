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
        let mut devices = DeviceMap::new();

        // Parse battery state changes
        let battery_matches = get_last_match_by_handle(&BATTERY_STATE_REGEX, &log_content);
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
                    serial_number: None, // V3 logs don't provide serial numbers
                    battery_percentage: level,
                    is_charging,
                    is_connected: false,
                    is_selected: is_visible,
                    category: DeviceCategory::Unknown, // V3 logs don't provide category info
                },
            );
        }

        // Parse connection status
        let loaded_matches = get_last_match_by_handle_with_index(&DEVICE_LOADED_REGEX, &log_content);
        let removed_matches = get_last_match_by_handle_with_index(&DEVICE_REMOVED_REGEX, &log_content);

        for handle in loaded_matches.keys().chain(removed_matches.keys()) {
            if let Some(device) = devices.get_mut(handle) {
                let loaded_idx = loaded_matches.get(handle).map(|(idx, _)| *idx).unwrap_or(0);
                let removed_idx = removed_matches.get(handle).map(|(idx, _)| *idx).unwrap_or(0);
                device.is_connected = loaded_idx > removed_idx;
            }
        }

        Ok(devices)
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


