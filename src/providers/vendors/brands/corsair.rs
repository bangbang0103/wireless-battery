//! Corsair wireless headset, mouse, and keyboard battery protocols (VID 0x1B1C):
//! Void / HS70 / HS80 legacy headsets and Bragi (Slipstream, Virtuoso, Void V2, HS80 MAX).

use crate::error::BatteryError;
use crate::model::{
    ChargeState, DeviceKind, Endpoint, ProviderReading, Reachability, ReadingSource,
};
use crate::providers::fixed::{
    capabilities, checked_percentage, drain_input, experimental_metadata, reading, send_with_modes,
    unnumbered_payload, wait_frame, OUTPUT,
};
use crate::providers::{
    infer_device_kind_from_name, Deadline, DeviceContext, MatchDecision, Provider, ProviderMetadata,
};
use crate::transport::HidSession;
use std::sync::Arc;
use std::time::Duration;

const VID_CORSAIR: u16 = 0x1B1C;

const VOID_PIDS: &[u16] = &[
    0x1B1C, 0x1B27, 0x0A14, 0x0A16, 0x0A17, 0x0A1D, 0x0A1A, 0x1B2A, 0x1B23, 0x1B29, 0x0A55, 0x0A51,
    0x0A52, 0x0A38, 0x0A4F, 0x0A2B, 0x0A75, 0x0A56,
];

const BRAGI_KNOWN_PIDS: &[u16] = &[
    0x2A08, 0x2A02, 0x0A97, 0x0A64, 0x0A62, 0x0A3E, 0x0A3D, 0x2B00,
];

const BRAGI_HEADSET_PIDS: &[u16] = &[0x0A97, 0x0A64, 0x0A62, 0x0A3E, 0x0A3D];

const CORSAIR_COMPX_PIDS: &[u16] = &[0x2B2A, 0x2B34];

pub(crate) fn providers() -> Vec<Arc<dyn Provider>> {
    vec![
        Arc::new(CorsairVoidProvider),
        Arc::new(CorsairBragiProvider),
    ]
}

struct CorsairVoidProvider;

impl Provider for CorsairVoidProvider {
    fn id(&self) -> &str {
        "corsair.void"
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
                .unwrap_or_else(|| "Corsair wireless headset".to_owned()),
        )
    }

    fn matches(&self, endpoint: &Endpoint) -> MatchDecision {
        if endpoint.vid == Some(VID_CORSAIR)
            && endpoint.pid.is_some_and(|pid| VOID_PIDS.contains(&pid))
            && matches!(endpoint.usage_page, Some(0xFFC5) | Some(0xFF00))
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
        let mut req = vec![0_u8; 64];
        req[..2].copy_from_slice(&[0xC9, 0x64]);
        send_with_modes(session, device, OUTPUT, &req)?;
        let frame = wait_frame(
            session,
            device,
            deadline,
            Duration::from_millis(500),
            12,
            |f| Ok(f.len() >= 5 && f[0] == 0xCA),
        )?;
        let st = frame[4];
        if st == 0 {
            return Ok(ProviderReading {
                reachability: Reachability::Offline,
                source: ReadingSource::VendorHid,
                ..ProviderReading::default()
            });
        }
        let charge = match st {
            4 | 5 => ChargeState::Charging,
            1 | 2 => ChargeState::Discharging,
            _ => ChargeState::Unknown,
        };
        let pct = if matches!(st, 1 | 2 | 4 | 5) {
            Some(checked_percentage(frame[2] & 0x7F)?)
        } else {
            None
        };
        Ok(reading(pct, charge, None))
    }
}

struct CorsairBragiProvider;

impl Provider for CorsairBragiProvider {
    fn id(&self) -> &str {
        "corsair.bragi"
    }

    fn metadata(&self) -> ProviderMetadata {
        experimental_metadata(
            DeviceKind::Peripheral,
            capabilities(true, true, false),
            Duration::from_millis(300),
        )
    }

    fn device_kind(&self, endpoint: &Endpoint) -> DeviceKind {
        if endpoint
            .pid
            .is_some_and(|p| BRAGI_HEADSET_PIDS.contains(&p))
        {
            return DeviceKind::Headset;
        }
        infer_device_kind_from_name(endpoint.product_name.as_deref())
            .unwrap_or(DeviceKind::Peripheral)
    }

    fn display_name(&self, endpoint: &Endpoint) -> Option<String> {
        Some(
            endpoint
                .product_name
                .clone()
                .filter(|name| !name.is_empty())
                .unwrap_or_else(|| "Corsair Bragi device".to_owned()),
        )
    }

    fn matches(&self, endpoint: &Endpoint) -> MatchDecision {
        if endpoint.vid != Some(VID_CORSAIR) {
            return MatchDecision::NoMatch;
        }
        let Some(pid) = endpoint.pid else {
            return MatchDecision::NoMatch;
        };
        if VOID_PIDS.contains(&pid) || CORSAIR_COMPX_PIDS.contains(&pid) {
            return MatchDecision::NoMatch;
        }
        let known = BRAGI_KNOWN_PIDS.contains(&pid);
        if known && endpoint.usage_page.is_some_and(|p| p >= 0xFF00) {
            MatchDecision::Exact
        } else if !known && endpoint.usage_page == Some(0xFF42) {
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
        for &target in &[0x09_u8, 0x0A, 0x0B, 0x08] {
            deadline.check("Corsair Bragi")?;
            let Some(level_raw) = bragi_get_prop(session, device, deadline, target, 0x0F)? else {
                continue;
            };
            if level_raw == 0 || level_raw > 1000 {
                continue;
            }
            let pct = checked_percentage(((level_raw + 5) / 10).min(100) as u8)?;
            let charge = match bragi_get_prop(session, device, deadline, target, 0x10)? {
                Some(1) => ChargeState::Charging,
                Some(3) => ChargeState::Full,
                Some(2) => ChargeState::Discharging,
                _ => ChargeState::Unknown,
            };
            return Ok(reading(Some(pct), charge, None));
        }
        Err(BatteryError::timeout("Corsair Bragi"))
    }
}

fn bragi_get_prop(
    session: &mut dyn HidSession,
    device: &DeviceContext<'_>,
    deadline: Deadline,
    target: u8,
    prop: u8,
) -> Result<Option<u32>, BatteryError> {
    drain_input(session, device, deadline, 4)?;
    let mut req = vec![0_u8; 64];
    req[0] = 0x02;
    req[1] = target;
    req[2] = 0x02;
    req[3] = prop;
    send_with_modes(session, device, OUTPUT, &req)?;
    let expected_source = if target == 0x08 { 0x00 } else { 0x01 };
    let waited = wait_frame(
        session,
        device,
        deadline,
        Duration::from_millis(300),
        8,
        |f| {
            let r = unnumbered_payload(f);
            Ok((r.len() >= 8 && r[1] == expected_source && r[2] == 0x02)
                || (r.len() >= 7 && r[0] == expected_source && r[1] == 0x02))
        },
    );
    let frame = match waited {
        Ok(f) => f,
        Err(BatteryError::Timeout { .. }) => return Ok(None),
        Err(error) => return Err(error),
    };
    let r = unnumbered_payload(&frame);
    let offset = if r.len() >= 8 && r[1] == expected_source && r[2] == 0x02 {
        1
    } else {
        0
    };
    if r[offset + 2] != 0x00 {
        return Ok(None);
    }
    let val = u32::from_le_bytes([r[offset + 3], r[offset + 4], r[offset + 5], r[offset + 6]]);
    Ok(Some(val))
}
