# Fixtures

> **Language / 语言**: **English** | [简体中文](README.zh-CN.md)

Fixtures are minimal JSON protocol sessions validated by `schemas/fixture.schema.json`. Report IDs are separate from payload hex and offsets.

- `synthetic/`: fictional normal and abnormal cases used by CI; never real support evidence.
- `vendors/<vendor>/<device>.json`: reviewed, sanitized captures and derived abnormal cases organized by vendor or OS source.

Never add PCAP files, serial numbers, raw system paths, user directories, keystrokes, mouse movement streams, or unrelated device traffic.

---

Related docs: [Capture and replay](../docs/capture-and-replay.md) · [Synthetic packages](synthetic/packages/README.md) · [Back to README](../README.md)
