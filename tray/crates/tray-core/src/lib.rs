//! Platform-agnostic core for the Wireless Battery system tray application.

pub mod config;
#[cfg(feature = "dev-tools")]
pub mod dev_tools;
pub mod device_kind;
pub mod notify;
pub mod service;
pub mod state;
pub mod version;

pub use config::{
    default_config_path, load_config, save_config, LanguageMode, ThemeMode, TrayConfig,
};
pub use device_kind::{classify_device, DeviceKind};
pub use notify::LowBatteryTracker;
pub use service::{PollerHandle, TrayService, TrayUpdate};
pub use state::{DeviceView, LowBatteryAlert, TrayIconState, TraySnapshot};
pub use version::{
    evaluate_release_json, is_newer_version, UpdateCheckResult, APP_DISPLAY_VERSION, APP_VERSION,
    LATEST_RELEASE_API_URL, RELEASES_URL,
};
