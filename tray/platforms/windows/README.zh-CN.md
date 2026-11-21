# Windows 原生壳

> **Language / 语言**: [English](README.md) | **简体中文**

基于纯 Rust + `windows` crate（Win32 + DWM Transient Acrylic + Direct2D/DirectWrite）实现的 Windows 原生系统托盘外壳：

- **零 WebView / 零 GPU 引擎**：零新增第三方 GUI 依赖，隐藏浮层即释放 Direct2D 画布，保持极低常驻内存；
- **纯 Logo 托盘**：`Shell_NotifyIconW` 内存超采样生成 32×32 ARGB 纯 Logo 图标（无数值），自适应系统任务栏深浅色，存在低电时在右上角叠加微红角标，悬停展示多行设备摘要，支持 Explorer 重启自动恢复；
- **原生 Fluent 质感**：Win11 `DWMSBT_TRANSIENTWINDOW` 亚克力毛玻璃 + 原生圆角 + Per-Monitor DPI v2 + 系统深浅色实时监听 + 失焦自动收起；
- **完整交互闭环**：主列表不展示 `Exp` 等工程角标；点击设备卡片原地展开协议/证据/总线/新鲜度抽屉，支持单设备重读与复制脱敏诊断 JSON；内嵌偏好与安全策略页并联动 `HKCU\...\Run` 开机自启。

## 本地构建与运行

```bash
cd tray
cargo build --locked --release -p wireless-battery-tray
.\target\release\wireless-battery-tray.exe
```

启用可选 BLE BAS 支持：

```bash
cd tray
cargo build --locked --release -p wireless-battery-tray --features bluetooth
```

---

相关文档：[托盘说明文档](../../README.zh-CN.md) · [平台支持](../../../docs/platform-support.zh-CN.md) · [返回根 README](../../../README.zh-CN.md)
