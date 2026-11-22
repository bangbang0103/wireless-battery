//! Keychron mouse and QMK keyboard HID battery protocols (Nape Pro, Legacy 1K, Ultra-Link 8K, Launcher, QMK Link).

use crate::error::BatteryError;
use crate::model::{
    ChargeState, DeviceKind, Endpoint, ProviderReading, Reachability, ReadingSource,
};
use crate::providers::fixed::{
    capabilities, checked_percentage, checksum_sum, drain_input, experimental_metadata,
    feature_frame, reading, send_with_modes, sleep, unnumbered_payload, wait_frame, SendMode,
    OUTPUT,
};
use crate::providers::{
    infer_device_kind_from_name, Deadline, DeviceContext, MatchDecision, Provider, ProviderMetadata,
};
use crate::transport::HidSession;
use std::sync::Arc;
use std::time::Duration;

pub(crate) fn providers() -> Vec<Arc<dyn Provider>> {
    vec![
        Arc::new(KeychronNapeProvider),
        Arc::new(KeychronLegacyMouseProvider),
        Arc::new(KeychronUltraLinkProvider),
        Arc::new(KeychronLauncherMouseProvider),
        Arc::new(KeychronQmkKeyboardProvider),
    ]
}

struct KeychronNapeProvider;

impl Provider for KeychronNapeProvider {
    fn id(&self) -> &str {
        "keychron.nape"
    }

    fn metadata(&self) -> ProviderMetadata {
        experimental_metadata(
            DeviceKind::Mouse,
            capabilities(true, true, false),
            Duration::from_millis(500),
        )
    }

    fn display_name(&self, _endpoint: &Endpoint) -> Option<String> {
        Some("Keychron Nape Pro".to_owned())
    }

    fn matches(&self, endpoint: &Endpoint) -> MatchDecision {
        if endpoint.vid == Some(0x3434)
            && matches!(endpoint.pid, Some(0xD026 | 0x0440))
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
        if device.endpoint.pid == Some(0xD026) {
            let mut slot_req = vec![0_u8; 33];
            slot_req[1] = 0xB2;
            send_with_modes(session, device, OUTPUT, &slot_req)?;
            let slot_frame = wait_frame(
                session,
                device,
                deadline,
                Duration::from_millis(500),
                12,
                |f| {
                    let r = unnumbered_payload(f);
                    Ok(r.len() >= 17 && r[0] == 0xB2)
                },
            )?;
            let r = unnumbered_payload(&slot_frame);
            let linked = [2_usize, 7, 12].into_iter().any(|off| {
                u16::from_be_bytes([r[off + 2], r[off + 3]]) == 0x4004 && r[off + 4] == 1
            });
            if !linked {
                return Ok(ProviderReading {
                    reachability: Reachability::Offline,
                    source: ReadingSource::VendorHid,
                    ..ProviderReading::default()
                });
            }
        }
        let mut req = vec![0_u8; 33];
        req[1] = 0xA7;
        req[2] = 0x31;
        send_with_modes(session, device, OUTPUT, &req)?;
        let frame = wait_frame(
            session,
            device,
            deadline,
            Duration::from_millis(500),
            12,
            |f| {
                let r = unnumbered_payload(f);
                Ok(r.len() >= 4 && r[0] == 0xA7 && r[1] == 0x31)
            },
        )?;
        let r = unnumbered_payload(&frame);
        let pct = checked_percentage(r[2])?;
        let charge = if r[3] != 0 {
            ChargeState::Charging
        } else {
            ChargeState::Discharging
        };
        Ok(reading(Some(pct), charge, None))
    }
}

struct KeychronLegacyMouseProvider;

impl Provider for KeychronLegacyMouseProvider {
    fn id(&self) -> &str {
        "keychron.legacy_mouse"
    }

    fn metadata(&self) -> ProviderMetadata {
        experimental_metadata(
            DeviceKind::Mouse,
            capabilities(true, true, false),
            Duration::from_millis(300),
        )
    }

    fn display_name(&self, _endpoint: &Endpoint) -> Option<String> {
        Some("Keychron wireless mouse".to_owned())
    }

    fn matches(&self, endpoint: &Endpoint) -> MatchDecision {
        if endpoint.vid == Some(0x3434)
            && matches!(endpoint.pid, Some(0xD030 | 0xD031))
            && endpoint.usage_page == Some(0x008C)
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
        let mut req = vec![0_u8; 21];
        req[0] = 0x51;
        req[1] = 0x06;
        send_with_modes(session, device, &[SendMode::Feature], &req)?;
        for _ in 0..5 {
            sleep(deadline, Duration::from_millis(40))?;
            let r = feature_frame(session, device, 0x51, 20)?;
            if r.len() >= 13
                && r[0] == 0x51
                && r[1] == 0x06
                && r[2] == 0x01
                && u16::from_le_bytes([r[4], r[5]]) == 0x3434
            {
                let pct = checked_percentage(r[11])?;
                let charge = if r[12] == 0x01 {
                    ChargeState::Charging
                } else {
                    ChargeState::Discharging
                };
                return Ok(reading(Some(pct), charge, None));
            }
        }
        Err(BatteryError::timeout("Keychron legacy mouse"))
    }
}

struct KeychronUltraLinkProvider;

impl Provider for KeychronUltraLinkProvider {
    fn id(&self) -> &str {
        "keychron.ultralink"
    }

    fn metadata(&self) -> ProviderMetadata {
        experimental_metadata(
            DeviceKind::Mouse,
            capabilities(true, true, false),
            Duration::from_millis(500),
        )
    }

    fn display_name(&self, _endpoint: &Endpoint) -> Option<String> {
        Some("Keychron Ultra-Link 8K mouse".to_owned())
    }

    fn matches(&self, endpoint: &Endpoint) -> MatchDecision {
        if endpoint.vid == Some(0x3434)
            && endpoint.pid == Some(0xD028)
            && endpoint.usage_page == Some(0xFFC1)
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
        let mut slot_req = vec![0_u8; 21];
        slot_req[..2].copy_from_slice(&[0xB5, 0x03]);
        send_with_modes(session, device, OUTPUT, &slot_req)?;
        let slot = wait_frame(
            session,
            device,
            deadline,
            Duration::from_millis(500),
            12,
            |r| Ok(r.len() >= 21 && r[1] == 0x03),
        )?;
        let linked = [3_usize, 8, 13]
            .into_iter()
            .any(|off| (slot[off] | slot[off + 1]) != 0 && slot[off + 4] != 0);
        if !linked {
            return Ok(ProviderReading {
                reachability: Reachability::Offline,
                source: ReadingSource::VendorHid,
                ..ProviderReading::default()
            });
        }
        let mut bat_req = vec![0_u8; 21];
        bat_req[..2].copy_from_slice(&[0xB3, 0x06]);
        send_with_modes(session, device, OUTPUT, &bat_req)?;
        let r = wait_frame(
            session,
            device,
            deadline,
            Duration::from_millis(500),
            12,
            |r| Ok(r.len() >= 21 && r[1] == 0x06),
        )?;
        let power = r[20];
        let pct = checked_percentage(power & 0x7F)?;
        if pct == 0 {
            return Err(BatteryError::invalid_response(
                "Keychron Ultra-Link returned 0%",
            ));
        }
        let charge = if (power & 0x80) != 0 {
            ChargeState::Charging
        } else {
            ChargeState::Discharging
        };
        Ok(reading(Some(pct), charge, None))
    }
}

struct KeychronLauncherMouseProvider;

impl Provider for KeychronLauncherMouseProvider {
    fn id(&self) -> &str {
        "keychron.launcher_mouse"
    }

    fn metadata(&self) -> ProviderMetadata {
        experimental_metadata(
            DeviceKind::Mouse,
            capabilities(true, true, false),
            Duration::from_millis(500),
        )
    }

    fn device_kind(&self, endpoint: &Endpoint) -> DeviceKind {
        infer_device_kind_from_name(endpoint.product_name.as_deref()).unwrap_or(DeviceKind::Mouse)
    }

    fn display_name(&self, endpoint: &Endpoint) -> Option<String> {
        Some(
            endpoint
                .product_name
                .clone()
                .filter(|name| !name.is_empty())
                .unwrap_or_else(|| "Keychron mouse".to_owned()),
        )
    }

    fn matches(&self, endpoint: &Endpoint) -> MatchDecision {
        // Exact Keychron mouse models on 0x008C / 0xFFC1 / 0xFF60 use dedicated matchers.
        if endpoint.vid == Some(0x3434)
            && endpoint
                .usage_page
                .is_some_and(|p| p >= 0xFF00 && p != 0xFF60 && p != 0xFFC1)
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
        let ask = |session: &mut dyn HidSession, cmd: u8, sub: u8| {
            drain_input(session, device, deadline, 6)?;
            let mut req = vec![0_u8; 65];
            req[1] = cmd;
            req[3] = 0x81;
            req[4] = sub;
            req[64] = 0xA1_u8.wrapping_sub(checksum_sum(&req[1..64]));
            send_with_modes(session, device, OUTPUT, &req)?;
            wait_frame(
                session,
                device,
                deadline,
                Duration::from_millis(500),
                12,
                |f| {
                    let r = unnumbered_payload(f);
                    Ok(r.len() >= 13 && r[0] == cmd && r[3] == sub)
                },
            )
        };
        if let Ok(frame1) = ask(session, 0x01, 0x01) {
            let r1 = unnumbered_payload(&frame1);
            let tag = u16::from_le_bytes([r1[6], r1[7]]);
            if device.endpoint.usage_page == Some(0xFF0A)
                && device.endpoint.usage == Some(0x0001)
                && tag != 0x3434
                && tag != 0x362D
            {
                let frame41 = ask(session, 0x41, 0x01)?;
                let r = unnumbered_payload(&frame41);
                let pct = checked_percentage(r[6])?;
                let charge = if r[5] != 0 {
                    ChargeState::Charging
                } else {
                    ChargeState::Discharging
                };
                return Ok(reading(Some(pct), charge, None));
            }
            if tag == 0x3434 && r1[11] <= 100 {
                let charge = if r1[10] != 0 {
                    ChargeState::Charging
                } else {
                    ChargeState::Discharging
                };
                return Ok(reading(Some(r1[11]), charge, None));
            }
            if r1[10] <= 3 && (1..=100).contains(&r1[11]) && (1..=8).contains(&r1[5]) {
                let charge = if r1[10] == 1 {
                    ChargeState::Charging
                } else {
                    ChargeState::Discharging
                };
                return Ok(reading(Some(r1[11]), charge, None));
            }
        }
        let frame41 = ask(session, 0x41, 0x01)?;
        let r = unnumbered_payload(&frame41);
        if r[10] <= 3 && r[11] <= 100 {
            let charge = if r[10] == 1 {
                ChargeState::Charging
            } else {
                ChargeState::Discharging
            };
            return Ok(reading(Some(r[11]), charge, None));
        }
        Err(BatteryError::invalid_response(
            "unexpected Keychron Launcher response",
        ))
    }
}

struct KeychronQmkKeyboardProvider;

impl Provider for KeychronQmkKeyboardProvider {
    fn id(&self) -> &str {
        "keychron.qmk_keyboard"
    }

    fn metadata(&self) -> ProviderMetadata {
        experimental_metadata(
            DeviceKind::Keyboard,
            capabilities(true, true, true),
            Duration::from_millis(400),
        )
    }

    fn display_name(&self, endpoint: &Endpoint) -> Option<String> {
        Some(
            endpoint
                .product_name
                .clone()
                .filter(|name| !name.is_empty())
                .unwrap_or_else(|| "Keychron QMK keyboard".to_owned()),
        )
    }

    fn matches(&self, endpoint: &Endpoint) -> MatchDecision {
        // Nape Pro shares VID 0x3434 and usage_page 0xFF60:0x0061, and wins with MatchDecision::Exact.
        if endpoint.vid == Some(0x3434)
            && endpoint.usage_page == Some(0xFF60)
            && endpoint.usage == Some(0x0061)
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
        drain_input(session, device, deadline, 6)?;
        let mut req = vec![0_u8; 33];
        req[1] = 0xA4;
        send_with_modes(session, device, OUTPUT, &req)?;
        let frame = wait_frame(
            session,
            device,
            deadline,
            Duration::from_millis(400),
            12,
            |f| {
                let r = unnumbered_payload(f);
                Ok(r.len() >= 8 && r[0] == 0xA4)
            },
        )?;
        let r = unnumbered_payload(&frame);
        let pct = checked_percentage(r[1])?;
        let mv = u32::from(u16::from_le_bytes([r[2], r[3]]));
        let charge = match r[4] {
            0 => ChargeState::Discharging,
            1 => ChargeState::Charging,
            2 => ChargeState::Full,
            _ => ChargeState::Unknown,
        };
        Ok(reading(Some(pct), charge, (mv > 0).then_some(mv)))
    }
}
