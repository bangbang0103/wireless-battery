//! Enthusiast wireless mouse and receiver HID battery protocols:
//! ATK (Zero, NearLink, Nano 8K), Finalmouse, Scyrox, RAWM/SADES, ZOWIE EC-CW,
//! Cooler Master, NZXT, MSI, Fantech XD5, Raigor, Sinowealth, Royal Kludge,
//! Darmoshark, Machenike, Niceboy, Motospeed, Xenta, Delux M800 Pro, Ajazz,
//! Incott, and Rongyuan receivers.

use crate::error::BatteryError;
use crate::model::{ChargeState, Endpoint, ProviderReading, Reachability, ReadingSource};
use crate::providers::fixed::{
    capabilities, charge_bool, checked_percentage, checksum_sum, drain_input,
    experimental_metadata, feature_frame, interpolate_mv, reading, send_with_modes, sleep,
    unnumbered_payload, wait_frame, SendMode, OUTPUT,
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
        Arc::new(CoolerMasterProvider),
        Arc::new(NzxtLiftEliteProvider),
        Arc::new(MsiQueryProvider),
        Arc::new(MsiGm41Provider),
        Arc::new(FantechXd5Provider),
        Arc::new(RaigorProvider),
        Arc::new(SinowealthReceiverProvider),
        Arc::new(RoyalKludgeProvider),
        Arc::new(DarmosharkM3Provider),
        Arc::new(DarmosharkNrfProvider),
        Arc::new(MachenikeL7Provider),
        Arc::new(NiceboyMw400Provider),
        Arc::new(MotospeedX6Provider),
        Arc::new(XentaProvider),
        Arc::new(DeluxM800Provider),
        Arc::new(AjazzAj179Provider),
        Arc::new(AjazzAjSeriesProvider),
        Arc::new(IncottProvider),
        Arc::new(RongyuanProvider),
    ]
}

struct AtkZeroProvider;

impl Provider for AtkZeroProvider {
    fn id(&self) -> &str {
        "atk.zero"
    }

    fn metadata(&self) -> ProviderMetadata {
        experimental_metadata(capabilities(true, true, false), Duration::from_millis(500))
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
        experimental_metadata(capabilities(true, true, false), Duration::from_millis(500))
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
        experimental_metadata(capabilities(true, true, false), Duration::from_millis(500))
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
        experimental_metadata(capabilities(true, true, true), Duration::from_millis(500))
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
        experimental_metadata(capabilities(true, false, false), Duration::from_millis(500))
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
        experimental_metadata(capabilities(true, true, false), Duration::from_millis(500))
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
        experimental_metadata(capabilities(true, true, true), Duration::from_millis(500))
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

struct CoolerMasterProvider;

impl Provider for CoolerMasterProvider {
    fn id(&self) -> &str {
        "coolermaster.mouse"
    }

    fn metadata(&self) -> ProviderMetadata {
        experimental_metadata(capabilities(true, true, false), Duration::from_millis(500))
    }

    fn display_name(&self, endpoint: &Endpoint) -> Option<String> {
        Some(match endpoint.pid {
            Some(0x0169) => "Cooler Master MM712".to_owned(),
            Some(0x0167) => "Cooler Master MM731".to_owned(),
            _ => "Cooler Master wireless mouse".to_owned(),
        })
    }

    fn matches(&self, endpoint: &Endpoint) -> MatchDecision {
        if endpoint.vid == Some(0x2516)
            && matches!(endpoint.pid, Some(0x0167 | 0x0169 | 0x01B7))
            && endpoint.usage_page == Some(0xFF0A)
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
        let mut req = vec![0_u8; 32];
        req[1] = 0x46;
        req[2] = 0x81;
        send_with_modes(session, device, OUTPUT, &req)?;
        let frame = wait_frame(
            session,
            device,
            deadline,
            Duration::from_millis(500),
            12,
            |f| {
                let r = unnumbered_payload(f);
                Ok(r.len() >= 13 && r[0] == 0x47 && r[1] == 0x81)
            },
        )?;
        let r = unnumbered_payload(&frame);
        if r[10] == 0 || r[10] > 100 {
            return Err(BatteryError::invalid_response(
                "invalid Cooler Master battery percentage",
            ));
        }
        let charge = if r[11] != 0 {
            ChargeState::Charging
        } else {
            ChargeState::Discharging
        };
        Ok(reading(Some(r[10]), charge, None))
    }
}

struct NzxtLiftEliteProvider;

impl Provider for NzxtLiftEliteProvider {
    fn id(&self) -> &str {
        "nzxt.lift_elite"
    }

    fn metadata(&self) -> ProviderMetadata {
        experimental_metadata(capabilities(true, true, true), Duration::from_millis(400))
    }

    fn display_name(&self, _endpoint: &Endpoint) -> Option<String> {
        Some("NZXT Lift Elite Wireless".to_owned())
    }

    fn matches(&self, endpoint: &Endpoint) -> MatchDecision {
        if endpoint.vid == Some(0x1E71)
            && matches!(endpoint.pid, Some(0x2101 | 0x2129))
            && endpoint.usage_page == Some(0xFFCA)
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
        let mut req = vec![0_u8; 65];
        req[1..6].copy_from_slice(&[0x4E, 0x02, 0x81, 0x00, 0xB0]);
        send_with_modes(session, device, OUTPUT, &req)?;
        let frame = wait_frame(
            session,
            device,
            deadline,
            Duration::from_millis(400),
            10,
            |f| {
                let r = unnumbered_payload(f);
                Ok(r.len() >= 27 && r[0] == 0x4E && r[1] == 0x02 && r[2] == 0x97)
            },
        )?;
        let r = unnumbered_payload(&frame);
        let pct_u16 = u16::from_le_bytes([r[17], r[18]]);
        if pct_u16 > 100 {
            return Err(BatteryError::invalid_response(
                "NZXT battery percentage exceeds 100",
            ));
        }
        let mv = u32::from(u16::from_be_bytes([r[8], r[9]]));
        let charge = if (r[22] & 0x01) != 0 {
            ChargeState::Charging
        } else {
            ChargeState::Discharging
        };
        Ok(reading(
            Some(pct_u16 as u8),
            charge,
            (3000..=4500).contains(&mv).then_some(mv),
        ))
    }
}

struct MsiQueryProvider;

impl Provider for MsiQueryProvider {
    fn id(&self) -> &str {
        "msi.mouse"
    }

    fn metadata(&self) -> ProviderMetadata {
        experimental_metadata(capabilities(true, true, false), Duration::from_millis(500))
    }

    fn display_name(&self, endpoint: &Endpoint) -> Option<String> {
        Some(match endpoint.pid {
            Some(0x9E22) => "MSI VERSA 300 ELITE WIRELESS".to_owned(),
            _ => "MSI Clutch GM31 Lightweight Wireless".to_owned(),
        })
    }

    fn matches(&self, endpoint: &Endpoint) -> MatchDecision {
        if endpoint.vid != Some(0x0DB0) {
            return MatchDecision::NoMatch;
        }
        let matched = match endpoint.pid {
            Some(0x9E22) => endpoint.usage_page == Some(0xFF0D) && endpoint.usage == Some(0x00B0),
            Some(0x0D3C | 0x0D3B) => {
                endpoint.usage_page == Some(0xFF00) && endpoint.usage == Some(0x0001)
            }
            _ => false,
        };
        if matched {
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
        req[1] = 0x0F;
        req[2] = 0x20;
        send_with_modes(session, device, OUTPUT, &req)?;
        let frame = wait_frame(
            session,
            device,
            deadline,
            Duration::from_millis(500),
            12,
            |f| {
                let r = unnumbered_payload(f);
                Ok(r.len() >= 15 && r[0] == 0x0F && r[1] == 0x20)
            },
        )?;
        let r = unnumbered_payload(&frame);
        let level = r[14] & 0x7F;
        let charging = (r[14] & 0x80) != 0;
        if level == 0 && !charging {
            return Ok(ProviderReading {
                reachability: Reachability::Sleeping,
                source: ReadingSource::VendorHid,
                ..ProviderReading::default()
            });
        }
        Ok(reading(
            Some(checked_percentage(level)?),
            if charging {
                ChargeState::Charging
            } else {
                ChargeState::Discharging
            },
            None,
        ))
    }
}

struct MsiGm41Provider;

impl Provider for MsiGm41Provider {
    fn id(&self) -> &str {
        "msi.gm41"
    }

    fn metadata(&self) -> ProviderMetadata {
        experimental_metadata(
            capabilities(true, false, false),
            Duration::from_millis(1000),
        )
    }

    fn display_name(&self, _endpoint: &Endpoint) -> Option<String> {
        Some("MSI Clutch GM41 Lightweight Wireless".to_owned())
    }

    fn matches(&self, endpoint: &Endpoint) -> MatchDecision {
        if endpoint.vid == Some(0x0DB0)
            && matches!(endpoint.pid, Some(0x0D4B | 0x0D4C))
            && endpoint.usage_page == Some(0xFF10)
            && endpoint.usage == Some(0x0006)
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
            Duration::from_millis(1000),
            16,
            |f| {
                let r = unnumbered_payload(f);
                Ok(r.len() >= 7 && r[0] == 0xA0 && r[2] == 0x07)
            },
        )?;
        let r = unnumbered_payload(&frame);
        let pct = ((u32::from(r[1]) * 51 + 1760 + 50) / 100).min(100) as u8;
        Ok(reading(Some(pct), ChargeState::Unknown, None))
    }
}

struct FantechXd5Provider;

impl Provider for FantechXd5Provider {
    fn id(&self) -> &str {
        "fantech.xd5"
    }

    fn metadata(&self) -> ProviderMetadata {
        experimental_metadata(capabilities(true, true, false), Duration::from_millis(500))
    }

    fn display_name(&self, endpoint: &Endpoint) -> Option<String> {
        Some(match endpoint.pid {
            Some(0xFA7E) => "Redragon Invader Pro".to_owned(),
            _ => "Fantech Helios Go XD5".to_owned(),
        })
    }

    fn matches(&self, endpoint: &Endpoint) -> MatchDecision {
        if endpoint.vid == Some(0x25A7)
            && matches!(endpoint.pid, Some(0xFA7B | 0xFA7C | 0xFA7E))
            && endpoint.usage_page.is_some_and(|p| p >= 0xFF00)
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
        let mut req = vec![0_u8; 17];
        req[0] = 0x08;
        req[1] = 0x04;
        req[16] = 0x49;
        send_with_modes(session, device, &[SendMode::Feature], &req)?;
        let r = match wait_frame(
            session,
            device,
            deadline,
            Duration::from_millis(400),
            8,
            |f| Ok(f.len() >= 8 && (f[0] == 0x04 || f[1] == 0x04)),
        ) {
            Ok(f) => f,
            Err(BatteryError::Timeout { .. }) => feature_frame(session, device, 0x08, 16)?,
            Err(error) => return Err(error),
        };
        let (power, chg) = if r.len() >= 8 && r[1] == 0x04 {
            (r[6], r[7])
        } else if r.len() >= 7 && r[0] == 0x04 {
            (r[5], r[6])
        } else {
            return Err(BatteryError::invalid_response(
                "invalid Fantech XD5 battery response",
            ));
        };
        let pct = if device.endpoint.pid == Some(0xFA7E) {
            (power.saturating_mul(10)).min(100)
        } else {
            checked_percentage(power)?
        };
        Ok(reading(Some(pct), charge_bool(chg), None))
    }
}

struct RaigorProvider;

impl Provider for RaigorProvider {
    fn id(&self) -> &str {
        "raigor.mouse"
    }

    fn metadata(&self) -> ProviderMetadata {
        experimental_metadata(capabilities(true, false, false), Duration::from_millis(500))
    }

    fn display_name(&self, endpoint: &Endpoint) -> Option<String> {
        Some(
            endpoint
                .product_name
                .clone()
                .filter(|name| !name.is_empty())
                .unwrap_or_else(|| "Fantech Raigor / 2.4 GHz mouse".to_owned()),
        )
    }

    fn matches(&self, endpoint: &Endpoint) -> MatchDecision {
        if endpoint.vid == Some(0x1EA7)
            && matches!(endpoint.pid, Some(0x0064 | 0x0066))
            && endpoint.usage_page == Some(0xFFB5)
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
        let req = [0xB5, 0x05, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00];
        send_with_modes(session, device, OUTPUT, &req)?;
        let r = wait_frame(
            session,
            device,
            deadline,
            Duration::from_millis(500),
            12,
            |r| {
                Ok(r.len() == 8
                    && r[0] == 0xB5
                    && ((r[1] == 0x05 && r[2] == 0x94 && r[3] == 0x04) || r[1] == 0x67))
            },
        )?;
        let raw = if r[1] == 0x67 { r[7] } else { r[6] };
        Ok(reading(
            Some(checked_percentage(raw)?),
            ChargeState::Unknown,
            None,
        ))
    }
}

struct SinowealthReceiverProvider;

impl Provider for SinowealthReceiverProvider {
    fn id(&self) -> &str {
        "sinowealth.receiver"
    }

    fn metadata(&self) -> ProviderMetadata {
        experimental_metadata(capabilities(true, false, false), Duration::from_millis(300))
    }

    fn display_name(&self, _endpoint: &Endpoint) -> Option<String> {
        Some("Redragon M602-KS / Sinowealth mouse".to_owned())
    }

    fn matches(&self, endpoint: &Endpoint) -> MatchDecision {
        if endpoint.vid == Some(0x258A)
            && endpoint.pid == Some(0x002F)
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
        let ask = |session: &mut dyn HidSession, cmd: u8| {
            let req = [0x05, cmd, 0, 0, 0, 0, 0, 0];
            send_with_modes(session, device, &[SendMode::Feature], &req)?;
            sleep(deadline, Duration::from_millis(100))?;
            let r = feature_frame(session, device, 0x05, 7)?;
            if r.len() >= 4 && r[0] == 0x05 && r[1] == cmd {
                Ok(r)
            } else {
                Err(BatteryError::invalid_response(
                    "invalid Sinowealth receiver response",
                ))
            }
        };
        let link = ask(session, 0x80)?;
        if link[2] != 0x01 {
            return Ok(ProviderReading {
                reachability: Reachability::Offline,
                source: ReadingSource::VendorHid,
                ..ProviderReading::default()
            });
        }
        let bat = ask(session, 0x90)?;
        if bat[2] != 0x11 {
            return Err(BatteryError::invalid_response(
                "unexpected Sinowealth battery tag",
            ));
        }
        Ok(reading(
            Some(checked_percentage(bat[3])?),
            ChargeState::Unknown,
            None,
        ))
    }
}

struct RoyalKludgeProvider;

impl Provider for RoyalKludgeProvider {
    fn id(&self) -> &str {
        "royal_kludge.mg6_pro"
    }

    fn metadata(&self) -> ProviderMetadata {
        experimental_metadata(capabilities(true, true, true), Duration::from_millis(500))
    }

    fn display_name(&self, _endpoint: &Endpoint) -> Option<String> {
        Some("Royal Kludge MG6 Pro".to_owned())
    }

    fn matches(&self, endpoint: &Endpoint) -> MatchDecision {
        let matched = match endpoint.pid {
            Some(0x00A6 | 0x00E6) => {
                endpoint.usage_page == Some(0xFF00) && endpoint.usage == Some(0x0001)
            }
            Some(0x00A5 | 0x00E5) => {
                endpoint.usage_page == Some(0xFFF2) && endpoint.usage == Some(0x0001)
            }
            _ => false,
        };
        if endpoint.vid == Some(0x2E3C) && matched {
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
        let cable = matches!(device.endpoint.pid, Some(0x00A5 | 0x00E5));
        drain_input(session, device, deadline, 6)?;
        let mut req = vec![0_u8; 65];
        req[1..5].copy_from_slice(&[0x01, 0x01, 0x02, 0x01]);
        send_with_modes(session, device, OUTPUT, &req)?;
        let frame = wait_frame(
            session,
            device,
            deadline,
            Duration::from_millis(500),
            12,
            |f| {
                let r = unnumbered_payload(f);
                Ok(r.len() >= 11 && (r[0] & 0x1F) == 0x01 && (r[1] & 0x0F) == 0x01 && r[2] == 0x02)
            },
        )?;
        let r = unnumbered_payload(&frame);
        if ((r[3] >> 4) & 1) == 0 {
            return Ok(ProviderReading {
                reachability: Reachability::Sleeping,
                source: ReadingSource::VendorHid,
                ..ProviderReading::default()
            });
        }
        let pct_u16 = u16::from_le_bytes([r[6], r[7]]);
        if pct_u16 > 100 {
            return Err(BatteryError::invalid_response(
                "Royal Kludge battery percentage exceeds 100",
            ));
        }
        let mv = u32::from(u16::from_le_bytes([r[4], r[5]]));
        let charge = if cable || r[10] != 0 {
            ChargeState::Charging
        } else {
            ChargeState::Discharging
        };
        Ok(reading(
            Some(pct_u16 as u8),
            charge,
            (3000..=4500).contains(&mv).then_some(mv),
        ))
    }
}

struct DarmosharkM3Provider;

impl Provider for DarmosharkM3Provider {
    fn id(&self) -> &str {
        "darmoshark.m3"
    }

    fn metadata(&self) -> ProviderMetadata {
        experimental_metadata(capabilities(true, false, false), Duration::from_millis(400))
    }

    fn display_name(&self, endpoint: &Endpoint) -> Option<String> {
        Some(match endpoint.pid {
            Some(0xFF31) => "Darmoshark M3S".to_owned(),
            _ => "Darmoshark M3".to_owned(),
        })
    }

    fn matches(&self, endpoint: &Endpoint) -> MatchDecision {
        if endpoint.vid == Some(0x248A)
            && matches!(endpoint.pid, Some(0xFF30 | 0xFF31))
            && endpoint.usage_page == Some(0x008C)
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
        let mouse_pid: u16 = if device.endpoint.pid == Some(0xFF31) {
            0xFF18
        } else {
            0xFF12
        };
        let mut req = vec![0_u8; 21];
        req[0] = 0x51;
        req[1] = 0x06;
        send_with_modes(session, device, &[SendMode::Feature], &req)?;
        for _ in 0..8 {
            sleep(deadline, Duration::from_millis(30))?;
            let r = feature_frame(session, device, 0x51, 20)?;
            if r.len() >= 12
                && r[0] == 0x51
                && r[1] == 0x06
                && r[2] == 0x01
                && u16::from_le_bytes([r[4], r[5]]) == 0x248A
                && u16::from_le_bytes([r[6], r[7]]) == mouse_pid
            {
                return Ok(reading(
                    Some(checked_percentage(r[11])?),
                    ChargeState::Unknown,
                    None,
                ));
            }
        }
        Err(BatteryError::timeout("Darmoshark M3"))
    }
}

struct DarmosharkNrfProvider;

impl Provider for DarmosharkNrfProvider {
    fn id(&self) -> &str {
        "darmoshark.nrf"
    }

    fn metadata(&self) -> ProviderMetadata {
        experimental_metadata(capabilities(true, false, false), Duration::from_millis(500))
    }

    fn display_name(&self, _endpoint: &Endpoint) -> Option<String> {
        Some("Darmoshark M3S Pro".to_owned())
    }

    fn matches(&self, endpoint: &Endpoint) -> MatchDecision {
        if endpoint.vid == Some(0x1915)
            && endpoint.pid == Some(0x0723)
            && endpoint.usage_page == Some(0xFF0A)
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
        let mut req = vec![0_u8; 65];
        req[1] = 0x41;
        req[3] = 0x81;
        req[4] = 0x01;
        req[64] = 0xA1_u8.wrapping_sub(checksum_sum(&req[1..64]));
        send_with_modes(session, device, OUTPUT, &req)?;
        let frame = wait_frame(
            session,
            device,
            deadline,
            Duration::from_millis(500),
            12,
            |f| {
                let r = unnumbered_payload(f);
                Ok(r.len() >= 64 && r[0] == 0x41 && r[2] == 0x8C && r[3] == 0x01)
            },
        )?;
        let r = unnumbered_payload(&frame);
        if r[63] != 0xA1_u8.wrapping_sub(checksum_sum(&r[..63])) {
            return Err(BatteryError::ChecksumFailed);
        }
        Ok(reading(
            Some(checked_percentage(r[6])?),
            ChargeState::Unknown,
            None,
        ))
    }
}

struct MachenikeL7Provider;

impl Provider for MachenikeL7Provider {
    fn id(&self) -> &str {
        "machenike.l7"
    }

    fn metadata(&self) -> ProviderMetadata {
        experimental_metadata(capabilities(true, true, false), Duration::from_millis(250))
    }

    fn display_name(&self, _endpoint: &Endpoint) -> Option<String> {
        Some("Machenike L7 Mix".to_owned())
    }

    fn matches(&self, endpoint: &Endpoint) -> MatchDecision {
        if endpoint.vid == Some(0x248A)
            && matches!(endpoint.pid, Some(0xFF4C | 0xFF4E))
            && endpoint.usage_page == Some(0x008C)
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
        let cable = device.endpoint.pid == Some(0xFF4E);
        let mut req = vec![0_u8; 21];
        req[0] = 0x51;
        req[1] = 0x06;
        send_with_modes(session, device, &[SendMode::Feature], &req)?;
        for _ in 0..10 {
            sleep(deadline, Duration::from_millis(10))?;
            let r = feature_frame(session, device, 0x51, 20)?;
            if r.len() >= 13
                && r[0] == 0x51
                && r[1] == 0x06
                && u16::from_le_bytes([r[4], r[5]]) == 0x248A
                && u16::from_le_bytes([r[6], r[7]]) == 0xFF4E
                && (r[10] & 0x07) == u8::from(!cable)
            {
                let connected = cable || (r[10] & 0x08) != 0;
                if !connected {
                    return Ok(ProviderReading {
                        reachability: Reachability::Offline,
                        source: ReadingSource::VendorHid,
                        ..ProviderReading::default()
                    });
                }
                return Ok(reading(
                    Some(checked_percentage(r[11])?),
                    charge_bool(r[12]),
                    None,
                ));
            }
        }
        Err(BatteryError::timeout("Machenike L7"))
    }
}

struct NiceboyMw400Provider;

impl Provider for NiceboyMw400Provider {
    fn id(&self) -> &str {
        "niceboy.mw400"
    }

    fn metadata(&self) -> ProviderMetadata {
        experimental_metadata(capabilities(true, true, true), Duration::from_millis(500))
    }

    fn display_name(&self, _endpoint: &Endpoint) -> Option<String> {
        Some("Niceboy MW400".to_owned())
    }

    fn matches(&self, endpoint: &Endpoint) -> MatchDecision {
        if endpoint.vid == Some(0x248A)
            && endpoint.pid == Some(0xFA02)
            && endpoint.usage_page == Some(0x0001)
            && endpoint.usage == Some(0x0000)
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
        let mut req = vec![0_u8; 33];
        req[..4].copy_from_slice(&[0x05, 0x24, 0xFA, 0x48]);
        send_with_modes(session, device, OUTPUT, &req)?;
        let r = wait_frame(
            session,
            device,
            deadline,
            Duration::from_millis(500),
            12,
            |r| Ok(r.len() >= 7 && r[0] == 0x05 && r[2] == 0xFA && r[3] == 0x48),
        )?;
        if (r[4] | r[5] | r[6]) == 0 {
            return Ok(ProviderReading {
                reachability: Reachability::Sleeping,
                source: ReadingSource::VendorHid,
                ..ProviderReading::default()
            });
        }
        let mv = (u32::from(u16::from_le_bytes([r[4], r[5]])) * 990) / 240;
        let pct = match mv {
            4200.. => 100,
            4100..=4199 => 90,
            4000..=4099 => 80,
            3900..=3999 => 70,
            3800..=3899 => 60,
            3700..=3799 => 50,
            3500..=3699 => 40,
            3400..=3499 => 30,
            3200..=3399 => 20,
            _ => 10,
        };
        let charge = if r[6] != 0 {
            ChargeState::Charging
        } else {
            ChargeState::Discharging
        };
        Ok(reading(Some(pct), charge, Some(mv)))
    }
}

struct MotospeedX6Provider;

impl Provider for MotospeedX6Provider {
    fn id(&self) -> &str {
        "motospeed.x6"
    }

    fn metadata(&self) -> ProviderMetadata {
        experimental_metadata(capabilities(true, true, false), Duration::from_millis(600))
    }

    fn display_name(&self, _endpoint: &Endpoint) -> Option<String> {
        Some("Motospeed X6".to_owned())
    }

    fn matches(&self, endpoint: &Endpoint) -> MatchDecision {
        if endpoint.vid == Some(0x0BDA)
            && matches!(endpoint.pid, Some(0xFFE0 | 0xFFF2))
            && endpoint.interface_number == Some(4)
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
        let mut req = vec![0_u8; 21];
        req[0] = 0xB5;
        req[1] = 0x06;
        send_with_modes(session, device, OUTPUT, &req)?;
        let r = wait_frame(
            session,
            device,
            deadline,
            Duration::from_millis(600),
            12,
            |r| Ok(r.len() > 20 && r[0] == 0xB4),
        )?;
        let raw = r[20];
        let charging = raw >= 128;
        let pct = checked_percentage(if charging { raw - 128 } else { raw })?;
        Ok(reading(
            Some(pct),
            if charging {
                ChargeState::Charging
            } else {
                ChargeState::Discharging
            },
            None,
        ))
    }
}

struct XentaProvider;

impl Provider for XentaProvider {
    fn id(&self) -> &str {
        "xenta.mouse"
    }

    fn metadata(&self) -> ProviderMetadata {
        experimental_metadata(capabilities(true, true, false), Duration::from_millis(1500))
    }

    fn display_name(&self, endpoint: &Endpoint) -> Option<String> {
        Some(
            endpoint
                .product_name
                .clone()
                .filter(|name| !name.is_empty())
                .unwrap_or_else(|| "Xenta wireless mouse".to_owned()),
        )
    }

    fn matches(&self, endpoint: &Endpoint) -> MatchDecision {
        if endpoint.vid == Some(0x1D57)
            && matches!(
                endpoint.pid,
                Some(0xFA60 | 0xFA61 | 0xFA65 | 0xFA55 | 0x2155)
            )
            && endpoint.usage_page == Some(0x000A)
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
        let cable = matches!(device.endpoint.pid, Some(0xFA55 | 0x2155));
        let r = wait_frame(
            session,
            device,
            deadline,
            Duration::from_millis(1500),
            24,
            |r| Ok(r.len() >= 5 && r[0] == 0x03 && matches!(r[2], 0x40 | 0x41)),
        )?;
        let model = r[1];
        let state = r[3];
        let level = r[4];
        if device.endpoint.pid == Some(0xFA60)
            && model == 0x00
            && r[2] == 0x40
            && (state & 0x0F) == 0x03
        {
            let pct = match state >> 4 {
                0x01 => 25,
                0x03 => 50,
                0x07 => 75,
                0x0F => 100,
                _ => {
                    return Err(BatteryError::invalid_response(
                        "unknown ARDOR Fury V2 battery tier",
                    ))
                }
            };
            return Ok(reading(Some(pct), ChargeState::Discharging, None));
        }
        if device.endpoint.pid == Some(0xFA65) && matches!(model, 0x30 | 0x50) {
            return match state {
                0x02 => Ok(reading(Some(100), ChargeState::Full, None)),
                0x03 => Ok(reading(
                    Some(checked_percentage(level)?),
                    ChargeState::Charging,
                    None,
                )),
                0x01 => Ok(reading(
                    Some(checked_percentage(level)?),
                    ChargeState::Discharging,
                    None,
                )),
                _ => Ok(ProviderReading {
                    reachability: Reachability::Sleeping,
                    source: ReadingSource::VendorHid,
                    ..ProviderReading::default()
                }),
            };
        }
        if cable || matches!(state, 0x02 | 0x03) {
            let (pct, charge) = if state == 0x02 && !cable {
                (Some(100), ChargeState::Full)
            } else {
                (
                    (1..=100).contains(&level).then_some(level),
                    ChargeState::Charging,
                )
            };
            return Ok(reading(pct, charge, None));
        }
        if model == 0x10 && (1..=10).contains(&level) {
            return Ok(reading(Some(level * 10), ChargeState::Discharging, None));
        }
        if state == 0x01 && (1..=100).contains(&level) {
            return Ok(reading(Some(level), ChargeState::Discharging, None));
        }
        Ok(ProviderReading {
            reachability: Reachability::Sleeping,
            source: ReadingSource::VendorHid,
            ..ProviderReading::default()
        })
    }
}

struct DeluxM800Provider;

impl Provider for DeluxM800Provider {
    fn id(&self) -> &str {
        "delux.m800_pro"
    }

    fn metadata(&self) -> ProviderMetadata {
        experimental_metadata(capabilities(true, true, false), Duration::from_millis(300))
    }

    fn display_name(&self, _endpoint: &Endpoint) -> Option<String> {
        Some("Delux M800 Pro".to_owned())
    }

    fn matches(&self, endpoint: &Endpoint) -> MatchDecision {
        if endpoint.vid == Some(0x248A)
            && matches!(endpoint.pid, Some(0x5B2E | 0x5B2F))
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
        let mut req = vec![0_u8; 33];
        req[..5].copy_from_slice(&[0x0C, 0x01, 0x20, 0x00, 0x01]);
        send_with_modes(session, device, &[SendMode::Feature], &req)?;
        sleep(deadline, Duration::from_millis(20))?;
        let r = feature_frame(session, device, 0x0C, 32)?;
        if r.len() < 20 || r[0] != 0x0C || r[2] != 0x20 {
            return Err(BatteryError::invalid_response(
                "invalid Delux M800 Pro response",
            ));
        }
        let connected = r[6] != 0 || r[18] > 0;
        if !connected {
            return Ok(ProviderReading {
                reachability: Reachability::Offline,
                source: ReadingSource::VendorHid,
                ..ProviderReading::default()
            });
        }
        let pct = checked_percentage(r[18])?;
        let (pct, charge) = match r[19] {
            2 => (Some(100), ChargeState::Full),
            1 => ((pct > 0).then_some(pct), ChargeState::Charging),
            0 => ((pct > 0).then_some(pct), ChargeState::Discharging),
            _ => ((pct > 0).then_some(pct), ChargeState::Unknown),
        };
        Ok(reading(pct, charge, None))
    }
}

struct AjazzAj179Provider;

impl Provider for AjazzAj179Provider {
    fn id(&self) -> &str {
        "ajazz.aj179"
    }

    fn metadata(&self) -> ProviderMetadata {
        experimental_metadata(capabilities(true, true, false), Duration::from_millis(600))
    }

    fn display_name(&self, _endpoint: &Endpoint) -> Option<String> {
        Some("AJAZZ AJ179".to_owned())
    }

    fn matches(&self, endpoint: &Endpoint) -> MatchDecision {
        if endpoint.vid == Some(0x249A) && endpoint.interface_number == Some(2) {
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
        let mut req = vec![0_u8; 33];
        req[1] = 0x10;
        send_with_modes(session, device, OUTPUT, &req)?;
        let frame = wait_frame(
            session,
            device,
            deadline,
            Duration::from_millis(600),
            12,
            |f| {
                let r = unnumbered_payload(f);
                Ok(r.len() >= 32 && r[0] == 0x10 && r[1] == 0x00)
            },
        )?;
        let r = unnumbered_payload(&frame);
        if r[31] != checksum_sum(&r[4..31]) {
            return Err(BatteryError::ChecksumFailed);
        }
        if r[14] == 0 {
            return Ok(ProviderReading {
                reachability: Reachability::Offline,
                source: ReadingSource::VendorHid,
                ..ProviderReading::default()
            });
        }
        let pct = checked_percentage(r[13])?;
        let charge = if r[12] != 0 {
            ChargeState::Charging
        } else {
            ChargeState::Discharging
        };
        Ok(reading(Some(pct), charge, None))
    }
}

const RONGYUAN_PAIRS: &[(u16, u16)] = &[
    (0x0DB0, 0x197E),
    (0x0DB0, 0x8E9A),
    (0x145F, 0x0376),
    (0x145F, 0x038D),
    (0x14A5, 0xC045),
    (0x25AA, 0x2008),
    (0x25AA, 0x4008),
    (0x25AA, 0x4018),
    (0x3151, 0x4011),
    (0x3151, 0x4014),
    (0x3151, 0x4017),
    (0x3151, 0x4021),
    (0x3151, 0x4027),
    (0x3151, 0x402D),
    (0x3151, 0x4038),
    (0x3151, 0x5006),
    (0x3151, 0x5007),
    (0x3151, 0x5008),
    (0x3151, 0x5020),
    (0x3151, 0x5026),
    (0x3151, 0x502B),
    (0x3151, 0x5031),
    (0x3151, 0x5037),
    (0x3151, 0x5038),
    (0x3151, 0x503D),
    (0x3151, 0x503F),
    (0x3151, 0x5040),
    (0x3151, 0x5042),
    (0x3151, 0x5045),
    (0x3151, 0x504B),
    (0x3151, 0x504E),
    (0x3151, 0x5050),
    (0x3151, 0x5052),
    (0x3151, 0x5055),
    (0x3151, 0x505A),
    (0x342D, 0xE3E0),
    (0x342D, 0xE3E1),
    (0x342D, 0xE3E2),
    (0x342D, 0xE3E6),
    (0x342D, 0xE3E7),
    (0x342D, 0xE3E8),
    (0x342D, 0xE3F5),
    (0x342D, 0xE3F6),
    (0x342D, 0xE3F7),
    (0x342D, 0xE3FB),
    (0x342D, 0xE3FC),
    (0x342D, 0xE3FD),
    (0x347A, 0xA302),
    (0x347A, 0xA305),
    (0x374A, 0xA205),
    (0x374A, 0xA217),
    (0x374A, 0xA220),
    (0x374A, 0xA223),
    (0x374A, 0xA226),
    (0x374A, 0xA229),
    (0x374A, 0xA234),
    (0x374A, 0xA237),
    (0x374A, 0xA251),
    (0x374A, 0xA263),
    (0x374A, 0xA268),
    (0x374A, 0xA311),
    (0x374A, 0xA314),
    (0x374A, 0xA319),
    (0x374A, 0xA322),
    (0x374A, 0xA329),
    (0x374A, 0xA355),
    (0x379A, 0x1804),
    (0x379A, 0x1807),
    (0x38A9, 0x0002),
    (0x38EE, 0x0000),
    (0x38EE, 0x0002),
    (0x38EE, 0x0039),
    (0x391D, 0x1A04),
    (0x391D, 0x1A08),
    (0x391D, 0x1A0E),
    (0x391D, 0x1A13),
    (0x391D, 0x1A25),
    (0x391D, 0x1A2B),
    (0x391D, 0x1A34),
    (0x391D, 0x2017),
    (0x391D, 0x2A05),
];

fn is_rongyuan_pair(vid: u16, pid: u16) -> bool {
    RONGYUAN_PAIRS.contains(&(vid, pid))
}

struct AjazzAjSeriesProvider;

impl Provider for AjazzAjSeriesProvider {
    fn id(&self) -> &str {
        "ajazz.aj_series"
    }

    fn metadata(&self) -> ProviderMetadata {
        experimental_metadata(capabilities(true, false, false), Duration::from_millis(300))
    }

    fn display_name(&self, endpoint: &Endpoint) -> Option<String> {
        Some(
            endpoint
                .product_name
                .clone()
                .filter(|name| !name.is_empty())
                .unwrap_or_else(|| "AJAZZ AJ-series mouse".to_owned()),
        )
    }

    fn matches(&self, endpoint: &Endpoint) -> MatchDecision {
        if endpoint.vid == Some(0x3151)
            && endpoint.pid.is_some_and(|p| !is_rongyuan_pair(0x3151, p))
            && endpoint.usage_page == Some(0xFFFF)
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
        let mut req = vec![0_u8; 65];
        req[1] = 0xF7;
        send_with_modes(session, device, &[SendMode::Feature], &req)?;
        sleep(deadline, Duration::from_millis(30))?;
        let r = feature_frame(session, device, 0x05, 64)?;
        if r.len() < 4 || r[0] != 0x05 || r[1] != 0 || r[2] != 0 {
            return Err(BatteryError::invalid_response(
                "invalid AJAZZ AJ-series response",
            ));
        }
        if r[3] == 0 {
            return Ok(ProviderReading {
                reachability: Reachability::Offline,
                source: ReadingSource::VendorHid,
                ..ProviderReading::default()
            });
        }
        Ok(reading(
            Some(checked_percentage(r[3])?),
            ChargeState::Unknown,
            None,
        ))
    }
}

struct IncottProvider;

impl Provider for IncottProvider {
    fn id(&self) -> &str {
        "incott.mouse"
    }

    fn metadata(&self) -> ProviderMetadata {
        experimental_metadata(capabilities(true, true, false), Duration::from_millis(300))
    }

    fn display_name(&self, _endpoint: &Endpoint) -> Option<String> {
        Some("Incott G23 V2 / G24".to_owned())
    }

    fn matches(&self, endpoint: &Endpoint) -> MatchDecision {
        if endpoint.vid == Some(0x093A)
            && matches!(endpoint.pid, Some(0x522C | 0x622C))
            && endpoint.usage_page == Some(0xFF05)
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
        let cable = device.endpoint.pid == Some(0x622C);
        let mut req = vec![0_u8; 9];
        req[0] = 0x09;
        req[1] = 0x89;
        send_with_modes(session, device, &[SendMode::Feature], &req)?;
        for _ in 0..10 {
            sleep(deadline, Duration::from_millis(50))?;
            let r = feature_frame(session, device, 0x09, 8)?;
            if r.len() >= 4 && r[0] == 0x09 && r[1] == 0x89 && (r[2] | r[3]) != 0 {
                let pct = checked_percentage(r[2] & 0x7F)?;
                if !cable && pct == 0 {
                    return Ok(ProviderReading {
                        reachability: Reachability::Offline,
                        source: ReadingSource::VendorHid,
                        ..ProviderReading::default()
                    });
                }
                let charge = if (r[2] & 0x80) != 0 {
                    ChargeState::Charging
                } else {
                    ChargeState::Discharging
                };
                return Ok(reading(Some(pct), charge, None));
            }
        }
        Err(BatteryError::timeout("Incott mouse"))
    }
}

struct RongyuanProvider;

impl Provider for RongyuanProvider {
    fn id(&self) -> &str {
        "rongyuan.receiver"
    }

    fn metadata(&self) -> ProviderMetadata {
        experimental_metadata(capabilities(true, false, false), Duration::from_millis(300))
    }

    fn display_name(&self, endpoint: &Endpoint) -> Option<String> {
        Some(
            endpoint
                .product_name
                .clone()
                .filter(|name| !name.is_empty())
                .unwrap_or_else(|| "Rongyuan wireless receiver".to_owned()),
        )
    }

    fn matches(&self, endpoint: &Endpoint) -> MatchDecision {
        let matched = endpoint
            .vid
            .zip(endpoint.pid)
            .is_some_and(|(v, p)| is_rongyuan_pair(v, p));
        if matched && endpoint.usage_page == Some(0xFFFF) && endpoint.usage == Some(0x0002) {
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
        let mut req = vec![0_u8; 65];
        req[1] = 0xF7;
        for _ in 0..3 {
            send_with_modes(session, device, &[SendMode::Feature], &req)?;
            sleep(deadline, Duration::from_millis(20))?;
            let r = feature_frame(session, device, 0x00, 64)?;
            if r.len() >= 7 && (r[2] | r[3] | r[4] | r[5] | r[6]) != 0 {
                let kb = (r[4] == 0 && (1..=100).contains(&r[2])).then_some(r[2]);
                let mouse = (r[5] == 0 && (1..=100).contains(&r[3])).then_some(r[3]);
                let pct = match (kb, mouse) {
                    (Some(a), Some(b)) => Some(a.min(b)),
                    (Some(a), None) => Some(a),
                    (None, Some(b)) => Some(b),
                    (None, None) => None,
                };
                if let Some(pct) = pct {
                    return Ok(reading(Some(pct), ChargeState::Unknown, None));
                }
                return Ok(ProviderReading {
                    reachability: Reachability::Offline,
                    source: ReadingSource::VendorHid,
                    ..ProviderReading::default()
                });
            }
            sleep(deadline, Duration::from_millis(100))?;
        }
        Err(BatteryError::timeout("Rongyuan receiver"))
    }
}
