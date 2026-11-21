# Support matrix

> **Language / 语言**: **English** | [简体中文](support-matrix.zh-CN.md)

Built-in vendor providers are **experimental** unless a specific device and capability say **verified**. “Replay” alone means a synthetic JSON session passes strict replay; it is not independent hardware verification. WOB Rainy75 percentage/charging/discharging and Logitech GPW3 percentage/discharging have real-hardware captures verified through this library.

| Vendor | Device / Match | Support method | Provider ID | Percentage | Charge state | Voltage | Evidence |
|---|---|---|---|---|---|---|---|
| Logitech | GPW3 `046D:C547` (verified); HID++ (`0x1004`/`0x1000`/`0x1001`/`0x1F20`) receivers (experimental) | Built-in Rust HID (Verified / Experimental) | `logitech.hidpp` | **verified: 46% real 0x1004/function-1 reading + replay** | discharging verified; charging/full not hardware-verified | experimental (`0x1001`/`0x1F20`) | real + synthetic normal/fallback |
| MCHOSE / AULA / Redragon | BY keyboards (`258A:01BB`, `3554:FA09/FB40`, `41E4:2001`, `05AC:024F` if1) | Built-in Rust HID (Experimental) | `mchose.keyboard` | replay | replay | unsupported | synthetic |
| MCHOSE | A7 Ultra, VID `5253` | Built-in Rust HID (Experimental) | `mchose.a7_ultra` | replay | replay | unsupported | synthetic |
| VXE | R1 Star Flash `373B:10C9` | Built-in Rust HID (Experimental) | `vxe.r1` | replay | replay | replay | synthetic |
| EWEADN | S9 Pro `372E:103D` / `1027` | Built-in Rust HID (Experimental) | `eweadn.s9_pro` | replay; discarded while charging | charging/discharging replay | unsupported | synthetic |
| EWEADN | A5 Pro `258A:0138` | Built-in Rust HID (Experimental) | `eweadn.a5_pro` | replay | unknown; no USB heuristic | unsupported | synthetic |
| DAREU | A950 Air V2 `260D:1175` | Built-in Rust HID (Experimental) | `dareu.a950_air_v2` | replay | replay; full mapping implemented | replay | synthetic |
| MCHOSE | V9 Turbo+ `3837:600A` | Built-in Rust HID (Experimental) | `mchose.v9` | replay | replay; full mapping implemented | unsupported | synthetic |
| MCHOSE | K7 V2 Ultra+ `3837:1014` | Built-in Rust HID (Experimental) | `mchose.k7_v2_ultra` | replay | replay | unsupported | synthetic |
| MCHOSE | A5-family radio child, VID `3837` | Built-in Rust HID (Experimental) | `mchose.a5_mouse` | replay | replay | unsupported | synthetic |
| ASUS ROG | ROG Omni `0B05:1ACE`, static key/mouse channels | Built-in Rust HID (Experimental) | `asus.rog_omni` | replay | replay | unsupported | synthetic |
| ASUS ROG | ROG Delta II / Pelta and named headsets | Built-in Rust HID (Experimental) | `asus.headset` | replay | replay | unsupported | synthetic |
| TAIDU | TSG809 and documented VID family | Built-in Rust HID (Experimental) | `taidu.mouse` | replay | replay | unsupported | synthetic |
| IMECOO | P1 `3554:FB34` | Built-in Rust HID (Experimental) | `imecoo.p1` | replay | replay | replay | synthetic |
| ATK | N9V2 Ultimate `373B:1262` | Built-in Rust HID (Experimental) | `atk.n9v2` | replay | replay; full mapping implemented | unsupported | synthetic |
| ATK | A9 Air+ `373B:1278` | Built-in Rust HID (Experimental) | `atk.a9_air` | replay | replay | replay | synthetic |
| Microsoft / XInput | Windows XInput controller slots | OS XInput API (Experimental) | `os.xinput` | unsupported; four levels only | unknown | unsupported | synthetic discrete-level replay |
| CROCIRIS | X7 `36F9:AA20` | Built-in Rust HID (Experimental) | `crociris.x7` | replay | replay | unsupported | synthetic |
| XIBERIA | T7 / T7 GT | Built-in Rust HID (Experimental) | `xiberia.t7` | replay | unknown | unsupported | synthetic |
| Sony | DualSense / DualSense Edge | Built-in Rust HID (Experimental) | `sony.dualsense` | replay; device-reported 10% steps | replay; full mapping implemented | unsupported | synthetic USB report |
| Rapoo | VT3S `24AE:1411` | Built-in Rust HID (Experimental) | `rapoo.vt3s` | replay | replay | unsupported | synthetic push report |
| Razer | Known wireless Razer mouse & keyboard PID list | Built-in Rust HID (Experimental) | `razer.mouse` | replay; 0–255 linear scale | replay | unsupported | synthetic feature transaction |
| WOB | Rainy75 `320F:5088`, `FF1C:0092` | Built-in Rust HID (Verified) | `wob.rainy75` | **verified: 50% / 62% real readings + replay** | **verified: discharging (`0x00`) and charging (`0x01`); other values stay unknown** | unsupported | real (discharging + charging) + synthetic fixtures |
| Razer | BlackShark V2 Pro `1532:0555` | Built-in Rust HID (Experimental) | `razer.headset` | replay | replay | unsupported | synthetic |

## Extended vendor HID providers (`experimental`, strict synthetic replay)

The following **128 additional built-in HID providers** reference [Wireless-Device-Battery-Monitor / BatteryHub](../NOTICE) (GPL-3.0-or-later) and are covered by zero-conflict strict wire-level replay tests in `tests/extended_vendor_replay_tests.rs`. All remain `experimental` until independently verified on real hardware.

### Headsets & Audio

| Vendor / Brand | Devices | Support method | Provider IDs |
|---|---|---|---|
| SteelSeries | Arctis 1 / 7 / 7+ / 7P / 7X / 9 / Pro Wireless, Arctis Nova 5 / 7 / Pro Wireless / Elite | Built-in Rust HID (Experimental, Replay) | `steelseries.arctis1`, `steelseries.arctis7`, `steelseries.arctis9`, `steelseries.arctis_pro_wireless`, `steelseries.nova_pro_wireless`, `steelseries.nova_family`, `steelseries.nova_elite` |
| HyperX | Cloud II / III / III S / Alpha / Alpha 2 / Flight / Stinger Core | Built-in Rust HID (Experimental, Replay) | `hyperx.cloud3`, `hyperx.cloud3_s`, `hyperx.cloud2`, `hyperx.cloud2_kingston`, `hyperx.cloud_alpha`, `hyperx.alpha2_haste`, `hyperx.cloud_flight`, `hyperx.stinger_core` |
| Corsair | Void / Void Pro / Void Elite / HS70 / HS80 / Virtuoso (and Bragi mice/keyboards) | Built-in Rust HID (Experimental, Replay) | `corsair.void`, `corsair.bragi` |
| Razer | BlackShark V2 HyperSpeed / V2 Pro 2020 / V3 / V3 X, Barracuda X / Pro | Built-in Rust HID (Experimental, Replay) | `razer.blackshark_v2_hs`, `razer.barracuda_x`, `razer.blackshark_v2_pro_2020`, `razer.blackshark_v3_x`, `razer.blackshark_v3`, `razer.barracuda_pro` |
| Logitech / Astro / Sony / ASUS | Logitech G435, Astro A50 (Gen 3/4/5/X) & A20/A30, Sony INZONE H5/H7/H9/Buds, ROG Cetra | Built-in Rust HID (Experimental, Replay) | `logitech.g435`, `astro.a50_modern`, `astro.legacy`, `sony.inzone`, `asus.rog_cetra` |
| Other Headsets & Audio (Audeze, Endorfy, AKG, Turtle Beach, Jabra, JBL, Poly, Fractal, Alienware, Moondrop, MCHOSE, VGN, Darkrim, Maono) | Audeze Maxwell, Endorfy Viro, AKG N9 Hybrid, Turtle Beach Stealth 700, Jabra Link, JBL Quantum, Poly BT600, Fractal Scape, Alienware AW920H, Moondrop GM01 Pro, MCHOSE V9 Pro2, VGN OmniSound, Darkrim Shadow, Maono PD200W | Built-in Rust HID (Experimental, Replay) | `audeze.maxwell`, `endorfy.viro`, `akg.n9_hybrid`, `turtlebeach.stealth700`, `jabra.gnp`, `jbl.hidv1`, `jbl.hidv3`, `plantronics.bt600`, `fractal.scape`, `alienware.aw920h`, `moondrop.gm01_pro`, `mchose.v9_pro2`, `vgn.omnisound`, `darkrim.shadow`, `maono.pd200w` |

### Mice & Multi-device Receivers

| Vendor / Brand | Devices | Support method | Provider IDs |
|---|---|---|---|
| ASUS ROG & SteelSeries | ROG Harpe/Gladius/Keris/Chakram/Azoth/Falchion, Aerox 3/5/9, Prime/Rival Wireless | Built-in Rust HID (Experimental, Replay) | `asus.rog_peripheral`, `steelseries.aerox`, `steelseries.rival`, `steelseries.rival_pro` |
| DriverCore / Nordic 8K (Glorious, Hitscan, G-Wolves, Arbiter, Lamzu, WLmouse, Ninjutso, Sprime, Pulsar, VAXEE, Endgame Gear) | Glorious Model O/D/I, Hitscan Hyperlight, G-Wolves, Arbiter Akitsu, Lamzu Maya/Atlantis/Thorn, WLmouse Beast X, Ninjutso Sora V2/Ten, Sprime PM1, Pulsar X2 V3, VAXEE, Endgame Gear OP1w/XM2w 4K | Built-in Rust HID (Experimental, Replay) | `glorious.mouse`, `hitscan.mouse`, `gwolves.mouse`, `arbiter.akitsu`, `lamzu.mouse`, `wlmouse.mouse`, `ninjutso.sora_v2`, `sprime.pm1`, `ninjutso.ten`, `pulsar.x2_v3`, `vaxee.mouse`, `endgame.4k` |
| CompX 0x08 & MCHOSE / ATK (Kysona, Chimera, Cherry Xtrfy, Corsair, MCHOSE, ATK) | Kysona, Chimera V2, Cherry Xtrfy, Corsair M75/Katar (CompX), MCHOSE A5/G3 V2/A7/L7, ATK F1/X1/Z1/Dragonfly | Built-in Rust HID (Experimental, Replay) | `kysona.mouse`, `chimera.v2`, `cherry.xtrfy`, `corsair.compx`, `mchose.a5_gen1`, `mchose.g3_v2`, `mchose.modern_mouse`, `atk.zero`, `atk.bitmouse_nk`, `atk.nano_8k` |
| Enthusiast & OEM Mice (Finalmouse, Scyrox, RAWM, ZOWIE, Cooler Master, NZXT, MSI, Fantech, Raigor, Sinowealth, Royal Kludge, Darmoshark, Machenike, Niceboy, Motospeed, Xenta, Delux, Ajazz, Incott, Rongyuan, DAREU, Zenith, Rapoo, Press Play) | Finalmouse ULX, Scyrox V6/V8, RAWM, ZOWIE EC-CW, Cooler Master, NZXT Lift Elite, MSI Clutch, Fantech, Raigor, Sinowealth, Royal Kludge, Darmoshark, Machenike, Niceboy, Motospeed, Xenta, Delux M800 Pro, Ajazz AJ139/AJ159/AJ179, Incott, Rongyuan, DAREU Togran, Zenith Pro, Rapoo VT/VT3 Pro, Press Play Icarus | Built-in Rust HID (Experimental, Replay) | `finalmouse.mouse`, `scyrox.v12m`, `rawm.mouse`, `zowie.ec_cw`, `coolermaster.mouse`, `nzxt.lift_elite`, `msi.mouse`, `msi.gm41`, `fantech.xd5`, `raigor.mouse`, `sinowealth.receiver`, `royal_kludge.mg6_pro`, `darmoshark.m3`, `darmoshark.nrf`, `machenike.l7`, `niceboy.mw400`, `motospeed.x6`, `xenta.mouse`, `delux.m800_pro`, `ajazz.aj179`, `ajazz.aj_series`, `incott.mouse`, `rongyuan.receiver`, `dareu.togran`, `zenith.pro`, `rapoo.vt`, `rapoo.legacy_vt3_pro`, `pressplay.icarus` |

### Keyboards & Keyboard/Mouse Suites

| Vendor / Brand | Devices | Support method | Provider IDs |
|---|---|---|---|
| Keychron / Lemokey | Keychron M-series / Lemokey mice, UltraLink 8K receivers, Keychron Q/K/V/P/Max/Pro QMK wireless keyboards | Built-in Rust HID (Experimental, Replay) | `keychron.nape`, `keychron.legacy_mouse`, `keychron.ultralink`, `keychron.launcher_mouse`, `keychron.qmk_keyboard` |
| Cherry / Akko / NuPhy / MCHOSE / DAREU | Cherry MX 3.0S/3.1, Akko/MonsGeek Dash, NuPhy Air/WH80, MCHOSE GLW & K87 V3, DAREU A84 | Built-in Rust HID (Experimental, Replay) | `cherry.mx3s`, `akko.dash`, `nuphy.wh80`, `mchose.glw_keyboard`, `mchose.k87_v3`, `dareu.a84` |
| EVision / AULA / Sonix / VIA / Semico | EVision-based ATK/Redragon/Ajazz/AULA keyboards & mice, AULA HERO 68 / SC580, Sonix/VIA/Semico wireless keyboards | Built-in Rust HID (Experimental, Replay) | `evision.atk_keyboard`, `atk.z87`, `evision.redragon_m999`, `evision.ajazz_ak992`, `evision.aula_mouse`, `aula.hero68`, `aula.sc580`, `sonix.keyboard`, `via.dongle`, `semico.keyboard` |

### Gamepads & Controllers

| Vendor / Brand | Devices | Support method | Provider IDs |
|---|---|---|---|
| Nintendo / Sony / Valve / GameSir / Flydigi / 8BitDo / Beitong | Nintendo Switch Pro / Joy-Con, Sony DualShock 4, Valve Steam Controller, GameSir Nova/Cyclone 2/SapJoy, Flydigi Apex/Vader/Direwolf, 8BitDo Ultimate/Pro, Beitong | Built-in Rust HID (Experimental, Replay) | `nintendo.switch_pro`, `sony.dualshock4`, `valve.steam_controller`, `gamesir.nova`, `gamesir.cyclone2`, `gamesir.sapjoy`, `flydigi.controller`, `eightbitdo.vendor`, `eightbitdo.dinput`, `beitong.controller` |

## Standard protocols

| Vendor / Standard | Devices | Support method | Lifecycle | Percentage | Charging | Platforms | Limitations |
|---|---|---|---|---|---|---|---|
| Bluetooth SIG (`bluetooth.bas`) | Known BLE peripherals | Read-only BLE BAS (`0x2A19`) | experimental | protocol defined; hardware unverified | unknown / unsupported | Windows, Linux BlueZ, macOS backend (enabled by default) | known peripherals only; explicit read may connect; no Classic/HFP |

## Deferred non-HID and dual-collection protocols

The following source types from upstream tools are intentionally **not** ported into the single-session HID layer yet:

- **Split-collection dual-handle HID devices** requiring two simultaneous HID handles (`Endgame Gear WE`, `Areson`, `DAREU EM901X / SGM50W`, `ZOWIE DW`, `Wraith W1`).
- **Non-HID / OS-specific / external-binary sources**: active BLE advertisement watchers (AirPods / Google Fast Pair), Classic Bluetooth RFCOMM (Samsung Galaxy Buds / Nothing Ear), Windows Phone Link local JSON cache, virtual COM serial ports (`atk_serial`), vendor BLE GATT (`gamesir_ble`), Windows PnP cached generic Bluetooth properties, and external `headsetcontrol.exe` delegation.

## Capability evidence

Package lifecycle and per-capability evidence are separate. A replayed percentage does not imply verified charging, full, low, sleeping, or offline behavior. Full, low, and sleep samples are encouraged but not contribution blockers. Synthetic packages under `fixtures/synthetic/` are test assets and never real support.

---

Related docs: [Verification checklist](hardware-verification.md) · [Contributing a device](contributing-device.md) · [Platform support](platform-support.md) · [Back to README](../README.md)
