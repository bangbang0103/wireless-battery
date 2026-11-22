# 设备贡献指南

> **Language / 语言**: [English](contributing-device.md) | **简体中文**

## 新增内置 Provider

1. 对于未知 HID 端点，可使用 `wireless-battery probe <device-id> --listen` 或 `--send` / `--get-feature`（必要时配合 Wireshark/USBPcap）收集请求与响应报文。
2. 在 `src/providers/vendors/` 下新增 `FixedSpec`（适合简单固定请求-响应）或聚焦的 `Provider` 实现，并在 `src/providers/vendors/mod.rs` 中注册。
3. 在 `tests/fixtures/vendors/` 下补充回放测试 Fixture（正常、截断/错包、非法字段以及超时/回退用例），并更新 `docs/support-matrix.zh-CN.md` 与 `docs/support-matrix.md`。
4. 运行 `cargo test --locked --all-features` 验证。

## 匹配安全性

优先组合使用 VID/PID、`usage_page`、`usage`、`interface_number` 等静态区分字段。`list_devices()` 与 `Provider::matches()` 绝不允许探测设备。出现多个精确匹配时应视为冲突，而不是放宽匹配规则或任选第一个 Provider。

当前版本的 HIDAPI 无法在所有平台上枚举 HID 报告长度，请勿假设报告长度匹配在所有系统上都可用。

## 状态语义要求

- 百分比范围为 0–100；`0` 是合法值，`255` 不能被隐式当作 `0`。
- 明确的未知哨兵值必须输出 `null`/`unknown`。
- 超范围百分比、非法长度、错误 Report ID、常量不匹配与校验和不匹配都必须报错。
- 在没有针对该设备的实证规则前，不要将电压或离散档位换算为百分比；v0.1 不做此类换算。
- 没有样本证据时不要宣称 `full`、低电量、休眠或离线状态已验证；这些状态不构成 PR 合并阻塞项。
- 新固件样本应以增量方式添加，避免覆盖历史回归证据。

## 验证标签规范

Provider 生命周期（`draft`、`experimental`、`verified`、`deprecated`）与单能力证据（`verified`、`experimental`、`unverified`、`unsupported`）相互独立。合成资产永远不能标为 `verified`，也不能进入真实支持矩阵。

参考其他实现时必须保留对应许可证与署名说明。协议事实、借鉴代码与实机验证属于不同类别的证据，必须分开记录。

---

相关文档：[录制与回放](capture-and-replay.zh-CN.md) · [设备支持矩阵](support-matrix.zh-CN.md) · [返回 README](../README.zh-CN.md)
