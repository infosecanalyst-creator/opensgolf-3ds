//! People in relief for the walk camera, built from their own pictures.
//!
//! A golfer's (or a worker's, or a resident's) picture is drawn for one of eight headings and changes with every step of
//! their animation, so a fixed solid can't stand in for it as one does for a tree (walk_trees.rs). Instead each frame is
//! raised into a relief, as a sculptor raises a figure from a panel: the deeper inside the figure's outline a point lies,
//! the further it stands out towards the eye, rounding off to nothing at the outline. Turned to the eye like the picture
//! it replaces, it shows the same outline and colours, but its surface faces the sun and turns away from it as a body
//! does, so an arm or a head is lit on one side and shaded on the other. The picture's views still choose which side of
//! the person the eye sees.

use crate::gfx::Vert;
use sg_core::assets::Rgba;

/// How far a point stands out, in pixels, for its distance inside the outline: rising quickly from the outline and
/// levelling off, like a rounded body.
fn rise(d: f32) -> f32 {
    1.25 * d.sqrt() + 0.2 * d.min(6.0)
}

/// Each opaque texel's distance (in texels, roughly Euclidean) to the nearest transparent one or the picture's edge.
fn inside(img: &Rgba) -> Vec<f32> {
    let (w, h) = (img.w as usize, img.h as usize);
    let big = (w + h) as f32;
    let mut d: Vec<f32> = (0..w * h).map(|i| if img.px[i * 4 + 3] > 127 { big } else { 0.0 }).collect();
    let (a, b) = (1.0f32, std::f32::consts::SQRT_2);
    let at = |d: &Vec<f32>, x: isize, y: isize| {
        if x < 0 || y < 0 || x >= w as isize || y >= h as isize {
            0.0
        } else {
            d[y as usize * w + x as usize]
        }
    };
    for y in 0..h as isize {
        for x in 0..w as isize {
            let i = y as usize * w + x as usize;
            if d[i] > 0.0 {
                let m = (at(&d, x - 1, y) + a).min(at(&d, x, y - 1) + a).min(at(&d, x - 1, y - 1) + b).min(at(&d, x + 1, y - 1) + b);
                d[i] = d[i].min(m);
            }
        }
    }
    for y in (0..h as isize).rev() {
        for x in (0..w as isize).rev() {
            let i = y as usize * w + x as usize;
            if d[i] > 0.0 {
                let m = (at(&d, x + 1, y) + a).min(at(&d, x, y + 1) + a).min(at(&d, x + 1, y + 1) + b).min(at(&d, x - 1, y + 1) + b);
                d[i] = d[i].min(m);
            }
        }
    }
    d
}

/// A frame's relief in its pixels: x across the picture from its anchor, y up from the anchor (the feet), z out towards
/// the eye; texture coordinates into the picture (0..1). Only the side towards the eye: it is always turned to the eye.
/// `None` when the picture is empty.
pub fn relief(img: &Rgba, ax: f32, ay: f32) -> Option<Vec<Vert>> {
    let (w, h) = (img.w as usize, img.h as usize);
    if w < 2 || h < 2 {
        return None;
    }
    let d = inside(img);
    let opaque = |x: usize, y: usize| d[y * w + x] > 0.0;
    // the height at a texel corner (cx, cy in 0..=w, 0..=h): the mean of the texels round it, so the outline's corners lie
    // at the panel and the relief swells inward
    let corner = |cx: usize, cy: usize| {
        let mut s = 0.0;
        for (x, y) in [(cx.wrapping_sub(1), cy.wrapping_sub(1)), (cx, cy.wrapping_sub(1)), (cx.wrapping_sub(1), cy), (cx, cy)] {
            if x < w && y < h {
                s += d[y * w + x];
            }
        }
        rise(s * 0.25)
    };
    let zs: Vec<f32> = (0..=h).flat_map(|cy| (0..=w).map(move |cx| (cx, cy))).map(|(cx, cy)| corner(cx, cy)).collect();
    let z = |cx: usize, cy: usize| zs[cy * (w + 1) + cx];
    // a corner's normal, from the slopes of the heights about it (x right, y up, z out)
    let normal = |cx: usize, cy: usize| {
        let (l, r) = (z(cx.saturating_sub(1), cy), z((cx + 1).min(w), cy));
        let (t, b) = (z(cx, cy.saturating_sub(1)), z(cx, (cy + 1).min(h)));
        let (gx, gy) = ((r - l) * 0.5, (t - b) * 0.5);
        let n = [-gx, -gy, 1.0];
        let len = (n[0] * n[0] + n[1] * n[1] + n[2] * n[2]).sqrt();
        n.map(|v| v / len)
    };
    let vert = |cx: usize, cy: usize| {
        let mut v = Vert::new(cx as f32 - ax, ay - cy as f32, z(cx, cy), cx as f32 / w as f32, cy as f32 / h as f32);
        v.normal = normal(cx, cy);
        // shaded towards the outline, as a rounded body turns from the light at its edges
        let s = 0.88 + 0.2 * (z(cx, cy) / rise(4.0)).min(1.0);
        v.color = [s, s, s, 1.0];
        v
    };
    let mut out = Vec::new();
    for y in 0..h {
        for x in 0..w {
            if !opaque(x, y) {
                continue;
            }
            let (a, b, c, e) = (vert(x, y), vert(x + 1, y), vert(x + 1, y + 1), vert(x, y + 1));
            out.extend([a, e, c, a, c, b]);
        }
    }
    (!out.is_empty()).then_some(out)
}

#[cfg(test)]
mod tests {
    use super::*;

    /// A figure: a round head over a body, its feet at the bottom.
    fn figure() -> Rgba {
        let (w, h) = (20u32, 40u32);
        let mut px = vec![0u8; (w * h * 4) as usize];
        for y in 0..h {
            for x in 0..w {
                let (dx, dy) = (x as f32 - 10.0, y as f32 - 6.0);
                let head = dx * dx + dy * dy < 16.0;
                let body = (x as i32 - 10).abs() < 5 && (10..38).contains(&y);
                if head || body {
                    let i = ((y * w + x) * 4) as usize;
                    px[i..i + 4].copy_from_slice(&[200, 60, 160, 255]);
                }
            }
        }
        Rgba { w, h, px }
    }

    #[test]
    fn a_relief_keeps_the_outline_and_stands_out_in_the_middle() {
        let v = relief(&figure(), 10.0, 38.0).unwrap();
        assert!(v.len() % 3 == 0);
        // as tall as the figure (feet at 38, head's top at 2)
        let top = v.iter().map(|p| p.pos[1]).fold(f32::MIN, f32::max);
        assert!((top - 36.0).abs() < 1.5, "top {top}");
        // the middle of the body stands out further than its sides
        let mid = v.iter().filter(|p| p.pos[0].abs() < 1.0 && (10.0..20.0).contains(&p.pos[1])).map(|p| p.pos[2]).fold(0.0, f32::max);
        let side =
            v.iter().filter(|p| (p.pos[0] + 4.0).abs() < 0.1 && (10.0..20.0).contains(&p.pos[1])).map(|p| p.pos[2]).fold(0.0, f32::max);
        assert!(mid > side + 1.0, "mid {mid} side {side}");
        // the sides face outward
        let left = v.iter().find(|p| p.pos[0] < -3.5 && (10.0..20.0).contains(&p.pos[1])).unwrap();
        assert!(left.normal[0] < 0.0, "{:?}", left.normal);
        assert!(v.iter().all(|p| (0.0..=1.0).contains(&p.uv[0]) && (0.0..=1.0).contains(&p.uv[1])));
    }

    #[test]
    fn an_empty_picture_has_no_relief() {
        assert!(relief(&Rgba { w: 8, h: 8, px: vec![0; 256] }, 4.0, 7.0).is_none());
    }
}
