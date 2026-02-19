use anyhow::Result;
use windows::Win32::Foundation::{HWND, LPARAM};
use windows::Win32::UI::Shell::{SHBrowseForFolderW, SHGetPathFromIDListW, BROWSEINFOW, BIF_RETURNONLYFSDIRS, BIF_NEWDIALOGSTYLE};
use windows::Win32::System::Com::{CoInitializeEx, CoUninitialize, COINIT_APARTMENTTHREADED};
use crate::util::{parse_hex_color, to_wide};
use super::ffi_types::*;

pub unsafe fn pick_color(parent: HWND, current_hex: &str) -> Option<String> {
    let (r, g, b) = parse_hex_color(current_hex).unwrap_or((255, 255, 255));
    let current_ref = ((b as u32) << 16) | ((g as u32) << 8) | (r as u32);
    let mut custom_colors: [u32; 16] = [0xFFFFFF; 16];

    let mut cc = CHOOSECOLORW {
        lStructSize: std::mem::size_of::<CHOOSECOLORW>() as u32,
        hwndOwner: parent,
        hInstance: HWND(std::ptr::null_mut()),
        rgbResult: current_ref,
        lpCustColors: custom_colors.as_mut_ptr(),
        Flags: CC_RGBINIT | CC_FULLOPEN,
        lCustData: 0, lpfnHook: None, lpTemplateName: std::ptr::null(),
    };

    if ChooseColorW(&mut cc) == 0 { return None; }
    let cr = cc.rgbResult;
    Some(format!("{:02X}{:02X}{:02X}", cr & 0xFF, (cr >> 8) & 0xFF, (cr >> 16) & 0xFF))
}

pub unsafe fn pick_font(parent: HWND, lf_data: &crate::model::LogFontData) -> Option<FontResult> {
    let mut logfont = logfont_from_data(lf_data);
    let point_size = super::helpers::compute_point_size(parent, logfont.lfHeight);

    let mut cf = CHOOSEFONTW {
        lStructSize: std::mem::size_of::<CHOOSEFONTW>() as u32,
        hwndOwner: parent, hDC: 0, lpLogFont: &mut logfont,
        iPointSize: (point_size * 10) as i32,
        Flags: CF_SCREENFONTS | CF_INITTOLOGFONTSTRUCT | CF_FORCEFONTEXIST,
        rgbColors: 0, lCustData: 0, lpfnHook: None, lpTemplateName: std::ptr::null(),
        hInstance: HWND(std::ptr::null_mut()), lpszStyle: std::ptr::null(),
        nFontType: 0, _MISSING_ALIGNMENT: 0, nSizeMin: 8, nSizeMax: 72,
    };

    if ChooseFontW(&mut cf) == 0 { return None; }
    let end = logfont.lfFaceName.iter().position(|&c| c == 0).unwrap_or(32);
    let face = String::from_utf16_lossy(&logfont.lfFaceName[..end]);
    Some(FontResult { face_name: face, point_size: (cf.iPointSize / 10) as u32, logfont })
}

pub struct FontResult {
    pub face_name: String,
    pub point_size: u32,
    pub logfont: LOGFONTW,
}

impl FontResult {
    pub fn to_logfont_data(&self) -> crate::model::LogFontData {
        let lf = &self.logfont;
        crate::model::LogFontData {
            lf_height: lf.lfHeight, lf_width: lf.lfWidth,
            lf_escapement: lf.lfEscapement, lf_orientation: lf.lfOrientation,
            lf_weight: lf.lfWeight, lf_italic: lf.lfItalic,
            lf_underline: lf.lfUnderline, lf_strike_out: lf.lfStrikeOut,
            lf_char_set: lf.lfCharSet, lf_out_precision: lf.lfOutPrecision,
            lf_clip_precision: lf.lfClipPrecision, lf_quality: lf.lfQuality,
            lf_pitch_and_family: lf.lfPitchAndFamily,
            lf_face_name: self.face_name.clone(),
        }
    }
}

fn logfont_from_data(d: &crate::model::LogFontData) -> LOGFONTW {
    let mut lf = LOGFONTW {
        lfHeight: d.lf_height, lfWidth: d.lf_width,
        lfEscapement: d.lf_escapement, lfOrientation: d.lf_orientation,
        lfWeight: d.lf_weight, lfItalic: d.lf_italic,
        lfUnderline: d.lf_underline, lfStrikeOut: d.lf_strike_out,
        lfCharSet: d.lf_char_set, lfOutPrecision: d.lf_out_precision,
        lfClipPrecision: d.lf_clip_precision, lfQuality: d.lf_quality,
        lfPitchAndFamily: d.lf_pitch_and_family, lfFaceName: [0; 32],
    };
    let wide = to_wide(&d.lf_face_name);
    let len = wide.len().min(32);
    lf.lfFaceName[..len].copy_from_slice(&wide[..len]);
    lf
}

pub unsafe fn pick_folder(parent: HWND) -> Result<String> {
    let _ = CoInitializeEx(None, COINIT_APARTMENTTHREADED);
    let title = to_wide("Select Custom Assets Folder");
    let bi = BROWSEINFOW {
        hwndOwner: parent, pidlRoot: std::ptr::null_mut(),
        pszDisplayName: windows::core::PWSTR(std::ptr::null_mut()),
        lpszTitle: windows::core::PCWSTR(title.as_ptr()),
        ulFlags: BIF_RETURNONLYFSDIRS | BIF_NEWDIALOGSTYLE,
        lpfn: None, lParam: LPARAM(0), iImage: 0,
    };
    let pidl = SHBrowseForFolderW(&bi as *const _);
    if pidl.is_null() { CoUninitialize(); anyhow::bail!("User cancelled"); }
    let mut path: [u16; 260] = [0; 260];
    let ok = SHGetPathFromIDListW(pidl, &mut path);
    windows::Win32::System::Com::CoTaskMemFree(Some(pidl as *const _));
    CoUninitialize();
    if ok.as_bool() {
        Ok(String::from_utf16_lossy(&path).trim_end_matches('\0').to_string())
    } else {
        anyhow::bail!("Failed to get path")
    }
}


