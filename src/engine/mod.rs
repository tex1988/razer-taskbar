pub mod event_loop;
pub mod icon_manager;
pub mod watcher_common;
#[cfg(debug_assertions)]
pub mod watcher_emulated;
pub mod watcher_v3;
pub mod watcher_v4;

pub use event_loop::run_event_loop;
pub use icon_manager::{
    create_theme_change_listener,
    set_icon_theme,
    set_themes_config, scan_themes, default_themes_root,
};
#[cfg(debug_assertions)]
pub use watcher_emulated::EmulationWatcher;
pub use watcher_v3::SynapseV3Watcher;
pub use watcher_v4::SynapseV4Watcher;
