# Wireless Battery

> **Language / 语言**: **English** | [简体中文](README.zh-CN.md)

A synchronous Rust library and CLI for conservative peripheral battery readings over HID, with optional read-only Bluetooth LE Battery Service (BAS) support.

The project prioritizes protocol safety and reproducibility over device count:

- passive discovery never opens HID sessions, writes reports, or starts a Bluetooth scan;
- unknown values stay unknown—timeouts never become 0%, and voltage or discrete levels are not converted into fake percentages;
- every declarative protocol can be replayed from minimal JSON without hardware;
- synthetic protocols are excluded from real discovery and the support matrix;
- raw system paths, serial numbers, and HID bytes are absent from normal readings and logs.

## Current support

**152 built-in vendor and OS provider paths** (plus optional Bluetooth LE BAS) are included, covering Logitech / Astro, Razer, SteelSeries, Corsair, HyperX, ASUS ROG, Sony, Nintendo, Valve, GameSir, Flydigi, 8BitDo, Beitong, Keychron, Cherry, Akko, NuPhy, AULA, MCHOSE, ATK / VXE / VGN, DAREU, EWEADN, Rapoo, WOB, Glorious, Lamzu, WLmouse, Pulsar, VAXEE, Ninjutso, Endgame Gear, Finalmouse, ZOWIE, Audeze, JBL, Jabra, Turtle Beach, AKG, Fractal, Alienware, Moondrop, Maono, XInput, and others listed in the [support matrix](docs/support-matrix.md). They are experimental unless a specific device and capability are marked verified in the matrix. Strict wire-level synthetic replay exists for every built-in provider; this does not independently verify hardware. WOB Rainy75 and Logitech GPW3 real-hardware verification is tracked in the [hardware verification checklist](docs/hardware-verification.md). A fictional declarative mouse remains available for interpreter tests and is excluded from real support.

Built-in vendor protocol definitions reference [Wireless-Device-Battery-Monitor / BatteryHub](NOTICE) (GPL-3.0-or-later) and [Voltide](NOTICE) (MIT); see [NOTICE](NOTICE) for full attribution.

Standard HID battery usages and Classic Bluetooth/HFP battery reports are reserved for later versions. Optional BLE support implements only BAS Battery Level (`0x2A19`) reads, not pairing, notifications, charging state, or cross-transport device deduplication.

## Build

Rust 1.76 or newer is supported. `Cargo.lock` pins dependency versions to preserve that MSRV (`--locked`). The `cli` feature is enabled by default; library consumers can disable it with `--no-default-features` to avoid compiling CLI-only dependencies (`clap`, `walkdir`).

```bash
cargo build --locked
cargo build --locked --no-default-features
cargo test --locked --all-features
cargo run --locked -- validate
```

For optional BLE BAS support:

```bash
cargo build --locked --features bluetooth
```

The `bluetooth` feature is deliberately disabled by default. See [platform support](docs/platform-support.md) for system dependencies and permission limitations.

## Rust API

```rust
use wireless_battery::{BatteryClient, ClientConfig};

let client = BatteryClient::new(ClientConfig::default())?;
let devices = client.list_devices()?;
for device in &devices {
    println!("{}: {} ({:?})", device.id, device.name, device.match_status);
}

if let Some(device) = devices.iter().find(|device| device.readable) {
    let reading = client.read_battery(&device.id.0)?;
    println!("{:?}% ({:?})", reading.percentage, reading.freshness);
}
# Ok::<(), wireless_battery::BatteryError>(())
```

Experimental providers require `ClientConfig::allow_experimental` or `ReadOptions::allow_experimental`. A stale value is returned only when caching is enabled and the caller explicitly sets `ReadOptions::allow_stale`; its original `observed_at` is preserved and `freshness` becomes `stale`.

`ReadOptions::timeout` is the total call budget. A package's `timeout_ms` is a separate per-I/O preference, capped by the remaining total budget. Some platform feature operations cannot be forcibly cancelled.

## CLI

```bash
wireless-battery list --json
wireless-battery list --read
wireless-battery list --read --json --timeout-ms 5000
wireless-battery read <device-id> --json --timeout-ms 1000
wireless-battery diagnose <device-id> --json
wireless-battery capture --device <device-id> --output fixture.json --timeout-ms 5000
wireless-battery replay --fixture fixtures/synthetic/feature_normal.json
wireless-battery validate
```

Use `--include-experimental` on `list`, `read`, `diagnose`, or `capture` to opt into experimental providers. `list` includes every enumerated endpoint, including unmatched and ambiguous ones. Default `list` remains passive and displays battery as `not read`; `list --read` explicitly queries only currently readable devices. Its JSON entries contain the original device fields plus `reading` and `read_error`; one device failure does not fail the whole list. JSON is written only to stdout; warnings and progress go to stderr.

Device IDs are opaque hashes, not serial numbers. They are intended for session-local use. On platforms where the OS path is stable, they are also deterministic enough for a later CLI `read` invocation, but persistence across reboots, machines, or path reassignment is not guaranteed.

Exit codes: 0 success; 2 arguments; 3 absent/unsupported; 4 permission; 5 timeout/transport; 6 response/package/fixture validation; 70 internal error.

## Replay examples

```bash
cargo run --quiet -- replay \
  --fixture fixtures/synthetic/feature_normal.json \
  --fixture fixtures/synthetic/feature_unknown.json \
  --fixture fixtures/synthetic/bluetooth_bas.json
```

The replay transport checks operation order, report IDs, complete request payloads, response consumption, extra calls, final stable fields, and error categories. See [capture and replay](docs/capture-and-replay.md).

## Native System Tray (`tray/`)

An independent Cargo workspace under [`tray/`](tray/README.md) provides a native desktop system tray application built on a shared Rust core (`tray-core`) with thin OS-native shells (Win32 + DWM Acrylic + Direct2D on Windows; SwiftUI `MenuBarExtra` via `tray-ffi` on macOS). Keeping it in a separate workspace prevents GUI dependencies from polluting the core library or CLI.

```bash
cd tray
cargo test --locked
cargo build --locked --release -p wireless-battery-tray   # Windows native tray
```

See the [Tray README](tray/README.md) for architecture, macOS bundling, and optional BLE support.

## Documentation

- [Architecture](docs/architecture.md)
- [Support matrix](docs/support-matrix.md)
- [Platform support](docs/platform-support.md)
- [Capture and replay](docs/capture-and-replay.md)
- [Contributing a device](docs/contributing-device.md)
- [Hardware verification checklist](docs/hardware-verification.md)
- [Native system tray](tray/README.md)
- [Fixtures guide](fixtures/README.md) · [Provenance guide](provenance/README.md)

## Contributing

Start with [device contributions](docs/contributing-device.md). Simple fixed protocols should require only YAML, fixtures, provenance, and support metadata. Dynamic negotiation belongs in a focused Rust Provider, not an expanding YAML scripting language.

Licensed under the [GNU General Public License v3.0 or later](LICENSE).
