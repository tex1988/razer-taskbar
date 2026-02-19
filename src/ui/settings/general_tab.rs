use anyhow::Result;
use windows::Win32::Foundation::{HWND, LPARAM, WPARAM};
use windows::Win32::UI::WindowsAndMessaging::*;
use crate::model::Settings;
use crate::util::to_wide;
use super::helpers::{set_checkbox, enable_window};

pub const GENERAL_TAB_IDS: &[i32] = &[1014, 1011, 1012, 1013, 1015, 1007, 1005, 1006, 1010, 1003, 1004];

pub unsafe fn create_general_tab(hwnd: HWND, s: &Settings) -> Result<()> {
    create_device_overlay_checkbox(hwnd, s)?;
    create_theme_controls(hwnd, s)?;
    create_custom_assets_controls(hwnd, s)?;
    create_polling_controls(hwnd, s)?;
    create_autostart_checkbox(hwnd, s)?;
    Ok(())
}

unsafe fn create_device_overlay_checkbox(hwnd: HWND, s: &Settings) -> Result<()> {
    CreateWindowExW(WINDOW_EX_STYLE::default(), windows::core::w!("BUTTON"),
        windows::core::w!("Show device type overlay"),
        WS_VISIBLE | WS_CHILD | WINDOW_STYLE(BS_AUTOCHECKBOX as u32),
        30, 55, 350, 20, Some(hwnd), Some(HMENU(1014isize as *mut _)), None, None)?;
    set_checkbox(hwnd, 1014, s.show_device_type_overlay);
    Ok(())
}

unsafe fn create_theme_controls(hwnd: HWND, s: &Settings) -> Result<()> {
    // Label
    CreateWindowExW(WINDOW_EX_STYLE::default(), windows::core::w!("STATIC"),
        windows::core::w!("Icon theme:"), WS_VISIBLE | WS_CHILD,
        30, 85, 200, 20, Some(hwnd), Some(HMENU(1011isize as *mut _)), None, None)?;
    // Dark
    CreateWindowExW(WINDOW_EX_STYLE::default(), windows::core::w!("BUTTON"),
        windows::core::w!("Dark"),
        WS_VISIBLE | WS_CHILD | WINDOW_STYLE(BS_AUTORADIOBUTTON as u32 | WS_GROUP.0),
        30, 107, 70, 20, Some(hwnd), Some(HMENU(1012isize as *mut _)), None, None)?;
    // Light
    CreateWindowExW(WINDOW_EX_STYLE::default(), windows::core::w!("BUTTON"),
        windows::core::w!("Light"),
        WS_VISIBLE | WS_CHILD | WINDOW_STYLE(BS_AUTORADIOBUTTON as u32),
        110, 107, 70, 20, Some(hwnd), Some(HMENU(1013isize as *mut _)), None, None)?;
    // System
    CreateWindowExW(WINDOW_EX_STYLE::default(), windows::core::w!("BUTTON"),
        windows::core::w!("System"),
        WS_VISIBLE | WS_CHILD | WINDOW_STYLE(BS_AUTORADIOBUTTON as u32),
        190, 107, 80, 20, Some(hwnd), Some(HMENU(1015isize as *mut _)), None, None)?;

    set_theme_radio(hwnd, s);
    Ok(())
}

unsafe fn set_theme_radio(hwnd: HWND, s: &Settings) {
    let theme = s.icon_theme.as_str();
    if let (Ok(d), Ok(l), Ok(sy)) = (GetDlgItem(Some(hwnd),1012), GetDlgItem(Some(hwnd),1013), GetDlgItem(Some(hwnd),1015)) {
        SendMessageW(d,  BM_SETCHECK, Some(WPARAM(if theme=="dark"  {1} else {0})), Some(LPARAM(0)));
        SendMessageW(l,  BM_SETCHECK, Some(WPARAM(if theme=="light" {1} else {0})), Some(LPARAM(0)));
        SendMessageW(sy, BM_SETCHECK, Some(WPARAM(if theme=="system"{1} else {0})), Some(LPARAM(0)));
    }
}

unsafe fn create_custom_assets_controls(hwnd: HWND, s: &Settings) -> Result<()> {
    let use_custom = s.custom_assets_folder.is_some();
    // Checkbox
    CreateWindowExW(WINDOW_EX_STYLE::default(), windows::core::w!("BUTTON"),
        windows::core::w!("Use custom assets"),
        WS_VISIBLE | WS_CHILD | WINDOW_STYLE(BS_AUTOCHECKBOX as u32),
        30, 142, 200, 20, Some(hwnd), Some(HMENU(1007isize as *mut _)), None, None)?;
    set_checkbox(hwnd, 1007, use_custom);
    // Edit
    let edit = CreateWindowExW(WS_EX_CLIENTEDGE, windows::core::w!("EDIT"), windows::core::w!(""),
        WS_VISIBLE | WS_CHILD | WS_BORDER | WINDOW_STYLE(ES_AUTOHSCROLL as u32),
        30, 166, 270, 25, Some(hwnd), Some(HMENU(1005isize as *mut _)), None, None)?;
    if let Some(ref fp) = s.custom_assets_folder {
        let w = to_wide(fp);
        let _ = SetWindowTextW(edit, windows::core::PCWSTR(w.as_ptr()));
    }
    enable_window(edit, use_custom);
    // Browse
    let btn = CreateWindowExW(WINDOW_EX_STYLE::default(), windows::core::w!("BUTTON"),
        windows::core::w!("Browse..."),
        WS_VISIBLE | WS_CHILD | WINDOW_STYLE(BS_PUSHBUTTON as u32),
        305, 166, 75, 25, Some(hwnd), Some(HMENU(1006isize as *mut _)), None, None)?;
    enable_window(btn, use_custom);
    Ok(())
}

unsafe fn create_polling_controls(hwnd: HWND, s: &Settings) -> Result<()> {
    CreateWindowExW(WINDOW_EX_STYLE::default(), windows::core::w!("STATIC"),
        windows::core::w!("Log polling interval (minutes):"), WS_VISIBLE | WS_CHILD,
        30, 206, 250, 20, Some(hwnd), Some(HMENU(1010isize as *mut _)), None, None)?;
    let edit = CreateWindowExW(WS_EX_CLIENTEDGE, windows::core::w!("EDIT"), windows::core::w!(""),
        WS_VISIBLE | WS_CHILD | WS_BORDER | WINDOW_STYLE(ES_NUMBER as u32),
        30, 228, 100, 25, Some(hwnd), Some(HMENU(1003isize as *mut _)), None, None)?;
    let txt = to_wide(&format!("{}", s.polling_interval_minutes));
    let _ = SetWindowTextW(edit, windows::core::PCWSTR(txt.as_ptr()));
    Ok(())
}

unsafe fn create_autostart_checkbox(hwnd: HWND, s: &Settings) -> Result<()> {
    CreateWindowExW(WINDOW_EX_STYLE::default(), windows::core::w!("BUTTON"),
        windows::core::w!("Start with Windows (autostart)"),
        WS_VISIBLE | WS_CHILD | WINDOW_STYLE(BS_AUTOCHECKBOX as u32),
        30, 268, 350, 25, Some(hwnd), Some(HMENU(1004isize as *mut _)), None, None)?;
    set_checkbox(hwnd, 1004, s.run_at_startup);
    Ok(())
}


