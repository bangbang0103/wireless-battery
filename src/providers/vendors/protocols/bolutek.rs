//! Shared Bolutek headset battery protocol (ATK N9V2, MCHOSE V9, VGN OMNISOUND).

use crate::error::BatteryError;
use crate::model::{ChargeState, DeviceKind, Endpoint, ProviderReading, SupportStatus};
use crate::providers::fixed::{
    capabilities, checked_percentage, drain_input, experimental_metadata, reading, send_with_modes,
    unnumbered_payload, wait_frame, FixedProvider, FixedSpec, OUTPUT, SET_OUTPUT,
};
use crate::providers::{Deadline, DeviceContext, MatchDecision, Provider, ProviderMetadata};
use crate::transport::HidSession;
use std::sync::Arc;
use std::time::Duration;

static ATK_N9: FixedSpec = FixedSpec {
    id: "atk.n9v2",
    name: "ATK N9V2 ULTIMATE",
    kind: DeviceKind::Headset,
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
    kind: DeviceKind::Headset,
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
        Arc::new(VgnOmnisoundProvider),
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

struct VgnOmnisoundProvider;

impl Provider for VgnOmnisoundProvider {
    fn id(&self) -> &str {
        "vgn.omnisound"
    }

    fn metadata(&self) -> ProviderMetadata {
        experimental_metadata(
            DeviceKind::Headset,
            capabilities(true, false, false),
            Duration::from_millis(500),
        )
    }

    fn display_name(&self, endpoint: &Endpoint) -> Option<String> {
        Some(
            endpoint
                .product_name
                .clone()
                .filter(|name| !name.is_empty())
                .unwrap_or_else(|| "VGN OMNISOUND".to_owned()),
        )
    }

    fn matches(&self, endpoint: &Endpoint) -> MatchDecision {
        if endpoint.vid == Some(0x372E)
            && endpoint.pid == Some(0x10BB)
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
        drain_input(session, device, deadline, 8)?;
        let mut request = vec![0_u8; 64];
        request[..3].copy_from_slice(&[0x55, 0x65, 0x01]);
        send_with_modes(session, device, SET_OUTPUT, &request)?;
        let frame =
            wait_frame(
                session,
                device,
                deadline,
                Duration::from_millis(500),
                12,
                |frame| {
                    let r = unnumbered_payload(frame);
                    Ok((r.len() >= 3 && r[0] == 0x55 && r[1] == 0x65)
                        || (r.len() >= 2 && r[0] == 0x65))
                },
            )?;
        let r = unnumbered_payload(&frame);
        let raw = if r[0] == 0x55 { r[2] } else { r[1] };
        Ok(reading(
            Some(checked_percentage(raw)?),
            ChargeState::Unknown,
            None,
        ))
    }
}
