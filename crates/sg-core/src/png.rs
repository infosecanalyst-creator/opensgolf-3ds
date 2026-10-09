//! Minimal PNG writer (RGBA8, zlib via miniz_oxide). For screenshots and asset export.
use crate::assets::Rgba;

fn crc_table() -> [u32; 256] {
    let mut t = [0u32; 256];
    for n in 0..256u32 {
        let mut c = n;
        for _ in 0..8 {
            c = if c & 1 != 0 { 0xEDB8_8320 ^ (c >> 1) } else { c >> 1 };
        }
        t[n as usize] = c;
    }
    t
}

fn chunk(out: &mut Vec<u8>, table: &[u32; 256], ty: &[u8; 4], body: &[u8]) {
    out.extend_from_slice(&(body.len() as u32).to_be_bytes());
    let start = out.len();
    out.extend_from_slice(ty);
    out.extend_from_slice(body);
    let mut c = 0xFFFF_FFFFu32;
    for &b in &out[start..] {
        c = table[((c ^ b as u32) & 0xFF) as usize] ^ (c >> 8);
    }
    out.extend_from_slice(&(!c).to_be_bytes());
}

/// Encodes an image as PNG bytes.
pub fn encode_png(img: &Rgba) -> Option<Vec<u8>> {
    if img.w == 0 || img.h == 0 || img.px.len() < img.w as usize * img.h as usize * 4 {
        return None;
    }
    let table = crc_table();
    let mut out = vec![0x89, b'P', b'N', b'G', 0x0D, 0x0A, 0x1A, 0x0A];
    let mut ihdr = Vec::new();
    ihdr.extend_from_slice(&img.w.to_be_bytes());
    ihdr.extend_from_slice(&img.h.to_be_bytes());
    ihdr.extend_from_slice(&[8, 6, 0, 0, 0]);
    chunk(&mut out, &table, b"IHDR", &ihdr);
    let stride = img.w as usize * 4;
    let mut raw = Vec::with_capacity((stride + 1) * img.h as usize);
    for y in 0..img.h as usize {
        raw.push(0);
        raw.extend_from_slice(&img.px[y * stride..(y + 1) * stride]);
    }
    let z = miniz_oxide::deflate::compress_to_vec_zlib(&raw, 6);
    chunk(&mut out, &table, b"IDAT", &z);
    chunk(&mut out, &table, b"IEND", &[]);
    Some(out)
}

pub fn write_png(path: impl AsRef<std::path::Path>, img: &Rgba) -> bool {
    let Some(bytes) = encode_png(img) else { return false };
    crate::fsutil::write_file(path, &bytes)
}

/// Decodes a PNG as `encode_png` writes it (RGBA8, every row unfiltered), e.g. the accomplishment snapshots the board
/// reloads; anything else gives None.
pub fn decode_png(b: &[u8]) -> Option<Rgba> {
    if b.len() < 8 || &b[1..4] != b"PNG" {
        return None;
    }
    let (mut w, mut h, mut z) = (0u32, 0u32, Vec::new());
    let mut p = 8;
    while p + 8 <= b.len() {
        let n = u32::from_be_bytes(b[p..p + 4].try_into().ok()?) as usize;
        let body = b.get(p + 8..p + 8 + n)?;
        match &b[p + 4..p + 8] {
            b"IHDR" if n >= 13 => {
                if body[8..13] != [8, 6, 0, 0, 0] {
                    return None;
                }
                w = u32::from_be_bytes(body[0..4].try_into().ok()?);
                h = u32::from_be_bytes(body[4..8].try_into().ok()?);
            }
            b"IDAT" => z.extend_from_slice(body),
            b"IEND" => break,
            _ => {}
        }
        p += n + 12;
    }
    let raw = miniz_oxide::inflate::decompress_to_vec_zlib(&z).ok()?;
    let stride = w as usize * 4;
    if w == 0 || h == 0 || raw.len() < (stride + 1) * h as usize {
        return None;
    }
    let mut img = Rgba::new(w, h);
    for y in 0..h as usize {
        let row = &raw[y * (stride + 1)..(y + 1) * (stride + 1)];
        if row[0] != 0 {
            return None;
        }
        img.px[y * stride..(y + 1) * stride].copy_from_slice(&row[1..]);
    }
    Some(img)
}

#[cfg(test)]
mod tests {
    #[test]
    fn png_has_signature_and_iend() {
        let img = crate::assets::Rgba::new(2, 2);
        let b = super::encode_png(&img).unwrap();
        assert_eq!(&b[1..4], b"PNG");
        assert_eq!(&b[b.len() - 8..b.len() - 4], b"IEND");
        // CRC of an empty IEND chunk is fixed by the PNG spec.
        assert_eq!(&b[b.len() - 4..], &[0xAE, 0x42, 0x60, 0x82]);
    }

    #[test]
    fn png_round_trips() {
        let mut img = crate::assets::Rgba::new(3, 2);
        for (i, v) in img.px.iter_mut().enumerate() {
            *v = (i * 7) as u8;
        }
        let back = super::decode_png(&super::encode_png(&img).unwrap()).unwrap();
        assert_eq!((back.w, back.h), (3, 2));
        assert_eq!(back.px, img.px);
        assert!(super::decode_png(b"not a png").is_none());
    }
}
