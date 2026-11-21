# 设备支持矩阵

> **Language / 语言**: [English](support-matrix.md) | **简体中文**

除非针对特定设备与能力明确标注为 **verified（已实机验证）**，否则所有内置厂商适配均处于 **experimental（实验性）** 状态。仅标注“回放通过”表示合成 JSON 会话已通过本 Rust 端口严格回放，不代表独立硬件实机验证。WOB Rainy75 百分比与充/放电状态以及 Logitech GPW3 百分比/放电状态已通过本端口完成实机抓包验证。

| Provider ID | 代表设备 / 匹配规则 | 百分比 | 充电状态 | 电压 | 证据状态 |
|---|---|---|---|---|---|
| `logitech.hidpp` | GPW3 `046D:C547` 已实机验证；其余 HID++（`0x1004`/`0x1000`/`0x1001`/`0x1F20`）接收器为实验性 | **已验证：46% 实机 0x1004/function-1 读数 + 回放** | 放电已验证；充电/满电未实机验证 | 实验性（`0x1001`/`0x1F20`） | 实机抓包 + 合成正常/回退 |
| `mchose.keyboard` | MCHOSE / AULA / Redragon BY 键盘（`258A:01BB`、`3554:FA09/FB40`、`41E4:2001`、`05AC:024F` if1） | 回放通过 | 回放通过 | 不支持 | 合成 |
| `mchose.a7_ultra` | MCHOSE A7 Ultra，VID `5253` | 回放通过 | 回放通过 | 不支持 | 合成 |
| `vxe.r1` | VXE R1 Star Flash `373B:10C9` | 回放通过 | 回放通过 | 回放通过 | 合成 |
| `eweadn.s9_pro` | EWEADN S9 Pro `372E:103D` / `1027` | 回放通过；充电时丢弃百分比 | 充电/放电回放通过 | 不支持 | 合成 |
| `eweadn.a5_pro` | EWEADN A5 Pro `258A:0138` | 回放通过 | 未知；不做 USB 启发式推断 | 不支持 | 合成 |
| `dareu.a950_air_v2` | DAREU A950 Air V2 `260D:1175` | 回放通过 | 回放通过；已实现满电映射 | 回放通过 | 合成 |
| `mchose.v9` | MCHOSE V9 Turbo+ `3837:600A` | 回放通过 | 回放通过；已实现满电映射 | 不支持 | 合成 |
| `mchose.k7_v2_ultra` | MCHOSE K7 V2 Ultra+ `3837:1014` | 回放通过 | 回放通过 | 不支持 | 合成 |
| `mchose.a5_mouse` | MCHOSE A5 系列无线子设备，VID `3837` | 回放通过 | 回放通过 | 不支持 | 合成 |
| `asus.rog_omni` | ROG Omni `0B05:1ACE`，静态键盘/鼠标通道 | 回放通过 | 回放通过 | 不支持 | 合成 |
| `asus.headset` | ROG Delta II / Pelta 及命名耳机 | 回放通过 | 回放通过 | 不支持 | 合成 |
| `taidu.mouse` | TAIDU TSG809 及已知 VID 家族 | 回放通过 | 回放通过 | 不支持 | 合成 |
| `imecoo.p1` | IMECOO P1 `3554:FB34` | 回放通过 | 回放通过 | 回放通过 | 合成 |
| `atk.n9v2` | ATK N9V2 Ultimate `373B:1262` | 回放通过 | 回放通过；已实现满电映射 | 不支持 | 合成 |
| `atk.a9_air` | ATK A9 Air+ `373B:1278` | 回放通过 | 回放通过 | 回放通过 | 合成 |
| `os.xinput` | Windows XInput 手柄槽位 | 不支持；仅返回 4 档离散电量 | 未知 | 不支持 | 合成离散档位回放 |
| `crociris.x7` | CROCIRIS X7 `36F9:AA20` | 回放通过 | 回放通过 | 不支持 | 合成 |
| `xiberia.t7` | XIBERIA T7 / T7 GT | 回放通过 | 未知 | 不支持 | 合成 |
| `sony.dualsense` | Sony DualSense / Edge | 回放通过；设备上报 10% 步进 | 回放通过；已实现满电映射 | 不支持 | 合成 USB/蓝牙报告 |
| `rapoo.vt3s` | Rapoo VT3S `24AE:1411` | 回放通过 | 回放通过 | 不支持 | 合成推送报告 |
| `razer.mouse` | 已知无线 Razer 鼠标与键盘 PID 列表 | 回放通过；0–255 线性换算 | 回放通过 | 不支持 | 合成特征报告事务 |
| `wob.rainy75` | WOB Rainy75 `320F:5088`，`FF1C:0092` | **已验证：50% / 62% 实机读数 + 回放** | **已验证：放电（`0x00`）与充电（`0x01`）；其余值保守保持 `unknown`** | 不支持 | 实机抓包（放电 + 充电）+ 合成 |
| `razer.headset` | Razer BlackShark V2 Pro `1532:0555` | 回放通过 | 回放通过 | 不支持 | 合成 |

## 扩展厂商 HID Provider（`experimental`，均已通过严格合成回放）

以下新增的 **128 条内置 HID Provider** 参考自 [Wireless-Device-Battery-Monitor / BatteryHub](../NOTICE)（GPL-3.0-or-later），全部在 `tests/extended_vendor_replay_tests.rs` 中通过了零冲突静态匹配与严格线级回放验证；在独立完成实机抓包前均保持 `experimental` 状态。

### 耳机与音频设备

| 品牌 / 家族 | Provider IDs | 代表设备 |
|---|---|---|
| SteelSeries（赛睿） | `steelseries.arctis1`, `steelseries.arctis7`, `steelseries.arctis9`, `steelseries.arctis_pro_wireless`, `steelseries.nova_pro_wireless`, `steelseries.nova_family`, `steelseries.nova_elite` | Arctis 1 / 7 / 7+ / 7P / 7X / 9 / Pro Wireless、Arctis Nova 5 / 7 / Pro Wireless / Elite |
| HyperX（极度未知） | `hyperx.cloud3`, `hyperx.cloud3_s`, `hyperx.cloud2`, `hyperx.cloud2_kingston`, `hyperx.cloud_alpha`, `hyperx.alpha2_haste`, `hyperx.cloud_flight`, `hyperx.stinger_core` | Cloud II / III / III S / Alpha / Alpha 2 / Flight / Stinger Core |
| Corsair（海盗船） | `corsair.void`, `corsair.bragi` | Void / Void Pro / Void Elite / HS70 / HS80 / Virtuoso（含 Bragi 鼠标与键盘） |
| Razer 耳机 | `razer.blackshark_v2_hs`, `razer.barracuda_x`, `razer.blackshark_v2_pro_2020`, `razer.blackshark_v3_x`, `razer.blackshark_v3`, `razer.barracuda_pro` | BlackShark V2 HyperSpeed / V2 Pro 2020 / V3 / V3 X、Barracuda X / Pro |
| Logitech / Astro / Sony / ASUS | `logitech.g435`, `astro.a50_modern`, `astro.legacy`, `sony.inzone`, `asus.rog_cetra` | Logitech G435、Astro A50（Gen 3/4/5/X）与 A20/A30、Sony INZONE H5/H7/H9/Buds、ROG Cetra |
| 其他耳机与麦克风 | `audeze.maxwell`, `endorfy.viro`, `akg.n9_hybrid`, `turtlebeach.stealth700`, `jabra.gnp`, `jbl.hidv1`, `jbl.hidv3`, `plantronics.bt600`, `fractal.scape`, `alienware.aw920h`, `moondrop.gm01_pro`, `mchose.v9_pro2`, `vgn.omnisound`, `darkrim.shadow`, `maono.pd200w` | Audeze Maxwell、Endorfy Viro、AKG N9 Hybrid、Turtle Beach Stealth 700、Jabra Link、JBL Quantum、Poly BT600、Fractal Scape、Alienware AW920H、水月雨 GM01 Pro、迈从 V9 Pro2、VGN OmniSound、Darkrim Shadow、闪克 Maono PD200W |

### 鼠标与多设备接收器

| 品牌 / 家族 | Provider IDs | 代表设备 |
|---|---|---|
| ASUS ROG 与 SteelSeries | `asus.rog_peripheral`, `steelseries.aerox`, `steelseries.rival`, `steelseries.rival_pro` | ROG Harpe/Gladius/Keris/Chakram/Azoth/Falchion、Aerox 3/5/9、Prime/Rival 无线系列 |
| DriverCore / Nordic 8K 系列 | `glorious.mouse`, `hitscan.mouse`, `gwolves.mouse`, `arbiter.akitsu`, `lamzu.mouse`, `wlmouse.mouse`, `ninjutso.sora_v2`, `sprime.pm1`, `ninjutso.ten`, `pulsar.x2_v3`, `vaxee.mouse`, `endgame.4k` | Glorious Model O/D/I、Hitscan Hyperlight、G-Wolves、Arbiter Akitsu、Lamzu Maya/Atlantis/Thorn、WLmouse Beast X、Ninjutso Sora V2/Ten、Sprime PM1、Pulsar X2 V3、VAXEE、Endgame Gear OP1w/XM2w 4K |
| CompX 0x08 与 MCHOSE / ATK | `kysona.mouse`, `chimera.v2`, `cherry.xtrfy`, `corsair.compx`, `mchose.a5_gen1`, `mchose.g3_v2`, `mchose.modern_mouse`, `atk.zero`, `atk.bitmouse_nk`, `atk.nano_8k` | Kysona、Chimera V2、Cherry Xtrfy、Corsair M75/Katar（CompX）、迈从 A5/G3 V2/A7/L7、ATK F1/X1/Z1/蜻蜓系列 |
| 客制化与各品牌无线鼠标 | `finalmouse.mouse`, `scyrox.v12m`, `rawm.mouse`, `zowie.ec_cw`, `coolermaster.mouse`, `nzxt.lift_elite`, `msi.mouse`, `msi.gm41`, `fantech.xd5`, `raigor.mouse`, `sinowealth.receiver`, `royal_kludge.mg6_pro`, `darmoshark.m3`, `darmoshark.nrf`, `machenike.l7`, `niceboy.mw400`, `motospeed.x6`, `xenta.mouse`, `delux.m800_pro`, `ajazz.aj179`, `ajazz.aj_series`, `incott.mouse`, `rongyuan.receiver`, `dareu.togran`, `zenith.pro`, `rapoo.vt`, `rapoo.legacy_vt3_pro`, `pressplay.icarus` | Finalmouse ULX、Scyrox V6/V8、RAWM、卓威 EC-CW、酷冷至尊、NZXT Lift Elite、微星 Clutch、Fantech、Raigor、中颖方案、RK、达摩鲨、机械师、Niceboy、摩豹、Xenta、多彩 M800 Pro、黑爵 AJ139/AJ159/AJ179、因科特、荣元方案、达尔优 Togran、Zenith Pro、雷柏 VT/VT3 Pro、Press Play Icarus |

### 键盘与键鼠套件

| 品牌 / 家族 | Provider IDs | 代表设备 |
|---|---|---|
| Keychron / Lemokey | `keychron.nape`, `keychron.legacy_mouse`, `keychron.ultralink`, `keychron.launcher_mouse`, `keychron.qmk_keyboard` | Keychron M 系列 / Lemokey 鼠标、UltraLink 8K 接收器、Keychron Q/K/V/P/Max/Pro QMK 无线键盘 |
| Cherry / Akko / NuPhy / MCHOSE / DAREU | `cherry.mx3s`, `akko.dash`, `nuphy.wh80`, `mchose.glw_keyboard`, `mchose.k87_v3`, `dareu.a84` | Cherry MX 3.0S/3.1、Akko/MonsGeek Dash、NuPhy Air/WH80、迈从 GLW 与 K87 V3、达尔优 A84 |
| EVision / 狼蛛 AULA / Sonix / VIA / Semico | `evision.atk_keyboard`, `atk.z87`, `evision.redragon_m999`, `evision.ajazz_ak992`, `evision.aula_mouse`, `aula.hero68`, `aula.sc580`, `sonix.keyboard`, `via.dongle`, `semico.keyboard` | EVision 方案 ATK/红龙/黑爵/狼蛛键鼠、狼蛛 HERO 68 / SC580、Sonix / VIA / Semico 无线键盘 |

### 游戏手柄

| 品牌 / 家族 | Provider IDs | 代表设备 |
|---|---|---|
| 主机与 PC 手柄 | `nintendo.switch_pro`, `sony.dualshock4`, `valve.steam_controller`, `gamesir.nova`, `gamesir.cyclone2`, `gamesir.sapjoy`, `flydigi.controller`, `eightbitdo.vendor`, `eightbitdo.dinput`, `beitong.controller` | Nintendo Switch Pro / Joy-Con、Sony DualShock 4、Valve Steam Controller、盖世小鸡 Nova/Cyclone 2/SapJoy、飞智冰原狼/八爪鱼/黑武士、八位堂 Ultimate/Pro、北通手柄 |

## 标准协议

| 来源 | 生命周期 | 百分比 | 充电状态 | 平台 | 限制说明 |
|---|---|---|---|---|---|
| Bluetooth LE BAS (`bluetooth.bas`) | experimental | 协议已实现；未实机验证 | 未知 / 不支持 | 可选支持 Windows、Linux BlueZ、macOS | 仅枚举已知外设；显式读取时可能发起连接；不含经典蓝牙/HFP |

## 暂缓接入的非 HID 与双句柄拆分集合协议

上游工具中的以下设备来源因架构边界暂未迁入单会话 HID 层：

- **需同时持有两个 HID 集合句柄的拆分集合设备**：`Endgame Gear WE`、`Areson`、`DAREU EM901X / SGM50W`、`ZOWIE DW`、`Wraith W1`。
- **非 HID / 系统专有 / 外部进程来源**：BLE 主动广播监听（AirPods / Google Fast Pair）、经典蓝牙 RFCOMM（Samsung Galaxy Buds / Nothing Ear）、Windows Phone Link 本地 JSON 缓存、虚拟串口 COM（`atk_serial`）、厂商私有 BLE GATT（`gamesir_ble`）、Windows PnP 通用蓝牙缓存属性，以及外部 `headsetcontrol.exe` 进程委托。

## 能力证据说明

设备包生命周期与单能力证据彼此独立。百分比通过回放不代表充电、满电、低电量、休眠或离线行为已经过验证。欢迎补充满电、低电与休眠样本，但不作为贡献合并阻塞项。`fixtures/synthetic/` 下的测试包仅用于测试，绝不进入真实设备支持。

---

相关文档：[实机验证清单](hardware-verification.zh-CN.md) · [设备贡献指南](contributing-device.zh-CN.md) · [平台支持](platform-support.zh-CN.md) · [返回 README](../README.zh-CN.md)
