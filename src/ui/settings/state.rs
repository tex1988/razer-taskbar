use crate::model::Settings;
use std::sync::{Arc, Mutex, OnceLock};

pub struct SettingsWindowState {
    pub show_percentage: bool,
    pub percentage_text_size: u32,
    pub percentage_text_color: String,
    pub percentage_text_font: String,
    pub percentage_text_align: String,
    pub percentage_text_x: i32,
    pub percentage_text_y: i32,
    pub show_percent_symbol: bool,
    pub polling_interval_minutes: u64,
    pub run_at_startup: bool,
    pub use_custom_assets: bool,
    pub custom_assets_folder: Option<String>,
    pub icon_theme: String,
    pub show_device_type_overlay: bool,
    pub settings: Settings,
}

static STATE: OnceLock<Arc<Mutex<SettingsWindowState>>> = OnceLock::new();

pub fn init_state(s: &Settings) {
    let new = SettingsWindowState {
        show_percentage: s.show_percentage,
        percentage_text_size: s.percentage_text_size,
        percentage_text_color: s.percentage_text_color.clone(),
        percentage_text_font: s.percentage_text_font.clone(),
        percentage_text_align: s.percentage_text_align.as_str().to_string(),
        percentage_text_x: s.percentage_text_x,
        percentage_text_y: s.percentage_text_y,
        show_percent_symbol: s.show_percent_symbol,
        polling_interval_minutes: s.polling_interval_minutes,
        run_at_startup: s.run_at_startup,
        use_custom_assets: s.custom_assets_folder.is_some(),
        custom_assets_folder: s.custom_assets_folder.clone(),
        icon_theme: s.icon_theme.as_str().to_string(),
        show_device_type_overlay: s.show_device_type_overlay,
        settings: s.clone(),
    };
    // Re-initialize each time the settings window opens
    let _ = STATE.set(Arc::new(Mutex::new(new)));
}

pub fn with_state<F, R>(f: F) -> Option<R>
where F: FnOnce(&mut SettingsWindowState) -> R {
    STATE.get().map(|arc| {
        let mut guard = arc.lock().unwrap();
        f(&mut guard)
    })
}

pub fn has_changed(original: &Settings) -> bool {
    STATE.get().map(|arc| {
        let s = arc.lock().unwrap();
        s.show_percentage != original.show_percentage
            || s.percentage_text_size != original.percentage_text_size
            || s.percentage_text_color != original.percentage_text_color
            || s.percentage_text_font != original.percentage_text_font
            || s.percentage_text_align != original.percentage_text_align.as_str()
            || s.percentage_text_x != original.percentage_text_x
            || s.percentage_text_y != original.percentage_text_y
            || s.show_percent_symbol != original.show_percent_symbol
            || s.polling_interval_minutes != original.polling_interval_minutes
            || s.run_at_startup != original.run_at_startup
            || s.custom_assets_folder != original.custom_assets_folder
            || s.icon_theme != original.icon_theme.as_str()
            || s.show_device_type_overlay != original.show_device_type_overlay
    }).unwrap_or(false)
}

pub fn save_if_changed(original: &Settings) -> anyhow::Result<bool> {
    let changed = has_changed(original);
    if !changed { return Ok(false); }

    if let Some(arc) = STATE.get() {
        let s = arc.lock().unwrap();
        let mut ns = s.settings.clone();
        apply_state_to_settings(&s, &mut ns);
        if s.run_at_startup != original.run_at_startup {
            let _ = crate::util::startup::set_startup(s.run_at_startup);
        }
        ns.save()?;
    }
    Ok(changed)
}

fn apply_state_to_settings(s: &SettingsWindowState, ns: &mut Settings) {
    ns.show_percentage = s.show_percentage;
    ns.percentage_text_size = s.percentage_text_size;
    ns.percentage_text_color = s.percentage_text_color.clone();
    ns.percentage_text_font = s.percentage_text_font.clone();
    ns.percentage_text_align = match s.percentage_text_align.as_str() {
        "left" => crate::model::TextAlignment::Left,
        "right" => crate::model::TextAlignment::Right,
        _ => crate::model::TextAlignment::Center,
    };
    ns.percentage_text_x = s.percentage_text_x;
    ns.percentage_text_y = s.percentage_text_y;
    ns.show_percent_symbol = s.show_percent_symbol;
    ns.polling_interval_minutes = s.polling_interval_minutes;
    ns.run_at_startup = s.run_at_startup;
    ns.custom_assets_folder = s.custom_assets_folder.clone();
    ns.icon_theme = match s.icon_theme.as_str() {
        "light" => crate::model::IconTheme::Light,
        "system" => crate::model::IconTheme::System,
        _ => crate::model::IconTheme::Dark,
    };
    ns.show_device_type_overlay = s.show_device_type_overlay;
}

