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
│  ┌────────────────────┐  ┌─────────────────────┐                │
│  │  Event Loop        │  │  Log Parsers        │                │
│  │  (engine)          │──│  - watcher_v3       │                │
│  │                    │  │  - watcher_v4       │                │
│  │  Polls & dispatches│  │  Regex/JSON parse   │                │
│  └────────────────────┘  └─────────┬───────────┘                │
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
                    ┌──────────────────────────┐
                    │  Windows System Tray     │
                    │  [Battery Icon] 85%      │
                    └──────────────────────────┘
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

## Event Loop Flow

```
┌─────────────────────────────────────────────────────────────┐
│ main.rs → run_app()                                         │
└─────┬───────────────────────────────────────────────────────┘
      │
      ├─► Step 1: Load settings (model::Settings)
      │
      ├─► Step 2: Init assets & theme (engine::icon_manager)
      │
      ├─► Step 3: Detect Synapse version
      │   ├─ Check for V4 logs → exists? Use V4
      │   └─ Otherwise → Use V3
      │
      ├─► Step 4: Create tray icon (ui::TrayManager)
      │
      └─► Step 5: Enter event loop (engine::event_loop)
          │
          ┌───────────────────────────────────────┐
          │ Every polling interval:                │
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
          │ 4. Poll log file for changes           │
          │    ├─ Check log rotation (V4)          │
          │    ├─ Parse devices                    │
          │    └─ Update tray icon                 │
          └───────────────────────────────────────┘
```

## Device Selection Logic

```
Multiple Devices Connected:
┌────────────────────────────────────────────────────────────┐
│ Device A: Razer Viper (25%, not charging)  → Priority: 25  │
│ Device B: Razer Naga  (80%, charging)      → Priority: 800 │
│ Device C: Razer Mamba (15%, not charging)  → Priority: 15  │
└────────────────────────────────────────────────────────────┘
                           │
                           ▼ sort_by_key(battery * if charging {100} else {1})
                           │
┌────────────────────────────────────────────────────────────┐
│ Sorted Priority:                                           │
│ 1. Device C: 15% (not charging)  ← SHOW THIS              │
│ 2. Device A: 25% (not charging)                           │
│ 3. Device B: 80% (charging)                               │
└────────────────────────────────────────────────────────────┘
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
│ set_tooltip("87%")      │
└─────────────────────────┘
```

## Code Organization

```
src/
│
├── main.rs ─────────────────── Entry point, app bootstrap, version detection
│
├── engine/ ─────────────────── Core logic: event loop, watchers, icon generation
│   ├── mod.rs                  Module declarations & re-exports
│   ├── event_loop.rs           Main event loop, Watcher trait, settings/theme handlers
│   ├── watcher_common.rs       Shared watcher utilities (log_devices)
│   ├── watcher_v3.rs           Synapse V3 log parser (regex-based)
│   ├── watcher_v4.rs           Synapse V4 log parser (JSON-based)
│   └── icon_manager/           Icon loading, theming, and text overlay
│       ├── mod.rs              Public API: load_icon, load_unknown_icon, LoadIconParams
│       ├── assets.rs           Embedded & custom asset loading, battery range lookup
│       ├── text_overlay.rs     Percentage text rendering with font & outline
│       └── theme.rs            Dark/light/system theme detection & switching
│
├── model/ ──────────────────── Data structures & configuration (no logic)
│   ├── mod.rs                  Module declarations & re-exports
│   ├── device.rs               RazerDevice, DeviceCategory, DeviceMap
│   ├── icon_settings.rs        IconSettings, TextOverlayConfig
│   ├── settings.rs             Settings (JSON persist), IconTheme, TextAlignment,
│   │                           SynapseVersion, LogFontData
│   └── v4_log_types.rs         LoggedDeviceInfo, PowerStatus, ChargingStatus, NameMap
│
├── ui/ ─────────────────────── User interface: tray, menus, settings window
│   ├── mod.rs                  Module declarations & re-exports
│   ├── constants.rs            UI string constants (menu text, tooltips)
│   ├── context_menu.rs         Native Win32 popup menu (unused, for reference)
│   ├── menu_events.rs          MenuAction enum, tray menu event handling
│   ├── tray_manager.rs         TrayManager: icon updates, menu building, device picking
│   └── settings/               Settings dialog (Win32 native)
│       ├── mod.rs              SettingsWindow::show(), window creation, message loop
│       ├── state.rs            SettingsWindowState, change detection, save logic
│       ├── general_tab.rs      General tab controls (theme, assets, polling, autostart)
│       ├── text_tab.rs         Text tab controls (font, color, alignment, position)
│       ├── event_handlers.rs   WM_COMMAND/WM_NOTIFY dispatch, checkbox/edit/picker handlers
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
| `ui`      | Tray icon, menus, settings window (Win32)           | `model`, `engine`, `util` |
| `util`    | Logging, autostart, shared helper functions         | —                  |

## Key Design Decisions

- **Single Responsibility**: Each package has a clear purpose — `model` for data,
  `engine` for logic, `ui` for presentation, `util` for cross-cutting concerns.
- **Watcher Trait**: `engine::event_loop::Watcher` abstracts V3/V4 differences
  behind a common interface, enabling polymorphic dispatch in the event loop.
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
