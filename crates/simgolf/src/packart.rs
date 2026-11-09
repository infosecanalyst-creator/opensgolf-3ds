//! High-resolution art of content packs (docs/MODDING.md, "High-resolution packs and your own art"). The port's own.
//!
//! A pack with `scale = 2` or `4` in its pack.toml, or a sprite given as a PNG sheet, is found by sg_core::vfs::art; this
//! module makes the textures. Like the HD mode (hd.rs) it creates them at the pack's resolution but reports the original's
//! size, so every layout, cut and hit test is unchanged; it works with HD on or off, in Classic and Enhanced. A pack's file
//! comes before the HD pack's upscale (the loaders ask here first), and whatever the pack does not have is loaded as before.
//!
//! With no such pack mounted [`active`] is false and none of this runs, so Classic stays exactly as it was.
use crate::app::App;
use crate::gfx::Gfx;
use crate::ui::Image;
use miniquad::TextureId;
use sg_core::assets::Rgba;
use std::path::Path;

/// True when a mounted pack has high-resolution pictures or sprite sheets. Every hook checks this first.
pub fn active() -> bool {
    sg_core::vfs::art_active()
}

/// The pack's high-resolution picture for a file, with the original's size.
fn picture(path: &Path) -> Option<(Rgba, u32, u32)> {
    let a = sg_core::vfs::art(path).filter(|a| !a.sheet)?;
    let img = sg_core::png::decode_image(&a.data)?;
    Some((img, a.size.0, a.size.1))
}

/// A texture at the picture's resolution: trilinear with mipmaps (it is drawn smaller than its resolution at most window
/// sizes), with the colours spread under the transparent pixels so the filtering does not darken the outlines.
fn texture(g: &mut Gfx, img: &Rgba) -> TextureId {
    if img.px.chunks_exact(4).any(|p| p[3] == 0) {
        g.texture_hd(&crate::gfx::bleed_alpha(img))
    } else {
        g.texture_hd(img)
    }
}

fn finish(g: &mut Gfx, img: &Rgba, w: u32, h: u32) -> Image {
    let tex = texture(g, img);
    crate::hd::track(tex);
    Image { tex: Some(tex), w: w as f32, h: h as f32 }
}

/// Clears the classic key colours of a picture (the pack's PNG has its own transparency; a key colour drawn on purpose is
/// honoured as in the classic loader).
fn key_out(img: &mut Rgba, magenta_key: bool, key_rgb: Option<u32>) {
    for p in img.px.chunks_exact_mut(4) {
        if magenta_key && p[0] == 255 && p[1] == 0 && p[2] == 255 {
            p[3] = 0;
        }
        if let Some(k) = key_rgb {
            if p[0] as u32 == (k >> 16) & 255 && p[1] as u32 == (k >> 8) & 255 && p[2] as u32 == k & 255 {
                p[3] = 0;
            }
        }
    }
}

/// ui::load_pcx from a high-resolution pack.
pub fn load_pcx(g: &mut Gfx, path: &Path, magenta_key: bool, key_rgb: Option<u32>) -> Option<Image> {
    let (mut img, w, h) = picture(path)?;
    key_out(&mut img, magenta_key, key_rgb);
    Some(finish(g, &img, w, h))
}

/// ui::load_pcx_alpha from a high-resolution pack: the PNG's own transparency, times the pack's alpha sheet when it has
/// one at the same size (the original's alpha sheet would not fit the pack's drawing).
pub fn load_pcx_alpha(g: &mut Gfx, path: &Path, alpha: &Path) -> Option<Image> {
    let (mut img, w, h) = picture(path)?;
    if let Some((a, _, _)) = picture(alpha).filter(|(a, _, _)| (a.w, a.h) == (img.w, img.h)) {
        for (c, m) in img.px.chunks_exact_mut(4).zip(a.px.chunks_exact(4)) {
            c[3] = c[3].min(m[0].max(m[1]).max(m[2]));
        }
    }
    key_out(&mut img, true, None);
    Some(finish(g, &img, w, h))
}

/// ui::split_overlay from a high-resolution pack: the pieces are found at the original's size (a pixel there is opaque
/// when any pixel of its block is), and each high-resolution pixel goes with its block's piece.
pub fn split_overlay(g: &mut Gfx, path: &Path, rects: &[(f32, f32, f32, f32)]) -> Option<Vec<Image>> {
    let (mut img, w, h) = picture(path)?;
    key_out(&mut img, true, None);
    let s = (img.w / w.max(1)) as usize;
    let (wu, hu, pw) = (w as usize, h as usize, img.w as usize);
    let mut low = Rgba::new(w, h);
    for (i, q) in img.px.chunks_exact(4).enumerate() {
        if q[3] != 0 {
            low.px[((i / pw / s) * wu + i % pw / s) * 4 + 3] = 255;
        }
    }
    let label = crate::hd::label_pieces(&low);
    let mut out = Vec::new();
    for keep in crate::hd::pieces_in_rects(&label, wu, hu, rects) {
        let mut part = img.clone();
        for (i, q) in part.px.chunks_exact_mut(4).enumerate() {
            if !keep[label[(i / pw / s) * wu + i % pw / s] as usize] {
                q[3] = 0;
            }
        }
        out.push(finish(g, &part, w, h));
    }
    Some(out)
}

/// A terrain texture from a high-resolution pack (App::texture_for).
pub fn terrain_texture(g: &mut Gfx, path: &Path) -> Option<TextureId> {
    let (img, _, _) = picture(path)?;
    Some(g.texture_hd(&img))
}

impl App {
    /// A sprite just loaded from `path` (in the palette of `pal`), replaced by a pack's PNG sheet when the pack has one
    /// (sg_core::hires::sheet_sprite). The sheet is kept for drawing when it is high-resolution or a person's (recoloured
    /// per outfit); the returned sprite has its frames at the original's size.
    pub(crate) fn pack_sheet(&mut self, path: &Path, pal: Option<&Path>, s: sg_core::sprites::Sprite) -> sg_core::sprites::Sprite {
        if !active() {
            return s;
        }
        match sg_core::hires::sheet_sprite(path, pal, &s) {
            None => s,
            Some(Err(e)) => {
                crate::bugreport::warn(&format!("content pack: {}: {e}", path.display()));
                s
            }
            Some(Ok((small, sheet))) => {
                if sheet.scale > 1 || sheet.people {
                    self.pack_art.insert(self.sprites.len(), sheet);
                }
                small
            }
        }
    }

    /// True when sprite `si` is drawn from a high-resolution sheet (the Enhanced effects that redraw the classic frames
    /// leave it alone, as they do in HD).
    pub(crate) fn pack_hires(&self, si: usize) -> bool {
        self.pack_art.get(&si).is_some_and(|s| s.scale > 1)
    }

    /// Frame `i` of sprite `si` from its high-resolution sheet.
    pub(crate) fn pack_sprite_texture(&mut self, g: &mut Gfx, si: usize, i: usize) -> Option<TextureId> {
        let img = self.pack_art.get(&si).filter(|s| s.scale > 1)?.frame(i)?;
        Some(texture(g, &img))
    }

    /// A person's frame from a pack's sheet in the colours of an outfit, cached with the classic ones.
    pub(crate) fn pack_outfit_texture(&mut self, g: &mut Gfx, si: usize, i: usize, o: sg_core::bodies::Outfit) -> Option<TextureId> {
        let scale = self.pack_art.get(&si).filter(|s| s.people)?.scale;
        if let Some(&t) = self.outfit_tex.get(&(si, i, o)) {
            return Some(t);
        }
        if !self.outfit_pals.contains_key(&o) {
            let pal = self.swaps.compose(&o)?;
            self.outfit_pals.insert(o, pal);
        }
        let pal = self.outfit_pals[&o];
        let img = self.pack_art.get(&si)?.outfit_frame(i, &pal, o.female)?;
        // the cache holds as many pixels as the classic one at most
        if self.outfit_tex.len() > 6000 / (scale * scale) as usize {
            for (_, t) in self.outfit_tex.drain() {
                g.ctx.delete_texture(t);
            }
        }
        let t = if scale > 1 { texture(g, &img) } else { g.texture(&img, false) };
        self.outfit_tex.insert((si, i, o), t);
        Some(t)
    }
}
