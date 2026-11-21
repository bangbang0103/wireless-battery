//! Shared HID fixed-frame runner and common protocol helpers.

use crate::error::BatteryError;
use crate::model::{
    Capabilities, ChargeState, Endpoint, ProviderReading, ReadingSource, SupportStatus,
};
use crate::providers::{Deadline, DeviceContext, MatchDecision, Provider, ProviderMetadata};
use crate::transport::{HidReport, HidSession, TransportError};
use std::time::Duration;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum SendMode {
    Output,
    SetOutput,
    Feature,
}

pub(crate) const OUTPUT: &[SendMode] = &[SendMode::Output];
pub(crate) const SET_OUTPUT: &[SendMode] = &[SendMode::SetOutput];
pub(crate) const OUTPUT_THEN_SET: &[SendMode] = &[SendMode::Output, SendMode::SetOutput];

pub(crate) struct FixedSpec {
    pub(crate) id: &'static str,
    pub(crate) name: &'static str,
    pub(crate) status: SupportStatus,
    pub(crate) matches: fn(&Endpoint) -> bool,
    pub(crate) query: fn() -> Vec<u8>,
    pub(crate) modes: &'static [SendMode],
    pub(crate) attempts: usize,
    pub(crate) max_reports: usize,
    pub(crate) io_timeout: Duration,
    pub(crate) capabilities: Capabilities,
    pub(crate) parse: fn(&[u8]) -> Result<Option<ProviderReading>, BatteryError>,
}

pub(crate) struct FixedProvider {
    pub(crate) spec: &'static FixedSpec,
}

impl FixedProvider {
    pub(crate) const fn new(spec: &'static FixedSpec) -> Self {
        Self { spec }
    }
}

impl Provider for FixedProvider {
    fn id(&self) -> &str {
        self.spec.id
    }

    fn metadata(&self) -> ProviderMetadata {
        ProviderMetadata {
            version: "1.0.0".to_owned(),
            support_status: self.spec.status,
            capabilities: self.spec.capabilities,
            io_timeout: self.spec.io_timeout,
            cooldown: Duration::from_millis(100),
        }
    }

    fn display_name(&self, _endpoint: &Endpoint) -> Option<String> {
        Some(self.spec.name.to_owned())
    }

    fn matches(&self, endpoint: &Endpoint) -> MatchDecision {
        if (self.spec.matches)(endpoint) {
            MatchDecision::Exact
        } else {
            MatchDecision::NoMatch
        }
    }

    fn read(
        &self,
        device: &DeviceContext<'_>,
        session: &mut dyn HidSession,
        deadline: Deadline,
    ) -> Result<ProviderReading, BatteryError> {
        for _ in 0..8 {
            let timeout = deadline
                .remaining(self.spec.id)?
                .min(Duration::from_millis(10));
            match session.read_input(timeout) {
                Ok(_) => {}
                Err(TransportError::Timeout) => break,
                Err(error) => return Err(error.into_battery(&device.endpoint.id)),
            }
        }
        for _attempt in 0..self.spec.attempts {
            deadline.check(self.spec.id)?;
            send_with_modes(session, device, self.spec.modes, &(self.spec.query)())?;
            for _ in 0..self.spec.max_reports {
                let timeout = deadline.remaining(self.spec.id)?.min(self.spec.io_timeout);
                let frame = match session.read_input(timeout) {
                    Ok(report) => frame(report),
                    Err(TransportError::Timeout) => break,
                    Err(error) => return Err(error.into_battery(&device.endpoint.id)),
                };
                if let Some(reading) = (self.spec.parse)(&frame)? {
                    return Ok(reading);
                }
            }
        }
        Err(BatteryError::timeout(self.spec.id))
    }
}

pub(crate) fn experimental_metadata(
    capabilities: Capabilities,
    io_timeout: Duration,
) -> ProviderMetadata {
    ProviderMetadata {
        version: "1.0.0".to_owned(),
        support_status: SupportStatus::Experimental,
        capabilities,
        io_timeout,
        cooldown: Duration::from_millis(100),
    }
}

pub(crate) fn sleep(deadline: Deadline, duration: Duration) -> Result<(), BatteryError> {
    let remaining = deadline.remaining("protocol wait")?;
    std::thread::sleep(duration.min(remaining));
    deadline.check("protocol wait")
}

pub(crate) fn wait_frame(
    session: &mut dyn HidSession,
    device: &DeviceContext<'_>,
    deadline: Deadline,
    timeout: Duration,
    max_reports: usize,
    validate: impl Fn(&[u8]) -> Result<bool, BatteryError>,
) -> Result<Vec<u8>, BatteryError> {
    for _ in 0..max_reports {
        let remaining = deadline.remaining("HID input")?.min(timeout);
        let report = match session.read_input(remaining) {
            Ok(report) => frame(report),
            Err(TransportError::Timeout) => return Err(BatteryError::timeout("HID input")),
            Err(error) => return Err(error.into_battery(&device.endpoint.id)),
        };
        if validate(&report)? {
            return Ok(report);
        }
    }
    Err(BatteryError::timeout("HID input"))
}

pub(crate) fn query(
    session: &mut dyn HidSession,
    device: &DeviceContext<'_>,
    deadline: Deadline,
    request: Vec<u8>,
    timeout: Duration,
    validate: impl Fn(&[u8]) -> Result<bool, BatteryError>,
) -> Result<Vec<u8>, BatteryError> {
    let mut last_timeout = None;
    for _ in 0..3 {
        send_with_modes(session, device, OUTPUT, &request)?;
        match wait_frame(session, device, deadline, timeout, 16, &validate) {
            Ok(frame) => return Ok(frame),
            Err(error @ BatteryError::Timeout { .. }) => last_timeout = Some(error),
            Err(error) => return Err(error),
        }
    }
    Err(last_timeout.unwrap_or_else(|| BatteryError::timeout("HID query")))
}

pub(crate) fn send_with_modes(
    session: &mut dyn HidSession,
    device: &DeviceContext<'_>,
    modes: &[SendMode],
    frame: &[u8],
) -> Result<(), BatteryError> {
    let Some((&report_id, payload)) = frame.split_first() else {
        return Err(BatteryError::invalid_response("empty request frame"));
    };
    let mut last_error: Option<TransportError> = None;
    for mode in modes {
        let result = match mode {
            SendMode::Output => session.write_output(report_id, payload),
            SendMode::SetOutput => session.set_output(report_id, payload),
            SendMode::Feature => session.send_feature(report_id, payload),
        };
        match result {
            Ok(()) => return Ok(()),
            Err(
                error @ (TransportError::PermissionDenied
                | TransportError::DeviceDisconnected
                | TransportError::DeviceBusy),
            ) => return Err(error.into_battery(&device.endpoint.id)),
            Err(error @ TransportError::Unsupported(_)) => {
                if last_error.is_none() {
                    last_error = Some(error);
                }
            }
            Err(error) => last_error = Some(error),
        }
    }
    Err(last_error
        .unwrap_or_else(|| TransportError::Unsupported("no send mode".to_owned()))
        .into_battery(&device.endpoint.id))
}

pub(crate) fn frame(report: HidReport) -> Vec<u8> {
    let mut frame = Vec::with_capacity(report.data.len() + 1);
    frame.push(report.report_id);
    frame.extend_from_slice(&report.data);
    frame
}

/// Strip a leading `0x00` report ID from an unnumbered HID frame, matching
/// the wire convention where numbered reports start with their report ID
/// and unnumbered reports start with payload byte 0.
pub(crate) fn unnumbered_payload(frame: &[u8]) -> &[u8] {
    if frame.first() == Some(&0) {
        &frame[1..]
    } else {
        frame
    }
}

pub(crate) fn crc16_poly_1021(bytes: &[u8], init: u16) -> u16 {
    let mut crc = init;
    for &byte in bytes {
        crc ^= u16::from(byte) << 8;
        for _ in 0..8 {
            if (crc & 0x8000) != 0 {
                crc = (crc << 1) ^ 0x1021;
            } else {
                crc <<= 1;
            }
        }
    }
    crc
}

pub(crate) fn feature_frame(
    session: &mut dyn HidSession,
    device: &DeviceContext<'_>,
    report_id: u8,
    payload_length: usize,
) -> Result<Vec<u8>, BatteryError> {
    let payload = session
        .get_feature(report_id, payload_length)
        .map_err(|error| error.into_battery(&device.endpoint.id))?;
    Ok(frame(HidReport {
        report_id,
        data: payload,
    }))
}

pub(crate) fn input_frame(
    session: &mut dyn HidSession,
    device: &DeviceContext<'_>,
    report_id: u8,
    payload_length: usize,
) -> Result<Vec<u8>, BatteryError> {
    let payload = session
        .get_input_report(report_id, payload_length)
        .map_err(|error| error.into_battery(&device.endpoint.id))?;
    Ok(frame(HidReport {
        report_id,
        data: payload,
    }))
}

pub(crate) fn drain_input(
    session: &mut dyn HidSession,
    device: &DeviceContext<'_>,
    deadline: Deadline,
    max_reports: usize,
) -> Result<(), BatteryError> {
    for _ in 0..max_reports {
        let timeout = deadline
            .remaining("HID flush")?
            .min(Duration::from_millis(10));
        match session.read_input(timeout) {
            Ok(_) => {}
            Err(TransportError::Timeout) => break,
            Err(error) => return Err(error.into_battery(&device.endpoint.id)),
        }
    }
    Ok(())
}

/// Piecewise-linear interpolation through `(mv, pct)` points sorted by descending millivolts.
pub(crate) fn interpolate_mv(mv: u32, table: &[(u32, u8)]) -> u8 {
    let Some(&(hi_mv, hi_pct)) = table.first() else {
        return 0;
    };
    if mv >= hi_mv {
        return hi_pct.min(100);
    }
    for window in table.windows(2) {
        let (m0, p0) = window[0];
        let (m1, p1) = window[1];
        if mv >= m1 && m0 > m1 {
            let num = u64::from(mv - m1) * u64::from(p0.saturating_sub(p1));
            let den = u64::from(m0 - m1);
            let pct = u64::from(p1) + (num + den / 2) / den;
            return pct.min(100) as u8;
        }
    }
    table.last().map_or(0, |&(_, p)| p.min(100))
}

pub(crate) fn reading(
    percentage: Option<u8>,
    charge_state: ChargeState,
    voltage_mv: Option<u32>,
) -> ProviderReading {
    ProviderReading {
        percentage,
        charge_state,
        voltage_mv,
        source: ReadingSource::VendorHid,
        ..ProviderReading::default()
    }
}

pub(crate) fn checked_percentage(value: u8) -> Result<u8, BatteryError> {
    if value <= 100 {
        Ok(value)
    } else {
        Err(BatteryError::invalid_response(format!(
            "percentage {value} exceeds 100"
        )))
    }
}

pub(crate) fn charge_bool(value: u8) -> ChargeState {
    if value == 0 {
        ChargeState::Discharging
    } else if value == 1 {
        ChargeState::Charging
    } else {
        ChargeState::Unknown
    }
}

pub(crate) fn checksum_sum(bytes: &[u8]) -> u8 {
    bytes.iter().fold(0_u8, |sum, byte| sum.wrapping_add(*byte))
}

pub(crate) const fn capabilities(
    percentage: bool,
    charge_state: bool,
    voltage: bool,
) -> Capabilities {
    Capabilities {
        percentage,
        charge_state,
        voltage,
        discrete_level: false,
        reachability: false,
    }
}
