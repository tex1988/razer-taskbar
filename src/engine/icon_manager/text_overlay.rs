use ab_glyph::{FontRef, PxScale};
use image::{Rgba, RgbaImage};
use imageproc::drawing::draw_text_mut;
use crate::util::parse_hex_color;
use crate::model::TextOverlayConfig;

const TARGET_ICON_SIZE: u32 = 32;

pub(crate) fn draw_percentage_overlay(
    img: &mut RgbaImage,
    percentage: u8,
    config: &TextOverlayConfig,
) {
    let font_data = match resolve_font(&config.font_name) {
        Some(d) => d,
        None => return,
    };
    let font = match FontRef::try_from_slice(&font_data) {
        Ok(f) => f,
        Err(_) => return,
    };
    let text = format_percentage_text(percentage, config.show_percent_symbol);
    let scale = PxScale::from(config.text_size as f32);
    let (r, g, b) = parse_hex_color(&config.text_color).unwrap_or((255, 255, 255));
    let text_width = measure_text_width(&font, &text, scale);
    let (x, y) = compute_text_position(&config.text_align, text_width, config.text_x, config.text_y);
    draw_text_with_outline(img, x, y, scale, &font, &text, r, g, b);
}

fn format_percentage_text(pct: u8, show_symbol: bool) -> String {
    if show_symbol { format!("{}%", pct) } else { format!("{}", pct) }
}

fn resolve_font(font_name: &str) -> Option<Vec<u8>> {
    load_font_from_registry(font_name)
        .or_else(|| load_font_from_registry("Arial"))
        .or_else(|| load_font_from_registry("Segoe UI"))
}

fn measure_text_width(font: &FontRef, text: &str, scale: PxScale) -> f32 {
    use ab_glyph::{Font, ScaleFont};
    let sf = font.as_scaled(scale);
    text.chars().map(|c| sf.h_advance(font.glyph_id(c))).sum()
}

fn compute_text_position(
    align: &str, text_w: f32, offset_x: i32, offset_y: i32,
) -> (i32, i32) {
    let base_x = match align {
        "left" => 0,
        "right" => (TARGET_ICON_SIZE as f32 - text_w).max(0.0) as i32,
        _ => ((TARGET_ICON_SIZE as f32 / 2.0) - (text_w / 2.0)).max(0.0) as i32,
    };
    (base_x + offset_x, offset_y)
}

fn draw_text_with_outline(
    img: &mut RgbaImage, x: i32, y: i32,
    scale: PxScale, font: &FontRef, text: &str,
    r: u8, g: u8, b: u8,
) {
    let outline = Rgba([0u8, 0u8, 0u8, 80u8]);
    for (dx, dy) in [(-1, 0), (1, 0), (0, -1), (0, 1)] {
        draw_text_mut(img, outline, x + dx, y + dy, scale, font, text);
    }
    draw_text_mut(img, Rgba([r, g, b, 255u8]), x, y, scale, font, text);
}

// ── Font loading ───────────────────────────────────────────────

fn load_font_from_registry(font_name: &str) -> Option<Vec<u8>> {
    use winreg::RegKey;
    use winreg::enums::*;
    let hklm = RegKey::predef(HKEY_LOCAL_MACHINE);
    let fonts_key = hklm
        .open_subkey(r"SOFTWARE\Microsoft\Windows NT\CurrentVersion\Fonts")
        .ok()?;
    for (name, _) in fonts_key.enum_values().filter_map(|x| x.ok()) {
        if !name.contains(font_name) { continue; }
        let filename: String = match fonts_key.get_value(&name) {
            Ok(v) => v,
            Err(_) => continue,
        };
        let full_path = build_font_path(&filename);
        if let Ok(data) = std::fs::read(&full_path) { return Some(data); }
    }
    None
}

fn build_font_path(filename: &str) -> String {
    if filename.contains(":\\") { filename.to_string() }
    else { format!("C:\\Windows\\Fonts\\{}", filename) }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::model::TextOverlayConfig;

    fn make_config(align: &str, offset_x: i32, offset_y: i32) -> TextOverlayConfig {
        TextOverlayConfig {
            text_size: 16,
            text_color: "FFFFFF".into(),
            font_name: "Arial".into(),
            text_align: align.into(),
            text_x: offset_x,
            text_y: offset_y,
            show_percent_symbol: false,
        }
    }

    // ── format_percentage_text ────────────────────────────────

    #[test]
    fn format_text_without_symbol_returns_number_only() {
        assert_eq!(format_percentage_text(75, false), "75");
        assert_eq!(format_percentage_text(0, false),  "0");
        assert_eq!(format_percentage_text(100, false), "100");
    }

    #[test]
    fn format_text_with_symbol_appends_percent() {
        assert_eq!(format_percentage_text(75, true),  "75%");
        assert_eq!(format_percentage_text(0, true),   "0%");
        assert_eq!(format_percentage_text(100, true), "100%");
    }

    // ── compute_text_position ─────────────────────────────────

    #[test]
    fn compute_position_left_align_starts_at_zero_plus_offset() {
        let (x, y) = compute_text_position("left", 10.0, 2, 5);
        assert_eq!(x, 0 + 2);
        assert_eq!(y, 5);
    }

    #[test]
    fn compute_position_right_align_places_text_at_right_edge() {
        // icon width = 32, text_width = 10 → base_x = 32 - 10 = 22
        let (x, _) = compute_text_position("right", 10.0, 0, 0);
        assert_eq!(x, 22);
    }

    #[test]
    fn compute_position_right_align_applies_offset() {
        let (x, y) = compute_text_position("right", 10.0, 3, 7);
        assert_eq!(x, 22 + 3);
        assert_eq!(y, 7);
    }

    #[test]
    fn compute_position_center_align_centers_text() {
        // icon=32, text_w=8 → base_x = (32/2 - 8/2) = 12
        let (x, _) = compute_text_position("center", 8.0, 0, 0);
        assert_eq!(x, 12);
    }

    #[test]
    fn compute_position_unknown_align_acts_as_center() {
        let (x, _) = compute_text_position("unknown", 8.0, 0, 0);
        // falls through to the `_` branch which is center
        assert_eq!(x, 12);
    }

    #[test]
    fn compute_position_right_clamps_to_zero_when_text_wider_than_icon() {
        // text_w > 32 → (32 - text_w).max(0) = 0
        let (x, _) = compute_text_position("right", 40.0, 0, 0);
        assert_eq!(x, 0);
    }

    // ── build_font_path ───────────────────────────────────────

    #[test]
    fn build_font_path_absolute_path_is_unchanged() {
        let abs = r"C:\Windows\Fonts\Arial.ttf";
        assert_eq!(build_font_path(abs), abs);
    }

    #[test]
    fn build_font_path_relative_name_gets_windows_fonts_prefix() {
        assert_eq!(
            build_font_path("Arial.ttf"),
            r"C:\Windows\Fonts\Arial.ttf"
        );
    }
}

