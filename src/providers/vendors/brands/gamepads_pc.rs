//! PC wireless gamepad HID battery protocols
//! (GameSir Nova/Cyclone 2/SAPJoy, Flydigi Vader 5/Apex 5, BEITONG KP20/KP40D, NVIDIA Shield).

use crate::error::BatteryError;
use crate::model::{ChargeState, DeviceKind, Endpoint, ProviderReading};
use crate::providers::fixed::{
    capabilities, checked_percentage, drain_input, experimental_metadata, reading, send_with_modes,
    unnumbered_payload, wait_frame, OUTPUT, OUTPUT_THEN_SET,
};
use crate::providers::{Deadline, DeviceContext, MatchDecision, Provider, ProviderMetadata};
use crate::transport::HidSession;
use std::sync::Arc;
use std::time::Duration;

pub(crate) fn providers() -> Vec<Arc<dyn Provider>> {
    vec![
        Arc::new(GameSirNovaProvider),
        Arc::new(GameSirCyclone2Provider),
        Arc::new(GameSirSapjoyProvider),
        Arc::new(FlydigiProvider),
        Arc::new(BeitongProvider),
        Arc::new(NvidiaShieldProvider),
    ]
}

struct GameSirNovaProvider;

impl Provider for GameSirNovaProvider {
    fn id(&self) -> &str {
        "gamesir.nova"
    }

    fn metadata(&self) -> ProviderMetadata {
        experimental_metadata(
            DeviceKind::Gamepad,
            capabilities(true, true, false),
            Duration::from_millis(400),
        )
    }

    fn display_name(&self, _endpoint: &Endpoint) -> Option<String> {
        Some("GameSir Nova".to_owned())
    }

    fn matches(&self, endpoint: &Endpoint) -> MatchDecision {
        if endpoint.vid == Some(0x3537)
            && matches!(endpoint.pid, Some(0x1098 | 0x100F | 0x1040))
            && endpoint.usage_page == Some(0xFF7A)
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
        request[1] = 0x01;
        request[2] = 0x01;
        send_with_modes(session, device, OUTPUT, &request)?;
        let frame = wait_frame(
            session,
            device,
            deadline,
            Duration::from_millis(400),
            16,
            |frame| {
                let r = unnumbered_payload(frame);
                Ok(r.len() >= 19 && r[0] == 0x01)
            },
        )?;
        let r = unnumbered_payload(&frame);
        if r[18] == 0xFF {
            Ok(reading(None, ChargeState::Charging, None))
        } else {
            Ok(reading(
                Some(checked_percentage(r[18])?),
                ChargeState::Discharging,
                None,
            ))
        }
    }
}

struct GameSirCyclone2Provider;

impl Provider for GameSirCyclone2Provider {
    fn id(&self) -> &str {
        "gamesir.cyclone2"
    }

    fn metadata(&self) -> ProviderMetadata {
        experimental_metadata(
            DeviceKind::Gamepad,
            capabilities(true, true, false),
            Duration::from_millis(500),
        )
    }

    fn display_name(&self, _endpoint: &Endpoint) -> Option<String> {
        Some("GameSir Cyclone 2".to_owned())
    }

    fn matches(&self, endpoint: &Endpoint) -> MatchDecision {
        if endpoint.vid == Some(0x3537)
            && endpoint.pid == Some(0x100B)
            && endpoint.usage_page == Some(0xFFF0)
            && endpoint.usage == Some(0x0040)
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
        request[0] = 0x0F;
        request[1] = 0x03;
        send_with_modes(session, device, OUTPUT, &request)?;
        let frame = wait_frame(
            session,
            device,
            deadline,
            Duration::from_millis(500),
            16,
            |frame| Ok(frame.len() >= 37 && frame[0] == 0x12),
        )?;
        let percentage = checked_percentage(frame[36])?;
        let charge = match frame[35] {
            0x00 => ChargeState::Discharging,
            0x01 => ChargeState::Charging,
            _ => ChargeState::Unknown,
        };
        Ok(reading(Some(percentage), charge, None))
    }
}

struct GameSirSapjoyProvider;

impl Provider for GameSirSapjoyProvider {
    fn id(&self) -> &str {
        "gamesir.sapjoy"
    }

    fn metadata(&self) -> ProviderMetadata {
        experimental_metadata(
            DeviceKind::Gamepad,
            capabilities(true, true, false),
            Duration::from_millis(450),
        )
    }

    fn display_name(&self, endpoint: &Endpoint) -> Option<String> {
        Some(
            endpoint
                .product_name
                .clone()
                .filter(|name| !name.is_empty())
                .unwrap_or_else(|| "GameSir controller".to_owned()),
        )
    }

    fn matches(&self, endpoint: &Endpoint) -> MatchDecision {
        if endpoint.vid == Some(0x3537)
            && matches!(endpoint.pid, Some(0x100E | 0x103E | 0x1050))
            && endpoint.usage_page == Some(0xFFA0)
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
        let mut request = vec![0_u8; 32];
        request[0] = 0x07;
        request[1] = 0x03;
        request[2] = 0x0A;
        request[3] = 0x01;
        send_with_modes(session, device, OUTPUT, &request)?;
        let frame = wait_frame(
            session,
            device,
            deadline,
            Duration::from_millis(450),
            16,
            |frame| {
                Ok(frame.len() >= 15
                    && frame[0] == 0x06
                    && frame[2] == 0x0A
                    && frame[3] == 0x01
                    && frame[1] >= 14
                    && usize::from(frame[1]) < frame.len())
            },
        )?;
        if frame[13] == 0xFF {
            Ok(reading(None, ChargeState::Charging, None))
        } else {
            Ok(reading(
                Some(checked_percentage(frame[13])?),
                ChargeState::Discharging,
                None,
            ))
        }
    }
}

struct FlydigiProvider;

impl Provider for FlydigiProvider {
    fn id(&self) -> &str {
        "flydigi.controller"
    }

    fn metadata(&self) -> ProviderMetadata {
        experimental_metadata(
            DeviceKind::Gamepad,
            capabilities(true, true, false),
            Duration::from_millis(500),
        )
    }

    fn display_name(&self, endpoint: &Endpoint) -> Option<String> {
        Some(match endpoint.pid {
            Some(0x2401) => "Flydigi Vader 5 Pro".to_owned(),
            Some(0x2501) => "Flydigi Apex 5".to_owned(),
            _ => "Flydigi controller".to_owned(),
        })
    }

    fn matches(&self, endpoint: &Endpoint) -> MatchDecision {
        if endpoint.vid == Some(0x37D7)
            && matches!(endpoint.pid, Some(0x2401 | 0x2501))
            && endpoint.usage_page == Some(0xFFA0)
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
        let report_id = if device.endpoint.pid == Some(0x2501) {
            0x03
        } else {
            0x00
        };
        drain_input(session, device, deadline, 8)?;

        let mut info_req = vec![0_u8; 32];
        info_req[0] = report_id;
        info_req[1..6].copy_from_slice(&[0x5A, 0xA5, 0x01, 0x02, 0x00]);
        send_with_modes(session, device, OUTPUT, &info_req)?;

        let frame = wait_frame(
            session,
            device,
            deadline,
            Duration::from_millis(500),
            16,
            |frame| {
                let p = flydigi_payload(frame);
                Ok(p.len() >= 12 && p[0] == 0x5A && p[1] == 0xA5 && p[2] == 0x01)
            },
        )?;
        let p = flydigi_payload(&frame);
        let model_ok = match device.endpoint.pid {
            Some(0x2401) => p[5] == 0x82,
            Some(0x2501) => matches!(p[5], 0x80 | 0x81 | 0x85 | 0x86 | 0x87),
            _ => false,
        };
        if !model_ok {
            return Err(BatteryError::invalid_response(
                "unexpected Flydigi model identifier",
            ));
        }
        let level = p[11] & 0x0F;
        let state = p[11] >> 4;
        if level > 5 || state > 2 {
            return Err(BatteryError::invalid_response(
                "invalid Flydigi battery nibble",
            ));
        }
        let (percentage, charge) = match state {
            0 => (level.saturating_mul(20), ChargeState::Discharging),
            1 => (level.saturating_mul(20), ChargeState::Charging),
            2 => (100, ChargeState::Full),
            _ => unreachable!(),
        };
        Ok(reading(Some(percentage), charge, None))
    }
}

fn flydigi_payload(frame: &[u8]) -> &[u8] {
    let r = unnumbered_payload(frame);
    if r.len() >= 4 && r[0] == 0x04 && r[1] == 0x5A && r[2] == 0xA5 {
        &r[1..]
    } else {
        r
    }
}

struct BeitongProvider;

impl Provider for BeitongProvider {
    fn id(&self) -> &str {
        "beitong.controller"
    }

    fn metadata(&self) -> ProviderMetadata {
        experimental_metadata(
            DeviceKind::Gamepad,
            capabilities(true, true, false),
            Duration::from_millis(450),
        )
    }

    fn display_name(&self, endpoint: &Endpoint) -> Option<String> {
        Some(match endpoint.pid {
            Some(0x5128) => "BEITONG KP20".to_owned(),
            Some(0x515C) => "BEITONG KP40D".to_owned(),
            _ => "BEITONG controller".to_owned(),
        })
    }

    fn matches(&self, endpoint: &Endpoint) -> MatchDecision {
        if endpoint.vid == Some(0x20BC)
            && matches!(endpoint.pid, Some(0x5128 | 0x515C))
            && endpoint.usage_page == Some(0x00FF)
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
        drain_input(session, device, deadline, 8)?;
        let mut request = vec![0_u8; 64];
        request[0] = 0x02;
        request[1] = beitong_enc(0x15);
        send_with_modes(session, device, OUTPUT, &request)?;
        let frame = wait_frame(
            session,
            device,
            deadline,
            Duration::from_millis(450),
            16,
            |frame| Ok(frame.len() >= 5 && frame[0] == 0x02 && beitong_dec(frame[1]) == 0x15),
        )?;
        let pct = checked_percentage(beitong_dec(frame[2]))?;
        let mut raw_charge = beitong_dec(frame[4]);
        if device.endpoint.pid == Some(0x515C) {
            raw_charge &= 0x0F;
        }
        let charge = match raw_charge {
            0x00 => ChargeState::Discharging,
            0x01 => ChargeState::Charging,
            _ => ChargeState::Unknown,
        };
        Ok(reading(Some(pct), charge, None))
    }
}

const BEITONG_SBOX: [u8; 16] = [
    0x0, 0xA, 0x6, 0xF, 0x1, 0xC, 0x2, 0x9, 0xE, 0x3, 0x7, 0xD, 0x4, 0xB, 0x5, 0x8,
];

fn beitong_enc(x: u8) -> u8 {
    (BEITONG_SBOX[usize::from(x >> 4)] << 4) | ((x & 0x0F) ^ 0x08)
}

fn beitong_dec(x: u8) -> u8 {
    let hi = BEITONG_SBOX
        .iter()
        .position(|&v| v == (x >> 4))
        .unwrap_or(0) as u8;
    (hi << 4) | ((x & 0x0F) ^ 0x08)
}

struct NvidiaShieldProvider;

impl Provider for NvidiaShieldProvider {
    fn id(&self) -> &str {
        "nvidia.shield"
    }

    fn metadata(&self) -> ProviderMetadata {
        experimental_metadata(
            DeviceKind::Gamepad,
            capabilities(true, true, true),
            Duration::from_millis(500),
        )
    }

    fn display_name(&self, endpoint: &Endpoint) -> Option<String> {
        Some(
            endpoint
                .product_name
                .clone()
                .filter(|name| !name.is_empty())
                .unwrap_or_else(|| "NVIDIA Shield Controller".to_owned()),
        )
    }

    fn matches(&self, endpoint: &Endpoint) -> MatchDecision {
        if endpoint.vid == Some(0x0955)
            && endpoint.pid == Some(0x7214)
            && (endpoint.usage_page.is_none()
                || matches!(endpoint.usage_page, Some(0x0001) | Some(0xFF00..=0xFFFF)))
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

        // Step 1: Query battery percentage and voltage.
        let mut bat_req = vec![0_u8; 33];
        bat_req[0] = 0x04;
        bat_req[1] = 0x07;
        send_with_modes(session, device, OUTPUT_THEN_SET, &bat_req)?;
        let bat = wait_frame(
            session,
            device,
            deadline,
            Duration::from_millis(500),
            16,
            |f| Ok(f.len() >= 15 && f[0] == 0x03 && f[1] == 0x07),
        )?;
        let pct = checked_percentage(bat[14])?;
        let mv = u16::from_le_bytes([bat[12], bat[13]]);
        let voltage_mv = (mv > 0).then_some(u32::from(mv));

        // Step 2: Query charger connection and state.
        let mut chg_req = vec![0_u8; 33];
        chg_req[0] = 0x04;
        chg_req[1] = 0x3A;
        send_with_modes(session, device, OUTPUT_THEN_SET, &chg_req)?;
        let chg = wait_frame(
            session,
            device,
            deadline,
            Duration::from_millis(500),
            16,
            |f| Ok(f.len() >= 6 && f[0] == 0x03 && f[1] == 0x3A),
        )?;
        let connected = chg[3];
        let state = chg[5];
        let charge = if connected == 0 {
            ChargeState::Discharging
        } else {
            match state {
                2 => ChargeState::Charging,
                3 => ChargeState::Full,
                1 | 8 => ChargeState::Discharging,
                _ => ChargeState::Unknown,
            }
        };

        Ok(reading(Some(pct), charge, voltage_mv))
    }
}
