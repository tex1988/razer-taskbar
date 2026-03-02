use anyhow::Result;
use windows::Win32::Foundation::{HWND, LPARAM, LRESULT, RECT, WPARAM};
use windows::Win32::Graphics::Gdi::{
    DrawTextW, GetDC, ReleaseDC, SelectObject,
    DT_CALCRECT, DT_WORDBREAK, DT_LEFT,
    COLOR_3DFACE, GetSysColorBrush, SetBkMode, TRANSPARENT,
    UpdateWindow as GdiUpdateWindow,
    HDC, HBRUSH,
};
use windows::Win32::UI::WindowsAndMessaging::*;
use crate::model::DeviceConfig;
use crate::util::to_wide;
use super::helpers::set_checkbox;
use super::state;

// Raw FFI for SetScrollInfo
extern "system" {
    fn SetScrollInfo(hwnd: *mut core::ffi::c_void, nbar: i32, lpsi: *const SCROLLINFO, redraw: i32) -> i32;
}
unsafe fn set_scroll_info(hwnd: HWND, nbar: SCROLLBAR_CONSTANTS, si: &SCROLLINFO, redraw: bool) {
    SetScrollInfo(hwnd.0 as _, nbar.0, si as *const _, if redraw { 1 } else { 0 });
}

// Scroll codes
const SC_LINEUP: i32 = 0;
const SC_LINEDOWN: i32 = 1;
const SC_PAGEUP: i32 = 2;
const SC_PAGEDOWN: i32 = 3;
const SC_THUMBTRACK: i32 = 5;
const SCROLL_STEP: i32 = 20;

// ── Control IDs ───────────────────────────────────────────────
const DESC_LABEL_ID: i32 = 3100;
const SCROLL_PANEL_ID: i32 = 3102;
const HINT_LABEL_ID: i32 = 3199;

const DEVICE_CTRL_BASE: i32 = 3110;
const MAX_DEVICES: usize = 20;
const CTRLS_PER_DEVICE: i32 = 3;

// ── Layout ────────────────────────────────────────────────────
const PANEL_X: i32 = 20;
const PANEL_Y: i32 = 76;
const PANEL_W: i32 = 395;
const PANEL_H: i32 = 201;

// Column layout (positions inside the panel)
const COL1_X: i32 = 8;          // checkbox column left (small left margin)
const COL1_W: i32 = CB_SIZE;    // checkbox column width
const COL_GAP: i32 = 8;         // gap between checkbox and name columns
const COL2_X: i32 = COL1_X + COL1_W + COL_GAP; // name column left
const COL2_W: i32 = PANEL_W - COL2_X - 20;      // name column width

// Row metrics
const ROW_PAD: i32 = 6;
const MIN_ROW_H: i32 = 24;
const CB_SIZE: i32 = 16;

fn cb_id(i: usize) -> i32 { DEVICE_CTRL_BASE + (i as i32) * CTRLS_PER_DEVICE }
fn label_id(i: usize) -> i32 { DEVICE_CTRL_BASE + (i as i32) * CTRLS_PER_DEVICE + 1 }
fn del_id(i: usize) -> i32 { DEVICE_CTRL_BASE + (i as i32) * CTRLS_PER_DEVICE + 2 }

pub fn devices_tab_ids() -> Vec<i32> {
    vec![DESC_LABEL_ID, SCROLL_PANEL_ID, HINT_LABEL_ID]
}

pub fn delete_button_index(id: i32) -> Option<usize> {
    for i in 0..MAX_DEVICES {
        if id == del_id(i) { return Some(i); }
    }
    None
}

// ── Text measurement ──────────────────────────────────────────

unsafe fn measure_text_height(hwnd: HWND, text: &str, width: i32) -> i32 {
    let hdc = GetDC(Some(hwnd));
    let hfont = SendMessageW(hwnd, WM_GETFONT, None, None);
    let old = if hfont.0 != 0 {
        Some(SelectObject(hdc, windows::Win32::Graphics::Gdi::HGDIOBJ(hfont.0 as _)))
    } else { None };
    let mut wide = to_wide(text);
    let mut rc = RECT { left: 0, top: 0, right: width, bottom: 0 };
    DrawTextW(hdc, &mut wide, &mut rc, DT_CALCRECT | DT_WORDBREAK | DT_LEFT);
    if let Some(prev) = old { SelectObject(hdc, prev); }
    ReleaseDC(Some(hwnd), hdc);
    rc.bottom.max(16)
}

fn device_label(cfg: &DeviceConfig) -> String {
    if cfg.connected {
        format!("{} ({})", cfg.name, cfg.id)
    } else {
        format!("{} ({}) - disconnected", cfg.name, cfg.id)
    }
}

unsafe fn compute_layout(panel: HWND, configs: &[DeviceConfig], count: usize) -> (Vec<(i32, i32)>, i32) {
    let mut rows = Vec::with_capacity(count);
    let mut y = 4i32;
    for cfg in configs.iter().take(count) {
        let text = device_label(cfg);
        let text_h = measure_text_height(panel, &text, COL2_W);
        let row_h = (text_h + ROW_PAD).max(MIN_ROW_H);
        rows.push((y, row_h));
        y += row_h;
    }
    (rows, y + 4)
}

// ── Scroll panel ──────────────────────────────────────────────

unsafe fn ensure_scroll_class() {
    let class = windows::core::w!("RazerDevScrollPanel");
    let wc = WNDCLASSW {
        lpfnWndProc: Some(scroll_panel_proc),
        lpszClassName: class,
        hCursor: LoadCursorW(None, IDC_ARROW).unwrap_or_default(),
        hbrBackground: HBRUSH(((COLOR_3DFACE.0 + 1) as isize) as *mut _),
        ..Default::default()
    };
    let _ = RegisterClassW(&wc);
}

unsafe extern "system" fn scroll_panel_proc(
    hwnd: HWND, msg: u32, wparam: WPARAM, lparam: LPARAM,
) -> LRESULT {
    match msg {
        WM_VSCROLL => { do_vscroll(hwnd, wparam); LRESULT(0) }
        WM_MOUSEWHEEL => { do_wheel(hwnd, wparam); LRESULT(0) }
        WM_COMMAND => {
            if let Ok(parent) = GetParent(hwnd) {
                SendMessageW(parent, WM_COMMAND, Some(wparam), Some(lparam));
            }
            LRESULT(0)
        }
        WM_CTLCOLORSTATIC | WM_CTLCOLORBTN => {
            let hdc = HDC(wparam.0 as *mut _);
            SetBkMode(hdc, TRANSPARENT);
            LRESULT(GetSysColorBrush(COLOR_3DFACE).0 as isize)
        }
        _ => DefWindowProcW(hwnd, msg, wparam, lparam),
    }
}

unsafe fn do_vscroll(hwnd: HWND, wparam: WPARAM) {
    let mut si = SCROLLINFO { cbSize: std::mem::size_of::<SCROLLINFO>() as u32, fMask: SIF_ALL, ..Default::default() };
    let _ = GetScrollInfo(hwnd, SB_VERT, &mut si);
    let old = si.nPos;
    let page = si.nPage as i32;
    let code = (wparam.0 & 0xFFFF) as i32;
    si.nPos = match code {
        SC_LINEUP    => si.nPos - SCROLL_STEP,
        SC_LINEDOWN  => si.nPos + SCROLL_STEP,
        SC_PAGEUP    => si.nPos - page,
        SC_PAGEDOWN  => si.nPos + page,
        SC_THUMBTRACK => si.nTrackPos,
        _ => si.nPos,
    };
    si.fMask = SIF_POS;
    set_scroll_info(hwnd, SB_VERT, &si, true);
    let _ = GetScrollInfo(hwnd, SB_VERT, &mut si);
    let dy = old - si.nPos;
    if dy != 0 {
        let _ = ScrollWindow(hwnd, 0, dy, None, None);
        let _ = GdiUpdateWindow(hwnd);
    }
}

unsafe fn do_wheel(hwnd: HWND, wparam: WPARAM) {
    let delta = (wparam.0 >> 16) as i16;
    let lines = delta as i32 / 120;
    let mut si = SCROLLINFO { cbSize: std::mem::size_of::<SCROLLINFO>() as u32, fMask: SIF_ALL, ..Default::default() };
    let _ = GetScrollInfo(hwnd, SB_VERT, &mut si);
    let old = si.nPos;
    si.nPos -= lines * SCROLL_STEP;
    si.fMask = SIF_POS;
    set_scroll_info(hwnd, SB_VERT, &si, true);
    let _ = GetScrollInfo(hwnd, SB_VERT, &mut si);
    let dy = old - si.nPos;
    if dy != 0 {
        let _ = ScrollWindow(hwnd, 0, dy, None, None);
        let _ = GdiUpdateWindow(hwnd);
    }
}

// ── Public API ────────────────────────────────────────────────

pub unsafe fn create_devices_tab(hwnd: HWND) -> Result<()> {
    // Description label
    CreateWindowExW(
        WINDOW_EX_STYLE::default(), windows::core::w!("STATIC"),
        windows::core::w!("Toggle tray icon visibility for each device:"),
        WS_CHILD, 30, 55, 380, 20,
        Some(hwnd), Some(HMENU(DESC_LABEL_ID as isize as *mut _)), None, None,
    )?;

    // Scrollable panel
    ensure_scroll_class();
    let panel = CreateWindowExW(
        WINDOW_EX_STYLE::default(), windows::core::w!("RazerDevScrollPanel"),
        windows::core::w!(""),
        WS_CHILD | WS_CLIPCHILDREN | WS_BORDER | WS_VSCROLL,
        PANEL_X, PANEL_Y, PANEL_W, PANEL_H,
        Some(hwnd), Some(HMENU(SCROLL_PANEL_ID as isize as *mut _)), None, None,
    )?;

    populate_panel(panel);

    // Hint
    CreateWindowExW(
        WINDOW_EX_STYLE::default(), windows::core::w!("STATIC"),
        windows::core::w!("Drag icons in the system tray to reorder them."),
        WS_CHILD, 30, 285, 380, 18,
        Some(hwnd), Some(HMENU(HINT_LABEL_ID as isize as *mut _)), None, None,
    )?;

    Ok(())
}

unsafe fn populate_panel(panel: HWND) {
    // Sort: connected first
    state::with_state(|s| {
        s.device_configs.sort_by_key(|c| if c.connected { 0 } else { 1 });
    });

    let configs: Vec<DeviceConfig> = state::with_state(|s| s.device_configs.clone()).unwrap_or_default();
    let count = configs.len().min(MAX_DEVICES);

    let (rows, content_h) = compute_layout(panel, &configs, count);

    // Scrollbar — SIF_DISABLENOSCROLL (0x0008) keeps it visible but greyed when not needed
    let si = SCROLLINFO {
        cbSize: std::mem::size_of::<SCROLLINFO>() as u32,
        fMask: SIF_RANGE | SIF_PAGE | SIF_POS | SCROLLINFO_MASK(0x0008),
        nMin: 0, nMax: content_h - 1, nPage: PANEL_H as u32, nPos: 0, nTrackPos: 0,
    };
    set_scroll_info(panel, SB_VERT, &si, true);

    for (i, cfg) in configs.iter().take(count).enumerate() {
        let (y, row_h) = rows[i];
        let label_h = row_h - ROW_PAD;

        let ctrl_y = y + (row_h - CB_SIZE) / 2;
        let ctrl_x = COL1_X;

        if cfg.connected {
            CreateWindowExW(
                WINDOW_EX_STYLE::default(), windows::core::w!("BUTTON"), windows::core::w!(""),
                WS_VISIBLE | WS_CHILD | WINDOW_STYLE(BS_AUTOCHECKBOX as u32),
                ctrl_x, ctrl_y, CB_SIZE, CB_SIZE,
                Some(panel), Some(HMENU(cb_id(i) as isize as *mut _)), None, None,
            ).ok();
            set_checkbox(panel, cb_id(i), cfg.visible);
        } else {
            // × delete — SS_NOTIFY=0x0100, SS_CENTER=0x01, SS_CENTERIMAGE=0x0200
            CreateWindowExW(
                WINDOW_EX_STYLE::default(), windows::core::w!("STATIC"),
                windows::core::w!("×"),
                WS_VISIBLE | WS_CHILD | WINDOW_STYLE(0x0100 | 0x01 | 0x0200),
                ctrl_x, y + 4, CB_SIZE, CB_SIZE,
                Some(panel), Some(HMENU(del_id(i) as isize as *mut _)), None, None,
            ).ok();
        }

        // Label in Col2
        let text = device_label(cfg);
        let wide = to_wide(&text);
        let style = if cfg.connected {
            WS_VISIBLE | WS_CHILD
        } else {
            WS_VISIBLE | WS_CHILD | WS_DISABLED
        };
        CreateWindowExW(
            WINDOW_EX_STYLE::default(), windows::core::w!("STATIC"),
            windows::core::PCWSTR(wide.as_ptr()),
            style,
            COL2_X, y + 2, COL2_W, label_h,
            Some(panel), Some(HMENU(label_id(i) as isize as *mut _)), None, None,
        ).ok();
    }
}

unsafe fn rebuild_panel(hwnd: HWND) {
    if let Ok(old_panel) = GetDlgItem(Some(hwnd), SCROLL_PANEL_ID) {
        let _ = DestroyWindow(old_panel);
    }

    let panel = match CreateWindowExW(
        WINDOW_EX_STYLE::default(), windows::core::w!("RazerDevScrollPanel"),
        windows::core::w!(""),
        WS_CHILD | WS_VISIBLE | WS_CLIPCHILDREN | WS_BORDER | WS_VSCROLL,
        PANEL_X, PANEL_Y, PANEL_W, PANEL_H,
        Some(hwnd), Some(HMENU(SCROLL_PANEL_ID as isize as *mut _)), None, None,
    ) {
        Ok(p) => p,
        Err(_) => return,
    };

    populate_panel(panel);
}

pub unsafe fn handle_delete_device(hwnd: HWND, device_index: usize) {
    let removed = state::with_state(|s| {
        if device_index < s.device_configs.len() && !s.device_configs[device_index].connected {
            s.device_configs.remove(device_index);
            true
        } else {
            false
        }
    }).unwrap_or(false);
    if removed { rebuild_panel(hwnd); }
}

pub unsafe fn read_devices_tab(hwnd: HWND) {
    let panel = match GetDlgItem(Some(hwnd), SCROLL_PANEL_ID) {
        Ok(p) => p,
        Err(_) => return,
    };
    state::with_state(|s| {
        let count = s.device_configs.len().min(MAX_DEVICES);
        for i in 0..count {
            if !s.device_configs[i].connected { continue; }
            if let Ok(cb) = GetDlgItem(Some(panel), cb_id(i)) {
                let checked = SendMessageW(cb, BM_GETCHECK, Some(WPARAM(0)), Some(LPARAM(0))).0 != 0;
                s.device_configs[i].visible = checked;
            }
        }
    });
}

#[allow(dead_code)]
pub fn handle_device_checkbox(_id: i32) {}

#[allow(dead_code)]
pub fn is_devices_tab_control(id: i32) -> bool {
    id >= DEVICE_CTRL_BASE && id < DEVICE_CTRL_BASE + (MAX_DEVICES as i32) * CTRLS_PER_DEVICE
}

