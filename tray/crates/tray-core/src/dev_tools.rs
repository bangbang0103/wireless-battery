//! Development-only state injection and UI debugging helpers.
//!
//! Compiled strictly behind `#[cfg(feature = "dev-tools")]` so release builds
//! contain zero mock logic or debugging overhead.

use crate::device_kind::DeviceKind;
use crate::service::{device_sort_rank, TrayService};
use crate::state::{
    format_tooltip_with_locale, DeviceView, LowBatteryAlert, TrayIconState, TraySnapshot,
};
use chrono::Utc;
use wireless_battery::{
    ChargeState, ErrorKind, Freshness, Precision, Reachability, SupportStatus, TransportKind,
};

/// Specification for manually injecting a simulated peripheral into the tray snapshot.
#[derive(Debug, Clone)]
pub struct DevDeviceSpec {
    pub name: String,
    pub kind: DeviceKind,
    pub percentage: Option<u8>,
    pub charge_state: ChargeState,
    pub reachability: Reachability,
    pub freshness: Option<Freshness>,
    pub last_error: Option<ErrorKind>,
}

/// Dev-only state layered on top of real hardware readings inside `ServiceState`.
#[derive(Default)]
pub(crate) struct DevState {
    force_low_battery: bool,
    virtual_devices: Vec<DeviceView>,
    next_id: u32,
}

impl DevState {
    pub(crate) fn apply_to_snapshot(
        &self,
        snapshot: &mut TraySnapshot,
        threshold: u8,
        low_battery_notify: bool,
        zh: bool,
    ) {
        if !self.virtual_devices.is_empty() {
            for dev in &self.virtual_devices {
                let mut cloned = dev.clone();
                cloned.is_low = cloned.precision == Precision::Percentage
                    && cloned.percentage.is_some_and(|pct| pct <= threshold)
                    && !matches!(
                        cloned.charge_state,
                        ChargeState::Charging | ChargeState::Full
                    );
                snapshot.devices.push(cloned);
            }
        }

        if self.force_low_battery {
            if let Some(first) = snapshot.devices.first_mut() {
                if !first.is_low {
                    first.precision = Precision::Percentage;
                    first.percentage = Some(threshold.min(10));
                    first.charge_state = ChargeState::Discharging;
                    first.reachability = Reachability::Online;
                    first.freshness = Some(Freshness::Live);
                    first.observed_at = Some(Utc::now());
                    first.last_error = None;
                    first.is_low = true;
                }
            } else {
                snapshot.devices.push(DeviceView {
                    id: "dev:forced-low".to_owned(),
                    name: "Dev Low-Battery Mouse".to_owned(),
                    kind: DeviceKind::Mouse,
                    transport: TransportKind::Mock,
                    transport_label: "Mock".to_owned(),
                    provider_id: Some("dev.mock".to_owned()),
                    support_status: SupportStatus::Verified,
                    precision: Precision::Percentage,
                    percentage: Some(threshold.min(10)),
                    level: None,
                    voltage_mv: Some(3620),
                    charge_state: ChargeState::Discharging,
                    reachability: Reachability::Online,
                    freshness: Some(Freshness::Live),
                    observed_at: Some(Utc::now()),
                    last_error: None,
                    is_low: true,
                });
            }
        }

        if self.force_low_battery || !self.virtual_devices.is_empty() {
            snapshot.devices.sort_by(|a, b| {
                device_sort_rank(a).cmp(&device_sort_rank(b)).then_with(|| {
                    a.name
                        .to_ascii_lowercase()
                        .cmp(&b.name.to_ascii_lowercase())
                })
            });

            snapshot.icon_state = if snapshot.devices.is_empty() {
                TrayIconState::Idle
            } else if self.force_low_battery
                || (low_battery_notify && snapshot.devices.iter().any(|d| d.is_low))
            {
                TrayIconState::LowBattery
            } else {
                TrayIconState::Normal
            };
            snapshot.tooltip = format_tooltip_with_locale(&snapshot.devices, zh);
        }
    }
}

impl TrayService {
    /// Toggle forced low-battery UI state on/off and return `(forced, updated_snapshot)`.
    pub fn dev_toggle_low_battery(&self) -> (bool, TraySnapshot) {
        let mut guard = self.state.lock().unwrap_or_else(|e| e.into_inner());
        guard.dev.force_low_battery = !guard.dev.force_low_battery;
        let forced = guard.dev.force_low_battery;
        let snapshot = crate::service::build_snapshot(&guard);
        (forced, snapshot)
    }

    /// Return whether forced low-battery mode is currently active.
    #[must_use]
    pub fn dev_is_low_battery_forced(&self) -> bool {
        self.state
            .lock()
            .unwrap_or_else(|e| e.into_inner())
            .dev
            .force_low_battery
    }

    /// Construct a synthetic `LowBatteryAlert` for testing native OS notifications.
    #[must_use]
    pub fn dev_make_low_battery_alert(&self, percentage: u8) -> LowBatteryAlert {
        let snap = self.snapshot();
        if let Some(dev) = snap.devices.first() {
            LowBatteryAlert {
                device_id: dev.id.clone(),
                device_name: dev.name.clone(),
                percentage: percentage.clamp(1, 100),
            }
        } else {
            LowBatteryAlert {
                device_id: "dev:mock-alert".to_owned(),
                device_name: "Dev Mock Mouse".to_owned(),
                percentage: percentage.clamp(1, 100),
            }
        }
    }

    /// Manually append a simulated device with custom battery state to the snapshot.
    pub fn dev_add_device(&self, spec: DevDeviceSpec) -> TraySnapshot {
        let mut guard = self.state.lock().unwrap_or_else(|e| e.into_inner());
        guard.dev.next_id = guard.dev.next_id.saturating_add(1);
        let id = format!("dev:mock-{}", guard.dev.next_id);
        let threshold = guard.config.low_battery_threshold;
        let precision = if spec.percentage.is_some() {
            Precision::Percentage
        } else {
            Precision::None
        };
        let is_low = precision == Precision::Percentage
            && spec.percentage.is_some_and(|pct| pct <= threshold)
            && !matches!(spec.charge_state, ChargeState::Charging | ChargeState::Full);

        guard.dev.virtual_devices.push(DeviceView {
            id,
            name: spec.name,
            kind: spec.kind,
            transport: TransportKind::Mock,
            transport_label: "Mock".to_owned(),
            provider_id: Some("dev.mock".to_owned()),
            support_status: SupportStatus::Verified,
            precision,
            percentage: spec.percentage.map(|p| p.min(100)),
            level: None,
            voltage_mv: spec.percentage.map(|p| 3500 + u32::from(p) * 7),
            charge_state: spec.charge_state,
            reachability: spec.reachability,
            freshness: spec.freshness,
            observed_at: spec.freshness.map(|_| Utc::now()),
            last_error: spec.last_error,
            is_low,
        });
        guard.last_sync_at = Some(Utc::now());
        crate::service::build_snapshot(&guard)
    }

    /// Remove all simulated devices and exit forced low-battery state.
    pub fn dev_reset(&self) -> TraySnapshot {
        let mut guard = self.state.lock().unwrap_or_else(|e| e.into_inner());
        guard.dev.force_low_battery = false;
        guard.dev.virtual_devices.clear();
        crate::service::build_snapshot(&guard)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::config::TrayConfig;
    use std::sync::Arc;
    use std::time::Duration;
    use wireless_battery::registry::Registry;
    use wireless_battery::{
        BatteryClient, ClientConfig, Endpoint, EndpointId, HidSession, Transport, TransportError,
    };

    struct EmptyTransport;
    impl Transport for EmptyTransport {
        fn enumerate(&self) -> Result<Vec<Endpoint>, TransportError> {
            Ok(Vec::new())
        }
        fn open(&self, _endpoint: &EndpointId) -> Result<Box<dyn HidSession>, TransportError> {
            Err(TransportError::Timeout)
        }
    }

    fn make_service() -> TrayService {
        let client = BatteryClient::with_transport(
            ClientConfig {
                default_timeout: Duration::from_secs(1),
                allow_experimental: false,
                enable_cache: true,
            },
            Arc::new(EmptyTransport),
            Registry::new(Vec::new()).unwrap(),
        )
        .unwrap();
        TrayService::with_client(Arc::new(client), TrayConfig::default(), None)
    }

    #[test]
    fn dev_tools_toggle_low_and_add_clear_devices() {
        let service = make_service();
        assert_eq!(service.snapshot().icon_state, TrayIconState::Idle);

        let (forced, snap) = service.dev_toggle_low_battery();
        assert!(forced);
        assert_eq!(snap.icon_state, TrayIconState::LowBattery);
        assert_eq!(snap.devices.len(), 1);
        assert!(snap.devices[0].is_low);

        let snap = service.dev_add_device(DevDeviceSpec {
            name: "Dev Keyboard".to_owned(),
            kind: DeviceKind::Keyboard,
            percentage: Some(72),
            charge_state: ChargeState::Charging,
            reachability: Reachability::Online,
            freshness: Some(Freshness::Live),
            last_error: None,
        });
        assert_eq!(snap.devices.len(), 1);
        assert_eq!(snap.icon_state, TrayIconState::LowBattery);

        let snap = service.dev_reset();
        assert!(snap.devices.is_empty());
        assert_eq!(snap.icon_state, TrayIconState::Idle);
    }
}
