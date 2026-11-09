//! Enhanced graphics' ball effects (docs/ENHANCED.md, "Ball trail and impacts"): a soft trail fading behind a ball in the air,
//! and where a ball comes down a small effect for what it lands on: a puff of sand in a bunker, a ring spreading on water,
//! a flick of grass elsewhere. Drawing only: the balls' places are read from the simulation once a tick (after it has
//! run) and kept here; nothing is written back, and what varies between impacts comes from a hash of the impact, not from
//! the game's random generator.
use crate::app::App;
use crate::gfx::{Gfx, Mode, Uniforms, Vert};
use miniquad::*;
use sg_core::golfer::flag;

/// How many ticks a trail point lasts (about two thirds of a second).
const TRAIL_TICKS: f32 = 8.0;

/// What a ball landed on.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
enum Ground {
    Grass,
    Sand,
    Water,
}

#[derive(Clone, Copy, Debug)]
struct Impact {
    /// Where, in the simulation's units.
    x: f32,
    y: f32,
    ground: Ground,
    /// The game tick it happened at.
    born: u32,
    /// 1 for a ball coming down from a flight, smaller for a bounce.
    size: f32,
}

/// The trails (per golfer: the ball's place, in the simulation's units with its height, at the last few ticks in the air)
/// and the impacts still showing.
#[derive(Default)]
pub struct BallState {
    trails: Vec<Vec<(u32, [f32; 3])>>,
    /// Each golfer's ball at the last tick (x, y, height), and how many times it has come down since it was struck.
    last: Vec<Option<([i32; 3], u32)>>,
    impacts: Vec<Impact>,
}

/// A soft round dot, white with its density in alpha (the impacts' particles and the rain's splashes).
pub fn dot_texture(ctx: &mut dyn RenderingBackend) -> TextureId {
    const N: usize = 16;
    let mut px = vec![0u8; N * N * 4];
    for y in 0..N {
        for x in 0..N {
            let (dx, dy) = ((x as f32 + 0.5) / N as f32 * 2.0 - 1.0, (y as f32 + 0.5) / N as f32 * 2.0 - 1.0);
            let d = (dx * dx + dy * dy).sqrt();
            let a = (1.0 - d).clamp(0.0, 1.0);
            let k = (y * N + x) * 4;
            px[k..k + 4].copy_from_slice(&[255, 255, 255, (a * a * (3.0 - 2.0 * a) * 255.0) as u8]);
        }
    }
    let t = ctx.new_texture_from_rgba8(N as u16, N as u16, &px);
    crate::fx::plain_texture(t);
    t
}

fn hash(a: u32, b: u32) -> f32 {
    let mut h = a.wrapping_mul(0x9e37_79b9) ^ b.wrapping_mul(0x85eb_ca6b);
    h ^= h >> 15;
    h = h.wrapping_mul(0x2c1b_3c6d);
    h ^= h >> 12;
    (h >> 8) as f32 / (1u32 << 24) as f32
}

impl App {
    /// After each simulation tick (Enhanced graphics' ball effects on): keeps the balls' places in the air and notes where
    /// they come down. Reads the simulation only.
    pub(crate) fn fx_ball_tick(&mut self) {
        let s = crate::fx::settings();
        if !(s.ball_trail || s.ball_impacts) {
            return;
        }
        let tick = self.game_tick;
        // (the golfers' only: not the hole tool's trial golfer past them, render.rs draw_balls)
        let n = self.club.g.len().min(sg_core::golfer::SLOTS);
        let st = &mut self.fx.balls;
        st.trails.resize_with(n, Vec::new);
        st.last.resize(n, None);
        st.impacts.retain(|i| tick.wrapping_sub(i.born) < 16);
        for i in 0..n {
            let gl = &self.club.g[i];
            let gone = gl.bx == 0 || gl.hole <= 0 || gl.hole >= 19;
            let now = [gl.bx, gl.by, gl.bz];
            let in_water = gl.flags & flag::PENALTY != 0;
            let prev = st.last[i];
            st.trails[i].retain(|p| (tick.wrapping_sub(p.0) as f32) < TRAIL_TICKS);
            if gone {
                st.last[i] = None;
                continue;
            }
            let mut downs = prev.map(|p| p.1).unwrap_or(0);
            if now[2] > 0 {
                if s.ball_trail {
                    st.trails[i].push((tick, [now[0] as f32, now[1] as f32, now[2] as f32]));
                }
            } else if let Some((p, d)) = prev {
                if p[2] > 0 && s.ball_impacts {
                    // came down this tick
                    let ty = self.course.tile_type(now[0], now[1]);
                    let ground = match ty {
                        7..=9 => Ground::Sand,
                        17..=19 => Ground::Water,
                        _ if in_water => Ground::Water,
                        _ => Ground::Grass,
                    };
                    // a ball in the water may be taken back to its drop at once: the splash is where it came down
                    let at = if in_water { p } else { now };
                    if std::env::var_os("SG_FX_LOG").is_some() {
                        println!(
                            "fx: ball {i} came down on {ground:?} at tile {},{} (tick {tick}, time {:.2})",
                            at[0] >> 10,
                            at[1] >> 10,
                            self.sim_time
                        );
                    }
                    st.impacts.push(Impact { x: at[0] as f32, y: at[1] as f32, ground, born: tick, size: if d == 0 { 1.0 } else { 0.55 } });
                    downs = d + 1;
                }
                if p[2] == 0 && (p[0], p[1]) != (now[0], now[1]) && gl.flags & flag::BALL_MOVING == 0 {
                    downs = 0;
                }
            }
            // a ball at rest again is ready for its next stroke
            if now[2] == 0 && gl.flags & flag::BALL_MOVING == 0 {
                downs = 0;
            }
            st.last[i] = Some((now, downs));
        }
    }

    /// A ball's point in the world from its place in the simulation's units and its height, as draw_balls draws it with
    /// smooth motion (the height without whole-pixel steps).
    fn fx_ball_point(&self, x: f32, y: f32, h: f32) -> [f32; 3] {
        let (x0, z0) = self.units_to_world(0, 0);
        let k = sg_core::terrain::TILE_SIZE / sg_core::staff::UNIT as f32;
        let (bx, bz) = (x0 + x * k, z0 + y * k);
        let gy = self.terrain.height_at(bx, bz);
        let px = self.upp * crate::ui::view_scale(self.draw_w, self.draw_h);
        let lift = sg_core::course::HEIGHT_STEP_PX as f32 * self.exe_zoom() * h / 5.0 / 16.0 * px;
        let mv = self.mv;
        [bx + mv[1] * lift, gy + mv[5] * lift, bz + mv[9] * lift]
    }

    /// The trails, under the balls: a ribbon through the ball's last places in the air, widest and brightest at the ball.
    pub(crate) fn fx_ball_trails(&mut self, g: &mut Gfx, u: &Uniforms) {
        let s = crate::fx::settings();
        if !s.ball_trail {
            return;
        }
        let now = self.game_tick as f32 + self.tick_fraction();
        let mv = self.mv;
        let (rx, ry, rz, ux, uy, uz) = (mv[0], mv[4], mv[8], mv[1], mv[5], mv[9]);
        let vscale = crate::ui::view_scale(self.draw_w, self.draw_h);
        let half_px = 1.3 * (self.exe_zoom() / 4.0).max(0.5) * vscale * self.upp;
        let mut v = Vec::new();
        for i in 0..self.fx.balls.trails.len() {
            let pts = &self.fx.balls.trails[i];
            if pts.is_empty() {
                continue;
            }
            let mut line: Vec<([f32; 3], f32)> = pts.iter().map(|&(t, p)| (self.fx_ball_point(p[0], p[1], p[2]), now - t as f32)).collect();
            // up to the ball as it is drawn now
            let gl = &self.club.g[i];
            if gl.bz > 0 && gl.flags & flag::PENALTY == 0 {
                let (x, y, h) = self.shown_ball(i).unwrap_or((gl.bx as f32, gl.by as f32, gl.bz as f32));
                line.push((self.fx_ball_point(x, y, h), 0.0));
            }
            for w in line.windows(2) {
                let ((a, ta), (b, tb)) = (w[0], w[1]);
                let d = [b[0] - a[0], b[1] - a[1], b[2] - a[2]];
                let (sx, sy) = (rx * d[0] + ry * d[1] + rz * d[2], ux * d[0] + uy * d[1] + uz * d[2]);
                let l = (sx * sx + sy * sy).sqrt();
                if l < 1e-4 {
                    continue;
                }
                let fade = |age: f32| (1.0 - age / TRAIL_TICKS).clamp(0.0, 1.0);
                let (fa, fb) = (fade(ta), fade(tb));
                let side = |p: [f32; 3], f: f32, sgn: f32| {
                    let (px, py) = (-sy / l * half_px * f * sgn, sx / l * half_px * f * sgn);
                    Vert::new(p[0] + rx * px + ux * py, p[1] + ry * px + uy * py, p[2] + rz * px + uz * py, 0.0, 0.0).col([
                        1.0,
                        1.0,
                        0.97,
                        0.75 * f * f,
                    ])
                };
                let (a0, a1, b0, b1) = (side(a, fa, -1.0), side(a, fa, 1.0), side(b, fb, -1.0), side(b, fb, 1.0));
                v.extend_from_slice(&[a0, b0, b1, a0, b1, a1]);
            }
        }
        g.tris(Mode::Flat, None, u, &v);
    }

    /// The impacts: sand thrown up in a puff, a ring spreading on water with a few drops, a flick of grass.
    pub(crate) fn fx_ball_impacts(&mut self, g: &mut Gfx, u: &Uniforms) {
        let s = crate::fx::settings();
        if !s.ball_impacts || self.fx.balls.impacts.is_empty() {
            return;
        }
        let now = self.game_tick as f32 + self.tick_fraction();
        let mv = self.mv;
        let (rx, ry, rz, ux, uy, uz) = (mv[0], mv[4], mv[8], mv[1], mv[5], mv[9]);
        let vscale = crate::ui::view_scale(self.draw_w, self.draw_h);
        let px = self.upp * vscale; // world units per 800 x 600 pixel
        let z = self.exe_zoom() / 4.0; // 1 at the closest zoom
        let dot = crate::fx::gpu(g).dot;
        let tint = self.fx_sprite_tint(0.0, 0.0, false).map(|t| [t[0], t[1], t[2]]).unwrap_or([1.0; 3]);
        let mut dots: Vec<Vert> = Vec::new();
        let mut rings: Vec<(Vec<[f32; 3]>, [f32; 4])> = Vec::new();
        // a soft dot `r` pixels round, `up` pixels above a world point, `side` pixels to the right
        let mut blob = |c: [f32; 3], side: f32, up: f32, r: f32, col: [f32; 4]| {
            let p = |cx: f32, cy: f32, tu: f32, tv: f32| {
                let (cx, cy) = ((side + cx) * px, (up + cy) * px);
                Vert::new(c[0] + rx * cx + ux * cy, c[1] + ry * cx + uy * cy, c[2] + rz * cx + uz * cy, tu, tv).col(col)
            };
            let (a, b, cc, d) = (p(-r, r, 0.0, 0.0), p(r, r, 1.0, 0.0), p(r, -r, 1.0, 1.0), p(-r, -r, 0.0, 1.0));
            dots.extend_from_slice(&[a, b, cc, a, cc, d]);
        };
        for im in self.fx.balls.impacts.clone() {
            let age = now - im.born as f32;
            let c = self.fx_ball_point(im.x, im.y, 0.0);
            let seed = im.born ^ (im.x as u32).wrapping_mul(31) ^ (im.y as u32).wrapping_mul(17);
            let k = im.size * z;
            match im.ground {
                Ground::Sand => {
                    // a puff: grains thrown up and out, slowing, as a cloud that spreads and thins
                    let a = age / 11.0;
                    if a >= 1.0 {
                        continue;
                    }
                    // the pale dust cloud
                    for j in 0..7 {
                        let (h1, h2) = (hash(seed, j), hash(seed, j + 50));
                        let dir = (h1 - 0.5) * 2.6;
                        let reach = (1.0 - (-a * 4.0).exp()) * (6.0 + 9.0 * h2) * k;
                        let rise = (1.0 - (1.0 - a).powi(2)) * (5.0 + 7.0 * h2) * k - a * a * 3.0 * k;
                        let r = (3.5 + 8.0 * a) * k.max(0.4);
                        let al = 0.8 * (1.0 - a).powf(1.3);
                        let col = [0.93 * tint[0], 0.84 * tint[1], 0.66 * tint[2], al];
                        blob(c, dir.sin() * reach, rise.max(0.0), r, col);
                    }
                    // and darker grains thrown out of it, falling back
                    let t = (age / 8.0).min(1.0);
                    for j in 0..10 {
                        let (h1, h2) = (hash(seed, j + 100), hash(seed, j + 150));
                        let side = (h1 - 0.5) * 22.0 * k * t;
                        let up = t * (1.0 - t) * 4.0 * (4.0 + 8.0 * h2) * k;
                        let col = [0.62 * tint[0], 0.50 * tint[1], 0.34 * tint[2], 0.9 * (1.0 - t)];
                        blob(c, side, up, 0.9 * k.max(0.5), col);
                    }
                }
                Ground::Water => {
                    let a = age / 14.0;
                    if a >= 1.0 {
                        continue;
                    }
                    // two rings on the water, opening as they fade
                    let (x0, z0) = self.units_to_world(0, 0);
                    let kk = sg_core::terrain::TILE_SIZE / sg_core::staff::UNIT as f32;
                    let (wx, wz) = (x0 + im.x * kk, z0 + im.y * kk);
                    let wy = self.terrain.height_at(wx, wz) + 0.5;
                    for (j, lag) in [0.0f32, 0.25].iter().enumerate() {
                        let e = ((a - lag) / (1.0 - lag)).clamp(0.0, 1.0);
                        if e <= 0.0 || e >= 1.0 {
                            continue;
                        }
                        let rad = (0.06 + 0.32 * e.sqrt()) * sg_core::terrain::TILE_SIZE * im.size;
                        let pts: Vec<[f32; 3]> = (0..24)
                            .map(|q| {
                                let t = q as f32 / 24.0 * std::f32::consts::TAU;
                                [wx + rad * t.cos(), wy, wz + rad * t.sin()]
                            })
                            .collect();
                        rings.push((pts, [0.92 * tint[0], 0.96 * tint[1], 1.0 * tint[2], 0.6 * (1.0 - e) * (1.0 - 0.3 * j as f32)]));
                    }
                    // and drops thrown up in an arc
                    for j in 0..6 {
                        let (h1, h2) = (hash(seed, j), hash(seed, j + 50));
                        let t = a * 14.0 / 8.0;
                        if t >= 1.0 {
                            continue;
                        }
                        let side = (h1 - 0.5) * 12.0 * k;
                        let up = (t * (1.0 - t) * 4.0) * (5.0 + 6.0 * h2) * k;
                        blob(c, side * t, up, 1.1 * k.max(0.5), [0.9 * tint[0], 0.95 * tint[1], 1.0 * tint[2], 0.7 * (1.0 - t)]);
                    }
                }
                Ground::Grass => {
                    // a few blades flicked up and falling back
                    let t = age / 7.0;
                    if t >= 1.0 {
                        continue;
                    }
                    for j in 0..8 {
                        let (h1, h2) = (hash(seed, j), hash(seed, j + 50));
                        let side = (h1 - 0.5) * 14.0 * k * t;
                        let up = t * (1.0 - t) * 4.0 * (3.0 + 5.0 * h2) * k;
                        let col = [(0.30 + 0.15 * h2) * tint[0], (0.55 + 0.15 * h1) * tint[1], 0.20 * tint[2], 0.85 * (1.0 - t * t)];
                        blob(c, side, up, 0.9 * k.max(0.5), col);
                    }
                }
            }
        }
        g.tris(Mode::Flat, Some(dot), u, &dots);
        for (pts, col) in rings {
            self.draw_lines(g, u, &pts, true, 0.6 * vscale * z.max(0.5), col);
        }
    }
}
