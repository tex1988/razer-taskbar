pub mod devices_tab;
pub mod dialogs;
pub mod event_handlers;
pub mod ffi_types;
pub mod general_tab;
pub mod helpers;
pub mod state;
pub mod text_tab;

use anyhow::Result;
use crate::model::Settings;
use windows::Win32::Foundation::{HWND, LPARAM};
use windows::Win32::UI::WindowsAndMessaging::*;
use windows::Win32::UI::Controls::{TCITEMW, TCM_INSERTITEMW, TCIF_TEXT, TCS_TABS};

pub struct SettingsWindow;

impl SettingsWindow {
    pub unsafe fn show(current_settings: Settings) -> Result<bool> {
        state::init_state(&current_settings);
        let hwnd = create_main_window()?;
        set_window_icon(hwnd);
        let tab_control = create_tab_control(hwnd)?;
        add_tabs(tab_control);
        general_tab::create_general_tab(hwnd, &current_settings)?;
        text_tab::create_text_tab(hwnd, &current_settings)?;
        devices_tab::create_devices_tab(hwnd)?;
        create_ok_button(hwnd)?;
        hide_text_tab_initially(hwnd);
        hide_devices_tab_initially(hwnd);

        let _ = ShowWindow(hwnd, SW_SHOW);
        let _ = windows::Win32::Graphics::Gdi::UpdateWindow(hwnd);

        run_message_loop(hwnd);
        state::save_if_changed(&current_settings)
    }
}

unsafe fn create_main_window() -> Result<HWND> {
    let class_name = windows::core::w!("RazerSettingsWindow");
    let wc = WNDCLASSW {
        lpfnWndProc: Some(event_handlers::settings_wnd_proc),
        lpszClassName: class_name,
        hCursor: LoadCursorW(None, IDC_ARROW)?,
        hbrBackground: windows::Win32::Graphics::Gdi::HBRUSH(
            ((windows::Win32::Graphics::Gdi::COLOR_3DFACE.0 + 1) as isize) as *mut _,
        ),
        ..Default::default()
    };
    let _ = RegisterClassW(&wc);

    let mut cursor = windows::Win32::Foundation::POINT { x: 0, y: 0 };
    let _ = GetCursorPos(&mut cursor);
    let (w, h) = (450, 435);

    Ok(CreateWindowExW(
        WINDOW_EX_STYLE::default(), class_name,
        windows::core::w!("Razer Taskbar - Settings"),
        WS_OVERLAPPED | WS_CAPTION | WS_SYSMENU,
        cursor.x - w / 2, cursor.y - h - 10, w, h,
        None, None, None, None,
    )?)
}

unsafe fn set_window_icon(hwnd: HWND) {
    let hmod = windows::Win32::System::LibraryLoader::GetModuleHandleW(None).ok();
    let hmod = match hmod { Some(h) => h, None => return };
    let hinst = windows::Win32::Foundation::HINSTANCE(hmod.0);
    let res_id = windows::core::PCWSTR(1 as *const u16);
    for (sz, icon_type) in [(32i32, 1usize), (16, 0)] {
        if let Ok(icon) = LoadImageW(Some(hinst), res_id, IMAGE_ICON, sz, sz, LR_DEFAULTCOLOR | LR_SHARED) {
            SendMessageW(hwnd, WM_SETICON, Some(windows::Win32::Foundation::WPARAM(icon_type)), Some(windows::Win32::Foundation::LPARAM(icon.0 as isize)));
        }
    }
}

unsafe fn create_tab_control(hwnd: HWND) -> Result<HWND> {
    Ok(CreateWindowExW(
        WINDOW_EX_STYLE::default(),
        windows::core::w!("SysTabControl32"), windows::core::w!(""),
        WS_VISIBLE | WS_CHILD | WINDOW_STYLE(TCS_TABS as u32),
        10, 10, 420, 345,
        Some(hwnd), Some(HMENU(3000isize as *mut _)), None, None,
    )?)
}

unsafe fn add_tabs(tab: HWND) {
    let mut tie = TCITEMW {
        mask: TCIF_TEXT, dwState: Default::default(), dwStateMask: Default::default(),
        pszText: windows::core::PWSTR(windows::core::w!("General").as_ptr() as *mut u16),
        cchTextMax: 0, iImage: 0, lParam: LPARAM(0),
    };
    SendMessageW(tab, TCM_INSERTITEMW, Some(windows::Win32::Foundation::WPARAM(0)), Some(LPARAM(&tie as *const _ as isize)));
    tie.pszText = windows::core::PWSTR(windows::core::w!("Percentage Text").as_ptr() as *mut u16);
    SendMessageW(tab, TCM_INSERTITEMW, Some(windows::Win32::Foundation::WPARAM(1)), Some(LPARAM(&tie as *const _ as isize)));
    tie.pszText = windows::core::PWSTR(windows::core::w!("Devices").as_ptr() as *mut u16);
    SendMessageW(tab, TCM_INSERTITEMW, Some(windows::Win32::Foundation::WPARAM(2)), Some(LPARAM(&tie as *const _ as isize)));
}

unsafe fn create_ok_button(hwnd: HWND) -> Result<()> {
    CreateWindowExW(
        WINDOW_EX_STYLE::default(), windows::core::w!("BUTTON"), windows::core::w!("OK"),
        WS_VISIBLE | WS_CHILD | WINDOW_STYLE(BS_DEFPUSHBUTTON as u32),
        177, 360, 80, 30,
        Some(hwnd), Some(HMENU(9001isize as *mut _)), None, None,
    )?;
    Ok(())
}

unsafe fn hide_text_tab_initially(hwnd: HWND) {
    for &id in text_tab::TEXT_TAB_IDS {
        helpers::show_hide_control(hwnd, id, false);
    }
}

unsafe fn hide_devices_tab_initially(hwnd: HWND) {
    for id in devices_tab::devices_tab_ids() {
        helpers::show_hide_control(hwnd, id, false);
    }
}

unsafe fn run_message_loop(hwnd: HWND) {
    let mut msg = MSG::default();
    while GetMessageW(&mut msg, None, 0, 0).as_bool() {
        let _ = TranslateMessage(&msg);
        DispatchMessageW(&msg);
        if !IsWindow(Some(hwnd)).as_bool() { break; }
    }
}




