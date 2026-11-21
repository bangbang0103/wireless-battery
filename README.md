# Wireless Battery

> **Language / 语言**: **English** | [简体中文](README.zh-CN.md)

A synchronous Rust library and CLI for conservative peripheral battery readings over HID, with built-in read-only Bluetooth LE Battery Service (BAS) support.

The project prioritizes protocol safety and reproducibility over device count:

- passive discovery never opens HID sessions, writes reports, or starts a Bluetooth scan;
- unknown values stay unknown—timeouts never become 0%, and voltage or discrete levels are not converted into fake percentages;
- every declarative protocol can be replayed from minimal JSON without hardware;
- synthetic protocols are excluded from real discovery and the support matrix;
- raw system paths, serial numbers, and HID bytes are absent from normal readings and logs.

## Current support

> **Supported Product Matrix**: See [**Support Matrix (`docs/support-matrix.md`)**](docs/support-matrix.md) for the full table organized by **Vendor**, **Device**, and **Support Method** (Built-in Rust HID, OS XInput API, or BLE BAS; Verified vs. Experimental).

**152 built-in vendor and OS provider paths** (plus Bluetooth LE BAS) are included, covering Logitech / Astro, Razer, SteelSeries, Corsair, HyperX, ASUS ROG, Sony, Nintendo, Valve, GameSir, Flydigi, 8BitDo, Beitong, Keychron, Cherry, Akko, NuPhy, AULA, MCHOSE, ATK / VXE / VGN, DAREU, EWEADN, Rapoo, WOB, Glorious, Lamzu, WLmouse, Pulsar, VAXEE, Ninjutso, Endgame Gear, Finalmouse, ZOWIE, Audeze, JBL, Jabra, Turtle Beach, AKG, Fractal, Alienware, Moondrop, Maono, XInput, and others listed in the [support matrix](docs/support-matrix.md). They are experimental unless a specific device and capability are marked verified in the matrix. Strict wire-level synthetic replay exists for every built-in provider; this does not independently verify hardware. WOB Rainy75 and Logitech GPW3 real-hardware verification is tracked in the [hardware verification checklist](docs/hardware-verification.md). A fictional declarative mouse remains available for interpreter tests and is excluded from real support.

Built-in vendor protocol definitions reference [Wireless-Device-Battery-Monitor / BatteryHub](NOTICE) (GPL-3.0-or-later) and [Voltide](NOTICE) (MIT); see [NOTICE](NOTICE) for full attribution.

Standard HID battery usages and Classic Bluetooth/HFP battery reports are reserved for later versions. BLE support implements only BAS Battery Level (`0x2A19`) reads, not pairing, notifications, charging state, or cross-transport device deduplication.

## Build

Rust 1.76 or newer is supported. `Cargo.lock` pins dependency versions to preserve that MSRV (`--locked`). Both `cli` and `bluetooth` features are enabled by default; library consumers can disable default features with `--no-default-features` to avoid compiling CLI (`clap`, `walkdir`) and BLE (`btleplug`, `tokio`, `uuid`) dependencies, or enable only `--features cli` / `--features bluetooth`.

```bash
cargo build --locked
cargo build --locked --no-default-features
cargo test --locked --all-features
cargo run --locked -- validate
```

See [platform support](docs/platform-support.md) for system dependencies and permission limitations.

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

See the [Tray README](tray/README.md) for architecture, macOS bundling, and BLE support details.

## AI Collaboration

You can hand the following prompt directly to an AI coding agent (such as Antigravity, Claude Code, Cursor, or Codex) to **detect connected devices, collect sanitized battery/charging fixtures, and reverse-engineer or integrate unsupported devices** via the `wireless-battery` CLI:

```text
Help me detect, collect, and reverse-engineer the battery percentage and charging/discharging state of my currently connected wireless peripherals using the wireless-battery CLI.

Follow these steps in order:
0. **Prepare the CLI Binary**:
   - First check if `wireless-battery` / `wireless-battery.exe` is already available in the current directory or PATH. If not, guide me to download the prebuilt binary for my OS (e.g., `wireless-battery.exe` on Windows from the project's Releases) and place it in the working directory or PATH (if we are inside the source repository, `cargo run --locked --` can also be used).
   - Run all subsequent commands using that binary (written below as `wireless-battery`; on Windows use `.\wireless-battery.exe` if in the current directory).

1. **Detect & Inspect**:
   - Run `wireless-battery list --json --include-experimental` to passively enumerate all HID/BLE endpoints, then run `wireless-battery list --read --json --include-experimental` to query currently readable devices.
   - Summarize every detected device (name, VID/PID, `usage_page`, `usage`, `interface_number`, `match_status`, `support_status`, `provider_id`, current battery percentage, and charge state). For any matched or conflicting device, run `wireless-battery diagnose <device-id> --json --include-experimental` if needed.

2. **Collect Real-Hardware Fixtures (Matched / Experimental Devices)**:
   - For any matched device that needs verification, guide me to test both **discharging** (wireless) and **charging** (plugged in) states.
   - Capture sanitized fixtures without overwriting existing files:
     `wireless-battery capture --device <device-id> --output fixtures/<vendor_device>/discharging.json --include-experimental --timeout-ms 5000`
     `wireless-battery capture --device <device-id> --output fixtures/<vendor_device>/charging.json --include-experimental --timeout-ms 5000`
   - Verify captured fixtures with `wireless-battery replay --fixture ...` and `wireless-battery validate`.

3. **Reverse-Engineer & Integrate (Unmatched / Incomplete Devices)**:
   - If a target device is `unmatched` or its charging/discharging/battery reading is missing or inaccurate, inspect its enumerated HID endpoint fields from `list --json` (VID/PID, `usage_page`, `usage`, `interface_number`) and compare against existing protocols in `src/providers/vendors/`, `docs/contributing-device.md`, and `schemas/`.
   - Propose a minimal, safe HID query/decode hypothesis (never guess percentages from voltage/levels; map unknown sentinels to `unknown`), implement it either as a declarative YAML package under `packages/vendors/<vendor>/<device>.yaml` (for <=4 static Feature/Output/Input rounds) or a focused Rust provider under `src/providers/vendors/`, and ask me to test both **discharging** and **charging** states to confirm the raw bytes and state transitions.
   - Once confirmed, add normal/malformed/invalid-field fixtures, `provenance/<device>.md`, and update `docs/support-matrix.md` & `docs/support-matrix.zh-CN.md`.
   - Finish by verifying with `replay`, `validate`, and the test suite.
```

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
