//! Wireless keyboard vendor HID battery protocols:
//! Apple Magic Keyboard, NuPhy WH80, AULA HERO 68 Air, SONiX 0xFF60:0x0061 dongles, PMO/VIA 36B0,
//! and Semico 1A2C.

use crate::error::BatteryError;
use crate::model::{
    ChargeState, DeviceKind, Endpoint, ProviderReading, Reachability, ReadingSource, SupportStatus,
};
use crate::providers::fixed::{
    capabilities, charge_bool, checked_percentage, checksum_sum, drain_input,
    experimental_metadata, reading, send_with_modes, unnumbered_payload, wait_frame, FixedProvider,
    FixedSpec, OUTPUT,
};
use crate::providers::{Deadline, DeviceContext, MatchDecision, Provider, ProviderMetadata};
use crate::transport::HidSession;
use std::sync::Arc;
use std::time::Duration;

static WOB_RAINY75: FixedSpec = FixedSpec {
    id: "wob.rainy75",
    name: "WOB Rainy75",
    kind: DeviceKind::Keyboard,
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
        Arc::new(FixedProvider::new(&WOB_RAINY75)),
        Arc::new(AppleMagicProvider),
        Arc::new(NuphyWh80Provider),
        Arc::new(AulaHero68Provider),
        Arc::new(SonixKeyboardProvider),
        Arc::new(ViaDongleProvider),
        Arc::new(SemicoKeyboardProvider),
    ]
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

const APPLE_MAGIC_PIDS: &[u16] = &[
    // Magic Keyboards
    0x0267, 0x026C, 0x029A, 0x029C, 0x029F, 0x0320, 0x0321, 0x0322, // Magic Mice
    0x0269, 0x0323, // Magic Trackpads
    0x0265, 0x0324,
];

struct AppleMagicProvider;

impl Provider for AppleMagicProvider {
    fn id(&self) -> &str {
        "apple.magic"
    }

    fn metadata(&self) -> ProviderMetadata {
        experimental_metadata(
            DeviceKind::Keyboard,
            capabilities(true, false, false),
            Duration::from_millis(500),
        )
    }

    fn device_kind(&self, endpoint: &Endpoint) -> DeviceKind {
        match endpoint.pid {
            Some(0x0269 | 0x0323) => DeviceKind::Mouse,
            Some(0x0265 | 0x0324) => DeviceKind::Peripheral,
            _ => DeviceKind::Keyboard,
        }
    }

    fn display_name(&self, endpoint: &Endpoint) -> Option<String> {
        Some(match endpoint.pid {
            Some(0x0267) => "Apple Magic Keyboard 2".to_owned(),
            Some(0x026C) => "Apple Magic Keyboard with Numeric Keypad".to_owned(),
            Some(0x029A) => "Apple Magic Keyboard with Touch ID".to_owned(),
            Some(0x029C) => "Apple Magic Keyboard (2021)".to_owned(),
            Some(0x029F) => "Apple Magic Keyboard with Touch ID and Numeric Keypad".to_owned(),
            Some(0x0320) => "Apple Magic Keyboard (USB-C)".to_owned(),
            Some(0x0321) => "Apple Magic Keyboard with Touch ID (USB-C)".to_owned(),
            Some(0x0322) => {
                "Apple Magic Keyboard with Touch ID and Numeric Keypad (USB-C)".to_owned()
            }
            Some(0x0269) => "Apple Magic Mouse 2".to_owned(),
            Some(0x0323) => "Apple Magic Mouse (USB-C)".to_owned(),
            Some(0x0265) => "Apple Magic Trackpad 2".to_owned(),
            Some(0x0324) => "Apple Magic Trackpad (USB-C)".to_owned(),
            _ => endpoint
                .product_name
                .clone()
                .filter(|name| !name.is_empty())
                .unwrap_or_else(|| "Apple Magic Keyboard".to_owned()),
        })
    }

    fn matches(&self, endpoint: &Endpoint) -> MatchDecision {
        let collection_ok = endpoint
            .usage_page
            .is_some_and(|p| p >= 0xFF00 || p == 0x0006 || p == 0x000C)
            || (endpoint.usage_page.is_none() && endpoint.interface_number == Some(1));
        if matches!(endpoint.vid, Some(0x004C | 0x05AC))
            && endpoint.pid.is_some_and(|p| APPLE_MAGIC_PIDS.contains(&p))
            && collection_ok
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
        deadline.check("Apple Magic")?;
        let payload = session
            .get_input_report(0x90, 7)
            .map_err(|error| error.into_battery(&device.endpoint.id))?;
        if payload.len() < 2 {
            return Err(BatteryError::invalid_response(
                "truncated Apple Magic battery report",
            ));
        }
        Ok(reading(
            Some(checked_percentage(payload[1])?),
            ChargeState::Unknown,
            None,
        ))
    }
}

struct NuphyWh80Provider;

impl Provider for NuphyWh80Provider {
    fn id(&self) -> &str {
        "nuphy.wh80"
    }

    fn metadata(&self) -> ProviderMetadata {
        experimental_metadata(
            DeviceKind::Keyboard,
            capabilities(true, true, false),
            Duration::from_millis(400),
        )
    }

    fn display_name(&self, _endpoint: &Endpoint) -> Option<String> {
        Some("NuPhy WH80".to_owned())
    }

    fn matches(&self, endpoint: &Endpoint) -> MatchDecision {
        if endpoint.vid == Some(0x19F5)
            && endpoint.pid == Some(0xA011)
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
        let mut req = vec![0_u8; 65];
        req[1..9].copy_from_slice(&[0x55, 0x05, 0x00, 0x21, 0x01, 0x20, 0x00, 0x00]);
        send_with_modes(session, device, OUTPUT, &req)?;
        let frame = wait_frame(
            session,
            device,
            deadline,
            Duration::from_millis(400),
            12,
            |f| {
                let r = unnumbered_payload(f);
                Ok(r.len() == 64
                    && r[..3] == [0xAA, 0x05, 0x00]
                    && r[4..8] == [0x01, 0x20, 0x00, 0x00])
            },
        )?;
        let r = unnumbered_payload(&frame);
        if r[3] != checksum_sum(&r[4..64]) {
            return Err(BatteryError::ChecksumFailed);
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

struct AulaHero68Provider;

impl Provider for AulaHero68Provider {
    fn id(&self) -> &str {
        "aula.hero68"
    }

    fn metadata(&self) -> ProviderMetadata {
        experimental_metadata(
            DeviceKind::Keyboard,
            capabilities(true, true, false),
            Duration::from_millis(500),
        )
    }

    fn display_name(&self, _endpoint: &Endpoint) -> Option<String> {
        Some("AULA HERO 68 Air".to_owned())
    }

    fn matches(&self, endpoint: &Endpoint) -> MatchDecision {
        if endpoint.vid == Some(0x372E)
            && endpoint.pid == Some(0x106C)
            && endpoint.usage_page == Some(0xFF60)
            && endpoint.usage == Some(0x0061)
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
        let mut msg = [0_u8; 56];
        msg[..5].copy_from_slice(&[0x09, 0x87, 0x00, 0x00, 0x01]);
        let mut counter: u8 = 1;
        for idx in 1..=4_u8 {
            let mut f = vec![0_u8; 20];
            f[0] = 0x09;
            f[1] = 0x66;
            f[2] = 0x05 | if (counter & 4) != 0 { 0x80 } else { 0 };
            f[3] = idx | if (counter & 2) != 0 { 0x80 } else { 0 };
            f[4] = 0x0E | if (counter & 1) != 0 { 0x80 } else { 0 };
            let off = usize::from(idx - 1) * 14;
            f[5..19].copy_from_slice(&msg[off..off + 14]);
            f[19] = checksum_sum(&f[1..19]);
            send_with_modes(session, device, OUTPUT, &f)?;
            counter = counter.wrapping_add(1) & 7;
        }
        let mut trailer = vec![0_u8; 20];
        trailer[0] = 0x09;
        trailer[1] = 0x66;
        trailer[2] = 0x05 | if (counter & 4) != 0 { 0x80 } else { 0 };
        trailer[3] = 5 | if (counter & 2) != 0 { 0x80 } else { 0 };
        trailer[4] = 0x08 | if (counter & 1) != 0 { 0x80 } else { 0 };
        trailer[12] = 0xFF_u8.wrapping_sub(checksum_sum(&msg));
        trailer[13] = checksum_sum(&trailer[1..13]);
        send_with_modes(session, device, OUTPUT, &trailer)?;

        let r = wait_frame(
            session,
            device,
            deadline,
            Duration::from_millis(500),
            16,
            |r| {
                if r.len() < 20 || r[0] != 0x09 || r[1] != 0x66 || (r[3] & 0x7F) != 1 {
                    return Ok(false);
                }
                if r[19] != checksum_sum(&r[1..19]) {
                    return Err(BatteryError::ChecksumFailed);
                }
                Ok(r[5] == 0x87 || (r[5] == 0xFE && r[6] == 0x05))
            },
        )?;
        let (pct_byte, flags) = if r[5] == 0x87 {
            (r[11], r[12])
        } else {
            (r[7], r[8])
        };
        if pct_byte == 0 || pct_byte > 100 {
            return Err(BatteryError::invalid_response(
                "invalid AULA HERO 68 battery percentage",
            ));
        }
        let charge = if (flags & 0x10) != 0 {
            ChargeState::Charging
        } else {
            ChargeState::Discharging
        };
        Ok(reading(Some(pct_byte), charge, None))
    }
}

const SONIX_IF3_PAIRS: &[(u16, u16)] = &[(0x05AC, 0x024F), (0x0C45, 0xFEFE), (0x0C45, 0xFDFD)];
const SONIX_ANY_IF_PAIRS: &[(u16, u16)] = &[
    (0x342D, 0xE479),
    (0x342D, 0xE485),
    (0x342D, 0xE4BC),
    (0x342D, 0xE4C2),
    (0x342D, 0xE407),
    (0x388D, 0x0003),
    (0x0DB0, 0x0CA0),
    (0x36B0, 0x30BD),
    (0x342D, 0xE51A),
    (0x342D, 0xE487),
    (0x342D, 0xE537),
    (0x388D, 0x0002),
    (0x19F5, 0x3245),
    (0x362D, 0xD028),
    (0x362D, 0xD030),
    (0x362D, 0xD101),
    (0x342D, 0xE49F),
];

struct SonixKeyboardProvider;

impl Provider for SonixKeyboardProvider {
    fn id(&self) -> &str {
        "sonix.keyboard"
    }

    fn metadata(&self) -> ProviderMetadata {
        experimental_metadata(
            DeviceKind::Keyboard,
            capabilities(true, false, false),
            Duration::from_millis(500),
        )
    }

    fn display_name(&self, endpoint: &Endpoint) -> Option<String> {
        Some(
            endpoint
                .product_name
                .clone()
                .filter(|name| !name.is_empty())
                .unwrap_or_else(|| "SONiX wireless keyboard".to_owned()),
        )
    }

    fn matches(&self, endpoint: &Endpoint) -> MatchDecision {
        if endpoint.usage_page != Some(0xFF60) || endpoint.usage != Some(0x0061) {
            return MatchDecision::NoMatch;
        }
        let Some((v, p)) = endpoint.vid.zip(endpoint.pid) else {
            return MatchDecision::NoMatch;
        };
        let matched = (SONIX_IF3_PAIRS.contains(&(v, p)) && endpoint.interface_number == Some(3))
            || SONIX_ANY_IF_PAIRS.contains(&(v, p));
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
        let mut req = vec![0_u8; 33];
        req[1] = 0x20;
        req[2] = 0x01;
        req[32] = 0x21;
        send_with_modes(session, device, OUTPUT, &req)?;
        let frame = wait_frame(
            session,
            device,
            deadline,
            Duration::from_millis(500),
            12,
            |f| {
                let r = unnumbered_payload(f);
                Ok(r.len() >= 4 && r[0] == 0x20 && r[1] == 0x01 && r[2] == 0x00)
            },
        )?;
        let r = unnumbered_payload(&frame);
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

struct ViaDongleProvider;

impl Provider for ViaDongleProvider {
    fn id(&self) -> &str {
        "via.dongle"
    }

    fn metadata(&self) -> ProviderMetadata {
        experimental_metadata(
            DeviceKind::Keyboard,
            capabilities(true, false, false),
            Duration::from_millis(500),
        )
    }

    fn display_name(&self, endpoint: &Endpoint) -> Option<String> {
        Some(match endpoint.pid {
            Some(0x3009) => "PMO Wave 75".to_owned(),
            Some(0x3004) => "PMO Aurora 80".to_owned(),
            Some(0x302B) => "PMO P75".to_owned(),
            Some(0x3012) => "PMO 80".to_owned(),
            _ => endpoint
                .product_name
                .clone()
                .filter(|name| !name.is_empty())
                .unwrap_or_else(|| "PMO / VIA wireless keyboard".to_owned()),
        })
    }

    fn matches(&self, endpoint: &Endpoint) -> MatchDecision {
        if endpoint.vid == Some(0x36B0)
            && matches!(
                endpoint.pid,
                Some(0x3002 | 0x3004 | 0x3009 | 0x3012 | 0x302B)
            )
            && endpoint.usage_page == Some(0xFF60)
            && endpoint.usage == Some(0x0061)
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
        let prefer_via = device.endpoint.pid == Some(0x3002)
            && device
                .endpoint
                .product_name
                .as_deref()
                .is_some_and(|n| n.to_ascii_uppercase().contains("URX"));
        if prefer_via {
            return read_via_custom(session, device, deadline);
        }
        match read_pmo_hub(session, device, deadline) {
            Ok(reading) => Ok(reading),
            Err(BatteryError::Timeout { .. }) if device.endpoint.pid == Some(0x3002) => {
                read_via_custom(session, device, deadline)
            }
            Err(error) => Err(error),
        }
    }
}

fn read_pmo_hub(
    session: &mut dyn HidSession,
    device: &DeviceContext<'_>,
    deadline: Deadline,
) -> Result<ProviderReading, BatteryError> {
    let ask = |session: &mut dyn HidSession, cmd: u8| {
        let mut req = vec![0_u8; 33];
        req[1] = 0xAA;
        req[2] = cmd;
        req[5] = 0x18;
        send_with_modes(session, device, OUTPUT, &req)?;
        wait_frame(
            session,
            device,
            deadline,
            Duration::from_millis(400),
            8,
            |f| {
                let r = unnumbered_payload(f);
                Ok(r.len() >= 9 && r[0] == 0xAA && r[1] == cmd && r[2] == 0 && r[3] == 0)
            },
        )
    };
    drain_input(session, device, deadline, 6)?;
    let start = ask(session, 0x10)?;
    if unnumbered_payload(&start)[7] != 0x55 {
        return Err(BatteryError::invalid_response(
            "PMO Hub START_COMM rejected",
        ));
    }
    let bat = ask(session, 0x30);
    let _ = ask(session, 0x11);
    let frame = bat?;
    let r = unnumbered_payload(&frame);
    if r[7] != 0x55 {
        return Err(BatteryError::invalid_response(
            "PMO Hub GET_BATTERY rejected",
        ));
    }
    Ok(reading(
        Some(checked_percentage(r[8])?),
        ChargeState::Unknown,
        None,
    ))
}

fn read_via_custom(
    session: &mut dyn HidSession,
    device: &DeviceContext<'_>,
    deadline: Deadline,
) -> Result<ProviderReading, BatteryError> {
    drain_input(session, device, deadline, 6)?;
    let mut req = vec![0_u8; 33];
    req[1..4].copy_from_slice(&[0x08, 0x00, 0x07]);
    send_with_modes(session, device, OUTPUT, &req)?;
    let frame = wait_frame(
        session,
        device,
        deadline,
        Duration::from_millis(500),
        12,
        |f| {
            let r = unnumbered_payload(f);
            Ok(r.len() >= 4 && r[..3] == [0x08, 0x00, 0x07])
        },
    )?;
    let r = unnumbered_payload(&frame);
    if r[3] == 0 || r[3] > 100 {
        return Err(BatteryError::invalid_response(
            "invalid VIA battery percentage",
        ));
    }
    Ok(reading(Some(r[3]), ChargeState::Unknown, None))
}

struct SemicoKeyboardProvider;

impl Provider for SemicoKeyboardProvider {
    fn id(&self) -> &str {
        "semico.keyboard"
    }

    fn metadata(&self) -> ProviderMetadata {
        experimental_metadata(
            DeviceKind::Keyboard,
            capabilities(true, false, false),
            Duration::from_millis(500),
        )
    }

    fn display_name(&self, endpoint: &Endpoint) -> Option<String> {
        Some(match endpoint.pid {
            Some(0xA034) => "Attack Shark X98".to_owned(),
            Some(0xA036) => "AJAZZ AK820 Max Plus".to_owned(),
            _ => endpoint
                .product_name
                .clone()
                .filter(|name| !name.is_empty())
                .unwrap_or_else(|| "Semico wireless keyboard".to_owned()),
        })
    }

    fn matches(&self, endpoint: &Endpoint) -> MatchDecision {
        if endpoint.vid == Some(0x1A2C)
            && matches!(endpoint.pid, Some(0x8FFF | 0xA034 | 0xA036))
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
        let mut req = vec![0_u8; 65];
        req[1..6].copy_from_slice(&[0xAA, 0x55, 0xCC, 0x33, 0x0E]);
        send_with_modes(session, device, OUTPUT, &req)?;
        let frame = wait_frame(
            session,
            device,
            deadline,
            Duration::from_millis(500),
            12,
            |f| {
                let r = unnumbered_payload(f);
                Ok(r.len() >= 7 && r[..5] == [0xAA, 0x55, 0xCC, 0x33, 0x0E])
            },
        )?;
        let r = unnumbered_payload(&frame);
        match r[6] {
            0 => Ok(ProviderReading {
                reachability: Reachability::Offline,
                source: ReadingSource::VendorHid,
                ..ProviderReading::default()
            }),
            1..=3 => Ok(reading(
                Some(checked_percentage(r[5])?),
                ChargeState::Unknown,
                None,
            )),
            _ => Err(BatteryError::invalid_response(
                "invalid Semico connection state",
            )),
        }
    }
}
