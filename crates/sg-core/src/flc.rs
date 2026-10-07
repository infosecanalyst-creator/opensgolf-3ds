//! Autodesk FLI/FLC animation decoder (8-bit). Frames are fully composited.
use crate::assets::{u16le, u32le, Indexed};

#[derive(Clone, Debug, Default)]
pub struct Flc {
    pub w: u32,
    pub h: u32,
    /// Per-frame delay in milliseconds.
    pub frame_ms: u32,
    /// Fully composited frame each.
    pub frames: Vec<Indexed>,
    // Firaxis extension of the 128 byte header (all sprite FLCs have it, see docs/SPRITES.md): the frames are `views` runs of
    // `frames_per_view` frames, view-major. Each run is stored with one extra trailing "ring" frame that decode_flc drops. The
    // crop box origin is where the frame sits in a 480x480 render canvas whose centre (240,240) is the object's ground point.
    pub views: u32,
    pub frames_per_view: u32,
    pub crop_x: u32,
    pub crop_y: u32,
    pub canvas_w: u32,
    pub canvas_h: u32,
    /// Bit set per stored view (0x0F = 4 views, 0xFF = 8 views).
    pub view_mask: u32,
    pub has_ext: bool,
}

struct Cur<'a> {
    d: &'a [u8],
    p: usize,
    end: usize,
    ok: bool,
}

impl<'a> Cur<'a> {
    fn new(d: &'a [u8], p: usize, end: usize) -> Self {
        Cur { d, p, end, ok: true }
    }
    fn left(&self) -> usize {
        self.end - self.p
    }
    fn u8(&mut self) -> u8 {
        if self.p >= self.end {
            self.ok = false;
            return 0;
        }
        self.p += 1;
        self.d[self.p - 1]
    }
    fn w(&mut self) -> u16 {
        if self.left() < 2 {
            self.ok = false;
            self.p = self.end;
            return 0;
        }
        self.p += 2;
        u16le(self.d, self.p - 2)
    }
}

fn read_palette(c: &mut Cur, pal: &mut [u8; 768], six_bit: bool) {
    let mut n = c.w() as u32;
    let mut idx = 0u32;
    while n > 0 && c.ok {
        n -= 1;
        idx += c.u8() as u32;
        let mut cnt = c.u8() as u32;
        if cnt == 0 {
            cnt = 256;
        }
        let mut i = 0;
        while i < cnt && c.ok {
            let rgb = [c.u8(), c.u8(), c.u8()];
            if idx < 256 {
                for k in 0..3 {
                    pal[idx as usize * 3 + k] = if six_bit { (rgb[k] << 2) | (rgb[k] >> 4) } else { rgb[k] };
                }
            }
            i += 1;
            idx += 1;
        }
    }
}

fn byte_run(c: &mut Cur, f: &mut Indexed) {
    let (w, h) = (f.w as usize, f.h as usize);
    for y in 0..h {
        if !c.ok {
            break;
        }
        c.u8(); // packet count, unreliable
        let mut x = 0usize;
        let row = y * w;
        while x < w && c.ok {
            let s = c.u8() as i8;
            if s >= 0 {
                let v = c.u8();
                for _ in 0..s {
                    if x >= w {
                        break;
                    }
                    f.idx[row + x] = v;
                    x += 1;
                }
            } else {
                for _ in 0..(-(s as i32)) {
                    if !c.ok {
                        break;
                    }
                    let v = c.u8();
                    if x < w {
                        f.idx[row + x] = v;
                        x += 1;
                    }
                }
            }
        }
    }
}

/// DELTA_FLI (type 12): byte oriented.
fn delta_fli(c: &mut Cur, f: &mut Indexed) {
    let (w, h) = (f.w as usize, f.h as usize);
    let mut y = c.w() as usize;
    let mut lines = c.w() as u32;
    while lines > 0 && c.ok && y < h {
        let mut packets = c.u8() as u32;
        let mut x = 0usize;
        let row = y * w;
        while packets > 0 && c.ok {
            packets -= 1;
            x += c.u8() as usize;
            let s = c.u8() as i8;
            if s >= 0 {
                for _ in 0..s {
                    if !c.ok {
                        break;
                    }
                    let v = c.u8();
                    if x < w {
                        f.idx[row + x] = v;
                    }
                    x += 1;
                }
            } else {
                let v = c.u8();
                for _ in 0..(-(s as i32)) {
                    if x < w {
                        f.idx[row + x] = v;
                    }
                    x += 1;
                }
            }
        }
        lines -= 1;
        y += 1;
    }
}

/// DELTA_FLC (type 7): word oriented.
fn delta_flc(c: &mut Cur, f: &mut Indexed) {
    let (w, h) = (f.w as usize, f.h as usize);
    let mut lines = c.w() as u32;
    let mut y: usize = 0;
    let put = |idx: &mut Vec<u8>, y: usize, x: usize, v: u8| {
        if y < h && x < w {
            idx[y * w + x] = v;
        }
    };
    while lines > 0 && c.ok {
        let op = c.w();
        if !c.ok {
            break;
        }
        match op >> 14 {
            0 => {
                // packet count for this line
                let mut packets = op as u32;
                let mut x = 0usize;
                while packets > 0 && c.ok {
                    packets -= 1;
                    x += c.u8() as usize;
                    let s = c.u8() as i8;
                    if s >= 0 {
                        for _ in 0..s {
                            if !c.ok {
                                break;
                            }
                            let (a, b) = (c.u8(), c.u8());
                            put(&mut f.idx, y, x, a);
                            put(&mut f.idx, y, x + 1, b);
                            x += 2;
                        }
                    } else {
                        let (a, b) = (c.u8(), c.u8());
                        for _ in 0..(-(s as i32)) {
                            put(&mut f.idx, y, x, a);
                            put(&mut f.idx, y, x + 1, b);
                            x += 2;
                        }
                    }
                }
                y += 1;
                lines -= 1;
            }
            2 => {
                // last pixel of the current line; the line continues with the next word
                if y < h && w > 0 {
                    f.idx[y * w + w - 1] = (op & 0xFF) as u8;
                }
            }
            3 => {
                // skip lines
                y += (op as i16).wrapping_neg() as u16 as usize;
            }
            _ => return, // opcode 01 is undefined
        }
    }
}

pub fn decode_flc(d: &[u8]) -> Result<Flc, String> {
    if d.len() < 128 {
        return Err("short FLC".into());
    }
    let magic = u16le(d, 4);
    if magic != 0xAF12 && magic != 0xAF11 {
        return Err("bad FLC magic".into());
    }
    let nframes = u16le(d, 6) as u32;
    let mut out = Flc { w: u16le(d, 8) as u32, h: u16le(d, 10) as u32, views: 1, ..Default::default() };
    if u16le(d, 12) != 8 {
        return Err("not 8-bit FLC".into());
    }
    let speed = u32le(d, 16);
    {
        // Firaxis header extension, accepted only when it is self-consistent.
        let v = u16le(d, 96) as u32;
        let fpv = u16le(d, 98) as u32;
        if (1..=8).contains(&v) && fpv >= 1 && v * fpv == nframes && u16le(d, 104) == 480 && u16le(d, 106) == 480 {
            out.views = v;
            out.frames_per_view = fpv;
            out.has_ext = true;
            out.crop_x = u16le(d, 100) as u32;
            out.crop_y = u16le(d, 102) as u32;
            out.canvas_w = 480;
            out.canvas_h = 480;
            out.view_mask = u16le(d, 112) as u32;
        }
    }
    out.frame_ms = if magic == 0xAF11 { speed.wrapping_mul(1000) / 70 } else { speed };
    if out.w == 0 || out.h == 0 || out.w > 4096 || out.h > 4096 {
        return Err("bad dimensions".into());
    }

    let mut cur = Indexed { w: out.w, h: out.h, idx: vec![0; out.w as usize * out.h as usize], pal: [0; 768] };
    let mut pos = 128usize;
    let mut done = 0u32;
    // With the extension every view is stored as frames_per_view + 1 frames (the last is a ring frame).
    let stored = if out.has_ext { out.views * (out.frames_per_view + 1) } else { nframes };
    while done < stored && pos + 16 <= d.len() {
        let fsize = u32le(d, pos) as usize;
        let ftype = u16le(d, pos + 4);
        let nchunks = u16le(d, pos + 6) as u32;
        if fsize < 16 || pos + fsize > d.len() {
            return Err("bad frame size".into());
        }
        if ftype != 0xF1FA {
            pos += fsize; // e.g. prefix chunk
            continue;
        }
        let mut cp = pos + 16;
        let fend = pos + fsize;
        for i in 0..nchunks {
            if cp + 6 > fend {
                break;
            }
            let mut csz = u32le(d, cp) as usize;
            let ct = u16le(d, cp + 4);
            if csz < 6 || cp + csz > fend {
                // Firaxis' FLC writer sometimes left the chunk size field uninitialised (0xCDCDCDCD). Recover: palettes are
                // self-delimiting, a final chunk runs to the end of the frame.
                if ct == 4 || ct == 11 {
                    let mut scratch = [0u8; 768];
                    let mut probe = Cur::new(d, cp + 6, fend);
                    read_palette(&mut probe, &mut scratch, ct == 11);
                    if !probe.ok {
                        return Err("bad palette chunk size".into());
                    }
                    csz = 6 + (probe.p - (cp + 6));
                } else if i + 1 == nchunks {
                    csz = fend - cp;
                } else {
                    return Err("bad chunk size".into());
                }
            }
            let mut c = Cur::new(d, cp + 6, cp + csz);
            match ct {
                4 => read_palette(&mut c, &mut cur.pal, false),
                11 => read_palette(&mut c, &mut cur.pal, true),
                7 => delta_flc(&mut c, &mut cur),
                12 => delta_fli(&mut c, &mut cur),
                15 => byte_run(&mut c, &mut cur),
                13 => cur.idx.iter_mut().for_each(|v| *v = 0),
                16 if c.left() >= cur.idx.len() => {
                    let n = cur.idx.len();
                    cur.idx.copy_from_slice(&d[c.p..c.p + n]);
                }
                _ => {} // 18 = postage stamp, others ignored
            }
            if !c.ok {
                return Err(format!("truncated chunk in frame {done}"));
            }
            cp += csz;
        }
        if !(out.has_ext && done % (out.frames_per_view + 1) == out.frames_per_view) {
            out.frames.push(cur.clone());
        }
        done += 1;
        pos = fend;
    }
    if out.frames.is_empty() {
        return Err("no frames".into());
    }
    Ok(out)
}

#[cfg(test)]
mod tests {
    use super::*;

    fn header(frames: u16, w: u16, h: u16) -> Vec<u8> {
        let mut d = vec![0u8; 128];
        d[4..6].copy_from_slice(&0xAF12u16.to_le_bytes());
        d[6..8].copy_from_slice(&frames.to_le_bytes());
        d[8..10].copy_from_slice(&w.to_le_bytes());
        d[10..12].copy_from_slice(&h.to_le_bytes());
        d[12..14].copy_from_slice(&8u16.to_le_bytes());
        d[16..20].copy_from_slice(&83u32.to_le_bytes());
        d
    }
    fn frame(chunks: &[(u16, Vec<u8>)]) -> Vec<u8> {
        let mut body = Vec::new();
        for (t, c) in chunks {
            body.extend_from_slice(&((c.len() + 6) as u32).to_le_bytes());
            body.extend_from_slice(&t.to_le_bytes());
            body.extend_from_slice(c);
        }
        let mut f = Vec::new();
        f.extend_from_slice(&((body.len() + 16) as u32).to_le_bytes());
        f.extend_from_slice(&0xF1FAu16.to_le_bytes());
        f.extend_from_slice(&(chunks.len() as u16).to_le_bytes());
        f.extend_from_slice(&[0; 8]);
        f.extend_from_slice(&body);
        f
    }

    #[test]
    fn brun_then_delta() {
        let mut d = header(2, 4, 2);
        // palette: 1 packet, skip 0, count 2: colours 0 and 1
        let pal = vec![1, 0, 0, 2, 10, 20, 30, 40, 50, 60];
        // BRUN: per line a packet count byte, then runs.
        let brun = vec![0, 4, 1, 0, 2, 1, 0xFE, 0, 1];
        d.extend(frame(&[(4, pal), (15, brun)]));
        // DELTA_FLC: 1 line, packet count 1, skip 1, literal 1 word (0,0)
        let delta = vec![1, 0, 1, 0, 1, 1, 0, 0];
        d.extend(frame(&[(7, delta)]));
        let f = decode_flc(&d).unwrap();
        assert_eq!(f.frames.len(), 2);
        assert_eq!(f.frames[0].idx, vec![1, 1, 1, 1, 1, 1, 0, 1]);
        assert_eq!(f.frames[1].idx, vec![1, 0, 0, 1, 1, 1, 0, 1]);
        assert_eq!(&f.frames[0].pal[3..6], &[40, 50, 60]);
        assert_eq!(f.frame_ms, 83);
    }

    #[test]
    fn uninitialised_palette_size_is_recovered() {
        let mut d = header(1, 1, 1);
        let mut chunk_body = vec![1, 0, 0, 1, 7, 8, 9];
        let mut f = Vec::new();
        let total = 16 + 6 + chunk_body.len();
        f.extend_from_slice(&(total as u32).to_le_bytes());
        f.extend_from_slice(&0xF1FAu16.to_le_bytes());
        f.extend_from_slice(&1u16.to_le_bytes());
        f.extend_from_slice(&[0; 8]);
        f.extend_from_slice(&0xCDCDCDCDu32.to_le_bytes());
        f.extend_from_slice(&4u16.to_le_bytes());
        f.append(&mut chunk_body);
        d.extend(f);
        let flc = decode_flc(&d).unwrap();
        assert_eq!(&flc.frames[0].pal[0..3], &[7, 8, 9]);
    }
}
