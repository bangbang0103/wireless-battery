//! Interpreter for bounded declarative device packages.

use super::{Deadline, DeviceContext, MatchDecision, Provider, ProviderMetadata};
use crate::codec;
use crate::error::BatteryError;
use crate::model::{
    BatteryLevel, ChargeState, Endpoint, ProviderReading, Reachability, ReadingSource,
};
use crate::package::schema::{
    Checksum, ChecksumAlgorithm, Constant, DevicePackage, FieldSpec, FieldType, PackageTransport,
    TransportKind,
};
use crate::transport::HidSession;
use std::time::Duration;

/// A provider backed by a validated YAML package.
#[derive(Debug, Clone)]
pub struct DeclarativeProvider {
    package: DevicePackage,
}

impl DeclarativeProvider {
    /// Construct from an already validated package.
    #[must_use]
    pub fn new(package: DevicePackage) -> Self {
        Self { package }
    }

    /// Parse and validate a package without loading evidence from disk.
    pub fn from_yaml(yaml: &str, name: &str) -> Result<Self, BatteryError> {
        crate::package::parse_package(yaml, name).map(Self::new)
    }

    /// Access the immutable package definition.
    #[must_use]
    pub fn package(&self) -> &DevicePackage {
        &self.package
    }
}

impl Provider for DeclarativeProvider {
    fn id(&self) -> &str {
        &self.package.id
    }

    fn metadata(&self) -> ProviderMetadata {
        ProviderMetadata {
            version: self.package.version.clone(),
            support_status: self.package.status,
            capabilities: self.package.capabilities,
            io_timeout: Duration::from_millis(self.package.transport.timeout_ms),
            cooldown: Duration::from_millis(self.package.transport.cooldown_ms),
        }
    }

    fn display_name(&self, _endpoint: &Endpoint) -> Option<String> {
        Some(self.package.name.clone())
    }

    fn is_declarative(&self) -> bool {
        true
    }

    fn matches(&self, endpoint: &Endpoint) -> MatchDecision {
        if endpoint.transport != crate::model::TransportKind::Hid
            && endpoint.transport != crate::model::TransportKind::Mock
        {
            return MatchDecision::NoMatch;
        }
        let rule = &self.package.r#match.all;
        let matches = rule.vid.is_none() || rule.vid == endpoint.vid;
        let matches = matches && (rule.pid.is_none() || rule.pid == endpoint.pid);
        let matches =
            matches && (rule.usage_page.is_none() || rule.usage_page == endpoint.usage_page);
        let matches = matches && (rule.usage.is_none() || rule.usage == endpoint.usage);
        let matches = matches
            && (rule.input_report_length.is_none()
                || rule.input_report_length == endpoint.input_report_length);
        let matches = matches
            && (rule.output_report_length.is_none()
                || rule.output_report_length == endpoint.output_report_length);
        let matches = matches
            && (rule.feature_report_length.is_none()
                || rule.feature_report_length == endpoint.feature_report_length);
        if matches {
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
        let mut final_report = None;
        for (index, step) in self.package.query.steps.iter().enumerate() {
            deadline.check("declarative query")?;
            let operation = format!("declarative query round {}", index + 1);
            match self.package.transport.kind {
                TransportKind::HidFeature => {
                    let request = step.send_feature.as_ref().ok_or_else(|| {
                        BatteryError::invalid_response("validated feature request is absent")
                    })?;
                    let receive = step.receive_feature.as_ref().ok_or_else(|| {
                        BatteryError::invalid_response("validated feature response is absent")
                    })?;
                    let payload = codec::decode_hex(&request.payload_hex)
                        .map_err(BatteryError::invalid_response)?;
                    session
                        .send_feature(request.report_id, &payload)
                        .map_err(|error| error.into_battery(&device.endpoint.id))?;
                    deadline.check(&operation)?;
                    let report = session
                        .get_feature(receive.report_id, receive.length)
                        .map_err(|error| error.into_battery(&device.endpoint.id))?;
                    validate_report(
                        receive.length,
                        &receive.constants,
                        receive.checksum.as_ref(),
                        &report,
                    )?;
                    final_report = Some(report);
                }
                TransportKind::HidOutput => {
                    let request = step.write_output.as_ref().ok_or_else(|| {
                        BatteryError::invalid_response("validated output request is absent")
                    })?;
                    let receive = step.read_input.as_ref().ok_or_else(|| {
                        BatteryError::invalid_response("validated input response is absent")
                    })?;
                    let payload = codec::decode_hex(&request.payload_hex)
                        .map_err(BatteryError::invalid_response)?;
                    session
                        .write_output(request.report_id, &payload)
                        .map_err(|error| error.into_battery(&device.endpoint.id))?;
                    deadline.check(&operation)?;
                    let timeout = operation_timeout(&self.package.transport, deadline, &operation)?;
                    let report = session
                        .read_input(timeout)
                        .map_err(|error| error.into_battery(&device.endpoint.id))?;
                    if report.report_id != receive.report_id {
                        return Err(BatteryError::invalid_response(format!(
                            "report ID mismatch in round {}",
                            index + 1
                        )));
                    }
                    validate_report(
                        receive.length,
                        &receive.constants,
                        receive.checksum.as_ref(),
                        &report.data,
                    )?;
                    final_report = Some(report.data);
                }
            }
        }

        let report = final_report.ok_or_else(|| BatteryError::invalid_response("empty query"))?;
        validate_report(
            self.package.response.length,
            &self.package.response.constants,
            self.package.response.checksum.as_ref(),
            &report,
        )?;
        parse_fields(&self.package, &report)
    }
}

fn operation_timeout(
    transport: &PackageTransport,
    deadline: Deadline,
    operation: &str,
) -> Result<Duration, BatteryError> {
    Ok(deadline
        .remaining(operation)?
        .min(Duration::from_millis(transport.timeout_ms)))
}

fn validate_report(
    length: usize,
    constants: &[Constant],
    checksum: Option<&Checksum>,
    report: &[u8],
) -> Result<(), BatteryError> {
    if report.len() != length {
        return Err(BatteryError::invalid_response(format!(
            "response length mismatch: expected {length}, got {}",
            report.len()
        )));
    }
    for constant in constants {
        let expected = codec::decode_hex(&constant.hex).map_err(BatteryError::invalid_response)?;
        let actual = report
            .get(constant.offset..constant.offset.saturating_add(expected.len()))
            .ok_or_else(|| BatteryError::invalid_response("constant exceeds response"))?;
        if actual != expected {
            return Err(BatteryError::invalid_response(format!(
                "constant mismatch at payload offset {}",
                constant.offset
            )));
        }
    }
    if let Some(checksum) = checksum {
        let [start, end] = checksum.range;
        let bytes = report
            .get(start..=end)
            .ok_or_else(|| BatteryError::invalid_response("checksum range exceeds response"))?;
        let calculated = match checksum.algorithm {
            ChecksumAlgorithm::Sum8 => bytes.iter().fold(0_u8, |sum, byte| sum.wrapping_add(*byte)),
            ChecksumAlgorithm::Xor8 => bytes.iter().fold(0_u8, |sum, byte| sum ^ byte),
        };
        let expected = report
            .get(checksum.offset)
            .ok_or_else(|| BatteryError::invalid_response("checksum offset exceeds response"))?;
        if calculated != *expected {
            return Err(BatteryError::ChecksumFailed);
        }
    }
    Ok(())
}

fn parse_fields(package: &DevicePackage, report: &[u8]) -> Result<ProviderReading, BatteryError> {
    let fields = &package.fields;
    let percentage = fields
        .percentage
        .as_ref()
        .map(|field| parse_percentage(field, report))
        .transpose()?
        .flatten();
    let level = fields
        .level
        .as_ref()
        .map(|field| parse_level(field, report))
        .transpose()?
        .flatten();
    let voltage_mv = fields
        .voltage_mv
        .as_ref()
        .map(|field| parse_voltage(field, report))
        .transpose()?
        .flatten();
    let charge_state = fields
        .charge_state
        .as_ref()
        .map(|field| parse_charge_state(field, report))
        .transpose()?
        .unwrap_or(ChargeState::Unknown);
    let reachability = fields
        .reachability
        .as_ref()
        .map(|field| parse_reachability(field, report))
        .transpose()?
        .unwrap_or(Reachability::Unknown);
    Ok(ProviderReading {
        percentage,
        level,
        voltage_mv,
        charge_state,
        reachability,
        source: ReadingSource::VendorHid,
    })
}

fn raw_value(field: &FieldSpec, report: &[u8]) -> Result<Option<(u8, String)>, BatteryError> {
    let byte = *report
        .get(field.offset)
        .ok_or_else(|| BatteryError::invalid_response("field offset exceeds response"))?;
    if field.unknown_values.contains(&byte) {
        return Ok(None);
    }
    let value = match field.r#type {
        FieldType::U8 => byte.to_string(),
        FieldType::BoolBit => {
            let bit = field
                .bit
                .ok_or_else(|| BatteryError::invalid_response("bool_bit field has no bit"))?;
            ((byte >> bit) & 1 == 1).to_string()
        }
    };
    Ok(Some((byte, value)))
}

fn parse_percentage(field: &FieldSpec, report: &[u8]) -> Result<Option<u8>, BatteryError> {
    let Some((value, _)) = raw_value(field, report)? else {
        return Ok(None);
    };
    let [minimum, maximum] = field.valid_range.unwrap_or([0, 100]);
    if value < minimum || value > maximum || value > 100 {
        return Err(BatteryError::invalid_response(format!(
            "percentage {value} is outside {minimum}..={maximum}"
        )));
    }
    Ok(Some(value))
}

fn parse_level(field: &FieldSpec, report: &[u8]) -> Result<Option<BatteryLevel>, BatteryError> {
    let Some((_, value)) = raw_value(field, report)? else {
        return Ok(None);
    };
    let Some(mapped) = field.map.get(&value) else {
        return Ok(None);
    };
    match mapped.as_str() {
        "critical" => Ok(Some(BatteryLevel::Critical)),
        "low" => Ok(Some(BatteryLevel::Low)),
        "medium" => Ok(Some(BatteryLevel::Medium)),
        "high" => Ok(Some(BatteryLevel::High)),
        "full" => Ok(Some(BatteryLevel::Full)),
        _ => Err(BatteryError::invalid_response(
            "invalid battery level mapping",
        )),
    }
}

fn parse_voltage(field: &FieldSpec, report: &[u8]) -> Result<Option<u32>, BatteryError> {
    let Some((value, _)) = raw_value(field, report)? else {
        return Ok(None);
    };
    u32::from(value)
        .checked_mul(field.scale)
        .map(Some)
        .ok_or_else(|| BatteryError::invalid_response("voltage overflow"))
}

fn parse_charge_state(field: &FieldSpec, report: &[u8]) -> Result<ChargeState, BatteryError> {
    let Some((_, value)) = raw_value(field, report)? else {
        return Ok(ChargeState::Unknown);
    };
    let Some(mapped) = field.map.get(&value) else {
        return Ok(ChargeState::Unknown);
    };
    match mapped.as_str() {
        "charging" => Ok(ChargeState::Charging),
        "not_charging" => Ok(ChargeState::NotCharging),
        "full" => Ok(ChargeState::Full),
        "discharging" => Ok(ChargeState::Discharging),
        "unknown" => Ok(ChargeState::Unknown),
        _ => Err(BatteryError::invalid_response(
            "invalid charge state mapping",
        )),
    }
}

fn parse_reachability(field: &FieldSpec, report: &[u8]) -> Result<Reachability, BatteryError> {
    let Some((_, value)) = raw_value(field, report)? else {
        return Ok(Reachability::Unknown);
    };
    let Some(mapped) = field.map.get(&value) else {
        return Ok(Reachability::Unknown);
    };
    match mapped.as_str() {
        "online" => Ok(Reachability::Online),
        "sleeping" => Ok(Reachability::Sleeping),
        "offline" => Ok(Reachability::Offline),
        "unknown" => Ok(Reachability::Unknown),
        _ => Err(BatteryError::invalid_response(
            "invalid reachability mapping",
        )),
    }
}
