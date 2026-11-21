# Linux 原生壳（预留）

> **Language / 语言**: [English](README.md) | **简体中文**

预留用于 Linux 原生托盘外壳：

- **方案方向**：GTK4 + Libadwaita 轻量弹窗，配合 StatusNotifierItem（SNI / D-Bus）托盘协议，原生贴合 GNOME / KDE 桌面风格；
- **共享核心**：直接依赖 `tray/crates/tray-core`（纯 Rust），复用同一套轮询、缓存与脱敏诊断逻辑。

---

相关文档：[托盘说明文档](../../README.zh-CN.md) · [平台支持](../../../docs/platform-support.zh-CN.md) · [返回根 README](../../../README.zh-CN.md)
