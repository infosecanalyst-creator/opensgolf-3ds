//! The sprite atlas (the port's own; Classic only): people's, animals' and props' frames packed into a few large textures
//! instead of one texture each. Making a texture is slow in a browser (each one holds up the page while the driver sets it
//! up) and a busy course shows thousands of frames, each the first time it is needed, so play hitched as golfers came and
//! went; copying a frame into a page that is already there is quick, and the frames of one page draw together.
//!
//! The picture is the same: each frame keeps a border of copies of its own edge texels (GUTTER), so linear filtering at
//! its edge reads what the frame's own clamped texture would; the pages are filtered like the frames' textures were. When
//! every page is full the one used longest ago is emptied and filled again, a page at a time, never all at once.
//!
//! Frames larger than a page's slot (MAX_SIDE), and everything drawn with Enhanced graphics, HD art or a content pack's
//! art, keep a texture of their own as before.
use crate::gfx::{bleed_alpha, Gfx};
use miniquad::*;
use sg_core::assets::Rgba;

/// A page's side, in texels (every GPU the game runs on takes textures of at least this size).
pub const PAGE: u32 = 2048;
/// The copies of a frame's edge kept round it.
pub const GUTTER: u32 = 2;
/// Frames larger than this (with their border) keep a texture of their own.
pub const MAX_SIDE: u32 = 512;
/// The pages kept at most (16 MB each); beyond them the page used longest ago is filled again.
pub const MAX_PAGES: usize = 12;

/// A picture to draw: a texture and the part of it the picture covers (u0, v0, u1, v1).
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct STex {
    pub id: TextureId,
    pub uv: [f32; 4],
}

impl STex {
    /// A whole texture.
    pub fn whole(id: TextureId) -> STex {
        STex { id, uv: [0.0, 0.0, 1.0, 1.0] }
    }
    /// The texture coordinates of the picture's own (u, v), 0..1 across it.
    pub fn at(&self, u: f32, v: f32) -> (f32, f32) {
        (self.uv[0] + (self.uv[2] - self.uv[0]) * u, self.uv[1] + (self.uv[3] - self.uv[1]) * v)
    }
}

/// Where a frame was put: its page and the page's fill count then (a page filled again since has a new count).
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct Slot {
    page: usize,
    fill: u32,
}

struct Page {
    tex: TextureId,
    /// Rows of frames: their top, their height, and how far along they are filled.
    shelves: Vec<(u32, u32, u32)>,
    /// The first free row.
    top: u32,
    /// How many times the page has been emptied.
    fill: u32,
    /// The frame it was last drawn in (or a frame was put in it).
    used: u64,
}

#[derive(Default)]
pub struct Atlas {
    pages: Vec<Page>,
    frame: u64,
    /// Whether the pages are magnified unsmoothed (pixel-perfect scaling), as the frames' own textures would be.
    nearest: bool,
}

impl Atlas {
    /// Counts the frames (the age of the pages is in frames).
    pub fn next_frame(&mut self) {
        self.frame += 1;
    }

    /// Whether a frame put at `slot` is still there; it counts as used this frame.
    pub fn valid(&mut self, slot: Slot) -> bool {
        match self.pages.get_mut(slot.page) {
            Some(p) if p.fill == slot.fill => {
                p.used = self.frame;
                true
            }
            _ => false,
        }
    }

    /// Empties the atlas (the art or its filtering changed): the pages go and the next frames are put again.
    pub fn clear(&mut self, ctx: &mut dyn RenderingBackend) {
        for p in self.pages.drain(..) {
            ctx.delete_texture(p.tex);
        }
    }

    /// Room for a w x h frame (with its border): (page, x, y), making or emptying a page when none has room.
    fn place(&mut self, ctx: &mut dyn RenderingBackend, w: u32, h: u32) -> (usize, u32, u32) {
        for (k, p) in self.pages.iter_mut().enumerate() {
            if let Some(at) = fit(p, w, h) {
                return (k, at.0, at.1);
            }
        }
        let k = if self.pages.len() < MAX_PAGES {
            let params = TextureParams {
                kind: TextureKind::Texture2D,
                width: PAGE,
                height: PAGE,
                format: TextureFormat::RGBA8,
                wrap: TextureWrap::Clamp,
                min_filter: FilterMode::Linear,
                mag_filter: if self.nearest { FilterMode::Nearest } else { FilterMode::Linear },
                mipmap_filter: MipmapFilterMode::None,
                allocate_mipmaps: false,
                sample_count: 1,
            };
            // made without a picture (no 16 MB of blank to send): a frame and its border are all that is ever read
            let tex = ctx.new_texture(TextureAccess::Static, TextureSource::Empty, params);
            self.pages.push(Page { tex, shelves: Vec::new(), top: 0, fill: 0, used: self.frame });
            self.pages.len() - 1
        } else {
            // the page used longest ago is filled again; what was on it is put again when next needed
            let k = (0..self.pages.len()).min_by_key(|&k| self.pages[k].used).unwrap_or(0);
            let p = &mut self.pages[k];
            p.shelves.clear();
            p.top = 0;
            p.fill += 1;
            k
        };
        let p = &mut self.pages[k];
        p.used = self.frame;
        let at = fit(p, w, h).unwrap_or((0, 0));
        (k, at.0, at.1)
    }
}

/// A place for a w x h frame on a page, along a row of about its height or in a new row.
fn fit(p: &mut Page, w: u32, h: u32) -> Option<(u32, u32)> {
    // a row no more than a quarter taller than the frame, so short frames do not waste tall rows
    for s in p.shelves.iter_mut() {
        if h <= s.1 && h * 4 >= s.1 * 3 && s.2 + w <= PAGE {
            let x = s.2;
            s.2 += w;
            return Some((x, s.0));
        }
    }
    if p.top + h <= PAGE && w <= PAGE {
        let y = p.top;
        p.shelves.push((y, h, w));
        p.top += h;
        return Some((0, y));
    }
    None
}

/// The frame with GUTTER copies of its edge texels round it (what a clamped texture reads beyond its edge).
fn with_gutter(img: &Rgba) -> Rgba {
    let g = GUTTER as usize;
    let (w, h) = (img.w as usize, img.h as usize);
    let (pw, ph) = (w + 2 * g, h + 2 * g);
    let mut out = Rgba::new(pw as u32, ph as u32);
    for y in 0..ph {
        let sy = y.saturating_sub(g).min(h - 1);
        for x in 0..pw {
            let sx = x.saturating_sub(g).min(w - 1);
            let (s, d) = ((sy * w + sx) * 4, (y * pw + x) * 4);
            out.px[d..d + 4].copy_from_slice(&img.px[s..s + 4]);
        }
    }
    out
}

impl Gfx {
    /// A frame in the atlas, or None when it keeps a texture of its own: too large, or drawn with Enhanced graphics' own
    /// textures (smooth interface art) where the classic texture would not be what is drawn.
    pub fn atlas_texture(&mut self, img: &Rgba) -> Option<(STex, Slot)> {
        // the 3DS frontend makes its own textures (its gfx.rs); the atlas pages here would be a GPU that draws nothing
        if cfg!(target_os = "horizon") {
            return None;
        }
        if img.w == 0 || img.h == 0 || img.w + 2 * GUTTER > MAX_SIDE || img.h + 2 * GUTTER > MAX_SIDE {
            return None;
        }
        if crate::enhanced::smooth_ui() || crate::fx::active() || crate::hd::active() {
            return None;
        }
        let nearest = crate::enhanced::pixel_perfect();
        if nearest != self.atlas.nearest {
            // the filtering changed: the pages are made again with the new one
            self.atlas.clear(&mut *self.ctx);
            self.atlas.nearest = nearest;
        }
        // as Gfx::texture does: transparent pixels take their neighbours' colours, so smoothing mixes in no key colour
        let bled;
        let img = if img.px.chunks_exact(4).any(|p| p[3] == 0) {
            bled = bleed_alpha(img);
            &bled
        } else {
            img
        };
        let padded = with_gutter(img);
        let (page, x, y) = self.atlas.place(&mut *self.ctx, padded.w, padded.h);
        let p = &self.atlas.pages[page];
        let (tex, fill) = (p.tex, p.fill);
        self.ctx.texture_update_part(tex, x as i32, y as i32, padded.w as i32, padded.h as i32, &padded.px);
        let n = PAGE as f32;
        let (x0, y0) = ((x + GUTTER) as f32, (y + GUTTER) as f32);
        let uv = [x0 / n, y0 / n, (x0 + img.w as f32) / n, (y0 + img.h as f32) / n];
        Some((STex { id: tex, uv }, Slot { page, fill }))
    }

    /// Whether a frame put at `slot` is still in the atlas (it counts as used this frame).
    pub fn atlas_valid(&mut self, slot: Slot) -> bool {
        self.atlas.valid(slot)
    }

    /// Empties the atlas (the art changed: a theme, HD art or a pack's art on or off).
    pub fn atlas_clear(&mut self) {
        self.atlas.clear(&mut *self.ctx);
    }

    /// Counts a frame drawn (the pages' ages).
    pub fn atlas_frame(&mut self) {
        self.atlas.next_frame();
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn the_border_repeats_the_edge() {
        let mut img = Rgba::new(2, 1);
        img.px = vec![1, 2, 3, 255, 4, 5, 6, 0];
        let out = with_gutter(&img);
        assert_eq!((out.w, out.h), (2 + 2 * GUTTER, 1 + 2 * GUTTER));
        // the corner and the left border copy the first texel, the right border the last
        assert_eq!(&out.px[0..4], &[1, 2, 3, 255]);
        let row = (out.w * 4) as usize;
        let mid = row * GUTTER as usize;
        assert_eq!(&out.px[mid + (GUTTER as usize) * 4..mid + (GUTTER as usize) * 4 + 4], &[1, 2, 3, 255]);
        assert_eq!(&out.px[mid + row - 4..mid + row], &[4, 5, 6, 0]);
    }

    #[test]
    fn rows_fill_then_a_new_row_starts() {
        let mut p = Page { tex: TextureId::from_raw_id(RawId::OpenGl(0)), shelves: Vec::new(), top: 0, fill: 0, used: 0 };
        assert_eq!(fit(&mut p, 100, 50), Some((0, 0)));
        assert_eq!(fit(&mut p, 100, 48), Some((100, 0)));
        // a much shorter frame starts a row of its own
        assert_eq!(fit(&mut p, 100, 10), Some((0, 50)));
        // a frame wider than what is left of every row starts a new one
        assert_eq!(fit(&mut p, PAGE - 150, 50), Some((0, 60)));
        // a short frame still goes along a row of its height
        p.top = PAGE - 5;
        assert_eq!(fit(&mut p, 10, 10), Some((100, 50)));
        // a frame taller than every row, with no room for a new one: none
        assert_eq!(fit(&mut p, 10, 100), None);
    }
}
