# Windows 原生壳

> **Language / 语言**: [English](README.md) | **简体中文**

基于纯 Rust + `windows` crate（Win32 + DWM Transient Acrylic + Direct2D/DirectWrite）实现的 Windows 原生系统托盘外壳：

- **零 WebView / 零 GPU 引擎**：零新增第三方 GUI 依赖；模块化 Direct2D/DirectWrite UI（`src/renderer/{mod.rs, anim.rs, widgets.rs, main_view.rs, settings_view.rs}`）在活跃 `ID2D1HwndRenderTarget` 上复用单个缓存画刷 `PaintBrush`（`ID2D1SolidColorBrush`），避免逐帧创建 COM 画刷对象，并在遇到 `D2DERR_RECREATE_TARGET` 时自动安全重建；
- **纯矢量托盘图标**：`Shell_NotifyIconW` 按系统小图标尺寸（`GetSystemMetrics(SM_CXSMICON)`，钳位 16–64 px，回退 32 px）直接用 Direct2D 绘制闪电与无线电波图标（无数值），自适应系统任务栏深浅色，存在低电设备时将无线电波高亮为红色，悬停展示本地化多行摘要，支持 Explorer 重启自动恢复；
- **原生 Fluent 质感**：Win11 `DWMSBT_TRANSIENTWINDOW` 亚克力毛玻璃 + 原生圆角 + Per-Monitor DPI v2 + 系统深浅色实时监听（支持跟随系统 / 浅色 / 深色）+ 中英双语界面（支持跟随系统 / 简体中文 / English）+ 失焦自动收起；
- **完整交互闭环**：直接读取核心库 `DeviceKind` 展示外设品类，主列表不展示 `Exp` 等工程角标；点击设备卡片原地展开协议/总线/状态（实时或缓存）抽屉，支持单设备重读与复制脱敏诊断 JSON；内嵌偏好与安全策略页（支持外观主题、语言、轮询频率、低电与静默通知、实验性设备开关 `allow_experimental`、`HKCU\...\Run` 开机自启与更新检查；`allow_stale` 可通过 `tray.json` 配置），安装器已配置 `AppMutex` 并支持卸载时清理自启注册表与 `PATH`。

## 本地构建与运行

```bash
cd tray
cargo build --locked --release -p wireless-battery-tray
.\target\release\wireless-battery-tray.exe
```

BLE BAS 支持（`bluetooth` feature）默认已启用。若需要禁用 BLE 支持：

```bash
cd tray
cargo build --locked --release --no-default-features -p wireless-battery-tray
```

---

相关文档：[托盘说明文档](../../README.zh-CN.md) · [平台支持](../../../docs/platform-support.zh-CN.md) · [返回根 README](../../../README.zh-CN.md)
