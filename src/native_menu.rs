#![allow(dead_code)]

use anyhow::Result;
use windows::Win32::{
    Foundation::{HWND, LPARAM, LRESULT, WPARAM},
    UI::WindowsAndMessaging::*,
};

const WM_APP_TRAY: u32 = WM_APP + 1;
const ID_QUIT: u32 = 1001;

pub struct NativeContextMenu {
    hwnd: HWND,
    hmenu: HMENU,
}

impl NativeContextMenu {
    pub fn new() -> Result<Self> {
        unsafe {
            // Create a hidden window to handle menu messages
            let class_name = w!("RazerTaskbarMenuClass");

            let wc = WNDCLASSW {
                lpfnWndProc: Some(menu_wnd_proc),
                lpszClassName: class_name,
                ..Default::default()
            };

            RegisterClassW(&wc);

            let hwnd = CreateWindowExW(
                WINDOW_EX_STYLE::default(),
                class_name,
                w!("Razer Taskbar Menu"),
                WINDOW_STYLE::default(),
                0, 0, 0, 0,
                None,
                None,
                None,
                None,
            );

            if hwnd.0 == 0 {
                return Err(anyhow::anyhow!("Failed to create menu window"));
            }

            // Create popup menu
            let hmenu = CreatePopupMenu()?;

            // Enable dark mode for the menu (Windows 10 1809+)
            #[allow(non_snake_case)]
            let DWMWA_USE_IMMERSIVE_DARK_MODE = 20;
            let use_dark_mode: i32 = 1;

            // Try to set dark mode (will fail on older Windows, that's ok)
            let _ = windows::Win32::Graphics::Dwm::DwmSetWindowAttribute(
                hwnd,
                windows::Win32::Graphics::Dwm::DWMWINDOWATTRIBUTE(DWMWA_USE_IMMERSIVE_DARK_MODE),
                &use_dark_mode as *const _ as *const std::ffi::c_void,
                std::mem::size_of::<i32>() as u32,
            );

            // Add menu items
            let mii = MENUITEMINFOW {
                cbSize: std::mem::size_of::<MENUITEMINFOW>() as u32,
                fMask: MIIM_ID | MIIM_STRING | MIIM_STATE,
                wID: ID_QUIT,
                fState: MFS_ENABLED,
                dwTypeData: *(w!("Quit").as_ptr() as *mut _),
                ..Default::default()
            };

            InsertMenuItemW(hmenu, 0, true, &mii)?;

            Ok(Self { hwnd, hmenu })
        }
    }

    pub fn show(&self, x: i32, y: i32) -> Result<()> {
        unsafe {
            SetForegroundWindow(self.hwnd);

            let result = TrackPopupMenu(
                self.hmenu,
                TPM_LEFTALIGN | TPM_BOTTOMALIGN,
                x,
                y,
                0,
                self.hwnd,
                None,
            );

            if !result.as_bool() {
                return Err(anyhow::anyhow!("Failed to show menu"));
            }

            PostMessageW(self.hwnd, WM_NULL, WPARAM(0), LPARAM(0))?;

            Ok(())
        }
    }

    pub fn check_quit_clicked() -> bool {
        unsafe {
            static mut QUIT_CLICKED: bool = false;
            let result = QUIT_CLICKED;
            QUIT_CLICKED = false;
            result
        }
    }
}

unsafe extern "system" fn menu_wnd_proc(
    hwnd: HWND,
    msg: u32,
    wparam: WPARAM,
    lparam: LPARAM,
) -> LRESULT {
    match msg {
        WM_COMMAND => {
            let id = (wparam.0 & 0xFFFF) as u32;
            if id == ID_QUIT {
                static mut QUIT_CLICKED: bool = false;
                QUIT_CLICKED = true;
            }
            LRESULT(0)
        }
        _ => DefWindowProcW(hwnd, msg, wparam, lparam),
    }
}

// Wide string literal macro
macro_rules! w {
    ($s:expr) => {{
        const STR: &str = concat!($s, '\0');
        const WIDE: &[u16] = &{
            let bytes = STR.as_bytes();
            let mut wide = [0u16; STR.len()];
            let mut i = 0;
            while i < bytes.len() {
                wide[i] = bytes[i] as u16;
                i += 1;
            }
            wide
        };
        windows::core::PCWSTR::from_raw(WIDE.as_ptr())
    }};
}

use w;
