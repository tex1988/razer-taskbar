// Embedded resources - battery icons
use std::collections::HashMap;
use anyhow::Result;
use image::RgbaImage;
use lazy_static::lazy_static;

lazy_static! {
    static ref ASSET_CACHE: HashMap<&'static str, &'static [u8]> = {
        let mut m = HashMap::new();

        // Standard battery icons
        m.insert("battery0_@2x.png", include_bytes!("assets/battery0_@2x.png") as &[u8]);
        m.insert("battery0_chrg_@2x.png", include_bytes!("assets/battery0_chrg_@2x.png") as &[u8]);
        m.insert("battery25_@2x.png", include_bytes!("assets/battery25_@2x.png") as &[u8]);
        m.insert("battery25_chrg_@2x.png", include_bytes!("assets/battery25_chrg_@2x.png") as &[u8]);
        m.insert("battery50_@2x.png", include_bytes!("assets/battery50_@2x.png") as &[u8]);
        m.insert("battery50_chrg_@2x.png", include_bytes!("assets/battery50_chrg_@2x.png") as &[u8]);
        m.insert("battery75_@2x.png", include_bytes!("assets/battery75_@2x.png") as &[u8]);
        m.insert("battery75_chrg_@2x.png", include_bytes!("assets/battery75_chrg_@2x.png") as &[u8]);
        m.insert("battery100_@2x.png", include_bytes!("assets/battery100_@2x.png") as &[u8]);
        m.insert("battery100_chrg_@2x.png", include_bytes!("assets/battery100_chrg_@2x.png") as &[u8]);
        m.insert("battery_unknown_@2x.png", include_bytes!("assets/battery_unknown_@2x.png") as &[u8]);

        m
    };
}

pub fn load_embedded_image(filename: &str) -> Result<RgbaImage> {
    if let Some(bytes) = ASSET_CACHE.get(filename) {
        let img = image::load_from_memory(bytes)
            .map_err(|e| anyhow::anyhow!("Failed to decode embedded image {}: {}", filename, e))?
            .to_rgba8();
        Ok(img)
    } else {
        Err(anyhow::anyhow!("Asset {} not found in embedded resources", filename))
    }
}

// Generate numeric icon data on the fly
pub fn generate_numeric_icon(percentage: u8, is_charging: bool) -> Result<RgbaImage> {
    // Clamp percentage to 0-100
    let percentage = percentage.min(100);

    // Use a macro to generate the match arms for all percentages
    macro_rules! include_numeric_icon {
        ($p:expr, charging) => {
            include_bytes!(concat!("assets/numeric-icon-chrg/battery", stringify!($p), ".png"))
        };
        ($p:expr, normal) => {
            include_bytes!(concat!("assets/numeric-icon/battery", stringify!($p), ".png"))
        };
    }

    // Match the exact percentage to load the correct embedded icon
    let bytes: &[u8] = match (is_charging, percentage) {
        // Charging icons
        (true, 0) => include_numeric_icon!(000, charging),
        (true, 1) => include_numeric_icon!(001, charging),
        (true, 2) => include_numeric_icon!(002, charging),
        (true, 3) => include_numeric_icon!(003, charging),
        (true, 4) => include_numeric_icon!(004, charging),
        (true, 5) => include_numeric_icon!(005, charging),
        (true, 6) => include_numeric_icon!(006, charging),
        (true, 7) => include_numeric_icon!(007, charging),
        (true, 8) => include_numeric_icon!(008, charging),
        (true, 9) => include_numeric_icon!(009, charging),
        (true, 10) => include_numeric_icon!(010, charging),
        (true, 11) => include_numeric_icon!(011, charging),
        (true, 12) => include_numeric_icon!(012, charging),
        (true, 13) => include_numeric_icon!(013, charging),
        (true, 14) => include_numeric_icon!(014, charging),
        (true, 15) => include_numeric_icon!(015, charging),
        (true, 16) => include_numeric_icon!(016, charging),
        (true, 17) => include_numeric_icon!(017, charging),
        (true, 18) => include_numeric_icon!(018, charging),
        (true, 19) => include_numeric_icon!(019, charging),
        (true, 20) => include_numeric_icon!(020, charging),
        (true, 21) => include_numeric_icon!(021, charging),
        (true, 22) => include_numeric_icon!(022, charging),
        (true, 23) => include_numeric_icon!(023, charging),
        (true, 24) => include_numeric_icon!(024, charging),
        (true, 25) => include_numeric_icon!(025, charging),
        (true, 26) => include_numeric_icon!(026, charging),
        (true, 27) => include_numeric_icon!(027, charging),
        (true, 28) => include_numeric_icon!(028, charging),
        (true, 29) => include_numeric_icon!(029, charging),
        (true, 30) => include_numeric_icon!(030, charging),
        (true, 31) => include_numeric_icon!(031, charging),
        (true, 32) => include_numeric_icon!(032, charging),
        (true, 33) => include_numeric_icon!(033, charging),
        (true, 34) => include_numeric_icon!(034, charging),
        (true, 35) => include_numeric_icon!(035, charging),
        (true, 36) => include_numeric_icon!(036, charging),
        (true, 37) => include_numeric_icon!(037, charging),
        (true, 38) => include_numeric_icon!(038, charging),
        (true, 39) => include_numeric_icon!(039, charging),
        (true, 40) => include_numeric_icon!(040, charging),
        (true, 41) => include_numeric_icon!(041, charging),
        (true, 42) => include_numeric_icon!(042, charging),
        (true, 43) => include_numeric_icon!(043, charging),
        (true, 44) => include_numeric_icon!(044, charging),
        (true, 45) => include_numeric_icon!(045, charging),
        (true, 46) => include_numeric_icon!(046, charging),
        (true, 47) => include_numeric_icon!(047, charging),
        (true, 48) => include_numeric_icon!(048, charging),
        (true, 49) => include_numeric_icon!(049, charging),
        (true, 50) => include_numeric_icon!(050, charging),
        (true, 51) => include_numeric_icon!(051, charging),
        (true, 52) => include_numeric_icon!(052, charging),
        (true, 53) => include_numeric_icon!(053, charging),
        (true, 54) => include_numeric_icon!(054, charging),
        (true, 55) => include_numeric_icon!(055, charging),
        (true, 56) => include_numeric_icon!(056, charging),
        (true, 57) => include_numeric_icon!(057, charging),
        (true, 58) => include_numeric_icon!(058, charging),
        (true, 59) => include_numeric_icon!(059, charging),
        (true, 60) => include_numeric_icon!(060, charging),
        (true, 61) => include_numeric_icon!(061, charging),
        (true, 62) => include_numeric_icon!(062, charging),
        (true, 63) => include_numeric_icon!(063, charging),
        (true, 64) => include_numeric_icon!(064, charging),
        (true, 65) => include_numeric_icon!(065, charging),
        (true, 66) => include_numeric_icon!(066, charging),
        (true, 67) => include_numeric_icon!(067, charging),
        (true, 68) => include_numeric_icon!(068, charging),
        (true, 69) => include_numeric_icon!(069, charging),
        (true, 70) => include_numeric_icon!(070, charging),
        (true, 71) => include_numeric_icon!(071, charging),
        (true, 72) => include_numeric_icon!(072, charging),
        (true, 73) => include_numeric_icon!(073, charging),
        (true, 74) => include_numeric_icon!(074, charging),
        (true, 75) => include_numeric_icon!(075, charging),
        (true, 76) => include_numeric_icon!(076, charging),
        (true, 77) => include_numeric_icon!(077, charging),
        (true, 78) => include_numeric_icon!(078, charging),
        (true, 79) => include_numeric_icon!(079, charging),
        (true, 80) => include_numeric_icon!(080, charging),
        (true, 81) => include_numeric_icon!(081, charging),
        (true, 82) => include_numeric_icon!(082, charging),
        (true, 83) => include_numeric_icon!(083, charging),
        (true, 84) => include_numeric_icon!(084, charging),
        (true, 85) => include_numeric_icon!(085, charging),
        (true, 86) => include_numeric_icon!(086, charging),
        (true, 87) => include_numeric_icon!(087, charging),
        (true, 88) => include_numeric_icon!(088, charging),
        (true, 89) => include_numeric_icon!(089, charging),
        (true, 90) => include_numeric_icon!(090, charging),
        (true, 91) => include_numeric_icon!(091, charging),
        (true, 92) => include_numeric_icon!(092, charging),
        (true, 93) => include_numeric_icon!(093, charging),
        (true, 94) => include_numeric_icon!(094, charging),
        (true, 95) => include_numeric_icon!(095, charging),
        (true, 96) => include_numeric_icon!(096, charging),
        (true, 97) => include_numeric_icon!(097, charging),
        (true, 98) => include_numeric_icon!(098, charging),
        (true, 99) => include_numeric_icon!(099, charging),
        (true, 100) => include_numeric_icon!(100, charging),

        // Normal (non-charging) icons
        (false, 0) => include_numeric_icon!(000, normal),
        (false, 1) => include_numeric_icon!(001, normal),
        (false, 2) => include_numeric_icon!(002, normal),
        (false, 3) => include_numeric_icon!(003, normal),
        (false, 4) => include_numeric_icon!(004, normal),
        (false, 5) => include_numeric_icon!(005, normal),
        (false, 6) => include_numeric_icon!(006, normal),
        (false, 7) => include_numeric_icon!(007, normal),
        (false, 8) => include_numeric_icon!(008, normal),
        (false, 9) => include_numeric_icon!(009, normal),
        (false, 10) => include_numeric_icon!(010, normal),
        (false, 11) => include_numeric_icon!(011, normal),
        (false, 12) => include_numeric_icon!(012, normal),
        (false, 13) => include_numeric_icon!(013, normal),
        (false, 14) => include_numeric_icon!(014, normal),
        (false, 15) => include_numeric_icon!(015, normal),
        (false, 16) => include_numeric_icon!(016, normal),
        (false, 17) => include_numeric_icon!(017, normal),
        (false, 18) => include_numeric_icon!(018, normal),
        (false, 19) => include_numeric_icon!(019, normal),
        (false, 20) => include_numeric_icon!(020, normal),
        (false, 21) => include_numeric_icon!(021, normal),
        (false, 22) => include_numeric_icon!(022, normal),
        (false, 23) => include_numeric_icon!(023, normal),
        (false, 24) => include_numeric_icon!(024, normal),
        (false, 25) => include_numeric_icon!(025, normal),
        (false, 26) => include_numeric_icon!(026, normal),
        (false, 27) => include_numeric_icon!(027, normal),
        (false, 28) => include_numeric_icon!(028, normal),
        (false, 29) => include_numeric_icon!(029, normal),
        (false, 30) => include_numeric_icon!(030, normal),
        (false, 31) => include_numeric_icon!(031, normal),
        (false, 32) => include_numeric_icon!(032, normal),
        (false, 33) => include_numeric_icon!(033, normal),
        (false, 34) => include_numeric_icon!(034, normal),
        (false, 35) => include_numeric_icon!(035, normal),
        (false, 36) => include_numeric_icon!(036, normal),
        (false, 37) => include_numeric_icon!(037, normal),
        (false, 38) => include_numeric_icon!(038, normal),
        (false, 39) => include_numeric_icon!(039, normal),
        (false, 40) => include_numeric_icon!(040, normal),
        (false, 41) => include_numeric_icon!(041, normal),
        (false, 42) => include_numeric_icon!(042, normal),
        (false, 43) => include_numeric_icon!(043, normal),
        (false, 44) => include_numeric_icon!(044, normal),
        (false, 45) => include_numeric_icon!(045, normal),
        (false, 46) => include_numeric_icon!(046, normal),
        (false, 47) => include_numeric_icon!(047, normal),
        (false, 48) => include_numeric_icon!(048, normal),
        (false, 49) => include_numeric_icon!(049, normal),
        (false, 50) => include_numeric_icon!(050, normal),
        (false, 51) => include_numeric_icon!(051, normal),
        (false, 52) => include_numeric_icon!(052, normal),
        (false, 53) => include_numeric_icon!(053, normal),
        (false, 54) => include_numeric_icon!(054, normal),
        (false, 55) => include_numeric_icon!(055, normal),
        (false, 56) => include_numeric_icon!(056, normal),
        (false, 57) => include_numeric_icon!(057, normal),
        (false, 58) => include_numeric_icon!(058, normal),
        (false, 59) => include_numeric_icon!(059, normal),
        (false, 60) => include_numeric_icon!(060, normal),
        (false, 61) => include_numeric_icon!(061, normal),
        (false, 62) => include_numeric_icon!(062, normal),
        (false, 63) => include_numeric_icon!(063, normal),
        (false, 64) => include_numeric_icon!(064, normal),
        (false, 65) => include_numeric_icon!(065, normal),
        (false, 66) => include_numeric_icon!(066, normal),
        (false, 67) => include_numeric_icon!(067, normal),
        (false, 68) => include_numeric_icon!(068, normal),
        (false, 69) => include_numeric_icon!(069, normal),
        (false, 70) => include_numeric_icon!(070, normal),
        (false, 71) => include_numeric_icon!(071, normal),
        (false, 72) => include_numeric_icon!(072, normal),
        (false, 73) => include_numeric_icon!(073, normal),
        (false, 74) => include_numeric_icon!(074, normal),
        (false, 75) => include_numeric_icon!(075, normal),
        (false, 76) => include_numeric_icon!(076, normal),
        (false, 77) => include_numeric_icon!(077, normal),
        (false, 78) => include_numeric_icon!(078, normal),
        (false, 79) => include_numeric_icon!(079, normal),
        (false, 80) => include_numeric_icon!(080, normal),
        (false, 81) => include_numeric_icon!(081, normal),
        (false, 82) => include_numeric_icon!(082, normal),
        (false, 83) => include_numeric_icon!(083, normal),
        (false, 84) => include_numeric_icon!(084, normal),
        (false, 85) => include_numeric_icon!(085, normal),
        (false, 86) => include_numeric_icon!(086, normal),
        (false, 87) => include_numeric_icon!(087, normal),
        (false, 88) => include_numeric_icon!(088, normal),
        (false, 89) => include_numeric_icon!(089, normal),
        (false, 90) => include_numeric_icon!(090, normal),
        (false, 91) => include_numeric_icon!(091, normal),
        (false, 92) => include_numeric_icon!(092, normal),
        (false, 93) => include_numeric_icon!(093, normal),
        (false, 94) => include_numeric_icon!(094, normal),
        (false, 95) => include_numeric_icon!(095, normal),
        (false, 96) => include_numeric_icon!(096, normal),
        (false, 97) => include_numeric_icon!(097, normal),
        (false, 98) => include_numeric_icon!(098, normal),
        (false, 99) => include_numeric_icon!(099, normal),
        (false, 100) => include_numeric_icon!(100, normal),

        _ => unreachable!("Percentage should be clamped to 0-100"),
    };

    // Decode the PNG image
    let img = image::load_from_memory(bytes)
        .map_err(|e| anyhow::anyhow!("Failed to decode numeric icon for {}%: {}", percentage, e))?
        .to_rgba8();

    Ok(img)
}
