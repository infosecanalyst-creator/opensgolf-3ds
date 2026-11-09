//! What stands on the walk camera's ground (walk.rs): the course's objects, trees, people, flags, tee markers and balls.
//!
//! The game's art is drawn for its overhead view: each sprite is the object seen from above at an angle (the overhead
//! camera's pitch, from one of its four quarter turns). Stood upright at eye level such a picture shows its roof and leans
//! wrongly, so here:
//!
//! * a building or landmark gets a simple solid (a box, with a flat, ridged or pointed roof) fitted to its sprite's
//!   outline (`fit_proxy`), and the sprite itself is projected onto it from the direction the overhead camera saw it
//!   from, so each face carries the player's own art; a face the sprite never shows takes the view of the quarter turn
//!   that does show it (the object's own art for that turn, or the same picture again for art with one view). Objects
//!   whose outline no solid fits well (a statue, the oil pump) are drawn as their picture on a card leaning back at the
//!   angle the art was drawn from, as the overhead view itself draws it;
//! * pictures that lie on the ground (a building's base, the shadows drawn with the art) are laid onto the ground by the
//!   same projection, following its rise and fall;
//! * trees stand as upright cards turned to the eye, swaying a little: their art has one view and is round, so a card
//!   shows the same picture from every side without the seams or crossed lines of solid or crossed shapes;
//! * people stand as upright cards in the frame whose facing matches their heading as seen from the eye, at a person's
//!   height, with a soft round shadow;
//! * the flags, tee markers and balls are small solids of their own.
//!
//! Everything is left out when it is off the screen or lost in the haze, and people and the finer touches are left out
//! further off. Nothing here changes the game's state.
use crate::app::{App, Prop};
use crate::gfx::{Gfx, Mat4, Vert};
use crate::walk_gl::{mode, sprite_texture, Pipe, WalkGpu, WalkUniforms};
use miniquad::TextureId;
use sg_core::assets::Rgba;
use sg_core::sprites::SPRITE_UNITS_PER_PIXEL;
use sg_core::terrain::TILE_SIZE;
use std::collections::HashMap;

/// World units per metre (walk.rs).
const M: f32 = crate::walk::METRE;
/// A person's height.
const PERSON: f32 = 1.8 * M;
/// Trees' size against the overhead art's (whose trees are drawn far larger than the ground they stand on).
const TREE_SCALE: f32 = 0.42;
/// Other small things' size against the art's (animals, weeds and the like).
const SMALL_SCALE: f32 = 0.4;
/// The least fit for which a building keeps a roof (`raise_roof`).
const RAISE_FIT: f32 = 0.78;
/// The least fit for a building on more than one tile: a looser one (towers, cupolas, wings standing out of any box) would
/// wear them smeared across its walls, and its card is drawn instead.
const MIN_FIT_BUILDING: f32 = 0.78;
/// A small object's size against the art's, when no solid fits it (a ball washer, benches).
const SMALL_OBJECT: f32 = 0.5;
/// The least fit (intersection over union) for which an object's solid is used rather than its card.
const MIN_FIT: f32 = 0.76;

/// Something that stops a walk (walk.rs `Cam::step`): a building's footprint or a tree's trunk.
#[derive(Clone, Copy, Debug, PartialEq)]
pub enum Obstacle {
    Box { cx: f32, cz: f32, hx: f32, hz: f32 },
    Circle { x: f32, z: f32, r: f32 },
}

/// The overhead camera's view at a quarter turn: the screen's right and up and its line of sight, in world space.
#[derive(Clone, Copy, Debug)]
pub struct Iso {
    pub r: [f32; 3],
    pub u: [f32; 3],
    pub f: [f32; 3],
}

/// The overhead view's pitch the art was drawn for (the 800 x 600 screen's).
pub fn art_pitch() -> f32 {
    crate::app::pitch_for(800.0, 600.0) as f32
}

pub fn iso(quarter: i32) -> Iso {
    let m = Mat4::rotate(art_pitch() as f64, 1.0, 0.0, 0.0)
        .mul(&Mat4::rotate((45.0 + 90.0 * quarter.rem_euclid(4) as f32) as f64, 0.0, 1.0, 0.0))
        .0;
    Iso { r: [m[0], m[4], m[8]], u: [m[1], m[5], m[9]], f: [-m[2], -m[6], -m[10]] }
}

fn dot(a: [f32; 3], b: [f32; 3]) -> f32 {
    a[0] * b[0] + a[1] * b[1] + a[2] * b[2]
}

impl Iso {
    /// Where a point (relative to a sprite's ground point) lands on the sprite: pixel x, y, for `s` world units a pixel and
    /// the sprite's anchor.
    pub fn to_px(self, d: [f32; 3], s: f32, ax: f32, ay: f32) -> (f32, f32) {
        (ax + dot(d, self.r) / s, ay - dot(d, self.u) / s)
    }
    /// The ground point (relative, y = 0) a sprite pixel lands on: the projection undone on the ground's plane.
    pub fn ground_of(self, px: f32, py: f32, s: f32, ax: f32, ay: f32) -> (f32, f32) {
        let (cx, cy) = ((px - ax) * s, (ay - py) * s);
        let (a, b, c, d) = (self.r[0], self.r[2], self.u[0], self.u[2]);
        let det = a * d - b * c;
        ((cx * d - b * cy) / det, (a * cy - c * cx) / det)
    }
}

// ---- fitting a solid to a sprite's outline ---------------------------------------------------------------------------------

/// A frame's opaque texels, column by column, for counting quickly how many fall in a span.
pub struct Mask {
    pub w: usize,
    pub h: usize,
    /// For each column, the count of opaque texels above each row (h + 1 a column).
    prefix: Vec<u32>,
    pub total: u32,
    /// The opaque texels' bounds: left, top, right, bottom (exclusive).
    pub bbox: (usize, usize, usize, usize),
}

impl Mask {
    pub fn new(img: &Rgba) -> Mask {
        let (w, h) = (img.w as usize, img.h as usize);
        let mut prefix = vec![0u32; w * (h + 1)];
        let mut total = 0;
        let mut bbox = (w, h, 0, 0);
        for x in 0..w {
            let base = x * (h + 1);
            for y in 0..h {
                let on = img.px[(y * w + x) * 4 + 3] > 127;
                prefix[base + y + 1] = prefix[base + y] + on as u32;
                if on {
                    total += 1;
                    bbox = (bbox.0.min(x), bbox.1.min(y), bbox.2.max(x + 1), bbox.3.max(y + 1));
                }
            }
        }
        Mask { w, h, prefix, total, bbox }
    }
    /// Opaque texels in column x from row y0 up to (not including) y1.
    fn count(&self, x: usize, y0: usize, y1: usize) -> u32 {
        let base = x * (self.h + 1);
        self.prefix[base + y1.min(self.h)] - self.prefix[base + y0.min(self.h)]
    }
}

/// A solid for an object: on a footprint `hx` x `hz` (half sizes) round (cx, cz), a platform `base` high (none at 0), on
/// it a building `tier` times the footprint's size with its walls up to `wall` over the ground, under a roof `roof` higher:
/// flat (0), ridged along X (1) or Z (2) with gable ends, pointed (3), or hipped (4: sloping on all four sides to a ridge
/// along the longer side). The original art draws most buildings on a plinth, a deck or a terrace with the building on it.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct Fit {
    pub cx: f32,
    pub cz: f32,
    pub hx: f32,
    pub hz: f32,
    pub base: f32,
    pub tier: f32,
    pub wall: f32,
    pub roof: f32,
    pub kind: u8,
    pub iou: f32,
}

impl Default for Fit {
    fn default() -> Fit {
        Fit { cx: 0.0, cz: 0.0, hx: 1.0, hz: 1.0, base: 0.0, tier: 1.0, wall: 1.0, roof: 0.0, kind: 0, iou: 0.0 }
    }
}

impl Fit {
    /// The building's half sizes (on the platform).
    pub fn building(&self) -> (f32, f32) {
        (self.hx * self.tier, self.hz * self.tier)
    }

    /// The solid's corners and roof points (relative to the ground point).
    pub fn points(&self) -> Vec<[f32; 3]> {
        let (cx, cz, w, top) = (self.cx, self.cz, self.wall, self.wall + self.roof);
        let (bx, bz) = self.building();
        let mut p = Vec::with_capacity(20);
        for (sx, sz) in [(-1.0, -1.0), (1.0, -1.0), (1.0, 1.0), (-1.0, 1.0)] {
            if self.base > 0.0 {
                p.push([cx + sx * self.hx, 0.0, cz + sz * self.hz]);
                p.push([cx + sx * self.hx, self.base, cz + sz * self.hz]);
            }
            p.push([cx + sx * bx, self.base, cz + sz * bz]);
            p.push([cx + sx * bx, w, cz + sz * bz]);
        }
        match self.kind {
            1 => p.extend([[cx - bx, top, cz], [cx + bx, top, cz]]),
            2 => p.extend([[cx, top, cz - bz], [cx, top, cz + bz]]),
            3 => p.push([cx, top, cz]),
            4 => {
                let (a, b) = self.hip_ridge();
                p.extend([a, b]);
            }
            _ => {}
        }
        p
    }

    /// A hipped roof's ridge ends: along the building's longer side, short of the ends by the shorter side's half.
    fn hip_ridge(&self) -> ([f32; 3], [f32; 3]) {
        let top = self.wall + self.roof;
        let (bx, bz) = self.building();
        if bx >= bz {
            let d = bx - bz;
            ([self.cx - d, top, self.cz], [self.cx + d, top, self.cz])
        } else {
            let d = bz - bx;
            ([self.cx, top, self.cz - d], [self.cx, top, self.cz + d])
        }
    }

    /// The four walls of a box from `b` up to `w`, half sizes `hx`, `hz` round (cx, cz): north (-z), east, south, west.
    fn walls(cx: f32, cz: f32, hx: f32, hz: f32, b: f32, w: f32) -> [Vec<[f32; 3]>; 4] {
        let (x0, x1, z0, z1) = (cx - hx, cx + hx, cz - hz, cz + hz);
        [
            vec![[x1, b, z0], [x0, b, z0], [x0, w, z0], [x1, w, z0]],
            vec![[x1, b, z1], [x1, b, z0], [x1, w, z0], [x1, w, z1]],
            vec![[x0, b, z1], [x1, b, z1], [x1, w, z1], [x0, w, z1]],
            vec![[x0, b, z0], [x0, b, z1], [x0, w, z1], [x0, w, z0]],
        ]
    }

    /// The solid's faces: each a convex polygon (counter clockwise seen from outside) and its outward normal. The walls
    /// reach a little below the ground, so a slope shows no gap under them.
    pub fn faces(&self) -> Vec<(Vec<[f32; 3]>, [f32; 3])> {
        let (cx, cz, w, top) = (self.cx, self.cz, self.wall, self.wall + self.roof);
        let below = -0.6 * M;
        let normals = [[0.0, 0.0, -1.0], [1.0, 0.0, 0.0], [0.0, 0.0, 1.0], [-1.0, 0.0, 0.0]];
        let mut f: Vec<(Vec<[f32; 3]>, [f32; 3])> = Vec::new();
        if self.base > 0.0 {
            for (wall, n) in Self::walls(cx, cz, self.hx, self.hz, below, self.base).into_iter().zip(normals) {
                f.push((wall, n));
            }
            let (x0, x1, z0, z1) = (cx - self.hx, cx + self.hx, cz - self.hz, cz + self.hz);
            f.push((vec![[x0, self.base, z1], [x1, self.base, z1], [x1, self.base, z0], [x0, self.base, z0]], [0.0, 1.0, 0.0]));
        }
        let (bx, bz) = self.building();
        let (x0, x1, z0, z1) = (cx - bx, cx + bx, cz - bz, cz + bz);
        let foot = if self.base > 0.0 { self.base - 0.05 * M } else { below };
        let [mut north, mut east, mut south, mut west] = Self::walls(cx, cz, bx, bz, foot, w);
        // a ridge's ends carry its gable
        match self.kind {
            1 => {
                east.insert(3, [x1, top, cz]);
                west.insert(3, [x0, top, cz]);
            }
            2 => {
                north.insert(3, [cx, top, z0]);
                south.insert(3, [cx, top, z1]);
            }
            _ => {}
        }
        for (wall, n) in [north, east, south, west].into_iter().zip(normals) {
            f.push((wall, n));
        }
        let norm = |v: [f32; 3]| {
            let l = (v[0] * v[0] + v[1] * v[1] + v[2] * v[2]).sqrt().max(1e-6);
            [v[0] / l, v[1] / l, v[2] / l]
        };
        let r = self.roof;
        match self.kind {
            1 => {
                f.push((vec![[x0, w, z0], [x0, top, cz], [x1, top, cz], [x1, w, z0]], norm([0.0, bz, -r])));
                f.push((vec![[x1, w, z1], [x1, top, cz], [x0, top, cz], [x0, w, z1]], norm([0.0, bz, r])));
            }
            2 => {
                f.push((vec![[x0, w, z1], [cx, top, z1], [cx, top, z0], [x0, w, z0]], norm([-r, bx, 0.0])));
                f.push((vec![[x1, w, z0], [cx, top, z0], [cx, top, z1], [x1, w, z1]], norm([r, bx, 0.0])));
            }
            3 => {
                let a = [cx, top, cz];
                f.push((vec![[x1, w, z0], [x0, w, z0], a], norm([0.0, bz, -r])));
                f.push((vec![[x1, w, z1], [x1, w, z0], a], norm([r, bx, 0.0])));
                f.push((vec![[x0, w, z1], [x1, w, z1], a], norm([0.0, bz, r])));
                f.push((vec![[x0, w, z0], [x0, w, z1], a], norm([-r, bx, 0.0])));
            }
            4 => {
                // the slopes all rise at the shorter side's pitch
                let (a, b) = self.hip_ridge();
                let m = bx.min(bz);
                if bx >= bz {
                    f.push((vec![[x1, w, z0], [x0, w, z0], a, b], norm([0.0, m, -r])));
                    f.push((vec![[x0, w, z1], [x1, w, z1], b, a], norm([0.0, m, r])));
                    f.push((vec![[x1, w, z1], [x1, w, z0], b], norm([r, m, 0.0])));
                    f.push((vec![[x0, w, z0], [x0, w, z1], a], norm([-r, m, 0.0])));
                } else {
                    f.push((vec![[x0, w, z0], [x0, w, z1], b, a], norm([-r, m, 0.0])));
                    f.push((vec![[x1, w, z1], [x1, w, z0], a, b], norm([r, m, 0.0])));
                    f.push((vec![[x1, w, z0], [x0, w, z0], a], norm([0.0, m, -r])));
                    f.push((vec![[x0, w, z1], [x1, w, z1], b], norm([0.0, m, r])));
                }
            }
            _ => f.push((vec![[x0, w, z1], [x1, w, z1], [x1, w, z0], [x0, w, z0]], [0.0, 1.0, 0.0])),
        }
        f
    }
}

/// The convex hull of points (Andrew's monotone chain), counter clockwise.
fn hull(mut p: Vec<(f32, f32)>) -> Vec<(f32, f32)> {
    p.sort_by(|a, b| {
        a.0.partial_cmp(&b.0).unwrap_or(std::cmp::Ordering::Equal).then(a.1.partial_cmp(&b.1).unwrap_or(std::cmp::Ordering::Equal))
    });
    if p.len() < 3 {
        return p;
    }
    let cross = |o: (f32, f32), a: (f32, f32), b: (f32, f32)| (a.0 - o.0) * (b.1 - o.1) - (a.1 - o.1) * (b.0 - o.0);
    let mut h: Vec<(f32, f32)> = Vec::with_capacity(p.len() * 2);
    for &q in p.iter().chain(p.iter().rev().skip(1)) {
        // the lower chain, then the upper one back
        while h.len() >= 2 && cross(h[h.len() - 2], h[h.len() - 1], q) <= 0.0 {
            h.pop();
        }
        h.push(q);
    }
    h.pop();
    h
}

/// The span of rows a convex polygon covers at column centre x, if any.
fn span_at(poly: &[(f32, f32)], x: f32) -> Option<(f32, f32)> {
    let (mut lo, mut hi) = (f32::MAX, f32::MIN);
    for i in 0..poly.len() {
        let (a, b) = (poly[i], poly[(i + 1) % poly.len()]);
        let (x0, x1) = (a.0.min(b.0), a.0.max(b.0));
        if x < x0 || x > x1 {
            continue;
        }
        let y = if (b.0 - a.0).abs() < 1e-6 {
            lo = lo.min(a.1.min(b.1));
            hi = hi.max(a.1.max(b.1));
            continue;
        } else {
            a.1 + (b.1 - a.1) * (x - a.0) / (b.0 - a.0)
        };
        lo = lo.min(y);
        hi = hi.max(y);
    }
    (lo <= hi).then_some((lo, hi))
}

/// How well a solid's outline, seen as the art was drawn, matches the art's opaque texels: intersection over union.
pub fn score(fit: &Fit, mask: &Mask, iso: &Iso, s: f32, ax: f32, ay: f32) -> f32 {
    let pts: Vec<(f32, f32)> = fit.points().into_iter().map(|p| iso.to_px(p, s, ax, ay)).collect();
    let poly = hull(pts);
    if poly.len() < 3 {
        return 0.0;
    }
    let minx = poly.iter().map(|p| p.0).fold(f32::MAX, f32::min).floor().max(0.0) as usize;
    let maxx = (poly.iter().map(|p| p.0).fold(f32::MIN, f32::max).ceil().max(0.0) as usize).min(mask.w);
    let (mut inter, mut area) = (0u32, 0u32);
    for x in minx..maxx {
        if let Some((a, b)) = span_at(&poly, x as f32 + 0.5) {
            let (t, bt) = (a.round().clamp(0.0, mask.h as f32) as usize, b.round().clamp(0.0, mask.h as f32) as usize);
            if bt > t {
                inter += mask.count(x, t, bt);
                area += (bt - t) as u32;
            }
        }
    }
    let union = mask.total + area - inter;
    if union == 0 {
        0.0
    } else {
        inter as f32 / union as f32
    }
}

/// The solid that best matches a sprite's outlines (one per view: its mask and the overhead camera that drew it; `s` world
/// units a pixel, the sprite's anchor at the ground point), its footprint at most `half` either way from the ground point
/// when the object's size is known (a building's tiles). Fitting every view at once keeps the art projected from each one
/// in line with the solid's faces. A coarse search over its sizes, platforms and roofs, then a finer one round the best.
pub fn fit_proxy(views: &[(&Mask, Iso)], s: f32, ax: f32, ay: f32, half: Option<f32>) -> Option<Fit> {
    if views.is_empty() || views.iter().any(|(m, _)| m.total < 30) {
        return None;
    }
    // the widest footprint the outlines allow: a square's corners reach (|r.x| + |r.z|) times its half size sideways
    let mut wide: f32 = 0.0;
    let mut tall: f32 = 0.0;
    for (m, iso) in views {
        let (l, top, r, _) = m.bbox;
        let spread = iso.r[0].abs() + iso.r[2].abs();
        wide = wide.max(((r as f32 - ax).abs().max((ax - l as f32).abs())) * s / spread * 1.15);
        // the tallest the outline allows, if all of it were wall
        tall = tall.max(((ay - top as f32) * s / iso.u[1].max(0.2)).max(1.0) * 1.05);
    }
    let hmax = half.map(|h| h.min(wide * 1.2)).unwrap_or(wide).max(2.0);
    // a roof that stays inside the outline cannot be told from a flat top (the art is seen from well above), so of fits
    // that match alike the lower roof wins; and as a narrow tall building and a wide low one look alike from there, the
    // wider lower one (the art's roof then lands on its roof, not up its walls)
    let score_all = |f: &Fit| {
        views.iter().map(|(m, iso)| score(f, m, iso, s, ax, ay)).sum::<f32>() / views.len() as f32
            - 0.02 * f.roof / tall
            - 0.025 * (f.wall - f.base) / tall
            - 0.015 * (1.0 - f.tier)
    };
    let mut best = Fit { hx: hmax * 0.5, hz: hmax * 0.5, wall: tall * 0.5, ..Fit::default() };
    let try_fit = |f: Fit, best: &mut Fit| {
        let v = score_all(&f);
        if v > best.iou {
            *best = Fit { iou: v, ..f };
        }
    };
    // (platform height as a share of the outline's height, the building's size on it)
    const PLATFORMS: [(f32, f32); 8] =
        [(0.0, 1.0), (0.05, 0.8), (0.05, 0.62), (0.1, 0.8), (0.1, 0.62), (0.1, 0.48), (0.18, 0.62), (0.18, 0.48)];
    for i in 0..7 {
        let hx = hmax * (0.35 + 0.1 * i as f32);
        for j in 0..7 {
            let hz = hmax * (0.35 + 0.1 * j as f32);
            for (bf, tier) in PLATFORMS {
                let base = tall * bf;
                for k in 1..=10 {
                    let wall = base + (tall - base) * k as f32 / 10.0;
                    let f0 = Fit { hx, hz, base, tier, wall, ..Fit::default() };
                    try_fit(f0, &mut best);
                    for kind in 1..=4u8 {
                        for rr in 1..=5 {
                            let roof = tall * rr as f32 * 0.14;
                            let (bx, bz) = f0.building();
                            if wall + roof > tall * 1.15 || roof > pitch_cap(kind, bx, bz) {
                                break;
                            }
                            try_fit(Fit { roof, kind, ..f0 }, &mut best);
                        }
                    }
                }
            }
        }
    }
    // the finer search: each measure nudged up and down, the step halving
    let mut step = 0.08;
    for _ in 0..5 {
        for _ in 0..3 {
            let b = best;
            let tries = [
                Fit { hx: b.hx * (1.0 + step), ..b },
                Fit { hx: b.hx * (1.0 - step), ..b },
                Fit { hz: b.hz * (1.0 + step), ..b },
                Fit { hz: b.hz * (1.0 - step), ..b },
                Fit { wall: b.wall * (1.0 + step), ..b },
                Fit { wall: b.wall * (1.0 - step), ..b },
                Fit { roof: b.roof * (1.0 + step), ..b },
                Fit { roof: b.roof * (1.0 - step), ..b },
                Fit { base: b.base * (1.0 + step), ..b },
                Fit { base: b.base * (1.0 - step), ..b },
                Fit { tier: (b.tier * (1.0 + step)).min(1.0), ..b },
                Fit { tier: b.tier * (1.0 - step), ..b },
                Fit { cx: b.cx + hmax * step * 0.5, ..b },
                Fit { cx: b.cx - hmax * step * 0.5, ..b },
                Fit { cz: b.cz + hmax * step * 0.5, ..b },
                Fit { cz: b.cz - hmax * step * 0.5, ..b },
            ];
            for f in tries {
                let (bx, bz) = f.building();
                let ok = f.wall > f.base && f.roof <= pitch_cap(f.kind, bx, bz);
                if ok && half.is_none_or(|h| f.cx.abs() + f.hx <= h * 1.05 && f.cz.abs() + f.hz <= h * 1.05) {
                    try_fit(f, &mut best);
                }
            }
        }
        step *= 0.5;
    }
    // the match itself, without the roof's penalty
    let iou = views.iter().map(|(m, iso)| score(&best, m, iso, s, ax, ay)).sum::<f32>() / views.len() as f32;
    (iou > 0.0).then_some(Fit { iou, ..best })
}

/// The highest roof of a kind on a building: a 45 degree pitch up from the shorter side's half (an outline that seems to
/// want a steeper one is a building on a platform, or tapering).
pub fn pitch_cap(kind: u8, bx: f32, bz: f32) -> f32 {
    match kind {
        1..=4 => bx.min(bz),
        _ => f32::MAX,
    }
}

/// A building's roof, raised: of the hipped and gabled roofs that stay inside every view's outline (the fit losing at most
/// half a percent), the highest, at most a 30 degree pitch. The outline cannot show a roof the art's angle hides, and the
/// art paints its roof in that place: on slopes it shows from the ground, where on a flat top it would be lost.
pub fn raise_roof(fit: Fit, views: &[(&Mask, Iso)], s: f32, ax: f32, ay: f32) -> Fit {
    if fit.kind != 0 || views.is_empty() {
        return fit;
    }
    let score_all = |f: &Fit| views.iter().map(|(m, iso)| score(f, m, iso, s, ax, ay)).sum::<f32>() / views.len() as f32;
    let base = score_all(&fit);
    let (bx, bz) = fit.building();
    let cap = bx.min(bz) * 0.58;
    let mut best = fit;
    for kind in [4u8, 1, 2] {
        // the gable runs along the longer side
        if (kind == 1 && bx < bz) || (kind == 2 && bz < bx) {
            continue;
        }
        let (mut lo, mut hi) = (0.0f32, cap);
        for _ in 0..12 {
            let mid = (lo + hi) * 0.5;
            if score_all(&Fit { roof: mid, kind, ..fit }) >= base - 0.005 {
                lo = mid;
            } else {
                hi = mid;
            }
        }
        if lo > best.roof * 1.05 + 0.5 {
            best = Fit { roof: lo, kind, iou: score_all(&Fit { roof: lo, kind, ..fit }), ..fit };
        }
    }
    best
}

/// The quarter turn whose art paints a face with this outward normal, for an eye looking along heading `yaw` (walk.rs Cam):
/// of the overhead camera's quarter turns that see the face, the one whose heading (45 + 90 q) is nearest the eye's, so
/// the side looked at carries the art drawn from that side, and from an overhead camera's own angle every face it sees
/// carries its own picture (the solid then looks exactly as the sprite does); a face none sees, the one that faces it best.
pub fn face_quarter(n: [f32; 3], yaw: f32) -> i32 {
    let sees = |q: i32| -dot(n, iso(q).f);
    let off = |q: i32| ((45.0 + 90.0 * q as f32 - yaw + 540.0).rem_euclid(360.0) - 180.0).abs();
    let seen: Vec<i32> = (0..4).filter(|&q| sees(q) > 0.02).collect();
    if seen.is_empty() {
        (0..4).max_by(|&a, &b| sees(a).partial_cmp(&sees(b)).unwrap_or(std::cmp::Ordering::Equal).then(b.cmp(&a))).unwrap_or(0)
    } else {
        seen.into_iter().min_by(|&a, &b| off(a).partial_cmp(&off(b)).unwrap_or(std::cmp::Ordering::Equal).then(a.cmp(&b))).unwrap_or(0)
    }
}

// ---- the camera's view of the frame -----------------------------------------------------------------------------------------

/// The six planes of a view's frustum (a x + b y + c z + d >= 0 inside), from its projection times view matrix.
pub fn frustum(vp: &Mat4) -> [[f32; 4]; 6] {
    let m = &vp.0;
    let row = |r: usize| [m[r], m[4 + r], m[8 + r], m[12 + r]];
    let (r0, r1, r2, r3) = (row(0), row(1), row(2), row(3));
    let add = |a: [f32; 4], b: [f32; 4], k: f32| [a[0] + k * b[0], a[1] + k * b[1], a[2] + k * b[2], a[3] + k * b[3]];
    let mut p = [add(r3, r0, 1.0), add(r3, r0, -1.0), add(r3, r1, 1.0), add(r3, r1, -1.0), add(r3, r2, 1.0), add(r3, r2, -1.0)];
    for q in p.iter_mut() {
        let l = (q[0] * q[0] + q[1] * q[1] + q[2] * q[2]).sqrt().max(1e-12);
        *q = q.map(|v| v / l);
    }
    p
}

/// Whether a sphere is at least partly inside the frustum.
pub fn sphere_visible(planes: &[[f32; 4]; 6], c: [f32; 3], r: f32) -> bool {
    planes.iter().all(|p| p[0] * c[0] + p[1] * c[1] + p[2] * c[2] + p[3] >= -r)
}

/// Whether a box (lo..hi) is at least partly inside the frustum.
pub fn box_visible(planes: &[[f32; 4]; 6], lo: [f32; 3], hi: [f32; 3]) -> bool {
    planes.iter().all(|p| {
        // the corner farthest along the plane's normal
        let x = if p[0] >= 0.0 { hi[0] } else { lo[0] };
        let y = if p[1] >= 0.0 { hi[1] } else { lo[1] };
        let z = if p[2] >= 0.0 { hi[2] } else { lo[2] };
        p[0] * x + p[1] * y + p[2] * z + p[3] >= 0.0
    })
}

/// How much of a thing is drawn at a distance: everything near; far off its picture without the shadows under it (people
/// have a shorter limit of their own); nothing past the haze.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Detail {
    Full,
    Far,
    None,
}

/// The detail for a thing at `dist` with the haze's end at `far`.
pub fn detail_at(dist: f32, far: f32) -> Detail {
    if dist > far {
        Detail::None
    } else if dist > far.min(400.0 * M) {
        Detail::Far
    } else {
        Detail::Full
    }
}

/// What the camera sees this frame, for the objects' drawing.
pub struct Scene {
    pub eye: [f32; 3],
    /// The camera's yaw (walk.rs Cam) and the horizontal right, for cards turned to the eye.
    pub yaw: f32,
    pub right: [f32; 2],
    pub planes: [[f32; 4]; 6],
    /// Base uniforms (the mode is set per draw).
    pub u: WalkUniforms,
    /// Where the haze hides everything.
    pub far: f32,
    pub time: f32,
    pub quarter: i32,
}

/// What a sprite frame looks like, worked out once: its opaque texels' bounds and its most colourful colour.
#[derive(Clone, Copy, Debug)]
struct FrameInfo {
    bbox: (usize, usize, usize, usize),
    vivid: [f32; 3],
}

/// The objects' caches: textures of their frames, fitted solids, per-frame facts, and people's eased positions.
#[derive(Default)]
pub struct Objects {
    tex: HashMap<(usize, usize), TextureId>,
    people: HashMap<(usize, usize, sg_core::bodies::Outfit), TextureId>,
    fits: HashMap<(usize, [usize; 4], i32), Option<Fit>>,
    info: HashMap<(usize, usize), FrameInfo>,
    person_scale: HashMap<usize, f32>,
    /// A moving prop's shown place (x, z) by its identity, eased between the simulation's ticks.
    eased: HashMap<u32, (f32, f32)>,
    /// The trees beyond the property (built with the ground, `walk_far_trees`).
    far: Option<(u64, Vec<(TextureId, crate::gfx::Mesh)>)>,
    /// The balls in the air: their last places, by golfer, for their trails.
    trails: HashMap<usize, Vec<[f32; 3]>>,
    /// Sprites the walk's start reported (SG_WALK_DUMP).
    dumped: bool,
    /// The things that stop a walk, from the last frame drawn.
    pub obstacles: Vec<Obstacle>,
    /// How many things were drawn and left out last frame (for the performance report).
    pub drawn: usize,
    pub culled: usize,
}

impl Objects {
    pub fn free(&mut self, g: &mut Gfx) {
        if let Some((_, parts)) = self.far.take() {
            for (_, m) in parts {
                g.delete_mesh(m);
            }
        }
        for t in self.tex.drain().map(|e| e.1).chain(self.people.drain().map(|e| e.1)) {
            g.ctx.delete_texture(t);
        }
    }
}

fn identity(p: &Prop) -> Option<u32> {
    if let Some(g) = p.golfer {
        Some(0x1000_0000 | g as u32)
    } else if let Some(e) = p.employee {
        Some(0x2000_0000 | e as u32)
    } else if let Some(r) = p.resident {
        Some(0x3000_0000 | r as u32)
    } else if p.track != 0 {
        Some(p.track)
    } else {
        None
    }
}

/// The frame of an 8 view person seen from the camera at heading `yaw` (walk.rs Cam): the view whose drawn facing matches
/// the person's heading as the eye sees it. The overhead camera at heading 45 + 90 q draws view (180 - heading) / 45 + 2 q
/// (App::prop_view), so a camera at any heading draws (135 + yaw - heading) / 45, to the nearest of the eight.
pub fn person_view(heading: f32, yaw: f32, mirror: bool) -> i32 {
    let k = ((135.0 + yaw - heading) / 45.0).round() as i32;
    let v = k.rem_euclid(8);
    if mirror {
        (2 - v).rem_euclid(8)
    } else {
        v
    }
}

/// A unit sphere's triangles (an octahedron split twice), for the ball and the tee markers.
fn sphere() -> &'static [[f32; 3]] {
    static S: std::sync::OnceLock<Vec<[f32; 3]>> = std::sync::OnceLock::new();
    S.get_or_init(|| {
        let o = [[1.0, 0.0, 0.0], [-1.0, 0.0, 0.0], [0.0, 1.0, 0.0], [0.0, -1.0, 0.0], [0.0, 0.0, 1.0], [0.0, 0.0, -1.0]];
        let mut t: Vec<[[f32; 3]; 3]> = Vec::new();
        for &a in &[0usize, 1] {
            for &b in &[2usize, 3] {
                for &c in &[4usize, 5] {
                    t.push([o[a], o[b], o[c]]);
                }
            }
        }
        let n = |v: [f32; 3]| {
            let l = dot(v, v).sqrt();
            v.map(|q| q / l)
        };
        for _ in 0..2 {
            let mut next = Vec::new();
            for [a, b, c] in t {
                let m = |p: [f32; 3], q: [f32; 3]| n([(p[0] + q[0]) * 0.5, (p[1] + q[1]) * 0.5, (p[2] + q[2]) * 0.5]);
                let (ab, bc, ca) = (m(a, b), m(b, c), m(c, a));
                next.extend([[a, ab, ca], [ab, b, bc], [ca, bc, c], [ab, bc, ca]]);
            }
            t = next;
        }
        t.into_iter().flatten().collect()
    })
}

/// A ball of radius `r` at `c`.
fn ball_tris(c: [f32; 3], r: f32, col: [f32; 4], out: &mut Vec<Vert>) {
    for &p in sphere() {
        let mut v = Vert::new(c[0] + p[0] * r, c[1] + p[1] * r, c[2] + p[2] * r, 0.0, 0.0).col(col);
        v.normal = p;
        out.push(v);
    }
}

/// An upright pole (a square prism) from `a` up `h`, `r` thick.
fn pole_tris(a: [f32; 3], h: f32, r: f32, col: [f32; 4], out: &mut Vec<Vert>) {
    let c = [(-r, -r), (r, -r), (r, r), (-r, r)];
    for i in 0..4 {
        let (p, q) = (c[i], c[(i + 1) % 4]);
        let n = [(p.0 + q.0) * 0.5 / r, 0.0, (p.1 + q.1) * 0.5 / r];
        let v = |x: f32, y: f32, z: f32| {
            let mut v = Vert::new(a[0] + x, a[1] + y, a[2] + z, 0.0, 0.0).col(col);
            v.normal = n;
            v
        };
        let (a0, a1, b0, b1) = (v(p.0, 0.0, p.1), v(p.0, h, p.1), v(q.0, 0.0, q.1), v(q.0, h, q.1));
        out.extend([a0, b0, b1, a0, b1, a1]);
    }
}

/// A ball's trail: a thin band through its last places turned to the eye, fading out behind it.
fn ribbon(pts: &[[f32; 3]], eye: [f32; 3], out: &mut Vec<Vert>) {
    if pts.len() < 2 {
        return;
    }
    let n = pts.len();
    let side = |k: usize| {
        let (a, b) = (pts[k.saturating_sub(1)], pts[(k + 1).min(n - 1)]);
        let d = [b[0] - a[0], b[1] - a[1], b[2] - a[2]];
        let p = pts[k];
        let v = [eye[0] - p[0], eye[1] - p[1], eye[2] - p[2]];
        let c = [d[1] * v[2] - d[2] * v[1], d[2] * v[0] - d[0] * v[2], d[0] * v[1] - d[1] * v[0]];
        let l = dot(c, c).sqrt().max(1e-6);
        let w = (0.03 * M).max(dot(v, v).sqrt() * 0.0011);
        c.map(|q| q / l * w)
    };
    for k in 0..n - 1 {
        let (s0, s1) = (side(k), side(k + 1));
        let (a0, a1) = (k as f32 / n as f32 * 0.7, (k + 1) as f32 / n as f32 * 0.7);
        let v = |p: [f32; 3], s: [f32; 3], sign: f32, a: f32| {
            let mut v = Vert::new(p[0] + s[0] * sign, p[1] + s[1] * sign, p[2] + s[2] * sign, 0.0, 0.0).col([1.0, 1.0, 1.0, a]);
            v.normal = [0.0; 3];
            v
        };
        let (p, q) = (pts[k], pts[k + 1]);
        let (x0, x1, y0, y1) = (v(p, s0, -1.0, a0), v(p, s0, 1.0, a0), v(q, s1, -1.0, a1), v(q, s1, 1.0, a1));
        out.extend([x0, y0, y1, x0, y1, x1]);
    }
}

/// A soft round shadow on the ground at (x, z), `r` across.
fn blob(app: &App, x: f32, z: f32, r: f32, alpha: f32, out: &mut Vec<Vert>) {
    let n = 2;
    let step = 2.0 * r / n as f32;
    for j in 0..n {
        for i in 0..n {
            let q = |a: usize, b: usize| {
                let (px, pz) = (x - r + a as f32 * step, z - r + b as f32 * step);
                Vert::new(px, app.terrain.height_at(px, pz) + 0.15, pz, a as f32 / n as f32, b as f32 / n as f32)
                    .col([0.0, 0.0, 0.0, alpha])
            };
            let (a, b, c, d) = (q(i, j), q(i + 1, j), q(i + 1, j + 1), q(i, j + 1));
            out.extend([a, b, c, a, c, d]);
        }
    }
}

impl App {
    /// A frame's texture of the walk camera's own (walk_gl::sprite_texture).
    fn walk_tex(&self, g: &mut Gfx, o: &mut Objects, si: usize, i: usize) -> TextureId {
        if let Some(&t) = o.tex.get(&(si, i)) {
            return t;
        }
        let t = sprite_texture(g, &self.sprites[si].s.frame(i));
        o.tex.insert((si, i), t);
        t
    }

    fn walk_frame_info(&self, o: &mut Objects, si: usize, i: usize) -> FrameInfo {
        if let Some(&f) = o.info.get(&(si, i)) {
            return f;
        }
        let img = self.sprites[si].s.frame(i);
        let m = Mask::new(&img);
        // the most colourful opaque texels' mean: a flag's cloth, a tee marker's paint
        let mut best: Vec<(f32, [f32; 3])> = img
            .px
            .chunks_exact(4)
            .filter(|p| p[3] > 127)
            .map(|p| {
                let c = [p[0] as f32 / 255.0, p[1] as f32 / 255.0, p[2] as f32 / 255.0];
                let (mx, mn) = (c[0].max(c[1]).max(c[2]), c[0].min(c[1]).min(c[2]));
                (mx - mn, c)
            })
            .collect();
        best.sort_by(|a, b| b.0.partial_cmp(&a.0).unwrap_or(std::cmp::Ordering::Equal));
        let top = &best[..best.len().min(12)];
        let vivid = if top.is_empty() {
            [0.8, 0.1, 0.1]
        } else {
            let s = top.iter().fold([0.0; 3], |a, (_, c)| [a[0] + c[0], a[1] + c[1], a[2] + c[2]]);
            s.map(|v| v / top.len() as f32)
        };
        let f = FrameInfo { bbox: m.bbox, vivid };
        o.info.insert((si, i), f);
        f
    }

    /// The fitted solid of an object's sprite (cached), from the frames its four quarter turns draw (`frames`, the
    /// first frame of each view), its footprint at most `half` from its ground point.
    fn walk_fit(&self, o: &mut Objects, si: usize, frames: [usize; 4], scale: f32, half: Option<f32>) -> Option<Fit> {
        let key = (si, frames, half.map(|h| h as i32).unwrap_or(-1));
        if let Some(f) = o.fits.get(&key) {
            return *f;
        }
        let sp = &self.sprites[si].s;
        let masks: Vec<Mask> = frames.iter().map(|&i| Mask::new(&sp.frame(i))).collect();
        let views: Vec<(&Mask, Iso)> = masks.iter().enumerate().map(|(q, m)| (m, iso(q as i32))).collect();
        let s = SPRITE_UNITS_PER_PIXEL * scale;
        let (ax, ay) = (sp.anchor_x as f32, sp.anchor_y as f32);
        // a building on more than one tile always stands as a solid (its card would lean back across the ground it covers);
        // a small thing only when the solid fits it well
        let big = half.is_some_and(|h| h > TILE_SIZE * 0.75);
        let fit = fit_proxy(&views, s, ax, ay, half).filter(|f| f.iou >= if big { MIN_FIT_BUILDING } else { MIN_FIT }).map(|f| {
            if half.is_some() && f.iou >= RAISE_FIT {
                raise_roof(f, &views, s, ax, ay)
            } else if f.iou < RAISE_FIT {
                // a loose fit: towers, cupolas and the like stand out of the solid's outline, and a roof would wear them
                // smeared across its slopes; a flat top leaves them on the unseen top and the walls clean
                Fit { roof: 0.0, kind: 0, ..f }
            } else {
                f
            }
        });
        if std::env::var_os("SG_WALK_DUMP").is_some() {
            println!("walk fit: sprite {si} frames {frames:?} ({}x{}) half {half:?}: {fit:?}", sp.w, sp.h);
            // and the frames, for looking at what was fitted
            if let Some(dir) = std::env::var_os("SG_WALK_DUMP_DIR") {
                for (q, &i) in frames.iter().enumerate() {
                    sg_core::png::write_png(std::path::Path::new(&dir).join(format!("sprite{si}-q{q}.png")), &sp.frame(i));
                }
            }
        }
        o.fits.insert(key, fit);
        fit
    }

    /// The scale that gives a person's sprite a person's height (its first frame's opaque height, drawn upright).
    fn walk_person_scale(&self, o: &mut Objects, si: usize) -> f32 {
        if let Some(&k) = o.person_scale.get(&si) {
            return k;
        }
        let m = Mask::new(&self.sprites[si].s.frame(0));
        let h = (m.bbox.3.saturating_sub(m.bbox.1)).max(8) as f32;
        let k = PERSON * art_pitch().to_radians().cos() / (h * SPRITE_UNITS_PER_PIXEL);
        o.person_scale.insert(si, k);
        k
    }

    /// The land objects' footprints: half their size in tiles (world units) by their ground point.
    fn walk_footprints(&self) -> Vec<((f32, f32), f32)> {
        let Some(land) = &self.land else { return Vec::new() };
        land.objects
            .iter()
            .filter(|o| o.kind >= 0)
            .map(|o| {
                let size = sg_core::data::buildings().get(o.kind as usize).map(|b| b.1).unwrap_or(1).max(1);
                let (cx, cz) = self.terrain.tile_centre(o.a, o.b);
                let off = (size - 1) as f32 * TILE_SIZE * 0.5;
                ((cx + off, cz + off), size as f32 * TILE_SIZE * 0.5)
            })
            .collect()
    }

    /// Lays a sprite frame on the ground by the overhead projection: the ground under it in a grid that follows the terrain.
    #[allow(clippy::too_many_arguments)]
    fn walk_decal(&self, o: &mut Objects, si: usize, i: usize, x: f32, z: f32, s: f32, col: [f32; 4], out: &mut Vec<Vert>) {
        self.walk_decal_rows(o, si, i, (x, z), s, None, col, out, 0);
    }

    /// `walk_decal` of the frame's rows from `from` down only (its quarter turn `q`'s projection).
    #[allow(clippy::too_many_arguments)]
    fn walk_decal_rows(
        &self,
        o: &mut Objects,
        si: usize,
        i: usize,
        (x, z): (f32, f32),
        s: f32,
        from: Option<f32>,
        col: [f32; 4],
        out: &mut Vec<Vert>,
        q: i32,
    ) {
        let info = self.walk_frame_info(o, si, i);
        let sp = &self.sprites[si].s;
        let (w, h, ax, ay) = (sp.w as f32, sp.h as f32, sp.anchor_x as f32, sp.anchor_y as f32);
        let (l, t, r, b) = (info.bbox.0 as f32 - 1.0, info.bbox.1 as f32 - 1.0, info.bbox.2 as f32 + 1.0, info.bbox.3 as f32 + 1.0);
        let t = from.map(|f| f.max(t)).unwrap_or(t);
        if r <= l || b <= t {
            return;
        }
        let iso = iso(q);
        let corner = |px: f32, py: f32| iso.ground_of(px, py, s, ax, ay);
        let (p00, p10, p01) = (corner(l, t), corner(r, t), corner(l, b));
        let len = |a: (f32, f32), b: (f32, f32)| ((a.0 - b.0).powi(2) + (a.1 - b.1).powi(2)).sqrt();
        let (nx, ny) = (((len(p00, p10) / 40.0).ceil() as usize).clamp(1, 12), ((len(p00, p01) / 40.0).ceil() as usize).clamp(1, 12));
        let mut grid = Vec::with_capacity((nx + 1) * (ny + 1));
        for j in 0..=ny {
            for k in 0..=nx {
                let (px, py) = (l + (r - l) * k as f32 / nx as f32, t + (b - t) * j as f32 / ny as f32);
                let (gx, gz) = corner(px, py);
                let (wx, wz) = (x + gx, z + gz);
                grid.push(Vert::new(wx, self.terrain.height_at(wx, wz) + 0.2, wz, px / w, py / h).col(col));
            }
        }
        for j in 0..ny {
            for k in 0..nx {
                let at = |a: usize, b: usize| grid[b * (nx + 1) + a];
                let (a, b2, c, d) = (at(k, j), at(k + 1, j), at(k + 1, j + 1), at(k, j + 1));
                out.extend([a, b2, c, a, c, d]);
            }
        }
    }
}

/// The vertices of a frame's draws, grouped by texture in the order first met.
#[derive(Default)]
struct Lists {
    order: Vec<TextureId>,
    by: HashMap<TextureId, Vec<Vert>>,
}

impl Lists {
    fn get(&mut self, t: TextureId) -> &mut Vec<Vert> {
        if !self.by.contains_key(&t) {
            self.order.push(t);
        }
        self.by.entry(t).or_default()
    }
    fn draw(self, gpu: &mut WalkGpu, p: Pipe, u: &WalkUniforms) {
        let mut by = self.by;
        for t in self.order {
            if let Some(v) = by.remove(&t) {
                gpu.tris(p, t, u, &v);
            }
        }
    }
}

/// A picture on a card: its corners at `o` plus `right` times x and `up` times y for x in l..r and y in b..t, the whole
/// texture across it (mirrored with `flip`); `sway` moves its top along `right`.
#[allow(clippy::too_many_arguments)]
fn card(
    out: &mut Vec<Vert>,
    o: [f32; 3],
    right: [f32; 3],
    up: [f32; 3],
    (l, r, t, b): (f32, f32, f32, f32),
    flip: bool,
    sway: f32,
    col: [f32; 4],
) {
    let f = if flip { -1.0 } else { 1.0 };
    let c = |x: f32, y: f32, tu: f32, tv: f32, top: bool| {
        let x = x * f + if top { sway } else { 0.0 };
        Vert::new(o[0] + right[0] * x + up[0] * y, o[1] + right[1] * x + up[1] * y, o[2] + right[2] * x + up[2] * y, tu, tv).col(col)
    };
    let (a, b2, c2, d) = (c(l, t, 0.0, 0.0, true), c(r, t, 1.0, 0.0, true), c(r, b, 1.0, 1.0, false), c(l, b, 0.0, 1.0, false));
    out.extend([a, b2, c2, a, c2, d]);
}

/// A number from 0 to 1 that stays with a place (a tree's sway).
fn place_hash(x: f32, z: f32) -> f32 {
    let h = ((x * 12.9898 + z * 78.233).sin() * 43758.547).fract();
    h.abs()
}

impl App {
    /// Draws everything standing on the ground (see the module's notes) into the frame's stream, and notes what stops a
    /// walk near the eye.
    pub(crate) fn walk_props(&mut self, g: &mut Gfx, gpu: &mut WalkGpu, o: &mut Objects, sc: &Scene, dt: f32) {
        o.obstacles.clear();
        o.drawn = 0;
        o.culled = 0;
        if !self.show_props {
            return;
        }
        self.update_props();
        if !o.dumped && std::env::var_os("SG_WALK_DUMP").is_some() {
            o.dumped = true;
            for p in self.props.iter().filter(|p| !p.hidden) {
                if let Some(si) = p.body {
                    let sp = &self.sprites[si].s;
                    println!(
                        "walk prop: sprite {si} {}x{} anchor {},{} views {} object {} tree {} flat {} decor {} golfer {:?} scale {}",
                        sp.w, sp.h, sp.anchor_x, sp.anchor_y, sp.views, p.object, p.tree, p.flat, p.decor, p.golfer, p.scale
                    );
                }
            }
        }
        let foot = self.walk_footprints();
        let half_at = |x: f32, z: f32| foot.iter().find(|((fx, fz), _)| (fx - x).abs() < 1.0 && (fz - z).abs() < 1.0).map(|f| f.1);
        let eye = sc.eye;
        let up = [0.0, 1.0, 0.0];
        let right = [sc.right[0], 0.0, sc.right[1]];
        let cos_p = art_pitch().to_radians().cos();
        let (mut cut, mut over, mut decals) = (Lists::default(), Lists::default(), Lists::default());
        // people and small things stand where a tree or a building's picture may stand too (an animal at a cactus): they
        // are drawn a little towards the eye, in front, as the overhead view draws them over it
        let mut front = Lists::default();
        let (mut blobs, mut solids) = (Vec::new(), Vec::new());
        let mut proxies: Vec<((f32, f32), Fit)> = Vec::new();
        let mut eased: HashMap<u32, (f32, f32)> = HashMap::new();
        let white = [1.0; 4];
        let near_obstacle = 120.0 * M;
        for idx in 0..self.props.len() {
            let p = self.props[idx].clone();
            if p.hidden {
                continue;
            }
            let Some(si) = p.body else { continue };
            let (w, h, ax, ay, views) = {
                let sp = &self.sprites[si].s;
                (sp.w as f32, sp.h as f32, sp.anchor_x as f32, sp.anchor_y as f32, sp.views)
            };
            let person = p.golfer.is_some() || p.employee.is_some() || p.resident.is_some() || (views >= 8 && p.heading > -999.0);
            // a moving thing is shown eased between the simulation's ticks
            let (x, z) = match identity(&p).filter(|_| person || p.ambient) {
                Some(k) => {
                    let prev = o.eased.get(&k).copied().unwrap_or((p.x, p.z));
                    let (dx, dz) = (p.x - prev.0, p.z - prev.1);
                    let e = if dx * dx + dz * dz > (4.0 * M) * (4.0 * M) {
                        (p.x, p.z)
                    } else {
                        let k = (dt * 9.0).min(1.0);
                        (prev.0 + dx * k, prev.1 + dz * k)
                    };
                    eased.insert(k, e);
                    e
                }
                None => (p.x, p.z),
            };
            let ground = self.terrain.height_at(x, z);
            let (ex, ez) = (x - eye[0], z - eye[2]);
            let dist = (ex * ex + ez * ez + (ground - eye[1]).powi(2)).sqrt();
            let level = detail_at(dist, sc.far);
            if level == Detail::None {
                o.culled += 1;
                continue;
            }
            let s = SPRITE_UNITS_PER_PIXEL * p.scale;
            if p.decor {
                self.walk_marker(o, &p, si, x, z, dist, &mut solids, &mut blobs);
                continue;
            }
            if p.flat {
                let r = (w.max(h)) * s;
                if !sphere_visible(&sc.planes, [x, ground, z], r) {
                    o.culled += 1;
                    continue;
                }
                let i = self.sprites[si].s.frame_index(self.prop_view(&p, si, 0), p.frame);
                let t = self.walk_tex(g, o, si, i);
                let mut v = Vec::new();
                self.walk_decal(o, si, i, x, z, s, white, &mut v);
                decals.get(t).extend(v);
                o.drawn += 1;
                continue;
            }
            let anchor = [x, ground + p.lift, z];
            if p.object {
                let half = half_at(x, z);
                let frames = [0, 1, 2, 3].map(|q| self.sprites[si].s.frame_index(self.prop_view(&p, si, q), 0));
                let layer = proxies.iter().find(|((px, pz), _)| (px - x).abs() < 0.5 && (pz - z).abs() < 0.5).map(|f| f.1);
                let fit = match layer {
                    Some(f) => Some(f),
                    // (test hook: SG_WALK_CARDS draws every object as its card)
                    None if std::env::var_os("SG_WALK_CARDS").is_some() => None,
                    None => self.walk_fit(o, si, frames, p.scale, half),
                };
                let radius = (w.max(h)) * s;
                if !sphere_visible(&sc.planes, [x, ground + radius * 0.4, z], radius) {
                    o.culled += 1;
                    if let Some(f) = fit.filter(|_| dist < near_obstacle) {
                        o.obstacles.push(Obstacle::Box { cx: x + f.cx, cz: z + f.cz, hx: f.hx, hz: f.hz });
                    }
                    continue;
                }
                o.drawn += 1;
                // (a small thing with no solid is drawn shrunk, with its shadow)
                let small = fit.is_none() && !half.is_some_and(|hf| hf > TILE_SIZE * 0.75);
                if let (Some(sh), true) = (p.shadow, layer.is_none()) {
                    let i = self.sprites[sh].s.frame_index(self.prop_view(&p, sh, 0), p.frame);
                    let t = self.walk_tex(g, o, sh, i);
                    let mut v = Vec::new();
                    self.walk_decal(o, sh, i, x, z, if small { s * SMALL_OBJECT } else { s }, white, &mut v);
                    decals.get(t).extend(v);
                }
                match fit {
                    Some(f) => {
                        if layer.is_none() {
                            proxies.push(((x, z), f));
                            if dist < near_obstacle {
                                o.obstacles.push(Obstacle::Box { cx: x + f.cx, cz: z + f.cz, hx: f.hx, hz: f.hz });
                            }
                        }
                        let list = if layer.is_some() { &mut over } else { &mut cut };
                        for (poly, n) in f.faces() {
                            let q = face_quarter(n, sc.yaw);
                            let i = self.sprites[si].s.frame_index(self.prop_view(&p, si, q), p.frame);
                            let t = self.walk_tex(g, o, si, i);
                            let iq = iso(q);
                            let vert = |d: [f32; 3]| {
                                let (px, py) = iq.to_px(d, s, ax, ay);
                                let mut v = Vert::new(anchor[0] + d[0], anchor[1] + d[1], anchor[2] + d[2], px / w, py / h);
                                v.normal = n;
                                v
                            };
                            let vs: Vec<Vert> = poly.iter().map(|&d| vert(d)).collect();
                            let l = list.get(t);
                            for k in 1..vs.len() - 1 {
                                l.extend([vs[0], vs[k], vs[k + 1]]);
                            }
                        }
                    }
                    None => {
                        // no solid fits: a small thing's picture as the overhead view draws it, leaning back at the art's
                        // angle, shrunk like the trees (they are all drawn large); a building's at its size
                        let k = if small { s * SMALL_OBJECT } else { s };
                        let iq = iso(sc.quarter);
                        let i = self.sprites[si].s.frame_index(self.prop_view(&p, si, sc.quarter), p.frame);
                        let t = self.walk_tex(g, o, si, i);
                        if small {
                            let rect = (-ax * k, (w - ax) * k, ay * k, -(h - ay) * k);
                            card(cut.get(t), anchor, iq.r, iq.u, rect, false, 0.0, white);
                        } else {
                            // a building: bent at its ground point, its picture above leaning back at the art's angle and
                            // below (its front, its deck, the planting before it) laid on the ground, so from the overhead
                            // camera it is the sprite itself and nothing of it sinks into the ground
                            let c = |cx: f32, cy: f32, tu: f32, tv: f32| {
                                let p = |a: usize| anchor[a] + iq.r[a] * cx + iq.u[a] * cy;
                                Vert::new(p(0), p(1), p(2), tu, tv)
                            };
                            let (l, r, top, v1) = (-ax * k, (w - ax) * k, ay * k, ay / h);
                            let q = [c(l, top, 0.0, 0.0), c(r, top, 1.0, 0.0), c(r, 0.0, 1.0, v1), c(l, 0.0, 0.0, v1)];
                            cut.get(t).extend([q[0], q[1], q[2], q[0], q[2], q[3]]);
                            let mut v = Vec::new();
                            self.walk_decal_rows(o, si, i, (x, z), k, Some(ay), white, &mut v, sc.quarter);
                            // (over the art's own shadow, which lies under it)
                            front.get(t).extend(v);
                        }
                        if dist < near_obstacle {
                            let r = half.map(|h| h * 0.7).unwrap_or(((w * 0.25) * s).min(3.0 * M));
                            o.obstacles.push(Obstacle::Circle { x, z, r });
                        }
                    }
                }
                continue;
            }
            if p.tree {
                let k = s * TREE_SCALE;
                let v = 1.12;
                let height = ay * k * v;
                if !sphere_visible(&sc.planes, [x, ground + height * 0.5, z], height.max(w * k)) {
                    o.culled += 1;
                    continue;
                }
                o.drawn += 1;
                let i = self.sprites[si].s.frame_index(self.prop_view(&p, si, sc.quarter), p.frame);
                let t = self.walk_tex(g, o, si, i);
                let ph = place_hash(x, z);
                let sway =
                    ((sc.time * (0.9 + ph * 0.5) + ph * std::f32::consts::TAU).sin() * 0.12 + (sc.time * 2.3 + ph * 3.0).sin() * 0.04) * M;
                let rect = (-ax * k, (w - ax) * k, ay * k * v, -(h - ay) * k * v);
                card(cut.get(t), anchor, right, up, rect, false, sway, white);
                if level != Detail::Far {
                    if let Some(sh) = p.shadow {
                        let i = self.sprites[sh].s.frame_index(self.prop_view(&p, sh, 0), p.frame);
                        let t = self.walk_tex(g, o, sh, i);
                        let mut vv = Vec::new();
                        self.walk_decal(o, sh, i, x, z, k, white, &mut vv);
                        decals.get(t).extend(vv);
                    }
                    blob(self, x, z, 1.1 * M, 0.35, &mut blobs);
                }
                if dist < near_obstacle {
                    o.obstacles.push(Obstacle::Circle { x, z, r: 0.7 * M });
                }
                continue;
            }
            if person {
                if dist > 260.0 * M {
                    o.culled += 1;
                    continue;
                }
                let k = self.walk_person_scale(o, si) * p.scale;
                let (sh, sv) = (SPRITE_UNITS_PER_PIXEL * k, SPRITE_UNITS_PER_PIXEL * k / cos_p);
                let lift = p.lift * k;
                if !sphere_visible(&sc.planes, [x, ground + lift + PERSON * 0.5, z], PERSON) {
                    o.culled += 1;
                    continue;
                }
                o.drawn += 1;
                let view = match p.view {
                    Some(v) => v.rem_euclid(views.max(1)),
                    None if views >= 8 && p.heading > -999.0 => person_view(p.heading, sc.yaw, p.mirror),
                    None => self.prop_view(&p, si, sc.quarter),
                };
                let i = self.sprites[si].s.frame_index(view, p.frame);
                let t = match p.outfit {
                    Some(of) => self.walk_person_tex(g, o, si, i, of),
                    None => self.walk_tex(g, o, si, i),
                };
                let flip = p.mirror && views < 8;
                let rect = (-ax * sh, (w - ax) * sh, ay * sv, -(h - ay) * sv);
                card(front.get(t), [x, ground + lift, z], right, up, rect, flip, 0.0, white);
                blob(self, x, z, 0.5 * M, 0.38, &mut blobs);
                continue;
            }
            // anything else (animals, weeds, ripples): a small card leaning back as the art was drawn
            let k = s * SMALL_SCALE;
            if !sphere_visible(&sc.planes, [x, ground, z], w.max(h) * k) || dist > 300.0 * M {
                o.culled += 1;
                continue;
            }
            o.drawn += 1;
            let iq = iso(sc.quarter);
            let i = self.sprites[si].s.frame_index(self.prop_view(&p, si, sc.quarter), p.frame);
            let t = self.walk_tex(g, o, si, i);
            let a = [x, ground + p.lift * SMALL_SCALE, z];
            let rect = (-ax * k, (w - ax) * k, ay * k, -(h - ay) * k);
            card(front.get(t), a, iq.r, iq.u, rect, p.mirror, 0.0, white);
        }
        o.eased = eased;
        let mut trails = Vec::new();
        self.walk_balls(sc, &mut o.trails, &mut solids, &mut blobs, &mut trails);
        let mut u = sc.u;
        u.pull[3] = 0.0012;
        decals.draw(gpu, Pipe::Blend, &u.with_mode(mode::DECAL));
        gpu.tris(Pipe::Blend, g.white, &u.with_mode(mode::BLOB), &blobs);
        let mut uc = sc.u.with_mode(mode::CUTOUT);
        uc.zenith[3] = 0.5;
        cut.draw(gpu, Pipe::Solid, &uc);
        over.draw(gpu, Pipe::Solid, &uc);
        let mut uf = uc;
        uf.pull[3] = 0.004;
        front.draw(gpu, Pipe::Solid, &uf);
        gpu.tris(Pipe::Solid, g.white, &sc.u.with_mode(mode::FLAT), &solids);
        gpu.tris(Pipe::Blend, g.white, &sc.u.with_mode(mode::FLAT), &trails);
    }

    /// A person's frame in an outfit's colours, as a texture of the walk camera's own (cached).
    fn walk_person_tex(&self, g: &mut Gfx, o: &mut Objects, si: usize, i: usize, of: sg_core::bodies::Outfit) -> TextureId {
        if let Some(&t) = o.people.get(&(si, i, of)) {
            return t;
        }
        let img = match self.swaps.compose(&of).and_then(|pal| sg_core::sprites::recolour(&self.sprites[si].s, i, &pal)) {
            Some(img) => img,
            None => return self.walk_tex(g, o, si, i),
        };
        if o.people.len() > 4000 {
            let keys: Vec<_> = o.people.keys().copied().take(2000).collect();
            for k in keys {
                if let Some(t) = o.people.remove(&k) {
                    g.ctx.delete_texture(t);
                }
            }
        }
        let t = sprite_texture(g, &img);
        o.people.insert((si, i, of), t);
        t
    }

    /// A flag on a green (its pole, cloth and cup) or a tee's markers, in the colours of the art's own sprite.
    #[allow(clippy::too_many_arguments)]
    fn walk_marker(&self, o: &mut Objects, p: &Prop, si: usize, x: f32, z: f32, dist: f32, solids: &mut Vec<Vert>, blobs: &mut Vec<Vert>) {
        let i = self.sprites[si].s.frame_index(self.prop_view(p, si, 0), 0);
        let vivid = self.walk_frame_info(o, si, i).vivid;
        let col = [vivid[0], vivid[1], vivid[2], 1.0];
        let ground = self.terrain.height_at(x, z);
        let tile =
            ((x / TILE_SIZE + self.terrain.w as f32 * 0.5).floor() as i32, (z / TILE_SIZE + self.terrain.h as f32 * 0.5).floor() as i32);
        // thin things are kept a pixel or so wide far off
        let thick = |r: f32| r.max(dist * 0.0011);
        match self.terrain.type_at(tile.0, tile.1) {
            1 | 26 => {
                let top = 2.3 * M;
                pole_tris([x, ground - 0.1 * M, z], top, thick(0.022 * M), [0.95, 0.95, 0.92, 1.0], solids);
                // the cloth, waving in the wind along +x
                let t = self.clock as f32;
                let (fw, fh) = (0.55 * M, 0.38 * M);
                let n = 5;
                for k in 0..n {
                    let a = |j: usize| {
                        let u = j as f32 / n as f32;
                        let wave = (t * 5.0 - u * 4.0).sin() * 0.06 * M * u;
                        (x + thick(0.022 * M) + u * fw, z + wave)
                    };
                    let ((x0, z0), (x1, z1)) = (a(k), a(k + 1));
                    let y1 = ground + top;
                    let y0 = y1 - fh;
                    let mut q = [
                        Vert::new(x0, y1, z0, 0.0, 0.0).col(col),
                        Vert::new(x1, y1, z1, 0.0, 0.0).col(col),
                        Vert::new(x1, y0, z1, 0.0, 0.0).col(col),
                        Vert::new(x0, y0, z0, 0.0, 0.0).col(col),
                    ];
                    for v in q.iter_mut() {
                        v.normal = [0.0, 0.25, 1.0];
                    }
                    solids.extend([q[0], q[1], q[2], q[0], q[2], q[3]]);
                }
                // the cup
                blob(self, x, z, 0.16 * M, 0.85, blobs);
            }
            _ => {
                // two markers either side of the tee, across the line to its hole's pin
                let pin = self
                    .club
                    .holes
                    .iter()
                    .filter(|h| h.par > 0 && (h.back == tile || h.fwd == tile))
                    .map(|h| self.terrain.tile_centre(h.pin.0, h.pin.1))
                    .next();
                let (dx, dz) = match pin {
                    Some((px, pz)) => {
                        let (dx, dz) = (px - x, pz - z);
                        let l = (dx * dx + dz * dz).sqrt().max(1e-3);
                        (dx / l, dz / l)
                    }
                    None => (0.0, -1.0),
                };
                let (sx, sz) = (-dz * 2.2 * M, dx * 2.2 * M);
                for k in [-1.0, 1.0] {
                    let (mx, mz) = (x + sx * k, z + sz * k);
                    let r = thick(0.11 * M);
                    ball_tris([mx, self.terrain.height_at(mx, mz) + r * 0.8, mz], r, col, solids);
                }
            }
        }
    }

    /// Where golfer `i`'s ball is (world), if it has one on the course.
    pub(crate) fn walk_ball(&self, i: usize) -> Option<[f32; 3]> {
        use sg_core::golfer::flag;
        let gl = self.club.g.get(i)?;
        if gl.bx == 0 || gl.hole <= 0 || gl.hole >= 19 || gl.flags & flag::PENALTY != 0 {
            return None;
        }
        let (bx, bz) = self.units_to_world(gl.bx, gl.by);
        Some([bx, self.terrain.height_at(bx, bz) + gl.bz as f32 * sg_core::terrain::HEIGHT_STEP / 20.0, bz])
    }

    fn walk_balls(
        &self,
        sc: &Scene,
        trails: &mut HashMap<usize, Vec<[f32; 3]>>,
        solids: &mut Vec<Vert>,
        blobs: &mut Vec<Vert>,
        bands: &mut Vec<Vert>,
    ) {
        for i in 0..self.club.g.len().min(sg_core::golfer::SLOTS) {
            let Some(b) = self.walk_ball(i) else {
                trails.remove(&i);
                continue;
            };
            // a ball in the air leaves a short fading trail, so it can be seen against the sky
            let flying =
                self.club.g[i].flags & sg_core::golfer::flag::BALL_MOVING != 0 && b[1] > self.terrain.height_at(b[0], b[2]) + 0.5 * M;
            let tr = trails.entry(i).or_default();
            if flying {
                if tr.last().is_none_or(|l| (l[0] - b[0]).abs() + (l[1] - b[1]).abs() + (l[2] - b[2]).abs() > 0.2 * M) {
                    tr.push(b);
                }
                if tr.len() > 40 {
                    tr.remove(0);
                }
            } else if !tr.is_empty() {
                tr.remove(0);
            }
            ribbon(tr, sc.eye, bands);
            let d = ((b[0] - sc.eye[0]).powi(2) + (b[1] - sc.eye[1]).powi(2) + (b[2] - sc.eye[2]).powi(2)).sqrt();
            if d > 300.0 * M {
                continue;
            }
            let r = (0.045 * M).max(d * 0.0016);
            if !sphere_visible(&sc.planes, b, r * 2.0) {
                continue;
            }
            ball_tris([b[0], b[1] + r, b[2]], r, [1.0, 1.0, 1.0, 1.0], solids);
            blob(self, b[0], b[2], r * 1.6, 0.4, blobs);
        }
    }
}

/// How far beyond the property's edge its trees go on.
const FAR_TREES: f32 = 1300.0 * M;

impl App {
    /// The trees beyond the property: the course's own kinds of tree in clumps over the land out there (none on the sea),
    /// thinning with distance, as two crossed cards each in one mesh per picture, made once with the ground.
    pub(crate) fn walk_far_trees(&mut self, g: &mut Gfx, o: &mut Objects, ground: &crate::walk_ground::Ground) {
        if o.far.as_ref().is_some_and(|f| f.0 == ground.key) {
            return;
        }
        if let Some((_, parts)) = o.far.take() {
            for (_, m) in parts {
                g.delete_mesh(m);
            }
        }
        self.update_props();
        // each kind of tree in the frame the course shows it in (a young tree's frames are its growth)
        let mut kinds: Vec<(usize, usize)> = self
            .props
            .iter()
            .filter(|p| p.tree && !p.hidden && p.sprout.is_none())
            .filter_map(|p| p.body.map(|si| (si, self.sprites[si].s.frame_index(self.prop_view(p, si, 0), p.frame))))
            .collect();
        kinds.sort_unstable();
        kinds.dedup();
        let mut by: HashMap<TextureId, Vec<Vert>> = HashMap::new();
        if !kinds.is_empty() {
            let (hx, hz) = (self.terrain.w as f32 * TILE_SIZE * 0.5, self.terrain.h as f32 * TILE_SIZE * 0.5);
            let step = 14.0 * M;
            let (nx, nz) = (((hx + FAR_TREES) / step) as i32, ((hz + FAR_TREES) / step) as i32);
            for j in -nz..=nz {
                for i in -nx..=nx {
                    let h1 = place_hash(i as f32 * 1.37, j as f32 * 2.11);
                    let h2 = place_hash(i as f32 * 3.07 + 11.0, j as f32 * 0.91 - 5.0);
                    let (x, z) = ((i as f32 + h1 - 0.5) * step, (j as f32 + h2 - 0.5) * step);
                    let d = (x.abs() - hx).max(z.abs() - hz);
                    if !(4.0 * M..=FAR_TREES).contains(&d) {
                        continue;
                    }
                    // clumps of wood, thinning out with distance
                    let clump = crate::walk_ground::noise2(x / (220.0 * M), z / (220.0 * M), 77);
                    let keep = ((clump - 0.56) / 0.2).clamp(0.0, 1.0) * 0.75 * (1.0 - d / FAR_TREES).sqrt() + 0.01;
                    if place_hash(x * 0.37, z * 0.53) > keep {
                        continue;
                    }
                    let Some((y, wet)) = ground.beyond.at(x, z) else { continue };
                    if wet > 0.02 {
                        continue;
                    }
                    let (si, i0) = kinds[((h1 * 7919.0) as usize) % kinds.len()];
                    let sp = &self.sprites[si].s;
                    let (w, h, ax, ay) = (sp.w as f32, sp.h as f32, sp.anchor_x as f32, sp.anchor_y as f32);
                    let t = self.walk_tex(g, o, si, i0);
                    let k = SPRITE_UNITS_PER_PIXEL * TREE_SCALE * (0.8 + h2 * 0.5);
                    let a = h1 * std::f32::consts::PI;
                    let list = by.entry(t).or_default();
                    for turn in [0.0, std::f32::consts::FRAC_PI_2] {
                        let (sn, cs) = (a + turn).sin_cos();
                        let mut v = Vec::new();
                        card(
                            &mut v,
                            [x, y, z],
                            [cs, 0.0, sn],
                            [0.0, 1.0, 0.0],
                            (-ax * k, (w - ax) * k, ay * k * 1.12, -(h - ay) * k * 1.12),
                            false,
                            0.0,
                            [1.0; 4],
                        );
                        for vv in v.iter_mut() {
                            vv.normal = [0.0; 3];
                        }
                        list.extend(v);
                    }
                }
            }
        }
        if std::env::var_os("SG_WALK_DUMP").is_some() {
            println!("walk far trees: {} kinds, {} trees", kinds.len(), by.values().map(|v| v.len() / 12).sum::<usize>());
        }
        let parts = by.into_iter().map(|(t, v)| (t, g.mesh(&v))).collect();
        o.far = Some((ground.key, parts));
    }

    /// Draws the trees beyond the property.
    pub(crate) fn walk_draw_far_trees(&self, g: &mut Gfx, gpu: &WalkGpu, o: &Objects, u: &WalkUniforms) {
        let Some((_, parts)) = &o.far else { return };
        let u = u.with_mode(mode::CUTOUT).with_ref(0.5);
        for (t, m) in parts {
            gpu.draw_mesh(g, Pipe::Solid, m, *t, *t, &u);
        }
    }
}

impl WalkUniforms {
    pub fn with_ref(mut self, r: f32) -> Self {
        self.zenith[3] = r;
        self
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    /// The art of a box seen from the overhead camera: its projected outline filled in a picture.
    fn render(f: &Fit, iso: &Iso, s: f32, w: u32, h: u32, ax: f32, ay: f32) -> Rgba {
        let poly = hull(f.points().into_iter().map(|p| iso.to_px(p, s, ax, ay)).collect());
        let mut img = Rgba::new(w, h);
        for x in 0..w as usize {
            if let Some((a, b)) = span_at(&poly, x as f32 + 0.5) {
                for y in a.round().max(0.0) as usize..(b.round().max(0.0) as usize).min(h as usize) {
                    img.px[(y * w as usize + x) * 4 + 3] = 255;
                }
            }
        }
        img
    }

    #[test]
    fn a_box_is_fitted_back_from_its_own_picture() {
        let iso = iso(0);
        let s = SPRITE_UNITS_PER_PIXEL;
        for truth in [
            Fit { hx: 120.0, hz: 120.0, wall: 60.0, roof: 50.0, kind: 1, ..Fit::default() },
            Fit { hx: 90.0, hz: 140.0, wall: 80.0, roof: 0.0, kind: 0, ..Fit::default() },
            Fit { hx: 80.0, hz: 80.0, wall: 60.0, roof: 75.0, kind: 3, ..Fit::default() },
            Fit { hx: 120.0, hz: 120.0, base: 20.0, tier: 0.62, wall: 80.0, roof: 0.0, kind: 0, ..Fit::default() },
            Fit { hx: 140.0, hz: 80.0, wall: 45.0, roof: 50.0, kind: 4, ..Fit::default() },
        ] {
            let imgs: Vec<Rgba> = (0..4).map(|q| render(&truth, &super::iso(q), s, 200, 200, 100.0, 140.0)).collect();
            let masks: Vec<Mask> = imgs.iter().map(Mask::new).collect();
            let views: Vec<(&Mask, Iso)> = masks.iter().enumerate().map(|(q, m)| (m, super::iso(q as i32))).collect();
            let f = fit_proxy(&views, s, 100.0, 140.0, Some(150.0)).unwrap();
            let _ = iso;
            assert!(f.iou > 0.9, "{truth:?} -> {f:?}");
            // where the roof stands out of the outline its height comes back within a tenth; a lower one cannot be seen,
            // and is raised again by `raise_roof` up to the outline
            if truth.base > 0.0 {
                // a flat-topped building on a platform: smaller and taller looks alike from above; the platform shows
                assert!((f.base - truth.base).abs() < truth.base * 0.2 && f.tier < 0.8, "{truth:?} -> {f:?}");
            } else if truth.kind != 3 && truth.kind != 4 {
                assert!(((f.wall + f.roof) - (truth.wall + truth.roof)).abs() < (truth.wall + truth.roof) * 0.12, "{truth:?} -> {f:?}");
            } else {
                let r = raise_roof(f, &views, s, 100.0, 140.0);
                assert!(r.kind != 0 && r.roof > 20.0 && r.iou > f.iou - 0.006, "{f:?} -> {r:?}");
            }
        }
    }

    #[test]
    fn every_view_is_fitted_at_once() {
        // a long building seen from all four quarter turns: one solid fits all four pictures
        let truth = Fit { hx: 140.0, hz: 70.0, wall: 50.0, roof: 40.0, kind: 1, ..Fit::default() };
        let s = SPRITE_UNITS_PER_PIXEL;
        let imgs: Vec<Rgba> = (0..4).map(|q| render(&truth, &iso(q), s, 220, 200, 110.0, 140.0)).collect();
        let masks: Vec<Mask> = imgs.iter().map(Mask::new).collect();
        let views: Vec<(&Mask, Iso)> = masks.iter().enumerate().map(|(q, m)| (m, iso(q as i32))).collect();
        let f = fit_proxy(&views, s, 110.0, 140.0, Some(160.0)).unwrap();
        assert!(f.iou > 0.9, "{f:?}");
        // the long side stays the long side (one view alone cannot tell X from Z at 45 degrees)
        assert!(f.hx > f.hz * 1.4, "{f:?}");
    }

    #[test]
    fn a_thin_outline_fits_no_box() {
        // a pole and a beam: an oil pump's kind of outline
        let mut img = Rgba::new(120, 160);
        for y in 20..150 {
            for x in 55..62 {
                img.px[(y * 120 + x) * 4 + 3] = 255;
            }
        }
        for x in 10..110 {
            for y in 30..36 {
                img.px[(y * 120 + x) * 4 + 3] = 255;
            }
        }
        let m = Mask::new(&img);
        let f = fit_proxy(&[(&m, iso(0))], SPRITE_UNITS_PER_PIXEL, 60.0, 150.0, None);
        assert!(f.is_none_or(|f| f.iou < MIN_FIT), "{f:?}");
    }

    #[test]
    fn the_projection_and_its_inverse_on_the_ground_agree() {
        for q in 0..4 {
            let iso = iso(q);
            let (px, py) = iso.to_px([37.0, 0.0, -81.0], 2.0, 50.0, 70.0);
            let (x, z) = iso.ground_of(px, py, 2.0, 50.0, 70.0);
            assert!((x - 37.0).abs() < 1e-3 && (z + 81.0).abs() < 1e-3);
            // up is up the picture
            let (_, hy) = iso.to_px([0.0, 10.0, 0.0], 2.0, 50.0, 70.0);
            assert!(hy < 70.0);
        }
    }

    #[test]
    fn each_face_takes_a_quarter_that_sees_it_nearest_the_eye() {
        // the camera at quarter 0 looks along heading 45 (towards +x, -z): it sees the south (+z) and west (-x) walls
        let f0 = iso(0).f;
        assert!(f0[0] > 0.0 && f0[2] < 0.0 && f0[1] < 0.0);
        for yaw in [0.0f32, 45.0, 100.0, 200.0, 300.0] {
            for n in [[0.0, 0.0, 1.0], [-1.0, 0.0, 0.0], [0.0, 0.0, -1.0], [1.0, 0.0, 0.0], [0.0, 1.0, 0.0], [0.3, 0.9, 0.0]] {
                let q = face_quarter(n, yaw);
                assert!(-dot(n, iso(q).f) > 0.0, "{n:?} {yaw} {q}");
            }
        }
        // seen from an overhead camera's own heading, every face that camera sees takes that camera's picture
        for q in 0..4 {
            let yaw = 45.0 + 90.0 * q as f32;
            for n in [[0.0, 1.0, 0.0], [0.0, 0.0, 1.0], [-1.0, 0.0, 0.0], [0.0, 0.0, -1.0], [1.0, 0.0, 0.0]] {
                if -dot(n, iso(q).f) > 0.02 {
                    assert_eq!(face_quarter(n, yaw), q, "{n:?} quarter {q}");
                }
            }
        }
    }

    #[test]
    fn people_face_the_way_they_head() {
        // the overhead camera's own headings give its own views (App::prop_view)
        for q in 0..4 {
            for heading in [0.0f32, 45.0, 180.0, 270.0] {
                let overhead = ((((180.0 - heading) / 45.0).round() as i32 + 2 * q) % 8 + 8) % 8;
                assert_eq!(person_view(heading, 45.0 + 90.0 * q as f32, false), overhead);
            }
        }
        // turning the eye round a person by 45 degrees moves one view
        assert_eq!((person_view(90.0, 100.0, false) - person_view(90.0, 55.0, false)).rem_euclid(8), 1);
        // a mirrored person takes the view whose mirror image faces the same way
        assert_eq!(person_view(180.0, 45.0, true), 2);
    }

    #[test]
    fn culling_keeps_what_is_in_view() {
        let proj = Mat4::perspective(60.0, 1.6, 1.0, 10000.0);
        let view = Mat4::look_at([0.0, 7.0, 0.0], [0.0, 7.0, -10.0], [0.0, 1.0, 0.0]);
        let planes = frustum(&proj.mul(&view));
        assert!(sphere_visible(&planes, [0.0, 0.0, -500.0], 5.0));
        // behind the eye, far off to the side, past the far plane
        assert!(!sphere_visible(&planes, [0.0, 0.0, 500.0], 5.0));
        assert!(!sphere_visible(&planes, [3000.0, 0.0, -500.0], 5.0));
        assert!(!sphere_visible(&planes, [0.0, 0.0, -20000.0], 5.0));
        // a big thing reaching into the view is kept
        assert!(sphere_visible(&planes, [700.0, 0.0, -500.0], 300.0));
        assert!(box_visible(&planes, [-50.0, -5.0, -600.0], [50.0, 5.0, -400.0]));
        assert!(!box_visible(&planes, [-50.0, -5.0, 400.0], [50.0, 5.0, 600.0]));
    }

    #[test]
    fn detail_falls_off_with_distance() {
        let far = 2000.0 * M;
        assert_eq!(detail_at(10.0 * M, far), Detail::Full);
        assert_eq!(detail_at(200.0 * M, far), Detail::Full);
        assert_eq!(detail_at(800.0 * M, far), Detail::Far);
        assert_eq!(detail_at(2500.0 * M, far), Detail::None);
    }
}
