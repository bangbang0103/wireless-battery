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

/// User-facing tray settings aligned with `wireless-battery` safety policies.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(default)]
pub struct TrayConfig {
    /// Background polling interval in seconds. `0` disables automatic polling (manual only).
    pub poll_interval_secs: u64,
    /// Opt-in flag matching `ClientConfig::allow_experimental`.
    pub allow_experimental: bool,
    /// Return the last successful reading marked as `Stale` when a refresh fails.
    pub allow_stale: bool,
    /// Enable low-battery OS notifications and subtle low-battery dot on the tray logo.
    pub low_battery_notify: bool,
    /// Percentage threshold at or below which a device is considered low battery.
    pub low_battery_threshold: u8,
    /// Launch silently into the system tray on login.
    pub autostart: bool,
    /// Total read time budget per device in milliseconds.
    pub read_timeout_ms: u64,
}

impl Default for TrayConfig {
    fn default() -> Self {
        Self {
            poll_interval_secs: DEFAULT_POLL_INTERVAL_SECS,
            allow_experimental: true,
            allow_stale: true,
            low_battery_notify: true,
            low_battery_threshold: DEFAULT_LOW_BATTERY_THRESHOLD,
            autostart: false,
            read_timeout_ms: DEFAULT_READ_TIMEOUT_MS,
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
            poll_interval_secs: 60,
            allow_experimental: true,
            allow_stale: false,
            low_battery_notify: true,
            low_battery_threshold: 20,
            autostart: true,
            read_timeout_ms: 5_500,
        };
        save_config(&path, &custom).unwrap();
        assert_eq!(load_config(&path).unwrap(), custom);

        let _ = fs::remove_dir_all(&dir);
    }
}
