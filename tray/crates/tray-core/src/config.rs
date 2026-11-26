//! Persistent user configuration for the tray application.

use serde::{Deserialize, Serialize};
use std::fs;
use std::io;
use std::path::{Path, PathBuf};
use std::time::Duration;

/// Default polling interval: 5 minutes.
pub const DEFAULT_POLL_INTERVAL_SECS: u64 = 300;
/// Default low-battery alert threshold (%).
pub const DEFAULT_LOW_BATTERY_THRESHOLD: u8 = 15;
/// Default per-device read timeout (ms). Matches CLI recommendation so multi-attempt
/// providers like WOB Rainy75 (3 x 1200ms) and Rapoo VT3S (4500ms) have full budget.
pub const DEFAULT_READ_TIMEOUT_MS: u64 = 5_000;

/// User-selected appearance theme for the tray UI.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum ThemeMode {
    /// Follow the operating system's light/dark appearance automatically.
    #[default]
    System,
    /// Always render the light theme.
    Light,
    /// Always render the dark theme.
    Dark,
}

impl ThemeMode {
    /// Cycle through `System -> Light -> Dark -> System`.
    #[must_use]
    pub const fn next(self) -> Self {
        match self {
            Self::System => Self::Light,
            Self::Light => Self::Dark,
            Self::Dark => Self::System,
        }
    }
}

/// User-selected UI language preference.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum LanguageMode {
    /// Follow the operating system's UI language automatically.
    #[default]
    System,
    /// Simplified Chinese (`简体中文`).
    ZhCn,
    /// English.
    En,
}

/// User-facing tray settings aligned with `wireless-battery` safety policies.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(default)]
pub struct TrayConfig {
    /// Background polling interval in seconds. `0` disables automatic polling (manual only).
    pub poll_interval_secs: u64,
    /// Return the last successful reading marked as `Stale` when a refresh fails.
    pub allow_stale: bool,
    /// Enable low-battery warnings (tray badge & tiered OS notifications at 15%, 10%, 5%, 1%).
    pub low_battery_notify: bool,
    /// Sub-option under `low_battery_notify`: when true, suppress OS system notifications
    /// while still keeping the low-battery visual indicator.
    pub low_battery_silent: bool,
    /// Percentage threshold at or below which a device is considered low battery.
    pub low_battery_threshold: u8,
    /// Launch silently into the system tray on login.
    pub autostart: bool,
    /// Show the pin-on-top button in the main panel header.
    pub show_pin_button: bool,
    /// Total read time budget per device in milliseconds.
    pub read_timeout_ms: u64,
    /// UI appearance theme preference (`system`, `light`, or `dark`).
    pub theme_mode: ThemeMode,
    /// UI language preference (`system`, `zh_cn`, or `en`).
    pub language: LanguageMode,
}

impl Default for TrayConfig {
    fn default() -> Self {
        Self {
            poll_interval_secs: DEFAULT_POLL_INTERVAL_SECS,
            allow_stale: true,
            low_battery_notify: true,
            low_battery_silent: false,
            low_battery_threshold: DEFAULT_LOW_BATTERY_THRESHOLD,
            autostart: false,
            show_pin_button: false,
            read_timeout_ms: DEFAULT_READ_TIMEOUT_MS,
            theme_mode: ThemeMode::System,
            language: LanguageMode::System,
        }
    }
}

impl TrayConfig {
    /// Clamp user-supplied values into safe operational bounds.
    #[must_use]
    pub fn normalized(mut self) -> Self {
        if self.poll_interval_secs > 0 && self.poll_interval_secs < 15 {
            self.poll_interval_secs = 15;
        }
        self.low_battery_threshold = self.low_battery_threshold.clamp(5, 50);
        self.read_timeout_ms = self.read_timeout_ms.clamp(4_000, 15_000);
        self
    }

    /// Return the polling interval as a `Duration`, or `None` for manual-only mode.
    #[must_use]
    pub fn poll_interval(&self) -> Option<Duration> {
        (self.poll_interval_secs > 0).then(|| Duration::from_secs(self.poll_interval_secs))
    }

    /// Return the per-device read timeout as a `Duration`.
    #[must_use]
    pub fn read_timeout(&self) -> Duration {
        Duration::from_millis(self.read_timeout_ms.clamp(4_000, 15_000))
    }
}

/// Resolve the OS-native configuration file path without adding platform path crates.
#[must_use]
pub fn default_config_path() -> Option<PathBuf> {
    #[cfg(target_os = "windows")]
    {
        if let Some(portable_path) = std::env::current_exe()
            .ok()
            .and_then(|exe| exe.parent().map(PathBuf::from))
            .filter(|dir| dir.join("portable.txt").is_file() || dir.join("tray.json").is_file())
            .map(|dir| dir.join("tray.json"))
        {
            return Some(portable_path);
        }
        std::env::var_os("APPDATA")
            .map(PathBuf::from)
            .map(|dir| dir.join("wireless-battery").join("tray.json"))
    }
    #[cfg(target_os = "macos")]
    {
        std::env::var_os("HOME").map(PathBuf::from).map(|home| {
            home.join("Library")
                .join("Application Support")
                .join("wireless-battery")
                .join("tray.json")
        })
    }
    #[cfg(not(any(target_os = "windows", target_os = "macos")))]
    {
        std::env::var_os("XDG_CONFIG_HOME")
            .filter(|value| !value.is_empty())
            .map(PathBuf::from)
            .or_else(|| {
                std::env::var_os("HOME")
                    .map(PathBuf::from)
                    .map(|home| home.join(".config"))
            })
            .map(|dir| dir.join("wireless-battery").join("tray.json"))
    }
}

/// Load a config file, falling back to `TrayConfig::default()` if the file does not exist.
pub fn load_config(path: &Path) -> Result<TrayConfig, io::Error> {
    match fs::read_to_string(path) {
        Ok(raw) => serde_json::from_str::<TrayConfig>(&raw)
            .map(TrayConfig::normalized)
            .map_err(|error| io::Error::new(io::ErrorKind::InvalidData, error)),
        Err(error) if error.kind() == io::ErrorKind::NotFound => Ok(TrayConfig::default()),
        Err(error) => Err(error),
    }
}

/// Persist configuration atomically by writing a sibling temporary file before renaming.
pub fn save_config(path: &Path, config: &TrayConfig) -> Result<(), io::Error> {
    if let Some(parent) = path.parent() {
        fs::create_dir_all(parent)?;
    }
    let normalized = config.clone().normalized();
    let payload = serde_json::to_string_pretty(&normalized)
        .map_err(|error| io::Error::new(io::ErrorKind::InvalidData, error))?;
    let temp_path = path.with_extension("json.tmp");
    fs::write(&temp_path, payload)?;
    match fs::rename(&temp_path, path) {
        Ok(()) => Ok(()),
        Err(_) => {
            // Windows `rename` can fail if destination exists in some environments.
            let _ = fs::remove_file(path);
            fs::rename(&temp_path, path)
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn config_normalizes_bounds() {
        let cfg = TrayConfig {
            poll_interval_secs: 5,
            low_battery_threshold: 1,
            read_timeout_ms: 50,
            ..TrayConfig::default()
        }
        .normalized();

        assert_eq!(cfg.poll_interval_secs, 15);
        assert_eq!(cfg.low_battery_threshold, 5);
        assert_eq!(cfg.read_timeout_ms, 4_000);
    }

    #[test]
    fn manual_polling_interval_is_preserved() {
        let cfg = TrayConfig {
            poll_interval_secs: 0,
            ..TrayConfig::default()
        }
        .normalized();
        assert_eq!(cfg.poll_interval(), None);
    }

    #[test]
    fn save_and_load_round_trip() {
        let dir = std::env::temp_dir().join(format!("wb-tray-config-test-{}", std::process::id()));
        let path = dir.join("tray.json");
        let _ = fs::remove_dir_all(&dir);

        assert_eq!(load_config(&path).unwrap(), TrayConfig::default());

        let custom = TrayConfig {
            poll_interval_secs: 3600,
            allow_stale: false,
            low_battery_notify: true,
            low_battery_silent: true,
            low_battery_threshold: 20,
            autostart: true,
            show_pin_button: true,
            read_timeout_ms: 5_500,
            theme_mode: ThemeMode::Dark,
            language: LanguageMode::En,
        };
        save_config(&path, &custom).unwrap();
        assert_eq!(load_config(&path).unwrap(), custom);

        let _ = fs::remove_dir_all(&dir);
    }

    #[test]
    fn legacy_config_without_new_fields_uses_safe_defaults() {
        let legacy_json = r#"{
            "poll_interval_secs": 300,
            "allow_stale": true,
            "low_battery_notify": true,
            "low_battery_threshold": 15,
            "autostart": false,
            "read_timeout_ms": 5000
        }"#;
        let parsed: TrayConfig = serde_json::from_str(legacy_json).unwrap();
        assert_eq!(parsed.theme_mode, ThemeMode::System);
        assert_eq!(parsed.language, LanguageMode::System);
        assert!(!parsed.low_battery_silent);
        assert!(!parsed.show_pin_button);
        assert_eq!(ThemeMode::System.next(), ThemeMode::Light);
        assert_eq!(ThemeMode::Light.next(), ThemeMode::Dark);
        assert_eq!(ThemeMode::Dark.next(), ThemeMode::System);
    }
}
