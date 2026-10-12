//! Enhanced graphics' smooth edges for colour-keyed art (the "smooth interface art" option, enhanced.rs): a picture whose
//! transparency is all or nothing (the original's colour keys: magenta, 0xF800F8 and the like) has a hard, one-texel outline
//! that a large window magnifies into stairs. Such a picture gets, in place of its 0 / 255 alpha, a signed distance to a
//! smoothed outline (distance field, as text renderers use), and the renderer turns that back into coverage one screen pixel
//! wide at whatever size the picture is drawn (gfx.rs FRAGMENT_SDF and the scaler, fx_glsl.rs). The colours are not touched.
//!
//! The outline: the exact distance from each texel centre to the nearest texel of the other kind (an Euclidean distance
//! transform), smoothed with a Gaussian so a staircase becomes the line or curve it was drawn from, but
//! - only where the shape is thick on both sides (a one or two texel line, a single texel or a one texel gap keeps its raw
//!   distance, so it stays as wide as it was),
//! - by at most half a texel (a staircase never needs more; a square corner is only slightly rounded), and
//! - never across a texel centre (an opaque texel stays inside the outline and a clear one outside, so nothing appears or
//!   disappears at the original size).
use sg_core::assets::Rgba;

/// Distances stored in alpha run from -SPREAD to +SPREAD texels (alpha 0 to 255, the outline at 127.5).
pub const SPREAD: f32 = 4.0;
/// The Gaussian's sigma, in texels.
const SIGMA: f32 = 1.5;
/// How far the smoothing may move the outline, in texels.
const CAP: f32 = 0.5;
/// Texels further than this from the outline (raw distance) keep their raw distance.
const BAND: f32 = 3.0;
/// The least distance a texel centre keeps from the outline (keeps every texel on its own side).
const KEEP: f32 = 0.2;

/// True when the picture's alpha is all or nothing with both kinds present (a colour-keyed picture).
pub fn is_keyed(img: &Rgba) -> bool {
    let (mut clear, mut solid) = (false, false);
    for p in img.px.chunks_exact(4) {
        match p[3] {
            0 => clear = true,
            255 => solid = true,
            _ => return false,
        }
    }
    clear && solid
}

/// The picture with its alpha replaced by the distance to its smoothed outline (see the module notes), or None when its
/// alpha is not all or nothing.
pub fn soft_edges(img: &Rgba) -> Option<Rgba> {
    if !is_keyed(img) {
        return None;
    }
    let (w, h) = (img.w as usize, img.h as usize);
    let inside: Vec<bool> = img.px.chunks_exact(4).map(|p| p[3] != 0).collect();
    let field = distance_field(&inside, w, h);
    let mut out = img.clone();
    for (p, d) in out.px.chunks_exact_mut(4).zip(&field) {
        p[3] = ((0.5 + d / (2.0 * SPREAD)).clamp(0.0, 1.0) * 255.0).round() as u8;
    }
    Some(out)
}

/// The smoothed signed distance (texels, positive inside) at each texel centre of a mask.
pub fn distance_field(inside: &[bool], w: usize, h: usize) -> Vec<f32> {
    // raw: the distance to the nearest centre of the other kind, less half a texel (the outline between two neighbours)
    let din = edt(inside, w, h, false);
    let dout = edt(inside, w, h, true);
    let raw: Vec<f32> = (0..w * h).map(|i| if inside[i] { din[i] - 0.5 } else { 0.5 - dout[i] }.clamp(-SPREAD, SPREAD)).collect();
    // only texels near the outline are smoothed (further out the smoothing could only matter far below the picture's size)
    let r = (SIGMA * 3.0).ceil() as i32;
    let k: Vec<f32> = (-r..=r).map(|i| (-(i * i) as f32 / (2.0 * SIGMA * SIGMA)).exp()).collect();
    let at = |x: i32, y: i32| raw[y.clamp(0, h as i32 - 1) as usize * w + x.clamp(0, w as i32 - 1) as usize];
    (0..w * h)
        .map(|i| {
            let d = raw[i];
            if d.abs() < BAND {
                let (x, y) = ((i % w) as i32, (i / w) as i32);
                // how thick the shape is round the texel, inside and outside (the 5 x 5 maxima of the raw distances)
                let (mut thick_in, mut thick_out) = (0.0f32, 0.0f32);
                for dy in -2..=2 {
                    for dx in -2..=2 {
                        let v = at(x + dx, y + dy);
                        thick_in = thick_in.max(v);
                        thick_out = thick_out.max(-v);
                    }
                }
                let weight = ((thick_in.min(thick_out) - 1.0) / 1.5).clamp(0.0, 1.0);
                if weight > 0.0 {
                    let (mut sum, mut norm) = (0.0f32, 0.0f32);
                    for (j, ky) in k.iter().enumerate() {
                        for (m, kx) in k.iter().enumerate() {
                            sum += at(x + m as i32 - r, y + j as i32 - r) * kx * ky;
                            norm += kx * ky;
                        }
                    }
                    let smooth = sum / norm;
                    let moved = d + ((smooth - d) * weight).clamp(-CAP, CAP);
                    return if inside[i] { moved.max(KEEP) } else { moved.min(-KEEP) };
                }
            }
            if inside[i] {
                d.max(KEEP)
            } else {
                d.min(-KEEP)
            }
        })
        .collect()
}

/// The Euclidean distance from each texel centre to the nearest centre where `inside` differs from `of_clear` (for
/// of_clear false: the distance from inside texels to the nearest clear one), by Felzenszwalb and Huttenlocher's separable
/// transform. Texels with no such centre anywhere get a large distance.
fn edt(inside: &[bool], w: usize, h: usize, of_clear: bool) -> Vec<f32> {
    const INF: f32 = 1e12;
    // target texels (distance 0): the clear ones when measuring from inside, the inside ones when measuring from clear
    let mut f: Vec<f32> = inside.iter().map(|&k| if k == of_clear { 0.0 } else { INF }).collect();
    let mut buf = vec![0.0f32; w.max(h)];
    let mut out = vec![0.0f32; w.max(h)];
    let mut v = vec![0usize; w.max(h)];
    let mut z = vec![0.0f32; w.max(h) + 1];
    for x in 0..w {
        for y in 0..h {
            buf[y] = f[y * w + x];
        }
        dt1(&buf[..h], &mut out[..h], &mut v, &mut z);
        for y in 0..h {
            f[y * w + x] = out[y];
        }
    }
    for y in 0..h {
        buf[..w].copy_from_slice(&f[y * w..y * w + w]);
        dt1(&buf[..w], &mut out[..w], &mut v, &mut z);
        f[y * w..y * w + w].copy_from_slice(&out[..w]);
    }
    f.iter().map(|d| d.sqrt()).collect()
}

/// One dimension of the squared distance transform (the lower envelope of parabolas).
fn dt1(f: &[f32], d: &mut [f32], v: &mut [usize], z: &mut [f32]) {
    let n = f.len();
    if n == 0 {
        return;
    }
    let mut k = 0usize;
    v[0] = 0;
    z[0] = f32::NEG_INFINITY;
    z[1] = f32::INFINITY;
    for q in 1..n {
        loop {
            let p = v[k];
            let s = ((f[q] + (q * q) as f32) - (f[p] + (p * p) as f32)) / (2.0 * (q as f32 - p as f32));
            if s <= z[k] && k > 0 {
                k -= 1;
                continue;
            }
            if s <= z[k] {
                // k == 0: the first parabola is replaced
                v[0] = q;
                z[0] = f32::NEG_INFINITY;
                z[1] = f32::INFINITY;
                break;
            }
            k += 1;
            v[k] = q;
            z[k] = s;
            z[k + 1] = f32::INFINITY;
            break;
        }
    }
    k = 0;
    for (q, out) in d.iter_mut().enumerate() {
        while z[k + 1] < q as f32 {
            k += 1;
        }
        let p = v[k];
        let dq = q as f32 - p as f32;
        *out = dq * dq + f[p];
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn mask(w: usize, h: usize, f: impl Fn(usize, usize) -> bool) -> Vec<bool> {
        (0..w * h).map(|i| f(i % w, i / w)).collect()
    }

    #[test]
    fn edt_measures_to_the_nearest_other_texel() {
        let m = mask(7, 1, |x, _| x >= 3);
        let d = edt(&m, 7, 1, false);
        assert_eq!(d, vec![0.0, 0.0, 0.0, 1.0, 2.0, 3.0, 4.0]);
        let m = mask(5, 5, |x, y| x == 2 && y == 2);
        let d = edt(&m, 5, 5, true);
        assert!((d[0] - 8f32.sqrt()).abs() < 1e-5);
        assert_eq!(d[12], 0.0);
    }

    #[test]
    fn every_texel_keeps_its_side_and_thin_lines_keep_their_width() {
        let (w, h) = (40, 30);
        let m = mask(w, h, |x, y| {
            let (dx, dy) = (x as f32 - 12.0, y as f32 - 14.0);
            dx * dx + dy * dy < 100.0 || x == 30 || (y == 5 && x > 26)
        });
        let f = distance_field(&m, w, h);
        for i in 0..w * h {
            assert_eq!(f[i] > 0.0, m[i], "texel {i} changed sides");
        }
        // the one texel line at x 30 keeps its raw distances (the outline half a texel either side)
        assert!((f[15 * w + 30] - 0.5).abs() < 1e-5 && (f[15 * w + 31] + 0.5).abs() < 1e-5);
    }

    #[test]
    fn a_staircase_is_smoothed() {
        // a shallow staircase edge: the outline's place along each column follows a line more closely than the steps do
        let (w, h) = (48, 16);
        let edge = |x: usize| 4.0 + x as f32 / 6.0;
        // texel centres at whole numbers: inside below the line
        let m = mask(w, h, |x, y| y as f32 > edge(x));
        let f = distance_field(&m, w, h);
        let place = |x: usize| {
            // the zero crossing down the column, linear between centres
            (0..h - 1).find(|&y| f[y * w + x] <= 0.0 && f[(y + 1) * w + x] > 0.0).map(|y| {
                let (a, b) = (f[y * w + x], f[(y + 1) * w + x]);
                y as f32 + a / (a - b)
            })
        };
        let (mut raw_err, mut err) = (0.0, 0.0);
        for x in 8..40 {
            let p = place(x).unwrap();
            let target = edge(x);
            err += (p - target).abs();
            raw_err += (edge(x).floor() + 0.5 - target).abs();
        }
        assert!(err < raw_err * 0.6, "smoothed {err}, steps {raw_err}");
    }

    #[test]
    fn only_keyed_pictures_change() {
        let mut img = Rgba::new(4, 1);
        img.px = vec![1, 2, 3, 0, 1, 2, 3, 255, 1, 2, 3, 255, 1, 2, 3, 0];
        let out = soft_edges(&img).unwrap();
        assert_eq!(out.px[0..3], [1, 2, 3]);
        assert!(out.px[3] < 128 && out.px[7] > 128);
        img.px[7] = 200;
        assert!(soft_edges(&img).is_none());
    }
}
