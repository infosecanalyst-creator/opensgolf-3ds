//! Modelled fly-overs and odd landmarks for the walk camera: the blimp, the hot air balloons and the desert's oil pump.
//!
//! Their pictures are drawn for the overhead view (a blimp seen from above at an angle, a pump jack's frame from one
//! corner), so stood up at eye level as cards they lean and flatten. Here each is plain coloured geometry of the
//! project's own design, in the walk camera's cartoon finish, coloured after the game's own art: the blimp orange at
//! its ends round a cream middle, the four balloons in their four patterns, the pump jack red on a sandy pad.
//!
//! Models are built in metres with y up, about their own centre (the blimp, the balloon) or their foot (the pump), and
//! facing +x; `place` turns, scales and moves them into the world.

use crate::gfx::Vert;
use std::f32::consts::{PI, TAU};

type V = [f32; 3];
type Rgb = [f32; 3];

fn add(a: V, b: V) -> V {
    [a[0] + b[0], a[1] + b[1], a[2] + b[2]]
}
fn sub(a: V, b: V) -> V {
    [a[0] - b[0], a[1] - b[1], a[2] - b[2]]
}
fn mul(a: V, k: f32) -> V {
    [a[0] * k, a[1] * k, a[2] * k]
}
fn dot(a: V, b: V) -> f32 {
    a[0] * b[0] + a[1] * b[1] + a[2] * b[2]
}
fn cross(a: V, b: V) -> V {
    [a[1] * b[2] - a[2] * b[1], a[2] * b[0] - a[0] * b[2], a[0] * b[1] - a[1] * b[0]]
}
fn norm(a: V) -> V {
    let l = dot(a, a).sqrt().max(1e-6);
    mul(a, 1.0 / l)
}

fn vert(p: V, n: V, c: Rgb) -> Vert {
    let mut v = Vert::new(p[0], p[1], p[2], 0.0, 0.0).col([c[0], c[1], c[2], 1.0]);
    v.normal = n;
    v
}

/// Two unit vectors square to `axis` and to each other.
fn frame(axis: V) -> (V, V) {
    let a = norm(axis);
    let h = if a[1].abs() < 0.9 { [0.0, 1.0, 0.0] } else { [1.0, 0.0, 0.0] };
    let u = norm(cross(a, h));
    (u, cross(a, u))
}

/// A body of revolution about the line from `base` along `axis` (unit): `profile` is (distance along the axis, radius)
/// from one end to the other, `seg` the divisions round it, `paint(around 0..1, along index)` its colour. Smooth: each
/// point's normal is the profile's own, so it shades round.
fn lathe(out: &mut Vec<Vert>, base: V, axis: V, profile: &[(f32, f32)], seg: usize, paint: impl Fn(f32, usize) -> Rgb) {
    let (u, w) = frame(axis);
    let n = profile.len();
    // each profile point's normal in the (along, out) plane, from the slope either side
    let slope: Vec<(f32, f32)> = (0..n)
        .map(|j| {
            let (a0, r0) = profile[j.saturating_sub(1)];
            let (a1, r1) = profile[(j + 1).min(n - 1)];
            let (da, dr) = (a1 - a0, r1 - r0);
            let l = (da * da + dr * dr).sqrt().max(1e-6);
            (-dr / l, da / l)
        })
        .collect();
    let at = |i: usize, j: usize| {
        let th = TAU * i as f32 / seg as f32;
        let (s, c) = th.sin_cos();
        let radial = add(mul(u, c), mul(w, s));
        let (a, r) = profile[j];
        let (na, nr) = slope[j];
        (add(base, add(mul(axis, a), mul(radial, r))), norm(add(mul(axis, na), mul(radial, nr))))
    };
    for j in 0..n - 1 {
        for i in 0..seg {
            let col = paint((i as f32 + 0.5) / seg as f32, j);
            let (p00, n00) = at(i, j);
            let (p10, n10) = at(i + 1, j);
            let (p01, n01) = at(i, j + 1);
            let (p11, n11) = at(i + 1, j + 1);
            out.extend([vert(p00, n00, col), vert(p10, n10, col), vert(p11, n11, col)]);
            out.extend([vert(p00, n00, col), vert(p11, n11, col), vert(p01, n01, col)]);
        }
    }
}

/// A round rod from `a` to `b`, radii `ra` and `rb`, capped.
fn rod(out: &mut Vec<Vert>, a: V, b: V, ra: f32, rb: f32, seg: usize, col: Rgb) {
    let d = sub(b, a);
    let l = dot(d, d).sqrt();
    let prof = [(0.0, 0.0), (0.0, ra), (l, rb), (l, 0.0)];
    lathe(out, a, norm(d), &prof, seg, |_, _| col);
}

/// A box with its centre `c`, half sizes `h` along the unit axes `x` and `y` (and their cross product), flat faced.
fn block(out: &mut Vec<Vert>, c: V, h: V, x: V, y: V, col: Rgb) {
    let z = cross(x, y);
    let axes = [x, y, z];
    for (k, &ax) in axes.iter().enumerate() {
        let (p, q) = (axes[(k + 1) % 3], axes[(k + 2) % 3]);
        let (hp, hq) = (h[(k + 1) % 3], h[(k + 2) % 3]);
        for s in [-1.0f32, 1.0] {
            let n = mul(ax, s);
            let f = add(c, mul(ax, s * h[k]));
            let corner = |a: f32, b: f32| add(f, add(mul(p, a * hp), mul(q, b * hq)));
            let (c0, c1, c2, c3) = (corner(-1.0, -1.0), corner(1.0, -1.0), corner(1.0, 1.0), corner(-1.0, 1.0));
            if s > 0.0 {
                out.extend([vert(c0, n, col), vert(c1, n, col), vert(c2, n, col), vert(c0, n, col), vert(c2, n, col), vert(c3, n, col)]);
            } else {
                out.extend([vert(c0, n, col), vert(c2, n, col), vert(c1, n, col), vert(c0, n, col), vert(c3, n, col), vert(c2, n, col)]);
            }
        }
    }
}

/// A box between `a` and `b` (its long axis), `w` wide (across, square to `up`) and `t` thick (along `up`).
fn beam(out: &mut Vec<Vert>, a: V, b: V, w: f32, t: f32, up: V, col: Rgb) {
    let d = sub(b, a);
    let l = dot(d, d).sqrt();
    let x = norm(d);
    let y = norm(sub(up, mul(x, dot(up, x))));
    block(out, mul(add(a, b), 0.5), [l * 0.5, t * 0.5, w * 0.5], x, y, col);
}

const ORANGE: Rgb = [0.93, 0.43, 0.10];
const RUST: Rgb = [0.78, 0.26, 0.08];
const CREAM: Rgb = [0.96, 0.92, 0.80];
const DARK: Rgb = [0.18, 0.17, 0.20];

/// The blimp, about 52 m long, nose to +x: an orange envelope with a cream middle band, a round emblem on each flank, a
/// cross of fins at the tail and a gondola with its engines underneath.
pub fn blimp() -> Vec<Vert> {
    let mut o = Vec::with_capacity(9000);
    let len = 52.0;
    let r = 7.0;
    let n = 18;
    // a teardrop: full at the front third, drawing to a fine tail
    let prof: Vec<(f32, f32)> = (0..=n)
        .map(|j| {
            let t = j as f32 / n as f32;
            let a = (t * PI).sin().max(0.0).powf(if t < 0.4 { 0.62 } else { 0.85 });
            ((0.5 - t) * len, r * a)
        })
        .collect();
    lathe(&mut o, [0.0, 0.0, 0.0], [-1.0, 0.0, 0.0], &prof.iter().map(|&(a, rr)| (-a, rr)).collect::<Vec<_>>(), 28, |_, j| {
        let t = j as f32 / n as f32;
        if (0.3..0.68).contains(&t) {
            CREAM
        } else if (0.26..0.3).contains(&t) || (0.68..0.72).contains(&t) {
            RUST
        } else {
            ORANGE
        }
    });
    // the emblems: an orange ring round a cream disc, on each flank
    for s in [-1.0f32, 1.0] {
        let c = [1.5, 0.6, s * (r * 0.99)];
        lathe(&mut o, c, [0.0, 0.0, s], &[(-0.3, 0.0), (-0.3, 4.6), (0.15, 4.6), (0.3, 3.9), (0.3, 0.0)], 24, |_, j| {
            if j >= 3 {
                CREAM
            } else {
                ORANGE
            }
        });
        lathe(&mut o, add(c, [0.0, 0.0, s * 0.32]), [0.0, 0.0, s], &[(0.0, 0.0), (0.0, 3.2), (0.12, 3.0), (0.12, 0.0)], 20, |_, _| CREAM);
    }
    // fins: a cross at the tail, swept back, orange edged in rust
    let tail = -len * 0.5;
    for (dy, dz) in [(1.0f32, 0.0f32), (-1.0, 0.0), (0.0, 1.0), (0.0, -1.0)] {
        let out_dir = [0.0, dy, dz];
        let root_front = [tail + 13.0, 0.0, 0.0];
        let root_back = [tail + 2.5, 0.0, 0.0];
        let tip_front = add([tail + 6.5, 0.0, 0.0], mul(out_dir, 8.0));
        let tip_back = add([tail + 1.0, 0.0, 0.0], mul(out_dir, 8.0));
        let side = cross([1.0, 0.0, 0.0], out_dir);
        let th = 0.35;
        for (a, b, c, d, col) in [(root_front, root_back, tip_back, tip_front, ORANGE)] {
            for s in [-1.0f32, 1.0] {
                let n = mul(side, s);
                let off = mul(side, s * th);
                let q = [add(a, off), add(b, off), add(c, off), add(d, off)];
                if s > 0.0 {
                    o.extend([
                        vert(q[0], n, col),
                        vert(q[1], n, col),
                        vert(q[2], n, col),
                        vert(q[0], n, col),
                        vert(q[2], n, col),
                        vert(q[3], n, col),
                    ]);
                } else {
                    o.extend([
                        vert(q[0], n, col),
                        vert(q[2], n, col),
                        vert(q[1], n, col),
                        vert(q[0], n, col),
                        vert(q[3], n, col),
                        vert(q[2], n, col),
                    ]);
                }
            }
        }
        // the fin's rim
        rod(&mut o, tip_front, tip_back, 0.45, 0.45, 6, RUST);
    }
    // the gondola: a cream capsule with a band of dark windows, slung under the front half
    let gy = -r - 1.1;
    lathe(
        &mut o,
        [-4.0, gy, 0.0],
        [1.0, 0.0, 0.0],
        &[(0.0, 0.0), (0.5, 1.0), (1.5, 1.45), (8.0, 1.45), (9.5, 1.1), (10.6, 0.0)],
        14,
        |a, j| {
            if (j == 2 || j == 3) && (0.15..0.35).contains(&a) || (j == 2 || j == 3) && (0.65..0.85).contains(&a) {
                DARK
            } else {
                CREAM
            }
        },
    );
    // the engines either side, on short pylons
    for s in [-1.0f32, 1.0] {
        let e = [0.5, gy + 0.3, s * 3.2];
        rod(&mut o, [0.5, gy + 0.3, s * 1.2], e, 0.22, 0.22, 6, DARK);
        lathe(&mut o, add(e, [-1.6, 0.0, 0.0]), [1.0, 0.0, 0.0], &[(0.0, 0.0), (0.3, 0.6), (2.6, 0.6), (3.2, 0.0)], 10, |_, _| ORANGE);
    }
    o
}

/// One of the four balloons' patterns (the game's balloon art, one per row of the course): the colour of the envelope
/// `around` (0..1) and `up` (0 mouth .. 1 crown).
fn balloon_paint(design: usize, around: f32, up: f32) -> Rgb {
    let gore = (around * 16.0) as usize;
    match design & 3 {
        // red, white, purple and blue bands, a red and white target on the crown
        0 => {
            const B: [Rgb; 6] =
                [[0.36, 0.25, 0.80], [0.30, 0.42, 0.95], [0.95, 0.95, 0.96], [0.86, 0.12, 0.22], [0.95, 0.95, 0.96], [0.86, 0.12, 0.22]];
            B[((up * 6.0) as usize).min(5)]
        }
        // teal and deep blue gores, a band of white chevrons round the middle
        1 => {
            let chevron = ((up - 0.55).abs() - (around * 32.0).fract() * 0.08).abs() < 0.035;
            if chevron {
                [0.95, 0.97, 0.98]
            } else if gore.is_multiple_of(2) {
                [0.10, 0.62, 0.66]
            } else {
                [0.10, 0.26, 0.62]
            }
        }
        // blue and violet gores with pink seams
        2 => {
            if (around * 16.0).fract() < 0.12 {
                [0.92, 0.42, 0.78]
            } else if gore.is_multiple_of(2) {
                [0.22, 0.30, 0.86]
            } else {
                [0.46, 0.24, 0.80]
            }
        }
        // red and yellow stripes
        _ => {
            if gore.is_multiple_of(2) {
                [0.92, 0.16, 0.10]
            } else {
                [0.99, 0.82, 0.16]
            }
        }
    }
}

/// A hot air balloon about 22 m tall, its basket's floor at y 0: the envelope in `design`'s pattern, ropes and a wicker
/// basket with its burner.
pub fn balloon(design: usize) -> Vec<Vert> {
    let mut o = Vec::with_capacity(9000);
    let (mouth, top, wide) = (5.5, 22.0, 8.0);
    let n = 16;
    // the envelope: a narrow mouth flaring to its widest two thirds of the way up, then round over the crown
    let prof: Vec<(f32, f32)> = (0..=n)
        .map(|j| {
            let t = j as f32 / n as f32;
            let y = mouth + (top - mouth) * t;
            let r = if t < 0.62 {
                let k = t / 0.62;
                1.3 + (wide - 1.3) * (k * PI * 0.5).sin().powf(0.9)
            } else {
                let k = (t - 0.62) / 0.38;
                wide * (1.0 - k * k).max(0.0).sqrt()
            };
            (y, r)
        })
        .collect();
    let profile: Vec<(f32, f32)> = std::iter::once((mouth, 0.0)).chain(prof.iter().copied()).collect();
    lathe(&mut o, [0.0, 0.0, 0.0], [0.0, 1.0, 0.0], &profile, 32, |a, j| balloon_paint(design, a, (j as f32) / (profile.len() - 1) as f32));
    // ropes from the mouth to the basket's corners
    let rope = [0.45, 0.36, 0.24];
    for (cx, cz) in [(1.0f32, 1.0f32), (-1.0, 1.0), (1.0, -1.0), (-1.0, -1.0)] {
        rod(&mut o, [cx * 1.05, mouth + 0.2, cz * 1.05], [cx * 0.75, 1.3, cz * 0.75], 0.06, 0.06, 4, rope);
    }
    // the burner
    lathe(&mut o, [0.0, 1.7, 0.0], [0.0, 1.0, 0.0], &[(0.0, 0.0), (0.0, 0.45), (0.9, 0.35), (0.9, 0.0)], 8, |_, _| [0.55, 0.56, 0.6]);
    // the wicker basket with a leather rim
    let wicker = [0.62, 0.43, 0.22];
    block(&mut o, [0.0, 0.6, 0.0], [0.8, 0.6, 0.8], [1.0, 0.0, 0.0], [0.0, 1.0, 0.0], wicker);
    block(&mut o, [0.0, 1.23, 0.0], [0.86, 0.07, 0.86], [1.0, 0.0, 0.0], [0.0, 1.0, 0.0], [0.36, 0.2, 0.1]);
    o
}

/// The desert's pump jack, about 9 m long and 6 m high, its well to +x: a sandy pad and a skid, the Samson post, the
/// walking beam with its horse head over the well, and the cranks and their counterweights turning at the back. `t` in
/// seconds turns the cranks and nods the head.
pub fn pump_jack(t: f32) -> Vec<Vert> {
    let mut o = Vec::with_capacity(6000);
    let red = [0.80, 0.18, 0.12];
    let deep = [0.52, 0.10, 0.08];
    let steel = [0.60, 0.60, 0.62];
    let sand = [0.86, 0.70, 0.46];
    let up = [0.0, 1.0, 0.0];
    // the pad, a low round slab of packed sand, and the skid along it
    lathe(&mut o, [0.0, 0.0, 0.0], up, &[(0.0, 0.0), (0.0, 5.6), (0.12, 5.4), (0.18, 0.0)], 20, |_, _| sand);
    block(&mut o, [0.0, 0.35, 0.0], [4.6, 0.18, 0.75], [1.0, 0.0, 0.0], up, deep);
    // the Samson post: an A of two legs each side, up to the saddle
    let saddle = [0.9, 5.3, 0.0];
    for s in [-1.0f32, 1.0] {
        for fx in [-0.9f32, 2.5] {
            rod(&mut o, [fx, 0.5, s * 0.85], add(saddle, [0.0, -0.2, s * 0.22]), 0.16, 0.13, 6, red);
        }
    }
    // the walking beam, rocking about the saddle as the cranks turn
    let ang = (t * 1.6).sin() * 0.28;
    let (sa, ca) = ang.sin_cos();
    let along = [ca, sa, 0.0];
    let beam_up = [-sa, ca, 0.0];
    let front = add(saddle, mul(along, 3.6));
    let back = add(saddle, mul(along, -3.4));
    beam(&mut o, back, front, 0.5, 0.7, beam_up, red);
    // the horse head: a curved plate over the well, and the bridle (two wires) down to the polished rod
    let head = add(front, mul(beam_up, -0.2));
    let hr = 1.5;
    let steps = 6;
    for i in 0..steps {
        let a0 = -0.9 + 1.7 * i as f32 / steps as f32;
        let a1 = -0.9 + 1.7 * (i + 1) as f32 / steps as f32;
        let p = |a: f32| add(head, add(mul(along, a.cos() * hr * 0.55), mul(beam_up, a.sin() * hr)));
        beam(&mut o, p(a0), p(a1), 0.62, 0.32, along, red);
    }
    let hang = add(head, add(mul(along, hr * 0.55), mul(beam_up, -0.2)));
    let rod_top = [hang[0], hang[1] - 2.2, 0.0];
    for s in [-1.0f32, 1.0] {
        rod(&mut o, add(hang, [0.0, 0.0, s * 0.12]), add(rod_top, [0.0, 0.0, s * 0.12]), 0.03, 0.03, 4, DARK);
    }
    block(&mut o, rod_top, [0.2, 0.08, 0.3], [1.0, 0.0, 0.0], up, DARK);
    rod(&mut o, rod_top, [rod_top[0], 0.4, 0.0], 0.06, 0.06, 6, steel);
    // the wellhead
    lathe(
        &mut o,
        [rod_top[0], 0.15, 0.0],
        up,
        &[(0.0, 0.0), (0.0, 0.35), (0.9, 0.3), (1.0, 0.45), (1.25, 0.45), (1.3, 0.0)],
        10,
        |_, _| steel,
    );
    // the gearbox on its stand, and the cranks with their counterweights, and the pitmans up to the beam's tail
    let hub = [-2.9, 1.7, 0.0];
    block(&mut o, [-2.9, 1.0, 0.0], [0.7, 1.0, 0.55], [1.0, 0.0, 0.0], up, deep);
    let crank = t * 1.6 - PI * 0.5;
    let (sc, cc) = crank.sin_cos();
    for s in [-1.0f32, 1.0] {
        let side = s * 0.85;
        let pin = add(hub, [cc * 1.1, sc * 1.1, side]);
        let weight = add(hub, [-cc * 0.9, -sc * 0.9, side]);
        beam(&mut o, add(hub, [0.0, 0.0, side]), pin, 0.18, 0.4, [0.0, 0.0, 1.0], deep);
        // the counterweight: a fat round block opposite the pin
        lathe(
            &mut o,
            add(weight, [0.0, 0.0, -0.18 * s]),
            [0.0, 0.0, s],
            &[(0.0, 0.0), (0.0, 0.8), (0.36, 0.8), (0.36, 0.0)],
            12,
            |_, _| DARK,
        );
        // the pitman, up to the beam's tail
        rod(&mut o, pin, add(back, [0.0, 0.0, side * 0.5]), 0.09, 0.09, 6, red);
    }
    o
}

/// `v` turned to face `heading` (radians, from +x towards +z), scaled by `k` and moved to `at`.
pub fn place(v: &[Vert], heading: f32, k: f32, at: V) -> impl Iterator<Item = Vert> + '_ {
    let (s, c) = heading.sin_cos();
    v.iter().map(move |q| {
        let mut w = *q;
        let (x, z) = (q.pos[0], q.pos[2]);
        w.pos = [at[0] + (x * c - z * s) * k, at[1] + q.pos[1] * k, at[2] + (x * s + z * c) * k];
        let (nx, nz) = (q.normal[0], q.normal[2]);
        w.normal = [nx * c - nz * s, q.normal[1], nx * s + nz * c];
        w
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    fn bounds(v: &[Vert]) -> (V, V) {
        let mut lo = [f32::MAX; 3];
        let mut hi = [f32::MIN; 3];
        for p in v {
            for k in 0..3 {
                lo[k] = lo[k].min(p.pos[k]);
                hi[k] = hi[k].max(p.pos[k]);
            }
        }
        (lo, hi)
    }

    #[test]
    fn the_models_are_the_size_they_say() {
        let (lo, hi) = bounds(&blimp());
        assert!((hi[0] - lo[0] - 52.0).abs() < 2.0, "{lo:?} {hi:?}");
        let (lo, hi) = bounds(&balloon(2));
        assert!(lo[1].abs() < 0.01 && (hi[1] - 22.0).abs() < 0.5, "{lo:?} {hi:?}");
        let (lo, hi) = bounds(&pump_jack(0.3));
        assert!(lo[1] > -0.01 && hi[1] < 7.5 && hi[1] > 5.0, "{lo:?} {hi:?}");
    }

    #[test]
    fn every_normal_is_a_unit_and_every_triangle_whole() {
        for v in [blimp(), balloon(0), balloon(1), balloon(3), pump_jack(1.0)] {
            assert_eq!(v.len() % 3, 0);
            for p in &v {
                let l = dot(p.normal, p.normal).sqrt();
                assert!((l - 1.0).abs() < 1e-3, "{:?}", p.normal);
            }
        }
    }

    #[test]
    fn placing_turns_the_nose_to_the_heading() {
        let v = [vert([1.0, 0.0, 0.0], [1.0, 0.0, 0.0], CREAM)];
        let w: Vec<Vert> = place(&v, PI * 0.5, 2.0, [10.0, 0.0, 0.0]).collect();
        assert!((w[0].pos[0] - 10.0).abs() < 1e-4 && (w[0].pos[2] - 2.0).abs() < 1e-4);
    }

    /// Writes the models for looking at outside the game (SG_SKY_DUMP=dir): a line a point, position, normal, colour.
    #[test]
    #[ignore]
    fn dump() {
        let Some(dir) = std::env::var_os("SG_SKY_DUMP") else { return };
        let dir = std::path::PathBuf::from(dir);
        let models = [
            ("blimp", blimp()),
            ("balloon0", balloon(0)),
            ("balloon1", balloon(1)),
            ("balloon2", balloon(2)),
            ("balloon3", balloon(3)),
            ("pump", pump_jack(0.4)),
        ];
        for (name, v) in models {
            let s: String = v
                .iter()
                .map(|p| {
                    format!(
                        "{} {} {} {} {} {} {} {} {}\n",
                        p.pos[0], p.pos[1], p.pos[2], p.normal[0], p.normal[1], p.normal[2], p.color[0], p.color[1], p.color[2]
                    )
                })
                .collect();
            std::fs::write(dir.join(format!("{name}.txt")), s).unwrap();
        }
    }
}
