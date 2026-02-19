# Razer Taskbar - Rust Implementation

A lightweight Rust application that displays battery status of Razer wireless devices in the Windows system tray by monitoring Razer Synapse log files.

## Features

✅ **Works with both Synapse V3 and V4** - Automatically detects which version you're using
✅ **Minimal resource usage** - ~5-10MB RAM vs ~150MB for the Electron version
✅ **Fast startup** - Launches in <100ms
✅ **Real-time monitoring** - Watches log files for battery status changes
✅ **System tray icon** - Visual battery indicator in taskbar
✅ **Multiple devices** - Shows lowest battery device (non-charging preferred)

## How It Works

This app **does NOT use the Razer SDK**. Instead, it reads the same log files that Razer Synapse writes to extract battery information:

### Synapse V3
- **Log Location**: `%LOCALAPPDATA%\Razer\Synapse3\Log\Razer Synapse 3.log`
- **Method**: Parses log entries with regex to find `_OnBatteryLevelChanged` events
- **Example log line**:
  ```
  2024-01-30 14:23:45 INFO _OnBatteryLevelChanged
  Name: Razer Viper Ultimate
  Handle: 12345
  level 87 state 0
  ```

### Synapse V4
- **Log Location**: `%LOCALAPPDATA%\Razer\RazerAppEngine\User Data\Logs\systray_systrayv2*.log`
- **Method**: Parses JSON data from log entries with `connectingDeviceData`
- **Example log line**:
  ```
  [2024-01-30T14:23:45] connectingDeviceData: [{"serialNumber":"AB123","hasBattery":true,"powerStatus":{"level":87,"chargingStatus":"NoCharge_BatteryFull"},"name":{"en":"Razer Viper Ultimate"}}]
  ```

## Building

### Prerequisites
- Rust (install from https://rustup.rs/)
- Windows 10/11

### Build Steps

```powershell
cd rust-implementation
cargo build --release
```

The compiled executable will be at: `target\release\razer-taskbar.exe`

## Running

Simply run the executable:
```powershell
.\target\release\razer-taskbar.exe
```

The app will:
1. Create a tray icon in your taskbar
2. Auto-detect Synapse V3 or V4
3. Start monitoring log files
4. Update the tray icon when battery levels change

## Configuration

Settings are stored in: `%LOCALAPPDATA%\razer-taskbar\settings.json`

## Theming

Razer Taskbar supports fully custom icon sets. You can create your own theme and point the app to it via **Settings → General → Use custom assets**.

### Folder structure

A custom assets folder must follow this layout:

```
my-theme/
├── icon.properties       ← battery-range → filename mapping
├── dark/                 ← icons used when "Dark" theme is selected
│   ├── 100.png
│   ├── 80.png
│   ├── 60.png
│   ├── 40.png
│   ├── 20.png
│   ├── 5.png
│   ├── charging.png      ← overlay drawn on top when device is charging
│   ├── no_device.png     ← shown when no device is detected
│   ├── mouse.png         ← device type overlay (optional)
│   ├── keyboard.png      ← device type overlay (optional)
│   ├── headphones.png    ← device type overlay (optional)
│   └── unknown.png       ← device type overlay for unrecognised category (optional)
└── light/                ← icons used when "Light" theme is selected
    ├── 100.png
    ├── 80.png
    ├── 60.png
    ├── 40.png
    ├── 20.png
    ├── 5.png
    ├── charging.png
    ├── no_device.png
    ├── mouse.png
    ├── keyboard.png
    ├── headphones.png
    └── unknown.png
```

> **Note:** Both `dark/` and `light/` subfolders are optional. If a subfolder is missing the app automatically falls back to the built-in embedded assets for that theme.

### icon.properties

This file maps battery percentage ranges to icon filenames. The same mapping applies to both the `dark/` and `light/` subfolders — you only need one `icon.properties` at the root of your theme folder.

```ini
# Format: min-max=filename.png
81-100=100.png
61-80=80.png
41-60=60.png
21-40=40.png
6-20=20.png
0-5=5.png
```

- Ranges are **inclusive** on both ends.
- Filenames are relative to the active theme subfolder (`dark/` or `light/`).
- `charging.png` and `no_device.png` are reserved special names and do **not** need to be listed in `icon.properties`.
- `mouse.png`, `keyboard.png`, `headphones.png`, and `unknown.png` are optional device type overlays drawn on top of the battery icon when **Show device type overlay** is enabled in settings. If a file is absent from the custom folder the built-in embedded overlay for the active theme is used as fallback.
- You can define as many or as few ranges as you like (e.g. a single `0-100=battery.png` is valid).
- Ranges must not overlap; the first matching range wins.

### Icon specifications

| Property | Recommended value |
|---|---|
| Format | PNG with transparency (RGBA) |
| Size | **32 × 32 px** (Windows scales down if larger, which may look blurry) |
| Color depth | 32-bit |

### Asset resolution order

When loading an icon the app tries the following sources in order, stopping at the first hit:

1. `<custom_folder>/<theme>/filename` — your themed custom asset
2. `<custom_folder>/filename` — flat custom asset (no `dark/`/`light/` subfolder needed; useful for single-theme packs)
3. Embedded `<theme>/filename` — the built-in asset for the selected theme
4. Embedded opposite-theme `filename` — automatic fallback if the other embedded theme also has the file

### Switching themes

The active theme is chosen in **Settings → General → Icon theme** with three options:

| Option | Behaviour |
|---|---|
| **Dark** | Always use icons from the `dark/` subfolder |
| **Light** | Always use icons from the `light/` subfolder |
| **System** | Automatically follows the Windows system theme (dark/light app mode). The OS preference is read from `HKCU\Software\Microsoft\Windows\CurrentVersion\Themes\Personalize\AppsUseLightTheme` each time settings are applied. |

The setting is saved to `settings.json` and takes effect immediately after closing the settings window. The default is **System**.

## License

MIT (same as original project)
