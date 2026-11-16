//! Trees in the round for the walk camera, built from their own pictures.
//!
//! A tree's sprite is a picture of it from the side (and a little above). Seen from the ground as a card it is flat, and
//! up close its pixels show. Here each picture is turned into a solid: row by row, the picture's opaque span gives the
//! tree's width at that height and where its middle leans, and a ring of that radius is set at that height. The rings
//! are joined into a body of revolution, roughened a little so the crown is leafy rather than turned on a lathe, and
//! painted with the picture itself: each point takes the colour of the texel the picture shows there, as though seen
//! from its front (and the back mirrors it). So the tree keeps its outline, its colours and its painted light and shade,
//! whatever the art in use: the free set's or the original game's.

use crate::gfx::Vert;
use sg_core::assets::Rgba;
use std::f32::consts::TAU;

/// Sides round each ring.
const SIDES: usize = 12;
/// The most rings up a tree.
const RINGS: usize = 16;

/// The picture's opaque span (left, right) on each row, `None` where the row is empty.
fn spans(img: &Rgba) -> Vec<Option<(f32, f32)>> {
    let (w, h) = (img.w as usize, img.h as usize);
    (0..h)
        .map(|y| {
            let row = &img.px[y * w * 4..(y + 1) * w * 4];
            let on = |x: usize| row[x * 4 + 3] > 127;
            let l = (0..w).find(|&x| on(x))?;
            let r = (0..w).rev().find(|&x| on(x))?;
            Some((l as f32, r as f32 + 1.0))
        })
        .collect()
}

/// A number from 0 to 1 that stays with its arguments (the crown's roughness).
fn hash(a: f32, b: f32) -> f32 {
    ((a * 12.9898 + b * 78.233).sin() * 43758.547).fract().abs()
}

/// A tree's solid in the picture's pixels: x across the picture from its anchor, y up from the anchor (the tree's foot),
/// z towards the eye; texture coordinates into the picture (0..1). `None` when the picture has no tree above its foot.
pub fn solid(img: &Rgba, ax: f32, ay: f32) -> Option<Vec<Vert>> {
    let (w, h) = (img.w as f32, img.h as f32);
    let sp = spans(img);
    // the rows above the foot that hold the tree; rows below it are its shadow and the ground it stands on
    let foot = (ay.round() as usize).min(sp.len());
    let top = (0..foot).find(|&y| sp[y].is_some())?;
    if foot <= top + 3 {
        return None;
    }
    // the rings: evenly up the tree, each the mean of its band of rows (a smoother outline than any one row)
    let n = RINGS.min(foot - top);
    let mut rings: Vec<(f32, f32, f32)> = Vec::with_capacity(n + 2);
    for k in 0..n {
        let (y0, y1) = (top + (foot - top) * k / n, top + (foot - top) * (k + 1) / n);
        let rows: Vec<(f32, f32)> = (y0..y1.max(y0 + 1)).filter_map(|y| sp[y]).collect();
        if rows.is_empty() {
            continue;
        }
        let m = rows.len() as f32;
        let (l, r) = rows.iter().fold((0.0, 0.0), |a, s| (a.0 + s.0, a.1 + s.1));
        let (l, r) = (l / m, r / m);
        let yc = (y0 + y1) as f32 * 0.5;
        rings.push((yc, (l + r) * 0.5, ((r - l) * 0.5).max(0.6)));
    }
    if rings.len() < 2 {
        return None;
    }
    // closed at the top (a point over the first ring) and standing on the ground at the foot
    let c0 = rings[0].1;
    rings.insert(0, (top as f32 - 0.5, c0, 0.0));
    let last = *rings.last()?;
    rings.push((ay, last.1, last.2));
    // a point of ring `k` at angle `a` (0 faces the eye), and its texture coordinates
    let point = |k: usize, i: usize| {
        let (y, c, r) = rings[k];
        let a = TAU * i as f32 / SIDES as f32;
        // the crown roughened: a ring's radius varies round it and from ring to ring (not the trunk's, nor the tips')
        let leafy = r > 2.5 && k > 0 && k + 1 < rings.len();
        let rr = if leafy { r * (0.86 + 0.24 * hash(k as f32 * 1.7 + c, i as f32 * 2.3)) } else { r };
        let (sx, sz) = (a.sin(), a.cos());
        let pos = [c - ax + rr * sx, ay - y, rr * sz];
        // the texel the picture shows there seen from the front: across by the point's offset, a little in from the
        // outline (whose dark edge would otherwise run down the sides)
        let u = ((c + r * 0.85 * sx) / w).clamp(0.0, 1.0);
        let v = ((y + 0.5) / h).clamp(0.0, 1.0);
        (pos, [u, v])
    };
    let mut out = Vec::with_capacity((rings.len() - 1) * SIDES * 6);
    for k in 0..rings.len() - 1 {
        for i in 0..SIDES {
            let j = (i + 1) % SIDES;
            let q = [point(k, i), point(k, j), point(k + 1, j), point(k + 1, i)];
            // the face's normal, outward
            let (a, b, c) = (q[0].0, q[1].0, q[2].0);
            let (d, e) = ([b[0] - a[0], b[1] - a[1], b[2] - a[2]], [c[0] - a[0], c[1] - a[1], c[2] - a[2]]);
            let mut nrm = [d[1] * e[2] - d[2] * e[1], d[2] * e[0] - d[0] * e[2], d[0] * e[1] - d[1] * e[0]];
            if nrm == [0.0; 3] {
                let (b, c) = (q[3].0, q[2].0);
                let (d, e) = ([b[0] - a[0], b[1] - a[1], b[2] - a[2]], [c[0] - a[0], c[1] - a[1], c[2] - a[2]]);
                nrm = [d[1] * e[2] - d[2] * e[1], d[2] * e[0] - d[0] * e[2], d[0] * e[1] - d[1] * e[0]];
            }
            let mid = [(a[0] + c[0]) * 0.5 + ax - rings[k].1, 0.0, (a[2] + c[2]) * 0.5];
            if nrm[0] * mid[0] + nrm[2] * mid[2] < 0.0 {
                nrm = nrm.map(|v| -v);
            }
            let l = (nrm[0] * nrm[0] + nrm[1] * nrm[1] + nrm[2] * nrm[2]).sqrt().max(1e-6);
            let nrm = nrm.map(|v| v / l);
            let vert = |(p, uv): ([f32; 3], [f32; 2])| {
                let mut v = Vert::new(p[0], p[1], p[2], uv[0], uv[1]);
                v.normal = nrm;
                v
            };
            let [p0, p1, p2, p3] = q.map(vert);
            out.extend([p0, p1, p2, p0, p2, p3]);
        }
    }
    Some(out)
}

#[cfg(test)]
mod tests {
    use super::*;

    /// A picture of a round crown on a thin trunk, its foot at the bottom.
    fn tree_picture() -> Rgba {
        let (w, h) = (40u32, 60u32);
        let mut px = vec![0u8; (w * h * 4) as usize];
        for y in 0..h {
            for x in 0..w {
                let (dx, dy) = (x as f32 - 20.0, y as f32 - 20.0);
                let crown = dx * dx + dy * dy < 15.0 * 15.0;
                let trunk = (x as i32 - 20).abs() < 2 && y >= 30 && y < 55;
                if crown || trunk {
                    let i = ((y * w + x) * 4) as usize;
                    px[i..i + 4].copy_from_slice(&if crown { [40, 120, 40, 255] } else { [90, 60, 30, 255] });
                }
            }
        }
        Rgba { w, h, px }
    }

    #[test]
    fn a_tree_solid_keeps_its_pictures_outline() {
        let v = solid(&tree_picture(), 20.0, 55.0).unwrap();
        assert!(v.len() % 3 == 0 && !v.is_empty());
        let top = v.iter().map(|p| p.pos[1]).fold(f32::MIN, f32::max);
        let wide = v.iter().map(|p| p.pos[0].abs()).fold(0.0, f32::max);
        // as tall as the picture's tree (its foot at 55, its top at 5) and about as wide as its crown
        assert!((top - 50.0).abs() < 1.5, "top {top}");
        assert!(wide > 12.0 && wide < 19.0, "wide {wide}");
        // and the trunk thin near the ground
        let low = v.iter().filter(|p| p.pos[1] < 8.0).map(|p| p.pos[0].abs()).fold(0.0, f32::max);
        assert!(low < 3.0, "trunk {low}");
        // texture coordinates in the picture
        assert!(v.iter().all(|p| (0.0..=1.0).contains(&p.uv[0]) && (0.0..=1.0).contains(&p.uv[1])));
    }

    #[test]
    fn an_empty_picture_has_no_solid() {
        let img = Rgba { w: 8, h: 8, px: vec![0; 256] };
        assert!(solid(&img, 4.0, 7.0).is_none());
    }
}
