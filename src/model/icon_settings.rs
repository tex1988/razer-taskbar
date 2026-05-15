/// Grouped icon display settings — passed as a single struct instead of 10+ args.
#[derive(Debug, Clone, PartialEq)]
pub struct IconSettings {
    pub show_percentage: bool,
    pub text_size: u32,
    pub text_color: String,
    pub font_name: String,
    pub text_align: String,
    pub text_x: i32,
    pub text_y: i32,
    pub show_percent_symbol: bool,
    pub show_device_type_overlay: bool,
}

/// Subset of IconSettings used by the text overlay drawing code.
#[derive(Debug, Clone)]
pub struct TextOverlayConfig {
    pub text_size: u32,
    pub text_color: String,
    pub font_name: String,
    pub text_align: String,
    pub text_x: i32,
    pub text_y: i32,
    pub show_percent_symbol: bool,
}

impl From<&IconSettings> for TextOverlayConfig {
    fn from(s: &IconSettings) -> Self {
        Self {
            text_size: s.text_size,
            text_color: s.text_color.clone(),
            font_name: s.font_name.clone(),
            text_align: s.text_align.clone(),
            text_x: s.text_x,
            text_y: s.text_y,
            show_percent_symbol: s.show_percent_symbol,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn sample_icon_settings() -> IconSettings {
        IconSettings {
            show_percentage: true,
            text_size: 24,
            text_color: "FF0000".into(),
            font_name: "Consolas".into(),
            text_align: "left".into(),
            text_x: 3,
            text_y: 7,
            show_percent_symbol: true,
            show_device_type_overlay: false,
        }
    }

    #[test]
    fn from_copies_all_text_overlay_fields() {
        let icon = sample_icon_settings();
        let overlay = TextOverlayConfig::from(&icon);
        assert_eq!(overlay.text_size, 24);
        assert_eq!(overlay.text_color, "FF0000");
        assert_eq!(overlay.font_name, "Consolas");
        assert_eq!(overlay.text_align, "left");
        assert_eq!(overlay.text_x, 3);
        assert_eq!(overlay.text_y, 7);
        assert_eq!(overlay.show_percent_symbol, true);
    }

    #[test]
    fn icon_settings_partial_eq_reflects_field_values() {
        let a = sample_icon_settings();
        let mut b = sample_icon_settings();
        assert_eq!(a, b);
        b.text_size = 99;
        assert_ne!(a, b);
    }
}

