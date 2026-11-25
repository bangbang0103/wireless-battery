//! ASUS ROG standalone keyboard and mouse HID battery protocols.

use crate::error::BatteryError;
use crate::model::{
    ChargeState, DeviceKind, Endpoint, ProviderReading, Reachability, ReadingSource,
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
    vec![Arc::new(AsusRogPeripheralProvider)]
}

#[derive(Clone, Copy)]
struct RogModel {
    pid: u16,
    name: &'static str,
    keyboard: bool,
    report_id: u8,
    offset: usize,
    scale: u8,
}

const ROG_MODELS: &[RogModel] = &[
    // Keyboards
    RogModel {
        pid: 0x1A85,
        name: "ASUS ROG Azoth",
        keyboard: true,
        report_id: 2,
        offset: 6,
        scale: 1,
    },
    RogModel {
        pid: 0x1A83,
        name: "ASUS ROG Azoth",
        keyboard: true,
        report_id: 0,
        offset: 6,
        scale: 1,
    },
    RogModel {
        pid: 0x1B42,
        name: "ASUS ROG Azoth Extreme",
        keyboard: true,
        report_id: 2,
        offset: 6,
        scale: 1,
    },
    RogModel {
        pid: 0x1B3F,
        name: "ASUS ROG Azoth Extreme",
        keyboard: true,
        report_id: 0,
        offset: 6,
        scale: 1,
    },
    RogModel {
        pid: 0x1CF1,
        name: "ASUS ROG Azoth Extreme SE",
        keyboard: true,
        report_id: 2,
        offset: 6,
        scale: 1,
    },
    RogModel {
        pid: 0x1CEF,
        name: "ASUS ROG Azoth Extreme SE",
        keyboard: true,
        report_id: 0,
        offset: 6,
        scale: 1,
    },
    RogModel {
        pid: 0x1C25,
        name: "ASUS ROG Azoth X",
        keyboard: true,
        report_id: 0,
        offset: 6,
        scale: 1,
    },
    RogModel {
        pid: 0x1C24,
        name: "ASUS ROG Azoth X",
        keyboard: true,
        report_id: 0,
        offset: 6,
        scale: 1,
    },
    RogModel {
        pid: 0x193E,
        name: "ASUS ROG Falchion",
        keyboard: true,
        report_id: 0,
        offset: 11,
        scale: 1,
    },
    RogModel {
        pid: 0x193C,
        name: "ASUS ROG Falchion",
        keyboard: true,
        report_id: 0,
        offset: 11,
        scale: 1,
    },
    RogModel {
        pid: 0x1B04,
        name: "ASUS ROG Falchion RX",
        keyboard: true,
        report_id: 0,
        offset: 11,
        scale: 1,
    },
    RogModel {
        pid: 0x1B06,
        name: "ASUS ROG Falchion RX LP",
        keyboard: true,
        report_id: 2,
        offset: 11,
        scale: 1,
    },
    RogModel {
        pid: 0x1AB0,
        name: "ASUS ROG Strix Scope II 96",
        keyboard: true,
        report_id: 2,
        offset: 6,
        scale: 1,
    },
    RogModel {
        pid: 0x1AAE,
        name: "ASUS ROG Strix Scope II 96",
        keyboard: true,
        report_id: 0,
        offset: 6,
        scale: 1,
    },
    RogModel {
        pid: 0x1B7A,
        name: "ASUS ROG Strix Scope II 96 RX",
        keyboard: true,
        report_id: 2,
        offset: 6,
        scale: 1,
    },
    RogModel {
        pid: 0x1B78,
        name: "ASUS ROG Strix Scope II 96 RX",
        keyboard: true,
        report_id: 0,
        offset: 6,
        scale: 1,
    },
    RogModel {
        pid: 0x1A07,
        name: "ASUS ROG Strix Scope RX TKL",
        keyboard: true,
        report_id: 0,
        offset: 6,
        scale: 1,
    },
    RogModel {
        pid: 0x1A05,
        name: "ASUS ROG Strix Scope RX TKL",
        keyboard: true,
        report_id: 0,
        offset: 6,
        scale: 1,
    },
    // Mice (0..4 gauge, scale 25)
    RogModel {
        pid: 0x18E5,
        name: "ASUS ROG Chakram",
        keyboard: false,
        report_id: 0,
        offset: 5,
        scale: 25,
    },
    RogModel {
        pid: 0x18E3,
        name: "ASUS ROG Chakram",
        keyboard: false,
        report_id: 0,
        offset: 5,
        scale: 25,
    },
    RogModel {
        pid: 0x18A0,
        name: "ASUS ROG Gladius II Wireless",
        keyboard: false,
        report_id: 0,
        offset: 5,
        scale: 25,
    },
    RogModel {
        pid: 0x1960,
        name: "ASUS ROG Keris Wireless",
        keyboard: false,
        report_id: 0,
        offset: 5,
        scale: 25,
    },
    RogModel {
        pid: 0x195E,
        name: "ASUS ROG Keris Wireless",
        keyboard: false,
        report_id: 0,
        offset: 5,
        scale: 25,
    },
    RogModel {
        pid: 0x1A59,
        name: "ASUS ROG Keris EVA",
        keyboard: false,
        report_id: 0,
        offset: 5,
        scale: 25,
    },
    RogModel {
        pid: 0x1A57,
        name: "ASUS ROG Keris EVA",
        keyboard: false,
        report_id: 0,
        offset: 5,
        scale: 25,
    },
    RogModel {
        pid: 0x1908,
        name: "ASUS ROG Pugio II",
        keyboard: false,
        report_id: 0,
        offset: 5,
        scale: 25,
    },
    RogModel {
        pid: 0x1906,
        name: "ASUS ROG Pugio II",
        keyboard: false,
        report_id: 0,
        offset: 5,
        scale: 25,
    },
    RogModel {
        pid: 0x18B4,
        name: "ASUS ROG Strix Carry",
        keyboard: false,
        report_id: 0,
        offset: 7,
        scale: 25,
    },
    RogModel {
        pid: 0x1949,
        name: "ASUS ROG Strix Impact II Wireless",
        keyboard: false,
        report_id: 0,
        offset: 5,
        scale: 25,
    },
    RogModel {
        pid: 0x1947,
        name: "ASUS ROG Strix Impact II Wireless",
        keyboard: false,
        report_id: 0,
        offset: 5,
        scale: 25,
    },
    // Mice (0..100%)
    RogModel {
        pid: 0x1A1A,
        name: "ASUS ROG Chakram X",
        keyboard: false,
        report_id: 0,
        offset: 5,
        scale: 1,
    },
    RogModel {
        pid: 0x1A18,
        name: "ASUS ROG Chakram X",
        keyboard: false,
        report_id: 0,
        offset: 5,
        scale: 1,
    },
    RogModel {
        pid: 0x197F,
        name: "ASUS ROG Gladius III Wireless",
        keyboard: false,
        report_id: 0,
        offset: 5,
        scale: 1,
    },
    RogModel {
        pid: 0x197D,
        name: "ASUS ROG Gladius III Wireless",
        keyboard: false,
        report_id: 0,
        offset: 5,
        scale: 1,
    },
    RogModel {
        pid: 0x1A72,
        name: "ASUS ROG Gladius III Aimpoint",
        keyboard: false,
        report_id: 3,
        offset: 5,
        scale: 1,
    },
    RogModel {
        pid: 0x1A70,
        name: "ASUS ROG Gladius III Aimpoint",
        keyboard: false,
        report_id: 0,
        offset: 5,
        scale: 1,
    },
    RogModel {
        pid: 0x1B0C,
        name: "ASUS ROG Gladius III Aimpoint EVA-02",
        keyboard: false,
        report_id: 0,
        offset: 5,
        scale: 1,
    },
    RogModel {
        pid: 0x1B0A,
        name: "ASUS ROG Gladius III Aimpoint EVA-02",
        keyboard: false,
        report_id: 0,
        offset: 5,
        scale: 1,
    },
    RogModel {
        pid: 0x1A94,
        name: "ASUS ROG Harpe Ace Aim Lab",
        keyboard: false,
        report_id: 3,
        offset: 5,
        scale: 1,
    },
    RogModel {
        pid: 0x1A92,
        name: "ASUS ROG Harpe Ace Aim Lab",
        keyboard: false,
        report_id: 0,
        offset: 5,
        scale: 1,
    },
    RogModel {
        pid: 0x1B68,
        name: "ASUS ROG Harpe Ace Extreme",
        keyboard: false,
        report_id: 3,
        offset: 5,
        scale: 1,
    },
    RogModel {
        pid: 0x1B69,
        name: "ASUS ROG Harpe Ace Extreme",
        keyboard: false,
        report_id: 3,
        offset: 5,
        scale: 1,
    },
    RogModel {
        pid: 0x1B67,
        name: "ASUS ROG Harpe Ace Extreme",
        keyboard: false,
        report_id: 0,
        offset: 5,
        scale: 1,
    },
    RogModel {
        pid: 0x1B65,
        name: "ASUS ROG Harpe Ace Mini",
        keyboard: false,
        report_id: 3,
        offset: 5,
        scale: 1,
    },
    RogModel {
        pid: 0x1B63,
        name: "ASUS ROG Harpe Ace Mini",
        keyboard: false,
        report_id: 0,
        offset: 5,
        scale: 1,
    },
    RogModel {
        pid: 0x1AD0,
        name: "ASUS ROG Harpe II Ace",
        keyboard: false,
        report_id: 3,
        offset: 5,
        scale: 1,
    },
    RogModel {
        pid: 0x1C69,
        name: "ASUS ROG Harpe II Ace",
        keyboard: false,
        report_id: 0,
        offset: 5,
        scale: 1,
    },
    RogModel {
        pid: 0x1A68,
        name: "ASUS ROG Keris Aimpoint",
        keyboard: false,
        report_id: 3,
        offset: 5,
        scale: 1,
    },
    RogModel {
        pid: 0x1A6A,
        name: "ASUS ROG Keris Aimpoint",
        keyboard: false,
        report_id: 3,
        offset: 5,
        scale: 1,
    },
    RogModel {
        pid: 0x1A66,
        name: "ASUS ROG Keris Aimpoint",
        keyboard: false,
        report_id: 0,
        offset: 5,
        scale: 1,
    },
    RogModel {
        pid: 0x1B1A,
        name: "ASUS ROG Keris II Ace",
        keyboard: false,
        report_id: 3,
        offset: 5,
        scale: 1,
    },
    RogModel {
        pid: 0x1B18,
        name: "ASUS ROG Keris II Ace",
        keyboard: false,
        report_id: 3,
        offset: 5,
        scale: 1,
    },
    RogModel {
        pid: 0x1B16,
        name: "ASUS ROG Keris II Ace",
        keyboard: false,
        report_id: 0,
        offset: 5,
        scale: 1,
    },
    RogModel {
        pid: 0x1C0E,
        name: "ASUS ROG Keris II Origin",
        keyboard: false,
        report_id: 3,
        offset: 5,
        scale: 1,
    },
    RogModel {
        pid: 0x1C0C,
        name: "ASUS ROG Keris II Origin",
        keyboard: false,
        report_id: 0,
        offset: 5,
        scale: 1,
    },
    RogModel {
        pid: 0x1D4E,
        name: "ASUS ROG Keris II Origin KJP",
        keyboard: false,
        report_id: 3,
        offset: 5,
        scale: 1,
    },
    RogModel {
        pid: 0x1D4C,
        name: "ASUS ROG Keris II Origin KJP",
        keyboard: false,
        report_id: 0,
        offset: 5,
        scale: 1,
    },
    RogModel {
        pid: 0x1979,
        name: "ASUS ROG Spatha X",
        keyboard: false,
        report_id: 0,
        offset: 5,
        scale: 1,
    },
    RogModel {
        pid: 0x1977,
        name: "ASUS ROG Spatha X",
        keyboard: false,
        report_id: 0,
        offset: 5,
        scale: 1,
    },
    RogModel {
        pid: 0x1AD7,
        name: "ASUS ROG Strix Impact III Wireless",
        keyboard: false,
        report_id: 3,
        offset: 5,
        scale: 1,
    },
];

fn rog_model(pid: u16) -> Option<&'static RogModel> {
    ROG_MODELS.iter().find(|m| m.pid == pid)
}

pub(super) fn is_rog_peripheral_pid(pid: u16) -> bool {
    rog_model(pid).is_some()
}

struct AsusRogPeripheralProvider;

impl Provider for AsusRogPeripheralProvider {
    fn id(&self) -> &str {
        "asus.rog_peripheral"
    }

    fn metadata(&self) -> ProviderMetadata {
        experimental_metadata(
            DeviceKind::Peripheral,
            capabilities(true, true, false),
            Duration::from_millis(500),
        )
    }

    fn device_kind(&self, endpoint: &Endpoint) -> DeviceKind {
        match endpoint.pid.and_then(rog_model) {
            Some(model) if model.keyboard => DeviceKind::Keyboard,
            Some(_) => DeviceKind::Mouse,
            None => DeviceKind::Peripheral,
        }
    }

    fn display_name(&self, endpoint: &Endpoint) -> Option<String> {
        endpoint.pid.and_then(rog_model).map(|m| m.name.to_owned())
    }

    fn matches(&self, endpoint: &Endpoint) -> MatchDecision {
        if endpoint.vid == Some(0x0B05)
            && endpoint.pid.is_some_and(is_rog_peripheral_pid)
            && endpoint.usage_page.is_some_and(|p| p >= 0xFF00)
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
        let model =
            device
                .endpoint
                .pid
                .and_then(rog_model)
                .ok_or_else(|| BatteryError::Unsupported {
                    reason: "unknown ROG peripheral PID".to_owned(),
                })?;
        let sub = if model.keyboard { 0x01 } else { 0x07 };
        drain_input(session, device, deadline, 6)?;
        let mut req = vec![0_u8; 64];
        req[..3].copy_from_slice(&[model.report_id, 0x12, sub]);
        send_with_modes(session, device, OUTPUT, &req)?;
        let frame = wait_frame(
            session,
            device,
            deadline,
            Duration::from_millis(500),
            12,
            |f| {
                let r = if model.report_id == 0 && f.first() != Some(&0) {
                    return Ok(false);
                } else {
                    f
                };
                Ok(r.len() >= 12
                    && ((r[1] == 0xFF && r[2] == 0xAA) || (r[1] == 0x12 && r[2] == sub)))
            },
        )?;
        let r = if model.report_id == 0 {
            &frame[..]
        } else {
            unnumbered_payload(&frame)
        };
        if r[1] == 0xFF && r[2] == 0xAA {
            return Ok(ProviderReading {
                reachability: Reachability::Sleeping,
                source: ReadingSource::VendorHid,
                ..ProviderReading::default()
            });
        }
        if model.keyboard {
            let pct = checked_percentage(r[model.offset])?;
            let charge = if r[9] == 1 {
                ChargeState::Charging
            } else {
                ChargeState::Discharging
            };
            Ok(reading(Some(pct), charge, None))
        } else {
            let val = u16::from(r[model.offset]) * u16::from(model.scale);
            if val == 0 || val > 100 {
                return Ok(ProviderReading {
                    reachability: Reachability::Sleeping,
                    source: ReadingSource::VendorHid,
                    ..ProviderReading::default()
                });
            }
            let charge = if r[10] != 0 {
                ChargeState::Charging
            } else {
                ChargeState::Discharging
            };
            Ok(reading(Some(val as u8), charge, None))
        }
    }
}
