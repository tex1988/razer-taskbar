#![cfg_attr(not(debug_assertions), windows_subsystem = "windows")]

mod engine;
mod model;
mod ui;
mod util;

use anyhow::Result;
use util::{log, write_error_log};
use model::{Settings, SynapseVersion};
use std::path::PathBuf;
use std::time::Duration;
use ui::TrayManager;
use engine::{SynapseV3Watcher, SynapseV4Watcher};

#[cfg(target_os = "windows")]
use windows::Win32::System::Console::AllocConsole;

fn main() -> Result<()> {
    std::panic::set_hook(Box::new(|panic_info| {
        write_error_log(&format!("Panic occurred: {:?}", panic_info));
    }));

    match run_app() {
        Ok(_) => Ok(()),
        Err(e) => {
            write_error_log(&format!("Application error: {:?}", e));
            Err(e)
        }
    }
}

fn run_app() -> Result<()> {
    let args: Vec<String> = std::env::args().collect();
    let debug = args.iter().any(|a| a == "--debug" || a == "-d");

    #[cfg(target_os = "windows")]
    if debug {
        unsafe { let _ = AllocConsole(); }
        std::thread::sleep(Duration::from_millis(100));
        println!("=== Razer Taskbar - Debug Mode ===");
    }

    write_error_log("Application starting...");
    let settings = Settings::load()?;
    log(&format!("Settings loaded: {:?}", settings), debug);

    init_assets_and_theme(&settings, debug);
    engine::create_theme_change_listener();

    if let Err(e) = util::startup::set_startup(settings.run_at_startup) {
        log(&format!("Failed to apply autostart setting: {}", e), debug);
    }

    let mut tray = TrayManager::new()?;
    tray.initialize()?;
    log("Tray icon initialized", debug);

    let version = detect_synapse_version(&settings);
    log(&format!("Using Synapse version: {:?}", version), debug);
    write_error_log(&format!("Using Synapse version: {:?}", version));

    start_watcher(version, tray, settings, debug)?;
    write_error_log("Application exiting normally");
    Ok(())
}

fn init_assets_and_theme(settings: &Settings, debug: bool) {
    if let Some(ref fp) = settings.custom_assets_folder {
        engine::set_custom_assets_folder(Some(PathBuf::from(fp)));
        log(&format!("Custom assets folder set to: {}", fp), debug);
    } else {
        engine::set_custom_assets_folder(None);
    }
    let theme = settings.icon_theme.as_str();
    engine::set_icon_theme(&theme);
    log(&format!("Icon theme set to: {}", theme), debug);
}

fn detect_synapse_version(settings: &Settings) -> SynapseVersion {
    match settings.synapse_version {
        SynapseVersion::V3 => SynapseVersion::V3,
        SynapseVersion::V4 => SynapseVersion::V4,
        SynapseVersion::Auto => {
            if SynapseV4Watcher::new().is_some() {
                SynapseVersion::V4
            } else {
                SynapseVersion::V3
            }
        }
    }
}

fn start_watcher(
    version: SynapseVersion,
    tray: TrayManager,
    settings: Settings,
    debug: bool,
) -> Result<()> {
    match version {
        SynapseVersion::V3 => {
            let watcher = SynapseV3Watcher::new()
                .ok_or_else(|| anyhow::anyhow!("Synapse V3 log not found"))?;
            engine::run_event_loop(watcher, tray, settings, debug)
        }
        SynapseVersion::V4 => {
            let mut watcher = SynapseV4Watcher::new()
                .ok_or_else(|| anyhow::anyhow!("Synapse V4 log dir not found"))?;
            watcher.init(&settings.shown_device_handle)?;
            engine::run_event_loop(watcher, tray, settings, debug)
        }
        _ => unreachable!(),
    }
}
