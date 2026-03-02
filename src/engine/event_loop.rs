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
        let poll_ticks = (settings.polling_interval_minutes * 60 * 10) as u32;
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
    if let Some(ref folder_path) = settings.custom_assets_folder {
        let path = PathBuf::from(folder_path);
        log(&format!("Setting custom assets folder to: {}", folder_path), debug);
        icon_manager::set_custom_assets_folder(Some(path));
    } else {
        log("Clearing custom assets folder (reverting to embedded)", debug);
        icon_manager::set_custom_assets_folder(None);
    }
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

