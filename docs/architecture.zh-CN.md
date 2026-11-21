# 架构设计

> **Language / 语言**: [English](architecture.md) | **简体中文**

## 读取主链路

`BatteryClient` 将被动端点枚举、静态 `Registry`、精确匹配策略、按端点串行锁、总超时预算、Provider 执行、结果归一化、诊断与进程内最近成功缓存组合在一起。

1. 枚举静态端点元数据，过程中不打开任何设备会话。
2. 返回所有枚举到的端点；未匹配、候选（candidate）与冲突（conflict）端点均带有明确的匹配状态及对应支持状态。
3. 发起显式读取时，要求在统一 `Registry` 中存在唯一精确匹配的 Provider 且其生命周期状态已启用。内置厂商 HID Provider、声明式 YAML 包、BLE BAS（`bluetooth.bas`）以及 XInput（`os.xinput`）全部通过同一套 `Provider` 接口派发。
4. 在调用方的总截止时间（deadline）内获取端点锁并满足冷却间隔，随后执行协议读取。
5. 仅执行有界的协议交互步骤，每一步都校验共享截止时间。
6. 校验电量百分比范围，并根据实际存在的字段推导精度（`precision`）。仅当协议未显式报告可达性时，才在一次成功交互后保守推断为 `online`。
7. 仅缓存成功的读数。刷新失败时默认返回错误，除非调用方显式允许返回旧值（`allow_stale`）。

## 传输层边界

HID Provider 仅依赖 `HidSession`，不直接耦合 HIDAPI 类型。Report ID 与有效载荷在 YAML 包、Fixture、Provider 与传输层 trait 中全程分离，仅由 `HidApiTransport` 负责按 HIDAPI 要求拼装或剥离前导字节。

- `HidApiTransport`：真实 HID 枚举与 I/O，内置脱敏错误分类与无编号 Report ID 0 检测。
- `RecordTransport`：包装真实传输层，仅录制已注册 Provider 发起的调用。
- `ReplayTransport`：持有共享有序游标，严格拒绝请求篡改、遗漏调用或多余调用。
- `BluetoothTransport`（通过 `bluetooth` feature 默认启用）：单个有界工作线程与异步运行时，仅用于只读 BAS；`list_devices()` 绝不触发扫描。
- `XInputTransport`：Windows 操作系统手柄槽位，仅返回 4 级离散电量，不生成 HID 帧，也不伪造百分比。
- Windows `SetOutputReport`：作为独立操作被录制与回放，兼容拒绝 WriteFile 与 SetFeature 的设备。

经典蓝牙、任意 GATT 读写、事件订阅以及任意 HID 写入均不在本抽象范围内。

## Provider 体系

`Provider::matches()` 是静态且零 I/O 的。仅 `Exact` 匹配允许读取；`Candidate` 仅用于诊断；多个 `Exact` 匹配会报告 `Conflict`，绝不随意择一。`draft` 仅限开发，`experimental` 需显式启用，`verified` 须具备实机证据，`deprecated` 仍可读但保留可见状态。

HID Provider 实现 `Provider::read(&DeviceContext, &mut dyn HidSession, Deadline)`，非 HID Provider（`bluetooth.bas`、`os.xinput`）直接覆写 `Provider::execute(&DeviceContext, &dyn Transport, Deadline)`。声明式包与回放 Fixture 的 JSON Schema 统一存放在 `schemas/` 目录下。

`DeclarativeProvider` 最多支持 4 轮固定请求-响应交互。中间轮次各自校验响应，最终归一化字段从最后一轮响应提取。动态设备索引、协议回退、无关事件过滤、接收器扇出与有状态协商属于 Rust Provider 的职责。

接收器背后的动态逻辑设备发现因与被动发现原则冲突，在本版本中暂缓实现；当前版本仅使用静态已知的逻辑映射。

## 缓存与设备身份

缓存仅存在于内存中且无 TTL。回退到旧值必须显式声明，并保留原始观测时间戳。当端点在枚举刷新中消失时，其已知设备记录、缓存读数、最近错误和未占用的端点锁会被立即清理，既避免常驻进程内存增长，也降低操作系统路径复用导致串读旧数据的风险。

公开 ID 是基于操作系统枚举标识的不可逆哈希，而非序列号；仅在系统标识稳定期间保持一致，不提供跨机器或跨重启保证。

## 能力证据

运行时 `Capabilities` 刻意保持粗粒度；包生命周期与单能力/单状态证据分开记录。例如，支持 `charge_state` 并不意味着 `full`、`sleeping` 或 `offline` 状态已经过实机验证。

---

相关文档：[设备支持矩阵](support-matrix.zh-CN.md) · [录制与回放](capture-and-replay.zh-CN.md) · [平台支持](platform-support.zh-CN.md) · [原生系统托盘](../tray/README.zh-CN.md) · [返回 README](../README.zh-CN.md)
