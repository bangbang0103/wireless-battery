# Windows Native Shell

> **Language / 语言**: **English** | [简体中文](README.zh-CN.md)

A native Windows system tray application implemented in pure Rust + the `windows` crate (Win32 + DWM Transient Acrylic + Direct2D/DirectWrite):

- **Zero WebView / zero GPU framework**: no extra third-party GUI dependencies; Direct2D render targets are released when the popover is hidden to keep resident memory minimal.
- **Minimal logo tray icon**: `Shell_NotifyIconW` renders a supersampled 32×32 ARGB logo icon (without numeric overlays), adapts to light/dark taskbar themes, overlays a subtle red badge when any device is low on battery, shows a multi-line summary tooltip, and restores automatically on Explorer restart.
- **Native Fluent styling**: Windows 11 `DWMSBT_TRANSIENTWINDOW` acrylic backdrop, rounded corners, Per-Monitor DPI v2 scaling, live light/dark theme tracking, and click-outside dismissal.
- **Full interactive loop**: clean primary device list without engineering badges; clicking a device card expands an inline protocol/evidence/bus/freshness drawer with single-device re-read and sanitized diagnostic JSON copy; includes an embedded preferences and safety policy view with `HKCU\...\Run` autostart integration.

## Local build & run

```bash
cd tray
cargo build --locked --release -p wireless-battery-tray
.\target\release\wireless-battery-tray.exe
```

To enable optional BLE BAS support:

```bash
cd tray
cargo build --locked --release -p wireless-battery-tray --features bluetooth
```

---

Related docs: [Tray README](../../README.md) · [Platform support](../../../docs/platform-support.md) · [Back to root README](../../../README.md)
