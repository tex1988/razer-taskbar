use windows::Win32::Foundation::{HWND, LPARAM, WPARAM};

// ── Color dialog ───────────────────────────────────────────────

pub const CC_RGBINIT: u32 = 0x00000001;
pub const CC_FULLOPEN: u32 = 0x00000002;

#[repr(C)]
#[allow(non_snake_case)]
pub struct CHOOSECOLORW {
    pub lStructSize: u32,
    pub hwndOwner: HWND,
    pub hInstance: HWND,
    pub rgbResult: u32,
    pub lpCustColors: *mut u32,
    pub Flags: u32,
    pub lCustData: isize,
    pub lpfnHook: Option<unsafe extern "system" fn(HWND, u32, WPARAM, LPARAM) -> usize>,
    pub lpTemplateName: *const u16,
}

// ── Font dialog ────────────────────────────────────────────────

pub const CF_SCREENFONTS: u32 = 0x00000001;
pub const CF_INITTOLOGFONTSTRUCT: u32 = 0x00000040;
pub const CF_FORCEFONTEXIST: u32 = 0x00010000;

#[repr(C)]
#[derive(Clone)]
#[allow(non_snake_case)]
pub struct LOGFONTW {
    pub lfHeight: i32,
    pub lfWidth: i32,
    pub lfEscapement: i32,
    pub lfOrientation: i32,
    pub lfWeight: i32,
    pub lfItalic: u8,
    pub lfUnderline: u8,
    pub lfStrikeOut: u8,
    pub lfCharSet: u8,
    pub lfOutPrecision: u8,
    pub lfClipPrecision: u8,
    pub lfQuality: u8,
    pub lfPitchAndFamily: u8,
    pub lfFaceName: [u16; 32],
}

#[repr(C)]
#[allow(non_snake_case)]
pub struct CHOOSEFONTW {
    pub lStructSize: u32,
    pub hwndOwner: HWND,
    pub hDC: isize,
    pub lpLogFont: *mut LOGFONTW,
    pub iPointSize: i32,
    pub Flags: u32,
    pub rgbColors: u32,
    pub lCustData: isize,
    pub lpfnHook: Option<unsafe extern "system" fn(HWND, u32, WPARAM, LPARAM) -> usize>,
    pub lpTemplateName: *const u16,
    pub hInstance: HWND,
    pub lpszStyle: *const u16,
    pub nFontType: u16,
    pub _MISSING_ALIGNMENT: u16,
    pub nSizeMin: i32,
    pub nSizeMax: i32,
}

extern "system" {
    pub fn ChooseColorW(lpcc: *mut CHOOSECOLORW) -> i32;
    pub fn ChooseFontW(lpcf: *mut CHOOSEFONTW) -> i32;
}

// ── Combo / notification constants ─────────────────────────────

// Constants NOT in the windows crate
pub const TCN_SELCHANGE: i32 = -551;




