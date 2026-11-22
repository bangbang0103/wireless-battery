# macOS 原生壳

> **Language / 语言**: [English](README.md) | **简体中文**

基于 **AppKit `NSStatusItem` / `NSPopover` + SwiftUI 内容视图** + `tray/crates/tray-ffi` 静态库实现的 macOS 13+ 原生菜单栏应用（零第三方 Swift 依赖）：

- **100% 苹果原生质感**：模块化 SwiftUI 视图结构（`App.swift`、`MenuBarIcon.swift`、`FloatingPinController.swift`、`DeviceRowView.swift`、`SettingsView.swift`、`Bridge.swift`、`Localization.swift`），原生 `.ultraThinMaterial` 毛玻璃、菜单栏闪电与无线波纹图标（平时单色，低电时波纹变红，无数值）、由核心 `DeviceKind` 驱动的 SF Symbols 硬件图标、SF Pro 字体、深浅色外观切换与中英双语界面（跟随系统 / 简体中文 / English）；
- **完整体验对齐**：支持打开浮层智能补刷、并发渐进读数、诚实错误状态（超时/正忙/输入监控权限提示）、点击展开协议/总线/状态抽屉、脱敏诊断 JSON 复制，以及设置页控制（外观、语言、轮询频率、`UNUserNotificationCenter` 低电与静默通知、`SMAppService` 登录自启与更新检查；`allow_stale` 可通过 `tray.json` 配置）；
- **无 Dock 图标与 TCC 合规**：通过 `Resources/Info.plist`（`LSUIElement = true`）与 `NSApplication.setActivationPolicy(.accessory)` 保持纯菜单栏静默常驻，并在 `Info.plist` 声明 `NSBluetoothAlwaysUsageDescription` 以满足 macOS 13+ CoreBluetooth 隐私权限要求。

## 本地一键构建 `.app`（macOS）

```bash
chmod +x tray/platforms/macos/scripts/bundle-app.sh
./tray/platforms/macos/scripts/bundle-app.sh
open "tray/platforms/macos/dist/Wireless Battery.app"
```

BLE BAS 支持（`bluetooth` feature）默认已启用。若需要禁用 BLE 支持：

```bash
./tray/platforms/macos/scripts/bundle-app.sh --no-default-features
```

如需启用开发调试面板与模拟设备命令（`dev-tools` feature，脚本会自动同步 Cargo 与 Swift 的 `WB_DEV_TOOLS=1`）：

```bash
./tray/platforms/macos/scripts/bundle-app.sh --features dev-tools
```

---

相关文档：[托盘说明文档](../../README.zh-CN.md) · [平台支持](../../../docs/platform-support.zh-CN.md) · [返回根 README](../../../README.zh-CN.md)
