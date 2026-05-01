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
#[cfg(debug_assertions)]
use engine::EmulationWatcher;

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
    #[cfg(debug_assertions)]
    let emulate = args.iter().any(|a| a == "--emulate" || a == "-e");

    #[cfg(target_os = "windows")]
    {
        #[cfg(debug_assertions)]
        let show_console = debug || emulate;
        #[cfg(not(debug_assertions))]
        let show_console = debug;

        if show_console {
            unsafe { let _ = AllocConsole(); }
            std::thread::sleep(Duration::from_millis(100));
            #[cfg(debug_assertions)]
            if emulate {
                println!("=== Razer Taskbar - Emulation Mode ===");
                println!("Using fake devices (no Razer Synapse required)");
            } else {
                println!("=== Razer Taskbar - Debug Mode ===");
            }
            #[cfg(not(debug_assertions))]
            println!("=== Razer Taskbar - Debug Mode ===");
        }
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

    #[cfg(debug_assertions)]
    if emulate {
        log("Starting emulation watcher with fake devices", debug);
        write_error_log("Using emulation mode");
        let watcher = EmulationWatcher::new();
        engine::run_event_loop(watcher, tray, settings, debug)?;
    } else {
        let version = detect_synapse_version(&settings);
        log(&format!("Using Synapse version: {:?}", version), debug);
        write_error_log(&format!("Using Synapse version: {:?}", version));
        start_watcher(version, tray, settings, debug)?;
    }

    #[cfg(not(debug_assertions))]
    {
        let version = detect_synapse_version(&settings);
        log(&format!("Using Synapse version: {:?}", version), debug);
        write_error_log(&format!("Using Synapse version: {:?}", version));
        start_watcher(version, tray, settings, debug)?;
    }
    write_error_log("Application exiting normally");
    Ok(())
}

fn init_assets_and_theme(settings: &Settings, debug: bool) {
    // Determine themes root: explicit setting, then default (next to exe)
    let themes_root = settings.themes_folder
        .as_ref()
        .map(|f| PathBuf::from(f))
        .or_else(engine::default_themes_root);

    engine::set_themes_config(themes_root, &settings.active_theme);
    log(&format!("Theme set to: {}", settings.active_theme), debug);

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
            watcher.init(&settings.device_configs)?;
            engine::run_event_loop(watcher, tray, settings, debug)
        }
        _ => unreachable!(),
    }
}
