//! Windows XInput controller battery provider (discrete levels only).

use crate::error::BatteryError;
use crate::model::{
    BatteryLevel, Capabilities, DeviceKind, Endpoint, ProviderReading, ReadingSource,
    SupportStatus, TransportKind,
};
use crate::providers::{Deadline, DeviceContext, MatchDecision, Provider, ProviderMetadata};
use crate::transport::Transport;
use std::time::Duration;

pub(crate) struct XInputProvider;

impl Provider for XInputProvider {
    fn id(&self) -> &str {
        "os.xinput"
    }

    fn metadata(&self) -> ProviderMetadata {
        ProviderMetadata {
            version: "1.0.0".to_owned(),
            device_kind: DeviceKind::Gamepad,
            support_status: SupportStatus::Experimental,
            capabilities: Capabilities {
                discrete_level: true,
                ..Capabilities::default()
            },
            io_timeout: Duration::from_millis(200),
            cooldown: Duration::ZERO,
        }
    }

    fn display_name(&self, endpoint: &Endpoint) -> Option<String> {
        Some(
            endpoint
                .product_name
                .clone()
                .filter(|name| !name.is_empty())
                .unwrap_or_else(|| "XInput controller".to_owned()),
        )
    }

    fn matches(&self, endpoint: &Endpoint) -> MatchDecision {
        if endpoint.transport == TransportKind::XInput {
            MatchDecision::Exact
        } else {
            MatchDecision::NoMatch
        }
    }

    fn execute(
        &self,
        device: &DeviceContext<'_>,
        transport: &dyn Transport,
        deadline: Deadline,
    ) -> Result<ProviderReading, BatteryError> {
        deadline.check("XInput read")?;
        let report = transport
            .read_xinput(&device.endpoint.id)
            .map_err(|error| error.into_battery(&device.endpoint.id))?
            .ok_or_else(|| BatteryError::Unsupported {
                reason: "XInput is unavailable on this platform".to_owned(),
            })?;
        if report.battery_type == 0 {
            return Err(BatteryError::DeviceDisconnected {
                endpoint: device.endpoint.id.to_string(),
            });
        }
        let level = if report.battery_type == 1 {
            None
        } else if matches!(report.battery_type, 2 | 3) {
            Some(match report.battery_level {
                0 => BatteryLevel::Critical,
                1 => BatteryLevel::Low,
                2 => BatteryLevel::Medium,
                3 => BatteryLevel::Full,
                _ => return Err(BatteryError::invalid_response("invalid XInput level")),
            })
        } else {
            return Err(BatteryError::invalid_response(
                "invalid XInput battery type",
            ));
        };
        Ok(ProviderReading {
            level,
            source: ReadingSource::Os,
            ..ProviderReading::default()
        })
    }
}
