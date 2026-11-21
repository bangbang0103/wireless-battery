# 录制与回放

> **Language / 语言**: [English](capture-and-replay.md) | **简体中文**

## 规范化报告格式

Fixture 中的 `hex`、设备包中的 `payload_hex` 以及字段偏移量均**不包含** HID Report ID；Report ID 单独保存在 `report_id` 字段中。`get_feature.length` 与响应长度仅统计有效载荷字节数。HIDAPI 的前导报告字节属于底层传输实现细节。

每个 Fixture 步骤与一次传输操作一一对应：HID 的 `send_feature`、`get_feature`、`write_output`、`set_output`、`read_input`，以及标准传输的 `read_bas` 与 `read_xinput`。这避免了无法区分错误发生在请求阶段还是响应阶段的歧义。Fixture 与设备包的 JSON Schema 统一位于 `schemas/` 目录下。

## 录制（Capture）

先列出设备，再针对已匹配设备显式发起录制：

```bash
wireless-battery list --json
wireless-battery capture --device <device-id> --output fixtures/vendors/<vendor>/<device>.json
```

仅在审查过对应 Provider 后才使用 `--include-experimental`。`capture` 只会执行已注册 Provider 的正常电量查询，不是任意 HID 写入工具。已存在的输出文件会被拒绝写入而不会被覆盖。终端会输出目标设备、步骤数和输出路径。

v0.1 的 `capture` 命令封装了真实 HID 传输。BLE BAS Fixture 可以根据标准单字节读数构造，暂不提供任意 GATT 抓包工具。

## 脱敏规范

请勿提交完整 PCAP 文件、键盘按键输入、鼠标移动轨迹、无关 USB 流量、用户名、原始系统路径、序列号或原始操作系统标识符。Fixture 仅允许包含静态端点元数据、已注册会话步骤、期望归一化结果以及非敏感环境信息。

`validate` 会拒绝已知敏感字段与 Schema 违规；但在提交前仍请人工复查导出的 JSON，自动化校验无法识别所有隐私信息。

## 回放（Replay）

```bash
wireless-battery replay --fixture fixtures/synthetic/feature_normal.json
```

回放会严格校验操作类型、执行顺序、目标 Provider 匹配、请求 Report ID、完整请求字节、请求长度、录制的响应或错误、最终稳定读数字段或错误类别，以及遗漏或多余的调用。时间戳、进程 ID 和缓存新鲜度不属于协议证据。

真实抓包的正常用例保留 `source: real`；截断、坏校验和、保留值等衍生异常用例必须标记为 `source: synthetic`。合成异常用例不会削弱原始真实抓包的证据效力，但也不代表真实硬件一定产生过该异常响应。

## 必备覆盖用例

每个声明式包至少需要覆盖：正常读数、畸形/截断或错误 Report ID、非法字段值。复杂 Provider 还应补充超时、协商失败或回退路径测试。条件允许时鼓励补充充电/未充电、满电、低电量、休眠及离线样本，但这些边界状态不构成合并阻塞项。

---

相关文档：[架构设计](architecture.zh-CN.md) · [设备贡献指南](contributing-device.zh-CN.md) · [Fixture 目录说明](../fixtures/README.zh-CN.md) · [返回 README](../README.zh-CN.md)
