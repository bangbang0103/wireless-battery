//! Wireless keyboard and shared keyboard/dongle HID battery protocols:
//! Keychron (Nape Pro, Legacy 1K, Ultra-Link 8K, Launcher 4K/8K, QMK Link),
//! CHERRY MX 3.0S, Akko Dash, NuPhy WH80, Evision (ATK/VGN/VXE/Z87/Redragon/AJAZZ/AULA),
//! AULA HERO 68 Air & SC580, SONiX (20 dongles), PMO/VIA 36B0, and Semico 1A2C.

use crate::error::BatteryError;
use crate::model::{ChargeState, Endpoint, ProviderReading, Reachability, ReadingSource};
use crate::providers::fixed::{
    capabilities, checked_percentage, checksum_sum, drain_input, experimental_metadata,
    feature_frame, reading, send_with_modes, sleep, unnumbered_payload, wait_frame, SendMode,
    OUTPUT,
};
use crate::providers::{Deadline, DeviceContext, MatchDecision, Provider, ProviderMetadata};
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
        Arc::new(CherryAkkoEvisionProvider::cherry()),
        Arc::new(CherryAkkoEvisionProvider::akko()),
        Arc::new(NuphyWh80Provider),
        Arc::new(EvisionSeqProvider::atk_cable()),
        Arc::new(EvisionSeqProvider::atk_z87()),
        Arc::new(EvisionSeqProvider::redragon_m999()),
        Arc::new(EvisionSeqProvider::ajazz_ak992()),
        Arc::new(AulaMouseEvisionProvider),
        Arc::new(AulaHero68Provider),
        Arc::new(AulaSc580Provider),
        Arc::new(SonixKeyboardProvider),
        Arc::new(ViaDongleProvider),
        Arc::new(SemicoKeyboardProvider),
    ]
}

struct KeychronNapeProvider;

impl Provider for KeychronNapeProvider {
    fn id(&self) -> &str {
        "keychron.nape"
    }

    fn metadata(&self) -> ProviderMetadata {
        experimental_metadata(capabilities(true, true, false), Duration::from_millis(500))
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
        experimental_metadata(capabilities(true, true, false), Duration::from_millis(300))
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
        experimental_metadata(capabilities(true, true, false), Duration::from_millis(500))
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
        experimental_metadata(capabilities(true, true, false), Duration::from_millis(500))
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
        if endpoint.vid == Some(0x3434)
            && !matches!(endpoint.pid, Some(0xD026 | 0x0440 | 0xD028))
            && endpoint
                .usage_page
                .is_some_and(|p| p >= 0xFF00 && p != 0xFF60 && p != 0xFFC1)
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
        experimental_metadata(capabilities(true, true, true), Duration::from_millis(400))
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
        if endpoint.vid == Some(0x3434)
            && !matches!(endpoint.pid, Some(0xD026 | 0x0440))
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

struct CherryAkkoEvisionProvider {
    akko: bool,
}

impl CherryAkkoEvisionProvider {
    const fn cherry() -> Self {
        Self { akko: false }
    }
    const fn akko() -> Self {
        Self { akko: true }
    }
}

impl Provider for CherryAkkoEvisionProvider {
    fn id(&self) -> &str {
        if self.akko {
            "akko.dash"
        } else {
            "cherry.mx3s"
        }
    }

    fn metadata(&self) -> ProviderMetadata {
        experimental_metadata(capabilities(true, true, false), Duration::from_millis(400))
    }

    fn display_name(&self, endpoint: &Endpoint) -> Option<String> {
        Some(if self.akko {
            match endpoint.pid {
                Some(0x000F) => "Akko Dash V9 Ultra".to_owned(),
                _ => "Akko V9 Dash Pro".to_owned(),
            }
        } else {
            "CHERRY MX 3.0S Wireless".to_owned()
        })
    }

    fn matches(&self, endpoint: &Endpoint) -> MatchDecision {
        let vid_pid = if self.akko {
            endpoint.vid == Some(0x38EE) && matches!(endpoint.pid, Some(0x000C | 0x000F))
        } else {
            endpoint.vid == Some(0x046A) && matches!(endpoint.pid, Some(0x00E3 | 0x00EA))
        };
        if vid_pid && endpoint.usage_page == Some(0xFF1C) && endpoint.usage == Some(0x0092) {
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
        req[..8].copy_from_slice(&[0x04, 0x72, 0x00, 0x1A, 0x03, 0x00, 0x00, 0x55]);
        send_with_modes(session, device, OUTPUT, &req)?;
        let r = wait_frame(
            session,
            device,
            deadline,
            Duration::from_millis(400),
            12,
            |r| Ok(r.len() >= 10 && r[0] == 0x04 && r[3] == 0x1A),
        )?;
        if r[8] > 100 {
            return Ok(ProviderReading {
                reachability: Reachability::Sleeping,
                source: ReadingSource::VendorHid,
                ..ProviderReading::default()
            });
        }
        let charge = if r[9] != 0 {
            ChargeState::Charging
        } else {
            ChargeState::Discharging
        };
        Ok(reading(Some(r[8]), charge, None))
    }
}

struct NuphyWh80Provider;

impl Provider for NuphyWh80Provider {
    fn id(&self) -> &str {
        "nuphy.wh80"
    }

    fn metadata(&self) -> ProviderMetadata {
        experimental_metadata(capabilities(true, true, false), Duration::from_millis(400))
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

#[derive(Clone, Copy)]
enum EvisionSeqKind {
    AtkCable,
    AtkZ87,
    RedragonM999,
    AjazzAk992,
}

struct EvisionSeqProvider {
    kind: EvisionSeqKind,
}

impl EvisionSeqProvider {
    const fn atk_cable() -> Self {
        Self {
            kind: EvisionSeqKind::AtkCable,
        }
    }
    const fn atk_z87() -> Self {
        Self {
            kind: EvisionSeqKind::AtkZ87,
        }
    }
    const fn redragon_m999() -> Self {
        Self {
            kind: EvisionSeqKind::RedragonM999,
        }
    }
    const fn ajazz_ak992() -> Self {
        Self {
            kind: EvisionSeqKind::AjazzAk992,
        }
    }
}

impl Provider for EvisionSeqProvider {
    fn id(&self) -> &str {
        match self.kind {
            EvisionSeqKind::AtkCable => "evision.atk_keyboard",
            EvisionSeqKind::AtkZ87 => "atk.z87",
            EvisionSeqKind::RedragonM999 => "evision.redragon_m999",
            EvisionSeqKind::AjazzAk992 => "evision.ajazz_ak992",
        }
    }

    fn metadata(&self) -> ProviderMetadata {
        experimental_metadata(capabilities(true, true, false), Duration::from_millis(600))
    }

    fn display_name(&self, endpoint: &Endpoint) -> Option<String> {
        let fallback = match self.kind {
            EvisionSeqKind::AtkCable => "ATK / VGN keyboard",
            EvisionSeqKind::AtkZ87 => "ATK Z87",
            EvisionSeqKind::RedragonM999 => "Redragon K1NG M999",
            EvisionSeqKind::AjazzAk992 => "AJAZZ AK992",
        };
        Some(
            endpoint
                .product_name
                .clone()
                .filter(|n| !n.is_empty())
                .unwrap_or_else(|| fallback.to_owned()),
        )
    }

    fn matches(&self, endpoint: &Endpoint) -> MatchDecision {
        if endpoint.usage_page != Some(0xFF1C) || endpoint.usage != Some(0x0092) {
            return MatchDecision::NoMatch;
        }
        let matched = match self.kind {
            EvisionSeqKind::AtkCable => {
                endpoint.vid == Some(0x320F) && endpoint.pid == Some(0x5055)
            }
            EvisionSeqKind::AtkZ87 => endpoint.vid == Some(0x373B) && endpoint.pid == Some(0x1012),
            EvisionSeqKind::RedragonM999 => {
                endpoint.vid == Some(0x320F) && endpoint.pid == Some(0x226F)
            }
            EvisionSeqKind::AjazzAk992 => {
                endpoint.vid == Some(0x320F) && endpoint.pid == Some(0x50C3)
            }
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
        let sub = match self.kind {
            EvisionSeqKind::AtkCable => {
                let is_vgn = device.endpoint.product_name.as_deref().is_some_and(|n| {
                    let u = n.to_ascii_uppercase();
                    u.contains("VGN") || u.contains("VXE")
                });
                if is_vgn {
                    0x06
                } else {
                    0x00
                }
            }
            EvisionSeqKind::AtkZ87 | EvisionSeqKind::AjazzAk992 => 0x00,
            EvisionSeqKind::RedragonM999 => 0x02,
        };
        drain_input(session, device, deadline, 6)?;
        let mut req = vec![0_u8; 64];
        req[..5].copy_from_slice(&[0x04, 0x01, 0x00, 0x1A, sub]);
        send_with_modes(session, device, OUTPUT, &req)?;
        let r = wait_frame(
            session,
            device,
            deadline,
            Duration::from_millis(600),
            12,
            |r| {
                if r.len() < 10 || r[0] != 0x04 || r[1] != 0x01 || r[2] != 0x00 {
                    return Ok(false);
                }
                if matches!(self.kind, EvisionSeqKind::AtkZ87) && r[3] == 0xFF {
                    return Ok(true);
                }
                Ok(r[3] == 0x1A && r[5] == 0x00 && r[6] == 0x00)
            },
        )?;
        if matches!(self.kind, EvisionSeqKind::AtkZ87) && r[3] == 0xFF {
            return Ok(ProviderReading {
                reachability: Reachability::Sleeping,
                source: ReadingSource::VendorHid,
                ..ProviderReading::default()
            });
        }
        let pct = checked_percentage(r[8])?;
        let charge = match r[9] {
            0 => ChargeState::Discharging,
            1 | 2 => ChargeState::Charging,
            _ => ChargeState::Unknown,
        };
        Ok(reading(Some(pct), charge, None))
    }
}

struct AulaMouseEvisionProvider;

impl Provider for AulaMouseEvisionProvider {
    fn id(&self) -> &str {
        "evision.aula_mouse"
    }

    fn metadata(&self) -> ProviderMetadata {
        experimental_metadata(capabilities(true, true, false), Duration::from_millis(500))
    }

    fn display_name(&self, _endpoint: &Endpoint) -> Option<String> {
        Some("AULA X9 Ultra / V9 mouse".to_owned())
    }

    fn matches(&self, endpoint: &Endpoint) -> MatchDecision {
        if endpoint.vid == Some(0x320F)
            && endpoint.pid == Some(0x22A0)
            && endpoint.usage_page == Some(0xFF1C)
            && endpoint.usage == Some(0x0092)
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
        let cmd_frame = |cmd: u8| {
            let mut f = vec![0_u8; 64];
            f[0] = 0x04;
            f[3] = cmd;
            f
        };
        let is_reply = |r: &[u8], cmd: u8| {
            r.len() >= 8
                && r[0] == 0x04
                && r[1] == 0
                && r[2] == 0
                && r[3] == cmd
                && r[4..8] == [0, 0, 0, 0]
        };
        drain_input(session, device, deadline, 6)?;
        send_with_modes(session, device, OUTPUT, &cmd_frame(0x01))?;
        let _start = wait_frame(
            session,
            device,
            deadline,
            Duration::from_millis(300),
            8,
            |r| Ok(is_reply(r, 0x01)),
        )?;
        send_with_modes(session, device, OUTPUT, &cmd_frame(0x1A))?;
        let bat = wait_frame(
            session,
            device,
            deadline,
            Duration::from_millis(400),
            8,
            |r| Ok(r.len() >= 10 && is_reply(r, 0x1A)),
        );
        let _ = send_with_modes(session, device, OUTPUT, &cmd_frame(0x02));
        let r = bat?;
        if r[8] == 0 || r[8] > 100 {
            return Ok(ProviderReading {
                reachability: Reachability::Sleeping,
                source: ReadingSource::VendorHid,
                ..ProviderReading::default()
            });
        }
        let charge = match r[9] {
            0 => ChargeState::Discharging,
            1 | 2 => ChargeState::Charging,
            _ => ChargeState::Unknown,
        };
        Ok(reading(Some(r[8]), charge, None))
    }
}

struct AulaHero68Provider;

impl Provider for AulaHero68Provider {
    fn id(&self) -> &str {
        "aula.hero68"
    }

    fn metadata(&self) -> ProviderMetadata {
        experimental_metadata(capabilities(true, true, false), Duration::from_millis(500))
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

struct AulaSc580Provider;

impl Provider for AulaSc580Provider {
    fn id(&self) -> &str {
        "aula.sc580"
    }

    fn metadata(&self) -> ProviderMetadata {
        experimental_metadata(capabilities(true, true, false), Duration::from_millis(500))
    }

    fn display_name(&self, _endpoint: &Endpoint) -> Option<String> {
        Some("AULA SC580".to_owned())
    }

    fn matches(&self, endpoint: &Endpoint) -> MatchDecision {
        let vid_pid = (endpoint.vid == Some(0x088D) && endpoint.pid == Some(0x062E))
            || (endpoint.vid == Some(0x089D) && endpoint.pid == Some(0x062F));
        if vid_pid && endpoint.usage_page == Some(0xFF02) && endpoint.usage == Some(0x0000) {
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
        req[1] = 0x30;
        req[3] = 0x01;
        send_with_modes(session, device, OUTPUT, &req)?;
        let frame = wait_frame(
            session,
            device,
            deadline,
            Duration::from_millis(500),
            12,
            |f| {
                let r = unnumbered_payload(f);
                Ok(r.len() >= 32 && r[0] == 0x30 && matches!(r[1], 0x00 | 0x02))
            },
        )?;
        let r = unnumbered_payload(&frame);
        if r[1] == 0x02 {
            return Ok(ProviderReading {
                reachability: Reachability::Sleeping,
                source: ReadingSource::VendorHid,
                ..ProviderReading::default()
            });
        }
        if r[31] != checksum_sum(&r[4..31]) {
            return Err(BatteryError::ChecksumFailed);
        }
        if r[14] == 0 {
            return Ok(ProviderReading {
                reachability: Reachability::Sleeping,
                source: ReadingSource::VendorHid,
                ..ProviderReading::default()
            });
        }
        if r[12] == 1 {
            // SC580 firmware freezes percentage at 100 while charging; keep percentage unknown.
            return Ok(reading(None, ChargeState::Charging, None));
        }
        let pct = checked_percentage(r[13])?;
        Ok(reading(Some(pct), ChargeState::Discharging, None))
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
        experimental_metadata(capabilities(true, false, false), Duration::from_millis(500))
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
        experimental_metadata(capabilities(true, false, false), Duration::from_millis(500))
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
        experimental_metadata(capabilities(true, false, false), Duration::from_millis(500))
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
