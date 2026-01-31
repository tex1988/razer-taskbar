use anyhow::Result;
use crate::settings::Settings;
use windows::Win32::{
    Foundation::{HWND, LPARAM, LRESULT, WPARAM},
    Graphics::Gdi::*,
    UI::WindowsAndMessaging::*,
};
use std::sync::{Arc, Mutex};

static mut SETTINGS_STATE: Option<Arc<Mutex<SettingsWindowState>>> = None;

struct SettingsWindowState {
    show_percentage: bool,
    polling_interval_minutes: u64,
    run_at_startup: bool,
    settings: Settings,
}

pub struct SettingsWindow;

impl SettingsWindow {
    pub unsafe fn show(current_settings: Settings) -> Result<bool> {
        // Initialize global state
        SETTINGS_STATE = Some(Arc::new(Mutex::new(SettingsWindowState {
            show_percentage: current_settings.show_percentage,
            polling_interval_minutes: current_settings.polling_interval_minutes,
            run_at_startup: current_settings.run_at_startup,
            settings: current_settings.clone(),
        })));

        // Register window class
        let class_name = windows::core::w!("RazerSettingsWindow");
        let wc = WNDCLASSW {
            lpfnWndProc: Some(settings_wnd_proc),
            lpszClassName: class_name,
            hCursor: LoadCursorW(None, IDC_ARROW)?,
            hbrBackground: HBRUSH((COLOR_WINDOW.0 + 1) as isize),
            ..Default::default()
        };
        let _ = RegisterClassW(&wc);

        // Get cursor position (near tray icon) and calculate window position
        let mut cursor_pos = windows::Win32::Foundation::POINT { x: 0, y: 0 };
        let _ = GetCursorPos(&mut cursor_pos);

        // Window dimensions
        let window_width = 400;
        let window_height = 320;

        // Position window above the cursor/tray icon
        // Offset to the left to center it, and move up by window height + some padding
        let window_x = cursor_pos.x - (window_width / 2);
        let window_y = cursor_pos.y - window_height - 10; // 10px padding above cursor

        // Create settings window
        let hwnd = CreateWindowExW(
            WINDOW_EX_STYLE::default(),
            class_name,
            windows::core::w!("Razer Taskbar - Settings"),
            WS_OVERLAPPED | WS_CAPTION | WS_SYSMENU,
            window_x,
            window_y,
            window_width,
            window_height,
            None,
            None,
            None,
            None,
        );

        if hwnd.0 == 0 {
            return Err(anyhow::anyhow!("Failed to create settings window"));
        }

        // Create checkbox for "Show percentage"
        CreateWindowExW(
            WINDOW_EX_STYLE::default(),
            windows::core::w!("BUTTON"),
            windows::core::w!("Show percentage icons instead of standard icons"),
            WS_VISIBLE | WS_CHILD | WINDOW_STYLE(BS_AUTOCHECKBOX as u32),
            20,
            30,
            350,
            25,
            hwnd,
            HMENU(1001isize),
            None,
            None,
        );

        // Set checkbox state based on current settings
        let checkbox = GetDlgItem(hwnd, 1001);
        if checkbox.0 != 0 {
            let check_state = if current_settings.show_percentage {
                WPARAM(1) // BST_CHECKED
            } else {
                WPARAM(0) // BST_UNCHECKED
            };
            SendMessageW(checkbox, BM_SETCHECK, check_state, LPARAM(0));
        }


        // Create label for polling interval
        CreateWindowExW(
            WINDOW_EX_STYLE::default(),
            windows::core::w!("STATIC"),
            windows::core::w!("Log polling interval (minutes):"),
            WS_VISIBLE | WS_CHILD,
            20,
            75,
            250,
            20,
            hwnd,
            None,
            None,
            None,
        );

        // Create text input for polling interval
        let edit_hwnd = CreateWindowExW(
            WS_EX_CLIENTEDGE,
            windows::core::w!("EDIT"),
            windows::core::w!(""),
            WS_VISIBLE | WS_CHILD | WS_BORDER | WINDOW_STYLE(ES_NUMBER as u32),
            20,
            100,
            100,
            25,
            hwnd,
            HMENU(1003isize),
            None,
            None,
        );

        // Set the current polling interval value
        let interval_text: Vec<u16> = format!("{}", current_settings.polling_interval_minutes)
            .encode_utf16()
            .chain(Some(0))
            .collect();
        let _ = SetWindowTextW(edit_hwnd, windows::core::PCWSTR(interval_text.as_ptr()));

        // Create checkbox for "Run at startup"
        CreateWindowExW(
            WINDOW_EX_STYLE::default(),
            windows::core::w!("BUTTON"),
            windows::core::w!("Start with Windows (autostart)"),
            WS_VISIBLE | WS_CHILD | WINDOW_STYLE(BS_AUTOCHECKBOX as u32),
            20,
            140,
            350,
            25,
            hwnd,
            HMENU(1004isize),
            None,
            None,
        );

        // Set checkbox state for run at startup
        let checkbox_startup = GetDlgItem(hwnd, 1004);
        if checkbox_startup.0 != 0 {
            let check_state = if current_settings.run_at_startup {
                WPARAM(1)
            } else {
                WPARAM(0)
            };
            SendMessageW(checkbox_startup, BM_SETCHECK, check_state, LPARAM(0));
        }

        // Create OK button
        CreateWindowExW(
            WINDOW_EX_STYLE::default(),
            windows::core::w!("BUTTON"),
            windows::core::w!("OK"),
            WS_VISIBLE | WS_CHILD | WINDOW_STYLE(BS_DEFPUSHBUTTON as u32),
            150,
            200,
            80,
            30,
            hwnd,
            HMENU(2001isize),
            None,
            None,
        );

        ShowWindow(hwnd, SW_SHOW);
        let _ = UpdateWindow(hwnd);

        // Message loop
        let mut msg = MSG::default();
        let mut settings_changed = false;

        while GetMessageW(&mut msg, None, 0, 0).as_bool() {
            TranslateMessage(&msg);
            DispatchMessageW(&msg);

            // Check if window was closed
            if !IsWindow(hwnd).as_bool() {
                break;
            }
        }

        // Get final settings state
        if let Some(state) = unsafe { (*std::ptr::addr_of!(SETTINGS_STATE)).as_ref() } {
            let state_guard = state.lock().unwrap();
            settings_changed = state_guard.show_percentage != current_settings.show_percentage
                || state_guard.polling_interval_minutes != current_settings.polling_interval_minutes
                || state_guard.run_at_startup != current_settings.run_at_startup;

            // Save settings if changed
            if settings_changed {
                let mut new_settings = state_guard.settings.clone();
                new_settings.show_percentage = state_guard.show_percentage;
                new_settings.polling_interval_minutes = state_guard.polling_interval_minutes;
                new_settings.run_at_startup = state_guard.run_at_startup;

                // Apply autostart registry change
                if state_guard.run_at_startup != current_settings.run_at_startup {
                    if let Err(e) = crate::startup::set_startup(state_guard.run_at_startup) {
                        eprintln!("Failed to set autostart: {}", e);
                    }
                }

                new_settings.save()?;
            }
        }

        SETTINGS_STATE = None;
        Ok(settings_changed)
    }
}

unsafe extern "system" fn settings_wnd_proc(
    hwnd: HWND,
    msg: u32,
    wparam: WPARAM,
    lparam: LPARAM,
) -> LRESULT {
    match msg {
        WM_COMMAND => {
            let control_id = (wparam.0 & 0xFFFF) as i32;
            let notification = ((wparam.0 >> 16) & 0xFFFF) as u32;

            match control_id {
                1001 => {
                    // First checkbox clicked (show percentage icons)
                    if notification == BN_CLICKED {
                        let checkbox = GetDlgItem(hwnd, 1001);
                        let check_state = SendMessageW(checkbox, BM_GETCHECK, WPARAM(0), LPARAM(0));

                        if let Some(state) = unsafe { (*std::ptr::addr_of!(SETTINGS_STATE)).as_ref() } {
                            let mut state_guard = state.lock().unwrap();
                            state_guard.show_percentage = check_state.0 != 0;
                        }
                    }
                }
                1004 => {
                    // Autostart checkbox clicked
                    if notification == BN_CLICKED {
                        let checkbox = GetDlgItem(hwnd, 1004);
                        let check_state = SendMessageW(checkbox, BM_GETCHECK, WPARAM(0), LPARAM(0));

                        if let Some(state) = unsafe { (*std::ptr::addr_of!(SETTINGS_STATE)).as_ref() } {
                            let mut state_guard = state.lock().unwrap();
                            state_guard.run_at_startup = check_state.0 != 0;
                        }
                    }
                }
                2001 => {
                    // OK button clicked - read the text input value
                    let edit_hwnd = GetDlgItem(hwnd, 1003);
                    if edit_hwnd.0 != 0 {
                        // Get text length
                        let text_len = GetWindowTextLengthW(edit_hwnd);
                        if text_len > 0 {
                            // Read text
                            let mut buffer: Vec<u16> = vec![0; (text_len + 1) as usize];
                            GetWindowTextW(edit_hwnd, &mut buffer);

                            // Convert to string and parse
                            let text = String::from_utf16_lossy(&buffer);
                            let text = text.trim_end_matches('\0');

                            if let Ok(interval) = text.parse::<u64>() {
                                // Ensure interval is at least 1 minute
                                let interval = interval.max(1);

                                if let Some(state) = unsafe { (*std::ptr::addr_of!(SETTINGS_STATE)).as_ref() } {
                                    let mut state_guard = state.lock().unwrap();
                                    state_guard.polling_interval_minutes = interval;
                                }
                            }
                        }
                    }

                    let _ = DestroyWindow(hwnd);
                }
                _ => {}
            }
            LRESULT(0)
        }
        WM_CLOSE => {
            let _ = DestroyWindow(hwnd);
            LRESULT(0)
        }
        WM_DESTROY => {
            PostQuitMessage(0);
            LRESULT(0)
        }
        _ => DefWindowProcW(hwnd, msg, wparam, lparam),
    }
}
