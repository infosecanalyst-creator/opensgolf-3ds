//! Writers for the file formats the game reads: PCX (with a colour quantiser), BMP, TGA, FLC (with the Firaxis header
//! extension, docs/SPRITES.md) and WAV. Only what the free set needs.
use crate::assets::{encode_pcx, Indexed, Rgba};

/// A 24-bit bottom-up BMP.
pub fn bmp24(img: &Rgba) -> Vec<u8> {
    let (w, h) = (img.w as usize, img.h as usize);
    let stride = (w * 3 + 3) & !3;
    let size = 54 + stride * h;
    let mut d = vec![0u8; 54];
    d[0] = b'B';
    d[1] = b'M';
    d[2..6].copy_from_slice(&(size as u32).to_le_bytes());
    d[10] = 54;
    d[14] = 40;
    d[18..22].copy_from_slice(&(w as u32).to_le_bytes());
    d[22..26].copy_from_slice(&(h as u32).to_le_bytes());
    d[26] = 1;
    d[28] = 24;
    d[34..38].copy_from_slice(&((stride * h) as u32).to_le_bytes());
    for y in (0..h).rev() {
        let mut row = vec![0u8; stride];
        for x in 0..w {
            let p = &img.px[(y * w + x) * 4..];
            row[x * 3] = p[2];
            row[x * 3 + 1] = p[1];
            row[x * 3 + 2] = p[0];
        }
        d.extend(row);
    }
    d
}

/// A 32-bit top-down TGA with alpha.
pub fn tga32(img: &Rgba) -> Vec<u8> {
    let mut d = vec![0u8; 18];
    d[2] = 2;
    d[12..14].copy_from_slice(&(img.w as u16).to_le_bytes());
    d[14..16].copy_from_slice(&(img.h as u16).to_le_bytes());
    d[16] = 32;
    d[17] = 0x28;
    for p in img.px.chunks_exact(4) {
        d.extend_from_slice(&[p[2], p[1], p[0], p[3]]);
    }
    d
}

/// Reduces a picture to at most `n` colours (popularity of 5-bit buckets, then the nearest of those for every pixel), with
/// `key` (when given) as palette entry `n` for the transparent pixels (alpha below 128).
pub fn quantise(img: &Rgba, n: usize, key: Option<[u8; 3]>) -> Indexed {
    let mut count = vec![0u32; 32768];
    let mut sum = vec![[0u64; 3]; 32768];
    let bucket = |p: &[u8]| ((p[0] as usize >> 3) << 10) | ((p[1] as usize >> 3) << 5) | (p[2] as usize >> 3);
    for p in img.px.chunks_exact(4) {
        if key.is_some() && p[3] < 128 {
            continue;
        }
        let b = bucket(p);
        count[b] += 1;
        for k in 0..3 {
            sum[b][k] += p[k] as u64;
        }
    }
    let mut order: Vec<usize> = (0..32768).filter(|&b| count[b] > 0).collect();
    order.sort_by_key(|&b| (std::cmp::Reverse(count[b]), b));
    // keep colours that differ: skip a bucket next to one already taken while there are more buckets than slots
    let mut pal: Vec<[u8; 3]> = Vec::new();
    let crowded = order.len() > n;
    for &b in &order {
        if pal.len() >= n {
            break;
        }
        let c = [(sum[b][0] / count[b] as u64) as u8, (sum[b][1] / count[b] as u64) as u8, (sum[b][2] / count[b] as u64) as u8];
        if crowded && pal.iter().any(|q| dist(q, &c) < 48) {
            continue;
        }
        pal.push(c);
    }
    if pal.is_empty() {
        pal.push([0, 0, 0]);
    }
    let mut out = Indexed { w: img.w, h: img.h, idx: vec![0; (img.w * img.h) as usize], pal: [0; 768] };
    for (k, c) in pal.iter().enumerate() {
        out.pal[k * 3..k * 3 + 3].copy_from_slice(c);
    }
    let key_index = n.min(255);
    if let Some(k) = key {
        out.pal[key_index * 3..key_index * 3 + 3].copy_from_slice(&k);
    }
    let mut cache: std::collections::HashMap<usize, u8> = std::collections::HashMap::new();
    for (i, p) in img.px.chunks_exact(4).enumerate() {
        if key.is_some() && p[3] < 128 {
            out.idx[i] = key_index as u8;
            continue;
        }
        let b = bucket(p);
        let v = *cache.entry(b).or_insert_with(|| {
            let c = [p[0], p[1], p[2]];
            (0..pal.len()).min_by_key(|&k| dist(&pal[k], &c)).unwrap_or(0) as u8
        });
        out.idx[i] = v;
    }
    out
}

fn dist(a: &[u8; 3], b: &[u8; 3]) -> i32 {
    (0..3).map(|k| (a[k] as i32 - b[k] as i32).pow(2)).sum()
}

/// An 8-bit PCX of a picture; with `key` the transparent pixels take that colour (the original's colour key).
pub fn pcx(img: &Rgba, key: Option<[u8; 3]>) -> Vec<u8> {
    encode_pcx(&quantise(img, if key.is_some() { 255 } else { 256 }, key))
}

/// An 8-bit PCX straight from indices and a palette.
pub fn pcx_indexed(w: u32, h: u32, idx: Vec<u8>, pal: &[[u8; 3]; 256]) -> Vec<u8> {
    let mut p = [0u8; 768];
    for (k, c) in pal.iter().enumerate() {
        p[k * 3..k * 3 + 3].copy_from_slice(c);
    }
    encode_pcx(&Indexed { w, h, idx, pal: p })
}

/// What a sprite file's header says (docs/SPRITES.md): `views` 0 writes a plain FLC without the Firaxis extension.
#[derive(Clone, Copy, Debug)]
pub struct FlcHead {
    pub w: u32,
    pub h: u32,
    pub views: u32,
    pub fpv: u32,
    pub crop: (u32, u32),
    pub view_ms: u32,
    pub mask: u32,
    pub speed: u32,
}

/// An 8-bit FLC. `frames` are view-major (views x fpv, or the plain frames when views is 0), each w x h indices; with the
/// extension every view is followed by its ring frame (the view's first frame again), as the game's files store them.
pub fn flc(head: &FlcHead, pal: &[[u8; 3]; 256], frames: &[Vec<u8>]) -> Vec<u8> {
    let (w, h) = (head.w as usize, head.h as usize);
    let mut d = vec![0u8; 128];
    d[4..6].copy_from_slice(&0xAF12u16.to_le_bytes());
    d[6..8].copy_from_slice(&(frames.len() as u16).to_le_bytes());
    d[8..10].copy_from_slice(&(head.w as u16).to_le_bytes());
    d[10..12].copy_from_slice(&(head.h as u16).to_le_bytes());
    d[12..14].copy_from_slice(&8u16.to_le_bytes());
    d[16..20].copy_from_slice(&head.speed.to_le_bytes());
    let mut stored: Vec<&Vec<u8>> = Vec::new();
    if head.views > 0 {
        d[96..98].copy_from_slice(&(head.views as u16).to_le_bytes());
        d[98..100].copy_from_slice(&(head.fpv as u16).to_le_bytes());
        d[100..102].copy_from_slice(&(head.crop.0 as u16).to_le_bytes());
        d[102..104].copy_from_slice(&(head.crop.1 as u16).to_le_bytes());
        d[104..106].copy_from_slice(&480u16.to_le_bytes());
        d[106..108].copy_from_slice(&480u16.to_le_bytes());
        d[108..110].copy_from_slice(&(head.view_ms as u16).to_le_bytes());
        d[112..114].copy_from_slice(&(head.mask as u16).to_le_bytes());
        for v in 0..head.views as usize {
            let run = &frames[v * head.fpv as usize..(v + 1) * head.fpv as usize];
            stored.extend(run.iter());
            stored.push(&run[0]);
        }
    } else {
        stored.extend(frames.iter());
    }
    let blank = vec![0u8; w * h];
    let mut prev: &Vec<u8> = &blank;
    for (k, fr) in stored.iter().enumerate() {
        let mut chunks: Vec<(u16, Vec<u8>)> = Vec::new();
        if k == 0 {
            let mut p = vec![1, 0, 0, 0];
            for c in pal {
                p.extend_from_slice(c);
            }
            chunks.push((4, p));
            chunks.push((15, brun(fr, w, h)));
        } else if *fr != prev {
            let delta = delta_flc(prev, fr, w, h);
            let full = brun(fr, w, h);
            chunks.push(if delta.len() < full.len() { (7, delta) } else { (15, full) });
        }
        let mut body = Vec::new();
        for (t, c) in &chunks {
            body.extend_from_slice(&((c.len() + 6) as u32).to_le_bytes());
            body.extend_from_slice(&t.to_le_bytes());
            body.extend_from_slice(c);
        }
        d.extend_from_slice(&((body.len() + 16) as u32).to_le_bytes());
        d.extend_from_slice(&0xF1FAu16.to_le_bytes());
        d.extend_from_slice(&(chunks.len() as u16).to_le_bytes());
        d.extend_from_slice(&[0; 8]);
        d.extend(body);
        prev = fr;
    }
    let len = d.len() as u32;
    d[0..4].copy_from_slice(&len.to_le_bytes());
    d
}

/// BYTE_RUN (chunk 15): every line as runs (a positive count repeats the next byte) and literals (a negative count).
fn brun(f: &[u8], w: usize, h: usize) -> Vec<u8> {
    let mut out = Vec::new();
    for y in 0..h {
        out.push(0);
        let row = &f[y * w..(y + 1) * w];
        let mut x = 0;
        while x < w {
            let mut run = 1;
            while x + run < w && run < 127 && row[x + run] == row[x] {
                run += 1;
            }
            if run >= 3 {
                out.push(run as u8);
                out.push(row[x]);
                x += run;
                continue;
            }
            // a literal up to the next run of three
            let start = x;
            while x < w && x - start < 127 {
                if x + 2 < w && row[x] == row[x + 1] && row[x] == row[x + 2] {
                    break;
                }
                x += 1;
            }
            out.push((-((x - start) as i32)) as i8 as u8);
            out.extend_from_slice(&row[start..x]);
        }
    }
    out
}

/// DELTA_FLC (chunk 7): changed lines only, as packets of pixel pairs; skipped lines and an odd last pixel by their words.
fn delta_flc(prev: &[u8], f: &[u8], w: usize, h: usize) -> Vec<u8> {
    let mut out = vec![0u8, 0];
    let mut lines = 0u16;
    let mut skip = 0usize;
    let pairs = w / 2;
    for y in 0..h {
        let (a, b) = (&prev[y * w..(y + 1) * w], &f[y * w..(y + 1) * w]);
        if a == b {
            skip += 1;
            continue;
        }
        while skip > 0 {
            let s = skip.min(16000);
            out.extend_from_slice(&((-(s as i32)) as i16 as u16).to_le_bytes());
            skip -= s;
        }
        if w % 2 == 1 && a[w - 1] != b[w - 1] {
            out.extend_from_slice(&(0x8000u16 | b[w - 1] as u16).to_le_bytes());
        }
        let mut packets: Vec<u8> = Vec::new();
        let mut np = 0u16;
        let mut x = 0usize; // pair index the last packet ended at
        let mut k = 0usize;
        while k < pairs {
            if a[2 * k] == b[2 * k] && a[2 * k + 1] == b[2 * k + 1] {
                k += 1;
                continue;
            }
            // advance to pair k
            let mut adv = (k - x) * 2;
            while adv > 254 {
                packets.extend_from_slice(&[254, 0]);
                np += 1;
                adv -= 254;
            }
            // the changed run
            let mut e = k;
            while e < pairs && e - k < 127 && (a[2 * e] != b[2 * e] || a[2 * e + 1] != b[2 * e + 1]) {
                e += 1;
            }
            let words: Vec<[u8; 2]> = (k..e).map(|i| [b[2 * i], b[2 * i + 1]]).collect();
            if words.len() >= 2 && words.iter().all(|wd| *wd == words[0]) {
                packets.push(adv as u8);
                packets.push((-(words.len() as i32)) as i8 as u8);
                packets.extend_from_slice(&words[0]);
            } else {
                packets.push(adv as u8);
                packets.push(words.len() as u8);
                for wd in &words {
                    packets.extend_from_slice(wd);
                }
            }
            np += 1;
            x = e;
            k = e;
        }
        out.extend_from_slice(&np.to_le_bytes());
        out.extend(packets);
        lines += 1;
    }
    out[0..2].copy_from_slice(&lines.to_le_bytes());
    out
}

/// A mono 16-bit PCM WAV.
pub fn wav(rate: u32, samples: &[f32]) -> Vec<u8> {
    let pcm: Vec<u8> = samples.iter().flat_map(|s| ((s.clamp(-1.0, 1.0) * 32000.0) as i16).to_le_bytes()).collect();
    let mut d = Vec::with_capacity(44 + pcm.len());
    d.extend_from_slice(b"RIFF");
    d.extend_from_slice(&(36 + pcm.len() as u32).to_le_bytes());
    d.extend_from_slice(b"WAVEfmt ");
    d.extend_from_slice(&16u32.to_le_bytes());
    d.extend_from_slice(&[1, 0, 1, 0]);
    d.extend_from_slice(&rate.to_le_bytes());
    d.extend_from_slice(&(rate * 2).to_le_bytes());
    d.extend_from_slice(&[2, 0, 16, 0]);
    d.extend_from_slice(b"data");
    d.extend_from_slice(&(pcm.len() as u32).to_le_bytes());
    d.extend(pcm);
    d
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn flc_round_trip_with_views_and_deltas() {
        let (w, h) = (13usize, 7usize);
        let mut pal = [[0u8; 3]; 256];
        pal[3] = [10, 20, 30];
        let frames: Vec<Vec<u8>> =
            (0..6).map(|k| (0..w * h).map(|i| ((i * 7 + k * 3) % 5) as u8 + if i % 9 == 0 { 200 } else { 0 }).collect()).collect();
        let head = FlcHead { w: w as u32, h: h as u32, views: 2, fpv: 3, crop: (100, 120), view_ms: 250, mask: 3, speed: 83 };
        let d = flc(&head, &pal, &frames);
        let f = crate::flc::decode_flc(&d).unwrap();
        assert_eq!((f.views, f.frames_per_view, f.crop_x, f.crop_y), (2, 3, 100, 120));
        assert_eq!(f.frames.len(), 6);
        for (a, b) in f.frames.iter().zip(&frames) {
            assert_eq!(&a.idx, b);
        }
        assert_eq!(&f.frames[0].pal[9..12], &[10, 20, 30]);
    }

    #[test]
    fn quantised_pcx_keeps_the_key() {
        let mut img = Rgba::new(4, 2);
        for (i, p) in img.px.chunks_exact_mut(4).enumerate() {
            p.copy_from_slice(&[(i * 30) as u8, 100, 50, if i == 0 { 0 } else { 255 }]);
        }
        let d = pcx(&img, Some([255, 0, 255]));
        let back = crate::assets::decode_pcx(&d).unwrap();
        assert_eq!(&back.px[0..3], &[255, 0, 255]);
        assert_eq!(&back.px[4..7], &[30, 100, 50]);
    }
}
