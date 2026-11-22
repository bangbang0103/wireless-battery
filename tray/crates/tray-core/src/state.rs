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
    /// Number of matched experimental devices hidden because `allow_experimental` is off.
    pub hidden_experimental_count: usize,
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

/// Format a concise multi-line tooltip for OS hover preview.
#[must_use]
pub fn format_tooltip(devices: &[DeviceView]) -> String {
    if devices.is_empty() {
        return "Wireless Battery\nNo supported peripherals".to_owned();
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
            Precision::DiscreteLevel => match device.level {
                Some(BatteryLevel::Critical) => "Critical".to_owned(),
                Some(BatteryLevel::Low) => "Low".to_owned(),
                Some(BatteryLevel::Medium) => "Medium".to_owned(),
                Some(BatteryLevel::High) => "High".to_owned(),
                Some(BatteryLevel::Full) => "Full".to_owned(),
                None => "--".to_owned(),
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
        lines.push(format!("+{} more", devices.len() - 5));
    }
    lines.join("\n")
}
