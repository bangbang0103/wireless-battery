# 内置协议来源与验证记录

> **Language / 语言**: [English](builtin-providers.md) | **简体中文**

## 协议参考与署名

`src/providers/vendors/` 与 `src/providers/os/` 下的内置 Provider 共覆盖 152 条厂商及系统协议路径（另含可选 Bluetooth LE BAS）。相关协议实现参考了社区公开文档与开源实现（GPL-3.0-or-later 与 MIT 许可证署名详见根目录 [NOTICE](../NOTICE)），包括：

- https://github.com/krisgfx9/Wireless-Device-Battery-Monitor （GPL-3.0-or-later；BatteryHub HID 设备协议实现）
- https://github.com/mitsimi/mchose-tray/blob/main/PROTOCOL.md
- https://github.com/Rapou7/offline-mchose/blob/master/REVERSE_ENGINEERING_MCHOSE.md

## 证据状态

`fixtures/vendors/` 下除 `*_real.json` 实机抓包外的 JSON 文件，以及 `tests/extended_vendor_replay_tests.rs` 中的程序化严格回放测试集，均为依据已知字节布局构造的 `synthetic` 回归会话。它们用于验证全部内置 Provider 的无冲突静态匹配、请求构造、校验和、过滤、解析与归一化逻辑，但不构成独立的实机硬件验证。

因此，除非通过本库单独完成实机验证，否则内置 Provider 默认保持 `experimental`。

## 保守协议语义约定

- 被动发现阶段绝不发送配对查询，也不探测未知 PID。
- 所有共享厂商 VID（`0x046D`、`0x1532`、`0x0B05`、`0x320F`、`0x3554`、`0x373B`、`0x258A`、`0x260D`、`0x372E`、`0x3837`、`0x24AE`、`0x046A`、`0x1B1C`、`0x054C` 等）均使用互不相交的 `(vid, pid, usage_page, usage, interface_number)` 匹配谓词，确保不会出现 `MatchStatus::Conflict`。
- ROG Omni 仅暴露静态键盘/鼠标通道，动态配对列表发现暂缓实现。
- XInput（`os.xinput`）返回 `critical/low/medium/full` 且 `percentage=null`，不伪造百分比；有线模式不推断充电或满电状态。
- EWEADN A5 Pro 不根据独立有线端点的存在与否推断充电状态。
- Rapoo 使用有界读取与核心旧值回退机制，不长期持有持久 Provider 会话。
- Logitech HID++（`logitech.hidpp`）从响应第 6 字节（`charging_status` / `battery_status`）解析充电状态，支持 `0x1004` / `0x1000` / `0x1001` / `0x1F20`，并在收到 HID++ `0xFF` 错误帧时快速失败。
- Razer（`razer.mouse`）统一采用 0–255 单调线性换算，并将匹配限定在厂商 Usage Page（`>= 0xFF00`）。
- DualSense（`sony.dualsense`）与 DualShock 4（`sony.dualshock4`）通过只读特征/输入报告（`get_feature` / `get_input_report`）读取状态，并将未知状态码保守保留为 `unknown`。
- 在 HIDAPI 2.6 无法枚举报告长度的场景下省略长度匹配；优先使用跨平台 `interface_number` 与 `bus_type` 替代系统专属路径子串，并显式排除已知冲突 PID。
- 非 HID 或双句柄拆分集合协议（BLE 主动广播监听、经典蓝牙 RFCOMM、Phone Link 本地缓存、虚拟串口 COM、厂商 BLE GATT、外部 `headsetcontrol.exe` 进程，或需同时打开两个 HID 集合句柄的设备）暂不强行模拟。

## 当前实机硬件验证

WOB Rainy75（`wob.rainy75`）在 Windows 上实机分别返回 `50%` 放电中（`frame[9]=0x00`，`fixtures/vendors/wob/rainy75_real.json`）与 `62%` 充电中（`frame[9]=0x01`，`fixtures/vendors/wob/rainy75_charging_real.json`），两份脱敏录制会话均已通过严格回放，百分比与充/放电状态均为 `verified`（其余未验证值保守保持 `unknown`）。Logitech GPW3（`logitech.hidpp`）在 Windows 上通过索引 1 与 `0x1004` function-1 回退路径实机返回 `46%`、放电中，脱敏录制的 `fixtures/vendors/logitech/hidpp_real.json` 已通过严格回放，该型号/路径为 `verified`。其余 Provider 保持 `experimental`。

---

相关文档：[实机验证清单](../docs/hardware-verification.zh-CN.md) · [设备支持矩阵](../docs/support-matrix.zh-CN.md) · [协议来源说明](README.zh-CN.md) · [返回 README](../README.zh-CN.md)
