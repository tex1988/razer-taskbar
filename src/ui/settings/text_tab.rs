use anyhow::Result;
use windows::Win32::Foundation::{HWND, LPARAM, WPARAM};
use windows::Win32::UI::WindowsAndMessaging::*;
use crate::model::Settings;
use crate::util::to_wide;
use super::helpers::{set_checkbox, compute_point_size};

pub const TEXT_TAB_IDS: &[i32] = &[2001, 2003, 2004, 2005, 2006, 2007, 2008, 2011, 2012, 2013, 2014];

pub unsafe fn create_text_tab(hwnd: HWND, s: &Settings) -> Result<()> {
    create_show_percentage(hwnd, s)?;
    create_color_and_font(hwnd, s)?;
    create_alignment_controls(hwnd, s)?;
    create_position_controls(hwnd, s)?;
    create_percent_symbol(hwnd, s)?;
    Ok(())
}

unsafe fn create_show_percentage(hwnd: HWND, s: &Settings) -> Result<()> {
    CreateWindowExW(WINDOW_EX_STYLE::default(), windows::core::w!("BUTTON"),
        windows::core::w!("Show percentage"),
        WS_CHILD | WINDOW_STYLE(BS_AUTOCHECKBOX as u32),
        30, 55, 200, 20, Some(hwnd), Some(HMENU(2001isize as *mut _)), None, None)?;
    set_checkbox(hwnd, 2001, s.show_percentage);
    Ok(())
}

unsafe fn create_color_and_font(hwnd: HWND, s: &Settings) -> Result<()> {
    // Color label
    CreateWindowExW(WINDOW_EX_STYLE::default(), windows::core::w!("STATIC"),
        windows::core::w!("Text color:"), WS_CHILD,
        30, 90, 80, 20, Some(hwnd), Some(HMENU(2011isize as *mut _)), None, None)?;
    // Color picker button
    CreateWindowExW(WINDOW_EX_STYLE::default(), windows::core::w!("BUTTON"),
        windows::core::w!("Pick Color"),
        WS_CHILD | WINDOW_STYLE(BS_PUSHBUTTON as u32),
        115, 88, 105, 22, Some(hwnd), Some(HMENU(2003isize as *mut _)), None, None)?;
    // Font button
    let point_size = compute_point_size(hwnd, s.logfont_data.lf_height);
    let text = format!("Font: {}, {}pt", s.logfont_data.lf_face_name, point_size);
    let wide = to_wide(&text);
    CreateWindowExW(WINDOW_EX_STYLE::default(), windows::core::w!("BUTTON"),
        windows::core::PCWSTR(wide.as_ptr()),
        WS_CHILD | WINDOW_STYLE(BS_PUSHBUTTON as u32),
        230, 88, 195, 22, Some(hwnd), Some(HMENU(2008isize as *mut _)), None, None)?;
    Ok(())
}

unsafe fn create_alignment_controls(hwnd: HWND, s: &Settings) -> Result<()> {
    CreateWindowExW(WINDOW_EX_STYLE::default(), windows::core::w!("STATIC"),
        windows::core::w!("Alignment:"), WS_CHILD,
        30, 125, 100, 20, Some(hwnd), Some(HMENU(2012isize as *mut _)), None, None)?;

    let combo = CreateWindowExW(WINDOW_EX_STYLE::default(), windows::core::w!("COMBOBOX"),
        windows::core::w!(""),
        WS_CHILD | WINDOW_STYLE(CBS_DROPDOWNLIST as u32 | WS_VSCROLL.0),
        140, 123, 100, 100, Some(hwnd), Some(HMENU(2004isize as *mut _)), None, None)?;

    for label in [windows::core::w!("Left"), windows::core::w!("Center"), windows::core::w!("Right")] {
        SendMessageW(combo, CB_ADDSTRING, None, Some(LPARAM(label.as_ptr() as isize)));
    }
    let sel = match s.percentage_text_align.as_str() {
        "left" => 0, "right" => 2, _ => 1,
    };
    SendMessageW(combo, CB_SETCURSEL, Some(WPARAM(sel)), None);
    Ok(())
}

unsafe fn create_position_controls(hwnd: HWND, s: &Settings) -> Result<()> {
    // X
    CreateWindowExW(WINDOW_EX_STYLE::default(), windows::core::w!("STATIC"),
        windows::core::w!("Position X (0=auto):"), WS_CHILD,
        30, 160, 130, 20, Some(hwnd), Some(HMENU(2013isize as *mut _)), None, None)?;
    let px = CreateWindowExW(WS_EX_CLIENTEDGE, windows::core::w!("EDIT"), windows::core::w!(""),
        WS_CHILD | WS_BORDER | WINDOW_STYLE(ES_NUMBER as u32),
        165, 158, 60, 22, Some(hwnd), Some(HMENU(2005isize as *mut _)), None, None)?;
    set_edit_int(px, s.percentage_text_x);
    // Y
    CreateWindowExW(WINDOW_EX_STYLE::default(), windows::core::w!("STATIC"),
        windows::core::w!("Position Y:"), WS_CHILD,
        30, 195, 100, 20, Some(hwnd), Some(HMENU(2014isize as *mut _)), None, None)?;
    let py = CreateWindowExW(WS_EX_CLIENTEDGE, windows::core::w!("EDIT"), windows::core::w!(""),
        WS_CHILD | WS_BORDER | WINDOW_STYLE(ES_NUMBER as u32),
        140, 193, 60, 22, Some(hwnd), Some(HMENU(2006isize as *mut _)), None, None)?;
    set_edit_int(py, s.percentage_text_y);
    Ok(())
}

unsafe fn set_edit_int(edit: HWND, val: i32) {
    let txt = to_wide(&format!("{}", val));
    let _ = SetWindowTextW(edit, windows::core::PCWSTR(txt.as_ptr()));
}

unsafe fn create_percent_symbol(hwnd: HWND, s: &Settings) -> Result<()> {
    CreateWindowExW(WINDOW_EX_STYLE::default(), windows::core::w!("BUTTON"),
        windows::core::w!("Show '%' symbol"),
        WS_CHILD | WINDOW_STYLE(BS_AUTOCHECKBOX as u32),
        30, 228, 200, 20, Some(hwnd), Some(HMENU(2007isize as *mut _)), None, None)?;
    set_checkbox(hwnd, 2007, s.show_percent_symbol);
    Ok(())
}


