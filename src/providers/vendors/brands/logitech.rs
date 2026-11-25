//! Logitech HID++ 2.0, HID++ 1.0, G930, G435, and ASTRO headset battery protocols.

use crate::error::BatteryError;
use crate::model::{
    BatteryLevel, Capabilities, ChargeState, DeviceKind, Endpoint, EndpointId, HidBusType,
    ProviderReading, Reachability, ReadingSource, SupportStatus,
};
use crate::providers::fixed::{
    capabilities, checked_percentage, crc16_poly_1021, drain_input, experimental_metadata,
    feature_frame, frame, interpolate_mv, reading, send_with_modes, sleep, unnumbered_payload,
    wait_frame, SendMode, OUTPUT,
};
use crate::providers::{
    infer_device_kind_from_name, Deadline, DeviceContext, MatchDecision, Provider, ProviderMetadata,
};
use crate::transport::{HidSession, TransportError};
use std::collections::HashMap;
use std::sync::{Arc, Mutex};
use std::time::Duration;

const REPORT_LONG: u8 = 0x11;
const SW_ID: u8 = 0x0E;
const INDEX_CACHE_LIMIT: usize = 64;
const RECEIVER_SCAN_INDEXES: [u8; 7] = [1, 2, 3, 4, 5, 6, 0xFF];
const DIRECT_SCAN_INDEXES: [u8; 7] = [0xFF, 1, 2, 3, 4, 5, 6];

fn scan_indexes_for(endpoint: &Endpoint) -> &'static [u8] {
    let is_direct = endpoint.bus_type == Some(HidBusType::Bluetooth)
        || endpoint
            .pid
            .is_some_and(|p| !(0xC500..=0xC5FF).contains(&p));
    if is_direct {
        &DIRECT_SCAN_INDEXES
    } else {
        &RECEIVER_SCAN_INDEXES
    }
}

pub(crate) fn providers() -> Vec<Arc<dyn Provider>> {
    vec![
        Arc::new(LogitechProvider::new()),
        Arc::new(LogitechHidpp10Provider),
        Arc::new(LogitechG930Provider),
        Arc::new(LogitechG435Provider),
        Arc::new(AstroModernProvider),
        Arc::new(AstroLegacyProvider),
    ]
}

struct LogitechProvider {
    indexes: Mutex<HashMap<EndpointId, u8>>,
}

impl LogitechProvider {
    fn new() -> Self {
        Self {
            indexes: Mutex::new(HashMap::new()),
        }
    }

    #[allow(clippy::too_many_arguments)]
    fn wait_for(
        &self,
        session: &mut dyn HidSession,
        device: &DeviceContext<'_>,
        deadline: Deadline,
        budget: Duration,
        expected_device: u8,
        feature: u8,
        function: u8,
    ) -> Result<Vec<u8>, BatteryError> {
        let operation_deadline = Deadline::after(deadline.remaining("Logitech HID++")?.min(budget));
        for _ in 0..32 {
            let timeout = deadline
                .remaining("Logitech HID++")?
                .min(operation_deadline.remaining("Logitech HID++")?)
                .min(Duration::from_millis(60));
            let report = match session.read_input(timeout) {
                Ok(report) => frame(report),
                Err(TransportError::Timeout) => {
                    if operation_deadline.check("Logitech HID++").is_err() {
                        return Err(BatteryError::timeout("Logitech HID++"));
                    }
                    continue;
                }
                Err(error) => return Err(error.into_battery(&device.endpoint.id)),
            };
            if report.len() >= 7 && report[0] == REPORT_LONG && report[1] == expected_device {
                if report[2] == 0xFF && report[3] == feature && report[4] == function {
                    return Err(BatteryError::invalid_response(format!(
                        "HID++ error response (code 0x{:02X})",
                        report[5]
                    )));
                }
                if report[2] == feature && report[3] == function {
                    return Ok(report);
                }
            }
        }
        Err(BatteryError::timeout("Logitech HID++"))
    }

    fn call(
        &self,
        session: &mut dyn HidSession,
        device: &DeviceContext<'_>,
        deadline: Deadline,
        device_index: u8,
        feature: u8,
        function: u8,
    ) -> Result<Vec<u8>, BatteryError> {
        let budget = if feature == 0 && function == 1 {
            Duration::from_millis(300)
        } else {
            Duration::from_millis(350)
        };
        self.call_with_budget(
            session,
            device,
            deadline,
            device_index,
            feature,
            function,
            budget,
        )
    }

    #[allow(clippy::too_many_arguments)]
    fn call_with_budget(
        &self,
        session: &mut dyn HidSession,
        device: &DeviceContext<'_>,
        deadline: Deadline,
        device_index: u8,
        feature: u8,
        function: u8,
        budget: Duration,
    ) -> Result<Vec<u8>, BatteryError> {
        let request = logitech_request(device_index, feature, function);
        session
            .write_output(REPORT_LONG, &request[1..])
            .map_err(|error| error.into_battery(&device.endpoint.id))?;
        self.wait_for(
            session,
            device,
            deadline,
            budget,
            device_index,
            feature,
            request[3],
        )
    }

    fn feature_index(
        &self,
        session: &mut dyn HidSession,
        device: &DeviceContext<'_>,
        deadline: Deadline,
        device_index: u8,
        feature: u16,
    ) -> Result<u8, BatteryError> {
        let mut last_timeout = None;
        for _ in 0..2 {
            let mut request = logitech_request(device_index, 0, 0);
            request[4..6].copy_from_slice(&feature.to_be_bytes());
            session
                .write_output(REPORT_LONG, &request[1..])
                .map_err(|error| error.into_battery(&device.endpoint.id))?;
            match self.wait_for(
                session,
                device,
                deadline,
                Duration::from_millis(350),
                device_index,
                0,
                request[3],
            ) {
                Ok(response) => return Ok(response[4]),
                Err(error @ BatteryError::Timeout { .. }) => last_timeout = Some(error),
                Err(error) => return Err(error),
            }
        }
        Err(last_timeout.unwrap_or_else(|| BatteryError::timeout("Logitech HID++ feature index")))
    }

    fn resolve_index(
        &self,
        session: &mut dyn HidSession,
        device: &DeviceContext<'_>,
        deadline: Deadline,
    ) -> Result<(u8, bool), BatteryError> {
        if let Some(index) = self
            .indexes
            .lock()
            .map_err(|_| BatteryError::Internal {
                message: "Logitech index cache lock poisoned".to_owned(),
            })?
            .get(&device.endpoint.id)
            .copied()
        {
            return Ok((index, true));
        }
        let indexes = scan_indexes_for(device.endpoint);
        let total = indexes.len();
        for (pos, &index) in indexes.iter().enumerate() {
            let remaining = deadline.remaining("Logitech index scan")?;
            let slots_left = (total - pos) as u32;
            // Reserve ~220ms for post-discovery feature/battery queries when possible so
            // all 7 candidate indexes (`1..=6` + `0xFF`) remain reachable under the 1s default budget
            // while still granting up to 300ms per slot under larger tray/CLI budgets.
            let scan_pool = remaining.saturating_sub(Duration::from_millis(220));
            let per_slot = if pos == 0 {
                (scan_pool / slots_left.min(4))
                    .clamp(Duration::from_millis(80), Duration::from_millis(300))
            } else {
                (scan_pool / slots_left)
                    .clamp(Duration::from_millis(70), Duration::from_millis(300))
            }
            .min(remaining);
            match self.call_with_budget(session, device, deadline, index, 0, 1, per_slot) {
                Ok(_) => {
                    let mut map = self.indexes.lock().map_err(|_| BatteryError::Internal {
                        message: "Logitech index cache lock poisoned".to_owned(),
                    })?;
                    if map.len() >= INDEX_CACHE_LIMIT && !map.contains_key(&device.endpoint.id) {
                        map.clear();
                    }
                    map.insert(device.endpoint.id.clone(), index);
                    return Ok((index, false));
                }
                // Empty/offline receiver slots may either time out or return an HID++ error
                // frame (e.g. ResourceError 0x08 / UnknownDevice 0x09); continue probing.
                Err(BatteryError::Timeout { .. } | BatteryError::InvalidResponse { .. }) => {}
                Err(error) => return Err(error),
            }
        }
        Err(BatteryError::timeout("Logitech device index discovery"))
    }

    fn invalidate_index(&self, endpoint_id: &EndpointId) {
        if let Ok(mut indexes) = self.indexes.lock() {
            indexes.remove(endpoint_id);
        }
    }

    fn read_for_index(
        &self,
        session: &mut dyn HidSession,
        device: &DeviceContext<'_>,
        deadline: Deadline,
        index: u8,
    ) -> Result<ProviderReading, BatteryError> {
        let feature_1000 = self.feature_index(session, device, deadline, index, 0x1000)?;
        if feature_1000 != 0 {
            let response = self.call(session, device, deadline, index, feature_1000, 0)?;
            return parse_logitech_battery(&response, 0);
        }
        let feature_1004 = self.feature_index(session, device, deadline, index, 0x1004)?;
        if feature_1004 != 0 {
            let response = self.call(session, device, deadline, index, feature_1004, 1)?;
            return parse_logitech_battery(&response, 1);
        }
        let feature_1001 = self.feature_index(session, device, deadline, index, 0x1001)?;
        if feature_1001 != 0 {
            let response = self.call(session, device, deadline, index, feature_1001, 0)?;
            return parse_logitech_voltage(&response);
        }
        let feature_1f20 = self.feature_index(session, device, deadline, index, 0x1F20)?;
        if feature_1f20 != 0 {
            let response = self.call(session, device, deadline, index, feature_1f20, 0)?;
            return parse_logitech_adc(&response, device.endpoint.pid.unwrap_or(0));
        }
        Err(BatteryError::Unsupported {
            reason: "Logitech device exposes no supported HID++ battery feature".to_owned(),
        })
    }
}

impl Provider for LogitechProvider {
    fn id(&self) -> &str {
        "logitech.hidpp"
    }

    fn metadata(&self) -> ProviderMetadata {
        ProviderMetadata {
            version: "1.0.0".to_owned(),
            device_kind: DeviceKind::Mouse,
            support_status: SupportStatus::Verified,
            capabilities: capabilities(true, true, false),
            io_timeout: Duration::from_millis(350),
            cooldown: Duration::from_millis(250),
        }
    }

    fn device_kind(&self, endpoint: &Endpoint) -> DeviceKind {
        if let Some(kind) = infer_device_kind_from_name(endpoint.product_name.as_deref()) {
            return kind;
        }
        if matches!(
            endpoint.pid,
            Some(
                0x0A5C
                    | 0x0A89
                    | 0x0A5B
                    | 0x0A87
                    | 0x0AB5
                    | 0x0AFE
                    | 0x0B1F
                    | 0x0AAA
                    | 0x0ABA
                    | 0x0AA7
                    | 0x0AFB
                    | 0x0AFC
                    | 0x0AC4
                    | 0x0A66
            )
        ) {
            return DeviceKind::Headset;
        }
        DeviceKind::Mouse
    }

    fn display_name(&self, endpoint: &Endpoint) -> Option<String> {
        let model = match endpoint.pid {
            Some(0xC53F) => "G304",
            Some(0xC547) | Some(0xC54D) | Some(0xC09B) => "GPW3",
            Some(0xC54F) => "PRO X3 SUPERSTRIKE",
            Some(0xC548) => "M750M",
            Some(0xC539) => "G102",
            Some(0xC52B) => "Unifying receiver",
            _ => "HID++ device",
        };
        Some(format!("Logitech {model}"))
    }

    fn matches(&self, endpoint: &Endpoint) -> MatchDecision {
        let standard_long = endpoint.usage_page == Some(0xFF00) && endpoint.usage == Some(2);
        let bolt_or_pro = endpoint.usage_page == Some(0xFF43)
            && (endpoint.usage == Some(0x0202)
                || (endpoint.pid == Some(0xC54F) && endpoint.usage == Some(0x0302)));
        let g535_consumer = endpoint.pid == Some(0x0AC4)
            && endpoint.usage_page == Some(0x000C)
            && endpoint.usage == Some(0x0001)
            && matches!(endpoint.interface_number, None | Some(3));
        if endpoint.vid == Some(0x046D) && (standard_long || bolt_or_pro || g535_consumer) {
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
        let (index, from_cache) = self.resolve_index(session, device, deadline)?;
        let result = self.read_for_index(session, device, deadline, index);
        if from_cache
            && matches!(
                result,
                Err(BatteryError::Timeout { .. } | BatteryError::DeviceDisconnected { .. })
            )
        {
            self.invalidate_index(&device.endpoint.id);
        }
        result
    }
}

fn parse_logitech_battery(
    response: &[u8],
    battery_function: u8,
) -> Result<ProviderReading, BatteryError> {
    if response.len() < 7 {
        return Err(BatteryError::invalid_response(
            "truncated Logitech HID++ battery response",
        ));
    }
    let percentage = checked_percentage(response[4])?;
    let charge = if battery_function == 1 {
        match response[6] {
            0 => ChargeState::Discharging,
            1 | 2 => ChargeState::Charging,
            3 => ChargeState::Full,
            _ => ChargeState::Unknown,
        }
    } else {
        match response[6] {
            0 => ChargeState::Discharging,
            1 | 2 | 4 => ChargeState::Charging,
            3 => ChargeState::Full,
            _ => ChargeState::Unknown,
        }
    };
    Ok(reading(Some(percentage), charge, None))
}

fn logitech_request(device: u8, feature: u8, function: u8) -> Vec<u8> {
    let mut request = vec![0_u8; 20];
    request[0] = REPORT_LONG;
    request[1] = device;
    request[2] = feature;
    request[3] = ((function & 0x0F) << 4) | SW_ID;
    request
}

const LOGITECH_LIION_TABLE: &[(u32, u8)] = &[
    (4186, 100),
    (4067, 90),
    (3989, 80),
    (3922, 70),
    (3859, 60),
    (3811, 50),
    (3778, 40),
    (3751, 30),
    (3717, 20),
    (3671, 10),
    (3646, 5),
    (3579, 2),
    (3500, 0),
];

const LOGITECH_HEADSET_DEFAULT_TABLE: &[(u32, u8)] = &[
    (4100, 100),
    (3850, 50),
    (3780, 30),
    (3730, 20),
    (3650, 5),
    (3300, 0),
];

const LOGITECH_G435_TABLE: &[(u32, u8)] = &[
    (4200, 100),
    (4150, 95),
    (4100, 90),
    (4050, 85),
    (4000, 80),
    (3950, 75),
    (3900, 70),
    (3850, 62),
    (3800, 55),
    (3750, 45),
    (3700, 35),
    (3650, 25),
    (3600, 15),
    (3550, 10),
    (3500, 5),
    (3400, 0),
];

fn parse_logitech_voltage(response: &[u8]) -> Result<ProviderReading, BatteryError> {
    if response.len() < 7 {
        return Err(BatteryError::invalid_response(
            "truncated Logitech HID++ 0x1001 response",
        ));
    }
    let mv = u32::from(u16::from_be_bytes([response[4], response[5]]));
    let charge = if (response[6] & 0x90) != 0 {
        ChargeState::Charging
    } else {
        ChargeState::Discharging
    };
    let pct = interpolate_mv(mv, LOGITECH_LIION_TABLE);
    Ok(reading(Some(pct), charge, Some(mv)))
}

fn parse_logitech_adc(response: &[u8], pid: u16) -> Result<ProviderReading, BatteryError> {
    if response.len() < 7 {
        return Err(BatteryError::invalid_response(
            "truncated Logitech HID++ 0x1F20 response",
        ));
    }
    let mv = u32::from(u16::from_be_bytes([response[4], response[5]]));
    let charge = match response[6] {
        0x03 => ChargeState::Charging,
        0x01 => ChargeState::Discharging,
        _ => ChargeState::Unknown,
    };
    let table: &[(u32, u8)] = match pid {
        0x0A66 => &[
            (4200, 100),
            (3850, 50),
            (3790, 30),
            (3750, 20),
            (3680, 5),
            (3330, 0),
        ],
        0x0AC4 => &[
            (4175, 100),
            (3817, 50),
            (3766, 30),
            (3730, 20),
            (3664, 5),
            (3310, 0),
        ],
        0x0A5C | 0x0A89 | 0x0A5B | 0x0A87 | 0x0AB5 | 0x0AFE | 0x0B1F => &[
            (4100, 100),
            (3950, 80),
            (3850, 60),
            (3750, 40),
            (3650, 20),
            (3500, 10),
            (3300, 5),
            (3150, 0),
        ],
        0x0AAA | 0x0ABA => &[
            (4102, 100),
            (4046, 95),
            (3993, 90),
            (3950, 85),
            (3915, 80),
            (3877, 75),
            (3838, 70),
            (3809, 65),
            (3789, 60),
            (3772, 55),
            (3758, 50),
            (3747, 45),
            (3736, 40),
            (3722, 35),
            (3707, 30),
            (3687, 25),
            (3661, 20),
            (3644, 15),
            (3548, 10),
            (3426, 5),
            (3293, 0),
        ],
        0x0AA7 | 0x0AFB | 0x0AFC => &[
            (4150, 100),
            (3830, 50),
            (3780, 30),
            (3740, 20),
            (3670, 5),
            (3320, 0),
        ],
        _ => LOGITECH_HEADSET_DEFAULT_TABLE,
    };
    Ok(reading(Some(interpolate_mv(mv, table)), charge, Some(mv)))
}

struct LogitechG435Provider;

impl Provider for LogitechG435Provider {
    fn id(&self) -> &str {
        "logitech.g435"
    }

    fn metadata(&self) -> ProviderMetadata {
        experimental_metadata(
            DeviceKind::Headset,
            capabilities(true, false, true),
            Duration::from_millis(500),
        )
    }

    fn display_name(&self, _endpoint: &Endpoint) -> Option<String> {
        Some("Logitech G435".to_owned())
    }

    fn matches(&self, endpoint: &Endpoint) -> MatchDecision {
        if endpoint.vid == Some(0x046D)
            && endpoint.pid == Some(0x0ADF)
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
        let send_g435 = |session: &mut dyn HidSession, cmd: u8, body: &[u8]| {
            let mut payload = vec![0_u8; 64];
            payload[0] = cmd;
            payload[1] = body.len() as u8;
            payload[4..4 + body.len()].copy_from_slice(body);
            let crc = crc16_poly_1021(&payload, 0x0000);
            payload[2] = (crc & 0xFF) as u8;
            payload[3] = (crc >> 8) as u8;
            let mut req = Vec::with_capacity(65);
            req.push(0x00);
            req.extend_from_slice(&payload);
            send_with_modes(session, device, OUTPUT, &req)
        };
        send_g435(session, 0x84, &[0x01])?;
        send_g435(session, 0x85, &[0x01])?;
        send_g435(
            session,
            0x80,
            &[
                0x50, 0x41, 0x06, 0x01, 0x0D, 0x00, 0x25, 0x34, 0x12, 0x5A, 0x5A, 0x01, 0x00, 0x00,
                0x00, 0xF0, 0x00, 0x00, 0x00,
            ],
        )?;
        send_g435(session, 0x80, &[0x50, 0x41, 0x0E, 0x02, 0x01, 0xE0])?;
        send_g435(session, 0x80, &[0x50, 0x41, 0x0E, 0x03, 0x02, 0xE1, 0x01])?;
        send_g435(session, 0x80, &[0x50, 0x41, 0x0E, 0x04, 0x01, 0xE0])?;
        send_g435(session, 0x80, &[0x50, 0x41, 0x06, 0x05, 0x01, 0x00, 0x31])?;
        let reply = wait_frame(
            session,
            device,
            deadline,
            Duration::from_millis(400),
            16,
            |f| Ok(find_g435_voltage(unnumbered_payload(f)).is_some()),
        );
        let _ = send_g435(session, 0x85, &[0x02]);
        let frame = reply?;
        let mv = find_g435_voltage(unnumbered_payload(&frame))
            .ok_or_else(|| BatteryError::invalid_response("missing G435 battery voltage"))?;
        Ok(reading(
            Some(interpolate_mv(mv, LOGITECH_G435_TABLE)),
            ChargeState::Unknown,
            Some(mv),
        ))
    }
}

fn find_g435_voltage(r: &[u8]) -> Option<u32> {
    for w in r.windows(8) {
        if w[0] == 0x00 && w[1] == 0x07 && w[2] == 0x00 && w[3] == 0x06 && w[5] == 0x00 {
            let mv = u32::from(u16::from_le_bytes([w[6], w[7]]));
            if (2500..=4500).contains(&mv) {
                return Some(mv);
            }
        }
    }
    None
}

struct AstroModernProvider;

impl Provider for AstroModernProvider {
    fn id(&self) -> &str {
        "astro.a50_modern"
    }

    fn metadata(&self) -> ProviderMetadata {
        experimental_metadata(
            DeviceKind::Headset,
            capabilities(true, true, false),
            Duration::from_millis(500),
        )
    }

    fn display_name(&self, endpoint: &Endpoint) -> Option<String> {
        Some(match endpoint.pid {
            Some(0x0B0B) => "ASTRO A50 X".to_owned(),
            Some(0x0B1C) => "ASTRO A50 Gen 5".to_owned(),
            _ => "ASTRO A50".to_owned(),
        })
    }

    fn matches(&self, endpoint: &Endpoint) -> MatchDecision {
        if endpoint.vid == Some(0x046D)
            && matches!(endpoint.pid, Some(0x0B0B | 0x0B1C))
            && endpoint.usage_page == Some(0xFF32)
            && endpoint.usage == Some(0x0074)
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
        req[..6].copy_from_slice(&[0x02, 0x0C, 0x03, 0x00, 0x06, 0x0C]);
        send_with_modes(session, device, OUTPUT, &req)?;
        let r = wait_frame(
            session,
            device,
            deadline,
            Duration::from_millis(500),
            8,
            |r| {
                Ok(r.len() >= 9
                    && r[0] == 0x02
                    && r[1] == 0x0C
                    && r[2] >= 6
                    && usize::from(r[2]) + 3 <= r.len()
                    && r[3] == 0x00
                    && r[4] == 0x06
                    && matches!(r[5], 0x00 | 0x0C))
            },
        )?;
        let pct = checked_percentage(r[6])?;
        let charge = if r[8] != 0 {
            ChargeState::Charging
        } else {
            ChargeState::Discharging
        };
        Ok(reading(Some(pct), charge, None))
    }
}

const ASTRO_LEGACY_PIDS: &[u16] = &[
    0x002C, 0x002D, 0x002A, 0x0015, 0x0014, 0x0018, 0x001D, 0x0026, 0x0037, 0x0038, 0x0039, 0x0043,
    0x0044, 0x0045,
];

struct AstroLegacyProvider;

impl Provider for AstroLegacyProvider {
    fn id(&self) -> &str {
        "astro.legacy"
    }

    fn metadata(&self) -> ProviderMetadata {
        experimental_metadata(
            DeviceKind::Headset,
            capabilities(true, true, false),
            Duration::from_millis(500),
        )
    }

    fn display_name(&self, endpoint: &Endpoint) -> Option<String> {
        Some(match endpoint.pid {
            Some(0x002C | 0x002D | 0x002A) => "ASTRO A50 Gen 4".to_owned(),
            Some(0x0015 | 0x0014 | 0x0018 | 0x001D | 0x0026) => "ASTRO A50 Gen 3".to_owned(),
            _ => "ASTRO A20 Wireless".to_owned(),
        })
    }

    fn matches(&self, endpoint: &Endpoint) -> MatchDecision {
        if endpoint.vid == Some(0x9886)
            && endpoint.pid.is_some_and(|p| ASTRO_LEGACY_PIDS.contains(&p))
            && matches!(endpoint.usage_page, Some(0xFF32 | 0xFF31 | 0xFF00))
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
        let ask = |session: &mut dyn HidSession, cmd: u8| {
            drain_input(session, device, deadline, 6)?;
            let mut req = vec![0_u8; 64];
            req[0] = 0x02;
            req[1] = cmd;
            send_with_modes(session, device, OUTPUT, &req)?;
            wait_frame(
                session,
                device,
                deadline,
                Duration::from_millis(500),
                8,
                |r| Ok(r.len() >= 4 && r[0] == 0x02 && matches!(r[1], 0x01 | 0x02)),
            )
        };
        let dock_reply = ask(session, 0x54)?;
        let docked = if dock_reply[1] == 0x02 {
            let st = dock_reply[3];
            let d = (st & 0x01) != 0;
            let connected = d || (st & 0x02) != 0;
            if !connected {
                return Ok(ProviderReading {
                    reachability: Reachability::Offline,
                    source: ReadingSource::VendorHid,
                    ..ProviderReading::default()
                });
            }
            d
        } else {
            false
        };
        let bat_reply = ask(session, 0x7C)?;
        if bat_reply[1] != 0x02 {
            return Ok(ProviderReading {
                reachability: Reachability::Offline,
                source: ReadingSource::VendorHid,
                ..ProviderReading::default()
            });
        }
        let raw = bat_reply[3];
        let pct = checked_percentage(raw & 0x7F)?;
        let charging = ((raw & 0x80) != 0) || (docked && pct < 100);
        let charge = if charging {
            ChargeState::Charging
        } else if docked && pct == 100 {
            ChargeState::Full
        } else {
            ChargeState::Discharging
        };
        Ok(reading(Some(pct), charge, None))
    }
}

struct LogitechG930Provider;

impl Provider for LogitechG930Provider {
    fn id(&self) -> &str {
        "logitech.g930"
    }

    fn metadata(&self) -> ProviderMetadata {
        experimental_metadata(
            DeviceKind::Headset,
            capabilities(true, false, false),
            Duration::from_millis(500),
        )
    }

    fn display_name(&self, _endpoint: &Endpoint) -> Option<String> {
        Some("Logitech G930".to_owned())
    }

    fn matches(&self, endpoint: &Endpoint) -> MatchDecision {
        if endpoint.vid == Some(0x046D)
            && endpoint.pid == Some(0x0A1F)
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
        let mut req = vec![0_u8; 64];
        req[..11].copy_from_slice(&[
            0xFF, 0x09, 0x00, 0xFD, 0xF4, 0x10, 0x05, 0xB1, 0xBF, 0xA0, 0x04,
        ]);
        send_with_modes(session, device, &[SendMode::Feature], &req)?;

        let mut buf = Vec::new();
        for attempt in 0..3 {
            buf = feature_frame(session, device, 0xFF, 64)?;
            if attempt < 2 {
                sleep(deadline, Duration::from_millis(100))?;
            }
        }
        if buf.len() < 14 {
            return Err(BatteryError::invalid_response(
                "truncated Logitech G930 feature report",
            ));
        }
        let pct = interpolate_mv(u32::from(buf[13]), &[(91, 100), (44, 0)]);
        Ok(reading(Some(pct), ChargeState::Unknown, None))
    }
}

const REPORT_SHORT: u8 = 0x10;
const HIDPP10_GET_REGISTER: u8 = 0x81;
const HIDPP10_ERROR_SUB_ID: u8 = 0x8F;
const HIDPP10_REG_BATTERY_STATUS: u8 = 0x07;
const HIDPP10_REG_BATTERY_MILEAGE: u8 = 0x0D;
const HIDPP10_SCAN_INDEXES: [u8; 7] = [0x00, 1, 2, 3, 4, 5, 6];

enum Hidpp10RegisterReply {
    Data([u8; 3]),
    Error(u8),
}

struct LogitechHidpp10Provider;

impl LogitechHidpp10Provider {
    fn query_register(
        session: &mut dyn HidSession,
        device: &DeviceContext<'_>,
        deadline: Deadline,
        budget: Duration,
        device_index: u8,
        register: u8,
    ) -> Result<Hidpp10RegisterReply, BatteryError> {
        let payload = [
            device_index,
            HIDPP10_GET_REGISTER,
            register,
            0x00,
            0x00,
            0x00,
        ];
        session
            .write_output(REPORT_SHORT, &payload)
            .map_err(|error| error.into_battery(&device.endpoint.id))?;

        let op_deadline = Deadline::after(deadline.remaining("Logitech HID++ 1.0")?.min(budget));
        for _ in 0..16 {
            let timeout = deadline
                .remaining("Logitech HID++ 1.0")?
                .min(op_deadline.remaining("Logitech HID++ 1.0")?)
                .min(Duration::from_millis(60));
            let report = match session.read_input(timeout) {
                Ok(report) => frame(report),
                Err(TransportError::Timeout) => {
                    if op_deadline.check("Logitech HID++ 1.0").is_err() {
                        return Err(BatteryError::timeout("Logitech HID++ 1.0"));
                    }
                    continue;
                }
                Err(error) => return Err(error.into_battery(&device.endpoint.id)),
            };
            if report.len() < 6 || report[0] != REPORT_SHORT || report[1] != device_index {
                continue;
            }
            if report[2] == HIDPP10_ERROR_SUB_ID
                && report[3] == HIDPP10_GET_REGISTER
                && report[4] == register
            {
                return Ok(Hidpp10RegisterReply::Error(report[5]));
            }
            if report.len() >= 7 && report[2] == HIDPP10_GET_REGISTER && report[3] == register {
                return Ok(Hidpp10RegisterReply::Data([
                    report[4], report[5], report[6],
                ]));
            }
        }
        Err(BatteryError::timeout("Logitech HID++ 1.0"))
    }
}

impl Provider for LogitechHidpp10Provider {
    fn id(&self) -> &str {
        "logitech.hidpp10"
    }

    fn metadata(&self) -> ProviderMetadata {
        ProviderMetadata {
            version: "1.0.0".to_owned(),
            device_kind: DeviceKind::Peripheral,
            support_status: SupportStatus::Experimental,
            capabilities: Capabilities {
                percentage: true,
                discrete_level: true,
                charge_state: true,
                voltage: false,
                reachability: false,
            },
            io_timeout: Duration::from_millis(250),
            cooldown: Duration::from_millis(250),
        }
    }

    fn device_kind(&self, endpoint: &Endpoint) -> DeviceKind {
        infer_device_kind_from_name(endpoint.product_name.as_deref())
            .unwrap_or(DeviceKind::Peripheral)
    }

    fn display_name(&self, _endpoint: &Endpoint) -> Option<String> {
        Some("Logitech HID++ 1.0 device".to_owned())
    }

    fn matches(&self, endpoint: &Endpoint) -> MatchDecision {
        if endpoint.vid == Some(0x046D)
            && endpoint.usage_page == Some(0xFF00)
            && endpoint.usage == Some(0x0001)
        {
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
        let total = HIDPP10_SCAN_INDEXES.len();
        for (pos, &idx) in HIDPP10_SCAN_INDEXES.iter().enumerate() {
            let remaining = deadline.remaining("Logitech HID++ 1.0")?;
            let slots_left = (total - pos) as u32;
            let per_slot = (remaining / slots_left)
                .clamp(Duration::from_millis(60), Duration::from_millis(250))
                .min(remaining);

            match Self::query_register(
                session,
                device,
                deadline,
                per_slot,
                idx,
                HIDPP10_REG_BATTERY_MILEAGE,
            ) {
                Ok(Hidpp10RegisterReply::Data([p0, _p1, p2])) => {
                    let pct = checked_percentage(p0 & 0x7F)?;
                    let charge = match p2 >> 6 {
                        0 => ChargeState::Discharging,
                        1 => ChargeState::Charging,
                        2 => ChargeState::Full,
                        _ => ChargeState::Unknown,
                    };
                    return Ok(reading(Some(pct), charge, None));
                }
                Ok(Hidpp10RegisterReply::Error(code)) => {
                    // 0x08 ResourceError / 0x09 UnknownDevice / 0x0A ConnectionRequestFailed
                    if matches!(code, 0x08..=0x0A) {
                        continue;
                    }
                    let fallback_budget = deadline
                        .remaining("Logitech HID++ 1.0")?
                        .min(Duration::from_millis(250));
                    match Self::query_register(
                        session,
                        device,
                        deadline,
                        fallback_budget,
                        idx,
                        HIDPP10_REG_BATTERY_STATUS,
                    ) {
                        Ok(Hidpp10RegisterReply::Data([p0, p1, _p2])) => {
                            return parse_hidpp10_battery_status(p0, p1);
                        }
                        Ok(Hidpp10RegisterReply::Error(_)) | Err(BatteryError::Timeout { .. }) => {}
                        Err(error) => return Err(error),
                    }
                }
                Err(BatteryError::Timeout { .. }) => {}
                Err(error) => return Err(error),
            }
        }
        Err(BatteryError::timeout("Logitech HID++ 1.0"))
    }
}

fn parse_hidpp10_battery_status(p0: u8, p1: u8) -> Result<ProviderReading, BatteryError> {
    let level = match p0 {
        0 => None,
        1 | 2 => Some(BatteryLevel::Critical),
        3 | 4 => Some(BatteryLevel::Low),
        5 | 6 => Some(BatteryLevel::High),
        7 => Some(BatteryLevel::Full),
        _ => {
            return Err(BatteryError::invalid_response(format!(
                "unknown Logitech HID++ 1.0 battery level 0x{p0:02X}"
            )));
        }
    };
    let charge_state = match p1 {
        0x00..=0x1F => ChargeState::Discharging,
        0x21 | 0x24 | 0x25 | 0x26 => ChargeState::Charging,
        0x22 => ChargeState::Full,
        _ => ChargeState::Unknown,
    };
    Ok(ProviderReading {
        percentage: None,
        level,
        charge_state,
        source: ReadingSource::VendorHid,
        ..ProviderReading::default()
    })
}
