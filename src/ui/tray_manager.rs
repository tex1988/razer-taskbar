use crate::engine::icon_manager;
use crate::model::{DeviceMap, RazerDevice, IconSettings};
use super::constants::*;
use anyhow::Result;
use std::sync::{Arc, Mutex};
use tray_icon::menu::{Menu, MenuId, MenuItem, PredefinedMenuItem};
use tray_icon::{TrayIcon, TrayIconBuilder};

pub struct TrayManager {
    tray_icon: Option<TrayIcon>,
    quit_id: MenuId,
    settings_id: MenuId,
    devices: Arc<Mutex<DeviceMap>>,
    last_displayed_device: Option<(String, u8, bool)>,
    last_icon_settings: Option<IconSettings>,
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
        let icon = icon_manager::load_unknown_icon()?;
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
        self.add_header_items(&menu, device)?;
        self.add_action_items(&menu)?;
        Ok(menu)
    }

    fn add_header_items(&self, menu: &Menu, device: Option<&RazerDevice>) -> Result<()> {
        let title = MenuItem::new(TOOLTIP_DEFAULT_PREFIX, false, None);
        menu.append(&title)?;

        if let Some(dev) = device {
            let charging = if dev.is_charging { " (charging)" } else { "" };
            let info = format!("{} - {}%{}", dev.name, dev.battery_percentage, charging);
            menu.append(&MenuItem::new(&info, false, None))?;
        }

        menu.append(&PredefinedMenuItem::separator())?;
        Ok(())
    }

    fn add_action_items(&self, menu: &Menu) -> Result<()> {
        let settings = MenuItem::with_id(self.settings_id.clone(), MENU_TEXT_SETTINGS, true, None);
        menu.append(&settings)?;
        let quit = MenuItem::with_id(self.quit_id.clone(), MENU_TEXT_QUIT, true, None);
        menu.append(&quit)?;
        Ok(())
    }

    pub fn quit_id(&self) -> MenuId { self.quit_id.clone() }
    pub fn settings_id(&self) -> MenuId { self.settings_id.clone() }

    pub fn force_refresh(&mut self) {
        self.last_displayed_device = None;
        self.last_icon_settings = None;
    }

    pub fn update_devices(
        &mut self,
        devices: DeviceMap,
        icon_settings: &IconSettings,
    ) -> Result<()> {
        *self.devices.lock().unwrap() = devices.clone();
        let device = Self::pick_device_to_display(&devices);

        if let Some(tray_icon) = &self.tray_icon {
            let current_device_state = device.as_ref().map(|d| {
                (d.name.clone(), d.battery_percentage, d.is_charging)
            });

            let device_changed = self.last_displayed_device != current_device_state;
            let settings_changed = self.last_icon_settings.as_ref() != Some(icon_settings);

            if device_changed || settings_changed {
                self.update_icon(tray_icon, device.as_ref(), icon_settings)?;
                self.last_icon_settings = Some(icon_settings.clone());
            }

            if device_changed {
                let new_menu = self.build_menu(device.as_ref())?;
                tray_icon.set_menu(Some(Box::new(new_menu)));
                self.last_displayed_device = current_device_state;
            }
        }

        Ok(())
    }

    fn update_icon(
        &self,
        tray_icon: &TrayIcon,
        device: Option<&RazerDevice>,
        icon_settings: &IconSettings,
    ) -> Result<()> {
        if let Some(dev) = device {
            let params = icon_manager::LoadIconParams {
                percentage: dev.battery_percentage,
                is_charging: dev.is_charging,
                icon_settings: icon_settings.clone(),
                device_category: dev.category,
            };
            let icon = icon_manager::load_icon(&params)?;
            tray_icon.set_icon(Some(icon))?;
            let charging = if dev.is_charging { TOOLTIP_CHARGING_SUFFIX } else { "" };
            tray_icon.set_tooltip(Some(format!(
                "{}: {}%{}", dev.name, dev.battery_percentage, charging
            )))?;
        } else {
            let icon = icon_manager::load_unknown_icon()?;
            tray_icon.set_icon(Some(icon))?;
            tray_icon.set_tooltip(Some(TOOLTIP_NO_DEVICES))?;
        }
        Ok(())
    }

    fn pick_device_to_display(devices: &DeviceMap) -> Option<RazerDevice> {
        let mut selected: Vec<_> = devices.values()
            .filter(|d| d.is_connected && d.is_selected)
            .cloned()
            .collect();
        selected.sort_by_key(|d| {
            d.battery_percentage as u32 * if d.is_charging { 100 } else { 1 }
        });
        selected.first().cloned()
    }
}


