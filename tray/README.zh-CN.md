# Wireless Battery Tray

> **Language / 语言**: [English](README.md) | **简体中文**

独立于根库与 CLI 的原生系统托盘工作区，采用 **共享 Rust 核心（`tray-core`） + 各平台极薄原生 UI 壳** 架构，以实现：

- 各操作系统原生一致的毛玻璃 / 字体 / 交互质感；
- 极轻依赖（零 WebView、零 GPU 渲染框架）与极低常驻内存；
- 根包 `wireless-battery`（lib / cli）完全不受 GUI 或系统窗口依赖污染。

## 目录结构

```text
tray/
├── Cargo.toml                 # 独立托盘工作区，避免污染根包依赖与 MSRV
├── Cargo.lock                 # 独立锁文件
├── assets/                    # 应用主图标与托盘纯 Logo 矢量源图
│   ├── app-icon.png           # 1024×1024 透明背景主应用图标（圆角闪电与无线波纹设计）
│   ├── app-icon.ico           # Windows 多分辨率应用图标（16–256 px，构建时自动嵌入 EXE）
│   ├── logo.svg
│   ├── tray-normal.svg        # 静默单色模板源图（无数值）
│   └── tray-low.svg           # 低电红色波纹托盘图标源图
├── crates/
│   ├── tray-core/             # 平台无关状态机、轮询、低电防抖、配置与脱敏诊断（#![deny(unsafe_code)]）
│   └── tray-ffi/              # 极薄 C-ABI 静态库，供 macOS SwiftUI 等非 Rust 原生壳调用
└── platforms/
    ├── windows/               # Windows 原生壳（纯 Rust：Win32 + DWM Acrylic + Direct2D/DirectWrite）
    ├── macos/                 # macOS 原生壳（AppKit NSStatusItem + SwiftUI 弹层 + tray-ffi staticlib）
    └── linux/                 # Linux 原生壳（预留：GTK4 + Libadwaita / SNI）
```

## 构建与测试

在 `tray/` 目录下运行：

```bash
cargo test --locked
cargo clippy --locked --all-targets -- -D warnings
cargo fmt --all -- --check
```

BLE BAS 读取能力（`bluetooth` feature）默认已启用。若需要禁用 BLE 支持进行测试：

```bash
cargo test --locked --no-default-features -p wireless-battery-tray-core
```

## 平台原生壳

- [Windows 原生壳](platforms/windows/README.zh-CN.md)
- [macOS 原生壳](platforms/macos/README.zh-CN.md)
- [Linux 原生壳（预留）](platforms/linux/README.zh-CN.md)

---

相关文档：[架构设计](../docs/architecture.zh-CN.md) · [平台支持](../docs/platform-support.zh-CN.md) · [返回 README](../README.zh-CN.md)
