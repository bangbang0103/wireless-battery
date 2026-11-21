# Wireless Battery

> **Language / 语言**: [English](README.md) | **简体中文**

一个同步 Rust 库与命令行工具，用于通过 HID 保守地读取外设电量，并可选支持只读 Bluetooth LE Battery Service（BAS）。

本项目将**协议安全性**和**可复现性**置于设备数量之上：

- 被动发现阶段绝不打开 HID 会话、不发送任何报告、不启动蓝牙扫描；
- 未知值保持未知——超时绝不会变成 0%，电压或离散档位也不会被伪造成百分比；
- 每个声明式协议和内置 Provider 都可以通过精简 JSON Fixture 在无硬件环境下严格回放；
- 合成（synthetic）协议与真实发现及支持矩阵完全隔离；
- 正常读数与日志中绝不暴露原始系统路径、序列号或原始 HID 字节。

## 当前支持情况

**当前内置 152 条厂商与操作系统协议路径**（另含可选 Bluetooth LE BAS），覆盖 Logitech / Astro、Razer、SteelSeries、Corsair、HyperX、ASUS ROG、Sony、Nintendo、Valve、GameSir、Flydigi、8BitDo、Beitong、Keychron、Cherry、Akko、NuPhy、AULA、MCHOSE、ATK / VXE / VGN、DAREU、EWEADN、Rapoo、WOB、Glorious、Lamzu、WLmouse、Pulsar、VAXEE、Ninjutso、Endgame Gear、Finalmouse、ZOWIE、Audeze、JBL、Jabra、Turtle Beach、AKG、Fractal、Alienware、Moondrop、Maono、XInput 等设备，完整列表见[支持矩阵](docs/support-matrix.zh-CN.md)。除非在矩阵中针对具体设备与能力明确标注为 `verified`，否则均处于 `experimental` 状态。每条内置路径都具备严格线级合成回放用例，但合成回放不代表真实硬件验证。WOB Rainy75 与 Logitech GPW3 的实机验证记录见[实机验证清单](docs/hardware-verification.zh-CN.md)。仓库中还保留了一个虚构的声明式鼠标包用于解释器测试，但绝不参与真实发现。

内置厂商协议实现参考了 [Wireless-Device-Battery-Monitor / BatteryHub](NOTICE)（GPL-3.0-or-later）及 [Voltide](NOTICE)（MIT），完整署名见 [NOTICE](NOTICE)。

标准 HID 电池 Usage 与经典蓝牙/HFP 电量报告预留给后续版本。可选的 BLE 支持仅实现 BAS Battery Level（`0x2A19`）只读查询，不包含配对、通知订阅、充电状态推断或跨传输设备去重。

## 构建

支持 Rust 1.76 及以上版本。`Cargo.lock` 锁定依赖版本以维持 MSRV（`--locked`）。默认启用 `cli` feature；纯库消费者可通过 `--no-default-features` 禁用它，避免编译仅 CLI 使用的依赖（`clap`、`walkdir`）。

```bash
cargo build --locked
cargo build --locked --no-default-features
cargo test --locked --all-features
cargo run --locked -- validate
```

启用可选的 BLE BAS 支持：

```bash
cargo build --locked --features bluetooth
```

`bluetooth` feature 默认关闭。系统依赖与权限限制详见[平台支持](docs/platform-support.zh-CN.md)。

## Rust API

```rust
use wireless_battery::{BatteryClient, ClientConfig};

let client = BatteryClient::new(ClientConfig::default())?;
let devices = client.list_devices()?;
for device in &devices {
    println!("{}: {} ({:?})", device.id, device.name, device.match_status);
}

if let Some(device) = devices.iter().find(|device| device.readable) {
    let reading = client.read_battery(&device.id.0)?;
    println!("{:?}% ({:?})", reading.percentage, reading.freshness);
}
# Ok::<(), wireless_battery::BatteryError>(())
```

实验性 Provider 需要显式开启 `ClientConfig::allow_experimental` 或 `ReadOptions::allow_experimental`。只有在启用缓存且调用方显式设置 `ReadOptions::allow_stale` 时，刷新失败才会返回缓存的旧值；旧值保留原始 `observed_at` 时间戳，并将 `freshness` 标记为 `stale`。

`ReadOptions::timeout` 是整次调用的总时间预算。设备包中的 `timeout_ms` 是单次 I/O 偏好值，受限于剩余总预算。部分平台的 Feature 报告操作无法被强制取消。

## 命令行（CLI）

```bash
wireless-battery list --json
wireless-battery list --read
wireless-battery list --read --json --timeout-ms 5000
wireless-battery read <device-id> --json --timeout-ms 1000
wireless-battery diagnose <device-id> --json
wireless-battery capture --device <device-id> --output fixture.json --timeout-ms 5000
wireless-battery replay --fixture fixtures/synthetic/feature_normal.json
wireless-battery validate
```

在 `list`、`read`、`diagnose` 或 `capture` 上使用 `--include-experimental` 可显式启用实验性 Provider。`list` 会返回所有枚举到的端点（含未匹配与冲突端点）。默认 `list` 保持完全被动，电量显示为 `not read`；`list --read` 仅显式读取当前可读的设备，其 JSON 条目在原始设备字段基础上附加 `reading` 与 `read_error`，单个设备失败不会导致整个列表失败。JSON 仅输出到 stdout，警告与进度信息输出到 stderr。

设备 ID 是不可逆哈希值而非序列号，主要面向单次会话使用。在操作系统路径稳定的平台上，其确定性足以支持后续 `read` 命令调用，但不保证跨重启、跨机器或路径重分配后保持不变。

退出码约定：`0` 成功；`2` 参数错误；`3` 设备不存在或不支持；`4` 权限不足；`5` 超时或传输错误；`6` 响应/设备包/Fixture 校验失败；`70` 内部错误。

## 回放示例

```bash
cargo run --quiet -- replay \
  --fixture fixtures/synthetic/feature_normal.json \
  --fixture fixtures/synthetic/feature_unknown.json \
  --fixture fixtures/synthetic/bluetooth_bas.json
```

回放传输层严格校验操作顺序、Report ID、完整请求字节、响应消费、多余调用、最终稳定字段以及错误类别。详见[录制与回放](docs/capture-and-replay.zh-CN.md)。

## 原生系统托盘（`tray/`）

[`tray/`](tray/README.zh-CN.md) 是一个独立于根库与 CLI 的 Cargo 工作区，采用 **共享 Rust 核心（`tray-core`） + 各平台极薄原生 UI 壳** 架构（Windows 为 Win32 + DWM Acrylic + Direct2D；macOS 为 SwiftUI `MenuBarExtra` + `tray-ffi` 静态库），避免 GUI 依赖污染核心库与 CLI。

```bash
cd tray
cargo test --locked
cargo build --locked --release -p wireless-battery-tray   # Windows 原生托盘
```

架构细节、macOS `.app` 打包及可选 BLE 支持详见 [托盘说明文档](tray/README.zh-CN.md)。

## 文档导航

- [架构设计](docs/architecture.zh-CN.md)
- [设备支持矩阵](docs/support-matrix.zh-CN.md)
- [平台支持与限制](docs/platform-support.zh-CN.md)
- [录制与回放规范](docs/capture-and-replay.zh-CN.md)
- [设备贡献指南](docs/contributing-device.zh-CN.md)
- [实机验证清单](docs/hardware-verification.zh-CN.md)
- [原生系统托盘](tray/README.zh-CN.md)
- [Fixture 说明](fixtures/README.zh-CN.md) · [协议来源与证据说明](provenance/README.zh-CN.md)

## 参与贡献

请从[设备贡献指南](docs/contributing-device.zh-CN.md)开始。简单的固定协议应仅需 YAML 包、Fixture、来源记录与支持矩阵元数据；动态协商逻辑应使用聚焦的 Rust Provider 实现，而不应膨胀 YAML 脚本能力。

本项目基于 [GNU GPL-3.0 或更高版本](LICENSE)开源。
