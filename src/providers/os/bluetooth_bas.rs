//! Standard Bluetooth LE Battery Service (BAS `0x2A19`) provider.

use crate::error::BatteryError;
use crate::model::{
    Capabilities, Endpoint, ProviderReading, ReadingSource, SupportStatus, TransportKind,
};
use crate::providers::{Deadline, DeviceContext, MatchDecision, Provider, ProviderMetadata};
use crate::transport::Transport;
use std::time::Duration;

pub(crate) struct BluetoothBasProvider;

impl Provider for BluetoothBasProvider {
    fn id(&self) -> &str {
        "bluetooth.bas"
    }

    fn metadata(&self) -> ProviderMetadata {
        ProviderMetadata {
            version: "1.0.0".to_owned(),
            support_status: SupportStatus::Experimental,
            capabilities: Capabilities {
                percentage: true,
                ..Capabilities::default()
            },
            io_timeout: Duration::from_secs(3),
            cooldown: Duration::ZERO,
        }
    }

    fn display_name(&self, endpoint: &Endpoint) -> Option<String> {
        Some(
            endpoint
                .product_name
                .clone()
                .filter(|name| !name.is_empty())
                .unwrap_or_else(|| "Unknown Bluetooth LE device".to_owned()),
        )
    }

    fn matches(&self, endpoint: &Endpoint) -> MatchDecision {
        if endpoint.transport == TransportKind::BluetoothLe {
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
        let timeout = deadline.remaining("BLE BAS read")?;
        let percentage = transport
            .read_bas(&device.endpoint.id, timeout)
            .map_err(|error| error.into_battery(&device.endpoint.id))?
            .ok_or_else(|| BatteryError::Unsupported {
                reason: "Bluetooth Battery Service is unavailable".to_owned(),
            })?;
        Ok(ProviderReading {
            percentage: Some(percentage),
            source: ReadingSource::BluetoothBas,
            ..ProviderReading::default()
        })
    }
}
