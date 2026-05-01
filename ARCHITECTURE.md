# Razer Taskbar Architecture Diagram

## High-Level Flow

```
┌─────────────────────────────────────────────────────────────────┐
│                     Razer Wireless Device                        │
│                  (e.g., Razer Viper Ultimate)                    │
└────────────────────────────┬────────────────────────────────────┘
                             │ USB Dongle / Bluetooth
                             ▼
┌─────────────────────────────────────────────────────────────────┐
│                     Razer Synapse 3/4                            │
│  - Communicates with device via USB/Bluetooth                    │
│  - Receives battery status updates                               │
│  - Writes log files with device info                             │
└────────────────────────────┬────────────────────────────────────┘
                             │ Writes to log files
                             ▼
┌─────────────────────────────────────────────────────────────────┐
│                         Log Files                                │
│                                                                   │
│  V3: %LOCALAPPDATA%\Razer\Synapse3\Log\                          │
│      └── Razer Synapse 3.log                                     │
│                                                                   │
│  V4: %LOCALAPPDATA%\Razer\RazerAppEngine\User Data\Logs\         │
│      └── systray_systrayv2*.log                                  │
└────────────────────────────┬────────────────────────────────────┘
                             │ Our app reads these
                             ▼
┌─────────────────────────────────────────────────────────────────┐
│                   Razer Taskbar (Rust)                           │
│                                                                   │
│  ┌────────────────────┐  ┌──────────────────────────┐           │
│  │  Event Loop        │  │  Log Parsers / Emulation │           │
│  │  (engine)          │──│  - watcher_v3            │           │
│  │                    │  │  - watcher_v4            │           │
│  │  Polls & dispatches│  │  - watcher_emulated      │           │
│  └────────────────────┘  └─────────┬────────────────┘           │
│                                    │                             │
│                                    ▼                             │
│                          ┌──────────────────┐                    │
│                          │  Model           │                    │
│                          │  (model)         │                    │
│                          │                  │                    │
│                          │  - Device data   │                    │
│                          │  - Settings      │                    │
│                          │  - Icon config   │                    │
│                          └────────┬─────────┘                    │
│                                   │                              │
│                                   ▼                              │
│                          ┌──────────────────┐                    │
│                          │  UI Layer        │                    │
│                          │  (ui)            │                    │
│                          │                  │                    │
│                          │  - Tray manager  │                    │
│                          │  - Settings GUI  │                    │
│                          │  - Menu events   │                    │
│                          └────────┬─────────┘                    │
└─────────────────────────────────┼──────────────────────────────┘
                                  │
                                  ▼
                    ┌──────────────────────────────────┐
                    │  Windows System Tray             │
                    │  [Mouse 25%] [Keyboard 80%] ...  │
                    └──────────────────────────────────┘
```

## Data Flow - Synapse V3

```
┌──────────────────────────────────────────────────────────────────┐
│ Razer Synapse 3.log                                              │
├──────────────────────────────────────────────────────────────────┤
│ 2024-01-30 14:23:45 INFO _OnBatteryLevelChanged                  │
│   Device Information:                                            │
│     Name: Razer Viper Ultimate                                   │
│     Handle: 12345                                                │
│   Battery Information:                                           │
│     level 87 state 0                                             │
└────────────────────────┬─────────────────────────────────────────┘
                         │
                         ▼ engine::watcher_v3
        ┌────────────────────────────────────────┐
        │ Regex Pattern Match                    │
        ├────────────────────────────────────────┤
        │ (?P<name>.*) → "Razer Viper Ultimate"  │
        │ (?P<handle>\d+) → "12345"              │
        │ (?P<level>\d+) → "87"                  │
        │ (?P<isCharging>\d+) → "0"              │
        └────────────────┬───────────────────────┘
                         │
                         ▼
                ┌────────────────────┐
                │ model::RazerDevice │
                ├────────────────────┤
                │ name: "Razer..."   │
                │ handle: "12345"    │
                │ battery: 87        │
                │ charging: false    │
                │ connected: true    │
                └────────────────────┘
```

## Data Flow - Synapse V4

```
┌──────────────────────────────────────────────────────────────────┐
│ systray_systrayv2.log                                            │
├──────────────────────────────────────────────────────────────────┤
│ [2024-01-30T14:23:45] connectingDeviceData:                      │
│ [{                                                               │
│   "serialNumber": "AB123",                                       │
│   "hasBattery": true,                                            │
│   "powerStatus": {                                               │
│     "level": 87,                                                 │
│     "chargingStatus": "NoCharge_BatteryFull"                     │
│   },                                                             │
│   "name": {"en": "Razer Viper Ultimate"}                         │
│ }]                                                               │
└────────────────────────┬─────────────────────────────────────────┘
                         │
                         ▼ engine::watcher_v4
        ┌────────────────────────────────────────┐
        │ JSON Deserialization                   │
        ├────────────────────────────────────────┤
        │ model::LoggedDeviceInfo {             │
        │   serial_number: "AB123",              │
        │   has_battery: true,                   │
        │   power_status: {                      │
        │     level: 87,                         │
        │     charging_status: NoCharge          │
        │   },                                   │
        │   name: { en: "Razer Viper Ultimate" } │
        │ }                                      │
        └────────────────┬───────────────────────┘
                         │
                         ▼
                ┌────────────────────┐
                │ model::RazerDevice │
                ├────────────────────┤
                │ name: "Razer..."   │
                │ handle: "AB123"    │
                │ battery: 87        │
                │ charging: false    │
                │ connected: true    │
                └────────────────────┘
```

## Data Flow - Emulation Mode

```
┌──────────────────────────────────────────────────────────────────┐
│ engine::watcher_emulated  (--emulate / -e flag)                  │
├──────────────────────────────────────────────────────────────────┤
│  Hard-coded EMULATED_DEVICES list (4 fake devices)               │
│  Each tick: battery % increments/decrements to simulate drain    │
└────────────────────────┬─────────────────────────────────────────┘
                         │
                         ▼ same Watcher trait
                ┌────────────────────┐
                │ model::RazerDevice │
                │ (same as V3/V4)    │
                └────────────────────┘
```

## Event Loop Flow

```
┌─────────────────────────────────────────────────────────────┐
│ main.rs → run_app()                                         │
└─────┬───────────────────────────────────────────────────────┘
      │
      ├─► Step 1: Load settings (model::Settings)
      │
      ├─► Step 2: Init assets & theme (engine::icon_manager)
      │   ├─ Load themes_folder + active_theme from settings
      │   ├─ Call set_themes_config(themes_root, active_theme)
      │   └─ Resolve theme assets path or use embedded defaults
      │
      ├─► Step 3: Detect watcher
      │   ├─ --emulate flag? → EmulationWatcher (no Synapse needed)
      │   ├─ Check for V4 logs → exists? → SynapseV4Watcher
      │   └─ Otherwise → SynapseV3Watcher
      │
      ├─► Step 4: Create tray icon (ui::TrayManager)
      │
      └─► Step 5: Enter event loop (engine::event_loop)
          │
          ┌───────────────────────────────────────┐
          │ Every 100ms tick:                      │
          ├───────────────────────────────────────┤
          │ 1. Pump Windows messages               │
          │                                        │
          │ 2. Check menu events                   │
          │    ├─ Quit → Exit                      │
          │    └─ Settings → Open settings window  │
          │                                        │
          │ 3. Check system theme changes          │
          │    └─ Theme changed? Refresh icons     │
          │                                        │
          │ 4. Poll log file (every N seconds)     │
          │    ├─ Counter >= poll_interval_total?  │
          │    ├─ Check log rotation (V4)          │
          │    ├─ Parse devices                    │
          │    └─ Update tray icons (per device)   │
          └───────────────────────────────────────┘
```

## Multi-Device Tray Icon Logic

```
Devices returned by watcher:
┌─────────────────────────────────────────────────────────────┐
│ Device A: Razer Viper   (25%, not charging, is_selected)    │
│ Device B: Razer Kraken  (80%, charging,     is_selected)    │
│ Device C: Razer Mamba   (15%, not charging, NOT selected)   │
└─────────────────────────────────────────────────────────────┘
                           │
                           ▼ filter(is_connected && is_selected), sort by handle
                           │
┌─────────────────────────────────────────────────────────────┐
│ Active devices: [Device A, Device B]                        │
│ One tray icon created/updated per active device.            │
│ Device C is hidden (in settings.hidden_device_handles).     │
└─────────────────────────────────────────────────────────────┘
                           │
              ┌────────────┴─────────────┐
              ▼                          ▼
   ┌──────────────────┐      ┌──────────────────┐
   │ TrayIcon A       │      │ TrayIcon B       │
   │ Razer Viper 25%  │      │ Razer Kraken 80% │
   └──────────────────┘      └──────────────────┘

No active devices → single fallback icon shown (unknown/no-device).
```

## Device Visibility — Settings Model

```
settings.hidden_device_handles: Vec<String>
  │
  ├─ Empty → all connected devices are shown (default)
  └─ Contains handle → that device's tray icon is suppressed

Previously (dev branch): shown_device_handle: String  (single device)
Now (multi_device branch): hidden_device_handles: Vec<String>  (exclusion list)
```

## Icon Generation Flow

```
Battery Percentage: 87%
        │
        ▼
┌─────────────────────────┐
│ icon.properties lookup  │
│ 81-100 → 100.png        │
└────────┬────────────────┘
         │
         ▼
┌─────────────────────────┐
│ Load base icon from     │
│ theme (dark/light/       │
│ system/custom)          │
└────────┬────────────────┘
         │
         ▼
┌─────────────────────────┐     ┌──────────────────┐
│ Is Charging?            │ ──► │ Yes: Overlay      │
│                         │     │ charging.png      │
└────────┬────────────────┘     └──────────────────┘
         │
         ▼
┌─────────────────────────┐     ┌──────────────────┐
│ Show device overlay?    │ ──► │ Yes: Overlay      │
│                         │     │ mouse/keyboard/   │
│                         │     │ headphones.png    │
└────────┬────────────────┘     └──────────────────┘
         │
         ▼
┌─────────────────────────┐     ┌──────────────────┐
│ Show percentage text?   │ ──► │ Yes: Draw text    │
│                         │     │ with font, color, │
│                         │     │ alignment, outline│
└────────┬────────────────┘     └──────────────────┘
         │
         ▼
┌─────────────────────────┐
│ Convert to Icon         │
│ Icon::from_rgba(...)    │
└────────┬────────────────┘
         │
         ▼
┌─────────────────────────┐
│ Update Tray             │
│ set_icon(icon)          │
│ set_tooltip("Name 87%") │
└─────────────────────────┘
```

## Code Organization

```
src/
│
├── main.rs ─────────────────── Entry point, app bootstrap, version/mode detection
│
├── engine/ ─────────────────── Core logic: event loop, watchers, icon generation
│   ├── mod.rs                  Module declarations & re-exports
│   ├── event_loop.rs           Main event loop, Watcher trait, settings/theme handlers
│   ├── watcher_common.rs       Shared watcher utilities (log_devices)
│   ├── watcher_v3.rs           Synapse V3 log parser (regex-based)
│   ├── watcher_v4.rs           Synapse V4 log parser (JSON-based)
│   ├── watcher_emulated.rs     Fake device watcher for testing (--emulate flag)
│   └── icon_manager/           Icon loading, theming, and text overlay
│       ├── mod.rs              Public API: load_icon, load_unknown_icon, LoadIconParams
│       ├── assets.rs           Theme scanning, embedded & custom asset loading,
│       │                       set_themes_config(), scan_themes(), default_themes_root()
│       ├── text_overlay.rs     Percentage text rendering with font & outline
│       └── theme.rs            Dark/light/system theme detection & switching
│
├── model/ ──────────────────── Data structures & configuration (no logic)
│   ├── mod.rs                  Module declarations & re-exports
│   ├── device.rs               RazerDevice, DeviceCategory, DeviceMap
│   ├── icon_settings.rs        IconSettings, TextOverlayConfig
│   ├── settings.rs             Settings (JSON persist), IconTheme, TextAlignment,
│   │                           SynapseVersion, LogFontData
│   │                           Notable: polling_interval_minutes + polling_interval_seconds
│   │                           themes_folder (root), active_theme (name)
│   │                           polling_interval_total_seconds() helper
│   └── v4_log_types.rs         LoggedDeviceInfo, PowerStatus, ChargingStatus, NameMap
│
├── ui/ ─────────────────────── User interface: tray, menus, settings window
│   ├── mod.rs                  Module declarations & re-exports
│   ├── constants.rs            UI string constants (menu text, tooltips)
│   ├── context_menu.rs         Native Win32 popup menu (unused, for reference)
│   ├── menu_events.rs          MenuAction enum, tray menu event handling
│   ├── tray_manager.rs         TrayManager: per-device icon map, fallback icon,
│   │                           change detection, menu building
│   └── settings/               Settings dialog (Win32 native)
│       ├── mod.rs              SettingsWindow::show(), window creation, message loop
│       ├── state.rs            SettingsWindowState, change detection, save logic
│       ├── general_tab.rs      General tab: theme dropdown, theme folder picker,
│       │                       dark/light/system radios, device overlay checkbox,
│       │                       polling interval (minutes + seconds), autostart
│       ├── text_tab.rs         Text tab controls (font, color, alignment, position)
│       ├── devices_tab.rs      Devices tab: per-device visibility checkboxes,
│       │                       scrollable panel, disconnect/remove controls
│       ├── event_handlers.rs   WM_COMMAND/WM_NOTIFY dispatch, checkbox/edit/picker handlers,
│       │                       theme combobox handler, polling interval validation
│       ├── dialogs.rs          Color picker, font picker, folder browser, FontResult
│       ├── helpers.rs          Win32 control helpers (checkbox, enable, show/hide, DPI)
│       └── ffi_types.rs        Raw FFI structs (CHOOSECOLORW, CHOOSEFONTW, LOGFONTW)
│
├── util/ ───────────────────── Cross-cutting utilities
│   ├── mod.rs                  Module declarations & re-exports
│   ├── logging.rs              log() (debug console), write_error_log() (file)
│   ├── startup.rs              Windows autostart shortcut (Startup folder .lnk)
│   └── utils.rs                parse_hex_color(), to_wide() helpers
│
└── assets/ ─────────────────── Embedded icon assets
    ├── icon.properties          Battery level → icon filename mapping
    ├── app_icon.ico             Application icon
    ├── dark/                    Dark theme icons (5/20/40/60/80/100.png, overlays)
    └── light/                   Light theme icons (5/20/40/60/80/100.png, overlays)
```

## Package Responsibilities

| Package   | Responsibility                                     | Depends On         |
|-----------|----------------------------------------------------|--------------------|
| `main`    | Bootstrap, CLI args, wiring packages together       | all packages       |
| `engine`  | Event loop, log parsing, icon generation            | `model`, `ui`, `util` |
| `model`   | Data structures, settings persistence               | —                  |
| `ui`      | Tray icons (per device), menus, settings window     | `model`, `engine`, `util` |
| `util`    | Logging, autostart, shared helper functions         | —                  |

## Key Design Decisions

- **Single Responsibility**: Each package has a clear purpose — `model` for data,
  `engine` for logic, `ui` for presentation, `util` for cross-cutting concerns.
- **Watcher Trait**: `engine::event_loop::Watcher` abstracts V3/V4/Emulation differences
  behind a common interface, enabling polymorphic dispatch in the event loop.
- **Multi-Device Tray Icons**: `TrayManager` maintains a `HashMap<String, TrayIcon>`
  keyed by device handle. One system tray icon is created per visible connected device;
  icons are created/removed dynamically as devices appear or disappear.
- **Hidden Devices (Exclusion List)**: `settings.hidden_device_handles: Vec<String>`
  replaces the old single `shown_device_handle`. Empty = show all; adding a handle
  suppresses that device's tray icon.
- **Emulation Mode**: `--emulate` / `-e` flag starts `EmulationWatcher` with four
  hard-coded fake devices and no dependency on Razer Synapse, useful for UI testing.
- **Shared Utilities**: Common patterns (`to_wide`, `log_devices`, `compute_point_size`)
  are extracted to avoid duplication across watchers and UI code.
- **Enum `as_str()` methods**: `IconTheme` and `TextAlignment` provide `as_str()`
  for consistent, allocation-free string conversion instead of `format!("{:?}")`.

## Key Insight: Why This Works

```
┌───────────────────────────────────────────────────────────────┐
│ Traditional Approach (Doesn't Work)                           │
├───────────────────────────────────────────────────────────────┤
│                                                               │
│  App → Razer SDK → USB Driver → Device                       │
│         ❌ No public SDK                                      │
│         ❌ Reverse engineering required                       │
│         ❌ May break with updates                             │
│                                                               │
└───────────────────────────────────────────────────────────────┘

┌───────────────────────────────────────────────────────────────┐
│ Our Approach (Works!)                                         │
├───────────────────────────────────────────────────────────────┤
│                                                               │
│  Synapse → USB Driver → Device                               │
│     │                                                         │
│     └─→ Writes logs ─→ Our App reads logs                    │
│                                                               │
│  ✅ No SDK needed                                             │
│  ✅ Works with any device Synapse supports                    │
│  ✅ Simple text/JSON parsing                                  │
│  ✅ Synapse already running anyway                            │
│                                                               │
└───────────────────────────────────────────────────────────────┘
```

This is why the Rust implementation works - we piggyback on Synapse's own logging!
