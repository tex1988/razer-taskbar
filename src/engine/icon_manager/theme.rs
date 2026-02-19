use std::sync::Mutex;
use std::sync::atomic::{AtomicBool, Ordering};
use lazy_static::lazy_static;

lazy_static! {
    pub(crate) static ref ICON_THEME: Mutex<String> = Mutex::new("dark".to_string());
}

pub(crate) static SYSTEM_THEME_CHANGED: AtomicBool = AtomicBool::new(false);

// ── Public API ─────────────────────────────────────────────────

pub fn create_theme_change_listener() {
    use windows::Win32::UI::WindowsAndMessaging::*;
    use windows::Win32::Foundation::{HWND, LPARAM, LRESULT, WPARAM};

    unsafe extern "system" fn wnd_proc(
        hwnd: HWND, msg: u32, wparam: WPARAM, lparam: LPARAM,
    ) -> LRESULT {
        if msg == 0x001A { flag_theme_change(lparam); }
        unsafe { DefWindowProcW(hwnd, msg, wparam, lparam) }
    }

    unsafe {
        use windows::core::w;
        let class_name = w!("RazerTaskbarThemeWatcher");
        let wc = WNDCLASSW {
            lpfnWndProc: Some(wnd_proc),
            lpszClassName: class_name,
            ..Default::default()
        };
        RegisterClassW(&wc);
        let _ = CreateWindowExW(
            WS_EX_TOOLWINDOW, class_name, w!(""),
            WINDOW_STYLE(0), 0, 0, 0, 0, None, None, None, None,
        );
    }
}

fn flag_theme_change(lparam: windows::Win32::Foundation::LPARAM) {
    let param_name = if lparam.0 != 0 {
        let ptr = lparam.0 as *const u16;
        let mut len = 0usize;
        unsafe { while *ptr.add(len) != 0 { len += 1; } }
        String::from_utf16_lossy(unsafe {
            std::slice::from_raw_parts(ptr, len)
        })
    } else {
        String::new()
    };
    if param_name == "ImmersiveColorSet" || param_name.is_empty() {
        SYSTEM_THEME_CHANGED.store(true, Ordering::Relaxed);
    }
}

pub fn consume_system_theme_changed() -> bool {
    SYSTEM_THEME_CHANGED.swap(false, Ordering::Relaxed)
}

pub fn set_icon_theme(theme: &str) -> bool {
    let resolved = if theme == "system" { detect_system_theme() } else { theme };
    let mut lock = ICON_THEME.lock().unwrap();
    let changed = *lock != resolved;
    if changed {
        *lock = resolved.to_string();
        drop(lock);
        *super::assets::BATTERY_RANGES.lock().unwrap() = None;
    }
    changed
}

pub(crate) fn get_icon_theme() -> String {
    ICON_THEME.lock().unwrap().clone()
}

fn detect_system_theme() -> &'static str {
    use winreg::RegKey;
    use winreg::enums::*;
    let hkcu = RegKey::predef(HKEY_CURRENT_USER);
    if let Some(key) = hkcu
        .open_subkey(r"Software\Microsoft\Windows\CurrentVersion\Themes\Personalize")
        .ok()
    {
        if let Ok(v) = key.get_value::<u32, _>("AppsUseLightTheme") {
            return if v == 0 { "dark" } else { "light" };
        }
    }
    "dark"
}

