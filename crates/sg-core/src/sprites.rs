//! Pre-rendered sprites (trees, buildings, golfers...). Layout facts come from the Firaxis header extension of the .flc files,
//! see docs/SPRITES.md.
use crate::assets::{read_pcx_palette, to_rgba_parts, Rgba};
use crate::flc::decode_flc;
use std::borrow::Cow;
use std::path::Path;
use std::sync::Arc;

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
    /// View-major; `frame` gives a frame's RGBA, transparent where the key colour was.
    pub frames: Vec<Frame>,
}

/// What a .flc frame's palette indices are drawn as.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Kind {
    /// The palette's colours.
    Plain,
    /// The palette's colours, and the indices kept for recolouring: a golfer's or a celebrity's clip, recoloured per person
    /// (sg_core::bodies).
    Person,
    /// Translucent black (a shadow sprite).
    Shadow,
}

/// One frame of a sprite.
///
/// A frame read from a .flc keeps its palette indices and is made RGBA each time it is asked for (`Sprite::frame`), which
/// is when the game makes its texture or an effect's picture from it, both kept. The port's own: a game holds every frame
/// of the sprites it has loaded (some 31,000 when it starts, about 7,000 more as golfers of each new body come), while only
/// those drawn ever become textures; kept as RGBA they took four times the memory, four fifths of the game's heap, and the
/// browser's memory never shrinks again.
#[derive(Clone, Debug)]
pub enum Frame {
    /// A .flc frame: w x h palette indices (255 the transparent key) and its palette, shared with the sprite's other frames.
    Indexed { w: u32, h: u32, idx: Vec<u8>, pal: Arc<[u8; 768]>, kind: Kind },
    /// RGBA pixels (a content pack's sheet, a picture the game makes).
    Rgba(Rgba),
    /// OpenSGolf (3DS): a .flc frame kept at half size with its runs packed (`pack`), its palette indices and their
    /// coverage (how much of each 2 x 2 block was opaque, which becomes its alpha): the decoded frames of one golfer's clips
    /// were 35 MB as RGBA, more than the console had left; packed they are under 3. Only made on the 3DS (`PACKED`).
    Packed { w: u32, h: u32, rle: Vec<u8>, cov: Vec<u8>, pal: Arc<[u8; 768]>, kind: Kind },
}

/// Frames kept packed at half size: the 3DS only.
const PACKED: bool = cfg!(target_os = "horizon");

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
    out.shrink_to_fit();
    out
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

/// (value, run) pairs; a trailing run of 4 (fully opaque, or empty) is dropped, `unpack_runs` fills it back.
fn pack_runs(v: &[u8]) -> Vec<u8> {
    let mut out = Vec::new();
    let mut i = 0;
    while i < v.len() {
        let s = i;
        while i < v.len() && v[i] == v[s] && i - s < 255 {
            i += 1;
        }
        out.extend_from_slice(&[v[s], (i - s) as u8]);
    }
    while out.len() >= 2 && out[out.len() - 2] == 4 {
        out.truncate(out.len() - 2);
    }
    out.shrink_to_fit();
    out
}

fn unpack_runs(r: &[u8], n: usize) -> Vec<u8> {
    let mut out = Vec::with_capacity(n);
    for p in r.chunks_exact(2) {
        out.resize(out.len() + p[1] as usize, p[0]);
    }
    out.resize(n, 4);
    out
}

/// Palette indices at half size: each 2 x 2 block keeps its first opaque texel, and how many of the four were opaque (its
/// coverage, 0..4). Shadows' indices are shade levels, kept as they are.
fn halve_indices(idx: &[u8], w: u32, h: u32) -> (Vec<u8>, Vec<u8>) {
    let (hw, hh) = (w.div_ceil(2).max(1), h.div_ceil(2).max(1));
    let mut out = vec![255u8; (hw * hh) as usize];
    let mut cov = vec![0u8; (hw * hh) as usize];
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
            if n >= 1 {
                out[(y * hw + x) as usize] = first;
            }
            // a block cut by the frame's edge counts the missing texels as opaque; an empty block says 4 (see-through
            // anyway), so the runs only break at edges
            let present = (if x * 2 + 1 < w { 2 } else { 1 }) * (if y * 2 + 1 < h { 2 } else { 1 });
            cov[(y * hw + x) as usize] = if n == 0 { 4 } else { (n + 4 - present).min(4) as u8 };
        }
    }
    (out, cov)
}

impl Sprite {
    pub fn frame_index(&self, view: i32, f: i32) -> usize {
        let view = view.rem_euclid(self.views.max(1));
        let f = f.rem_euclid(self.frames_per_view.max(1));
        // bounds-safe (docs/FIXES.md 5): a file whose header promises more views or frames than it holds (a mod's, a
        // damaged copy's) shows its last frame instead of reading past the end
        ((view * self.frames_per_view + f) as usize).min(self.frames.len().saturating_sub(1))
    }

    /// Frame `i` as RGBA (made from the palette indices for a .flc frame, so worth keeping when it is needed again).
    pub fn frame(&self, i: usize) -> Cow<'_, Rgba> {
        match &self.frames[i] {
            Frame::Rgba(img) => Cow::Borrowed(img),
            Frame::Indexed { w, h, idx, pal, kind } => Cow::Owned(expand(*w, *h, idx, pal, *kind)),
            Frame::Packed { w, h, rle, cov, pal, kind } => {
                let n = (*w * *h) as usize;
                let mut img = expand(*w, *h, &unpack(rle, n), pal, *kind);
                for (i, c) in unpack_runs(cov, n).into_iter().enumerate() {
                    if c < 4 {
                        let a = &mut img.px[i * 4 + 3];
                        *a = (*a as u32 * c as u32 / 4) as u8;
                    }
                }
                Cow::Owned(img)
            }
        }
    }

    /// Frame `i`'s width and height.
    pub fn frame_size(&self, i: usize) -> (u32, u32) {
        match &self.frames[i] {
            Frame::Rgba(img) => (img.w, img.h),
            Frame::Indexed { w, h, .. } | Frame::Packed { w, h, .. } => (*w, *h),
        }
    }

    /// The palette indices of frame `i` of a golfer's or a celebrity's clip, for recolouring (None for anything else).
    pub fn indexed(&self, i: usize) -> Option<Cow<'_, [u8]>> {
        match self.frames.get(i)? {
            Frame::Indexed { idx, kind: Kind::Person, .. } => Some(Cow::Borrowed(idx)),
            Frame::Packed { w, h, rle, kind: Kind::Person, .. } => Some(Cow::Owned(unpack(rle, (*w * *h) as usize))),
            _ => None,
        }
    }

    /// Whether the frames can be recoloured per person (`indexed`): a golfer's or a celebrity's clip read from a .flc.
    pub fn recolourable(&self) -> bool {
        matches!(self.frames.first(), Some(Frame::Indexed { kind: Kind::Person, .. } | Frame::Packed { kind: Kind::Person, .. }))
    }
}

/// A .flc frame's RGBA: the palette's colours with the key transparent and the neighbours' colours spread under it, or a
/// shadow's translucent black.
fn expand(w: u32, h: u32, idx: &[u8], pal: &[u8; 768], kind: Kind) -> Rgba {
    let mut img = to_rgba_parts(w, h, idx, pal, Some(255));
    if kind != Kind::Shadow {
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
        for i in 0..(w * h) as usize {
            if idx[i] == 255 {
                continue;
            }
            // jgl's blit (0x10013400) draws indices 0xf8..0xfc through the exe's shading tables (0x824148, built by
            // 0x461830): 0xf8 to 0xfb scale the screen pixel's 5-bit channels by 4/8, 5/8, 6/8 and 7/8 (black at
            // 1/2, 3/8, 1/4 and 1/8 strength here). Other indices keep the earlier ramp reading (white = lightest).
            let alpha = match idx[i] {
                0xf8 => 128,
                0xf9 => 96,
                0xfa => 64,
                0xfb => 32,
                k => {
                    let dens = 1.0 - pal[k as usize * 3] as f32 / 255.0;
                    (70.0 + dens * 90.0) as u8
                }
            };
            img.px[i * 4] = 0;
            img.px[i * 4 + 1] = 0;
            img.px[i * 4 + 2] = 0;
            img.px[i * 4 + 3] = alpha;
        }
    }
    img
}

/// Loads a sprite .flc. Palette index 255 is the transparent key colour (magenta or cyan depending on the file). For shadow
/// sprites the palette is a ramp of 4 greens plus white that becomes translucent black of increasing density.
/// `palette_pcx`, when given, replaces the FLC palette with the one of an 8-bit PCX (colour variants).
pub fn load_sprite(flc_path: &Path, shadow: bool, palette_pcx: Option<&Path>) -> Result<Sprite, String> {
    let d = crate::fsutil::read_file(flc_path)
        .ok_or_else(|| format!("cannot read {} ({})", flc_path.display(), crate::fsutil::why_unreadable(flc_path)))?;
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
    };
    let lower = flc_path.to_string_lossy().to_lowercase().replace('\\', "/");
    let people = !shadow && ["/male/", "/female/", "/celebs/"].iter().any(|d| lower.contains(d));
    let kind = if shadow {
        Kind::Shadow
    } else if people {
        Kind::Person
    } else {
        Kind::Plain
    };
    // the frames of a file nearly always share one palette: kept once
    let mut shared: Option<Arc<[u8; 768]>> = None;
    for src in f.frames {
        let pal = over.unwrap_or(src.pal);
        let pal = match &shared {
            Some(p) if **p == pal => p.clone(),
            _ => shared.insert(Arc::new(pal)).clone(),
        };
        if PACKED {
            let (half, cov) = halve_indices(&src.idx, src.w, src.h);
            let (w, h) = (src.w.div_ceil(2).max(1), src.h.div_ceil(2).max(1));
            out.frames.push(Frame::Packed { w, h, rle: pack(&half), cov: pack_runs(&cov), pal, kind });
            continue;
        }
        out.frames.push(Frame::Indexed { w: src.w, h: src.h, idx: src.idx, pal, kind });
    }
    if out.frames.is_empty() {
        return Err(format!("{}: no frames", flc_path.display()));
    }
    Ok(out)
}

/// One frame of a person clip in the given palette: index 255 is transparent.
pub fn recolour(s: &Sprite, frame: usize, pal: &[u8; 768]) -> Option<Rgba> {
    let idx = s.indexed(frame)?;
    let (w, h) = s.frame_size(frame);
    let mut img = Rgba::new(w, h);
    for (i, &c) in idx.iter().enumerate() {
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
    fn frame_index_stays_in_the_file() {
        // a header promising 8 views of 4 frames over a file holding 3 frames (docs/FIXES.md 5)
        let s = Sprite { views: 8, frames_per_view: 4, frames: vec![Frame::Rgba(Rgba::new(1, 1)); 3], ..Default::default() };
        for v in -9..20 {
            for f in -9..20 {
                assert!(s.frame_index(v, f) < 3);
            }
        }
        let s = Sprite { views: 2, frames_per_view: 3, frames: vec![Frame::Rgba(Rgba::new(1, 1)); 6], ..Default::default() };
        assert_eq!(s.frame_index(1, 2), 5);
    }

    #[test]
    fn indexed_frames_are_made_rgba_on_use() {
        let mut pal = [0u8; 768];
        pal[3..6].copy_from_slice(&[10, 20, 30]); // entry 1
        pal[0xf9 * 3] = 0; // a shadow's 3/8 strength entry
        pal[255 * 3..].copy_from_slice(&[255, 0, 255]); // the key
        let pal = Arc::new(pal);
        let sprite = |idx: [u8; 2], kind| Sprite {
            w: 2,
            h: 1,
            frames: vec![Frame::Indexed { w: 2, h: 1, idx: idx.to_vec(), pal: pal.clone(), kind }],
            ..Default::default()
        };
        // a plain frame: the palette's colours, the key transparent with its neighbour's colour spread under it
        let s = sprite([1, 255], Kind::Plain);
        assert_eq!(s.frame(0).px, [10, 20, 30, 255, 10, 20, 30, 0]);
        assert_eq!(s.frame_size(0), (2, 1));
        assert!(!s.recolourable() && s.indexed(0).is_none());
        // a shadow: translucent black, the key left clear
        let s = sprite([255, 0xf9], Kind::Shadow);
        assert_eq!(s.frame(0).px, [255, 0, 255, 0, 0, 0, 0, 96]);
        // a person's clip keeps its indices for the outfits
        let s = sprite([1, 255], Kind::Person);
        assert!(s.recolourable());
        assert_eq!(s.indexed(0).as_deref(), Some(&[1u8, 255][..]));
        assert_eq!(recolour(&s, 0, &pal).unwrap().px, [10, 20, 30, 255, 255, 0, 255, 0]);
        // RGBA frames are lent as they are
        let s = Sprite { frames: vec![Frame::Rgba(Rgba::new(2, 2))], ..Default::default() };
        assert!(matches!(s.frame(0), Cow::Borrowed(_)));
    }
}
