//! Decoded-texture cache on the SD card. Opening a file on the 3DS costs tens of milliseconds whatever its size, so the
//! hundreds of 64 x 64 terrain textures a course needs take ~20 s to read one by one; the cache holds their decoded
//! pixels in one file (a few MB) that loads in a single read. Keyed by lowercase file name; entries never go stale
//! because the art set's textures do not change (a replaced art set: delete `3ds/opensgolf/cache`).
//!
//! Layout: `SGTX` u8 version, u32 count, then per image: u16 name length, name, u16 w, u16 h, w*h*4 RGBA bytes
//! (all little-endian).
use sg_core::assets::Rgba;
use std::collections::HashMap;
use std::path::Path;

const MAGIC: &[u8; 4] = b"SGTX";
const VERSION: u8 = 1;

pub fn key(path: &Path) -> String {
    path.file_name().map(|n| n.to_string_lossy().to_lowercase()).unwrap_or_default()
}

pub fn read(file: &Path) -> HashMap<String, Rgba> {
    let mut out = HashMap::new();
    let Ok(d) = std::fs::read(file) else { return out };
    let _ = parse(&d, &mut out);
    out
}

fn parse(d: &[u8], out: &mut HashMap<String, Rgba>) -> Option<()> {
    let mut at = 0usize;
    let take = |at: &mut usize, n: usize| -> Option<&[u8]> {
        let s = d.get(*at..*at + n)?;
        *at += n;
        Some(s)
    };
    if take(&mut at, 4)? != MAGIC || take(&mut at, 1)?[0] != VERSION {
        return None;
    }
    let count = u32::from_le_bytes(take(&mut at, 4)?.try_into().ok()?);
    for _ in 0..count {
        let n = u16::from_le_bytes(take(&mut at, 2)?.try_into().ok()?) as usize;
        let name = String::from_utf8_lossy(take(&mut at, n)?).into_owned();
        let w = u16::from_le_bytes(take(&mut at, 2)?.try_into().ok()?) as u32;
        let h = u16::from_le_bytes(take(&mut at, 2)?.try_into().ok()?) as u32;
        let px = take(&mut at, (w * h * 4) as usize)?.to_vec();
        out.insert(name, Rgba { w, h, px });
    }
    Some(())
}

/// Writes the whole cache; returns the byte count.
pub fn write(file: &Path, cache: &HashMap<String, Rgba>) -> Result<usize, String> {
    let fits = |n: &String| {
        let img = &cache[n];
        img.w <= u16::MAX as u32 && img.h <= u16::MAX as u32 && n.len() <= u16::MAX as usize
    };
    let mut names: Vec<&String> = cache.keys().filter(|n| fits(n)).collect();
    names.sort();
    let mut d: Vec<u8> = Vec::with_capacity(cache.values().map(|i| i.px.len() + 64).sum());
    d.extend_from_slice(MAGIC);
    d.push(VERSION);
    d.extend_from_slice(&(names.len() as u32).to_le_bytes());
    for n in names {
        let img = &cache[n];
        d.extend_from_slice(&(n.len() as u16).to_le_bytes());
        d.extend_from_slice(n.as_bytes());
        d.extend_from_slice(&(img.w as u16).to_le_bytes());
        d.extend_from_slice(&(img.h as u16).to_le_bytes());
        d.extend_from_slice(&img.px);
    }
    if let Some(dir) = file.parent() {
        std::fs::create_dir_all(dir).map_err(|e| e.to_string())?;
    }
    std::fs::write(file, &d).map_err(|e| e.to_string())?;
    Ok(d.len())
}
