/// Integration tests for Settings persistence (save/load roundtrip).
use razer_taskbar::model::{DeviceConfig, IconTheme, Settings, SynapseVersion, TextAlignment};
use std::path::PathBuf;

fn temp_path(name: &str) -> PathBuf {
    std::env::temp_dir().join(format!("razer_test_settings_{name}.json"))
}

fn cleanup(path: &PathBuf) {
    let _ = std::fs::remove_file(path);
}

// ── default values ─────────────────────────────────────────────

#[test]
fn default_settings_have_expected_polling_interval() {
    let s = Settings::default();
    assert_eq!(s.polling_interval_minutes, 10);
    assert_eq!(s.polling_interval_seconds, 0);
}

#[test]
fn default_settings_active_theme_is_default() {
    assert_eq!(Settings::default().active_theme, "Default");
}

#[test]
fn default_settings_show_percentage_is_false() {
    assert!(!Settings::default().show_percentage);
}

// ── save_to_path / load_from_path roundtrip ────────────────────

#[test]
fn save_and_load_roundtrip_preserves_all_scalar_fields() {
    let path = temp_path("roundtrip");
    cleanup(&path);

    let mut s = Settings::default();
    s.polling_interval_minutes = 3;
    s.polling_interval_seconds = 15;
    s.show_percentage = true;
    s.percentage_text_color = "FF8800".into();
    s.active_theme = "Custom".into();
    s.icon_theme = IconTheme::Dark;
    s.display_charging_state = false;
    s.synapse_version = SynapseVersion::V4;

    s.save_to_path(&path).expect("save failed");
    let loaded = Settings::load_from_path(&path).expect("load failed");

    assert_eq!(loaded.polling_interval_minutes, 3);
    assert_eq!(loaded.polling_interval_seconds, 15);
    assert_eq!(loaded.show_percentage, true);
    assert_eq!(loaded.percentage_text_color, "FF8800");
    assert_eq!(loaded.active_theme, "Custom");
    assert_eq!(loaded.icon_theme, IconTheme::Dark);
    assert_eq!(loaded.display_charging_state, false);
    assert_eq!(loaded.synapse_version, SynapseVersion::V4);

    cleanup(&path);
}

#[test]
fn save_and_load_roundtrip_preserves_device_configs() {
    let path = temp_path("device_configs");
    cleanup(&path);

    let mut s = Settings::default();
    s.device_configs.push(DeviceConfig {
        id: "SN-MOUSE-1".into(),
        name: "Razer DeathAdder".into(),
        visible: false,
        connected: false,
    });

    s.save_to_path(&path).expect("save failed");
    let loaded = Settings::load_from_path(&path).expect("load failed");

    assert_eq!(loaded.device_configs.len(), 1);
    assert_eq!(loaded.device_configs[0].id, "SN-MOUSE-1");
    assert_eq!(loaded.device_configs[0].name, "Razer DeathAdder");
    assert!(!loaded.device_configs[0].visible);

    cleanup(&path);
}

#[test]
fn load_from_path_fails_for_missing_file() {
    let path = temp_path("does_not_exist");
    cleanup(&path);
    assert!(Settings::load_from_path(&path).is_err());
}

// ── backward-compatible JSON deserialization ───────────────────

#[test]
fn partial_json_gets_defaults_for_missing_fields() {
    let path = temp_path("partial_json");
    cleanup(&path);

    // Write a minimal JSON with only one field
    let json = r#"{"show_percentage": true}"#;
    std::fs::write(&path, json).unwrap();

    let loaded = Settings::load_from_path(&path).expect("load failed");
    assert!(loaded.show_percentage);
    // All other fields should be defaults
    assert_eq!(loaded.active_theme, "Default");
    assert_eq!(loaded.polling_interval_minutes, 10);
    assert_eq!(loaded.percentage_text_align, TextAlignment::Center);

    cleanup(&path);
}

// ── polling_interval_total_seconds (cross-component) ──────────

#[test]
fn total_seconds_computed_correctly_after_roundtrip() {
    let path = temp_path("interval_roundtrip");
    cleanup(&path);

    let mut s = Settings::default();
    s.polling_interval_minutes = 2;
    s.polling_interval_seconds = 30;
    s.save_to_path(&path).unwrap();

    let loaded = Settings::load_from_path(&path).unwrap();
    assert_eq!(loaded.polling_interval_total_seconds(), 150);

    cleanup(&path);
}
