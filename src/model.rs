//! Public data types and serialization contracts.

use crate::error::ErrorKind;
use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use std::time::Duration;

/// An opaque device identifier derived deterministically from OS endpoint identity.
///
/// It is intended for session-local use; persistence across reboots, machines, or
/// OS path reassignment is not guaranteed.
#[derive(Debug, Clone, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(transparent)]
pub struct DeviceId(pub String);

impl std::fmt::Display for DeviceId {
    fn fmt(&self, formatter: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        formatter.write_str(&self.0)
    }
}

/// An opaque endpoint identifier.
#[derive(Debug, Clone, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(transparent)]
pub struct EndpointId(pub String);

impl std::fmt::Display for EndpointId {
    fn fmt(&self, formatter: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        formatter.write_str(&self.0)
    }
}

/// The transport used by an endpoint.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum TransportKind {
    /// USB or operating-system HID.
    Hid,
    /// Bluetooth Low Energy GATT.
    BluetoothLe,
    /// Windows XInput gamepad slot.
    XInput,
    /// An in-memory test transport.
    Mock,
}

/// Physical bus reported by HID enumeration, when available.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum HidBusType {
    /// USB HID.
    Usb,
    /// Bluetooth HID.
    Bluetooth,
    /// I2C HID.
    I2c,
    /// SPI HID.
    Spi,
}

/// Static information obtained without opening an endpoint.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct Endpoint {
    /// Opaque session-local endpoint ID.
    pub id: EndpointId,
    /// Endpoint transport.
    pub transport: TransportKind,
    /// USB vendor ID, when applicable.
    pub vid: Option<u16>,
    /// USB product ID, when applicable.
    pub pid: Option<u16>,
    /// HID usage page.
    pub usage_page: Option<u16>,
    /// HID usage.
    pub usage: Option<u16>,
    /// USB/HID interface number, when supplied by the operating system.
    #[serde(default)]
    pub interface_number: Option<i32>,
    /// Physical bus type reported by HID enumeration, when known.
    #[serde(default)]
    pub bus_type: Option<HidBusType>,
    /// HID input report length as supplied by the operating system.
    pub input_report_length: Option<usize>,
    /// HID output report length as supplied by the operating system.
    pub output_report_length: Option<usize>,
    /// HID feature report length as supplied by the operating system.
    pub feature_report_length: Option<usize>,
    /// Product name supplied by the operating system.
    pub product_name: Option<String>,
    /// Manufacturer supplied by the operating system.
    pub manufacturer: Option<String>,
    #[serde(rename = "synthetic_path", skip_serializing, default)]
    pub(crate) system_path: String,
}

impl Endpoint {
    /// Construct a synthetic endpoint for tests and replay.
    #[must_use]
    pub fn synthetic(id: impl Into<String>, vid: u16, pid: u16) -> Self {
        let id = EndpointId(id.into());
        Self {
            system_path: id.0.clone(),
            id,
            transport: TransportKind::Mock,
            vid: Some(vid),
            pid: Some(pid),
            usage_page: None,
            usage: None,
            interface_number: None,
            bus_type: None,
            input_report_length: None,
            output_report_length: None,
            feature_report_length: None,
            product_name: Some("Synthetic device".to_owned()),
            manufacturer: None,
        }
    }
}

/// Coarse runtime capabilities. Evidence for individual states is tracked separately.
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq, Serialize, Deserialize)]
pub struct Capabilities {
    /// Exact percentage can be returned.
    pub percentage: bool,
    /// A discrete battery level can be returned.
    pub discrete_level: bool,
    /// Voltage in millivolts can be returned.
    pub voltage: bool,
    /// At least one charge state can be returned.
    pub charge_state: bool,
    /// The protocol explicitly reports reachability states.
    pub reachability: bool,
}

/// Coarse static match state for an enumerated endpoint.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum MatchStatus {
    /// One highest-priority provider matched statically.
    Exact,
    /// Multiple highest-priority providers matched the same endpoint.
    Conflict,
    /// No provider matched.
    Unmatched,
}

/// Provider lifecycle and evidence status. `Unknown` is used for unmatched endpoints.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum SupportStatus {
    /// Unknown because no provider is selected.
    Unknown,
    /// Provider with inferred/unverified protocol evidence.
    Experimental,
    /// Provider with real-hardware evidence.
    Verified,
}

/// Peripheral form factor reported from authoritative static provider metadata.
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum DeviceKind {
    Mouse,
    Keyboard,
    Headset,
    Gamepad,
    #[default]
    Peripheral,
}

/// A discovered endpoint and its selected provider, if any.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct Device {
    /// Session-local logical device ID.
    pub id: DeviceId,
    /// Endpoint used for reads.
    pub endpoint_id: EndpointId,
    /// Static endpoint information.
    pub endpoint: Endpoint,
    /// Display name, or an explicit unknown placeholder.
    pub name: String,
    /// Manufacturer, when supplied by the operating system.
    pub vendor: Option<String>,
    /// Product name, when supplied by the operating system.
    pub model: Option<String>,
    /// Selected provider ID.
    pub provider_id: Option<String>,
    /// Selected provider version.
    pub provider_version: Option<String>,
    /// Provider lifecycle status.
    pub support_status: SupportStatus,
    /// Static match result.
    pub match_status: MatchStatus,
    /// Coarse hardware category derived from provider metadata.
    #[serde(default)]
    pub device_kind: DeviceKind,
    /// Coarse provider capabilities.
    pub capabilities: Capabilities,
    /// Whether policy currently allows an explicit read.
    pub readable: bool,
}

/// Discrete battery level without fabricated percentage conversion.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum BatteryLevel {
    /// Critically low.
    Critical,
    /// Low.
    Low,
    /// Medium.
    Medium,
    /// High.
    High,
    /// Full discrete level.
    Full,
}

/// Charging state; unknown is distinct from not charging.
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum ChargeState {
    /// External power is charging the battery.
    Charging,
    /// External power is present but the battery is not charging.
    NotCharging,
    /// The protocol explicitly reports full.
    Full,
    /// The battery is discharging.
    Discharging,
    /// The state is not available or not understood.
    #[default]
    Unknown,
}

/// Device reachability reported or conservatively inferred by a successful exchange.
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum Reachability {
    /// A successful exchange or explicit online state.
    Online,
    /// The protocol explicitly reports sleep.
    Sleeping,
    /// The protocol explicitly reports offline.
    Offline,
    /// Reachability is unavailable.
    #[default]
    Unknown,
}

/// Precision of the values actually present in this reading.
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum Precision {
    /// An exact reported percentage is present.
    Percentage,
    /// Only a discrete level is present.
    DiscreteLevel,
    /// Only voltage is present.
    VoltageOnly,
    /// No measurement value is present.
    #[default]
    None,
}

/// Origin of a battery reading.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum ReadingSource {
    /// An operating-system battery property.
    Os,
    /// A standard HID battery usage.
    StandardHid,
    /// A vendor-specific HID protocol.
    VendorHid,
    /// The standard Bluetooth LE Battery Service.
    BluetoothBas,
    /// A deterministic test source.
    Mock,
}

/// Whether this reading came from a new observation or the process-local cache.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum Freshness {
    /// Newly observed from the device.
    Live,
    /// Last successful value returned after a refresh error.
    Stale,
}

/// Normalized battery observation.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct BatteryReading {
    /// Logical device ID.
    pub device_id: DeviceId,
    /// Endpoint used for the observation.
    pub endpoint_id: EndpointId,
    /// Reported percentage, without conversion from voltage or levels.
    pub percentage: Option<u8>,
    /// Reported discrete level.
    pub level: Option<BatteryLevel>,
    /// Reported voltage in millivolts.
    pub voltage_mv: Option<u32>,
    /// Charging state.
    pub charge_state: ChargeState,
    /// Reachability state.
    pub reachability: Reachability,
    /// Precision of the values actually present.
    pub precision: Precision,
    /// Protocol or operating-system source.
    pub source: ReadingSource,
    /// Original observation time, retained for stale values.
    pub observed_at: DateTime<Utc>,
    /// Live or stale cache result.
    pub freshness: Freshness,
}

/// Provider output before core normalization and timestamping.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ProviderReading {
    /// Reported percentage.
    pub percentage: Option<u8>,
    /// Reported discrete level.
    pub level: Option<BatteryLevel>,
    /// Reported voltage in millivolts.
    pub voltage_mv: Option<u32>,
    /// Reported charge state.
    pub charge_state: ChargeState,
    /// Reported reachability; `Unknown` becomes `Online` after a successful exchange.
    pub reachability: Reachability,
    /// Reading origin.
    pub source: ReadingSource,
}

impl Default for ProviderReading {
    fn default() -> Self {
        Self {
            percentage: None,
            level: None,
            voltage_mv: None,
            charge_state: ChargeState::Unknown,
            reachability: Reachability::Unknown,
            source: ReadingSource::VendorHid,
        }
    }
}

/// Client-wide policy and default deadline.
#[derive(Debug, Clone)]
pub struct ClientConfig {
    /// Default total read budget.
    pub default_timeout: Duration,
    /// Whether the last successful value is cached.
    pub enable_cache: bool,
}

impl Default for ClientConfig {
    fn default() -> Self {
        Self {
            default_timeout: Duration::from_millis(1_000),
            enable_cache: true,
        }
    }
}

/// Per-read policy.
#[derive(Debug, Clone)]
pub struct ReadOptions {
    /// Total call budget, including locking, cooldown and all protocol steps.
    pub timeout: Duration,
    /// Whether a failed refresh may return the last successful reading.
    pub allow_stale: bool,
}

impl Default for ReadOptions {
    fn default() -> Self {
        Self {
            timeout: Duration::from_millis(1_000),
            allow_stale: false,
        }
    }
}

/// Passive diagnostic information for a device.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct DeviceDiagnostic {
    /// Session-local device ID.
    pub device_id: DeviceId,
    /// Selected provider ID.
    pub provider_id: Option<String>,
    /// Selected provider version.
    pub provider_version: Option<String>,
    /// Package lifecycle status.
    pub support_status: SupportStatus,
    /// Static match state.
    pub match_status: MatchStatus,
    /// IDs of exact and candidate providers considered during matching.
    pub matches: Vec<String>,
    /// Coarse capabilities.
    pub capabilities: Capabilities,
    /// Endpoint transport.
    pub transport: TransportKind,
    /// Operating system running the library.
    pub platform: String,
    /// Most recent read error, if any.
    pub last_error: Option<ErrorKind>,
    /// Original observation time of the cached reading, if any.
    pub cached_observed_at: Option<DateTime<Utc>>,
    /// Platform-specific passive permission hint.
    pub permission_hint: Option<String>,
}
