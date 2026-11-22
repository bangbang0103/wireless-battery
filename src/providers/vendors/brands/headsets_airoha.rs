//! Airoha RACE wireless headset HID battery protocols
//! (Audeze Maxwell, ENDORFY Viro, AKG N9 Hybrid, Turtle Beach Stealth 700 Gen 3).

use crate::error::BatteryError;
use crate::model::{ChargeState, DeviceKind, Endpoint, ProviderReading};
use crate::providers::fixed::{
    capabilities, experimental_metadata, input_frame, reading, send_with_modes, sleep, OUTPUT,
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
        experimental_metadata(
            DeviceKind::Headset,
            capabilities(true, true, false),
            Duration::from_millis(300),
        )
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
        experimental_metadata(
            DeviceKind::Headset,
            capabilities(true, false, false),
            Duration::from_millis(300),
        )
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
        experimental_metadata(
            DeviceKind::Headset,
            capabilities(true, false, false),
            Duration::from_millis(300),
        )
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
        experimental_metadata(
            DeviceKind::Headset,
            capabilities(true, false, false),
            Duration::from_millis(600),
        )
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
