# Wireless Battery

> **Language / 语言**: [English](README.md) | **简体中文**

一个同步 Rust 库与命令行工具，用于通过 HID 保守地读取外设电量，并默认支持只读 Bluetooth LE Battery Service（BAS）。

本项目将**协议安全性**和**可复现性**置于设备数量之上：

- 被动发现阶段绝不打开 HID 会话、不发送任何报告、不启动蓝牙扫描；
- 未知值保持未知——超时绝不会变成 0%，电压或离散档位也不会被伪造成百分比；
- 每个声明式协议和内置 Provider 都可以通过精简 JSON Fixture 在无硬件环境下严格回放；
- 合成（synthetic）协议与真实发现及支持矩阵完全隔离；
- 正常读数与日志中绝不暴露原始系统路径、序列号或原始 HID 字节。

## 当前支持情况

> **支持的产品矩阵**：完整设备列表见 [**设备支持矩阵（`docs/support-matrix.zh-CN.md`）**](docs/support-matrix.zh-CN.md)，内含按**厂商、设备、支持方式**（内置 Rust HID、系统 XInput API、BLE BAS；已实机验证 / 实验性）分类的完整表格。

**当前内置 152 条厂商与操作系统协议路径**（另含默认启用的 Bluetooth LE BAS），覆盖 Logitech / Astro、Razer、SteelSeries、Corsair、HyperX、ASUS ROG、Sony、Nintendo、Valve、GameSir、Flydigi、8BitDo、Beitong、Keychron、Cherry、Akko、NuPhy、AULA、MCHOSE、ATK / VXE / VGN、DAREU、EWEADN、Rapoo、WOB、Glorious、Lamzu、WLmouse、Pulsar、VAXEE、Ninjutso、Endgame Gear、Finalmouse、ZOWIE、Audeze、JBL、Jabra、Turtle Beach、AKG、Fractal、Alienware、Moondrop、Maono、XInput 等设备，完整列表见[支持矩阵](docs/support-matrix.zh-CN.md)。除非在矩阵中针对具体设备与能力明确标注为 `verified`，否则均处于 `experimental` 状态。每条内置路径都具备严格线级合成回放用例，但合成回放不代表真实硬件验证。WOB Rainy75 与 Logitech GPW3 的实机验证记录见[实机验证清单](docs/hardware-verification.zh-CN.md)。仓库中还保留了一个虚构的声明式鼠标包用于解释器测试，但绝不参与真实发现。

内置厂商协议实现参考了 [Wireless-Device-Battery-Monitor / BatteryHub](NOTICE)（GPL-3.0-or-later）及 [Voltide](NOTICE)（MIT），完整署名见 [NOTICE](NOTICE)。

标准 HID 电池 Usage 与经典蓝牙/HFP 电量报告预留给后续版本。BLE 支持仅实现 BAS Battery Level（`0x2A19`）只读查询，不包含配对、通知订阅、充电状态推断或跨传输设备去重。

## 构建

支持 Rust 1.76 及以上版本。`Cargo.lock` 锁定依赖版本以维持 MSRV（`--locked`）。`cli` 与 `bluetooth` feature 默认均已开启；纯库消费者可通过 `--no-default-features` 关闭默认特性，避免编译 CLI（`clap`、`walkdir`）与 BLE（`btleplug`、`tokio`、`uuid`）依赖，也可按需单独开启 `--features cli` 或 `--features bluetooth`。

```bash
cargo build --locked
cargo build --locked --no-default-features
cargo test --locked --all-features
cargo run --locked -- validate
```

系统依赖与权限限制详见[平台支持](docs/platform-support.zh-CN.md)。

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

架构细节、macOS `.app` 打包及 BLE 支持说明详见 [托盘说明文档](tray/README.zh-CN.md)。

## 与 AI 协作

你可以直接将以下 Prompt 复制给 AI Agent（如 Antigravity、Claude Code、Cursor、Codex 等），让它通过 `wireless-battery` 命令行工具 **一键检测已连接设备、收集脱敏实机抓包，并逆向/适配未知设备的电量与充放电状态**：

```text
请帮我使用 wireless-battery CLI 检测、收集并逆向当前已连接无线外设的电量百分比与充/放电状态。

0. **准备 CLI 工具**：
   - 先检查当前环境是否已有可用的 `wireless-battery` / `wireless-battery.exe` 二进制文件；如果没有，请提示或协助我从项目 Releases 下载对应平台的可执行文件（如 Windows 下的 `wireless-battery.exe`）并放入当前目录或加入 PATH（若当前就在源码仓库中，也可使用 `cargo run --locked --` 代替）。
   - 后续所有命令均直接调用该二进制（以下以 `wireless-battery` 代指，Windows 下可替换为 `.\wireless-battery.exe`）。

1. **一键检测与枚举**：
   - 运行 `wireless-battery list --json --include-experimental` 被动枚举所有 HID/BLE 端点，再运行 `wireless-battery list --read --json --include-experimental` 读取当前可读设备的电量与充电状态。
   - 汇总展示所有检测到的设备信息（名称、VID/PID、`usage_page`、`usage`、`interface_number`、`match_status`、`support_status`、`provider_id`、当前电量百分比与充电状态）。如有匹配冲突或异常设备，补充运行 `wireless-battery diagnose <device-id> --json --include-experimental`。

2. **收集实机样本（已匹配 / 实验性设备）**：
   - 若目标设备已匹配但需要实机验证，请引导我分别在**放电（无线使用）**和**充电（插线/底座充电）**两种状态下采集脱敏 Fixture（注意绝不覆盖已有文件）：
     `wireless-battery capture --device <device-id> --output fixtures/<vendor_device>/discharging.json --include-experimental --timeout-ms 5000`
     `wireless-battery capture --device <device-id> --output fixtures/<vendor_device>/charging.json --include-experimental --timeout-ms 5000`
   - 使用 `wireless-battery replay --fixture ...` 和 `wireless-battery validate` 验证采集结果。

3. **逆向与协议适配（未匹配或充放电/电量不完整设备）**：
   - 若目标设备处于 `unmatched`，或电量/充放电状态缺失或不准确，请结合 `list --json` 中的 HID 端点元数据（VID/PID、`usage_page`、`usage`、`interface_number`），对照项目源码中的现有协议（`src/providers/vendors/`、`docs/contributing-device.zh-CN.md`、`schemas/`）。
   - 提出保守、安全的 HID 查询与字段解析方案（严禁将电压或离散档位伪造成百分比，未知哨兵值必须映射为 `unknown`）：若属于最多 4 轮静态 Feature/Output/Input 请求-响应协议，优先编写声明式 YAML 包（`packages/vendors/<vendor>/<device>.yaml`）；若需动态逻辑，则在 `src/providers/vendors/` 下实现聚焦的 Rust Provider。
   - 引导我分别在**放电**与**充电**状态下实测验证原始报文与状态变化；确认无误后，补充正常/截断/非法字段 Fixture、`provenance/<device>.md` 来源记录，并更新 `docs/support-matrix.zh-CN.md` 与 `docs/support-matrix.md`。
   - 最后通过 `replay`、`validate` 以及测试套件确认全部通过。
```

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
