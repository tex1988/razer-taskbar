use crate::device::{DeviceMap, RazerDevice};
use crate::ui_constants::*;
use anyhow::Result;
use std::sync::{Arc, Mutex};
use tray_icon::menu::{Menu, MenuEvent, MenuId, MenuItem, PredefinedMenuItem};
use tray_icon::{Icon, TrayIcon, TrayIconBuilder};

pub struct TrayManager {
    tray_icon: Option<TrayIcon>,
    quit_id: MenuId,
    settings_id: MenuId,
    devices: Arc<Mutex<DeviceMap>>,
}

impl TrayManager {
    pub fn new() -> Result<Self> {
        Ok(Self {
            tray_icon: None,
            quit_id: MenuId::new("quit"),
            settings_id: MenuId::new("settings"),
            devices: Arc::new(Mutex::new(DeviceMap::new())),
        })
    }

    pub fn initialize(&mut self) -> Result<()> {
        let icon = self.create_unknown_battery_icon()?;
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

    pub fn update_devices(
        &mut self,
        devices: DeviceMap,
        show_percentage: bool,
        display_charging: bool,
    ) -> Result<()> {
        *self.devices.lock().unwrap() = devices.clone();

        let device = Self::pick_device_to_display(&devices);

        if let Some(tray_icon) = &self.tray_icon {
            // Update icon
            if let Some(ref dev) = device {
                let icon = if show_percentage {
                    self.create_numeric_battery_icon(
                        dev.battery_percentage,
                        dev.is_charging && display_charging,
                    )?
                } else {
                    self.create_battery_icon(
                        dev.battery_percentage,
                        dev.is_charging && display_charging,
                    )?
                };

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
                let icon = self.create_unknown_battery_icon()?;
                tray_icon.set_icon(Some(icon))?;
                tray_icon.set_tooltip(Some(TOOLTIP_NO_DEVICES))?;
            }

            // Rebuild menu with updated device info
            let new_menu = self.build_menu(device.as_ref())?;
            tray_icon.set_menu(Some(Box::new(new_menu)));
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

    fn create_unknown_battery_icon(&self) -> Result<Icon> {
        // Load the unknown battery icon for when no devices are found
        let img = crate::resources::load_embedded_image("battery_unknown.png")?;

        let (width, height) = img.dimensions();
        let rgba = img.into_raw();

        Ok(Icon::from_rgba(rgba, width, height)?)
    }

    fn create_battery_icon(
        &self,
        percentage: u8,
        is_charging: bool,
    ) -> Result<Icon> {
        // Determine which asset to load based on battery level
        let level = if percentage <= 12 {
            0
        } else if percentage <= 37 {
            25
        } else if percentage <= 62 {
            50
        } else if percentage <= 87 {
            75
        } else {
            100
        };

        // Build the asset filename (always use non-charging version)
        let filename = format!("battery{}.png", level);

        // Load the PNG from embedded resources
        let mut img = crate::resources::load_embedded_image(&filename)?;

        // Apply charging overlay if needed
        if is_charging {
            img = crate::resources::apply_charging_overlay(img)?;
        }

        let (width, height) = img.dimensions();
        let rgba = img.into_raw();

        Ok(Icon::from_rgba(rgba, width, height)?)
    }

    fn create_numeric_battery_icon(
        &self,
        percentage: u8,
        is_charging: bool,
    ) -> Result<Icon> {
        // Use generate_numeric_icon from resources module
        let img = crate::resources::generate_numeric_icon(percentage, is_charging)?;

        let (width, height) = img.dimensions();
        let rgba = img.into_raw();

        Ok(Icon::from_rgba(rgba, width, height)?)
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
