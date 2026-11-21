//! Built-in, OS-standard, and declarative protocol providers.

pub(crate) mod fixed;
pub(crate) mod os;
pub(crate) mod vendors;

use crate::error::BatteryError;
use crate::model::{Capabilities, Device, Endpoint, ProviderReading, SupportStatus};
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
    /// Package or provider version.
    pub version: String,
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
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum MatchDecision {
    /// The provider does not recognize this endpoint.
    NoMatch,
    /// The provider recognizes the endpoint without probing.
    Exact,
    /// A possible match that is diagnostic only.
    Candidate {
        /// Sanitized reason for the candidate.
        reason: String,
    },
}

/// A built-in, OS-standard, or declarative battery protocol.
pub trait Provider: Send + Sync {
    /// Stable provider or package ID.
    fn id(&self) -> &str;

    /// Static metadata and query policy.
    fn metadata(&self) -> ProviderMetadata;

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
