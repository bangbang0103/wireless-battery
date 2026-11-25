//! Logitech Centurion headset battery protocol (usage page `0xFFA0`).
//!
//! Covers wireless bridged receivers (G PRO X 2 LIGHTSPEED, G522 LIGHTSPEED,
//! G PRO X 3 LIGHTSPEED, ASTRO A20 X wireless) and direct USB-wired Centurion
//! headsets (ASTRO A20 X USB wired).

use crate::error::BatteryError;
use crate::model::{
    Capabilities, ChargeState, DeviceKind, Endpoint, ProviderReading, Reachability, ReadingSource,
};
use crate::providers::fixed::{
    checked_percentage, experimental_metadata, frame, send_with_modes, OUTPUT,
};
use crate::providers::{Deadline, DeviceContext, MatchDecision, Provider, ProviderMetadata};
use crate::transport::{HidSession, TransportError};
use std::sync::Arc;
use std::time::Duration;

const VENDOR_LOGITECH: u16 = 0x046D;
const CENTURION_USAGE_PAGE: u16 = 0xFFA0;
const CENTURION_USAGE: u16 = 0x0001;

const FRAME_SIZE: usize = 64;
const SOFTWARE_ID: u8 = 0x01;
const POLL_ATTEMPTS: usize = 8;

const FEATURE_ROOT: u8 = 0x00;
const FEATURE_CENTURION_BRIDGE: u16 = 0x0003;
const FEATURE_CENTURION_BATTERY_SOC: u16 = 0x0104;

const BRIDGE_SEND_FRAGMENT_FN: u8 = 0x10;
const BRIDGE_MESSAGE_EVENT_FN: u8 = 0x10;

struct CenturionSpec {
    pid: u16,
    name: &'static str,
    interface: i32,
    prefix: &'static [u8],
    bridged: bool,
    legacy_fallback: bool,
}

const CENTURION_SPECS: &[CenturionSpec] = &[
    CenturionSpec {
        pid: 0x0AF7,
        name: "Logitech G PRO X 2 LIGHTSPEED",
        interface: 3,
        prefix: &[0x51],
        bridged: true,
        legacy_fallback: true,
    },
    CenturionSpec {
        pid: 0x0B18,
        name: "Logitech G522 LIGHTSPEED",
        interface: 3,
        prefix: &[0x50, 0x23],
        bridged: true,
        legacy_fallback: false,
    },
    CenturionSpec {
        pid: 0x0B45,
        name: "Logitech G PRO X 3 LIGHTSPEED",
        interface: 3,
        prefix: &[0x50, 0x23],
        bridged: true,
        legacy_fallback: false,
    },
    CenturionSpec {
        pid: 0x0B35,
        name: "Logitech ASTRO A20 X",
        interface: 5,
        prefix: &[0x50, 0x23],
        bridged: true,
        legacy_fallback: false,
    },
    CenturionSpec {
        pid: 0x0B2E,
        name: "Logitech ASTRO A20 X (USB wired)",
        interface: 3,
        prefix: &[0x50, 0x23],
        bridged: false,
        legacy_fallback: false,
    },
];

fn centurion_spec(pid: u16) -> Option<&'static CenturionSpec> {
    CENTURION_SPECS.iter().find(|spec| spec.pid == pid)
}

pub(crate) fn providers() -> Vec<Arc<dyn Provider>> {
    vec![Arc::new(LogitechCenturionProvider)]
}

struct LogitechCenturionProvider;

enum CenturionReply {
    Data(Vec<u8>),
    Offline,
}

impl Provider for LogitechCenturionProvider {
    fn id(&self) -> &str {
        "logitech.centurion"
    }

    fn metadata(&self) -> ProviderMetadata {
        experimental_metadata(
            DeviceKind::Headset,
            Capabilities {
                percentage: true,
                charge_state: true,
                reachability: true,
                ..Capabilities::default()
            },
            Duration::from_millis(500),
        )
    }

    fn display_name(&self, endpoint: &Endpoint) -> Option<String> {
        let name = endpoint
            .pid
            .and_then(centurion_spec)
            .map_or("Logitech Centurion headset", |spec| spec.name);
        Some(name.to_owned())
    }

    fn matches(&self, endpoint: &Endpoint) -> MatchDecision {
        if endpoint.vid != Some(VENDOR_LOGITECH)
            || endpoint.usage_page != Some(CENTURION_USAGE_PAGE)
        {
            return MatchDecision::NoMatch;
        }
        let Some(spec) = endpoint.pid.and_then(centurion_spec) else {
            return MatchDecision::NoMatch;
        };
        let iface_ok = endpoint.interface_number.is_none()
            || endpoint.interface_number == Some(spec.interface);
        let usage_ok = endpoint.usage == Some(CENTURION_USAGE)
            || (endpoint.usage.is_none() && endpoint.interface_number == Some(spec.interface));
        if iface_ok && usage_ok {
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
        let spec = device
            .endpoint
            .pid
            .and_then(centurion_spec)
            .ok_or_else(|| BatteryError::Unsupported {
                reason: "unknown Logitech Centurion product ID".to_owned(),
            })?;

        if spec.bridged {
            read_bridged_battery(session, device, deadline, spec)
        } else {
            read_direct_battery(session, device, deadline, spec)
        }
    }
}

fn offline_reading() -> ProviderReading {
    ProviderReading {
        reachability: Reachability::Offline,
        source: ReadingSource::VendorHid,
        ..ProviderReading::default()
    }
}

fn online_reading(percentage: u8, charge_state: ChargeState) -> ProviderReading {
    ProviderReading {
        percentage: Some(percentage),
        charge_state,
        reachability: Reachability::Online,
        source: ReadingSource::VendorHid,
        ..ProviderReading::default()
    }
}

fn build_centurion_frame(prefix: &[u8], flags: u8, payload: &[u8]) -> Vec<u8> {
    let mut buf = vec![0_u8; FRAME_SIZE];
    let header = prefix.len();
    buf[..header].copy_from_slice(prefix);
    buf[header] = (payload.len() + 1) as u8;
    buf[header + 1] = flags;
    let max_copy = FRAME_SIZE.saturating_sub(header + 2);
    let copy_len = payload.len().min(max_copy);
    buf[header + 2..header + 2 + copy_len].copy_from_slice(&payload[..copy_len]);
    buf
}

fn extract_centurion_payload<'a>(raw: &'a [u8], prefix: &[u8]) -> Result<&'a [u8], BatteryError> {
    let header = prefix.len();
    if raw.len() < header + 3 || !raw.starts_with(prefix) {
        return Err(BatteryError::invalid_response(
            "unexpected Logitech Centurion frame prefix",
        ));
    }
    let cpl_length = usize::from(raw[header]);
    if cpl_length <= 1 || header + 1 + cpl_length > raw.len() {
        return Err(BatteryError::invalid_response(
            "invalid Logitech Centurion frame length",
        ));
    }
    Ok(&raw[header + 2..header + 1 + cpl_length])
}

fn is_bridge_connection_event(reply: &[u8], bridge_index: u8) -> bool {
    reply.len() >= 4 && reply[0] == bridge_index && reply[1] == 0x00
}

fn is_bridge_status_disconnected(params: &[u8]) -> bool {
    params.len() >= 4 && params[2] == 0x00 && params[3] == 0x00
}

#[allow(clippy::too_many_arguments)]
fn send_direct(
    session: &mut dyn HidSession,
    device: &DeviceContext<'_>,
    deadline: Deadline,
    prefix: &[u8],
    bridge_index: Option<u8>,
    feature_index: u8,
    function: u8,
    params: &[u8],
    match_function: bool,
) -> Result<CenturionReply, BatteryError> {
    let func_sw = (function & 0xF0) | SOFTWARE_ID;
    let mut payload = Vec::with_capacity(2 + params.len());
    payload.push(feature_index);
    payload.push(func_sw);
    payload.extend_from_slice(params);

    let req = build_centurion_frame(prefix, 0x00, &payload);
    send_with_modes(session, device, OUTPUT, &req)?;

    for _ in 0..POLL_ATTEMPTS {
        let timeout = deadline
            .remaining("Logitech Centurion")?
            .min(Duration::from_millis(500));
        let raw = match session.read_input(timeout) {
            Ok(report) => frame(report),
            Err(TransportError::Timeout) => {
                return Err(BatteryError::timeout("Logitech Centurion"))
            }
            Err(error) => return Err(error.into_battery(&device.endpoint.id)),
        };

        let reply = extract_centurion_payload(&raw, prefix)?;
        if reply.len() < 2 {
            continue;
        }
        if let Some(b_idx) = bridge_index {
            if is_bridge_connection_event(reply, b_idx) {
                if reply[3] == 0x00 {
                    return Ok(CenturionReply::Offline);
                }
                continue;
            }
        }
        if reply.len() >= 4 && reply[0] == 0xFF && reply[1] == feature_index && reply[2] == func_sw
        {
            return Err(BatteryError::invalid_response(format!(
                "Logitech Centurion error response (code 0x{:02X})",
                reply[3]
            )));
        }
        if reply[0] != feature_index {
            continue;
        }
        if match_function && reply[1] != func_sw {
            continue;
        }
        return Ok(CenturionReply::Data(reply[2..].to_vec()));
    }

    Err(BatteryError::timeout("Logitech Centurion"))
}

fn probe_offline_after_failure(
    session: &mut dyn HidSession,
    device: &DeviceContext<'_>,
    deadline: Deadline,
    prefix: &[u8],
    bridge_index: u8,
    original_error: BatteryError,
) -> Result<CenturionReply, BatteryError> {
    match send_direct(
        session,
        device,
        deadline,
        prefix,
        Some(bridge_index),
        bridge_index,
        0x00,
        &[],
        true,
    ) {
        Ok(CenturionReply::Offline) => Ok(CenturionReply::Offline),
        Ok(CenturionReply::Data(status)) if is_bridge_status_disconnected(&status) => {
            Ok(CenturionReply::Offline)
        }
        _ => Err(original_error),
    }
}

#[allow(clippy::too_many_arguments)]
fn send_bridged(
    session: &mut dyn HidSession,
    device: &DeviceContext<'_>,
    deadline: Deadline,
    prefix: &[u8],
    bridge_index: u8,
    sub_feature_index: u8,
    function: u8,
    params: &[u8],
) -> Result<CenturionReply, BatteryError> {
    let sub_len = 3 + params.len();
    let bridge_func_sw = BRIDGE_SEND_FRAGMENT_FN | SOFTWARE_ID;
    let mut layer3 = Vec::with_capacity(4 + sub_len);
    layer3.push(bridge_index);
    layer3.push(bridge_func_sw);
    layer3.push(((sub_len >> 8) & 0x0F) as u8);
    layer3.push((sub_len & 0xFF) as u8);
    layer3.push(0x00);
    layer3.push(sub_feature_index);
    layer3.push((function & 0xF0) | SOFTWARE_ID);
    layer3.extend_from_slice(params);

    let req = build_centurion_frame(prefix, 0x00, &layer3);
    send_with_modes(session, device, OUTPUT, &req)?;

    let mut ack_received = false;
    for _ in 0..POLL_ATTEMPTS {
        let timeout = deadline
            .remaining("Logitech Centurion bridge")?
            .min(Duration::from_millis(500));
        let raw = match session.read_input(timeout) {
            Ok(report) => frame(report),
            Err(TransportError::Timeout) => {
                let err = BatteryError::timeout("Logitech Centurion bridge");
                if ack_received {
                    return probe_offline_after_failure(
                        session,
                        device,
                        deadline,
                        prefix,
                        bridge_index,
                        err,
                    );
                }
                return Err(err);
            }
            Err(error) => return Err(error.into_battery(&device.endpoint.id)),
        };

        let reply = extract_centurion_payload(&raw, prefix)?;
        if reply.len() >= 4
            && reply[0] == 0xFF
            && reply[1] == bridge_index
            && reply[2] == bridge_func_sw
        {
            return probe_offline_after_failure(
                session,
                device,
                deadline,
                prefix,
                bridge_index,
                BatteryError::invalid_response("Logitech Centurion bridge rejected request"),
            );
        }
        if reply.len() < 2 || reply[0] != bridge_index {
            continue;
        }
        if is_bridge_connection_event(reply, bridge_index) {
            if reply[3] == 0x00 {
                return Ok(CenturionReply::Offline);
            }
            continue;
        }
        let func_sw = reply[1];
        if (func_sw >> 4) != (BRIDGE_MESSAGE_EVENT_FN >> 4) {
            continue;
        }
        if (func_sw & 0x0F) == SOFTWARE_ID {
            ack_received = true;
            continue;
        }
        if (func_sw & 0x0F) != 0x00 {
            continue;
        }
        if reply.len() < 6 || reply[4] != 0x00 {
            continue;
        }
        let sub_feature = reply[5];
        let matches_sub = sub_feature == sub_feature_index
            || (sub_feature == 0xFF && reply.len() >= 7 && reply[6] == sub_feature_index);
        if !matches_sub {
            continue;
        }
        if reply.len() < 7 {
            return Err(BatteryError::invalid_response(
                "malformed Logitech Centurion bridge response",
            ));
        }
        if sub_feature == 0xFF {
            return Err(BatteryError::invalid_response(
                "Logitech Centurion sub-device rejected request",
            ));
        }
        return Ok(CenturionReply::Data(reply[7..].to_vec()));
    }

    if !ack_received {
        return Err(BatteryError::timeout(
            "Logitech Centurion bridge acknowledgment",
        ));
    }
    probe_offline_after_failure(
        session,
        device,
        deadline,
        prefix,
        bridge_index,
        BatteryError::timeout("Logitech Centurion bridge response"),
    )
}

fn parse_centurion_battery(data: &[u8]) -> Result<ProviderReading, BatteryError> {
    if data.is_empty() {
        return Err(BatteryError::invalid_response(
            "empty Logitech Centurion battery response",
        ));
    }
    let pct = checked_percentage(data[0])?;
    let charge = match data.get(2).copied().unwrap_or(0) {
        1 | 2 => ChargeState::Charging,
        0 => ChargeState::Discharging,
        _ => ChargeState::Unknown,
    };
    Ok(online_reading(pct, charge))
}

fn read_direct_battery(
    session: &mut dyn HidSession,
    device: &DeviceContext<'_>,
    deadline: Deadline,
    spec: &CenturionSpec,
) -> Result<ProviderReading, BatteryError> {
    let lookup = send_direct(
        session,
        device,
        deadline,
        spec.prefix,
        None,
        FEATURE_ROOT,
        0x00,
        &FEATURE_CENTURION_BATTERY_SOC.to_be_bytes(),
        true,
    )?;
    let battery_index = match lookup {
        CenturionReply::Offline => return Ok(offline_reading()),
        CenturionReply::Data(data) if data.first().copied().unwrap_or(0) != 0 => data[0],
        CenturionReply::Data(_) => {
            return Err(BatteryError::Unsupported {
                reason: "Logitech Centurion battery feature not available".to_owned(),
            });
        }
    };
    match send_direct(
        session,
        device,
        deadline,
        spec.prefix,
        None,
        battery_index,
        0x00,
        &[],
        true,
    )? {
        CenturionReply::Offline => Ok(offline_reading()),
        CenturionReply::Data(data) => parse_centurion_battery(&data),
    }
}

fn read_bridged_battery(
    session: &mut dyn HidSession,
    device: &DeviceContext<'_>,
    deadline: Deadline,
    spec: &CenturionSpec,
) -> Result<ProviderReading, BatteryError> {
    let bridge_lookup = send_direct(
        session,
        device,
        deadline,
        spec.prefix,
        None,
        FEATURE_ROOT,
        0x00,
        &FEATURE_CENTURION_BRIDGE.to_be_bytes(),
        false,
    )?;
    let bridge_index = match bridge_lookup {
        CenturionReply::Offline => return Ok(offline_reading()),
        CenturionReply::Data(data) if data.first().copied().unwrap_or(0) != 0 => data[0],
        CenturionReply::Data(_) => {
            return Err(BatteryError::invalid_response(
                "Logitech Centurion bridge feature not found",
            ));
        }
    };

    let battery_lookup = send_bridged(
        session,
        device,
        deadline,
        spec.prefix,
        bridge_index,
        FEATURE_ROOT,
        0x00,
        &FEATURE_CENTURION_BATTERY_SOC.to_be_bytes(),
    )?;
    let battery_index = match battery_lookup {
        CenturionReply::Offline => return Ok(offline_reading()),
        CenturionReply::Data(data) if data.first().copied().unwrap_or(0) != 0 => data[0],
        CenturionReply::Data(_) => {
            if spec.legacy_fallback {
                return read_legacy_gpro_x2(session, device, deadline);
            }
            return Err(BatteryError::Unsupported {
                reason: "Logitech Centurion battery feature not available".to_owned(),
            });
        }
    };

    match send_bridged(
        session,
        device,
        deadline,
        spec.prefix,
        bridge_index,
        battery_index,
        0x00,
        &[],
    )? {
        CenturionReply::Offline => Ok(offline_reading()),
        CenturionReply::Data(data) => parse_centurion_battery(&data),
    }
}

fn read_legacy_gpro_x2(
    session: &mut dyn HidSession,
    device: &DeviceContext<'_>,
    deadline: Deadline,
) -> Result<ProviderReading, BatteryError> {
    let mut req = vec![0_u8; FRAME_SIZE];
    req[0] = 0x51;
    req[1] = 0x08;
    req[3] = 0x03;
    req[4] = 0x1A;
    req[6] = 0x03;
    req[8] = 0x04;
    req[9] = 0x0A;
    send_with_modes(session, device, OUTPUT, &req)?;

    for _ in 0..4 {
        let timeout = deadline
            .remaining("Logitech G PRO X 2 legacy battery")?
            .min(Duration::from_millis(500));
        let raw = match session.read_input(timeout) {
            Ok(report) => frame(report),
            Err(TransportError::Timeout) => {
                return Err(BatteryError::timeout("Logitech G PRO X 2 legacy battery"));
            }
            Err(error) => return Err(error.into_battery(&device.endpoint.id)),
        };

        if raw.len() >= 7 && raw[0] == 0x51 && raw[1] == 0x05 && raw[6] == 0x00 {
            return Ok(offline_reading());
        }
        if raw.len() >= 2 && raw[0] == 0x51 && matches!(raw[1], 0x03 | 0x05) {
            continue;
        }
        if raw.len() >= 13 && raw[0] == 0x51 && raw[1] == 0x0B && raw[8] == 0x04 {
            let pct = checked_percentage(raw[10])?;
            let charge = if raw[12] == 0x02 {
                ChargeState::Charging
            } else {
                ChargeState::Discharging
            };
            return Ok(online_reading(pct, charge));
        }
    }

    Err(BatteryError::invalid_response(
        "Logitech G PRO X 2 legacy battery packet not received",
    ))
}
