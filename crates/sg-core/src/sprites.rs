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
}

impl Sprite {
    pub fn frame_index(&self, view: i32, f: i32) -> usize {
        let view = view.rem_euclid(self.views.max(1));
        let f = f.rem_euclid(self.frames_per_view.max(1));
        (view * self.frames_per_view + f) as usize
    }
}

/// Loads a sprite .flc. Palette index 255 is the transparent key colour (magenta or cyan depending on the file). For shadow
/// sprites the palette is a ramp of 4 greens plus white that becomes translucent black of increasing density.
/// `palette_pcx`, when given, replaces the FLC palette with the one of an 8-bit PCX (colour variants).
pub fn load_sprite(flc_path: &Path, shadow: bool, palette_pcx: Option<&Path>) -> Result<Sprite, String> {
    let d = std::fs::read(flc_path).map_err(|_| format!("cannot read {}", flc_path.display()))?;
    let f = decode_flc(&d).map_err(|e| format!("{}: {}", flc_path.display(), e))?;
    let over = palette_pcx.and_then(|p| std::fs::read(p).ok()).and_then(|p| read_pcx_palette(&p));
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
    for src in &f.frames {
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
                // white = lightest density, pure green = darkest.
                let dens = 1.0 - ix.pal[ix.idx[i] as usize * 3] as f32 / 255.0;
                img.px[i * 4] = 0;
                img.px[i * 4 + 1] = 0;
                img.px[i * 4 + 2] = 0;
                img.px[i * 4 + 3] = (70.0 + dens * 90.0) as u8;
            }
        }
        out.frames.push(img);
    }
    Ok(out)
}
