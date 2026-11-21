//! Low-battery notification deduplication with hysteresis.

use crate::state::{DeviceView, LowBatteryAlert};
use std::collections::HashSet;
use wireless_battery::{ChargeState, Freshness, Precision};

/// Hysteresis margin above threshold before a device is eligible to re-alert.
const RESET_HYSTERESIS_PCT: u8 = 5;

/// Tracks which devices have already emitted a low-battery notification in the current
/// discharge cycle so users are never spammed on every polling tick.
#[derive(Debug, Default)]
pub struct LowBatteryTracker {
    alerted: HashSet<String>,
}

impl LowBatteryTracker {
    /// Evaluate current devices and return newly triggered low-battery alerts.
    pub fn evaluate(
        &mut self,
        devices: &[DeviceView],
        enabled: bool,
        threshold: u8,
    ) -> Vec<LowBatteryAlert> {
        let present: HashSet<&str> = devices.iter().map(|d| d.id.as_str()).collect();
        self.alerted.retain(|id| present.contains(id.as_str()));

        let mut alerts = Vec::new();
        for device in devices {
            let is_charging = matches!(
                device.charge_state,
                ChargeState::Charging | ChargeState::Full
            );

            // Reset when charging or recovered past threshold + hysteresis.
            if is_charging {
                self.alerted.remove(&device.id);
                continue;
            }
            if device.precision != Precision::Percentage {
                continue;
            }
            let Some(percentage) = device.percentage else {
                continue;
            };
            if percentage > threshold.saturating_add(RESET_HYSTERESIS_PCT) {
                self.alerted.remove(&device.id);
                continue;
            }

            // Only alert on live readings to avoid alerting repeatedly on stale sleep values.
            if !enabled || device.freshness != Some(Freshness::Live) {
                continue;
            }

            if percentage <= threshold && self.alerted.insert(device.id.clone()) {
                alerts.push(LowBatteryAlert {
                    device_id: device.id.clone(),
                    device_name: device.name.clone(),
                    percentage,
                });
            }
        }

        alerts
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::device_kind::DeviceKind;
    use wireless_battery::{Reachability, SupportStatus, TransportKind};

    fn make_view(id: &str, pct: u8, charge: ChargeState, freshness: Freshness) -> DeviceView {
        DeviceView {
            id: id.to_owned(),
            name: "GPW3".to_owned(),
            kind: DeviceKind::Mouse,
            transport: TransportKind::Hid,
            transport_label: "HID".to_owned(),
            provider_id: Some("logitech.hidpp".to_owned()),
            support_status: SupportStatus::Verified,
            precision: Precision::Percentage,
            percentage: Some(pct),
            level: None,
            voltage_mv: None,
            charge_state: charge,
            reachability: Reachability::Online,
            freshness: Some(freshness),
            observed_at: None,
            last_error: None,
            is_low: pct <= 15,
        }
    }

    #[test]
    fn alerts_once_per_discharge_cycle_with_hysteresis() {
        let mut tracker = LowBatteryTracker::default();

        // First live reading at 14% -> alerts once.
        let first = tracker.evaluate(
            &[make_view(
                "d1",
                14,
                ChargeState::Discharging,
                Freshness::Live,
            )],
            true,
            15,
        );
        assert_eq!(first.len(), 1);
        assert_eq!(first[0].percentage, 14);

        // Next poll at 12% -> suppressed.
        let second = tracker.evaluate(
            &[make_view(
                "d1",
                12,
                ChargeState::Discharging,
                Freshness::Live,
            )],
            true,
            15,
        );
        assert!(second.is_empty());

        // Minor fluctuation to 17% (within hysteresis 15 + 5) -> still suppressed.
        assert!(tracker
            .evaluate(
                &[make_view(
                    "d1",
                    17,
                    ChargeState::Discharging,
                    Freshness::Live
                )],
                true,
                15,
            )
            .is_empty());
        assert!(tracker
            .evaluate(
                &[make_view(
                    "d1",
                    14,
                    ChargeState::Discharging,
                    Freshness::Live
                )],
                true,
                15,
            )
            .is_empty());

        // Charging resets the cycle; discharging back to 13% alerts again.
        assert!(tracker
            .evaluate(
                &[make_view("d1", 16, ChargeState::Charging, Freshness::Live)],
                true,
                15,
            )
            .is_empty());
        let after_charge = tracker.evaluate(
            &[make_view(
                "d1",
                13,
                ChargeState::Discharging,
                Freshness::Live,
            )],
            true,
            15,
        );
        assert_eq!(after_charge.len(), 1);
    }

    #[test]
    fn stale_readings_never_trigger_alert() {
        let mut tracker = LowBatteryTracker::default();
        let alerts = tracker.evaluate(
            &[make_view(
                "d1",
                10,
                ChargeState::Discharging,
                Freshness::Stale,
            )],
            true,
            15,
        );
        assert!(alerts.is_empty());
    }
}
