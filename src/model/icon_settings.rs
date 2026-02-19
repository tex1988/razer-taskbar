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

