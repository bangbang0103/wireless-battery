//! HID transport abstraction, real HIDAPI access, recording and replay.

mod hidapi;
pub mod record;
pub mod replay;
#[cfg(windows)]
mod windows;
pub mod xinput;

#[cfg(feature = "bluetooth")]
pub mod bluetooth;

pub use hidapi::HidApiTransport;

use crate::error::BatteryError;
use crate::model::{Endpoint, EndpointId};
use serde::{Deserialize, Serialize};
use std::time::Duration;

/// A normalized HID report: report ID is separate from payload bytes.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct HidReport {
    /// HID report ID.
    pub report_id: u8,
    /// Payload bytes, excluding the report ID.
    pub data: Vec<u8>,
}

/// Raw XInput battery values. Levels are discrete, not percentages.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct XInputReport {
    /// XInput battery type: 1 is wired, 2 alkaline, 3 NiMH.
    pub battery_type: u8,
    /// XInput level: 0 empty through 3 full.
    pub battery_level: u8,
}

/// A serial HID session used identically by real, record and replay transports.
pub trait HidSession: Send {
    /// Send a feature report with a separately supplied report ID.
    fn send_feature(&mut self, report_id: u8, report: &[u8]) -> Result<(), TransportError>;
    /// Get a feature report and return payload bytes excluding its report ID.
    fn get_feature(&mut self, report_id: u8, length: usize) -> Result<Vec<u8>, TransportError>;
    /// Write an output report over the operating system's interrupt/output path.
    fn write_output(&mut self, report_id: u8, report: &[u8]) -> Result<(), TransportError>;
    /// Send HID `SET_REPORT` for an output report, where the platform supports it.
    fn set_output(&mut self, report_id: u8, report: &[u8]) -> Result<(), TransportError>;
    /// Read one input report, without skipping unrelated reports.
    fn read_input(&mut self, timeout: Duration) -> Result<HidReport, TransportError>;
    /// Read a specific input report over the control pipe (`HID GET_REPORT` Input),
    /// returning payload bytes excluding its report ID.
    fn get_input_report(
        &mut self,
        _report_id: u8,
        _length: usize,
    ) -> Result<Vec<u8>, TransportError> {
        Err(TransportError::Unsupported(
            "GET_INPUT_REPORT is not supported by this transport".to_owned(),
        ))
    }
}

/// Endpoint enumeration and session creation.
pub trait Transport: Send + Sync {
    /// Enumerate static endpoint information without device writes.
    fn enumerate(&self) -> Result<Vec<Endpoint>, TransportError>;

    /// Open an already enumerated HID endpoint.
    fn open(&self, endpoint: &EndpointId) -> Result<Box<dyn HidSession>, TransportError>;

    /// Read the standard Bluetooth LE Battery Service, when implemented.
    fn read_bas(
        &self,
        _endpoint: &EndpointId,
        _timeout: Duration,
    ) -> Result<Option<u8>, TransportError> {
        Ok(None)
    }

    /// Read one connected XInput slot, when implemented.
    fn read_xinput(&self, _endpoint: &EndpointId) -> Result<Option<XInputReport>, TransportError> {
        Ok(None)
    }
}

/// Sanitized transport failures.
#[derive(Debug, Clone, thiserror::Error)]
pub enum TransportError {
    /// The operating system denied endpoint access.
    #[error("permission denied")]
    PermissionDenied,
    /// The operation timed out.
    #[error("operation timed out")]
    Timeout,
    /// The endpoint is busy.
    #[error("device busy")]
    DeviceBusy,
    /// The endpoint disconnected.
    #[error("device disconnected")]
    DeviceDisconnected,
    /// The requested standard service is unavailable.
    #[error("unsupported: {0}")]
    Unsupported(String),
    /// A transport response was structurally invalid.
    #[error("invalid transport response: {0}")]
    InvalidResponse(String),
    /// Another transport failure occurred.
    #[error("{0}")]
    Message(String),
}

impl TransportError {
    /// Convert this failure into the public error model without exposing system paths.
    pub fn into_battery(self, endpoint: &EndpointId) -> BatteryError {
        match self {
            Self::PermissionDenied => BatteryError::PermissionDenied {
                endpoint: endpoint.to_string(),
            },
            Self::Timeout => BatteryError::timeout("device I/O"),
            Self::DeviceBusy => BatteryError::DeviceBusy {
                endpoint: endpoint.to_string(),
            },
            Self::DeviceDisconnected => BatteryError::DeviceDisconnected {
                endpoint: endpoint.to_string(),
            },
            Self::Unsupported(reason) => BatteryError::Unsupported { reason },
            Self::InvalidResponse(reason) => BatteryError::InvalidResponse { reason },
            Self::Message(message) => BatteryError::Transport { message },
        }
    }
}

pub(crate) fn classify_hid_error(message: String) -> TransportError {
    let lowercase = message.to_ascii_lowercase();
    if lowercase.contains("permission")
        || lowercase.contains("access is denied")
        || lowercase.contains("not permitted")
    {
        TransportError::PermissionDenied
    } else if lowercase.contains("timeout") || lowercase.contains("timed out") {
        TransportError::Timeout
    } else if lowercase.contains("busy") || lowercase.contains("resource temporarily unavailable") {
        TransportError::DeviceBusy
    } else if lowercase.contains("disconnect")
        || lowercase.contains("no such device")
        || lowercase.contains("device not found")
        || lowercase.contains("not connected")
    {
        TransportError::DeviceDisconnected
    } else {
        TransportError::Message(message)
    }
}
