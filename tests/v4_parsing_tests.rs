/// Integration tests for Synapse V4 JSON log parsing.
///
/// Tests the full parse pipeline via `parse_devices_from_str`, which operates
/// on raw log strings so no actual Synapse installation is required.
use razer_taskbar::engine::watcher_v4::parse_devices_from_str;
use razer_taskbar::model::{DeviceCategory, DeviceConfig};

// ── Fixture helpers ────────────────────────────────────────────

fn make_log_line(json_array: &str, timestamp: &str) -> String {
    format!("[{timestamp}] [SystrayApp] connectingDeviceData: {json_array}")
}

fn single_device_json(serial: &str, level: u8, charging: &str, category: &str) -> String {
    format!(
        r#"[{{"serialNumber":"{serial}","hasBattery":true,"deviceContainerId":"CONT","powerStatus":{{"chargingStatus":"{charging}","level":{level}}},"name":{{"en":"Razer Mouse"}},"productName":{{"en":"Razer Mouse"}},"category":"{category}"}}]"#
    )
}

// ── Tests ──────────────────────────────────────────────────────

#[test]
fn empty_log_returns_empty_device_map() {
    let result = parse_devices_from_str("", &[], false).unwrap();
    assert!(result.is_empty());
}

#[test]
fn log_without_connecting_device_data_line_returns_empty() {
    let result = parse_devices_from_str("[2024] [App] unrelated: data", &[], false).unwrap();
    assert!(result.is_empty());
}

#[test]
fn parses_single_device_name_and_battery() {
    let json = single_device_json("SN-001", 75, "Discharging", "MOUSE");
    let log = make_log_line(&json, "2024-01-15 10:30:00.000");
    let devices = parse_devices_from_str(&log, &[], false).unwrap();
    assert_eq!(devices.len(), 1);
    let dev = devices.get("SN-001").expect("SN-001 not found");
    assert_eq!(dev.name, "Razer Mouse");
    assert_eq!(dev.battery_percentage, 75);
    assert_eq!(dev.serial_number.as_deref(), Some("SN-001"));
}

#[test]
fn device_is_not_charging_for_discharging_status() {
    let json = single_device_json("SN-001", 80, "Discharging", "MOUSE");
    let log = make_log_line(&json, "2024-01-15 10:30:00.000");
    let devices = parse_devices_from_str(&log, &[], false).unwrap();
    assert!(!devices["SN-001"].is_charging);
}

#[test]
fn device_is_charging_for_charging_status() {
    let json = single_device_json("SN-001", 55, "Charging", "MOUSE");
    let log = make_log_line(&json, "2024-01-15 10:30:00.000");
    let devices = parse_devices_from_str(&log, &[], false).unwrap();
    assert!(devices["SN-001"].is_charging);
}

#[test]
fn parses_device_category_mouse() {
    let json = single_device_json("SN-001", 75, "Discharging", "MOUSE");
    let log = make_log_line(&json, "2024-01-15 10:30:00.000");
    let devices = parse_devices_from_str(&log, &[], false).unwrap();
    assert_eq!(devices["SN-001"].category, DeviceCategory::Mouse);
}

#[test]
fn parses_device_category_keyboard() {
    let json = single_device_json("SN-001", 75, "Discharging", "KEYBOARD");
    let log = make_log_line(&json, "2024-01-15 10:30:00.000");
    let devices = parse_devices_from_str(&log, &[], false).unwrap();
    assert_eq!(devices["SN-001"].category, DeviceCategory::Keyboard);
}

#[test]
fn skips_device_without_battery() {
    let json = r#"[{"serialNumber":"SN-WIRED","hasBattery":false,"deviceContainerId":"CONT","name":{"en":"Wired Mouse"},"productName":{"en":"Wired Mouse"},"category":"MOUSE"}]"#;
    let log = make_log_line(json, "2024-01-15 10:30:00.000");
    let devices = parse_devices_from_str(&log, &[], false).unwrap();
    assert!(devices.is_empty());
}

#[test]
fn skips_device_without_power_status() {
    let json = r#"[{"serialNumber":"SN-001","hasBattery":true,"deviceContainerId":"CONT","name":{"en":"Mouse"},"productName":{"en":"Mouse"},"category":"MOUSE"}]"#;
    let log = make_log_line(json, "2024-01-15 10:30:00.000");
    let devices = parse_devices_from_str(&log, &[], false).unwrap();
    assert!(devices.is_empty());
}

#[test]
fn parses_multiple_devices_from_single_json_array() {
    // JSON must be on a single line — the V4 parser regex requires it.
    let json = r#"[{"serialNumber":"SN-MOUSE","hasBattery":true,"deviceContainerId":"C1","powerStatus":{"chargingStatus":"Discharging","level":80},"name":{"en":"Razer Mouse"},"productName":{"en":"Mouse"},"category":"MOUSE"},{"serialNumber":"SN-KB","hasBattery":true,"deviceContainerId":"C2","powerStatus":{"chargingStatus":"Charging","level":30},"name":{"en":"Razer Keyboard"},"productName":{"en":"Keyboard"},"category":"KEYBOARD"}]"#;
    let log = make_log_line(json, "2024-01-15 10:30:00.000");
    let devices = parse_devices_from_str(&log, &[], false).unwrap();
    assert_eq!(devices.len(), 2);
    assert_eq!(devices["SN-MOUSE"].battery_percentage, 80);
    assert_eq!(devices["SN-KB"].battery_percentage, 30);
    assert!(devices["SN-KB"].is_charging);
}

#[test]
fn only_last_log_line_is_parsed() {
    // Two log lines — only the last connectingDeviceData entry is processed
    let first = make_log_line(
        &single_device_json("SN-OLD", 10, "Discharging", "MOUSE"),
        "2024-01-15 09:00:00.000",
    );
    let last = make_log_line(
        &single_device_json("SN-NEW", 90, "Charging", "KEYBOARD"),
        "2024-01-15 10:00:00.000",
    );
    let log = format!("{first}\n{last}");
    let devices = parse_devices_from_str(&log, &[], false).unwrap();
    assert!(!devices.contains_key("SN-OLD"));
    assert!(devices.contains_key("SN-NEW"));
    assert_eq!(devices["SN-NEW"].battery_percentage, 90);
}

#[test]
fn device_visible_by_default_when_no_config() {
    let json = single_device_json("SN-001", 75, "Discharging", "MOUSE");
    let log = make_log_line(&json, "2024-01-15 10:30:00.000");
    let devices = parse_devices_from_str(&log, &[], false).unwrap();
    assert!(devices["SN-001"].is_selected);
}

#[test]
fn device_hidden_when_config_marks_not_visible() {
    let json = single_device_json("SN-001", 75, "Discharging", "MOUSE");
    let log = make_log_line(&json, "2024-01-15 10:30:00.000");
    let configs = vec![DeviceConfig {
        id: "SN-001".into(), name: "Mouse".into(), visible: false, connected: false,
    }];
    let devices = parse_devices_from_str(&log, &configs, false).unwrap();
    assert!(!devices["SN-001"].is_selected);
}

#[test]
fn noserialnumber_device_removed_when_named_duplicate_exists() {
    // A device with serialNumber="NOSERIALNUMBER" should be deduped against a real serial entry.
    // JSON must be on a single line — the V4 parser regex requires it.
    let json = r#"[{"serialNumber":"NOSERIALNUMBER","hasBattery":true,"deviceContainerId":"C1","powerStatus":{"chargingStatus":"Discharging","level":75},"name":{"en":"Razer Mouse"},"productName":{"en":"Mouse"},"category":"MOUSE"},{"serialNumber":"SN-REAL","hasBattery":true,"deviceContainerId":"C2","powerStatus":{"chargingStatus":"Discharging","level":75},"name":{"en":"Razer Mouse"},"productName":{"en":"Mouse"},"category":"MOUSE"}]"#;
    let log = make_log_line(json, "2024-01-15 10:30:00.000");
    let devices = parse_devices_from_str(&log, &[], false).unwrap();
    assert!(!devices.contains_key("NOSERIALNUMBER"), "NOSERIALNUMBER should be deduped");
    assert!(devices.contains_key("SN-REAL"));
}

#[test]
fn noserialnumber_device_kept_when_no_named_duplicate() {
    // JSON must be on a single line — the V4 parser regex requires it.
    let json = r#"[{"serialNumber":"NOSERIALNUMBER","hasBattery":true,"deviceContainerId":"C1","powerStatus":{"chargingStatus":"Discharging","level":50},"name":{"en":"Unique Device"},"productName":{"en":"Unique"},"category":"UNKNOWN"}]"#;
    let log = make_log_line(json, "2024-01-15 10:30:00.000");
    let devices = parse_devices_from_str(&log, &[], false).unwrap();
    assert!(devices.contains_key("NOSERIALNUMBER"));
}
