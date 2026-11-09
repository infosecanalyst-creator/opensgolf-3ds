//! Enhanced graphics' dawn sprinklers (the port's own; docs/ENHANCED.md, "Dawn sprinklers"), with the time of day: from
//! first light to the early morning a few sprinklers stand on the fairways, greens and tees, each throwing a jet of water
//! that sweeps slowly to and fro, its drops arcing out and falling, and the grass round it darkens as it is watered. They
//! are off by mid-morning, on rainy days, and on frosty or snowy mornings.
//!
//! Drawing only, like the rest of the effects: where they stand comes from a render-side hash of the course (the same each
//! time, a few tiles apart), the jets' sweep and the drops from the effects' clock. The golfers do not see them and the
//! game's rules are untouched. The drops are a few crisp pixels at the art's pixel size, lit like the sprites (fx_life.rs
//! keeps to the same rule for what has no original art).
use crate::app::App;
use crate::fx_light::Atmos;
use crate::gfx::{Gfx, Mode, Uniforms, Vert};
use miniquad::TextureId;
use sg_core::sprites::SPRITE_UNITS_PER_PIXEL;
use sg_core::terrain::{TILE_SIZE, TT_FAIRWAY, TT_FIRM_FAIRWAY, TT_PUTTING_GREEN, TT_TEE, TT_TRICKY_GREEN};
use std::collections::HashSet;

/// The most sprinklers out at once.
const MOST: usize = 24;
/// How far a jet throws (world units).
const REACH: f32 = 1.5 * TILE_SIZE;
/// Drops along a jet.
const DROPS: u32 = 30;

fn smoothstep(a: f32, b: f32, x: f32) -> f32 {
    let t = ((x - a) / (b - a)).clamp(0.0, 1.0);
    t * t * (3.0 - 2.0 * t)
}

fn unit(h: u32) -> f32 {
    (h >> 8) as f32 / (1u32 << 24) as f32
}

fn hash(w: &[u32]) -> u32 {
    crate::fx_light::hash_of(w)
}

/// How strongly the sprinklers run at day phase `p` (0 dawn, 0.15 morning): from just before dawn, off by mid-morning;
/// none in the rain or on a frosty or snowy morning (the atmosphere `a` as worked out so far).
pub fn sprinklers_at(p: f32, a: &Atmos) -> f32 {
    let k = if p > 0.5 { smoothstep(0.975, 0.995, p) } else { 1.0 - smoothstep(0.09, 0.15, p) };
    let cold = if a.snow > 0.0 || a.snowfall > 0.0 { 1.0 } else { smoothstep(0.05, 0.3, a.season.frost) };
    k * (1.0 - a.rain.clamp(0.0, 1.0)) * (1.0 - cold)
}

/// Whether a sprinkler may stand on a tile of this type: the mown ground (tees, greens, fairways).
fn watered(ty: i32) -> bool {
    [TT_TEE, TT_PUTTING_GREEN, TT_FAIRWAY, TT_FIRM_FAIRWAY, TT_TRICKY_GREEN].iter().any(|&t| t as i32 == ty)
}

/// Where the sprinklers stand on a `w` x `h` course whose tile types `ty` gives, for the course's `seed`: about one mown
/// tile in fifteen, at least four tiles apart, at most [`MOST`]; each with its own hash (tile x, y, hash).
pub fn spots(w: i32, h: i32, seed: u32, ty: impl Fn(i32, i32) -> i32) -> Vec<(i32, i32, u32)> {
    let mut out = Vec::new();
    let mut taken: HashSet<(i32, i32)> = HashSet::new();
    for y in 0..h {
        for x in 0..w {
            let hs = hash(&[seed, x as u32, y as u32, 0x7370_726b]);
            if unit(hs) > 0.07 || !watered(ty(x, y)) {
                continue;
            }
            if (-3..=3).any(|dy| (-3..=3).any(|dx| taken.contains(&(x + dx, y + dy)))) {
                continue;
            }
            taken.insert((x, y));
            out.push((x, y, hs));
            if out.len() >= MOST {
                return out;
            }
        }
    }
    out
}

/// Where a jet's drop `i` of [`DROPS`] is at time `t` (seconds), from a sprinkler with hash `hs`: along the ground from the
/// head (world x, z offsets) and its height, and how far along the throw it is (0 at the head .. 1 where it lands).
pub fn drop_at(hs: u32, i: u32, t: f32) -> (f32, f32, f32, f32) {
    // the jet sweeps to and fro across a half circle round its own heading, slowly
    let heading = unit(hash(&[hs, 1])) * std::f32::consts::TAU;
    // the drops leave the head one after another (a throw takes 1 / 0.55 seconds) and fly out along the jet as it was when
    // they left
    let s = (i as f32 / DROPS as f32 + t * 0.55).fract();
    let left = t - s / 0.55;
    let dir = heading + 1.4 * (left * (0.22 + 0.08 * unit(hash(&[hs, 2]))) + unit(hash(&[hs, 3])) * 6.3).sin();
    let jitter = (unit(hash(&[hs, i, 4])) - 0.5) * 0.12;
    let d = s * REACH * (0.9 + 0.1 * unit(hash(&[hs, i, 5])));
    let lift = 4.0 * 0.32 * REACH * s * (1.0 - s);
    ((dir + jitter).cos() * d, (dir + jitter).sin() * d, lift, s)
}

impl App {
    /// The sprinklers of this course: (world x, z, hash) of each.
    fn fx_sprinklers(&self) -> Vec<(f32, f32, u32)> {
        let t = &self.terrain;
        spots(t.w, t.h, self.fx_seed(), |x, y| t.type_at(x, y))
            .into_iter()
            .map(|(x, y, hs)| {
                let (cx, cz) = t.tile_centre(x, y);
                // somewhere on the tile, not always at its middle
                (cx + (unit(hash(&[hs, 6])) - 0.5) * 0.6 * TILE_SIZE, cz + (unit(hash(&[hs, 7])) - 0.5) * 0.6 * TILE_SIZE, hs)
            })
            .collect()
    }

    /// The watered grass round each sprinkler, a little darker: called with the evening's pools, before the sprites.
    pub(crate) fn fx_sprinkler_wet(&mut self, g: &mut Gfx, u: &Uniforms) {
        let k = self.fx.atmos.sprinklers;
        if k <= 0.0 {
            return;
        }
        let dot = crate::fx::gpu(g).dot;
        let mut v: Vec<Vert> = Vec::new();
        for (x, z, _) in self.fx_sprinklers() {
            let y = self.terrain.height_at(x, z) + 0.5;
            let rad = REACH * 1.15;
            let c = [0.0, 0.07, 0.04, 0.38 * k];
            let p = |dx: f32, dz: f32, tu: f32, tv: f32| Vert::new(x + dx, y, z + dz, tu, tv).col(c);
            v.extend_from_slice(&[
                p(-rad, -rad, 0.0, 0.0),
                p(rad, -rad, 1.0, 0.0),
                p(rad, rad, 1.0, 1.0),
                p(-rad, -rad, 0.0, 0.0),
                p(rad, rad, 1.0, 1.0),
                p(-rad, rad, 0.0, 1.0),
            ]);
        }
        g.tris(Mode::Flat, Some(dot), u, &v);
        g.flush();
    }

    /// The sprinklers' heads and their jets' drops, as (depth key, texture, triangles) to be drawn in the sprites' order
    /// (render.rs, with the lamps): each drop its own square of the art's pixel size.
    pub(crate) fn fx_sprinkler_quads(&self, white: TextureId) -> Vec<(f32, Option<TextureId>, Vec<Vert>)> {
        let k = self.fx.atmos.sprinklers;
        if k <= 0.0 {
            return Vec::new();
        }
        let m = self.mv;
        let (rx, ry, rz) = (m[0], m[4], m[8]);
        let (ux, uy, uz) = (m[1], m[5], m[9]);
        let px = SPRITE_UNITS_PER_PIXEL;
        let time = self.fx.time;
        let mut out = Vec::new();
        for (x, z, hs) in self.fx_sprinklers() {
            let gy = self.terrain.height_at(x, z);
            // the dawn's light on the water and the head, as on the sprites
            let tint = self.fx_sprite_tint(x, z, false).unwrap_or([1.0; 4]);
            let square = |wx: f32, wy: f32, wz: f32, x0: f32, y0: f32, x1: f32, y1: f32, c: [f32; 4]| -> Vec<Vert> {
                let at = |cx: f32, cy: f32| {
                    let (cx, cy) = (cx * px, cy * px);
                    Vert::new(wx + rx * cx + ux * cy, wy + ry * cx + uy * cy, wz + rz * cx + uz * cy, 0.5, 0.5).col(c)
                };
                let (a, b, cc, d) = (at(x0, y1), at(x1, y1), at(x1, y0), at(x0, y0));
                vec![a, b, cc, a, cc, d]
            };
            let key = |wx: f32, wy: f32, wz: f32| m[2] * wx + m[6] * wy + m[10] * wz;
            // the head: a dark riser two pixels tall
            let head = [0.20 * tint[0], 0.21 * tint[1], 0.20 * tint[2], k];
            out.push((key(x, gy, z), Some(white), square(x, gy, z, -0.5, 0.0, 0.5, 2.0, head)));
            let water = |a: f32| [0.82 * tint[0] + 0.1, 0.9 * tint[1] + 0.08, 1.0 * tint[2], a];
            for i in 0..DROPS {
                let (dx, dz, lift, s) = drop_at(hs, i, time);
                let (wx, wz) = (x + dx, z + dz);
                let wy = gy + (self.terrain.height_at(wx, wz) - gy) * s + lift;
                // brighter and closer together near the nozzle, thinning out as the jet breaks up
                let a = k * (0.9 - 0.5 * s);
                // a solid stream out of the nozzle (two pixels a drop), single drops as it breaks up
                let tall = if s < 0.4 { 2.0 } else { 1.0 };
                out.push((key(wx, wy, wz), Some(white), square(wx, wy, wz, -0.5, 0.0, 0.5, tall, water(a))));
                // the spray breaking off the far half of the jet, falling
                if s > 0.5 && hash(&[hs, i, 8]) & 1 == 0 {
                    let fall = (s - 0.5) * 2.0;
                    let off = (unit(hash(&[hs, i, 9])) - 0.5) * 0.3 * TILE_SIZE;
                    let (fx, fz) = (wx + off * 0.7, wz - off * 0.7);
                    let fy = self.terrain.height_at(fx, fz) + lift * (1.0 - fall);
                    out.push((key(fx, fy, fz), Some(white), square(fx, fy, fz, -0.5, 0.0, 0.5, 1.0, water(a * 0.6))));
                }
            }
        }
        out
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn sprinklers_run_at_dawn_only() {
        let a = Atmos::default();
        assert!(sprinklers_at(0.02, &a) > 0.99);
        assert!(sprinklers_at(0.99, &a) > 0.5);
        assert_eq!(sprinklers_at(0.2, &a), 0.0);
        assert_eq!(sprinklers_at(0.5, &a), 0.0);
        assert_eq!(sprinklers_at(0.9, &a), 0.0);
        // not in the rain, nor on a frosty morning
        let wet = Atmos { rain: 1.0, ..Atmos::default() };
        assert_eq!(sprinklers_at(0.02, &wet), 0.0);
        let mut frosty = Atmos::default();
        frosty.season.frost = 0.8;
        assert_eq!(sprinklers_at(0.02, &frosty), 0.0);
    }

    #[test]
    fn sprinklers_stand_on_mown_ground_apart() {
        // a 40 x 40 course of fairway with a strip of rough down the middle
        let ty = |x: i32, _y: i32| if (18..22).contains(&x) { sg_core::terrain::TT_ROUGH as i32 } else { TT_FAIRWAY as i32 };
        let s = spots(40, 40, 7, ty);
        assert!(!s.is_empty() && s.len() <= MOST);
        for (i, a) in s.iter().enumerate() {
            assert!(!(18..22).contains(&a.0));
            for b in &s[i + 1..] {
                assert!((a.0 - b.0).abs() > 3 || (a.1 - b.1).abs() > 3);
            }
        }
        // the same each time
        assert_eq!(s, spots(40, 40, 7, ty));
        // none on a course without mown ground
        assert!(spots(40, 40, 7, |_, _| sg_core::terrain::TT_ROUGH as i32).is_empty());
    }

    #[test]
    fn a_drop_arcs_out_and_lands() {
        for i in 0..DROPS {
            let (dx, dz, lift, s) = drop_at(1234, i, 3.7);
            let d = dx.hypot(dz);
            assert!(d <= REACH * 1.001);
            assert!((0.0..1.0).contains(&s));
            assert!(lift >= 0.0 && lift <= 0.33 * REACH);
        }
    }
}
