//! Multi-tier low-battery notification deduplication (15%, 10%, 5%, 1%) with hysteresis.

use crate::state::{DeviceView, LowBatteryAlert};
use std::collections::{HashMap, HashSet};
use wireless_battery::{ChargeState, Freshness, Precision};

/// Hysteresis margin above the top alert threshold before a device's cycle resets.
const RESET_HYSTERESIS_PCT: u8 = 5;

/// Notification percentage milestones in descending order: 15%, 10%, 5%, 1%.
pub const LOW_BATTERY_TIERS: [u8; 4] = [15, 10, 5, 1];

/// Tracks the lowest notified milestone per device in the current discharge cycle so
/// users are alerted once at each of 15%, 10%, 5%, and 1% without spam on every poll.
#[derive(Debug, Default)]
pub struct LowBatteryTracker {
    last_alerted_tier: HashMap<String, u8>,
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
        self.last_alerted_tier
            .retain(|id, _| present.contains(id.as_str()));

        let mut alerts = Vec::new();
        for device in devices {
            let is_charging = matches!(
                device.charge_state,
                ChargeState::Charging | ChargeState::Full
            );

            // Reset discharge cycle when charging or recovered past top threshold + hysteresis.
            if is_charging {
                self.last_alerted_tier.remove(&device.id);
                continue;
            }
            if device.precision != Precision::Percentage {
                continue;
            }
            let Some(percentage) = device.percentage else {
                continue;
            };
            if percentage > threshold.saturating_add(RESET_HYSTERESIS_PCT) {
                self.last_alerted_tier.remove(&device.id);
                continue;
            }

            // Only alert on live readings to avoid alerting repeatedly on stale sleep values.
            if !enabled || device.freshness != Some(Freshness::Live) {
                continue;
            }

            let Some(tier) = matched_tier(percentage, threshold) else {
                continue;
            };

            let should_alert = match self.last_alerted_tier.get(&device.id).copied() {
                None => true,
                Some(prev_tier) => tier < prev_tier,
            };

            if should_alert {
                self.last_alerted_tier.insert(device.id.clone(), tier);
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

/// Find the lowest active tier (`15`, `10`, `5`, `1`) reached by `percentage` within `threshold`.
fn matched_tier(percentage: u8, threshold: u8) -> Option<u8> {
    if percentage > threshold {
        return None;
    }
    for &tier in LOW_BATTERY_TIERS.iter().rev() {
        if tier <= threshold && percentage <= tier {
            return Some(tier);
        }
    }
    // If custom threshold > 15 (e.g. 20%) and 16..=20%, treat threshold itself as the top tier.
    Some(threshold)
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
    fn alerts_once_at_each_tier_15_10_5_1_with_hysteresis() {
        let mut tracker = LowBatteryTracker::default();

        // 15% -> alerts tier 15.
        let a15 = tracker.evaluate(
            &[make_view(
                "d1",
                15,
                ChargeState::Discharging,
                Freshness::Live,
            )],
            true,
            15,
        );
        assert_eq!(a15.len(), 1);
        assert_eq!(a15[0].percentage, 15);

        // 14%, 11% -> still inside tier 15, suppressed.
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
        assert!(tracker
            .evaluate(
                &[make_view(
                    "d1",
                    11,
                    ChargeState::Discharging,
                    Freshness::Live
                )],
                true,
                15,
            )
            .is_empty());

        // 10% -> alerts tier 10.
        let a10 = tracker.evaluate(
            &[make_view(
                "d1",
                10,
                ChargeState::Discharging,
                Freshness::Live,
            )],
            true,
            15,
        );
        assert_eq!(a10.len(), 1);
        assert_eq!(a10[0].percentage, 10);

        // Bounce to 12% then back to 10% -> suppressed (no duplicate spam).
        assert!(tracker
            .evaluate(
                &[make_view(
                    "d1",
                    12,
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
                    10,
                    ChargeState::Discharging,
                    Freshness::Live
                )],
                true,
                15,
            )
            .is_empty());

        // 5% -> alerts tier 5.
        let a5 = tracker.evaluate(
            &[make_view(
                "d1",
                5,
                ChargeState::Discharging,
                Freshness::Live,
            )],
            true,
            15,
        );
        assert_eq!(a5.len(), 1);
        assert_eq!(a5[0].percentage, 5);

        // 2% -> still tier 5, suppressed.
        assert!(tracker
            .evaluate(
                &[make_view(
                    "d1",
                    2,
                    ChargeState::Discharging,
                    Freshness::Live
                )],
                true,
                15,
            )
            .is_empty());

        // 1% -> alerts tier 1.
        let a1 = tracker.evaluate(
            &[make_view(
                "d1",
                1,
                ChargeState::Discharging,
                Freshness::Live,
            )],
            true,
            15,
        );
        assert_eq!(a1.len(), 1);
        assert_eq!(a1[0].percentage, 1);

        // Repeat 1% -> suppressed.
        assert!(tracker
            .evaluate(
                &[make_view(
                    "d1",
                    1,
                    ChargeState::Discharging,
                    Freshness::Live
                )],
                true,
                15,
            )
            .is_empty());

        // Charging resets the cycle; discharging back to 15% alerts again.
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
                15,
                ChargeState::Discharging,
                Freshness::Live,
            )],
            true,
            15,
        );
        assert_eq!(after_charge.len(), 1);
    }

    #[test]
    fn jumping_directly_to_lower_tier_emits_single_alert_and_still_alerts_lower_milestones() {
        let mut tracker = LowBatteryTracker::default();

        // First seen at 8% (tier 10) -> alerts once.
        let first = tracker.evaluate(
            &[make_view(
                "d1",
                8,
                ChargeState::Discharging,
                Freshness::Live,
            )],
            true,
            15,
        );
        assert_eq!(first.len(), 1);
        assert_eq!(first[0].percentage, 8);

        // Next at 5% -> alerts tier 5.
        let second = tracker.evaluate(
            &[make_view(
                "d1",
                5,
                ChargeState::Discharging,
                Freshness::Live,
            )],
            true,
            15,
        );
        assert_eq!(second.len(), 1);
        assert_eq!(second[0].percentage, 5);
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
