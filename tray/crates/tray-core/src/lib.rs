//! Platform-agnostic core for the Wireless Battery system tray application.

pub mod config;
#[cfg(feature = "dev-tools")]
pub mod dev_tools;
pub mod device_kind;
pub mod links;
pub mod notify;
pub mod service;
pub mod state;
pub mod version;

pub use config::{
    default_config_path, load_config, save_config, LanguageMode, ThemeMode, TrayConfig,
};
pub use device_kind::{classify_device, DeviceKind};
pub use links::{
    community_links, is_valid_http_url, CommunityLinks, DISCORD_INVITE_URL, ISSUES_URL,
    QQ_GROUP_NUMBER, QQ_GROUP_URL, REPO_URL,
};
pub use notify::LowBatteryTracker;
pub use service::{PollerHandle, TrayService, TrayUpdate};
pub use state::{
    format_footer_summary, format_footer_summary_rfc3339, format_poll_label, format_relative_time,
    format_relative_time_rfc3339, format_relative_time_secs, format_tooltip,
    format_tooltip_with_locale, DeviceView, LowBatteryAlert, TrayIconState, TraySnapshot,
};
pub use version::{
    evaluate_release_json, is_newer_version, is_valid_update_url, UpdateCheckResult,
    APP_DISPLAY_VERSION, APP_VERSION, GITHUB_API_HOST, LATEST_RELEASE_API_PATH,
    LATEST_RELEASE_API_URL, RELEASES_URL,
};
