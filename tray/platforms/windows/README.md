# Windows Native Shell

> **Language / 语言**: **English** | [简体中文](README.zh-CN.md)

A native Windows system tray application implemented in pure Rust + the `windows` crate (Win32 + DWM Transient Acrylic + Direct2D/DirectWrite):

- **Zero WebView / zero GPU framework**: no extra third-party GUI dependencies; modular Direct2D/DirectWrite UI (`src/renderer/{mod.rs, anim.rs, widgets.rs, main_view.rs, settings_view.rs}`) reuses a single cached `ID2D1SolidColorBrush` (`PaintBrush`) on the active `ID2D1HwndRenderTarget` to avoid per-frame brush allocations and rebuilds cleanly on device loss (`D2DERR_RECREATE_TARGET`).
- **Minimal vector tray icon**: `Shell_NotifyIconW` renders a Direct2D vector lightning + wireless broadcast wave icon at the OS small-icon metric (`GetSystemMetrics(SM_CXSMICON)`, clamped 16–64 px with 32 px fallback, no numeric overlays), adapts to light/dark taskbar themes, highlights the broadcast waves red when any device is low on battery, shows a multi-line localized summary tooltip, and restores automatically on Explorer restart.
- **Native Fluent styling**: Windows 11 `DWMSBT_TRANSIENTWINDOW` acrylic backdrop, rounded corners, Per-Monitor DPI v2 scaling, live light/dark theme tracking (`system` / `light` / `dark`), bilingual UI (`system` / `zh_cn` / `en`), and click-outside dismissal.
- **Full interactive loop**: clean primary device list powered directly by core `DeviceKind` metadata; clicking a device card expands an inline protocol/bus/status drawer with single-device re-read and sanitized diagnostic JSON copy; includes an embedded preferences and safety policy view (appearance, language, poll interval, low-battery & silent notifications, `allow_experimental` toggle, `HKCU\...\Run` autostart, and update check; `allow_stale` remains configurable via `tray.json`) plus Inno Setup `AppMutex` upgrade handling and registry/`PATH` cleanup on uninstall.

## Local build & run

```bash
cd tray
cargo build --locked --release -p wireless-battery-tray
.\target\release\wireless-battery-tray.exe
```

BLE BAS support (`bluetooth` feature) is enabled by default. To build without BLE support:

```bash
cd tray
cargo build --locked --release --no-default-features -p wireless-battery-tray
```

---

Related docs: [Tray README](../../README.md) · [Platform support](../../../docs/platform-support.md) · [Back to root README](../../../README.md)
