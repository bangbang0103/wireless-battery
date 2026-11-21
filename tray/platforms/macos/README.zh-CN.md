# macOS 原生壳

> **Language / 语言**: [English](README.md) | **简体中文**

基于 **SwiftUI `MenuBarExtra(.window)`** + `tray/crates/tray-ffi` 静态库实现的 macOS 13+ 原生菜单栏应用（零第三方 Swift 依赖）：

- **100% 苹果原生质感**：原生 `.ultraThinMaterial` 毛玻璃、纯单色菜单栏图标（低电微红角标，无数值）、SF Symbols 硬件图标、SF Pro 字体与深浅色自适应；
- **完整体验对齐**：支持打开浮层智能补刷、并发渐进读数、诚实错误状态（超时/正忙/输入监控权限提示）、点击展开协议与证据抽屉、脱敏诊断 JSON 复制、`UNUserNotificationCenter` 低电通知与 `SMAppService` 登录自启；
- **无 Dock 图标**：通过 `Resources/Info.plist`（`LSUIElement = true`）与 `NSApplication.setActivationPolicy(.accessory)` 保持纯菜单栏静默常驻。

## 本地一键构建 `.app`（macOS）

```bash
chmod +x tray/platforms/macos/scripts/bundle-app.sh
./tray/platforms/macos/scripts/bundle-app.sh
open "tray/platforms/macos/dist/Wireless Battery.app"
```

启用可选 BLE BAS 支持：

```bash
./tray/platforms/macos/scripts/bundle-app.sh --features bluetooth
```

---

相关文档：[托盘说明文档](../../README.zh-CN.md) · [平台支持](../../../docs/platform-support.zh-CN.md) · [返回根 README](../../../README.zh-CN.md)
