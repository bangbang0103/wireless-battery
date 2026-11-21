# Built-in providers provenance

> **Language / 语言**: **English** | [简体中文](builtin-providers.zh-CN.md)

## Protocol references and attribution

Built-in vendor and OS providers cover 152 vendor/OS protocol paths under `src/providers/vendors/` and `src/providers/os/` (plus optional Bluetooth LE BAS). Protocol details reference community documentation and open-source implementations—including GPL-3.0-or-later and MIT-licensed protocol references noted in [NOTICE](../NOTICE), such as:

- https://github.com/krisgfx9/Wireless-Device-Battery-Monitor (GPL-3.0-or-later; BatteryHub HID device protocols)
- https://github.com/mitsimi/mchose-tray/blob/main/PROTOCOL.md
- https://github.com/Rapou7/offline-mchose/blob/master/REVERSE_ENGINEERING_MCHOSE.md

## Evidence status

JSON files under `fixtures/vendors/` (excluding `*_real.json` real-hardware captures) and the programmatic strict replay suite in `tests/extended_vendor_replay_tests.rs` are `synthetic` regression sessions constructed from documented byte layouts. They verify zero-conflict matching, request generation, checksums, filtering, parsing, and normalization across every built-in provider; they do not independently verify hardware.

All built-in providers therefore remain `experimental` until separately verified on real hardware through this library.

## Conservative protocol rules

- Discovery never performs pairing queries or probes unknown PIDs.
- All shared vendor IDs (`0x046D`, `0x1532`, `0x0B05`, `0x320F`, `0x3554`, `0x373B`, `0x258A`, `0x260D`, `0x372E`, `0x3837`, `0x24AE`, `0x046A`, `0x1B1C`, `0x054C`, etc.) use mutually disjoint `(vid, pid, usage_page, usage, interface_number)` predicates so endpoints never hit `MatchStatus::Conflict`.
- ROG Omni exposes static keyboard/mouse channels; dynamic pair-list discovery is deferred.
- XInput (`os.xinput`) returns `critical/low/medium/full` with `percentage=null`, without fabricating percentages. Wired mode has no inferred charge or full battery.
- EWEADN A5 Pro does not infer charging from the presence of a separate wired endpoint.
- Rapoo uses bounded reads and core stale fallback rather than holding a permanent provider session.
- Logitech HID++ (`logitech.hidpp`) parses charge state from response byte 6 (`charging_status` / `battery_status`), supports `0x1004` / `0x1000` / `0x1001` / `0x1F20`, and fast-fails on HID++ `0xFF` error frames.
- Razer (`razer.mouse`) uses a monotonic 0–255 linear scale and restricts matching to vendor usage pages (`>= 0xFF00`).
- DualSense (`sony.dualsense`) and DualShock 4 (`sony.dualshock4`) use read-only feature/input reports (`get_feature` / `get_input_report`) and keep unknown status nibbles as `unknown`.
- HID report-length matching is omitted where HIDAPI 2.6 cannot enumerate lengths; cross-platform `interface_number` and `bus_type` are preferred over OS-specific path substrings, and known conflicting PIDs are excluded explicitly.
- Non-HID or split-collection dual-handle protocols (BLE advertisements, Classic BT RFCOMM, Phone Link local JSON, serial COM ports, vendor BLE GATT, external `headsetcontrol.exe`, or devices requiring two simultaneous HID collection handles) are intentionally deferred rather than emulated unsafely.

## Current hardware verification

WOB Rainy75 (`wob.rainy75`) returned real readings on Windows: 50% discharging (`frame[9]=0x00`, `fixtures/vendors/wob/rainy75_real.json`) and 62% charging (`frame[9]=0x01`, `fixtures/vendors/wob/rainy75_charging_real.json`). Both sanitized sessions pass strict replay; percentage and discharging/charging state are `verified` (other values remain `unknown`). Logitech GPW3 (`logitech.hidpp`) returned a real 46%, discharging reading through index 1 and the `0x1004` function-1 fallback. Sanitized `fixtures/vendors/logitech/hidpp_real.json` passes strict replay; this model/path is `verified`. Other providers remain `experimental`.

---

Related docs: [Hardware verification checklist](../docs/hardware-verification.md) · [Support matrix](../docs/support-matrix.md) · [Provenance guide](README.md) · [Back to README](../README.md)
