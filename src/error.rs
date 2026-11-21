use serde::{Deserialize, Serialize};
use std::time::Duration;
use thiserror::Error;

/// Stable, machine-readable error categories.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum ErrorKind {
    /// The session-local device identifier is unknown.
    NotFound,
    /// No enabled provider can read the device.
    Unsupported,
    /// The operating system denied access.
    PermissionDenied,
    /// An operation exceeded its deadline.
    Timeout,
    /// Another session owns the endpoint.
    DeviceBusy,
    /// The device disappeared or its connection closed.
    DeviceDisconnected,
    /// A response violated the protocol.
    InvalidResponse,
    /// A response checksum did not match.
    ChecksumFailed,
    /// A device package failed validation.
    InvalidPackage,
    /// A replay fixture failed validation or did not match.
    InvalidFixture,
    /// An underlying transport failed.
    Transport,
    /// An unexpected internal failure occurred.
    Internal,
}

/// Public library error. Display text is diagnostic, not a stable contract.
#[derive(Debug, Error)]
pub enum BatteryError {
    /// The device ID is not known to this client.
    #[error("device not found: {device_id}")]
    NotFound {
        /// The absent session-local identifier.
        device_id: String,
    },
    /// The device cannot be read under the current policy.
    #[error("unsupported device: {reason}")]
    Unsupported {
        /// A sanitized explanation.
        reason: String,
    },
    /// Endpoint access was denied.
    #[error("permission denied for endpoint {endpoint}")]
    PermissionDenied {
        /// The opaque endpoint identifier.
        endpoint: String,
    },
    /// The operation did not finish within its deadline.
    #[error("operation timed out: {operation}")]
    Timeout {
        /// The operation that expired.
        operation: String,
    },
    /// The endpoint is owned by another process.
    #[error("device busy: {endpoint}")]
    DeviceBusy {
        /// The opaque endpoint identifier.
        endpoint: String,
    },
    /// The endpoint disconnected.
    #[error("device disconnected: {endpoint}")]
    DeviceDisconnected {
        /// The opaque endpoint identifier.
        endpoint: String,
    },
    /// The device returned an invalid response.
    #[error("invalid device response: {reason}")]
    InvalidResponse {
        /// A sanitized explanation.
        reason: String,
    },
    /// The device returned a checksum mismatch.
    #[error("response checksum failed")]
    ChecksumFailed,
    /// A package is malformed or semantically unsafe.
    #[error("invalid package {package}: {reason}")]
    InvalidPackage {
        /// The package identifier or path.
        package: String,
        /// The validation failure.
        reason: String,
    },
    /// A replay fixture is malformed or did not match.
    #[error("invalid fixture {fixture}: {reason}")]
    InvalidFixture {
        /// The fixture case or path.
        fixture: String,
        /// The validation failure.
        reason: String,
    },
    /// An underlying transport operation failed.
    #[error("transport error: {message}")]
    Transport {
        /// Sanitized transport detail.
        message: String,
    },
    /// An unexpected internal failure occurred.
    #[error("internal error: {message}")]
    Internal {
        /// Sanitized internal detail.
        message: String,
    },
}

impl BatteryError {
    /// Return the stable CLI and programmatic error category.
    #[must_use]
    pub const fn kind(&self) -> ErrorKind {
        match self {
            Self::NotFound { .. } => ErrorKind::NotFound,
            Self::Unsupported { .. } => ErrorKind::Unsupported,
            Self::PermissionDenied { .. } => ErrorKind::PermissionDenied,
            Self::Timeout { .. } => ErrorKind::Timeout,
            Self::DeviceBusy { .. } => ErrorKind::DeviceBusy,
            Self::DeviceDisconnected { .. } => ErrorKind::DeviceDisconnected,
            Self::InvalidResponse { .. } => ErrorKind::InvalidResponse,
            Self::ChecksumFailed => ErrorKind::ChecksumFailed,
            Self::InvalidPackage { .. } => ErrorKind::InvalidPackage,
            Self::InvalidFixture { .. } => ErrorKind::InvalidFixture,
            Self::Transport { .. } => ErrorKind::Transport,
            Self::Internal { .. } => ErrorKind::Internal,
        }
    }

    /// Return the documented CLI process exit code.
    #[must_use]
    pub const fn exit_code(&self) -> i32 {
        match self.kind() {
            ErrorKind::NotFound | ErrorKind::Unsupported => 3,
            ErrorKind::PermissionDenied => 4,
            ErrorKind::Timeout
            | ErrorKind::DeviceBusy
            | ErrorKind::DeviceDisconnected
            | ErrorKind::Transport => 5,
            ErrorKind::InvalidResponse
            | ErrorKind::ChecksumFailed
            | ErrorKind::InvalidPackage
            | ErrorKind::InvalidFixture => 6,
            ErrorKind::Internal => 70,
        }
    }

    pub(crate) fn timeout(operation: impl Into<String>) -> Self {
        Self::Timeout {
            operation: operation.into(),
        }
    }

    pub(crate) fn invalid_response(reason: impl Into<String>) -> Self {
        Self::InvalidResponse {
            reason: reason.into(),
        }
    }

    pub(crate) fn invalid_package(package: impl Into<String>, reason: impl Into<String>) -> Self {
        Self::InvalidPackage {
            package: package.into(),
            reason: reason.into(),
        }
    }

    pub(crate) fn remaining(
        deadline: std::time::Instant,
        operation: &str,
    ) -> Result<Duration, Self> {
        deadline
            .checked_duration_since(std::time::Instant::now())
            .filter(|duration| !duration.is_zero())
            .ok_or_else(|| Self::timeout(operation.to_owned()))
    }
}
