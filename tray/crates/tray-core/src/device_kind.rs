//! Authoritative peripheral category access for tray UI iconography.

use wireless_battery::Device;
pub use wireless_battery::DeviceKind;

/// Return the device category declared by the core provider metadata.
#[must_use]
pub fn classify_device(device: &Device) -> DeviceKind {
    device.device_kind
}

#[cfg(test)]
mod tests {
    use super::*;
    use wireless_battery::{
        Capabilities, DeviceId, Endpoint, MatchStatus, SupportStatus, TransportKind,
    };

    fn fake_device(name: &str, kind: DeviceKind) -> Device {
        let endpoint = Endpoint::synthetic("ep", 0x046D, 0xC547);
        Device {
            id: DeviceId("dev".to_owned()),
            endpoint_id: endpoint.id.clone(),
            endpoint,
            name: name.to_owned(),
            vendor: None,
            model: None,
            provider_id: Some("test.provider".to_owned()),
            provider_version: None,
            support_status: SupportStatus::Verified,
            match_status: MatchStatus::Exact,
            device_kind: kind,
            capabilities: Capabilities::default(),
            readable: true,
        }
    }

    #[test]
    fn returns_core_declared_device_kind() {
        assert_eq!(
            classify_device(&fake_device("Logitech GPW3", DeviceKind::Mouse)),
            DeviceKind::Mouse
        );
        assert_eq!(
            classify_device(&fake_device("WOB Rainy75", DeviceKind::Keyboard)),
            DeviceKind::Keyboard
        );
        assert_eq!(
            classify_device(&fake_device("HyperX Cloud Alpha 2", DeviceKind::Headset)),
            DeviceKind::Headset
        );
        assert_eq!(
            classify_device(&fake_device("DualSense", DeviceKind::Gamepad)),
            DeviceKind::Gamepad
        );
        let mut unknown = fake_device("Unknown", DeviceKind::Peripheral);
        unknown.endpoint.transport = TransportKind::Hid;
        assert_eq!(classify_device(&unknown), DeviceKind::Peripheral);
    }
}
