//! MCHOSE keyboard and V9 Pro 2 headset battery protocols.

use super::mchose_mice::read_mchose_modern;
use crate::error::BatteryError;
use crate::model::{
    ChargeState, DeviceKind, Endpoint, ProviderReading, Reachability, ReadingSource, SupportStatus,
};
use crate::providers::fixed::{
    capabilities, checked_percentage, checksum_sum, drain_input, experimental_metadata, reading,
    send_with_modes, unnumbered_payload, wait_frame, FixedProvider, FixedSpec, OUTPUT, SET_OUTPUT,
};
use crate::providers::{Deadline, DeviceContext, MatchDecision, Provider, ProviderMetadata};
use crate::transport::HidSession;
use std::sync::Arc;
use std::time::Duration;

static MCHOSE_KEYBOARD: FixedSpec = FixedSpec {
    id: "mchose.keyboard",
    name: "MCHOSE keyboard",
    kind: DeviceKind::Keyboard,
    status: SupportStatus::Experimental,
    matches: match_mchose_keyboard,
    query: mchose_keyboard_query,
    modes: SET_OUTPUT,
    attempts: 3,
    max_reports: 12,
    io_timeout: Duration::from_millis(450),
    capabilities: capabilities(true, true, false),
    parse: parse_mchose_keyboard,
};

pub(crate) fn providers() -> Vec<Arc<dyn Provider>> {
    vec![
        Arc::new(FixedProvider::new(&MCHOSE_KEYBOARD)),
        Arc::new(MchoseV9Pro2Provider),
        Arc::new(MchoseGlwKeyboardProvider),
        Arc::new(MchoseK87V3Provider),
    ]
}

fn match_mchose_keyboard(endpoint: &Endpoint) -> bool {
    let by_vid_pid = (endpoint.vid == Some(0x3554)
        && matches!(endpoint.pid, Some(0xFA09 | 0xFB40)))
        || (endpoint.vid == Some(0x41E4) && endpoint.pid == Some(0x2001))
        || (endpoint.vid == Some(0x258A) && endpoint.pid == Some(0x01BB))
        || (endpoint.vid == Some(0x05AC)
            && endpoint.pid == Some(0x024F)
            && endpoint.interface_number == Some(1));
    by_vid_pid && endpoint.usage_page == Some(0xFF02)
}

fn mchose_keyboard_query() -> Vec<u8> {
    let mut frame = vec![0_u8; 20];
    frame[0] = 0x13;
    frame[1] = 0x4A;
    frame[2] = 1;
    frame[19] = checksum_sum(&frame[..19]);
    frame
}

fn parse_mchose_keyboard(frame: &[u8]) -> Result<Option<ProviderReading>, BatteryError> {
    if frame.len() < 2 || frame[0] != 0x13 || frame[1] != 0x4A {
        return Ok(None);
    }
    if frame.len() == 20 && frame[4] == 0 && checksum_sum(&frame[..19]) == frame[19] {
        return Ok(None);
    }
    if frame.len() != 20 || frame[2..5] != [1, 0, 2] {
        return Err(BatteryError::invalid_response(
            "invalid MCHOSE keyboard response",
        ));
    }
    if checksum_sum(&frame[..19]) != frame[19] {
        return Err(BatteryError::ChecksumFailed);
    }
    let charge = match frame[6] >> 4 {
        0 => ChargeState::Discharging,
        1 => ChargeState::Charging,
        _ => ChargeState::Unknown,
    };
    Ok(Some(reading(
        Some(checked_percentage(frame[5])?),
        charge,
        None,
    )))
}

struct MchoseV9Pro2Provider;

impl Provider for MchoseV9Pro2Provider {
    fn id(&self) -> &str {
        "mchose.v9_pro2"
    }

    fn metadata(&self) -> ProviderMetadata {
        experimental_metadata(
            DeviceKind::Headset,
            capabilities(true, true, false),
            Duration::from_millis(500),
        )
    }

    fn display_name(&self, endpoint: &Endpoint) -> Option<String> {
        Some(
            endpoint
                .product_name
                .clone()
                .filter(|name| !name.is_empty())
                .unwrap_or_else(|| "MCHOSE V9 Pro 2".to_owned()),
        )
    }

    fn matches(&self, endpoint: &Endpoint) -> MatchDecision {
        if endpoint.vid == Some(0x3837)
            && endpoint.pid == Some(0x6011)
            && endpoint.usage_page == Some(0xFF07)
            && endpoint.usage == Some(0x0222)
        {
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
        drain_input(session, device, deadline, 8)?;
        let mut request = vec![0_u8; 64];
        request[..8].copy_from_slice(&[0x55, 0xAA, 0x01, 0x00, 0x01, 0x00, 0x01, 0x0B]);
        send_with_modes(session, device, OUTPUT, &request)?;
        let frame = wait_frame(
            session,
            device,
            deadline,
            Duration::from_millis(500),
            16,
            |r| {
                if r.len() < 9 || r[0] != 0x55 {
                    return Ok(false);
                }
                let f = &r[1..];
                if f[0] != 0xAA || f[1] != 0x01 || f[4] != 0x00 {
                    return Ok(false);
                }
                let is_reply = (f[5] == 0x01 && f[6] == 0x0B) || (f[5] == 0x04 && f[6] == 0x22);
                if !is_reply {
                    return Ok(false);
                }
                let len = usize::from(f[3]);
                if len < 3 || 6 + len >= f.len() {
                    return Err(BatteryError::invalid_response(
                        "truncated MCHOSE V9 Pro 2 response",
                    ));
                }
                let expected = if f[2] == 0x01 {
                    f[2] ^ f[3] ^ f[4] ^ f[5] ^ f[6..6 + len].iter().fold(0_u8, |a, b| a ^ b)
                } else {
                    0
                };
                if f[6 + len] != expected {
                    return Err(BatteryError::ChecksumFailed);
                }
                Ok(true)
            },
        )?;
        let body = &frame[8..];
        if body[0] == 0x00 {
            return Ok(ProviderReading {
                reachability: Reachability::Sleeping,
                source: ReadingSource::VendorHid,
                ..ProviderReading::default()
            });
        }
        let pct = checked_percentage(body[1] & 0x7F)?;
        let charge = if (body[1] & 0x80) != 0 {
            ChargeState::Charging
        } else {
            ChargeState::Discharging
        };
        Ok(reading(Some(pct), charge, None))
    }
}

struct MchoseGlwKeyboardProvider;

impl Provider for MchoseGlwKeyboardProvider {
    fn id(&self) -> &str {
        "mchose.glw_keyboard"
    }

    fn metadata(&self) -> ProviderMetadata {
        experimental_metadata(
            DeviceKind::Keyboard,
            capabilities(true, true, false),
            Duration::from_millis(500),
        )
    }

    fn display_name(&self, endpoint: &Endpoint) -> Option<String> {
        Some(
            endpoint
                .product_name
                .clone()
                .filter(|name| !name.is_empty())
                .unwrap_or_else(|| "MCHOSE keyboard".to_owned()),
        )
    }

    fn matches(&self, endpoint: &Endpoint) -> MatchDecision {
        let coll = (endpoint.usage_page == Some(0x0001) && endpoint.usage == Some(0x0000))
            || (endpoint.usage_page == Some(0xFF31) && endpoint.usage == Some(0x0074));
        if endpoint.vid == Some(0x3837) && endpoint.pid == Some(0x3033) && coll {
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
        drain_input(session, device, deadline, 8)?;
        let mut request = vec![0_u8; 65];
        request[1..9].copy_from_slice(&[0x55, 0x05, 0x00, 0x38, 0x38, 0x00, 0x00, 0x00]);
        send_with_modes(session, device, OUTPUT, &request)?;
        let frame = wait_frame(
            session,
            device,
            deadline,
            Duration::from_millis(500),
            16,
            |frame| {
                let r = unnumbered_payload(frame);
                Ok(r.len() >= 41 && r[0] == 0xAA && r[1] == 0x05 && r[5] == 0x00 && r[6] == 0x00)
            },
        )?;
        let r = unnumbered_payload(&frame);
        let size = usize::from(r[4]);
        if size < 33 || 8 + size > r.len() {
            return Err(BatteryError::invalid_response(
                "invalid MCHOSE GLW keyboard payload size",
            ));
        }
        let expected = checksum_sum(&r[4..8 + size]);
        if r[3] != expected {
            return Err(BatteryError::ChecksumFailed);
        }
        let pct = checked_percentage(r[40] & 0x7F)?;
        let charge = if (r[40] & 0x80) != 0 {
            ChargeState::Charging
        } else {
            ChargeState::Discharging
        };
        Ok(reading(Some(pct), charge, None))
    }
}

struct MchoseK87V3Provider;

impl Provider for MchoseK87V3Provider {
    fn id(&self) -> &str {
        "mchose.k87_v3"
    }

    fn metadata(&self) -> ProviderMetadata {
        experimental_metadata(
            DeviceKind::Keyboard,
            capabilities(true, true, false),
            Duration::from_millis(250),
        )
    }

    fn display_name(&self, endpoint: &Endpoint) -> Option<String> {
        Some(
            endpoint
                .product_name
                .clone()
                .filter(|name| !name.is_empty())
                .unwrap_or_else(|| "MCHOSE K87 V3".to_owned()),
        )
    }

    fn matches(&self, endpoint: &Endpoint) -> MatchDecision {
        if endpoint.vid == Some(0x3837)
            && endpoint.pid == Some(0x1015)
            && endpoint.usage_page == Some(0xFF80)
            && endpoint.usage == Some(0x0001)
        {
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
        read_mchose_modern(device, session, deadline, true)
    }
}
