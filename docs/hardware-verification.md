# Hardware verification checklist

> **Language / 语言**: **English** | [简体中文](hardware-verification.zh-CN.md)

Built-in providers remain experimental until the checks below pass on real hardware through this library.

## WOB Rainy75

- Passive match is `320F:5088`, usage page `FF1C`, usage `0092`.
- Confirm 2.4G mode and wake the keyboard before reading.
- Read percentage and compare with the vendor UI if available.
- Capture both discharging and charging sessions to confirm payload byte 8 (`frame[9]`: `0x00` discharging, `0x01` charging; other values remain `unknown` until verified).
- Verify timeout behavior separately from offline claims.
- Verified on Windows: real queries returned 50% discharging (`frame[9]=0x00`, `fixtures/vendors/wob/rainy75_real.json`) and 62% charging (`frame[9]=0x01`, `fixtures/vendors/wob/rainy75_charging_real.json`); both sanitized real sessions pass strict replay. Full-charge indicator behavior (if any) remains unverified.

## Logitech GPW3

- Passive match is the `046D:C547` `FF00/0002` long-report collection.
- Confirm 2.4G mode and move the mouse during the first index discovery if necessary.
- Verify dynamic device index, then either `0x1000` or the `0x1004` function-1 percentage path.
- Compare percentage and charging state with G HUB without asking either application to share raw reports.
- Do not use `0x1004` function 0 as a percentage; it exposes capability/voltage information rather than state of charge.
- Verified on Windows: index 1, no `0x1000`, fallback to `0x1004` function 1 returned 46%, discharging. Sanitized `fixtures/vendors/logitech/hidpp_real.json` passes strict replay. Other models and charging/full states remain unverified.

Timeouts are retained as errors. They do not establish sleeping or offline state.

---

Related docs: [Support matrix](support-matrix.md) · [Built-in providers provenance](../provenance/builtin-providers.md) · [Back to README](../README.md)
