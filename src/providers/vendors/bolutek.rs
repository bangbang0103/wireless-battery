//! Shared Bolutek headset battery protocol (ATK N9V2, MCHOSE V9).

use crate::error::BatteryError;
use crate::model::{ChargeState, Endpoint, ProviderReading, SupportStatus};
use crate::providers::fixed::{
    capabilities, checked_percentage, reading, FixedProvider, FixedSpec, OUTPUT,
};
use crate::providers::Provider;
use std::sync::Arc;
use std::time::Duration;

static ATK_N9: FixedSpec = FixedSpec {
    id: "atk.n9v2",
    name: "ATK N9V2 ULTIMATE",
    status: SupportStatus::Experimental,
    matches: match_atk_n9,
    query: bolutek_query,
    modes: OUTPUT,
    attempts: 3,
    max_reports: 12,
    io_timeout: Duration::from_millis(600),
    capabilities: capabilities(true, true, false),
    parse: parse_bolutek,
};

static MCHOSE_V9: FixedSpec = FixedSpec {
    id: "mchose.v9",
    name: "MCHOSE V9 Turbo+",
    status: SupportStatus::Experimental,
    matches: match_mchose_v9,
    query: bolutek_query,
    modes: OUTPUT,
    attempts: 3,
    max_reports: 12,
    io_timeout: Duration::from_millis(650),
    capabilities: capabilities(true, true, false),
    parse: parse_bolutek,
};

pub(crate) fn providers() -> Vec<Arc<dyn Provider>> {
    vec![
        Arc::new(FixedProvider::new(&ATK_N9)),
        Arc::new(FixedProvider::new(&MCHOSE_V9)),
    ]
}

fn match_atk_n9(endpoint: &Endpoint) -> bool {
    endpoint.vid == Some(0x373B)
        && endpoint.pid == Some(0x1262)
        && endpoint.usage_page == Some(0xFF90)
}

fn match_mchose_v9(endpoint: &Endpoint) -> bool {
    endpoint.vid == Some(0x3837)
        && endpoint.pid == Some(0x600A)
        && endpoint.usage_page == Some(0xFF90)
}

fn bolutek_query() -> Vec<u8> {
    let mut frame = vec![0_u8; 64];
    frame[0] = 0x55;
    frame[1] = 0x65;
    frame[2] = 1;
    frame
}

fn parse_bolutek(frame: &[u8]) -> Result<Option<ProviderReading>, BatteryError> {
    if frame.len() < 2 || frame[0] != 0x55 || frame[1] != 0x65 {
        return Ok(None);
    }
    if frame.len() < 4 {
        return Err(BatteryError::invalid_response("truncated Bolutek response"));
    }
    let percentage = checked_percentage(frame[2])?;
    let (percentage, charge) = match frame[3] {
        0x02 => (percentage, ChargeState::Discharging),
        0x03 => (percentage, ChargeState::Charging),
        0x04 => (100, ChargeState::Full),
        _ => {
            return Err(BatteryError::invalid_response(
                "unknown Bolutek charge state",
            ))
        }
    };
    Ok(Some(reading(Some(percentage), charge, None)))
}
