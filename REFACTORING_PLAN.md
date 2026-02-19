# Razer Taskbar — Clean Code & SOLID Refactoring Plan

## Constraints
| Rule | Limit |
|------|-------|
| Max function length | **30 lines** |
| Max function arguments | **4 arguments** |
| Max file length | **200 lines** |
| Every file | **Single Responsibility Principle** |

---

## Current Violations Summary

| File | Lines | Violations |
|------|------:|-----------|
| `settings_window.rs` | **1456** | 7× over limit; `show()` 700+ lines; `settings_wnd_proc` 540+ lines; mixes FFI, UI, events, dialogs |
| `icon_manager.rs` | **614** | 3× over limit; `draw_percentage_overlay` 120 lines, 9 args; `load_icon` 60 lines, 12 args; mixes theme, assets, composition, fonts |
| `main.rs` | **447** | 2× over limit; `run_v3_watcher` 112 lines; `run_v4_watcher` 109 lines; massive duplication between the two; `parse_and_update_v4` has 5 args |
| `tray_manager.rs` | **230** | `update_devices` has 10 args, 74 lines |
| `settings.rs` | **206** | Slightly over (trim default fns) |
| `watcher_v4.rs` | **204** | `parse_devices` 88 lines |
| `ui_constants.rs` | 171 | ✅ OK |
| `r_click_menu.rs` | 147 | ✅ OK |
| `watcher_v3.rs` | 115 | ✅ OK |
| `startup.rs` | 98 | ✅ OK |
| `device.rs` | 32 | ✅ OK |
| `utils.rs` | 17 | ✅ OK |

---

## Step 1 — Introduce Parameter Structs in `settings.rs`

**Goal:** Eliminate the root cause of >4 argument violations everywhere.

**What to do:**
1. Add an `IconSettings` struct that groups all icon-related fields currently passed as individual arguments:
   ```rust
   #[derive(Debug, Clone, PartialEq)]
   pub struct IconSettings {
       pub show_percentage: bool,
       pub text_size: u32,
       pub text_color: String,
       pub font_name: String,
       pub text_align: String,
       pub text_x: i32,
       pub text_y: i32,
       pub show_percent_symbol: bool,
       pub show_device_type_overlay: bool,
   }
   ```
2. Add a `TextOverlayConfig` struct (subset for text drawing):
   ```rust
   #[derive(Debug, Clone)]
   pub struct TextOverlayConfig {
       pub text_size: u32,
       pub text_color: String,
       pub font_name: String,
       pub text_align: String,
       pub text_x: i32,
       pub text_y: i32,
       pub show_percent_symbol: bool,
   }
   ```
3. Add `Settings::to_icon_settings(&self) -> IconSettings` method.
4. Move default value functions into `impl Default` blocks to reduce line count.

**Result:** `settings.rs` ≤ 190 lines. Unblocks all subsequent steps.

---

## Step 2 — Refactor `tray_manager.rs` to Use `IconSettings`

**Goal:** Reduce `update_devices` from 10 args → 3 (self + devices + icon_settings).

**Changes:**
1. Change signature: `update_devices(&mut self, devices: DeviceMap, icon_settings: &IconSettings) -> Result<()>`
2. Change `last_icon_settings` from a 9-element tuple to `Option<IconSettings>`
3. Extract `MenuAction` enum + `handle_menu_events()` → new file **`menu_events.rs`** (~30 lines)

**Result:** `tray_manager.rs` ≤ 180 lines, `menu_events.rs` ~30 lines.

---

## Step 3 — Unify Event Loops: Extract `event_loop.rs` + `logging.rs`

**Goal:** Eliminate the ~110-line near-identical duplication between `run_v3_watcher` and `run_v4_watcher`.

### 3a — Extract `logging.rs` (~30 lines)
Move `write_error_log()` and `log()` into their own module.

### 3b — Create `Watcher` trait
```rust
pub trait Watcher {
    fn parse_and_update(
        &mut self,
        tray_manager: &mut TrayManager,
        icon_settings: &IconSettings,
        debug: bool,
    ) -> Result<()>;

    fn check_log_rotation(&mut self, debug: bool);
}
```
- `SynapseV3Watcher` and `SynapseV4Watcher` implement this trait.

### 3c — Extract `event_loop.rs` (~140 lines)
Unified event loop handling:
- `run_event_loop(watcher, tray_manager, settings, debug)` — max 4 args ✅
- `pump_windows_messages()` — extracted helper (~10 lines)
- `handle_settings_change(settings, tray_manager, debug)` — (~25 lines)
- `handle_theme_change(settings, tray_manager, watcher, debug)` — (~15 lines)
- `handle_periodic_poll(watcher, tray_manager, settings, counter, debug)` — (~15 lines)

### 3d — Simplify `main.rs` (~80 lines)
Only: mod declarations, `main()`, `run_app()` (init settings, detect version, call `run_event_loop`).

**Result:** `main.rs` ~80 lines, `event_loop.rs` ~140 lines, `logging.rs` ~30 lines.

---

## Step 4 — Split `icon_manager.rs` into `src/icon/` Module (614 → 5 files)

### New file structure:
```
src/icon/
  mod.rs          ~40 lines   — re-exports, LoadIconParams, load_icon(), load_unknown_icon()
  theme.rs        ~80 lines   — theme state, theme change listener, detect_system_theme()
  assets.rs       ~120 lines  — embedded/custom asset loading, battery ranges, icon.properties
  overlay.rs      ~100 lines  — text overlay drawing (split into small helpers)
  font.rs         ~50 lines   — load_font_from_registry()
```

### Key changes:
1. **`LoadIconParams` struct** replaces 12 individual args:
   ```rust
   pub struct LoadIconParams {
       pub percentage: u8,
       pub is_charging: bool,
       pub icon_settings: IconSettings,
       pub device_category: DeviceCategory,
   }
   ```
   `load_icon(params: &LoadIconParams) -> Result<Icon>` — 1 arg ✅

2. **`draw_percentage_overlay`** split from 120 lines (9 args) into:
   - `resolve_font(font_name: &str) -> Option<Vec<u8>>` (~15 lines)
   - `measure_text_width(font, text, scale) -> f32` (~10 lines)
   - `compute_text_position(align, text_width, offset_x, offset_y) -> (i32, i32)` (~15 lines)
   - `draw_text_with_outline(img, pos, text, font, scale, color)` — uses struct (~15 lines)
   - `draw_percentage_overlay(img, percentage, config: &TextOverlayConfig)` — 3 args, ~25 lines ✅

3. **`create_theme_change_listener`** split: extract `wnd_proc` definition, `register_and_create_window()`.

4. Update all `use crate::icon_manager::` → `use crate::icon::`.

---

## Step 5 — Decompose `settings_window.rs` into `src/settings_ui/` Module (1456 → 8 files)

### New file structure:
```
src/settings_ui/
  mod.rs              ~60 lines   — SettingsWindow::show() orchestrator
  ffi_types.rs        ~80 lines   — LOGFONTW, CHOOSECOLORW, CHOOSEFONTW, extern decls, constants
  state.rs            ~60 lines   — SettingsWindowState, init, save, has_changed()
  general_tab.rs      ~130 lines  — create_general_tab_controls(), per-control helper fns
  text_tab.rs         ~110 lines  — create_text_tab_controls(), per-control helper fns
  event_handlers.rs   ~180 lines  — settings_wnd_proc dispatching to small handler fns
  dialogs.rs          ~80 lines   — pick_folder(), pick_color(), pick_font()
  helpers.rs          ~40 lines   — enable_window(), show_hide_control(), read_edit_text()
```

### Key changes:
1. **`SettingsWindow::show()`** becomes an orchestrator (~50 lines): init state → register class → create window → call `create_general_tab_controls()` → call `create_text_tab_controls()` → hide Tab 1 controls → run msg loop → collect result.

2. **`settings_wnd_proc`** dispatches to small handler functions:
   - `handle_checkbox_clicked(hwnd, id)` — generic for all checkboxes
   - `handle_color_picker(hwnd)` — calls `dialogs::pick_color()`
   - `handle_font_picker(hwnd)` — calls `dialogs::pick_font()`
   - `handle_ok_button(hwnd)` — reads final values, destroys window
   - `handle_tab_switch(hwnd, tab_index)` — uses const ID arrays instead of repeated calls

3. **Tab control ID arrays** replace 40+ manual `show_hide_control` calls:
   ```rust
   const GENERAL_TAB_CONTROLS: &[i32] = &[1007, 1005, 1006, 1011, 1012, 1013, 1015, 1010, 1003, 1004, 1014];
   const TEXT_TAB_CONTROLS: &[i32] = &[2001, 2003, 2004, 2005, 2006, 2007, 2008, 2011, 2012, 2013, 2014];
   ```

4. **Replace `static mut SETTINGS_STATE`** with `OnceLock<Arc<Mutex<SettingsWindowState>>>` for safety.

5. Update `use crate::settings_window::` → `use crate::settings_ui::`.

---

## Step 6 — Clean Up `watcher_v4.rs` and Implement `Watcher` Trait

**Split `parse_devices` (88 lines) into:**
- `find_latest_entry(log_content) -> Option<(&str, &str)>` — returns (timestamp, json_str) (~15 lines)
- `deserialize_devices(json_str, shown_handle, debug) -> Result<DeviceMap>` (~30 lines)
- `remove_duplicate_no_serial(devices: &mut DeviceMap)` (~10 lines)
- Orchestrating `parse_devices()` (~20 lines)

**Implement `Watcher` trait** on both `SynapseV3Watcher` and `SynapseV4Watcher`.

**Result:** `watcher_v4.rs` ≤ 190 lines, all functions ≤ 30 lines.

---

## Execution Order

```
Step 1 ──→ Step 2 ──→ Step 3
  │                      ↑
  ├──→ Step 4 ──────────┘  (can run in parallel with Step 3)
  │
  └──→ Step 5              (independent, can start after Step 1)
                           
Step 3 + Step 6            (Step 6 depends on Watcher trait from Step 3)
```

| Order | Step | Depends On | Estimated Effort |
|------:|------|-----------|-----------------|
| 1 | Step 1: Parameter structs | None | Small |
| 2 | Step 2: Refactor tray_manager | Step 1 | Small |
| 3 | Step 3: Unify event loops | Steps 1-2 | Medium |
| 4 | Step 4: Split icon_manager | Step 1 | Medium |
| 5 | Step 5: Decompose settings_window | Step 1 | Large |
| 6 | Step 6: Clean watcher_v4 | Step 3 | Small |

---

## Final File Map (Target: ~26 files, all ≤200 lines)

| File | Lines | Responsibility |
|------|------:|---------------|
| `main.rs` | ~80 | Entry point, init, version detection |
| `logging.rs` | ~30 | Error logging, debug logging |
| `event_loop.rs` | ~140 | Unified event loop, message pump, settings reload |
| `menu_events.rs` | ~30 | MenuAction enum, handle_menu_events() |
| `device.rs` | 32 | RazerDevice struct, DeviceMap type |
| `settings.rs` | ~190 | Settings, IconSettings, TextOverlayConfig, load/save |
| `startup.rs` | 98 | Windows startup shortcut management |
| `tray_manager.rs` | ~180 | Tray icon lifecycle, device display |
| `icon/mod.rs` | ~40 | LoadIconParams, load_icon(), load_unknown_icon() |
| `icon/theme.rs` | ~80 | Theme state, system theme detection, WM_SETTINGCHANGE |
| `icon/assets.rs` | ~120 | Asset loading (embedded + custom), battery ranges |
| `icon/overlay.rs` | ~100 | Text percentage overlay drawing |
| `icon/font.rs` | ~50 | Windows registry font loading |
| `settings_ui/mod.rs` | ~60 | SettingsWindow::show() orchestrator |
| `settings_ui/ffi_types.rs` | ~80 | Win32 FFI struct definitions |
| `settings_ui/state.rs` | ~60 | Window state management |
| `settings_ui/general_tab.rs` | ~130 | General tab control creation |
| `settings_ui/text_tab.rs` | ~110 | Percentage text tab control creation |
| `settings_ui/event_handlers.rs` | ~180 | WndProc + per-control handlers |
| `settings_ui/dialogs.rs` | ~80 | Color/font/folder picker dialogs |
| `settings_ui/helpers.rs` | ~40 | UI helper functions |
| `watcher_v3.rs` | 115 | Synapse V3 log parsing |
| `watcher_v4.rs` | ~190 | Synapse V4 log parsing |
| `r_click_menu.rs` | 147 | Native context menu |
| `ui_constants.rs` | 171 | UI color/size/text constants |
| `utils.rs` | 17 | Hex color parsing utility |

---

## Additional Recommendations (Optional Follow-ups)

1. **Replace `debug: bool` threading** with the `log` or `tracing` crate — would further reduce argument counts across the entire codebase
2. **Replace `static mut` in `r_click_menu.rs`** with `AtomicBool` for safety
3. **Add unit tests** for pure functions: `parse_icon_properties()`, `parse_hex_color()`, `find_icon_for_percentage()`, `pick_device_to_display()`

