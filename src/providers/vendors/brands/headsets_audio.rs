//! Wireless audio headset and microphone HID battery protocols:
//! Jabra GNP, JBL Quantum (HIDV1 & HIDV3), Poly/Plantronics BT600, Fractal Scape,
//! Alienware AW920H, MOONDROP GM01 Pro, XIBERIA T7, MAONO PD200W, DARKRIM SHADOW,
//! Lenovo Wireless VoIP, EPOS H3PRO Hybrid, FIFINE HT6, and MCHOSE X9 / V9 Pro.

use crate::error::BatteryError;
use crate::model::{
    ChargeState, DeviceKind, Endpoint, ProviderReading, Reachability, ReadingSource, SupportStatus,
};
use crate::providers::fixed::{
    capabilities, checked_percentage, crc16_poly_1021, drain_input, experimental_metadata,
    feature_frame, input_frame, reading, send_with_modes, sleep, unnumbered_payload, wait_frame,
    FixedProvider, FixedSpec, SendMode, OUTPUT, OUTPUT_THEN_SET,
};
use crate::providers::{Deadline, DeviceContext, MatchDecision, Provider, ProviderMetadata};
use crate::transport::HidSession;
use std::sync::Arc;
use std::time::Duration;

static XIBERIA_T7: FixedSpec = FixedSpec {
    id: "xiberia.t7",
    name: "XIBERIA T7",
    kind: DeviceKind::Headset,
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

pub(crate) fn providers() -> Vec<Arc<dyn Provider>> {
    vec![
        Arc::new(FixedProvider::new(&XIBERIA_T7)),
        Arc::new(JabraGnpProvider),
        Arc::new(JblHidv1Provider),
        Arc::new(JblHidv3Provider),
        Arc::new(PlantronicsProvider),
        Arc::new(FractalScapeProvider),
        Arc::new(AlienwareAw920hProvider),
        Arc::new(MoondropGm01Provider),
        Arc::new(MaonoPd200wProvider),
        Arc::new(DarkrimShadowProvider),
        Arc::new(LenovoWirelessVoipProvider),
        Arc::new(EposH3ProHybridProvider),
        Arc::new(FifineHt6Provider),
        Arc::new(MchoseX9V9ProProvider),
    ]
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

const JABRA_DECT_EXCLUDED_PIDS: &[u16] = &[
    0x1000, 0x1016, 0x1017, 0x1018, 0x1021, 0x1022, 0x1023, 0x1037, 0x1038, 0x1039, 0x1041, 0x1042,
    0x1043, 0x1110, 0x1112, 0x1113, 0x1118, 0x111A, 0x111B, 0x111D, 0x111E, 0x111F, 0x1120, 0x1121,
    0x1122, 0x1123, 0x1124, 0x1126, 0x1131, 0x1132, 0x1133, 0x1134, 0x1135, 0x1136, 0x1140, 0x1141,
    0x1142, 0x1146, 0x1147, 0x1148, 0x114A, 0x114B, 0x114C, 0x114D, 0x114E, 0x114F, 0x1150, 0x1151,
    0xA700, 0xA701, 0xA702, 0xA703, 0xA704, 0xA705, 0xA706, 0xAE60, 0xAE62, 0xAE63,
];

struct JabraGnpProvider;

impl Provider for JabraGnpProvider {
    fn id(&self) -> &str {
        "jabra.gnp"
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
                .unwrap_or_else(|| "Jabra headset".to_owned()),
        )
    }

    fn matches(&self, endpoint: &Endpoint) -> MatchDecision {
        if endpoint.vid == Some(0x0B0E)
            && endpoint
                .pid
                .is_some_and(|pid| !JABRA_DECT_EXCLUDED_PIDS.contains(&pid))
            && endpoint.usage_page == Some(0xFF00)
            && matches!(endpoint.usage, Some(0x0005) | Some(0x0001))
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
        let is_link = device
            .endpoint
            .product_name
            .as_deref()
            .is_some_and(|n| n.to_ascii_lowercase().contains("link"));
        let targets = if is_link { [0x04, 0x08] } else { [0x08, 0x04] };
        let seq = 0x20_u8;
        for dst in targets {
            drain_input(session, device, deadline, 6)?;
            let mut req = vec![0_u8; 64];
            req[..7].copy_from_slice(&[0x05, dst, 0x00, seq, 0x46, 0x12, 0x02]);
            send_with_modes(session, device, OUTPUT, &req)?;
            let frame = match wait_frame(
                session,
                device,
                deadline,
                Duration::from_millis(500),
                12,
                |r| {
                    Ok(r.len() >= 7
                        && r[0] == 0x05
                        && r[1] == 0x00
                        && r[2] == dst
                        && r[3] == seq
                        && (r[4] & 0xC0) == 0xC0)
                },
            ) {
                Ok(f) => f,
                Err(BatteryError::Timeout { .. }) => continue,
                Err(error) => return Err(error),
            };
            if frame[5] == 0xFE {
                if dst == 0x04 && (is_link || matches!(frame[6], 0xF3..=0xF5)) {
                    return Ok(ProviderReading {
                        reachability: Reachability::Offline,
                        source: ReadingSource::VendorHid,
                        ..ProviderReading::default()
                    });
                }
                continue;
            }
            let len = usize::from(frame[4] & 0x3F);
            if frame[5] == 0x12 && frame[6] == 0x02 && len >= 6 && len < frame.len() {
                let p = &frame[7..=len];
                if p.len() >= 2 {
                    let pct = checked_percentage(p[1])?;
                    let charge = if (p[0] & 0x01) != 0 {
                        ChargeState::Charging
                    } else {
                        ChargeState::Discharging
                    };
                    return Ok(reading(Some(pct), charge, None));
                }
            }
        }
        Err(BatteryError::timeout("Jabra GNP"))
    }
}

struct JblHidv1Provider;

impl Provider for JblHidv1Provider {
    fn id(&self) -> &str {
        "jbl.hidv1"
    }

    fn metadata(&self) -> ProviderMetadata {
        experimental_metadata(
            DeviceKind::Headset,
            capabilities(true, false, false),
            Duration::from_millis(300),
        )
    }

    fn display_name(&self, endpoint: &Endpoint) -> Option<String> {
        Some(match endpoint.pid {
            Some(0x206B) => "JBL Quantum 350 Wireless".to_owned(),
            Some(0x205C) => "JBL Quantum 610 Wireless".to_owned(),
            Some(0x2115) => "JBL Quantum 360 Wireless".to_owned(),
            _ => "JBL Quantum Wireless".to_owned(),
        })
    }

    fn matches(&self, endpoint: &Endpoint) -> MatchDecision {
        if endpoint.vid != Some(0x0ECB) {
            return MatchDecision::NoMatch;
        }
        let matched = match endpoint.pid {
            Some(0x206B) => {
                endpoint.interface_number == Some(3)
                    && endpoint.usage_page == Some(0x0001)
                    && endpoint.usage == Some(0)
            }
            Some(0x205C) => {
                endpoint.interface_number == Some(5)
                    && endpoint
                        .usage_page
                        .is_some_and(|p| p == 0x0001 || p >= 0xFF00)
            }
            Some(0x2115) => {
                endpoint.interface_number == Some(5)
                    && endpoint.usage_page == Some(0x0001)
                    && endpoint.usage == Some(0)
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
        deadline.check("JBL HIDV1")?;
        let linked = feature_frame(session, device, 0x68, 1)?;
        if linked.len() < 2 || linked[1] != 1 {
            return Ok(ProviderReading {
                reachability: Reachability::Offline,
                source: ReadingSource::VendorHid,
                ..ProviderReading::default()
            });
        }
        let bat = feature_frame(session, device, 0x49, 1)?;
        if bat.len() < 2 {
            return Err(BatteryError::invalid_response("truncated JBL HIDV1 report"));
        }
        Ok(reading(
            Some(checked_percentage(bat[1])?),
            ChargeState::Unknown,
            None,
        ))
    }
}

const JBL_HIDV3_PIDS: &[u16] = &[
    0x2125, 0x2126, 0x213B, 0x213D, 0x2127, 0x2128, 0x213E, 0x2140, 0x218D, 0x218E, 0x21A7, 0x21A9,
    0x219A, 0x219B,
];

struct JblHidv3Provider;

impl Provider for JblHidv3Provider {
    fn id(&self) -> &str {
        "jbl.hidv3"
    }

    fn metadata(&self) -> ProviderMetadata {
        experimental_metadata(
            DeviceKind::Headset,
            capabilities(true, true, false),
            Duration::from_millis(500),
        )
    }

    fn display_name(&self, endpoint: &Endpoint) -> Option<String> {
        Some(match endpoint.pid {
            Some(0x2125 | 0x2126) => "JBL Quantum 650 Wireless".to_owned(),
            Some(0x213B | 0x213D) => "JBL Quantum 650X Wireless".to_owned(),
            Some(0x2127 | 0x2128) => "JBL Quantum 950 Wireless".to_owned(),
            Some(0x213E | 0x2140) => "JBL Quantum 950X Wireless".to_owned(),
            Some(0x218D | 0x218E) => "JBL Quantum 520 Wireless".to_owned(),
            Some(0x21A7 | 0x21A9) => "JBL Quantum 520X Wireless".to_owned(),
            Some(0x219A | 0x219B) => "JBL Quantum 850 Wireless".to_owned(),
            _ => "JBL Quantum Wireless".to_owned(),
        })
    }

    fn matches(&self, endpoint: &Endpoint) -> MatchDecision {
        if endpoint.vid == Some(0x0ECB)
            && endpoint.pid.is_some_and(|p| JBL_HIDV3_PIDS.contains(&p))
            && endpoint.usage_page == Some(0xFF12)
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
        let req = [
            0x88, 0x00, 0xDD, 0x01, 0x00, 0x01, 0x00, 0x02, 0x00, 0x0D, 0x00,
        ];
        send_with_modes(session, device, &[SendMode::Feature], &req)?;
        let r = wait_frame(
            session,
            device,
            deadline,
            Duration::from_millis(500),
            12,
            |r| {
                Ok(r.len() >= 16
                    && r[..6] == [0x86, 0x00, 0xDD, 0x01, 0x00, 0x01]
                    && (r[10] & 0x80) == 0
                    && r[11] == 0x0D
                    && r[12] == 0x00
                    && (r[13] != 0 || r[14] != 0))
            },
        )?;
        let info = r[15];
        let pct = checked_percentage(info & 0x7F)?;
        let charge = if (info & 0x80) != 0 {
            ChargeState::Charging
        } else {
            ChargeState::Discharging
        };
        Ok(reading(Some(pct), charge, None))
    }
}

struct PlantronicsProvider;

impl Provider for PlantronicsProvider {
    fn id(&self) -> &str {
        "plantronics.bt600"
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
                .unwrap_or_else(|| "Poly / Plantronics BT600 headset".to_owned()),
        )
    }

    fn matches(&self, endpoint: &Endpoint) -> MatchDecision {
        if endpoint.vid == Some(0x047F)
            && matches!(endpoint.pid, Some(0x02EE | 0x02F7))
            && endpoint.usage_page == Some(0xFFA2)
            && endpoint.usage == Some(0x0003)
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
        let send_cmd =
            |session: &mut dyn HidSession, op: u8, grp: u8, item: u8, addr: u8, len: u8| {
                let mut f = vec![0_u8; 64];
                f[..11].copy_from_slice(&[
                    0x07, 0x01, 0x01, 0x10, len, addr, 0x00, 0x00, op, grp, item,
                ]);
                send_with_modes(session, device, OUTPUT, &f)
            };
        drain_input(session, device, deadline, 6)?;
        send_with_modes(session, device, OUTPUT, &[0x06, 0x01])?;
        send_cmd(session, 0x01, 0x01, 0x02, 0x20, 0x07)?;
        let _hello = wait_frame(
            session,
            device,
            deadline,
            Duration::from_millis(400),
            16,
            |r| {
                Ok(r.len() > 10
                    && r[0] == 0x07
                    && r[1] == 0x01
                    && r[2] == 0x01
                    && r[3] == 0x10
                    && r[5] == 0x02
                    && r[8] == 0x08
                    && r[9] == 0x01
                    && r[10] == 0x02)
            },
        )?;
        send_cmd(session, 0x02, 0x0A, 0x1A, 0x20, 0x06)?;
        let bat = wait_frame(
            session,
            device,
            deadline,
            Duration::from_millis(500),
            16,
            |r| {
                Ok(r.len() >= 14
                    && r[0] == 0x07
                    && r[1] == 0x01
                    && r[2] == 0x01
                    && r[3] == 0x10
                    && r[5] == 0x02
                    && r[8] == 0x03
                    && r[9] == 0x0A
                    && r[10] == 0x1A)
            },
        )?;
        let level = bat[11];
        let count = bat[12];
        if count < 2 || level >= count {
            return Err(BatteryError::invalid_response(
                "invalid Plantronics battery ratio",
            ));
        }
        let pct = ((u16::from(level) * 100) / u16::from(count - 1)).min(100) as u8;
        let charge = if bat[13] != 0 {
            ChargeState::Charging
        } else {
            ChargeState::Discharging
        };
        Ok(reading(Some(pct), charge, None))
    }
}

struct FractalScapeProvider;

impl Provider for FractalScapeProvider {
    fn id(&self) -> &str {
        "fractal.scape"
    }

    fn metadata(&self) -> ProviderMetadata {
        experimental_metadata(
            DeviceKind::Headset,
            capabilities(true, true, false),
            Duration::from_millis(500),
        )
    }

    fn display_name(&self, _endpoint: &Endpoint) -> Option<String> {
        Some("Fractal Design Scape".to_owned())
    }

    fn matches(&self, endpoint: &Endpoint) -> MatchDecision {
        if endpoint.vid == Some(0x36BC)
            && matches!(endpoint.pid, Some(0x0001 | 0x2001))
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
        let ask = |session: &mut dyn HidSession, kind: u8| {
            drain_input(session, device, deadline, 6)?;
            let mut req = vec![0_u8; 64];
            req[..3].copy_from_slice(&[0x02, kind, 0x21]);
            send_with_modes(session, device, OUTPUT, &req)?;
            wait_frame(
                session,
                device,
                deadline,
                Duration::from_millis(500),
                12,
                |r| Ok(r.len() >= 24 && r[0] == 0x02 && r[1] == kind && r[2] == 0x21),
            )
        };
        if device.endpoint.pid == Some(0x0001) {
            let dongle = ask(session, 0x11)?;
            if dongle[4] != 1 {
                return Ok(ProviderReading {
                    reachability: Reachability::Offline,
                    source: ReadingSource::VendorHid,
                    ..ProviderReading::default()
                });
            }
        }
        let headset = ask(session, 0xF1)?;
        let pct = checked_percentage(headset[15])?;
        let charge = if headset[23] == 1 {
            ChargeState::Charging
        } else {
            ChargeState::Discharging
        };
        Ok(reading(Some(pct), charge, None))
    }
}

struct AlienwareAw920hProvider;

impl Provider for AlienwareAw920hProvider {
    fn id(&self) -> &str {
        "alienware.aw920h"
    }

    fn metadata(&self) -> ProviderMetadata {
        experimental_metadata(
            DeviceKind::Headset,
            capabilities(true, true, false),
            Duration::from_millis(300),
        )
    }

    fn display_name(&self, _endpoint: &Endpoint) -> Option<String> {
        Some("Alienware AW920H".to_owned())
    }

    fn matches(&self, endpoint: &Endpoint) -> MatchDecision {
        if endpoint.vid == Some(0x413C)
            && endpoint.pid == Some(0xA515)
            && endpoint.usage_page == Some(0xFF13)
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
        let mut req = vec![0_u8; 62];
        req[..3].copy_from_slice(&[0x06, 0xC0, 0x0A]);
        for _ in 0..3 {
            send_with_modes(session, device, OUTPUT_THEN_SET, &req)?;
            for _ in 0..6 {
                sleep(deadline, Duration::from_millis(15))?;
                let r = input_frame(session, device, 0x07, 61)?;
                if r.len() >= 14 && r[0] == 0x07 && r[1] == 0xC0 && r[2] == 0x0A && r[3] == 0x09 {
                    let xor = r[3..=12].iter().fold(0_u8, |a, b| a ^ b);
                    if r[13] != xor {
                        return Err(BatteryError::ChecksumFailed);
                    }
                    if r[6] == 0 {
                        continue;
                    }
                    let raw = if r[8] == 1 { r[11] } else { r[6] };
                    let pct = checked_percentage(raw)?;
                    let charge = if r[7] == 1 {
                        ChargeState::Charging
                    } else {
                        ChargeState::Discharging
                    };
                    return Ok(reading(Some(pct), charge, None));
                }
            }
        }
        Err(BatteryError::timeout("Alienware AW920H"))
    }
}

struct MoondropGm01Provider;

impl Provider for MoondropGm01Provider {
    fn id(&self) -> &str {
        "moondrop.gm01_pro"
    }

    fn metadata(&self) -> ProviderMetadata {
        experimental_metadata(
            DeviceKind::Headset,
            capabilities(true, false, false),
            Duration::from_millis(400),
        )
    }

    fn display_name(&self, _endpoint: &Endpoint) -> Option<String> {
        Some("MOONDROP GM01 Pro".to_owned())
    }

    fn matches(&self, endpoint: &Endpoint) -> MatchDecision {
        if endpoint.vid == Some(0x35D8)
            && endpoint.pid == Some(0x0133)
            && endpoint.interface_number == Some(3)
            && endpoint.usage_page == Some(0x000C)
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
        req[..4].copy_from_slice(&[0x4B, 0x80, 0x27, 0x00]);
        send_with_modes(session, device, OUTPUT, &req)?;
        let r = wait_frame(
            session,
            device,
            deadline,
            Duration::from_millis(400),
            12,
            |r| Ok(r.len() >= 10 && r[0] == 0x4B && r[1] == 0x80 && r[2] == 0x27),
        )?;
        if r[3] < 6 || r[8] == 0 {
            return Ok(ProviderReading {
                reachability: Reachability::Offline,
                source: ReadingSource::VendorHid,
                ..ProviderReading::default()
            });
        }
        Ok(reading(
            Some(checked_percentage(r[9])?),
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
        experimental_metadata(
            DeviceKind::Peripheral,
            capabilities(true, false, false),
            Duration::from_millis(500),
        )
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
        experimental_metadata(
            DeviceKind::Headset,
            capabilities(true, true, false),
            Duration::from_millis(500),
        )
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

struct LenovoWirelessVoipProvider;

impl Provider for LenovoWirelessVoipProvider {
    fn id(&self) -> &str {
        "lenovo.wireless_voip"
    }

    fn metadata(&self) -> ProviderMetadata {
        experimental_metadata(
            DeviceKind::Headset,
            capabilities(true, false, false),
            Duration::from_millis(500),
        )
    }

    fn display_name(&self, _endpoint: &Endpoint) -> Option<String> {
        Some("Lenovo Wireless VoIP Headset".to_owned())
    }

    fn matches(&self, endpoint: &Endpoint) -> MatchDecision {
        let matched = endpoint.vid == Some(0x17EF)
            && endpoint.pid == Some(0xA07D)
            && ((endpoint.usage_page == Some(0xFF07)
                && matches!(endpoint.usage, None | Some(0x0222)))
                || (endpoint.usage_page.is_none() && endpoint.interface_number == Some(3)));
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
        let mut req = vec![0_u8; 61];
        req[0] = 0x24;
        req[1] = 0x01;
        send_with_modes(session, device, &[SendMode::Feature], &req)?;
        let f = wait_frame(
            session,
            device,
            deadline,
            Duration::from_millis(500),
            12,
            |r| Ok(r.len() >= 8 && r[0] == 0x27 && r[1] == 0x01),
        )?;
        if f[2] != 0x00 {
            return Ok(ProviderReading {
                reachability: Reachability::Offline,
                source: ReadingSource::VendorHid,
                ..ProviderReading::default()
            });
        }
        Ok(reading(
            Some(checked_percentage(f[7])?),
            ChargeState::Unknown,
            None,
        ))
    }
}

struct EposH3ProHybridProvider;

fn crc8_epos(data: &[u8]) -> u8 {
    let mut crc = 0xFF_u8;
    for &byte in data {
        crc ^= byte;
        for _ in 0..8 {
            if (crc & 0x80) != 0 {
                crc = (crc << 1) ^ 0x07;
            } else {
                crc <<= 1;
            }
        }
    }
    crc
}

struct EposFrame {
    target: u16,
    group: u16,
    command: u16,
    values: Vec<u16>,
}

fn parse_epos_frame(report: &[u8]) -> Option<EposFrame> {
    if report.len() < 13 || report[0] != 0x01 || report[1] != 0xEB || report[2] != 0xFE {
        return None;
    }
    let payload_len = usize::from(u16::from_le_bytes([report[9], report[10]]));
    if payload_len < 4 || payload_len % 2 != 0 || 13 + payload_len > report.len() {
        return None;
    }
    let payload = &report[13..13 + payload_len];
    let expected_crc = u16::from(crc8_epos(payload));
    if u16::from_le_bytes([report[11], report[12]]) != expected_crc {
        return None;
    }
    let target = u16::from_le_bytes([report[7], report[8]]);
    let group = u16::from_le_bytes([report[13], report[14]]);
    let command = u16::from_le_bytes([report[15], report[16]]);
    let mut values = Vec::with_capacity((payload_len - 4) / 2);
    for chunk in report[17..13 + payload_len].chunks_exact(2) {
        values.push(u16::from_le_bytes([chunk[0], chunk[1]]));
    }
    Some(EposFrame {
        target,
        group,
        command,
        values,
    })
}

fn epos_request(
    session: &mut dyn HidSession,
    device: &DeviceContext<'_>,
    deadline: Deadline,
    target: u16,
    group: u16,
    cmd: u16,
) -> Result<EposFrame, BatteryError> {
    let mut payload = [0_u8; 4];
    payload[0..2].copy_from_slice(&group.to_le_bytes());
    payload[2..4].copy_from_slice(&cmd.to_le_bytes());
    let mut req = vec![0_u8; 107];
    req[0] = 0x01;
    req[1] = 0xEB;
    req[2] = 0xFE;
    req[3..5].copy_from_slice(&0x0001_u16.to_le_bytes());
    req[5..7].copy_from_slice(&0x0004_u16.to_le_bytes());
    req[7..9].copy_from_slice(&target.to_le_bytes());
    req[9..11].copy_from_slice(&(payload.len() as u16).to_le_bytes());
    req[11..13].copy_from_slice(&u16::from(crc8_epos(&payload)).to_le_bytes());
    req[13..17].copy_from_slice(&payload);
    send_with_modes(session, device, OUTPUT, &req)?;

    let raw = wait_frame(
        session,
        device,
        deadline,
        Duration::from_millis(500),
        16,
        |r| {
            let Some(frame) = parse_epos_frame(r) else {
                return Ok(false);
            };
            if frame.group != group || frame.command != cmd {
                return Ok(false);
            }
            let is_reply = frame.target == target.saturating_sub(1);
            let is_offline_bounce = target == 0x0005 && frame.target == 0x0001;
            Ok(is_reply || is_offline_bounce)
        },
    )?;
    parse_epos_frame(&raw)
        .ok_or_else(|| BatteryError::invalid_response("invalid EPOS H3PRO Hybrid frame"))
}

impl Provider for EposH3ProHybridProvider {
    fn id(&self) -> &str {
        "epos.h3pro_hybrid"
    }

    fn metadata(&self) -> ProviderMetadata {
        experimental_metadata(
            DeviceKind::Headset,
            capabilities(true, false, false),
            Duration::from_millis(500),
        )
    }

    fn display_name(&self, _endpoint: &Endpoint) -> Option<String> {
        Some("EPOS H3PRO Hybrid".to_owned())
    }

    fn matches(&self, endpoint: &Endpoint) -> MatchDecision {
        let matched = endpoint.vid == Some(0x1395)
            && matches!(endpoint.pid, Some(0x0303 | 0x0101))
            && ((endpoint.usage_page == Some(0xFFFF)
                && matches!(endpoint.usage, None | Some(0x0001)))
                || (endpoint.usage_page.is_none() && endpoint.interface_number == Some(3)));
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
        let dongle = device.endpoint.pid == Some(0x0303);
        if dongle {
            let conn = epos_request(session, device, deadline, 0x0001, 0x0001, 0x0009)?;
            if conn.values.len() != 1 {
                return Err(BatteryError::invalid_response(
                    "unexpected EPOS H3PRO Hybrid connection reply",
                ));
            }
            if conn.values[0] == 0 {
                return Ok(ProviderReading {
                    reachability: Reachability::Offline,
                    source: ReadingSource::VendorHid,
                    ..ProviderReading::default()
                });
            }
        }
        let target = if dongle { 0x0005 } else { 0x0001 };
        let bat = epos_request(session, device, deadline, target, 0x0012, 0x0003)?;
        if dongle && bat.target == 0x0001 {
            return Ok(ProviderReading {
                reachability: Reachability::Offline,
                source: ReadingSource::VendorHid,
                ..ProviderReading::default()
            });
        }
        if bat.target != target.saturating_sub(1) || bat.values.len() != 1 || bat.values[0] > 100 {
            return Err(BatteryError::invalid_response(
                "unexpected EPOS H3PRO Hybrid battery reply",
            ));
        }
        Ok(reading(
            Some(checked_percentage(bat.values[0] as u8)?),
            ChargeState::Unknown,
            None,
        ))
    }
}

struct FifineHt6Provider;

impl Provider for FifineHt6Provider {
    fn id(&self) -> &str {
        "fifine.ht6"
    }

    fn metadata(&self) -> ProviderMetadata {
        experimental_metadata(
            DeviceKind::Headset,
            capabilities(true, true, false),
            Duration::from_millis(500),
        )
    }

    fn display_name(&self, _endpoint: &Endpoint) -> Option<String> {
        Some("FIFINE HT6 Wireless".to_owned())
    }

    fn matches(&self, endpoint: &Endpoint) -> MatchDecision {
        let matched = endpoint.vid == Some(0x3142)
            && endpoint.pid == Some(0x00C3)
            && ((endpoint.usage_page == Some(0xFF90)
                && matches!(endpoint.usage, None | Some(0x0001)))
                || (endpoint.usage_page.is_none() && endpoint.interface_number == Some(0)));
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
        let mut req = vec![0_u8; 64];
        req[0] = 0x55;
        req[1] = 0x65;
        req[2] = 0x01;
        send_with_modes(session, device, OUTPUT, &req)?;
        let f = wait_frame(
            session,
            device,
            deadline,
            Duration::from_millis(500),
            16,
            |r| Ok(r.len() >= 4 && r[0] == 0x55 && (r[1] == 0x65 || r[1] == 0x0A || r[3] == 0x1A)),
        )?;
        if f[1] == 0x0A || f[3] == 0x1A {
            return Ok(ProviderReading {
                reachability: Reachability::Offline,
                source: ReadingSource::VendorHid,
                ..ProviderReading::default()
            });
        }
        let pct = checked_percentage(f[2])?;
        let charge = if f[3] == 0x03 {
            ChargeState::Charging
        } else {
            ChargeState::Discharging
        };
        Ok(reading(Some(pct), charge, None))
    }
}

struct MchoseX9V9ProProvider;

impl Provider for MchoseX9V9ProProvider {
    fn id(&self) -> &str {
        "mchose.x9_v9pro"
    }

    fn metadata(&self) -> ProviderMetadata {
        experimental_metadata(
            DeviceKind::Headset,
            capabilities(true, false, false),
            Duration::from_millis(500),
        )
    }

    fn display_name(&self, endpoint: &Endpoint) -> Option<String> {
        Some(match (endpoint.vid, endpoint.pid) {
            (Some(0x3837), Some(0x6045)) => "MCHOSE X9 Wireless".to_owned(),
            (Some(0x291D), Some(0x385D)) => "MCHOSE V9 Pro Wireless".to_owned(),
            _ => "MCHOSE Wireless Headset".to_owned(),
        })
    }

    fn matches(&self, endpoint: &Endpoint) -> MatchDecision {
        let matched = matches!(
            (endpoint.vid, endpoint.pid),
            (Some(0x3837), Some(0x6045)) | (Some(0x291D), Some(0x385D))
        ) && ((endpoint.usage_page == Some(0xFF90)
            && matches!(endpoint.usage, None | Some(0x0001)))
            || (endpoint.usage_page.is_none() && endpoint.interface_number == Some(0)));
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
        let mut req = vec![0_u8; 64];
        req[0] = 0x55;
        req[1] = 0x65;
        req[2] = 0x01;
        send_with_modes(session, device, OUTPUT, &req)?;
        let f = wait_frame(
            session,
            device,
            deadline,
            Duration::from_millis(500),
            16,
            |r| Ok(r.len() >= 4 && r[0] == 0x55 && r[1] == 0x65 && r[3] == 0x02),
        )?;
        Ok(reading(
            Some(checked_percentage(f[2])?),
            ChargeState::Unknown,
            None,
        ))
    }
}
