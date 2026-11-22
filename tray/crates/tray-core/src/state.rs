//! View models serialized for native OS shells (Windows, macOS, Linux).

use crate::config::TrayConfig;
use crate::device_kind::DeviceKind;
use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use wireless_battery::{
    BatteryLevel, ChargeState, ErrorKind, Freshness, Precision, Reachability, SupportStatus,
    TransportKind,
};

/// Visual state for the minimalist pure-logo system tray icon (no numbers).
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum TrayIconState {
    /// No readable peripherals currently visible (rendered at reduced opacity).
    Idle,
    /// Devices are healthy (monochrome adaptive template logo).
    Normal,
    /// At least one device is low battery and low-battery alerts are enabled (logo + red corner dot).
    LowBattery,
}

/// One peripheral row inside the tray popover.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct DeviceView {
    /// Session-local opaque device ID (never a serial number or raw system path).
    pub id: String,
    /// Clean display name.
    pub name: String,
    /// Hardware silhouette icon category.
    pub kind: DeviceKind,
    /// Underlying transport type.
    pub transport: TransportKind,
    /// Concise transport label for the UI (`HID`, `BLE BAS`, `XInput`).
    pub transport_label: String,
    /// Selected provider ID (shown inside the expanded drawer).
    pub provider_id: Option<String>,
    /// Evidence lifecycle status (shown inside the expanded drawer).
    pub support_status: SupportStatus,
    /// Honest precision indicator matching `wireless-battery`.
    pub precision: Precision,
    /// Exact percentage when reported by the device.
    pub percentage: Option<u8>,
    /// Discrete battery level without fabricated percentage conversion.
    pub level: Option<BatteryLevel>,
    /// Reported voltage in millivolts, if available.
    pub voltage_mv: Option<u32>,
    /// Charging state.
    pub charge_state: ChargeState,
    /// Reachability state.
    pub reachability: Reachability,
    /// Live or cached stale reading indicator.
    pub freshness: Option<Freshness>,
    /// Original observation timestamp (preserved for stale cache entries).
    pub observed_at: Option<DateTime<Utc>>,
    /// Most recent sanitized read error, if the latest poll failed.
    pub last_error: Option<ErrorKind>,
    /// True when this device is currently at or below the configured low-battery threshold.
    pub is_low: bool,
}

/// Complete immutable snapshot pushed to native OS tray shells.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct TraySnapshot {
    /// Application display version (e.g. `v0.1.0` or `v0.1.0-dev`).
    #[serde(default = "default_app_version")]
    pub app_version: String,
    /// Visible supported devices sorted by priority (low -> active -> stale/sleeping -> offline).
    pub devices: Vec<DeviceView>,
    /// Visual state for the pure-logo tray icon.
    pub icon_state: TrayIconState,
    /// Multi-line plain-text summary for native OS tray hover tooltips.
    pub tooltip: String,
    /// Timestamp of the last completed sync cycle.
    pub last_sync_at: Option<DateTime<Utc>>,
    /// Whether an active refresh pass is currently running.
    pub refreshing: bool,
    /// Current persisted user settings.
    pub config: TrayConfig,
}

fn default_app_version() -> String {
    crate::version::APP_DISPLAY_VERSION.to_owned()
}

/// Deduplicated low-battery alert emitted by `TrayService` for OS native notifications.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct LowBatteryAlert {
    pub device_id: String,
    pub device_name: String,
    pub percentage: u8,
}

/// Format a concise multi-line tooltip for OS hover preview in English.
#[must_use]
pub fn format_tooltip(devices: &[DeviceView]) -> String {
    format_tooltip_with_locale(devices, false)
}

/// Format a concise multi-line tooltip for OS hover preview using localized UI strings.
#[must_use]
pub fn format_tooltip_with_locale(devices: &[DeviceView], zh: bool) -> String {
    if devices.is_empty() {
        return if zh {
            "Wireless Battery\n未发现支持的外设".to_owned()
        } else {
            "Wireless Battery\nNo supported peripherals".to_owned()
        };
    }

    let mut lines = Vec::with_capacity(devices.len().min(6) + 1);
    lines.push("Wireless Battery".to_owned());
    for device in devices.iter().take(5) {
        let value = match device.precision {
            Precision::Percentage => match device.percentage {
                Some(pct) => {
                    if device.charge_state == ChargeState::Charging {
                        format!("{pct}% ⚡")
                    } else {
                        format!("{pct}%")
                    }
                }
                None => "--".to_owned(),
            },
            Precision::DiscreteLevel => match (zh, device.level) {
                (true, Some(BatteryLevel::Critical)) => "极低".to_owned(),
                (true, Some(BatteryLevel::Low)) => "低".to_owned(),
                (true, Some(BatteryLevel::Medium)) => "中".to_owned(),
                (true, Some(BatteryLevel::High)) => "高".to_owned(),
                (true, Some(BatteryLevel::Full)) => "满".to_owned(),
                (false, Some(BatteryLevel::Critical)) => "Critical".to_owned(),
                (false, Some(BatteryLevel::Low)) => "Low".to_owned(),
                (false, Some(BatteryLevel::Medium)) => "Medium".to_owned(),
                (false, Some(BatteryLevel::High)) => "High".to_owned(),
                (false, Some(BatteryLevel::Full)) => "Full".to_owned(),
                (_, None) => "--".to_owned(),
            },
            Precision::VoltageOnly => match device.voltage_mv {
                Some(mv) => format!("{:.2} V", f64::from(mv) / 1000.0),
                None => "--".to_owned(),
            },
            Precision::None => "--".to_owned(),
        };
        lines.push(format!("{}  {}", device.name, value));
    }
    if devices.len() > 5 {
        let rem = devices.len() - 5;
        if zh {
            lines.push(format!("+{rem} 台更多"));
        } else {
            lines.push(format!("+{rem} more"));
        }
    }
    lines.join("\n")
}

/// Format an elapsed second count relative to now using localized strings.
#[must_use]
pub fn format_relative_time_secs(elapsed_secs: Option<i64>, zh: bool) -> String {
    let Some(raw_secs) = elapsed_secs else {
        return if zh {
            "未同步".to_owned()
        } else {
            "Not synced".to_owned()
        };
    };
    let elapsed = raw_secs.max(0);
    if elapsed < 60 {
        if zh {
            "刚刚".to_owned()
        } else {
            "Just now".to_owned()
        }
    } else if elapsed < 3600 {
        let mins = elapsed / 60;
        if zh {
            format!("{mins} 分钟前")
        } else {
            format!("{mins}m ago")
        }
    } else {
        let hrs = elapsed / 3600;
        if zh {
            format!("{hrs} 小时前")
        } else {
            format!("{hrs}h ago")
        }
    }
}

/// Format an optional UTC observation timestamp relative to `Utc::now()`.
#[must_use]
pub fn format_relative_time(at: Option<DateTime<Utc>>, zh: bool) -> String {
    let elapsed = at.map(|ts| Utc::now().signed_duration_since(ts).num_seconds());
    format_relative_time_secs(elapsed, zh)
}

/// Format an optional RFC3339/ISO8601 timestamp string relative to `Utc::now()`.
#[must_use]
pub fn format_relative_time_rfc3339(raw: Option<&str>, zh: bool) -> String {
    let parsed = raw.and_then(|s| {
        DateTime::parse_from_rfc3339(s)
            .ok()
            .map(|dt| dt.with_timezone(&Utc))
    });
    format_relative_time(parsed, zh)
}

/// Format a polling interval in seconds for Settings UI dropdown labels (supporting custom values).
#[must_use]
pub fn format_poll_label(secs: u64, zh: bool) -> String {
    if secs == 0 {
        return if zh {
            "仅手动".to_owned()
        } else {
            "Manual".to_owned()
        };
    }
    if secs % 3600 == 0 {
        let hrs = secs / 3600;
        return if zh {
            format!("{hrs} 小时")
        } else if hrs == 1 {
            "1 hour".to_owned()
        } else {
            format!("{hrs} hours")
        };
    }
    if secs % 60 == 0 {
        let mins = secs / 60;
        return if zh {
            format!("{mins} 分钟")
        } else {
            format!("{mins} min")
        };
    }
    if zh {
        format!("{secs} 秒")
    } else {
        format!("{secs}s")
    }
}

/// Format the popover footer summary (`version · sync_time · poll_cadence`).
#[must_use]
pub fn format_footer_summary(
    app_version: &str,
    last_sync_at: Option<DateTime<Utc>>,
    poll_secs: u64,
    zh: bool,
) -> String {
    let sync = format_relative_time(last_sync_at, zh);
    let poll = if poll_secs == 0 {
        if zh {
            "仅手动".to_owned()
        } else {
            "Manual".to_owned()
        }
    } else if poll_secs % 3600 == 0 {
        let hrs = poll_secs / 3600;
        if zh {
            format!("每 {hrs} 小时")
        } else {
            format!("Every {hrs} hr")
        }
    } else if poll_secs % 60 == 0 {
        let mins = poll_secs / 60;
        if zh {
            format!("每 {mins} 分钟")
        } else {
            format!("Every {mins} min")
        }
    } else if zh {
        format!("每 {poll_secs} 秒")
    } else {
        format!("Every {poll_secs}s")
    };
    format!("{app_version} · {sync} · {poll}")
}

/// RFC3339 string variant of `format_footer_summary` for FFI callers.
#[must_use]
pub fn format_footer_summary_rfc3339(
    app_version: &str,
    raw_last_sync: Option<&str>,
    poll_secs: u64,
    zh: bool,
) -> String {
    let parsed = raw_last_sync.and_then(|s| {
        DateTime::parse_from_rfc3339(s)
            .ok()
            .map(|dt| dt.with_timezone(&Utc))
    });
    format_footer_summary(app_version, parsed, poll_secs, zh)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn formats_relative_time_without_zero_minutes() {
        assert_eq!(format_relative_time_secs(None, false), "Not synced");
        assert_eq!(format_relative_time_secs(None, true), "未同步");
        assert_eq!(format_relative_time_secs(Some(45), false), "Just now");
        assert_eq!(format_relative_time_secs(Some(59), true), "刚刚");
        assert_eq!(format_relative_time_secs(Some(60), false), "1m ago");
        assert_eq!(format_relative_time_secs(Some(120), true), "2 分钟前");
        assert_eq!(format_relative_time_secs(Some(3600), false), "1h ago");
    }

    #[test]
    fn formats_custom_poll_intervals() {
        assert_eq!(format_poll_label(0, false), "Manual");
        assert_eq!(format_poll_label(300, true), "5 分钟");
        assert_eq!(format_poll_label(120, false), "2 min");
        assert_eq!(format_poll_label(45, true), "45 秒");
        assert_eq!(format_poll_label(7200, false), "2 hours");
    }

    #[test]
    fn formats_localized_tooltip() {
        assert_eq!(
            format_tooltip_with_locale(&[], true),
            "Wireless Battery\n未发现支持的外设"
        );
        assert_eq!(
            format_tooltip_with_locale(&[], false),
            "Wireless Battery\nNo supported peripherals"
        );
    }
}
