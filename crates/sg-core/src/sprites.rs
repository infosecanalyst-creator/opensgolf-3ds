//! Pre-rendered sprites (trees, buildings, golfers...). Layout facts come from the Firaxis header extension of the .flc files,
//! see docs/SPRITES.md.
use crate::assets::{read_pcx_palette, to_rgba, Rgba};
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
}

impl Sprite {
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
    };
    let lower = flc_path.to_string_lossy().to_lowercase().replace('\\', "/");
    let people = !shadow && ["/male/", "/female/", "/celebs/"].iter().any(|d| lower.contains(d));
    for src in &f.frames {
        if people {
            out.indexed.push(src.idx.clone());
        }
        let mut ix = src.clone();
        if let Some(p) = over {
            ix.pal = p;
        }
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
        out.frames.push(img);
    }
    if out.frames.is_empty() {
        return Err(format!("{}: no frames", flc_path.display()));
    }
    Ok(out)
}

/// One frame of a person clip in the given palette: index 255 is transparent.
pub fn recolour(s: &Sprite, frame: usize, pal: &[u8; 768]) -> Option<Rgba> {
    let idx = s.indexed.get(frame)?;
    let mut img = Rgba::new(s.w, s.h);
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
