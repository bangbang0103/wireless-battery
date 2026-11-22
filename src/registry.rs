//! Static provider registry.

use crate::error::BatteryError;
use crate::model::{
    Capabilities, ChargeState, DeviceKind, Endpoint, ProviderReading, ReadingSource, SupportStatus,
    TransportKind,
};
use crate::providers::{Deadline, DeviceContext, MatchDecision, Provider, ProviderMetadata};
use crate::transport::HidSession;
use std::collections::HashSet;
use std::sync::Arc;
use std::time::Duration;

/// Immutable provider collection.
#[derive(Default, Clone)]
pub struct Registry {
    providers: Vec<Arc<dyn Provider>>,
}

impl Registry {
    /// Build a registry from explicit providers, rejecting duplicate IDs.
    pub fn new(providers: Vec<Arc<dyn Provider>>) -> Result<Self, BatteryError> {
        let mut ids = HashSet::new();
        for provider in &providers {
            if !ids.insert(provider.id().to_owned()) {
                return Err(BatteryError::Internal {
                    message: format!("duplicate provider ID: {}", provider.id()),
                });
            }
        }
        Ok(Self { providers })
    }

    /// Load all built-in OS and vendor providers.
    pub fn builtin() -> Result<Self, BatteryError> {
        let mut providers = crate::providers::os::providers();
        providers.extend(crate::providers::vendors::providers());
        Self::new(providers)
    }

    /// Load built-in providers plus the synthetic test provider for replay and tests.
    pub fn with_synthetic() -> Result<Self, BatteryError> {
        let mut providers = Self::builtin()?.providers;
        providers.push(Arc::new(SyntheticFeatureMouseProvider));
        Self::new(providers)
    }

    /// Return all registered providers.
    #[must_use]
    pub fn providers(&self) -> &[Arc<dyn Provider>] {
        &self.providers
    }

    /// Find a provider by stable ID.
    #[must_use]
    pub fn find(&self, id: &str) -> Option<Arc<dyn Provider>> {
        self.providers
            .iter()
            .find(|provider| provider.id() == id)
            .cloned()
    }

    pub(crate) fn matches(
        &self,
        endpoint: &crate::model::Endpoint,
    ) -> Vec<(Arc<dyn Provider>, MatchDecision)> {
        self.providers
            .iter()
            .filter_map(|provider| {
                let decision = provider.matches(endpoint);
                (decision != MatchDecision::NoMatch).then(|| (Arc::clone(provider), decision))
            })
            .collect()
    }
}

/// Fictional HID Feature-report provider used exclusively by synthetic replay and client tests.
struct SyntheticFeatureMouseProvider;

impl Provider for SyntheticFeatureMouseProvider {
    fn id(&self) -> &str {
        "synthetic.feature.mouse"
    }

    fn metadata(&self) -> ProviderMetadata {
        ProviderMetadata {
            version: "1.0.0".to_owned(),
            device_kind: DeviceKind::Mouse,
            support_status: SupportStatus::Experimental,
            capabilities: Capabilities {
                percentage: true,
                charge_state: true,
                ..Capabilities::default()
            },
            io_timeout: Duration::from_millis(200),
            cooldown: Duration::ZERO,
        }
    }

    fn display_name(&self, _endpoint: &Endpoint) -> Option<String> {
        Some("Synthetic Feature Mouse".to_owned())
    }

    fn matches(&self, endpoint: &Endpoint) -> MatchDecision {
        if (endpoint.transport == TransportKind::Hid || endpoint.transport == TransportKind::Mock)
            && endpoint.vid == Some(0x1234)
            && endpoint.pid == Some(0x5678)
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
        deadline.check("synthetic query")?;
        session
            .send_feature(0x09, &[0x01, 0x00, 0x00, 0x00])
            .map_err(|error| error.into_battery(&device.endpoint.id))?;
        deadline.check("synthetic query round 1")?;
        let report = session
            .get_feature(0x09, 4)
            .map_err(|error| error.into_battery(&device.endpoint.id))?;
        if report.len() != 4 {
            return Err(BatteryError::invalid_response(format!(
                "response length mismatch: expected 4, got {}",
                report.len()
            )));
        }
        if report[0] != 0x01 {
            return Err(BatteryError::invalid_response(
                "constant mismatch at offset 0",
            ));
        }
        let expected_sum = report[0].wrapping_add(report[1]).wrapping_add(report[2]);
        if report[3] != expected_sum {
            return Err(BatteryError::ChecksumFailed);
        }
        let percentage = match report[1] {
            255 => None,
            value if value <= 100 => Some(value),
            _ => {
                return Err(BatteryError::invalid_response(
                    "percentage out of valid_range",
                ))
            }
        };
        let charge_state = if (report[2] & 1) != 0 {
            ChargeState::Charging
        } else {
            ChargeState::Discharging
        };
        Ok(ProviderReading {
            percentage,
            charge_state,
            source: ReadingSource::VendorHid,
            ..ProviderReading::default()
        })
    }
}
