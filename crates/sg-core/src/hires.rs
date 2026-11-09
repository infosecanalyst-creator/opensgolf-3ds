//! High-resolution art of content packs (docs/MODDING.md, "High-resolution packs and your own art"). The port's own.
//!
//! A pack whose pack.toml says `scale = 2` or `4` holds pictures that many times the original's size, and any pack may give
//! a sprite as a PNG sheet (`Flics/Trees/TreeMapleLarge.png` for `TreeMapleLarge.flc`): one row per view, one column per
//! frame, every frame `scale` times the original's frame, with the original's ground point. The engine reads the layout
//! (views, frames, frame size, ground point) from the file the sheet replaces, so a sheet needs no other file; a
//! `<name>.json` next to it may change the layout.
//!
//! Colour variants are made from the one sheet: a pixel takes the colour change of the nearest colour of the original's
//! palette (tree and flower variants from their `*Pal*.pcx`, golfers' outfits from the swap palettes). A `<name>.mask.png`
//! limits that to the pixels it covers (and, for people, says which part of the clothes a pixel is), and a
//! `<name>~<palette>.png` gives a variant drawn by hand.
//!
//! This module is the part without graphics: sizes, slicing, scaling and recolouring. The loaders are in the app
//! (crates/simgolf/src/packart.rs); the files are found by crate::vfs.
use crate::assets::Rgba;
use crate::sprites::Sprite;
use serde::Deserialize;
use std::path::Path;

/// The size of a picture from its header: PNG, PCX, BMP or TGA (None for anything else).
pub fn picture_size(d: &[u8]) -> Option<(u32, u32)> {
    let u16le = |o: usize| d.get(o..o + 2).map(|b| u16::from_le_bytes([b[0], b[1]]) as u32);
    if crate::assets::is_png(d) {
        let be = |o: usize| d.get(o..o + 4).map(|b| u32::from_be_bytes([b[0], b[1], b[2], b[3]]));
        return Some((be(16)?, be(20)?));
    }
    if d.len() >= 12 && &d[..4] == b"RIFF" && &d[8..12] == b"WEBP" {
        return crate::png::decode_image(d).map(|i| (i.w, i.h));
    }
    if d.len() >= 128 && d[0] == 10 {
        let (x0, y0, x1, y1) = (u16le(4)?, u16le(6)?, u16le(8)?, u16le(10)?);
        return (x1 >= x0 && y1 >= y0).then(|| (x1 - x0 + 1, y1 - y0 + 1));
    }
    if d.len() >= 26 && d[0] == b'B' && d[1] == b'M' {
        let w = i32::from_le_bytes([d[18], d[19], d[20], d[21]]);
        let h = i32::from_le_bytes([d[22], d[23], d[24], d[25]]);
        return Some((w.unsigned_abs(), h.unsigned_abs()));
    }
    if d.len() >= 18 && d[1] == 0 && (d[2] == 2 || d[2] == 10) {
        return Some((u16le(12)?, u16le(14)?));
    }
    None
}

/// `img` made `s` times smaller: each pixel the average of its s x s block, colours weighted by their alpha (so the
/// transparent pixels' colours do not darken the edges).
pub fn downscale(img: &Rgba, s: u32) -> Rgba {
    if s <= 1 {
        return img.clone();
    }
    let (w, h) = (img.w / s, img.h / s);
    let mut out = Rgba::new(w, h);
    let iw = img.w as usize;
    for y in 0..h as usize {
        for x in 0..w as usize {
            let mut acc = [0u32; 4];
            for dy in 0..s as usize {
                for dx in 0..s as usize {
                    let i = ((y * s as usize + dy) * iw + x * s as usize + dx) * 4;
                    let a = img.px[i + 3] as u32;
                    for k in 0..3 {
                        acc[k] += img.px[i + k] as u32 * a;
                    }
                    acc[3] += a;
                }
            }
            let o = (y * w as usize + x) * 4;
            if let Some(a) = std::num::NonZeroU32::new(acc[3]) {
                for k in 0..3 {
                    out.px[o + k] = ((acc[k] + a.get() / 2) / a) as u8;
                }
            }
            let n = s * s;
            out.px[o + 3] = ((acc[3] + n / 2) / n) as u8;
        }
    }
    out
}

/// `img` made `s` times bigger, every pixel an s x s block (the starter kit's reference pictures).
pub fn upscale_nearest(img: &Rgba, s: u32) -> Rgba {
    let s = s.max(1);
    let mut out = Rgba::new(img.w * s, img.h * s);
    let ow = out.w as usize;
    for y in 0..out.h as usize {
        for x in 0..ow {
            let i = ((y / s as usize) * img.w as usize + x / s as usize) * 4;
            out.px[(y * ow + x) * 4..(y * ow + x) * 4 + 4].copy_from_slice(&img.px[i..i + 4]);
        }
    }
    out
}

/// The layout of a sprite sheet, in the original's pixels.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct Grid {
    pub views: u32,
    pub frames: u32,
    pub w: u32,
    pub h: u32,
    /// The ground point in the frame.
    pub ax: i32,
    pub ay: i32,
}

impl Grid {
    /// The layout of a loaded sprite.
    pub fn of(s: &Sprite) -> Grid {
        Grid { views: s.views.max(1) as u32, frames: s.frames_per_view.max(1) as u32, w: s.w, h: s.h, ax: s.anchor_x, ay: s.anchor_y }
    }

    /// The layout of an original sprite from the free set's layout manifest.
    pub fn of_reference(r: crate::freeset::Reference) -> Option<Grid> {
        match r {
            crate::freeset::Reference::Sprite { w, h, views, frames, ax, ay, .. } => Some(Grid { views, frames, w, h, ax, ay }),
            _ => None,
        }
    }

    /// The size of the sheet at `scale`.
    pub fn sheet_size(&self, scale: u32) -> (u32, u32) {
        (self.w * self.frames * scale, self.h * self.views * scale)
    }

    pub fn count(&self) -> usize {
        (self.views * self.frames) as usize
    }
}

/// Frame `i` (view-major: view i / frames, frame i % frames) cut from a sheet laid out as `g` at `scale`.
pub fn slice(sheet: &Rgba, g: &Grid, scale: u32, i: usize) -> Option<Rgba> {
    if i >= g.count() || (sheet.w, sheet.h) != g.sheet_size(scale) {
        return None;
    }
    let (fw, fh) = ((g.w * scale) as usize, (g.h * scale) as usize);
    let (v, f) = (i / g.frames as usize, i % g.frames as usize);
    let mut out = Rgba::new(fw as u32, fh as u32);
    let sw = sheet.w as usize;
    for y in 0..fh {
        let src = ((v * fh + y) * sw + f * fw) * 4;
        out.px[y * fw * 4..(y + 1) * fw * 4].copy_from_slice(&sheet.px[src..src + fw * 4]);
    }
    Some(out)
}

/// `<name>.json` next to a sheet: the layout where it differs from the original's (all in the original's pixels).
#[derive(Clone, Debug, Default, Deserialize, PartialEq)]
#[serde(deny_unknown_fields)]
pub struct SheetJson {
    /// The frame's width and height.
    pub frame: Option<[u32; 2]>,
    pub views: Option<u32>,
    /// Frames per view.
    pub frames: Option<u32>,
    /// The ground point in the frame.
    pub anchor: Option<[i32; 2]>,
    /// Milliseconds per frame.
    pub frame_ms: Option<u32>,
}

impl SheetJson {
    pub fn parse(text: &str) -> Result<SheetJson, String> {
        let j: SheetJson = serde_json::from_str(text).map_err(|e| e.to_string())?;
        let bad = |v: Option<u32>| v.is_some_and(|v| v == 0 || v > 256);
        if bad(j.views) || bad(j.frames) || j.frame.is_some_and(|[w, h]| w == 0 || h == 0 || w > 2048 || h > 2048) {
            return Err("a size, view or frame count is 0 or too big".into());
        }
        Ok(j)
    }

    pub fn apply(&self, g: &Grid) -> Grid {
        let mut g = *g;
        if let Some([w, h]) = self.frame {
            (g.w, g.h) = (w, h);
        }
        g.views = self.views.unwrap_or(g.views);
        g.frames = self.frames.unwrap_or(g.frames);
        if let Some([x, y]) = self.anchor {
            (g.ax, g.ay) = (x, y);
        }
        g
    }
}

// ---- recolouring --------------------------------------------------------------------------------------------------------------

/// A 15-bit colour's place in a lookup table.
fn key15(r: u8, g: u8, b: u8) -> usize {
    ((r as usize >> 3) << 10) | ((g as usize >> 3) << 5) | (b as usize >> 3)
}

/// For every 15-bit colour, the nearest of the palette entries `cands` (an empty list gives entry 0 everywhere).
pub fn nearest_table(pal: &[u8; 768], cands: &[u8]) -> Vec<u8> {
    let mut t = vec![cands.first().copied().unwrap_or(0); 32768];
    if cands.len() < 2 {
        return t;
    }
    for (k, slot) in t.iter_mut().enumerate() {
        let c = [((k >> 10) & 31) * 8 + 4, ((k >> 5) & 31) * 8 + 4, (k & 31) * 8 + 4].map(|v| v as i32);
        let mut best = (i32::MAX, 0u8);
        for &e in cands {
            let p = &pal[e as usize * 3..e as usize * 3 + 3];
            let d = [0, 1, 2].iter().map(|&i| (c[i] - p[i] as i32).pow(2)).sum::<i32>();
            if d < best.0 {
                best = (d, e);
            }
        }
        *slot = best.1;
    }
    t
}

/// Moves a pixel by the colour change of its nearest palette entry (looked up in `table`): `to` minus `from`.
pub fn shift(px: &mut [u8], table: &[u8], from: &[u8; 768], to: &[u8; 768]) {
    let e = table[key15(px[0], px[1], px[2])] as usize * 3;
    for k in 0..3 {
        px[k] = (px[k] as i32 + to[e + k] as i32 - from[e + k] as i32).clamp(0, 255) as u8;
    }
}

/// The parts of a person's clothes a mask colour stands for, as the swap palettes' ranges (sg_core::bodies::Swaps::compose):
/// red shirt, green trousers, blue skin, yellow hat, magenta women's hair, cyan hands and arms. None: not recoloured.
pub fn people_part(m: &[u8], female: bool) -> Option<std::ops::Range<u8>> {
    if m[3] < 128 {
        return None;
    }
    let hi = |v: u8| v >= 128;
    Some(match (hi(m[0]), hi(m[1]), hi(m[2])) {
        (true, false, false) => 0..20,
        (false, true, false) => 20..40,
        (false, false, true) => 40..60,
        (true, true, false) => 60..80,
        (true, false, true) => 80..90,
        (false, true, true) if female => 90..97,
        (false, true, true) => 80..90,
        _ => return None,
    })
}

/// The palette entries a sprite's frames use (the transparent 255 left out).
pub fn used_entries(frames: &[crate::assets::Indexed]) -> Vec<u8> {
    let mut used = [false; 256];
    for f in frames {
        for &c in &f.idx {
            used[c as usize] = true;
        }
    }
    (0..255u8).filter(|&c| used[c as usize]).collect()
}

/// Recolours a picture drawn in the colours of `from` into those of `to` (a palette variant): every pixel (or, with a
/// mask, every pixel the mask covers) moves by the change of its nearest entry among `used`.
pub fn recolour_variant(img: &mut Rgba, mask: Option<&Rgba>, from: &[u8; 768], to: &[u8; 768], used: &[u8]) {
    let changed: Vec<u8> =
        used.iter().copied().filter(|&e| from[e as usize * 3..e as usize * 3 + 3] != to[e as usize * 3..e as usize * 3 + 3]).collect();
    if changed.is_empty() {
        return;
    }
    let table = nearest_table(from, used);
    let mask = mask.filter(|m| (m.w, m.h) == (img.w, img.h));
    for (i, p) in img.px.chunks_exact_mut(4).enumerate() {
        if p[3] == 0 || mask.is_some_and(|m| m.px[i * 4 + 3] < 128) {
            continue;
        }
        shift(p, &table, from, to);
    }
}

// ---- sheets ------------------------------------------------------------------------------------------------------------------

/// Nearest-colour tables by (outfit part's first entry, female).
type Tables = std::collections::HashMap<(u8, bool), std::rc::Rc<Vec<u8>>>;

/// A sprite given as a sheet, at the pack's resolution.
pub struct SheetSprite {
    pub scale: u32,
    pub grid: Grid,
    /// The whole sheet (already in the variant's colours for a palette variant).
    pub sheet: Rgba,
    /// `<name>.mask.png`, for people's outfits.
    pub mask: Option<Rgba>,
    /// The palette the original's frames are drawn in, and the entries they use (for people's outfits).
    pub base_pal: [u8; 768],
    pub used: Vec<u8>,
    /// Golfers and celebrities, recoloured per person.
    pub people: bool,
    /// Lookup tables of the outfit parts, made on first use: (part start, female) -> table.
    tables: std::cell::RefCell<Tables>,
}

impl SheetSprite {
    /// Frame `i` at the pack's resolution.
    pub fn frame(&self, i: usize) -> Option<Rgba> {
        slice(&self.sheet, &self.grid, self.scale, i)
    }

    /// Frame `i` in the colours of a person's outfit palette (`pal`, sg_core::bodies::Swaps::compose).
    pub fn outfit_frame(&self, i: usize, pal: &[u8; 768], female: bool) -> Option<Rgba> {
        let mut img = self.frame(i)?;
        let mask = self.mask.as_ref().and_then(|m| slice(m, &self.grid, self.scale, i));
        let table = |part: Option<std::ops::Range<u8>>| {
            let key = (part.as_ref().map_or(255, |r| r.start), female);
            self.tables
                .borrow_mut()
                .entry(key)
                .or_insert_with(|| {
                    let cands: Vec<u8> = match &part {
                        Some(r) => {
                            let inside: Vec<u8> = self.used.iter().copied().filter(|e| r.contains(e)).collect();
                            if inside.is_empty() {
                                r.clone().collect()
                            } else {
                                inside
                            }
                        }
                        None => self.used.clone(),
                    };
                    std::rc::Rc::new(nearest_table(&self.base_pal, &cands))
                })
                .clone()
        };
        let all = mask.is_none().then(|| table(None));
        for (k, p) in img.px.chunks_exact_mut(4).enumerate() {
            if p[3] == 0 {
                continue;
            }
            let t = match &mask {
                Some(m) => match people_part(&m.px[k * 4..k * 4 + 4], female) {
                    Some(r) => table(Some(r)),
                    None => continue,
                },
                None => all.clone().unwrap_or_default(),
            };
            shift(p, &t, &self.base_pal, pal);
        }
        Some(img)
    }
}

/// What a sheet is checked against: the sprite it replaces (as loaded from the file under the pack), and the original's
/// layout from the free set's manifest when that differs (a sheet drawn for the original's frames also fits over the free
/// set, whose buildings have frames of their own).
pub fn sheet_grid(img: (u32, u32), scale: u32, base: &Grid, reference: Option<Grid>, json: Option<&SheetJson>) -> Result<Grid, String> {
    let want = |g: &Grid| g.sheet_size(scale);
    if let Some(j) = json {
        let g = j.apply(base);
        return if want(&g) == img { Ok(g) } else { Err(size_msg(img, &g, scale)) };
    }
    if want(base) == img {
        return Ok(*base);
    }
    if let Some(r) = reference.filter(|r| want(r) == img) {
        return Ok(r);
    }
    Err(size_msg(img, base, scale))
}

fn size_msg(img: (u32, u32), g: &Grid, scale: u32) -> String {
    let (w, h) = g.sheet_size(scale);
    format!(
        "the sheet is {}x{}, it should be {w}x{h} ({} frames of {}x{} across, {} views down, at {scale}x)",
        img.0,
        img.1,
        g.frames,
        g.w * scale,
        g.h * scale,
        g.views
    )
}

/// The sprite at `flc` made from a pack's sheet, when the topmost pack that has anything for it has a sheet: the sprite
/// (`base` with the sheet's layout and its frames scaled down to the original's size, for everything that works at that
/// size) and the sheet itself. `pal` is the colour variant's palette file. Errors say what is wrong with the sheet; None
/// when there is no sheet.
pub fn sheet_sprite(flc: &Path, pal: Option<&Path>, base: &Sprite) -> Option<Result<(Sprite, SheetSprite), String>> {
    let art = crate::vfs::art(flc).filter(|a| a.sheet)?;
    Some(build_sheet(flc, pal, base, &art))
}

fn build_sheet(flc: &Path, pal: Option<&Path>, base: &Sprite, art: &crate::vfs::Art) -> Result<(Sprite, SheetSprite), String> {
    let stem = art.rel.strip_suffix(".flc").unwrap_or(&art.rel).to_string();
    let mut sheet = crate::png::decode_image(&art.data).ok_or("not a PNG this game reads (8 bits per channel, not interlaced)")?;
    let json = match crate::vfs::layer_file(art.layer, &format!("{stem}.json")) {
        Some(d) => Some(SheetJson::parse(&String::from_utf8_lossy(&d)).map_err(|e| format!("{stem}.json: {e}"))?),
        None => None,
    };
    let reference = crate::freeset::reference(&format!("{stem}.flc")).and_then(Grid::of_reference);
    let grid = sheet_grid((sheet.w, sheet.h), art.scale, &Grid::of(base), reference, json.as_ref())?;
    // the palette the original's frames are drawn in, and the colours they use
    let flc_data = crate::fsutil::read_file(flc).and_then(|d| crate::flc::decode_flc(&d).ok());
    let base_pal = flc_data.as_ref().and_then(|f| f.frames.first()).map(|f| f.pal).unwrap_or([0; 768]);
    let used = flc_data.as_ref().map(|f| used_entries(&f.frames)).unwrap_or_default();
    let mask = crate::vfs::layer_file(art.layer, &format!("{stem}.mask.png"))
        .and_then(|d| crate::png::decode_image(&d))
        .filter(|m| (m.w, m.h) == (sheet.w, sheet.h));
    if let Some(pal) = pal {
        let name = pal.file_stem().map(|n| n.to_string_lossy().to_lowercase()).unwrap_or_default();
        let by_hand = crate::vfs::layer_file(art.layer, &format!("{stem}~{name}.png"))
            .and_then(|d| crate::png::decode_image(&d))
            .filter(|v| (v.w, v.h) == (sheet.w, sheet.h));
        match by_hand {
            Some(v) => sheet = v,
            None => {
                if let Some(to) = crate::fsutil::read_file(pal).and_then(|d| crate::assets::read_pcx_palette(&d)) {
                    recolour_variant(&mut sheet, mask.as_ref(), &base_pal, &to, &used);
                }
            }
        }
    }
    let lower = flc.to_string_lossy().to_lowercase().replace('\\', "/");
    let people = !base.shadow && ["/male/", "/female/", "/celebs/"].iter().any(|d| lower.contains(d));
    let small = downscale(&sheet, art.scale);
    let frames = (0..grid.count()).filter_map(|i| slice(&small, &grid, 1, i)).map(crate::sprites::Frame::Rgba).collect();
    let mut s = base.clone();
    s.views = grid.views as i32;
    s.frames_per_view = grid.frames as i32;
    (s.w, s.h, s.anchor_x, s.anchor_y) = (grid.w, grid.h, grid.ax, grid.ay);
    if let Some(ms) = json.as_ref().and_then(|j| j.frame_ms) {
        s.frame_ms = ms;
    }
    s.frames = frames;
    let sheet = SheetSprite { scale: art.scale, grid, sheet, mask, base_pal, used, people, tables: Default::default() };
    Ok((s, sheet))
}

#[cfg(test)]
mod tests {
    use super::*;

    fn grid() -> Grid {
        Grid { views: 2, frames: 3, w: 4, h: 5, ax: 2, ay: 4 }
    }

    /// A sheet where every pixel says which frame it belongs to.
    fn numbered(g: &Grid, s: u32) -> Rgba {
        let (w, h) = g.sheet_size(s);
        let mut img = Rgba::new(w, h);
        for y in 0..h {
            for x in 0..w {
                let i = (y / (g.h * s)) * g.frames + x / (g.w * s);
                let o = ((y * w + x) * 4) as usize;
                img.px[o..o + 4].copy_from_slice(&[i as u8 * 10, (x % (g.w * s)) as u8, (y % (g.h * s)) as u8, 255]);
            }
        }
        img
    }

    #[test]
    fn slicing_a_sheet() {
        let g = grid();
        assert_eq!(g.sheet_size(4), (48, 40));
        let sheet = numbered(&g, 4);
        for i in 0..6 {
            let f = slice(&sheet, &g, 4, i).unwrap();
            assert_eq!((f.w, f.h), (16, 20));
            assert!(f.px.chunks(4).all(|p| p[0] == i as u8 * 10));
            assert_eq!(&f.px[..3], &[i as u8 * 10, 0, 0]);
            let last = f.px.len() - 4;
            assert_eq!(&f.px[last..last + 3], &[i as u8 * 10, 15, 19]);
        }
        assert!(slice(&sheet, &g, 4, 6).is_none(), "past the last frame");
        assert!(slice(&sheet, &g, 2, 0).is_none(), "a sheet of another scale");
    }

    #[test]
    fn scaling_keeps_the_layout() {
        let g = grid();
        let sheet = numbered(&g, 4);
        let small = downscale(&sheet, 4);
        assert_eq!((small.w, small.h), g.sheet_size(1));
        assert_eq!(slice(&small, &g, 1, 5).unwrap().px[0], 50);
        // transparent pixels do not darken the colour, and alpha averages
        let mut img = Rgba::new(2, 2);
        img.px.copy_from_slice(&[200, 100, 50, 255, 0, 0, 0, 0, 200, 100, 50, 255, 0, 0, 0, 0]);
        assert_eq!(downscale(&img, 2).px, [200, 100, 50, 128]);
        let big = upscale_nearest(&small, 4);
        assert_eq!((big.w, big.h), (sheet.w, sheet.h));
        assert_eq!(downscale(&big, 4).px, small.px);
    }

    #[test]
    fn grid_from_the_original_or_a_json() {
        let base = grid();
        assert_eq!(sheet_grid((48, 40), 4, &base, None, None), Ok(base));
        let err = sheet_grid((40, 40), 4, &base, None, None).unwrap_err();
        assert!(err.contains("48x40"), "{err}");
        // a sheet drawn for the original's layout fits over a base set with other frames
        let other = Grid { w: 6, ..base };
        assert_eq!(sheet_grid((48, 40), 4, &other, Some(base), None), Ok(base));
        let j = SheetJson::parse(r#"{"frames": 4, "anchor": [1, 1], "frame_ms": 50}"#).unwrap();
        let g = sheet_grid((64, 40), 4, &base, None, Some(&j)).unwrap();
        assert_eq!((g.frames, g.ax, g.ay, g.w), (4, 1, 1, 4));
        assert!(SheetJson::parse(r#"{"frames": 0}"#).is_err());
        assert!(SheetJson::parse(r#"{"colour": 1}"#).is_err());
    }

    #[test]
    fn header_sizes() {
        let img = Rgba::new(7, 3);
        assert_eq!(picture_size(&crate::png::encode_png(&img).unwrap()), Some((7, 3)));
        let pcx = crate::assets::encode_pcx(&crate::assets::index_colours(&img));
        assert_eq!(picture_size(&pcx), Some((7, 3)));
        assert_eq!(picture_size(b"nothing"), None);
    }

    #[test]
    fn recolouring_by_the_nearest_colour() {
        let mut from = [0u8; 768];
        let mut to = [0u8; 768];
        // entry 1 green leaves turning red, entry 2 brown trunk staying
        from[3..6].copy_from_slice(&[40, 160, 40]);
        to[3..6].copy_from_slice(&[200, 40, 30]);
        from[6..9].copy_from_slice(&[100, 70, 40]);
        to[6..9].copy_from_slice(&[100, 70, 40]);
        let mut img = Rgba::new(2, 1);
        img.px.copy_from_slice(&[50, 170, 50, 255, 110, 72, 36, 255]);
        let mut masked = img.clone();
        recolour_variant(&mut img, None, &from, &to, &[1, 2]);
        assert_eq!(&img.px[..4], &[210, 50, 40, 255], "a leaf colour takes the leaves' change");
        assert_eq!(&img.px[4..], &[110, 72, 36, 255], "the trunk keeps its colour");
        // a mask that covers nothing keeps everything
        let mask = Rgba::new(2, 1);
        recolour_variant(&mut masked, Some(&mask), &from, &to, &[1, 2]);
        assert_eq!(&masked.px[..4], &[50, 170, 50, 255]);
        assert_eq!(people_part(&[255, 0, 0, 255], false), Some(0..20));
        assert_eq!(people_part(&[0, 255, 255, 255], true), Some(90..97));
        assert_eq!(people_part(&[0, 255, 255, 255], false), Some(80..90));
        assert_eq!(people_part(&[0, 0, 0, 255], false), None);
        assert_eq!(people_part(&[255, 0, 0, 0], false), None);
    }

    #[test]
    fn outfits_recolour_the_masked_parts() {
        let g = Grid { views: 1, frames: 1, w: 2, h: 1, ax: 1, ay: 1 };
        let mut base_pal = [0u8; 768];
        base_pal[0..3].copy_from_slice(&[200, 0, 0]); // shirt
        base_pal[60..63].copy_from_slice(&[0, 0, 200]); // trousers (entry 20)
        let mut sheet = Rgba::new(2, 1);
        sheet.px.copy_from_slice(&[190, 10, 10, 255, 10, 10, 190, 255]);
        let mut mask = Rgba::new(2, 1);
        mask.px.copy_from_slice(&[255, 0, 0, 255, 0, 0, 0, 0]);
        let s = SheetSprite {
            scale: 1,
            grid: g,
            sheet,
            mask: Some(mask),
            base_pal,
            used: vec![0, 20],
            people: true,
            tables: Default::default(),
        };
        let mut outfit = base_pal;
        outfit[0..3].copy_from_slice(&[0, 200, 0]);
        outfit[60..63].copy_from_slice(&[200, 200, 0]);
        let f = s.outfit_frame(0, &outfit, false).unwrap();
        assert_eq!(&f.px[..4], &[0, 210, 10, 255], "the shirt takes the outfit's shirt");
        assert_eq!(&f.px[4..], &[10, 10, 190, 255], "unmasked pixels keep their colour");
    }
}
