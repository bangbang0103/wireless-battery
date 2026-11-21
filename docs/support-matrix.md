# Support matrix

> **Language / 语言**: **English** | [简体中文](support-matrix.zh-CN.md)

**152 built-in vendor and OS provider paths** (plus Bluetooth LE BAS) are included and enabled by default. If you encounter reading issues on a specific model or firmware, please open an Issue.

## Built-in Vendor & OS Providers

| Vendor | Type | Device / Model | Support method | Provider ID |
|---|---|---|---|---|
| 8BitDo | Gamepad | 8BitDo Ultimate / Pro (Vendor mode) | Built-in Rust HID | `eightbitdo.vendor` |
| | Gamepad | 8BitDo controllers (DInput mode) | Built-in Rust HID | `eightbitdo.dinput` |
| Ajazz | Mouse | AJ179 | Built-in Rust HID | `ajazz.aj179` |
| | Mouse | AJ139 / AJ159 / AJ series | Built-in Rust HID | `ajazz.aj_series` |
| | Keyboard | AK992 (EVision) | Built-in Rust HID | `evision.ajazz_ak992` |
| AKG | Headset | AKG N9 Hybrid | Built-in Rust HID | `akg.n9_hybrid` |
| Akko / MonsGeek | Keyboard | Akko / MonsGeek Dash | Built-in Rust HID | `akko.dash` |
| Alienware | Headset | Alienware AW920H | Built-in Rust HID | `alienware.aw920h` |
| Arbiter | Mouse | Arbiter Akitsu | Built-in Rust HID | `arbiter.akitsu` |
| Astro | Headset | Astro A50 (Gen 4 / 5 / X) / A30 | Built-in Rust HID | `astro.a50_modern` |
| | Headset | Astro A50 (Gen 3) / A20 | Built-in Rust HID | `astro.legacy` |
| ASUS ROG | Receiver / Peripherals | ROG Omni `0B05:1ACE` | Built-in Rust HID | `asus.rog_omni` |
| | Mouse / Keyboard | ROG Harpe / Gladius / Keris / Chakram / Azoth / Falchion | Built-in Rust HID | `asus.rog_peripheral` |
| | Headset | ROG Delta II / Pelta | Built-in Rust HID | `asus.headset` |
| | Headset | ROG Cetra | Built-in Rust HID | `asus.rog_cetra` |
| ATK | Mouse | ATK A9 Air+ `373B:1278` | Built-in Rust HID | `atk.a9_air` |
| | Mouse | ATK Zero | Built-in Rust HID | `atk.zero` |
| | Mouse | ATK Bitmouse NK | Built-in Rust HID | `atk.bitmouse_nk` |
| | Mouse | ATK Nano 8K | Built-in Rust HID | `atk.nano_8k` |
| | Headset | ATK N9V2 Ultimate `373B:1262` | Built-in Rust HID | `atk.n9v2` |
| | Keyboard | ATK Z87 | Built-in Rust HID | `atk.z87` |
| | Keyboard | ATK keyboards (EVision) | Built-in Rust HID | `evision.atk_keyboard` |
| Audeze | Headset | Audeze Maxwell | Built-in Rust HID | `audeze.maxwell` |
| AULA | Mouse | AULA mice (EVision) | Built-in Rust HID | `evision.aula_mouse` |
| | Mouse | AULA SC580 | Built-in Rust HID | `aula.sc580` |
| | Keyboard | AULA HERO 68 | Built-in Rust HID | `aula.hero68` |
| Beitong | Gamepad | Beitong wireless controllers | Built-in Rust HID | `beitong.controller` |
| Cherry | Keyboard | Cherry MX 3.0S / 3.1 | Built-in Rust HID | `cherry.mx3s` |
| | Mouse | Cherry Xtrfy | Built-in Rust HID | `cherry.xtrfy` |
| Chimera | Mouse | Chimera V2 | Built-in Rust HID | `chimera.v2` |
| Cooler Master | Mouse | Cooler Master wireless mice | Built-in Rust HID | `coolermaster.mouse` |
| Corsair | Headset | Void / Void Pro / Void Elite / HS70 / HS80 / Virtuoso | Built-in Rust HID | `corsair.void` |
| | Peripherals | Corsair Bragi series | Built-in Rust HID | `corsair.bragi` |
| | Mouse | Corsair M75 / Katar (CompX) | Built-in Rust HID | `corsair.compx` |
| CROCIRIS | Receiver / Mouse | CROCIRIS X7 `36F9:AA20` | Built-in Rust HID | `crociris.x7` |
| DAREU | Mouse | DAREU A950 Air V2 `260D:1175` | Built-in Rust HID | `dareu.a950_air_v2` |
| | Mouse | DAREU Togran mice | Built-in Rust HID | `dareu.togran` |
| | Keyboard | DAREU A84 | Built-in Rust HID | `dareu.a84` |
| Darkrim | Headset | Darkrim Shadow | Built-in Rust HID | `darkrim.shadow` |
| Darmoshark | Mouse | Darmoshark M3 | Built-in Rust HID | `darmoshark.m3` |
| | Mouse | Darmoshark NRF series | Built-in Rust HID | `darmoshark.nrf` |
| Delux | Mouse | Delux M800 Pro | Built-in Rust HID | `delux.m800_pro` |
| Endgame Gear | Mouse | OP1w / XM2w 4K | Built-in Rust HID | `endgame.4k` |
| Endorfy | Headset | Endorfy Viro | Built-in Rust HID | `endorfy.viro` |
| EWEADN | Mouse | EWEADN S9 Pro `372E:103D` / `1027` | Built-in Rust HID | `eweadn.s9_pro` |
| | Mouse | EWEADN A5 Pro `258A:0138` | Built-in Rust HID | `eweadn.a5_pro` |
| Fantech | Mouse | Fantech XD5 | Built-in Rust HID | `fantech.xd5` |
| Finalmouse | Mouse | Finalmouse ULX | Built-in Rust HID | `finalmouse.mouse` |
| Flydigi | Gamepad | Flydigi Vader 5 Pro / Apex 5 | Built-in Rust HID | `flydigi.controller` |
| Fractal | Headset | Fractal Scape | Built-in Rust HID | `fractal.scape` |
| G-Wolves | Mouse | G-Wolves wireless mice | Built-in Rust HID | `gwolves.mouse` |
| GameSir | Gamepad | GameSir Nova | Built-in Rust HID | `gamesir.nova` |
| | Gamepad | GameSir Cyclone 2 | Built-in Rust HID | `gamesir.cyclone2` |
| | Gamepad | GameSir SapJoy | Built-in Rust HID | `gamesir.sapjoy` |
| Glorious | Mouse | Glorious Model O / D / I | Built-in Rust HID | `glorious.mouse` |
| Hitscan | Mouse | Hitscan Hyperlight | Built-in Rust HID | `hitscan.mouse` |
| HyperX | Headset | Cloud II | Built-in Rust HID | `hyperx.cloud2` |
| | Headset | Cloud II (Kingston) | Built-in Rust HID | `hyperx.cloud2_kingston` |
| | Headset | Cloud III | Built-in Rust HID | `hyperx.cloud3` |
| | Headset | Cloud III S | Built-in Rust HID | `hyperx.cloud3_s` |
| | Headset | Cloud Alpha | Built-in Rust HID | `hyperx.cloud_alpha` |
| | Headset | Cloud Alpha 2 / Pulsefire Haste | Built-in Rust HID | `hyperx.alpha2_haste` |
| | Headset | Cloud Flight | Built-in Rust HID | `hyperx.cloud_flight` |
| | Headset | Cloud Stinger Core | Built-in Rust HID | `hyperx.stinger_core` |
| IMECOO | Mouse | IMECOO P1 `3554:FB34` | Built-in Rust HID | `imecoo.p1` |
| Incott | Mouse | Incott wireless mice | Built-in Rust HID | `incott.mouse` |
| Jabra | Headset / Dongle | Jabra Link (GNP) | Built-in Rust HID | `jabra.gnp` |
| JBL | Headset | JBL Quantum (HID v1) | Built-in Rust HID | `jbl.hidv1` |
| | Headset | JBL Quantum (HID v3) | Built-in Rust HID | `jbl.hidv3` |
| Keychron / Lemokey | Mouse | Keychron Nape | Built-in Rust HID | `keychron.nape` |
| | Mouse | Keychron legacy wireless mice | Built-in Rust HID | `keychron.legacy_mouse` |
| | Receiver | UltraLink 8K receiver | Built-in Rust HID | `keychron.ultralink` |
| | Mouse | Keychron Launcher mice | Built-in Rust HID | `keychron.launcher_mouse` |
| | Keyboard | Keychron Q / K / V / P / Max / Pro QMK wireless keyboards | Built-in Rust HID | `keychron.qmk_keyboard` |
| Kysona | Mouse | Kysona wireless mice | Built-in Rust HID | `kysona.mouse` |
| Lamzu | Mouse | Lamzu Maya / Atlantis / Thorn | Built-in Rust HID | `lamzu.mouse` |
| Logitech | Mouse / Keyboard | GPW3 `046D:C547` & HID++ devices (`0x1004`/`0x1000`/`0x1001`/`0x1F20`) | Built-in Rust HID | `logitech.hidpp` |
| | Headset | Logitech G435 | Built-in Rust HID | `logitech.g435` |
| Machenike | Mouse | Machenike L7 | Built-in Rust HID | `machenike.l7` |
| Maono | Microphone | Maono PD200W | Built-in Rust HID | `maono.pd200w` |
| MCHOSE | Mouse | MCHOSE A5 (Gen 1) | Built-in Rust HID | `mchose.a5_gen1` |
| | Mouse | MCHOSE A5-family radio child (VID `3837`) | Built-in Rust HID | `mchose.a5_mouse` |
| | Mouse | MCHOSE A7 Ultra (VID `5253`) | Built-in Rust HID | `mchose.a7_ultra` |
| | Mouse | MCHOSE G3 V2 | Built-in Rust HID | `mchose.g3_v2` |
| | Mouse | MCHOSE K7 V2 Ultra+ `3837:1014` | Built-in Rust HID | `mchose.k7_v2_ultra` |
| | Mouse | MCHOSE modern mice | Built-in Rust HID | `mchose.modern_mouse` |
| | Keyboard | MCHOSE / AULA / Redragon BY keyboards | Built-in Rust HID | `mchose.keyboard` |
| | Keyboard | MCHOSE GLW keyboards | Built-in Rust HID | `mchose.glw_keyboard` |
| | Keyboard | MCHOSE K87 V3 | Built-in Rust HID | `mchose.k87_v3` |
| | Headset | MCHOSE V9 Turbo+ `3837:600A` | Built-in Rust HID | `mchose.v9` |
| | Headset | MCHOSE V9 Pro2 | Built-in Rust HID | `mchose.v9_pro2` |
| Microsoft | Gamepad | Windows XInput controller slots (4 discrete levels only) | OS XInput API | `os.xinput` |
| Moondrop | Headset | Moondrop GM01 Pro | Built-in Rust HID | `moondrop.gm01_pro` |
| Motospeed | Mouse | Motospeed X6 | Built-in Rust HID | `motospeed.x6` |
| MSI | Mouse | MSI Clutch wireless mice | Built-in Rust HID | `msi.mouse` |
| | Mouse | MSI GM41 | Built-in Rust HID | `msi.gm41` |
| Niceboy | Mouse | Niceboy MW400 | Built-in Rust HID | `niceboy.mw400` |
| Ninjutso | Mouse | Ninjutso Sora V2 | Built-in Rust HID | `ninjutso.sora_v2` |
| | Mouse | Ninjutso Ten | Built-in Rust HID | `ninjutso.ten` |
| Nintendo | Gamepad | Switch Pro / Joy-Con | Built-in Rust HID | `nintendo.switch_pro` |
| NuPhy | Keyboard | NuPhy Air / WH80 | Built-in Rust HID | `nuphy.wh80` |
| NZXT | Mouse | NZXT Lift Elite | Built-in Rust HID | `nzxt.lift_elite` |
| Plantronics / Poly | Receiver | Poly BT600 | Built-in Rust HID | `plantronics.bt600` |
| Press Play | Mouse | Press Play Icarus | Built-in Rust HID | `pressplay.icarus` |
| Pulsar | Mouse | Pulsar X2 V3 | Built-in Rust HID | `pulsar.x2_v3` |
| Raigor | Mouse | Raigor wireless mice | Built-in Rust HID | `raigor.mouse` |
| Rapoo | Mouse | Rapoo VT3S `24AE:1411` | Built-in Rust HID | `rapoo.vt3s` |
| | Mouse | Rapoo VT series | Built-in Rust HID | `rapoo.vt` |
| | Mouse | Rapoo legacy VT3 Pro | Built-in Rust HID | `rapoo.legacy_vt3_pro` |
| RAWM | Mouse | RAWM wireless mice | Built-in Rust HID | `rawm.mouse` |
| Razer | Mouse / Keyboard | Razer wireless mice & keyboards (Feature report) | Built-in Rust HID | `razer.mouse` |
| | Headset | BlackShark V2 Pro `1532:0555` | Built-in Rust HID | `razer.headset` |
| | Headset | BlackShark V2 HyperSpeed | Built-in Rust HID | `razer.blackshark_v2_hs` |
| | Headset | BlackShark V2 Pro (2020) | Built-in Rust HID | `razer.blackshark_v2_pro_2020` |
| | Headset | BlackShark V3 | Built-in Rust HID | `razer.blackshark_v3` |
| | Headset | BlackShark V3 X | Built-in Rust HID | `razer.blackshark_v3_x` |
| | Headset | Barracuda X | Built-in Rust HID | `razer.barracuda_x` |
| | Headset | Barracuda Pro | Built-in Rust HID | `razer.barracuda_pro` |
| Redragon | Mouse | Redragon M999 (EVision) | Built-in Rust HID | `evision.redragon_m999` |
| Rongyuan | Receiver | Rongyuan receivers | Built-in Rust HID | `rongyuan.receiver` |
| Royal Kludge | Mouse | Royal Kludge MG6 Pro | Built-in Rust HID | `royal_kludge.mg6_pro` |
| Scyrox | Mouse | Scyrox V6 / V8 (V12M) | Built-in Rust HID | `scyrox.v12m` |
| Semico | Keyboard | Semico wireless keyboards | Built-in Rust HID | `semico.keyboard` |
| Sinowealth | Receiver | Sinowealth receivers | Built-in Rust HID | `sinowealth.receiver` |
| Sonix | Keyboard | Sonix wireless keyboards | Built-in Rust HID | `sonix.keyboard` |
| Sony | Gamepad | DualSense / DualSense Edge | Built-in Rust HID | `sony.dualsense` |
| | Gamepad | DualShock 4 | Built-in Rust HID | `sony.dualshock4` |
| | Headset | INZONE H5 / H7 / H9 / Buds | Built-in Rust HID | `sony.inzone` |
| Sprime | Mouse | Sprime PM1 | Built-in Rust HID | `sprime.pm1` |
| SteelSeries | Headset | Arctis 1 | Built-in Rust HID | `steelseries.arctis1` |
| | Headset | Arctis 7 / 7+ / 7P / 7X | Built-in Rust HID | `steelseries.arctis7` |
| | Headset | Arctis 9 | Built-in Rust HID | `steelseries.arctis9` |
| | Headset | Arctis Pro Wireless | Built-in Rust HID | `steelseries.arctis_pro_wireless` |
| | Headset | Arctis Nova Pro Wireless | Built-in Rust HID | `steelseries.nova_pro_wireless` |
| | Headset | Arctis Nova 5 / 7 | Built-in Rust HID | `steelseries.nova_family` |
| | Headset | Arctis Nova Elite | Built-in Rust HID | `steelseries.nova_elite` |
| | Mouse | Aerox 3 / 5 / 9 Wireless | Built-in Rust HID | `steelseries.aerox` |
| | Mouse | Rival Wireless | Built-in Rust HID | `steelseries.rival` |
| | Mouse | Prime / Rival Pro Wireless | Built-in Rust HID | `steelseries.rival_pro` |
| TAIDU | Mouse | TAIDU TSG809 & VID family | Built-in Rust HID | `taidu.mouse` |
| Turtle Beach | Headset | Stealth 700 | Built-in Rust HID | `turtlebeach.stealth700` |
| Valve | Gamepad | Steam Controller | Built-in Rust HID | `valve.steam_controller` |
| VAXEE | Mouse | VAXEE wireless mice | Built-in Rust HID | `vaxee.mouse` |
| VGN | Headset | VGN OmniSound | Built-in Rust HID | `vgn.omnisound` |
| VIA | Receiver / Keyboard | VIA Dongle | Built-in Rust HID | `via.dongle` |
| VXE | Mouse | VXE R1 Star Flash `373B:10C9` | Built-in Rust HID | `vxe.r1` |
| WLmouse | Mouse | WLmouse Beast X | Built-in Rust HID | `wlmouse.mouse` |
| WOB | Keyboard | WOB Rainy75 `320F:5088` | Built-in Rust HID | `wob.rainy75` |
| Xenta | Mouse | Xenta wireless mice | Built-in Rust HID | `xenta.mouse` |
| XIBERIA | Headset | XIBERIA T7 / T7 GT | Built-in Rust HID | `xiberia.t7` |
| Zenith | Mouse | Zenith Pro | Built-in Rust HID | `zenith.pro` |
| ZOWIE | Mouse | ZOWIE EC-CW | Built-in Rust HID | `zowie.ec_cw` |

## Standard Protocols

| Vendor / Standard | Type | Devices | Support method | Platforms & Limitations | Provider ID |
|---|---|---|---|---|---|
| Bluetooth SIG | Generic BLE peripherals | Known / paired BLE devices | Read-only BLE BAS (`0x2A19`) | Windows, Linux BlueZ, macOS; connects only on explicit read, excludes Classic/HFP | `bluetooth.bas` |

## Deferred Non-HID & Dual-Collection Protocols

The following source types are intentionally **not** ported into the single-session HID layer yet:

- **Split-collection dual-handle HID devices** requiring two simultaneous HID handles (`Endgame Gear WE`, `Areson`, `DAREU EM901X / SGM50W`, `ZOWIE DW`, `Wraith W1`).
- **Non-HID / OS-specific / external-binary sources**: active BLE advertisement watchers (AirPods / Google Fast Pair), Classic Bluetooth RFCOMM (Samsung Galaxy Buds / Nothing Ear), Windows Phone Link local JSON cache, virtual COM serial ports (`atk_serial`), vendor BLE GATT (`gamesir_ble`), Windows PnP cached generic Bluetooth properties, and external `headsetcontrol.exe` delegation.

---

Related docs: [Contributing a device](contributing-device.md) · [Platform support](platform-support.md) · [Back to README](../README.md)
