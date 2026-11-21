# Capture and replay

> **Language / 语言**: **English** | [简体中文](capture-and-replay.zh-CN.md)

## Canonical reports

Fixture `hex` and field offsets exclude the HID Report ID. The ID is stored separately in `report_id`. `get_feature.length` and response lengths count payload bytes only. HIDAPI's leading report byte is a transport implementation detail.

Each fixture step maps one-to-one to a transport operation: HID `send_feature`, `get_feature`, `write_output`, `set_output`, or `read_input`, as well as standard `read_bas` and `read_xinput`. This avoids ambiguity about whether an error belongs to a request or its response.

## Capture

First list devices, then explicitly capture a matched device:

```bash
wireless-battery list --json
wireless-battery capture --device <device-id> --output tests/fixtures/vendors/<vendor>/<device>.json
```

Capture executes only the registered provider's battery query; for unknown/unmatched HID devices, use `wireless-battery probe` instead. Existing output files are rejected rather than overwritten. The terminal reports the target, step count, and output path.

The v0.1 capture command wraps real HID transport. BLE BAS fixtures can be constructed from the standard one-byte reading, but active BLE capture is not exposed as an arbitrary GATT tool.

## Sanitization

Do not submit full PCAP files, keyboard input, mouse movement streams, unrelated USB traffic, usernames, system paths, serial numbers, or raw OS identifiers. Fixtures contain only static endpoint metadata, the registered session, expected normalized results, and non-sensitive environment information.

Review captured JSON manually before submitting to ensure no private values are included.

## Replay

Replay fixtures are executed automatically via the test suite:

```bash
cargo test --locked --all-features
```

Replay checks operation type, order, request Report ID, complete request bytes, requested length, recorded responses/errors, final stable reading fields or error kind, omitted calls, and extra calls. Timestamps, process IDs, and cache freshness are not protocol evidence.

Normal real fixtures retain `source: real`. Truncated, bad-checksum, reserved-value, and other derived abnormal cases must use `source: synthetic`. A synthetic abnormal fixture does not weaken the evidence of the original real normal capture, but it also does not prove that real hardware produces that malformed response.

## Required cases

Each provider should have normal, malformed/truncated or wrong-report, and invalid-field coverage. Complex providers also need timeout, negotiation failure, or fallback coverage. Add charging/not-charging samples when practical. Full, low battery, sleep, and offline samples are encouraged but not merge blockers.

---

Related docs: [Architecture](architecture.md) · [Contributing a device](contributing-device.md) · [Fixtures guide](../tests/fixtures/README.md) · [Back to README](../README.md)
