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

/// Decodes a PNG: 8 bits per channel, not interlaced, greyscale, RGB, palette (with tRNS), grey + alpha or RGBA, any row
/// filter. Covers the accomplishment snapshots `encode_png` writes, the HD packs' pictures (docs/HD.md) and the PNG art of
/// content packs (docs/MODDING.md). Anything else gives None.
pub fn decode_png(d: &[u8]) -> Option<Rgba> {
    if d.len() < 8 || d[..8] != [0x89, b'P', b'N', b'G', 0x0D, 0x0A, 0x1A, 0x0A] {
        return None;
    }
    let be = |o: usize| u32::from_be_bytes([d[o], d[o + 1], d[o + 2], d[o + 3]]);
    let (mut w, mut h, mut ct) = (0usize, 0usize, 0u8);
    let mut pal: Vec<u8> = Vec::new();
    let mut trns: Vec<u8> = Vec::new();
    let mut idat = Vec::new();
    let mut p = 8;
    while p + 8 <= d.len() {
        let len = be(p) as usize;
        let ty = &d[p + 4..p + 8];
        let body = d.get(p + 8..p + 8 + len)?;
        match ty {
            b"IHDR" => {
                if len < 13 || body[8] != 8 || body[12] != 0 {
                    return None; // only 8 bits per channel, not interlaced
                }
                w = u32::from_be_bytes(body[0..4].try_into().ok()?) as usize;
                h = u32::from_be_bytes(body[4..8].try_into().ok()?) as usize;
                ct = body[9];
            }
            b"PLTE" => pal = body.to_vec(),
            b"tRNS" => trns = body.to_vec(),
            b"IDAT" => idat.extend_from_slice(body),
            b"IEND" => break,
            _ => {}
        }
        p += 12 + len;
    }
    let ch = match ct {
        0 | 3 => 1,
        2 => 3,
        4 => 2,
        6 => 4,
        _ => return None,
    };
    if w == 0 || h == 0 || w > 16384 || h > 16384 {
        return None;
    }
    let raw = miniz_oxide::inflate::decompress_to_vec_zlib(&idat).ok()?;
    let stride = w * ch;
    if raw.len() < (stride + 1) * h {
        return None;
    }
    let mut cur = vec![0u8; stride];
    let mut prev = vec![0u8; stride];
    let mut out = Rgba::new(w as u32, h as u32);
    for y in 0..h {
        let line = &raw[y * (stride + 1)..(y + 1) * (stride + 1)];
        let f = line[0];
        for i in 0..stride {
            let x = line[i + 1];
            let a = if i >= ch { cur[i - ch] } else { 0 };
            let b = prev[i];
            let c = if i >= ch { prev[i - ch] } else { 0 };
            cur[i] = match f {
                0 => x,
                1 => x.wrapping_add(a),
                2 => x.wrapping_add(b),
                3 => x.wrapping_add(((a as u16 + b as u16) / 2) as u8),
                4 => {
                    let pp = a as i16 + b as i16 - c as i16;
                    let (pa, pb, pc) = ((pp - a as i16).abs(), (pp - b as i16).abs(), (pp - c as i16).abs());
                    x.wrapping_add(if pa <= pb && pa <= pc {
                        a
                    } else if pb <= pc {
                        b
                    } else {
                        c
                    })
                }
                _ => return None,
            };
        }
        let row = &mut out.px[y * w * 4..(y + 1) * w * 4];
        for (x, o) in row.chunks_exact_mut(4).enumerate() {
            let s = &cur[x * ch..x * ch + ch];
            let px = match ct {
                0 => [s[0], s[0], s[0], 255],
                2 => [s[0], s[1], s[2], 255],
                3 => {
                    let i = s[0] as usize;
                    let c = pal.get(i * 3..i * 3 + 3)?;
                    [c[0], c[1], c[2], trns.get(i).copied().unwrap_or(255)]
                }
                4 => [s[0], s[0], s[0], s[1]],
                _ => [s[0], s[1], s[2], s[3]],
            };
            o.copy_from_slice(&px);
        }
        std::mem::swap(&mut cur, &mut prev);
    }
    Some(out)
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

    #[test]
    fn png_filters() {
        // a 2x2 RGB image, row 0 with the Sub filter, row 1 with Paeth
        let raw = [1u8, 10, 20, 30, 5, 5, 5, 4, 1, 2, 3, 4, 5, 6];
        let z = miniz_oxide::deflate::compress_to_vec_zlib(&raw, 6);
        let mut png = vec![0x89, b'P', b'N', b'G', 0x0D, 0x0A, 0x1A, 0x0A];
        let mut chunk = |ty: &[u8], body: &[u8]| {
            png.extend_from_slice(&(body.len() as u32).to_be_bytes());
            png.extend_from_slice(ty);
            png.extend_from_slice(body);
            png.extend_from_slice(&[0; 4]); // the decoder does not check CRCs
        };
        chunk(b"IHDR", &[0, 0, 0, 2, 0, 0, 0, 2, 8, 2, 0, 0, 0]);
        chunk(b"IDAT", &z);
        chunk(b"IEND", &[]);
        let img = super::decode_png(&png).unwrap();
        assert_eq!(&img.px[0..8], &[10, 20, 30, 255, 15, 25, 35, 255]);
        // Paeth: first pixel predicts from above (b), second from the left/above/upper-left
        assert_eq!(&img.px[8..11], &[11, 22, 33]);
    }
}
