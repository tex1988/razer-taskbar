use anyhow::Result;
use crate::settings::Settings;
use windows::Win32::{
    Foundation::{HWND, LPARAM, LRESULT, WPARAM},
    Graphics::Gdi::*,
    UI::WindowsAndMessaging::*,
    UI::Shell::{SHBrowseForFolderW, SHGetPathFromIDListW, BROWSEINFOW, BIF_RETURNONLYFSDIRS, BIF_NEWDIALOGSTYLE},
    System::Com::{CoInitializeEx, CoUninitialize, COINIT_APARTMENTTHREADED},
};
use std::sync::{Arc, Mutex};

static mut SETTINGS_STATE: Option<Arc<Mutex<SettingsWindowState>>> = None;

struct SettingsWindowState {
    show_percentage: bool,
    polling_interval_minutes: u64,
    run_at_startup: bool,
    use_custom_assets: bool,
    custom_assets_folder: Option<String>,
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
            use_custom_assets: current_settings.custom_assets_folder.is_some(),
            custom_assets_folder: current_settings.custom_assets_folder.clone(),
            settings: current_settings.clone(),
        })));

        // Register window class
        let class_name = windows::core::w!("RazerSettingsWindow");
        let wc = WNDCLASSW {
            lpfnWndProc: Some(settings_wnd_proc),
            lpszClassName: class_name,
            hCursor: LoadCursorW(None, IDC_ARROW)?,
            hbrBackground: HBRUSH(((COLOR_3DFACE.0 + 1) as isize) as *mut _),
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
        )?;

        // Load and set the application icon for the window
        let hmodule = windows::Win32::System::LibraryLoader::GetModuleHandleW(None)?;
        let hinstance = windows::Win32::Foundation::HINSTANCE(hmodule.0);

        // Load the large icon (32x32) - IDI_APPLICATION is typically ID 1 for the main icon
        if let Ok(large_icon) = windows::Win32::UI::WindowsAndMessaging::LoadImageW(
            Some(hinstance),
            windows::core::PCWSTR(1 as *const u16), // Resource ID 1 is the first icon
            IMAGE_ICON,
            32,
            32,
            LR_DEFAULTCOLOR | LR_SHARED,
        ) {
            SendMessageW(hwnd, WM_SETICON, Some(WPARAM(1)), Some(LPARAM(large_icon.0 as isize))); // ICON_BIG
        }

        // Load the small icon (16x16)
        if let Ok(small_icon) = windows::Win32::UI::WindowsAndMessaging::LoadImageW(
            Some(hinstance),
            windows::core::PCWSTR(1 as *const u16), // Resource ID 1
            IMAGE_ICON,
            16,
            16,
            LR_DEFAULTCOLOR | LR_SHARED,
        ) {
            SendMessageW(hwnd, WM_SETICON, Some(WPARAM(0)), Some(LPARAM(small_icon.0 as isize))); // ICON_SMALL
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
            Some(hwnd),
            Some(HMENU(1001isize as *mut _)),
            None,
            None,
        )?;

        // Set checkbox state based on current settings
        let checkbox = GetDlgItem(Some(hwnd), 1001)?;
        let check_state = if current_settings.show_percentage {
            Some(WPARAM(1)) // BST_CHECKED
        } else {
            Some(WPARAM(0)) // BST_UNCHECKED
        };
        SendMessageW(checkbox, BM_SETCHECK, check_state, Some(LPARAM(0)));

        // Create checkbox for custom assets folder with full label text
        CreateWindowExW(
            WINDOW_EX_STYLE::default(),
            windows::core::w!("BUTTON"),
            windows::core::w!("Use custom assets"),
            WS_VISIBLE | WS_CHILD | WINDOW_STYLE(BS_AUTOCHECKBOX as u32),
            20,
            63,
            200,
            20,
            Some(hwnd),
            Some(HMENU(1007isize as *mut _)),
            None,
            None,
        )?;

        // Set checkbox state based on whether custom assets folder is set
        let checkbox_custom_assets = GetDlgItem(Some(hwnd), 1007)?;
        let use_custom_assets = current_settings.custom_assets_folder.is_some();
        let check_state = if use_custom_assets {
            Some(WPARAM(1))
        } else {
            Some(WPARAM(0))
        };
        SendMessageW(checkbox_custom_assets, BM_SETCHECK, check_state, Some(LPARAM(0)));

        // Create text input for custom assets folder (below label and checkbox)
        let assets_edit_hwnd = CreateWindowExW(
            WS_EX_CLIENTEDGE,
            windows::core::w!("EDIT"),
            windows::core::w!(""),
            WS_VISIBLE | WS_CHILD | WS_BORDER | WINDOW_STYLE(ES_AUTOHSCROLL as u32),
            20,
            90,
            270,
            25,
            Some(hwnd),
            Some(HMENU(1005isize as *mut _)),
            None,
            None,
        )?;

        // Set the current custom assets folder value if set
        if let Some(ref folder_path) = current_settings.custom_assets_folder {
            let folder_text: Vec<u16> = folder_path
                .encode_utf16()
                .chain(Some(0))
                .collect();
            let _ = SetWindowTextW(assets_edit_hwnd, windows::core::PCWSTR(folder_text.as_ptr()));
        }

        // Enable/disable text input based on checkbox
        enable_window(assets_edit_hwnd, use_custom_assets);

        // Create Browse button for custom assets folder (same row as text input)
        let browse_button_hwnd = CreateWindowExW(
            WINDOW_EX_STYLE::default(),
            windows::core::w!("BUTTON"),
            windows::core::w!("Browse..."),
            WS_VISIBLE | WS_CHILD | WINDOW_STYLE(BS_PUSHBUTTON as u32),
            295,
            90,
            75,
            25,
            Some(hwnd),
            Some(HMENU(1006isize as *mut _)),
            None,
            None,
        )?;

        // Enable/disable browse button based on checkbox
        enable_window(browse_button_hwnd, use_custom_assets);

        // Create label for polling interval
        CreateWindowExW(
            WINDOW_EX_STYLE::default(),
            windows::core::w!("STATIC"),
            windows::core::w!("Log polling interval (minutes):"),
            WS_VISIBLE | WS_CHILD,
            20,
            130,
            250,
            20,
            Some(hwnd),
            None,
            None,
            None,
        )?;

        // Create text input for polling interval
        let edit_hwnd = CreateWindowExW(
            WS_EX_CLIENTEDGE,
            windows::core::w!("EDIT"),
            windows::core::w!(""),
            WS_VISIBLE | WS_CHILD | WS_BORDER | WINDOW_STYLE(ES_NUMBER as u32),
            20,
            155,
            100,
            25,
            Some(hwnd),
            Some(HMENU(1003isize as *mut _)),
            None,
            None,
        )?;

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
            195,
            350,
            25,
            Some(hwnd),
            Some(HMENU(1004isize as *mut _)),
            None,
            None,
        )?;

        // Set checkbox state for run at startup
        let checkbox_startup = GetDlgItem(Some(hwnd), 1004)?;
        let check_state = if current_settings.run_at_startup {
            Some(WPARAM(1))
        } else {
            Some(WPARAM(0))
        };
        SendMessageW(checkbox_startup, BM_SETCHECK, check_state, Some(LPARAM(0)));

        // Create OK button
        CreateWindowExW(
            WINDOW_EX_STYLE::default(),
            windows::core::w!("BUTTON"),
            windows::core::w!("OK"),
            WS_VISIBLE | WS_CHILD | WINDOW_STYLE(BS_DEFPUSHBUTTON as u32),
            150,
            240,
            80,
            30,
            Some(hwnd),
            Some(HMENU(2001isize as *mut _)),
            None,
            None,
        )?;

        let _ = ShowWindow(hwnd, SW_SHOW);
        let _ = UpdateWindow(hwnd);

        // Message loop
        let mut msg = MSG::default();
        let mut settings_changed = false;

        while GetMessageW(&mut msg, None, 0, 0).as_bool() {
            let _ = TranslateMessage(&msg);
            DispatchMessageW(&msg);

            // Check if window was closed
            if !IsWindow(Some(hwnd)).as_bool() {
                break;
            }
        }

        // Get final settings state
        if let Some(state) = unsafe { (*std::ptr::addr_of!(SETTINGS_STATE)).as_ref() } {
            let state_guard = state.lock().unwrap();
            settings_changed = state_guard.show_percentage != current_settings.show_percentage
                || state_guard.polling_interval_minutes != current_settings.polling_interval_minutes
                || state_guard.run_at_startup != current_settings.run_at_startup
                || state_guard.custom_assets_folder != current_settings.custom_assets_folder;

            // Save settings if changed
            if settings_changed {
                let mut new_settings = state_guard.settings.clone();
                new_settings.show_percentage = state_guard.show_percentage;
                new_settings.polling_interval_minutes = state_guard.polling_interval_minutes;
                new_settings.run_at_startup = state_guard.run_at_startup;
                new_settings.custom_assets_folder = state_guard.custom_assets_folder.clone();

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
        WM_CTLCOLORSTATIC => {
            // Make static text controls have the same background as the window
            let hdc = HDC(wparam.0 as *mut _);
            SetBkMode(hdc, TRANSPARENT);
            // Return the grey dialog background brush
            LRESULT(GetSysColorBrush(COLOR_3DFACE).0 as isize)
        }
        WM_COMMAND => {
            let control_id = (wparam.0 & 0xFFFF) as i32;
            let notification = ((wparam.0 >> 16) & 0xFFFF) as u32;

            match control_id {
                1001 => {
                    // First checkbox clicked (show percentage icons)
                    if notification == BN_CLICKED {
                        if let Ok(checkbox) = GetDlgItem(Some(hwnd), 1001) {
                            let check_state = SendMessageW(checkbox, BM_GETCHECK, Some(WPARAM(0)), Some(LPARAM(0)));

                            if let Some(state) = unsafe { (*std::ptr::addr_of!(SETTINGS_STATE)).as_ref() } {
                                let mut state_guard = state.lock().unwrap();
                                state_guard.show_percentage = check_state.0 != 0;
                            }
                        }
                    }
                }
                1004 => {
                    // Autostart checkbox clicked
                    if notification == BN_CLICKED {
                        if let Ok(checkbox) = GetDlgItem(Some(hwnd), 1004) {
                            let check_state = SendMessageW(checkbox, BM_GETCHECK, Some(WPARAM(0)), Some(LPARAM(0)));

                            if let Some(state) = unsafe { (*std::ptr::addr_of!(SETTINGS_STATE)).as_ref() } {
                                let mut state_guard = state.lock().unwrap();
                                state_guard.run_at_startup = check_state.0 != 0;
                            }
                        }
                    }
                }
                1007 => {
                    // Custom assets checkbox clicked
                    if notification == BN_CLICKED {
                        if let Ok(checkbox) = GetDlgItem(Some(hwnd), 1007) {
                            let check_state = SendMessageW(checkbox, BM_GETCHECK, Some(WPARAM(0)), Some(LPARAM(0)));
                            let is_enabled = check_state.0 != 0;

                            // Enable/disable text field and browse button
                            if let (Ok(assets_edit), Ok(browse_button)) = (GetDlgItem(Some(hwnd), 1005), GetDlgItem(Some(hwnd), 1006)) {
                                enable_window(assets_edit, is_enabled);
                                enable_window(browse_button, is_enabled);
                            }

                            if let Some(state) = unsafe { (*std::ptr::addr_of!(SETTINGS_STATE)).as_ref() } {
                                let mut state_guard = state.lock().unwrap();
                                state_guard.use_custom_assets = is_enabled;
                            }
                        }
                    }
                }
                1006 => {
                    // Browse button clicked - open folder picker
                    if notification == BN_CLICKED {
                        // Check if the checkbox is enabled first
                        if let Ok(checkbox) = GetDlgItem(Some(hwnd), 1007) {
                            let check_state = SendMessageW(checkbox, BM_GETCHECK, Some(WPARAM(0)), Some(LPARAM(0)));

                            // Only proceed if checkbox is checked
                            if check_state.0 != 0 {
                                if let Ok(folder) = pick_folder(hwnd) {
                                    if let Ok(assets_edit) = GetDlgItem(Some(hwnd), 1005) {
                                        let folder_text: Vec<u16> = folder
                                            .encode_utf16()
                                            .chain(Some(0))
                                            .collect();
                                        let _ = SetWindowTextW(assets_edit, windows::core::PCWSTR(folder_text.as_ptr()));
                                    }
                                }
                            }
                        }
                    }
                }
                2001 => {
                    // OK button clicked - read all text input values

                    // Read polling interval
                    if let Ok(edit_hwnd) = GetDlgItem(Some(hwnd), 1003) {
                        let text_len = GetWindowTextLengthW(edit_hwnd);
                        if text_len > 0 {
                            let mut buffer: Vec<u16> = vec![0; (text_len + 1) as usize];
                            GetWindowTextW(edit_hwnd, &mut buffer);

                            let text = String::from_utf16_lossy(&buffer);
                            let text = text.trim_end_matches('\0');

                            if let Ok(interval) = text.parse::<u64>() {
                                let interval = interval.max(1);

                                if let Some(state) = unsafe { (*std::ptr::addr_of!(SETTINGS_STATE)).as_ref() } {
                                    let mut state_guard = state.lock().unwrap();
                                    state_guard.polling_interval_minutes = interval;
                                }
                            }
                        }
                    }

                    // Read custom assets folder only if checkbox is enabled
                    let use_custom_assets = if let Ok(checkbox_custom_assets) = GetDlgItem(Some(hwnd), 1007) {
                        let check_state = SendMessageW(checkbox_custom_assets, BM_GETCHECK, Some(WPARAM(0)), Some(LPARAM(0)));
                        check_state.0 != 0
                    } else {
                        false
                    };

                    let folder_path = if use_custom_assets {
                        if let Ok(assets_edit) = GetDlgItem(Some(hwnd), 1005) {
                            let text_len = GetWindowTextLengthW(assets_edit);
                            if text_len > 0 {
                                let mut buffer: Vec<u16> = vec![0; (text_len + 1) as usize];
                                GetWindowTextW(assets_edit, &mut buffer);

                                let text = String::from_utf16_lossy(&buffer);
                                let text = text.trim_end_matches('\0').trim();

                                if text.is_empty() {
                                    None
                                } else {
                                    Some(text.to_string())
                                }
                            } else {
                                None
                            }
                        } else {
                            None
                        }
                    } else {
                        None
                    };

                    if let Some(state) = unsafe { (*std::ptr::addr_of!(SETTINGS_STATE)).as_ref() } {
                        let mut state_guard = state.lock().unwrap();
                        state_guard.custom_assets_folder = folder_path;
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

/// Helper function to enable or disable a window control
unsafe fn enable_window(hwnd: HWND, enable: bool) {
    let current_style = GetWindowLongW(hwnd, GWL_STYLE);
    let new_style = if enable {
        current_style & !WS_DISABLED.0 as i32
    } else {
        current_style | WS_DISABLED.0 as i32
    };
    SetWindowLongW(hwnd, GWL_STYLE, new_style);

    // For edit controls, also set/remove ES_READONLY to change visual appearance
    let mut class_name_buf: [u16; 256] = [0; 256];
    let len = GetClassNameW(hwnd, &mut class_name_buf);
    if len > 0 {
        let class_name = String::from_utf16_lossy(&class_name_buf[0..len as usize]);
        if class_name.eq_ignore_ascii_case("Edit") {
            // EM_SETREADONLY message (0x00CF)
            SendMessageW(hwnd, 0x00CF, Some(WPARAM(if enable { 0 } else { 1 })), None);
        }
    }

    // Force the control to redraw with the new state
    let _ = InvalidateRect(Some(hwnd), None, true);
    let _ = UpdateWindow(hwnd);
}

/// Helper function to show a folder picker dialog
unsafe fn pick_folder(parent: HWND) -> Result<String> {
    // Initialize COM
    let _ = CoInitializeEx(None, COINIT_APARTMENTTHREADED);

    let title: Vec<u16> = "Select Custom Assets Folder"
        .encode_utf16()
        .chain(Some(0))
        .collect();

    let bi = BROWSEINFOW {
        hwndOwner: parent,
        pidlRoot: std::ptr::null_mut(),
        pszDisplayName: windows::core::PWSTR(std::ptr::null_mut()),
        lpszTitle: windows::core::PCWSTR(title.as_ptr()),
        ulFlags: BIF_RETURNONLYFSDIRS | BIF_NEWDIALOGSTYLE,
        lpfn: None,
        lParam: LPARAM(0),
        iImage: 0,
    };

    let pidl = SHBrowseForFolderW(&bi as *const _);

    if pidl.is_null() {
        CoUninitialize();
        return Err(anyhow::anyhow!("User cancelled folder selection"));
    }

    let mut path: [u16; 260] = [0; 260];
    let result = SHGetPathFromIDListW(pidl, &mut path);

    // Free the PIDL
    windows::Win32::System::Com::CoTaskMemFree(Some(pidl as *const _));
    CoUninitialize();

    if result.as_bool() {
        let path_str = String::from_utf16_lossy(&path);
        let path_str = path_str.trim_end_matches('\0');
        Ok(path_str.to_string())
    } else {
        Err(anyhow::anyhow!("Failed to get path from folder selection"))
    }
}

