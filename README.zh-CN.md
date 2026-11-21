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

> **支持的产品矩阵**：完整设备列表见 [**设备支持矩阵（`docs/support-matrix.zh-CN.md`）**](docs/support-matrix.zh-CN.md)，内含按**厂商、设备、支持方式**分类的完整表格。

**当前内置 152 条厂商与操作系统协议路径**（另含默认启用的 Bluetooth LE BAS），覆盖 Logitech / Astro、Razer、SteelSeries、Corsair、HyperX、ASUS ROG、Sony、Nintendo、Valve、GameSir、Flydigi、8BitDo、Beitong、Keychron、Cherry、Akko、NuPhy、AULA、MCHOSE、ATK / VXE / VGN、DAREU、EWEADN、Rapoo、WOB、Glorious、Lamzu、WLmouse、Pulsar、VAXEE、Ninjutso、Endgame Gear、Finalmouse、ZOWIE、Audeze、JBL、Jabra、Turtle Beach、AKG、Fractal、Alienware、Moondrop、Maono、XInput 等设备，完整列表见[支持矩阵](docs/support-matrix.zh-CN.md)。所有已移植设备默认均已启用；如在特定型号或固件上遇到读数异常，欢迎直接提交 Issue 反馈。

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
wireless-battery probe <device-id> --listen 3000
wireless-battery probe <device-id> --report-id 0 --send "5A A5 01 02 00"
```

所有已内置的厂商与系统 Provider 默认均已启用，无需额外开关。`list` 会返回所有枚举到的端点（含未匹配与冲突端点）。默认 `list` 保持完全被动，电量显示为 `not read`；`list --read` 仅显式读取当前可读的设备，其 JSON 条目在原始设备字段基础上附加 `reading` 与 `read_error`，单个设备失败不会导致整个列表失败。JSON 仅输出到 stdout，警告与进度信息输出到 stderr。

对于未支持的 HID 设备，可以直接使用 `probe` 对任意端点进行被动监听（`--listen`）或发送自定义 Output / Feature 报文（`--send`、`--send-feature`、`--get-feature`、`--get-input`）以辅助逆向分析。

设备 ID 是不可逆哈希值而非序列号，主要面向单次会话使用。在操作系统路径稳定的平台上，其确定性足以支持后续 `read` 命令调用，但不保证跨重启、跨机器或路径重分配后保持不变。

退出码约定：`0` 成功；`2` 参数错误；`3` 设备不存在或不支持；`4` 权限不足；`5` 超时或传输错误；`6` 响应/设备包/Fixture 校验失败；`70` 内部错误。

## 回放示例

```bash
cargo run --quiet -- replay \
  --fixture tests/fixtures/synthetic/feature_normal.json \
  --fixture tests/fixtures/synthetic/feature_unknown.json \
  --fixture tests/fixtures/synthetic/bluetooth_bas.json
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

## 未支持的设备怎么办

如果设备显示为 `unmatched` 或电量读取为 `null` / `unknown`，可按以下 3 步处理：

1. **先确认连接模式与端点**
   - 运行 `wireless-battery list --json` 查看所有端点。
   - 不少多模外设或手柄（如飞智、八爪鱼等）在纯 **XInput 模式**（`045E:028E`）下会被 Windows 保留，且多数第三方手柄不通过 XInput 上报电量；尝试切换至 **2.4G 接收器、有线、DInput / 厂商模式** 或 **BLE 模式**（支持标准 BAS `0x2A19` 可直接读取），再重新 `list` 确认是否出现厂商自定义 HID 端点（通常 `usage_page >= 0xFF00`）。

2. **用 `probe` 探测报文（供提交 Issue 或自行适配）**
   - 监听设备主动上报：`wireless-battery probe <device-id> --listen 5000`（插拔充电线或操作设备观察变化）。
   - 发送探测指令（可结合 **Wireshark + USBPcap** / `usbmon` 抓到的官方查询包）：
     `wireless-battery probe <device-id> --report-id 0 --send "5A A5 ..."` 或 `--get-feature 0x03`，对比**放电**与**充电**状态下的原始字节。

3. **提交 Issue 或 PR**
   - **提 Issue**：附上设备型号、连接模式、`wireless-battery list --json` 输出，以及 `probe` 抓到的放电/充电报文片段。
   - **提 PR**：在 `src/providers/vendors/` 中参考现有 `FixedSpec` 或 `Provider` 实现新增协议，补充测试并通过 `cargo test --locked`。

### 直接复制给 AI Agent 的排查 / 逆向 Prompt

你可以将下面这段 Prompt 直接发给 AI Agent（如 Antigravity、Claude Code、Cursor、Codex 等），让它用 `wireless-battery` CLI 帮你检测设备、探测原始报文，并在有源码时完成适配：

```text
请帮我使用 wireless-battery CLI 检测已连接的无线外设，并对未支持的设备进行报文探测与协议分析。

请严格按以下步骤执行：
0. **准备 CLI 工具**：
   - 先检查当前目录或 PATH 是否已有 `wireless-battery` / `wireless-battery.exe`；若当前在项目源码仓库中，也可用 `cargo run --locked --` 代替。若都没有，请引导我从项目 Releases 下载对应系统的预编译可执行文件。

1. **枚举与读取**：
   - 运行 `wireless-battery list --json` 与 `wireless-battery list --read --json`，汇总所有检测到的设备（名称、VID/PID、`usage_page`、`usage`、`interface_number`、`match_status`、`provider_id`、电量百分比与充电状态）。
   - 如果目标设备只出现了 Windows 保留的标准端点（例如 XInput `045E:028E`、`usage_page=1, usage=5`）而没有厂商自定义 HID 端点（通常 `usage_page >= 0xFF00`），请先引导我切换设备的连接模式（2.4G 接收器 / 有线 / DInput 厂商模式 / BLE 模式）后重新执行 `list --json`。

2. **使用 `probe` 探测原始报文（仅针对未匹配或读数异常的厂商 HID 端点）**：
   - 先运行 `wireless-battery probe <device-id> --listen 5000`，引导我在期间插拔充电线或操作设备，观察是否有主动上报的 Input 报文。
   - 如有候选查询指令（或结合 Wireshark + USBPcap / usbmon 抓到的官方查询包），使用 `wireless-battery probe <device-id> --report-id <id> --send "<hex>"` 或 `--get-feature <id>` 分别在**放电**和**充电**两种状态下收集原始响应字节，并对比字段含义（严禁将电压或离散档位强行猜成百分比，未知哨兵值必须保持 `unknown`）。

3. **输出与下一步**：
   - 若当前只有可执行文件（无源码仓库）：请整理好「设备型号 + 连接模式 + `list --json` 端点信息 + 放电/充电态 `probe` 请求与响应报文」，方便我直接提交到 GitHub Issue。
   - 若当前就在源码仓库中：请在 `src/providers/vendors/` 下参考现有 `FixedSpec` 或 `Provider` 实现新增协议并注册，在 `tests/fixtures/vendors/` 补充回放测试，更新 `docs/support-matrix.zh-CN.md` 与 `docs/support-matrix.md`，最后运行 `cargo test --locked` 验证全部通过。
```

## 文档导航

- [架构设计](docs/architecture.zh-CN.md)
- [设备支持矩阵](docs/support-matrix.zh-CN.md)
- [平台支持与限制](docs/platform-support.zh-CN.md)
- [设备贡献指南](docs/contributing-device.zh-CN.md)
- [原生系统托盘](tray/README.zh-CN.md)

## 参与贡献

请从[设备贡献指南](docs/contributing-device.zh-CN.md)开始。简单的固定协议优先使用 `FixedSpec`，需要动态协商的逻辑使用聚焦的 `Provider` 在 `src/providers/vendors/` 下实现。

本项目基于 [GNU GPL-3.0 或更高版本](LICENSE)开源。
