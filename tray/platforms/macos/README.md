# macOS Native Shell

> **Language / 语言**: **English** | [简体中文](README.zh-CN.md)

A native macOS 13+ menu bar application built with **SwiftUI `MenuBarExtra(.window)`** and the `tray/crates/tray-ffi` static library (zero third-party Swift dependencies):

- **Native macOS look and feel**: `.ultraThinMaterial` translucency, monochrome menu bar template icon (with subtle low-battery badge and no numeric overlay), SF Symbols hardware icons, SF Pro typography, automatic/manual light & dark appearance, and bilingual UI (`system` / `zh_cn` / `en`).
- **Full feature parity**: smart refresh on popover open, concurrent progressive readings, honest error states (timeout / busy / Input Monitoring permission guidance), expandable protocol and evidence drawers, sanitized diagnostic JSON copy, `UNUserNotificationCenter` low-battery alerts, and `SMAppService` login autostart.
- **No Dock icon**: stays quietly in the menu bar via `Resources/Info.plist` (`LSUIElement = true`) and `NSApplication.setActivationPolicy(.accessory)`.

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

---

Related docs: [Tray README](../../README.md) · [Platform support](../../../docs/platform-support.md) · [Back to root README](../../../README.md)
