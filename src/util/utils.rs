/// Utility functions shared across modules

/// Parse a hex color string (RRGGBB or #RRGGBB) into RGB components
pub fn parse_hex_color(hex: &str) -> Option<(u8, u8, u8)> {
    let hex = hex.trim_start_matches('#');
    if hex.len() != 6 {
        return None;
    }

    let r = u8::from_str_radix(&hex[0..2], 16).ok()?;
    let g = u8::from_str_radix(&hex[2..4], 16).ok()?;
    let b = u8::from_str_radix(&hex[4..6], 16).ok()?;

    Some((r, g, b))
}

/// Convert a &str to a null-terminated wide string (Vec<u16>) for Win32 APIs
pub fn to_wide(s: &str) -> Vec<u16> {
    s.encode_utf16().chain(Some(0)).collect()
}

#[cfg(test)]
mod tests {
    use super::*;

    // ── parse_hex_color ────────────────────────────────────────

    #[test]
    fn parse_hex_color_returns_rgb_for_bare_hex() {
        assert_eq!(parse_hex_color("FF0000"), Some((255, 0, 0)));
        assert_eq!(parse_hex_color("00FF00"), Some((0, 255, 0)));
        assert_eq!(parse_hex_color("0000FF"), Some((0, 0, 255)));
        assert_eq!(parse_hex_color("FFFFFF"), Some((255, 255, 255)));
        assert_eq!(parse_hex_color("000000"), Some((0, 0, 0)));
    }

    #[test]
    fn parse_hex_color_strips_leading_hash() {
        assert_eq!(parse_hex_color("#FF0000"), Some((255, 0, 0)));
        assert_eq!(parse_hex_color("#1A2B3C"), Some((0x1A, 0x2B, 0x3C)));
    }

    #[test]
    fn parse_hex_color_lowercase_digits() {
        assert_eq!(parse_hex_color("ff8800"), Some((255, 136, 0)));
    }

    #[test]
    fn parse_hex_color_returns_none_for_wrong_length() {
        assert_eq!(parse_hex_color(""), None);
        assert_eq!(parse_hex_color("FFF"), None);       // too short
        assert_eq!(parse_hex_color("FFFFFFF"), None);   // too long (7 after strip)
    }

    #[test]
    fn parse_hex_color_returns_none_for_invalid_chars() {
        assert_eq!(parse_hex_color("GGGGGG"), None);
        assert_eq!(parse_hex_color("XYZ123"), None);
        assert_eq!(parse_hex_color("12 456"), None);
    }

    // ── to_wide ───────────────────────────────────────────────

    #[test]
    fn to_wide_encodes_ascii_with_null_terminator() {
        let wide = to_wide("hi");
        assert_eq!(wide, vec![b'h' as u16, b'i' as u16, 0]);
    }

    #[test]
    fn to_wide_empty_string_yields_only_null() {
        assert_eq!(to_wide(""), vec![0]);
    }

    #[test]
    fn to_wide_last_element_is_always_null() {
        let wide = to_wide("test");
        assert_eq!(*wide.last().unwrap(), 0);
    }
}

