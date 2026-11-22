# Architecture

> **Language / 语言**: **English** | [简体中文](architecture.zh-CN.md)

## Read path

`BatteryClient` combines passive endpoint enumeration, a static `Registry`, exact-match policy, per-endpoint serial locks, total deadlines, provider execution, normalization, diagnostics, and a process-local last-success cache.

1. Enumerate static endpoint metadata without opening a session.
2. Return every endpoint. Unmatched, conflicting, and matched endpoints have explicit match status, `DeviceKind` derived from provider metadata, and unknown support where appropriate.
3. On an explicit read, require one highest-priority provider (`Exact` automatically outranks `Family`) from the unified `Registry`. Built-in vendor HID providers, BLE BAS (`bluetooth.bas`), and XInput (`os.xinput`) all dispatch through the same `Provider` interface.
4. Acquire the endpoint lock within the caller's total deadline, apply provider cooldown, then open a session.
5. Run at most the provider's bounded protocol. Each step checks the shared deadline.
6. Validate percentage range and derive precision from values actually present. A successful exchange infers `online` only when the protocol did not explicitly report reachability.
7. Cache only successful readings. A failed refresh returns an error unless stale was explicitly allowed.

## Transport boundary

Providers depend on `HidSession`, not HIDAPI types. Report IDs are separate from payloads throughout fixtures, providers, and the transport trait. Only `HidApiTransport` adds or removes the leading byte required by HIDAPI.

- `HidApiTransport`: real enumeration and I/O, with sanitized error classification.
- `RecordTransport`: wraps a real session and records only registered provider calls.
- `ReplayTransport`: owns a shared ordered cursor and rejects changed, missing, or extra calls.
- `BluetoothTransport` (enabled by default via `bluetooth` feature): one bounded worker thread and async runtime for read-only BAS. It never scans during `list_devices()`.
- `XInputTransport`: Windows OS slots with discrete levels only; no HID frame or fabricated percentage.
- Windows `SetOutputReport` is a distinct recorded operation for devices that reject both WriteFile and SetFeature.

Classic Bluetooth, arbitrary GATT access, event subscriptions, and arbitrary HID writes are outside the abstraction.

## Providers

`Provider::matches()` is static and I/O-free, returning `Exact`, `Family`, or `NoMatch`. `Exact` outranks `Family`, while multiple matches at the same priority become `MatchStatus::Conflict` rather than choosing arbitrarily. `ClientConfig::default()` enables built-in experimental providers by default while `ReadOptions::default().allow_experimental` defaults to `false`.

HID providers implement `Provider::read(&DeviceContext, &mut dyn HidSession, Deadline)`, while non-HID providers (`bluetooth.bas`, `os.xinput`) override `Provider::execute(&DeviceContext, &dyn Transport, Deadline)` directly.

Dynamic discovery of logical devices behind a receiver is intentionally deferred because it conflicts with passive discovery. A provider may use only statically known logical mappings in this version.

## Cache and identity

Cache is in-memory only and has no TTL. Stale fallback is explicit and retains the original timestamp. When an endpoint disappears during enumeration refresh, its known-device record, cached reading, last error, and unused endpoint lock are pruned immediately to prevent unbounded growth and reduce the risk of OS path reuse returning another device's reading.

Public IDs are irreversible hashes of OS enumeration identity, not serial numbers. They are stable only while the OS identity remains stable; no cross-machine or reboot guarantee is made.

## Capability evidence

Runtime `Capabilities` is deliberately coarse. Provider lifecycle and per-capability/per-state evidence are separate metadata. For example, charge-state support does not prove that `full`, sleeping, or offline states have been verified.

---

Related docs: [Support matrix](support-matrix.md) · [Capture and replay](capture-and-replay.md) · [Platform support](platform-support.md) · [Contributing a device](contributing-device.md) · [Native system tray](../tray/README.md) · [Back to README](../README.md)
