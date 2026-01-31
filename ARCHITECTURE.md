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
│  │  File Watcher      │  │  Log Parsers        │                │
│  │  (notify crate)    │──│  - watcher_v3.rs   │                │
│  │                    │  │  - watcher_v4.rs   │                │
│  │  Detects changes   │  │  Regex/JSON parse  │                │
│  └────────────────────┘  └─────────┬───────────┘                │
│                                    │                             │
│                                    ▼                             │
│                          ┌──────────────────┐                    │
│                          │  Device Manager  │                    │
│                          │  (device.rs)     │                    │
│                          │                  │                    │
│                          │  - Device list   │                    │
│                          │  - Battery %     │                    │
│                          │  - Charging?     │                    │
│                          └────────┬─────────┘                    │
│                                   │                              │
│                                   ▼                              │
│                          ┌──────────────────┐                    │
│                          │  Tray Manager    │                    │
│                          │  (tray_icon)     │                    │
│                          │                  │                    │
│                          │  - Pick device   │                    │
│                          │  - Create icon   │                    │
│                          │  - Update tray   │                    │
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
                         ▼ watcher_v3.rs
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
                │ RazerDevice        │
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
                         ▼ watcher_v4.rs
        ┌────────────────────────────────────────┐
        │ JSON Deserialization                   │
        ├────────────────────────────────────────┤
        │ struct LoggedDeviceInfo {              │
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
                │ RazerDevice        │
                ├────────────────────┤
                │ name: "Razer..."   │
                │ handle: "AB123"    │
                │ battery: 87        │
                │ charging: false    │
                │ connected: true    │
                └────────────────────┘
```

## File Watching Flow

```
┌─────────────────────────────────────────────────────────────┐
│ Main Event Loop (main.rs)                                   │
└─────┬───────────────────────────────────────────────────────┘
      │
      ├─► Step 1: Detect Synapse version
      │   ├─ Check for V4 logs → exists? Use V4
      │   └─ Otherwise → Use V3
      │
      ├─► Step 2: Create file watcher
      │   ├─ V3: watch("Razer Synapse 3.log")
      │   └─ V4: poll("systray_systrayv2*.log", every 5 sec)
      │
      ├─► Step 3: Initial parse
      │   └─ Parse log file → Update tray icon
      │
      └─► Step 4: Event loop
          │
          ┌───────────────────────────┐
          │ Every 5 seconds:          │
          ├───────────────────────────┤
          │ 1. Check menu events      │
          │    └─ Quit clicked? Exit  │
          │                           │
          │ 2. Check log file         │
          │    └─ Modified? Re-parse  │
          │                           │
          │ 3. Update tray icon       │
          │    ├─ Pick device         │
          │    ├─ Create icon         │
          │    └─ Set tooltip         │
          └───────────────────────────┘
                      │
                      └──► Repeat ──┐
                                    │
                      ┌─────────────┘
                      ▼
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
│ Determine Level         │
│ floor(87 / 20) * 25     │
│ = floor(4.35) * 25      │
│ = 4 * 25 = 100          │
└────────┬────────────────┘
         │
         ▼
┌─────────────────────────┐     ┌──────────────────┐
│ Is Charging?            │ ──► │ Yes: Green icon  │
└────────┬────────────────┘     └──────────────────┘
         │ No
         ▼
┌─────────────────────────┐
│ Select Color:           │
│ 0-25%   → Red           │
│ 26-50%  → Orange        │
│ 51-75%  → Yellow        │
│ 76-100% → Green         │
└────────┬────────────────┘
         │
         ▼ Level = 100, Color = Green
┌─────────────────────────┐
│ Create 32x32 image      │
│ Fill bottom 87% green   │
│ Fill top 13% dark gray  │
└────────┬────────────────┘
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
rust-implementation/
│
├── Cargo.toml ──────────────────┐
│   Dependencies:                │
│   - tray-icon (system tray)    │
│   - notify (file watching)     │
│   - regex (log parsing)        │
│   - serde_json (JSON parsing)  │
│   - image (icon generation)    │
│                                │
├── src/                         │
│   │                            │
│   ├── main.rs ─────────────────┤── Entry point
│   │   - Detect V3/V4           │   Event loop
│   │   - Setup file watcher     │   Error handling
│   │   - Main event loop        │
│   │                            │
│   ├── device.rs ───────────────┤── Data structures
│   │   - RazerDevice struct     │
│   │   - DeviceMap type         │
│   │                            │
│   ├── settings.rs ─────────────┤── Configuration
│   │   - Settings struct        │   JSON save/load
│   │   - load() / save()        │
│   │                            │
│   ├── watcher_v3.rs ───────────┤── Synapse V3
│   │   - Regex patterns         │   Log parsing
│   │   - parse_devices()        │   Regex matching
│   │                            │
│   ├── watcher_v4.rs ───────────┤── Synapse V4
│   │   - JSON structures        │   Log parsing
│   │   - parse_devices()        │   JSON deserialization
│   │                            │
│   └── tray_manager.rs ─────────┤── System tray
│       - TrayManager struct     │   Icon creation
│       - update_devices()       │   Menu management
│       - create_battery_icon()  │
│                                │
└── Documentation/               │
    ├── README.md                │
    ├── HOW_IT_WORKS.md          │
    ├── QUICKSTART.md            │
    └── IMPLEMENTATION_SUMMARY.md│
```

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
