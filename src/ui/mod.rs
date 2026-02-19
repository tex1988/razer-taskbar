pub mod constants;
pub mod context_menu;
pub mod menu_events;
pub mod settings;
pub mod tray_manager;

// Re-exports for convenience
pub use menu_events::{MenuAction, handle_menu_events};
pub use tray_manager::TrayManager;
pub use settings::SettingsWindow;


