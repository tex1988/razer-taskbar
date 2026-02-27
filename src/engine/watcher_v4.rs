use super::event_loop::Watcher;
use super::watcher_common::log_devices;
use crate::util::log;
use crate::model::{DeviceMap, IconSettings, LoggedDeviceInfo, RazerDevice};
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
    hidden_device_handles: Vec<String>,
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
                hidden_device_handles: Vec::new(),
                last_parsed_timestamp: String::new(),
                last_devices: DeviceMap::new(),
            })
        } else { None }
    }

    pub fn init(&mut self, hidden_handles: &[String]) -> Result<()> {
        self.hidden_device_handles = hidden_handles.to_vec();
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
        let hidden = &self.hidden_device_handles;
        let mut devices = build_device_map(&infos, hidden, debug);
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
    infos: &[LoggedDeviceInfo], hidden_handles: &[String], debug: bool,
) -> DeviceMap {
    let mut devices = DeviceMap::new();
    for info in infos.iter().filter(|d| d.has_battery) {
        if let Some(dev) = build_device(info, infos, hidden_handles, debug) {
            devices.insert(dev.handle.clone(), dev);
        }
    }
    devices
}

fn build_device(
    info: &LoggedDeviceInfo, all: &[LoggedDeviceInfo],
    hidden_handles: &[String], debug: bool,
) -> Option<RazerDevice> {
    let handle = info.serial_number.clone()
        .unwrap_or_else(|| info.device_container_id.clone());
    let ps = info.power_status.as_ref()?;
    if debug { println!("  Device: {} level={}", info.name.en, ps.level); }
    let is_connected = all.iter().any(|d| {
        d.serial_number.as_ref().map(|s| s == &handle).unwrap_or(false)
            || d.device_container_id == handle
    });
    let sn = info.serial_number.as_deref()
        .unwrap_or(&info.device_container_id);
    Some(RazerDevice {
        name: info.name.en.clone(), handle,
        battery_percentage: ps.level,
        is_charging: ps.charging_status.is_charging(),
        is_connected,
        is_selected: !hidden_handles.iter().any(|h| h == sn || h == &info.device_container_id),
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

impl Watcher for SynapseV4Watcher {
    fn parse_and_update(
        &mut self, tray: &mut TrayManager,
        icon_settings: &IconSettings, debug: bool,
    ) -> Result<()> {
        let devices = self.parse_devices(debug)?;
        log_devices(&devices, debug);
        tray.update_devices(devices, icon_settings)
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

