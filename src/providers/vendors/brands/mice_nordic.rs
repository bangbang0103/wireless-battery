//! Wireless mouse protocols: ATK (Zero, NearLink, Nano 8K), Finalmouse, Scyrox,
//! RAWM/SADES, and ZOWIE EC-CW.

use crate::error::BatteryError;
use crate::model::{
    ChargeState, DeviceKind, Endpoint, ProviderReading, Reachability, ReadingSource,
};
use crate::providers::fixed::{
    capabilities, charge_bool, checked_percentage, checksum_sum, drain_input,
    experimental_metadata, interpolate_mv, reading, send_with_modes, unnumbered_payload,
    wait_frame, SendMode, OUTPUT,
};
use crate::providers::{Deadline, DeviceContext, MatchDecision, Provider, ProviderMetadata};
use crate::transport::HidSession;
use std::sync::Arc;
use std::time::Duration;

pub(crate) fn providers() -> Vec<Arc<dyn Provider>> {
    vec![
        Arc::new(AtkZeroProvider),
        Arc::new(AtkBitmouseNkProvider),
        Arc::new(AtkNano8kProvider),
        Arc::new(FinalmouseProvider),
        Arc::new(ScyroxProvider),
        Arc::new(RawmProvider),
        Arc::new(ZowieCwProvider),
    ]
}

struct AtkZeroProvider;

impl Provider for AtkZeroProvider {
    fn id(&self) -> &str {
        "atk.zero"
    }

    fn metadata(&self) -> ProviderMetadata {
        experimental_metadata(
            DeviceKind::Mouse,
            capabilities(true, true, false),
            Duration::from_millis(500),
        )
    }

    fn display_name(&self, _endpoint: &Endpoint) -> Option<String> {
        Some("ATK Zero".to_owned())
    }

    fn matches(&self, endpoint: &Endpoint) -> MatchDecision {
        if endpoint.vid == Some(0x373B)
            && matches!(endpoint.pid, Some(0x1154 | 0x124F | 0x1155))
            && endpoint.usage_page == Some(0xFF05)
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
        let cable = device.endpoint.pid == Some(0x1154);
        let target = if cable { 0x00 } else { 0x01 };
        let ask = |session: &mut dyn HidSession, cmd: u8| {
            drain_input(session, device, deadline, 6)?;
            let mut req = vec![0_u8; 64];
            req[..8].copy_from_slice(&[0x08, 0x7D, 0x72, 0x02, 0x00, target, cmd, 0x01]);
            send_with_modes(session, device, OUTPUT, &req)?;
            wait_frame(
                session,
                device,
                deadline,
                Duration::from_millis(500),
                12,
                |r| {
                    Ok(r.len() >= 8
                        && r[0] == 0x08
                        && r[1] == 0x72
                        && r[2] == 0x00
                        && r[5] == cmd
                        && r[6] == 0x01)
                },
            )
        };
        let bat = ask(session, 0x07)?;
        let pct = checked_percentage(bat[7])?;
        let charge = if cable {
            match ask(session, 0x08) {
                Ok(c) if c[7] == 0x02 => ChargeState::Charging,
                Ok(_) => ChargeState::Discharging,
                Err(_) => ChargeState::Unknown,
            }
        } else {
            ChargeState::Unknown
        };
        Ok(reading(Some(pct), charge, None))
    }
}

struct AtkBitmouseNkProvider;

impl Provider for AtkBitmouseNkProvider {
    fn id(&self) -> &str {
        "atk.bitmouse_nk"
    }

    fn metadata(&self) -> ProviderMetadata {
        experimental_metadata(
            DeviceKind::Mouse,
            capabilities(true, true, false),
            Duration::from_millis(500),
        )
    }

    fn display_name(&self, _endpoint: &Endpoint) -> Option<String> {
        Some("ATK / VXE NK mouse".to_owned())
    }

    fn matches(&self, endpoint: &Endpoint) -> MatchDecision {
        if endpoint.vid == Some(0x373B)
            && endpoint.pid == Some(0x1216)
            && endpoint.usage_page == Some(0xFF05)
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
        let ask = |session: &mut dyn HidSession, cmd: u8| {
            drain_input(session, device, deadline, 6)?;
            let mut req = vec![0_u8; 64];
            req[..8].copy_from_slice(&[0x08, 0x00, 0x72, 0x02, 0x00, 0x01, cmd, 0x01]);
            req[1] = checksum_sum(&req[2..]);
            send_with_modes(session, device, OUTPUT, &req)?;
            wait_frame(
                session,
                device,
                deadline,
                Duration::from_millis(500),
                12,
                |r| Ok(r.len() >= 8 && r[0] == 0x08 && r[1] == 0x72 && r[2] != 0xFF && r[5] == cmd),
            )
        };
        let bat = ask(session, 0x07)?;
        if bat[7] == 0 || bat[7] > 100 {
            return Err(BatteryError::invalid_response(
                "invalid ATK NearLink battery byte",
            ));
        }
        let charge = match ask(session, 0x0F) {
            Ok(c) => charge_bool(c[7]),
            Err(BatteryError::Timeout { .. }) => ChargeState::Unknown,
            Err(error) => return Err(error),
        };
        Ok(reading(Some(bat[7]), charge, None))
    }
}

struct AtkNano8kProvider;

impl Provider for AtkNano8kProvider {
    fn id(&self) -> &str {
        "atk.nano_8k"
    }

    fn metadata(&self) -> ProviderMetadata {
        experimental_metadata(
            DeviceKind::Mouse,
            capabilities(true, true, false),
            Duration::from_millis(500),
        )
    }

    fn display_name(&self, _endpoint: &Endpoint) -> Option<String> {
        Some("ATK 8K Nano mouse".to_owned())
    }

    fn matches(&self, endpoint: &Endpoint) -> MatchDecision {
        if endpoint.vid == Some(0x373B)
            && endpoint.pid == Some(0x1178)
            && endpoint.usage_page == Some(0xFF10)
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
        let mut req = vec![0_u8; 16];
        req[1] = 0x82;
        send_with_modes(session, device, &[SendMode::Feature], &req)?;
        let frame = wait_frame(
            session,
            device,
            deadline,
            Duration::from_millis(500),
            12,
            |f| {
                let r = unnumbered_payload(f);
                Ok(r.len() >= 10 && r[0] == 0x82)
            },
        )?;
        let r = unnumbered_payload(&frame);
        if r[9] == 0 {
            return Ok(ProviderReading {
                reachability: Reachability::Offline,
                source: ReadingSource::VendorHid,
                ..ProviderReading::default()
            });
        }
        let pct = checked_percentage(r[8] & 0x7F)?;
        let charge = if (r[8] & 0x80) != 0 {
            ChargeState::Charging
        } else {
            ChargeState::Discharging
        };
        Ok(reading(Some(pct), charge, None))
    }
}

const FINALMOUSE_CURVE: &[(u32, u8)] = &[
    (4380, 100),
    (4170, 75),
    (3880, 50),
    (3740, 25),
    (3660, 10),
    (3620, 5),
    (3000, 0),
];

struct FinalmouseProvider;

impl Provider for FinalmouseProvider {
    fn id(&self) -> &str {
        "finalmouse.mouse"
    }

    fn metadata(&self) -> ProviderMetadata {
        experimental_metadata(
            DeviceKind::Mouse,
            capabilities(true, true, true),
            Duration::from_millis(500),
        )
    }

    fn display_name(&self, endpoint: &Endpoint) -> Option<String> {
        Some(match endpoint.pid {
            Some(0x0100) => "Finalmouse UltralightX".to_owned(),
            _ => "Finalmouse Starlight X".to_owned(),
        })
    }

    fn matches(&self, endpoint: &Endpoint) -> MatchDecision {
        let matched = match endpoint.pid {
            Some(0x0100) => endpoint.usage_page == Some(0xFF00) && endpoint.usage == Some(0x0001),
            Some(0x0300 | 0x0301) => endpoint.usage_page == Some(0xFF00),
            _ => false,
        };
        if endpoint.vid == Some(0x361D) && matched {
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
        let (out_id, in_id, first_get) = if device.endpoint.pid == Some(0x0100) {
            (0x04_u8, 0x05_u8, 0x15_u8)
        } else {
            (0x01_u8, 0x02_u8, 0x39_u8)
        };
        drain_input(session, device, deadline, 6)?;
        send_with_modes(
            session,
            device,
            OUTPUT,
            &[out_id, 0x02, 0x80 | first_get, 0x00],
        )?;
        send_with_modes(session, device, OUTPUT, &[out_id, 0x02, 0x80 | 0x60, 0x00])?;

        let mut soc: Option<u8> = None;
        let mut mv: Option<u32> = None;
        let mut charge = ChargeState::Unknown;
        let mut link: Option<u8> = None;

        for _ in 0..16 {
            let Ok(frame) = wait_frame(
                session,
                device,
                deadline,
                Duration::from_millis(200),
                4,
                |r| Ok(r.len() >= 4 && r[0] == in_id),
            ) else {
                break;
            };
            let inner = usize::from(frame[1]).min(frame.len() - 2);
            if inner < 2 {
                continue;
            }
            let n = usize::from(frame[3]).min(inner - 2);
            let p = &frame[4..4 + n];
            match frame[2] {
                0x05 if n >= 2 => {
                    let v = u32::from(u16::from_le_bytes([p[0], p[1]]));
                    if (1..=4999).contains(&v) {
                        mv = Some(v);
                    }
                }
                0x26 if n >= 3 && p[0] <= 100 => {
                    soc = Some(p[0]);
                    let v = u32::from(u16::from_le_bytes([p[1], p[2]]));
                    if (1..=4999).contains(&v) {
                        mv = Some(v);
                    }
                }
                0x25 if n >= 1 => {
                    charge = charge_bool(p[0]);
                }
                0x24 if n >= 1 => {
                    link = Some(p[0]);
                }
                _ => {}
            }
            if soc.is_some() || mv.is_some() {
                break;
            }
        }
        if let Some(pct) = soc {
            Ok(reading(Some(pct), charge, mv))
        } else if let Some(v) = mv {
            Ok(reading(
                Some(interpolate_mv(v, FINALMOUSE_CURVE)),
                charge,
                Some(v),
            ))
        } else if link == Some(0) {
            Ok(ProviderReading {
                reachability: Reachability::Sleeping,
                source: ReadingSource::VendorHid,
                ..ProviderReading::default()
            })
        } else {
            Err(BatteryError::timeout("Finalmouse"))
        }
    }
}

struct ScyroxProvider;

impl Provider for ScyroxProvider {
    fn id(&self) -> &str {
        "scyrox.v12m"
    }

    fn metadata(&self) -> ProviderMetadata {
        experimental_metadata(
            DeviceKind::Mouse,
            capabilities(true, false, false),
            Duration::from_millis(500),
        )
    }

    fn display_name(&self, _endpoint: &Endpoint) -> Option<String> {
        Some("Scyrox V12M".to_owned())
    }

    fn matches(&self, endpoint: &Endpoint) -> MatchDecision {
        if endpoint.vid == Some(0x395A)
            && endpoint.pid == Some(0x0006)
            && endpoint.usage_page == Some(0xFF00)
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
        let mut req = vec![0_u8; 64];
        req[0] = 0x08;
        req[1] = 0x01;
        req[63] = 0x09;
        send_with_modes(session, device, OUTPUT, &req)?;
        let r = wait_frame(
            session,
            device,
            deadline,
            Duration::from_millis(500),
            16,
            |r| {
                Ok(r.len() == 64
                    && r[0] == 0x08
                    && r[1] == 0x01
                    && r[3] != 0
                    && r[62] != 1
                    && r[4] <= 3
                    && r[4] == r[5]
                    && (1..=5).contains(&r[7]))
            },
        )?;
        Ok(reading(
            Some(checked_percentage(r[20])?),
            ChargeState::Unknown,
            None,
        ))
    }
}

const RAWM_RX_PIDS: &[u16] = &[
    0x232A, 0x232B, 0x2335, 0x2336, 0x233B, 0x233C, 0x2346, 0x2347, 0x234B,
];
const RAWM_WIRED_PIDS: &[u16] = &[
    0x2328, 0x2329, 0x232C, 0x232D, 0x232E, 0x232F, 0x2330, 0x2331, 0x2332, 0x2333, 0x2334, 0x2337,
    0x2338, 0x2339, 0x233A, 0x233E, 0x233F, 0x2340, 0x2341, 0x2343, 0x2344, 0x2348, 0x2349, 0x234A,
    0x2352,
];

struct RawmProvider;

impl Provider for RawmProvider {
    fn id(&self) -> &str {
        "rawm.mouse"
    }

    fn metadata(&self) -> ProviderMetadata {
        experimental_metadata(
            DeviceKind::Mouse,
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
                .unwrap_or_else(|| "RAWM wireless mouse".to_owned()),
        )
    }

    fn matches(&self, endpoint: &Endpoint) -> MatchDecision {
        let pid_ok = endpoint
            .pid
            .is_some_and(|p| RAWM_RX_PIDS.contains(&p) || RAWM_WIRED_PIDS.contains(&p));
        if endpoint.vid == Some(0x1915)
            && pid_ok
            && endpoint.usage_page == Some(0xFF00)
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
        let is_rx = device
            .endpoint
            .pid
            .is_some_and(|p| RAWM_RX_PIDS.contains(&p));
        drain_input(session, device, deadline, 6)?;
        let query_event = [0x01_u8, 0x0D, 0x03, 0x00, 0x00, 0, 0, 0, 0, 0, 0, 0, 0];
        let mut req = vec![0_u8; 65];
        if is_rx {
            req[1] = 0xC0;
            req[2] = 0x80 | (query_event.len() as u8);
            req[3..3 + query_event.len()].copy_from_slice(&query_event);
        } else {
            req[1] = 0x80 | (query_event.len() as u8);
            req[2..2 + query_event.len()].copy_from_slice(&query_event);
        }
        send_with_modes(session, device, OUTPUT, &req)?;

        let mut stream = Vec::new();
        for _ in 0..16 {
            let frame = wait_frame(
                session,
                device,
                deadline,
                Duration::from_millis(250),
                4,
                |f| Ok(!unnumbered_payload(f).is_empty()),
            )?;
            let mut r = unnumbered_payload(&frame);
            if !r.is_empty() && (r[0] & 0xC0) == 0xC0 {
                if (r[0] & 0x3F) != 0 {
                    continue;
                }
                r = &r[1..];
            }
            if !r.is_empty() && (r[0] & 0xC0) == 0x80 {
                let n = usize::from(r[0] & 0x3F).min(r.len() - 1);
                stream.extend_from_slice(&r[1..1 + n]);
            }
            if let Some(parsed) = parse_rawm_stream(&mut stream)? {
                return Ok(parsed);
            }
        }
        Err(BatteryError::timeout("RAWM mouse"))
    }
}

fn parse_rawm_stream(stream: &mut Vec<u8>) -> Result<Option<ProviderReading>, BatteryError> {
    while let Some(pos) = stream
        .windows(4)
        .position(|w| w == [0xFF, 0xFF, 0xFF, 0xFF])
    {
        if pos > 0 {
            stream.drain(..pos);
        }
        if stream.len() < 6 {
            break;
        }
        let h0 = stream[4];
        let h1 = stream[5];
        let cmd = h0 & 0x0F;
        let len = (usize::from(h0 & 0xF0) << 4) | usize::from(h1);
        if len < 2 {
            stream.drain(..4);
            continue;
        }
        let total = 4 + len;
        if stream.len() < total {
            break;
        }
        let body: Vec<u8> = stream[6..total].to_vec();
        stream.drain(..total);
        if cmd == 0x02 {
            let text_end = body.iter().position(|&b| b == 0).unwrap_or(body.len());
            if let Ok(json_str) = std::str::from_utf8(&body[..text_end]) {
                if let Ok(value) = serde_json::from_str::<serde_json::Value>(json_str) {
                    if let Some(bat) = value.get("battery").and_then(serde_json::Value::as_u64) {
                        let pct = checked_percentage(bat.min(255) as u8)?;
                        let charge = match value.get("chr").and_then(serde_json::Value::as_u64) {
                            Some(1) => ChargeState::Charging,
                            Some(0) => ChargeState::Discharging,
                            _ => ChargeState::Unknown,
                        };
                        return Ok(Some(reading(Some(pct), charge, None)));
                    }
                }
            }
        } else if cmd == 0x0B && body.len() >= 3 && body[0] == 0x17 {
            let pct = checked_percentage(body[1])?;
            return Ok(Some(reading(Some(pct), charge_bool(body[2]), None)));
        }
    }
    Ok(None)
}

const ZOWIE_TABLE: &[(u32, u8)] = &[
    (4200, 100),
    (4000, 75),
    (3890, 50),
    (3760, 25),
    (3680, 10),
    (3500, 0),
];

struct ZowieCwProvider;

impl Provider for ZowieCwProvider {
    fn id(&self) -> &str {
        "zowie.ec_cw"
    }

    fn metadata(&self) -> ProviderMetadata {
        experimental_metadata(
            DeviceKind::Mouse,
            capabilities(true, true, true),
            Duration::from_millis(500),
        )
    }

    fn display_name(&self, _endpoint: &Endpoint) -> Option<String> {
        Some("ZOWIE EC-CW".to_owned())
    }

    fn matches(&self, endpoint: &Endpoint) -> MatchDecision {
        if endpoint.vid == Some(0x04A5)
            && matches!(endpoint.pid, Some(0x8005 | 0x8006))
            && endpoint
                .usage_page
                .is_some_and(|p| p >= 0xFF00 || p == 0x00FF)
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
        let mut req = vec![0_u8; 65];
        req[1] = 0xA5;
        req[2] = 0x03;
        send_with_modes(session, device, OUTPUT, &req)?;
        let frame = wait_frame(
            session,
            device,
            deadline,
            Duration::from_millis(500),
            12,
            |f| {
                let r = unnumbered_payload(f);
                if r.len() < 3 {
                    return Ok(false);
                }
                let mv = u32::from(u16::from_le_bytes([r[0], r[1]]));
                Ok(mv == 0 || (3000..=4500).contains(&mv))
            },
        )?;
        let r = unnumbered_payload(&frame);
        let mv = u32::from(u16::from_le_bytes([r[0], r[1]]));
        if mv == 0 {
            return Ok(ProviderReading {
                reachability: Reachability::Offline,
                source: ReadingSource::VendorHid,
                ..ProviderReading::default()
            });
        }
        let pct = if (1..=100).contains(&r[2]) {
            r[2]
        } else {
            interpolate_mv(mv, ZOWIE_TABLE)
        };
        let charge = if device.endpoint.pid == Some(0x8005) {
            ChargeState::Charging
        } else {
            ChargeState::Unknown
        };
        Ok(reading(Some(pct), charge, Some(mv)))
    }
}
