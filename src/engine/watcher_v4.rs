use super::event_loop::Watcher;
use super::watcher_common::log_devices;
use crate::util::log;
use crate::model::{DeviceConfig, DeviceMap, IconSettings, LoggedDeviceInfo, RazerDevice};
use crate::model::Settings;
use crate::ui::TrayManager;
use anyhow::Result;
use lazy_static::lazy_static;
use regex::Regex;
use std::env;
use std::fs;
use std::path::PathBuf;

lazy_static! {
    static ref BATTERY_STATE_REGEX: Regex = Regex::new(
        r"(?m)^\[(?P<timestamp>[^\]]+)\].*connectingDeviceData:\s*(?P<json>.+)$"
    ).unwrap();
    static ref LOG_FILE_REGEX: Regex = Regex::new(
        r"^systray_systrayv2(?P<index>\d*).log$"
    ).unwrap();
}

pub struct SynapseV4Watcher {
    log_dir: PathBuf,
    log_path: Option<PathBuf>,
    device_configs: Vec<DeviceConfig>,
    last_parsed_timestamp: String,
    last_devices: DeviceMap,
}

impl SynapseV4Watcher {
    pub fn new() -> Option<Self> {
        let local_appdata = env::var("LOCALAPPDATA").ok()?;
        let log_dir = PathBuf::from(local_appdata)
            .join("Razer").join("RazerAppEngine")
            .join("User Data").join("Logs");
        if log_dir.exists() {
            Some(Self {
                log_dir, log_path: None,
                device_configs: Vec::new(),
                last_parsed_timestamp: String::new(),
                last_devices: DeviceMap::new(),
            })
        } else { None }
    }

    pub fn init(&mut self, device_configs: &[DeviceConfig]) -> Result<()> {
        self.device_configs = device_configs.to_vec();
        self.log_path = self.find_latest_log_file();
        if self.log_path.is_none() {
            anyhow::bail!("No Synapse V4 log files found");
        }
        Ok(())
    }

    pub fn find_latest_log_file(&self) -> Option<PathBuf> {
        let entries = fs::read_dir(&self.log_dir).ok()?;
        let mut candidates: Vec<(i32, PathBuf)> = Vec::new();
        for entry in entries.flatten() {
            let name = entry.file_name();
            let name_str = name.to_string_lossy();
            if let Some(caps) = LOG_FILE_REGEX.captures(&name_str) {
                let idx = caps.name("index")
                    .and_then(|m| m.as_str().parse::<i32>().ok())
                    .unwrap_or(-1);
                candidates.push((idx, entry.path()));
            }
        }
        candidates.sort_by(|a, b| b.0.cmp(&a.0));
        candidates.first().map(|(_, path)| path.clone())
    }

    fn parse_devices(&mut self, debug: bool) -> Result<DeviceMap> {
        let log_path = self.log_path.as_ref()
            .ok_or_else(|| anyhow::anyhow!("No log path set"))?;
        let log_content = fs::read_to_string(log_path)?;
        let matches: Vec<_> = BATTERY_STATE_REGEX
            .captures_iter(&log_content).collect();
        if debug {
            println!("Found {} connectingDeviceData entries", matches.len());
        }
        let last_match = match matches.last() {
            Some(m) => m,
            None => return Ok(DeviceMap::new()),
        };
        let timestamp = last_match.name("timestamp").unwrap().as_str();
        if self.last_parsed_timestamp == timestamp {
            if debug { println!("No new changes. Last at {}", timestamp); }
            return Ok(self.last_devices.clone());
        }
        self.last_parsed_timestamp = timestamp.to_string();
        let json_str = last_match.name("json").unwrap().as_str();
        let infos = parse_json(json_str, debug)?;
        let configs = &self.device_configs;
        let mut devices = build_device_map(&infos, configs, debug);
        remove_duplicate_no_serial(&mut devices);
        if debug { println!("Parsed battery changes until {}", timestamp); }
        self.last_devices = devices.clone();
        Ok(devices)
    }
}

fn parse_json(json_str: &str, debug: bool) -> Result<Vec<LoggedDeviceInfo>> {
    match serde_json::from_str(json_str) {
        Ok(infos) => Ok(infos),
        Err(e) => {
            if debug {
                eprintln!("JSON parse error: {}", e);
                let preview: String = json_str.chars().take(500).collect();
                eprintln!("JSON preview: {}", preview);
            }
            Err(e.into())
        }
    }
}

fn build_device_map(
    infos: &[LoggedDeviceInfo], configs: &[DeviceConfig], debug: bool,
) -> DeviceMap {
    let mut devices = DeviceMap::new();
    for info in infos.iter().filter(|d| d.has_battery) {
        if let Some(dev) = build_device(info, infos, configs, debug) {
            devices.insert(dev.unique_id().to_string(), dev);
        }
    }
    devices
}

fn build_device(
    info: &LoggedDeviceInfo, all: &[LoggedDeviceInfo],
    configs: &[DeviceConfig], debug: bool,
) -> Option<RazerDevice> {
    let handle = info.serial_number.clone()
        .unwrap_or_else(|| info.device_container_id.clone());
    let ps = info.power_status.as_ref()?;
    if debug { println!("  Device: {} level={}", info.name.en, ps.level); }
    let is_connected = all.iter().any(|d| {
        d.serial_number.as_ref().map(|s| s == &handle).unwrap_or(false)
            || d.device_container_id == handle
    });
    let unique_id = info.serial_number.as_deref()
        .unwrap_or(&info.device_container_id);
    let is_visible = configs.iter()
        .find(|c| c.id == unique_id)
        .map(|c| c.visible)
        .unwrap_or(true);
    Some(RazerDevice {
        name: info.name.en.clone(),
        handle,
        serial_number: info.serial_number.clone(),
        battery_percentage: ps.level,
        is_charging: ps.charging_status.is_charging(),
        is_connected,
        is_selected: is_visible,
        category: info.category,
    })
}

fn remove_duplicate_no_serial(devices: &mut DeviceMap) {
    if let Some(no_serial) = devices.get("NOSERIALNUMBER").cloned() {
        let has_dup = devices.values().any(|d| {
            d.handle != "NOSERIALNUMBER" && d.name == no_serial.name
        });
        if has_dup { devices.remove("NOSERIALNUMBER"); }
    }
}

/// Parse devices from raw Synapse V4 log content.
/// Exposed for testing without requiring an actual log file.
pub fn parse_devices_from_str(content: &str, configs: &[DeviceConfig], debug: bool) -> Result<DeviceMap> {
    let matches: Vec<_> = BATTERY_STATE_REGEX.captures_iter(content).collect();
    let last_match = match matches.last() {
        Some(m) => m,
        None => return Ok(DeviceMap::new()),
    };
    let json_str = last_match.name("json").unwrap().as_str();
    let infos = parse_json(json_str, debug)?;
    let mut devices = build_device_map(&infos, configs, debug);
    remove_duplicate_no_serial(&mut devices);
    Ok(devices)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::model::{DeviceCategory, DeviceConfig};

    // ── Helpers ────────────────────────────────────────────────

    fn single_device_log(serial: &str, level: u8, charging: &str, category: &str) -> String {
        format!(
            r#"[2024-01-15 10:30:00.000] [SystrayApp] connectingDeviceData: [{{"serialNumber":"{serial}","hasBattery":true,"deviceContainerId":"CONT1","powerStatus":{{"chargingStatus":"{charging}","level":{level}}},"name":{{"en":"Razer Mouse"}},"productName":{{"en":"Razer Mouse"}},"category":"{category}"}}]"#
        )
    }

    // ── parse_devices_from_str ────────────────────────────────

    #[test]
    fn returns_empty_map_for_empty_log() {
        let result = parse_devices_from_str("", &[], false).unwrap();
        assert!(result.is_empty());
    }

    #[test]
    fn returns_empty_map_when_no_connecting_device_data_line() {
        let result = parse_devices_from_str("[2024] [App] something else: foo", &[], false).unwrap();
        assert!(result.is_empty());
    }

    #[test]
    fn parses_single_device_battery_and_name() {
        let log = single_device_log("SN-001", 75, "Discharging", "MOUSE");
        let devices = parse_devices_from_str(&log, &[], false).unwrap();
        assert_eq!(devices.len(), 1);
        let dev = devices.get("SN-001").expect("device SN-001 missing");
        assert_eq!(dev.name, "Razer Mouse");
        assert_eq!(dev.battery_percentage, 75);
    }

    #[test]
    fn parses_charging_status_correctly() {
        let charging_log = single_device_log("SN-001", 50, "Charging", "MOUSE");
        let discharging_log = single_device_log("SN-002", 80, "Discharging", "MOUSE");

        let dev_charging = parse_devices_from_str(&charging_log, &[], false).unwrap();
        assert!(dev_charging["SN-001"].is_charging);

        let dev_dis = parse_devices_from_str(&discharging_log, &[], false).unwrap();
        assert!(!dev_dis["SN-002"].is_charging);
    }

    #[test]
    fn parses_device_category() {
        let log = single_device_log("SN-001", 75, "Discharging", "MOUSE");
        let devices = parse_devices_from_str(&log, &[], false).unwrap();
        assert_eq!(devices["SN-001"].category, DeviceCategory::Mouse);
    }

    #[test]
    fn skips_devices_without_battery() {
        let log = r#"[2024-01-15 10:30:00.000] [App] connectingDeviceData: [{"serialNumber":"SN-NO-BAT","hasBattery":false,"deviceContainerId":"CONT1","name":{"en":"Wired Mouse"},"productName":{"en":"Wired Mouse"},"category":"MOUSE"}]"#;
        let devices = parse_devices_from_str(log, &[], false).unwrap();
        assert!(devices.is_empty());
    }

    #[test]
    fn uses_last_connecting_device_data_entry() {
        // Two log lines — only the last one is parsed
        let log = format!(
            "{}\n{}",
            single_device_log("SN-OLD", 10, "Discharging", "MOUSE"),
            single_device_log("SN-NEW", 90, "Charging", "KEYBOARD"),
        );
        let devices = parse_devices_from_str(&log, &[], false).unwrap();
        assert!(!devices.contains_key("SN-OLD"));
        assert!(devices.contains_key("SN-NEW"));
    }

    #[test]
    fn device_visible_by_default_when_no_config() {
        let log = single_device_log("SN-001", 75, "Discharging", "MOUSE");
        let devices = parse_devices_from_str(&log, &[], false).unwrap();
        assert!(devices["SN-001"].is_selected);
    }

    #[test]
    fn device_hidden_when_config_marks_not_visible() {
        let log = single_device_log("SN-001", 75, "Discharging", "MOUSE");
        let configs = vec![DeviceConfig { id: "SN-001".into(), name: "Mouse".into(), visible: false, connected: false }];
        let devices = parse_devices_from_str(&log, &configs, false).unwrap();
        assert!(!devices["SN-001"].is_selected);
    }

    // ── remove_duplicate_no_serial ────────────────────────────

    #[test]
    fn remove_duplicate_no_serial_removes_when_named_dup_exists() {
        let mut devices = DeviceMap::new();
        let base = RazerDevice {
            name: "Razer Mouse".into(), handle: "".into(), serial_number: None,
            battery_percentage: 75, is_charging: false, is_connected: true,
            is_selected: true, category: DeviceCategory::Mouse,
        };
        devices.insert("NOSERIALNUMBER".into(), RazerDevice { handle: "NOSERIALNUMBER".into(), ..base.clone() });
        devices.insert("SN-REAL".into(),        RazerDevice { handle: "SN-REAL".into(),        ..base });
        remove_duplicate_no_serial(&mut devices);
        assert!(!devices.contains_key("NOSERIALNUMBER"));
        assert!(devices.contains_key("SN-REAL"));
    }

    #[test]
    fn remove_duplicate_no_serial_keeps_when_no_dup() {
        let mut devices = DeviceMap::new();
        devices.insert("NOSERIALNUMBER".into(), RazerDevice {
            name: "Unique Device".into(), handle: "NOSERIALNUMBER".into(), serial_number: None,
            battery_percentage: 50, is_charging: false, is_connected: true,
            is_selected: true, category: DeviceCategory::Unknown,
        });
        remove_duplicate_no_serial(&mut devices);
        assert!(devices.contains_key("NOSERIALNUMBER"));
    }

    #[test]
    fn uses_device_container_id_when_serial_number_absent() {
        // No serialNumber field in JSON → serial_number is None → key = deviceContainerId
        let log = r#"[2024-01-15 10:30:00.000] [SystrayApp] connectingDeviceData: [{"hasBattery":true,"deviceContainerId":"CONTAINER-1","powerStatus":{"chargingStatus":"Discharging","level":60},"name":{"en":"Razer Headset"},"productName":{"en":"Razer Headset"},"category":"HEADPHONES"}]"#;
        let devices = parse_devices_from_str(log, &[], false).unwrap();
        assert!(devices.contains_key("CONTAINER-1"), "should fall back to deviceContainerId");
        assert_eq!(devices["CONTAINER-1"].battery_percentage, 60);
    }

    #[test]
    fn parse_json_invalid_input_returns_error() {
        let result = parse_json("not valid json", false);
        assert!(result.is_err());
    }

    #[test]
    fn parse_json_valid_array_returns_device_list() {
        let json = r#"[{"hasBattery":true,"deviceContainerId":"C1","powerStatus":{"chargingStatus":"Discharging","level":75},"name":{"en":"Mouse"},"productName":{"en":"Mouse"},"category":"MOUSE"}]"#;
        let infos = parse_json(json, false).unwrap();
        assert_eq!(infos.len(), 1);
        assert_eq!(infos[0].name.en, "Mouse");
    }
}

impl Watcher for SynapseV4Watcher {
    fn parse_and_update(
        &mut self, tray: &mut TrayManager,
        icon_settings: &IconSettings, debug: bool,
    ) -> Result<()> {
        let devices = self.parse_devices(debug)?;
        log_devices(&devices, debug);
        tray.update_devices(devices, icon_settings)
    }

    fn parse_and_update_with_settings(
        &mut self, tray: &mut TrayManager,
        settings: &mut Settings, debug: bool,
    ) -> Result<()> {
        self.device_configs = settings.device_configs.clone();
        let devices = self.parse_devices(debug)?;
        log_devices(&devices, debug);
        let icon_settings = settings.to_icon_settings();
        tray.update_devices(devices, &icon_settings)
    }

    fn last_devices(&self) -> DeviceMap {
        self.last_devices.clone()
    }

    fn check_log_rotation(&mut self, debug: bool) {
        if let Some(new_path) = self.find_latest_log_file() {
            if self.log_path.as_ref() != Some(&new_path) {
                log(&format!("Log file changed to: {:?}", new_path), debug);
                self.log_path = Some(new_path);
            }
        }
    }
}

