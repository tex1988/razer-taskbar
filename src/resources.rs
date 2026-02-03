// Embedded resources - battery icons
use std::collections::HashMap;
use std::sync::Mutex;
use std::path::PathBuf;
use anyhow::Result;
use image::{RgbaImage, imageops};
use lazy_static::lazy_static;

lazy_static! {
    static ref ASSET_CACHE: HashMap<&'static str, &'static [u8]> = {
        let mut m = HashMap::new();

        // Regular battery icons
        m.insert("battery0.png", include_bytes!("assets/regular-icon/battery0.png") as &[u8]);
        m.insert("battery25.png", include_bytes!("assets/regular-icon/battery25.png") as &[u8]);
        m.insert("battery50.png", include_bytes!("assets/regular-icon/battery50.png") as &[u8]);
        m.insert("battery75.png", include_bytes!("assets/regular-icon/battery75.png") as &[u8]);
        m.insert("battery100.png", include_bytes!("assets/regular-icon/battery100.png") as &[u8]);

        // Charging overlay
        m.insert("chrg_overlay.png", include_bytes!("assets/chrg_overlay.png") as &[u8]);

        // Unknown battery icon
        m.insert("battery_unknown.png", include_bytes!("assets/battery_unknown.png") as &[u8]);

        m
    };

    static ref CUSTOM_ASSETS_FOLDER: Mutex<Option<PathBuf>> = Mutex::new(None);
}

/// Set the custom assets folder path
pub fn set_custom_assets_folder(path: Option<PathBuf>) {
    *CUSTOM_ASSETS_FOLDER.lock().unwrap() = path;
}

/// Get the custom assets folder path
pub fn get_custom_assets_folder() -> Option<PathBuf> {
    CUSTOM_ASSETS_FOLDER.lock().unwrap().clone()
}

pub fn load_embedded_image(filename: &str) -> Result<RgbaImage> {
    // First, try to load from custom assets folder if set
    if let Some(custom_folder) = get_custom_assets_folder() {
        // For regular battery icons, look in regular-icon subfolder
        let paths_to_try = if filename.starts_with("battery") && filename != "battery_unknown.png" {
            vec![
                custom_folder.join("regular-icon").join(filename),
                custom_folder.join(filename),
            ]
        } else {
            vec![custom_folder.join(filename)]
        };

        for path in paths_to_try {
            if path.exists() {
                match image::open(&path) {
                    Ok(img) => return Ok(img.to_rgba8()),
                    Err(e) => eprintln!("Failed to load custom asset {}: {}", path.display(), e),
                }
            }
        }
    }

    // Fallback to embedded resources
    if let Some(bytes) = ASSET_CACHE.get(filename) {
        let img = image::load_from_memory(bytes)
            .map_err(|e| anyhow::anyhow!("Failed to decode embedded image {}: {}", filename, e))?
            .to_rgba8();
        Ok(img)
    } else {
        Err(anyhow::anyhow!("Asset {} not found in embedded resources", filename))
    }
}

/// Apply the charging overlay on top of a battery icon
pub fn apply_charging_overlay(mut base_img: RgbaImage) -> Result<RgbaImage> {
    let overlay = load_embedded_image("chrg_overlay.png")?;

    // Calculate position to place overlay on the right side
    let base_width = base_img.width() as i64;
    let overlay_width = overlay.width() as i64;

    // Position the overlay on the right side (aligned to the right edge)
    let x_pos = base_width - overlay_width;
    let y_pos = 0i64; // Keep at top, adjust if needed

    // Overlay the charging indicator on the right side of the base image
    imageops::overlay(&mut base_img, &overlay, x_pos, y_pos);

    Ok(base_img)
}

// Generate numeric icon data on the fly
pub fn generate_numeric_icon(percentage: u8, is_charging: bool) -> Result<RgbaImage> {
    // Clamp percentage to 0-100
    let percentage = percentage.min(100);

    // Build the filename for the numeric icon
    let filename = format!("battery{:03}.png", percentage);

    // First, try to load from custom assets folder if set
    if let Some(custom_folder) = get_custom_assets_folder() {
        let paths_to_try = vec![
            custom_folder.join("numeric-icon").join(&filename),
            custom_folder.join(&filename),
        ];

        for path in paths_to_try {
            if path.exists() {
                match image::open(&path) {
                    Ok(img) => {
                        let mut img = img.to_rgba8();
                        // Apply charging overlay if needed
                        if is_charging {
                            img = apply_charging_overlay(img)?;
                        }
                        return Ok(img);
                    }
                    Err(e) => eprintln!("Failed to load custom numeric asset {}: {}", path.display(), e),
                }
            }
        }
    }

    // Fallback to embedded numeric icons
    // Use a macro to generate the match arms for all percentages (only normal icons)
    macro_rules! include_numeric_icon {
        ($p:expr) => {
            include_bytes!(concat!("assets/numeric-icon/battery", stringify!($p), ".png"))
        };
    }

    // Match the exact percentage to load the correct embedded icon (always normal, non-charging)
    let bytes: &[u8] = match percentage {
        0 => include_numeric_icon!(000),
        1 => include_numeric_icon!(001),
        2 => include_numeric_icon!(002),
        3 => include_numeric_icon!(003),
        4 => include_numeric_icon!(004),
        5 => include_numeric_icon!(005),
        6 => include_numeric_icon!(006),
        7 => include_numeric_icon!(007),
        8 => include_numeric_icon!(008),
        9 => include_numeric_icon!(009),
        10 => include_numeric_icon!(010),
        11 => include_numeric_icon!(011),
        12 => include_numeric_icon!(012),
        13 => include_numeric_icon!(013),
        14 => include_numeric_icon!(014),
        15 => include_numeric_icon!(015),
        16 => include_numeric_icon!(016),
        17 => include_numeric_icon!(017),
        18 => include_numeric_icon!(018),
        19 => include_numeric_icon!(019),
        20 => include_numeric_icon!(020),
        21 => include_numeric_icon!(021),
        22 => include_numeric_icon!(022),
        23 => include_numeric_icon!(023),
        24 => include_numeric_icon!(024),
        25 => include_numeric_icon!(025),
        26 => include_numeric_icon!(026),
        27 => include_numeric_icon!(027),
        28 => include_numeric_icon!(028),
        29 => include_numeric_icon!(029),
        30 => include_numeric_icon!(030),
        31 => include_numeric_icon!(031),
        32 => include_numeric_icon!(032),
        33 => include_numeric_icon!(033),
        34 => include_numeric_icon!(034),
        35 => include_numeric_icon!(035),
        36 => include_numeric_icon!(036),
        37 => include_numeric_icon!(037),
        38 => include_numeric_icon!(038),
        39 => include_numeric_icon!(039),
        40 => include_numeric_icon!(040),
        41 => include_numeric_icon!(041),
        42 => include_numeric_icon!(042),
        43 => include_numeric_icon!(043),
        44 => include_numeric_icon!(044),
        45 => include_numeric_icon!(045),
        46 => include_numeric_icon!(046),
        47 => include_numeric_icon!(047),
        48 => include_numeric_icon!(048),
        49 => include_numeric_icon!(049),
        50 => include_numeric_icon!(050),
        51 => include_numeric_icon!(051),
        52 => include_numeric_icon!(052),
        53 => include_numeric_icon!(053),
        54 => include_numeric_icon!(054),
        55 => include_numeric_icon!(055),
        56 => include_numeric_icon!(056),
        57 => include_numeric_icon!(057),
        58 => include_numeric_icon!(058),
        59 => include_numeric_icon!(059),
        60 => include_numeric_icon!(060),
        61 => include_numeric_icon!(061),
        62 => include_numeric_icon!(062),
        63 => include_numeric_icon!(063),
        64 => include_numeric_icon!(064),
        65 => include_numeric_icon!(065),
        66 => include_numeric_icon!(066),
        67 => include_numeric_icon!(067),
        68 => include_numeric_icon!(068),
        69 => include_numeric_icon!(069),
        70 => include_numeric_icon!(070),
        71 => include_numeric_icon!(071),
        72 => include_numeric_icon!(072),
        73 => include_numeric_icon!(073),
        74 => include_numeric_icon!(074),
        75 => include_numeric_icon!(075),
        76 => include_numeric_icon!(076),
        77 => include_numeric_icon!(077),
        78 => include_numeric_icon!(078),
        79 => include_numeric_icon!(079),
        80 => include_numeric_icon!(080),
        81 => include_numeric_icon!(081),
        82 => include_numeric_icon!(082),
        83 => include_numeric_icon!(083),
        84 => include_numeric_icon!(084),
        85 => include_numeric_icon!(085),
        86 => include_numeric_icon!(086),
        87 => include_numeric_icon!(087),
        88 => include_numeric_icon!(088),
        89 => include_numeric_icon!(089),
        90 => include_numeric_icon!(090),
        91 => include_numeric_icon!(091),
        92 => include_numeric_icon!(092),
        93 => include_numeric_icon!(093),
        94 => include_numeric_icon!(094),
        95 => include_numeric_icon!(095),
        96 => include_numeric_icon!(096),
        97 => include_numeric_icon!(097),
        98 => include_numeric_icon!(098),
        99 => include_numeric_icon!(099),
        100 => include_numeric_icon!(100),

        _ => unreachable!("Percentage should be clamped to 0-100"),
    };

    // Decode the PNG image
    let mut img = image::load_from_memory(bytes)
        .map_err(|e| anyhow::anyhow!("Failed to decode numeric icon for {}%: {}", percentage, e))?
        .to_rgba8();

    // Apply charging overlay if needed
    if is_charging {
        img = apply_charging_overlay(img)?;
    }

    Ok(img)
}
