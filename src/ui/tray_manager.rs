use crate::engine::icon_manager;
use crate::model::{DeviceMap, RazerDevice, IconSettings};
use super::constants::*;
use anyhow::Result;
use std::collections::HashMap;
use std::sync::{Arc, Mutex};
use tray_icon::menu::{Menu, MenuId, MenuItem, PredefinedMenuItem};
use tray_icon::{TrayIcon, TrayIconBuilder};

/// Per-device cached state used for change detection.
type DeviceState = (String, u8, bool); // (name, battery_percentage, is_charging)

pub struct TrayManager {
    /// One tray icon per connected+selected device, keyed by device unique_id.
    tray_icons: HashMap<String, TrayIcon>,
    /// Fallback icon shown when no devices are available.
    fallback_icon: Option<TrayIcon>,
    quit_id: MenuId,
    settings_id: MenuId,
    devices: Arc<Mutex<DeviceMap>>,
    /// Cached state per device unique_id for change detection.
    last_device_states: HashMap<String, DeviceState>,
    last_icon_settings: Option<IconSettings>,
}

impl TrayManager {
    pub fn new() -> Result<Self> {
        Ok(Self {
            tray_icons: HashMap::new(),
            fallback_icon: None,
            quit_id: MenuId::new("quit"),
            settings_id: MenuId::new("settings"),
            devices: Arc::new(Mutex::new(DeviceMap::new())),
            last_device_states: HashMap::new(),
            last_icon_settings: None,
        })
    }

    pub fn initialize(&mut self) -> Result<()> {
        let icon = icon_manager::load_unknown_icon()?;
        let menu = self.build_fallback_menu()?;

        let tray_icon = TrayIconBuilder::new()
            .with_tooltip(TOOLTIP_NO_DEVICES)
            .with_icon(icon)
            .with_menu(Box::new(menu))
            .build()?;

        self.fallback_icon = Some(tray_icon);
        Ok(())
    }

    // ── Menu builders ──────────────────────────────────────────

    fn build_device_menu(&self, device: &RazerDevice) -> Result<Menu> {
        let menu = Menu::new();

        let title = MenuItem::new(TOOLTIP_DEFAULT_PREFIX, false, None);
        menu.append(&title)?;

        let charging = if device.is_charging { " (charging)" } else { "" };
        let info = format!("{} - {}%{}", device.name, device.battery_percentage, charging);
        menu.append(&MenuItem::new(&info, false, None))?;

        // Serial number line
        if let Some(ref sn) = device.serial_number {
            let sn_line = format!("SN: {}", sn);
            menu.append(&MenuItem::new(&sn_line, false, None))?;
        }

        menu.append(&PredefinedMenuItem::separator())?;

        let settings = MenuItem::with_id(self.settings_id.clone(), MENU_TEXT_SETTINGS, true, None);
        menu.append(&settings)?;
        let quit = MenuItem::with_id(self.quit_id.clone(), MENU_TEXT_QUIT, true, None);
        menu.append(&quit)?;

        Ok(menu)
    }

    fn build_fallback_menu(&self) -> Result<Menu> {
        let menu = Menu::new();

        let title = MenuItem::new(TOOLTIP_DEFAULT_PREFIX, false, None);
        menu.append(&title)?;
        menu.append(&PredefinedMenuItem::separator())?;

        let settings = MenuItem::with_id(self.settings_id.clone(), MENU_TEXT_SETTINGS, true, None);
        menu.append(&settings)?;
        let quit = MenuItem::with_id(self.quit_id.clone(), MENU_TEXT_QUIT, true, None);
        menu.append(&quit)?;

        Ok(menu)
    }

    // ── Public accessors ───────────────────────────────────────

    pub fn quit_id(&self) -> MenuId { self.quit_id.clone() }
    pub fn settings_id(&self) -> MenuId { self.settings_id.clone() }

    pub fn force_refresh(&mut self) {
        self.last_device_states.clear();
        self.last_icon_settings = None;
    }

    // ── Core update logic ──────────────────────────────────────

    pub fn update_devices(
        &mut self,
        devices: DeviceMap,
        icon_settings: &IconSettings,
    ) -> Result<()> {
        *self.devices.lock().unwrap() = devices.clone();

        let settings_changed = self.last_icon_settings.as_ref() != Some(icon_settings);

        // Collect active devices.
        let active: Vec<&RazerDevice> = devices.values()
            .filter(|d| d.is_connected && d.is_selected)
            .collect();

        let active_ids: Vec<String> = active.iter().map(|d| d.unique_id().to_string()).collect();

        // ── Remove icons for devices no longer active ──────────
        self.tray_icons.retain(|uid, _| active_ids.contains(uid));
        self.last_device_states.retain(|uid, _| active_ids.contains(uid));

        // ── Manage fallback icon ───────────────────────────────
        if active.is_empty() {
            if self.fallback_icon.is_none() {
                let icon = icon_manager::load_unknown_icon()?;
                let menu = self.build_fallback_menu()?;
                let tray_icon = TrayIconBuilder::new()
                    .with_tooltip(TOOLTIP_NO_DEVICES)
                    .with_icon(icon)
                    .with_menu(Box::new(menu))
                    .build()?;
                self.fallback_icon = Some(tray_icon);
            }
            self.last_icon_settings = Some(icon_settings.clone());
            return Ok(());
        }

        // We have active devices — drop the fallback icon.
        self.fallback_icon = None;

        // ── Create / update per-device icons ───────────────────
        for device in &active {
            let uid = device.unique_id().to_string();
            let current_state: DeviceState = (
                device.name.clone(),
                device.battery_percentage,
                device.is_charging,
            );

            let device_changed = self.last_device_states.get(&uid) != Some(&current_state);

            if self.tray_icons.contains_key(&uid) {
                // Icon exists — update if state or settings changed.
                if device_changed || settings_changed {
                    if let Some(tray_icon) = self.tray_icons.get(&uid) {
                        self.apply_device_to_icon(tray_icon, device, icon_settings)?;
                    }
                }
            } else {
                // New device — create a tray icon.
                let tray_icon = self.create_device_tray_icon(device, icon_settings)?;
                self.tray_icons.insert(uid.clone(), tray_icon);
            }

            self.last_device_states.insert(uid, current_state);
        }

        self.last_icon_settings = Some(icon_settings.clone());
        Ok(())
    }

    // ── Icon creation & update helpers ─────────────────────────

    fn create_device_tray_icon(
        &self,
        device: &RazerDevice,
        icon_settings: &IconSettings,
    ) -> Result<TrayIcon> {
        let icon = self.make_device_icon(device, icon_settings)?;
        let tooltip = Self::make_device_tooltip(device);
        let menu = self.build_device_menu(device)?;

        let tray_icon = TrayIconBuilder::new()
            .with_tooltip(&tooltip)
            .with_icon(icon)
            .with_menu(Box::new(menu))
            .build()?;

        Ok(tray_icon)
    }

    fn apply_device_to_icon(
        &self,
        tray_icon: &TrayIcon,
        device: &RazerDevice,
        icon_settings: &IconSettings,
    ) -> Result<()> {
        let icon = self.make_device_icon(device, icon_settings)?;
        tray_icon.set_icon(Some(icon))?;
        tray_icon.set_tooltip(Some(Self::make_device_tooltip(device)))?;

        let menu = self.build_device_menu(device)?;
        tray_icon.set_menu(Some(Box::new(menu)));

        Ok(())
    }

    fn make_device_icon(
        &self,
        device: &RazerDevice,
        icon_settings: &IconSettings,
    ) -> Result<tray_icon::Icon> {
        let params = icon_manager::LoadIconParams {
            percentage: device.battery_percentage,
            is_charging: device.is_charging,
            icon_settings: icon_settings.clone(),
            device_category: device.category,
        };
        icon_manager::load_icon(&params)
    }

    fn make_device_tooltip(device: &RazerDevice) -> String {
        let charging = if device.is_charging { TOOLTIP_CHARGING_SUFFIX } else { "" };
        format!("{}: {}%{}", device.name, device.battery_percentage, charging)
    }
}
