# Wireless Battery Tray

> **Language / 语言**: **English** | [简体中文](README.zh-CN.md)

A native system tray workspace isolated from the root library and CLI, built on a **shared Rust core (`tray-core`) + thin OS-native UI shells** architecture:

- native OS look and feel (acrylic/translucency, typography, dark/light mode, and system tray ergonomics);
- minimal dependencies (no WebView, no GPU rendering framework) and low resident memory usage;
- keeps the root `wireless-battery` crate (lib / cli) free of GUI or windowing dependencies.

## Directory layout

```text
tray/
├── Cargo.toml                 # Separate tray workspace to isolate GUI deps and MSRV
├── Cargo.lock                 # Separate lockfile
├── assets/                    # App icons and vector tray templates
│   ├── app-icon.png           # 1024×1024 transparent-background app icon
│   ├── app-icon.ico           # Windows multi-resolution icon (16–256 px, embedded in EXE)
│   ├── logo.svg
│   ├── tray-normal.svg        # Monochrome tray template source (no numeric overlay)
│   └── tray-low.svg           # Low-battery red-wave tray icon source
├── crates/
│   ├── tray-core/             # Platform-agnostic state machine, polling, hysteresis, config, and sanitized diagnostics (#![deny(unsafe_code)])
│   └── tray-ffi/              # Thin C-ABI static library for non-Rust native shells (e.g., macOS SwiftUI)
└── platforms/
    ├── windows/               # Windows native shell (pure Rust: Win32 + DWM Acrylic + Direct2D/DirectWrite)
    ├── macos/                 # macOS native shell (SwiftUI MenuBarExtra + tray-ffi staticlib)
    └── linux/                 # Linux native shell (planned: GTK4 + Libadwaita / SNI)
```

## Build & test

Run from `tray/`:

```bash
cargo test --locked
cargo clippy --locked --all-targets -- -D warnings
cargo fmt --all -- --check
```

BLE BAS support (`bluetooth` feature) is enabled by default. To build without BLE support:

```bash
cargo test --locked --no-default-features -p wireless-battery-tray-core
```

## Platform shells

- [Windows Native Shell](platforms/windows/README.md)
- [macOS Native Shell](platforms/macos/README.md)
- [Linux Native Shell (Planned)](platforms/linux/README.md)

---

Related docs: [Architecture](../docs/architecture.md) · [Platform support](../docs/platform-support.md) · [Back to README](../README.md)
