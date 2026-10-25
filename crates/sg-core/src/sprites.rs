//! Pre-rendered sprites (trees, buildings, golfers...). Layout facts come from the Firaxis header extension of the .flc files,
//! see docs/SPRITES.md.
use crate::assets::{read_pcx_palette, to_rgba, Indexed, Rgba};
use crate::flc::decode_flc;
use std::path::Path;

/// World units covered by one sprite pixel. The sprites were rendered for the 800x600 camera, whose orthographic window is
/// 1767 world units wide (see docs/TERRAIN.md): 1767 / 800.
pub const SPRITE_UNITS_PER_PIXEL: f32 = 1767.0 / 800.0;

#[derive(Clone, Debug, Default)]
pub struct Sprite {
    pub views: i32,
    pub frames_per_view: i32,
    /// Pixel in the frame that sits on the object's ground point.
    pub anchor_x: i32,
    pub anchor_y: i32,
    pub w: u32,
    pub h: u32,
    pub frame_ms: u32,
    pub view_mask: u32,
    pub shadow: bool,
    /// View-major, RGBA, transparent where the key colour was.
    pub frames: Vec<Rgba>,
    /// The palette indices of each frame, kept for golfer and celebrity clips, which are recoloured per person
    /// (sg_core::bodies); empty for everything else.
    pub indexed: Vec<Vec<u8>>,
    /// A person clip (golfers, celebrities): its frames can be recoloured per person (`recolour`).
    pub people: bool,
    /// The 3DS's frames, kept packed (`pack`) and turned into pictures only when first drawn (`unpacked`): the decoded
    /// frames of one golfer's clips alone were 35 MB, more than the console had left; packed they are under 3. `frames`
    /// then holds empty pictures of the right size. Empty elsewhere.
    pub packed: Vec<Packed>,
    /// The palettes the packed frames use (`Packed::pal` indexes this).
    pub pals: Vec<[u8; 768]>,
}

/// One frame's palette indices, the key colour's runs packed: [0, run] for a run of key texels (up to 255), [n, n
/// indices] for up to 255 other texels.
#[derive(Clone, Debug, Default)]
pub struct Packed {
    pub rle: Vec<u8>,
    pub pal: u16,
}

fn pack(idx: &[u8]) -> Vec<u8> {
    let mut out = Vec::with_capacity(idx.len() / 4);
    let mut i = 0;
    while i < idx.len() {
        let s = i;
        if idx[i] == 255 {
            while i < idx.len() && idx[i] == 255 && i - s < 255 {
                i += 1;
            }
            out.extend_from_slice(&[0, (i - s) as u8]);
        } else {
            while i < idx.len() && idx[i] != 255 && i - s < 255 {
                i += 1;
            }
            out.push((i - s) as u8);
            out.extend_from_slice(&idx[s..i]);
        }
    }
    out
}

/// Palette indices at half size, for the 3DS's packed frames: each 2 x 2 block keeps its first opaque texel when at least
/// two of the four are opaque (so outlines hold their shape and thin specks do not swell), else the key colour. Shadows'
/// indices are shade levels, which this keeps as they are.
fn halve_indices(idx: &[u8], w: u32, h: u32) -> (Vec<u8>, u32, u32) {
    let (hw, hh) = (w.div_ceil(2).max(1), h.div_ceil(2).max(1));
    let mut out = vec![255u8; (hw * hh) as usize];
    for y in 0..hh {
        for x in 0..hw {
            let mut first = 255u8;
            let mut n = 0;
            for (dx, dy) in [(0, 0), (1, 0), (0, 1), (1, 1)] {
                let (sx, sy) = (x * 2 + dx, y * 2 + dy);
                if sx < w && sy < h {
                    let v = idx[(sy * w + sx) as usize];
                    if v != 255 {
                        if n == 0 {
                            first = v;
                        }
                        n += 1;
                    }
                }
            }
            if n >= 2 {
                out[(y * hw + x) as usize] = first;
            }
        }
    }
    (out, hw, hh)
}

fn unpack(rle: &[u8], n: usize) -> Vec<u8> {
    let mut out = Vec::with_capacity(n);
    let mut p = 0;
    while p < rle.len() {
        let c = rle[p] as usize;
        if c == 0 {
            let run = rle.get(p + 1).copied().unwrap_or(0) as usize;
            out.resize(out.len() + run, 255);
            p += 2;
        } else {
            let end = (p + 1 + c).min(rle.len());
            out.extend_from_slice(&rle[p + 1..end]);
            p = end;
        }
    }
    out.resize(n, 255);
    out
}

impl Sprite {
    /// A packed frame's picture (the 3DS, see `packed`); None for a frame kept as a picture.
    pub fn unpacked(&self, i: usize) -> Option<Rgba> {
        let p = self.packed.get(i)?;
        let (w, h) = self.packed_size(i);
        let ix = Indexed { w, h, idx: unpack(&p.rle, (w * h) as usize), pal: *self.pals.get(p.pal as usize)? };
        Some(frame_picture(&ix, self.shadow))
    }

    /// A frame's size as stored: packed frames are kept at half size (the 3DS draws them at about half the 800 x 600
    /// screen's pixels anyway), the empty picture in `frames` saying how big.
    fn packed_size(&self, i: usize) -> (u32, u32) {
        match (self.packed.is_empty(), self.frames.get(i)) {
            (false, Some(f)) => (f.w, f.h),
            _ => (self.w, self.h),
        }
    }

    /// A frame's palette indices, kept or packed.
    pub fn frame_indices(&self, i: usize) -> Option<std::borrow::Cow<'_, [u8]>> {
        if let Some(v) = self.indexed.get(i) {
            return Some(std::borrow::Cow::Borrowed(v));
        }
        let p = self.packed.get(i)?;
        let (w, h) = self.packed_size(i);
        Some(std::borrow::Cow::Owned(unpack(&p.rle, (w * h) as usize)))
    }

    /// Whether `recolour` can draw it in a person's outfit.
    pub fn recolourable(&self) -> bool {
        !self.indexed.is_empty() || (self.people && !self.packed.is_empty())
    }

    pub fn frame_index(&self, view: i32, f: i32) -> usize {
        let view = view.rem_euclid(self.views.max(1));
        let f = f.rem_euclid(self.frames_per_view.max(1));
        // bounds-safe (docs/FIXES.md 5): a file whose header promises more views or frames than it holds (a mod's, a
        // damaged copy's) shows its last frame instead of reading past the end
        ((view * self.frames_per_view + f) as usize).min(self.frames.len().saturating_sub(1))
    }
}

/// Loads a sprite .flc. Palette index 255 is the transparent key colour (magenta or cyan depending on the file). For shadow
/// sprites the palette is a ramp of 4 greens plus white that becomes translucent black of increasing density.
/// `palette_pcx`, when given, replaces the FLC palette with the one of an 8-bit PCX (colour variants).
pub fn load_sprite(flc_path: &Path, shadow: bool, palette_pcx: Option<&Path>) -> Result<Sprite, String> {
    let d = crate::fsutil::read_file(flc_path).ok_or_else(|| format!("cannot read {}", flc_path.display()))?;
    let f = decode_flc(&d).map_err(|e| format!("{}: {}", flc_path.display(), e))?;
    let over = palette_pcx.and_then(crate::fsutil::read_file).and_then(|p| read_pcx_palette(&p));
    let mut out = Sprite {
        shadow,
        w: f.w,
        h: f.h,
        frame_ms: f.frame_ms,
        views: if f.has_ext { f.views as i32 } else { 1 },
        frames_per_view: if f.has_ext { f.frames_per_view as i32 } else { f.frames.len() as i32 },
        view_mask: f.view_mask,
        anchor_x: if f.has_ext { (f.canvas_w / 2) as i32 - f.crop_x as i32 } else { f.w as i32 / 2 },
        anchor_y: if f.has_ext { (f.canvas_h / 2) as i32 - f.crop_y as i32 } else { f.h as i32 },
        frames: Vec::with_capacity(f.frames.len()),
        indexed: Vec::new(),
        people: false,
        packed: Vec::new(),
        pals: Vec::new(),
    };
    let lower = flc_path.to_string_lossy().to_lowercase().replace('\\', "/");
    let people = !shadow && ["/male/", "/female/", "/celebs/"].iter().any(|d| lower.contains(d));
    out.people = people;
    for src in &f.frames {
        if people && !PACKED {
            out.indexed.push(src.idx.clone());
        }
        let mut ix = src.clone();
        if let Some(p) = over {
            ix.pal = p;
        }
        // the 3DS: packed, made into a picture when first drawn
        if PACKED {
            let pal = match out.pals.iter().position(|q| q[..] == ix.pal[..]) {
                Some(k) => k,
                None => {
                    out.pals.push(ix.pal);
                    out.pals.len() - 1
                }
            };
            let (half, _, _) = halve_indices(&ix.idx, ix.w, ix.h);
            out.packed.push(Packed { rle: pack(&half), pal: pal as u16 });
            out.frames.push(Rgba { w: f.w.div_ceil(2).max(1), h: f.h.div_ceil(2).max(1), px: Vec::new() });
            continue;
        }
        out.frames.push(frame_picture(&ix, shadow));
    }
    if out.frames.is_empty() {
        return Err(format!("{}: no frames", flc_path.display()));
    }
    Ok(out)
}

/// Packed frames on the 3DS (`Sprite::packed`).
const PACKED: bool = cfg!(target_os = "horizon");

/// A frame's picture from its palette indices: the key colour transparent with the neighbouring colours spread into it, or
/// for a shadow translucent black. Half size on the 3DS.
fn frame_picture(ix: &Indexed, shadow: bool) -> Rgba {
    let mut img = to_rgba(&ix, Some(255));
    if !shadow {
        // Texture filtering must not pull the key colour into the edges: spread neighbour colours outwards.
        let (w, h) = (img.w as i32, img.h as i32);
        for _pass in 0..2 {
            let mut next = img.px.clone();
            for y in 0..h {
                for x in 0..w {
                    let i = ((y * w + x) * 4) as usize;
                    if img.px[i + 3] != 0 {
                        continue;
                    }
                    'search: for dy in -1..=1 {
                        for dx in -1..=1 {
                            let (nx, ny) = (x + dx, y + dy);
                            if nx < 0 || ny < 0 || nx >= w || ny >= h {
                                continue;
                            }
                            let q = ((ny * w + nx) * 4) as usize;
                            if img.px[q + 3] != 0 && next[i..i + 3] != img.px[q..q + 3] {
                                let (r, g, b) = (img.px[q], img.px[q + 1], img.px[q + 2]);
                                next[i] = r;
                                next[i + 1] = g;
                                next[i + 2] = b;
                                break 'search;
                            }
                        }
                    }
                }
            }
            img.px = next;
        }
    } else {
        for i in 0..(ix.w * ix.h) as usize {
            if ix.idx[i] == 255 {
                continue;
            }
            // jgl's blit (0x10013400) draws indices 0xf8..0xfc through the exe's shading tables (0x824148, built by
            // 0x461830): 0xf8 to 0xfb scale the screen pixel's 5-bit channels by 4/8, 5/8, 6/8 and 7/8 (black at
            // 1/2, 3/8, 1/4 and 1/8 strength here). Other indices keep the earlier ramp reading (white = lightest).
            let alpha = match ix.idx[i] {
                0xf8 => 128,
                0xf9 => 96,
                0xfa => 64,
                0xfb => 32,
                k => {
                    let dens = 1.0 - ix.pal[k as usize * 3] as f32 / 255.0;
                    (70.0 + dens * 90.0) as u8
                }
            };
            img.px[i * 4] = 0;
            img.px[i * 4 + 1] = 0;
            img.px[i * 4 + 2] = 0;
            img.px[i * 4 + 3] = alpha;
        }
    }
    // (on the 3DS the frames come here already at half size: `halve_indices`)
    img
}

/// A frame at half size, each pixel the average of a 2 x 2 block (colour weighted by alpha, so transparent pixels do not
/// darken an edge). The 3DS's sprite frames (see `load_sprite`).
#[allow(dead_code)]
fn half_frame(img: &Rgba) -> Rgba {
    let (w, h) = (img.w.div_ceil(2).max(1), img.h.div_ceil(2).max(1));
    let mut out = Rgba::new(w, h);
    for y in 0..h {
        for x in 0..w {
            let (mut c, mut a, mut n, mut wsum) = ([0u32; 3], 0u32, 0u32, 0u32);
            for dy in 0..2 {
                for dx in 0..2 {
                    let (sx, sy) = (x * 2 + dx, y * 2 + dy);
                    if sx >= img.w || sy >= img.h {
                        continue;
                    }
                    let i = ((sy * img.w + sx) * 4) as usize;
                    let pa = img.px[i + 3] as u32;
                    for k in 0..3 {
                        c[k] += img.px[i + k] as u32 * pa.max(1);
                    }
                    wsum += pa.max(1);
                    a += pa;
                    n += 1;
                }
            }
            let o = ((y * w + x) * 4) as usize;
            for k in 0..3 {
                out.px[o + k] = (c[k] / wsum.max(1)).min(255) as u8;
            }
            out.px[o + 3] = (a / n.max(1)) as u8;
        }
    }
    out
}

/// One frame of a person clip in the given palette: index 255 is transparent.
pub fn recolour(s: &Sprite, frame: usize, pal: &[u8; 768]) -> Option<Rgba> {
    let idx = s.frame_indices(frame)?;
    let (w, h) = s.packed_size(frame);
    let mut img = Rgba::new(w, h);
    for (i, &c) in idx.iter().enumerate().take((w * h) as usize) {
        let c = c as usize;
        img.px[i * 4..i * 4 + 3].copy_from_slice(&pal[c * 3..c * 3 + 3]);
        img.px[i * 4 + 3] = if c == 255 { 0 } else { 255 };
    }
    Some(img)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn packed_frames_round_trip() {
        let mut idx = vec![255u8; 1000];
        idx[3] = 7;
        for i in 300..900 {
            idx[i] = (i % 250) as u8;
        }
        assert_eq!(unpack(&pack(&idx), idx.len()), idx);
        assert_eq!(unpack(&pack(&[]), 0), Vec::<u8>::new());
        let Ok(dir) = std::env::var("SG_FLCS") else { return };
        for e in std::fs::read_dir(dir).unwrap() {
            let d = std::fs::read(e.unwrap().path()).unwrap();
            let Ok(f) = decode_flc(&d) else { continue };
            for fr in &f.frames {
                assert_eq!(unpack(&pack(&fr.idx), fr.idx.len()), fr.idx);
            }
        }
    }

    #[test]
    fn frame_index_stays_in_the_file() {
        // a header promising 8 views of 4 frames over a file holding 3 frames (docs/FIXES.md 5)
        let s = Sprite { views: 8, frames_per_view: 4, frames: vec![Rgba::new(1, 1); 3], ..Default::default() };
        for v in -9..20 {
            for f in -9..20 {
                assert!(s.frame_index(v, f) < 3);
            }
        }
        let s = Sprite { views: 2, frames_per_view: 3, frames: vec![Rgba::new(1, 1); 6], ..Default::default() };
        assert_eq!(s.frame_index(1, 2), 5);
    }
}
