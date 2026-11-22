# Contributing a device

> **Language / 语言**: **English** | [简体中文](contributing-device.zh-CN.md)

## Adding a built-in provider

1. Reverse-engineer unknown HID endpoints using `wireless-battery probe <device-id> --listen` or `--send` / `--get-feature` (and Wireshark/USBPcap if needed).
2. Add a `FixedSpec` (for simple fixed request-response protocols) or a focused `Provider` implementation under `src/providers/vendors/protocols/` (shared chipset families such as CompX, DriverCore, EVision, or Bolutek) or `src/providers/vendors/brands/` (brand-specific modules), and register it in the subdirectory's `mod.rs` (wired through `src/providers/vendors/mod.rs`).
3. Populate `ProviderMetadata::device_kind` with the appropriate `DeviceKind` (`Mouse`, `Keyboard`, `Headset`, `Gamepad`, or `Peripheral`). If a single provider covers receivers or multiple hardware categories (such as Logitech HID++ or Razer), also override `Provider::device_kind(&self, endpoint: &Endpoint)` — desktop trays read `device.device_kind` directly from the core crate instead of maintaining separate ID tables.
4. Add replay fixtures under `tests/fixtures/vendors/` or `tests/extended_vendor/` (covering normal, truncated/wrong-report, invalid-field, and timeout/fallback cases where applicable) and update `docs/support-matrix.md`, `docs/support-matrix.zh-CN.md`, and `provenance/builtin-providers.md`.
5. Run `cargo test --locked --all-features` and `cargo clippy --locked --all-targets --all-features -- -D warnings` (`extended_vendor_replay_tests` enforces replay coverage for every provider in `Registry::builtin()`).

## Matching safety

Always combine VID/PID (where known) with `usage_page`, `usage`, and `interface_number` so non-vendor audio/consumer HID collections are ignored. `list_devices()` and `Provider::matches()` must never probe.

Use `MatchDecision::Exact` for model-specific rules and `MatchDecision::Family` only for broad chipset/VID wildcards (`Exact` automatically outranks `Family` on shared collections without requiring cross-file PID blacklists). Multiple matches at the same priority become `MatchStatus::Conflict` rather than picking one arbitrarily.

The current HIDAPI version does not expose report lengths during enumeration on all platforms. Do not rely on a report-length match being available everywhere.

## State semantics

- Percentage is 0–100 checked via `checked_percentage` (never `.min(100)` clamping); 0 is valid and 255 is not implicitly zero.
- Explicit unknown sentinels produce `null`/`unknown`.
- Out-of-range percentage, bad length, wrong report ID, constants mismatch, and checksum mismatch are errors.
- Never fabricate percentages from arbitrary bytes. Convert millivolts or discrete hardware levels to percentages only via an explicit, model-specific documented curve (`fixed::interpolate_mv`) or vendor step scale (e.g., documented 0–4 / 0–3 step mappings); otherwise report raw `voltage_mv` or `BatteryLevel` directly.
- Do not claim `full`, low, sleeping, or offline verification without samples. These states do not block an otherwise useful PR.
- New firmware samples are added, not used to overwrite old regression evidence.

## Verification labels

Provider status (`unknown`, `experimental`, `verified`) is separate from per-capability notes recorded in `provenance/builtin-providers.md`. Synthetic material (`source: "synthetic"`) can never be `verified` or claim physical hardware support.

Preserve applicable licenses and attribution in `NOTICE` and `provenance/builtin-providers.md` when referring to another implementation. Protocol references, copied code, and real-hardware verification are different kinds of evidence and must be recorded separately.

---

Related docs: [Capture and replay](capture-and-replay.md) · [Support matrix](support-matrix.md) · [Back to README](../README.md)
