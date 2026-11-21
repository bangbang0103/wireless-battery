//! Single-model vendor HID protocols (TAIDU, CROCIRIS X7, XIBERIA T7, Rapoo VT3S, WOB Rainy75).

use crate::error::BatteryError;
use crate::model::{ChargeState, Endpoint, ProviderReading, SupportStatus};
use crate::providers::fixed::{
    capabilities, charge_bool, checked_percentage, crc16_poly_1021, drain_input,
    experimental_metadata, reading, send_with_modes, unnumbered_payload, wait_frame, FixedProvider,
    FixedSpec, OUTPUT, OUTPUT_THEN_SET, SET_OUTPUT,
};
use crate::providers::{Deadline, DeviceContext, MatchDecision, Provider, ProviderMetadata};
use crate::transport::HidSession;
use std::sync::Arc;
use std::time::Duration;

static TAIDU: FixedSpec = FixedSpec {
    id: "taidu.mouse",
    name: "TAIDU mouse",
    status: SupportStatus::Experimental,
    matches: match_taidu,
    query: taidu_query,
    modes: OUTPUT_THEN_SET,
    attempts: 3,
    max_reports: 12,
    io_timeout: Duration::from_millis(450),
    capabilities: capabilities(true, true, false),
    parse: parse_taidu,
};

static X7: FixedSpec = FixedSpec {
    id: "crociris.x7",
    name: "CROCIRIS X7 Receiver 8K",
    status: SupportStatus::Experimental,
    matches: match_x7,
    query: x7_query,
    modes: OUTPUT,
    attempts: 2,
    max_reports: 10,
    io_timeout: Duration::from_millis(200),
    capabilities: capabilities(true, true, false),
    parse: parse_x7,
};

static XIBERIA_T7: FixedSpec = FixedSpec {
    id: "xiberia.t7",
    name: "XIBERIA T7",
    status: SupportStatus::Experimental,
    matches: match_xiberia,
    query: xiberia_query,
    modes: OUTPUT,
    attempts: 2,
    max_reports: 10,
    io_timeout: Duration::from_millis(200),
    capabilities: capabilities(true, false, false),
    parse: parse_xiberia,
};

static WOB_RAINY75: FixedSpec = FixedSpec {
    id: "wob.rainy75",
    name: "WOB Rainy75",
    status: SupportStatus::Verified,
    matches: match_wob,
    query: wob_query,
    modes: OUTPUT,
    attempts: 3,
    max_reports: 24,
    io_timeout: Duration::from_millis(1_200),
    capabilities: capabilities(true, true, false),
    parse: parse_wob,
};

pub(crate) fn providers() -> Vec<Arc<dyn Provider>> {
    vec![
        Arc::new(FixedProvider::new(&TAIDU)),
        Arc::new(FixedProvider::new(&X7)),
        Arc::new(FixedProvider::new(&XIBERIA_T7)),
        Arc::new(FixedProvider::new(&WOB_RAINY75)),
        Arc::new(RapooProvider),
        Arc::new(RapooVtNordicProvider),
        Arc::new(RapooLegacyProvider),
        Arc::new(PressPlayIcarusProvider),
        Arc::new(MaonoPd200wProvider),
        Arc::new(DarkrimShadowProvider),
    ]
}

fn match_taidu(endpoint: &Endpoint) -> bool {
    matches!(endpoint.vid, Some(0xA8A5) | Some(0xA8A4) | Some(0x3A37))
        && endpoint.usage_page == Some(0xFF01)
        && endpoint.usage == Some(0x10)
}

fn taidu_query() -> Vec<u8> {
    let mut frame = vec![0_u8; 65];
    frame[1] = 0x55;
    frame[2] = 0x30;
    frame[3..9].copy_from_slice(&[0xA5, 0x0B, 0x2E, 1, 1, 1]);
    frame
}

fn parse_taidu(frame: &[u8]) -> Result<Option<ProviderReading>, BatteryError> {
    if frame.len() < 3 || frame[1] != 0xAA || frame[2] != 0x30 {
        return Ok(None);
    }
    if frame.len() != 65 {
        return Err(BatteryError::invalid_response(
            "invalid TAIDU response length",
        ));
    }
    Ok(Some(reading(
        Some(checked_percentage(frame[9])?),
        charge_bool(frame[10]),
        None,
    )))
}

fn match_x7(endpoint: &Endpoint) -> bool {
    endpoint.vid == Some(0x36F9)
        && endpoint.pid == Some(0xAA20)
        && endpoint.usage_page == Some(0xFF0A)
}

fn x7_query() -> Vec<u8> {
    let mut frame = vec![0_u8; 65];
    frame[1] = 0xAA;
    frame[2] = 0x12;
    frame
}

fn parse_x7(frame: &[u8]) -> Result<Option<ProviderReading>, BatteryError> {
    if frame.len() < 3 || frame[1] != 0x55 || frame[2] != 0x12 {
        return Ok(None);
    }
    if frame.len() < 7 {
        return Err(BatteryError::invalid_response("truncated X7 response"));
    }
    Ok(Some(reading(
        Some(checked_percentage(frame[6])?),
        charge_bool(frame[5]),
        None,
    )))
}

fn match_xiberia(endpoint: &Endpoint) -> bool {
    endpoint.vid == Some(0x36F9) && endpoint.usage_page == Some(0xFF00)
}

fn xiberia_query() -> Vec<u8> {
    let mut frame = vec![0_u8; 64];
    frame[0] = 0x55;
    frame[1] = 0x65;
    frame
}

fn parse_xiberia(frame: &[u8]) -> Result<Option<ProviderReading>, BatteryError> {
    if frame.len() < 2 || frame[0] != 0x55 || frame[1] != 0x65 {
        return Ok(None);
    }
    if frame.len() < 4 {
        return Err(BatteryError::invalid_response("truncated XIBERIA response"));
    }
    Ok(Some(reading(
        Some(checked_percentage(frame[3])?),
        ChargeState::Unknown,
        None,
    )))
}

fn match_wob(endpoint: &Endpoint) -> bool {
    endpoint.vid == Some(0x320F)
        && endpoint.pid == Some(0x5088)
        && endpoint.usage_page == Some(0xFF1C)
        && endpoint.usage == Some(0x0092)
}

fn wob_query() -> Vec<u8> {
    let mut frame = vec![0_u8; 64];
    frame[0] = 0x04;
    frame[1] = 0x20;
    frame[3] = 0x1A;
    frame[4] = 0x06;
    frame
}

fn parse_wob(frame: &[u8]) -> Result<Option<ProviderReading>, BatteryError> {
    if frame.len() < 5 || frame[..5] != [0x04, 0x20, 0x00, 0x1A, 0x06] {
        return Ok(None);
    }
    if frame.len() != 64 {
        return Err(BatteryError::invalid_response(
            "invalid WOB response length",
        ));
    }
    if frame[5..].iter().all(|byte| *byte == 0) {
        return Ok(None);
    }
    Ok(Some(reading(
        Some(checked_percentage(frame[8])?),
        charge_bool(frame[9]),
        None,
    )))
}

struct RapooProvider;

impl Provider for RapooProvider {
    fn id(&self) -> &str {
        "rapoo.vt3s"
    }

    fn metadata(&self) -> ProviderMetadata {
        experimental_metadata(
            capabilities(true, true, false),
            Duration::from_millis(4_500),
        )
    }

    fn display_name(&self, _endpoint: &Endpoint) -> Option<String> {
        Some("Rapoo VT3S".to_owned())
    }

    fn matches(&self, endpoint: &Endpoint) -> MatchDecision {
        if endpoint.vid == Some(0x24AE)
            && endpoint.pid == Some(0x1411)
            && endpoint.usage_page == Some(0xFF00)
            && endpoint.usage == Some(2)
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
        let frame = wait_frame(
            session,
            device,
            deadline,
            Duration::from_millis(4_500),
            32,
            |frame| {
                if frame.len() < 2 || frame[0] != 0x07 {
                    return Ok(false);
                }
                if frame.len() <= 8 {
                    return Err(BatteryError::invalid_response("truncated Rapoo report"));
                }
                checked_percentage(frame[8])?;
                Ok(true)
            },
        )?;
        let charge = match frame[7] {
            1 => ChargeState::Discharging,
            2 | 3 => ChargeState::Charging,
            _ => ChargeState::Unknown,
        };
        Ok(reading(Some(frame[8]), charge, None))
    }
}

const RAPOO_LEGACY_PIDS: &[u16] = &[0x1215, 0x4415, 0x1233, 0x4433, 0x1244];

const RAPOO_NORDIC_PIDS: &[u16] = &[
    0x4610, 0x1410, 0x4620, 0x1420, 0x4615, 0x1415, 0x4644, 0x1444, 0x4622, 0x1422, 0x4623, 0x1423,
    0x4612, 0x1412, 0x4617, 0x1417, 0x4B06, 0x1B06, 0x4611, 0x4621, 0x1421, 0x4613, 0x1413, 0x4616,
    0x1416, 0x4614, 0x1414, 0x4618, 0x1418,
];

struct RapooVtNordicProvider;

impl Provider for RapooVtNordicProvider {
    fn id(&self) -> &str {
        "rapoo.vt"
    }

    fn metadata(&self) -> ProviderMetadata {
        experimental_metadata(
            capabilities(true, true, false),
            Duration::from_millis(4_500),
        )
    }

    fn display_name(&self, endpoint: &Endpoint) -> Option<String> {
        let name = match endpoint.pid {
            Some(0x4610 | 0x1410) => "Rapoo VT0",
            Some(0x4620 | 0x1420) => "Rapoo VT0 Max",
            Some(0x4615 | 0x1415) => "Rapoo VT1",
            Some(0x4644 | 0x1444) => "Rapoo VT1 Max",
            Some(0x4622 | 0x1422) => "Rapoo VT2",
            Some(0x4623 | 0x1423) => "Rapoo VT2 Max",
            Some(0x4612 | 0x1412) => "Rapoo VT3",
            Some(0x4617 | 0x1417) => "Rapoo VT3 Max Gen 2",
            Some(0x4B06 | 0x1B06) => "Rapoo VT3 Max V2",
            Some(0x4611) => "Rapoo VT3S",
            Some(0x4621 | 0x1421) => "Rapoo VT3S Max",
            Some(0x4613 | 0x1413) => "Rapoo VT7",
            Some(0x4616 | 0x1416) => "Rapoo VT7 Max",
            Some(0x4614 | 0x1414) => "Rapoo VT9 Gen 2",
            Some(0x4618 | 0x1418) => "Rapoo VT9 Max",
            _ => "Rapoo VT mouse",
        };
        Some(name.to_owned())
    }

    fn matches(&self, endpoint: &Endpoint) -> MatchDecision {
        if endpoint.vid == Some(0x24AE)
            && endpoint.pid.is_some_and(|p| RAPOO_NORDIC_PIDS.contains(&p))
            && endpoint.usage_page == Some(0xFF00)
            && endpoint.usage == Some(2)
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
        let frame = wait_frame(
            session,
            device,
            deadline,
            Duration::from_millis(4_500),
            32,
            |frame| {
                Ok(frame.len() >= 9
                    && frame[0] == 0x07
                    && (frame[1] & 0x0F) <= 2
                    && matches!(frame[7], 1 | 2))
            },
        )?;
        let pct = checked_percentage(frame[8])?;
        let charge = match frame[7] {
            1 => ChargeState::Discharging,
            2 => ChargeState::Charging,
            _ => ChargeState::Unknown,
        };
        Ok(reading(Some(pct), charge, None))
    }
}

struct RapooLegacyProvider;

impl Provider for RapooLegacyProvider {
    fn id(&self) -> &str {
        "rapoo.legacy_vt3_pro"
    }

    fn metadata(&self) -> ProviderMetadata {
        experimental_metadata(
            capabilities(true, true, false),
            Duration::from_millis(4_500),
        )
    }

    fn display_name(&self, endpoint: &Endpoint) -> Option<String> {
        Some(match endpoint.pid {
            Some(0x1233 | 0x4433) => "Rapoo VT1 Pro Max".to_owned(),
            Some(0x1244) => "Rapoo VT3 Pro Max".to_owned(),
            _ => "Rapoo VT3 Pro".to_owned(),
        })
    }

    fn matches(&self, endpoint: &Endpoint) -> MatchDecision {
        if endpoint.vid == Some(0x24AE)
            && endpoint.pid.is_some_and(|p| RAPOO_LEGACY_PIDS.contains(&p))
            && endpoint.usage_page == Some(0xFF00)
            && endpoint.usage == Some(2)
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
        let frame = wait_frame(
            session,
            device,
            deadline,
            Duration::from_millis(4_500),
            32,
            |frame| {
                Ok(frame.len() >= 7
                    && frame[0] == 0xBB
                    && frame[1] == 0xB0
                    && matches!(frame[5], 1 | 2))
            },
        )?;
        if frame[5] == 2 && frame[6] == 0x7F {
            return Ok(reading(None, ChargeState::Charging, None));
        }
        let pct = checked_percentage(frame[6])?;
        let charge = match frame[5] {
            1 => ChargeState::Discharging,
            2 => ChargeState::Charging,
            _ => ChargeState::Unknown,
        };
        Ok(reading(Some(pct), charge, None))
    }
}

struct PressPlayIcarusProvider;

impl Provider for PressPlayIcarusProvider {
    fn id(&self) -> &str {
        "pressplay.icarus"
    }

    fn metadata(&self) -> ProviderMetadata {
        experimental_metadata(capabilities(true, false, false), Duration::from_millis(500))
    }

    fn display_name(&self, _endpoint: &Endpoint) -> Option<String> {
        Some("Press Play ICARUS V2 Nearlink".to_owned())
    }

    fn matches(&self, endpoint: &Endpoint) -> MatchDecision {
        if endpoint.vid == Some(0x7768)
            && endpoint.pid == Some(0x6434)
            && endpoint.usage_page == Some(0xFFB2)
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
        drain_input(session, device, deadline, 6)?;
        let mut req = vec![0_u8; 64];
        req[0] = 0x06;
        req[1] = 0x05;
        send_with_modes(session, device, SET_OUTPUT, &req)?;
        let r = wait_frame(
            session,
            device,
            deadline,
            Duration::from_millis(500),
            12,
            |r| Ok(r.len() >= 14 && r[0] == 0x06 && r[1] == 0x05 && r[2] == 0x26),
        )?;
        Ok(reading(
            Some(checked_percentage(r[13])?),
            ChargeState::Unknown,
            None,
        ))
    }
}

struct MaonoPd200wProvider;

impl Provider for MaonoPd200wProvider {
    fn id(&self) -> &str {
        "maono.pd200w"
    }

    fn metadata(&self) -> ProviderMetadata {
        experimental_metadata(capabilities(true, false, false), Duration::from_millis(500))
    }

    fn display_name(&self, _endpoint: &Endpoint) -> Option<String> {
        Some("MAONO PD200W".to_owned())
    }

    fn matches(&self, endpoint: &Endpoint) -> MatchDecision {
        if endpoint.vid == Some(0x352F)
            && endpoint.pid == Some(0x040E)
            && endpoint.interface_number == Some(3)
            && endpoint.usage_page == Some(0x000C)
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
        drain_input(session, device, deadline, 6)?;
        let mut req = vec![0_u8; 64];
        req[..9].copy_from_slice(&[0xC4, 0x09, 0x00, 0x00, 0x04, 0x42, 0x00, 0xED, 0xFE]);
        send_with_modes(session, device, OUTPUT, &req)?;
        let r = wait_frame(
            session,
            device,
            deadline,
            Duration::from_millis(500),
            16,
            |r| Ok(r.len() >= 11 && r[0] == 0xC4 && r[2] == 0 && r[3] == 0 && r[4] == 0x04),
        )?;
        let len = usize::from(r[1]);
        if len < 11 || len > r.len() || (len - 7) % 4 != 0 {
            return Err(BatteryError::invalid_response(
                "invalid MAONO PD200W length",
            ));
        }
        let body_end = len - 2;
        let sum: u32 = r[..body_end].iter().map(|&b| u32::from(b)).sum();
        let expected = ((0x10000_u32.wrapping_sub(sum & 0xFFFF)) & 0xFFFF) as u16;
        if u16::from_le_bytes([r[body_end], r[body_end + 1]]) != expected {
            return Err(BatteryError::ChecksumFailed);
        }
        let mut off = 5;
        while off < body_end {
            if u16::from_le_bytes([r[off], r[off + 1]]) == 0x0042 {
                let val = u16::from_le_bytes([r[off + 2], r[off + 3]]);
                if val > 100 {
                    return Err(BatteryError::invalid_response(
                        "MAONO PD200W percentage exceeds 100",
                    ));
                }
                return Ok(reading(Some(val as u8), ChargeState::Unknown, None));
            }
            off += 4;
        }
        Err(BatteryError::invalid_response(
            "MAONO PD200W battery register missing",
        ))
    }
}

struct DarkrimShadowProvider;

impl Provider for DarkrimShadowProvider {
    fn id(&self) -> &str {
        "darkrim.shadow"
    }

    fn metadata(&self) -> ProviderMetadata {
        experimental_metadata(capabilities(true, true, false), Duration::from_millis(500))
    }

    fn display_name(&self, _endpoint: &Endpoint) -> Option<String> {
        Some("DARKRIM SHADOW".to_owned())
    }

    fn matches(&self, endpoint: &Endpoint) -> MatchDecision {
        if endpoint.vid == Some(0x3654)
            && endpoint.pid == Some(0x6901)
            && endpoint.usage_page == Some(0xFF02)
            && endpoint.usage == Some(0x0002)
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
        drain_input(session, device, deadline, 6)?;
        let seq = 1_u8;
        let body = [
            0xFE, seq, 0x10, 0x00, 0x01, 0x00, 0x01, 0x02, 0x00, 0x01, 0x00,
        ];
        let crc = crc16_poly_1021(&body, 0xFFFF);
        let mut req = vec![0_u8; 65];
        req[1..12].copy_from_slice(&body);
        req[12] = (crc >> 8) as u8;
        req[13] = (crc & 0xFF) as u8;
        send_with_modes(session, device, OUTPUT, &req)?;
        let frame = wait_frame(
            session,
            device,
            deadline,
            Duration::from_millis(500),
            12,
            |f| {
                let r = unnumbered_payload(f);
                Ok(r.len() >= 11
                    && r[0] == 0xFE
                    && r[1] == seq
                    && r[2..6] == [0x10, 0x00, 0x02, 0x00]
                    && r[6] <= 1)
            },
        )?;
        let r = unnumbered_payload(&frame);
        let len = usize::from(u16::from_le_bytes([r[7], r[8]]));
        if len > 53 || len + 11 > r.len() {
            return Err(BatteryError::invalid_response(
                "invalid DARKRIM payload length",
            ));
        }
        let expected = crc16_poly_1021(&r[..9 + len], 0xFFFF);
        let got = u16::from_be_bytes([r[9 + len], r[10 + len]]);
        if got != expected {
            return Err(BatteryError::ChecksumFailed);
        }
        let tlv = &r[9..9 + len];
        let mut i = 0;
        while i + 2 <= tlv.len() {
            let t = tlv[i];
            let l = usize::from(tlv[i + 1]);
            if i + 2 + l > tlv.len() {
                break;
            }
            if t == 0x01 && l == 1 {
                let val = tlv[i + 2];
                let pct = checked_percentage(val & 0x7F)?;
                let charge = if (val & 0x80) != 0 {
                    ChargeState::Charging
                } else {
                    ChargeState::Discharging
                };
                return Ok(reading(Some(pct), charge, None));
            }
            i += 2 + l;
        }
        Err(BatteryError::invalid_response(
            "DARKRIM battery TLV missing",
        ))
    }
}
