# 设备贡献指南

> **Language / 语言**: [English](contributing-device.md) | **简体中文**

## 选择扩展方式

仅当协议满足“静态与（AND）匹配 + 最多 4 轮固定 Feature 或 Output/Input 请求-响应”时使用 YAML 设备包。YAML 支持的提取能力包括 `u8`、布尔位（`bool_bit`）、枚举映射、固定常量以及 `sum8`/`xor8` 校验和。

动态索引、版本回退、条件请求、状态维护、有界无关事件过滤或其他复杂协议逻辑应使用内置 Rust Provider 实现。不要向 YAML 中引入脚本、文件访问、网络访问、动态库或通用循环。

## 简单设备包贡献清单

1. 新增 `packages/vendors/<vendor>/<device>.yaml`。
2. 至少补充正常、截断/错包、非法字段三类 Fixture；适用时补充超时或回退用例。
3. 新增 `provenance/<device>.md`，记录协议来源、复用的代码许可证、硬件型号、操作系统、连接方式、固件版本以及每项能力的验证人。
4. 更新单能力证据以及[支持矩阵](support-matrix.zh-CN.md)（及其英文版 [support-matrix.md](support-matrix.md)）。
5. 运行 `cargo test --all-features`、`cargo run -- validate` 以及所有相关回放 Fixture。

新增一个简单 YAML 设备包不应要求修改调度器、公开数据模型、CLI 或其他 Provider。

## 匹配安全性

优先组合使用 VID/PID、`usage_page`、`usage`、`interface_number` 等静态区分字段。仅依赖 VID/PID 的规则必须在 provenance 中说明其充分性。`list_devices()` 与 `Provider::matches()` 绝不允许探测设备。出现多个精确匹配时应视为冲突，而不是放宽匹配规则或任选第一个 Provider。

当前版本的 HIDAPI 无法在所有平台上枚举 HID 报告长度，请勿假设报告长度匹配在所有系统上都可用。

## 状态语义要求

- 百分比范围为 0–100；`0` 是合法值，`255` 不能被隐式当作 `0`。
- 明确的未知哨兵值必须输出 `null`/`unknown`。
- 超范围百分比、非法长度、错误 Report ID、常量不匹配与校验和不匹配都必须报错。
- 在没有针对该设备的实证规则前，不要将电压或离散档位换算为百分比；v0.1 不做此类换算。
- 没有样本证据时不要宣称 `full`、低电量、休眠或离线状态已验证；这些状态不构成 PR 合并阻塞项。
- 新固件样本应以增量方式添加，避免覆盖历史回归证据。

## 验证标签规范

包生命周期（`draft`、`experimental`、`verified`、`deprecated`）与单能力证据（`verified`、`experimental`、`unverified`、`unsupported`）相互独立。合成资产永远不能标为 `verified`，也不能进入真实支持矩阵。

参考其他实现时必须保留对应许可证与署名说明。协议事实、借鉴代码与实机验证属于不同类别的证据，必须分开记录。

---

相关文档：[录制与回放](capture-and-replay.zh-CN.md) · [设备支持矩阵](support-matrix.zh-CN.md) · [协议来源说明](../provenance/README.zh-CN.md) · [返回 README](../README.zh-CN.md)
