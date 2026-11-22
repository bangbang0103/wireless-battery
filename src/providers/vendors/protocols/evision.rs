//! Evision 0xFF1C:0x0092 wireless peripheral battery protocol family
//! (CHERRY MX 3.0S, Akko Dash, ATK/VGN keyboards, Redragon M999, AJAZZ AK992, AULA mice).

use crate::error::BatteryError;
use crate::model::{
    ChargeState, DeviceKind, Endpoint, ProviderReading, Reachability, ReadingSource,
};
use crate::providers::fixed::{
    capabilities, checked_percentage, drain_input, experimental_metadata, reading, send_with_modes,
    wait_frame, OUTPUT,
};
use crate::providers::{Deadline, DeviceContext, MatchDecision, Provider, ProviderMetadata};
use crate::transport::HidSession;
use std::sync::Arc;
use std::time::Duration;

pub(crate) fn providers() -> Vec<Arc<dyn Provider>> {
    vec![
        Arc::new(CherryAkkoEvisionProvider::cherry()),
        Arc::new(CherryAkkoEvisionProvider::akko()),
        Arc::new(EvisionSeqProvider::atk_cable()),
        Arc::new(EvisionSeqProvider::atk_z87()),
        Arc::new(EvisionSeqProvider::redragon_m999()),
        Arc::new(EvisionSeqProvider::ajazz_ak992()),
        Arc::new(AulaMouseEvisionProvider),
    ]
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
        experimental_metadata(
            DeviceKind::Keyboard,
            capabilities(true, true, false),
            Duration::from_millis(400),
        )
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
        let device_kind = match self.kind {
            EvisionSeqKind::RedragonM999 => DeviceKind::Mouse,
            EvisionSeqKind::AtkCable | EvisionSeqKind::AtkZ87 | EvisionSeqKind::AjazzAk992 => {
                DeviceKind::Keyboard
            }
        };
        experimental_metadata(
            device_kind,
            capabilities(true, true, false),
            Duration::from_millis(600),
        )
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
        experimental_metadata(
            DeviceKind::Mouse,
            capabilities(true, true, false),
            Duration::from_millis(500),
        )
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
