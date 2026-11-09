//! Enhanced graphics' tournament crowds (docs/ENHANCED.md, "Tournament atmosphere"): during a tournament or a championship,
//! spectators stand round the green and along the fairway of each hole being played, watching the ball while one is in
//! the air there and the green otherwise, now and then cheering. They are the game's own golfer sprites in the usual random colours, drawn only: nothing in the simulation
//! knows about them, golfers walk through them as through scenery, and what varies comes from hashes of the hole, never
//! from the game's random generator.
use crate::app::{App, Prop, GOLFER_BODIES};
use sg_core::terrain::*;

/// The golfer clips spectators use (bases of app::GOLFER_CLIPS): standing and cheering.
const SQ: i32 = 0x0a;
const HAPPY: i32 = 0x1e;
/// Spectators round a green and along a fairway.
const ROUND_GREEN: u32 = 14;
const ALONG_FAIRWAY: u32 = 8;
/// Seconds between a hole's cheers, and how long one lasts.
const CHEER_EVERY: f32 = 22.0;
const CHEER_FOR: f32 = 2.5;

fn hash(a: u32, b: u32) -> f32 {
    let mut h = a.wrapping_mul(0x9e37_79b9) ^ b.wrapping_mul(0x85eb_ca6b);
    h ^= h >> 15;
    h = h.wrapping_mul(0x2c1b_3c6d);
    h ^= h >> 12;
    (h >> 8) as f32 / (1u32 << 24) as f32
}

const DX: [f32; 8] = [0.0, 1.0, 1.0, 1.0, 0.0, -1.0, -1.0, -1.0];
const DY: [f32; 8] = [-1.0, -1.0, 0.0, 1.0, 1.0, 1.0, 0.0, -1.0];

impl App {
    /// The spectators to draw this tick (wild_ui.rs adds them with the other moving props).
    pub(crate) fn fx_spectators(&mut self) -> Vec<Prop> {
        let tour = self.club.game & sg_core::golfer::game::TOURNAMENT != 0 || self.club.championship();
        if !tour {
            return Vec::new();
        }
        let mut holes: Vec<usize> =
            self.club.g.iter().take(sg_core::golfer::SLOTS).filter(|g| (1..19).contains(&g.hole)).map(|g| g.hole as usize).collect();
        holes.sort_unstable();
        holes.dedup();
        let t = self.fx.time;
        let tick = self.game_tick as i32;
        let seed = self.fx_seed();
        let mut out = Vec::new();
        for h in holes {
            let hr = &self.club.holes[h];
            if hr.par == 0 {
                continue;
            }
            let (pin, tee) = (hr.pin, hr.back);
            // what the crowd watches: a ball in the air on this hole, else the green (in tiles)
            let ball = self
                .club
                .g
                .iter()
                .take(sg_core::golfer::SLOTS)
                .find(|g| g.hole == h as i32 && g.bz > 0)
                .map(|g| (g.bx as f32 / 1024.0, g.by as f32 / 1024.0));
            let look = ball.unwrap_or((pin.0 as f32 + 0.5, pin.1 as f32 + 0.5));
            let hs = seed ^ (h as u32).wrapping_mul(0x51ed);
            let cheering = (t + hash(hs, 99) * CHEER_EVERY).rem_euclid(CHEER_EVERY) < CHEER_FOR;
            // places in tiles (fractions allowed): a ring round the green, then both sides of the fairway
            let mut spots: Vec<(f32, f32)> = Vec::new();
            for i in 0..ROUND_GREEN {
                let a = (i as f32 + hash(hs, i) * 0.6) / ROUND_GREEN as f32 * std::f32::consts::TAU;
                let r = 2.0 + 0.7 * hash(hs, 100 + i);
                spots.push((pin.0 as f32 + 0.5 + r * a.cos(), pin.1 as f32 + 0.5 + r * a.sin()));
            }
            let (fx, fy) = ((pin.0 - tee.0) as f32, (pin.1 - tee.1) as f32);
            let len = (fx * fx + fy * fy).sqrt().max(1.0);
            let (nx, ny) = (-fy / len, fx / len);
            for i in 0..ALONG_FAIRWAY {
                let along = 0.3 + 0.45 * (i / 2) as f32 / (ALONG_FAIRWAY / 2) as f32 + 0.05 * hash(hs, 200 + i);
                let side = if i % 2 == 0 { 1.0 } else { -1.0 } * (2.8 + 0.8 * hash(hs, 300 + i));
                spots.push((tee.0 as f32 + 0.5 + fx * along + nx * side, tee.1 as f32 + 0.5 + fy * along + ny * side));
            }
            for (i, &(sx, sy)) in spots.iter().enumerate() {
                let (tx, ty) = (sx.floor() as i32, sy.floor() as i32);
                if tx < 0 || ty < 0 || tx >= self.terrain.w || ty >= self.terrain.h {
                    continue;
                }
                // only where people would stand: off the water, the sand, the greens, the tees and the buildings, on the
                // club's own land
                let ty_ = self.terrain.ty[self.terrain.tile_index(tx, ty)];
                let bad = matches!(
                    ty_,
                    TT_WATER_SHALLOW
                        | TT_WATER_MIDDLE
                        | TT_WATER_DEEP
                        | TT_WATER_SHALLOW_DESERT
                        | TT_SAND
                        | TT_GRASSY_SAND
                        | TT_POT_SAND_BUNKER
                        | TT_SAND_BUNKER1
                        | TT_PUTTING_GREEN
                        | TT_TRICKY_GREEN
                        | TT_TEE
                        | TT_BUILDING
                        | TT_WOODS
                        | TT_CLIFF
                        | TT_RAVINE
                );
                let out_of_land = self.land.as_ref().is_some_and(|l| l.ty.get((ty * l.n + tx) as usize) == Some(&sg_core::land::T_OUT));
                if bad || out_of_land {
                    continue;
                }
                let k = hs ^ (i as u32).wrapping_mul(0x2545);
                let bodies: Vec<usize> = (0..GOLFER_BODIES.len()).filter(|&b| !GOLFER_BODIES[b].is_empty()).collect();
                let body = bodies[(hash(k, 1) * bodies.len() as f32) as usize % bodies.len()];
                let r = hash(k, 2);
                // standing, or cheering with the hole's crowd (the golfers' fidget swings a club: not for spectators)
                let clip = if cheering && r < 0.8 { HAPPY } else { SQ };
                let (Some(b), shadow) = self.golfer_clip(clip + body as i32) else { continue };
                let n = self.frames_of(Some(b));
                let frame = if clip == SQ { 0 } else { (tick / 2 + (hash(k, 3) * 40.0) as i32).rem_euclid(n.max(1)) };
                // facing what the crowd watches: the eight-view clips (the original's) take the exact heading, the four-view
                // ones (the free set's) the nearest quarter, whose views run the other way round from the compass
                let (vx, vy) = (look.0 - sx, look.1 - sy);
                let heading = vy.atan2(vx).to_degrees();
                let d0 = (0..8).max_by(|&a, &c| (DX[a] * vx + DY[a] * vy).total_cmp(&(DX[c] * vx + DY[c] * vy))).unwrap_or(0);
                let d = (d0 + 4) % 8;
                let id = (k >> 4) % 997;
                let female = body >= 5;
                let skin = (id % 4) as u8;
                let outfit = sg_core::bodies::Outfit {
                    female,
                    shirt: ((id * 3) % 10) as u8,
                    pants: ((id * 7) % 10) as u8,
                    skin,
                    hat: if female { 9 } else { (id % 3) as u8 },
                    hair: ((id * 5) % 6) as u8,
                    alt_skin: skin,
                };
                let (x, z) = self.units_to_world((sx * 1024.0) as i32, (sy * 1024.0) as i32);
                out.push(Prop {
                    x,
                    z,
                    body: Some(b),
                    shadow,
                    frame,
                    heading,
                    facing: -((d as i32 + 2 + (d as i32 & 1)) / 2),
                    outfit: Some(outfit),
                    track: 0x5200_0000 | (h as u32) << 8 | i as u32,
                    ..Default::default()
                });
            }
        }
        out
    }
}
