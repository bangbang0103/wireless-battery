//! Gamepad HID battery protocols (Nintendo Switch Pro, Valve Steam Controller,
//! GameSir Nova/Cyclone 2/SAPJoy, Flydigi Vader 5/Apex 5, 8BitDo, BEITONG).

use crate::error::BatteryError;
use crate::model::{
    BatteryLevel, Capabilities, ChargeState, Endpoint, ProviderReading, Reachability, ReadingSource,
};
use crate::providers::fixed::{
    capabilities, checked_percentage, drain_input, experimental_metadata, reading, send_with_modes,
    unnumbered_payload, wait_frame, OUTPUT,
};
use crate::providers::{Deadline, DeviceContext, MatchDecision, Provider, ProviderMetadata};
use crate::transport::HidSession;
use std::sync::Arc;
use std::time::Duration;

pub(crate) fn providers() -> Vec<Arc<dyn Provider>> {
    vec![
        Arc::new(SwitchProProvider),
        Arc::new(SteamControllerProvider),
        Arc::new(GameSirNovaProvider),
        Arc::new(GameSirCyclone2Provider),
        Arc::new(GameSirSapjoyProvider),
        Arc::new(FlydigiProvider),
        Arc::new(EightBitDoVendorProvider),
        Arc::new(EightBitDoDInputProvider),
        Arc::new(BeitongProvider),
    ]
}

struct SwitchProProvider;

impl Provider for SwitchProProvider {
    fn id(&self) -> &str {
        "nintendo.switch_pro"
    }

    fn metadata(&self) -> ProviderMetadata {
        experimental_metadata(
            Capabilities {
                discrete_level: true,
                charge_state: true,
                ..Capabilities::default()
            },
            Duration::from_millis(350),
        )
    }

    fn display_name(&self, endpoint: &Endpoint) -> Option<String> {
        Some(
            endpoint
                .product_name
                .clone()
                .filter(|name| !name.is_empty())
                .unwrap_or_else(|| "Nintendo Switch Pro Controller".to_owned()),
        )
    }

    fn matches(&self, endpoint: &Endpoint) -> MatchDecision {
        if endpoint.vid == Some(0x057E)
            && endpoint.pid == Some(0x2009)
            && endpoint.usage_page == Some(0x0001)
            && matches!(endpoint.usage, Some(0x0004) | Some(0x0005))
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
            Duration::from_millis(350),
            16,
            |frame| Ok(frame.len() >= 13 && frame[0] == 0x30),
        )?;
        let raw_level = (frame[2] & 0xE0) >> 5;
        let level = match raw_level {
            0 | 1 => BatteryLevel::Critical,
            2 => BatteryLevel::Low,
            3 => BatteryLevel::Medium,
            4 => BatteryLevel::Full,
            _ => {
                return Err(BatteryError::invalid_response(
                    "invalid Switch Pro battery level",
                ))
            }
        };
        let charging_bit = (frame[2] & 0x10) != 0;
        let charge_state = if charging_bit && raw_level < 4 {
            ChargeState::Charging
        } else if raw_level == 4 && charging_bit {
            ChargeState::Full
        } else {
            ChargeState::Discharging
        };
        Ok(ProviderReading {
            level: Some(level),
            charge_state,
            reachability: Reachability::Online,
            source: ReadingSource::VendorHid,
            ..ProviderReading::default()
        })
    }
}

struct SteamControllerProvider;

impl Provider for SteamControllerProvider {
    fn id(&self) -> &str {
        "valve.steam_controller"
    }

    fn metadata(&self) -> ProviderMetadata {
        experimental_metadata(capabilities(true, true, false), Duration::from_millis(400))
    }

    fn display_name(&self, endpoint: &Endpoint) -> Option<String> {
        Some(match endpoint.pid {
            Some(0x1142 | 0x1304 | 0x1305) => "Steam Controller".to_owned(),
            _ => endpoint
                .product_name
                .clone()
                .filter(|name| !name.is_empty())
                .unwrap_or_else(|| "Steam Controller".to_owned()),
        })
    }

    fn matches(&self, endpoint: &Endpoint) -> MatchDecision {
        let pid_ok = matches!(
            endpoint.pid,
            Some(0x1142 | 0x1302 | 0x1303 | 0x1304 | 0x1305)
        );
        let usage_ok = endpoint.pid != Some(0x1304) || endpoint.usage == Some(0x0001);
        if endpoint.vid == Some(0x28DE) && pid_ok && endpoint.usage_page == Some(0xFF00) && usage_ok
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
        let legacy = device.endpoint.pid == Some(0x1142);
        let frame = wait_frame(
            session,
            device,
            deadline,
            Duration::from_millis(400),
            32,
            |frame| {
                let r = unnumbered_payload(frame);
                if legacy {
                    Ok(r.len() >= 15
                        && r[0] == 0x01
                        && r[1] == 0x00
                        && r[2] == 0x04
                        && r[3] == 0x0B)
                } else {
                    Ok(r.len() >= 15 && r[0] == 0x43)
                }
            },
        )?;
        let r = unnumbered_payload(&frame);
        if legacy {
            let percentage = checked_percentage(r[14])?;
            Ok(reading(Some(percentage), ChargeState::Unknown, None))
        } else {
            let percentage = checked_percentage(r[2])?;
            let charge = match r[1] {
                0x02 => ChargeState::Charging,
                0x00 | 0x01 => ChargeState::Discharging,
                _ => ChargeState::Unknown,
            };
            Ok(reading(Some(percentage), charge, None))
        }
    }
}

struct GameSirNovaProvider;

impl Provider for GameSirNovaProvider {
    fn id(&self) -> &str {
        "gamesir.nova"
    }

    fn metadata(&self) -> ProviderMetadata {
        experimental_metadata(capabilities(true, true, false), Duration::from_millis(400))
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
        experimental_metadata(capabilities(true, true, false), Duration::from_millis(500))
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
        experimental_metadata(capabilities(true, true, false), Duration::from_millis(450))
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
        experimental_metadata(capabilities(true, true, false), Duration::from_millis(500))
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
            0 => (level.saturating_mul(20).min(100), ChargeState::Discharging),
            1 => (level.saturating_mul(20).min(100), ChargeState::Charging),
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

const EIGHTBITDO_KNOWN_PIDS: &[u16] = &[
    0x310B, 0x6013, 0x6012, 0x202F, 0x202E, 0x301B, 0x301A, 0x310A, 0x3012, 0x3013, 0x3109, 0x6007,
    0x6009, 0x6003, 0x6006, 0x6001, 0x6101, 0x6000, 0x6100,
];

fn eightbitdo_name(endpoint: &Endpoint) -> String {
    match endpoint.pid {
        Some(0x310B | 0x6013) => "8BitDo Ultimate 2 Wireless".to_owned(),
        Some(0x6012) => "8BitDo Ultimate 2".to_owned(),
        Some(0x202F | 0x202E) => "8BitDo Ultimate 3-mode".to_owned(),
        Some(0x301B | 0x301A | 0x310A) => "8BitDo Ultimate 2C Wireless".to_owned(),
        Some(0x3012 | 0x3013 | 0x3109) => "8BitDo Ultimate 2.4G".to_owned(),
        Some(0x6007) => "8BitDo Ultimate Bluetooth".to_owned(),
        Some(0x6009) => "8BitDo Pro 3".to_owned(),
        Some(0x6003 | 0x6006) => "8BitDo Pro 2".to_owned(),
        Some(0x6001 | 0x6101) => "8BitDo SN30 Pro".to_owned(),
        Some(0x6000 | 0x6100) => "8BitDo SF30 Pro".to_owned(),
        _ => endpoint
            .product_name
            .clone()
            .filter(|name| !name.is_empty())
            .unwrap_or_else(|| "8BitDo Wireless Controller".to_owned()),
    }
}

struct EightBitDoVendorProvider;

impl Provider for EightBitDoVendorProvider {
    fn id(&self) -> &str {
        "eightbitdo.vendor"
    }

    fn metadata(&self) -> ProviderMetadata {
        experimental_metadata(capabilities(true, true, false), Duration::from_millis(350))
    }

    fn display_name(&self, endpoint: &Endpoint) -> Option<String> {
        Some(eightbitdo_name(endpoint))
    }

    fn matches(&self, endpoint: &Endpoint) -> MatchDecision {
        if endpoint.vid == Some(0x2DC8)
            && matches!(endpoint.usage_page, Some(0xFF7A) | Some(0xFFA0))
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
        // Query link status first (0x20 0x01).
        drain_input(session, device, deadline, 8)?;
        let mut link_req = vec![0_u8; 64];
        link_req[..4].copy_from_slice(&[0x81, 0x04, 0x20, 0x01]);
        send_with_modes(session, device, OUTPUT, &link_req)?;
        let link = wait_frame(
            session,
            device,
            deadline,
            Duration::from_millis(300),
            12,
            |frame| {
                Ok(frame.len() >= 19
                    && frame[0] == 0x02
                    && frame[1] == 0x04
                    && frame[4] == 0x20
                    && frame[5] == 0x01)
            },
        )?;
        if link[18] == 0 {
            return Ok(ProviderReading {
                reachability: Reachability::Offline,
                source: ReadingSource::VendorHid,
                ..ProviderReading::default()
            });
        }

        // Query battery (0x21 0x01).
        let mut bat_req = vec![0_u8; 64];
        bat_req[..7].copy_from_slice(&[0x81, 0x04, 0x21, 0x01, 0x00, 0x00, 0x06]);
        send_with_modes(session, device, OUTPUT, &bat_req)?;
        let bat = wait_frame(
            session,
            device,
            deadline,
            Duration::from_millis(300),
            12,
            |frame| {
                Ok(frame.len() >= 19
                    && frame[0] == 0x02
                    && frame[1] == 0x04
                    && frame[4] == 0x21
                    && frame[5] == 0x01)
            },
        )?;
        let pct = checked_percentage(bat[18] & 0x7F)?;
        let charging = (bat[18] & 0x80) != 0 && pct < 100;
        let charge = if (bat[18] & 0x80) != 0 && pct == 100 {
            ChargeState::Full
        } else if charging {
            ChargeState::Charging
        } else {
            ChargeState::Discharging
        };
        Ok(reading(Some(pct), charge, None))
    }
}

struct EightBitDoDInputProvider;

impl Provider for EightBitDoDInputProvider {
    fn id(&self) -> &str {
        "eightbitdo.dinput"
    }

    fn metadata(&self) -> ProviderMetadata {
        experimental_metadata(capabilities(true, true, false), Duration::from_millis(500))
    }

    fn display_name(&self, endpoint: &Endpoint) -> Option<String> {
        Some(eightbitdo_name(endpoint))
    }

    fn matches(&self, endpoint: &Endpoint) -> MatchDecision {
        if endpoint.vid == Some(0x2DC8)
            && endpoint
                .pid
                .is_some_and(|pid| EIGHTBITDO_KNOWN_PIDS.contains(&pid))
            && endpoint.usage_page == Some(0x0001)
            && matches!(endpoint.usage, Some(0x0004) | Some(0x0005))
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
        let needs_feature_06 = matches!(
            device.endpoint.pid,
            Some(0x6009 | 0x6003 | 0x6006 | 0x6001 | 0x6101 | 0x6000 | 0x6100)
        );
        if needs_feature_06 {
            let _ = session.get_feature(0x06, 63);
        }
        let min_len = if device.endpoint.pid == Some(0x6012) {
            34
        } else {
            15
        };
        let frame = wait_frame(
            session,
            device,
            deadline,
            Duration::from_millis(500),
            16,
            |frame| Ok(frame.len() >= min_len && matches!(frame[0], 0x01 | 0x04)),
        )?;
        let pct = checked_percentage(frame[14] & 0x7F)?;
        let charge = if (frame[14] & 0x80) != 0 {
            if pct == 100 {
                ChargeState::Full
            } else {
                ChargeState::Charging
            }
        } else {
            ChargeState::Discharging
        };
        Ok(reading(Some(pct), charge, None))
    }
}

struct BeitongProvider;

impl Provider for BeitongProvider {
    fn id(&self) -> &str {
        "beitong.controller"
    }

    fn metadata(&self) -> ProviderMetadata {
        experimental_metadata(capabilities(true, true, false), Duration::from_millis(450))
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
