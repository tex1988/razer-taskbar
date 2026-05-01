use windows::Win32::Foundation::{HWND, LPARAM, LRESULT, WPARAM};
use windows::Win32::Graphics::Gdi::*;
use windows::Win32::UI::WindowsAndMessaging::*;
use windows::Win32::UI::Controls::{NMHDR, TCM_GETCURSEL};
use super::ffi_types::*;
use super::helpers::*;
use super::state;
use super::general_tab::GENERAL_TAB_IDS;
use super::text_tab::TEXT_TAB_IDS;
use super::devices_tab;
use super::dialogs;
use crate::util::to_wide;

pub unsafe extern "system" fn settings_wnd_proc(
    hwnd: HWND, msg: u32, wparam: WPARAM, lparam: LPARAM,
) -> LRESULT {
    match msg {
        WM_CTLCOLORSTATIC => handle_ctl_color(wparam),
        WM_COMMAND => handle_command(hwnd, wparam),
        WM_NOTIFY => handle_notify(hwnd, lparam),
        WM_CLOSE => { let _ = DestroyWindow(hwnd); LRESULT(0) }
        WM_DESTROY => { PostQuitMessage(0); LRESULT(0) }
        _ => DefWindowProcW(hwnd, msg, wparam, lparam),
    }
}

unsafe fn handle_ctl_color(wparam: WPARAM) -> LRESULT {
    let hdc = HDC(wparam.0 as *mut _);
    SetBkMode(hdc, TRANSPARENT);
    LRESULT(GetSysColorBrush(COLOR_3DFACE).0 as isize)
}

unsafe fn handle_command(hwnd: HWND, wparam: WPARAM) -> LRESULT {
    let id = (wparam.0 & 0xFFFF) as i32;
    let notif = ((wparam.0 >> 16) & 0xFFFF) as u32;
    match id {
        2001 | 1001 => handle_checkbox_bool(hwnd, id, notif, "show_percentage"),
        2007 => handle_checkbox_bool(hwnd, id, notif, "show_percent_symbol"),
        1004 => handle_checkbox_bool(hwnd, id, notif, "run_at_startup"),
        1014 => handle_checkbox_bool(hwnd, id, notif, "show_device_type_overlay"),
        1012 => set_theme_if_clicked(notif, "dark"),
        1013 => set_theme_if_clicked(notif, "light"),
        1015 => set_theme_if_clicked(notif, "system"),
        1016 => { if notif == CBN_SELCHANGE { handle_theme_combobox(hwnd); } }
        2003 => { if notif == BN_CLICKED { handle_color_picker(hwnd); } }
        2008 => { if notif == BN_CLICKED { handle_font_picker(hwnd); } }
        2004 => { if notif == CBN_SELCHANGE { handle_alignment(hwnd); } }
        2005 => { if notif == EN_CHANGE { handle_edit_int(hwnd, 2005, "text_x"); } }
        2006 => { if notif == EN_CHANGE { handle_edit_int(hwnd, 2006, "text_y"); } }
        1007 => { if notif == BN_CLICKED { handle_custom_assets_toggle(hwnd); } }
        1006 => { if notif == BN_CLICKED { handle_browse(hwnd); } }
        9001 => handle_ok_button(hwnd),
        _ => {
            // Check if it's a per-device delete button from the Devices tab
            if notif == BN_CLICKED {
                if let Some(dev_idx) = devices_tab::delete_button_index(id) {
                    devices_tab::handle_delete_device(hwnd, dev_idx);
                }
            }
        }
    }
    LRESULT(0)
}

unsafe fn handle_checkbox_bool(hwnd: HWND, id: i32, notif: u32, field: &str) {
    if notif != BN_CLICKED { return; }
    let val = get_checkbox(hwnd, id);
    state::with_state(|s| match field {
        "show_percentage" => s.show_percentage = val,
        "show_percent_symbol" => s.show_percent_symbol = val,
        "run_at_startup" => s.run_at_startup = val,
        "show_device_type_overlay" => s.show_device_type_overlay = val,
        _ => {}
    });
}

fn set_theme_if_clicked(notif: u32, theme: &str) {
    if notif != BN_CLICKED { return; }
    state::with_state(|s| s.icon_theme = theme.to_string());
}

unsafe fn handle_color_picker(hwnd: HWND) {
    let current = state::with_state(|s| s.percentage_text_color.clone())
        .unwrap_or("FFFFFF".to_string());
    if let Some(hex) = dialogs::pick_color(hwnd, &current) {
        state::with_state(|s| s.percentage_text_color = hex.clone());
        if let Ok(btn) = GetDlgItem(Some(hwnd), 2003) {
            let t = to_wide(&format!("Color: #{}", hex));
            let _ = SetWindowTextW(btn, windows::core::PCWSTR(t.as_ptr()));
        }
    }
}

unsafe fn handle_font_picker(hwnd: HWND) {
    let lf = state::with_state(|s| s.settings.logfont_data.clone())
        .unwrap_or_default();
    if let Some(result) = dialogs::pick_font(hwnd, &lf) {
        let new_lf = result.to_logfont_data();
        state::with_state(|s| {
            s.settings.logfont_data = new_lf;
            s.percentage_text_font = result.face_name.clone();
            s.percentage_text_size = result.point_size;
        });
        if let Ok(btn) = GetDlgItem(Some(hwnd), 2008) {
            let t = to_wide(&format!("Font: {}, {}pt", result.face_name, result.point_size));
            let _ = SetWindowTextW(btn, windows::core::PCWSTR(t.as_ptr()));
        }
    }
}

unsafe fn handle_alignment(hwnd: HWND) {
    if let Ok(combo) = GetDlgItem(Some(hwnd), 2004) {
        let sel = SendMessageW(combo, CB_GETCURSEL, None, None).0;
        let align = match sel { 0 => "left", 2 => "right", _ => "center" };
        state::with_state(|s| s.percentage_text_align = align.to_string());
    }
}

unsafe fn handle_edit_int(hwnd: HWND, id: i32, field: &str) {
    if let Some(text) = read_edit_text(hwnd, id) {
        if let Ok(val) = text.parse::<i32>() {
            state::with_state(|s| match field {
                "text_x" => s.percentage_text_x = val,
                "text_y" => s.percentage_text_y = val,
                _ => {}
            });
        }
    }
}

unsafe fn handle_custom_assets_toggle(hwnd: HWND) {
    let enabled = get_checkbox(hwnd, 1007);
    if let (Ok(edit), Ok(btn)) = (GetDlgItem(Some(hwnd), 1005), GetDlgItem(Some(hwnd), 1006)) {
        enable_window(edit, enabled);
        enable_window(btn, enabled);
    }
    // When unchecking, clear the themes_folder in state so "Default" is used
    state::with_state(|s| {
        s.use_custom_theme_folder = enabled;
        if !enabled {
            s.themes_folder = None;
        }
    });
    // Refresh the combobox to show default themes (next to exe)
    refresh_theme_combobox(hwnd);
}

unsafe fn refresh_theme_combobox(hwnd: HWND) {
    let Ok(combo) = GetDlgItem(Some(hwnd), 1016) else { return };
    let (themes_folder, active_theme) = state::with_state(|s| {
        (s.themes_folder.clone(), s.active_theme.clone())
    }).unwrap_or((None, "Default".to_string()));

    // Determine themes root
    let themes_root = themes_folder
        .as_ref()
        .map(|f| std::path::PathBuf::from(f))
        .or_else(crate::engine::default_themes_root);

    let themes = themes_root
        .as_ref()
        .map(|root| crate::engine::scan_themes(root))
        .unwrap_or_else(|| vec!["Default".to_string()]);

    // Clear and repopulate
    SendMessageW(combo, CB_RESETCONTENT, None, None);
    let mut sel_idx = 0usize;
    for (i, theme) in themes.iter().enumerate() {
        let wide = to_wide(theme);
        SendMessageW(combo, CB_ADDSTRING, None, Some(LPARAM(wide.as_ptr() as isize)));
        if theme == &active_theme {
            sel_idx = i;
        }
    }
    SendMessageW(combo, CB_SETCURSEL, Some(WPARAM(sel_idx)), None);
}

unsafe fn handle_theme_combobox(hwnd: HWND) {
    let Ok(combo) = GetDlgItem(Some(hwnd), 1016) else { return };
    let sel = SendMessageW(combo, CB_GETCURSEL, None, None).0;
    if sel < 0 { return; }
    // Read the string at selected index
    let len = SendMessageW(combo, CB_GETLBTEXTLEN, Some(WPARAM(sel as usize)), None).0;
    if len <= 0 { return; }
    let mut buf = vec![0u16; (len + 1) as usize];
    SendMessageW(combo, CB_GETLBTEXT,
        Some(WPARAM(sel as usize)),
        Some(LPARAM(buf.as_mut_ptr() as isize)));
    let name = String::from_utf16_lossy(&buf[..len as usize]);
    state::with_state(|s| s.active_theme = name);
}

unsafe fn handle_browse(hwnd: HWND) {
    if !get_checkbox(hwnd, 1007) { return; }
    if let Ok(folder) = dialogs::pick_folder(hwnd) {
        if let Ok(edit) = GetDlgItem(Some(hwnd), 1005) {
            let t = to_wide(&folder);
            let _ = SetWindowTextW(edit, windows::core::PCWSTR(t.as_ptr()));
        }
        // Store the new themes folder and refresh the combobox
        state::with_state(|s| {
            s.themes_folder = Some(folder);
            s.active_theme = "Default".to_string();
        });
        refresh_theme_combobox(hwnd);
    }
}

unsafe fn handle_ok_button(hwnd: HWND) {
    // Read polling interval (minutes and seconds)
    let mut minutes = 0u64;
    let mut seconds = 0u64;
    if let Some(text) = read_edit_text(hwnd, 1003) {
        if let Ok(val) = text.parse::<u64>() {
            minutes = val;
        }
    }
    if let Some(text) = read_edit_text(hwnd, 1017) {
        if let Ok(val) = text.parse::<u64>() {
            seconds = val;
        }
    }
    // Ensure total is at least 1 second
    let total_seconds = minutes * 60 + seconds;
    if total_seconds < 1 {
        minutes = 1;
        seconds = 0;
    }
    state::with_state(|s| {
        s.polling_interval_minutes = minutes;
        s.polling_interval_seconds = seconds;
    });

    // Read themes folder (only if checkbox is checked)
    let use_custom = get_checkbox(hwnd, 1007);
    let folder = if use_custom {
        read_edit_text(hwnd, 1005).filter(|t| !t.trim().is_empty())
    } else { None };
    state::with_state(|s| s.themes_folder = folder);
    // Read active theme from combobox (already tracked live via handle_theme_combobox,
    // but re-read here to be safe in case selection changed without CBN_SELCHANGE)
    if let Ok(combo) = GetDlgItem(Some(hwnd), 1016) {
        let sel = SendMessageW(combo, CB_GETCURSEL, None, None).0;
        if sel >= 0 {
            let len = SendMessageW(combo, CB_GETLBTEXTLEN, Some(WPARAM(sel as usize)), None).0;
            if len > 0 {
                let mut buf = vec![0u16; (len + 1) as usize];
                SendMessageW(combo, CB_GETLBTEXT,
                    Some(WPARAM(sel as usize)),
                    Some(LPARAM(buf.as_mut_ptr() as isize)));
                let name = String::from_utf16_lossy(&buf[..len as usize]);
                state::with_state(|s| s.active_theme = name);
            }
        }
    }
    // Read devices tab (visibility)
    devices_tab::read_devices_tab(hwnd);
    let _ = DestroyWindow(hwnd);
}

unsafe fn handle_notify(hwnd: HWND, lparam: LPARAM) -> LRESULT {
    let nmhdr = *(lparam.0 as *const NMHDR);
    if nmhdr.idFrom == 3000 && (nmhdr.code as i32) == TCN_SELCHANGE {
        handle_tab_switch(hwnd);
    }
    LRESULT(0)
}

unsafe fn handle_tab_switch(hwnd: HWND) {
    if let Ok(tab) = GetDlgItem(Some(hwnd), 3000) {
        let idx = SendMessageW(tab, TCM_GETCURSEL, None, None).0;
        let (show_gen, show_txt, show_dev) = match idx {
            0 => (true, false, false),
            1 => (false, true, false),
            2 => (false, false, true),
            _ => (true, false, false),
        };
        for &id in GENERAL_TAB_IDS { show_hide_control(hwnd, id, show_gen); }
        for &id in TEXT_TAB_IDS { show_hide_control(hwnd, id, show_txt); }
        let dev_ids = devices_tab::devices_tab_ids();
        for id in dev_ids { show_hide_control(hwnd, id, show_dev); }
    }
}

