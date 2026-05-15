/// Integration tests for Synapse V3 log parsing.
///
/// Tests the full parse pipeline via `parse_log_content`, which operates on
/// raw log strings so no actual Synapse installation is required.
use razer_taskbar::engine::watcher_v3::parse_log_content;
use razer_taskbar::model::DeviceConfig;

// ── Fixture log snippets ───────────────────────────────────────

/// Single device, one battery event, no connection events.
const SINGLE_DEVICE_LOG: &str = "\
2024-01-15 10:30:00 INFO [SynapseAPI] _OnBatteryLevelChanged
  DeviceInfo:
  Name: Razer Basilisk V3
  Handle: 12345
  Battery: level 75 state 0";

/// Charging state variant (state != 0).
const CHARGING_LOG: &str = "\
2024-01-15 10:30:00 INFO [SynapseAPI] _OnBatteryLevelChanged
  Name: Razer Viper V2
  Handle: 99999
  Battery: level 42 state 1";

/// Two different devices in the same log.
const TWO_DEVICE_LOG: &str = "\
2024-01-15 10:00:00 INFO [SynapseAPI] _OnBatteryLevelChanged
  Name: Razer Mouse
  Handle: 11111
  Battery: level 80 state 0
2024-01-15 10:01:00 INFO [SynapseAPI] _OnBatteryLevelChanged
  Name: Razer Keyboard
  Handle: 22222
  Battery: level 60 state 0";

/// Device loaded twice (idx=0 then idx=1 in loaded iterator) → connected.
const CONNECTED_DEVICE_LOG: &str = "\
2024-01-15 09:00:00 INFO [SynapseAPI] _OnDeviceLoaded
  Name: Razer Basilisk V3
  Handle: 12345
2024-01-15 09:30:00 INFO [SynapseAPI] _OnDeviceLoaded
  Name: Razer Basilisk V3
  Handle: 12345
2024-01-15 10:30:00 INFO [SynapseAPI] _OnBatteryLevelChanged
  Name: Razer Basilisk V3
  Handle: 12345
  Battery: level 80 state 0";

/// Two battery events for the same handle — the later value must win.
const DUPLICATE_HANDLE_LOG: &str = "\
2024-01-15 09:00:00 INFO [SynapseAPI] _OnBatteryLevelChanged
  Name: Razer Basilisk V3
  Handle: 12345
  Battery: level 20 state 0
2024-01-15 10:00:00 INFO [SynapseAPI] _OnBatteryLevelChanged
  Name: Razer Basilisk V3
  Handle: 12345
  Battery: level 95 state 0";

// ── Tests ──────────────────────────────────────────────────────

#[test]
fn empty_log_returns_empty_device_map() {
    let devices = parse_log_content("", &[]).unwrap();
    assert!(devices.is_empty());
}

#[test]
fn log_without_battery_events_returns_empty_map() {
    let log = "[2024-01-15 10:00:00] SomeOtherEvent irrelevant data";
    let devices = parse_log_content(log, &[]).unwrap();
    assert!(devices.is_empty());
}

#[test]
fn parses_device_name_and_handle() {
    let devices = parse_log_content(SINGLE_DEVICE_LOG, &[]).unwrap();
    let dev = devices.get("12345").expect("device 12345 not found");
    assert_eq!(dev.name, "Razer Basilisk V3");
    assert_eq!(dev.handle, "12345");
}

#[test]
fn parses_battery_percentage() {
    let devices = parse_log_content(SINGLE_DEVICE_LOG, &[]).unwrap();
    assert_eq!(devices["12345"].battery_percentage, 75);
}

#[test]
fn not_charging_when_state_is_zero() {
    let devices = parse_log_content(SINGLE_DEVICE_LOG, &[]).unwrap();
    assert!(!devices["12345"].is_charging);
}

#[test]
fn is_charging_when_state_is_nonzero() {
    let devices = parse_log_content(CHARGING_LOG, &[]).unwrap();
    assert!(devices["99999"].is_charging);
}

#[test]
fn device_not_connected_without_device_loaded_events() {
    // Battery event only — no _OnDeviceLoaded → is_connected stays false
    let devices = parse_log_content(SINGLE_DEVICE_LOG, &[]).unwrap();
    assert!(!devices["12345"].is_connected);
}

#[test]
fn device_connected_when_loaded_idx_exceeds_removed_idx() {
    // Two _OnDeviceLoaded events → loaded_idx=1 > removed_idx=0 → connected
    let devices = parse_log_content(CONNECTED_DEVICE_LOG, &[]).unwrap();
    assert!(devices["12345"].is_connected);
}

#[test]
fn parses_two_distinct_devices() {
    let devices = parse_log_content(TWO_DEVICE_LOG, &[]).unwrap();
    assert_eq!(devices.len(), 2);
    assert_eq!(devices["11111"].battery_percentage, 80);
    assert_eq!(devices["22222"].battery_percentage, 60);
    assert_eq!(devices["22222"].name, "Razer Keyboard");
}

#[test]
fn last_battery_event_per_handle_wins() {
    let devices = parse_log_content(DUPLICATE_HANDLE_LOG, &[]).unwrap();
    // 95 is the later reading — must override 20
    assert_eq!(devices["12345"].battery_percentage, 95);
}

#[test]
fn device_is_selected_true_when_no_config_entry() {
    let devices = parse_log_content(SINGLE_DEVICE_LOG, &[]).unwrap();
    assert!(devices["12345"].is_selected);
}

#[test]
fn device_is_selected_false_when_config_marks_hidden() {
    let configs = vec![DeviceConfig {
        id: "12345".into(),
        name: "Razer Basilisk V3".into(),
        visible: false,
        connected: false,
    }];
    let devices = parse_log_content(SINGLE_DEVICE_LOG, &configs).unwrap();
    assert!(!devices["12345"].is_selected);
}

#[test]
fn device_is_selected_true_when_config_marks_visible() {
    let configs = vec![DeviceConfig {
        id: "12345".into(),
        name: "Razer Basilisk V3".into(),
        visible: true,
        connected: false,
    }];
    let devices = parse_log_content(SINGLE_DEVICE_LOG, &configs).unwrap();
    assert!(devices["12345"].is_selected);
}
