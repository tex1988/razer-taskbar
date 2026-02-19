use crate::device::{DeviceMap, RazerDevice};
use crate::ui_constants::*;
use anyhow::Result;
use std::sync::{Arc, Mutex};
use tray_icon::menu::{Menu, MenuEvent, MenuId, MenuItem, PredefinedMenuItem};
use tray_icon::{TrayIcon, TrayIconBuilder};

pub struct TrayManager {
    tray_icon: Option<TrayIcon>,
    quit_id: MenuId,
    settings_id: MenuId,
    devices: Arc<Mutex<DeviceMap>>,
    // Track previous state to avoid unnecessary updates
    last_displayed_device: Option<(String, u8, bool)>, // (name, battery_percentage, is_charging)
    last_icon_settings: Option<(bool, u32, String, String, String, i32, i32, bool, bool)>, // (show_percentage, text_size, text_color, font_name, text_align, text_x, text_y, show_percent_symbol, show_device_type_overlay)
}

impl TrayManager {
    pub fn new() -> Result<Self> {
        Ok(Self {
            tray_icon: None,
            quit_id: MenuId::new("quit"),
            settings_id: MenuId::new("settings"),
            devices: Arc::new(Mutex::new(DeviceMap::new())),
            last_displayed_device: None,
            last_icon_settings: None,
        })
    }

    pub fn initialize(&mut self) -> Result<()> {
        let icon = crate::icon_manager::load_unknown_icon()?;
        let menu = self.build_menu(None)?;

        let tray_icon = TrayIconBuilder::new()
            .with_tooltip(TOOLTIP_NO_DEVICES)
            .with_icon(icon)
            .with_menu(Box::new(menu))
            .build()?;

        self.tray_icon = Some(tray_icon);
        Ok(())
    }

    fn build_menu(&self, device: Option<&RazerDevice>) -> Result<Menu> {
        let menu = Menu::new();

        // Add header with device info
        if let Some(device) = device {
            // Title header - "Razer Taskbar"
            let title_item = MenuItem::new(TOOLTIP_DEFAULT_PREFIX, false, None);
            menu.append(&title_item)?;

            // Device info - "Device Name - XX%"
            let charging_text = if device.is_charging {
                " (charging)"
            } else {
                ""
            };
            let device_info = format!(
                "{} - {}%{}",
                device.name, device.battery_percentage, charging_text
            );
            let device_item = MenuItem::new(&device_info, false, None);
            menu.append(&device_item)?;

            // Separator
            menu.append(&PredefinedMenuItem::separator())?;
        } else {
            // No devices - show title only
            let title_item = MenuItem::new(TOOLTIP_DEFAULT_PREFIX, false, None);
            menu.append(&title_item)?;

            menu.append(&PredefinedMenuItem::separator())?;
        }

        // Settings item (for future use)
        let settings_item =
            MenuItem::with_id(self.settings_id.clone(), MENU_TEXT_SETTINGS, true, None);
        menu.append(&settings_item)?;

        // Quit item
        let quit_item = MenuItem::with_id(self.quit_id.clone(), MENU_TEXT_QUIT, true, None);
        menu.append(&quit_item)?;

        Ok(menu)
    }

    pub fn quit_id(&self) -> MenuId {
        self.quit_id.clone()
    }

    pub fn settings_id(&self) -> MenuId {
        self.settings_id.clone()
    }

    /// Force the next update to refresh the icon and menu regardless of state
    /// This should be called when settings change (e.g., custom assets folder)
    pub fn force_refresh(&mut self) {
        self.last_displayed_device = None;
        self.last_icon_settings = None;
    }

    pub fn update_devices(
        &mut self,
        devices: DeviceMap,
        show_percentage: bool,
        text_size: u32,
        text_color: &str,
        font_name: &str,
        text_align: &str,
        text_x: i32,
        text_y: i32,
        show_percent_symbol: bool,
        show_device_type_overlay: bool,
    ) -> Result<()> {
        *self.devices.lock().unwrap() = devices.clone();

        let device = Self::pick_device_to_display(&devices);

        if let Some(tray_icon) = &self.tray_icon {
            // Check if device info has changed
            let current_device_state = device.as_ref().map(|d| {
                (d.name.clone(), d.battery_percentage, d.is_charging)
            });

            // Check if icon settings have changed
            let current_icon_settings = (
                show_percentage,
                text_size,
                text_color.to_string(),
                font_name.to_string(),
                text_align.to_string(),
                text_x,
                text_y,
                show_percent_symbol,
                show_device_type_overlay,
            );

            let device_changed = self.last_displayed_device != current_device_state;
            let settings_changed = self.last_icon_settings.as_ref() != Some(&current_icon_settings);

            // Only update icon if device or settings changed
            if device_changed || settings_changed {
                if let Some(ref dev) = device {
                    let icon = crate::icon_manager::load_icon(
                        dev.battery_percentage,
                        dev.is_charging,
                        show_percentage,
                        text_size,
                        text_color,
                        font_name,
                        text_align,
                        text_x,
                        text_y,
                        show_percent_symbol,
                        show_device_type_overlay,
                        dev.category,
                    )?;

                    tray_icon.set_icon(Some(icon))?;

                    let charging_text = if dev.is_charging {
                        TOOLTIP_CHARGING_SUFFIX
                    } else {
                        ""
                    };
                    tray_icon.set_tooltip(Some(format!(
                        "{}: {}%{}",
                        dev.name, dev.battery_percentage, charging_text
                    )))?;
                } else {
                    let icon = crate::icon_manager::load_unknown_icon()?;
                    tray_icon.set_icon(Some(icon))?;
                    tray_icon.set_tooltip(Some(TOOLTIP_NO_DEVICES))?;
                }

                self.last_icon_settings = Some(current_icon_settings);
            }

            // Only rebuild menu if device info actually changed
            if device_changed {
                let new_menu = self.build_menu(device.as_ref())?;
                tray_icon.set_menu(Some(Box::new(new_menu)));
                self.last_displayed_device = current_device_state;
            }
        }

        Ok(())
    }

    fn pick_device_to_display(devices: &DeviceMap) -> Option<RazerDevice> {
        let mut selected_devices: Vec<_> = devices
            .values()
            .filter(|d| d.is_connected && d.is_selected)
            .cloned()
            .collect();

        selected_devices
            .sort_by_key(|d| d.battery_percentage as u32 * if d.is_charging { 100 } else { 1 });

        selected_devices.first().cloned()
    }
}

pub enum MenuAction {
    None,
    Settings,
    Quit,
}

pub fn handle_menu_events(quit_id: MenuId, settings_id: MenuId, debug: bool) -> Result<MenuAction> {
    if let Ok(event) = MenuEvent::receiver().try_recv() {
        if debug {
            println!("Menu event received: id={:?}", event.id);
        }
        if event.id == quit_id {
            if debug {
                println!("Quit menu clicked!");
            }
            return Ok(MenuAction::Quit);
        } else if event.id == settings_id {
            if debug {
                println!("Settings menu clicked!");
            }
            return Ok(MenuAction::Settings);
        }
    }
    Ok(MenuAction::None)
}
