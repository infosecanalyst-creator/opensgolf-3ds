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
    if crate::packart::active() {
        if let Some(im) = crate::packart::load_pcx(g, path, magenta_key, key_rgb) {
            return Some(im);
        }
    }
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
    if crate::packart::active() {
        if let Some(v) = crate::packart::split_overlay(g, path, rects) {
            return v;
        }
    }
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
    if crate::packart::active() {
        if let Some(im) = crate::packart::load_pcx_alpha(g, path, alpha) {
            return Some(im);
        }
    }
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

/// A character's place in the baked glyph sets: Latin-1, then Latin Extended-A (Central European letters such as
/// Hungarian ő and ű) and the typographic quotes, dashes and ellipsis (`PUNCT`); anything else draws as '?' (issue #153).
fn glyph_index(c: char) -> usize {
    let cp = c as u32;
    if (FIRST..FIRST + COUNT).contains(&cp) {
        (cp - FIRST) as usize
    } else if (LATIN_EXT_A..LATIN_EXT_A + 128).contains(&cp) {
        (COUNT + cp - LATIN_EXT_A) as usize
    } else if let Some(k) = PUNCT.iter().position(|&p| p == c) {
        COUNT as usize + 128 + k
    } else {
        ('?' as u32 - FIRST) as usize
    }
}

/// The character baked at place `i` of the glyph sets (the inverse of `glyph_index`).
fn glyph_char(i: usize) -> char {
    let i = i as u32;
    let cp = if i < COUNT {
        FIRST + i
    } else if i < COUNT + 128 {
        LATIN_EXT_A + i - COUNT
    } else {
        return PUNCT.get((i - COUNT - 128) as usize).copied().unwrap_or('?');
    };
    char::from_u32(cp).unwrap_or('?')
}

/// Where a face has no glyph for one of the added characters: the plain letter or mark it is drawn from (ő as o, „ as ").
fn plain_char(c: char) -> char {
    let cp = c as u32;
    if (LATIN_EXT_A..LATIN_EXT_A + 128).contains(&cp) {
        return LATIN_EXT_A_PLAIN.as_bytes()[(cp - LATIN_EXT_A) as usize] as char;
    }
    match c {
        '\u{2018}' | '\u{2019}' => '\'',
        '\u{201A}' => ',',
        '\u{201C}' | '\u{201D}' | '\u{201E}' => '"',
        '\u{2013}' | '\u{2014}' => '-',
        '\u{2026}' => '.',
        _ => '?',
    }
}

/// The double-acute letters of Hungarian and the face's own acute letter and plain letter each is made from.
const DOUBLE_ACUTE: [(char, char, char); 4] = [('Ő', 'Ó', 'O'), ('ő', 'ó', 'o'), ('Ű', 'Ú', 'U'), ('ű', 'ú', 'u')];

/// Glyph `i` of the sets, `ch`, rasterised at `px`: see `glyph_source`. Hungarian's ő Ő ű Ű are made from the face's own
/// ó Ó ú Ú with the accent drawn twice, so they match the face's other letters: the original game's fonts have no proper
/// glyphs for them (ő blank, Ő some other symbol) and Arial's stand out among them (issue #153). An added character the
/// face names but draws nothing for (an empty slot in an old font) is taken from the fallback as one it does not have.
fn raster_glyph(face: &fontdue::Font, fallback: Option<&fontdue::Font>, i: usize, ch: char, px: f32) -> (fontdue::Metrics, Vec<u8>) {
    if i >= COUNT as usize {
        if let Some(g) = DOUBLE_ACUTE.iter().find(|d| d.0 == ch).and_then(|&(_, acute, plain)| double_acute(face, acute, plain, px)) {
            return g;
        }
    }
    let (src, c) = glyph_source(face, fallback, i, ch);
    let (m, bmp) = src.rasterize(c, px);
    let blank = !c.is_whitespace() && bmp.iter().all(|&a| a == 0);
    if i >= COUNT as usize && blank && std::ptr::eq(src, face) {
        return match fallback.filter(|f| f.has_glyph(ch)) {
            Some(f) => f.rasterize(ch, px),
            None => face.rasterize(plain_char(ch), px),
        };
    }
    (m, bmp)
}

/// A double-acute letter made from the face's acute one (`acute`, ó) and plain one (`plain`, o): the rows of `acute` above
/// the top of `plain` are its accent, drawn twice side by side over the letter. None when the face lacks either or the
/// accent cannot be found.
fn double_acute(face: &fontdue::Font, acute: char, plain: char, px: f32) -> Option<(fontdue::Metrics, Vec<u8>)> {
    if !face.has_glyph(acute) || !face.has_glyph(plain) {
        return None;
    }
    let (am, bmp) = face.rasterize(acute, px);
    let (pm, _) = face.rasterize(plain, px);
    let (w, h) = (am.width, am.height);
    // bitmap row j (0 at the top) is at height ymin + h - 1 - j: the accent is what stands above the plain letter's top
    let rows = (am.ymin + h as i32 - (pm.ymin + pm.height as i32)).clamp(0, h as i32) as usize;
    let bm = &bmp;
    let ink: Vec<usize> = (0..rows).flat_map(|j| (0..w).filter(move |&c| bm[j * w + c] > 0)).collect();
    let (c0, c1) = (*ink.iter().min()?, *ink.iter().max()?);
    // each copy half the accent's width to the side, so the two just meet
    let s = (((c1 - c0 + 1) as f32 * 0.55).round() as usize).max(1);
    let ow = w + 2 * s;
    let mut out = vec![0u8; ow * h];
    for j in 0..h {
        for c in 0..w {
            let a = bmp[j * w + c];
            if j >= rows {
                out[j * ow + c + s] = a;
            } else if a > 0 {
                for x in [c, c + 2 * s] {
                    out[j * ow + x] = out[j * ow + x].max(a);
                }
            }
        }
    }
    Some((fontdue::Metrics { xmin: am.xmin - s as i32, width: ow, ..am }, out))
}

/// The face's glyph for `ch` and where to draw it from: the face itself for Latin-1 (as it always was) and for an added
/// character it has; else the fallback face (the system's Arial Bold or its bundled stand-in, which have them all); else
/// the plain letter from the face.
fn glyph_source<'a>(face: &'a fontdue::Font, fallback: Option<&'a fontdue::Font>, i: usize, ch: char) -> (&'a fontdue::Font, char) {
    if i < COUNT as usize || face.has_glyph(ch) {
        return (face, ch);
    }
    match fallback.filter(|f| f.has_glyph(ch)) {
        Some(f) => (f, ch),
        None => (face, plain_char(ch)),
    }
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
/// Latin Extended-A: U+0100 to U+017F.
const LATIN_EXT_A: u32 = 0x100;
/// Each Latin Extended-A letter's plain ASCII letter (its Unicode decomposition's base; ł as l, œ as o and so on).
const LATIN_EXT_A_PLAIN: &str =
    "AaAaAaCcCcCcCcDdDdEeEeEeEeEeGgGgGgGgHhHhIiIiIiIiIiIiJjKkkLlLlLlLlLlNnNnNnnNnOoOoOoOoRrRrRrSsSsSsSsTtTtTtUuUuUuUuUuUuWwYyYZzZzZzs";
/// The typographic quotes (single, double and the low ones of Hungarian, Polish, German...), dashes and ellipsis.
const PUNCT: [char; 9] = ['\u{2018}', '\u{2019}', '\u{201A}', '\u{201C}', '\u{201D}', '\u{201E}', '\u{2013}', '\u{2014}', '\u{2026}'];
/// Every glyph the sets hold.
const TOTAL: usize = COUNT as usize + 128 + PUNCT.len();
const BAKE_PX: f32 = 32.0;
const ATLAS: usize = 1024;

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
    /// The port's own bundled faces, the same with or without the player's SimGolf files (the launcher, launcher.rs):
    /// Fredoka SemiBold for headings and Nunito ExtraBold for the rest (SIL Open Font License 1.1, fonts/).
    Display,
    Body,
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
    /// The same face `k` times the size, to a whole pixel (the text size, textsize.rs; itself at 1).
    pub fn scaled(self, k: f32) -> Fnt {
        Fnt { face: self.face, px: crate::textsize::sized(self.px, k) }
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

/// The faces by `Face` (Info's slot stays empty: it is a pair of the others).
static FONTS: [std::sync::OnceLock<Font>; 6] = [const { std::sync::OnceLock::new() }; 6];

thread_local! {
    /// The face a screen asked for; None picks by size (Arial below 12 pixels, Manual SSi above).
    static FACE: std::cell::Cell<Option<Face>> = const { std::cell::Cell::new(None) };
    /// Text drawn at the screen's own pixel size whatever the graphics (`set_crisp`).
    static CRISP: std::cell::Cell<bool> = const { std::cell::Cell::new(false) };
}

/// Draws the following text rasterised at the window's own pixel size, as Enhanced graphics' text is, in Classic too (the
/// launcher, which is not the original's art: launcher.rs).
pub fn set_crisp(on: bool) {
    CRISP.with(|c| c.set(on));
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
    /// The typeface, kept for Enhanced graphics' text at the screen's own pixel size.
    raw: Option<fontdue::Font>,
    /// Arial Bold (or its stand-in) for the added characters this face does not have.
    fallback: Option<fontdue::Font>,
    /// The bundled faces (`load_bundled`): the pen goes by the glyphs' exact advances, not whole pixels as GDI's.
    exact: bool,
}

/// Enhanced graphics' text: a face's glyphs rasterised at one pixel size (the size asked for times the screen's scale).
struct SizedFont {
    tex: TextureId,
    /// Atlas rectangles and offsets in pixels at that size.
    glyphs: Vec<Glyph>,
    /// The atlas's size in pixels.
    w: f32,
    h: f32,
}

thread_local! {
    /// The sized glyph sets made so far, by face and pixel size (None: could not be made).
    static SIZED: std::cell::RefCell<std::collections::HashMap<(usize, u32), Option<SizedFont>>> = Default::default();
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
    /// Bakes the glyph sets (`glyph_index`) of the three faces at a fixed size, each into its own texture, once at start up: Klepto and
    /// Manual SSi from the game folder, Arial Bold from the system or the bundled stand-in.
    pub fn load(g: &mut Gfx, game_font: impl Fn(&str) -> std::path::PathBuf) -> bool {
        let read = |p: &Path| sg_core::fsutil::read_file(p);
        let klepto = read(&game_font("KLEPTO__.TTF"));
        let manual = read(&game_font("manu3_.TTF"));
        let arial = ["C:/Windows/Fonts/arialbd.ttf", "/Library/Fonts/Arial Bold.ttf", "/System/Library/Fonts/Supplementary/Arial Bold.ttf"]
            .iter()
            .find_map(|p| read(Path::new(p)))
            .unwrap_or_else(|| ARIAL_STANDIN.to_vec());
        let fallback = fontdue::Font::from_bytes(arial.clone(), fontdue::FontSettings::default()).ok();
        for (i, ttf) in [klepto, manual, Some(arial)].into_iter().enumerate() {
            let fb = if i < 2 { fallback.clone() } else { None };
            if let Some(f) = ttf.and_then(|b| Self::load_inner(g, b, fb)) {
                let _ = FONTS[i].set(f);
            }
        }
        FONTS[0].get().is_some()
    }

    /// The bundled faces (`Face::Display`, `Face::Body`), made the first time they are asked for (the launcher's first
    /// frame); nothing when they are made already.
    pub fn load_bundled(g: &mut Gfx) {
        let fallback = fontdue::Font::from_bytes(ARIAL_STANDIN, fontdue::FontSettings::default()).ok();
        for (face, ttf) in [(Face::Display, sg_core::freeset::paint::DISPLAY_TTF), (Face::Body, sg_core::freeset::paint::TEXT_TTF)] {
            if FONTS[face as usize].get().is_none() {
                if let Some(mut f) = Self::load_inner(g, ttf.to_vec(), fallback.clone()) {
                    f.exact = true;
                    let _ = FONTS[face as usize].set(f);
                }
            }
        }
    }

    /// The face's font, or the first one loaded when it is missing.
    fn get(face: Face) -> Option<&'static Font> {
        FONTS[face as usize].get().or_else(|| FONTS.iter().find_map(|f| f.get()))
    }

    fn load_inner(g: &mut Gfx, ttf: Vec<u8>, fallback: Option<fontdue::Font>) -> Option<Font> {
        let vm = VMetrics::parse(&ttf);
        let font = fontdue::Font::from_bytes(ttf, fontdue::FontSettings::default()).ok()?;
        let vm = vm.unwrap_or_else(|| {
            let lm = font.horizontal_line_metrics(1000.0);
            let (a, d, gap) = lm.map(|m| (m.ascent, -m.descent, m.line_gap)).unwrap_or((800.0, 200.0, 0.0));
            VMetrics { upem: 1000.0, win_asc: a, win_desc: d, hhea_asc: a, hhea_desc: -d, hhea_gap: gap }
        });
        let mut atlas = vec![0u8; ATLAS * ATLAS * 4];
        let mut glyphs = Vec::with_capacity(TOTAL);
        let (mut x, mut y, mut row_h) = (1usize, 1usize, 0usize);
        for i in 0..TOTAL {
            let (m, bmp) = raster_glyph(&font, fallback.as_ref(), i, glyph_char(i), BAKE_PX);
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
        let tex = g.texture_classic(&img);
        crate::fx::plain_texture(tex); // text is not pixel art: the effects' scaler leaves it alone
        Some(Font { tex: Some(tex), glyphs, vm, raw: Some(font), fallback, exact: false })
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

    /// A glyph's advance at `px`: GDI steps the pen by whole pixels (the bundled faces: exactly).
    fn advance(&self, gl: &Glyph, px: f32) -> f32 {
        if self.exact {
            gl.advance * px / BAKE_PX
        } else {
            (gl.advance * px / BAKE_PX).round()
        }
    }

    fn glyph(&self, c: char) -> Option<&Glyph> {
        self.glyphs.get(glyph_index(c))
    }

    /// Rasterises the glyph sets (`glyph_index`) at `px` pixels into one atlas (Enhanced graphics' text).
    fn bake(&self, g: &mut Gfx, px: u32) -> Option<SizedFont> {
        let font = self.raw.as_ref()?;
        let width: usize = if px > 40 { 2048 } else { 1024 };
        let mut cells = Vec::with_capacity(TOTAL);
        let (mut x, mut y, mut row_h) = (1usize, 1usize, 0usize);
        for i in 0..TOTAL {
            let (m, bmp) = raster_glyph(font, self.fallback.as_ref(), i, glyph_char(i), px as f32);
            if x + m.width + 1 >= width {
                x = 1;
                y += row_h + 1;
                row_h = 0;
            }
            cells.push((x, y, m, bmp));
            x += m.width + 1;
            row_h = row_h.max(m.height);
        }
        let height = (y + row_h + 2).next_multiple_of(4);
        if height > 4096 {
            return None;
        }
        let mut atlas = vec![0u8; width * height * 4];
        let mut glyphs = Vec::with_capacity(cells.len());
        for (x, y, m, bmp) in cells {
            for j in 0..m.height {
                for i in 0..m.width {
                    let o = ((y + j) * width + x + i) * 4;
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
        }
        let img = sg_core::assets::Rgba { w: width as u32, h: height as u32, px: atlas };
        Some(SizedFont { tex: g.text_atlas(&img), glyphs, w: width as f32, h: height as f32 })
    }

    pub fn width(&self, s: &str, size: f32) -> f32 {
        s.chars().filter_map(|c| self.glyph(c)).map(|g| self.advance(g, size)).sum::<f32>()
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
    /// The 800x600 point under a window pixel, for the pointer: with Enhanced widescreen, in the layout of the interface
    /// piece under it when that piece is drawn at a window edge (enhanced::unshift).
    pub fn to_virtual(self, px: f32, py: f32) -> (f32, f32) {
        let (x, y) = self.to_virtual_raw(px, py);
        if crate::enhanced::widescreen() {
            crate::enhanced::unshift(x, y)
        } else {
            (x, y)
        }
    }

    /// The centred 800x600 point under a window pixel (the course's places on the screen).
    pub fn to_virtual_raw(self, px: f32, py: f32) -> (f32, f32) {
        ((px - self.ox) / self.scale, (py - self.oy) / self.scale)
    }
}

/// How many window pixels one pixel of the 800 x 600 screen takes on a `w` x `h` drawable (the course's sprites and the
/// interface alike): as many as fit, or with pixel-perfect scaling (the port's own, enhanced.rs) the most whole pixels
/// that fit. A window smaller than 800 x 600 shrinks the picture either way.
pub fn view_scale(w: f32, h: f32) -> f32 {
    let s = (w / 800.0).min(h / 600.0);
    if s >= 1.0 && crate::enhanced::pixel_perfect() {
        s.floor()
    } else {
        s
    }
}

/// Whether pixel-perfect scaling applies on a `w` x `h` drawable (it is on and the window holds the 800 x 600 picture).
pub fn pixel_perfect_on(w: f32, h: f32) -> bool {
    crate::enhanced::pixel_perfect() && w >= 800.0 && h >= 600.0
}

/// Pixel-perfect scaling (the port's own): the window round the picture, black in Classic and Enhanced's dark green frame
/// with Enhanced graphics on. Nothing when it is off or the picture fills the window.
pub fn pixel_border(g: &mut Gfx, w: f32, h: f32) {
    if !pixel_perfect_on(w, h) {
        return;
    }
    let s = Screen::new(w, h);
    let (l, t, r, b) = s.extent();
    if l > -0.5 && t > -0.5 && r < 800.5 && b < 600.5 {
        return;
    }
    if crate::enhanced::settings().enabled {
        crate::enhanced::backdrop(g, &s, false);
        return;
    }
    let black = [0.0, 0.0, 0.0, 1.0];
    s.fill(g, l, t, -l, b - t, black);
    s.fill(g, 800.0, t, r - 800.0, b - t, black);
    s.fill(g, 0.0, t, 800.0, -t, black);
    s.fill(g, 0.0, 600.0, 800.0, b - 600.0, black);
}

thread_local! {
    /// The edges of the window the device hides (a phone's camera cut-out, rounded corners and home bar, where the browser
    /// lets the game draw under them): left, top, right, bottom in drawable pixels, set each frame (main.rs). None elsewhere.
    static SAFE: std::cell::Cell<[f32; 4]> = const { std::cell::Cell::new([0.0; 4]) };
}

/// The window's hidden edges for the frames that follow, in drawable pixels (left, top, right, bottom).
pub fn set_safe_area(px: [f32; 4]) {
    SAFE.with(|c| c.set(px.map(|p| p.max(0.0))));
}

/// The window's hidden edges as the platform reports them, in drawable pixels of a `w` x `h` drawable: in a browser the
/// page's CSS safe-area insets (web/opensgolf.js sgFitCanvas, as fractions of the canvas); elsewhere none, or
/// SG_SAFE_AREA="l,t,r,b" (drawable pixels) for scripted stills of a phone's screen.
pub fn platform_safe_area(w: f32, h: f32) -> [f32; 4] {
    #[cfg(target_arch = "wasm32")]
    {
        #[link(wasm_import_module = "env")]
        extern "C" {
            fn sg_safe_area(side: u32) -> f32;
        }
        // SAFETY: the page's function takes a number and returns one
        let f = |side| unsafe { sg_safe_area(side) };
        [f(0) * w, f(1) * h, f(2) * w, f(3) * h]
    }
    #[cfg(not(target_arch = "wasm32"))]
    {
        let _ = (w, h);
        static TEST: std::sync::OnceLock<[f32; 4]> = std::sync::OnceLock::new();
        *TEST.get_or_init(|| {
            let v: Vec<f32> = std::env::var("SG_SAFE_AREA").unwrap_or_default().split(',').filter_map(|s| s.trim().parse().ok()).collect();
            match v[..] {
                [l, t, r, b] => [l, t, r, b],
                _ => [0.0; 4],
            }
        })
    }
}

/// A 2D drawing session over the 800x600 virtual screen.
pub struct Screen {
    pub view: View,
    u: Uniforms,
    /// Enhanced widescreen: the shift of the anchor in use (0, 0 in Classic) and the window's extent in 800x600 units.
    shift: std::cell::Cell<(f32, f32)>,
    wide: bool,
    draw: (f32, f32),
}

impl Screen {
    /// Orthographic projection for 800x600 virtual units in a draw_w x draw_h drawable.
    pub fn new(draw_w: f32, draw_h: f32) -> Screen {
        let scale = view_scale(draw_w, draw_h);
        let (mut ox, mut oy) = ((draw_w - 800.0 * scale) * 0.5, (draw_h - 600.0 * scale) * 0.5);
        if pixel_perfect_on(draw_w, draw_h) {
            // whole window pixels, so every pixel of the art lands on the same number of them
            (ox, oy) = (ox.floor(), oy.floor());
        }
        let view = View { scale, ox, oy };
        let proj = Mat4::ortho(-view.ox / scale, (draw_w - view.ox) / scale, (draw_h - view.oy) / scale, -view.oy / scale, -1.0, 1.0);
        Screen {
            view,
            u: Uniforms::flat(&proj, &Mat4::identity()),
            shift: std::cell::Cell::new((0.0, 0.0)),
            wide: crate::enhanced::widescreen(),
            draw: (draw_w, draw_h),
        }
    }

    /// A drawing session over the whole drawable in units of `k` window pixels from its top left (the launcher's own
    /// layout, launcher.rs), with no anchors.
    pub fn with_unit(draw_w: f32, draw_h: f32, k: f32) -> Screen {
        let k = k.max(1e-3);
        let view = View { scale: k, ox: 0.0, oy: 0.0 };
        let proj = Mat4::ortho(0.0, draw_w / k, draw_h / k, 0.0, -1.0, 1.0);
        Screen {
            view,
            u: Uniforms::flat(&proj, &Mat4::identity()),
            shift: std::cell::Cell::new((0.0, 0.0)),
            wide: false,
            draw: (draw_w, draw_h),
        }
    }

    /// The window's visible area in 800x600 units (left, top, right, bottom; the centred 800x600 is 0, 0, 800, 600).
    pub fn extent(&self) -> (f32, f32, f32, f32) {
        let v = self.view;
        (-v.ox / v.scale, -v.oy / v.scale, (self.draw.0 - v.ox) / v.scale, (self.draw.1 - v.oy) / v.scale)
    }

    /// [`Self::extent`] less the edges the device hides (set_safe_area): where Enhanced widescreen puts what it draws at the
    /// window's edges, so a phone's camera cut-out and rounded corners do not cover the gauges or the touch buttons.
    pub fn safe_extent(&self) -> (f32, f32, f32, f32) {
        let (l, t, r, b) = self.extent();
        let [sl, st, sr, _] = SAFE.with(|c| c.get()).map(|p| p / self.view.scale);
        // the home bar at the bottom stays over the dock, as it always was in the browser: lifting the dock off the
        // window's foot would show a strip of the course under it
        (l + sl, t + st, r - sr, b)
    }

    /// Enhanced widescreen: what follows is laid out for the 800x600 screen and drawn at this edge of the window (whole
    /// pixels away). Nothing in Classic.
    pub fn anchor(&self, a: crate::enhanced::Anchor) {
        if !self.wide {
            return;
        }
        self.shift.set(self.anchor_shift(a));
    }

    /// How far [`Self::anchor`] moves what follows (0, 0 in Classic), for hit tests that cannot wait for the frame's
    /// rectangles (enhanced::unshift).
    pub fn anchor_shift(&self, a: crate::enhanced::Anchor) -> (f32, f32) {
        if !self.wide {
            return (0.0, 0.0);
        }
        let (dx, dy) = a.shift(self.safe_extent());
        let k = self.view.scale;
        ((dx * k).round() / k, (dy * k).round() / k)
    }

    /// One quad of the interface: corners (x0, y0) to (x1, y1) in 800x600 units, moved by the anchor in use.
    fn emit(&self, g: &mut Gfx, tex: Option<TextureId>, q: [Vert; 4], record: bool) {
        if !self.wide {
            g.quad(Mode::Flat, tex, &self.u, q);
            return;
        }
        let (dx, dy) = self.shift.get();
        let q = q.map(|mut v| {
            v.pos[0] += dx;
            v.pos[1] += dy;
            v
        });
        if record {
            let xs = q.map(|v| v.pos[0]);
            let ys = q.map(|v| v.pos[1]);
            let (x0, x1) = (xs.iter().cloned().fold(f32::MAX, f32::min), xs.iter().cloned().fold(f32::MIN, f32::max));
            let (y0, y1) = (ys.iter().cloned().fold(f32::MAX, f32::min), ys.iter().cloned().fold(f32::MIN, f32::max));
            crate::enhanced::record(x0, y0, x1, y1, (dx, dy));
        }
        g.quad(Mode::Flat, tex, &self.u, q);
    }

    /// A rectangle shaded from `top` to `bottom` (Enhanced's frames).
    pub fn gradient(&self, g: &mut Gfx, x: f32, y: f32, w: f32, h: f32, top: [f32; 4], bottom: [f32; 4]) {
        self.emit(
            g,
            None,
            [
                Vert::new(x, y, 0.0, 0.0, 0.0).col(top),
                Vert::new(x + w, y, 0.0, 0.0, 0.0).col(top),
                Vert::new(x + w, y + h, 0.0, 0.0, 0.0).col(bottom),
                Vert::new(x, y + h, 0.0, 0.0, 0.0).col(bottom),
            ],
            false,
        );
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
        let (mut u0, mut v0, mut u1, mut v1) = (sx / im.w, sy / im.h, (sx + sw) / im.w, (sy + sh) / im.h);
        // A piece cut from a sheet and magnified (any window other than 800x600, and every Retina screen) is smoothed with
        // the texels round it, so the neighbouring piece's edge or the sheet's guide line showed along the cut, or its
        // edge went part transparent and showed what lies under it (the green guide line in the comments report's header
        // piece, under the first row). The texture coordinates of a cut inside the sheet are moved in by half a texel,
        // so no sample reaches past the piece's outer texel centres. At 1:1 (and smaller) nothing moves.
        let (tw, th) = g.ctx.texture_size(tex);
        // the inset as a fraction of the piece: dst is its size on screen in 800x600 units, texels its size in the texture
        let inset = |dst: f32, texels: f32| {
            let per_texel = dst * self.view.scale / texels.max(1e-3);
            if per_texel > 1.001 && texels > 2.0 {
                0.5 / texels
            } else {
                0.0
            }
        };
        let (texels_x, texels_y) = (sw * tw as f32 / im.w, sh * th as f32 / im.h);
        let (ix, iy) = (inset(dw, texels_x) * sw / im.w, inset(dh, texels_y) * sh / im.h);
        if sx > 0.0 {
            u0 += ix;
        }
        if sx + sw < im.w {
            u1 -= ix;
        }
        if sy > 0.0 {
            v0 += iy;
        }
        if sy + sh < im.h {
            v1 -= iy;
        }
        self.emit(
            g,
            Some(tex),
            [
                Vert::new(dx, dy, 0.0, u0, v0).col(c),
                Vert::new(dx + dw, dy, 0.0, u1, v0).col(c),
                Vert::new(dx + dw, dy + dh, 0.0, u1, v1).col(c),
                Vert::new(dx, dy + dh, 0.0, u0, v1).col(c),
            ],
            true,
        );
    }
    pub fn image(&self, g: &mut Gfx, im: &Image, dx: f32, dy: f32) {
        self.image_part(g, im, dx, dy, 0.0, 0.0, im.w, im.h);
    }
    pub fn fill(&self, g: &mut Gfx, x: f32, y: f32, w: f32, h: f32, c: [f32; 4]) {
        self.emit(
            g,
            None,
            [
                Vert::new(x, y, 0.0, 0.0, 0.0).col(c),
                Vert::new(x + w, y, 0.0, 0.0, 0.0).col(c),
                Vert::new(x + w, y + h, 0.0, 0.0, 0.0).col(c),
                Vert::new(x, y + h, 0.0, 0.0, 0.0).col(c),
            ],
            true,
        );
    }

    /// A filled diamond centred on (x, y), reaching `hw` pixels left and right and `hh` up and down.
    pub fn diamond(&self, g: &mut Gfx, x: f32, y: f32, hw: f32, hh: f32, c: [f32; 4]) {
        self.emit(
            g,
            None,
            [
                Vert::new(x - hw, y, 0.0, 0.0, 0.0).col(c),
                Vert::new(x, y - hh, 0.0, 0.0, 0.0).col(c),
                Vert::new(x + hw, y, 0.0, 0.0, 0.0).col(c),
                Vert::new(x, y + hh, 0.0, 0.0, 0.0).col(c),
            ],
            true,
        );
    }

    /// A filled four-cornered shape through these points in turn.
    pub fn quad(&self, g: &mut Gfx, p: [(f32, f32); 4], c: [f32; 4]) {
        self.emit(g, None, p.map(|(x, y)| Vert::new(x, y, 0.0, 0.0, 0.0).col(c)), true);
    }

    /// A straight line `w` pixels wide.
    #[allow(clippy::too_many_arguments)]
    pub fn line(&self, g: &mut Gfx, x0: f32, y0: f32, x1: f32, y1: f32, w: f32, c: [f32; 4]) {
        let (dx, dy) = (x1 - x0, y1 - y0);
        let l = (dx * dx + dy * dy).sqrt().max(1e-4);
        let (nx, ny) = (-dy / l * w * 0.5, dx / l * w * 0.5);
        self.emit(
            g,
            None,
            [
                Vert::new(x0 + nx, y0 + ny, 0.0, 0.0, 0.0).col(c),
                Vert::new(x1 + nx, y1 + ny, 0.0, 0.0, 0.0).col(c),
                Vert::new(x1 - nx, y1 - ny, 0.0, 0.0, 0.0).col(c),
                Vert::new(x0 - nx, y0 - ny, 0.0, 0.0, 0.0).col(c),
            ],
            true,
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
    pub(crate) fn text_in(&self, g: &mut Gfx, face: Face, x: f32, y: f32, s: &str, size: f32, c: [f32; 4]) {
        log_text(s);
        let Some(f) = Font::get(face) else { return };
        if (crate::enhanced::native_text() || CRISP.with(|c| c.get())) && self.text_native(g, f, face, x, y, s, size, c) {
            return;
        }
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
                self.emit(
                    g,
                    Some(tex),
                    [
                        Vert::new(x0, y0, 0.0, u0, v0).col(c),
                        Vert::new(x1, y0, 0.0, u1, v0).col(c),
                        Vert::new(x1, y1, 0.0, u1, v1).col(c),
                        Vert::new(x0, y1, 0.0, u0, v1).col(c),
                    ],
                    false,
                );
            }
            pen += f.advance(gl, size);
        }
    }
    /// Enhanced graphics' text: the glyphs rasterised at the screen's own pixel size and drawn texel for pixel, each on a
    /// whole pixel of the window; the pen advances exactly as in `text_in`, so the layout is the same. False when the
    /// glyphs could not be made (the caller draws the classic way).
    fn text_native(&self, g: &mut Gfx, f: &Font, face: Face, x: f32, y: f32, s: &str, size: f32, c: [f32; 4]) -> bool {
        let k = self.view.scale;
        let px = (size * k).round();
        if !(4.0..=320.0).contains(&px) {
            return false;
        }
        let key = (face as usize, px as u32);
        let (ox, oy) = (self.view.ox, self.view.oy);
        let (dx, dy) = self.shift.get();
        SIZED.with(|m| {
            let mut m = m.borrow_mut();
            if !m.contains_key(&key) {
                // a resized window leaves old sizes behind: start afresh now and then
                if m.len() >= 40 {
                    for t in m.drain().filter_map(|(_, v)| v) {
                        g.ctx.delete_texture(t.tex);
                    }
                }
                let baked = f.bake(g, key.1);
                m.insert(key, baked);
            }
            let Some(sf) = m.get(&key).and_then(|v| v.as_ref()) else { return false };
            let tex = sf.tex;
            let (aw, atlas_h) = (sf.w, sf.h);
            let mut pen = x;
            for ch in s.chars() {
                let Some(gl) = f.glyph(ch) else { continue };
                if let Some(ng) = sf.glyphs.get(glyph_index(ch)).filter(|ng| ng.aw > 0.0) {
                    let sx = ((pen + dx) * k + ox).round() + ng.xoff;
                    let sy = ((y + dy) * k + oy).round() + ng.yoff;
                    let (x0, y0) = ((sx - ox) / k - dx, (sy - oy) / k - dy);
                    let (x1, y1) = (x0 + ng.aw / k, y0 + ng.ah / k);
                    let (u0, v0, u1, v1) = (ng.ax / aw, ng.ay / atlas_h, (ng.ax + ng.aw) / aw, (ng.ay + ng.ah) / atlas_h);
                    self.emit(
                        g,
                        Some(tex),
                        [
                            Vert::new(x0, y0, 0.0, u0, v0).col(c),
                            Vert::new(x1, y0, 0.0, u1, v0).col(c),
                            Vert::new(x1, y1, 0.0, u1, v1).col(c),
                            Vert::new(x0, y1, 0.0, u0, v1).col(c),
                        ],
                        false,
                    );
                }
                pen += f.advance(gl, size);
            }
            true
        })
    }

    pub fn text_centered(&self, g: &mut Gfx, cx: f32, y: f32, s: &str, size: f32, c: [f32; 4]) {
        // as the exe's centred call: half the width rounded down
        self.text(g, cx - (text_width(s, size) / 2.0).floor(), y, s, size, c);
    }
}

/// Test hook: SG_TEXTLOG=FILE writes every text drawn to FILE, each different one once, in the order first drawn (for
/// checking a translation, or that a change left the text alone).
pub fn log_text(s: &str) {
    use std::io::Write;
    struct Log {
        file: Option<std::fs::File>,
        seen: std::collections::HashSet<String>,
    }
    thread_local! {
        static LOG: std::cell::RefCell<Option<Log>> = const { std::cell::RefCell::new(None) };
    }
    LOG.with(|l| {
        let mut l = l.borrow_mut();
        let log = l.get_or_insert_with(|| Log {
            file: std::env::var_os("SG_TEXTLOG").and_then(|p| std::fs::File::create(p).ok()),
            seen: Default::default(),
        });
        if let Some(f) = log.file.as_mut() {
            if log.seen.insert(s.to_string()) {
                let _ = writeln!(f, "{}", s.replace('\n', "\\n"));
            }
        }
    });
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

/// The colour of a Terrain::drawLine call (Terrain.dll 0x100048a0) for a 15-bit exe colour: each channel's five bits moved
/// up three over 255 (so a full channel is 248 / 255), the alpha the call's last argument in tenths (glColor4f, blended
/// SRC_ALPHA / ONE_MINUS_SRC_ALPHA). The exe's world overlay lines pass 7, the tooltip bar 5, the routing map 10.
pub fn line_rgba(c: u32, tenths: u32) -> [f32; 4] {
    let ch = |s: u32| ((c >> s) & 31) as f32 * 8.0 / 255.0;
    [ch(10), ch(5), ch(0), tenths as f32 / 10.0]
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

// ---- text too wide for its box (the port's own, issue #164) -----------------------------------------------------------------
//
// A translation is often longer than the English the original's buttons, tabs and titles were drawn for. A label that
// measures wider than its box is first drawn a little smaller (a pixel at a time, down to `FIT_MIN` of its size), then,
// where the box is tall enough, wrapped onto more lines, and as a last resort cut with an ellipsis, so it never runs over
// its neighbours. A label that fits is left exactly as it was: every helper below hands back the font, text and place it
// was given, so English (and Classic graphics) draw the same pixels as before.

/// The room of a title over one of the info screens or title screens (Klepto 24): wider than any of the original's titles
/// in English, inside the art's frames.
pub const TITLE_ROOM: f32 = 440.0;

/// The smallest share of its own size a label shrinks to before it wraps or is cut.
pub const FIT_MIN: f32 = 0.75;
/// What a cut label ends with (in the glyph sets: `PUNCT`).
const ELLIPSIS: char = '\u{2026}';

/// A label laid out to fit its box: the font (its own, or smaller) and its lines.
#[derive(Clone, Debug, PartialEq)]
pub struct Fitted {
    pub font: Fnt,
    pub lines: Vec<String>,
}

/// Test hook: SG_TEXTFIT=1 prints each label that had to be fitted to its box, once (for checking a translation).
fn log_fit(s: &str, w: f32) {
    thread_local! {
        static SEEN: std::cell::RefCell<Option<std::collections::HashSet<String>>> = const { std::cell::RefCell::new(None) };
    }
    if std::env::var_os("SG_TEXTFIT").is_none() {
        return;
    }
    SEEN.with(|v| {
        if v.borrow_mut().get_or_insert_with(Default::default).insert(s.to_string()) {
            println!("text fitted to {w:.0} pixels: {s}");
        }
    });
}

/// Lines of `s` no wider than `w` in `f` (a word wider than the line stays whole: `fit_box` cuts it).
fn wrap_fnt(f: Fnt, s: &str, w: f32) -> Vec<String> {
    let mut out = Vec::new();
    let mut cur = String::new();
    for word in s.split_whitespace() {
        let t = if cur.is_empty() { word.to_string() } else { format!("{cur} {word}") };
        if !cur.is_empty() && f.width(&t) > w {
            out.push(std::mem::replace(&mut cur, word.to_string()));
        } else {
            cur = t;
        }
    }
    if !cur.is_empty() || out.is_empty() {
        out.push(cur);
    }
    out
}

/// `s` cut to fit `w` in `f` with an ellipsis at its end (always one when `cut`: words were dropped after it).
fn ellipsize(f: Fnt, s: &str, w: f32, cut: bool) -> String {
    if !cut && f.width(s) <= w {
        return s.to_string();
    }
    let mut t: Vec<char> = s.trim_end().chars().collect();
    loop {
        let mut out: String = t.iter().collect::<String>().trim_end().to_string();
        out.push(ELLIPSIS);
        if t.is_empty() || f.width(&out) <= w {
            return out;
        }
        t.pop();
    }
}

/// How many lines of `f` a box `h` high holds (at least one; a box no taller than a line holds one).
fn rows_in(f: Fnt, h: f32) -> usize {
    if h <= f.px {
        return 1;
    }
    ((h - f.px) / f.line().max(1.0)).floor() as usize + 1
}

/// How `s` in `f` goes into a box `w` wide and `h` high: None when it fits as it is (draw it as always). Else the first
/// that fits of: one line a pixel smaller at a time, down to `FIT_MIN` of the size; the lines the box has room for, at the
/// biggest of those sizes that holds them; at the smallest size, as many lines as there is room for, the last (and any
/// word too long for a line) cut with an ellipsis.
pub fn fit_box(f: Fnt, s: &str, w: f32, h: f32) -> Option<Fitted> {
    if w <= 0.0 || f.width(s) <= w {
        return None;
    }
    log_fit(s, w);
    let lo = (f.px * FIT_MIN).ceil().min(f.px);
    let sizes: Vec<Fnt> = (0..).map(|i| Fnt { face: f.face, px: f.px - i as f32 }).take_while(|g| g.px >= lo).collect();
    let one = |g: Fnt| Fitted { font: g, lines: vec![s.to_string()] };
    if let Some(&g) = sizes.iter().skip(1).find(|g| g.width(s) <= w) {
        return Some(one(g));
    }
    for &g in &sizes {
        let n = rows_in(g, h);
        if n > 1 {
            let lines = wrap_fnt(g, s, w);
            if lines.len() <= n && lines.iter().all(|l| g.width(l) <= w) {
                return Some(Fitted { font: g, lines });
            }
        }
    }
    let g = sizes.last().copied().unwrap_or(f);
    let n = rows_in(g, h);
    let mut lines = wrap_fnt(g, s, w);
    let cut = lines.len() > n;
    lines.truncate(n);
    let last = lines.len() - 1;
    let lines = lines.iter().enumerate().map(|(i, l)| ellipsize(g, l, w, cut && i == last)).collect();
    Some(Fitted { font: g, lines })
}

/// One line of `s` in `f` no wider than `w` (`fit_box` with room for one line): the font, the text and how far down to
/// put it so a smaller font keeps the line's middle where it was. `f`, `s` and 0 when it fits.
pub fn fit_line(f: Fnt, s: &str, w: f32) -> (Fnt, std::borrow::Cow<'_, str>, f32) {
    match fit_box(f, s, w, 0.0) {
        None => (f, std::borrow::Cow::Borrowed(s), 0.0),
        Some(t) => {
            let dy = ((f.px - t.font.px) / 2.0).floor();
            (t.font, std::borrow::Cow::Owned(t.lines.into_iter().next().unwrap_or_default()), dy)
        }
    }
}

/// A fitted label's font and its lines, each with the x it is centred on and its top (`fit_in`).
pub type FitLines = (Fnt, Vec<(String, f32, f32)>);

/// `s` in `f` centred in the box (x, y, w, h) when it is too wide for it (`fit_box`): the font and each line with its
/// top, the lines spaced by the font's line and the block in the box's middle (a single line at
/// `y + ((h - px) / 2).floor()`, as the port's buttons and tabs place a label). None when it fits: draw it as always.
pub fn fit_in(f: Fnt, s: &str, (x, y, w, h): (f32, f32, f32, f32)) -> Option<FitLines> {
    let t = fit_box(f, s, w, h)?;
    let (g, n) = (t.font, t.lines.len());
    let pitch = if n > 1 { g.line().min((h - g.px) / (n - 1) as f32).floor() } else { 0.0 };
    let block = pitch * (n - 1) as f32 + g.px;
    let top = y + ((h - block) / 2.0).floor();
    let cx = x + w / 2.0;
    Some((g, t.lines.into_iter().enumerate().map(|(i, l)| (l, cx, top + pitch * i as f32)).collect()))
}

/// `s` in `f` in a room `w` wide with room for `lines` lines (`fit_box`): None when it fits as it is, else the font and
/// the lines, at most `lines` of them.
pub fn fit_lines(f: Fnt, s: &str, w: f32, lines: usize) -> Option<Fitted> {
    fit_box(f, s, w, f.px + f.line() * lines.saturating_sub(1) as f32)
}

/// The font object `text` draws a size in (Arial below 12 pixels, Manual SSi above, or the screen's face): for fitting
/// text that is drawn by its size.
pub fn fnt_for(size: f32) -> Fnt {
    Fnt { face: face_for(size), px: size }
}

impl Screen {
    /// `text_centered` (by size, the baseline at y) for a label in a room `w` wide centred on cx: as before when it fits,
    /// else a little smaller or cut (`fit_line`), on the same baseline and in the same face.
    #[allow(clippy::too_many_arguments)]
    pub fn text_centered_fit(&self, g: &mut Gfx, cx: f32, y: f32, w: f32, s: &str, size: f32, c: [f32; 4]) {
        let (f, s, _) = fit_line(fnt_for(size), s, w);
        self.text_in(g, f.face, cx - (f.width(&s) / 2.0).floor(), y, &s, f.px, c);
    }

    /// `put_centered` for a label in a room `w` wide: as before when it fits, else a little smaller or cut (`fit_line`).
    #[allow(clippy::too_many_arguments)]
    pub fn put_centered_fit(&self, g: &mut Gfx, f: Fnt, cx: f32, y: f32, w: f32, s: &str, c: [f32; 4]) {
        let (f, s, dy) = fit_line(f, s, w);
        self.put_centered(g, f, cx, y + dy, &s, c);
    }

    /// `put` for a label in a room `w` wide (from x): as before when it fits, else a little smaller or cut (`fit_line`).
    #[allow(clippy::too_many_arguments)]
    pub fn put_fit(&self, g: &mut Gfx, f: Fnt, x: f32, y: f32, w: f32, s: &str, c: [f32; 4]) {
        let (f, s, dy) = fit_line(f, s, w);
        self.put(g, f, x, y + dy, &s, c);
    }

    /// `text` (by size, the baseline at y) for a label in a room `w` wide from x: as before when it fits, else a little
    /// smaller or cut (`fit_line`), on the same baseline.
    #[allow(clippy::too_many_arguments)]
    pub fn text_fit(&self, g: &mut Gfx, x: f32, y: f32, w: f32, s: &str, size: f32, c: [f32; 4]) {
        let (f, s, _) = fit_line(Fnt { face: face_for(size), px: size }, s, w);
        self.text_in(g, f.face, x, y, &s, f.px, c);
    }
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
    fn central_european_letters_and_typographic_quotes_have_glyphs() {
        // issue #153: a Hungarian translation needs ő ű Ő Ű and the low and high quotes „ ”
        use super::{glyph_char, glyph_index, glyph_source, plain_char, TOTAL};
        for i in 0..TOTAL {
            assert_eq!(glyph_index(glyph_char(i)), i, "place {i}");
        }
        for c in ['ő', 'Ő', 'ű', 'Ű', 'ł', '\u{201E}', '\u{201D}', '\u{2026}', 'é', 'A'] {
            assert_ne!(glyph_char(glyph_index(c)), '?', "{c}");
        }
        assert_eq!(glyph_index('\u{4E2D}'), glyph_index('?'), "outside the sets: '?'");
        assert_eq!((plain_char('ő'), plain_char('Ű'), plain_char('ł'), plain_char('\u{201E}')), ('o', 'U', 'l', '"'));
        // the free set's heading face has no ő or „: they come from the stand-in for Arial, which has them all; Latin-1
        // stays the face's own
        let font = |b: &[u8]| fontdue::Font::from_bytes(b, fontdue::FontSettings::default()).unwrap();
        let (face, arial) = (font(sg_core::freeset::paint::DISPLAY_TTF), font(super::ARIAL_STANDIN));
        for c in ['ő', '\u{201E}'] {
            assert!(!face.has_glyph(c));
            let (src, ch) = glyph_source(&face, Some(&arial), glyph_index(c), c);
            assert!(std::ptr::eq(src, &arial) && ch == c, "{c} from the fallback");
            assert_eq!(glyph_source(&face, None, glyph_index(c), c).1, plain_char(c), "{c} with no fallback");
        }
        let (src, _) = glyph_source(&face, Some(&arial), glyph_index('A'), 'A');
        assert!(std::ptr::eq(src, &face));
    }

    #[test]
    fn hungarian_double_acute_letters_are_made_from_the_faces_own_acute_letters() {
        // issue #153: with the original game's fonts ő drew blank, Ő as some other symbol, and ű Ű from Arial stood out
        use super::{glyph_index, raster_glyph};
        let font = |b: &[u8]| fontdue::Font::from_bytes(b, fontdue::FontSettings::default()).unwrap();
        let arial = font(super::ARIAL_STANDIN);
        for face in [font(sg_core::freeset::paint::DISPLAY_TTF), font(sg_core::freeset::paint::TEXT_TTF)] {
            for (dbl, acute) in [('ő', 'ó'), ('Ő', 'Ó'), ('ű', 'ú'), ('Ű', 'Ú')] {
                let (m, bmp) = raster_glyph(&face, Some(&arial), glyph_index(dbl), dbl, 32.0);
                let (am, abmp) = face.rasterize(acute, 32.0);
                let ink = |b: &[u8]| b.iter().map(|&a| a as u32).sum::<u32>();
                // the face's letter with its accent twice: as tall, wider, more ink than the acute letter, same advance
                assert_eq!((m.height, m.ymin, m.advance_width), (am.height, am.ymin, am.advance_width), "{dbl}");
                assert!(m.width > am.width && ink(&bmp) > ink(&abmp), "{dbl}");
                assert!(ink(&bmp) < 2 * ink(&abmp), "{dbl}: one letter, two accents");
            }
        }
    }

    #[test]
    fn the_edges_a_phone_hides_narrow_where_the_window_edge_pieces_go() {
        // an iPhone 16 Pro Max sideways at 3 drawable pixels to a CSS pixel: 2868 x 1320, the camera cut-out on the left
        let s = super::Screen::new(2868.0, 1320.0);
        super::set_safe_area([0.0; 4]);
        assert_eq!(s.safe_extent(), s.extent());
        super::set_safe_area([186.0, 0.0, 186.0, 63.0]);
        let (l, t, r, b) = s.extent();
        let (sl, st, sr, sb) = s.safe_extent();
        let k = s.view.scale;
        assert!((sl - (l + 186.0 / k)).abs() < 1e-3 && (sr - (r - 186.0 / k)).abs() < 1e-3);
        // the top has nothing hidden, and the home bar is left over the dock
        assert_eq!((st, sb), (t, b));
        // still wider than the 800 x 600 screen: the edge pieces only come in from the window's sides
        assert!(sl < 0.0 && sr > 800.0);
        super::set_safe_area([0.0; 4]);
    }

    #[test]
    fn text_too_wide_for_its_box_shrinks_then_wraps_then_is_cut() {
        // issue #164 (the fonts are not loaded in tests: a letter is half the size wide, a line 1.25 sizes)
        use super::{fit_box, fit_in, fit_line, Face, Fnt, FIT_MIN};
        let f = Fnt { face: Face::Manual, px: 20.0 };
        // a label that fits is left as it is, to the pixel
        assert_eq!(fit_box(f, "abcdefghij", 100.0, 40.0), None);
        let (g, t, dy) = fit_line(f, "abcdefghij", 100.0);
        assert_eq!((g, &*t, dy), (f, "abcdefghij", 0.0));
        assert_eq!(fit_in(f, "abcdefghij", (0.0, 0.0, 100.0, 30.0)), None);
        // a little too wide: a pixel or two smaller, on one line, its middle kept
        let t = fit_box(f, "abcdefghij", 90.0, 0.0).unwrap();
        assert_eq!((t.font.px, t.lines.len()), (18.0, 1));
        assert_eq!(fit_line(f, "abcdefghij", 90.0).2, 1.0);
        // too wide even at the smallest size: onto two lines where the box is tall enough, at the size that holds them
        let s = "aaaa bbbb cccc dddd";
        let t = fit_box(f, s, 100.0, 60.0).unwrap();
        assert_eq!((t.font.px, t.lines.clone()), (20.0, vec!["aaaa bbbb".to_string(), "cccc dddd".to_string()]));
        // the lines in the box's middle, one line apart, inside it
        let (_, lines) = fit_in(f, s, (10.0, 100.0, 100.0, 60.0)).unwrap();
        assert_eq!(lines.len(), 2);
        assert!(lines[0].2 >= 100.0 && lines[1].2 + 20.0 <= 160.0 && lines[1].2 - lines[0].2 == 25.0);
        assert!(lines.iter().all(|l| l.1 == 60.0));
        // no room for a second line: at the smallest size, cut with an ellipsis, never wider than the box
        let t = fit_box(f, s, 100.0, 22.0).unwrap();
        assert_eq!(t.font.px, (20.0 * FIT_MIN).ceil());
        assert_eq!(t.lines.len(), 1);
        assert!(t.lines[0].ends_with('\u{2026}') && t.font.width(&t.lines[0]) <= 100.0, "{:?}", t.lines);
        // more words than lines: the last line kept says it was cut
        let t = fit_box(f, "aaaa bbbb cccc dddd eeee ffff gggg", 100.0, 42.0).unwrap();
        assert_eq!(t.lines.len(), 2);
        assert!(t.lines[1].ends_with('\u{2026}') && t.lines.iter().all(|l| t.font.width(l) <= 100.0), "{:?}", t.lines);
        // one word longer than the box is cut too
        let t = fit_box(f, "abcdefghijklmnopqrstuvwxyz", 100.0, 80.0).unwrap();
        assert!(t.lines.iter().all(|l| t.font.width(l) <= 100.0));
    }

    #[test]
    fn fit_lines_keeps_to_its_lines_and_leaves_what_fits() {
        // issue #164 (no fonts in tests: a letter is half the size wide, a line 1.25 sizes)
        use super::{fit_lines, Face, Fnt};
        let f = Fnt { face: Face::Arial, px: 10.0 };
        assert_eq!(fit_lines(f, "aaaa bbbb", 50.0, 3), None);
        let s = "aaaa bbbb cccc dddd eeee ffff gggg hhhh";
        for n in 1..4 {
            let t = fit_lines(f, s, 50.0, n).unwrap();
            assert!(t.lines.len() <= n && t.lines.iter().all(|l| t.font.width(l) <= 50.0), "{n}: {:?}", t.lines);
        }
        // room for three: every word is kept, on three lines at the biggest size that holds them
        let t = fit_lines(f, "aaaa bbbb cccc dddd eeee ffff", 50.0, 3).unwrap();
        assert_eq!((t.font.px, t.lines.len()), (10.0, 3));
        assert!(!t.lines.iter().any(|l| l.ends_with('\u{2026}')));
    }

    #[test]
    fn money_format() {
        assert_eq!(super::money(1234567), "\u{a7}1,234,567");
        assert_eq!(super::money(-500), "-\u{a7}500");
    }
}
