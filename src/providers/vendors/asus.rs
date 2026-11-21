//! ASUS ROG headset and ROG Omni receiver battery protocols.

use crate::error::BatteryError;
use crate::model::{
    ChargeState, Endpoint, ProviderReading, Reachability, ReadingSource, SupportStatus,
};
use crate::providers::fixed::{
    capabilities, charge_bool, checked_percentage, drain_input, experimental_metadata, frame,
    query, reading, send_with_modes, unnumbered_payload, wait_frame, OUTPUT, OUTPUT_THEN_SET,
};
use crate::providers::{Deadline, DeviceContext, MatchDecision, Provider, ProviderMetadata};
use crate::transport::{HidSession, TransportError};
use std::sync::Arc;
use std::time::Duration;

pub(crate) fn providers() -> Vec<Arc<dyn Provider>> {
    vec![
        Arc::new(AsusHeadsetProvider),
        Arc::new(AsusOmniProvider),
        Arc::new(AsusRogCetraProvider),
        Arc::new(AsusRogPeripheralProvider),
    ]
}

struct AsusHeadsetProvider;

impl Provider for AsusHeadsetProvider {
    fn id(&self) -> &str {
        "asus.headset"
    }

    fn metadata(&self) -> ProviderMetadata {
        experimental_metadata(capabilities(true, true, false), Duration::from_millis(300))
    }

    fn display_name(&self, _endpoint: &Endpoint) -> Option<String> {
        Some("ASUS ROG headset".to_owned())
    }

    fn matches(&self, endpoint: &Endpoint) -> MatchDecision {
        if matches!(endpoint.pid, Some(0x1ACE | 0x1CAC))
            || endpoint.pid.is_some_and(|p| rog_model(p).is_some())
        {
            return MatchDecision::NoMatch;
        }
        let known = matches!(
            endpoint.pid,
            Some(0x1AFA) | Some(0x1D41) | Some(0x1B82) | Some(0x1B84)
        );
        let named = endpoint.product_name.as_deref().map_or(false, |name| {
            let name = name.to_ascii_uppercase();
            [
                "DELTA", "PELTA", "CETRA", "CLAVIS", "STRIX GO", "HEADSET", "HATSUNE",
            ]
            .iter()
            .any(|part| name.contains(part))
        });
        if endpoint.vid == Some(0x0B05) && endpoint.usage_page == Some(0xFF00) && (known || named) {
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
        let battery = asus_headset_query(session, device, deadline, 0x07)?;
        let percentage = checked_percentage(battery[6])?;
        let charge = match asus_headset_query(session, device, deadline, 0x08) {
            Ok(frame) => charge_bool(frame[5]),
            Err(BatteryError::Timeout { .. }) => ChargeState::Unknown,
            Err(error) => return Err(error),
        };
        Ok(reading(Some(percentage), charge, None))
    }
}

fn asus_headset_query(
    session: &mut dyn HidSession,
    device: &DeviceContext<'_>,
    deadline: Deadline,
    command: u8,
) -> Result<Vec<u8>, BatteryError> {
    let mut request = vec![0_u8; 64];
    request[..3].copy_from_slice(&[0xCC, 0x12, command]);
    query(
        session,
        device,
        deadline,
        request,
        Duration::from_millis(300),
        |frame| {
            if frame.len() >= 3 && frame[1] == 0xFF && frame[2] == 0xAA {
                return Ok(false);
            }
            Ok(frame.len() >= 7 && frame[0] == 0xCC && frame[1] == 0x12 && frame[2] == command)
        },
    )
}

/// Static ROG Omni keyboard/mouse channels. Pair-list discovery is intentionally
/// deferred because it would require I/O during passive enumeration.
struct AsusOmniProvider;

impl Provider for AsusOmniProvider {
    fn id(&self) -> &str {
        "asus.rog_omni"
    }

    fn metadata(&self) -> ProviderMetadata {
        ProviderMetadata {
            version: "1.0.0".to_owned(),
            support_status: SupportStatus::Experimental,
            capabilities: capabilities(true, true, false),
            io_timeout: Duration::from_millis(300),
            cooldown: Duration::from_millis(200),
        }
    }

    fn display_name(&self, endpoint: &Endpoint) -> Option<String> {
        Some(match omni_channel(endpoint) {
            Some(2) => "ASUS ROG Omni keyboard".to_owned(),
            Some(3) => "ASUS ROG Omni mouse".to_owned(),
            _ => "ASUS ROG Omni receiver".to_owned(),
        })
    }

    fn matches(&self, endpoint: &Endpoint) -> MatchDecision {
        if omni_channel(endpoint).is_some() {
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
        let keyboard = omni_channel(device.endpoint) == Some(2);
        let report_id = if keyboard { 2 } else { 3 };
        let command = if keyboard { 0x01 } else { 0x07 };
        let mut request = [0_u8; 64];
        request[..3].copy_from_slice(&[report_id, 0x12, command]);
        session
            .write_output(report_id, &request[1..])
            .map_err(|error| error.into_battery(&device.endpoint.id))?;
        for _ in 0..16 {
            let timeout = deadline
                .remaining("ROG Omni")?
                .min(Duration::from_millis(300));
            let report = match session.read_input(timeout) {
                Ok(report) => frame(report),
                Err(TransportError::Timeout) => return Err(BatteryError::timeout("ROG Omni")),
                Err(error) => return Err(error.into_battery(&device.endpoint.id)),
            };
            let minimum = if keyboard { 14 } else { 11 };
            if report.len() < 3 || report[..3] != [report_id, 0x12, command] {
                continue;
            }
            if report.len() < minimum {
                return Err(BatteryError::invalid_response(
                    "truncated ROG Omni response",
                ));
            }
            let (percent_index, charge_index, voltage_index) =
                if keyboard { (6, 9, 12) } else { (5, 10, 8) };
            let percentage = checked_percentage(report[percent_index])?;
            let charge = match report[charge_index] {
                0 => ChargeState::Discharging,
                1 => ChargeState::Charging,
                _ => ChargeState::Unknown,
            };
            if percentage == 0
                && charge == ChargeState::Discharging
                && report[voltage_index] == 0
                && report[voltage_index + 1] == 0
            {
                return Err(BatteryError::invalid_response(
                    "ROG Omni returned an empty status",
                ));
            }
            return Ok(reading(Some(percentage), charge, None));
        }
        Err(BatteryError::timeout("ROG Omni"))
    }
}

fn omni_channel(endpoint: &Endpoint) -> Option<u8> {
    if endpoint.vid != Some(0x0B05) || endpoint.pid != Some(0x1ACE) {
        return None;
    }
    let path = endpoint.system_path.to_ascii_lowercase();
    if endpoint.usage_page == Some(0xFF00)
        && (endpoint.interface_number == Some(2) || path.contains("&mi_02&col02"))
    {
        Some(2)
    } else if endpoint.usage_page == Some(0xFF01)
        && (endpoint.interface_number == Some(2) || path.contains("&mi_02&col03"))
    {
        Some(3)
    } else {
        None
    }
}

struct AsusRogCetraProvider;

impl Provider for AsusRogCetraProvider {
    fn id(&self) -> &str {
        "asus.rog_cetra"
    }

    fn metadata(&self) -> ProviderMetadata {
        experimental_metadata(capabilities(true, false, false), Duration::from_millis(600))
    }

    fn display_name(&self, _endpoint: &Endpoint) -> Option<String> {
        Some("ASUS ROG Cetra Open Wireless".to_owned())
    }

    fn matches(&self, endpoint: &Endpoint) -> MatchDecision {
        if endpoint.vid == Some(0x0B05)
            && endpoint.pid == Some(0x1CAC)
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
        drain_input(session, device, deadline, 6)?;
        let mut req = vec![0_u8; 64];
        req[..3].copy_from_slice(&[0xCC, 0x12, 0x12]);
        send_with_modes(session, device, OUTPUT_THEN_SET, &req)?;
        let r = wait_frame(
            session,
            device,
            deadline,
            Duration::from_millis(600),
            12,
            |r| {
                Ok(r.len() >= 8
                    && r[0] == 0xCC
                    && ((r[1] == 0xFF && r[2] == 0xAA) || (r[1] == 0x12 && r[2] == 0x12)))
            },
        )?;
        if (r[1] == 0xFF && r[2] == 0xAA) || (r[5] == 0xFF && r[6] == 0xAA) {
            return Ok(ProviderReading {
                reachability: Reachability::Sleeping,
                source: ReadingSource::VendorHid,
                ..ProviderReading::default()
            });
        }
        let left = (1..=100).contains(&r[5]).then_some(r[5]);
        let right = (1..=100).contains(&r[6]).then_some(r[6]);
        let pct = match (left, right) {
            (Some(l), Some(rt)) => Some(l.min(rt)),
            (Some(l), None) => Some(l),
            (None, Some(rt)) => Some(rt),
            (None, None) => None,
        };
        if let Some(pct) = pct {
            Ok(reading(Some(pct), ChargeState::Unknown, None))
        } else {
            Ok(ProviderReading {
                reachability: Reachability::Sleeping,
                source: ReadingSource::VendorHid,
                ..ProviderReading::default()
            })
        }
    }
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

struct AsusRogPeripheralProvider;

impl Provider for AsusRogPeripheralProvider {
    fn id(&self) -> &str {
        "asus.rog_peripheral"
    }

    fn metadata(&self) -> ProviderMetadata {
        experimental_metadata(capabilities(true, true, false), Duration::from_millis(500))
    }

    fn display_name(&self, endpoint: &Endpoint) -> Option<String> {
        endpoint.pid.and_then(rog_model).map(|m| m.name.to_owned())
    }

    fn matches(&self, endpoint: &Endpoint) -> MatchDecision {
        if endpoint.vid == Some(0x0B05)
            && endpoint.pid.is_some_and(|p| rog_model(p).is_some())
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
