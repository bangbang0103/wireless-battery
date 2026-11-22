# 平台支持与限制

> **Language / 语言**: [English](platform-support.md) | **简体中文**

## 构建矩阵

CI 已配置在 Windows 与 macOS 上使用 Rust 1.76 运行格式化检查、全特性及纯库 Clippy、全特性测试（含合成与厂商 Fixture 回放）以及发布产物构建。通过 CI 配置仅代表编译与回放健康，不代表所有平台都已完成实机硬件验证。

Linux 下编译 HIDAPI 可能需要 C 编译器、`pkg-config`、`libudev-dev` 与 `libusb-1.0-0-dev`（取决于所选 HIDAPI 后端）。BLE 通过 `btleplug` 使用 BlueZ，不暴露任意蓝牙命令接口。

## Windows

Windows 会独占保留键盘、鼠标及系统 HID 集合；电池通信通常需要访问厂商自定义 Usage Page 端点。在 HIDAPI 提供足够信息的情况下，`PermissionDenied`、`DeviceBusy` 与 `DeviceDisconnected` 会被清晰区分。

## Linux

访问 HIDRAW 通常需要配置最小权限范围的 udev 规则或用户组权限，不建议将整个程序以 root 身份作为默认运行方式。BlueZ 适配器与 D-Bus 权限会影响 BLE 枚举与读取。

## macOS

访问 HID 与蓝牙设备可能会触发系统隐私授权弹窗（如“输入监控”与“蓝牙”权限），请仅向宿主应用授予必要权限。BLE 行为受 CoreBluetooth 及外设是否已连接/已知的影响。

## BLE BAS 限制

`bluetooth` feature（默认启用，可通过 `--no-default-features` 关闭）仅列出操作系统适配器**已知**的外设，`list_devices()` 绝不会发起蓝牙扫描。因此，支持 BAS 的设备在系统未发现前可能不会出现在列表中。显式读取时可能会发起连接与 GATT 服务发现；已被其他传输占用或被其他中心设备连接的外设可能不可用。

BLE BAS 仅返回 Battery Level 百分比；在缺乏独立证据来源前，充电状态保持 `unknown`。不实现经典蓝牙/HFP 以及各系统私有的蓝牙缓存属性。

## 超时与报告长度限制

总截止时间（deadline）覆盖端点锁等待、冷却时间、协议交互与解析全过程。输入读取超时取“Provider 单次 I/O 超时”与“剩余总预算”的较小值。部分 HID Feature 操作可能在操作系统内核层阻塞且无法安全取消，本库如实说明这一限制而不加以掩盖。

HIDAPI 2.6 的 `DeviceInfo` 不暴露输入/输出/特征报告长度，因此真实枚举中这些字段可能为 `null`，依赖报告长度匹配的 Provider 在实机上可能无法匹配（合成回放可显式提供）。`interface_number` 与 `bus_type` 在 HIDAPI 提供时会跨平台填充；当设备提供 HID Report Descriptor 时，会自动识别无编号报告（Report ID 0）。

## 实机验证检查项

针对每个宣称支持的平台，建议依次验证：正常读数、充电/未充电（若支持）、权限失败行为、断开与重连、重复读取与冷却间隔、休眠/离线语义（若宣称）以及与 BLE 共存场景。记录证据时请务必去除序列号与原始系统路径。

## 厂商与操作系统传输说明

内置厂商 Provider 保留了 `WriteFile`、`SetFeature` 与 `SetOutputReport` 三种独立操作。由于 HIDAPI 2.6 未暴露 `SetOutputReport`，目前在 Windows 上通过原生 API 实现并在首次使用时延迟打开句柄；录制与回放均显式区分该操作。在未经独立实机验证前，这些设备在其他桌面平台上保持实验状态。

XInput 在运行时通过 `LoadLibraryExW`（限制 `LOAD_LIBRARY_SEARCH_SYSTEM32`）动态加载 `XInput1_4.dll`（回退 `XInput1_3.dll`），无需链接 XInput 导入库。XInput 仅限 Windows，返回 4 档离散电量且不换算为百分比。WOB Rainy75 与 Logitech HID++ 等实机回放证据存放在 [`tests/fixtures/vendors/`](../tests/fixtures/vendors/)。

---

相关文档：[架构设计](architecture.zh-CN.md) · [设备支持矩阵](support-matrix.zh-CN.md) · [录制与回放](capture-and-replay.zh-CN.md) · [返回 README](../README.zh-CN.md)
