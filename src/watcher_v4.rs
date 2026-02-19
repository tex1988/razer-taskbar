use crate::device::{DeviceCategory, DeviceMap, RazerDevice};
use anyhow::Result;
use lazy_static::lazy_static;
use regex::Regex;
use serde::Deserialize;
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

#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
struct LoggedDeviceInfo {
    serial_number: Option<String>,
    #[serde(default)]
    has_battery: bool,
    device_container_id: String,
    #[serde(default)]
    power_status: Option<PowerStatus>,
    name: NameMap,
    #[allow(dead_code)]
    product_name: NameMap,
    #[serde(default)]
    category: DeviceCategory,
}

#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
struct PowerStatus {
    charging_status: ChargingStatus,
    level: u8,
}

#[derive(Debug, Deserialize)]
#[serde(untagged)]
enum ChargingStatus {
    Text(String),
}

impl ChargingStatus {
    fn is_charging(&self) -> bool {
        match self {
            ChargingStatus::Text(s) => {
                s.to_lowercase() == "charging" || s.to_lowercase() == "charge"
            }
        }
    }
}

#[derive(Debug, Deserialize)]
struct NameMap {
    en: String,
}

pub struct SynapseV4Watcher {
    log_dir: PathBuf,
    last_parsed_timestamp: String,
    last_devices: DeviceMap,
}

impl SynapseV4Watcher {
    pub fn new() -> Option<Self> {
        let local_appdata = env::var("LOCALAPPDATA").ok()?;
        let log_dir = PathBuf::from(local_appdata)
            .join("Razer")
            .join("RazerAppEngine")
            .join("User Data")
            .join("Logs");

        if log_dir.exists() {
            Some(Self {
                log_dir,
                last_parsed_timestamp: String::new(),
                last_devices: DeviceMap::new(),
            })
        } else {
            None
        }
    }

    pub fn find_latest_log_file(&self) -> Option<PathBuf> {
        let entries = fs::read_dir(&self.log_dir).ok()?;
        let mut candidates: Vec<(i32, PathBuf)> = Vec::new();

        for entry in entries.flatten() {
            let file_name = entry.file_name();
            let file_name_str = file_name.to_string_lossy();

            if let Some(caps) = LOG_FILE_REGEX.captures(&file_name_str) {
                let index = caps.name("index")
                    .and_then(|m| m.as_str().parse::<i32>().ok())
                    .unwrap_or(-1);
                candidates.push((index, entry.path()));
            }
        }

        candidates.sort_by(|a, b| b.0.cmp(&a.0));
        candidates.first().map(|(_, path)| path.clone())
    }

    pub fn parse_devices(&mut self, log_path: &PathBuf, shown_device_handle: &str, debug: bool) -> Result<DeviceMap> {
        let log_content = fs::read_to_string(log_path)?;
        let mut devices = DeviceMap::new();

        let matches: Vec<_> = BATTERY_STATE_REGEX.captures_iter(&log_content).collect();
        if debug { println!("Found {} connectingDeviceData entries in log", matches.len()); }

        if let Some(last_match) = matches.last() {
            let timestamp = last_match.name("timestamp").unwrap().as_str();
            if self.last_parsed_timestamp == timestamp {
                if debug { println!("No new changes detected. Last change at {}", timestamp); }
                return Ok(self.last_devices.clone());
            }
            self.last_parsed_timestamp = timestamp.to_string();

            let json_str = last_match.name("json").unwrap().as_str();
            if debug { println!("Parsing JSON (first 200 chars): {}", &json_str.chars().take(200).collect::<String>()); }

            let device_infos: Vec<LoggedDeviceInfo> = match serde_json::from_str(json_str) {
                Ok(infos) => infos,
                Err(e) => {
                    if debug {
                        eprintln!("JSON parse error: {}", e);
                        eprintln!("JSON content (first 500 chars): {}", &json_str.chars().take(500).collect::<String>());
                    }
                    return Err(e.into());
                }
            };

            if debug { println!("Parsed {} devices from JSON", device_infos.len()); }

            let devices_with_battery: Vec<_> = device_infos.iter().filter(|d| d.has_battery).collect();
            if debug { println!("Devices with battery: {}", devices_with_battery.len()); }

            for device_info in devices_with_battery {
                let handle = device_info.serial_number.clone()
                    .unwrap_or_else(|| device_info.device_container_id.clone());

                if debug { println!("Processing device: {} (handle: {})", device_info.name.en, handle); }

                let power_status = match &device_info.power_status {
                    Some(ps) => {
                        if debug { println!("  Power status: level={}, charging={:?}", ps.level, ps.charging_status); }
                        ps
                    },
                    None => {
                        if debug { println!("  No power status - skipping"); }
                        continue;
                    }
                };

                let is_charging = power_status.charging_status.is_charging();
                let is_connected = device_infos.iter().any(|d| {
                    d.serial_number.as_ref().map(|s| s == &handle).unwrap_or(false)
                        || d.device_container_id == handle
                });
                let is_selected = shown_device_handle == handle || shown_device_handle.is_empty();

                if debug { println!("  is_charging: {}, is_connected: {}, is_selected: {}", is_charging, is_connected, is_selected); }

                devices.insert(
                    handle.clone(),
                    RazerDevice {
                        name: device_info.name.en.clone(),
                        handle: handle.clone(),
                        battery_percentage: power_status.level,
                        is_charging,
                        is_connected,
                        is_selected,
                        category: device_info.category,
                    },
                );

                if debug { println!("  Added device: {} at {}% (charging: {}, connected: {}, selected: {})",
                    device_info.name.en, power_status.level, is_charging, is_connected, is_selected); }
            }

            if let Some(no_serial_device) = devices.get("NOSERIALNUMBER").cloned() {
                if devices.values().any(|d| d.handle != "NOSERIALNUMBER" && d.name == no_serial_device.name) {
                    devices.remove("NOSERIALNUMBER");
                }
            }

            if debug { println!("Parsed battery changes until {}", timestamp); }
        }

        self.last_devices = devices.clone();
        Ok(devices)
    }

    pub fn log_dir(&self) -> &PathBuf {
        &self.log_dir
    }
}
