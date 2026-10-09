//! Screen-space drawing: images from the disc's Interface folder, and text in the game's own font (KLEPTO__.TTF from the disc,
//! rasterised at run time). Everything is drawn in a virtual 800x600 space (the original's UI resolution), scaled to fit the window
//! with black bars.
use crate::gfx::{Gfx, Mat4, Mode, Uniforms, Vert};
use miniquad::TextureId;
use sg_core::assets::decode_pcx;
use std::path::Path;

#[derive(Clone, Copy, Debug, Default)]
pub struct Image {
    pub tex: Option<TextureId>,
    pub w: f32,
    pub h: f32,
}

/// Loads a PCX from the disc. With `magenta_key`, pure magenta pixels become transparent (the original's colour key); `key_rgb` is
/// another colour key as 0xRRGGBB.
pub fn load_pcx(g: &mut Gfx, path: &Path, magenta_key: bool, key_rgb: Option<u32>) -> Option<Image> {
    let d = sg_core::fsutil::read_file(path)?;
    let mut img = decode_pcx(&d).ok()?;
    for p in img.px.as_chunks_mut::<4>().0 {
        if magenta_key && p[0] == 255 && p[1] == 0 && p[2] == 255 {
            p[3] = 0;
        }
        if let Some(k) = key_rgb {
            if p[0] as u32 == (k >> 16) & 255 && p[1] as u32 == (k >> 8) & 255 && p[2] as u32 == k & 255 {
                p[3] = 0;
            }
        }
    }
    let tex = g.texture(&img, false);
    Some(Image { tex: Some(tex), w: img.w as f32, h: img.h as f32 })
}

/// Splits a colour-keyed overlay into the pieces touching each rectangle: every 8-connected run of opaque pixels with at
/// least one pixel inside the rectangle is kept whole. The title menu's highlight art is one picture for every button; this
/// gives each button exactly its own lit shapes instead of a rectangle cut out of the picture.
pub fn split_overlay(g: &mut Gfx, path: &Path, rects: &[(f32, f32, f32, f32)]) -> Vec<Image> {
    let Some(mut img) = sg_core::fsutil::read_file(path).and_then(|d| decode_pcx(&d).ok()) else { return Vec::new() };
    for p in img.px.as_chunks_mut::<4>().0 {
        if p[0] == 255 && p[1] == 0 && p[2] == 255 {
            p[3] = 0;
        }
    }
    let (w, h) = (img.w as usize, img.h as usize);
    let mut label = vec![0u32; w * h];
    let mut next = 0u32;
    let mut stack = Vec::new();
    for start in 0..w * h {
        if img.px[start * 4 + 3] == 0 || label[start] != 0 {
            continue;
        }
        next += 1;
        label[start] = next;
        stack.push(start);
        while let Some(i) = stack.pop() {
            let (x, y) = ((i % w) as i32, (i / w) as i32);
            for dy in -1..=1 {
                for dx in -1..=1 {
                    let (nx, ny) = (x + dx, y + dy);
                    if nx < 0 || ny < 0 || nx >= w as i32 || ny >= h as i32 {
                        continue;
                    }
                    let j = ny as usize * w + nx as usize;
                    if label[j] == 0 && img.px[j * 4 + 3] != 0 {
                        label[j] = next;
                        stack.push(j);
                    }
                }
            }
        }
    }
    rects
        .iter()
        .map(|&(rx, ry, rw, rh)| {
            let mut keep = vec![false; next as usize + 1];
            for y in (ry.max(0.0) as usize)..((ry + rh) as usize).min(h) {
                for x in (rx.max(0.0) as usize)..((rx + rw) as usize).min(w) {
                    keep[label[y * w + x] as usize] = true;
                }
            }
            keep[0] = false;
            let mut part = img.clone();
            for (p, &l) in part.px.as_chunks_mut::<4>().0.iter_mut().zip(&label) {
                if !keep[l as usize] {
                    p[3] = 0;
                }
            }
            Image { tex: Some(g.texture(&part, false)), w: part.w as f32, h: part.h as f32 }
        })
        .collect()
}

/// Loads a PCX with a separate alpha PCX of the same size (the interface's `_A` / `_alpha` files: white opaque, black clear).
pub fn load_pcx_alpha(g: &mut Gfx, path: &Path, alpha: &Path) -> Option<Image> {
    let d = sg_core::fsutil::read_file(path)?;
    let mut img = decode_pcx(&d).ok()?;
    if let Some(a) = sg_core::fsutil::read_file(alpha).and_then(|d| decode_pcx(&d).ok()) {
        if a.w == img.w && a.h == img.h {
            for (p, q) in img.px.as_chunks_mut::<4>().0.iter_mut().zip(a.px.as_chunks::<4>().0) {
                p[3] = q[0].max(q[1]).max(q[2]);
            }
        }
    }
    let tex = g.texture(&img, false);
    Some(Image { tex: Some(tex), w: img.w as f32, h: img.h as f32 })
}

#[derive(Clone, Copy, Debug, Default)]
struct Glyph {
    /// Atlas rectangle in pixels.
    ax: f32,
    ay: f32,
    aw: f32,
    ah: f32,
    /// Offset of the bitmap's top-left from the pen position on the baseline, at the bake size.
    xoff: f32,
    yoff: f32,
    advance: f32,
}

const FIRST: u32 = 32;
const COUNT: u32 = 224;
const BAKE_PX: f32 = 32.0;
const ATLAS: usize = 512;

static FONT: std::sync::OnceLock<Font> = std::sync::OnceLock::new();

#[derive(Default)]
pub struct Font {
    tex: Option<TextureId>,
    glyphs: Vec<Glyph>,
}

impl Font {
    /// Bakes the Latin-1 glyphs at a fixed size into one texture. The game has one font, loaded once at start up.
    pub fn load(g: &mut Gfx, ttf_path: &Path) -> bool {
        match Self::load_inner(g, ttf_path) {
            Some(f) => FONT.set(f).is_ok() || FONT.get().is_some(),
            None => false,
        }
    }
    pub fn get() -> Option<&'static Font> {
        FONT.get()
    }

    fn load_inner(g: &mut Gfx, ttf_path: &Path) -> Option<Font> {
        let ttf = sg_core::fsutil::read_file(ttf_path)?;
        let font = fontdue::Font::from_bytes(ttf, fontdue::FontSettings::default()).ok()?;
        let mut atlas = vec![0u8; ATLAS * ATLAS * 4];
        let mut glyphs = Vec::with_capacity(COUNT as usize);
        let (mut x, mut y, mut row_h) = (1usize, 1usize, 0usize);
        for cp in FIRST..FIRST + COUNT {
            let ch = char::from_u32(cp).unwrap_or('?');
            let (m, bmp) = font.rasterize(ch, BAKE_PX);
            if x + m.width + 1 >= ATLAS {
                x = 1;
                y += row_h + 1;
                row_h = 0;
            }
            if y + m.height + 1 >= ATLAS {
                return None;
            }
            for j in 0..m.height {
                for i in 0..m.width {
                    let o = ((y + j) * ATLAS + x + i) * 4;
                    atlas[o..o + 3].copy_from_slice(&[255, 255, 255]);
                    atlas[o + 3] = bmp[j * m.width + i];
                }
            }
            glyphs.push(Glyph {
                ax: x as f32,
                ay: y as f32,
                aw: m.width as f32,
                ah: m.height as f32,
                xoff: m.xmin as f32,
                yoff: -(m.ymin as f32 + m.height as f32),
                advance: m.advance_width,
            });
            x += m.width + 1;
            row_h = row_h.max(m.height);
        }
        let img = sg_core::assets::Rgba { w: ATLAS as u32, h: ATLAS as u32, px: atlas };
        Some(Font { tex: Some(g.texture(&img, false)), glyphs })
    }

    fn glyph(&self, c: char) -> Option<&Glyph> {
        let cp = c as u32;
        if (FIRST..FIRST + COUNT).contains(&cp) {
            self.glyphs.get((cp - FIRST) as usize)
        } else {
            self.glyphs.get(('?' as u32 - FIRST) as usize)
        }
    }

    pub fn width(&self, s: &str, size: f32) -> f32 {
        let k = size / BAKE_PX;
        s.chars().filter_map(|c| self.glyph(c)).map(|g| g.advance).sum::<f32>() * k
    }
}

/// The virtual to window mapping of the current screen.
#[derive(Clone, Copy, Debug)]
pub struct View {
    pub scale: f32,
    pub ox: f32,
    pub oy: f32,
}

impl Default for View {
    fn default() -> Self {
        View { scale: 1.0, ox: 0.0, oy: 0.0 }
    }
}

impl View {
    pub fn to_virtual(self, px: f32, py: f32) -> (f32, f32) {
        ((px - self.ox) / self.scale, (py - self.oy) / self.scale)
    }
}

/// A 2D drawing session over the 800x600 virtual screen.
pub struct Screen {
    pub view: View,
    u: Uniforms,
}

impl Screen {
    /// Orthographic projection for 800x600 virtual units in a draw_w x draw_h drawable.
    pub fn new(draw_w: f32, draw_h: f32) -> Screen {
        let scale = (draw_w / 800.0).min(draw_h / 600.0);
        let view = View { scale, ox: (draw_w - 800.0 * scale) * 0.5, oy: (draw_h - 600.0 * scale) * 0.5 };
        let proj = Mat4::ortho(-view.ox / scale, (draw_w - view.ox) / scale, (draw_h - view.oy) / scale, -view.oy / scale, -1.0, 1.0);
        Screen { view, u: Uniforms::flat(&proj, &Mat4::identity()) }
    }

    /// Sub-rectangle (sx, sy, sw, sh) of an image at its own size, top-left at (dx, dy).
    pub fn image_part(&self, g: &mut Gfx, im: &Image, dx: f32, dy: f32, sx: f32, sy: f32, sw: f32, sh: f32) {
        self.image_scaled(g, im, (dx, dy, sw, sh), (sx, sy, sw, sh));
    }
    /// Sub-rectangle `src` (x, y, w, h) of an image stretched over `dst`.
    pub fn image_scaled(&self, g: &mut Gfx, im: &Image, dst: (f32, f32, f32, f32), src: (f32, f32, f32, f32)) {
        let Some(tex) = im.tex else { return };
        let (dx, dy, dw, dh) = dst;
        let (sx, sy, sw, sh) = src;
        let (u0, v0, u1, v1) = (sx / im.w, sy / im.h, (sx + sw) / im.w, (sy + sh) / im.h);
        g.quad(
            Mode::Flat,
            Some(tex),
            &self.u,
            [
                Vert::new(dx, dy, 0.0, u0, v0),
                Vert::new(dx + dw, dy, 0.0, u1, v0),
                Vert::new(dx + dw, dy + dh, 0.0, u1, v1),
                Vert::new(dx, dy + dh, 0.0, u0, v1),
            ],
        );
    }
    pub fn image(&self, g: &mut Gfx, im: &Image, dx: f32, dy: f32) {
        self.image_part(g, im, dx, dy, 0.0, 0.0, im.w, im.h);
    }
    pub fn fill(&self, g: &mut Gfx, x: f32, y: f32, w: f32, h: f32, c: [f32; 4]) {
        g.quad(
            Mode::Flat,
            None,
            &self.u,
            [
                Vert::new(x, y, 0.0, 0.0, 0.0).col(c),
                Vert::new(x + w, y, 0.0, 0.0, 0.0).col(c),
                Vert::new(x + w, y + h, 0.0, 0.0, 0.0).col(c),
                Vert::new(x, y + h, 0.0, 0.0, 0.0).col(c),
            ],
        );
    }

    /// Text with the baseline at y. Sizes are in virtual pixels.
    pub fn text(&self, g: &mut Gfx, x: f32, y: f32, s: &str, size: f32, c: [f32; 4]) {
        let Some(f) = Font::get() else { return };
        let Some(tex) = f.tex else { return };
        let k = size / BAKE_PX;
        let mut pen = x;
        let a = ATLAS as f32;
        for ch in s.chars() {
            let Some(gl) = f.glyph(ch) else { continue };
            if gl.aw > 0.0 {
                let (x0, y0) = (pen + gl.xoff * k, y + gl.yoff * k);
                let (x1, y1) = (x0 + gl.aw * k, y0 + gl.ah * k);
                let (u0, v0, u1, v1) = (gl.ax / a, gl.ay / a, (gl.ax + gl.aw) / a, (gl.ay + gl.ah) / a);
                g.quad(
                    Mode::Flat,
                    Some(tex),
                    &self.u,
                    [
                        Vert::new(x0, y0, 0.0, u0, v0).col(c),
                        Vert::new(x1, y0, 0.0, u1, v0).col(c),
                        Vert::new(x1, y1, 0.0, u1, v1).col(c),
                        Vert::new(x0, y1, 0.0, u0, v1).col(c),
                    ],
                );
            }
            pen += gl.advance * k;
        }
    }
    pub fn text_centered(&self, g: &mut Gfx, cx: f32, y: f32, s: &str, size: f32, c: [f32; 4]) {
        self.text(g, cx - text_width(s, size) * 0.5, y, s, size, c);
    }
}

pub fn text_width(s: &str, size: f32) -> f32 {
    Font::get().map(|f| f.width(s, size)).unwrap_or(s.len() as f32 * size * 0.5)
}

pub fn rgb(r: f32, g: f32, b: f32) -> [f32; 4] {
    [r, g, b, 1.0]
}
pub fn rgba(r: f32, g: f32, b: f32, a: f32) -> [f32; 4] {
    [r, g, b, a]
}

/// Word wrap to a width in virtual pixels.
pub fn wrap_text(s: &str, size: f32, max_w: f32) -> Vec<String> {
    let mut out = Vec::new();
    let mut cur = String::new();
    for word in s.split_whitespace() {
        let t = if cur.is_empty() { word.to_string() } else { format!("{cur} {word}") };
        if text_width(&t, size) > max_w && !cur.is_empty() {
            out.push(std::mem::replace(&mut cur, word.to_string()));
        } else {
            cur = t;
        }
    }
    if !cur.is_empty() {
        out.push(cur);
    }
    out
}

/// "§1,234,567" (the game's currency sign).
pub fn money(v: i64) -> String {
    format!("{}\u{a7}{}", if v < 0 { "-" } else { "" }, group(v.unsigned_abs()))
}

pub fn group(n: u64) -> String {
    let d = n.to_string();
    let mut out = String::new();
    for (k, c) in d.chars().enumerate() {
        if k > 0 && (d.len() - k).is_multiple_of(3) {
            out.push(',');
        }
        out.push(c);
    }
    out
}

#[cfg(test)]
mod tests {
    #[test]
    fn money_format() {
        assert_eq!(super::money(1234567), "\u{a7}1,234,567");
        assert_eq!(super::money(-500), "-\u{a7}500");
    }
}
