use anyhow::Result;
use tray_icon::menu::{MenuEvent, MenuId};

pub enum MenuAction {
    None,
    Settings,
    Quit,
}

pub fn handle_menu_events(
    quit_id: MenuId,
    settings_id: MenuId,
    debug: bool,
) -> Result<MenuAction> {
    if let Ok(event) = MenuEvent::receiver().try_recv() {
        if debug {
            println!("Menu event received: id={:?}", event.id);
        }
        if event.id == quit_id {
            if debug { println!("Quit menu clicked!"); }
            return Ok(MenuAction::Quit);
        } else if event.id == settings_id {
            if debug { println!("Settings menu clicked!"); }
            return Ok(MenuAction::Settings);
        }
    }
    Ok(MenuAction::None)
}

