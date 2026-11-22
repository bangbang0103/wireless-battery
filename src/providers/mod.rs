//! Built-in, OS-standard, and declarative protocol providers.

pub(crate) mod fixed;
pub(crate) mod os;
pub(crate) mod vendors;

use crate::error::BatteryError;
use crate::model::{Capabilities, Device, DeviceKind, Endpoint, ProviderReading, SupportStatus};
use crate::transport::{HidSession, Transport};
use std::time::{Duration, Instant};

/// A total deadline shared by every step in a read.
#[derive(Debug, Clone, Copy)]
pub struct Deadline(Instant);

impl Deadline {
    /// Construct a deadline from a total timeout.
    #[must_use]
    pub fn after(timeout: Duration) -> Self {
        Self(Instant::now() + timeout)
    }

    /// Return the remaining budget or a timeout error.
    pub fn remaining(self, operation: &str) -> Result<Duration, BatteryError> {
        BatteryError::remaining(self.0, operation)
    }

    /// Check the deadline before performing a protocol step.
    pub fn check(self, operation: &str) -> Result<(), BatteryError> {
        self.remaining(operation).map(|_| ())
    }
}

/// Static provider metadata.
#[derive(Debug, Clone)]
pub struct ProviderMetadata {
    /// Provider version.
    pub version: String,
    /// Coarse device category shown in shells without name-guessing tables.
    pub device_kind: DeviceKind,
    /// Lifecycle and evidence gate.
    pub support_status: SupportStatus,
    /// Coarse runtime capabilities.
    pub capabilities: Capabilities,
    /// Default timeout for one device I/O operation.
    pub io_timeout: Duration,
    /// Minimum interval between queries on the same endpoint.
    pub cooldown: Duration,
}

/// Context for an explicitly requested provider read.
#[derive(Debug, Clone)]
pub struct DeviceContext<'a> {
    /// Matched logical device.
    pub device: &'a Device,
    /// Matched endpoint.
    pub endpoint: &'a Endpoint,
}

/// Static match outcome. Matching must never perform I/O.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum MatchDecision {
    /// The provider does not recognize this endpoint.
    NoMatch,
    /// Broad chipset/VID family rule; yields automatically to any `Exact` match.
    Family,
    /// Model-specific or uniquely scoped match without probing.
    Exact,
}

/// Infer a form factor from an endpoint product string when one provider covers
/// multi-category receivers or generic OS standards such as Bluetooth BAS.
#[must_use]
pub(crate) fn infer_device_kind_from_name(name: Option<&str>) -> Option<DeviceKind> {
    let lower = name?.to_ascii_lowercase();
    if lower.contains("headset")
        || lower.contains("headphone")
        || lower.contains("delta")
        || lower.contains("pelta")
        || lower.contains("cetra")
        || lower.contains("blackshark")
        || lower.contains("barracuda")
        || lower.contains("arctis")
        || lower.contains("virtuoso")
        || lower.contains("hs80")
        || lower.contains("cloud")
        || lower.contains("earbud")
        || lower.contains("inzone")
    {
        return Some(DeviceKind::Headset);
    }
    if lower.contains("keyboard")
        || lower.contains("rainy75")
        || lower.contains("azoth")
        || lower.contains("falchion")
        || lower.contains("mx keys")
        || lower.contains("g915")
        || lower.contains("g715")
        || lower.contains("scope")
    {
        return Some(DeviceKind::Keyboard);
    }
    if lower.contains("controller")
        || lower.contains("dualsense")
        || lower.contains("dualshock")
        || lower.contains("gamepad")
        || lower.contains("xbox")
    {
        return Some(DeviceKind::Gamepad);
    }
    if lower.contains("mouse")
        || lower.contains("gpw")
        || lower.contains("superlight")
        || lower.contains("viper")
        || lower.contains("deathadder")
        || lower.contains("basilisk")
        || lower.contains("harpe")
        || lower.contains("gladius")
        || lower.contains("keris")
        || lower.contains("mx master")
        || lower.contains("mx anywhere")
        || lower.contains("g304")
        || lower.contains("g502")
    {
        return Some(DeviceKind::Mouse);
    }
    None
}

/// A built-in or OS-standard battery protocol.
pub trait Provider: Send + Sync {
    /// Stable provider ID.
    fn id(&self) -> &str;

    /// Static metadata and query policy.
    fn metadata(&self) -> ProviderMetadata;

    /// Refine form factor classification using static endpoint metadata.
    fn device_kind(&self, endpoint: &Endpoint) -> DeviceKind {
        let _ = endpoint;
        self.metadata().device_kind
    }

    /// Return a provider-specific display name using only static endpoint data.
    fn display_name(&self, endpoint: &Endpoint) -> Option<String> {
        let _ = endpoint;
        None
    }

    /// Match only with operating-system enumeration information.
    fn matches(&self, endpoint: &Endpoint) -> MatchDecision;

    /// Execute a bounded HID protocol exchange after explicit user authorization.
    ///
    /// HID providers implement this method. Non-HID providers (such as BLE BAS or
    /// XInput) override [`Self::execute`] directly instead.
    fn read(
        &self,
        device: &DeviceContext<'_>,
        session: &mut dyn HidSession,
        deadline: Deadline,
    ) -> Result<ProviderReading, BatteryError> {
        let _ = (device, session, deadline);
        Err(BatteryError::Unsupported {
            reason: format!("provider {} does not use a HID session", self.id()),
        })
    }

    /// Execute a bounded read against the transport.
    ///
    /// By default this checks the deadline, opens a HID session, and delegates to
    /// [`Self::read`]. Non-HID providers override this method.
    fn execute(
        &self,
        device: &DeviceContext<'_>,
        transport: &dyn Transport,
        deadline: Deadline,
    ) -> Result<ProviderReading, BatteryError> {
        deadline.check("open HID endpoint")?;
        let mut session = transport
            .open(&device.endpoint.id)
            .map_err(|error| error.into_battery(&device.endpoint.id))?;
        self.read(device, session.as_mut(), deadline)
    }
}
