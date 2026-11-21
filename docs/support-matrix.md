# Support matrix

> **Language / 语言**: **English** | [简体中文](support-matrix.zh-CN.md)

Built-in vendor providers are **experimental** unless a specific device and capability say **verified**. “Replay” alone means a synthetic JSON session passes strict replay; it is not independent hardware verification. WOB Rainy75 percentage/charging/discharging and Logitech GPW3 percentage/discharging have real-hardware captures verified through this library.

| Provider ID | Representative devices / match | Percentage | Charge state | Voltage | Evidence |
|---|---|---|---|---|---|
| `logitech.hidpp` | GPW3 `046D:C547` verified; HID++ (`0x1004`/`0x1000`/`0x1001`/`0x1F20`) receivers experimental | **verified: 46% real 0x1004/function-1 reading + replay** | discharging verified; charging/full not hardware-verified | experimental (`0x1001`/`0x1F20`) | real + synthetic normal/fallback |
| `mchose.keyboard` | MCHOSE / AULA / Redragon BY keyboards (`258A:01BB`, `3554:FA09/FB40`, `41E4:2001`, `05AC:024F` if1) | replay | replay | unsupported | synthetic |
| `mchose.a7_ultra` | MCHOSE A7 Ultra, VID `5253` | replay | replay | unsupported | synthetic |
| `vxe.r1` | VXE R1 Star Flash `373B:10C9` | replay | replay | replay | synthetic |
| `eweadn.s9_pro` | EWEADN S9 Pro `372E:103D` / `1027` | replay; discarded while charging | charging/discharging replay | unsupported | synthetic |
| `eweadn.a5_pro` | EWEADN A5 Pro `258A:0138` | replay | unknown; no USB heuristic | unsupported | synthetic |
| `dareu.a950_air_v2` | DAREU A950 Air V2 `260D:1175` | replay | replay; full mapping implemented | replay | synthetic |
| `mchose.v9` | MCHOSE V9 Turbo+ `3837:600A` | replay | replay; full mapping implemented | unsupported | synthetic |
| `mchose.k7_v2_ultra` | MCHOSE K7 V2 Ultra+ `3837:1014` | replay | replay | unsupported | synthetic |
| `mchose.a5_mouse` | MCHOSE A5-family radio child, VID `3837` | replay | replay | unsupported | synthetic |
| `asus.rog_omni` | ROG Omni `0B05:1ACE`, static key/mouse channels | replay | replay | unsupported | synthetic |
| `asus.headset` | ROG Delta II / Pelta and named headsets | replay | replay | unsupported | synthetic |
| `taidu.mouse` | TAIDU TSG809 and documented VID family | replay | replay | unsupported | synthetic |
| `imecoo.p1` | IMECOO P1 `3554:FB34` | replay | replay | replay | synthetic |
| `atk.n9v2` | ATK N9V2 Ultimate `373B:1262` | replay | replay; full mapping implemented | unsupported | synthetic |
| `atk.a9_air` | ATK A9 Air+ `373B:1278` | replay | replay | replay | synthetic |
| `os.xinput` | Windows XInput slots | unsupported; four levels only | unknown | unsupported | synthetic discrete-level replay |
| `crociris.x7` | CROCIRIS X7 `36F9:AA20` | replay | replay | unsupported | synthetic |
| `xiberia.t7` | XIBERIA T7 / T7 GT | replay | unknown | unsupported | synthetic |
| `sony.dualsense` | Sony DualSense / Edge | replay; device-reported 10% steps | replay; full mapping implemented | unsupported | synthetic USB report |
| `rapoo.vt3s` | Rapoo VT3S `24AE:1411` | replay | replay | unsupported | synthetic push report |
| `razer.mouse` | Known wireless Razer mouse & keyboard PID list | replay; 0–255 linear scale | replay | unsupported | synthetic feature transaction |
| `wob.rainy75` | WOB Rainy75 `320F:5088`, `FF1C:0092` | **verified: 50% / 62% real readings + replay** | **verified: discharging (`0x00`) and charging (`0x01`); other values stay unknown** | unsupported | real (discharging + charging) + synthetic fixtures |
| `razer.headset` | Razer BlackShark V2 Pro `1532:0555` | replay | replay | unsupported | synthetic |

## Extended vendor HID providers (`experimental`, strict synthetic replay)

The following **128 additional built-in HID providers** reference [Wireless-Device-Battery-Monitor / BatteryHub](../NOTICE) (GPL-3.0-or-later) and are covered by zero-conflict strict wire-level replay tests in `tests/extended_vendor_replay_tests.rs`. All remain `experimental` until independently verified on real hardware.

### Headsets & Audio

| Brand / Family | Provider IDs | Representative devices |
|---|---|---|
| SteelSeries | `steelseries.arctis1`, `steelseries.arctis7`, `steelseries.arctis9`, `steelseries.arctis_pro_wireless`, `steelseries.nova_pro_wireless`, `steelseries.nova_family`, `steelseries.nova_elite` | Arctis 1 / 7 / 7+ / 7P / 7X / 9 / Pro Wireless, Arctis Nova 5 / 7 / Pro Wireless / Elite |
| HyperX | `hyperx.cloud3`, `hyperx.cloud3_s`, `hyperx.cloud2`, `hyperx.cloud2_kingston`, `hyperx.cloud_alpha`, `hyperx.alpha2_haste`, `hyperx.cloud_flight`, `hyperx.stinger_core` | Cloud II / III / III S / Alpha / Alpha 2 / Flight / Stinger Core |
| Corsair | `corsair.void`, `corsair.bragi` | Void / Void Pro / Void Elite / HS70 / HS80 / Virtuoso (and Bragi mice/keyboards) |
| Razer Headsets | `razer.blackshark_v2_hs`, `razer.barracuda_x`, `razer.blackshark_v2_pro_2020`, `razer.blackshark_v3_x`, `razer.blackshark_v3`, `razer.barracuda_pro` | BlackShark V2 HyperSpeed / V2 Pro 2020 / V3 / V3 X, Barracuda X / Pro |
| Logitech / Astro / Sony / ASUS | `logitech.g435`, `astro.a50_modern`, `astro.legacy`, `sony.inzone`, `asus.rog_cetra` | Logitech G435, Astro A50 (Gen 3/4/5/X) & A20/A30, Sony INZONE H5/H7/H9/Buds, ROG Cetra |
| Other Headsets & Audio | `audeze.maxwell`, `endorfy.viro`, `akg.n9_hybrid`, `turtlebeach.stealth700`, `jabra.gnp`, `jbl.hidv1`, `jbl.hidv3`, `plantronics.bt600`, `fractal.scape`, `alienware.aw920h`, `moondrop.gm01_pro`, `mchose.v9_pro2`, `vgn.omnisound`, `darkrim.shadow`, `maono.pd200w` | Audeze Maxwell, Endorfy Viro, AKG N9 Hybrid, Turtle Beach Stealth 700, Jabra Link, JBL Quantum, Poly BT600, Fractal Scape, Alienware AW920H, Moondrop GM01 Pro, MCHOSE V9 Pro2, VGN OmniSound, Darkrim Shadow, Maono PD200W |

### Mice & Multi-device Receivers

| Brand / Family | Provider IDs | Representative devices |
|---|---|---|
| ASUS ROG & SteelSeries | `asus.rog_peripheral`, `steelseries.aerox`, `steelseries.rival`, `steelseries.rival_pro` | ROG Harpe/Gladius/Keris/Chakram/Azoth/Falchion, Aerox 3/5/9, Prime/Rival Wireless |
| DriverCore / Nordic 8K | `glorious.mouse`, `hitscan.mouse`, `gwolves.mouse`, `arbiter.akitsu`, `lamzu.mouse`, `wlmouse.mouse`, `ninjutso.sora_v2`, `sprime.pm1`, `ninjutso.ten`, `pulsar.x2_v3`, `vaxee.mouse`, `endgame.4k` | Glorious Model O/D/I, Hitscan Hyperlight, G-Wolves, Arbiter Akitsu, Lamzu Maya/Atlantis/Thorn, WLmouse Beast X, Ninjutso Sora V2/Ten, Sprime PM1, Pulsar X2 V3, VAXEE, Endgame Gear OP1w/XM2w 4K |
| CompX 0x08 & MCHOSE / ATK | `kysona.mouse`, `chimera.v2`, `cherry.xtrfy`, `corsair.compx`, `mchose.a5_gen1`, `mchose.g3_v2`, `mchose.modern_mouse`, `atk.zero`, `atk.bitmouse_nk`, `atk.nano_8k` | Kysona, Chimera V2, Cherry Xtrfy, Corsair M75/Katar (CompX), MCHOSE A5/G3 V2/A7/L7, ATK F1/X1/Z1/Dragonfly |
| Enthusiast & OEM Mice | `finalmouse.mouse`, `scyrox.v12m`, `rawm.mouse`, `zowie.ec_cw`, `coolermaster.mouse`, `nzxt.lift_elite`, `msi.mouse`, `msi.gm41`, `fantech.xd5`, `raigor.mouse`, `sinowealth.receiver`, `royal_kludge.mg6_pro`, `darmoshark.m3`, `darmoshark.nrf`, `machenike.l7`, `niceboy.mw400`, `motospeed.x6`, `xenta.mouse`, `delux.m800_pro`, `ajazz.aj179`, `ajazz.aj_series`, `incott.mouse`, `rongyuan.receiver`, `dareu.togran`, `zenith.pro`, `rapoo.vt`, `rapoo.legacy_vt3_pro`, `pressplay.icarus` | Finalmouse ULX, Scyrox V6/V8, RAWM, ZOWIE EC-CW, Cooler Master, NZXT Lift Elite, MSI Clutch, Fantech, Raigor, Sinowealth, Royal Kludge, Darmoshark, Machenike, Niceboy, Motospeed, Xenta, Delux M800 Pro, Ajazz AJ139/AJ159/AJ179, Incott, Rongyuan, DAREU Togran, Zenith Pro, Rapoo VT/VT3 Pro, Press Play Icarus |

### Keyboards & Keyboard/Mouse Suites

| Brand / Family | Provider IDs | Representative devices |
|---|---|---|
| Keychron / Lemokey | `keychron.nape`, `keychron.legacy_mouse`, `keychron.ultralink`, `keychron.launcher_mouse`, `keychron.qmk_keyboard` | Keychron M-series / Lemokey mice, UltraLink 8K receivers, Keychron Q/K/V/P/Max/Pro QMK wireless keyboards |
| Cherry / Akko / NuPhy / MCHOSE / DAREU | `cherry.mx3s`, `akko.dash`, `nuphy.wh80`, `mchose.glw_keyboard`, `mchose.k87_v3`, `dareu.a84` | Cherry MX 3.0S/3.1, Akko/MonsGeek Dash, NuPhy Air/WH80, MCHOSE GLW & K87 V3, DAREU A84 |
| EVision / AULA / Sonix / VIA / Semico | `evision.atk_keyboard`, `atk.z87`, `evision.redragon_m999`, `evision.ajazz_ak992`, `evision.aula_mouse`, `aula.hero68`, `aula.sc580`, `sonix.keyboard`, `via.dongle`, `semico.keyboard` | EVision-based ATK/Redragon/Ajazz/AULA keyboards & mice, AULA HERO 68 / SC580, Sonix/VIA/Semico wireless keyboards |

### Gamepads & Controllers

| Brand / Family | Provider IDs | Representative devices |
|---|---|---|
| Console & PC Controllers | `nintendo.switch_pro`, `sony.dualshock4`, `valve.steam_controller`, `gamesir.nova`, `gamesir.cyclone2`, `gamesir.sapjoy`, `flydigi.controller`, `eightbitdo.vendor`, `eightbitdo.dinput`, `beitong.controller` | Nintendo Switch Pro / Joy-Con, Sony DualShock 4, Valve Steam Controller, GameSir Nova/Cyclone 2/SapJoy, Flydigi Apex/Vader/Direwolf, 8BitDo Ultimate/Pro, Beitong |

## Standard protocols

| Source | Lifecycle | Percentage | Charging | Platforms | Limitations |
|---|---|---|---|---|---|
| Bluetooth LE BAS (`bluetooth.bas`) | experimental | protocol defined; hardware unverified | unknown / unsupported | optional Windows, Linux BlueZ, macOS backend | known peripherals only; explicit read may connect; no Classic/HFP |

## Deferred non-HID and dual-collection protocols

The following source types from upstream tools are intentionally **not** ported into the single-session HID layer yet:

- **Split-collection dual-handle HID devices** requiring two simultaneous HID handles (`Endgame Gear WE`, `Areson`, `DAREU EM901X / SGM50W`, `ZOWIE DW`, `Wraith W1`).
- **Non-HID / OS-specific / external-binary sources**: active BLE advertisement watchers (AirPods / Google Fast Pair), Classic Bluetooth RFCOMM (Samsung Galaxy Buds / Nothing Ear), Windows Phone Link local JSON cache, virtual COM serial ports (`atk_serial`), vendor BLE GATT (`gamesir_ble`), Windows PnP cached generic Bluetooth properties, and external `headsetcontrol.exe` delegation.

## Capability evidence

Package lifecycle and per-capability evidence are separate. A replayed percentage does not imply verified charging, full, low, sleeping, or offline behavior. Full, low, and sleep samples are encouraged but not contribution blockers. Synthetic packages under `fixtures/synthetic/` are test assets and never real support.

---

Related docs: [Verification checklist](hardware-verification.md) · [Contributing a device](contributing-device.md) · [Platform support](platform-support.md) · [Back to README](../README.md)
