//! Razer wireless mouse and keyboard feature-report battery protocol.

use crate::error::BatteryError;
use crate::model::{DeviceKind, Endpoint, ProviderReading};
use crate::providers::fixed::{
    capabilities, charge_bool, experimental_metadata, feature_frame, reading, send_with_modes,
    sleep, SendMode,
};
use crate::providers::{Deadline, DeviceContext, MatchDecision, Provider, ProviderMetadata};
use crate::transport::HidSession;
use std::sync::Arc;
use std::time::Duration;

const RAZER_HEADSET_PIDS: &[u16] = &[
    0x0528, 0x053A, 0x0550, 0x0552, 0x0555, 0x0565, 0x0576, 0x0577, 0x0579, 0x057A, 0x057C, 0x057D,
    0x0A4E, 0x0A55,
];

const RAZER_WIRELESS_PIDS: &[u16] = &[
    // Wireless mice & dongles
    0x0013, 0x0015, 0x0016, 0x001F, 0x0020, 0x0024, 0x0025, 0x0029, 0x002E, 0x002F, 0x0032, 0x0034,
    0x0036, 0x0037, 0x0038, 0x0039, 0x003E, 0x003F, 0x0040, 0x0041, 0x0042, 0x0043, 0x0044, 0x0045,
    0x0046, 0x0048, 0x004C, 0x004F, 0x0050, 0x0053, 0x0054, 0x0059, 0x005A, 0x005B, 0x005C, 0x005E,
    0x005F, 0x0060, 0x0062, 0x0064, 0x0065, 0x0067, 0x006A, 0x006B, 0x006C, 0x006E, 0x006F, 0x0070,
    0x0071, 0x0072, 0x0073, 0x0077, 0x0078, 0x007A, 0x007B, 0x007C, 0x007D, 0x0080, 0x0083, 0x0084,
    0x0085, 0x0086, 0x0088, 0x008A, 0x008C, 0x008D, 0x008F, 0x0090, 0x0091, 0x0094, 0x0095, 0x0096,
    0x0098, 0x0099, 0x009A, 0x009C, 0x009E, 0x009F, 0x00A1, 0x00A3, 0x00A5, 0x00A6, 0x00A7, 0x00A8,
    0x00AA, 0x00AB, 0x00AF, 0x00B0, 0x00B2, 0x00B3, 0x00B4, 0x00B6, 0x00B7, 0x00B8, 0x00B9, 0x00BE,
    0x00BF, 0x00C0, 0x00C1, 0x00C2, 0x00C3, 0x00C4, 0x00C5, 0x00C7, 0x00C8, 0x00CB, 0x00CC, 0x00CD,
    0x00D0, 0x00D1, 0x00D3, 0x00D4, 0x00D6, 0x00D7, 0x00DA, 0x00DB, 0x00DE, 0x00DF, 0x00E0, 0x00E5,
    0x00E6, // Wireless keyboards
    0x0258, 0x025A, 0x025C, 0x0271, 0x0290, 0x0292, 0x0296, 0x0298, 0x02B9, 0x02BA, 0x02C9, 0x02CD,
    0x02D2, 0x02D4, 0x02D5, 0x02D7,
];

pub(crate) fn providers() -> Vec<Arc<dyn Provider>> {
    vec![Arc::new(RazerMouseProvider)]
}

struct RazerMouseProvider;

impl Provider for RazerMouseProvider {
    fn id(&self) -> &str {
        "razer.mouse"
    }

    fn metadata(&self) -> ProviderMetadata {
        experimental_metadata(
            DeviceKind::Mouse,
            capabilities(true, true, false),
            Duration::from_millis(160),
        )
    }

    fn device_kind(&self, endpoint: &Endpoint) -> DeviceKind {
        if endpoint.pid.is_some_and(|pid| pid >= 0x0200) {
            DeviceKind::Keyboard
        } else {
            DeviceKind::Mouse
        }
    }

    fn display_name(&self, endpoint: &Endpoint) -> Option<String> {
        Some(if endpoint.pid.is_some_and(|pid| pid >= 0x0200) {
            "Razer wireless keyboard".to_owned()
        } else {
            "Razer wireless mouse".to_owned()
        })
    }

    fn matches(&self, endpoint: &Endpoint) -> MatchDecision {
        if endpoint.vid == Some(0x1532)
            && endpoint.pid.is_some_and(|pid| {
                !RAZER_HEADSET_PIDS.contains(&pid) && RAZER_WIRELESS_PIDS.contains(&pid)
            })
            && endpoint.usage_page.is_some_and(|page| page >= 0xFF00)
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
        for attempt in 0..4_u8 {
            let transaction = 0x20 + attempt;
            let request = razer_request(transaction);
            send_with_modes(session, device, &[SendMode::Feature], &request)?;
            for _ in 0..8 {
                deadline.check("Razer feature read")?;
                let frame = feature_frame(session, device, 0, 90)?;
                if frame.len() < 3 || frame[2] != transaction {
                    continue;
                }
                match frame[1] {
                    2 => return parse_razer_mouse(&frame),
                    5 => {
                        return Err(BatteryError::Unsupported {
                            reason: "Razer receiver rejected battery query".to_owned(),
                        });
                    }
                    3 | 4 => break,
                    1 => sleep(deadline, Duration::from_millis(20))?,
                    _ => continue,
                }
            }
        }
        Err(BatteryError::timeout("Razer battery query"))
    }
}

fn razer_request(transaction: u8) -> Vec<u8> {
    let mut frame = vec![0_u8; 91];
    frame[2] = transaction;
    frame[6] = 2;
    frame[7] = 0x07;
    frame[8] = 0x80;
    frame[89] = frame[3..=88].iter().fold(0_u8, |crc, byte| crc ^ byte);
    frame
}

fn parse_razer_mouse(frame: &[u8]) -> Result<ProviderReading, BatteryError> {
    if frame.len() != 91 || frame[5] != 0 || frame[6] < 2 || frame[7] != 0x07 || frame[8] != 0x80 {
        return Err(BatteryError::invalid_response("invalid Razer response"));
    }
    let crc = frame[3..=88].iter().fold(0_u8, |crc, byte| crc ^ byte);
    if frame[89] != crc {
        return Err(BatteryError::ChecksumFailed);
    }
    let raw = frame[10];
    let percentage = ((u16::from(raw) * 100 + 127) / 255) as u8;
    Ok(reading(Some(percentage), charge_bool(frame[9]), None))
}
