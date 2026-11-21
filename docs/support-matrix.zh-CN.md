# 设备支持矩阵

> **Language / 语言**: [English](support-matrix.md) | **简体中文**

**当前内置 152 条厂商与操作系统协议路径**（另含默认启用的 Bluetooth LE BAS），所有已移植设备默认均已启用；如在特定型号或固件上遇到读数异常，欢迎直接提交 Issue 反馈。

## 内置厂商与系统支持列表

| 厂商 | 设备类型 | 设备 / 型号 | 支持方式 | Provider ID |
|---|---|---|---|---|
| 8BitDo（八位堂） | 手柄 | 8BitDo Ultimate / Pro（厂商模式） | 内置 Rust HID | `eightbitdo.vendor` |
| | 手柄 | 8BitDo 手柄（DInput 模式） | 内置 Rust HID | `eightbitdo.dinput` |
| Ajazz（黑爵） | 鼠标 | AJ179 | 内置 Rust HID | `ajazz.aj179` |
| | 鼠标 | AJ139 / AJ159 等 AJ 系列 | 内置 Rust HID | `ajazz.aj_series` |
| | 键盘 | AK992（EVision） | 内置 Rust HID | `evision.ajazz_ak992` |
| AKG | 耳机 | AKG N9 Hybrid | 内置 Rust HID | `akg.n9_hybrid` |
| Akko / MonsGeek | 键盘 | Akko / MonsGeek Dash | 内置 Rust HID | `akko.dash` |
| Alienware（外星人） | 耳机 | Alienware AW920H | 内置 Rust HID | `alienware.aw920h` |
| Arbiter | 鼠标 | Arbiter Akitsu | 内置 Rust HID | `arbiter.akitsu` |
| Astro | 耳机 | Astro A50（Gen 4 / 5 / X）/ A30 | 内置 Rust HID | `astro.a50_modern` |
| | 耳机 | Astro A50（Gen 3）/ A20 | 内置 Rust HID | `astro.legacy` |
| ASUS ROG（华硕玩家国度） | 接收器 / 键鼠 | ROG Omni `0B05:1ACE` | 内置 Rust HID | `asus.rog_omni` |
| | 鼠标 / 键盘 | ROG Harpe / Gladius / Keris / Chakram / Azoth / Falchion | 内置 Rust HID | `asus.rog_peripheral` |
| | 耳机 | ROG Delta II / Pelta | 内置 Rust HID | `asus.headset` |
| | 耳机 | ROG Cetra | 内置 Rust HID | `asus.rog_cetra` |
| ATK | 鼠标 | ATK A9 Air+ `373B:1278` | 内置 Rust HID | `atk.a9_air` |
| | 鼠标 | ATK Zero | 内置 Rust HID | `atk.zero` |
| | 鼠标 | ATK Bitmouse NK | 内置 Rust HID | `atk.bitmouse_nk` |
| | 鼠标 | ATK Nano 8K | 内置 Rust HID | `atk.nano_8k` |
| | 耳机 | ATK N9V2 Ultimate `373B:1262` | 内置 Rust HID | `atk.n9v2` |
| | 键盘 | ATK Z87 | 内置 Rust HID | `atk.z87` |
| | 键盘 | ATK 键盘（EVision） | 内置 Rust HID | `evision.atk_keyboard` |
| Audeze | 耳机 | Audeze Maxwell | 内置 Rust HID | `audeze.maxwell` |
| AULA（狼蛛） | 鼠标 | AULA 鼠标（EVision） | 内置 Rust HID | `evision.aula_mouse` |
| | 鼠标 | AULA SC580 | 内置 Rust HID | `aula.sc580` |
| | 键盘 | AULA HERO 68 | 内置 Rust HID | `aula.hero68` |
| Beitong（北通） | 手柄 | 北通无线手柄 | 内置 Rust HID | `beitong.controller` |
| Cherry（樱桃） | 键盘 | Cherry MX 3.0S / 3.1 | 内置 Rust HID | `cherry.mx3s` |
| | 鼠标 | Cherry Xtrfy | 内置 Rust HID | `cherry.xtrfy` |
| Chimera | 鼠标 | Chimera V2 | 内置 Rust HID | `chimera.v2` |
| Cooler Master（酷冷至尊） | 鼠标 | Cooler Master 无线鼠标 | 内置 Rust HID | `coolermaster.mouse` |
| Corsair（海盗船） | 耳机 | Void / Void Pro / Void Elite / HS70 / HS80 / Virtuoso | 内置 Rust HID | `corsair.void` |
| | 键鼠 / 耳机 | Corsair Bragi 系列 | 内置 Rust HID | `corsair.bragi` |
| | 鼠标 | Corsair M75 / Katar（CompX） | 内置 Rust HID | `corsair.compx` |
| CROCIRIS | 接收器 / 鼠标 | CROCIRIS X7 `36F9:AA20` | 内置 Rust HID | `crociris.x7` |
| DAREU（达尔优） | 鼠标 | DAREU A950 Air V2 `260D:1175` | 内置 Rust HID | `dareu.a950_air_v2` |
| | 鼠标 | DAREU Togran 方案鼠标 | 内置 Rust HID | `dareu.togran` |
| | 键盘 | DAREU A84 | 内置 Rust HID | `dareu.a84` |
| Darkrim | 耳机 | Darkrim Shadow | 内置 Rust HID | `darkrim.shadow` |
| Darmoshark（达摩鲨） | 鼠标 | Darmoshark M3 | 内置 Rust HID | `darmoshark.m3` |
| | 鼠标 | Darmoshark NRF 系列 | 内置 Rust HID | `darmoshark.nrf` |
| Delux（多彩） | 鼠标 | Delux M800 Pro | 内置 Rust HID | `delux.m800_pro` |
| Endgame Gear | 鼠标 | OP1w / XM2w 4K | 内置 Rust HID | `endgame.4k` |
| Endorfy | 耳机 | Endorfy Viro | 内置 Rust HID | `endorfy.viro` |
| EWEADN（前行者） | 鼠标 | EWEADN S9 Pro `372E:103D` / `1027` | 内置 Rust HID | `eweadn.s9_pro` |
| | 鼠标 | EWEADN A5 Pro `258A:0138` | 内置 Rust HID | `eweadn.a5_pro` |
| Fantech | 鼠标 | Fantech XD5 | 内置 Rust HID | `fantech.xd5` |
| Finalmouse | 鼠标 | Finalmouse ULX | 内置 Rust HID | `finalmouse.mouse` |
| Flydigi（飞智） | 手柄 | 飞智 Vader 5 Pro / Apex 5 等 | 内置 Rust HID | `flydigi.controller` |
| Fractal | 耳机 | Fractal Scape | 内置 Rust HID | `fractal.scape` |
| G-Wolves | 鼠标 | G-Wolves 无线鼠标 | 内置 Rust HID | `gwolves.mouse` |
| GameSir（盖世小鸡） | 手柄 | GameSir Nova | 内置 Rust HID | `gamesir.nova` |
| | 手柄 | GameSir Cyclone 2 | 内置 Rust HID | `gamesir.cyclone2` |
| | 手柄 | GameSir SapJoy | 内置 Rust HID | `gamesir.sapjoy` |
| Glorious | 鼠标 | Glorious Model O / D / I | 内置 Rust HID | `glorious.mouse` |
| Hitscan | 鼠标 | Hitscan Hyperlight | 内置 Rust HID | `hitscan.mouse` |
| HyperX（极度未知） | 耳机 | Cloud II | 内置 Rust HID | `hyperx.cloud2` |
| | 耳机 | Cloud II（Kingston） | 内置 Rust HID | `hyperx.cloud2_kingston` |
| | 耳机 | Cloud III | 内置 Rust HID | `hyperx.cloud3` |
| | 耳机 | Cloud III S | 内置 Rust HID | `hyperx.cloud3_s` |
| | 耳机 | Cloud Alpha | 内置 Rust HID | `hyperx.cloud_alpha` |
| | 耳机 | Cloud Alpha 2 / Pulsefire Haste | 内置 Rust HID | `hyperx.alpha2_haste` |
| | 耳机 | Cloud Flight | 内置 Rust HID | `hyperx.cloud_flight` |
| | 耳机 | Cloud Stinger Core | 内置 Rust HID | `hyperx.stinger_core` |
| IMECOO | 鼠标 | IMECOO P1 `3554:FB34` | 内置 Rust HID | `imecoo.p1` |
| Incott（因科特） | 鼠标 | Incott 无线鼠标 | 内置 Rust HID | `incott.mouse` |
| Jabra | 耳机 / 接收器 | Jabra Link（GNP） | 内置 Rust HID | `jabra.gnp` |
| JBL | 耳机 | JBL Quantum（HID v1） | 内置 Rust HID | `jbl.hidv1` |
| | 耳机 | JBL Quantum（HID v3） | 内置 Rust HID | `jbl.hidv3` |
| Keychron / Lemokey | 鼠标 | Keychron Nape | 内置 Rust HID | `keychron.nape` |
| | 鼠标 | Keychron 早期无线鼠标 | 内置 Rust HID | `keychron.legacy_mouse` |
| | 接收器 | UltraLink 8K 接收器 | 内置 Rust HID | `keychron.ultralink` |
| | 鼠标 | Keychron Launcher 鼠标 | 内置 Rust HID | `keychron.launcher_mouse` |
| | 键盘 | Keychron Q / K / V / P / Max / Pro QMK 无线键盘 | 内置 Rust HID | `keychron.qmk_keyboard` |
| Kysona | 鼠标 | Kysona 无线鼠标 | 内置 Rust HID | `kysona.mouse` |
| Lamzu | 鼠标 | Lamzu Maya / Atlantis / Thorn | 内置 Rust HID | `lamzu.mouse` |
| Logitech（罗技） | 鼠标 / 键盘 | GPW3 `046D:C547` 及 HID++ 设备（`0x1004`/`0x1000`/`0x1001`/`0x1F20`） | 内置 Rust HID | `logitech.hidpp` |
| | 耳机 | Logitech G435 | 内置 Rust HID | `logitech.g435` |
| Machenike（机械师） | 鼠标 | Machenike L7 | 内置 Rust HID | `machenike.l7` |
| Maono（闪克） | 麦克风 | Maono PD200W | 内置 Rust HID | `maono.pd200w` |
| MCHOSE（迈从） | 鼠标 | MCHOSE A5（Gen 1） | 内置 Rust HID | `mchose.a5_gen1` |
| | 鼠标 | MCHOSE A5 系列无线子设备（VID `3837`） | 内置 Rust HID | `mchose.a5_mouse` |
| | 鼠标 | MCHOSE A7 Ultra（VID `5253`） | 内置 Rust HID | `mchose.a7_ultra` |
| | 鼠标 | MCHOSE G3 V2 | 内置 Rust HID | `mchose.g3_v2` |
| | 鼠标 | MCHOSE K7 V2 Ultra+ `3837:1014` | 内置 Rust HID | `mchose.k7_v2_ultra` |
| | 鼠标 | MCHOSE 现代鼠标系列 | 内置 Rust HID | `mchose.modern_mouse` |
| | 键盘 | MCHOSE / AULA / Redragon BY 键盘 | 内置 Rust HID | `mchose.keyboard` |
| | 键盘 | MCHOSE GLW 键盘 | 内置 Rust HID | `mchose.glw_keyboard` |
| | 键盘 | MCHOSE K87 V3 | 内置 Rust HID | `mchose.k87_v3` |
| | 耳机 | MCHOSE V9 Turbo+ `3837:600A` | 内置 Rust HID | `mchose.v9` |
| | 耳机 | MCHOSE V9 Pro2 | 内置 Rust HID | `mchose.v9_pro2` |
| Microsoft（微软） | 手柄 | Windows XInput 手柄槽位（仅 4 档离散电量） | 系统 XInput API | `os.xinput` |
| Moondrop（水月雨） | 耳机 | Moondrop GM01 Pro | 内置 Rust HID | `moondrop.gm01_pro` |
| Motospeed（摩豹） | 鼠标 | Motospeed X6 | 内置 Rust HID | `motospeed.x6` |
| MSI（微星） | 鼠标 | MSI Clutch 无线鼠标 | 内置 Rust HID | `msi.mouse` |
| | 鼠标 | MSI GM41 | 内置 Rust HID | `msi.gm41` |
| Niceboy | 鼠标 | Niceboy MW400 | 内置 Rust HID | `niceboy.mw400` |
| Ninjutso | 鼠标 | Ninjutso Sora V2 | 内置 Rust HID | `ninjutso.sora_v2` |
| | 鼠标 | Ninjutso Ten | 内置 Rust HID | `ninjutso.ten` |
| Nintendo（任天堂） | 手柄 | Switch Pro / Joy-Con | 内置 Rust HID | `nintendo.switch_pro` |
| NuPhy | 键盘 | NuPhy Air / WH80 | 内置 Rust HID | `nuphy.wh80` |
| NZXT | 鼠标 | NZXT Lift Elite | 内置 Rust HID | `nzxt.lift_elite` |
| Plantronics / Poly | 接收器 | Poly BT600 | 内置 Rust HID | `plantronics.bt600` |
| Press Play | 鼠标 | Press Play Icarus | 内置 Rust HID | `pressplay.icarus` |
| Pulsar | 鼠标 | Pulsar X2 V3 | 内置 Rust HID | `pulsar.x2_v3` |
| Raigor | 鼠标 | Raigor 无线鼠标 | 内置 Rust HID | `raigor.mouse` |
| Rapoo（雷柏） | 鼠标 | Rapoo VT3S `24AE:1411` | 内置 Rust HID | `rapoo.vt3s` |
| | 鼠标 | Rapoo VT 系列 | 内置 Rust HID | `rapoo.vt` |
| | 鼠标 | Rapoo 早期 VT3 Pro | 内置 Rust HID | `rapoo.legacy_vt3_pro` |
| RAWM | 鼠标 | RAWM 无线鼠标 | 内置 Rust HID | `rawm.mouse` |
| Razer（雷蛇） | 鼠标 / 键盘 | Razer 无线鼠标与键盘（Feature 事务） | 内置 Rust HID | `razer.mouse` |
| | 耳机 | BlackShark V2 Pro `1532:0555` | 内置 Rust HID | `razer.headset` |
| | 耳机 | BlackShark V2 HyperSpeed | 内置 Rust HID | `razer.blackshark_v2_hs` |
| | 耳机 | BlackShark V2 Pro (2020) | 内置 Rust HID | `razer.blackshark_v2_pro_2020` |
| | 耳机 | BlackShark V3 | 内置 Rust HID | `razer.blackshark_v3` |
| | 耳机 | BlackShark V3 X | 内置 Rust HID | `razer.blackshark_v3_x` |
| | 耳机 | Barracuda X | 内置 Rust HID | `razer.barracuda_x` |
| | 耳机 | Barracuda Pro | 内置 Rust HID | `razer.barracuda_pro` |
| Redragon（红龙） | 鼠标 | Redragon M999（EVision） | 内置 Rust HID | `evision.redragon_m999` |
| Rongyuan（荣元） | 接收器 | 荣元方案接收器 | 内置 Rust HID | `rongyuan.receiver` |
| Royal Kludge（RK） | 鼠标 | RK MG6 Pro | 内置 Rust HID | `royal_kludge.mg6_pro` |
| Scyrox | 鼠标 | Scyrox V6 / V8 (V12M) | 内置 Rust HID | `scyrox.v12m` |
| Semico | 键盘 | Semico 无线键盘 | 内置 Rust HID | `semico.keyboard` |
| Sinowealth（中颖） | 接收器 | 中颖方案接收器 | 内置 Rust HID | `sinowealth.receiver` |
| Sonix | 键盘 | Sonix 无线键盘 | 内置 Rust HID | `sonix.keyboard` |
| Sony（索尼） | 手柄 | DualSense / DualSense Edge | 内置 Rust HID | `sony.dualsense` |
| | 手柄 | DualShock 4 | 内置 Rust HID | `sony.dualshock4` |
| | 耳机 | INZONE H5 / H7 / H9 / Buds | 内置 Rust HID | `sony.inzone` |
| Sprime | 鼠标 | Sprime PM1 | 内置 Rust HID | `sprime.pm1` |
| SteelSeries（赛睿） | 耳机 | Arctis 1 | 内置 Rust HID | `steelseries.arctis1` |
| | 耳机 | Arctis 7 / 7+ / 7P / 7X | 内置 Rust HID | `steelseries.arctis7` |
| | 耳机 | Arctis 9 | 内置 Rust HID | `steelseries.arctis9` |
| | 耳机 | Arctis Pro Wireless | 内置 Rust HID | `steelseries.arctis_pro_wireless` |
| | 耳机 | Arctis Nova Pro Wireless | 内置 Rust HID | `steelseries.nova_pro_wireless` |
| | 耳机 | Arctis Nova 5 / 7 | 内置 Rust HID | `steelseries.nova_family` |
| | 耳机 | Arctis Nova Elite | 内置 Rust HID | `steelseries.nova_elite` |
| | 鼠标 | Aerox 3 / 5 / 9 Wireless | 内置 Rust HID | `steelseries.aerox` |
| | 鼠标 | Rival Wireless | 内置 Rust HID | `steelseries.rival` |
| | 鼠标 | Prime / Rival Pro Wireless | 内置 Rust HID | `steelseries.rival_pro` |
| TAIDU（钛度） | 鼠标 | TAIDU TSG809 及同 VID 系列 | 内置 Rust HID | `taidu.mouse` |
| Turtle Beach | 耳机 | Stealth 700 | 内置 Rust HID | `turtlebeach.stealth700` |
| Valve | 手柄 | Steam Controller | 内置 Rust HID | `valve.steam_controller` |
| VAXEE | 鼠标 | VAXEE 无线鼠标 | 内置 Rust HID | `vaxee.mouse` |
| VGN | 耳机 | VGN OmniSound | 内置 Rust HID | `vgn.omnisound` |
| VIA | 接收器 / 键盘 | VIA Dongle | 内置 Rust HID | `via.dongle` |
| VXE | 鼠标 | VXE R1 Star Flash `373B:10C9` | 内置 Rust HID | `vxe.r1` |
| WLmouse | 鼠标 | WLmouse Beast X | 内置 Rust HID | `wlmouse.mouse` |
| WOB | 键盘 | WOB Rainy75 `320F:5088` | 内置 Rust HID | `wob.rainy75` |
| Xenta | 鼠标 | Xenta 无线鼠标 | 内置 Rust HID | `xenta.mouse` |
| XIBERIA（西伯利亚） | 耳机 | XIBERIA T7 / T7 GT | 内置 Rust HID | `xiberia.t7` |
| Zenith | 鼠标 | Zenith Pro | 内置 Rust HID | `zenith.pro` |
| ZOWIE（卓威） | 鼠标 | ZOWIE EC-CW | 内置 Rust HID | `zowie.ec_cw` |

## 标准协议

| 厂商 / 标准 | 设备类型 | 设备 | 支持方式 | 平台与限制 | Provider ID |
|---|---|---|---|---|---|
| Bluetooth SIG | 通用 BLE 外设 | 已配对/已知 BLE 设备 | 只读 BLE BAS (`0x2A19`) | 支持 Windows、Linux BlueZ、macOS；仅在显式读取时连接，不含经典蓝牙/HFP | `bluetooth.bas` |

## 暂缓接入的非 HID 与双句柄拆分集合协议

以下设备来源因架构边界暂未迁入单会话 HID 层：

- **需同时持有两个 HID 集合句柄的拆分集合设备**：`Endgame Gear WE`、`Areson`、`DAREU EM901X / SGM50W`、`ZOWIE DW`、`Wraith W1`。
- **非 HID / 系统专有 / 外部进程来源**：BLE 主动广播监听（AirPods / Google Fast Pair）、经典蓝牙 RFCOMM（Samsung Galaxy Buds / Nothing Ear）、Windows Phone Link 本地 JSON 缓存、虚拟串口 COM（`atk_serial`）、厂商私有 BLE GATT（`gamesir_ble`）、Windows PnP 通用蓝牙缓存属性，以及外部 `headsetcontrol.exe` 进程委托。

---

相关文档：[设备贡献指南](contributing-device.zh-CN.md) · [平台支持](platform-support.zh-CN.md) · [返回 README](../README.zh-CN.md)
