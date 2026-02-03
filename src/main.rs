#![cfg_attr(not(debug_assertions), windows_subsystem = "windows")]

mod device;
mod native_menu;
mod resources;
mod settings;
mod settings_window;
mod startup;
mod tray_manager;
mod ui_constants;
mod watcher_v3;
mod watcher_v4;

use anyhow::Result;
use device::DeviceMap;
use notify::RecursiveMode;
use notify_debouncer_full::{new_debouncer, DebounceEventResult};
use settings::{Settings, SynapseVersion};
use std::path::PathBuf;
use std::sync::{Arc, Mutex};
use std::time::Duration;
use tray_manager::TrayManager;
use watcher_v3::SynapseV3Watcher;
use watcher_v4::SynapseV4Watcher;
use std::fs::OpenOptions;
use std::io::Write;
#[cfg(target_os = "windows")]
use windows::Win32::{
    System::Console::AllocConsole,
    UI::WindowsAndMessaging::{DispatchMessageW, PeekMessageW, TranslateMessage, MSG, PM_REMOVE},
};

fn write_error_log(msg: &str) {
    // Write to temp directory so it works regardless of working directory
    if let Some(mut log_path) = std::env::temp_dir().parent().map(|p| p.to_path_buf()) {
        log_path.push("razer_taskbar_errors.log");
        if let Ok(mut file) = OpenOptions::new()
            .create(true)
            .append(true)
            .open(&log_path)
        {
            let _ = writeln!(file, "[{}] {}", chrono::Local::now().format("%Y-%m-%d %H:%M:%S"), msg);
        }
    }
}

fn main() -> Result<()> {
    // Set panic hook to log panics
    std::panic::set_hook(Box::new(|panic_info| {
        let msg = format!("Panic occurred: {:?}", panic_info);
        write_error_log(&msg);
    }));

    match run_app() {
        Ok(_) => Ok(()),
        Err(e) => {
            let msg = format!("Application error: {:?}", e);
            write_error_log(&msg);
            Err(e)
        }
    }
}

fn run_app() -> Result<()> {
    let args: Vec<String> = std::env::args().collect();
    let debug_mode = args.iter().any(|a| a == "--debug" || a == "-d");

    #[cfg(target_os = "windows")]
    if debug_mode {
        unsafe {
            let _ = AllocConsole();
            // Give the console time to initialize
            std::thread::sleep(Duration::from_millis(100));
        }
        println!("=== Razer Taskbar - Debug Mode ===");
    }

    write_error_log("Application starting...");
    log("Razer Taskbar - Starting...", debug_mode);

    write_error_log("Loading settings...");
    let settings = Settings::load()?;
    log(&format!("Settings loaded: {:?}", settings), debug_mode);

    // Initialize custom assets folder if set
    if let Some(ref folder_path) = settings.custom_assets_folder {
        let path = PathBuf::from(folder_path);
        resources::set_custom_assets_folder(Some(path));
        log(&format!("Custom assets folder set to: {}", folder_path), debug_mode);
    }

    write_error_log("Applying autostart settings...");
    // Apply autostart setting (sync registry with settings)
    if let Err(e) = startup::set_startup(settings.run_at_startup) {
        log(&format!("Failed to apply autostart setting: {}", e), debug_mode);
    }

    write_error_log("Creating tray manager...");
    let mut tray_manager = TrayManager::new()?;

    write_error_log("Initializing tray manager...");
    tray_manager.initialize()?;
    log("Tray icon initialized", debug_mode);

    write_error_log("Determining Synapse version...");
    let synapse_version = match settings.synapse_version {
        SynapseVersion::V3 => SynapseVersion::V3,
        SynapseVersion::V4 => SynapseVersion::V4,
        SynapseVersion::Auto => {
            if SynapseV4Watcher::new().is_some() { SynapseVersion::V4 } else { SynapseVersion::V3 }
        }
    };
    log(&format!("Using Synapse version: {:?}", synapse_version), debug_mode);
    write_error_log(&format!("Using Synapse version: {:?}", synapse_version));

    write_error_log("Starting watcher...");
    match synapse_version {
        SynapseVersion::V3 => run_v3_watcher(tray_manager, settings, debug_mode)?,
        SynapseVersion::V4 => run_v4_watcher(tray_manager, settings, debug_mode)?,
        _ => unreachable!(),
    }

    write_error_log("Application exiting normally");
    Ok(())
}

fn log(msg: &str, debug: bool) {
    if debug { println!("{}", msg); }
}

fn run_v3_watcher(mut tray_manager: TrayManager, mut settings: Settings, debug: bool) -> Result<()> {
    let watcher = SynapseV3Watcher::new().ok_or_else(|| anyhow::anyhow!("Synapse V3 log file not found"))?;
    let log_path = watcher.log_path().clone();
    log(&format!("Watching Synapse V3 log: {:?}", log_path), debug);

    let devices = Arc::new(Mutex::new(DeviceMap::new()));
    let _devices_clone = devices.clone();

    parse_and_update_v3(&watcher, &mut tray_manager, &settings, debug)?;

    let throttle_duration = Duration::from_secs(settings.polling_interval_minutes * 60);
    let mut debouncer = new_debouncer(
        throttle_duration,
        None,
        move |result: DebounceEventResult| {
            match result {
                Ok(_events) => {},
                Err(e) => {
                    if debug { println!("Watch error: {:?}", e); }
                }
            }
        },
    )?;

    // In notify 8.x, call watch directly on debouncer
    debouncer.watch(&log_path, RecursiveMode::NonRecursive)?;

    loop {
        #[cfg(target_os = "windows")]
        {
            unsafe {
                let mut msg = MSG::default();
                while PeekMessageW(&mut msg, None, 0, 0, PM_REMOVE).as_bool() {
                    let _ = TranslateMessage(&msg);
                    DispatchMessageW(&msg);
                }
            }
        }

        std::thread::sleep(Duration::from_millis(100));

        match tray_manager::handle_menu_events(tray_manager.quit_id(), tray_manager.settings_id(), debug)? {
            tray_manager::MenuAction::Quit => {
                log("Quit requested", debug);
                break;
            }
            tray_manager::MenuAction::Settings => {
                log("Opening settings window...", debug);
                unsafe {
                    if let Ok(changed) = settings_window::SettingsWindow::show(settings.clone()) {
                        if changed {
                            log("Settings changed, reloading...", debug);
                            settings = Settings::load()?;
                            
                            // Update custom assets folder
                            if let Some(ref folder_path) = settings.custom_assets_folder {
                                let path = PathBuf::from(folder_path);
                                resources::set_custom_assets_folder(Some(path));
                            } else {
                                resources::set_custom_assets_folder(None);
                            }
                            
                            // Force update to apply new icon style
                            if let Err(e) = parse_and_update_v3(&watcher, &mut tray_manager, &settings, debug) {
                                log(&format!("Error updating after settings change: {}", e), debug);
                            }
                        }
                    }
                }
            }
            tray_manager::MenuAction::None => {}
        }

        static mut COUNTER: u32 = 0;
        unsafe {
            COUNTER += 1;
            if COUNTER >= (settings.polling_interval_minutes * 60 * 10) as u32 {
                COUNTER = 0;
                if let Err(e) = parse_and_update_v3(&watcher, &mut tray_manager, &settings, debug) {
                    log(&format!("Error parsing devices: {}", e), debug);
                }
            }
        }
    }

    Ok(())
}

fn run_v4_watcher(mut tray_manager: TrayManager, mut settings: Settings, debug: bool) -> Result<()> {
    write_error_log("Creating V4 watcher...");
    let mut watcher = SynapseV4Watcher::new().ok_or_else(|| anyhow::anyhow!("Synapse V4 log directory not found"))?;

    write_error_log("Getting log directory...");
    let log_dir = watcher.log_dir().clone();
    log(&format!("Watching Synapse V4 logs in: {:?}", log_dir), debug);
    write_error_log(&format!("Watching Synapse V4 logs in: {:?}", log_dir));

    write_error_log("Finding latest log file...");
    let mut log_path = watcher.find_latest_log_file().ok_or_else(|| anyhow::anyhow!("No Synapse V4 log files found"))?;
    log(&format!("Using log file: {:?}", log_path), debug);
    write_error_log(&format!("Using log file: {:?}", log_path));

    write_error_log("Parsing and updating devices...");
    parse_and_update_v4(&mut watcher, &log_path, &mut tray_manager, &settings, debug)?;
    write_error_log("Successfully parsed devices, entering main loop...");

    let mut counter: u32 = 0;

    loop {
        #[cfg(target_os = "windows")]
        {
            unsafe {
                let mut msg = MSG::default();
                while PeekMessageW(&mut msg, None, 0, 0, PM_REMOVE).as_bool() {
                    let _ = TranslateMessage(&msg);
                    DispatchMessageW(&msg);
                }
            }
        }

        std::thread::sleep(Duration::from_millis(100));

        match tray_manager::handle_menu_events(tray_manager.quit_id(), tray_manager.settings_id(), debug)? {
            tray_manager::MenuAction::Quit => {
                log("Quit requested", debug);
                break;
            }
            tray_manager::MenuAction::Settings => {
                log("Opening settings window...", debug);
                unsafe {
                    if let Ok(changed) = settings_window::SettingsWindow::show(settings.clone()) {
                        if changed {
                            log("Settings changed, reloading...", debug);
                            settings = Settings::load()?;
                            
                            // Update custom assets folder
                            if let Some(ref folder_path) = settings.custom_assets_folder {
                                let path = PathBuf::from(folder_path);
                                resources::set_custom_assets_folder(Some(path));
                            } else {
                                resources::set_custom_assets_folder(None);
                            }
                            
                            // Force update to apply new icon style
                            if let Err(e) = parse_and_update_v4(&mut watcher, &log_path, &mut tray_manager, &settings, debug) {
                                log(&format!("Error updating after settings change: {}", e), debug);
                            }
                        }
                    }
                }
            }
            tray_manager::MenuAction::None => {}
        }

        counter += 1;
        if counter >= (settings.polling_interval_minutes * 60 * 10) as u32 {
            counter = 0;

            if let Some(new_log_path) = watcher.find_latest_log_file() {
                if new_log_path != log_path {
                    log(&format!("Log file changed to: {:?}", new_log_path), debug);
                    log_path = new_log_path;
                }
            }

            if let Err(e) = parse_and_update_v4(&mut watcher, &log_path, &mut tray_manager, &settings, debug) {
                log(&format!("Error parsing devices: {}", e), debug);
            }
        }
    }

    Ok(())
}

fn parse_and_update_v3(
    watcher: &SynapseV3Watcher,
    tray_manager: &mut TrayManager,
    settings: &Settings,
    debug: bool,
) -> Result<()> {
    let devices = watcher.parse_devices(&settings.shown_device_handle)?;

    if !devices.is_empty() {
        log(&format!("Found {} devices:", devices.len()), debug);
        for device in devices.values() {
            log(
                &format!(
                    "  - {} ({}%{}){}",
                    device.name,
                    device.battery_percentage,
                    if device.is_charging { " charging" } else { "" },
                    if !device.is_connected { " [disconnected]" } else { "" }
                ),
                debug,
            );
        }
    }

    tray_manager.update_devices(
        devices,
        settings.show_percentage,
        settings.display_charging_state,
    )?;

    Ok(())
}

fn parse_and_update_v4(
    watcher: &mut SynapseV4Watcher,
    log_path: &PathBuf,
    tray_manager: &mut TrayManager,
    settings: &Settings,
    debug: bool,
) -> Result<()> {
    let devices = watcher.parse_devices(log_path, &settings.shown_device_handle, debug)?;

    if !devices.is_empty() {
        log(&format!("Found {} devices:", devices.len()), debug);
        for device in devices.values() {
            log(
                &format!(
                    "  - {} ({}%{}){}",
                    device.name,
                    device.battery_percentage,
                    if device.is_charging { " charging" } else { "" },
                    if !device.is_connected { " [disconnected]" } else { "" }
                ),
                debug,
            );
        }
    }

    tray_manager.update_devices(
        devices,
        settings.show_percentage,
        settings.display_charging_state,
    )?;

    Ok(())
}
