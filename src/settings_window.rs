use anyhow::Result;
use crate::settings::Settings;
use crate::utils::parse_hex_color;
use windows::Win32::{
    Foundation::{HWND, LPARAM, LRESULT, WPARAM},
    Graphics::Gdi::*,
    UI::WindowsAndMessaging::*,
    UI::Controls::{TCITEMW, TCM_INSERTITEMW, TCIF_TEXT, TCS_TABS, NMHDR, TCM_GETCURSEL},
    UI::Shell::{SHBrowseForFolderW, SHGetPathFromIDListW, BROWSEINFOW, BIF_RETURNONLYFSDIRS, BIF_NEWDIALOGSTYLE},
    System::Com::{CoInitializeEx, CoUninitialize, COINIT_APARTMENTTHREADED},
};
use std::sync::{Arc, Mutex};

// Color dialog constants
const CC_RGBINIT: u32 = 0x00000001;
const CC_FULLOPEN: u32 = 0x00000002;

// Font dialog constants
const CF_SCREENFONTS: u32 = 0x00000001;
const CF_INITTOLOGFONTSTRUCT: u32 = 0x00000040;
const CF_FORCEFONTEXIST: u32 = 0x00010000;

// LOGFONT structure for font selection
#[repr(C)]
#[derive(Clone)]
#[allow(non_snake_case)]
struct LOGFONTW {
    lfHeight: i32,
    lfWidth: i32,
    lfEscapement: i32,
    lfOrientation: i32,
    lfWeight: i32,
    lfItalic: u8,
    lfUnderline: u8,
    lfStrikeOut: u8,
    lfCharSet: u8,
    lfOutPrecision: u8,
    lfClipPrecision: u8,
    lfQuality: u8,
    lfPitchAndFamily: u8,
    lfFaceName: [u16; 32],
}

// CHOOSECOLOR structure for color picker
#[repr(C)]
#[allow(non_snake_case)]
struct CHOOSECOLORW {
    lStructSize: u32,
    hwndOwner: HWND,
    hInstance: HWND,
    rgbResult: u32,
    lpCustColors: *mut u32,
    Flags: u32,
    lCustData: isize,
    lpfnHook: Option<unsafe extern "system" fn(HWND, u32, WPARAM, LPARAM) -> usize>,
    lpTemplateName: *const u16,
}

// CHOOSEFONT structure for font picker
#[repr(C)]
#[allow(non_snake_case)]
struct CHOOSEFONTW {
    lStructSize: u32,
    hwndOwner: HWND,
    hDC: isize,
    lpLogFont: *mut LOGFONTW,
    iPointSize: i32,
    Flags: u32,
    rgbColors: u32,
    lCustData: isize,
    lpfnHook: Option<unsafe extern "system" fn(HWND, u32, WPARAM, LPARAM) -> usize>,
    lpTemplateName: *const u16,
    hInstance: HWND,
    lpszStyle: *const u16,
    nFontType: u16,
    _MISSING_ALIGNMENT: u16,
    nSizeMin: i32,
    nSizeMax: i32,
}

extern "system" {
    fn ChooseColorW(lpcc: *mut CHOOSECOLORW) -> i32;
    fn ChooseFontW(lpcf: *mut CHOOSEFONTW) -> i32;
}

// Combo box constants
const CBS_DROPDOWNLIST: u32 = 0x0003;
const CB_ADDSTRING: u32 = 0x0143;
const CB_SETCURSEL: u32 = 0x014E;
const CB_GETCURSEL: u32 = 0x0147;

// Notification codes
const CBN_SELCHANGE: u32 = 1;
const EN_CHANGE: u32 = 0x0300;

// Tab control notification code
// TCN_FIRST is -550, TCN_SELCHANGE is TCN_FIRST - 1 = -551
const TCN_SELCHANGE: i32 = -551;

static mut SETTINGS_STATE: Option<Arc<Mutex<SettingsWindowState>>> = None;

struct SettingsWindowState {
    show_percentage: bool,
    percentage_text_size: u32,
    percentage_text_color: String,
    percentage_text_font: String,
    percentage_text_align: String,
    percentage_text_x: i32,
    percentage_text_y: i32,
    show_percent_symbol: bool,
    polling_interval_minutes: u64,
    run_at_startup: bool,
    use_custom_assets: bool,
    custom_assets_folder: Option<String>,
    icon_theme: String, // "dark", "light", or "system"
    show_device_type_overlay: bool,
    settings: Settings,
}

pub struct SettingsWindow;

impl SettingsWindow {
    pub unsafe fn show(current_settings: Settings) -> Result<bool> {
        // Initialize global state
        SETTINGS_STATE = Some(Arc::new(Mutex::new(SettingsWindowState {
            show_percentage: current_settings.show_percentage,
            percentage_text_size: current_settings.percentage_text_size,
            percentage_text_color: current_settings.percentage_text_color.clone(),
            percentage_text_font: current_settings.percentage_text_font.clone(),
            percentage_text_align: format!("{:?}", current_settings.percentage_text_align).to_lowercase(),
            percentage_text_x: current_settings.percentage_text_x,
            percentage_text_y: current_settings.percentage_text_y,
            show_percent_symbol: current_settings.show_percent_symbol,
            polling_interval_minutes: current_settings.polling_interval_minutes,
            run_at_startup: current_settings.run_at_startup,
            use_custom_assets: current_settings.custom_assets_folder.is_some(),
            custom_assets_folder: current_settings.custom_assets_folder.clone(),
            icon_theme: format!("{:?}", current_settings.icon_theme).to_lowercase(),
            show_device_type_overlay: current_settings.show_device_type_overlay,
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

        // Window dimensions - increased for tabs
        let window_width = 450;
        let window_height = 400;

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

        // Create Tab Control
        let tab_control = CreateWindowExW(
            WINDOW_EX_STYLE::default(),
            windows::core::w!("SysTabControl32"),
            windows::core::w!(""),
            WS_VISIBLE | WS_CHILD | WINDOW_STYLE(TCS_TABS as u32),
            10,
            10,
            window_width - 30,
            window_height - 90,
            Some(hwnd),
            Some(HMENU(3000isize as *mut _)),
            None,
            None,
        )?;

        // Add tabs
        let mut tie = TCITEMW {
            mask: TCIF_TEXT,
            dwState: Default::default(),
            dwStateMask: Default::default(),
            pszText: windows::core::PWSTR(windows::core::w!("General").as_ptr() as *mut u16),
            cchTextMax: 0,
            iImage: 0,
            lParam: LPARAM(0),
        };
        SendMessageW(tab_control, TCM_INSERTITEMW, Some(WPARAM(0)), Some(LPARAM(&tie as *const _ as isize)));

        tie.pszText = windows::core::PWSTR(windows::core::w!("Percentage Text").as_ptr() as *mut u16);
        SendMessageW(tab_control, TCM_INSERTITEMW, Some(WPARAM(1)), Some(LPARAM(&tie as *const _ as isize)));

        // === GENERAL TAB CONTROLS (initially visible) ===
        // Tab content area starts at Y=45 (below tab headers)

        // Show device type overlay checkbox — top of General tab (ID 1014)
        CreateWindowExW(
            WINDOW_EX_STYLE::default(),
            windows::core::w!("BUTTON"),
            windows::core::w!("Show device type overlay"),
            WS_VISIBLE | WS_CHILD | WINDOW_STYLE(BS_AUTOCHECKBOX as u32),
            30,
            55,
            350,
            20,
            Some(hwnd),
            Some(HMENU(1014isize as *mut _)),
            None,
            None,
        )?;

        let checkbox_device_overlay = GetDlgItem(Some(hwnd), 1014)?;
        let check_state = if current_settings.show_device_type_overlay {
            Some(WPARAM(1))
        } else {
            Some(WPARAM(0))
        };
        SendMessageW(checkbox_device_overlay, BM_SETCHECK, check_state, Some(LPARAM(0)));

        // === Theme mode section ===
        // Label for theme
        CreateWindowExW(
            WINDOW_EX_STYLE::default(),
            windows::core::w!("STATIC"),
            windows::core::w!("Icon theme:"),
            WS_VISIBLE | WS_CHILD,
            30,
            85,
            200,
            20,
            Some(hwnd),
            Some(HMENU(1011isize as *mut _)),
            None,
            None,
        )?;

        // Dark theme radio button (ID 1012) - first in group
        CreateWindowExW(
            WINDOW_EX_STYLE::default(),
            windows::core::w!("BUTTON"),
            windows::core::w!("Dark"),
            WS_VISIBLE | WS_CHILD | WINDOW_STYLE(BS_AUTORADIOBUTTON as u32 | WS_GROUP.0),
            30,
            107,
            70,
            20,
            Some(hwnd),
            Some(HMENU(1012isize as *mut _)),
            None,
            None,
        )?;

        // Light theme radio button (ID 1013)
        CreateWindowExW(
            WINDOW_EX_STYLE::default(),
            windows::core::w!("BUTTON"),
            windows::core::w!("Light"),
            WS_VISIBLE | WS_CHILD | WINDOW_STYLE(BS_AUTORADIOBUTTON as u32),
            110,
            107,
            70,
            20,
            Some(hwnd),
            Some(HMENU(1013isize as *mut _)),
            None,
            None,
        )?;

        // System theme radio button (ID 1015) - follows OS dark/light setting
        CreateWindowExW(
            WINDOW_EX_STYLE::default(),
            windows::core::w!("BUTTON"),
            windows::core::w!("System"),
            WS_VISIBLE | WS_CHILD | WINDOW_STYLE(BS_AUTORADIOBUTTON as u32),
            190,
            107,
            80,
            20,
            Some(hwnd),
            Some(HMENU(1015isize as *mut _)),
            None,
            None,
        )?;

        // Set radio button state based on current theme
        let current_theme = format!("{:?}", current_settings.icon_theme).to_lowercase();
        if let (Ok(radio_dark), Ok(radio_light), Ok(radio_system)) = (
            GetDlgItem(Some(hwnd), 1012),
            GetDlgItem(Some(hwnd), 1013),
            GetDlgItem(Some(hwnd), 1015),
        ) {
            SendMessageW(radio_dark,   BM_SETCHECK, Some(WPARAM(if current_theme == "dark"   { 1 } else { 0 })), Some(LPARAM(0)));
            SendMessageW(radio_light,  BM_SETCHECK, Some(WPARAM(if current_theme == "light"  { 1 } else { 0 })), Some(LPARAM(0)));
            SendMessageW(radio_system, BM_SETCHECK, Some(WPARAM(if current_theme == "system" { 1 } else { 0 })), Some(LPARAM(0)));
        }

        // Custom assets section
        // Create checkbox for custom assets folder
        CreateWindowExW(
            WINDOW_EX_STYLE::default(),
            windows::core::w!("BUTTON"),
            windows::core::w!("Use custom assets"),
            WS_VISIBLE | WS_CHILD | WINDOW_STYLE(BS_AUTOCHECKBOX as u32),
            30,
            142,
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

        // Create text input for custom assets folder
        let assets_edit_hwnd = CreateWindowExW(
            WS_EX_CLIENTEDGE,
            windows::core::w!("EDIT"),
            windows::core::w!(""),
            WS_VISIBLE | WS_CHILD | WS_BORDER | WINDOW_STYLE(ES_AUTOHSCROLL as u32),
            30,
            166,
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
            305,
            166,
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
            30,
            206,
            250,
            20,
            Some(hwnd),
            Some(HMENU(1010isize as *mut _)),
            None,
            None,
        )?;

        // Create text input for polling interval
        let edit_hwnd = CreateWindowExW(
            WS_EX_CLIENTEDGE,
            windows::core::w!("EDIT"),
            windows::core::w!(""),
            WS_VISIBLE | WS_CHILD | WS_BORDER | WINDOW_STYLE(ES_NUMBER as u32),
            30,
            228,
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
            30,
            268,
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


        // === PERCENTAGE TEXT TAB CONTROLS (initially hidden) ===

        // Show percentage checkbox
        CreateWindowExW(
            WINDOW_EX_STYLE::default(),
            windows::core::w!("BUTTON"),
            windows::core::w!("Show percentage"),
            WS_CHILD | WINDOW_STYLE(BS_AUTOCHECKBOX as u32), // Hidden initially
            30,
            55,
            200,
            20,
            Some(hwnd),
            Some(HMENU(2001isize as *mut _)),
            None,
            None,
        )?;

        let checkbox_percentage = GetDlgItem(Some(hwnd), 2001)?;
        let check_state = if current_settings.show_percentage {
            Some(WPARAM(1))
        } else {
            Some(WPARAM(0))
        };
        SendMessageW(checkbox_percentage, BM_SETCHECK, check_state, Some(LPARAM(0)));

        // Text color label (moved up to Y=90 where text size was)
        CreateWindowExW(
            WINDOW_EX_STYLE::default(),
            windows::core::w!("STATIC"),
            windows::core::w!("Text color:"),
            WS_CHILD,
            30,
            90,
            80,
            20,
            Some(hwnd),
            Some(HMENU(2011isize as *mut _)),
            None,
            None,
        )?;

        // Text color picker button
        let _color_button = CreateWindowExW(
            WINDOW_EX_STYLE::default(),
            windows::core::w!("BUTTON"),
            windows::core::w!("Pick Color"),
            WS_CHILD | WINDOW_STYLE(BS_PUSHBUTTON as u32),
            115,
            88,
            105,
            22,
            Some(hwnd),
            Some(HMENU(2003isize as *mut _)),
            None,
            None,
        )?;

        // Font button (right side) - opens Windows font dialog
        // Calculate point size from LOGFONT height
        let hdc = GetDC(Some(hwnd));
        let dpi_y = unsafe { GetDeviceCaps(Some(hdc), LOGPIXELSY) };
        let _ = ReleaseDC(Some(hwnd), hdc);
        let point_size = if current_settings.logfont_data.lf_height < 0 {
            ((-current_settings.logfont_data.lf_height * 72) / dpi_y) as u32
        } else {
            20 // Default
        };

        let font_button_text = format!("Font: {}, {}pt",
                                       current_settings.logfont_data.lf_face_name,
                                       point_size);
        let font_button_text_wide: Vec<u16> = font_button_text.encode_utf16().chain(Some(0)).collect();

        CreateWindowExW(
            WINDOW_EX_STYLE::default(),
            windows::core::w!("BUTTON"),
            windows::core::PCWSTR(font_button_text_wide.as_ptr()),
            WS_CHILD | WINDOW_STYLE(BS_PUSHBUTTON as u32),
            230,
            88,
            195,
            22,
            Some(hwnd),
            Some(HMENU(2008isize as *mut _)),
            None,
            None,
        )?;


        // Alignment label (moved up to Y=125)
        CreateWindowExW(
            WINDOW_EX_STYLE::default(),
            windows::core::w!("STATIC"),
            windows::core::w!("Alignment:"),
            WS_CHILD,
            30,
            125,
            100,
            20,
            Some(hwnd),
            Some(HMENU(2012isize as *mut _)),
            None,
            None,
        )?;

        // Alignment combo box (moved up to Y=123)
        let alignment_combo = CreateWindowExW(
            WINDOW_EX_STYLE::default(),
            windows::core::w!("COMBOBOX"),
            windows::core::w!(""),
            WS_CHILD | WINDOW_STYLE(CBS_DROPDOWNLIST as u32 | WS_VSCROLL.0),
            140,
            123,
            100,
            100,
            Some(hwnd),
            Some(HMENU(2004isize as *mut _)),
            None,
            None,
        )?;

        SendMessageW(alignment_combo, CB_ADDSTRING, None,
            Some(LPARAM(windows::core::w!("Left").as_ptr() as isize)));
        SendMessageW(alignment_combo, CB_ADDSTRING, None,
            Some(LPARAM(windows::core::w!("Center").as_ptr() as isize)));
        SendMessageW(alignment_combo, CB_ADDSTRING, None,
            Some(LPARAM(windows::core::w!("Right").as_ptr() as isize)));

        let align_sel = match format!("{:?}", current_settings.percentage_text_align).to_lowercase().as_str() {
            "left" => 0,
            "right" => 2,
            _ => 1, // center
        };
        SendMessageW(alignment_combo, CB_SETCURSEL, Some(WPARAM(align_sel)), None);

        // Position X label (moved up to Y=160)
        CreateWindowExW(
            WINDOW_EX_STYLE::default(),
            windows::core::w!("STATIC"),
            windows::core::w!("Position X (0=auto):"),
            WS_CHILD,
            30,
            160,
            130,
            20,
            Some(hwnd),
            Some(HMENU(2013isize as *mut _)),
            None,
            None,
        )?;

        // Position X input (moved up to Y=158)
        let pos_x_edit = CreateWindowExW(
            WS_EX_CLIENTEDGE,
            windows::core::w!("EDIT"),
            windows::core::w!(""),
            WS_CHILD | WS_BORDER | WINDOW_STYLE(ES_NUMBER as u32),
            165,
            158,
            60,
            22,
            Some(hwnd),
            Some(HMENU(2005isize as *mut _)),
            None,
            None,
        )?;

        let x_text: Vec<u16> = format!("{}", current_settings.percentage_text_x)
            .encode_utf16()
            .chain(Some(0))
            .collect();
        let _ = SetWindowTextW(pos_x_edit, windows::core::PCWSTR(x_text.as_ptr()));

        // Position Y label (moved up to Y=195)
        CreateWindowExW(
            WINDOW_EX_STYLE::default(),
            windows::core::w!("STATIC"),
            windows::core::w!("Position Y:"),
            WS_CHILD,
            30,
            195,
            100,
            20,
            Some(hwnd),
            Some(HMENU(2014isize as *mut _)),
            None,
            None,
        )?;

        // Position Y input (moved up to Y=193)
        let pos_y_edit = CreateWindowExW(
            WS_EX_CLIENTEDGE,
            windows::core::w!("EDIT"),
            windows::core::w!(""),
            WS_CHILD | WS_BORDER | WINDOW_STYLE(ES_NUMBER as u32),
            140,
            193,
            60,
            22,
            Some(hwnd),
            Some(HMENU(2006isize as *mut _)),
            None,
            None,
        )?;

        let y_text: Vec<u16> = format!("{}", current_settings.percentage_text_y)
            .encode_utf16()
            .chain(Some(0))
            .collect();
        let _ = SetWindowTextW(pos_y_edit, windows::core::PCWSTR(y_text.as_ptr()));

        // Show '%' symbol checkbox (moved up to Y=228)
        CreateWindowExW(
            WINDOW_EX_STYLE::default(),
            windows::core::w!("BUTTON"),
            windows::core::w!("Show '%' symbol"),
            WS_CHILD | WINDOW_STYLE(BS_AUTOCHECKBOX as u32),
            30,
            228,
            200,
            20,
            Some(hwnd),
            Some(HMENU(2007isize as *mut _)),
            None,
            None,
        )?;

        let checkbox_percent_symbol = GetDlgItem(Some(hwnd), 2007)?;
        let check_state = if current_settings.show_percent_symbol {
            Some(WPARAM(1))
        } else {
            Some(WPARAM(0))
        };
        SendMessageW(checkbox_percent_symbol, BM_SETCHECK, check_state, Some(LPARAM(0)));

        // Create OK button (below tab control)
        CreateWindowExW(
            WINDOW_EX_STYLE::default(),
            windows::core::w!("BUTTON"),
            windows::core::w!("OK"),
            WS_VISIBLE | WS_CHILD | WINDOW_STYLE(BS_DEFPUSHBUTTON as u32),
            180,
            320,
            80,
            30,
            Some(hwnd),
            Some(HMENU(9001isize as *mut _)),
            None,
            None,
        )?;

        // Initially hide all Percentage Text tab controls (Tab 1)
        // Only General tab (Tab 0) should be visible at startup
        show_hide_control(hwnd, 2001, false); // Show percentage checkbox
        show_hide_control(hwnd, 2003, false); // Color picker button
        show_hide_control(hwnd, 2004, false); // Alignment combo
        show_hide_control(hwnd, 2005, false); // Position X edit
        show_hide_control(hwnd, 2006, false); // Position Y edit
        show_hide_control(hwnd, 2007, false); // Show '%' symbol checkbox
        show_hide_control(hwnd, 2008, false); // Choose Font button
        show_hide_control(hwnd, 2011, false); // Text color label
        show_hide_control(hwnd, 2012, false); // Alignment label
        show_hide_control(hwnd, 2013, false); // Position X label
        show_hide_control(hwnd, 2014, false); // Position Y label

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
                || state_guard.percentage_text_size != current_settings.percentage_text_size
                || state_guard.percentage_text_color != current_settings.percentage_text_color
                || state_guard.percentage_text_font != current_settings.percentage_text_font
                || state_guard.percentage_text_align != format!("{:?}", current_settings.percentage_text_align).to_lowercase()
                || state_guard.percentage_text_x != current_settings.percentage_text_x
                || state_guard.percentage_text_y != current_settings.percentage_text_y
                || state_guard.show_percent_symbol != current_settings.show_percent_symbol
                || state_guard.polling_interval_minutes != current_settings.polling_interval_minutes
                || state_guard.run_at_startup != current_settings.run_at_startup
                || state_guard.custom_assets_folder != current_settings.custom_assets_folder
                || state_guard.icon_theme != format!("{:?}", current_settings.icon_theme).to_lowercase()
                || state_guard.show_device_type_overlay != current_settings.show_device_type_overlay;

            // Save settings if changed
            if settings_changed {
                let mut new_settings = state_guard.settings.clone();
                new_settings.show_percentage = state_guard.show_percentage;
                new_settings.percentage_text_size = state_guard.percentage_text_size;
                new_settings.percentage_text_color = state_guard.percentage_text_color.clone();
                new_settings.percentage_text_font = state_guard.percentage_text_font.clone();
                // Parse alignment from string back to enum
                new_settings.percentage_text_align = match state_guard.percentage_text_align.as_str() {
                    "left" => crate::settings::TextAlignment::Left,
                    "right" => crate::settings::TextAlignment::Right,
                    _ => crate::settings::TextAlignment::Center,
                };
                new_settings.percentage_text_x = state_guard.percentage_text_x;
                new_settings.percentage_text_y = state_guard.percentage_text_y;
                new_settings.show_percent_symbol = state_guard.show_percent_symbol;
                new_settings.polling_interval_minutes = state_guard.polling_interval_minutes;
                new_settings.run_at_startup = state_guard.run_at_startup;
                new_settings.custom_assets_folder = state_guard.custom_assets_folder.clone();
                new_settings.icon_theme = match state_guard.icon_theme.as_str() {
                    "light" => crate::settings::IconTheme::Light,
                    "system" => crate::settings::IconTheme::System,
                    _ => crate::settings::IconTheme::Dark,
                };
                new_settings.show_device_type_overlay = state_guard.show_device_type_overlay;

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
                2001 => {
                    // Show percentage checkbox clicked (in Percentage Text tab)
                    if notification == BN_CLICKED {
                        if let Ok(checkbox) = GetDlgItem(Some(hwnd), 2001) {
                            let check_state = SendMessageW(checkbox, BM_GETCHECK, Some(WPARAM(0)), Some(LPARAM(0)));

                            if let Some(state) = unsafe { (*std::ptr::addr_of!(SETTINGS_STATE)).as_ref() } {
                                let mut state_guard = state.lock().unwrap();
                                state_guard.show_percentage = check_state.0 != 0;
                            }
                        }
                    }
                }
                2003 => {
                    // Color picker button clicked
                    if notification == BN_CLICKED {
                        // Get current color from state
                        let current_color = if let Some(state) = unsafe { (*std::ptr::addr_of!(SETTINGS_STATE)).as_ref() } {
                            let state_guard = state.lock().unwrap();
                            state_guard.percentage_text_color.clone()
                        } else {
                            "FFFFFF".to_string()
                        };

                        // Parse current color to RGB
                        let (r, g, b) = parse_hex_color(&current_color).unwrap_or((255, 255, 255));
                        let current_colorref = ((b as u32) << 16) | ((g as u32) << 8) | (r as u32);

                        // Create custom colors array (required by ChooseColor)
                        let mut custom_colors: [u32; 16] = [0xFFFFFF; 16];

                        // Initialize CHOOSECOLORW structure
                        let mut cc = CHOOSECOLORW {
                            lStructSize: std::mem::size_of::<CHOOSECOLORW>() as u32,
                            hwndOwner: hwnd,
                            hInstance: HWND(std::ptr::null_mut()),
                            rgbResult: current_colorref,
                            lpCustColors: custom_colors.as_mut_ptr(),
                            Flags: CC_RGBINIT | CC_FULLOPEN,
                            lCustData: 0,
                            lpfnHook: None,
                            lpTemplateName: std::ptr::null(),
                        };

                        // Show color picker dialog
                        if unsafe { ChooseColorW(&mut cc) } != 0 {
                            // User selected a color
                            let colorref = cc.rgbResult;
                            let r = (colorref & 0xFF) as u8;
                            let g = ((colorref >> 8) & 0xFF) as u8;
                            let b = ((colorref >> 16) & 0xFF) as u8;

                            // Convert to hex string
                            let hex_color = format!("{:02X}{:02X}{:02X}", r, g, b);

                            // Update state
                            if let Some(state) = unsafe { (*std::ptr::addr_of!(SETTINGS_STATE)).as_ref() } {
                                let mut state_guard = state.lock().unwrap();
                                state_guard.percentage_text_color = hex_color.clone();
                            }

                            // Update button text to show selected color
                            let button_text: Vec<u16> = format!("Color: #{}", hex_color)
                                .encode_utf16()
                                .chain(Some(0))
                                .collect();
                            if let Ok(button) = GetDlgItem(Some(hwnd), 2003) {
                                let _ = SetWindowTextW(button, windows::core::PCWSTR(button_text.as_ptr()));
                            }
                        }
                    }
                }
                2004 => {
                    // Alignment combo box selection changed
                    if notification == CBN_SELCHANGE {
                        if let Ok(combo) = GetDlgItem(Some(hwnd), 2004) {
                            let sel = SendMessageW(combo, CB_GETCURSEL, None, None).0;
                            let align = match sel {
                                0 => "left",
                                2 => "right",
                                _ => "center",
                            };

                            if let Some(state) = unsafe { (*std::ptr::addr_of!(SETTINGS_STATE)).as_ref() } {
                                let mut state_guard = state.lock().unwrap();
                                state_guard.percentage_text_align = align.to_string();
                            }
                        }
                    }
                }
                2005 => {
                    // Position X input changed
                    if notification == EN_CHANGE {
                        if let Ok(edit) = GetDlgItem(Some(hwnd), 2005) {
                            let text_len = GetWindowTextLengthW(edit);
                            if text_len > 0 {
                                let mut buffer: Vec<u16> = vec![0; (text_len + 1) as usize];
                                GetWindowTextW(edit, &mut buffer);
                                let text = String::from_utf16_lossy(&buffer);
                                let text = text.trim_end_matches('\0');

                                if let Ok(x) = text.parse::<i32>() {
                                    if let Some(state) = unsafe { (*std::ptr::addr_of!(SETTINGS_STATE)).as_ref() } {
                                        let mut state_guard = state.lock().unwrap();
                                        state_guard.percentage_text_x = x;
                                    }
                                }
                            }
                        }
                    }
                }
                2006 => {
                    // Position Y input changed
                    if notification == EN_CHANGE {
                        if let Ok(edit) = GetDlgItem(Some(hwnd), 2006) {
                            let text_len = GetWindowTextLengthW(edit);
                            if text_len > 0 {
                                let mut buffer: Vec<u16> = vec![0; (text_len + 1) as usize];
                                GetWindowTextW(edit, &mut buffer);
                                let text = String::from_utf16_lossy(&buffer);
                                let text = text.trim_end_matches('\0');

                                if let Ok(y) = text.parse::<i32>() {
                                    if let Some(state) = unsafe { (*std::ptr::addr_of!(SETTINGS_STATE)).as_ref() } {
                                        let mut state_guard = state.lock().unwrap();
                                        state_guard.percentage_text_y = y;
                                    }
                                }
                            }
                        }
                    }
                }
                2008 => {
                    // Choose Font button clicked
                    if notification == BN_CLICKED {
                        // Get current LOGFONT data from settings
                        let logfont_data = if let Some(state) = unsafe { (*std::ptr::addr_of!(SETTINGS_STATE)).as_ref() } {
                            let state_guard = state.lock().unwrap();
                            state_guard.settings.logfont_data.clone()
                        } else {
                            crate::settings::LogFontData::default()
                        };

                        eprintln!("DEBUG: Opening font dialog with stored LOGFONT:");
                        eprintln!("  - Face: '{}', Height: {}, Weight: {}, Italic: {}",
                                 logfont_data.lf_face_name, logfont_data.lf_height,
                                 logfont_data.lf_weight, logfont_data.lf_italic);

                        // Create LOGFONT from stored data
                        let mut logfont = LOGFONTW {
                            lfHeight: logfont_data.lf_height,
                            lfWidth: logfont_data.lf_width,
                            lfEscapement: logfont_data.lf_escapement,
                            lfOrientation: logfont_data.lf_orientation,
                            lfWeight: logfont_data.lf_weight,
                            lfItalic: logfont_data.lf_italic,
                            lfUnderline: logfont_data.lf_underline,
                            lfStrikeOut: logfont_data.lf_strike_out,
                            lfCharSet: logfont_data.lf_char_set,
                            lfOutPrecision: logfont_data.lf_out_precision,
                            lfClipPrecision: logfont_data.lf_clip_precision,
                            lfQuality: logfont_data.lf_quality,
                            lfPitchAndFamily: logfont_data.lf_pitch_and_family,
                            lfFaceName: [0; 32],
                        };

                        // Copy font name
                        let font_name_wide: Vec<u16> = logfont_data.lf_face_name.encode_utf16().chain(Some(0)).collect();
                        let copy_len = font_name_wide.len().min(32);
                        logfont.lfFaceName[..copy_len].copy_from_slice(&font_name_wide[..copy_len]);
                        for i in copy_len..32 {
                            logfont.lfFaceName[i] = 0;
                        }

                        // Calculate point size from lfHeight
                        let hdc = GetDC(Some(hwnd));
                        let dpi_y = unsafe { GetDeviceCaps(Some(hdc), LOGPIXELSY) };
                        let _ = ReleaseDC(Some(hwnd), hdc);
                        let point_size = if logfont.lfHeight < 0 {
                            ((-logfont.lfHeight * 72) / dpi_y) as u32
                        } else { 20 };

                        // Initialize CHOOSEFONT
                        let mut choosefont = CHOOSEFONTW {
                            lStructSize: std::mem::size_of::<CHOOSEFONTW>() as u32,
                            hwndOwner: hwnd,
                            hDC: 0,
                            lpLogFont: &mut logfont,
                            iPointSize: (point_size * 10) as i32,
                            Flags: CF_SCREENFONTS | CF_INITTOLOGFONTSTRUCT | CF_FORCEFONTEXIST,
                            rgbColors: 0,
                            lCustData: 0,
                            lpfnHook: None,
                            lpTemplateName: std::ptr::null(),
                            hInstance: HWND(std::ptr::null_mut()),
                            lpszStyle: std::ptr::null(),
                            nFontType: 0,
                            _MISSING_ALIGNMENT: 0,
                            nSizeMin: 8,
                            nSizeMax: 72,
                        };

                        // Show dialog
                        if unsafe { ChooseFontW(&mut choosefont) } != 0 {
                            let face_name_end = logfont.lfFaceName.iter().position(|&c| c == 0).unwrap_or(32);
                            let font_name = String::from_utf16_lossy(&logfont.lfFaceName[..face_name_end]);
                            let font_size = (choosefont.iPointSize / 10) as u32;

                            eprintln!("DEBUG: Font selected - saving complete LOGFONT:");
                            eprintln!("  - Face: '{}', Height: {}, Weight: {}, Italic: {}",
                                     font_name, logfont.lfHeight, logfont.lfWeight, logfont.lfItalic);

                            // Save complete LOGFONT
                            let new_logfont = crate::settings::LogFontData {
                                lf_height: logfont.lfHeight,
                                lf_width: logfont.lfWidth,
                                lf_escapement: logfont.lfEscapement,
                                lf_orientation: logfont.lfOrientation,
                                lf_weight: logfont.lfWeight,
                                lf_italic: logfont.lfItalic,
                                lf_underline: logfont.lfUnderline,
                                lf_strike_out: logfont.lfStrikeOut,
                                lf_char_set: logfont.lfCharSet,
                                lf_out_precision: logfont.lfOutPrecision,
                                lf_clip_precision: logfont.lfClipPrecision,
                                lf_quality: logfont.lfQuality,
                                lf_pitch_and_family: logfont.lfPitchAndFamily,
                                lf_face_name: font_name.to_string(),
                            };

                            if let Some(state) = unsafe { (*std::ptr::addr_of!(SETTINGS_STATE)).as_ref() } {
                                let mut state_guard = state.lock().unwrap();
                                state_guard.settings.logfont_data = new_logfont;
                                state_guard.percentage_text_font = font_name.to_string();
                                state_guard.percentage_text_size = font_size;
                            }

                            // Update font button text
                            if let Ok(button) = GetDlgItem(Some(hwnd), 2008) {
                                let button_text: Vec<u16> = format!("Font: {}, {}pt", font_name, font_size)
                                    .encode_utf16().chain(Some(0)).collect();
                                let _ = SetWindowTextW(button, windows::core::PCWSTR(button_text.as_ptr()));
                            }
                        }
                    }
                }
                2007 => {
                    // Show '%' symbol checkbox clicked
                    if notification == BN_CLICKED {
                        if let Ok(checkbox) = GetDlgItem(Some(hwnd), 2007) {
                            let check_state = SendMessageW(checkbox, BM_GETCHECK, Some(WPARAM(0)), Some(LPARAM(0)));

                            if let Some(state) = unsafe { (*std::ptr::addr_of!(SETTINGS_STATE)).as_ref() } {
                                let mut state_guard = state.lock().unwrap();
                                state_guard.show_percent_symbol = check_state.0 != 0;
                            }
                        }
                    }
                }
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
                1014 => {
                    // Show device type overlay checkbox clicked
                    if notification == BN_CLICKED {
                        if let Ok(checkbox) = GetDlgItem(Some(hwnd), 1014) {
                            let check_state = SendMessageW(checkbox, BM_GETCHECK, Some(WPARAM(0)), Some(LPARAM(0)));

                            if let Some(state) = unsafe { (*std::ptr::addr_of!(SETTINGS_STATE)).as_ref() } {
                                let mut state_guard = state.lock().unwrap();
                                state_guard.show_device_type_overlay = check_state.0 != 0;
                            }
                        }
                    }
                }
                1012 => {
                    // Dark theme radio button clicked
                    if notification == BN_CLICKED {
                        if let Some(state) = unsafe { (*std::ptr::addr_of!(SETTINGS_STATE)).as_ref() } {
                            let mut state_guard = state.lock().unwrap();
                            state_guard.icon_theme = "dark".to_string();
                        }
                    }
                }
                1013 => {
                    // Light theme radio button clicked
                    if notification == BN_CLICKED {
                        if let Some(state) = unsafe { (*std::ptr::addr_of!(SETTINGS_STATE)).as_ref() } {
                            let mut state_guard = state.lock().unwrap();
                            state_guard.icon_theme = "light".to_string();
                        }
                    }
                }
                1015 => {
                    // System theme radio button clicked
                    if notification == BN_CLICKED {
                        if let Some(state) = unsafe { (*std::ptr::addr_of!(SETTINGS_STATE)).as_ref() } {
                            let mut state_guard = state.lock().unwrap();
                            state_guard.icon_theme = "system".to_string();
                        }
                    }
                }
                1007 => {
                    // Custom assets checkbox clicked
                    if notification == BN_CLICKED {                        if let Ok(checkbox) = GetDlgItem(Some(hwnd), 1007) {
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
                9001 => {
                    // OK button clicked - read all text input values
                    eprintln!("DEBUG: OK button (9001) clicked!");

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
        WM_NOTIFY => {
            // Handle tab control notifications
            eprintln!("DEBUG: WM_NOTIFY received");
            let nmhdr = *(lparam.0 as *const NMHDR);
            eprintln!("DEBUG: nmhdr.idFrom = {}, nmhdr.code = {} (as i32: {})", nmhdr.idFrom, nmhdr.code, nmhdr.code as i32);

            // Check if it's from the tab control and if the selection changed
            if nmhdr.idFrom == 3000 && (nmhdr.code as i32) == TCN_SELCHANGE {
                eprintln!("DEBUG: Tab control selection changed!");
                // Get the currently selected tab
                if let Ok(tab_control) = GetDlgItem(Some(hwnd), 3000) {
                    let current_tab = SendMessageW(tab_control, TCM_GETCURSEL, None, None).0;
                    eprintln!("DEBUG: Current tab index = {}", current_tab);

                    match current_tab {
                        0 => {
                            // General tab selected - show General controls, hide Percentage Text controls
                            // General tab controls
                            show_hide_control(hwnd, 1007, true);  // Use custom assets checkbox
                            show_hide_control(hwnd, 1005, true);  // Custom assets path edit
                            show_hide_control(hwnd, 1006, true);  // Browse button
                            show_hide_control(hwnd, 1011, true);  // Icon theme label
                            show_hide_control(hwnd, 1012, true);  // Dark radio button
                            show_hide_control(hwnd, 1013, true);  // Light radio button
                            show_hide_control(hwnd, 1015, true);  // System radio button
                            show_hide_control(hwnd, 1010, true);  // Polling interval label
                            show_hide_control(hwnd, 1003, true);  // Polling interval edit
                            show_hide_control(hwnd, 1004, true);  // Run at startup checkbox
                            show_hide_control(hwnd, 1014, true);  // Show device type overlay checkbox

                            // Percentage Text tab controls (hide)
                            show_hide_control(hwnd, 2001, false); // Show percentage checkbox
                            show_hide_control(hwnd, 2003, false); // Color picker button
                            show_hide_control(hwnd, 2004, false); // Alignment combo
                            show_hide_control(hwnd, 2005, false); // Position X edit
                            show_hide_control(hwnd, 2006, false); // Position Y edit
                            show_hide_control(hwnd, 2007, false); // Show '%' symbol checkbox
                            show_hide_control(hwnd, 2008, false); // Choose Font button
                            show_hide_control(hwnd, 2011, false); // Text color label
                            show_hide_control(hwnd, 2012, false); // Alignment label
                            show_hide_control(hwnd, 2013, false); // Position X label
                            show_hide_control(hwnd, 2014, false); // Position Y label
                        }
                        1 => {
                            // Percentage Text tab selected - hide General controls, show Percentage Text controls
                            // General tab controls (hide)
                            show_hide_control(hwnd, 1007, false); // Use custom assets checkbox
                            show_hide_control(hwnd, 1005, false); // Custom assets path edit
                            show_hide_control(hwnd, 1006, false); // Browse button
                            show_hide_control(hwnd, 1011, false); // Icon theme label
                            show_hide_control(hwnd, 1012, false); // Dark radio button
                            show_hide_control(hwnd, 1013, false); // Light radio button
                            show_hide_control(hwnd, 1015, false); // System radio button
                            show_hide_control(hwnd, 1010, false); // Polling interval label
                            show_hide_control(hwnd, 1003, false); // Polling interval edit
                            show_hide_control(hwnd, 1004, false); // Run at startup checkbox
                            show_hide_control(hwnd, 1014, false); // Show device type overlay checkbox

                            // Percentage Text tab controls (show)
                            show_hide_control(hwnd, 2001, true);  // Show percentage checkbox
                            show_hide_control(hwnd, 2003, true);  // Color picker button
                            show_hide_control(hwnd, 2004, true);  // Alignment combo
                            show_hide_control(hwnd, 2005, true);  // Position X edit
                            show_hide_control(hwnd, 2006, true);  // Position Y edit
                            show_hide_control(hwnd, 2007, true);  // Show '%' symbol checkbox
                            show_hide_control(hwnd, 2008, true);  // Choose Font button
                            show_hide_control(hwnd, 2011, true);  // Text color label
                            show_hide_control(hwnd, 2012, true);  // Alignment label
                            show_hide_control(hwnd, 2013, true);  // Position X label
                            show_hide_control(hwnd, 2014, true);  // Position Y label
                        }
                        _ => {}
                    }
                }
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

/// Helper function to show or hide a window control
unsafe fn show_hide_control(parent: HWND, control_id: i32, show: bool) {
    eprintln!("DEBUG: show_hide_control called - control_id: {}, show: {}", control_id, show);
    match GetDlgItem(Some(parent), control_id) {
        Ok(control) => {
            eprintln!("DEBUG: Found control {}, calling ShowWindow with {}", control_id, if show { "SW_SHOW" } else { "SW_HIDE" });
            let result = ShowWindow(control, if show { SW_SHOW } else { SW_HIDE });
            eprintln!("DEBUG: ShowWindow returned: {:?}", result);
        }
        Err(e) => {
            eprintln!("ERROR: Could not find control {}: {:?}", control_id, e);
        }
    }
}

