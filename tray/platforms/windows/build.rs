//! Embeds `tray/assets/app-icon.ico` into the Windows executable as a native Win32
//! `RT_GROUP_ICON` / `RT_ICON` resource (resource ID `1`) with zero external crates
//! or external resource-compiler tools.

use std::env;
use std::fs;
use std::path::PathBuf;

const RT_ICON: u16 = 3;
const RT_GROUP_ICON: u16 = 14;
const GROUP_ICON_ID: u16 = 1;
const LANG_EN_US: u16 = 0x0409;

fn main() {
    let manifest_dir = PathBuf::from(env::var("CARGO_MANIFEST_DIR").expect("CARGO_MANIFEST_DIR"));
    let ico_path = manifest_dir.join("../../assets/app-icon.ico");
    println!("cargo:rerun-if-changed={}", ico_path.display());
    println!("cargo:rerun-if-changed=build.rs");

    let target_os = env::var("CARGO_CFG_TARGET_OS").unwrap_or_default();
    let target_env = env::var("CARGO_CFG_TARGET_ENV").unwrap_or_default();
    if target_os != "windows" || target_env != "msvc" {
        return;
    }

    let ico_bytes = fs::read(&ico_path)
        .unwrap_or_else(|err| panic!("failed to read {}: {err}", ico_path.display()));
    let res_bytes = build_win32_icon_res(&ico_bytes)
        .unwrap_or_else(|err| panic!("failed to parse {}: {err}", ico_path.display()));

    let out_dir = PathBuf::from(env::var("OUT_DIR").expect("OUT_DIR"));
    let res_path = out_dir.join("app-icon.res");
    fs::write(&res_path, res_bytes)
        .unwrap_or_else(|err| panic!("failed to write {}: {err}", res_path.display()));

    println!(
        "cargo:rustc-link-arg-bin=wireless-battery-tray={}",
        res_path.display()
    );
}

struct IcoEntry<'a> {
    width: u8,
    height: u8,
    color_count: u8,
    reserved: u8,
    planes: u16,
    bit_count: u16,
    data: &'a [u8],
}

fn read_u16_le(buf: &[u8], offset: usize) -> Result<u16, String> {
    let slice = buf
        .get(offset..offset + 2)
        .ok_or_else(|| format!("unexpected EOF at offset {offset}"))?;
    Ok(u16::from_le_bytes([slice[0], slice[1]]))
}

fn read_u32_le(buf: &[u8], offset: usize) -> Result<u32, String> {
    let slice = buf
        .get(offset..offset + 4)
        .ok_or_else(|| format!("unexpected EOF at offset {offset}"))?;
    Ok(u32::from_le_bytes([slice[0], slice[1], slice[2], slice[3]]))
}

fn parse_ico(ico: &[u8]) -> Result<Vec<IcoEntry<'_>>, String> {
    if ico.len() < 6 {
        return Err("ICO file shorter than 6-byte header".to_owned());
    }
    let reserved = read_u16_le(ico, 0)?;
    let icon_type = read_u16_le(ico, 2)?;
    let count = read_u16_le(ico, 4)? as usize;
    if reserved != 0 || icon_type != 1 || count == 0 {
        return Err(format!(
            "invalid ICO header: reserved={reserved}, type={icon_type}, count={count}"
        ));
    }

    let mut entries = Vec::with_capacity(count);
    for idx in 0..count {
        let base = 6 + idx * 16;
        if ico.len() < base + 16 {
            return Err(format!("truncated ICO directory entry {idx}"));
        }
        let width = ico[base];
        let height = ico[base + 1];
        let color_count = ico[base + 2];
        let entry_reserved = ico[base + 3];
        let planes = read_u16_le(ico, base + 4)?;
        let bit_count = read_u16_le(ico, base + 6)?;
        let bytes_in_res = read_u32_le(ico, base + 8)? as usize;
        let image_offset = read_u32_le(ico, base + 12)? as usize;

        let end = image_offset
            .checked_add(bytes_in_res)
            .ok_or_else(|| format!("ICO entry {idx} offset overflow"))?;
        let data = ico
            .get(image_offset..end)
            .ok_or_else(|| format!("ICO entry {idx} points past EOF"))?;

        entries.push(IcoEntry {
            width,
            height,
            color_count,
            reserved: entry_reserved,
            planes,
            bit_count,
            data,
        });
    }
    Ok(entries)
}

fn build_win32_icon_res(ico: &[u8]) -> Result<Vec<u8>, String> {
    let entries = parse_ico(ico)?;
    let mut res = Vec::with_capacity(ico.len() + 512);

    // 32-bit Win32 RES file mandatory empty header record.
    write_resource_record(&mut res, 0, 0, 0, &[]);

    // Individual RT_ICON records (IDs 1..=N).
    for (idx, entry) in entries.iter().enumerate() {
        let icon_id = u16::try_from(idx + 1).map_err(|_| "too many ICO frames".to_owned())?;
        write_resource_record(&mut res, RT_ICON, icon_id, LANG_EN_US, entry.data);
    }

    // RT_GROUP_ICON directory record (ID 1).
    let mut group = Vec::with_capacity(6 + entries.len() * 14);
    group.extend_from_slice(&0u16.to_le_bytes()); // idReserved
    group.extend_from_slice(&1u16.to_le_bytes()); // idType = 1 (icon)
    group.extend_from_slice(&(entries.len() as u16).to_le_bytes()); // idCount
    for (idx, entry) in entries.iter().enumerate() {
        let icon_id = (idx + 1) as u16;
        group.push(entry.width);
        group.push(entry.height);
        group.push(entry.color_count);
        group.push(entry.reserved);
        group.extend_from_slice(&entry.planes.to_le_bytes());
        group.extend_from_slice(&entry.bit_count.to_le_bytes());
        group.extend_from_slice(&(entry.data.len() as u32).to_le_bytes());
        group.extend_from_slice(&icon_id.to_le_bytes());
    }
    write_resource_record(&mut res, RT_GROUP_ICON, GROUP_ICON_ID, LANG_EN_US, &group);

    Ok(res)
}

fn write_resource_record(out: &mut Vec<u8>, res_type: u16, res_id: u16, lang_id: u16, data: &[u8]) {
    let data_size = data.len() as u32;
    let header_size: u32 = 32; // Fixed size when both TYPE and NAME are 16-bit numeric ordinals.

    out.extend_from_slice(&data_size.to_le_bytes());
    out.extend_from_slice(&header_size.to_le_bytes());
    // TYPE: 0xFFFF + ordinal
    out.extend_from_slice(&0xFFFFu16.to_le_bytes());
    out.extend_from_slice(&res_type.to_le_bytes());
    // NAME: 0xFFFF + ordinal
    out.extend_from_slice(&0xFFFFu16.to_le_bytes());
    out.extend_from_slice(&res_id.to_le_bytes());
    out.extend_from_slice(&0u32.to_le_bytes()); // DataVersion
    out.extend_from_slice(&0x0030u16.to_le_bytes()); // MemoryFlags (MOVEABLE | PURE)
    out.extend_from_slice(&lang_id.to_le_bytes()); // LanguageId
    out.extend_from_slice(&0u32.to_le_bytes()); // Version
    out.extend_from_slice(&0u32.to_le_bytes()); // Characteristics
    out.extend_from_slice(data);

    // Every RES record must be 4-byte aligned.
    while out.len() % 4 != 0 {
        out.push(0);
    }
}
