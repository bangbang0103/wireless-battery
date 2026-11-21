//! Wireless headset HID battery protocols:
//! Airoha RACE (Audeze Maxwell, ENDORFY Viro, AKG N9 Hybrid, Turtle Beach Stealth 700 Gen 3),
//! Jabra GNP, JBL Quantum (HIDV1 & HIDV3), Poly/Plantronics BT600, Fractal Scape,
//! Alienware AW920H, and MOONDROP GM01 Pro.

use crate::error::BatteryError;
use crate::model::{ChargeState, Endpoint, ProviderReading, Reachability, ReadingSource};
use crate::providers::fixed::{
    capabilities, checked_percentage, drain_input, experimental_metadata, feature_frame,
    input_frame, reading, send_with_modes, sleep, wait_frame, SendMode, OUTPUT, OUTPUT_THEN_SET,
};
use crate::providers::{Deadline, DeviceContext, MatchDecision, Provider, ProviderMetadata};
use crate::transport::HidSession;
use std::sync::Arc;
use std::time::Duration;

pub(crate) fn providers() -> Vec<Arc<dyn Provider>> {
    vec![
        Arc::new(AudezeMaxwellProvider),
        Arc::new(EndorfyViroProvider),
        Arc::new(AkgN9HybridProvider),
        Arc::new(TurtleBeachProvider),
        Arc::new(JabraGnpProvider),
        Arc::new(JblHidv1Provider),
        Arc::new(JblHidv3Provider),
        Arc::new(PlantronicsProvider),
        Arc::new(FractalScapeProvider),
        Arc::new(AlienwareAw920hProvider),
        Arc::new(MoondropGm01Provider),
    ]
}

fn airoha_race_battery_request() -> Vec<u8> {
    let mut req = vec![0_u8; 62];
    req[..9].copy_from_slice(&[0x06, 0x07, 0x80, 0x05, 0x5A, 0x03, 0x00, 0xD6, 0x0C]);
    req
}

struct AudezeMaxwellProvider;

impl Provider for AudezeMaxwellProvider {
    fn id(&self) -> &str {
        "audeze.maxwell"
    }

    fn metadata(&self) -> ProviderMetadata {
        experimental_metadata(capabilities(true, true, false), Duration::from_millis(300))
    }

    fn display_name(&self, endpoint: &Endpoint) -> Option<String> {
        Some(match endpoint.pid {
            Some(0x4B28 | 0x4B29) => "Audeze Maxwell 2".to_owned(),
            _ => "Audeze Maxwell".to_owned(),
        })
    }

    fn matches(&self, endpoint: &Endpoint) -> MatchDecision {
        if endpoint.vid == Some(0x3329)
            && matches!(
                endpoint.pid,
                Some(0x4B18 | 0x4B19 | 0x4B1A | 0x4B1E | 0x4B28 | 0x4B29)
            )
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
        let cable = matches!(device.endpoint.pid, Some(0x4B1A | 0x4B1E));
        let req = airoha_race_battery_request();
        for _ in 0..2 {
            send_with_modes(session, device, OUTPUT, &req)?;
            for _ in 0..4 {
                sleep(deadline, Duration::from_millis(60))?;
                let frame = input_frame(session, device, 0x07, 61)?;
                for window in frame.windows(5) {
                    if window[0] == 0xD6
                        && window[1] == 0x0C
                        && window[2] == 0x00
                        && window[3] == 0x00
                        && window[4] <= 100
                    {
                        let charge = if cable {
                            ChargeState::Charging
                        } else {
                            ChargeState::Unknown
                        };
                        return Ok(reading(Some(window[4]), charge, None));
                    }
                }
            }
        }
        Err(BatteryError::timeout("Audeze Maxwell"))
    }
}

struct EndorfyViroProvider;

impl Provider for EndorfyViroProvider {
    fn id(&self) -> &str {
        "endorfy.viro"
    }

    fn metadata(&self) -> ProviderMetadata {
        experimental_metadata(capabilities(true, false, false), Duration::from_millis(300))
    }

    fn display_name(&self, _endpoint: &Endpoint) -> Option<String> {
        Some("ENDORFY Viro Wireless".to_owned())
    }

    fn matches(&self, endpoint: &Endpoint) -> MatchDecision {
        if endpoint.vid == Some(0x3738)
            && endpoint.pid == Some(0x0072)
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
        let req = airoha_race_battery_request();
        send_with_modes(session, device, OUTPUT, &req)?;
        for _ in 0..12 {
            sleep(deadline, Duration::from_millis(60))?;
            let r = input_frame(session, device, 0x07, 61)?;
            if let Some(pct) = parse_race_0cd6_indication(&r) {
                return Ok(reading(Some(pct), ChargeState::Unknown, None));
            }
        }
        Err(BatteryError::timeout("ENDORFY Viro"))
    }
}

fn parse_race_0cd6_indication(r: &[u8]) -> Option<u8> {
    if r.len() < 9 || r[0] != 0x07 {
        return None;
    }
    let mut i = 3;
    while i + 9 <= r.len() {
        if r[i] == 0x05 {
            let len = usize::from(u16::from_le_bytes([r[i + 2], r[i + 3]]));
            if len >= 5 && i + 4 + len <= r.len() {
                let op = u16::from_le_bytes([r[i + 4], r[i + 5]]);
                if r[i + 1] == 0x5D && op == 0x0CD6 && r[i + 6] == 0x00 && r[i + 8] <= 100 {
                    return Some(r[i + 8]);
                }
            }
        }
        i += 1;
    }
    None
}

struct AkgN9HybridProvider;

impl Provider for AkgN9HybridProvider {
    fn id(&self) -> &str {
        "akg.n9_hybrid"
    }

    fn metadata(&self) -> ProviderMetadata {
        experimental_metadata(capabilities(true, false, false), Duration::from_millis(300))
    }

    fn display_name(&self, _endpoint: &Endpoint) -> Option<String> {
        Some("AKG N9 Hybrid".to_owned())
    }

    fn matches(&self, endpoint: &Endpoint) -> MatchDecision {
        if endpoint.vid == Some(0x0ECB)
            && endpoint.pid == Some(0x20D9)
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
        let req = airoha_race_battery_request();
        send_with_modes(session, device, OUTPUT, &req)?;
        for _ in 0..24 {
            sleep(deadline, Duration::from_millis(15))?;
            let b = input_frame(session, device, 0x07, 61)?;
            if b.len() >= 4 && b[0] == 0x07 {
                let end = b.len().min(3 + usize::from(b[1]));
                let mut found = None;
                let mut i = 3;
                while i + 9 <= end {
                    if b[i..i + 7] == [0x05, 0x5D, 0x05, 0x00, 0xD6, 0x0C, 0x00] && b[i + 8] <= 100
                    {
                        found = Some(b[i + 8]);
                    }
                    i += 1;
                }
                if let Some(pct) = found {
                    return Ok(reading(Some(pct), ChargeState::Unknown, None));
                }
            }
        }
        Err(BatteryError::timeout("AKG N9 Hybrid"))
    }
}

struct TurtleBeachProvider;

impl Provider for TurtleBeachProvider {
    fn id(&self) -> &str {
        "turtlebeach.stealth700"
    }

    fn metadata(&self) -> ProviderMetadata {
        experimental_metadata(capabilities(true, false, false), Duration::from_millis(600))
    }

    fn display_name(&self, endpoint: &Endpoint) -> Option<String> {
        Some(
            endpoint
                .product_name
                .clone()
                .filter(|name| !name.is_empty())
                .unwrap_or_else(|| "Turtle Beach Stealth 700 Gen 3".to_owned()),
        )
    }

    fn matches(&self, endpoint: &Endpoint) -> MatchDecision {
        if endpoint.vid == Some(0x10F5)
            && matches!(endpoint.pid, Some(0x2251 | 0x2262 | 0x2253))
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
        let mut enable = vec![0_u8; 62];
        enable[..9].copy_from_slice(&[0x06, 0x06, 0x00, 0x05, 0x5A, 0x02, 0x00, 0x42, 0x99]);
        send_with_modes(session, device, OUTPUT, &enable)?;
        sleep(deadline, Duration::from_millis(120))?;

        let mut get_gsi = vec![0_u8; 62];
        get_gsi[..17].copy_from_slice(&[
            0x06, 0x0E, 0x00, 0x05, 0x5A, 0x0A, 0x00, 0x02, 0x99, 0x40, 0x01, 0x00, 0x01, 0xB3,
            b'G', b'S', b'I',
        ]);
        send_with_modes(session, device, OUTPUT, &get_gsi)?;

        let mut payload = String::new();
        for _ in 0..24 {
            sleep(deadline, Duration::from_millis(30))?;
            let r = input_frame(session, device, 0x07, 61)?;
            if r.len() >= 3 && r[0] == 0x07 {
                let take = usize::from(r[1]).min(r.len() - 3);
                for &b in &r[3..3 + take] {
                    if b.is_ascii() {
                        payload.push(char::from(b));
                    }
                }
                if let Some(pct) = parse_gsi_battery(&payload) {
                    return Ok(reading(Some(pct), ChargeState::Unknown, None));
                }
            }
        }
        Err(BatteryError::timeout("Turtle Beach Stealth 700 Gen 3"))
    }
}

fn parse_gsi_battery(payload: &str) -> Option<u8> {
    if !payload.contains("\"GSI\"") {
        return None;
    }
    let marker = "\"240\":\"";
    let start = payload.rfind(marker)? + marker.len();
    let rest = &payload[start..];
    let end = rest.find('"')?;
    if end == 0 || end > 4 {
        return None;
    }
    let val = rest[..end].parse::<u16>().ok()?;
    (val <= 100).then_some(val as u8)
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
        experimental_metadata(capabilities(true, true, false), Duration::from_millis(500))
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
                if dst == 0x04 && (is_link || frame[6] == 0xF3) {
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
        experimental_metadata(capabilities(true, false, false), Duration::from_millis(300))
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
            Some(0x205C) => endpoint.interface_number == Some(5),
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
        experimental_metadata(capabilities(true, true, false), Duration::from_millis(500))
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
        experimental_metadata(capabilities(true, true, false), Duration::from_millis(500))
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
        experimental_metadata(capabilities(true, true, false), Duration::from_millis(500))
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
        experimental_metadata(capabilities(true, true, false), Duration::from_millis(300))
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
        experimental_metadata(capabilities(true, false, false), Duration::from_millis(400))
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
