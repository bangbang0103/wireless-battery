# 实机验证清单

> **Language / 语言**: [English](hardware-verification.md) | **简体中文**

在通过本库完成以下实机检查项之前，对应内置 Provider 均保持实验状态。

## WOB Rainy75

- 被动匹配条件：`320F:5088`，`usage_page = FF1C`，`usage = 0092`。
- 确认处于 2.4G 模式，并在读取前唤醒键盘。
- 读取电量百分比，并与厂商驱动界面对比（若可用）。
- 分别录制放电与充电会话，确认有效载荷第 8 字节（完整帧 `frame[9]`：`0x00` 为放电、`0x01` 为充电，其余未验证值保守保持 `unknown`）。
- 单独验证超时行为，确保超时不会被误判为离线。
- **Windows 实机验证通过**：真实查询分别返回 `50%` 放电中（`frame[9]=0x00`，`fixtures/vendors/wob/rainy75_real.json`）与 `62%` 充电中（`frame[9]=0x01`，`fixtures/vendors/wob/rainy75_charging_real.json`），两份脱敏真实会话均已通过严格回放。满电独立状态码（若存在）仍待实机验证。

## Logitech GPW3

- 被动匹配条件：`046D:C547`，`FF00/0002` 长报告集合。
- 确认处于 2.4G 模式，首次扫描设备索引时如有必要轻移鼠标唤醒。
- 验证动态设备索引发现，以及 `0x1000` 或 `0x1004` function-1 百分比读取路径。
- 与 G HUB 对照百分比与充电状态，但不要求两个程序共享底层原始报告。
- 不要把 `0x1004` function-0 当作百分比使用；该功能返回能力/电压相关信息而非电量百分比。
- **Windows 实机验证通过**：命中索引 1，设备无 `0x1000`，回退至 `0x1004` function-1 返回 `46%`、放电中（`discharging`）。脱敏后的 `fixtures/vendors/logitech/hidpp_real.json` 已通过严格回放。其余型号及充电/满电状态仍待实机验证。

超时一律保留为错误，绝不用于推断休眠或离线状态。

---

相关文档：[设备支持矩阵](support-matrix.zh-CN.md) · [内置协议来源与验证记录](../provenance/builtin-providers.zh-CN.md) · [返回 README](../README.zh-CN.md)
