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
    if crate::hd::active() {
        if let Some(im) = crate::hd::load_pcx(g, path, magenta_key, key_rgb) {
            return Some(im);
        }
    }
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
    crate::hd::track(tex);
    Some(Image { tex: Some(tex), w: img.w as f32, h: img.h as f32 })
}

/// Splits a colour-keyed overlay into the pieces touching each rectangle: every 8-connected run of opaque pixels with at
/// least one pixel inside the rectangle is kept whole. The title menu's highlight art is one picture for every button; this
/// gives each button exactly its own lit shapes instead of a rectangle cut out of the picture.
pub fn split_overlay(g: &mut Gfx, path: &Path, rects: &[(f32, f32, f32, f32)]) -> Vec<Image> {
    if crate::hd::active() {
        if let Some(v) = crate::hd::split_overlay(g, path, rects) {
            return v;
        }
    }
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
            let tex = g.texture(&part, false);
            crate::hd::track(tex);
            Image { tex: Some(tex), w: part.w as f32, h: part.h as f32 }
        })
        .collect()
}

/// One cell of a PCX drawn through the alpha of another cell of its alpha sheet, as the exe's masked blit (0x473f60) does
/// when it pairs a colour sprite with a mask sprite cut elsewhere: the `w` x `h` colour cell at `c` with the alpha cell at
/// `a`. Built from the classic files in either mode (the cell is a small sprite).
pub fn load_pcx_cell(g: &mut Gfx, path: &Path, alpha: &Path, c: (usize, usize), a: (usize, usize), w: usize, h: usize) -> Option<Image> {
    let img = decode_pcx(&sg_core::fsutil::read_file(path)?).ok()?;
    let m = decode_pcx(&sg_core::fsutil::read_file(alpha)?).ok()?;
    let (iw, mw) = (img.w as usize, m.w as usize);
    if c.0 + w > iw || c.1 + h > img.h as usize || a.0 + w > mw || a.1 + h > m.h as usize {
        return None;
    }
    let mut px = vec![0u8; w * h * 4];
    for y in 0..h {
        for x in 0..w {
            let s = ((c.1 + y) * iw + c.0 + x) * 4;
            let q = ((a.1 + y) * mw + a.0 + x) * 4;
            let d = (y * w + x) * 4;
            px[d..d + 3].copy_from_slice(&img.px[s..s + 3]);
            px[d + 3] = m.px[q].max(m.px[q + 1]).max(m.px[q + 2]);
        }
    }
    let cell = sg_core::assets::Rgba { w: w as u32, h: h as u32, px };
    let tex = g.texture(&cell, false);
    crate::hd::track(tex);
    Some(Image { tex: Some(tex), w: w as f32, h: h as f32 })
}

/// Loads a PCX with a separate alpha PCX of the same size (the interface's `_A` / `_alpha` files: white opaque, black clear).
pub fn load_pcx_alpha(g: &mut Gfx, path: &Path, alpha: &Path) -> Option<Image> {
    if crate::hd::active() {
        if let Some(im) = crate::hd::load_pcx_alpha(g, path, alpha) {
            return Some(im);
        }
    }
    let d = sg_core::fsutil::read_file(path)?;
    let mut img = decode_pcx(&d).ok()?;
    if let Some(a) = sg_core::fsutil::read_file(alpha).and_then(|d| decode_pcx(&d).ok()) {
        if a.w == img.w && a.h == img.h {
            for (p, q) in img.px.as_chunks_mut::<4>().0.iter_mut().zip(a.px.as_chunks::<4>().0) {
                // the magenta key holds even where the alpha sheet is solid (the end of the Amenities design strip)
                let key = p[0] >= 240 && p[2] >= 240 && p[1] <= 8;
                p[3] = if key { 0 } else { q[0].max(q[1]).max(q[2]) };
            }
        }
    }
    let tex = g.texture(&img, false);
    crate::hd::track(tex);
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

/// The exe's typefaces (fonts made at start up, 0x45bd83): Klepto ITC (klepto__.ttf) only on the title screens, Manual SSi
/// Bold (manu3_.ttf) for most text, and Windows' Arial Bold for the small print. Arial is not on the disc: the system's copy
/// is used when there is one, else the bundled Liberation Sans Bold, which has Arial's metrics.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Face {
    Klepto,
    Manual,
    Arial,
    /// The info screens' pair (font objects 0x821020 and 0x821ee8): titles in Klepto 24, everything else in Manual SSi 14.
    Info,
}

/// One of the exe's font objects: a typeface at a size. The size is what the exe hands its graphics library (jgl.dll, font
/// create at 0x1003be80): it goes into a LOGFONT as lfHeight = -size with lfWeight 0, so it is GDI's character height, the em
/// square, in pixels; fontdue's pixel size is the same em measure. The exe passes the top of the text: its draw (0x4767a0)
/// adds the font's tmAscent - tmInternalLeading (kept at object +0x10) to y and TextOut runs with TA_BASELINE, so the
/// baseline sits `base()` pixels below the y the exe gives. A line is tmHeight + tmExternalLeading (0x477580).
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct Fnt {
    pub face: Face,
    pub px: f32,
}

impl Fnt {
    /// Pixels from the exe's text y (the top) down to the baseline.
    pub fn base(self) -> f32 {
        Font::get(self.face).map(|f| f.base(self.px)).unwrap_or(self.px * 0.8)
    }
    /// The exe's line height (0x477580).
    pub fn line(self) -> f32 {
        Font::get(self.face).map(|f| f.line(self.px)).unwrap_or(self.px * 1.25)
    }
    pub fn width(self, s: &str) -> f32 {
        Font::get(self.face).map(|f| f.width(s, self.px)).unwrap_or(s.len() as f32 * self.px * 0.5)
    }
}

// The start up set (0x45bd83), by font object.
// 0x51b320: Arial Bold 9, made but never selected.
/// 0x519fd8: Arial Bold 10, the small print (card meters, face strip, badge).
pub const F_ARIAL10: Fnt = Fnt { face: Face::Arial, px: 10.0 };
/// 0x51b360: Manual SSi Bold 15, the body text.
pub const F_MANUAL15: Fnt = Fnt { face: Face::Manual, px: 15.0 };

/// Palette colour 1, the shadow of the exe's shadowed text calls (0x404ad0, 0x404bc0): the Windows system palette's dark red
/// (0x800000), not black. Footage of the original shows it plainly under the cash pill's green digits, the "Hole 2 / 256
/// yards / Par 4" label and "Press 'h' to open hole" (a red fringe under white and green text, which the video's chroma
/// cannot make from black). The course badge's name and date have a black shadow instead (hud_ui).
pub const SHADOW_1: [f32; 4] = [128.0 / 255.0, 0.0, 0.0, 1.0];
/// 0x519928: Manual SSi Bold 20, the headings and the default font (0x83ad44).
pub const F_MANUAL20: Fnt = Fnt { face: Face::Manual, px: 20.0 };
/// 0x51a028: Klepto ITC 18.
pub const F_KLEPTO18: Fnt = Fnt { face: Face::Klepto, px: 18.0 };
/// 0x519a40: Klepto ITC 24.
pub const F_KLEPTO24: Fnt = Fnt { face: Face::Klepto, px: 24.0 };
/// 0x519948: Manual SSi Bold 24.
pub const F_MANUAL24: Fnt = Fnt { face: Face::Manual, px: 24.0 };
// 0x51b340 Comic Sans MS Bold italic at 40 * width / 320 and 0x519968 Times New Roman Bold at 7 * width / 320 are made too;
// neither face is on the disc and the port draws nothing in them.
// The info screens' set (0x44be1f).
/// 0x821020: Klepto ITC 24, the info screens' titles.
pub const F_INFO_TITLE: Fnt = Fnt { face: Face::Klepto, px: 24.0 };
/// 0x821f08: Manual SSi Bold 20.
pub const F_INFO20: Fnt = Fnt { face: Face::Manual, px: 20.0 };
// 0x821ec8: Manual SSi Bold 18, made but never selected.
/// 0x821f28: Manual SSi Bold 16.
pub const F_INFO16: Fnt = Fnt { face: Face::Manual, px: 16.0 };
/// 0x821ee8: Manual SSi Bold 14, the info screens' body.
pub const F_INFO14: Fnt = Fnt { face: Face::Manual, px: 14.0 };

static FONTS: [std::sync::OnceLock<Font>; 3] = [std::sync::OnceLock::new(), std::sync::OnceLock::new(), std::sync::OnceLock::new()];

thread_local! {
    /// The face a screen asked for; None picks by size (Arial below 12 pixels, Manual SSi above).
    static FACE: std::cell::Cell<Option<Face>> = const { std::cell::Cell::new(None) };
}

/// Draws the following text in `f` (None: the default by size), as the exe selects a font object before its text.
pub fn set_face(f: Option<Face>) {
    FACE.with(|c| c.set(f));
}

fn face_for(size: f32) -> Face {
    match FACE.with(|c| c.get()) {
        // the info set holds Klepto only at 24 (0x821020); its 20, 18, 16 and 14 are Manual SSi
        Some(Face::Info) => {
            if size >= 24.0 {
                Face::Klepto
            } else {
                Face::Manual
            }
        }
        Some(f) => f,
        None => {
            if size < 12.0 {
                Face::Arial
            } else {
                Face::Manual
            }
        }
    }
}

/// Liberation Sans Bold 2.1.5 (SIL Open Font License 1.1, see fonts/LiberationSans-OFL.txt), standing in for Arial Bold.
const ARIAL_STANDIN: &[u8] = include_bytes!("../fonts/LiberationSans-Bold.ttf");

#[derive(Default)]
pub struct Font {
    tex: Option<TextureId>,
    glyphs: Vec<Glyph>,
    vm: VMetrics,
}

/// The vertical metrics GDI builds its TEXTMETRIC from, in font units: units per em, the OS/2 table's usWinAscent and
/// usWinDescent, and the hhea table's ascender, descender and line gap (for tmExternalLeading).
#[derive(Clone, Copy, Debug, Default)]
struct VMetrics {
    upem: f32,
    win_asc: f32,
    win_desc: f32,
    hhea_asc: f32,
    hhea_desc: f32,
    hhea_gap: f32,
}

impl VMetrics {
    /// Reads the head, hhea and OS/2 tables of a TrueType file.
    fn parse(d: &[u8]) -> Option<VMetrics> {
        let u16_at = |o: usize| d.get(o..o + 2).map(|b| u16::from_be_bytes([b[0], b[1]]));
        let i16_at = |o: usize| u16_at(o).map(|v| v as i16 as f32);
        let n = u16_at(4)? as usize;
        let table = |tag: &[u8; 4]| {
            (0..n).find_map(|i| {
                let r = 12 + 16 * i;
                (d.get(r..r + 4)? == tag).then(|| d.get(r + 8..r + 12).map(|b| u32::from_be_bytes([b[0], b[1], b[2], b[3]]) as usize))?
            })
        };
        let (head, hhea, os2) = (table(b"head")?, table(b"hhea")?, table(b"OS/2")?);
        Some(VMetrics {
            upem: u16_at(head + 18)? as f32,
            hhea_asc: i16_at(hhea + 4)?,
            hhea_desc: i16_at(hhea + 6)?,
            hhea_gap: i16_at(hhea + 8)?,
            win_asc: u16_at(os2 + 74)? as f32,
            win_desc: u16_at(os2 + 76)? as f32,
        })
    }

    /// GDI's tmAscent, tmDescent and tmExternalLeading for an em of `px` pixels.
    fn tm(&self, px: f32) -> (f32, f32, f32) {
        let k = px / self.upem.max(1.0);
        let ext = (self.hhea_gap - ((self.win_asc + self.win_desc) - (self.hhea_asc - self.hhea_desc))).max(0.0);
        ((self.win_asc * k).round(), (self.win_desc * k).round(), (ext * k).round())
    }
}

impl Font {
    /// Bakes the Latin-1 glyphs of the three faces at a fixed size, each into its own texture, once at start up: Klepto and
    /// Manual SSi from the game folder, Arial Bold from the system or the bundled stand-in.
    pub fn load(g: &mut Gfx, game_font: impl Fn(&str) -> std::path::PathBuf) -> bool {
        let read = |p: &Path| sg_core::fsutil::read_file(p);
        let klepto = read(&game_font("KLEPTO__.TTF"));
        let manual = read(&game_font("manu3_.TTF"));
        let arial = ["C:/Windows/Fonts/arialbd.ttf", "/Library/Fonts/Arial Bold.ttf", "/System/Library/Fonts/Supplementary/Arial Bold.ttf"]
            .iter()
            .find_map(|p| read(Path::new(p)))
            .unwrap_or_else(|| ARIAL_STANDIN.to_vec());
        for (i, ttf) in [klepto, manual, Some(arial)].into_iter().enumerate() {
            if let Some(f) = ttf.and_then(|b| Self::load_inner(g, b)) {
                let _ = FONTS[i].set(f);
            }
        }
        FONTS[0].get().is_some()
    }

    /// The face's font, or the first one loaded when it is missing.
    fn get(face: Face) -> Option<&'static Font> {
        FONTS[face as usize].get().or_else(|| FONTS.iter().find_map(|f| f.get()))
    }

    fn load_inner(g: &mut Gfx, ttf: Vec<u8>) -> Option<Font> {
        let vm = VMetrics::parse(&ttf);
        let font = fontdue::Font::from_bytes(ttf, fontdue::FontSettings::default()).ok()?;
        let vm = vm.unwrap_or_else(|| {
            let lm = font.horizontal_line_metrics(1000.0);
            let (a, d, gap) = lm.map(|m| (m.ascent, -m.descent, m.line_gap)).unwrap_or((800.0, 200.0, 0.0));
            VMetrics { upem: 1000.0, win_asc: a, win_desc: d, hhea_asc: a, hhea_desc: -d, hhea_gap: gap }
        });
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
        Some(Font { tex: Some(g.texture(&img, false)), glyphs, vm })
    }

    /// Top of the exe's text to its baseline: tmAscent - tmInternalLeading, which is the em less tmDescent.
    fn base(&self, px: f32) -> f32 {
        let (_, d, _) = self.vm.tm(px);
        px.round() - d
    }

    /// tmHeight + tmExternalLeading.
    fn line(&self, px: f32) -> f32 {
        let (a, d, e) = self.vm.tm(px);
        a + d + e
    }

    /// A glyph's advance at `px`: GDI steps the pen by whole pixels.
    fn advance(gl: &Glyph, px: f32) -> f32 {
        (gl.advance * px / BAKE_PX).round()
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
        s.chars().filter_map(|c| self.glyph(c)).map(|g| Self::advance(g, size)).sum::<f32>()
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
        self.image_part_tint(g, im, dx, dy, sx, sy, sw, sh, [1.0, 1.0, 1.0, 1.0]);
    }

    /// `image_part` with its colours multiplied by `c` (a dimmed or faded copy).
    #[allow(clippy::too_many_arguments)]
    pub fn image_part_tint(&self, g: &mut Gfx, im: &Image, dx: f32, dy: f32, sx: f32, sy: f32, sw: f32, sh: f32, c: [f32; 4]) {
        self.image_scaled_tint(g, im, (dx, dy, sw, sh), (sx, sy, sw, sh), c);
    }

    /// Sub-rectangle `src` (x, y, w, h) of an image stretched over `dst`.
    pub fn image_scaled(&self, g: &mut Gfx, im: &Image, dst: (f32, f32, f32, f32), src: (f32, f32, f32, f32)) {
        self.image_scaled_tint(g, im, dst, src, [1.0, 1.0, 1.0, 1.0]);
    }

    /// `image_scaled` with its colours multiplied by `c`.
    pub fn image_scaled_tint(&self, g: &mut Gfx, im: &Image, dst: (f32, f32, f32, f32), src: (f32, f32, f32, f32), c: [f32; 4]) {
        let Some(tex) = im.tex else { return };
        let (dx, dy, dw, dh) = dst;
        let (sx, sy, sw, sh) = src;
        let (u0, v0, u1, v1) = (sx / im.w, sy / im.h, (sx + sw) / im.w, (sy + sh) / im.h);
        g.quad(
            Mode::Flat,
            Some(tex),
            &self.u,
            [
                Vert::new(dx, dy, 0.0, u0, v0).col(c),
                Vert::new(dx + dw, dy, 0.0, u1, v0).col(c),
                Vert::new(dx + dw, dy + dh, 0.0, u1, v1).col(c),
                Vert::new(dx, dy + dh, 0.0, u0, v1).col(c),
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

    /// A filled diamond centred on (x, y), reaching `hw` pixels left and right and `hh` up and down.
    pub fn diamond(&self, g: &mut Gfx, x: f32, y: f32, hw: f32, hh: f32, c: [f32; 4]) {
        g.quad(
            Mode::Flat,
            None,
            &self.u,
            [
                Vert::new(x - hw, y, 0.0, 0.0, 0.0).col(c),
                Vert::new(x, y - hh, 0.0, 0.0, 0.0).col(c),
                Vert::new(x + hw, y, 0.0, 0.0, 0.0).col(c),
                Vert::new(x, y + hh, 0.0, 0.0, 0.0).col(c),
            ],
        );
    }

    /// A straight line `w` pixels wide.
    #[allow(clippy::too_many_arguments)]
    pub fn line(&self, g: &mut Gfx, x0: f32, y0: f32, x1: f32, y1: f32, w: f32, c: [f32; 4]) {
        let (dx, dy) = (x1 - x0, y1 - y0);
        let l = (dx * dx + dy * dy).sqrt().max(1e-4);
        let (nx, ny) = (-dy / l * w * 0.5, dx / l * w * 0.5);
        g.quad(
            Mode::Flat,
            None,
            &self.u,
            [
                Vert::new(x0 + nx, y0 + ny, 0.0, 0.0, 0.0).col(c),
                Vert::new(x1 + nx, y1 + ny, 0.0, 0.0, 0.0).col(c),
                Vert::new(x1 - nx, y1 - ny, 0.0, 0.0, 0.0).col(c),
                Vert::new(x0 - nx, y0 - ny, 0.0, 0.0, 0.0).col(c),
            ],
        );
    }

    /// Text with the baseline at y. Sizes are in virtual pixels.
    pub fn text(&self, g: &mut Gfx, x: f32, y: f32, s: &str, size: f32, c: [f32; 4]) {
        self.text_in(g, face_for(size), x, y, s, size, c);
    }

    /// Text in an exe font object as the exe's left aligned call (0x4049d0) puts it: `y` is the top.
    pub fn put(&self, g: &mut Gfx, f: Fnt, x: f32, y: f32, s: &str, c: [f32; 4]) {
        self.text_in(g, f.face, x, y + f.base(), s, f.px, c);
    }

    /// The exe's centred call (0x404b70, 0x477da0): starts at x - width / 2, rounded down to a pixel; `y` is the top.
    pub fn put_centered(&self, g: &mut Gfx, f: Fnt, cx: f32, y: f32, s: &str, c: [f32; 4]) {
        self.put(g, f, cx - (f.width(s) / 2.0).floor(), y, s, c);
    }

    /// The exe's right aligned call (0x478140): ends at x; `y` is the top.
    pub fn put_right(&self, g: &mut Gfx, f: Fnt, x: f32, y: f32, s: &str, c: [f32; 4]) {
        self.put(g, f, x - f.width(s), y, s, c);
    }

    /// The exe's shadowed calls (0x404ad0 left, 0x404bc0 centred): palette colour 1 (`SHADOW_1`) one pixel below, then the
    /// text.
    pub fn put_shadowed(&self, g: &mut Gfx, f: Fnt, x: f32, y: f32, s: &str, c: [f32; 4], centred: bool) {
        let x = if centred { x - (f.width(s) / 2.0).floor() } else { x };
        self.put(g, f, x, y + 1.0, s, [SHADOW_1[0], SHADOW_1[1], SHADOW_1[2], c[3]]);
        self.put(g, f, x, y, s, c);
    }

    #[allow(clippy::too_many_arguments)]
    fn text_in(&self, g: &mut Gfx, face: Face, x: f32, y: f32, s: &str, size: f32, c: [f32; 4]) {
        let Some(f) = Font::get(face) else { return };
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
            pen += Font::advance(gl, size);
        }
    }
    pub fn text_centered(&self, g: &mut Gfx, cx: f32, y: f32, s: &str, size: f32, c: [f32; 4]) {
        // as the exe's centred call: half the width rounded down
        self.text(g, cx - (text_width(s, size) / 2.0).floor(), y, s, size, c);
    }
}

pub fn text_width(s: &str, size: f32) -> f32 {
    Font::get(face_for(size)).map(|f| f.width(s, size)).unwrap_or(s.len() as f32 * size * 0.5)
}

/// The baseline of text whose top the exe puts at `y`, in the face `text` would pick for `size` (see `Fnt`).
pub fn top(y: f32, size: f32) -> f32 {
    y + Fnt { face: face_for(size), px: size }.base()
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
