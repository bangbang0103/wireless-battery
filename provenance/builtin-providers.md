# Built-in Provider Provenance and Verification Status

This file tracks protocol attribution and hardware verification status separately for all **153 built-in providers** (`151` vendor HID providers + `os.xinput` + `bluetooth.bas`), as referenced by [`NOTICE`](../NOTICE) and [`src/providers/vendors/mod.rs`](../src/providers/vendors/mod.rs).

> **Important distinction**: Replay coverage with `source: "synthetic"` verifies parser correctness, framing, checksums, error paths, and `MatchDecision` isolation, **not** physical hardware behavior. Only providers backed by sanitized real-hardware captures (`source: "real"`) carry `SupportStatus::Verified`.

---

## Summary Counts

| Category | Count | Verification Status | Evidence Location |
|---|---|---|---|
| Real-hardware verified vendor HID | **2** | `Verified` | `tests/fixtures/vendors/{logitech,wob}/*_real.json` |
| Synthetic-replayed vendor HID | **149** | `Experimental` | `tests/fixtures/vendors/` & `tests/extended_vendor/` |
| Standard OS / transport providers | **2** | `Experimental` | `tests/fixtures/vendors/os/xinput_slot0.json`, `tests/fixtures/synthetic/bluetooth_bas.json` |
| **Total built-in providers** | **153** | `2 Verified / 151 Experimental` | Enforced in `tests/extended_vendor/mod.rs` |

---

## Verified Providers (`SupportStatus::Verified`)

| Provider ID | Device / Protocol | Module | Real-Hardware Fixtures (`source: "real"`) | Attribution |
|---|---|---|---|---|
| `logitech.hidpp` | Logitech HID++ 2.0 receivers & mice/keyboards (`0x1000`, `0x1004`, `0x1001`, `0x1F20`) | `src/providers/vendors/brands/logitech.rs` | [`tests/fixtures/vendors/logitech/hidpp_real.json`](../tests/fixtures/vendors/logitech/hidpp_real.json) | `Voltide` / `BatteryHub` (`NOTICE`); real-hardware capture |
| `wob.rainy75` | WOB Rainy75 (`320F:5088`) | `src/providers/vendors/brands/keyboards_vendors.rs` | [`tests/fixtures/vendors/wob/rainy75_real.json`](../tests/fixtures/vendors/wob/rainy75_real.json), [`rainy75_charging_real.json`](../tests/fixtures/vendors/wob/rainy75_charging_real.json) | `Voltide` (`NOTICE`); real-hardware discharging & charging captures |

---

## Standard OS Providers (`SupportStatus::Experimental`)

| Provider ID | Source | Module | Synthetic Replay Fixture | Attribution & Notes |
|---|---|---|---|---|
| `os.xinput` | Windows `XInputGetBatteryInformation` (`XInput1_4.dll` / `XInput1_3.dll`) | `src/providers/os/xinput.rs` | [`tests/fixtures/vendors/os/xinput_slot0.json`](../tests/fixtures/vendors/os/xinput_slot0.json) | Windows-only discrete 4-level battery reporting (`Empty` / `Low` / `Medium` / `Full`; no synthetic `%`) |
| `bluetooth.bas` | Cross-platform BLE Battery Service (`0x2A19`) via `btleplug` | `src/providers/os/bluetooth_bas.rs` | [`tests/fixtures/synthetic/bluetooth_bas.json`](../tests/fixtures/synthetic/bluetooth_bas.json) | Standard Bluetooth SIG GATT BAS; read-only, passive discovery, excludes Classic/HFP |

---

## Vendor HID Protocol Families (`src/providers/vendors/protocols/`, `SupportStatus::Experimental`)

Protocol definitions informed by `Wireless-Device-Battery-Monitor / BatteryHub` (GPL-3.0-or-later) and `Voltide` (MIT) unless otherwise noted in [`NOTICE`](../NOTICE):

| Module | Protocol Family | Provider IDs | Notes |
|---|---|---|---|
| `bolutek.rs` | Bolutek `0x55 0x65` headset protocol | `atk.n9v2`, `mchose.v9`, `vgn.omnisound` | Shared `0x55 0x65 0x01` query framing (`FixedSpec` for `atk.n9v2`/`mchose.v9`, `0x000C:0x0001` set/output for `vgn.omnisound`); fixtures under `tests/fixtures/vendors/{atk,mchose}/` and `tests/extended_vendor/` |
| `compx.rs` | CompX `0x08` 17-byte `0x55`-sum protocol | `vxe.r1`, `atk.a9_air`, `imecoo.p1`, `kysona.mouse`, `chimera.v2`, `cherry.xtrfy`, `corsair.compx` | `kysona.mouse` uses `MatchDecision::Family`; exact PIDs (`vxe.r1`, `atk.a9_air`, `imecoo.p1`, `chimera.v2`) automatically take precedence |
| `drivercore.rs` | DriverCore `0x83` mouse protocol family (`0x00` feature reports) | `glorious.mouse`, `hitscan.mouse`, `gwolves.mouse`, `arbiter.akitsu`, `lamzu.mouse`, `wlmouse.mouse` | Shared `0x02 0x02 .. 0x83` feature-report framing with brand-specific cable PID detection and polling delays |
| `evision.rs` | EVision `0xFF1C:0x0092` (`0x04` output report) | `cherry.mx3s`, `akko.dash`, `evision.atk_keyboard`, `atk.z87`, `evision.redragon_m999`, `evision.ajazz_ak992`, `evision.aula_mouse` | Shared `0x04` command/status framing (`0x1A` query, byte 8 percentage, byte 9 charge state) |

---

## Vendor Brand Modules (`src/providers/vendors/brands/`, `SupportStatus::Experimental` except as noted)

All providers in the modules below are currently `SupportStatus::Experimental` (except `logitech.hidpp` and `wob.rainy75` listed under Verified above). Protocol framing and device tables draw from the upstream GPL-3.0-or-later (`BatteryHub`), MIT (`Voltide`), and community references listed in [`NOTICE`](../NOTICE), with `cargo test --locked --all-features` enforcing 153/153 replay test coverage:

| Module | Coverage Scope | External References (`NOTICE`) |
|---|---|---|
| `asus_omni_headset.rs` | ASUS ROG Omni (`0B05:1ACE`), ROG Delta II / Pelta, ROG Cetra | `BatteryHub` (GPL-3.0-or-later) / `Voltide` (MIT) |
| `asus_peripherals.rs` | ASUS ROG standalone keyboards & mice (`asus.rog_peripheral`) | `BatteryHub` (GPL-3.0-or-later) / `Voltide` (MIT) |
| `corsair.rs` | Corsair Void / HS70 / HS80 / Bragi headsets, mice & keyboards | `BatteryHub` (GPL-3.0-or-later) / `Voltide` (MIT) |
| `dareu.rs` | DAREU A980/A950 series & Zenith Pro | `BatteryHub` / `Voltide` |
| `eweadn.rs` | EWEADN S9 Pro & A5 Pro (incl. cached channel resolution) | `BatteryHub` / `Voltide` |
| `gamepads_console.rs` | Nintendo Switch Pro, Valve Steam Controller, 8BitDo Vendor & DInput | `BatteryHub` / `Voltide` |
| `gamepads_pc.rs` | GameSir (Nova / Cyclone 2 / SAPJoy), Flydigi, BEITONG KP20/KP40D | `BatteryHub` / `Voltide` |
| `headsets_airoha.rs` | Airoha RACE (`Audeze Maxwell`, `ENDORFY Viro`, `AKG N9 Hybrid`, `Turtle Beach Stealth 700 Gen 3`) | `BatteryHub` / `Voltide` |
| `headsets_audio.rs` | Jabra GNP, JBL Quantum HIDV1/HIDV3, Poly BT600, Fractal Scape, Alienware AW920H, MOONDROP, XIBERIA T7, MAONO PD200W, DARKRIM | `BatteryHub` / `Voltide` |
| `hyperx.rs` | HyperX Cloud II/III/III S/Alpha/Alpha 2/Flight/Stinger Core & Pulsefire Haste 2 | `BatteryHub` / `Voltide` |
| `keyboards_vendors.rs` | `wob.rainy75` (**Verified**), NuPhy WH80, AULA HERO 68 Air, SONiX `0xFF60`, PMO/VIA `36B0`, Semico `1A2C` | `Voltide` / `BatteryHub` |
| `keychron.rs` | Keychron Nape Pro, Legacy 1K, Ultra-Link 8K, Keychron Launcher (`Family`), QMK Link (`Family`) | `BatteryHub` / `Voltide` |
| `logitech.rs` | `logitech.hidpp` (**Verified**), `logitech.g435`, `astro.a50_modern`, `astro.legacy` (`Experimental`) | `BatteryHub` / `Voltide` |
| `mchose_keyboards.rs` | MCHOSE keyboards (`mchose.keyboard`, `0x3837` `0xFF01`) & MCHOSE V9 Pro 2 | `BatteryHub` / `mitsimi/mchose-tray` |
| `mchose_mice.rs` | MCHOSE K7 V2 Ultra+, A5 (`Family`), A7 Ultra, A5 Gen1, G3 V2, modern mice | `BatteryHub` / `mitsimi/mchose-tray` / `Rapou7/offline-mchose` |
| `mice_esports.rs` | Ninjutso Sora V2 / Sprime PM1 / Ten, Pulsar X2 V3, VAXEE 4K/8K, Endgame Gear 4K | `BatteryHub` / `Voltide` |
| `mice_gaming.rs` | Cooler Master, NZXT, MSI, Fantech XD5, Raigor, Sinowealth, Royal Kludge, Darmoshark, Machenike, Niceboy, AULA SC580 | `BatteryHub` / `Voltide` |
| `mice_nordic.rs` | ATK (`yiwluoq` / Zero / NearLink / Nano 8K), Finalmouse, Scyrox, RAWM/SADES, ZOWIE EC-CW | `BatteryHub` / `Voltide` / `yiwluoq` |
| `motospeed_rongyuan.rs` | Motospeed, Xenta, Delux, Ajazz (`ajazz.aj_series` `Family`), Incott, Rongyuan, TAIDU, CROCIRIS, Press Play | `BatteryHub` / `Voltide` |
| `rapoo.rs` | Rapoo VT3S, Nordic VT series, Legacy VT Pro | `BatteryHub` / `Voltide` |
| `razer_headsets.rs` | Razer BlackShark V2/V3/V3 X, Barracuda X & Barracuda Pro | `BatteryHub` / `Voltide` |
| `razer_mice.rs` | Razer wireless mice & keyboards transaction protocol (`0x1F` / `0x90`) | `BatteryHub` / `Voltide` |
| `sony.rs` | Sony DualSense (`USB` / `BT`), DualShock 4, INZONE headsets | `BatteryHub` / `Voltide` |
| `steelseries_headsets.rs` | SteelSeries Arctis 1/7/9/Pro/Nova family | `BatteryHub` / `Voltide` |
| `steelseries_mice.rs` | SteelSeries Aerox / Rival / Rival Pro | `BatteryHub` / `Voltide` |

---

## Graduation from `Experimental` to `Verified`

An `Experimental` provider promotes to `SupportStatus::Verified` only when:

1. A maintainer or contributor captures raw protocol traffic from physical hardware using `wireless-battery probe` / `RecordTransport` (with official vendor software closed).
2. A sanitized fixture with `"source": "real"` (stripped of serial numbers, OS paths, and unrelated USB traffic) is committed under `tests/fixtures/vendors/<vendor>/`.
3. Discharging and (where supported) charging/offline edge behaviors match the real capture without percentage clamping or fabricated sentinels.
