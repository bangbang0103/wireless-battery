//! Minimal recording of registered provider sessions.

use super::replay::{Fixture, FixtureStep, Operation, ReplayError, StepOutcome};
use super::{HidReport, HidSession, Transport, TransportError};
use crate::model::{Endpoint, EndpointId};
use std::sync::{Arc, Mutex};
use std::time::Duration;

/// A transport wrapper that records only the explicitly opened provider session.
pub struct RecordTransport {
    inner: Arc<dyn Transport>,
    steps: Arc<Mutex<Vec<FixtureStep>>>,
}

impl RecordTransport {
    /// Wrap a real or mock transport.
    #[must_use]
    pub fn new(inner: Arc<dyn Transport>) -> Self {
        Self {
            inner,
            steps: Arc::new(Mutex::new(Vec::new())),
        }
    }

    /// Return a copy of the currently recorded calls.
    pub fn steps(&self) -> Result<Vec<FixtureStep>, String> {
        self.steps
            .lock()
            .map(|steps| steps.clone())
            .map_err(|_| "record lock poisoned".to_owned())
    }

    fn record(
        steps: &Arc<Mutex<Vec<FixtureStep>>>,
        step: FixtureStep,
    ) -> Result<(), TransportError> {
        steps
            .lock()
            .map_err(|_| TransportError::Message("record lock poisoned".to_owned()))?
            .push(step);
        Ok(())
    }
}

impl Transport for RecordTransport {
    fn enumerate(&self) -> Result<Vec<Endpoint>, TransportError> {
        self.inner.enumerate()
    }

    fn open(&self, endpoint: &EndpointId) -> Result<Box<dyn HidSession>, TransportError> {
        Ok(Box::new(RecordSession {
            inner: self.inner.open(endpoint)?,
            steps: Arc::clone(&self.steps),
        }))
    }

    fn read_bas(
        &self,
        endpoint: &EndpointId,
        timeout: Duration,
    ) -> Result<Option<u8>, TransportError> {
        let result = self.inner.read_bas(endpoint, timeout);
        let outcome = match &result {
            Ok(Some(percentage)) => Some(StepOutcome::Bas {
                percentage: *percentage,
            }),
            Ok(None) => None,
            Err(error) => Some(StepOutcome::Error {
                error: RecordSession::error(error),
            }),
        };
        if let Some(outcome) = outcome {
            Self::record(
                &self.steps,
                FixtureStep {
                    operation: Operation::ReadBas,
                    report_id: None,
                    request_hex: None,
                    length: None,
                    outcome,
                },
            )?;
        }
        result
    }

    fn read_xinput(
        &self,
        endpoint: &EndpointId,
    ) -> Result<Option<super::XInputReport>, TransportError> {
        let result = self.inner.read_xinput(endpoint);
        let outcome = match &result {
            Ok(Some(report)) => Some(StepOutcome::XInput {
                battery_type: report.battery_type,
                battery_level: report.battery_level,
            }),
            Ok(None) => None,
            Err(error) => Some(StepOutcome::Error {
                error: RecordSession::error(error),
            }),
        };
        if let Some(outcome) = outcome {
            Self::record(
                &self.steps,
                FixtureStep {
                    operation: Operation::ReadXinput,
                    report_id: None,
                    request_hex: None,
                    length: None,
                    outcome,
                },
            )?;
        }
        result
    }
}

struct RecordSession {
    inner: Box<dyn HidSession>,
    steps: Arc<Mutex<Vec<FixtureStep>>>,
}

impl RecordSession {
    fn error(error: &TransportError) -> ReplayError {
        match error {
            TransportError::PermissionDenied => ReplayError::PermissionDenied,
            TransportError::Timeout => ReplayError::Timeout,
            TransportError::DeviceBusy => ReplayError::DeviceBusy,
            TransportError::DeviceDisconnected => ReplayError::Disconnected,
            TransportError::Unsupported(_)
            | TransportError::InvalidResponse(_)
            | TransportError::Message(_) => ReplayError::Transport,
        }
    }
}

impl HidSession for RecordSession {
    fn send_feature(&mut self, report_id: u8, report: &[u8]) -> Result<(), TransportError> {
        let result = self.inner.send_feature(report_id, report);
        let outcome = match &result {
            Ok(()) => StepOutcome::Ok,
            Err(error) => StepOutcome::Error {
                error: Self::error(error),
            },
        };
        RecordTransport::record(
            &self.steps,
            FixtureStep {
                operation: Operation::SendFeature,
                report_id: Some(report_id),
                request_hex: Some(Fixture::encode_hex(report)),
                length: None,
                outcome,
            },
        )?;
        result
    }

    fn get_feature(&mut self, report_id: u8, length: usize) -> Result<Vec<u8>, TransportError> {
        let result = self.inner.get_feature(report_id, length);
        let outcome = match &result {
            Ok(bytes) => StepOutcome::Response {
                report_id,
                hex: Fixture::encode_hex(bytes),
            },
            Err(error) => StepOutcome::Error {
                error: Self::error(error),
            },
        };
        RecordTransport::record(
            &self.steps,
            FixtureStep {
                operation: Operation::GetFeature,
                report_id: Some(report_id),
                request_hex: None,
                length: Some(length),
                outcome,
            },
        )?;
        result
    }

    fn write_output(&mut self, report_id: u8, report: &[u8]) -> Result<(), TransportError> {
        let result = self.inner.write_output(report_id, report);
        let outcome = match &result {
            Ok(()) => StepOutcome::Ok,
            Err(error) => StepOutcome::Error {
                error: Self::error(error),
            },
        };
        RecordTransport::record(
            &self.steps,
            FixtureStep {
                operation: Operation::WriteOutput,
                report_id: Some(report_id),
                request_hex: Some(Fixture::encode_hex(report)),
                length: None,
                outcome,
            },
        )?;
        result
    }

    fn set_output(&mut self, report_id: u8, report: &[u8]) -> Result<(), TransportError> {
        let result = self.inner.set_output(report_id, report);
        let outcome = match &result {
            Ok(()) => StepOutcome::Ok,
            Err(error) => StepOutcome::Error {
                error: Self::error(error),
            },
        };
        RecordTransport::record(
            &self.steps,
            FixtureStep {
                operation: Operation::SetOutput,
                report_id: Some(report_id),
                request_hex: Some(Fixture::encode_hex(report)),
                length: None,
                outcome,
            },
        )?;
        result
    }

    fn read_input(&mut self, timeout: Duration) -> Result<HidReport, TransportError> {
        let result = self.inner.read_input(timeout);
        let outcome = match &result {
            Ok(report) => StepOutcome::Response {
                report_id: report.report_id,
                hex: Fixture::encode_hex(&report.data),
            },
            Err(error) => StepOutcome::Error {
                error: Self::error(error),
            },
        };
        RecordTransport::record(
            &self.steps,
            FixtureStep {
                operation: Operation::ReadInput,
                report_id: None,
                request_hex: None,
                length: None,
                outcome,
            },
        )?;
        result
    }

    fn get_input_report(
        &mut self,
        report_id: u8,
        length: usize,
    ) -> Result<Vec<u8>, TransportError> {
        let result = self.inner.get_input_report(report_id, length);
        let outcome = match &result {
            Ok(bytes) => StepOutcome::Response {
                report_id,
                hex: Fixture::encode_hex(bytes),
            },
            Err(error) => StepOutcome::Error {
                error: Self::error(error),
            },
        };
        RecordTransport::record(
            &self.steps,
            FixtureStep {
                operation: Operation::GetInput,
                report_id: Some(report_id),
                request_hex: None,
                length: Some(length),
                outcome,
            },
        )?;
        result
    }
}
