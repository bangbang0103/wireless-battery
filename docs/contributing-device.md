# Contributing a device

> **Language / 语言**: **English** | [简体中文](contributing-device.zh-CN.md)

## Choose the extension path

Use a YAML package only for static AND matching and at most four fixed Feature or Output/Input request-response rounds. Supported extraction is `u8`, boolean bits, enum maps, fixed constants, and `sum8`/`xor8`.

Use a built-in Rust Provider for dynamic indexes, version fallback, conditional requests, state, bounded unrelated-event filtering, or other protocol logic. Do not add scripts, file access, network access, dynamic libraries, or general loops to YAML.

## Simple package checklist

1. Add `packages/vendors/<vendor>/<device>.yaml`.
2. Add at least a normal, truncated/wrong-report, and invalid-field fixture. Add timeout or fallback coverage where applicable.
3. Add `provenance/<device>.md` documenting protocol source, code license if reused, hardware model, OS, connection, firmware, and who verified each capability.
4. Update per-capability evidence and `docs/support-matrix.md`.
5. Run `cargo test --all-features`, `cargo run -- validate`, and all relevant replay fixtures.

A new simple package must not require scheduler, public model, CLI, or another Provider changes.

## Matching safety

Prefer VID/PID plus usage page and other static discriminators. VID/PID-only rules need provenance explaining why they are sufficient. `list_devices()` and `Provider::matches()` must never probe. Multiple exact matches are a conflict, not a reason to loosen matching or pick the first provider.

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

Related docs: [Capture and replay](capture-and-replay.md) · [Support matrix](support-matrix.md) · [Provenance guide](../provenance/README.md) · [Back to README](../README.md)
