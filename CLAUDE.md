# CLAUDE.md

This file provides guidance to Claude Code (claude.ai/code) when working with code in this repository.

## Project Overview

**Razer Taskbar** is a Windows-only Rust application that displays Razer wireless device battery status in the system tray by parsing Razer Synapse log files — no Razer SDK required.

## Build & Run Commands

```powershell
# Development build (enables debug console, --emulate flag, debug_assertions)
cargo build

# Release build (optimized for size, strips symbols)
cargo build --release

# Run debug build with console output
cargo run -- --debug

# Run in emulation mode (4 fake devices, no Synapse required — debug builds only)
cargo run -- --emulate

# Both flags together
cargo run -- --debug --emulate
```

The compiled executable is at `target\release\razer-taskbar.exe`.

**Note:** `--emulate` is compiled out in release builds (`#[cfg(debug_assertions)]`). The release binary shows no console window (`windows_subsystem = "windows"`); pass `--debug` to allocate one.

## Testing

```powershell
cargo test
```

Tests live in `tests/` (`settings_tests.rs`, `v3_parsing_tests.rs`, `v4_parsing_tests.rs`) plus `#[cfg(test)]` unit tests inline in individual modules (e.g. `watcher_common.rs`, `logging.rs`). `main.rs` is a thin binary entrypoint; `lib.rs` re-exports `engine`, `model`, `ui`, `util` as a library crate so integration tests and the binary share the same code.

## Release Pipeline

Pushing a `v*` tag triggers `.github/workflows/release.yml`, which builds the release binary on `windows-latest`, then packages it with Inno Setup (`installer.iss`) into `razer-taskbar-<version>-setup.exe` and attaches it to the GitHub release. The installer is a per-user install (no UAC), with optional desktop icon and run-at-startup tasks.

## Architecture

The app is structured as four modules wired together in `main.rs`:

```
main.rs → loads Settings → inits assets/theme → detects Synapse version → enters run_event_loop()
```

### `engine/` — Core Logic

- **`event_loop.rs`**: Contains the `Watcher` trait (implemented by all three watchers) and `run_event_loop<W: Watcher>()`. The loop ticks every 100ms, pumps Windows messages, handles menu events, and polls log files on a configurable interval (`polling_interval_minutes` + `polling_interval_seconds`, minimum 1 second total).
- **`watcher_common.rs`**: Shared `log_devices()` helper used by all three watchers to print/log discovered device state.
- **`watcher_v3.rs`**: Reads `%LOCALAPPDATA%\Razer\Synapse3\Log\Razer Synapse 3.log` with multi-line regex to extract `_OnBatteryLevelChanged` events.
- **`watcher_v4.rs`**: Reads `%LOCALAPPDATA%\Razer\RazerAppEngine\User Data\Logs\systray_systrayv2*.log`, parses `connectingDeviceData` JSON entries, handles log rotation.
- **`watcher_emulated.rs`**: Returns hard-coded fake devices that cycle battery values; does not persist to `device_configs` (`persists_devices()` returns `false`).
- **`icon_manager/`**: Loads PNG icons (embedded via `build.rs`-generated code or from custom theme folders), composites charging/device-type overlays, renders optional percentage text via `ab_glyph`. Supports theming system with `set_themes_config()`, `scan_themes()`, and `default_themes_root()`.

### `model/` — Data Structures (no logic)

- **`device.rs`**: `RazerDevice` (name, handle/serial, battery %, charging, connected, category), `DeviceCategory`, `DeviceMap` (`HashMap<String, RazerDevice>`), `DeviceConfig` (id, name, visible, connected).
- **`settings.rs`**: `Settings` — JSON-persisted to `%LOCALAPPDATA%\razer-taskbar\settings.json`. Handles `load()`, `save()`, `sync_device_configs()`, and `polling_interval_total_seconds()`. Key fields: `hidden_device_handles: Vec<String>` (exclusion list), `device_configs: Vec<DeviceConfig>` (auto-discovered per-device state), `polling_interval_minutes` + `polling_interval_seconds` (minimum 1 second total), `themes_folder` (root for custom themes), `active_theme` (selected theme name), `icon_theme` (dark/light/system), `synapse_version`.
- **`icon_settings.rs`**: `IconSettings` — derived from `Settings` for icon generation (text overlay config).
- **`v4_log_types.rs`**: `LoggedDeviceInfo`, `PowerStatus`, `ChargingStatus` — serde types for V4 JSON parsing.

### `ui/` — User Interface

- **`tray_manager.rs`**: `TrayManager` — maintains a `HashMap<String, TrayIcon>` keyed by device handle. Creates/removes one tray icon per visible connected device dynamically. Falls back to a single "no device" icon when none are active. Text constants (menu labels, tooltip strings) live in `constants.rs`.
- **`context_menu.rs`**: `NativeContextMenu` — raw Win32 popup menu (hidden message-only window + `TrackPopupMenu`) used for the tray icon's right-click menu.
- **`menu_events.rs`**: `MenuAction` enum; reads tray menu events via a global channel.
- **`settings/`**: Native Win32 settings dialog with three tabs (General, Text, Devices). `SettingsWindow::show()` is `unsafe` because it creates a Win32 window. `state.rs` tracks change detection; tab files (`general_tab.rs`, `text_tab.rs`, `devices_tab.rs`) each create their tab controls.

### `util/` — Cross-cutting Utilities

- **`logging.rs`**: `log()` (console, debug-only), `write_error_log()` (appends to `%LOCALAPPDATA%\razer_taskbar_errors.log`), `log_memory_usage()` (Windows-only; logs process private/working-set memory via `GetProcessMemoryInfo`, no-op on other targets).
- **`startup.rs`**: Creates/removes a `.lnk` shortcut in the Windows Startup folder for run-at-startup.
- **`utils.rs`**: `parse_hex_color()`, `to_wide()` (Rust `&str` → `Vec<u16>` for Win32 APIs).

## Key Design Patterns

**Watcher trait polymorphism**: `run_event_loop<W: Watcher>()` is generic — all three watchers (V3, V4, Emulation) implement the same `Watcher` trait, so the event loop doesn't branch on version.

**Build-time asset embedding**: `build.rs` reads `src/assets/icon.properties`, discovers all referenced PNG filenames, and generates `embedded_assets.rs` in `OUT_DIR`. This file produces `get_embedded_assets() -> HashMap<&'static str, &'static [u8]>` using `include_bytes!()`. The `icon_manager` calls this at runtime.

**Theming system**: The app supports custom themes via a two-level structure: `themes_folder` (root directory containing theme subfolders) + `active_theme` (selected theme name). By default, scans `themes/` folder next to the executable. Each theme folder must contain `dark/` and/or `light/` subdirectories with PNG assets, plus an optional `icon.properties` file. The "Default" theme uses built-in embedded assets. Theme selection is done via a combobox in the General settings tab, and `set_themes_config(themes_root, active_theme)` resolves the final asset path.

**Asset resolution order** (first match wins): resolved theme folder (`themes_root/active_theme`) + theme (dark/light) → embedded theme → embedded opposite theme.

**Polling interval**: Configurable via two inputs (minutes + seconds) in the General settings tab. Total interval is calculated as `minutes * 60 + seconds` with a minimum of 1 second enforced. The event loop ticks every 100ms and polls log files when `counter >= polling_interval_total_seconds() * 10`.

**Device visibility**: `settings.hidden_device_handles` is an exclusion list (empty = show all). `settings.device_configs` is auto-populated by `discover_devices()` each poll cycle and persisted so the Devices tab can show previously seen devices.

**Settings flow**: Settings window closes → `Settings::load()` re-reads from disk → `discover_devices()` re-syncs connected state (not persisted) → `apply_settings_change()` calls `set_themes_config()` with `themes_folder` + `active_theme`, updates icon theme (dark/light/system), and forces tray refresh.

**`shown_device_handle`** in `settings.rs` is a legacy field kept for JSON backwards compatibility; device visibility is now controlled by `device_configs[n].visible` and `hidden_device_handles`.

**`custom_assets_folder`** in `settings.rs` is a legacy field kept for JSON backwards compatibility; theming is now controlled by `themes_folder` (root directory) + `active_theme` (selected theme name).
