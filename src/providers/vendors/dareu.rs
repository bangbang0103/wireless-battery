//! DAREU wireless mouse battery protocol.

use crate::error::BatteryError;
use crate::model::{ChargeState, Endpoint, ProviderReading};
use crate::providers::fixed::{
    capabilities, charge_bool, checked_percentage, checksum_sum, drain_input,
    experimental_metadata, feature_frame, query, reading, send_with_modes, sleep, wait_frame,
    SendMode, OUTPUT,
};
use crate::providers::{Deadline, DeviceContext, MatchDecision, Provider, ProviderMetadata};
use crate::transport::HidSession;
use std::sync::Arc;
use std::time::Duration;

pub(crate) fn providers() -> Vec<Arc<dyn Provider>> {
    vec![
        Arc::new(DareuProvider),
        Arc::new(DareuTogranProvider),
        Arc::new(DareuA84Provider),
        Arc::new(ZenithProProvider),
    ]
}

struct DareuProvider;

impl Provider for DareuProvider {
    fn id(&self) -> &str {
        "dareu.a950_air_v2"
    }

    fn metadata(&self) -> ProviderMetadata {
        experimental_metadata(capabilities(true, true, true), Duration::from_millis(300))
    }

    fn display_name(&self, _endpoint: &Endpoint) -> Option<String> {
        Some("DAREU A950 Air V2".to_owned())
    }

    fn matches(&self, endpoint: &Endpoint) -> MatchDecision {
        if endpoint.vid == Some(0x260D)
            && endpoint.pid == Some(0x1175)
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
        let active = dareu_request(0x03);
        query(
            session,
            device,
            deadline,
            active,
            Duration::from_millis(300),
            |frame| Ok(dareu_valid(frame, 0x03)? && frame[6] == 1),
        )?;
        let frame = query(
            session,
            device,
            deadline,
            dareu_request(0x04),
            Duration::from_millis(300),
            |frame| Ok(dareu_valid(frame, 0x04)? && frame[5] == 2),
        )?;
        let charge = match frame[7] {
            0 => ChargeState::Discharging,
            1 => ChargeState::Charging,
            2 => ChargeState::Full,
            _ => return Err(BatteryError::invalid_response("invalid DAREU charge state")),
        };
        Ok(reading(
            Some(checked_percentage(frame[6])?),
            charge,
            Some(u32::from(u16::from_be_bytes([frame[8], frame[9]]))),
        ))
    }
}

fn dareu_request(command: u8) -> Vec<u8> {
    let mut frame = vec![0_u8; 17];
    frame[0] = 0x08;
    frame[1] = command;
    let sum = frame[..16].iter().map(|byte| u16::from(*byte)).sum::<u16>();
    frame[16] = (if sum < 55 { 85 - sum } else { 341 - sum } & 0xFF) as u8;
    frame
}

fn dareu_valid(frame: &[u8], command: u8) -> Result<bool, BatteryError> {
    if frame.len() < 2 || frame[0] != 0x08 || frame[1] != command {
        return Ok(false);
    }
    if frame.len() != 17 {
        return Err(BatteryError::invalid_response("invalid DAREU frame length"));
    }
    let expected = dareu_request(command);
    if frame == expected {
        return Ok(false);
    }
    Ok(true)
}

struct DareuTogranProvider;

impl Provider for DareuTogranProvider {
    fn id(&self) -> &str {
        "dareu.togran"
    }

    fn metadata(&self) -> ProviderMetadata {
        experimental_metadata(capabilities(true, true, false), Duration::from_millis(450))
    }

    fn display_name(&self, _endpoint: &Endpoint) -> Option<String> {
        Some("DAREU A980 Pro".to_owned())
    }

    fn matches(&self, endpoint: &Endpoint) -> MatchDecision {
        if endpoint.vid == Some(0x260D)
            && matches!(endpoint.pid, Some(0x1131 | 0x1148))
            && endpoint.usage_page == Some(0x000C)
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
        let slot = if device.endpoint.pid == Some(0x1131) {
            let slots = togran_query(session, device, deadline, 0, 6, 0x00, 0xA0)?;
            let count = usize::from(slots[7]).min(5);
            let mut active = None;
            for idx in 1..=count {
                let off = 8 + (idx - 1) * 3;
                if off < slots.len() && slots[off] != 0 {
                    active = Some(idx as u8);
                    break;
                }
            }
            active.ok_or_else(|| BatteryError::DeviceDisconnected {
                endpoint: device.endpoint.id.to_string(),
            })?
        } else {
            0
        };
        let reply = togran_query(session, device, deadline, slot, 4, 0x07, 0x80)?;
        let max_level = if reply[9] == 0 { 100 } else { reply[9] };
        if reply[8] > max_level {
            return Err(BatteryError::invalid_response(
                "DAREU Togran battery exceeds max level",
            ));
        }
        let percentage = ((u16::from(reply[8]) * 100) / u16::from(max_level)).min(100) as u8;
        let charge = match reply[7] {
            0 => ChargeState::Discharging,
            1 => ChargeState::Charging,
            _ => ChargeState::Unknown,
        };
        Ok(reading(Some(percentage), charge, None))
    }
}

fn togran_query(
    session: &mut dyn HidSession,
    device: &DeviceContext<'_>,
    deadline: Deadline,
    slot: u8,
    len: u8,
    cls: u8,
    cmd: u8,
) -> Result<Vec<u8>, BatteryError> {
    let mut request = vec![0_u8; 65];
    request[0] = 0x0A;
    request[1] = slot << 4;
    request[2] = len;
    request[3] = cls;
    request[4] = cmd;
    send_with_modes(session, device, &[SendMode::Feature], &request)?;
    for _ in 0..15 {
        sleep(deadline, Duration::from_millis(30))?;
        let frame = feature_frame(session, device, 0x0A, 64)?;
        if frame.len() >= 11 && (frame[1] & 0x0F) == 0x02 && frame[3] == cls && frame[4] == cmd {
            return Ok(frame);
        }
    }
    Err(BatteryError::timeout("DAREU Togran"))
}

struct DareuA84Provider;

impl Provider for DareuA84Provider {
    fn id(&self) -> &str {
        "dareu.a84"
    }

    fn metadata(&self) -> ProviderMetadata {
        experimental_metadata(capabilities(true, false, false), Duration::from_millis(200))
    }

    fn display_name(&self, _endpoint: &Endpoint) -> Option<String> {
        Some("DAREU A84".to_owned())
    }

    fn matches(&self, endpoint: &Endpoint) -> MatchDecision {
        if endpoint.vid == Some(0x260D)
            && endpoint.pid == Some(0x0022)
            && endpoint.usage_page == Some(0x0001)
            && endpoint.usage == Some(0x0006)
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
        let mut request = vec![0_u8; 65];
        request[1] = 0x04;
        request[2] = 0xF6;
        send_with_modes(session, device, &[SendMode::Feature], &request)?;
        sleep(deadline, Duration::from_millis(10))?;
        for _ in 0..6 {
            let frame = feature_frame(session, device, 0x00, 64)?;
            if frame.len() >= 16 && frame[15] != 0 && frame[5] <= 100 {
                return Ok(reading(Some(frame[5]), ChargeState::Unknown, None));
            }
            sleep(deadline, Duration::from_millis(10))?;
        }
        Err(BatteryError::timeout("DAREU A84"))
    }
}

struct ZenithProProvider;

impl Provider for ZenithProProvider {
    fn id(&self) -> &str {
        "zenith.pro"
    }

    fn metadata(&self) -> ProviderMetadata {
        experimental_metadata(capabilities(true, true, true), Duration::from_millis(500))
    }

    fn display_name(&self, _endpoint: &Endpoint) -> Option<String> {
        Some("Zenith Pro".to_owned())
    }

    fn matches(&self, endpoint: &Endpoint) -> MatchDecision {
        if endpoint.vid == Some(0x260D)
            && matches!(endpoint.pid, Some(0x1114 | 0x1143))
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
        drain_input(session, device, deadline, 8)?;
        let mut request = vec![0_u8; 17];
        request[0] = 0x08;
        request[1] = 0x04;
        request[16] = 0x49;
        send_with_modes(session, device, OUTPUT, &request)?;
        let frame = wait_frame(
            session,
            device,
            deadline,
            Duration::from_millis(500),
            32,
            |frame| Ok(frame.len() == 17 && frame[0] == 0x08 && frame[1] == 0x04),
        )?;
        if checksum_sum(&frame) != 0x55 {
            return Err(BatteryError::ChecksumFailed);
        }
        let percentage = checked_percentage(frame[6])?;
        let voltage = u32::from(u16::from_be_bytes([frame[8], frame[9]]));
        Ok(reading(
            Some(percentage),
            charge_bool(frame[7]),
            (voltage > 0).then_some(voltage),
        ))
    }
}
