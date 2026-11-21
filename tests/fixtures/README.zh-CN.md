# 回放 Fixture 说明

> **Language / 语言**: [English](README.md) | **简体中文**

Fixture 是精简的 JSON 协议会话记录，用于确定性回放测试。Report ID 与有效载荷十六进制字节及偏移量分开存放。

- `synthetic/`：CI 使用的虚构正常与异常回归用例，绝不作为真实硬件支持证据。
- `vendors/<vendor>/<device>.json`：按厂商或操作系统来源分目录存放的脱敏实机抓包与衍生异常用例。

严禁提交 PCAP 文件、序列号、原始系统路径、用户目录、按键记录、鼠标移动轨迹或无关设备流量。

---

相关文档：[录制与回放规范](../../docs/capture-and-replay.zh-CN.md) · [返回 README](../../README.zh-CN.md)
