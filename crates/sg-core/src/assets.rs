//! Loaders for the original game's image and sound formats: PCX, TGA, BMP and WAV. FLC animations are in [`crate::flc`].
//! Each decoder returns `Err(message)` on failure.

/// RGBA8, row-major, top-left origin.
#[derive(Clone, Debug, Default)]
pub struct Rgba {
    pub w: u32,
    pub h: u32,
    pub px: Vec<u8>,
}

impl Rgba {
    pub fn new(w: u32, h: u32) -> Self {
        Rgba { w, h, px: vec![0; w as usize * h as usize * 4] }
    }
}

/// 8-bit palettised image (used by FLC frames and 8-bit PCX/BMP).
#[derive(Clone, Debug)]
pub struct Indexed {
    pub w: u32,
    pub h: u32,
    pub idx: Vec<u8>,
    pub pal: [u8; 768],
}

impl Default for Indexed {
    fn default() -> Self {
        Indexed { w: 0, h: 0, idx: Vec::new(), pal: [0; 768] }
    }
}

pub(crate) fn u16le(d: &[u8], o: usize) -> u16 {
    u16::from_le_bytes([d[o], d[o + 1]])
}
pub(crate) fn u32le(d: &[u8], o: usize) -> u32 {
    u32::from_le_bytes([d[o], d[o + 1], d[o + 2], d[o + 3]])
}

/// Palette lookup; pixels equal to `transparent` (when given) get alpha 0.
pub fn to_rgba(inp: &Indexed, transparent: Option<u8>) -> Rgba {
    let n = inp.w as usize * inp.h as usize;
    let mut o = Rgba { w: inp.w, h: inp.h, px: vec![0; n * 4] };
    for i in 0..n {
        let c = inp.idx[i] as usize;
        o.px[i * 4] = inp.pal[c * 3];
        o.px[i * 4 + 1] = inp.pal[c * 3 + 1];
        o.px[i * 4 + 2] = inp.pal[c * 3 + 2];
        o.px[i * 4 + 3] = if Some(c as u8) == transparent { 0 } else { 255 };
    }
    o
}

// ---------------------------------------------------------------- PCX (v5, RLE)
pub fn decode_pcx(d: &[u8]) -> Result<Rgba, String> {
    if d.len() < 128 || d[0] != 10 || d[2] != 1 {
        return Err("not an RLE PCX".into());
    }
    let bpp = d[3];
    let planes = d[65] as usize;
    let w = (u16le(d, 8) as i32 - u16le(d, 4) as i32 + 1) as i64;
    let h = (u16le(d, 10) as i32 - u16le(d, 6) as i32 + 1) as i64;
    let bpl = u16le(d, 66) as usize;
    if w <= 0 || h <= 0 || w > 16384 || h > 16384 {
        return Err("bad dimensions".into());
    }
    let (w, h) = (w as usize, h as usize);
    if bpp != 8 || (planes != 1 && planes != 3) || bpl < w {
        return Err("unsupported PCX layout".into());
    }
    let line_bytes = planes * bpl;
    let mut pos = 128;
    let mut scan = vec![0u8; line_bytes];
    let mut idx = if planes == 1 { vec![0u8; w * h] } else { Vec::new() };
    let mut rgb = if planes == 3 { vec![0u8; w * h * 4] } else { Vec::new() };
    for y in 0..h {
        let mut n = 0;
        while n < line_bytes {
            if pos >= d.len() {
                return Err("truncated PCX data".into());
            }
            let mut b = d[pos];
            pos += 1;
            let mut run = 1usize;
            if b & 0xC0 == 0xC0 {
                run = (b & 0x3F) as usize;
                if pos >= d.len() {
                    return Err("truncated PCX data".into());
                }
                b = d[pos];
                pos += 1;
            }
            while run > 0 && n < line_bytes {
                scan[n] = b;
                n += 1;
                run -= 1;
            }
        }
        if planes == 1 {
            idx[y * w..y * w + w].copy_from_slice(&scan[..w]);
        } else {
            for x in 0..w {
                let o = (y * w + x) * 4;
                rgb[o] = scan[x];
                rgb[o + 1] = scan[bpl + x];
                rgb[o + 2] = scan[2 * bpl + x];
                rgb[o + 3] = 255;
            }
        }
    }
    if planes == 3 {
        return Ok(Rgba { w: w as u32, h: h as u32, px: rgb });
    }
    if d.len() < pos + 769 || d[d.len() - 769] != 0x0C {
        return Err("missing PCX palette".into());
    }
    let mut ix = Indexed { w: w as u32, h: h as u32, idx, pal: [0; 768] };
    ix.pal.copy_from_slice(&d[d.len() - 768..]);
    Ok(to_rgba(&ix, None))
}

/// An 8-bit single-plane PCX as indices and palette.
pub fn decode_pcx_indexed(d: &[u8]) -> Option<Indexed> {
    if d.len() < 128 + 769 || d[0] != 10 || d[2] != 1 || d[3] != 8 || d[65] != 1 {
        return None;
    }
    let w = (u16le(d, 8) as i32 - u16le(d, 4) as i32 + 1) as usize;
    let h = (u16le(d, 10) as i32 - u16le(d, 6) as i32 + 1) as usize;
    let bpl = u16le(d, 66) as usize;
    if w == 0 || h == 0 || w > 16384 || h > 16384 || bpl < w {
        return None;
    }
    let mut idx = vec![0u8; w * h];
    let mut scan = vec![0u8; bpl];
    let mut pos = 128;
    for y in 0..h {
        let mut n = 0;
        while n < bpl {
            let mut b = *d.get(pos)?;
            pos += 1;
            let mut run = 1;
            if b & 0xC0 == 0xC0 {
                run = (b & 0x3F) as usize;
                b = *d.get(pos)?;
                pos += 1;
            }
            while run > 0 && n < bpl {
                scan[n] = b;
                n += 1;
                run -= 1;
            }
        }
        idx[y * w..y * w + w].copy_from_slice(&scan[..w]);
    }
    let pal = read_pcx_palette(d)?;
    Some(Indexed { w: w as u32, h: h as u32, idx, pal })
}

/// Writes an 8-bit, run-length coded PCX (version 5) with its 256-colour palette.
pub fn encode_pcx(img: &Indexed) -> Vec<u8> {
    let (w, h) = (img.w as usize, img.h as usize);
    let bpl = w + (w & 1);
    let mut out = vec![0u8; 128];
    out[0] = 10;
    out[1] = 5;
    out[2] = 1;
    out[3] = 8;
    out[8..10].copy_from_slice(&((w - 1) as u16).to_le_bytes());
    out[10..12].copy_from_slice(&((h - 1) as u16).to_le_bytes());
    out[12..14].copy_from_slice(&72u16.to_le_bytes());
    out[14..16].copy_from_slice(&72u16.to_le_bytes());
    out[65] = 1;
    out[66..68].copy_from_slice(&(bpl as u16).to_le_bytes());
    out[68] = 1;
    for y in 0..h {
        let row: Vec<u8> = (0..bpl).map(|x| if x < w { img.idx[y * w + x] } else { 0 }).collect();
        let mut x = 0;
        while x < bpl {
            let b = row[x];
            let mut run = 1;
            while x + run < bpl && run < 63 && row[x + run] == b {
                run += 1;
            }
            if run > 1 || b & 0xC0 == 0xC0 {
                out.push(0xC0 | run as u8);
            }
            out.push(b);
            x += run;
        }
    }
    out.push(0x0C);
    out.extend_from_slice(&img.pal);
    out
}

/// Palette of an 8-bit PCX (the 768 bytes after the 0x0C marker at the end of the file).
pub fn read_pcx_palette(d: &[u8]) -> Option<[u8; 768]> {
    if d.len() < 769 || d[d.len() - 769] != 0x0C {
        return None;
    }
    let mut p = [0u8; 768];
    p.copy_from_slice(&d[d.len() - 768..]);
    Some(p)
}

// ---------------------------------------------------------------- TGA
pub fn decode_tga(d: &[u8]) -> Result<Rgba, String> {
    if d.len() < 18 {
        return Err("short TGA".into());
    }
    let id_len = d[0] as usize;
    let ty = d[2];
    let bpp = d[16];
    let desc = d[17];
    let w = u16le(d, 12) as usize;
    let h = u16le(d, 14) as usize;
    if d[1] != 0 || (ty != 2 && ty != 10) || (bpp != 24 && bpp != 32) {
        return Err("unsupported TGA".into());
    }
    if w == 0 || h == 0 || w > 16384 || h > 16384 {
        return Err("bad dimensions".into());
    }
    let bytes_per = bpp as usize / 8;
    let mut pos = 18 + id_len;
    let total = w * h;
    let mut raw = vec![0u8; total * bytes_per];
    if ty == 2 {
        if d.len() < pos + raw.len() {
            return Err("truncated TGA".into());
        }
        raw.copy_from_slice(&d[pos..pos + total * bytes_per]);
    } else {
        let mut n = 0;
        while n < total {
            if pos >= d.len() {
                return Err("truncated TGA".into());
            }
            let c = d[pos];
            pos += 1;
            let cnt = ((c & 0x7F) as usize + 1).min(total - n);
            if c & 0x80 != 0 {
                if pos + bytes_per > d.len() {
                    return Err("truncated TGA".into());
                }
                for i in 0..cnt {
                    raw[(n + i) * bytes_per..(n + i + 1) * bytes_per].copy_from_slice(&d[pos..pos + bytes_per]);
                }
                pos += bytes_per;
            } else {
                if pos + cnt * bytes_per > d.len() {
                    return Err("truncated TGA".into());
                }
                raw[n * bytes_per..(n + cnt) * bytes_per].copy_from_slice(&d[pos..pos + cnt * bytes_per]);
                pos += cnt * bytes_per;
            }
            n += cnt;
        }
    }
    let mut out = Rgba::new(w as u32, h as u32);
    let top_down = desc & 0x20 != 0;
    for y in 0..h {
        let sy = if top_down { y } else { h - 1 - y };
        for x in 0..w {
            let s = (sy * w + x) * bytes_per;
            let o = (y * w + x) * 4;
            out.px[o] = raw[s + 2];
            out.px[o + 1] = raw[s + 1];
            out.px[o + 2] = raw[s];
            out.px[o + 3] = if bytes_per == 4 { raw[s + 3] } else { 255 };
        }
    }
    Ok(out)
}

// ---------------------------------------------------------------- BMP (uncompressed 8/24 bit)
pub fn decode_bmp(d: &[u8]) -> Result<Rgba, String> {
    if d.len() < 54 || d[0] != b'B' || d[1] != b'M' {
        return Err("not a BMP".into());
    }
    let off = u32le(d, 10) as usize;
    let dib = u32le(d, 14) as usize;
    let w = u32le(d, 18) as i32;
    let mut h = u32le(d, 22) as i32;
    let bpp = u16le(d, 28) as usize;
    let comp = u32le(d, 30);
    let bottom_up = h > 0;
    if h < 0 {
        h = -h;
    }
    if comp != 0 || (bpp != 24 && bpp != 8) || w <= 0 || h <= 0 || w > 16384 || h > 16384 {
        return Err("unsupported BMP".into());
    }
    let (w, h) = (w as usize, h as usize);
    let stride = (w * bpp / 8 + 3) & !3usize;
    if d.len() < off + stride * h {
        return Err("truncated BMP".into());
    }
    let mut out = Rgba::new(w as u32, h as u32);
    for y in 0..h {
        let row = off + stride * if bottom_up { h - 1 - y } else { y };
        for x in 0..w {
            let o = (y * w + x) * 4;
            if bpp == 24 {
                out.px[o] = d[row + x * 3 + 2];
                out.px[o + 1] = d[row + x * 3 + 1];
                out.px[o + 2] = d[row + x * 3];
            } else {
                let p = 14 + dib + d[row + x] as usize * 4;
                if p + 3 > d.len() {
                    return Err("truncated BMP palette".into());
                }
                out.px[o] = d[p + 2];
                out.px[o + 1] = d[p + 1];
                out.px[o + 2] = d[p];
            }
            out.px[o + 3] = 255;
        }
    }
    Ok(out)
}

// ---------------------------------------------------------------- WAV (PCM)
#[derive(Clone, Debug, Default)]
pub struct Wav {
    pub channels: u16,
    pub bits: u16,
    pub sample_rate: u32,
    pub pcm: Vec<u8>,
}

pub fn decode_wav(d: &[u8]) -> Result<Wav, String> {
    if d.len() < 12 || &d[0..4] != b"RIFF" || &d[8..12] != b"WAVE" {
        return Err("not a RIFF WAVE".into());
    }
    let mut out = Wav::default();
    let mut pos = 12;
    let mut have_fmt = false;
    while pos + 8 <= d.len() {
        let sz = u32le(d, pos + 4) as usize;
        let body = pos + 8;
        let avail = d.len() - body;
        if &d[pos..pos + 4] == b"fmt " && sz >= 16 && avail >= 16 {
            if u16le(d, body) != 1 {
                return Err("non-PCM WAV".into());
            }
            out.channels = u16le(d, body + 2);
            out.sample_rate = u32le(d, body + 4);
            out.bits = u16le(d, body + 14);
            have_fmt = true;
        } else if &d[pos..pos + 4] == b"data" {
            if !have_fmt {
                return Err("data before fmt".into());
            }
            let n = sz.min(avail);
            out.pcm = d[body..body + n].to_vec();
            return Ok(out);
        }
        pos += 8 + sz + (sz & 1);
    }
    Err("no data chunk".into())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn pcx_rle_8bit_roundtrip() {
        // 3x2 image, one plane, bytes per line 4 (padded), palette entry i = (i, 2i, 3i).
        let mut d = vec![0u8; 128];
        d[0] = 10;
        d[2] = 1;
        d[3] = 8;
        d[8] = 2; // xmax
        d[10] = 1; // ymax
        d[65] = 1;
        d[66] = 4;
        d.extend_from_slice(&[0xC3, 7, 9]); // run of 3 x 7, then 9 (padding)
        d.extend_from_slice(&[1, 2, 3, 0]);
        d.push(0x0C);
        for i in 0..256u32 {
            d.extend_from_slice(&[i as u8, (i * 2) as u8, (i * 3) as u8]);
        }
        let img = decode_pcx(&d).unwrap();
        assert_eq!((img.w, img.h), (3, 2));
        assert_eq!(&img.px[0..4], &[7, 14, 21, 255]);
        assert_eq!(&img.px[12..16], &[1, 2, 3, 255]);
        assert_eq!(&img.px[20..24], &[3, 6, 9, 255]);
    }

    #[test]
    fn bmp_24bit_bottom_up() {
        let mut d = vec![0u8; 54];
        d[0] = b'B';
        d[1] = b'M';
        d[10] = 54;
        d[14] = 40;
        d[18] = 1;
        d[22] = 2;
        d[28] = 24;
        d.extend_from_slice(&[1, 2, 3, 0]); // bottom row (BGR + pad)
        d.extend_from_slice(&[4, 5, 6, 0]); // top row
        let img = decode_bmp(&d).unwrap();
        assert_eq!(&img.px[0..4], &[6, 5, 4, 255]);
        assert_eq!(&img.px[4..8], &[3, 2, 1, 255]);
    }

    #[test]
    fn tga_rle() {
        let mut d = vec![0u8; 18];
        d[2] = 10;
        d[12] = 3;
        d[14] = 1;
        d[16] = 24;
        d[17] = 0x20;
        d.extend_from_slice(&[0x82, 10, 20, 30]); // 3 pixels of BGR(10,20,30)
        let img = decode_tga(&d).unwrap();
        assert_eq!(&img.px[8..12], &[30, 20, 10, 255]);
    }

    #[test]
    fn wav_pcm() {
        let mut d = Vec::new();
        d.extend_from_slice(b"RIFF\0\0\0\0WAVEfmt ");
        d.extend_from_slice(&16u32.to_le_bytes());
        d.extend_from_slice(&[1, 0, 2, 0]);
        d.extend_from_slice(&22050u32.to_le_bytes());
        d.extend_from_slice(&88200u32.to_le_bytes());
        d.extend_from_slice(&[4, 0, 16, 0]);
        d.extend_from_slice(b"data");
        d.extend_from_slice(&4u32.to_le_bytes());
        d.extend_from_slice(&[1, 2, 3, 4]);
        let w = decode_wav(&d).unwrap();
        assert_eq!((w.channels, w.bits, w.sample_rate, w.pcm.len()), (2, 16, 22050, 4));
    }

    #[test]
    fn pcx_encode_round_trip() {
        let mut pal = [0u8; 768];
        pal[765..].copy_from_slice(&[255, 0, 255]);
        pal[3..6].copy_from_slice(&[10, 20, 30]);
        let idx: Vec<u8> = (0..5 * 3).map(|i| if i % 4 == 0 { 255 } else { (i % 2) as u8 }).collect();
        let img = Indexed { w: 5, h: 3, idx: idx.clone(), pal };
        let d = encode_pcx(&img);
        let back = decode_pcx_indexed(&d).unwrap();
        assert_eq!((back.w, back.h), (5, 3));
        assert_eq!(back.idx, idx);
        assert_eq!(back.pal[3..6], [10, 20, 30]);
        assert_eq!(decode_pcx(&d).unwrap().px[4..8], [10, 20, 30, 255]);
    }
}
