//! MCHOSE keyboard and wireless mouse battery protocols (keyboard, K7 V2 Ultra+, A5, A7 Ultra).

use crate::error::BatteryError;
use crate::model::{
    ChargeState, Endpoint, ProviderReading, Reachability, ReadingSource, SupportStatus,
};
use crate::providers::fixed::{
    capabilities, charge_bool, checked_percentage, checksum_sum, drain_input,
    experimental_metadata, feature_frame, frame, query, reading, send_with_modes, sleep,
    unnumbered_payload, wait_frame, FixedProvider, FixedSpec, SendMode, OUTPUT, SET_OUTPUT,
};
use crate::providers::{Deadline, DeviceContext, MatchDecision, Provider, ProviderMetadata};
use crate::transport::HidSession;
use std::sync::Arc;
use std::time::Duration;

static MCHOSE_KEYBOARD: FixedSpec = FixedSpec {
    id: "mchose.keyboard",
    name: "MCHOSE keyboard",
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
        Arc::new(MchoseK7Provider),
        Arc::new(MchoseFeatureProvider::a5()),
        Arc::new(MchoseFeatureProvider::a7()),
        Arc::new(MchoseA5Gen1Provider),
        Arc::new(MchoseG3V2Provider),
        Arc::new(MchoseV9Pro2Provider),
        Arc::new(MchoseGlwKeyboardProvider),
        Arc::new(MchoseModernProvider::mouse()),
        Arc::new(MchoseModernProvider::k87_v3()),
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

struct MchoseK7Provider;

impl Provider for MchoseK7Provider {
    fn id(&self) -> &str {
        "mchose.k7_v2_ultra"
    }

    fn metadata(&self) -> ProviderMetadata {
        experimental_metadata(capabilities(true, true, false), Duration::from_millis(100))
    }

    fn display_name(&self, _endpoint: &Endpoint) -> Option<String> {
        Some("MCHOSE K7 V2 Ultra+".to_owned())
    }

    fn matches(&self, endpoint: &Endpoint) -> MatchDecision {
        let on_mi02 = endpoint.interface_number == Some(2)
            || endpoint
                .system_path
                .to_ascii_lowercase()
                .contains("&mi_02&");
        if endpoint.vid == Some(0x3837)
            && endpoint.pid == Some(0x1014)
            && endpoint.usage_page == Some(0xFF01)
            && endpoint.usage == Some(1)
            && on_mi02
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
        let mut request = vec![0_u8; 64];
        request[0] = 0x4D;
        request[1] = 1;
        request[2] = 1;
        request[4] = 0x00;
        request[5] = 0x09;
        request[8] = request[2] ^ request[3] ^ request[4] ^ request[5] ^ request[6] ^ request[7];
        let frame = query(
            session,
            device,
            deadline,
            request,
            Duration::from_millis(100),
            |frame| Ok(frame.len() >= 6 && frame[4] == 0 && frame[5] == 9),
        )?;
        parse_k7(&frame)
    }
}

fn parse_k7(frame: &[u8]) -> Result<ProviderReading, BatteryError> {
    if frame.len() != 64
        || frame[0..4] != [0x4D, 1, 1, 0x10]
        || frame[6] != 0
        || frame[25..].iter().any(|byte| *byte != 0)
    {
        return Err(BatteryError::invalid_response("invalid MCHOSE K7 response"));
    }
    if frame[24] != frame[2..=23].iter().fold(0_u8, |sum, byte| sum ^ byte) {
        return Err(BatteryError::ChecksumFailed);
    }
    if u16::from_le_bytes([frame[8], frame[9]]) != 0x3837
        || u16::from_le_bytes([frame[10], frame[11]]) != 0x4026
    {
        return Err(BatteryError::invalid_response(
            "MCHOSE K7 identity mismatch",
        ));
    }
    if frame[19] > 1 {
        return Err(BatteryError::invalid_response(
            "invalid MCHOSE K7 charge state",
        ));
    }
    let percentage = checked_percentage(frame[20])?;
    if frame[18] == 0 {
        return Ok(ProviderReading {
            reachability: Reachability::Offline,
            source: ReadingSource::VendorHid,
            ..ProviderReading::default()
        });
    }
    if frame[18] != 2 {
        return Err(BatteryError::invalid_response(
            "invalid MCHOSE K7 connection state",
        ));
    }
    Ok(reading(Some(percentage), charge_bool(frame[19]), None))
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum MchoseFeatureKind {
    A5,
    A7,
}

struct MchoseFeatureProvider {
    kind: MchoseFeatureKind,
}

impl MchoseFeatureProvider {
    const fn a5() -> Self {
        Self {
            kind: MchoseFeatureKind::A5,
        }
    }

    const fn a7() -> Self {
        Self {
            kind: MchoseFeatureKind::A7,
        }
    }

    fn expected_vid(&self) -> u16 {
        match self.kind {
            MchoseFeatureKind::A5 => 0x3837,
            MchoseFeatureKind::A7 => 0x5253,
        }
    }
}

impl Provider for MchoseFeatureProvider {
    fn id(&self) -> &str {
        match self.kind {
            MchoseFeatureKind::A5 => "mchose.a5_mouse",
            MchoseFeatureKind::A7 => "mchose.a7_ultra",
        }
    }

    fn metadata(&self) -> ProviderMetadata {
        experimental_metadata(capabilities(true, true, false), Duration::from_millis(180))
    }

    fn display_name(&self, _endpoint: &Endpoint) -> Option<String> {
        Some(match self.kind {
            MchoseFeatureKind::A5 => "MCHOSE A5 mouse".to_owned(),
            MchoseFeatureKind::A7 => "MCHOSE A7 Ultra".to_owned(),
        })
    }

    fn matches(&self, endpoint: &Endpoint) -> MatchDecision {
        let matches = endpoint.usage_page == Some(0xFF01)
            && match self.kind {
                MchoseFeatureKind::A5 => {
                    endpoint.vid == Some(0x3837)
                        && !matches!(
                            endpoint.pid,
                            Some(
                                0x1014
                                    | 0x1015
                                    | 0x1016
                                    | 0x1018
                                    | 0x102C
                                    | 0x102D
                                    | 0x4028
                                    | 0x418C
                                    | 0x4245
                                    | 0x3033
                                    | 0x600A
                                    | 0x6011
                            )
                        )
                }
                MchoseFeatureKind::A7 => endpoint.vid == Some(0x5253),
            };
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
        mchose_feature_command(session, device, 0x03)?;
        sleep(deadline, Duration::from_millis(120))?;
        for _ in 0..3 {
            mchose_feature_command(session, device, 0x06)?;
            if self.kind == MchoseFeatureKind::A5 {
                wait_mchose_ack(session, device, deadline)?;
            } else {
                sleep(deadline, Duration::from_millis(80))?;
            }
            for _ in 0..4 {
                let frame = feature_frame(session, device, 0x11, 64)?;
                if let Some(payload) = decode_mchose_payload(&frame)? {
                    return parse_mchose_payload(&payload, self.expected_vid());
                }
            }
        }
        Err(BatteryError::timeout("MCHOSE feature query"))
    }
}

fn mchose_feature_command(
    session: &mut dyn HidSession,
    device: &DeviceContext<'_>,
    command: u8,
) -> Result<(), BatteryError> {
    let mut frame = vec![0xFF_u8; 65];
    frame[0] = 0x11;
    frame[1] = command ^ 0xFF;
    send_with_modes(session, device, &[SendMode::Feature], &frame)
}

fn wait_mchose_ack(
    session: &mut dyn HidSession,
    device: &DeviceContext<'_>,
    deadline: Deadline,
) -> Result<(), BatteryError> {
    for _ in 0..16 {
        let timeout = deadline
            .remaining("MCHOSE ACK")?
            .min(Duration::from_millis(120));
        let report = session
            .read_input(timeout)
            .map_err(|error| error.into_battery(&device.endpoint.id))?;
        let frame = frame(report);
        if frame.len() >= 3 && frame[1] ^ 0xFF == 0xE5 && frame[2] ^ 0xFF == 0x06 {
            return Ok(());
        }
    }
    Err(BatteryError::timeout("MCHOSE ACK"))
}

fn decode_mchose_payload(frame: &[u8]) -> Result<Option<Vec<u8>>, BatteryError> {
    if frame.len() != 65 || frame[0] != 0x11 {
        return Err(BatteryError::invalid_response(
            "invalid MCHOSE feature length",
        ));
    }
    let xor = frame[1] == (0x06 ^ 0xFF);
    let plain = frame[1] == 0x06;
    if !xor && !plain {
        return Ok(None);
    }
    Ok(Some(
        frame[2..]
            .iter()
            .map(|byte| if xor { byte ^ 0xFF } else { *byte })
            .collect(),
    ))
}

fn parse_mchose_payload(
    payload: &[u8],
    expected_vid: u16,
) -> Result<ProviderReading, BatteryError> {
    if payload.len() < 11 {
        return Err(BatteryError::invalid_response(
            "truncated MCHOSE OST payload",
        ));
    }
    let vid = u16::from_le_bytes([payload[0], payload[1]]);
    if vid != expected_vid {
        return Err(BatteryError::invalid_response("MCHOSE OST vendor mismatch"));
    }
    if payload[9] == 0 {
        return Ok(ProviderReading {
            reachability: Reachability::Offline,
            source: ReadingSource::VendorHid,
            ..ProviderReading::default()
        });
    }
    Ok(reading(
        Some(checked_percentage(payload[9])?),
        charge_bool(payload[10]),
        None,
    ))
}

struct MchoseA5Gen1Provider;

impl Provider for MchoseA5Gen1Provider {
    fn id(&self) -> &str {
        "mchose.a5_gen1"
    }

    fn metadata(&self) -> ProviderMetadata {
        experimental_metadata(capabilities(true, true, false), Duration::from_millis(300))
    }

    fn display_name(&self, endpoint: &Endpoint) -> Option<String> {
        Some(match endpoint.pid {
            Some(0xF017) => "MCHOSE A5 Pro".to_owned(),
            Some(0xF018 | 0xF019) => "MCHOSE A5 Pro Max".to_owned(),
            _ => "MCHOSE A5 Pro / Pro Max".to_owned(),
        })
    }

    fn matches(&self, endpoint: &Endpoint) -> MatchDecision {
        if endpoint.vid == Some(0x2023)
            && matches!(
                endpoint.pid,
                Some(0xF013 | 0xF015 | 0xF017 | 0xF018 | 0xF019)
            )
            && endpoint.usage_page == Some(0xFFFF)
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
        let mut request = vec![0_u8; 65];
        request[3] = 0x02;
        request[4] = 0x02;
        request[6] = 0x83;
        send_with_modes(session, device, &[SendMode::Feature], &request)?;
        sleep(deadline, Duration::from_millis(34))?;
        for attempt in 0..8 {
            if attempt == 3 {
                send_with_modes(session, device, &[SendMode::Feature], &request)?;
                sleep(deadline, Duration::from_millis(34))?;
            }
            let frame = feature_frame(session, device, 0x00, 64)?;
            if frame.len() >= 9 {
                let at = if frame[1] == 0xA1 {
                    Some(1)
                } else if frame[0] == 0xA1 {
                    Some(0)
                } else {
                    None
                };
                if let Some(at) = at {
                    if frame.len() > at + 7 && frame[at + 4] == 0x00 && frame[at + 5] == 0x83 {
                        let lv = frame[at + 7];
                        if lv != 0xFF {
                            let pct = lv.min(100);
                            let charge = if frame[at + 6] != 0 {
                                ChargeState::Charging
                            } else {
                                ChargeState::Discharging
                            };
                            return Ok(reading(Some(pct), charge, None));
                        }
                    }
                }
            }
            sleep(deadline, Duration::from_millis(30))?;
        }
        Err(BatteryError::timeout("MCHOSE A5 Gen1"))
    }
}

struct MchoseG3V2Provider;

impl Provider for MchoseG3V2Provider {
    fn id(&self) -> &str {
        "mchose.g3_v2"
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
                .unwrap_or_else(|| "MCHOSE G3 V2".to_owned()),
        )
    }

    fn matches(&self, endpoint: &Endpoint) -> MatchDecision {
        if endpoint.vid == Some(0x3837)
            && matches!(endpoint.pid, Some(0x418C | 0x4245))
            && endpoint.usage_page == Some(0xFF01)
            && endpoint.usage == Some(0x0010)
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
        let mut request = vec![0_u8; 65];
        request[1..9].copy_from_slice(&[0x55, 0x30, 0xA5, 0x0B, 0x2E, 0x01, 0x01, 0x01]);
        send_with_modes(session, device, OUTPUT, &request)?;
        let frame = wait_frame(
            session,
            device,
            deadline,
            Duration::from_millis(500),
            16,
            |frame| {
                let r = unnumbered_payload(frame);
                Ok(r.len() >= 10 && r[0] == 0xAA && r[1] == 0x30)
            },
        )?;
        let r = unnumbered_payload(&frame);
        if r[8] == 0 || r[8] > 100 {
            return Err(BatteryError::invalid_response(
                "invalid MCHOSE G3 V2 battery byte",
            ));
        }
        Ok(reading(Some(r[8]), charge_bool(r[9]), None))
    }
}

struct MchoseV9Pro2Provider;

impl Provider for MchoseV9Pro2Provider {
    fn id(&self) -> &str {
        "mchose.v9_pro2"
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
        experimental_metadata(capabilities(true, true, false), Duration::from_millis(500))
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

struct MchoseModernProvider {
    keyboard: bool,
}

impl MchoseModernProvider {
    const fn mouse() -> Self {
        Self { keyboard: false }
    }

    const fn k87_v3() -> Self {
        Self { keyboard: true }
    }
}

impl Provider for MchoseModernProvider {
    fn id(&self) -> &str {
        if self.keyboard {
            "mchose.k87_v3"
        } else {
            "mchose.modern_mouse"
        }
    }

    fn metadata(&self) -> ProviderMetadata {
        experimental_metadata(capabilities(true, true, false), Duration::from_millis(250))
    }

    fn display_name(&self, endpoint: &Endpoint) -> Option<String> {
        Some(
            endpoint
                .product_name
                .clone()
                .filter(|name| !name.is_empty())
                .unwrap_or_else(|| {
                    if self.keyboard {
                        "MCHOSE K87 V3".to_owned()
                    } else {
                        "MCHOSE mouse".to_owned()
                    }
                }),
        )
    }

    fn matches(&self, endpoint: &Endpoint) -> MatchDecision {
        if endpoint.vid != Some(0x3837) {
            return MatchDecision::NoMatch;
        }
        let matched = if self.keyboard {
            endpoint.pid == Some(0x1015)
                && endpoint.usage_page == Some(0xFF80)
                && endpoint.usage == Some(0x0001)
        } else {
            matches!(
                endpoint.pid,
                Some(0x1016 | 0x1018 | 0x102C | 0x102D | 0x4028)
            ) && matches!(endpoint.usage_page, Some(0xFF01) | Some(0xFF0B))
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
        let mut request = vec![0_u8; 64];
        request[..9].copy_from_slice(&[0x4D, 0x01, 0x01, 0x00, 0x00, 0x09, 0x00, 0x00, 0x08]);
        let frame = query(
            session,
            device,
            deadline,
            request,
            Duration::from_millis(250),
            |frame| {
                Ok(frame.len() >= 10 && frame[0] == 0x4D && frame[4] == 0x00 && frame[5] == 0x09)
            },
        )?;
        let len = usize::from(frame[3]);
        if 8 + len >= frame.len() {
            return Err(BatteryError::invalid_response(
                "truncated MCHOSE modern response",
            ));
        }
        if len < 14 {
            if !self.keyboard || (len == 1 && frame[8] == 0xFF) {
                return Ok(ProviderReading {
                    reachability: Reachability::Sleeping,
                    source: ReadingSource::VendorHid,
                    ..ProviderReading::default()
                });
            }
            return Err(BatteryError::invalid_response(
                "invalid MCHOSE modern short response",
            ));
        }
        let xor = frame[2..8 + len].iter().fold(0_u8, |acc, byte| acc ^ byte);
        if frame[8 + len] != xor {
            return Err(BatteryError::ChecksumFailed);
        }
        if u16::from_le_bytes([frame[8], frame[9]]) != 0x3837 {
            return Err(BatteryError::invalid_response(
                "MCHOSE modern vendor mismatch",
            ));
        }
        if self.keyboard && (len < 20 || frame[10] != 0x23 || frame[11] != 0x20) {
            return Err(BatteryError::invalid_response(
                "MCHOSE K87 V3 model mismatch",
            ));
        }
        if frame[18] == 0 {
            return Ok(ProviderReading {
                reachability: Reachability::Sleeping,
                source: ReadingSource::VendorHid,
                ..ProviderReading::default()
            });
        }
        let percentage = checked_percentage(frame[20])?;
        Ok(reading(Some(percentage), charge_bool(frame[19]), None))
    }
}
