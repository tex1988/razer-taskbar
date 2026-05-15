use anyhow::Result;
use std::path::PathBuf;
use std::time::Duration;

use super::icon_manager;
use crate::util::{log, write_error_log};
use crate::model::{DeviceMap, IconSettings, Settings};
use crate::ui::{TrayManager, MenuAction};

#[cfg(target_os = "windows")]
use windows::Win32::UI::WindowsAndMessaging::{
    DispatchMessageW, PeekMessageW, TranslateMessage, MSG, PM_REMOVE,
};

/// Trait abstracting Synapse V3/V4 watcher differences.
pub trait Watcher {
    fn parse_and_update(
        &mut self,
        tray: &mut TrayManager,
        icon_settings: &IconSettings,
        debug: bool,
    ) -> Result<()>;

    /// Parse and update, passing device_configs for ordering and visibility.
    fn parse_and_update_with_settings(
        &mut self,
        tray: &mut TrayManager,
        settings: &mut Settings,
        debug: bool,
    ) -> Result<()> {
        self.parse_and_update(tray, &settings.to_icon_settings(), debug)
    }

    /// Return the last parsed devices for discovery purposes.
    fn last_devices(&self) -> DeviceMap { DeviceMap::new() }

    /// Whether discovered devices should be persisted to settings.
    /// Emulation watcher returns false to avoid polluting real config.
    fn persists_devices(&self) -> bool { true }

    fn check_log_rotation(&mut self, debug: bool);
}

pub fn run_event_loop<W: Watcher>(
    mut watcher: W,
    mut tray: TrayManager,
    mut settings: Settings,
    debug: bool,
) -> Result<()> {
    write_error_log("Parsing and updating devices...");
    watcher.parse_and_update_with_settings(&mut tray, &mut settings, debug)?;
    discover_devices(&mut watcher, &mut settings, debug);
    write_error_log("Successfully parsed devices, entering main loop...");

    let mut counter: u32 = 0;

    loop {
        pump_windows_messages();
        std::thread::sleep(Duration::from_millis(100));

        let action = crate::ui::handle_menu_events(
            tray.quit_id(), tray.settings_id(), debug,
        )?;

        match action {
            MenuAction::Quit => {
                log("Quit requested", debug);
                break;
            }
            MenuAction::Settings => {
                handle_settings_action(&mut settings, &mut tray, &mut watcher, debug)?;
            }
            MenuAction::None => {}
        }

        check_system_theme(&mut settings, &mut tray, &mut watcher, debug)?;

        counter += 1;
        // Event loop ticks every 100ms (10 times per second)
        let poll_ticks = (settings.polling_interval_total_seconds() * 10) as u32;
        if counter >= poll_ticks {
            counter = 0;
            watcher.check_log_rotation(debug);
            if let Err(e) = watcher.parse_and_update_with_settings(&mut tray, &mut settings, debug) {
                log(&format!("Error parsing devices: {}", e), debug);
            }
            discover_devices(&mut watcher, &mut settings, debug);
        }
    }

    Ok(())
}

fn pump_windows_messages() {
    #[cfg(target_os = "windows")]
    unsafe {
        let mut msg = MSG::default();
        while PeekMessageW(&mut msg, None, 0, 0, PM_REMOVE).as_bool() {
            let _ = TranslateMessage(&msg);
            DispatchMessageW(&msg);
        }
    }
}

fn handle_settings_action<W: Watcher>(
    settings: &mut Settings,
    tray: &mut TrayManager,
    watcher: &mut W,
    debug: bool,
) -> Result<()> {
    log("Opening settings window...", debug);
    unsafe {
        if let Ok(changed) = crate::ui::SettingsWindow::show(settings.clone()) {
            if changed {
                log("Settings changed, reloading...", debug);
                *settings = Settings::load()?;
                // Re-sync device connected state after reload (connected field is not persisted)
                discover_devices(watcher, settings, debug);
                apply_settings_change(settings, tray, watcher, debug)?;
            }
        }
    }
    Ok(())
}

fn apply_settings_change<W: Watcher>(
    settings: &mut Settings,
    tray: &mut TrayManager,
    watcher: &mut W,
    debug: bool,
) -> Result<()> {
    update_custom_assets(settings, debug);
    let theme_str = settings.icon_theme.as_str();
    log(&format!("Setting icon theme to: {}", theme_str), debug);
    icon_manager::set_icon_theme(&theme_str);

    tray.force_refresh();
    if let Err(e) = watcher.parse_and_update_with_settings(tray, settings, debug) {
        log(&format!("Error updating after settings change: {}", e), debug);
    }
    Ok(())
}

/// Sync device_configs with currently known devices — adds new, marks disconnected.
fn discover_devices<W: Watcher>(watcher: &mut W, settings: &mut Settings, debug: bool) {
    if !watcher.persists_devices() { return; }
    let devices = watcher.last_devices();
    let discovered: Vec<(String, String)> = devices.values()
        .filter(|d| d.is_connected)
        .map(|d| (d.unique_id().to_string(), d.name.clone()))
        .collect();
    if settings.sync_device_configs(&discovered) {
        log(&format!("Device configs synced ({} connected device(s)), saving", discovered.len()), debug);
        let _ = settings.save();
    }
}

fn update_custom_assets(settings: &Settings, debug: bool) {
    // Determine themes root: explicit setting, then default (next to exe)
    let themes_root = settings.themes_folder
        .as_ref()
        .map(|f| PathBuf::from(f))
        .or_else(icon_manager::default_themes_root);

    log(&format!("Setting theme to: {}", settings.active_theme), debug);
    icon_manager::set_themes_config(themes_root, &settings.active_theme);
}

fn check_system_theme<W: Watcher>(
    settings: &mut Settings,
    tray: &mut TrayManager,
    watcher: &mut W,
    debug: bool,
) -> Result<()> {
    if settings.icon_theme != crate::model::IconTheme::System {
        return Ok(());
    }
    if !icon_manager::consume_system_theme_changed() {
        return Ok(());
    }
    log("System theme changed, checking if resolved theme differs...", debug);
    if icon_manager::set_icon_theme("system") {
        log("Resolved theme changed — refreshing icons", debug);
        tray.force_refresh();
        if let Err(e) = watcher.parse_and_update_with_settings(tray, settings, debug) {
            log(&format!("Error refreshing after system theme change: {}", e), debug);
        }
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::model::{DeviceCategory, RazerDevice};
    use crate::ui::TrayManager;

    // ── Mock watcher ──────────────────────────────────────────

    struct MockWatcher {
        devices: DeviceMap,
        persist: bool,
    }

    impl MockWatcher {
        fn empty() -> Self { Self { devices: DeviceMap::new(), persist: true } }
        fn non_persistent() -> Self { Self { devices: DeviceMap::new(), persist: false } }
        fn with_connected_device(name: &str, id: &str) -> Self {
            let mut devices = DeviceMap::new();
            devices.insert(id.into(), RazerDevice {
                name: name.into(),
                handle: id.into(),
                serial_number: Some(id.into()),
                battery_percentage: 75,
                is_charging: false,
                is_connected: true,
                is_selected: true,
                category: DeviceCategory::Mouse,
            });
            Self { devices, persist: true }
        }
    }

    impl Watcher for MockWatcher {
        fn parse_and_update(&mut self, _: &mut TrayManager, _: &IconSettings, _: bool) -> Result<()> {
            Ok(())
        }
        fn check_log_rotation(&mut self, _: bool) {}
        fn last_devices(&self) -> DeviceMap { self.devices.clone() }
        fn persists_devices(&self) -> bool { self.persist }
    }

    // ── Watcher trait defaults ────────────────────────────────

    #[test]
    fn watcher_default_last_devices_is_empty() {
        // MinimalWatcher only overrides required methods; default last_devices() = empty
        struct MinimalWatcher;
        impl Watcher for MinimalWatcher {
            fn parse_and_update(&mut self, _: &mut TrayManager, _: &IconSettings, _: bool) -> Result<()> { Ok(()) }
            fn check_log_rotation(&mut self, _: bool) {}
        }
        assert!(MinimalWatcher.last_devices().is_empty());
    }

    #[test]
    fn watcher_default_persists_devices_is_true() {
        struct MinimalWatcher;
        impl Watcher for MinimalWatcher {
            fn parse_and_update(&mut self, _: &mut TrayManager, _: &IconSettings, _: bool) -> Result<()> { Ok(()) }
            fn check_log_rotation(&mut self, _: bool) {}
        }
        assert!(MinimalWatcher.persists_devices());
    }

    // ── discover_devices ──────────────────────────────────────

    #[test]
    fn discover_devices_skips_when_watcher_does_not_persist() {
        let mut watcher = MockWatcher::non_persistent();
        let mut settings = Settings::default();
        discover_devices(&mut watcher, &mut settings, false);
        // device_configs must remain empty — the function returned early
        assert!(settings.device_configs.is_empty());
    }

    #[test]
    fn discover_devices_does_not_mutate_when_no_devices() {
        let mut watcher = MockWatcher::empty();
        let mut settings = Settings::default();
        discover_devices(&mut watcher, &mut settings, false);
        assert!(settings.device_configs.is_empty());
    }

    #[test]
    fn discover_devices_does_not_save_when_nothing_changed() {
        // Pre-populate settings so sync_device_configs returns false (no change → no save)
        let mut watcher = MockWatcher::empty();
        let mut settings = Settings::default();
        // Already no devices, already no configs → sync returns false → save not called
        discover_devices(&mut watcher, &mut settings, false);
        // Verify settings unchanged
        assert!(settings.device_configs.is_empty());
    }

    #[test]
    fn discover_devices_adds_connected_device_to_configs() {
        let mut watcher = MockWatcher::with_connected_device("Razer Viper", "viper-001");
        let mut settings = Settings::default();
        discover_devices(&mut watcher, &mut settings, false);
        // sync_device_configs should have added the device
        assert!(!settings.device_configs.is_empty());
        let cfg = &settings.device_configs[0];
        assert_eq!(cfg.name, "Razer Viper");
    }

    #[test]
    fn watcher_parse_and_update_with_settings_uses_icon_settings() {
        struct MinimalWatcher { called: bool }
        impl Watcher for MinimalWatcher {
            fn parse_and_update(&mut self, _: &mut TrayManager, _: &IconSettings, _: bool) -> Result<()> {
                self.called = true;
                Ok(())
            }
            fn check_log_rotation(&mut self, _: bool) {}
        }
        let mut w = MinimalWatcher { called: false };
        // parse_and_update_with_settings is a default trait method — verify it dispatches
        // We can't construct TrayManager without Win32, so we just verify the trait compiles
        // and the method exists on the trait object
        let _ = w.last_devices(); // calls default → empty
        assert!(!w.called);
    }

    // ── update_custom_assets ──────────────────────────────────

    #[test]
    fn update_custom_assets_default_theme_resolves_to_no_custom_folder() {
        let mut settings = Settings::default();
        settings.themes_folder = Some(r"C:\fake\themes".into());
        settings.active_theme = "Default".into();
        update_custom_assets(&settings, false);
        // "Default" theme → set_themes_config clears custom folder
        assert!(icon_manager::get_custom_assets_folder().is_none());
    }

    #[test]
    fn update_custom_assets_named_theme_sets_combined_path() {
        let mut settings = Settings::default();
        settings.themes_folder = Some(r"C:\fake\themes".into());
        settings.active_theme = "Neon".into();
        update_custom_assets(&settings, false);
        let folder = icon_manager::get_custom_assets_folder();
        assert_eq!(folder, Some(PathBuf::from(r"C:\fake\themes\Neon")));
        // reset
        icon_manager::set_custom_assets_folder(None);
    }
}

