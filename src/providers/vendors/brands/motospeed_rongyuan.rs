//! Motospeed, Xenta, Delux, Ajazz, Incott, Rongyuan, TAIDU, CROCIRIS, and Press Play mouse protocols.

use crate::error::BatteryError;
use crate::model::{
    ChargeState, DeviceKind, Endpoint, ProviderReading, Reachability, ReadingSource, SupportStatus,
};
use crate::providers::fixed::{
    capabilities, charge_bool, checked_percentage, checksum_sum, drain_input,
    experimental_metadata, feature_frame, reading, send_with_modes, sleep, wait_frame,
    FixedProvider, FixedSpec, SendMode, OUTPUT, OUTPUT_THEN_SET, SET_OUTPUT,
};
use crate::providers::{
    infer_device_kind_from_name, Deadline, DeviceContext, MatchDecision, Provider, ProviderMetadata,
};
use crate::transport::HidSession;
use std::sync::Arc;
use std::time::Duration;

static TAIDU: FixedSpec = FixedSpec {
    id: "taidu.mouse",
    name: "TAIDU mouse",
    kind: DeviceKind::Mouse,
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
    kind: DeviceKind::Mouse,
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

pub(crate) fn providers() -> Vec<Arc<dyn Provider>> {
    vec![
        Arc::new(FixedProvider::new(&TAIDU)),
        Arc::new(FixedProvider::new(&X7)),
        Arc::new(PressPlayIcarusProvider),
        Arc::new(MotospeedX6Provider),
        Arc::new(XentaProvider),
        Arc::new(DeluxM800Provider),
        Arc::new(AjazzAj179Provider),
        Arc::new(AjazzAjSeriesProvider),
        Arc::new(IncottProvider),
        Arc::new(RongyuanProvider),
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

struct PressPlayIcarusProvider;

impl Provider for PressPlayIcarusProvider {
    fn id(&self) -> &str {
        "pressplay.icarus"
    }

    fn metadata(&self) -> ProviderMetadata {
        experimental_metadata(
            DeviceKind::Mouse,
            capabilities(true, false, false),
            Duration::from_millis(500),
        )
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

struct MotospeedX6Provider;

impl Provider for MotospeedX6Provider {
    fn id(&self) -> &str {
        "motospeed.x6"
    }

    fn metadata(&self) -> ProviderMetadata {
        experimental_metadata(
            DeviceKind::Mouse,
            capabilities(true, true, false),
            Duration::from_millis(600),
        )
    }

    fn display_name(&self, _endpoint: &Endpoint) -> Option<String> {
        Some("Motospeed X6".to_owned())
    }

    fn matches(&self, endpoint: &Endpoint) -> MatchDecision {
        if endpoint.vid == Some(0x0BDA)
            && matches!(endpoint.pid, Some(0xFFE0 | 0xFFF2))
            && endpoint.interface_number == Some(4)
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
        experimental_metadata(
            DeviceKind::Mouse,
            capabilities(true, true, false),
            Duration::from_millis(1500),
        )
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
        experimental_metadata(
            DeviceKind::Mouse,
            capabilities(true, true, false),
            Duration::from_millis(300),
        )
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
        experimental_metadata(
            DeviceKind::Mouse,
            capabilities(true, true, false),
            Duration::from_millis(600),
        )
    }

    fn display_name(&self, _endpoint: &Endpoint) -> Option<String> {
        Some("AJAZZ AJ179".to_owned())
    }

    fn matches(&self, endpoint: &Endpoint) -> MatchDecision {
        if endpoint.vid == Some(0x249A)
            && endpoint.interface_number == Some(2)
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
        use crate::providers::fixed::unnumbered_payload;
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
        experimental_metadata(
            DeviceKind::Mouse,
            capabilities(true, false, false),
            Duration::from_millis(300),
        )
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
        // Explicit Rongyuan pairs on VID 0x3151 share 0xFFFF:0x0002 and win via MatchDecision::Exact.
        if endpoint.vid == Some(0x3151)
            && endpoint.usage_page == Some(0xFFFF)
            && endpoint.usage == Some(0x0002)
        {
            MatchDecision::Family
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
        experimental_metadata(
            DeviceKind::Mouse,
            capabilities(true, true, false),
            Duration::from_millis(300),
        )
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
        experimental_metadata(
            DeviceKind::Peripheral,
            capabilities(true, false, false),
            Duration::from_millis(300),
        )
    }

    fn device_kind(&self, endpoint: &Endpoint) -> DeviceKind {
        infer_device_kind_from_name(endpoint.product_name.as_deref())
            .unwrap_or(DeviceKind::Peripheral)
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
