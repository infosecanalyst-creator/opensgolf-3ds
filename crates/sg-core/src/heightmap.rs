//! Heightmap import for the course editor (the port's own; docs/PLAYING.md, "Course editor"): a grey picture of any size is
//! resampled to the map's corner grid with bilinear filtering, its brightness mapped to the corner levels the Elevation tools
//! use, optionally smoothed, and the tiles whose corners all lie at or under a chosen water level become water. Water lies
//! at sea level, as the original's land generator and load path have it (golf.exe leaves water at sea level).
//!
//! Levels are the port's terrain levels (crate::terrain: the exe's corner heights less 3), so 0 is sea level and
//! `terrain::EXE_HIGH - terrain::EXE_SEA` (10) the highest corner the Elevation tools make. Black is the lowest level asked
//! for, white the highest. The picture is stretched over the whole map: its top-left pixel is corner (0, 0), its
//! bottom-right pixel corner (w, h).
use crate::terrain::{Terrain, EXE_HIGH, EXE_SEA};

/// The highest level the import makes (the Elevation tools' top, the exe's 13).
pub const TOP: i32 = EXE_HIGH - EXE_SEA;

/// A grey picture: brightness 0.0 (black) ..= 1.0 (white) per pixel, rows top to bottom.
#[derive(Clone, Debug, PartialEq)]
pub struct Grey {
    pub w: usize,
    pub h: usize,
    pub v: Vec<f32>,
}

impl Grey {
    /// Reads a PNG (8 or 16 bits per channel, grey, colour or palette; a colour picture counts by its luma).
    pub fn from_png(bytes: &[u8]) -> Result<Grey, String> {
        let (w, h, v) = crate::png::decode_png_grey(bytes).ok_or_else(|| crate::lang::tr("STR_EDITOR_HM_NOT_PNG").to_string())?;
        Ok(Grey { w, h, v })
    }

    fn at(&self, x: usize, y: usize) -> f32 {
        self.v[y.min(self.h - 1) * self.w + x.min(self.w - 1)]
    }

    /// The picture resampled to `cols` x `rows` points with bilinear filtering; the first and last points of each row and
    /// column fall on the picture's edge pixels.
    pub fn resample(&self, cols: usize, rows: usize) -> Vec<f32> {
        let mut out = Vec::with_capacity(cols * rows);
        let span = |n: usize, size: usize| if n > 1 { (size - 1) as f32 / (n - 1) as f32 } else { 0.0 };
        let (sx, sy) = (span(cols, self.w), span(rows, self.h));
        for j in 0..rows {
            let fy = j as f32 * sy;
            let (y0, ty) = (fy.floor() as usize, fy.fract());
            for i in 0..cols {
                let fx = i as f32 * sx;
                let (x0, tx) = (fx.floor() as usize, fx.fract());
                let top = self.at(x0, y0) * (1.0 - tx) + self.at(x0 + 1, y0) * tx;
                let bottom = self.at(x0, y0 + 1) * (1.0 - tx) + self.at(x0 + 1, y0 + 1) * tx;
                out.push(top * (1.0 - ty) + bottom * ty);
            }
        }
        out
    }
}

/// One smoothing pass over a `cols` x `rows` grid: the 3 x 3 binomial kernel (1 2 1), the edges repeating outwards.
pub fn smooth(v: &mut [f32], cols: usize, rows: usize) {
    let src = v.to_vec();
    let at = |x: isize, y: isize| src[(y.clamp(0, rows as isize - 1) as usize) * cols + x.clamp(0, cols as isize - 1) as usize];
    for y in 0..rows as isize {
        for x in 0..cols as isize {
            let mut sum = 0.0;
            for (dy, wy) in [(-1, 1.0), (0, 2.0), (1, 1.0)] {
                for (dx, wx) in [(-1, 1.0), (0, 2.0), (1, 1.0)] {
                    sum += at(x + dx, y + dy) * wy * wx;
                }
            }
            v[y as usize * cols + x as usize] = sum / 16.0;
        }
    }
}

/// What the import does with the picture.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct Options {
    /// The levels black and white become (0 ..= TOP, low below high).
    pub low: i32,
    pub high: i32,
    /// The water level: tiles whose four corners all lie at or under it become water, their corners at sea level. None:
    /// no water.
    pub water: Option<i32>,
    /// One smoothing pass before the levels are rounded.
    pub smooth: bool,
}

impl Default for Options {
    fn default() -> Options {
        Options { low: 0, high: 7, water: None, smooth: true }
    }
}

/// The corner levels of a `w` x `h` map ((w + 1) x (h + 1), row by row) made from the picture.
pub fn levels(img: &Grey, w: i32, h: i32, o: &Options) -> Vec<i8> {
    let (cols, rows) = ((w + 1) as usize, (h + 1) as usize);
    let mut v = img.resample(cols, rows);
    if o.smooth {
        smooth(&mut v, cols, rows);
    }
    let (low, high) = (o.low.clamp(0, TOP), o.high.clamp(0, TOP));
    let (low, high) = (low.min(high), high.max(low));
    v.iter().map(|&b| (low as f32 + b.clamp(0.0, 1.0) * (high - low) as f32).round() as i8).collect()
}

/// The water tiles of a map with these corner levels: (x, y, depth), the depth 0 shallow, 1 middle, 2 deep by how far the
/// tile's lowest corner lies under the water level.
pub fn water_tiles(corner: &[i8], w: i32, h: i32, surface: i32) -> Vec<(i32, i32, u8)> {
    let at = |x: i32, y: i32| corner[(y * (w + 1) + x) as usize] as i32;
    let mut out = Vec::new();
    for y in 0..h {
        for x in 0..w {
            let c = [at(x, y), at(x + 1, y), at(x, y + 1), at(x + 1, y + 1)];
            if c.iter().all(|&l| l <= surface) {
                let low = *c.iter().min().unwrap_or(&surface);
                out.push((x, y, (surface - low).clamp(0, 2) as u8));
            }
        }
    }
    out
}

/// What an import did: the water tiles it found (including those `fixed` kept from turning into water), and how many
/// tiles became water.
#[derive(Clone, Debug, Default, PartialEq)]
pub struct Report {
    pub water: Vec<(i32, i32, u8)>,
    pub made_water: usize,
}

/// Lays the picture over the terrain: every corner takes its level, and the water tiles become water (shallow, middle or
/// deep) with their corners at sea level, unless `fixed(x, y)` says the tile's type must stay (a building's footprint,
/// a hole's tee or green, land not owned).
pub fn apply(t: &mut Terrain, img: &Grey, o: &Options, fixed: &dyn Fn(i32, i32) -> bool) -> Report {
    t.corner = levels(img, t.w, t.h, o);
    let mut r = Report::default();
    if let Some(s) = o.water {
        r.water = water_tiles(&t.corner, t.w, t.h, s);
        for &(x, y, depth) in &r.water {
            if fixed(x, y) {
                continue;
            }
            t.paint(x, y, crate::terrain::TT_WATER_SHALLOW as i32, depth as i32);
            for (cx, cy) in [(x, y), (x + 1, y), (x, y + 1), (x + 1, y + 1)] {
                t.corner[(cy * (t.w + 1) + cx) as usize] = 0;
            }
            r.made_water += 1;
        }
    }
    r
}

/// Flattens regions of tiles to one level each: the 4-connected tiles with the same `key(x, y)` (None: not part of any
/// region) take the rounded mean of their corners. Used after an import for buildings' footprints and holes' tees and
/// greens, which must stay level.
pub fn flatten_regions(t: &mut Terrain, key: &dyn Fn(i32, i32) -> Option<u32>) {
    let (w, h) = (t.w, t.h);
    let mut seen = vec![false; (w * h) as usize];
    for y0 in 0..h {
        for x0 in 0..w {
            let Some(k) = key(x0, y0) else { continue };
            if seen[(y0 * w + x0) as usize] {
                continue;
            }
            seen[(y0 * w + x0) as usize] = true;
            let mut region = vec![(x0, y0)];
            let mut i = 0;
            while i < region.len() {
                let (x, y) = region[i];
                i += 1;
                for (nx, ny) in [(x + 1, y), (x - 1, y), (x, y + 1), (x, y - 1)] {
                    if t.inside(nx, ny) && !seen[(ny * w + nx) as usize] && key(nx, ny) == Some(k) {
                        seen[(ny * w + nx) as usize] = true;
                        region.push((nx, ny));
                    }
                }
            }
            let mut corners: Vec<usize> = region
                .iter()
                .flat_map(|&(x, y)| [(x, y), (x + 1, y), (x, y + 1), (x + 1, y + 1)])
                .map(|(cx, cy)| (cy * (w + 1) + cx) as usize)
                .collect();
            corners.sort_unstable();
            corners.dedup();
            let sum: i32 = corners.iter().map(|&c| t.corner[c] as i32).sum();
            let n = corners.len() as i32;
            let level = ((2 * sum + n) / (2 * n)) as i8;
            for c in corners {
                t.corner[c] = level;
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    /// A grey PNG of `depth` bits from rows of values (0..=255 or 0..=65535), unfiltered.
    pub(crate) fn grey_png(w: usize, h: usize, depth: u8, px: &[u32]) -> Vec<u8> {
        let mut raw = Vec::new();
        for y in 0..h {
            raw.push(0u8);
            for x in 0..w {
                let v = px[y * w + x];
                if depth == 16 {
                    raw.extend_from_slice(&(v as u16).to_be_bytes());
                } else {
                    raw.push(v as u8);
                }
            }
        }
        let z = miniz_oxide::deflate::compress_to_vec_zlib(&raw, 6);
        let mut png = vec![0x89, b'P', b'N', b'G', 0x0D, 0x0A, 0x1A, 0x0A];
        let mut chunk = |ty: &[u8], body: &[u8]| {
            png.extend_from_slice(&(body.len() as u32).to_be_bytes());
            png.extend_from_slice(ty);
            png.extend_from_slice(body);
            png.extend_from_slice(&[0; 4]);
        };
        let mut ihdr = Vec::new();
        ihdr.extend_from_slice(&(w as u32).to_be_bytes());
        ihdr.extend_from_slice(&(h as u32).to_be_bytes());
        ihdr.extend_from_slice(&[depth, 0, 0, 0, 0]);
        chunk(b"IHDR", &ihdr);
        chunk(b"IDAT", &z);
        chunk(b"IEND", &[]);
        png
    }

    fn flat(w: i32, h: i32) -> Terrain {
        let mut t = Terrain::demo_course(w, h, 1);
        t.ty.iter_mut().for_each(|v| *v = crate::terrain::TT_ROUGH);
        t.corner.iter_mut().for_each(|v| *v = 0);
        t
    }

    #[test]
    fn pngs_of_8_and_16_bits_are_read() {
        let g = Grey::from_png(&grey_png(2, 1, 8, &[0, 255])).unwrap();
        assert_eq!((g.w, g.h), (2, 1));
        assert_eq!(g.v, vec![0.0, 1.0]);
        let g = Grey::from_png(&grey_png(3, 1, 16, &[0, 32768, 65535])).unwrap();
        assert!((g.v[1] - 0.5).abs() < 0.001 && g.v[2] == 1.0);
        // the 8-bit decoders still refuse 16 bits
        assert!(crate::png::decode_png(&grey_png(3, 1, 16, &[0, 1, 2])).is_none());
        assert!(Grey::from_png(b"not a picture").is_err());
        // a colour picture counts by its luma
        let mut img = crate::assets::Rgba::new(1, 1);
        img.px.copy_from_slice(&[255, 255, 255, 255]);
        let g = Grey::from_png(&crate::png::encode_png(&img).unwrap()).unwrap();
        assert!((g.v[0] - 1.0).abs() < 0.001);
    }

    #[test]
    fn resampling_is_bilinear_and_keeps_the_edges() {
        // 2 x 2: black on the left, white on the right
        let g = Grey { w: 2, h: 2, v: vec![0.0, 1.0, 0.0, 1.0] };
        let r = g.resample(3, 3);
        assert_eq!(r, vec![0.0, 0.5, 1.0, 0.0, 0.5, 1.0, 0.0, 0.5, 1.0]);
        // a larger picture shrinks: the corners are the picture's corners
        let big = Grey { w: 100, h: 60, v: (0..6000).map(|i| (i % 100) as f32 / 99.0).collect() };
        let r = big.resample(51, 51);
        assert_eq!(r.len(), 51 * 51);
        assert_eq!(r[0], 0.0);
        assert!((r[50] - 1.0).abs() < 1e-5);
        assert!((r[25] - 0.5).abs() < 0.01);
        // a 1 x 1 picture fills everything
        assert_eq!(Grey { w: 1, h: 1, v: vec![0.25] }.resample(4, 2), vec![0.25; 8]);
    }

    #[test]
    fn brightness_maps_to_the_levels() {
        let black = Grey { w: 1, h: 1, v: vec![0.0] };
        let white = Grey { w: 1, h: 1, v: vec![1.0] };
        let o = Options { low: 2, high: 9, water: None, smooth: false };
        assert!(levels(&black, 4, 4, &o).iter().all(|&l| l == 2));
        assert!(levels(&white, 4, 4, &o).iter().all(|&l| l == 9));
        // out of range asks are held to the Elevation tools' range, and swapped when given the wrong way round
        let o = Options { low: 40, high: -5, water: None, smooth: false };
        assert!(levels(&white, 2, 2, &o).iter().all(|&l| l as i32 == TOP));
        assert!(levels(&black, 2, 2, &o).iter().all(|&l| l == 0));
        // a ramp gives every level in between, in order
        let ramp = Grey { w: 11, h: 1, v: (0..11).map(|i| i as f32 / 10.0).collect() };
        let l = levels(&ramp, 10, 1, &Options { low: 0, high: 10, water: None, smooth: false });
        assert_eq!(&l[..11], &[0, 1, 2, 3, 4, 5, 6, 7, 8, 9, 10]);
    }

    #[test]
    fn smoothing_softens_steps() {
        let mut v = vec![0.0; 25];
        v[12] = 16.0;
        smooth(&mut v, 5, 5);
        assert_eq!(v[12], 4.0);
        assert_eq!(v[11], 2.0);
        assert_eq!(v[6], 1.0);
        assert_eq!(v[0], 0.0);
        assert_eq!(v.iter().sum::<f32>(), 16.0);
        // a flat grid stays flat
        let mut f = vec![3.0; 12];
        smooth(&mut f, 4, 3);
        assert!(f.iter().all(|&x| (x - 3.0).abs() < 1e-6));
    }

    #[test]
    fn water_fills_the_low_tiles() {
        // a valley: black in the middle column, white at the sides
        let g = Grey { w: 3, h: 1, v: vec![1.0, 0.0, 1.0] };
        let mut t = flat(10, 4);
        let o = Options { low: 0, high: 8, water: Some(2), smooth: false };
        let r = apply(&mut t, &g, &o, &|_, _| false);
        assert!(r.made_water > 0 && r.made_water == r.water.len());
        for &(x, y, depth) in &r.water {
            assert_eq!(t.type_at(x, y), crate::terrain::TT_WATER_SHALLOW as i32);
            assert_eq!(t.variation[t.tile_index(x, y)], depth);
            assert!(t.tile_levels(x, y).iter().all(|&l| l == 0), "the water's corners at sea level");
        }
        // the middle is water, the high sides are not
        assert_eq!(t.type_at(4, 1), crate::terrain::TT_WATER_SHALLOW as i32);
        assert_eq!(t.type_at(0, 1), crate::terrain::TT_ROUGH as i32);
        assert_eq!(t.type_at(9, 1), crate::terrain::TT_ROUGH as i32);
        // the deepest tile, whose corners were at 0, is deep
        assert!(r.water.iter().any(|&(_, _, d)| d == 2));
        // a fixed tile keeps its type
        let mut t2 = flat(10, 4);
        let r2 = apply(&mut t2, &g, &o, &|x, _| x == 5);
        assert_eq!(r2.made_water + 4, r2.water.len());
        assert_eq!(t2.type_at(5, 0), crate::terrain::TT_ROUGH as i32);
        // no water level: nothing turns to water
        let mut t3 = flat(10, 4);
        let r3 = apply(&mut t3, &g, &Options { water: None, ..o }, &|_, _| false);
        assert!(r3.water.is_empty() && t3.ty.iter().all(|&ty| ty == crate::terrain::TT_ROUGH));
    }

    #[test]
    fn regions_are_flattened_one_level_each() {
        let mut t = flat(4, 1);
        t.corner = vec![0, 1, 2, 5, 9, 0, 1, 2, 5, 9];
        // tiles 0 and 1 one region, tile 3 another, tile 2 free
        flatten_regions(&mut t, &|x, _| match x {
            0 | 1 => Some(1),
            3 => Some(2),
            _ => None,
        });
        assert_eq!(t.tile_levels(0, 0), [1; 4]);
        assert_eq!(t.tile_levels(1, 0), [1; 4]);
        assert_eq!(t.tile_levels(3, 0), [7; 4]);
        assert_eq!(t.tile_levels(2, 0), [1, 7, 1, 7]);
    }

    #[test]
    fn the_whole_map_takes_the_picture() {
        let _size = crate::mapsize::scoped(74);
        let g = Grey::from_png(&grey_png(5, 3, 8, &[0, 64, 128, 192, 255, 0, 64, 128, 192, 255, 0, 64, 128, 192, 255])).unwrap();
        let mut t = flat(74, 74);
        let r = apply(&mut t, &g, &Options { low: 0, high: TOP, water: Some(0), smooth: true }, &|_, _| false);
        assert_eq!(t.corner.len(), 75 * 75);
        // left low, right high, and every level inside the tools' range
        assert!(t.corner_at(0, 30) < t.corner_at(74, 30));
        assert!(t.corner.iter().all(|&l| (0..=TOP).contains(&(l as i32))));
        assert!(r.made_water > 0);
    }
}
