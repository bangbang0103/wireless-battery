//! Cross-platform peripheral battery readings with conservative state semantics.
//!
//! Discovery is passive. Device I/O happens only through explicit battery reads
//! or captures. Synthetic packages are available to the replay CLI, but are not
//! registered as real device support.

mod client;
pub mod codec;
mod error;
mod identity;
pub mod model;
pub mod providers;
pub mod registry;
pub mod transport;

pub use client::BatteryClient;
pub use error::{BatteryError, ErrorKind};
pub use model::{
    BatteryLevel, BatteryReading, Capabilities, ChargeState, ClientConfig, Device,
    DeviceDiagnostic, DeviceId, Endpoint, EndpointId, Freshness, HidBusType, MatchStatus,
    Precision, ProviderReading, Reachability, ReadOptions, ReadingSource, SupportStatus,
    TransportKind,
};
pub use providers::{Deadline, DeviceContext, MatchDecision, Provider, ProviderMetadata};
pub use transport::{HidReport, HidSession, Transport, TransportError};
