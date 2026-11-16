//! Modelled buildings for the walk camera.
//!
//! A building's sprite is one picture from above. Stretched over a solid fitted to its outline (walk_objects.rs
//! `fit_proxy`), a shaped building such as the clubhouse, with its octagonal halls, awnings and garden, wears its roofs and
//! shrubs smeared down its walls. A building with a model here is drawn as that model instead: plain coloured geometry
//! of the project's own design, lit like the other solids (walk_gl.rs `mode::FLAT`). A building without one keeps its
//! fitted solid or its card.
//!
//! Models are built in metres about the footprint's centre, facing 0 (their front towards +z), and turned by the
//! object's facing.

use crate::gfx::Vert;
use std::f32::consts::{FRAC_PI_8, TAU};

const M: f32 = crate::walk::METRE;

/// A colour, opaque.
type Rgb = [f32; 3];

/// A part's colours: the theme's take on one building.
#[derive(Clone, Copy)]
struct Palette {
    wall: Rgb,
    trim: Rgb,
    roof: Rgb,
    deck: Rgb,
    rail: Rgb,
    glass: Rgb,
    frame: Rgb,
    door: Rgb,
    awning: Rgb,
    gold: Rgb,
    leaf: Rgb,
    leaf_dark: Rgb,
    trunk: Rgb,
    flower: Rgb,
    lamp: Rgb,
    stone: Rgb,
    /// How the theme builds: 0 clapboard (Parkland), 1 stone (Links), 2 adobe under flat roofs (Desert), 3 light timber
    /// under steep, deep roofs (Tropical).
    style: u8,
}

/// The clubhouse's colours by theme (app::THEMES: 0 Parkland, 1 Links, 2 Desert, 3 Tropical).
fn clubhouse_palette(theme: usize) -> Palette {
    let parkland = Palette {
        wall: [0.90, 0.85, 0.70],
        trim: [0.55, 0.16, 0.13],
        roof: [0.30, 0.26, 0.25],
        deck: [0.52, 0.40, 0.29],
        rail: [0.58, 0.17, 0.14],
        glass: [0.20, 0.26, 0.32],
        frame: [0.96, 0.95, 0.90],
        door: [0.42, 0.13, 0.10],
        awning: [0.78, 0.14, 0.12],
        gold: [0.86, 0.70, 0.30],
        leaf: [0.22, 0.42, 0.18],
        leaf_dark: [0.12, 0.27, 0.12],
        trunk: [0.36, 0.25, 0.16],
        flower: [0.82, 0.48, 0.78],
        lamp: [0.30, 0.66, 0.72],
        stone: [0.62, 0.58, 0.52],
        style: 0,
    };
    match theme {
        1 => Palette {
            wall: [0.92, 0.91, 0.87],
            roof: [0.25, 0.30, 0.34],
            trim: [0.20, 0.30, 0.38],
            rail: [0.92, 0.92, 0.90],
            awning: [0.18, 0.32, 0.52],
            style: 1,
            ..parkland
        },
        2 => Palette {
            wall: [0.86, 0.72, 0.55],
            roof: [0.66, 0.34, 0.22],
            trim: [0.48, 0.30, 0.20],
            deck: [0.70, 0.58, 0.44],
            rail: [0.48, 0.30, 0.20],
            awning: [0.20, 0.50, 0.50],
            leaf: [0.40, 0.48, 0.26],
            style: 2,
            ..parkland
        },
        3 => Palette {
            wall: [0.95, 0.93, 0.86],
            roof: [0.62, 0.52, 0.34],
            trim: [0.18, 0.46, 0.44],
            rail: [0.95, 0.95, 0.92],
            awning: [0.18, 0.55, 0.52],
            leaf: [0.20, 0.50, 0.22],
            style: 3,
            ..parkland
        },
        _ => parkland,
    }
}

/// Triangles in metres, built part by part.
#[derive(Default)]
struct Build {
    v: Vec<Vert>,
}

fn sub(a: [f32; 3], b: [f32; 3]) -> [f32; 3] {
    [a[0] - b[0], a[1] - b[1], a[2] - b[2]]
}

fn cross(a: [f32; 3], b: [f32; 3]) -> [f32; 3] {
    [a[1] * b[2] - a[2] * b[1], a[2] * b[0] - a[0] * b[2], a[0] * b[1] - a[1] * b[0]]
}

fn norm(v: [f32; 3]) -> [f32; 3] {
    let l = (v[0] * v[0] + v[1] * v[1] + v[2] * v[2]).sqrt().max(1e-6);
    [v[0] / l, v[1] / l, v[2] / l]
}

/// The corners of a regular polygon of `n` sides about (cx, cz), radius `r` to its corners, its first corner at `turn`.
fn ring(cx: f32, cz: f32, r: f32, n: usize, turn: f32) -> Vec<(f32, f32)> {
    (0..n)
        .map(|i| {
            let a = turn + TAU * i as f32 / n as f32;
            (cx + r * a.cos(), cz + r * a.sin())
        })
        .collect()
}

/// An octagon with its sides square to the axes.
const OCT: f32 = FRAC_PI_8;

impl Build {
    /// The lines across a wall from `y0` to `y1` that show how it is built: clapboards' shadow lines, stone courses, or
    /// none for adobe. `sides` are the wall's sides' lengths.
    fn courses(&mut self, c: &[(f32, f32)], sides: &[f32], y0: f32, y1: f32, pal: &Palette) {
        let (step, h, dark) = match pal.style {
            1 => (0.6, 0.05, 0.8),
            2 => return,
            _ => (0.45, 0.06, 0.86),
        };
        let mut y = y0 + step;
        let mut row = 0;
        while y < y1 - 0.4 {
            for (i, &len) in sides.iter().enumerate() {
                self.panel(c, i, 0.5, len, y - h, y, 0.015, scale(pal.wall, dark));
                if pal.style == 1 {
                    // the stones' joints, staggered course by course
                    let n = (len / 1.1).floor().max(1.0) as usize;
                    for k in 0..n {
                        let along = (k as f32 + if row % 2 == 0 { 0.25 } else { 0.75 }) / n as f32;
                        self.panel(c, i, along, 0.05, y, (y + step - h).min(y1), 0.015, scale(pal.wall, dark));
                    }
                }
            }
            y += step;
            row += 1;
        }
    }

    /// A flat polygon, its normal turned away from `inside`. `shade` darkens its corners below `y` (a wall's foot).
    fn poly(&mut self, p: &[[f32; 3]], inside: [f32; 3], col: Rgb) {
        if p.len() < 3 {
            return;
        }
        let mut n = norm(cross(sub(p[1], p[0]), sub(p[2], p[0])));
        let c = p.iter().fold([0.0; 3], |a, q| [a[0] + q[0], a[1] + q[1], a[2] + q[2]]).map(|s| s / p.len() as f32);
        let out = sub(c, inside);
        if n[0] * out[0] + n[1] * out[1] + n[2] * out[2] < 0.0 {
            n = n.map(|q| -q);
        }
        let vert = |q: [f32; 3]| {
            let mut v = Vert::new(q[0], q[1], q[2], 0.0, 0.0).col([col[0], col[1], col[2], 1.0]);
            v.normal = n;
            v
        };
        for k in 1..p.len() - 1 {
            self.v.extend([vert(p[0]), vert(p[k]), vert(p[k + 1])]);
        }
    }

    /// Upright walls round a polygon, from `y0` up to `y1`.
    fn walls(&mut self, corners: &[(f32, f32)], y0: f32, y1: f32, col: Rgb) {
        let (cx, cz) = centre(corners);
        for i in 0..corners.len() {
            let (a, b) = (corners[i], corners[(i + 1) % corners.len()]);
            self.poly(&[[a.0, y0, a.1], [b.0, y0, b.1], [b.0, y1, b.1], [a.0, y1, a.1]], [cx, (y0 + y1) * 0.5, cz], col);
        }
    }

    /// A polygon's flat top (or floor) at `y`.
    fn cap(&mut self, corners: &[(f32, f32)], y: f32, up: bool, col: Rgb) {
        let (cx, cz) = centre(corners);
        let p: Vec<[f32; 3]> = corners.iter().map(|c| [c.0, y, c.1]).collect();
        self.poly(&p, [cx, if up { y - 1.0 } else { y + 1.0 }, cz], col);
    }

    /// A prism: walls and a top.
    fn prism(&mut self, corners: &[(f32, f32)], y0: f32, y1: f32, col: Rgb) {
        self.walls(corners, y0, y1, col);
        self.cap(corners, y1, true, col);
    }

    /// A box from `lo` to `hi`.
    fn cuboid(&mut self, lo: [f32; 3], hi: [f32; 3], col: Rgb) {
        let c = [(lo[0], lo[2]), (hi[0], lo[2]), (hi[0], hi[2]), (lo[0], hi[2])];
        self.prism(&c, lo[1], hi[1], col);
    }

    /// A roof sloping from a polygon at `y0` up to the same polygon shrunk by `top` (0: a point) at `y1`.
    fn roof(&mut self, corners: &[(f32, f32)], y0: f32, y1: f32, top: f32, col: Rgb) {
        let (cx, cz) = centre(corners);
        let inner: Vec<(f32, f32)> = corners.iter().map(|&(x, z)| (cx + (x - cx) * top, cz + (z - cz) * top)).collect();
        let inside = [cx, y0 - 2.0, cz];
        // in rows, a little lighter and darker by turns, for its courses of shingles
        let rows = (((y1 - y0) / 0.9).round() as usize).clamp(1, 12);
        let at = |p: (f32, f32), t: f32| {
            let k = 1.0 - t * (1.0 - top.max(0.0));
            [cx + (p.0 - cx) * k, y0 + (y1 - y0) * t, cz + (p.1 - cz) * k]
        };
        for i in 0..corners.len() {
            let j = (i + 1) % corners.len();
            let (a, b) = (corners[i], corners[j]);
            for r in 0..rows {
                let (t0, t1) = (r as f32 / rows as f32, (r + 1) as f32 / rows as f32);
                let c = scale(col, if r % 2 == 0 { 1.0 } else { 0.9 });
                if top <= 0.0 && r == rows - 1 {
                    self.poly(&[at(a, t0), at(b, t0), [cx, y1, cz]], inside, c);
                } else {
                    self.poly(&[at(a, t0), at(b, t0), at(b, t1), at(a, t1)], inside, c);
                }
            }
            // the eaves' underside
            self.poly(&[[a.0, y0, a.1], [b.0, y0, b.1], [cx, y0, cz]], [cx, y0 + 1.0, cz], scale(col, 0.6));
        }
        if top > 0.0 {
            let inner: Vec<(f32, f32)> = inner.to_vec();
            self.cap(&inner, y1, true, col);
        }
    }

    /// A ball, of eight by six faces.
    fn ball(&mut self, c: [f32; 3], r: f32, col: Rgb) {
        let (nu, nv) = (10, 7);
        let at = |i: usize, j: usize| {
            let (u, v) = (TAU * i as f32 / nu as f32, std::f32::consts::PI * j as f32 / nv as f32);
            [c[0] + r * v.sin() * u.cos(), c[1] + r * v.cos(), c[2] + r * v.sin() * u.sin()]
        };
        for i in 0..nu {
            for j in 0..nv {
                let (a, b, cc, d) = (at(i, j), at(i + 1, j), at(i + 1, j + 1), at(i, j + 1));
                if j == 0 {
                    self.poly(&[a, cc, d], c, col);
                } else if j == nv - 1 {
                    self.poly(&[a, b, cc], c, col);
                } else {
                    self.poly(&[a, b, cc, d], c, col);
                }
            }
        }
    }

    /// A rounded rod from `a` to `b`, made of overlapping balls of radius `r` (thinner towards its ends by `taper`).
    fn rod(&mut self, a: [f32; 3], b: [f32; 3], r: f32, taper: f32, col: Rgb) {
        let len = sub(b, a).iter().map(|v| v * v).sum::<f32>().sqrt();
        let n = ((len / (r * 0.7)).ceil() as usize).max(2);
        for k in 0..=n {
            let t = k as f32 / n as f32;
            let end = (2.0 * t - 1.0).abs();
            let rr = r * (1.0 - taper * end * end);
            self.ball([a[0] + (b[0] - a[0]) * t, a[1] + (b[1] - a[1]) * t, a[2] + (b[2] - a[2]) * t], rr, col);
        }
    }

    /// A giant die, `s` across, its lowest corner at `lo`, its pips showing on the top, front and right.
    fn die(&mut self, lo: [f32; 3], s: f32) {
        self.cuboid(lo, [lo[0] + s, lo[1] + s, lo[2] + s], [0.96, 0.95, 0.92]);
        let pip = [0.10, 0.10, 0.12];
        let (r, q) = (s * 0.09, s * 0.27);
        let c = [lo[0] + s * 0.5, lo[1] + s * 0.5, lo[2] + s * 0.5];
        // five on top, three on the front, two on the right
        for (dx, dz) in [(0.0, 0.0), (-q, -q), (q, q), (-q, q), (q, -q)] {
            self.ball([c[0] + dx, lo[1] + s, c[2] + dz], r, pip);
        }
        for k in -1..=1 {
            self.ball([c[0] + k as f32 * q, c[1] - k as f32 * q, lo[2] + s], r, pip);
        }
        for k in [-1.0, 1.0] {
            self.ball([lo[0] + s, c[1] + k * q, c[2] + k * q], r, pip);
        }
    }

    /// Panels on a polygon's walls: on side `i` (from corner i to i + 1), a rectangle `w` wide and from `y0` to `y1`, at
    /// `along` (0..1) across the side, standing `out` off the wall.
    #[allow(clippy::too_many_arguments)]
    fn panel(&mut self, corners: &[(f32, f32)], i: usize, along: f32, w: f32, y0: f32, y1: f32, out: f32, col: Rgb) {
        let (cx, cz) = centre(corners);
        let (a, b) = (corners[i], corners[(i + 1) % corners.len()]);
        let (dx, dz) = (b.0 - a.0, b.1 - a.1);
        let len = (dx * dx + dz * dz).sqrt().max(1e-6);
        let (tx, tz) = (dx / len, dz / len);
        let (mut nx, mut nz) = (tz, -tx);
        let (mx, mz) = (a.0 + dx * along, a.1 + dz * along);
        if (mx - cx) * nx + (mz - cz) * nz < 0.0 {
            nx = -nx;
            nz = -nz;
        }
        let (px, pz) = (mx + nx * out, mz + nz * out);
        let h = w * 0.5;
        let p = [
            [px - tx * h, y0, pz - tz * h],
            [px + tx * h, y0, pz + tz * h],
            [px + tx * h, y1, pz + tz * h],
            [px - tx * h, y1, pz - tz * h],
        ];
        self.poly(&p, [cx, (y0 + y1) * 0.5, cz], col);
    }

    /// A window: a frame and its glass.
    #[allow(clippy::too_many_arguments)]
    fn window(&mut self, corners: &[(f32, f32)], i: usize, along: f32, w: f32, y0: f32, y1: f32, pal: &Palette) {
        self.panel(corners, i, along, w + 0.3, y0 - 0.15, y1 + 0.15, 0.04, pal.frame);
        self.panel(corners, i, along, w, y0, y1, 0.07, pal.glass);
        // its mullion
        self.panel(corners, i, along, 0.1, y0, y1, 0.09, pal.frame);
    }

    /// A striped awning over side `i` of a polygon: from the wall at `y` out `reach` and down `drop`.
    #[allow(clippy::too_many_arguments)]
    fn awning(&mut self, corners: &[(f32, f32)], i: usize, w: f32, y: f32, reach: f32, drop: f32, pal: &Palette) {
        let (cx, cz) = centre(corners);
        let (a, b) = (corners[i], corners[(i + 1) % corners.len()]);
        let (dx, dz) = (b.0 - a.0, b.1 - a.1);
        let len = (dx * dx + dz * dz).sqrt().max(1e-6);
        let (tx, tz) = (dx / len, dz / len);
        let (mut nx, mut nz) = (tz, -tx);
        let (mx, mz) = ((a.0 + b.0) * 0.5, (a.1 + b.1) * 0.5);
        if (mx - cx) * nx + (mz - cz) * nz < 0.0 {
            nx = -nx;
            nz = -nz;
        }
        let stripes = 7;
        for k in 0..stripes {
            let (s0, s1) = (-w * 0.5 + w * k as f32 / stripes as f32, -w * 0.5 + w * (k + 1) as f32 / stripes as f32);
            let col = if k % 2 == 0 { pal.awning } else { pal.frame };
            let wall = |s: f32| [mx + tx * s + nx * 0.05, y, mz + tz * s + nz * 0.05];
            let edge = |s: f32, d: f32| [mx + tx * s + nx * reach, y - drop - d, mz + tz * s + nz * reach];
            self.poly(&[wall(s0), wall(s1), edge(s1, 0.0), edge(s0, 0.0)], [cx, y - 3.0, cz], col);
            // its valance
            self.poly(&[edge(s0, 0.0), edge(s1, 0.0), edge(s1, 0.35), edge(s0, 0.35)], [cx, y - drop, cz], col);
        }
    }

    /// A railing round a polygon's edge at deck height `y`, its posts at most `step` apart; `gap` leaves side `gap` open
    /// (for steps).
    fn railing(&mut self, corners: &[(f32, f32)], y: f32, step: f32, gap: Option<usize>, col: Rgb) {
        let (cx, cz) = centre(corners);
        let h = 1.05;
        for i in 0..corners.len() {
            let (a, b) = (corners[i], corners[(i + 1) % corners.len()]);
            let len = ((b.0 - a.0).powi(2) + (b.1 - a.1).powi(2)).sqrt();
            let n = (len / step).ceil().max(1.0) as usize;
            for k in 0..=n {
                let t = k as f32 / n as f32;
                let (x, z) = (a.0 + (b.0 - a.0) * t, a.1 + (b.1 - a.1) * t);
                self.cuboid([x - 0.09, y, z - 0.09], [x + 0.09, y + h + 0.1, z + 0.09], col);
            }
            if gap == Some(i) {
                continue;
            }
            // top rail and bottom rail, as thin slabs
            for (y0, y1) in [(y + h - 0.08, y + h + 0.04), (y + 0.15, y + 0.25)] {
                let inset = |p: (f32, f32), d: f32| (p.0 + (cx - p.0) * d, p.1 + (cz - p.1) * d);
                let (a2, b2) = (inset(a, 0.0), inset(b, 0.0));
                self.poly(&[[a2.0, y0, a2.1], [b2.0, y0, b2.1], [b2.0, y1, b2.1], [a2.0, y1, a2.1]], [cx, y0, cz], col);
                self.poly(
                    &[
                        [a2.0, y1, a2.1],
                        [b2.0, y1, b2.1],
                        [b2.0 + (cx - b2.0) * 0.006, y1, b2.1 + (cz - b2.1) * 0.006],
                        [a2.0 + (cx - a2.0) * 0.006, y1, a2.1 + (cz - a2.1) * 0.006],
                    ],
                    [cx, y0, cz],
                    col,
                );
            }
            let m = (len / 0.45).floor() as usize;
            for k in 1..m {
                let t = k as f32 / m as f32;
                let (x, z) = (a.0 + (b.0 - a.0) * t, a.1 + (b.1 - a.1) * t);
                self.cuboid([x - 0.03, y + 0.2, z - 0.03], [x + 0.03, y + h - 0.05, z + 0.03], scale(col, 0.95));
            }
        }
    }

    /// A cypress: a trunk under a tall, narrow cone of leaves.
    fn cypress(&mut self, x: f32, z: f32, h: f32, pal: &Palette) {
        self.cuboid([x - 0.15, 0.0, z - 0.15], [x + 0.15, 0.8, z + 0.15], pal.trunk);
        let base = ring(x, z, h * 0.17, 8, OCT);
        self.roof(&base, 0.6, h * 0.45, 0.0, pal.leaf_dark);
        // (a second, narrower cone over the first gives the cypress its taper)
        let upper = ring(x, z, h * 0.15, 8, 0.0);
        self.walls(&base, 0.6, h * 0.3, pal.leaf_dark);
        self.roof(&upper, h * 0.3, h, 0.0, pal.leaf_dark);
    }

    /// A shrub: a low leafy ball.
    fn shrub(&mut self, x: f32, z: f32, r: f32, pal: &Palette, dark: bool) {
        let c = if dark { pal.leaf_dark } else { pal.leaf };
        self.ball([x, r * 0.7, z], r, c);
        for k in 0..3 {
            let a = TAU * (k as f32 / 3.0 + hash(x, z));
            self.ball([x + a.cos() * r * 0.6, r * 0.55, z + a.sin() * r * 0.6], r * 0.65, scale(c, 0.9 + 0.1 * k as f32));
        }
    }

    /// A lamp: a post with a coloured head.
    fn lamp(&mut self, x: f32, z: f32, pal: &Palette) {
        let iron = [0.16, 0.17, 0.17];
        self.cuboid([x - 0.12, 0.0, z - 0.12], [x + 0.12, 0.4, z + 0.12], iron);
        self.cuboid([x - 0.05, 0.4, z - 0.05], [x + 0.05, 2.7, z + 0.05], iron);
        self.ball([x, 2.95, z], 0.22, pal.lamp);
        self.roof(&ring(x, z, 0.3, 6, 0.0), 3.12, 3.35, 0.0, iron);
    }

    /// A flower bed: a low mound and its flowers.
    fn bed(&mut self, x: f32, z: f32, w: f32, d: f32, pal: &Palette) {
        self.cuboid([x - w * 0.5, 0.0, z - d * 0.5], [x + w * 0.5, 0.25, z + d * 0.5], pal.leaf);
        let n = ((w * d) / 0.8).ceil() as usize;
        for k in 0..n {
            let h = hash(k as f32 * 1.7 + x, z + k as f32 * 0.3);
            let h2 = hash(z - k as f32 * 2.1, x * 0.7 + k as f32);
            let (fx, fz) = (x + (h - 0.5) * w * 0.9, z + (h2 - 0.5) * d * 0.9);
            self.ball([fx, 0.4, fz], 0.22, if k % 3 == 0 { pal.frame } else { pal.flower });
        }
    }

    /// An octagonal hall: walls with a band of trim, a window on each side (two storeys when tall), and a roof.
    #[allow(clippy::too_many_arguments)]
    fn hall(
        &mut self,
        cx: f32,
        cz: f32,
        r: f32,
        floor: f32,
        wall: f32,
        roof: f32,
        finial: bool,
        pal: &Palette,
        door: Option<usize>,
    ) -> Vec<(f32, f32)> {
        let c = ring(cx, cz, r, 8, OCT);
        self.walls(&c, 0.0, floor + wall, pal.wall);
        // its clapboards or courses
        let side_len = 2.0 * r * OCT.sin();
        self.courses(&c, &[side_len; 8], floor, floor + wall, pal);
        let storeys = if wall > 6.5 { 2 } else { 1 };
        let side = 2.0 * r * OCT.sin();
        let sh = wall / storeys as f32;
        for i in 0..8 {
            for s in 0..storeys {
                let y0 = floor + sh * s as f32 + sh * 0.3;
                let y1 = floor + sh * s as f32 + sh * 0.8;
                if door == Some(i) && s == 0 {
                    // double doors and their frame
                    self.panel(&c, i, 0.5, 2.2, floor, floor + 2.8, 0.04, pal.frame);
                    self.panel(&c, i, 0.5, 1.9, floor, floor + 2.6, 0.07, pal.door);
                    self.panel(&c, i, 0.5, 0.08, floor, floor + 2.6, 0.09, pal.frame);
                } else if side > 4.0 {
                    let ww = (side * 0.22).min(1.4);
                    self.window(&c, i, 0.3, ww, y0, y1, pal);
                    self.window(&c, i, 0.7, ww, y0, y1, pal);
                } else {
                    self.window(&c, i, 0.5, (side * 0.4).min(1.4), y0, y1, pal);
                }
            }
            if storeys == 2 {
                // the band between the storeys
                self.panel(&c, i, 0.5, side, floor + sh - 0.15, floor + sh + 0.15, 0.06, pal.trim);
            }
        }
        let eave = ring(cx, cz, r * 1.12, 8, OCT);
        let top = floor + wall;
        if finial {
            self.roof(&eave, top, top + roof, 0.0, pal.roof);
            self.cuboid([cx - 0.08, top + roof - 0.1, cz - 0.08], [cx + 0.08, top + roof + 0.9, cz + 0.08], pal.gold);
            self.ball([cx, top + roof + 1.0, cz], 0.32, pal.gold);
        }
        c
    }
}

fn centre(c: &[(f32, f32)]) -> (f32, f32) {
    let n = c.len().max(1) as f32;
    let (x, z) = c.iter().fold((0.0, 0.0), |a, p| (a.0 + p.0, a.1 + p.1));
    (x / n, z / n)
}

fn scale(c: Rgb, k: f32) -> Rgb {
    c.map(|q| q * k)
}

fn hash(x: f32, z: f32) -> f32 {
    ((x * 12.9898 + z * 78.233).sin() * 43758.547).fract().abs()
}

/// The clubhouse: a two storey octagonal hall under a hipped roof and a lantern, three octagonal pavilions with striped
/// awnings at its corners, all on a railed veranda deck, with cypresses, shrubs, flower beds and lamps round it. In
/// metres, front to +z, on a footprint 4 tiles (91 m) across.
fn clubhouse(theme: usize) -> Build {
    let pal = clubhouse_palette(theme);
    let mut b = Build::default();
    let floor = 1.4;
    // the veranda: an octagon stretched to the building, raised on a stone skirt
    let deck: Vec<(f32, f32)> =
        vec![(-24.0, -30.0), (24.0, -30.0), (31.0, -23.0), (31.0, 22.0), (24.0, 29.0), (-24.0, 29.0), (-31.0, 22.0), (-31.0, -23.0)];
    b.walls(&deck, -0.6, floor, pal.stone);
    b.cap(&deck, floor, true, pal.deck);
    // its boards
    let mut x = -30.5;
    while x < 31.0 {
        b.poly(
            &[[x, floor + 0.01, -29.5], [x + 0.06, floor + 0.01, -29.5], [x + 0.06, floor + 0.01, 28.5], [x, floor + 0.01, 28.5]],
            [x, 0.0, 0.0],
            scale(pal.deck, 0.8),
        );
        x += 0.6;
    }
    b.railing(&deck, floor, 2.4, Some(4), pal.rail);
    // steps down from the front, between two lamps
    for k in 0..4 {
        let y = floor - (k as f32 + 1.0) * floor / 4.0;
        let z = 29.0 + k as f32 * 0.45;
        b.cuboid([-5.0, -0.5, z], [5.0, y + floor / 4.0, z + 0.45], pal.stone);
    }
    b.lamp(-6.5, 31.5, &pal);
    b.lamp(6.5, 31.5, &pal);
    // the hall: two storeys under a roof that rises to a lantern
    let r = 13.0;
    b.hall(0.0, -2.0, r, floor, 10.0, 0.0, false, &pal, Some(2));
    // a porch round the hall's ground floor: a sloping roof on posts
    let porch_in = ring(0.0, -2.0, r * 1.03, 8, OCT);
    let porch_out = ring(0.0, -2.0, r + 3.6, 8, OCT);
    let (py, pdrop) = (floor + 4.6, 0.8);
    for i in 0..8 {
        let j = (i + 1) % 8;
        let (a, bb, c, d) = (porch_in[i], porch_in[j], porch_out[j], porch_out[i]);
        b.poly(&[[a.0, py, a.1], [bb.0, py, bb.1], [c.0, py - pdrop, c.1], [d.0, py - pdrop, d.1]], [0.0, 0.0, -2.0], pal.roof);
        // its fascia
        b.poly(
            &[[d.0, py - pdrop, d.1], [c.0, py - pdrop, c.1], [c.0, py - pdrop - 0.35, c.1], [d.0, py - pdrop - 0.35, d.1]],
            [0.0, py, -2.0],
            pal.trim,
        );
        b.poly(
            &[[a.0, py - 0.05, a.1], [bb.0, py - 0.05, bb.1], [c.0, py - pdrop - 0.05, c.1], [d.0, py - pdrop - 0.05, d.1]],
            [0.0, py + 5.0, -2.0],
            scale(pal.wall, 0.8),
        );
        for t in [0.0, 0.5] {
            let (x, z) = (d.0 + (c.0 - d.0) * t, d.1 + (c.1 - d.1) * t);
            let (x, z) = (x * 0.97, -2.0 + (z + 2.0) * 0.97);
            b.cuboid([x - 0.14, floor, z - 0.14], [x + 0.14, py - pdrop, z + 0.14], pal.rail);
        }
    }
    let top = floor + 10.0;
    let eave = ring(0.0, -2.0, r * 1.12, 8, OCT);
    b.roof(&eave, top, top + 6.5, 0.32, pal.roof);
    // the lantern on top, its roof and finial
    let lantern = ring(0.0, -2.0, 3.6, 8, OCT);
    let ly = top + 6.5;
    b.walls(&lantern, ly, ly + 3.2, pal.wall);
    for i in 0..8 {
        b.window(&lantern, i, 0.5, 1.2, ly + 0.6, ly + 2.6, &pal);
    }
    b.walls(&ring(0.0, -2.0, 3.7, 8, OCT), ly + 2.9, ly + 3.2, pal.trim);
    b.roof(&ring(0.0, -2.0, 4.4, 8, OCT), ly + 3.2, ly + 7.0, 0.0, pal.roof);
    b.cuboid([-0.08, ly + 6.8, -2.08], [0.08, ly + 8.0, -1.92], pal.gold);
    b.ball([0.0, ly + 8.1, -2.0], 0.4, pal.gold);
    // the pavilions at three corners, each with awnings over the windows facing out
    for &(px, pz, out) in &[(-19.0, 15.0, [0usize, 1, 2]), (19.0, 15.0, [6, 7, 0]), (-19.0, -17.0, [3, 4, 5])] {
        // (sides are counted from +x round towards +z: 0 faces +x, 2 faces +z, 4 faces -x, 6 faces -z)
        let c = b.hall(px, pz, 7.0, floor, 6.0, 5.5, true, &pal, None);
        for &i in &out {
            let side = 2.0 * 7.0 * OCT.sin();
            b.awning(&c, i, side * 0.8, floor + 4.7, 1.3, 0.8, &pal);
        }
    }
    // the garden round the deck
    let cypresses = [
        (-34.0, 26.0),
        (34.0, 26.0),
        (-35.0, -10.0),
        (35.0, -14.0),
        (-26.0, -34.0),
        (12.0, -35.0),
        (-12.0, 35.0),
        (14.0, 35.5),
        (36.0, 5.0),
    ];
    for (k, &(x, z)) in cypresses.iter().enumerate() {
        b.cypress(x, z, 6.5 + hash(x, z) * 2.5 + (k % 2) as f32, &pal);
    }
    for k in 0..26 {
        let a = TAU * k as f32 / 26.0;
        let (x, z) = (34.0 * a.cos(), 33.0 * a.sin());
        if z > 28.0 && x.abs() < 9.0 {
            continue;
        }
        b.shrub(x, z, 0.9 + hash(x, z) * 0.7, &pal, k % 3 == 0);
    }
    b.bed(-14.0, 33.0, 8.0, 2.4, &pal);
    b.bed(16.0, 33.0, 8.0, 2.4, &pal);
    b.bed(-34.5, 6.0, 2.4, 9.0, &pal);
    b.lamp(-33.0, -24.0, &pal);
    b.lamp(33.0, -26.0, &pal);
    b
}

/// A rectangular building's roof.
#[derive(Clone, Copy, PartialEq)]
enum Roof {
    /// Sloping on all four sides to a ridge along the longer side.
    Hip,
    /// Sloping on the long sides to a ridge, with gable walls at its ends.
    Gable,
    /// Flat, behind a low parapet.
    Flat,
}

/// A rectangle's corners: north-west, north-east, south-east, south-west (sides 0 north, 1 east, 2 south, 3 west).
fn rect(x0: f32, z0: f32, x1: f32, z1: f32) -> Vec<(f32, f32)> {
    vec![(x0, z0), (x1, z0), (x1, z1), (x0, z1)]
}

impl Build {
    /// A sloping face from its foot (a to b) to its head (c to d), in courses shaded by turns.
    fn slope(&mut self, a: [f32; 3], b: [f32; 3], c: [f32; 3], d: [f32; 3], inside: [f32; 3], col: Rgb) {
        let rows = (((c[1] - a[1]).abs() / 0.9).round() as usize).clamp(1, 12);
        let lerp = |p: [f32; 3], q: [f32; 3], t: f32| [p[0] + (q[0] - p[0]) * t, p[1] + (q[1] - p[1]) * t, p[2] + (q[2] - p[2]) * t];
        for r in 0..rows {
            let (t0, t1) = (r as f32 / rows as f32, (r + 1) as f32 / rows as f32);
            let shade = scale(col, if r % 2 == 0 { 1.0 } else { 0.9 });
            let (p0, p1, p2, p3) = (lerp(a, d, t0), lerp(b, c, t0), lerp(b, c, t1), lerp(a, d, t1));
            if (p2[0] - p3[0]).abs() + (p2[2] - p3[2]).abs() < 1e-4 {
                self.poly(&[p0, p1, p2], inside, shade);
            } else {
                self.poly(&[p0, p1, p2, p3], inside, shade);
            }
        }
    }

    /// A rectangle's roof from `y` up `h`, overhanging by `o`.
    #[allow(clippy::too_many_arguments)]
    fn rect_roof(&mut self, x0: f32, z0: f32, x1: f32, z1: f32, y: f32, h: f32, o: f32, kind: Roof, pal: &Palette) {
        let (x0, z0, x1, z1) = (x0 - o, z0 - o, x1 + o, z1 + o);
        let (cx, cz) = ((x0 + x1) * 0.5, (z0 + z1) * 0.5);
        let inside = [cx, y - 3.0, cz];
        let col = pal.roof;
        if kind == Roof::Flat {
            self.cap(&rect(x0, z0, x1, z1), y + 0.3, true, scale(col, 0.85));
            let w = 0.25;
            for (lo, hi) in [
                ([x0, y, z0], [x1, y + 0.9, z0 + w]),
                ([x0, y, z1 - w], [x1, y + 0.9, z1]),
                ([x0, y, z0], [x0 + w, y + 0.9, z1]),
                ([x1 - w, y, z0], [x1, y + 0.9, z1]),
            ] {
                self.cuboid(lo, hi, pal.trim);
            }
            return;
        }
        // the eaves' underside
        self.cap(&rect(x0, z0, x1, z1), y, false, scale(pal.wall, 0.7));
        let along_x = x1 - x0 >= z1 - z0;
        let top = y + h;
        if along_x {
            let half = (z1 - z0) * 0.5;
            let (r0, r1) = if kind == Roof::Hip { (x0 + half, x1 - half) } else { (x0, x1) };
            let (r0, r1) = if r0 > r1 { (cx, cx) } else { (r0, r1) };
            self.slope([x0, y, z1], [x1, y, z1], [r1, top, cz], [r0, top, cz], inside, col);
            self.slope([x1, y, z0], [x0, y, z0], [r0, top, cz], [r1, top, cz], inside, col);
            if kind == Roof::Hip {
                self.slope([x1, y, z1], [x1, y, z0], [r1, top, cz], [r1, top, cz], inside, col);
                self.slope([x0, y, z0], [x0, y, z1], [r0, top, cz], [r0, top, cz], inside, col);
            } else {
                self.poly(&[[x1 - o, y, z1 - o], [x1 - o, y, z0 + o], [x1 - o, top, cz]], inside, pal.wall);
                self.poly(&[[x0 + o, y, z0 + o], [x0 + o, y, z1 - o], [x0 + o, top, cz]], inside, pal.wall);
            }
        } else {
            let half = (x1 - x0) * 0.5;
            let (r0, r1) = if kind == Roof::Hip { (z0 + half, z1 - half) } else { (z0, z1) };
            let (r0, r1) = if r0 > r1 { (cz, cz) } else { (r0, r1) };
            self.slope([x1, y, z1], [x1, y, z0], [cx, top, r0], [cx, top, r1], inside, col);
            self.slope([x0, y, z0], [x0, y, z1], [cx, top, r1], [cx, top, r0], inside, col);
            if kind == Roof::Hip {
                self.slope([x0, y, z1], [x1, y, z1], [cx, top, r1], [cx, top, r1], inside, col);
                self.slope([x1, y, z0], [x0, y, z0], [cx, top, r0], [cx, top, r0], inside, col);
            } else {
                self.poly(&[[x0 + o, y, z1 - o], [x1 - o, y, z1 - o], [cx, top, z1 - o]], inside, pal.wall);
                self.poly(&[[x1 - o, y, z0 + o], [x0 + o, y, z0 + o], [cx, top, z0 + o]], inside, pal.wall);
            }
        }
    }

    /// A rectangular building: `storeys` of `sh` metres on a low foundation, clapboarded, a window every 3.5 m or so on
    /// each side, a door in the middle of its front (south, +z) unless `door` is false, and its roof.
    #[allow(clippy::too_many_arguments)]
    fn block(&mut self, x0: f32, z0: f32, x1: f32, z1: f32, storeys: usize, sh: f32, roof: Roof, rh: f32, pal: &Palette, door: bool) {
        let c = rect(x0, z0, x1, z1);
        let floor = 0.5;
        let wall = storeys as f32 * sh;
        self.walls(&rect(x0 - 0.1, z0 - 0.1, x1 + 0.1, z1 + 0.1), -0.5, floor, pal.stone);
        self.walls(&c, floor - 0.05, floor + wall, pal.wall);
        let lens = [x1 - x0, z1 - z0, x1 - x0, z1 - z0];
        self.courses(&c, &lens, floor, floor + wall, pal);
        // corner boards and the band under the eaves
        for &(x, z) in &c {
            self.cuboid([x - 0.2, floor, z - 0.2], [x + 0.2, floor + wall, z + 0.2], pal.frame);
        }
        self.walls(&rect(x0 - 0.06, z0 - 0.06, x1 + 0.06, z1 + 0.06), floor + wall - 0.45, floor + wall, pal.trim);
        for (i, &len) in lens.iter().enumerate() {
            let n = ((len / 3.6).floor() as usize).max(1);
            for s in 0..storeys {
                let (wy0, wy1) = (floor + sh * s as f32 + sh * 0.3, floor + sh * s as f32 + sh * 0.8);
                for k in 0..n {
                    let along = (k as f32 + 0.5) / n as f32;
                    if door && i == 2 && s == 0 && (along - 0.5).abs() * len < 1.8 {
                        continue;
                    }
                    self.window(&c, i, along, 1.2_f32.min(len / n as f32 * 0.45), wy0, wy1, pal);
                }
            }
        }
        if door {
            self.panel(&c, 2, 0.5, 2.3, floor, floor + 2.7, 0.04, pal.frame);
            self.panel(&c, 2, 0.5, 2.0, floor, floor + 2.5, 0.07, pal.door);
            self.panel(&c, 2, 0.5, 0.08, floor, floor + 2.5, 0.09, pal.frame);
            self.cuboid([(x0 + x1) * 0.5 - 1.8, -0.3, z1], [(x0 + x1) * 0.5 + 1.8, floor, z1 + 1.2], pal.stone);
        }
        match pal.style {
            2 => self.rect_roof(x0, z0, x1, z1, floor + wall, rh, 0.2, Roof::Flat, pal),
            3 => self.rect_roof(x0, z0, x1, z1, floor + wall, rh * 1.35, 1.3, roof, pal),
            _ => self.rect_roof(x0, z0, x1, z1, floor + wall, rh, 0.6, roof, pal),
        }
    }

    /// A table under a striped umbrella, with chairs.
    fn parasol(&mut self, x: f32, z: f32, pal: &Palette) {
        self.prism(&ring(x, z, 0.5, 8, OCT), 0.0, 0.75, pal.frame);
        for k in 0..4 {
            let a = TAU * (k as f32 + 0.5) / 4.0;
            let (cx, cz) = (x + a.cos() * 0.95, z + a.sin() * 0.95);
            self.cuboid([cx - 0.22, 0.0, cz - 0.22], [cx + 0.22, 0.45, cz + 0.22], pal.trim);
        }
        self.cuboid([x - 0.04, 0.0, z - 0.04], [x + 0.04, 2.3, z + 0.04], pal.frame);
        let r = ring(x, z, 1.4, 8, 0.0);
        for i in 0..8 {
            let (a, b) = (r[i], r[(i + 1) % 8]);
            let col = if i % 2 == 0 { pal.awning } else { pal.frame };
            self.poly(&[[a.0, 2.0, a.1], [b.0, 2.0, b.1], [x, 2.6, z]], [x, 1.0, z], col);
            self.poly(&[[a.0, 2.0, a.1], [b.0, 2.0, b.1], [x, 2.6, z]], [x, 3.5, z], scale(col, 0.7));
        }
    }

    /// A golf cart: a white body, a roof on four posts and dark wheels.
    fn cart(&mut self, x: f32, z: f32, along_x: bool, pal: &Palette) {
        let (hx, hz) = if along_x { (1.2, 0.6) } else { (0.6, 1.2) };
        let body = [0.92, 0.92, 0.90];
        self.cuboid([x - hx, 0.3, z - hz], [x + hx, 0.95, z + hz], body);
        for (sx, sz) in [(-1.0, -1.0), (1.0, -1.0), (1.0, 1.0), (-1.0, 1.0)] {
            let (px, pz) = (x + sx * (hx - 0.1), z + sz * (hz - 0.1));
            self.cuboid([px - 0.04, 0.95, pz - 0.04], [px + 0.04, 1.9, pz + 0.04], [0.2, 0.2, 0.2]);
            self.cuboid([px - 0.18, 0.0, pz - 0.18], [px + 0.18, 0.36, pz + 0.18], [0.1, 0.1, 0.1]);
        }
        self.cuboid([x - hx - 0.1, 1.9, z - hz - 0.1], [x + hx + 0.1, 2.0, z + hz + 0.1], pal.awning);
    }

    /// A broad leafy tree: a trunk under a cluster of balls.
    fn tree(&mut self, x: f32, z: f32, h: f32, pal: &Palette) {
        self.cuboid([x - 0.2, 0.0, z - 0.2], [x + 0.2, h * 0.45, z + 0.2], pal.trunk);
        let r = h * 0.3;
        self.ball([x, h - r, z], r, pal.leaf);
        for k in 0..4 {
            let a = TAU * (k as f32 / 4.0 + hash(x, z));
            self.ball([x + a.cos() * r * 0.7, h - r * 1.35, z + a.sin() * r * 0.7], r * 0.75, scale(pal.leaf, 0.85 + 0.05 * k as f32));
        }
    }

    /// Flat ground cover: a rectangle just above the ground.
    fn pad(&mut self, x0: f32, z0: f32, x1: f32, z1: f32, y: f32, col: Rgb) {
        self.cap(&rect(x0, z0, x1, z1), y, true, col);
    }
}

/// A palette for a building of `wall` colour under a `roof` colour, in a theme.
fn palette(theme: usize, wall: Rgb, roof: Rgb) -> Palette {
    let mix = |a: Rgb, b: Rgb, t: f32| [a[0] + (b[0] - a[0]) * t, a[1] + (b[1] - a[1]) * t, a[2] + (b[2] - a[2]) * t];
    let (wall, roof) = match theme {
        // grey stone under slate
        1 => (mix(wall, [0.64, 0.62, 0.58], 0.75), [0.24, 0.26, 0.30]),
        // sand coloured adobe; its flat roofs show only their parapets
        2 => (mix(wall, [0.86, 0.71, 0.53], 0.75), [0.70, 0.36, 0.24]),
        // pale timber under thatch
        3 => (mix(wall, [0.96, 0.94, 0.86], 0.5), [0.64, 0.54, 0.34]),
        _ => (wall, roof),
    };
    Palette { wall, roof, ..clubhouse_palette(theme) }
}

/// The snack bar: a small cottage with a serving hatch under an awning and tables under umbrellas. 2 tiles (46 m).
fn snack_bar(theme: usize) -> Build {
    let pal = palette(theme, [0.86, 0.80, 0.66], [0.36, 0.40, 0.44]);
    let mut b = Build::default();
    b.pad(-10.0, -8.0, 10.0, 11.0, 0.03, pal.stone);
    b.block(-4.5, -5.0, 4.5, 2.0, 1, 3.4, Roof::Hip, 2.8, &pal, false);
    let c = rect(-4.5, -5.0, 4.5, 2.0);
    b.panel(&c, 2, 0.5, 4.5, 1.4, 2.8, 0.07, [0.12, 0.10, 0.08]);
    b.cuboid([-2.5, 0.0, 2.0], [2.5, 1.3, 2.7], pal.trim);
    b.awning(&c, 2, 6.0, 3.4, 1.8, 0.7, &pal);
    // the gag: a giant hot dog on the roof, on two struts, with a squiggle of mustard
    for x in [-2.0, 2.0] {
        b.cuboid([x - 0.12, 5.6, -1.6], [x + 0.12, 7.0, -1.4], [0.3, 0.3, 0.32]);
    }
    let (y, z) = (7.6, -1.5);
    b.rod([-3.6, y - 0.1, z - 0.55], [3.6, y - 0.1, z - 0.55], 0.62, 0.35, [0.88, 0.66, 0.36]);
    b.rod([-3.6, y - 0.1, z + 0.55], [3.6, y - 0.1, z + 0.55], 0.62, 0.35, [0.88, 0.66, 0.36]);
    b.rod([-4.3, y + 0.25, z], [4.3, y + 0.25, z], 0.55, 0.3, [0.72, 0.24, 0.16]);
    for k in 0..12 {
        let x = -3.3 + k as f32 * 0.6;
        b.ball([x, y + 0.78, z + if k % 2 == 0 { -0.22 } else { 0.22 }], 0.16, [0.98, 0.82, 0.12]);
    }
    for &(x, z) in &[(-6.0, 6.0), (0.0, 7.5), (6.0, 6.0), (-7.0, -1.0), (7.0, -1.0)] {
        b.parasol(x, z, &pal);
    }
    b.shrub(-9.0, -7.0, 1.0, &pal, false);
    b.shrub(9.0, -7.0, 1.0, &pal, true);
    b.tree(8.5, -12.0, 7.0, &pal);
    b
}

/// The pro shop: a single storey shop with a deep porch and shop windows, a sign and planting. 2 tiles (46 m).
fn pro_shop(theme: usize) -> Build {
    let pal = palette(theme, [0.80, 0.74, 0.62], [0.30, 0.34, 0.40]);
    let mut b = Build::default();
    b.pad(-12.0, -9.0, 12.0, 12.0, 0.03, pal.stone);
    b.block(-9.0, -7.0, 9.0, 4.0, 1, 4.0, Roof::Hip, 3.6, &pal, true);
    // the porch roof on posts across the front
    let (py, z1) = (4.2, 7.0);
    b.slope([-9.0, py - 0.6, z1], [9.0, py - 0.6, z1], [9.0, py, 4.0], [-9.0, py, 4.0], [0.0, 0.0, 0.0], pal.roof);
    b.cuboid([-9.0, py - 1.0, z1 - 0.1], [9.0, py - 0.6, z1 + 0.1], pal.trim);
    for k in 0..5 {
        let x = -8.8 + k as f32 * 4.4;
        b.cuboid([x - 0.15, 0.5, z1 - 0.3], [x + 0.15, py - 0.8, z1], pal.frame);
    }
    b.cuboid([-9.0, 0.0, 4.0], [9.0, 0.5, z1], pal.deck);
    // a sign on two posts
    b.cuboid([-2.5, 0.0, 10.0], [-2.3, 2.6, 10.2], pal.trunk);
    b.cuboid([2.3, 0.0, 10.0], [2.5, 2.6, 10.2], pal.trunk);
    b.cuboid([-2.8, 1.5, 9.95], [2.8, 2.6, 10.25], pal.trim);
    for &(x, z) in &[(-11.0, 9.0), (11.0, 9.0), (-11.0, -8.0), (11.0, -8.0)] {
        b.shrub(x, z, 1.2, &pal, x < 0.0);
    }
    b.bed(-6.0, 9.0, 5.0, 1.6, &pal);
    b.bed(6.0, 9.0, 5.0, 1.6, &pal);
    // the gag: a giant golf ball on a giant tee, standing on the roof's ridge, its dimples a ring of dints
    let tee = [0.95, 0.85, 0.2];
    b.prism(&ring(0.0, -1.5, 0.35, 10, 0.0), 6.8, 8.6, tee);
    b.prism(&ring(0.0, -1.5, 0.9, 12, 0.0), 8.6, 8.9, tee);
    let (cy, r) = (10.6, 1.8);
    b.ball([0.0, cy, -1.5], r, [0.97, 0.97, 0.95]);
    for k in 0..14 {
        let a = TAU * k as f32 / 14.0;
        for dy in [-0.7f32, 0.0, 0.7] {
            let rr = (r * r - dy * dy).sqrt() * 0.97;
            b.ball([a.cos() * rr, cy + dy, -1.5 + a.sin() * rr], 0.16, [0.82, 0.83, 0.82]);
        }
    }
    b.tree(-13.0, -12.0, 8.0, &pal);
    b
}

/// The cart garage: a long, low shed with roller doors and carts parked before it. 2 tiles (46 m).
fn cart_garage(theme: usize) -> Build {
    let pal = palette(theme, [0.76, 0.74, 0.70], [0.32, 0.36, 0.38]);
    let mut b = Build::default();
    b.pad(-15.0, -8.0, 15.0, 12.0, 0.03, [0.40, 0.40, 0.40]);
    b.block(-13.0, -6.0, 13.0, 3.0, 1, 4.2, Roof::Hip, 3.0, &pal, false);
    let c = rect(-13.0, -6.0, 13.0, 3.0);
    for k in 0..4 {
        let along = (k as f32 + 0.5) / 4.0;
        b.panel(&c, 2, along, 5.0, 0.5, 3.8, 0.05, pal.frame);
        b.panel(&c, 2, along, 4.6, 0.5, 3.6, 0.08, [0.58, 0.58, 0.56]);
        for r in 0..8 {
            let y = 0.7 + r as f32 * 0.38;
            b.panel(&c, 2, along, 4.6, y, y + 0.05, 0.1, [0.45, 0.45, 0.43]);
        }
    }
    for k in 0..4 {
        b.cart(-9.0 + k as f32 * 6.0, 7.0, false, &pal);
    }
    b.shrub(-14.5, -7.0, 1.1, &pal, false);
    b.shrub(14.5, -7.0, 1.1, &pal, true);
    b
}

/// The swim club: a pool in a paved surround, a changing house, loungers and umbrellas behind a low fence. 3 tiles (69 m).
fn swim_club(theme: usize) -> Build {
    let pal = palette(theme, [0.92, 0.90, 0.84], [0.62, 0.30, 0.22]);
    let mut b = Build::default();
    let paving = [0.82, 0.78, 0.70];
    b.pad(-24.0, -20.0, 24.0, 20.0, 0.05, paving);
    // the pool, its water a little below its coping
    b.pad(-14.0, -6.0, 14.0, 8.0, 0.08, [0.14, 0.52, 0.72]);
    b.walls(&rect(-14.4, -6.4, 14.4, 8.4), 0.0, 0.25, [0.95, 0.95, 0.93]);
    b.walls(&rect(-14.0, -6.0, 14.0, 8.0), 0.0, 0.25, [0.95, 0.95, 0.93]);
    for k in 1..6 {
        let z = -6.0 + k as f32 * 14.0 / 6.0;
        b.pad(-13.5, z - 0.08, 13.5, z + 0.08, 0.1, [0.80, 0.86, 0.92]);
    }
    // a diving board
    b.cuboid([-15.5, 0.0, 0.5], [-14.5, 0.9, 1.5], paving);
    b.cuboid([-14.5, 0.8, 0.7], [-11.5, 0.9, 1.3], pal.frame);
    b.block(-10.0, -18.0, 10.0, -11.0, 1, 3.4, Roof::Gable, 2.6, &pal, true);
    for k in 0..6 {
        let x = -12.5 + k as f32 * 5.0;
        b.cuboid([x - 0.4, 0.05, 11.0], [x + 0.4, 0.45, 13.0], pal.frame);
        b.cuboid([x - 0.4, 0.45, 12.4], [x + 0.4, 0.95, 13.0], pal.frame);
        if k % 2 == 0 {
            b.parasol(x + 2.5, 15.5, &pal);
        }
    }
    // a white fence round it
    let fence = rect(-23.5, -19.5, 23.5, 19.5);
    b.railing(&fence, 0.0, 2.5, Some(2), pal.frame);
    for &(x, z) in &[(-26.0, -22.0), (26.0, -22.0), (-26.0, 22.0), (26.0, 22.0)] {
        b.tree(x, z, 8.0, &pal);
    }
    b
}

/// The tennis court: a green hard court with its lines and net inside a tall wire fence, benches and a little pavilion. 3
/// tiles (69 m).
fn tennis_court(theme: usize) -> Build {
    let pal = palette(theme, [0.88, 0.84, 0.72], [0.32, 0.36, 0.30]);
    let mut b = Build::default();
    b.pad(-20.0, -26.0, 20.0, 26.0, 0.04, [0.70, 0.44, 0.32]);
    b.pad(-6.0, -12.0, 6.0, 12.0, 0.06, [0.22, 0.46, 0.34]);
    let white = [0.95, 0.95, 0.93];
    let line = |b: &mut Build, x0: f32, z0: f32, x1: f32, z1: f32| b.pad(x0, z0, x1, z1, 0.08, white);
    let (hw, sw, hl, sl) = (5.49, 4.11, 11.89, 6.40);
    for x in [-hw, -sw, sw, hw] {
        line(&mut b, x - 0.04, -hl, x + 0.04, hl);
    }
    for z in [-hl, hl] {
        line(&mut b, -hw, z - 0.04, hw, z + 0.04);
    }
    for z in [-sl, sl] {
        line(&mut b, -sw, z - 0.04, sw, z + 0.04);
    }
    line(&mut b, -0.04, -sl, 0.04, sl);
    // the net
    for x in [-6.4, 6.4] {
        b.cuboid([x - 0.05, 0.0, -0.05], [x + 0.05, 1.07, 0.05], [0.25, 0.25, 0.25]);
    }
    b.poly(&[[-6.4, 0.0, 0.0], [6.4, 0.0, 0.0], [6.4, 0.95, 0.0], [-6.4, 0.95, 0.0]], [0.0, 0.5, 1.0], [0.15, 0.15, 0.15]);
    b.poly(&[[-6.4, 0.0, 0.0], [6.4, 0.0, 0.0], [6.4, 0.95, 0.0], [-6.4, 0.95, 0.0]], [0.0, 0.5, -1.0], [0.15, 0.15, 0.15]);
    b.cuboid([-6.4, 0.9, -0.03], [6.4, 1.0, 0.03], white);
    // the fence: posts and a dark mesh, thin enough to see the court through only from above, so kept low at the sides
    let fence = rect(-9.0, -18.0, 9.0, 18.0);
    let mesh = [0.18, 0.28, 0.22];
    for i in 0..4 {
        let (a, c) = (fence[i], fence[(i + 1) % 4]);
        let len = ((c.0 - a.0).powi(2) + (c.1 - a.1).powi(2)).sqrt();
        let n = (len / 3.0).ceil() as usize;
        for k in 0..=n {
            let t = k as f32 / n as f32;
            let (x, z) = (a.0 + (c.0 - a.0) * t, a.1 + (c.1 - a.1) * t);
            b.cuboid([x - 0.06, 0.0, z - 0.06], [x + 0.06, 3.0, z + 0.06], mesh);
        }
        // its wire: a top rail and thin strands, so the court shows through
        let mut y = 0.25;
        while y < 3.0 {
            let t = if y > 2.8 { 0.08 } else { 0.025 };
            b.poly(&[[a.0, y - t, a.1], [c.0, y - t, c.1], [c.0, y + t, c.1], [a.0, y + t, a.1]], [0.0, 1.0, 0.0], mesh);
            b.poly(&[[a.0, y - t, a.1], [c.0, y - t, c.1], [c.0, y + t, c.1], [a.0, y + t, a.1]], [a.0 * 3.0, 1.0, a.1 * 3.0], mesh);
            y += if y > 2.6 { 1.0 } else { 0.45 };
        }
        y = 2.95;
        b.poly(&[[a.0, y - 0.08, a.1], [c.0, y - 0.08, c.1], [c.0, y + 0.06, c.1], [a.0, y + 0.06, a.1]], [0.0, 1.0, 0.0], mesh);
    }
    for z in [-6.0, 6.0] {
        b.cuboid([-8.5, 0.0, z - 1.0], [-8.0, 0.45, z + 1.0], pal.deck);
    }
    b.block(12.0, -6.0, 18.0, 2.0, 1, 3.0, Roof::Hip, 2.2, &pal, false);
    for &(x, z) in &[(-14.0, -22.0), (14.0, 20.0), (-14.0, 22.0), (15.0, -20.0)] {
        b.tree(x, z, 8.5, &pal);
    }
    b
}

/// The resort hotel: a five storey block with an entrance canopy, a turning circle and trees. 4 tiles (91 m).
fn resort_hotel(theme: usize) -> Build {
    let pal = palette(theme, [0.84, 0.80, 0.70], [0.32, 0.36, 0.42]);
    let mut b = Build::default();
    b.pad(-30.0, -16.0, 30.0, 26.0, 0.03, pal.stone);
    b.block(-26.0, -12.0, 26.0, 6.0, 5, 3.3, Roof::Hip, 5.0, &pal, true);
    // two wings stepping forward at the ends
    for x in [-26.0, 16.0] {
        b.block(x, 6.0, x + 10.0, 12.0, 4, 3.3, Roof::Hip, 4.0, &pal, false);
    }
    // the entrance canopy
    b.cuboid([-5.0, 3.6, 6.0], [5.0, 4.0, 11.0], pal.trim);
    for x in [-4.6, 4.6] {
        b.cuboid([x - 0.15, 0.0, 10.5], [x + 0.15, 3.6, 10.8], pal.frame);
    }
    // a turning circle with a fountain
    b.prism(&ring(0.0, 19.0, 4.5, 16, 0.0), 0.0, 0.5, pal.stone);
    b.pad(-3.8, 15.2, 3.8, 22.8, 0.0, pal.stone);
    b.prism(&ring(0.0, 19.0, 3.8, 16, 0.0), 0.0, 0.55, [0.20, 0.50, 0.70]);
    b.prism(&ring(0.0, 19.0, 0.4, 8, 0.0), 0.0, 2.2, pal.frame);
    b.ball([0.0, 2.4, 19.0], 0.6, [0.80, 0.90, 0.95]);
    for &(x, z) in &[(-30.0, 14.0), (30.0, 14.0), (-12.0, 22.0), (12.0, 22.0), (-30.0, -14.0), (30.0, -14.0)] {
        b.tree(x, z, 10.0 + hash(x, z) * 3.0, &pal);
    }
    for k in 0..10 {
        b.shrub(-24.0 + k as f32 * 5.3, 13.5, 0.9, &pal, k % 2 == 0);
    }
    b
}

/// The marina: a boathouse on piles, a jetty and a moored boat. 2 tiles (46 m); the water lies to the front.
fn marina(theme: usize) -> Build {
    let pal = palette(theme, [0.70, 0.56, 0.42], [0.30, 0.32, 0.34]);
    let mut b = Build::default();
    b.block(-8.0, -12.0, 4.0, -3.0, 1, 3.6, Roof::Gable, 3.4, &pal, true);
    // the jetty: boards on posts
    b.cuboid([-2.0, 0.3, -3.0], [2.0, 0.6, 18.0], pal.deck);
    for k in 0..8 {
        let z = -2.0 + k as f32 * 2.9;
        for x in [-2.1, 2.1] {
            b.cuboid([x - 0.15, -1.5, z - 0.15], [x + 0.15, 1.2, z + 0.15], pal.trunk);
        }
    }
    // a boat beside it
    let hull = [0.95, 0.95, 0.93];
    let (bx, bz) = (5.0, 10.0);
    b.cuboid([bx - 1.2, -0.3, bz - 3.0], [bx + 1.2, 0.6, bz + 2.5], hull);
    b.poly(&[[bx - 1.2, 0.6, bz + 2.5], [bx + 1.2, 0.6, bz + 2.5], [bx, 0.6, bz + 4.2]], [bx, -1.0, bz], hull);
    b.poly(&[[bx - 1.2, 0.6, bz + 2.5], [bx, 0.6, bz + 4.2], [bx, -0.3, bz + 3.6]], [bx + 1.0, 0.0, bz], scale(hull, 0.9));
    b.poly(&[[bx + 1.2, 0.6, bz + 2.5], [bx, 0.6, bz + 4.2], [bx, -0.3, bz + 3.6]], [bx - 1.0, 0.0, bz], scale(hull, 0.9));
    b.cuboid([bx - 0.8, 0.6, bz - 1.5], [bx + 0.8, 1.6, bz + 0.5], pal.trim);
    b.cuboid([bx - 0.05, 1.6, bz - 0.6], [bx + 0.05, 7.5, bz - 0.4], pal.frame);
    b.shrub(-10.0, -13.0, 1.1, &pal, false);
    b.tree(8.0, -14.0, 7.5, &pal);
    b
}

/// The airstrip: a paved runway with its markings, an arched hangar, a parked plane and a windsock. 6 tiles (137 m).
fn airstrip(theme: usize) -> Build {
    let pal = palette(theme, [0.70, 0.72, 0.74], [0.70, 0.72, 0.74]);
    let mut b = Build::default();
    let tar = [0.30, 0.30, 0.31];
    b.pad(-62.0, 8.0, 62.0, 24.0, 0.04, tar);
    let mut x = -55.0;
    while x < 55.0 {
        b.pad(x, 15.7, x + 5.0, 16.3, 0.06, [0.92, 0.92, 0.88]);
        x += 9.0;
    }
    for e in [-60.0, 56.0] {
        for k in 0..6 {
            let z = 10.0 + k as f32 * 2.4;
            b.pad(e, z, e + 4.0, z + 1.2, 0.06, [0.92, 0.92, 0.88]);
        }
    }
    // an apron and the hangar: an arched roof over end walls
    b.pad(-30.0, -24.0, 0.0, 8.0, 0.04, [0.45, 0.45, 0.45]);
    let (hx0, hx1, hz0, hz1, r) = (-28.0, -2.0, -22.0, -6.0, 8.0);
    let cz = (hz0 + hz1) * 0.5;
    let n = 10;
    let arc = |k: usize| {
        let a = std::f32::consts::PI * k as f32 / n as f32;
        (cz - r * a.cos(), r * a.sin())
    };
    for k in 0..n {
        let ((z0, y0), (z1, y1)) = (arc(k), arc(k + 1));
        let col = scale(pal.roof, if k % 2 == 0 { 1.0 } else { 0.93 });
        b.poly(&[[hx0, y0, z0], [hx1, y0, z0], [hx1, y1, z1], [hx0, y1, z1]], [(hx0 + hx1) * 0.5, 0.0, cz], col);
        for xe in [hx0, hx1] {
            b.poly(&[[xe, 0.0, cz], [xe, y0, z0], [xe, y1, z1]], [(hx0 + hx1) * 0.5, 2.0, cz], scale(pal.wall, 0.9));
        }
    }
    b.poly(
        &[[hx1 + 0.05, 0.0, cz - 5.0], [hx1 + 0.05, 0.0, cz + 5.0], [hx1 + 0.05, 5.5, cz + 5.0], [hx1 + 0.05, 5.5, cz - 5.0]],
        [hx0, 2.0, cz],
        [0.25, 0.25, 0.27],
    );
    // a little plane on the apron
    let (px, pz) = (-12.0, 2.0);
    let red = pal.awning;
    let white = pal.frame;
    b.cuboid([px - 3.5, 0.8, pz - 0.5], [px + 3.0, 1.8, pz + 0.5], white);
    b.cuboid([px + 1.0, 1.5, pz - 0.45], [px + 2.2, 2.2, pz + 0.45], [0.30, 0.40, 0.50]);
    b.cuboid([px + 0.0, 1.8, pz - 4.5], [px + 1.6, 1.95, pz + 4.5], red);
    b.cuboid([px - 3.4, 1.5, pz - 1.6], [px - 2.6, 1.6, pz + 1.6], red);
    b.cuboid([px - 3.4, 1.6, pz - 0.06], [px - 2.6, 3.0, pz + 0.06], red);
    b.cuboid([px + 3.0, 1.0, pz - 0.9], [px + 3.15, 1.6, pz + 0.9], [0.15, 0.15, 0.15]);
    for (wx, wz) in [(1.0, -1.2), (1.0, 1.2), (-2.8, 0.0)] {
        b.cuboid([px + wx - 0.25, 0.0, pz + wz - 0.12], [px + wx + 0.25, 0.5, pz + wz + 0.12], [0.1, 0.1, 0.1]);
        b.cuboid([px + wx - 0.04, 0.4, pz + wz - 0.04], [px + wx + 0.04, 0.9, pz + wz + 0.04], [0.5, 0.5, 0.5]);
    }
    // the windsock
    b.cuboid([20.0, 0.0, 2.0], [20.15, 6.0, 2.15], white);
    b.roof(&ring(21.6, 2.1, 0.45, 8, 0.0), 5.4, 5.6, 0.0, [0.95, 0.5, 0.1]);
    b.cuboid([20.1, 5.2, 1.7], [23.0, 5.9, 2.5], [0.95, 0.5, 0.1]);
    b.tree(-40.0, -20.0, 9.0, &pal);
    b.tree(30.0, -15.0, 9.0, &pal);
    b
}

/// A house on a home site: two storeys under a gabled roof, with a chimney, a porch and a garden. Its colours vary
/// by `variant` (0..4). 2 tiles (46 m).
fn house(theme: usize, variant: usize) -> Build {
    let walls = [[0.88, 0.84, 0.72], [0.74, 0.80, 0.84], [0.86, 0.72, 0.62], [0.80, 0.84, 0.70]];
    let roofs = [[0.32, 0.34, 0.38], [0.56, 0.26, 0.20], [0.30, 0.30, 0.32], [0.42, 0.30, 0.24]];
    let pal = palette(theme, walls[variant % 4], roofs[variant % 4]);
    let mut b = Build::default();
    let (w, d) = if variant.is_multiple_of(2) { (6.5, 5.0) } else { (5.5, 5.5) };
    b.block(-w, -d, w, d, 2, 3.0, Roof::Gable, 4.0, &pal, true);
    b.cuboid([w * 0.5, 4.0, -1.0], [w * 0.5 + 1.2, 12.6, 0.2], [0.55, 0.30, 0.24]);
    // the porch
    b.cuboid([-3.0, 0.0, d], [3.0, 0.5, d + 2.5], pal.deck);
    b.slope([-3.2, 3.0, d + 2.6], [3.2, 3.0, d + 2.6], [3.2, 3.5, d], [-3.2, 3.5, d], [0.0, 0.0, 0.0], pal.roof);
    for x in [-2.9, 2.9] {
        b.cuboid([x - 0.12, 0.5, d + 2.2], [x + 0.12, 3.0, d + 2.45], pal.frame);
    }
    // a path, a fence along the front and some planting
    b.pad(-1.0, d + 2.5, 1.0, 18.0, 0.04, pal.stone);
    let fence = rect(-14.0, -12.0, 14.0, 17.0);
    b.railing(&fence, 0.0, 2.2, Some(2), pal.frame);
    b.tree(-10.0, -8.0, 8.0 + variant as f32, &pal);
    b.shrub(-5.0, d + 1.5, 0.9, &pal, false);
    b.shrub(5.0, d + 1.5, 0.9, &pal, true);
    b.bed(-6.0, 12.0, 6.0, 1.5, &pal);
    b
}

/// The driving range's hitting shed: a long, open fronted bay under a lean-to roof, its mats, and target greens with
/// flags out in front. 5 tiles (114 m).
fn driving_range(theme: usize) -> Build {
    let pal = palette(theme, [0.80, 0.76, 0.66], [0.30, 0.36, 0.30]);
    let mut b = Build::default();
    let (x0, x1, z0, z1) = (-24.0, 24.0, -50.0, -42.0);
    b.cuboid([x0, 0.0, z0], [x1, 0.4, z1], pal.stone);
    b.cuboid([x0, 0.4, z0], [x1, 4.6, z0 + 0.4], pal.wall);
    for x in [x0, x1] {
        b.cuboid([x - 0.2, 0.4, z0], [x + 0.2, 4.0, z1], pal.wall);
    }
    b.slope(
        [x0 - 0.5, 3.6, z1 + 0.8],
        [x1 + 0.5, 3.6, z1 + 0.8],
        [x1 + 0.5, 4.9, z0 - 0.5],
        [x0 - 0.5, 4.9, z0 - 0.5],
        [0.0, 0.0, z0 + 3.0],
        pal.roof,
    );
    let bays = 10;
    for k in 0..=bays {
        let x = x0 + (x1 - x0) * k as f32 / bays as f32;
        b.cuboid([x - 0.15, 0.4, z1 - 0.3], [x + 0.15, 3.7, z1], pal.trim);
        if k < bays {
            let m = x + (x1 - x0) / bays as f32 * 0.5;
            b.cuboid([m - 0.8, 0.4, z1 - 2.5], [m + 0.8, 0.45, z1 - 0.8], [0.20, 0.45, 0.22]);
        }
    }
    // targets: a green disc and a flag each
    for (k, &(x, z, r)) in
        [(-14.0, -20.0, 4.0), (10.0, -5.0, 5.0), (-6.0, 15.0, 6.0), (16.0, 32.0, 7.0), (-18.0, 42.0, 6.0)].iter().enumerate()
    {
        b.prism(&ring(x, z, r, 16, 0.0), 0.0, 0.08, [0.36, 0.62, 0.30]);
        b.cuboid([x - 0.05, 0.0, z - 0.05], [x + 0.05, 2.6, z + 0.05], pal.frame);
        let flag = if k % 2 == 0 { pal.awning } else { [0.95, 0.85, 0.20] };
        b.poly(&[[x, 2.6, z], [x + 1.0, 2.35, z], [x, 2.1, z]], [x, 2.35, z - 1.0], flag);
        b.poly(&[[x, 2.6, z], [x + 1.0, 2.35, z], [x, 2.1, z]], [x, 2.35, z + 1.0], flag);
    }
    b
}

/// The TV tower: a tapering lattice mast in red and white bands with a light at its top. 1 tile.
fn tv_tower(theme: usize) -> Build {
    let pal = clubhouse_palette(theme);
    let mut b = Build::default();
    let h = 34.0;
    let half = |y: f32| 2.2 - 1.8 * (y / h);
    let legs = [(-1.0, -1.0), (1.0, -1.0), (1.0, 1.0), (-1.0, 1.0)];
    let sections = 12;
    for s in 0..sections {
        let (y0, y1) = (h * s as f32 / sections as f32, h * (s + 1) as f32 / sections as f32);
        let col = if s % 2 == 0 { [0.85, 0.18, 0.14] } else { [0.95, 0.95, 0.93] };
        for i in 0..4 {
            let (a, c) = (legs[i], legs[(i + 1) % 4]);
            let p = |l: (f32, f32), y: f32| [l.0 * half(y), y, l.1 * half(y)];
            let t = 0.09;
            // legs
            b.cuboid([a.0 * half(y0) - t, y0, a.1 * half(y0) - t], [a.0 * half(y1) + t, y1, a.1 * half(y1) + t], col);
            // a brace across each face
            let (p0, p1) = (p(a, y0), p(c, y1));
            b.poly(&[p0, [p0[0], p0[1] + 0.12, p0[2]], [p1[0], p1[1] + 0.12, p1[2]], p1], [0.0, (y0 + y1) * 0.5, 0.0], col);
            b.poly(&[p0, [p0[0], p0[1] + 0.12, p0[2]], [p1[0], p1[1] + 0.12, p1[2]], p1], [0.0, -(y0 + y1) * 0.5, 0.0], col);
        }
    }
    b.cuboid([-0.5, h, -0.5], [0.5, h + 0.6, 0.5], [0.85, 0.18, 0.14]);
    b.ball([0.0, h + 1.0, 0.0], 0.4, [1.0, 0.3, 0.2]);
    b.cuboid([-3.0, 0.0, -3.0], [3.0, 0.3, 3.0], pal.stone);
    b
}

/// The TV booth: a cabin with wide windows raised on legs, a ladder up to it. 1 tile.
fn tv_booth(theme: usize) -> Build {
    let pal = palette(theme, [0.85, 0.85, 0.82], [0.30, 0.32, 0.36]);
    let mut b = Build::default();
    let y = 4.5;
    for (x, z) in [(-2.0, -1.5), (2.0, -1.5), (2.0, 1.5), (-2.0, 1.5)] {
        b.cuboid([x - 0.12, 0.0, z - 0.12], [x + 0.12, y, z + 0.12], [0.35, 0.35, 0.36]);
    }
    b.cuboid([-2.4, y, -1.9], [2.4, y + 0.3, 1.9], pal.deck);
    let c = rect(-2.2, -1.7, 2.2, 1.7);
    b.walls(&c, y + 0.3, y + 2.8, pal.wall);
    for i in 0..4 {
        b.panel(&c, i, 0.5, if i % 2 == 0 { 3.8 } else { 2.8 }, y + 1.2, y + 2.5, 0.05, pal.glass);
    }
    b.rect_roof(-2.2, -1.7, 2.2, 1.7, y + 2.8, 0.2, 0.4, Roof::Flat, &pal);
    b.cuboid([-0.05, y + 3.1, -0.05], [0.05, y + 5.0, 0.05], [0.6, 0.6, 0.6]);
    b.prism(&ring(0.0, 0.0, 0.5, 8, 0.0), y + 4.0, y + 4.1, [0.85, 0.85, 0.85]);
    // the ladder
    for x in [-0.4, 0.4] {
        b.cuboid([x - 0.04, 0.0, 1.9], [x + 0.04, y + 0.3, 2.0], [0.35, 0.35, 0.36]);
    }
    let mut r = 0.3;
    while r < y {
        b.cuboid([-0.4, r, 1.9], [0.4, r + 0.05, 2.0], [0.35, 0.35, 0.36]);
        r += 0.35;
    }
    b
}

impl Build {
    /// A palm: a leaning, ringed trunk and drooping fronds.
    fn palm(&mut self, x: f32, z: f32, h: f32, pal: &Palette) {
        let lean = hash(x, z) * TAU;
        let (lx, lz) = (lean.cos() * 0.12, lean.sin() * 0.12);
        let n = 8;
        for k in 0..n {
            let (t0, t1) = (k as f32 / n as f32, (k + 1) as f32 / n as f32);
            let (y0, y1) = (h * t0, h * t1);
            let off = |t: f32| (x + lx * h * t * t, z + lz * h * t * t);
            let (a, b) = (off(t0), off(t1));
            let r0 = 0.28 - 0.08 * t0;
            let col = scale(pal.trunk, if k % 2 == 0 { 1.0 } else { 0.85 });
            let lo = ring(a.0, a.1, r0, 6, 0.0);
            let hi = ring(b.0, b.1, r0 - 0.01, 6, 0.0);
            for i in 0..6 {
                let j = (i + 1) % 6;
                self.poly(
                    &[[lo[i].0, y0, lo[i].1], [lo[j].0, y0, lo[j].1], [hi[j].0, y1, hi[j].1], [hi[i].0, y1, hi[i].1]],
                    [a.0, (y0 + y1) * 0.5, a.1],
                    col,
                );
            }
        }
        let top = [x + lx * h, h, z + lz * h];
        for k in 0..9 {
            let a = TAU * k as f32 / 9.0 + lean;
            let (dx, dz) = (a.cos(), a.sin());
            let len = h * 0.42;
            let mid = [top[0] + dx * len * 0.55, top[1] + 0.5, top[2] + dz * len * 0.55];
            let tip = [top[0] + dx * len, top[1] - h * 0.18, top[2] + dz * len];
            let (sx, sz) = (-dz * 0.55, dx * 0.55);
            let col = if k % 2 == 0 { pal.leaf } else { scale(pal.leaf, 0.8) };
            for up in [1.0, -1.0] {
                let inside = [top[0], top[1] - up * 3.0, top[2]];
                self.poly(&[top, [mid[0] + sx, mid[1], mid[2] + sz], tip], inside, col);
                self.poly(&[top, tip, [mid[0] - sx, mid[1], mid[2] - sz]], inside, col);
            }
        }
    }

    /// Battlements along the top of a wall from `a` to `b` at `y`, `t` thick.
    fn battlements(&mut self, a: (f32, f32), b: (f32, f32), y: f32, t: f32, col: Rgb) {
        let len = ((b.0 - a.0).powi(2) + (b.1 - a.1).powi(2)).sqrt();
        let n = (len / 1.6).floor().max(1.0) as usize;
        let (dx, dz) = ((b.0 - a.0) / len, (b.1 - a.1) / len);
        for k in 0..n {
            let s = len * (k as f32 + 0.25) / n as f32;
            let e = s + len * 0.5 / n as f32;
            let (p, q) = ((a.0 + dx * s, a.1 + dz * s), (a.0 + dx * e, a.1 + dz * e));
            let (nx, nz) = (-dz * t * 0.5, dx * t * 0.5);
            let c = [(p.0 - nx, p.1 - nz), (q.0 - nx, q.1 - nz), (q.0 + nx, q.1 + nz), (p.0 + nx, p.1 + nz)];
            self.prism(&c, y, y + 1.0, col);
        }
    }

    /// A round tower: stone walls, battlements round its top, slit windows and a conical roof if `cone`.
    fn tower(&mut self, x: f32, z: f32, r: f32, h: f32, cone: bool, pal: &Palette) {
        let c = ring(x, z, r, 12, 0.0);
        self.walls(&c, -0.5, h, pal.wall);
        self.courses(&c, &[2.0 * r * (std::f32::consts::PI / 12.0).sin(); 12], 0.0, h, pal);
        for i in (0..12).step_by(3) {
            for y in [h * 0.35, h * 0.7] {
                self.panel(&c, i, 0.5, 0.35, y, y + 1.4, 0.04, [0.10, 0.10, 0.12]);
            }
        }
        if cone {
            self.walls(&ring(x, z, r + 0.3, 12, 0.0), h - 0.6, h, scale(pal.wall, 0.9));
            self.roof(&ring(x, z, r + 0.5, 12, 0.0), h, h + r * 2.2, 0.0, pal.roof);
        } else {
            self.cap(&c, h, true, scale(pal.wall, 0.85));
            for i in 0..12 {
                self.battlements(c[i], c[(i + 1) % 12], h, 0.6, pal.wall);
            }
        }
    }

    /// A horse, standing: body, neck and head, legs and tail.
    fn horse(&mut self, x: f32, z: f32, along_x: bool, col: Rgb) {
        let p = |a: f32, b: f32| if along_x { (x + a, z + b) } else { (x + b, z + a) };
        let bx = |b: &mut Build, a0: f32, a1: f32, y0: f32, y1: f32, w: f32, c: Rgb| {
            let (p0, p1) = (p(a0, -w), p(a1, w));
            b.cuboid([p0.0.min(p1.0), y0, p0.1.min(p1.1)], [p0.0.max(p1.0), y1, p0.1.max(p1.1)], c);
        };
        bx(self, -0.9, 0.9, 1.0, 1.6, 0.3, col);
        for a in [-0.75, 0.75] {
            for w in [-0.2, 0.2] {
                let q = p(a, w);
                self.cuboid([q.0 - 0.07, 0.0, q.1 - 0.07], [q.0 + 0.07, 1.05, q.1 + 0.07], scale(col, 0.8));
            }
        }
        bx(self, 0.75, 1.1, 1.4, 2.1, 0.17, col);
        bx(self, 1.0, 1.55, 1.85, 2.15, 0.13, col);
        bx(self, -1.05, -0.9, 0.9, 1.5, 0.06, scale(col, 0.5));
    }
}

/// The pub (the Links snack bar): a whitewashed stone cottage under thatch, its chimneys and hanging sign, benches
/// outside. 2 tiles.
fn pub_inn(theme: usize) -> Build {
    let pal = Palette { wall: [0.93, 0.92, 0.88], roof: [0.62, 0.52, 0.32], style: 0, ..clubhouse_palette(theme) };
    let mut b = Build::default();
    b.pad(-11.0, -7.0, 11.0, 11.0, 0.03, pal.stone);
    b.block(-8.0, -5.0, 8.0, 2.5, 1, 3.4, Roof::Gable, 4.0, &pal, true);
    for x in [-7.5, 7.5] {
        b.cuboid([x - 0.6, 3.0, -2.0], [x + 0.6, 9.5, -0.6], [0.58, 0.55, 0.50]);
    }
    // the sign on its bracket
    b.cuboid([3.2, 0.0, 3.0], [3.4, 3.6, 3.2], pal.trunk);
    b.cuboid([3.3, 3.4, 3.05], [4.8, 3.5, 3.15], pal.trunk);
    b.cuboid([4.0, 2.2, 3.05], [4.7, 3.3, 3.15], [0.55, 0.15, 0.12]);
    // the gag: a giant pint by the door, its head of froth spilling over
    b.prism(&ring(-4.0, 4.4, 0.9, 12, 0.0), 0.0, 2.6, [0.50, 0.26, 0.08]);
    b.prism(&ring(-4.0, 4.4, 0.95, 12, 0.0), 2.6, 3.0, [0.96, 0.93, 0.84]);
    for k in 0..6 {
        let a = TAU * k as f32 / 6.0;
        b.ball([-4.0 + a.cos() * 0.6, 3.05, 4.4 + a.sin() * 0.6], 0.42, [0.97, 0.95, 0.88]);
    }
    b.ball([-3.1, 2.7, 4.6], 0.3, [0.97, 0.95, 0.88]);
    b.ball([-3.0, 2.2, 4.65], 0.22, [0.97, 0.95, 0.88]);
    for k in 0..3 {
        let x = -7.0 + k as f32 * 4.5;
        b.cuboid([x - 1.0, 0.6, 6.0], [x + 1.0, 0.7, 7.0], pal.deck);
        b.cuboid([x - 1.0, 0.35, 5.2], [x + 1.0, 0.45, 5.6], pal.deck);
        b.cuboid([x - 1.0, 0.35, 7.4], [x + 1.0, 0.45, 7.8], pal.deck);
        for dx in [-0.9, 0.9] {
            b.cuboid([x + dx - 0.05, 0.0, 5.3], [x + dx + 0.05, 0.6, 7.7], pal.deck);
        }
    }
    b.shrub(-10.0, -6.0, 1.2, &pal, false);
    b.tree(10.0, -8.0, 8.0, &pal);
    b
}

/// The stable (the Links take on kind 9): a long timber stable with half doors, a paddock with its rail fence, horses
/// and hay bales. 3 tiles.
fn stable(theme: usize) -> Build {
    let pal =
        Palette { wall: [0.48, 0.28, 0.20], roof: [0.26, 0.28, 0.30], frame: [0.92, 0.90, 0.84], style: 0, ..clubhouse_palette(theme) };
    let mut b = Build::default();
    b.block(-16.0, -22.0, 16.0, -14.0, 1, 3.6, Roof::Gable, 3.4, &pal, false);
    let c = rect(-16.0, -22.0, 16.0, -14.0);
    for k in 0..8 {
        let along = (k as f32 + 0.5) / 8.0;
        b.panel(&c, 2, along, 2.0, 0.5, 3.2, 0.04, pal.frame);
        b.panel(&c, 2, along, 1.7, 0.5, 1.8, 0.07, scale(pal.wall, 0.75));
        b.panel(&c, 2, along, 1.7, 1.9, 3.0, 0.07, [0.08, 0.06, 0.05]);
    }
    // a cupola on the ridge
    b.cuboid([-1.2, 7.2, -19.2], [1.2, 9.0, -16.8], pal.frame);
    b.roof(&rect(-1.6, -19.6, 1.6, -16.4), 9.0, 10.4, 0.0, pal.roof);
    let paddock = rect(-22.0, -8.0, 22.0, 24.0);
    b.railing(&paddock, 0.0, 3.0, None, pal.frame);
    for (k, &(x, z)) in [(-8.0, 2.0), (6.0, 10.0), (-2.0, 16.0)].iter().enumerate() {
        let col = [[0.42, 0.26, 0.15], [0.20, 0.14, 0.10], [0.75, 0.70, 0.62]][k];
        b.horse(x, z, k % 2 == 0, col);
    }
    for &(x, z) in &[(18.0, -12.0), (20.0, -11.0), (19.0, -12.4)] {
        b.cuboid([x - 0.9, 0.0, z - 0.5], [x + 0.9, 0.9, z + 0.5], [0.84, 0.74, 0.42]);
    }
    for &(x, z) in &[(-26.0, -24.0), (26.0, -24.0), (-27.0, 26.0)] {
        b.tree(x, z, 9.0, &pal);
    }
    b
}

/// The church (the Links take on kind 12): a stone nave under slate with tall windows, a square tower with a spire, and
/// a churchyard. 2 tiles.
fn church(theme: usize) -> Build {
    let pal = Palette { wall: [0.62, 0.60, 0.56], roof: [0.24, 0.26, 0.30], style: 1, ..clubhouse_palette(theme) };
    let mut b = Build::default();
    b.block(-5.0, -14.0, 5.0, 6.0, 1, 7.0, Roof::Gable, 5.0, &pal, false);
    // the tower at the front, its door, belfry openings and spire
    let t = rect(-3.0, 6.0, 3.0, 12.0);
    b.walls(&t, -0.5, 16.0, pal.wall);
    b.courses(&t, &[6.0; 4], 0.0, 16.0, &pal);
    b.panel(&t, 2, 0.5, 2.0, 0.0, 3.6, 0.05, pal.door);
    for i in 0..4 {
        b.panel(&t, i, 0.5, 1.2, 12.5, 14.8, 0.05, [0.08, 0.08, 0.10]);
    }
    b.cap(&t, 16.0, true, scale(pal.wall, 0.85));
    for i in 0..4 {
        b.battlements(t[i], t[(i + 1) % 4], 16.0, 0.5, pal.wall);
    }
    b.roof(&rect(-2.4, 6.6, 2.4, 11.4), 16.0, 26.0, 0.0, pal.roof);
    b.cuboid([-0.06, 26.0, 8.94], [0.06, 27.6, 9.06], pal.gold);
    b.cuboid([-0.5, 26.9, 8.94], [0.5, 27.0, 9.06], pal.gold);
    // gravestones and a wall round the churchyard
    for k in 0..12 {
        let (x, z) = (-14.0 + (k % 4) as f32 * 2.2 + hash(k as f32, 1.0), -10.0 + (k / 4) as f32 * 4.0);
        b.cuboid([x - 0.35, 0.0, z - 0.1], [x + 0.35, 0.9 + hash(x, z) * 0.4, z + 0.1], [0.55, 0.55, 0.53]);
    }
    let yard = rect(-18.0, -18.0, 18.0, 16.0);
    for i in 0..4 {
        let (a, c) = (yard[i], yard[(i + 1) % 4]);
        if i == 2 {
            b.cuboid([a.0.min(c.0), 0.0, a.1 - 0.3], [-2.0, 1.0, a.1 + 0.3], pal.wall);
            b.cuboid([2.0, 0.0, a.1 - 0.3], [a.0.max(c.0), 1.0, a.1 + 0.3], pal.wall);
        } else {
            b.cuboid([a.0.min(c.0) - 0.3, 0.0, a.1.min(c.1) - 0.3], [a.0.max(c.0) + 0.3, 1.0, a.1.max(c.1) + 0.3], pal.wall);
        }
    }
    b.cypress(-16.0, 12.0, 8.0, &pal);
    b.cypress(16.0, 12.0, 8.5, &pal);
    b.tree(14.0, -13.0, 10.0, &pal);
    b
}

/// The castle (the Links take on kind 14): a curtain wall with round corner towers and a gatehouse round a tall
/// square keep. 6 tiles.
fn castle(theme: usize) -> Build {
    let pal = Palette { wall: [0.60, 0.58, 0.54], roof: [0.24, 0.26, 0.30], style: 1, ..clubhouse_palette(theme) };
    let mut b = Build::default();
    let (h, w): (f32, f32) = (32.0, 9.0);
    let corners: [(f32, f32); 4] = [(-h, -h), (h, -h), (h, h), (-h, h)];
    for i in 0..4 {
        let (a, c) = (corners[i], corners[(i + 1) % 4]);
        let (x0, x1, z0, z1) = (a.0.min(c.0) - 1.2, a.0.max(c.0) + 1.2, a.1.min(c.1) - 1.2, a.1.max(c.1) + 1.2);
        let wall = rect(x0, z0, x1, z1);
        b.walls(&wall, -0.5, w, pal.wall);
        b.courses(&wall, &[x1 - x0, z1 - z0, x1 - x0, z1 - z0], 0.0, w, &pal);
        b.cap(&wall, w, true, scale(pal.wall, 0.85));
        let out = |p: (f32, f32)| (p.0 * (h + 1.2) / h, p.1 * (h + 1.2) / h);
        b.battlements(out(a), out(c), w, 0.6, pal.wall);
    }
    for &(x, z) in &corners {
        b.tower(x, z, 4.5, 14.0, false, &pal);
    }
    // the gatehouse
    b.tower(-5.5, h, 3.6, 13.0, false, &pal);
    b.tower(5.5, h, 3.6, 13.0, false, &pal);
    b.cuboid([-3.0, 0.0, h + 1.25], [3.0, 4.5, h + 1.3], [0.12, 0.10, 0.08]);
    // the keep
    let k = rect(-10.0, -12.0, 10.0, 8.0);
    b.walls(&k, -0.5, 24.0, pal.wall);
    b.courses(&k, &[20.0; 4], 0.0, 24.0, &pal);
    for i in 0..4 {
        for y in [6.0, 12.0, 18.0] {
            for along in [0.3, 0.7] {
                b.panel(&k, i, along, 0.9, y, y + 2.2, 0.05, [0.10, 0.10, 0.12]);
            }
        }
    }
    b.cap(&k, 24.0, true, scale(pal.wall, 0.85));
    for i in 0..4 {
        b.battlements(k[i], k[(i + 1) % 4], 24.0, 0.7, pal.wall);
    }
    for &(x, z) in &k {
        b.tower(x, z, 2.2, 28.0, true, &pal);
    }
    // a flag on the keep
    b.cuboid([-0.08, 24.0, -2.08], [0.08, 31.0, -1.92], [0.3, 0.3, 0.3]);
    b.poly(&[[0.0, 31.0, -2.0], [3.0, 30.3, -2.0], [3.0, 29.0, -2.0], [0.0, 29.6, -2.0]], [1.0, 30.0, -3.0], [0.15, 0.25, 0.55]);
    b.poly(&[[0.0, 31.0, -2.0], [3.0, 30.3, -2.0], [3.0, 29.0, -2.0], [0.0, 29.6, -2.0]], [1.0, 30.0, -1.0], [0.15, 0.25, 0.55]);
    b.pad(-2.5, h + 1.3, 2.5, h + 14.0, 0.04, [0.55, 0.50, 0.42]);
    for &(x, z) in &[(-50.0, 40.0), (48.0, 44.0), (-52.0, -40.0), (50.0, -46.0)] {
        b.tree(x, z, 11.0, &pal);
    }
    b
}

/// The spa (the Desert take on kind 9): a low adobe building under a flat roof with a shaded terrace, a pool and palms.
/// 3 tiles.
fn spa(theme: usize) -> Build {
    let pal = palette(theme, [0.88, 0.76, 0.60], [0.70, 0.36, 0.24]);
    let mut b = Build::default();
    b.pad(-24.0, -22.0, 24.0, 22.0, 0.04, [0.86, 0.78, 0.64]);
    b.block(-16.0, -20.0, 16.0, -9.0, 1, 4.0, Roof::Flat, 0.5, &pal, true);
    b.block(-16.0, -9.0, -6.0, 0.0, 1, 4.0, Roof::Flat, 0.5, &pal, false);
    // a pergola over the terrace
    for k in 0..6 {
        let x = -4.0 + k as f32 * 3.6;
        b.cuboid([x - 0.15, 0.0, -1.0], [x + 0.15, 3.2, -0.7], pal.frame);
        b.cuboid([x - 0.1, 3.2, -9.0], [x + 0.1, 3.4, -0.7], pal.trunk);
    }
    b.cuboid([-4.2, 3.2, -1.0], [14.2, 3.45, -0.7], pal.trunk);
    b.pad(-2.0, 4.0, 18.0, 14.0, 0.08, [0.20, 0.62, 0.70]);
    b.walls(&rect(-2.3, 3.7, 18.3, 14.3), 0.0, 0.25, [0.92, 0.90, 0.84]);
    b.walls(&rect(-2.0, 4.0, 18.0, 14.0), 0.0, 0.25, [0.92, 0.90, 0.84]);
    for k in 0..4 {
        let x = 0.0 + k as f32 * 4.5;
        b.cuboid([x - 0.4, 0.05, 16.0], [x + 0.4, 0.45, 18.0], pal.frame);
    }
    for &(x, z) in &[(-20.0, 4.0), (-20.0, 14.0), (21.0, 2.0), (21.0, 16.0), (-10.0, 19.0), (10.0, 20.0)] {
        b.palm(x, z, 8.0 + hash(x, z) * 3.0, &pal);
    }
    b
}

/// The helipad (the Desert take on kind 12): a raised pad with its markings, a helicopter, a windsock and a control
/// hut. 2 tiles.
fn helipad(theme: usize) -> Build {
    let pal = palette(theme, [0.85, 0.80, 0.70], [0.6, 0.4, 0.3]);
    let mut b = Build::default();
    let pad = ring(0.0, 2.0, 11.0, 24, 0.0);
    b.prism(&pad, -0.5, 0.6, [0.36, 0.36, 0.38]);
    let edge = ring(0.0, 2.0, 10.2, 24, 0.0);
    for i in 0..24 {
        let (a, c) = (edge[i], edge[(i + 1) % 24]);
        if i % 2 == 0 {
            b.poly(
                &[
                    [a.0, 0.62, a.1],
                    [c.0, 0.62, c.1],
                    [c.0 * 0.95, 0.62, 2.0 + (c.1 - 2.0) * 0.95],
                    [a.0 * 0.95, 0.62, 2.0 + (a.1 - 2.0) * 0.95],
                ],
                [0.0, 0.0, 2.0],
                [0.95, 0.85, 0.2],
            );
        }
    }
    let white = [0.95, 0.95, 0.93];
    b.pad(-3.0, -2.0, -2.0, 6.0, 0.63, white);
    b.pad(2.0, -2.0, 3.0, 6.0, 0.63, white);
    b.pad(-2.0, 1.5, 2.0, 2.5, 0.63, white);
    // the helicopter, standing to one side of the H
    let (hx, hz) = (0.0, 2.0);
    let body = [0.18, 0.30, 0.55];
    b.ball([hx, 2.2, hz], 1.5, body);
    b.ball([hx + 0.9, 2.3, hz], 1.0, [0.55, 0.70, 0.80]);
    b.cuboid([hx - 6.0, 2.2, hz - 0.2], [hx - 1.0, 2.6, hz + 0.2], body);
    b.cuboid([hx - 6.2, 2.2, hz - 0.05], [hx - 5.6, 3.6, hz + 0.05], body);
    for z in [hz - 1.0, hz + 1.0] {
        b.cuboid([hx - 1.5, 0.6, z - 0.06], [hx + 1.6, 0.7, z + 0.06], [0.2, 0.2, 0.2]);
    }
    b.cuboid([hx - 0.06, 3.6, hz - 0.06], [hx + 0.06, 4.0, hz + 0.06], [0.2, 0.2, 0.2]);
    for a in [0.3_f32, 0.3 + std::f32::consts::FRAC_PI_2] {
        let (dx, dz) = (a.cos() * 5.5, a.sin() * 5.5);
        b.poly(
            &[
                [hx - dx - dz * 0.03, 4.0, hz - dz + dx * 0.03],
                [hx + dx - dz * 0.03, 4.0, hz + dz + dx * 0.03],
                [hx + dx + dz * 0.03, 4.05, hz + dz - dx * 0.03],
                [hx - dx + dz * 0.03, 4.05, hz - dz - dx * 0.03],
            ],
            [hx, 3.0, hz],
            [0.15, 0.15, 0.15],
        );
    }
    b.block(10.0, -16.0, 16.0, -11.0, 1, 3.2, Roof::Flat, 0.5, &pal, true);
    b.cuboid([-14.0, 0.0, -12.0], [-13.85, 6.0, -11.85], white);
    b.cuboid([-13.9, 5.2, -12.3], [-11.0, 5.9, -11.5], [0.95, 0.5, 0.1]);
    for &(x, z) in &[(-16.0, 14.0), (16.0, 14.0)] {
        b.palm(x, z, 9.0, &pal);
    }
    b
}

/// The casino (the Desert take on kind 14): a tall hotel tower over a broad, lit podium with a grand entrance, a big
/// sign, fountains and palms. 6 tiles.
fn casino(theme: usize) -> Build {
    let pal =
        Palette { wall: [0.92, 0.86, 0.72], roof: [0.70, 0.36, 0.24], glass: [0.30, 0.42, 0.55], style: 2, ..clubhouse_palette(theme) };
    let mut b = Build::default();
    b.pad(-60.0, -40.0, 60.0, 50.0, 0.03, [0.82, 0.76, 0.64]);
    b.block(-40.0, -30.0, 40.0, 0.0, 2, 4.5, Roof::Flat, 0.5, &pal, true);
    // the tower, its windows in bands
    let t = rect(-14.0, -26.0, 14.0, -14.0);
    let top = 60.0;
    b.walls(&t, 9.0, top, pal.wall);
    let mut y = 10.0;
    while y < top - 2.0 {
        for (i, len) in [(0usize, 28.0), (1, 12.0), (2, 28.0), (3, 12.0)] {
            b.panel(&t, i, 0.5, len - 1.0, y, y + 1.4, 0.04, pal.glass);
        }
        y += 3.0;
    }
    b.rect_roof(-14.0, -26.0, 14.0, -14.0, top, 0.5, 0.3, Roof::Flat, &pal);
    // the gag: a pair of giant dice thrown on the roof
    b.die([20.0, 9.5, -12.0], 5.0);
    b.die([28.0, 9.5, -8.0], 5.0);
    // the sign: a tall pylon of lights
    let gold = pal.gold;
    b.cuboid([28.0, 0.0, 20.0], [30.0, 16.0, 22.0], [0.3, 0.3, 0.32]);
    b.cuboid([24.0, 16.0, 20.5], [34.0, 24.0, 21.5], [0.85, 0.15, 0.25]);
    for k in 0..6 {
        let yy = 17.0 + k as f32 * 1.2;
        b.cuboid([24.5, yy, 21.5], [33.5, yy + 0.4, 21.6], gold);
    }
    b.ball([29.0, 25.2, 21.0], 1.2, gold);
    // the porte-cochère
    b.cuboid([-12.0, 6.0, 0.0], [12.0, 7.0, 10.0], pal.trim);
    for &(x, z) in &[(-11.0, 9.0), (11.0, 9.0), (-11.0, 4.0), (11.0, 4.0)] {
        b.prism(&ring(x, z, 0.5, 10, 0.0), 0.0, 6.0, pal.frame);
    }
    for k in 0..12 {
        b.ball([-11.0 + k as f32 * 2.0, 6.0, 10.05], 0.2, gold);
    }
    // fountains and palms along the drive
    for x in [-26.0, 26.0] {
        b.prism(&ring(x, 25.0, 7.0, 20, 0.0), 0.0, 0.6, pal.stone);
        b.prism(&ring(x, 25.0, 6.4, 20, 0.0), 0.0, 0.65, [0.20, 0.55, 0.75]);
        b.prism(&ring(x, 25.0, 0.5, 8, 0.0), 0.0, 3.5, pal.frame);
        b.ball([x, 4.2, 25.0], 1.0, [0.82, 0.92, 0.96]);
    }
    for k in 0..8 {
        let z = 14.0 + k as f32 * 4.5;
        b.palm(-8.0, z, 9.0 + hash(-8.0, z) * 2.0, &pal);
        b.palm(8.0, z, 9.0 + hash(8.0, z) * 2.0, &pal);
    }
    b
}

/// The theme park (the Tropical take on kind 14): a big wheel, a carousel under a striped canopy, a roller coaster's
/// loop of track on its trestles, stalls and an entrance arch. 6 tiles.
fn theme_park(theme: usize) -> Build {
    let pal = clubhouse_palette(theme);
    let mut b = Build::default();
    b.pad(-60.0, -60.0, 60.0, 60.0, 0.03, [0.78, 0.72, 0.62]);
    let steel = [0.85, 0.86, 0.88];
    // the big wheel, upright in the x-y plane
    let (wx, wz, wy, r) = (-25.0, -25.0, 22.0, 18.0);
    for a in [-1.0, 1.0] {
        let z = wz + a * 1.0;
        let legs = [(wx - 9.0, 0.0), (wx, wy), (wx + 9.0, 0.0)];
        for leg in [(legs[0], legs[1]), (legs[1], legs[2])] {
            let ((x0, y0), (x1, y1)) = leg;
            b.poly(&[[x0 - 0.3, y0, z], [x0 + 0.3, y0, z], [x1 + 0.3, y1, z], [x1 - 0.3, y1, z]], [wx, wy, wz], steel);
            b.poly(&[[x0 - 0.3, y0, z], [x0 + 0.3, y0, z], [x1 + 0.3, y1, z], [x1 - 0.3, y1, z]], [wx, wy, z * 2.0 - wz], steel);
        }
    }
    let n = 16;
    for k in 0..n {
        let (a0, a1) = (TAU * k as f32 / n as f32, TAU * (k + 1) as f32 / n as f32);
        let p = |a: f32, rr: f32| (wx + a.cos() * rr, wy + a.sin() * rr);
        for z in [wz - 0.6, wz + 0.6] {
            let (q0, q1) = (p(a0, r), p(a1, r));
            b.poly(&[[q0.0, q0.1, z], [q1.0, q1.1, z], [q1.0, q1.1 + 0.3, z], [q0.0, q0.1 + 0.3, z]], [wx, wy, wz], steel);
            b.poly(&[[q0.0, q0.1, z], [q1.0, q1.1, z], [q1.0, q1.1 + 0.3, z], [q0.0, q0.1 + 0.3, z]], [wx, wy, z * 2.0 - wz], steel);
            let c = p(a0, r);
            b.poly(&[[wx, wy, z], [c.0, c.1, z], [c.0 + 0.15, c.1 + 0.15, z], [wx + 0.15, wy + 0.15, z]], [wx, wy, wz], steel);
            b.poly(&[[wx, wy, z], [c.0, c.1, z], [c.0 + 0.15, c.1 + 0.15, z], [wx + 0.15, wy + 0.15, z]], [wx, wy, z * 2.0 - wz], steel);
        }
        let c = p(a0, r);
        let col = [[0.85, 0.20, 0.18], [0.20, 0.45, 0.80], [0.95, 0.80, 0.20], [0.25, 0.65, 0.35]][k % 4];
        b.cuboid([c.0 - 1.0, c.1 - 2.4, wz - 1.0], [c.0 + 1.0, c.1 - 0.6, wz + 1.0], col);
    }
    // the carousel
    let (cx, cz) = (22.0, -22.0);
    b.prism(&ring(cx, cz, 9.0, 24, 0.0), 0.0, 0.8, pal.deck);
    b.prism(&ring(cx, cz, 1.2, 12, 0.0), 0.8, 6.0, pal.gold);
    let canopy = ring(cx, cz, 9.6, 24, 0.0);
    for i in 0..24 {
        let (a, c) = (canopy[i], canopy[(i + 1) % 24]);
        let col = if i % 2 == 0 { pal.awning } else { pal.frame };
        b.poly(&[[a.0, 6.0, a.1], [c.0, 6.0, c.1], [cx, 9.5, cz]], [cx, 4.0, cz], col);
        b.poly(&[[a.0, 6.0, a.1], [c.0, 6.0, c.1], [c.0, 5.2, c.1], [a.0, 5.2, a.1]], [cx, 5.6, cz], col);
        if i % 3 == 0 {
            let (px, pz) = (cx + (a.0 - cx) * 0.75, cz + (a.1 - cz) * 0.75);
            b.cuboid([px - 0.05, 0.8, pz - 0.05], [px + 0.05, 5.2, pz + 0.05], pal.gold);
            b.horse(px, pz, true, [0.95, 0.95, 0.93]);
        }
    }
    b.ball([cx, 9.8, cz], 0.5, pal.gold);
    // the roller coaster: a closed loop of track rising and falling on trestles
    let m = 48;
    let track = |k: usize| {
        let a = TAU * k as f32 / m as f32;
        let (x, z) = (10.0 + 40.0 * a.cos(), 26.0 + 20.0 * a.sin());
        let y = 6.0 + 9.0 * (0.5 + 0.5 * (a * 2.0).sin()) + 4.0 * a.cos().max(0.0);
        [x, y, z]
    };
    for k in 0..m {
        let (p0, p1) = (track(k), track((k + 1) % m));
        let col = [0.85, 0.20, 0.18];
        let (dx, dz) = (p1[0] - p0[0], p1[2] - p0[2]);
        let l = (dx * dx + dz * dz).sqrt().max(1e-3);
        let (nx, nz) = (-dz / l * 0.9, dx / l * 0.9);
        b.poly(
            &[
                [p0[0] - nx, p0[1], p0[2] - nz],
                [p0[0] + nx, p0[1], p0[2] + nz],
                [p1[0] + nx, p1[1], p1[2] + nz],
                [p1[0] - nx, p1[1], p1[2] - nz],
            ],
            [p0[0], p0[1] - 3.0, p0[2]],
            col,
        );
        b.poly(
            &[
                [p0[0] - nx, p0[1], p0[2] - nz],
                [p0[0] + nx, p0[1], p0[2] + nz],
                [p1[0] + nx, p1[1], p1[2] + nz],
                [p1[0] - nx, p1[1], p1[2] - nz],
            ],
            [p0[0], p0[1] + 3.0, p0[2]],
            scale(col, 0.7),
        );
        if k % 3 == 0 {
            b.cuboid([p0[0] - 0.2, 0.0, p0[2] - 0.2], [p0[0] + 0.2, p0[1], p0[2] + 0.2], steel);
        }
    }
    // stalls and the entrance arch
    for k in 0..4 {
        let x = -40.0 + k as f32 * 9.0;
        b.cuboid([x, 0.0, 34.0], [x + 6.0, 3.0, 38.0], pal.wall);
        let c = rect(x, 34.0, x + 6.0, 38.0);
        b.awning(&c, 0, 6.0, 3.0, 1.5, 0.6, &pal);
        b.rect_roof(x, 34.0, x + 6.0, 38.0, 3.0, 1.5, 0.3, Roof::Gable, &pal);
    }
    for x in [-6.0, 6.0] {
        b.cuboid([x - 0.8, 0.0, 54.0], [x + 0.8, 9.0, 55.6], pal.awning);
    }
    b.cuboid([-7.0, 9.0, 54.0], [7.0, 11.5, 55.6], pal.gold);
    for &(x, z) in &[(-50.0, 50.0), (50.0, 50.0), (-50.0, 0.0), (52.0, -40.0), (-52.0, -50.0)] {
        b.palm(x, z, 10.0, &pal);
    }
    b
}

/// The model of an object kind (data.rs OBJECT_IDS) in a theme, if it has one: its triangles in world units about the
/// footprint's centre, front to +z (facing 0). `variant` (0..4) picks among a kind's looks (the houses' colours).
pub fn model(kind: usize, theme: usize, variant: usize) -> Option<Vec<Vert>> {
    let b = match sg_core::data::object_id(kind) {
        "clubhouse" => clubhouse(theme),
        "pro-shop" => pro_shop(theme),
        "cart-garage" => cart_garage(theme),
        // (the themed kinds: app::THEMES 0 Parkland, 1 Links, 2 Desert, 3 Tropical; land.rs THEMED_NAMES)
        "snack-bar" if theme == 1 => pub_inn(theme),
        "swim-club" => [tennis_court, stable, spa, swim_club][theme.min(3)](theme),
        "marina" if theme == 1 => church(theme),
        "marina" if theme == 2 => helipad(theme),
        "airstrip" if theme > 0 => [airstrip, castle, casino, theme_park][theme.min(3)](theme),
        "snack-bar" => snack_bar(theme),
        "resort-hotel" => resort_hotel(theme),
        "marina" => marina(theme),
        "airstrip" => airstrip(theme),
        "home-site" => house(theme, variant),
        "driving-range" => driving_range(theme),
        "tv-tower" => tv_tower(theme),
        "tv-booth" => tv_booth(theme),
        _ => return None,
    };
    Some(
        b.v.into_iter()
            .map(|mut v| {
                v.pos = v.pos.map(|q| q * M);
                v
            })
            .collect(),
    )
}

/// What of a model stops a walk: its main building's half size and centre (world units, facing 0).
pub fn model_half(kind: usize, theme: usize) -> Option<(f32, f32, f32, f32)> {
    let (cx, cz, hx, hz) = match sg_core::data::object_id(kind) {
        "snack-bar" if theme == 1 => (0.0, -1.25, 8.0, 3.75),
        "swim-club" if theme == 1 => (0.0, -18.0, 16.0, 4.0),
        "swim-club" if theme == 2 => (0.0, -10.0, 16.0, 10.0),
        "marina" if theme == 1 => (0.0, -1.0, 5.0, 13.0),
        "marina" if theme == 2 => (0.0, 2.0, 11.0, 11.0),
        "airstrip" if theme == 1 => (0.0, 0.0, 34.0, 34.0),
        "airstrip" if theme == 2 => (0.0, -15.0, 40.0, 15.0),
        "airstrip" if theme == 3 => (0.0, 0.0, 0.0, 0.0),
        "clubhouse" => (0.0, -0.5, 31.0, 29.5),
        "snack-bar" => (0.0, -1.5, 4.5, 3.5),
        "pro-shop" => (0.0, -1.5, 9.0, 5.5),
        "cart-garage" => (0.0, -1.5, 13.0, 4.5),
        "swim-club" if theme == 0 => (15.0, -2.0, 3.0, 4.0),
        "swim-club" => (0.0, -14.5, 10.0, 3.5),
        "resort-hotel" => (0.0, -3.0, 26.0, 9.0),
        "marina" => (-2.0, -7.5, 6.0, 4.5),
        "airstrip" => (-15.0, -14.0, 13.0, 8.0),
        "home-site" => (0.0, 0.0, 6.5, 5.5),
        "driving-range" => (0.0, -46.0, 24.0, 4.0),
        "tv-tower" => (0.0, 0.0, 2.2, 2.2),
        "tv-booth" => (0.0, 0.0, 2.4, 1.9),
        _ => return None,
    };
    Some((cx * M, cz * M, hx * M, hz * M))
}

/// `v` turned `quarters` quarter turns about the vertical (counter clockwise from above, x towards -z) and moved to
/// `at`, its ground at `ground`.
pub fn placed(v: &[Vert], quarters: i32, at: [f32; 3]) -> impl Iterator<Item = Vert> + '_ {
    let q = quarters.rem_euclid(4);
    let turn = move |x: f32, z: f32| match q {
        0 => (x, z),
        1 => (z, -x),
        2 => (-x, -z),
        _ => (-z, x),
    };
    v.iter().map(move |v| {
        let mut o = *v;
        let (x, z) = turn(v.pos[0], v.pos[2]);
        o.pos = [at[0] + x, at[1] + v.pos[1], at[2] + z];
        let (nx, nz) = turn(v.normal[0], v.normal[2]);
        o.normal = [nx, v.normal[1], nz];
        o
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn every_modelled_building_stands_in_its_footprint_in_every_theme() {
        let mut modelled = 0;
        for kind in 0..20 {
            let tiles = sg_core::data::buildings()[kind].1.max(1) as f32;
            let half = tiles * sg_core::terrain::TILE_SIZE * 0.5;
            for theme in 0..4 {
                for variant in 0..4 {
                    let Some(v) = model(kind, theme, variant) else { continue };
                    modelled += 1;
                    assert!(v.len() >= 300 && v.len() % 3 == 0, "kind {kind}");
                    assert!(model_half(kind, theme).is_some(), "kind {kind}");
                    for p in &v {
                        assert!(p.pos[0].abs() <= half && p.pos[2].abs() <= half, "kind {kind} theme {theme}: {:?}", p.pos);
                        assert!(p.pos[1] > -2.0 * M && p.pos[1] < 70.0 * M, "kind {kind}");
                    }
                }
            }
        }
        assert_eq!(modelled, 12 * 4 * 4);
        let clubhouse = (0..20).find(|&k| sg_core::data::object_id(k) == "clubhouse").unwrap();
        assert!((0..4).all(|t| model(clubhouse, t, 0).is_some()));
        assert!(model(0, 0, 0).is_none());
    }

    #[test]
    fn a_turned_model_keeps_its_normals_with_its_faces() {
        let mut v = Vert::new(1.0, 0.0, 0.0, 0.0, 0.0);
        v.normal = [1.0, 0.0, 0.0];
        for q in 0..4 {
            let o: Vec<Vert> = placed(&[v], q, [0.0; 3]).collect();
            assert!((o[0].pos[0] - o[0].normal[0]).abs() < 1e-6 && (o[0].pos[2] - o[0].normal[2]).abs() < 1e-6);
        }
    }
}
