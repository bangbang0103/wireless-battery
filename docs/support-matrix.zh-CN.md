# 设备支持矩阵

> **Language / 语言**: [English](support-matrix.md) | **简体中文**

除非针对特定设备与能力明确标注为 **verified（已实机验证）**，否则所有内置厂商适配均处于 **experimental（实验性）** 状态。仅标注“回放通过”表示合成 JSON 会话已通过本 Rust 端口严格回放，不代表独立硬件实机验证。WOB Rainy75 百分比与充/放电状态以及 Logitech GPW3 百分比/放电状态已通过本端口完成实机抓包验证。

| 厂商 | 设备 / 匹配规则 | 支持方式 | Provider ID | 百分比 | 充电状态 | 电压 | 证据状态 |
|---|---|---|---|---|---|---|---|
| Logitech（罗技） | GPW3 `046D:C547` 已实机验证；其余 HID++（`0x1004`/`0x1000`/`0x1001`/`0x1F20`）接收器为实验性 | 内置 Rust HID（已实机验证 / 实验性） | `logitech.hidpp` | **已验证：46% 实机 0x1004/function-1 读数 + 回放** | 放电已验证；充电/满电未实机验证 | 实验性（`0x1001`/`0x1F20`） | 实机抓包 + 合成正常/回退 |
| MCHOSE / AULA / Redragon | MCHOSE / AULA / Redragon BY 键盘（`258A:01BB`、`3554:FA09/FB40`、`41E4:2001`、`05AC:024F` if1） | 内置 Rust HID（实验性） | `mchose.keyboard` | 回放通过 | 回放通过 | 不支持 | 合成 |
| MCHOSE（迈从） | MCHOSE A7 Ultra，VID `5253` | 内置 Rust HID（实验性） | `mchose.a7_ultra` | 回放通过 | 回放通过 | 不支持 | 合成 |
| VXE | VXE R1 Star Flash `373B:10C9` | 内置 Rust HID（实验性） | `vxe.r1` | 回放通过 | 回放通过 | 回放通过 | 合成 |
| EWEADN（前行者） | EWEADN S9 Pro `372E:103D` / `1027` | 内置 Rust HID（实验性） | `eweadn.s9_pro` | 回放通过；充电时丢弃百分比 | 充电/放电回放通过 | 不支持 | 合成 |
| EWEADN（前行者） | EWEADN A5 Pro `258A:0138` | 内置 Rust HID（实验性） | `eweadn.a5_pro` | 回放通过 | 未知；不做 USB 启发式推断 | 不支持 | 合成 |
| DAREU（达尔优） | DAREU A950 Air V2 `260D:1175` | 内置 Rust HID（实验性） | `dareu.a950_air_v2` | 回放通过 | 回放通过；已实现满电映射 | 回放通过 | 合成 |
| MCHOSE（迈从） | MCHOSE V9 Turbo+ `3837:600A` | 内置 Rust HID（实验性） | `mchose.v9` | 回放通过 | 回放通过；已实现满电映射 | 不支持 | 合成 |
| MCHOSE（迈从） | MCHOSE K7 V2 Ultra+ `3837:1014` | 内置 Rust HID（实验性） | `mchose.k7_v2_ultra` | 回放通过 | 回放通过 | 不支持 | 合成 |
| MCHOSE（迈从） | MCHOSE A5 系列无线子设备，VID `3837` | 内置 Rust HID（实验性） | `mchose.a5_mouse` | 回放通过 | 回放通过 | 不支持 | 合成 |
| ASUS ROG（华硕玩家国度） | ROG Omni `0B05:1ACE`，静态键盘/鼠标通道 | 内置 Rust HID（实验性） | `asus.rog_omni` | 回放通过 | 回放通过 | 不支持 | 合成 |
| ASUS ROG（华硕玩家国度） | ROG Delta II / Pelta 及命名耳机 | 内置 Rust HID（实验性） | `asus.headset` | 回放通过 | 回放通过 | 不支持 | 合成 |
| TAIDU（钛度） | TAIDU TSG809 及已知 VID 家族 | 内置 Rust HID（实验性） | `taidu.mouse` | 回放通过 | 回放通过 | 不支持 | 合成 |
| IMECOO | IMECOO P1 `3554:FB34` | 内置 Rust HID（实验性） | `imecoo.p1` | 回放通过 | 回放通过 | 回放通过 | 合成 |
| ATK | ATK N9V2 Ultimate `373B:1262` | 内置 Rust HID（实验性） | `atk.n9v2` | 回放通过 | 回放通过；已实现满电映射 | 不支持 | 合成 |
| ATK | ATK A9 Air+ `373B:1278` | 内置 Rust HID（实验性） | `atk.a9_air` | 回放通过 | 回放通过 | 回放通过 | 合成 |
| Microsoft / XInput | Windows XInput 手柄槽位 | 系统 XInput API（实验性） | `os.xinput` | 不支持；仅返回 4 档离散电量 | 未知 | 不支持 | 合成离散档位回放 |
| CROCIRIS | CROCIRIS X7 `36F9:AA20` | 内置 Rust HID（实验性） | `crociris.x7` | 回放通过 | 回放通过 | 不支持 | 合成 |
| XIBERIA（西伯利亚） | XIBERIA T7 / T7 GT | 内置 Rust HID（实验性） | `xiberia.t7` | 回放通过 | 未知 | 不支持 | 合成 |
| Sony（索尼） | Sony DualSense / Edge | 内置 Rust HID（实验性） | `sony.dualsense` | 回放通过；设备上报 10% 步进 | 回放通过；已实现满电映射 | 不支持 | 合成 USB/蓝牙报告 |
| Rapoo（雷柏） | Rapoo VT3S `24AE:1411` | 内置 Rust HID（实验性） | `rapoo.vt3s` | 回放通过 | 回放通过 | 不支持 | 合成推送报告 |
| Razer（雷蛇） | 已知无线 Razer 鼠标与键盘 PID 列表 | 内置 Rust HID（实验性） | `razer.mouse` | 回放通过；0–255 线性换算 | 回放通过 | 不支持 | 合成特征报告事务 |
| WOB | WOB Rainy75 `320F:5088`，`FF1C:0092` | 内置 Rust HID（已实机验证） | `wob.rainy75` | **已验证：50% / 62% 实机读数 + 回放** | **已验证：放电（`0x00`）与充电（`0x01`）；其余值保守保持 `unknown`** | 不支持 | 实机抓包（放电 + 充电）+ 合成 |
| Razer（雷蛇） | Razer BlackShark V2 Pro `1532:0555` | 内置 Rust HID（实验性） | `razer.headset` | 回放通过 | 回放通过 | 不支持 | 合成 |

## 扩展厂商 HID Provider（`experimental`，均已通过严格合成回放）

以下新增的 **128 条内置 HID Provider** 参考自 [Wireless-Device-Battery-Monitor / BatteryHub](../NOTICE)（GPL-3.0-or-later），全部在 `tests/extended_vendor_replay_tests.rs` 中通过了零冲突静态匹配与严格线级回放验证；在独立完成实机抓包前均保持 `experimental` 状态。

### 耳机与音频设备

| 厂商 / 品牌 | 设备 | 支持方式 | Provider IDs |
|---|---|---|---|
| SteelSeries（赛睿） | Arctis 1 / 7 / 7+ / 7P / 7X / 9 / Pro Wireless、Arctis Nova 5 / 7 / Pro Wireless / Elite | 内置 Rust HID（实验性，合成回放） | `steelseries.arctis1`, `steelseries.arctis7`, `steelseries.arctis9`, `steelseries.arctis_pro_wireless`, `steelseries.nova_pro_wireless`, `steelseries.nova_family`, `steelseries.nova_elite` |
| HyperX（极度未知） | Cloud II / III / III S / Alpha / Alpha 2 / Flight / Stinger Core | 内置 Rust HID（实验性，合成回放） | `hyperx.cloud3`, `hyperx.cloud3_s`, `hyperx.cloud2`, `hyperx.cloud2_kingston`, `hyperx.cloud_alpha`, `hyperx.alpha2_haste`, `hyperx.cloud_flight`, `hyperx.stinger_core` |
| Corsair（海盗船） | Void / Void Pro / Void Elite / HS70 / HS80 / Virtuoso（含 Bragi 鼠标与键盘） | 内置 Rust HID（实验性，合成回放） | `corsair.void`, `corsair.bragi` |
| Razer（雷蛇） | BlackShark V2 HyperSpeed / V2 Pro 2020 / V3 / V3 X、Barracuda X / Pro | 内置 Rust HID（实验性，合成回放） | `razer.blackshark_v2_hs`, `razer.barracuda_x`, `razer.blackshark_v2_pro_2020`, `razer.blackshark_v3_x`, `razer.blackshark_v3`, `razer.barracuda_pro` |
| Logitech / Astro / Sony / ASUS | Logitech G435、Astro A50（Gen 3/4/5/X）与 A20/A30、Sony INZONE H5/H7/H9/Buds、ROG Cetra | 内置 Rust HID（实验性，合成回放） | `logitech.g435`, `astro.a50_modern`, `astro.legacy`, `sony.inzone`, `asus.rog_cetra` |
| 其他耳机与麦克风（Audeze / Endorfy / AKG / Turtle Beach / Jabra / JBL / Poly / Fractal / Alienware / 水月雨 / 迈从 / VGN / Darkrim / 闪克） | Audeze Maxwell、Endorfy Viro、AKG N9 Hybrid、Turtle Beach Stealth 700、Jabra Link、JBL Quantum、Poly BT600、Fractal Scape、Alienware AW920H、水月雨 GM01 Pro、迈从 V9 Pro2、VGN OmniSound、Darkrim Shadow、闪克 Maono PD200W | 内置 Rust HID（实验性，合成回放） | `audeze.maxwell`, `endorfy.viro`, `akg.n9_hybrid`, `turtlebeach.stealth700`, `jabra.gnp`, `jbl.hidv1`, `jbl.hidv3`, `plantronics.bt600`, `fractal.scape`, `alienware.aw920h`, `moondrop.gm01_pro`, `mchose.v9_pro2`, `vgn.omnisound`, `darkrim.shadow`, `maono.pd200w` |

### 鼠标与多设备接收器

| 厂商 / 品牌 | 设备 | 支持方式 | Provider IDs |
|---|---|---|---|
| ASUS ROG 与 SteelSeries | ROG Harpe/Gladius/Keris/Chakram/Azoth/Falchion、Aerox 3/5/9、Prime/Rival 无线系列 | 内置 Rust HID（实验性，合成回放） | `asus.rog_peripheral`, `steelseries.aerox`, `steelseries.rival`, `steelseries.rival_pro` |
| DriverCore / Nordic 8K 系列（Glorious / Hitscan / G-Wolves / Arbiter / Lamzu / WLmouse / Ninjutso / Sprime / Pulsar / VAXEE / Endgame Gear） | Glorious Model O/D/I、Hitscan Hyperlight、G-Wolves、Arbiter Akitsu、Lamzu Maya/Atlantis/Thorn、WLmouse Beast X、Ninjutso Sora V2/Ten、Sprime PM1、Pulsar X2 V3、VAXEE、Endgame Gear OP1w/XM2w 4K | 内置 Rust HID（实验性，合成回放） | `glorious.mouse`, `hitscan.mouse`, `gwolves.mouse`, `arbiter.akitsu`, `lamzu.mouse`, `wlmouse.mouse`, `ninjutso.sora_v2`, `sprime.pm1`, `ninjutso.ten`, `pulsar.x2_v3`, `vaxee.mouse`, `endgame.4k` |
| CompX 0x08 与 MCHOSE / ATK（Kysona / Chimera / Cherry Xtrfy / Corsair / 迈从 / ATK） | Kysona、Chimera V2、Cherry Xtrfy、Corsair M75/Katar（CompX）、迈从 A5/G3 V2/A7/L7、ATK F1/X1/Z1/蜻蜓系列 | 内置 Rust HID（实验性，合成回放） | `kysona.mouse`, `chimera.v2`, `cherry.xtrfy`, `corsair.compx`, `mchose.a5_gen1`, `mchose.g3_v2`, `mchose.modern_mouse`, `atk.zero`, `atk.bitmouse_nk`, `atk.nano_8k` |
| 客制化与各品牌无线鼠标（Finalmouse / Scyrox / RAWM / 卓威 / 酷冷至尊 / NZXT / 微星 / Fantech / Raigor / 中颖 / RK / 达摩鲨 / 机械师 / Niceboy / 摩豹 / Xenta / 多彩 / 黑爵 / 因科特 / 荣元 / 达尔优 / Zenith / 雷柏 / Press Play） | Finalmouse ULX、Scyrox V6/V8、RAWM、卓威 EC-CW、酷冷至尊、NZXT Lift Elite、微星 Clutch、Fantech、Raigor、中颖方案、RK、达摩鲨、机械师、Niceboy、摩豹、Xenta、多彩 M800 Pro、黑爵 AJ139/AJ159/AJ179、因科特、荣元方案、达尔优 Togran、Zenith Pro、雷柏 VT/VT3 Pro、Press Play Icarus | 内置 Rust HID（实验性，合成回放） | `finalmouse.mouse`, `scyrox.v12m`, `rawm.mouse`, `zowie.ec_cw`, `coolermaster.mouse`, `nzxt.lift_elite`, `msi.mouse`, `msi.gm41`, `fantech.xd5`, `raigor.mouse`, `sinowealth.receiver`, `royal_kludge.mg6_pro`, `darmoshark.m3`, `darmoshark.nrf`, `machenike.l7`, `niceboy.mw400`, `motospeed.x6`, `xenta.mouse`, `delux.m800_pro`, `ajazz.aj179`, `ajazz.aj_series`, `incott.mouse`, `rongyuan.receiver`, `dareu.togran`, `zenith.pro`, `rapoo.vt`, `rapoo.legacy_vt3_pro`, `pressplay.icarus` |

### 键盘与键鼠套件

| 厂商 / 品牌 | 设备 | 支持方式 | Provider IDs |
|---|---|---|---|
| Keychron / Lemokey | Keychron M 系列 / Lemokey 鼠标、UltraLink 8K 接收器、Keychron Q/K/V/P/Max/Pro QMK 无线键盘 | 内置 Rust HID（实验性，合成回放） | `keychron.nape`, `keychron.legacy_mouse`, `keychron.ultralink`, `keychron.launcher_mouse`, `keychron.qmk_keyboard` |
| Cherry / Akko / NuPhy / MCHOSE / DAREU | Cherry MX 3.0S/3.1、Akko/MonsGeek Dash、NuPhy Air/WH80、迈从 GLW 与 K87 V3、达尔优 A84 | 内置 Rust HID（实验性，合成回放） | `cherry.mx3s`, `akko.dash`, `nuphy.wh80`, `mchose.glw_keyboard`, `mchose.k87_v3`, `dareu.a84` |
| EVision / 狼蛛 AULA / Sonix / VIA / Semico | EVision 方案 ATK/红龙/黑爵/狼蛛键鼠、狼蛛 HERO 68 / SC580、Sonix / VIA / Semico 无线键盘 | 内置 Rust HID（实验性，合成回放） | `evision.atk_keyboard`, `atk.z87`, `evision.redragon_m999`, `evision.ajazz_ak992`, `evision.aula_mouse`, `aula.hero68`, `aula.sc580`, `sonix.keyboard`, `via.dongle`, `semico.keyboard` |

### 游戏手柄

| 厂商 / 品牌 | 设备 | 支持方式 | Provider IDs |
|---|---|---|---|
| Nintendo / Sony / Valve / 盖世小鸡 / 飞智 / 八位堂 / 北通 | Nintendo Switch Pro / Joy-Con、Sony DualShock 4、Valve Steam Controller、盖世小鸡 Nova/Cyclone 2/SapJoy、飞智冰原狼/八爪鱼/黑武士、八位堂 Ultimate/Pro、北通手柄 | 内置 Rust HID（实验性，合成回放） | `nintendo.switch_pro`, `sony.dualshock4`, `valve.steam_controller`, `gamesir.nova`, `gamesir.cyclone2`, `gamesir.sapjoy`, `flydigi.controller`, `eightbitdo.vendor`, `eightbitdo.dinput`, `beitong.controller` |

## 标准协议

| 厂商 / 标准 | 设备 | 支持方式 | 生命周期 | 百分比 | 充电状态 | 平台 | 限制说明 |
|---|---|---|---|---|---|---|---|
| Bluetooth SIG (`bluetooth.bas`) | 已知 BLE 外设 | 只读 BLE BAS (`0x2A19`) | experimental | 协议已实现；未实机验证 | 未知 / 不支持 | 默认启用（支持 Windows、Linux BlueZ、macOS） | 仅枚举已知外设；显式读取时可能发起连接；不含经典蓝牙/HFP |

## 暂缓接入的非 HID 与双句柄拆分集合协议

上游工具中的以下设备来源因架构边界暂未迁入单会话 HID 层：

- **需同时持有两个 HID 集合句柄的拆分集合设备**：`Endgame Gear WE`、`Areson`、`DAREU EM901X / SGM50W`、`ZOWIE DW`、`Wraith W1`。
- **非 HID / 系统专有 / 外部进程来源**：BLE 主动广播监听（AirPods / Google Fast Pair）、经典蓝牙 RFCOMM（Samsung Galaxy Buds / Nothing Ear）、Windows Phone Link 本地 JSON 缓存、虚拟串口 COM（`atk_serial`）、厂商私有 BLE GATT（`gamesir_ble`）、Windows PnP 通用蓝牙缓存属性，以及外部 `headsetcontrol.exe` 进程委托。

## 能力证据说明

设备包生命周期与单能力证据彼此独立。百分比通过回放不代表充电、满电、低电量、休眠或离线行为已经过验证。欢迎补充满电、低电与休眠样本，但不作为贡献合并阻塞项。`fixtures/synthetic/` 下的测试包仅用于测试，绝不进入真实设备支持。

---

相关文档：[实机验证清单](hardware-verification.zh-CN.md) · [设备贡献指南](contributing-device.zh-CN.md) · [平台支持](platform-support.zh-CN.md) · [返回 README](../README.zh-CN.md)
