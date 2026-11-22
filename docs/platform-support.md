# Platform support

> **Language / 语言**: **English** | [简体中文](platform-support.zh-CN.md)

## Build matrix

CI is configured for Windows and macOS using Rust 1.76. It runs formatting, Clippy (all-features and `--no-default-features`), all-feature tests (including synthetic and vendor replay), and release binary/tray builds. Configuration is not a claim that every platform has completed real-hardware verification.

Linux HIDAPI builds may need a C compiler, `pkg-config`, `libudev-dev`, and `libusb-1.0-0-dev`, depending on the selected HIDAPI backend. BLE uses BlueZ through `btleplug`; no arbitrary Bluetooth command interface is exposed.

## Windows

Windows can reserve keyboard, mouse, and system HID collections. Vendor usage-page endpoints are usually the appropriate battery interface. `PermissionDenied`, `DeviceBusy`, and `DeviceDisconnected` are kept distinct where HIDAPI provides enough information.

## Linux

HIDRAW access may require a narrowly scoped udev rule or group membership. Do not run the whole library as root as the default solution. BlueZ adapter and D-Bus permissions can affect BLE enumeration and reads.

## macOS

HID and Bluetooth access can trigger operating-system permission requirements. Grant only the permissions needed by the host application. BLE behavior depends on CoreBluetooth and whether the peripheral is already known or connected.

## BLE BAS limitations

The `bluetooth` feature (enabled by default; disable with `--no-default-features`) lists only peripherals already known by OS adapters; it never starts a scan in `list_devices()`. Consequently, a BAS-capable device may be absent until the OS knows it. An explicit read may connect and perform GATT service discovery. A device connected through another transport or occupied by another central can be unavailable.

Only BAS Battery Level percentage is returned. Charging remains unknown unless a future separately evidenced source supports it. Classic Bluetooth/HFP and OS-specific cached Bluetooth properties are not implemented.

## Timeout and report-length limitations

The total deadline covers lock wait, cooldown, protocol steps, and processing. Input reads use the smaller of the provider's I/O timeout and remaining total budget. Some HID Feature operations can block in the OS and cannot be safely cancelled; the library documents rather than hides this limitation.

HIDAPI 2.6 does not expose input/output/feature report lengths in `DeviceInfo`. Those endpoint fields can therefore be `null`, and providers requiring them may not match on real enumeration even though synthetic replay can supply them. `interface_number` and `bus_type` are populated cross-platform when supplied by HIDAPI, and unnumbered HID reports (Report ID 0) are detected from the device's report descriptor when available.

## Real-hardware checklist

For each claimed platform, verify normal read, charging/not-charging where supported, permission failure behavior, disconnect/reconnect, repeated reads and cooldown, sleep/offline semantics if claimed, and BLE coexistence if applicable. Record evidence without serial numbers or raw paths.

## Vendor & OS transport notes

Built-in vendor providers preserve WriteFile, SetFeature, and SetOutputReport as distinct operations. SetOutputReport currently has a small Windows-native implementation (opened lazily on first use) because HIDAPI 2.6 does not expose it; record and replay use the same explicit operation. These device queries are experimental on other desktop platforms unless separately verified.

XInput uses runtime loading of `XInput1_4.dll` (with `XInput1_3.dll` fallback) restricted to System32 via `LoadLibraryExW`, so no XInput import library is required. It is Windows-only and returns four discrete levels without percentage conversion. Real-hardware replay fixtures (such as WOB Rainy75 and Logitech HID++) are stored under [`tests/fixtures/vendors/`](../tests/fixtures/vendors/).

---

Related docs: [Architecture](architecture.md) · [Support matrix](support-matrix.md) · [Capture and replay](capture-and-replay.md) · [Back to README](../README.md)
