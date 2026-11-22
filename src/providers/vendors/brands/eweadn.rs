//! EWEADN feature-report battery protocols (S9 Pro, A5 Pro).

use crate::error::BatteryError;
use crate::model::{ChargeState, DeviceKind, Endpoint, EndpointId, ProviderReading};
use crate::providers::fixed::{
    capabilities, checked_percentage, experimental_metadata, feature_frame, reading,
    send_with_modes, sleep, SendMode,
};
use crate::providers::{Deadline, DeviceContext, MatchDecision, Provider, ProviderMetadata};
use crate::transport::HidSession;
use std::collections::HashMap;
use std::sync::{Arc, Mutex};
use std::time::Duration;

const SEQUENCE_CACHE_LIMIT: usize = 64;

pub(crate) fn providers() -> Vec<Arc<dyn Provider>> {
    vec![
        Arc::new(EweadnS9ProProvider),
        Arc::new(EweadnA5ProProvider::new()),
    ]
}

struct EweadnS9ProProvider;

impl Provider for EweadnS9ProProvider {
    fn id(&self) -> &str {
        "eweadn.s9_pro"
    }

    fn metadata(&self) -> ProviderMetadata {
        experimental_metadata(
            DeviceKind::Mouse,
            capabilities(true, true, false),
            Duration::from_millis(150),
        )
    }

    fn display_name(&self, _endpoint: &Endpoint) -> Option<String> {
        Some("EWEADN S9 Pro".to_owned())
    }

    fn matches(&self, endpoint: &Endpoint) -> MatchDecision {
        if endpoint.vid == Some(0x372E)
            && matches!(endpoint.pid, Some(0x103D | 0x1027))
            && endpoint.usage_page == Some(0xFF00)
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
        let mut request = vec![0_u8; 64];
        request[..7].copy_from_slice(&[0x03, 0x20, 0x50, 0, 0, 0x4F, 0x81]);
        for _ in 0..4 {
            send_with_modes(session, device, &[SendMode::Feature], &request)?;
            sleep(deadline, Duration::from_millis(60))?;
            let frame = feature_frame(session, device, 0x03, 63)?;
            if let Some(reading) = parse_eweadn_s9(&frame)? {
                return Ok(reading);
            }
        }
        Err(BatteryError::timeout("EWEADN S9 Pro"))
    }
}

fn parse_eweadn_s9(frame: &[u8]) -> Result<Option<ProviderReading>, BatteryError> {
    if frame.len() < 2 || frame[0] != 0x03 {
        return Ok(None);
    }
    if frame.len() != 64 || frame[1] != 0x50 || frame[3] != 0x0B || frame[13..15] != [0xA5, 0x5A] {
        return Ok(None);
    }
    let charging = frame[6] & 0x80 != 0;
    let percentage = checked_percentage(frame[6] & 0x7F)?;
    Ok(Some(reading(
        if charging { None } else { Some(percentage) },
        if charging {
            ChargeState::Charging
        } else {
            ChargeState::Discharging
        },
        None,
    )))
}

struct EweadnA5ProProvider {
    sequences: Mutex<HashMap<EndpointId, u8>>,
}

impl EweadnA5ProProvider {
    fn new() -> Self {
        Self {
            sequences: Mutex::new(HashMap::new()),
        }
    }

    fn next_sequence(&self, endpoint: &EndpointId) -> Result<u8, BatteryError> {
        let mut map = self.sequences.lock().map_err(|_| BatteryError::Internal {
            message: "EWEADN sequence lock poisoned".to_owned(),
        })?;
        if map.len() >= SEQUENCE_CACHE_LIMIT && !map.contains_key(endpoint) {
            map.clear();
        }
        let entry = map.entry(endpoint.clone()).or_insert(0);
        let current = *entry;
        *entry = entry.wrapping_add(1);
        Ok(current)
    }
}

impl Provider for EweadnA5ProProvider {
    fn id(&self) -> &str {
        "eweadn.a5_pro"
    }

    fn metadata(&self) -> ProviderMetadata {
        experimental_metadata(
            DeviceKind::Mouse,
            capabilities(true, false, false),
            Duration::from_millis(300),
        )
    }

    fn display_name(&self, _endpoint: &Endpoint) -> Option<String> {
        Some("EWEADN A5 Pro".to_owned())
    }

    fn matches(&self, endpoint: &Endpoint) -> MatchDecision {
        if endpoint.vid == Some(0x258A)
            && endpoint.pid == Some(0x0138)
            && endpoint.usage_page == Some(0xFF02)
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
        let sequence = self.next_sequence(&device.endpoint.id)?;
        let mut request = vec![0_u8; 520];
        request[0] = 9;
        request[1] = 12_u8.wrapping_add(sequence);
        request[2..7].copy_from_slice(&[9, 1, 1, sequence, 1]);
        send_with_modes(session, device, &[SendMode::Feature], &request)?;
        sleep(deadline, Duration::from_millis(100))?;
        let frame = feature_frame(session, device, 9, 519)?;
        if frame.len() != 520 || frame[2..8] != [9, 1, 1, sequence, 1, 0] {
            return Err(BatteryError::invalid_response(
                "invalid EWEADN A5 Pro response",
            ));
        }
        Ok(reading(
            Some(checked_percentage(frame[8])?),
            ChargeState::Unknown,
            None,
        ))
    }
}
