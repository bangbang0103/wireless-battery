//! Strict, ordered JSON fixture replay.

use super::{HidReport, HidSession, Transport, TransportError, XInputReport};
use crate::client::BatteryClient;
use crate::codec;
use crate::error::{BatteryError, ErrorKind};
use crate::model::{
    BatteryLevel, BatteryReading, ChargeState, ClientConfig, Endpoint, EndpointId, Freshness,
    Precision, Reachability, ReadOptions,
};
use crate::registry::Registry;
use serde::{Deserialize, Serialize};
use std::path::Path;
use std::sync::{Arc, Mutex};
use std::time::Duration;

/// A complete recorded or synthetic protocol session.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Fixture {
    /// Fixture format version.
    pub schema_version: u32,
    /// Provider or package under test.
    pub target: FixtureTarget,
    /// Stable test case name.
    pub case: String,
    /// Real or synthetic evidence origin.
    pub source: FixtureSource,
    /// Collection environment.
    pub environment: FixtureEnvironment,
    /// Static synthetic endpoint used by replay.
    pub endpoint: Endpoint,
    /// Ordered normalized HID calls.
    pub steps: Vec<FixtureStep>,
    /// Expected normalized result.
    pub expect: Expectation,
}

/// Fixture target.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct FixtureTarget {
    /// Target type: `package`, `provider`, or `bluetooth_bas`.
    pub kind: String,
    /// Stable target ID.
    pub id: String,
}

/// Evidence origin.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum FixtureSource {
    /// Captured from real hardware.
    Real,
    /// Constructed or derived without claiming hardware verification.
    Synthetic,
}

/// Non-sensitive collection environment.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct FixtureEnvironment {
    /// Operating system, if known.
    pub os: Option<String>,
    /// Connection description.
    pub connection: String,
    /// Firmware version, if publicly supplied by the device.
    pub firmware: Option<String>,
}

/// One exact `HidSession` call and its outcome.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct FixtureStep {
    /// Expected operation.
    pub operation: Operation,
    /// Expected request report ID, when applicable.
    pub report_id: Option<u8>,
    /// Exact request payload, excluding report ID.
    pub request_hex: Option<String>,
    /// Expected requested payload length for `get_feature`.
    pub length: Option<usize>,
    /// Recorded success, response or error.
    pub outcome: StepOutcome,
}

/// HID and standard transport operation names shared with the transport trait.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum Operation {
    /// `send_feature`.
    SendFeature,
    /// `get_feature`.
    GetFeature,
    /// `write_output`.
    WriteOutput,
    /// `set_output` (HID SET_REPORT for an output report).
    SetOutput,
    /// `read_input`.
    ReadInput,
    /// `get_input_report` (HID GET_REPORT for an input report).
    GetInput,
    /// Bluetooth LE Battery Service read.
    ReadBas,
    /// Windows XInput battery query.
    ReadXinput,
}

/// Outcome of one transport operation.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(tag = "kind", rename_all = "snake_case")]
pub enum StepOutcome {
    /// A write completed without a returned report.
    Ok,
    /// A HID report was returned.
    Response {
        /// Returned report ID.
        report_id: u8,
        /// Returned payload, excluding report ID.
        hex: String,
    },
    /// A Bluetooth LE BAS percentage byte was returned.
    Bas {
        /// Raw percentage value returned by BAS `0x2A19`.
        percentage: u8,
    },
    /// A raw XInput battery report was returned.
    #[serde(rename = "xinput")]
    XInput {
        /// Raw XInput battery type.
        battery_type: u8,
        /// Raw XInput battery level.
        battery_level: u8,
    },
    /// A transport error occurred.
    Error {
        /// Stable transport error category.
        error: ReplayError,
    },
}

/// Transport errors representable in replay.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum ReplayError {
    /// Permission denied.
    PermissionDenied,
    /// Operation timeout.
    Timeout,
    /// Endpoint busy.
    DeviceBusy,
    /// Endpoint disconnected.
    Disconnected,
    /// Other transport error.
    Transport,
}

impl From<ReplayError> for TransportError {
    fn from(value: ReplayError) -> Self {
        match value {
            ReplayError::PermissionDenied => Self::PermissionDenied,
            ReplayError::Timeout => Self::Timeout,
            ReplayError::DeviceBusy => Self::DeviceBusy,
            ReplayError::Disconnected => Self::DeviceDisconnected,
            ReplayError::Transport => Self::Message("recorded transport failure".to_owned()),
        }
    }
}

/// Expected stable reading fields or public error category.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(tag = "kind", rename_all = "snake_case")]
pub enum Expectation {
    /// A successful normalized reading.
    Reading {
        /// Expected percentage, including an explicit null.
        percentage: Option<u8>,
        /// Expected discrete level, when asserted.
        #[serde(default)]
        level: Option<BatteryLevel>,
        /// Expected voltage, when asserted.
        #[serde(default)]
        voltage_mv: Option<u32>,
        /// Expected charge state.
        charge_state: ChargeState,
        /// Expected reachability.
        reachability: Reachability,
        /// Expected precision.
        precision: Precision,
    },
    /// A failed read.
    Error {
        /// Expected stable error category.
        error: ErrorKind,
    },
}

impl Fixture {
    /// Load and parse one fixture. Schema validation is performed by the CLI validator.
    pub fn from_path(path: &Path) -> Result<Self, String> {
        let text = std::fs::read_to_string(path)
            .map_err(|error| format!("cannot read fixture: {error}"))?;
        serde_json::from_str(&text).map_err(|error| format!("cannot parse fixture: {error}"))
    }

    /// Encode payload bytes in the canonical fixture representation.
    #[must_use]
    pub fn encode_hex(bytes: &[u8]) -> String {
        codec::encode_hex(bytes)
    }

    /// Decode canonical whitespace-separated or contiguous hexadecimal bytes.
    pub fn decode_hex(value: &str) -> Result<Vec<u8>, String> {
        codec::decode_hex(value)
    }
}

/// Replay a fixture against `registry`, asserting target match, expected outcome,
/// and strict step consumption.
pub fn verify_fixture(fixture: &Fixture, registry: Registry) -> Result<(), BatteryError> {
    if registry.find(&fixture.target.id).is_none() {
        return Err(BatteryError::InvalidFixture {
            fixture: fixture.case.clone(),
            reason: format!("target provider is not registered: {}", fixture.target.id),
        });
    }
    let (result, replay) = run_fixture(fixture, registry)?;
    assert_expectation(&fixture.expect, &result, &fixture.case)?;
    replay
        .finish()
        .map_err(|reason| BatteryError::InvalidFixture {
            fixture: fixture.case.clone(),
            reason,
        })
}

/// Execute a single fixture against `registry` and return the raw read result
/// alongside the [`ReplayTransport`] so callers can inspect errors or call `finish()`.
pub fn run_fixture(
    fixture: &Fixture,
    registry: Registry,
) -> Result<(Result<BatteryReading, BatteryError>, ReplayTransport), BatteryError> {
    let replay = Arc::new(ReplayTransport::new(fixture));
    let client = BatteryClient::with_transport(
        ClientConfig {
            allow_experimental: true,
            enable_cache: false,
            ..ClientConfig::default()
        },
        replay.clone(),
        registry,
    )?;
    let device =
        client
            .list_devices()?
            .into_iter()
            .next()
            .ok_or_else(|| BatteryError::InvalidFixture {
                fixture: fixture.case.clone(),
                reason: "fixture endpoint was not enumerated".to_owned(),
            })?;
    if device.provider_id.as_deref() != Some(fixture.target.id.as_str()) {
        return Err(BatteryError::InvalidFixture {
            fixture: fixture.case.clone(),
            reason: format!(
                "endpoint matched {:?}, expected target provider {}",
                device.provider_id, fixture.target.id
            ),
        });
    }
    let result = client.read_battery_with_options(
        &device.id.0,
        ReadOptions {
            timeout: Duration::from_secs(1),
            allow_stale: false,
            allow_experimental: true,
        },
    );
    Ok((result, (*replay).clone()))
}

/// Assert that a replay read outcome matches the fixture's [`Expectation`].
pub fn assert_expectation(
    expected: &Expectation,
    actual: &Result<BatteryReading, BatteryError>,
    case: &str,
) -> Result<(), BatteryError> {
    match (expected, actual) {
        (
            Expectation::Reading {
                percentage,
                level,
                voltage_mv,
                charge_state,
                reachability,
                precision,
            },
            Ok(reading),
        ) => {
            let matches = reading.percentage == *percentage
                && reading.level == *level
                && reading.voltage_mv == *voltage_mv
                && reading.charge_state == *charge_state
                && reading.reachability == *reachability
                && reading.precision == *precision
                && reading.freshness == Freshness::Live;
            if matches {
                Ok(())
            } else {
                Err(BatteryError::InvalidFixture {
                    fixture: case.to_owned(),
                    reason: format!("reading mismatch: expected {expected:?}, got {reading:?}"),
                })
            }
        }
        (Expectation::Error { error }, Err(actual)) if *error == actual.kind() => Ok(()),
        (Expectation::Error { error }, Err(actual)) => Err(BatteryError::InvalidFixture {
            fixture: case.to_owned(),
            reason: format!("expected error {error:?}, got {:?}", actual.kind()),
        }),
        (Expectation::Error { error }, Ok(reading)) => Err(BatteryError::InvalidFixture {
            fixture: case.to_owned(),
            reason: format!("expected error {error:?}, got successful reading {reading:?}"),
        }),
        (Expectation::Reading { .. }, Err(error)) => Err(BatteryError::InvalidFixture {
            fixture: case.to_owned(),
            reason: format!("expected reading, got error ({:?}): {error}", error.kind()),
        }),
    }
}

/// Strict replay transport. Requests must match in order and all steps must be consumed.
#[derive(Clone)]
pub struct ReplayTransport {
    endpoint: Endpoint,
    state: Arc<Mutex<ReplayState>>,
}

struct ReplayState {
    steps: Vec<FixtureStep>,
    position: usize,
}

impl ReplayTransport {
    /// Construct replay for a fixture's synthetic endpoint.
    #[must_use]
    pub fn new(fixture: &Fixture) -> Self {
        Self {
            endpoint: fixture.endpoint.clone(),
            state: Arc::new(Mutex::new(ReplayState {
                steps: fixture.steps.clone(),
                position: 0,
            })),
        }
    }

    /// Verify that the provider consumed every recorded step.
    pub fn finish(&self) -> Result<(), String> {
        let state = self
            .state
            .lock()
            .map_err(|_| "replay lock poisoned".to_owned())?;
        if state.position == state.steps.len() {
            Ok(())
        } else {
            Err(format!(
                "provider omitted {} recorded step(s), next is step {}",
                state.steps.len() - state.position,
                state.position + 1
            ))
        }
    }

    fn apply(
        &self,
        operation: Operation,
        report_id: Option<u8>,
        request: Option<&[u8]>,
        length: Option<usize>,
    ) -> Result<StepOutcome, TransportError> {
        let mut state = self
            .state
            .lock()
            .map_err(|_| TransportError::Message("replay lock poisoned".to_owned()))?;
        let position = state.position;
        let Some(step) = state.steps.get(position).cloned() else {
            return Err(TransportError::InvalidResponse(format!(
                "extra {operation:?} request after step {}",
                state.steps.len()
            )));
        };
        let actual_hex = request.map(Fixture::encode_hex);
        let expected_hex = step
            .request_hex
            .as_deref()
            .map(Fixture::decode_hex)
            .transpose()
            .map_err(TransportError::InvalidResponse)?
            .map(|bytes| Fixture::encode_hex(&bytes));
        let matches = step.operation == operation
            && step.report_id == report_id
            && expected_hex == actual_hex
            && step.length == length;
        if !matches {
            return Err(TransportError::InvalidResponse(format!(
                "request mismatch at replay step {}: expected {:?} {:?} {:?} {:?}, got {:?} {:?} {:?} {:?}",
                position + 1,
                step.operation,
                step.report_id,
                step.request_hex,
                step.length,
                operation,
                report_id,
                actual_hex,
                length
            )));
        }
        state.position += 1;
        Ok(step.outcome.clone())
    }
}

impl Transport for ReplayTransport {
    fn enumerate(&self) -> Result<Vec<Endpoint>, TransportError> {
        Ok(vec![self.endpoint.clone()])
    }

    fn open(&self, endpoint: &EndpointId) -> Result<Box<dyn HidSession>, TransportError> {
        if endpoint != &self.endpoint.id {
            return Err(TransportError::DeviceDisconnected);
        }
        Ok(Box::new(ReplaySession {
            transport: self.clone(),
        }))
    }

    fn read_bas(
        &self,
        endpoint: &EndpointId,
        _timeout: Duration,
    ) -> Result<Option<u8>, TransportError> {
        if endpoint != &self.endpoint.id {
            return Err(TransportError::DeviceDisconnected);
        }
        match self.apply(Operation::ReadBas, None, None, None)? {
            StepOutcome::Bas { percentage } => Ok(Some(percentage)),
            StepOutcome::Error { error } => Err(error.into()),
            _ => Err(TransportError::InvalidResponse(
                "non-BAS outcome used for read_bas".to_owned(),
            )),
        }
    }

    fn read_xinput(&self, endpoint: &EndpointId) -> Result<Option<XInputReport>, TransportError> {
        if endpoint != &self.endpoint.id {
            return Err(TransportError::DeviceDisconnected);
        }
        match self.apply(Operation::ReadXinput, None, None, None)? {
            StepOutcome::XInput {
                battery_type,
                battery_level,
            } => Ok(Some(XInputReport {
                battery_type,
                battery_level,
            })),
            StepOutcome::Error { error } => Err(error.into()),
            _ => Err(TransportError::InvalidResponse(
                "non-XInput outcome used for read_xinput".to_owned(),
            )),
        }
    }
}

struct ReplaySession {
    transport: ReplayTransport,
}

impl ReplaySession {
    fn response(outcome: StepOutcome) -> Result<HidReport, TransportError> {
        match outcome {
            StepOutcome::Response { report_id, hex } => Ok(HidReport {
                report_id,
                data: Fixture::decode_hex(&hex).map_err(TransportError::InvalidResponse)?,
            }),
            StepOutcome::Error { error } => Err(error.into()),
            StepOutcome::Ok | StepOutcome::Bas { .. } | StepOutcome::XInput { .. } => {
                Err(TransportError::InvalidResponse(
                    "non-report outcome used as a HID report response".to_owned(),
                ))
            }
        }
    }

    fn write(outcome: StepOutcome) -> Result<(), TransportError> {
        match outcome {
            StepOutcome::Ok => Ok(()),
            StepOutcome::Error { error } => Err(error.into()),
            StepOutcome::Response { .. } | StepOutcome::Bas { .. } | StepOutcome::XInput { .. } => {
                Err(TransportError::InvalidResponse(
                    "non-write outcome used for a HID write".to_owned(),
                ))
            }
        }
    }
}

impl HidSession for ReplaySession {
    fn send_feature(&mut self, report_id: u8, report: &[u8]) -> Result<(), TransportError> {
        Self::write(self.transport.apply(
            Operation::SendFeature,
            Some(report_id),
            Some(report),
            None,
        )?)
    }

    fn get_feature(&mut self, report_id: u8, length: usize) -> Result<Vec<u8>, TransportError> {
        let response = Self::response(self.transport.apply(
            Operation::GetFeature,
            Some(report_id),
            None,
            Some(length),
        )?)?;
        if response.report_id != report_id {
            return Err(TransportError::InvalidResponse(
                "feature report ID mismatch".to_owned(),
            ));
        }
        Ok(response.data)
    }

    fn write_output(&mut self, report_id: u8, report: &[u8]) -> Result<(), TransportError> {
        Self::write(self.transport.apply(
            Operation::WriteOutput,
            Some(report_id),
            Some(report),
            None,
        )?)
    }

    fn set_output(&mut self, report_id: u8, report: &[u8]) -> Result<(), TransportError> {
        Self::write(self.transport.apply(
            Operation::SetOutput,
            Some(report_id),
            Some(report),
            None,
        )?)
    }

    fn read_input(&mut self, _timeout: Duration) -> Result<HidReport, TransportError> {
        Self::response(
            self.transport
                .apply(Operation::ReadInput, None, None, None)?,
        )
    }

    fn get_input_report(
        &mut self,
        report_id: u8,
        length: usize,
    ) -> Result<Vec<u8>, TransportError> {
        let response = Self::response(self.transport.apply(
            Operation::GetInput,
            Some(report_id),
            None,
            Some(length),
        )?)?;
        if response.report_id != report_id {
            return Err(TransportError::InvalidResponse(
                "input report ID mismatch".to_owned(),
            ));
        }
        Ok(response.data)
    }
}
