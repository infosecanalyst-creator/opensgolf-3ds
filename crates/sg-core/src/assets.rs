//! Loaders for the original game's image and sound formats: PCX, TGA, BMP and WAV. FLC animations are in [`crate::flc`].
//! Each decoder returns `Err(message)` on failure.
//!
//! Content packs (docs/MODDING.md) may replace a PCX, TGA or BMP picture with a PNG and a WAV sound with an OGG Vorbis file
//! of the same name; the virtual file system (crate::vfs) then hands the PNG or OGG bytes to the loader that asked for the
//! original, so every picture decoder here takes a PNG as well (recognised by its signature) and [`decode_sound`] takes WAV
//! or OGG.

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
/// True when the bytes are a PNG (a content pack's replacement for a PCX, TGA or BMP).
pub fn is_png(d: &[u8]) -> bool {
    d.len() >= 8 && d[..8] == [0x89, b'P', b'N', b'G', 0x0D, 0x0A, 0x1A, 0x0A]
}

fn png_or_err(d: &[u8]) -> Result<Rgba, String> {
    crate::png::decode_png(d).ok_or_else(|| "unsupported PNG (8 bits per channel, not interlaced)".to_string())
}

pub fn decode_pcx(d: &[u8]) -> Result<Rgba, String> {
    if is_png(d) {
        return png_or_err(d);
    }
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
///
/// A PNG (a content pack's replacement) is turned into indices by [`index_colours`].
pub fn decode_pcx_indexed(d: &[u8]) -> Option<Indexed> {
    if is_png(d) {
        return Some(index_colours(&crate::png::decode_png(d)?));
    }
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

/// An RGBA picture as a palette image: its first 256 distinct colours, in the order they appear, make the palette; any
/// further colour takes the nearest of those. Alpha is dropped (palette art keys its transparency by colour).
pub fn index_colours(img: &Rgba) -> Indexed {
    let mut colours: Vec<[u8; 3]> = Vec::new();
    let mut seen = std::collections::HashMap::new();
    let mut idx = Vec::with_capacity(img.px.len() / 4);
    for p in img.px.chunks_exact(4) {
        let c = [p[0], p[1], p[2]];
        let i = match seen.get(&c) {
            Some(&i) => i,
            None if colours.len() < 256 => {
                let i = colours.len() as u8;
                colours.push(c);
                seen.insert(c, i);
                i
            }
            None => {
                let d = |a: &[u8; 3]| (0..3).map(|k| (a[k] as i32 - c[k] as i32).pow(2)).sum::<i32>();
                (0..colours.len()).min_by_key(|&k| d(&colours[k])).unwrap_or(0) as u8
            }
        };
        idx.push(i);
    }
    let mut pal = [0u8; 768];
    for (k, c) in colours.iter().enumerate() {
        pal[k * 3..k * 3 + 3].copy_from_slice(c);
    }
    Indexed { w: img.w, h: img.h, idx, pal }
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
    if is_png(d) {
        return png_or_err(d);
    }
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
    if is_png(d) {
        return png_or_err(d);
    }
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

/// A WAV file as 8- or 16-bit PCM, whatever common coding it holds (crate::wav).
pub fn decode_wav(d: &[u8]) -> Result<Wav, String> {
    crate::wav::decode(d)
}

/// A sound: a PCM WAV, or an OGG Vorbis file (a content pack's replacement for a WAV) decoded to 16-bit PCM.
pub fn decode_sound(d: &[u8]) -> Result<Wav, String> {
    if d.len() >= 4 && &d[0..4] == b"OggS" {
        return decode_ogg(d);
    }
    decode_wav(d)
}

/// An OGG Vorbis file decoded to interleaved 16-bit PCM (mono or stereo).
pub fn decode_ogg(d: &[u8]) -> Result<Wav, String> {
    use lewton::inside_ogg::OggStreamReader;
    let mut r = OggStreamReader::new(std::io::Cursor::new(d)).map_err(|e| format!("OGG: {e}"))?;
    let channels = r.ident_hdr.audio_channels as u16;
    if channels == 0 || channels > 2 {
        return Err(format!("OGG: {channels} channels (only mono and stereo are played)"));
    }
    let mut out = Wav { channels, bits: 16, sample_rate: r.ident_hdr.audio_sample_rate, pcm: Vec::new() };
    while let Some(pck) = r.read_dec_packet_itl().map_err(|e| format!("OGG: {e}"))? {
        for v in pck {
            out.pcm.extend_from_slice(&v.to_le_bytes());
        }
    }
    Ok(out)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn pack_replacements_png_for_pictures_ogg_for_sounds() {
        // a 0.1 s 440 Hz tone at 22050 Hz, mono (made with ffmpeg's sine source; tests/data/tone.ogg)
        let w = decode_sound(include_bytes!("../tests/data/tone.ogg")).unwrap();
        assert_eq!((w.channels, w.bits, w.sample_rate), (1, 16, 22050));
        assert!((2000..2600).contains(&(w.pcm.len() / 2)), "{} samples", w.pcm.len() / 2);
        assert!(w.pcm.chunks_exact(2).map(|s| i16::from_le_bytes([s[0], s[1]]).unsigned_abs()).max().unwrap() > 1000);
        assert!(decode_sound(b"OggS not really").is_err());
        // a PNG where a PCX, TGA or BMP was asked for
        let mut img = Rgba::new(3, 1);
        img.px.copy_from_slice(&[255, 0, 0, 255, 0, 255, 0, 255, 255, 0, 0, 255]);
        let png = crate::png::encode_png(&img).unwrap();
        for d in [decode_pcx(&png), decode_tga(&png), decode_bmp(&png)] {
            assert_eq!(d.unwrap().px, img.px);
        }
        let ix = decode_pcx_indexed(&png).unwrap();
        assert_eq!(ix.idx, [0, 1, 0]);
        assert_eq!(&ix.pal[..6], &[255, 0, 0, 0, 255, 0]);
    }

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
