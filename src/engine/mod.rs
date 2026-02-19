pub mod event_loop;
pub mod icon_manager;
pub mod watcher_common;
pub mod watcher_v3;
pub mod watcher_v4;

pub use event_loop::run_event_loop;
pub use icon_manager::{
    create_theme_change_listener,
    set_custom_assets_folder, set_icon_theme,
};
pub use watcher_v3::SynapseV3Watcher;
pub use watcher_v4::SynapseV4Watcher;
