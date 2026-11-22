# 设备贡献指南

> **Language / 语言**: [English](contributing-device.md) | **简体中文**

## 新增内置 Provider

1. 对于未知 HID 端点，可使用 `wireless-battery probe <device-id> --listen` 或 `--send` / `--get-feature`（必要时配合 Wireshark/USBPcap）收集请求与响应报文。
2. 在 `src/providers/vendors/protocols/`（CompX、DriverCore、EVision、Bolutek 等共享芯片/协议族）或 `src/providers/vendors/brands/`（按品牌划分）下新增 `FixedSpec`（适合简单固定请求-响应）或聚焦的 `Provider` 实现，并在对应子模块的 `mod.rs` 中注册（经 `src/providers/vendors/mod.rs` 汇总）。
3. 在 `ProviderMetadata::device_kind` 设置对应的 `DeviceKind`（`Mouse`、`Keyboard`、`Headset`、`Gamepad` 或 `Peripheral`）；若单个 Provider 同时服务接收器与多品类设备（如 Logitech HID++、Razer），请同步覆写 `Provider::device_kind(&self, endpoint: &Endpoint)`——托盘端直接读取核心库输出的 `device.device_kind`，无需再单独维护映射表。
4. 在 `tests/fixtures/vendors/` 或 `tests/extended_vendor/` 下补充回放测试（正常、截断/错包、非法字段以及超时/回退用例），并更新 `docs/support-matrix.zh-CN.md` 与 `docs/support-matrix.md`。
5. 运行 `cargo test --locked --all-features` 与 `cargo clippy --locked --all-targets --all-features -- -D warnings` 验证（`extended_vendor_replay_tests` 会强制检查 `Registry::builtin()` 中全部 Provider 的回放覆盖率）。

## 匹配安全性

务必组合使用 VID/PID、`usage_page`、`usage`、`interface_number` 等静态区分字段，避免误匹配音频/消费控制 HID 集合。`list_devices()` 与 `Provider::matches()` 绝不允许探测设备。

具体型号匹配返回 `MatchDecision::Exact`，仅宽泛芯片/VID 通配使用 `MatchDecision::Family`（在共享同一 `usage_page` 的集合上，`Exact` 会自动优先于 `Family`，无需跨文件手写 PID 黑名单）。同优先级出现多个命中时会报告 `MatchStatus::Conflict`，绝不随意择一。

当前版本的 HIDAPI 无法在所有平台上枚举 HID 报告长度，请勿假设报告长度匹配在所有系统上都可用。

## 状态语义要求

- 百分比范围为 0–100，统一使用 `checked_percentage` 校验（禁止 `.min(100)` 静默钳位）；`0` 是合法值，`255` 不能被隐式当作 `0`。
- 明确的未知哨兵值必须输出 `null`/`unknown`。
- 超范围百分比、非法长度、错误 Report ID、常量不匹配与校验和不匹配都必须报错。
- 严禁凭空猜测百分比；仅在有明确型号电压分压曲线（使用 `fixed::interpolate_mv`）或厂商文档化的离散档位映射规则时才可将电压/档位换算为百分比，否则应直接上报 `voltage_mv` 或离散 `BatteryLevel`。
- 没有样本证据时不要宣称 `full`、低电量、休眠或离线状态已验证；这些状态不构成 PR 合并阻塞项。
- 新固件样本应以增量方式添加，避免覆盖历史回归证据。

## 验证标签规范

Provider 状态（`unknown`、`experimental`、`verified`）用于记录验证证据，不影响 Provider 是否启用。合成 Fixture（`source: "synthetic"`）永远不能标为 `verified`，也不能代替真实硬件验证。

参考其他实现时必须在 `NOTICE` 中保留对应许可证与署名说明。协议参考来源、借鉴代码与实机抓包验证属于不同类别的证据，必须分开记录。

---

相关文档：[录制与回放](capture-and-replay.zh-CN.md) · [设备支持矩阵](support-matrix.zh-CN.md) · [返回 README](../README.zh-CN.md)
