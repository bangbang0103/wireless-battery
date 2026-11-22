//! Platform-agnostic core for the Wireless Battery system tray application.

pub mod config;
#[cfg(feature = "dev-tools")]
pub mod dev_tools;
pub mod device_kind;
pub mod notify;
pub mod service;
pub mod state;

pub use config::{
    default_config_path, load_config, save_config, LanguageMode, ThemeMode, TrayConfig,
};
pub use device_kind::{classify_device, DeviceKind};
pub use notify::LowBatteryTracker;
pub use service::{PollerHandle, TrayService, TrayUpdate};
pub use state::{DeviceView, LowBatteryAlert, TrayIconState, TraySnapshot};
