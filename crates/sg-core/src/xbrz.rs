//! xBRZ: a pixel-art upscaler (the built-in HD maker, docs/HD.md). Pure computation, no files: it runs the same natively and
//! in a browser.
//!
//! Provenance: written for OpenSGolf from the published description of xBRZ ("xBRZ: 'Scale by rules' - high quality image
//! upscaling filter by Zenju", version 1.8, <https://sourceforge.net/projects/xbrz/>, GPL-3.0; itself derived from Hyllian's
//! xBR). It follows that algorithm step for step: the same 4x4 corner pre-pass deciding which corners of each pixel are
//! blended (and how strongly), the same rules for line, steep, shallow and corner blends in a 3x3 kernel rotated four
//! ways, the same YCbCr (BT.2020) colour distance and thresholds (luminance weight 1, equal colour tolerance 30, dominant
//! direction 3.6, steep direction 2.2), and its ARGB variant: the alpha channel takes part in the distance, and blends
//! weight the colours by their alpha, so a fully transparent pixel's colour never shows (no fringes of a colour key).
//! Recorded in README.md (Licence).

use crate::assets::Rgba;

const LUMINANCE_WEIGHT: f32 = 1.0;
const EQUAL_COLOR_TOLERANCE: f32 = 30.0;
const DOMINANT_DIRECTION_THRESHOLD: f32 = 3.6;
const STEEP_DIRECTION_THRESHOLD: f32 = 2.2;

const BLEND_NONE: u8 = 0;
const BLEND_NORMAL: u8 = 1;
const BLEND_DOMINANT: u8 = 2;

/// The factors implemented.
pub const FACTORS: [u32; 3] = [2, 3, 4];

#[inline]
fn ch(p: u32, shift: u32) -> i32 {
    ((p >> shift) & 0xFF) as i32
}

#[inline]
fn alpha(p: u32) -> i32 {
    ch(p, 24)
}

/// The colour distance of xBRZ: the length of the difference in (analog) YCbCr, BT.2020 weights, not divided by 255.
#[inline]
fn dist_ycbcr(p1: u32, p2: u32) -> f32 {
    let r = (ch(p1, 16) - ch(p2, 16)) as f32;
    let g = (ch(p1, 8) - ch(p2, 8)) as f32;
    let b = (ch(p1, 0) - ch(p2, 0)) as f32;
    const K_B: f32 = 0.0593;
    const K_R: f32 = 0.2627;
    const K_G: f32 = 1.0 - K_B - K_R;
    const SCALE_B: f32 = 0.5 / (1.0 - K_B);
    const SCALE_R: f32 = 0.5 / (1.0 - K_R);
    let y = K_R * r + K_G * g + K_B * b;
    let cb = SCALE_B * (b - y);
    let cr = SCALE_R * (r - y);
    ((LUMINANCE_WEIGHT * y).powi(2) + cb * cb + cr * cr).sqrt()
}

/// The ARGB distance: the colour distance weighted by the smaller alpha plus the difference of the alphas (xBRZ's
/// ColorDistanceARGB).
#[inline]
fn dist(p1: u32, p2: u32) -> f32 {
    if p1 == p2 {
        return 0.0;
    }
    let a1 = alpha(p1) as f32 / 255.0;
    let a2 = alpha(p2) as f32 / 255.0;
    let d = if a1.min(a2) > 0.0 { dist_ycbcr(p1, p2) } else { 0.0 };
    if a1 < a2 {
        a1 * d + 255.0 * (a2 - a1)
    } else {
        a2 * d + 255.0 * (a1 - a2)
    }
}

#[inline]
fn eq(p1: u32, p2: u32) -> bool {
    dist(p1, p2) < EQUAL_COLOR_TOLERANCE
}

/// `back` moved M/N of the way to `front`, the colours weighted by their alpha (xBRZ's gradientARGB: no alpha blending).
#[inline]
fn grad(back: &mut u32, front: u32, m: u32, n: u32) {
    let wf = alpha(front) as u32 * m;
    let wb = alpha(*back) as u32 * (n - m);
    let sum = wf + wb;
    if sum == 0 {
        *back = 0;
        return;
    }
    let c = |s: u32| ((((front >> s) & 0xFF) * wf + ((*back >> s) & 0xFF) * wb) / sum) & 0xFF;
    *back = (sum / n) << 24 | c(16) << 16 | c(8) << 8 | c(0);
}

/// The blend of the four corners of a 2x2 block F G / J K, from its 4x4 surroundings (xBRZ's preProcessCorners):
/// (f, g, j, k), each BLEND_NONE, BLEND_NORMAL or BLEND_DOMINANT.
///
/// ```text
/// A B C D
/// E F G H
/// I J K L
/// M N O P
/// ```
#[inline]
fn corners(k: &[u32; 16]) -> [u8; 4] {
    let [a, b, c, _d, e, f, g, h, i, j, kk, l, _m, n, o, _p] = *k;
    let _ = a;
    let mut r = [BLEND_NONE; 4];
    if (f == g && j == kk) || (f == j && g == kk) {
        return r;
    }
    const WEIGHT: f32 = 4.0;
    let jg = dist(i, f) + dist(f, c) + dist(n, kk) + dist(kk, h) + WEIGHT * dist(j, g);
    let fk = dist(e, j) + dist(j, o) + dist(b, g) + dist(g, l) + WEIGHT * dist(f, kk);
    if jg < fk {
        let t = if DOMINANT_DIRECTION_THRESHOLD * jg < fk { BLEND_DOMINANT } else { BLEND_NORMAL };
        if f != g && f != j {
            r[0] = t;
        }
        if kk != j && kk != g {
            r[3] = t;
        }
    } else if fk < jg {
        let t = if DOMINANT_DIRECTION_THRESHOLD * fk < jg { BLEND_DOMINANT } else { BLEND_NORMAL };
        if j != f && j != kk {
            r[2] = t;
        }
        if g != f && g != kk {
            r[1] = t;
        }
    }
    r
}

/// The scale x scale block of one source pixel in the target, seen turned by `rot` quarter turns (xBRZ's OutputMatrix).
struct Out<'a> {
    px: &'a mut [u32],
    base: usize,
    stride: usize,
    n: usize,
    rot: u8,
}

impl Out<'_> {
    #[inline]
    fn at(&mut self, i: usize, j: usize) -> &mut u32 {
        let (mut i, mut j) = (i, j);
        for _ in 0..self.rot {
            (i, j) = (self.n - 1 - j, i);
        }
        &mut self.px[self.base + i * self.stride + j]
    }
    #[inline]
    fn g(&mut self, i: usize, j: usize, col: u32, m: u32, n: u32) {
        grad(self.at(i, j), col, m, n);
    }
    #[inline]
    fn set(&mut self, i: usize, j: usize, col: u32) {
        *self.at(i, j) = col;
    }
}

#[derive(Clone, Copy)]
enum Line {
    Shallow,
    Steep,
    SteepAndShallow,
    Diagonal,
    Corner,
}

/// The blends of each factor (xBRZ's Scaler2x, Scaler3x, Scaler4x).
fn blend(out: &mut Out, line: Line, col: u32) {
    let s = out.n;
    match (s, line) {
        (2, Line::Shallow) => {
            out.g(s - 1, 0, col, 1, 4);
            out.g(s - 1, 1, col, 3, 4);
        }
        (2, Line::Steep) => {
            out.g(0, s - 1, col, 1, 4);
            out.g(1, s - 1, col, 3, 4);
        }
        (2, Line::SteepAndShallow) => {
            out.g(1, 0, col, 1, 4);
            out.g(0, 1, col, 1, 4);
            out.g(1, 1, col, 5, 6);
        }
        (2, Line::Diagonal) => out.g(1, 1, col, 1, 2),
        (2, Line::Corner) => out.g(1, 1, col, 21, 100),
        (3, Line::Shallow) => {
            out.g(s - 1, 0, col, 1, 4);
            out.g(s - 2, 2, col, 1, 4);
            out.g(s - 1, 1, col, 3, 4);
            out.set(s - 1, 2, col);
        }
        (3, Line::Steep) => {
            out.g(0, s - 1, col, 1, 4);
            out.g(2, s - 2, col, 1, 4);
            out.g(1, s - 1, col, 3, 4);
            out.set(2, s - 1, col);
        }
        (3, Line::SteepAndShallow) => {
            out.g(2, 0, col, 1, 4);
            out.g(0, 2, col, 1, 4);
            out.g(2, 1, col, 3, 4);
            out.g(1, 2, col, 3, 4);
            out.set(2, 2, col);
        }
        (3, Line::Diagonal) => {
            out.g(1, 2, col, 1, 8);
            out.g(2, 1, col, 1, 8);
            out.g(2, 2, col, 7, 8);
        }
        (3, Line::Corner) => out.g(2, 2, col, 45, 100),
        (_, Line::Shallow) => {
            out.g(s - 1, 0, col, 1, 4);
            out.g(s - 2, 2, col, 1, 4);
            out.g(s - 1, 1, col, 3, 4);
            out.g(s - 2, 3, col, 3, 4);
            out.set(s - 1, 2, col);
            out.set(s - 1, 3, col);
        }
        (_, Line::Steep) => {
            out.g(0, s - 1, col, 1, 4);
            out.g(2, s - 2, col, 1, 4);
            out.g(1, s - 1, col, 3, 4);
            out.g(3, s - 2, col, 3, 4);
            out.set(2, s - 1, col);
            out.set(3, s - 1, col);
        }
        (_, Line::SteepAndShallow) => {
            out.g(3, 1, col, 3, 4);
            out.g(1, 3, col, 3, 4);
            out.g(3, 0, col, 1, 4);
            out.g(0, 3, col, 1, 4);
            out.g(2, 2, col, 1, 3);
            out.set(3, 3, col);
            out.set(3, 2, col);
            out.set(2, 3, col);
        }
        (_, Line::Diagonal) => {
            out.g(s - 1, s / 2, col, 1, 2);
            out.g(s - 2, s / 2 + 1, col, 1, 2);
            out.set(s - 1, s - 1, col);
        }
        (_, Line::Corner) => {
            out.g(3, 3, col, 68, 100);
            out.g(3, 2, col, 9, 100);
            out.g(2, 3, col, 9, 100);
        }
    }
}

/// The bottom right corner of pixel E in a 3x3 kernel (already turned) blended into the target (xBRZ's blendPixel). `info`
/// holds the four corners' blends, turned likewise: bits 0-1 top left, 2-3 top right, 4-5 bottom right, 6-7 bottom left.
fn blend_pixel(k: &[u32; 9], info: u8, out: &mut Out) {
    let [_a, b, c, d, e, f, g, h, i] = *k;
    let bottom_r = (info >> 4) & 3;
    if bottom_r < BLEND_NORMAL {
        return;
    }
    let top_r = (info >> 2) & 3;
    let bottom_l = (info >> 6) & 3;
    let line = bottom_r >= BLEND_DOMINANT || {
        // no second blend in a neighbouring turn of this pixel (insular pixels), but double blends for 90 degree corners
        !((top_r != BLEND_NONE && !eq(e, g)) || (bottom_l != BLEND_NONE && !eq(e, c)))
            // no full blend for L shapes: only the corner
            && !(!eq(e, i) && eq(g, h) && eq(h, i) && eq(i, f) && eq(f, c))
    };
    // the more similar of the two neighbours
    let px = if dist(e, f) <= dist(e, h) { f } else { h };
    if !line {
        return blend(out, Line::Corner, px);
    }
    let fg = dist(f, g);
    let hc = dist(h, c);
    let shallow = STEEP_DIRECTION_THRESHOLD * fg <= hc && e != g && d != g;
    let steep = STEEP_DIRECTION_THRESHOLD * hc <= fg && e != c && b != c;
    let kind = match (shallow, steep) {
        (true, true) => Line::SteepAndShallow,
        (true, false) => Line::Shallow,
        (false, true) => Line::Steep,
        (false, false) => Line::Diagonal,
    };
    blend(out, kind, px);
}

/// A 3x3 kernel turned a quarter (what is to the left becomes the top).
#[inline]
fn turn(k: &[u32; 9]) -> [u32; 9] {
    [k[6], k[3], k[0], k[7], k[4], k[1], k[8], k[5], k[2]]
}

/// Scales ARGB pixels (0xAARRGGBB, row-major) by `factor` (2, 3 or 4); the edges repeat. Fully transparent pixels should all
/// be 0 (their colour is irrelevant and would otherwise count as a difference).
pub fn scale_argb(src: &[u32], w: usize, h: usize, factor: u32) -> Vec<u32> {
    assert!(FACTORS.contains(&factor), "xBRZ factor {factor}");
    assert_eq!(src.len(), w * h);
    let s = factor as usize;
    let tw = w * s;
    let mut out = vec![0u32; tw * h * s];
    if w == 0 || h == 0 {
        return out;
    }
    let at = |x: isize, y: isize| src[y.clamp(0, h as isize - 1) as usize * w + x.clamp(0, w as isize - 1) as usize];
    // the pre-pass: the corners of each 2x2 block whose top left pixel is (x, y)
    let mut res = vec![[BLEND_NONE; 4]; w * h];
    for y in 0..h as isize {
        for x in 0..w as isize {
            let mut k = [0u32; 16];
            for dy in 0..4 {
                for dx in 0..4 {
                    k[(dy * 4 + dx) as usize] = at(x + dx - 1, y + dy - 1);
                }
            }
            res[y as usize * w + x as usize] = corners(&k);
        }
    }
    for y in 0..h {
        for x in 0..w {
            let (xi, yi) = (x as isize, y as isize);
            let e = src[y * w + x];
            let base = y * s * tw + x * s;
            for r in 0..s {
                out[base + r * tw..base + r * tw + s].fill(e);
            }
            // this pixel's four corners: its own block's F, the left block's G, the upper block's J, the upper left's K
            let top_l = if x > 0 && y > 0 { res[(y - 1) * w + x - 1][3] } else { 0 };
            let top_r = if y > 0 { res[(y - 1) * w + x][2] } else { 0 };
            let bottom_r = res[y * w + x][0];
            let bottom_l = if x > 0 { res[y * w + x - 1][1] } else { 0 };
            let mut info = top_l | top_r << 2 | bottom_r << 4 | bottom_l << 6;
            if info == 0 {
                continue;
            }
            let mut k = [
                at(xi - 1, yi - 1),
                at(xi, yi - 1),
                at(xi + 1, yi - 1),
                at(xi - 1, yi),
                e,
                at(xi + 1, yi),
                at(xi - 1, yi + 1),
                at(xi, yi + 1),
                at(xi + 1, yi + 1),
            ];
            for rot in 0..4u8 {
                let mut o = Out { px: &mut out, base, stride: tw, n: s, rot };
                blend_pixel(&k, info, &mut o);
                k = turn(&k);
                info = info.rotate_left(2);
            }
        }
    }
    out
}

/// Packs RGBA bytes as ARGB words, every fully transparent pixel as 0.
pub fn to_argb(img: &Rgba) -> Vec<u32> {
    img.px
        .chunks_exact(4)
        .map(|p| if p[3] == 0 { 0 } else { (p[3] as u32) << 24 | (p[0] as u32) << 16 | (p[1] as u32) << 8 | p[2] as u32 })
        .collect()
}

/// ARGB words back to RGBA bytes.
pub fn from_argb(w: u32, h: u32, px: &[u32]) -> Rgba {
    let mut out = Rgba::new(w, h);
    for (q, &p) in out.px.chunks_exact_mut(4).zip(px) {
        q.copy_from_slice(&[(p >> 16) as u8, (p >> 8) as u8, p as u8, (p >> 24) as u8]);
    }
    out
}

/// An RGBA picture scaled by `factor` (2, 3 or 4) with xBRZ, the edges repeated.
pub fn scale(img: &Rgba, factor: u32) -> Rgba {
    let px = scale_argb(&to_argb(img), img.w as usize, img.h as usize, factor);
    from_argb(img.w * factor, img.h * factor, &px)
}

#[cfg(test)]
mod tests {
    use super::*;

    const R: u32 = 0xFFFF_0000;
    const W: u32 = 0xFFFF_FFFF;
    const K: u32 = 0xFF00_0000;
    const T: u32 = 0;

    #[test]
    fn flat_pictures_stay_flat_at_every_factor() {
        for f in FACTORS {
            let out = scale_argb(&[R; 12], 4, 3, f);
            assert_eq!(out.len(), 12 * (f * f) as usize);
            assert!(out.iter().all(|&p| p == R));
            assert!(scale_argb(&[T; 6], 3, 2, f).iter().all(|&p| p == 0));
        }
    }

    #[test]
    fn a_single_pixel_scales_to_a_block() {
        let out = scale_argb(&[K], 1, 1, 4);
        assert_eq!(out, vec![K; 16]);
    }

    #[test]
    fn a_checkerboard_is_left_alone() {
        // every corner is ambiguous (f == k and g == j): nothing blends, each pixel becomes a block
        let src = [K, W, K, W, W, K, W, K, K, W, K, W, W, K, W, K];
        let out = scale_argb(&src, 4, 4, 2);
        for y in 0..8 {
            for x in 0..8 {
                assert_eq!(out[y * 8 + x], src[(y / 2) * 4 + x / 2], "{x},{y}");
            }
        }
    }

    #[test]
    fn a_diagonal_edge_is_smoothed() {
        // black below the diagonal, white above: the scaled edge gets blended pixels along it, unlike a plain enlargement
        let n = 6;
        let src: Vec<u32> = (0..n * n).map(|i| if i % n <= i / n { K } else { W }).collect();
        let out = scale_argb(&src, n, n, 4);
        let tw = n * 4;
        let mixed = out.iter().filter(|&&p| p != K && p != W).count();
        assert!(mixed > 0, "no blend along the diagonal");
        // only near the diagonal
        for y in 0..tw {
            for x in 0..tw {
                if (x as i32 - y as i32).abs() > 6 {
                    assert!(out[y * tw + x] == K || out[y * tw + x] == W, "{x},{y}");
                }
            }
        }
        // the far corners keep their colours
        assert_eq!(out[tw - 1], W);
        assert_eq!(out[(tw - 1) * tw], K);
    }

    #[test]
    fn transparency_never_lends_its_colour() {
        // a red disc on transparent pixels that carry a magenta "key" colour: once keyed (alpha 0 -> 0), no output pixel
        // takes any blue from the key
        let n = 9;
        let key = 0x00FF_00FF;
        let src: Vec<u32> = (0..n * n)
            .map(|i| {
                let (x, y) = ((i % n) as i32 - 4, (i / n) as i32 - 4);
                if x * x + y * y <= 10 {
                    R
                } else {
                    key
                }
            })
            .collect();
        let img = from_argb(n as u32, n as u32, &src);
        let out = scale(&img, 4);
        let mut partial = 0;
        for q in out.px.chunks_exact(4) {
            assert!(q[2] == 0 && q[1] == 0, "a fringe of the key: {q:?}");
            if q[3] > 0 && q[3] < 255 {
                partial += 1;
                assert_eq!(q[0], 255, "partly clear pixels keep the disc's colour");
            }
        }
        assert!(partial > 0, "the outline is anti-aliased");
    }

    #[test]
    fn gradient_weights_by_alpha() {
        let mut back = T;
        grad(&mut back, R, 1, 2);
        assert_eq!(back, 0x7FFF_0000);
        let mut back = W;
        grad(&mut back, K, 1, 4);
        assert_eq!(back, 0xFFBF_BFBF);
    }

    #[test]
    fn distance_matches_the_reference_formula() {
        assert_eq!(dist(R, R), 0.0);
        // opaque against clear: 255 whatever the colours
        assert_eq!(dist(R, T), 255.0);
        // black to white is the luminance difference alone
        assert!((dist(K, W) - 255.0).abs() < 0.01);
        assert!(eq(0xFF10_1010, 0xFF12_1212) && !eq(K, W));
    }
}
