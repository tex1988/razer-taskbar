use windows::Win32::Foundation::{HWND, LPARAM, WPARAM};
use windows::Win32::Graphics::Gdi::{InvalidateRect, UpdateWindow, GetDC, GetDeviceCaps, ReleaseDC, LOGPIXELSY};
use windows::Win32::UI::WindowsAndMessaging::*;

/// Helper function to enable or disable a window control
pub unsafe fn enable_window(hwnd: HWND, enable: bool) {
    let current_style = GetWindowLongW(hwnd, GWL_STYLE);
    let new_style = if enable {
        current_style & !WS_DISABLED.0 as i32
    } else {
        current_style | WS_DISABLED.0 as i32
    };
    SetWindowLongW(hwnd, GWL_STYLE, new_style);
    set_edit_readonly(hwnd, enable);
    let _ = InvalidateRect(Some(hwnd), None, true);
    let _ = UpdateWindow(hwnd);
}

unsafe fn set_edit_readonly(hwnd: HWND, enable: bool) {
    let mut class_buf: [u16; 256] = [0; 256];
    let len = GetClassNameW(hwnd, &mut class_buf);
    if len > 0 {
        let class_name = String::from_utf16_lossy(&class_buf[0..len as usize]);
        if class_name.eq_ignore_ascii_case("Edit") {
            SendMessageW(hwnd, 0x00CF, Some(WPARAM(if enable { 0 } else { 1 })), None);
        }
    }
}

/// Helper function to show or hide a window control
pub unsafe fn show_hide_control(parent: HWND, control_id: i32, show: bool) {
    if let Ok(control) = GetDlgItem(Some(parent), control_id) {
        let _ = ShowWindow(control, if show { SW_SHOW } else { SW_HIDE });
    }
}

/// Read text from an EDIT control, returning trimmed string
pub unsafe fn read_edit_text(parent: HWND, control_id: i32) -> Option<String> {
    let edit = GetDlgItem(Some(parent), control_id).ok()?;
    let text_len = GetWindowTextLengthW(edit);
    if text_len <= 0 { return None; }
    let mut buffer: Vec<u16> = vec![0; (text_len + 1) as usize];
    GetWindowTextW(edit, &mut buffer);
    let text = String::from_utf16_lossy(&buffer);
    Some(text.trim_end_matches('\0').to_string())
}

/// Set a checkbox check state
pub unsafe fn set_checkbox(parent: HWND, id: i32, checked: bool) {
    if let Ok(cb) = GetDlgItem(Some(parent), id) {
        let val = if checked { Some(WPARAM(1)) } else { Some(WPARAM(0)) };
        SendMessageW(cb, BM_SETCHECK, val, Some(LPARAM(0)));
    }
}

/// Get a checkbox check state
pub unsafe fn get_checkbox(parent: HWND, id: i32) -> bool {
    if let Ok(cb) = GetDlgItem(Some(parent), id) {
        SendMessageW(cb, BM_GETCHECK, Some(WPARAM(0)), Some(LPARAM(0))).0 != 0
    } else {
        false
    }
}

/// Compute font point size from LOGFONT height using DPI
pub unsafe fn compute_point_size(hwnd: HWND, lf_height: i32) -> u32 {
    let hdc = GetDC(Some(hwnd));
    let dpi_y = GetDeviceCaps(Some(hdc), LOGPIXELSY);
    let _ = ReleaseDC(Some(hwnd), hdc);
    if lf_height < 0 {
        ((-lf_height * 72) / dpi_y) as u32
    } else {
        20
    }
}

