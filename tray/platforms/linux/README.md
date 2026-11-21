# Linux Native Shell (Planned)

> **Language / 语言**: **English** | [简体中文](README.zh-CN.md)

Reserved for the Linux native system tray shell:

- **Planned stack**: lightweight GTK4 + Libadwaita popover paired with StatusNotifierItem (SNI / D-Bus) tray integration to blend natively with GNOME and KDE desktops.
- **Shared core**: depends directly on `tray/crates/tray-core` (pure Rust) to reuse polling, caching, and sanitized diagnostic logic.

---

Related docs: [Tray README](../../README.md) · [Platform support](../../../docs/platform-support.md) · [Back to root README](../../../README.md)
