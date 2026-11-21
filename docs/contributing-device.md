# Contributing a device

> **Language / 语言**: **English** | [简体中文](contributing-device.zh-CN.md)

## Adding a built-in provider

1. Reverse-engineer unknown HID endpoints using `wireless-battery probe <device-id> --listen` or `--send` / `--get-feature` (and Wireshark/USBPcap if needed).
2. Add a `FixedSpec` (for simple fixed request-response protocols) or a focused `Provider` implementation under `src/providers/vendors/` and register it in `src/providers/vendors/mod.rs`.
3. Add replay fixtures under `tests/fixtures/vendors/` (normal, truncated/wrong-report, invalid-field, and timeout/fallback cases where applicable) and update `docs/support-matrix.md` & `docs/support-matrix.zh-CN.md`.
4. Run `cargo test --locked --all-features`.

## Matching safety

Prefer VID/PID plus usage page and other static discriminators. `list_devices()` and `Provider::matches()` must never probe. Multiple exact matches are a conflict, not a reason to loosen matching or pick the first provider.

The current HIDAPI version does not expose report lengths during enumeration on all platforms. Do not rely on a report-length match being available everywhere.

## State semantics

- Percentage is 0–100; 0 is valid and 255 is not implicitly zero.
- Explicit unknown sentinels produce `null`/`unknown`.
- Out-of-range percentage, bad length, wrong report ID, constants mismatch, and checksum mismatch are errors.
- Do not convert voltage or discrete levels into percentages without a device-specific, evidence-backed rule; v0.1 does not perform such conversion.
- Do not claim `full`, low, sleeping, or offline verification without samples. These states do not block an otherwise useful PR.
- New firmware samples are added, not used to overwrite old regression evidence.

## Verification labels

Package lifecycle (`draft`, `experimental`, `verified`, `deprecated`) is separate from per-capability evidence (`verified`, `experimental`, `unverified`, `unsupported`). Synthetic material can never be `verified` or enter the real support matrix.

Preserve applicable licenses and attribution when referring to another implementation. Protocol facts, copied code, and real-hardware verification are different kinds of evidence and must be recorded separately.

---

Related docs: [Capture and replay](capture-and-replay.md) · [Support matrix](support-matrix.md) · [Back to README](../README.md)
