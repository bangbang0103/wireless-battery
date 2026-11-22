//! Rapoo wireless mouse HID battery protocols (VT3S, Nordic VT series, Legacy VT Pro).

use crate::error::BatteryError;
use crate::model::{ChargeState, DeviceKind, Endpoint, ProviderReading};
use crate::providers::fixed::{
    capabilities, checked_percentage, experimental_metadata, reading, wait_frame,
};
use crate::providers::{Deadline, DeviceContext, MatchDecision, Provider, ProviderMetadata};
use crate::transport::HidSession;
use std::sync::Arc;
use std::time::Duration;

pub(crate) fn providers() -> Vec<Arc<dyn Provider>> {
    vec![
        Arc::new(RapooProvider),
        Arc::new(RapooVtNordicProvider),
        Arc::new(RapooLegacyProvider),
    ]
}

struct RapooProvider;

impl Provider for RapooProvider {
    fn id(&self) -> &str {
        "rapoo.vt3s"
    }

    fn metadata(&self) -> ProviderMetadata {
        experimental_metadata(
            DeviceKind::Mouse,
            capabilities(true, true, false),
            Duration::from_millis(4_500),
        )
    }

    fn display_name(&self, _endpoint: &Endpoint) -> Option<String> {
        Some("Rapoo VT3S".to_owned())
    }

    fn matches(&self, endpoint: &Endpoint) -> MatchDecision {
        if endpoint.vid == Some(0x24AE)
            && endpoint.pid == Some(0x1411)
            && endpoint.usage_page == Some(0xFF00)
            && endpoint.usage == Some(2)
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
            Duration::from_millis(4_500),
            32,
            |frame| {
                if frame.len() < 2 || frame[0] != 0x07 {
                    return Ok(false);
                }
                if frame.len() <= 8 {
                    return Err(BatteryError::invalid_response("truncated Rapoo report"));
                }
                checked_percentage(frame[8])?;
                Ok(true)
            },
        )?;
        let charge = match frame[7] {
            1 => ChargeState::Discharging,
            2 | 3 => ChargeState::Charging,
            _ => ChargeState::Unknown,
        };
        Ok(reading(Some(frame[8]), charge, None))
    }
}

const RAPOO_LEGACY_PIDS: &[u16] = &[0x1215, 0x4415, 0x1233, 0x4433, 0x1244];

const RAPOO_NORDIC_PIDS: &[u16] = &[
    0x4610, 0x1410, 0x4620, 0x1420, 0x4615, 0x1415, 0x4644, 0x1444, 0x4622, 0x1422, 0x4623, 0x1423,
    0x4612, 0x1412, 0x4617, 0x1417, 0x4B06, 0x1B06, 0x4611, 0x4621, 0x1421, 0x4613, 0x1413, 0x4616,
    0x1416, 0x4614, 0x1414, 0x4618, 0x1418,
];

struct RapooVtNordicProvider;

impl Provider for RapooVtNordicProvider {
    fn id(&self) -> &str {
        "rapoo.vt"
    }

    fn metadata(&self) -> ProviderMetadata {
        experimental_metadata(
            DeviceKind::Mouse,
            capabilities(true, true, false),
            Duration::from_millis(4_500),
        )
    }

    fn display_name(&self, endpoint: &Endpoint) -> Option<String> {
        let name = match endpoint.pid {
            Some(0x4610 | 0x1410) => "Rapoo VT0",
            Some(0x4620 | 0x1420) => "Rapoo VT0 Max",
            Some(0x4615 | 0x1415) => "Rapoo VT1",
            Some(0x4644 | 0x1444) => "Rapoo VT1 Max",
            Some(0x4622 | 0x1422) => "Rapoo VT2",
            Some(0x4623 | 0x1423) => "Rapoo VT2 Max",
            Some(0x4612 | 0x1412) => "Rapoo VT3",
            Some(0x4617 | 0x1417) => "Rapoo VT3 Max Gen 2",
            Some(0x4B06 | 0x1B06) => "Rapoo VT3 Max V2",
            Some(0x4611) => "Rapoo VT3S",
            Some(0x4621 | 0x1421) => "Rapoo VT3S Max",
            Some(0x4613 | 0x1413) => "Rapoo VT7",
            Some(0x4616 | 0x1416) => "Rapoo VT7 Max",
            Some(0x4614 | 0x1414) => "Rapoo VT9 Gen 2",
            Some(0x4618 | 0x1418) => "Rapoo VT9 Max",
            _ => "Rapoo VT mouse",
        };
        Some(name.to_owned())
    }

    fn matches(&self, endpoint: &Endpoint) -> MatchDecision {
        if endpoint.vid == Some(0x24AE)
            && endpoint.pid.is_some_and(|p| RAPOO_NORDIC_PIDS.contains(&p))
            && endpoint.usage_page == Some(0xFF00)
            && endpoint.usage == Some(2)
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
            Duration::from_millis(4_500),
            32,
            |frame| {
                Ok(frame.len() >= 9
                    && frame[0] == 0x07
                    && (frame[1] & 0x0F) <= 2
                    && matches!(frame[7], 1 | 2))
            },
        )?;
        let pct = checked_percentage(frame[8])?;
        let charge = match frame[7] {
            1 => ChargeState::Discharging,
            2 => ChargeState::Charging,
            _ => ChargeState::Unknown,
        };
        Ok(reading(Some(pct), charge, None))
    }
}

struct RapooLegacyProvider;

impl Provider for RapooLegacyProvider {
    fn id(&self) -> &str {
        "rapoo.legacy_vt3_pro"
    }

    fn metadata(&self) -> ProviderMetadata {
        experimental_metadata(
            DeviceKind::Mouse,
            capabilities(true, true, false),
            Duration::from_millis(4_500),
        )
    }

    fn display_name(&self, endpoint: &Endpoint) -> Option<String> {
        Some(match endpoint.pid {
            Some(0x1233 | 0x4433) => "Rapoo VT1 Pro Max".to_owned(),
            Some(0x1244) => "Rapoo VT3 Pro Max".to_owned(),
            _ => "Rapoo VT3 Pro".to_owned(),
        })
    }

    fn matches(&self, endpoint: &Endpoint) -> MatchDecision {
        if endpoint.vid == Some(0x24AE)
            && endpoint.pid.is_some_and(|p| RAPOO_LEGACY_PIDS.contains(&p))
            && endpoint.usage_page == Some(0xFF00)
            && endpoint.usage == Some(2)
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
            Duration::from_millis(4_500),
            32,
            |frame| {
                Ok(frame.len() >= 7
                    && frame[0] == 0xBB
                    && frame[1] == 0xB0
                    && matches!(frame[5], 1 | 2))
            },
        )?;
        if frame[5] == 2 && frame[6] == 0x7F {
            return Ok(reading(None, ChargeState::Charging, None));
        }
        let pct = checked_percentage(frame[6])?;
        let charge = match frame[5] {
            1 => ChargeState::Discharging,
            2 => ChargeState::Charging,
            _ => ChargeState::Unknown,
        };
        Ok(reading(Some(pct), charge, None))
    }
}
