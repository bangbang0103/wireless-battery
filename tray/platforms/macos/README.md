# macOS Native Shell

> **Language / 语言**: **English** | [简体中文](README.zh-CN.md)

A native macOS 13+ menu bar application built with **SwiftUI `MenuBarExtra(.window)`** and the `tray/crates/tray-ffi` static library (zero third-party Swift dependencies):

- **Native macOS look and feel**: modular SwiftUI views (`App.swift`, `MenuBarIcon.swift`, `FloatingPinController.swift`, `DeviceRowView.swift`, `SettingsView.swift`, `Bridge.swift`, `Localization.swift`), `.ultraThinMaterial` translucency, monochrome menu bar template icon (with subtle low-battery badge and no numeric overlay), SF Symbols hardware icons driven by core `DeviceKind`, SF Pro typography, automatic/manual light & dark appearance, and bilingual UI (`system` / `zh_cn` / `en`).
- **Full feature parity**: smart refresh on popover open, concurrent progressive readings, honest error states (timeout / busy / Input Monitoring permission guidance), expandable protocol/bus/status drawers, sanitized diagnostic JSON copy, Settings controls (appearance, language, poll interval, low-battery & silent `UNUserNotificationCenter` alerts, `allow_experimental` toggle, `SMAppService` login autostart, and update check; `allow_stale` remains configurable via `tray.json`).
- **App permissions & background mode**: stays quietly in the menu bar (`LSUIElement = true`, `.accessory` activation policy) and declares `NSBluetoothAlwaysUsageDescription` in `Resources/Info.plist` for macOS 13+ CoreBluetooth TCC compliance.

## Local `.app` bundle build (macOS)

```bash
chmod +x tray/platforms/macos/scripts/bundle-app.sh
./tray/platforms/macos/scripts/bundle-app.sh
open "tray/platforms/macos/dist/Wireless Battery.app"
```

BLE BAS support (`bluetooth` feature) is enabled by default. To build without BLE support:

```bash
./tray/platforms/macos/scripts/bundle-app.sh --no-default-features
```

To enable mock/developer test controls (`dev-tools` feature, synchronized automatically between Cargo and Swift `WB_DEV_TOOLS=1`):

```bash
./tray/platforms/macos/scripts/bundle-app.sh --features dev-tools
```

---

Related docs: [Tray README](../../README.md) · [Platform support](../../../docs/platform-support.md) · [Back to root README](../../../README.md)
