//! Enhanced graphics' water (docs/ENHANCED.md, "Water"), the port's own: the water shader's newer options (truer colour
//! with depth, painterly waves following the wind, the shoreline, caustics, rain rings, ice; fx_glsl.rs `water_fs`) get
//! their uniforms here, and the life on and round the water is drawn here as a few crisp pixels at the art's own pixel
//! size, as fx_life.rs draws its butterflies: reeds at the edges of ponds and lakes, lily pads in their sheltered shallows,
//! ducks and swans gliding on the larger lakes, now and then a fish jumping, and spray round the original's fountains and
//! at the foot of its waterfalls. Drawing only: nothing touches the simulation, and what varies comes from hashes of the
//! course, the tile and the effects' clock, never from the game's random generator, so nothing flickers from frame to frame.
use crate::app::App;
use crate::fx_life::{hash, Dots};
use crate::gfx::{Gfx, Mode, Uniforms};
use sg_core::terrain::{Terrain, TILE_SIZE};

/// How far the light map's signed distance to the shore reaches each way, in tiles (fx.rs `build_lightmap`; the shaders
/// decode it with the same 3).
pub const SHORE_TILES: f32 = 3.0;
/// A body of water this many tiles or more is the sea (or a great lake), and so is one of a third of that running off the
/// map: no lily pads, reeds or ducks on it (a river running off the map is not the sea).
const SEA_TILES: u32 = 450;
/// Ducks and swans only on lakes of at least this many tiles (not on small ponds).
const DUCK_TILES: u32 = 24;
/// Seconds between a lake's chances of a fish jumping, and how long a jump and its ring last.
const FISH_EVERY: f32 = 7.0;
const FISH_JUMP: f32 = 0.9;
const FISH_RING: f32 = 1.6;

/// One body of water: its tiles, whether it is the sea, and its open water (where ducks swim): tiles at least two steps from
/// the land and from the property's edge.
#[derive(Clone, Debug, Default)]
pub struct Body {
    pub tiles: Vec<(i32, i32)>,
    pub sea: bool,
    pub open: Vec<(i32, i32)>,
}

/// The course's water, per tile: which body it belongs to and how many tiles it is from the land.
#[derive(Clone, Debug, Default)]
pub struct Ponds {
    pub w: i32,
    pub h: i32,
    /// Per tile: the body's index, or u32::MAX off the water.
    pub body: Vec<u32>,
    /// Per tile: steps (8-connected) to the nearest land tile, 0 on the land; 255 when none is near.
    pub dist: Vec<u8>,
    pub bodies: Vec<Body>,
}

impl Ponds {
    pub fn body_at(&self, x: i32, y: i32) -> Option<&Body> {
        if x < 0 || y < 0 || x >= self.w || y >= self.h {
            return None;
        }
        self.bodies.get(self.body[(y * self.w + x) as usize] as usize)
    }
    pub fn dist_at(&self, x: i32, y: i32) -> u8 {
        if x < 0 || y < 0 || x >= self.w || y >= self.h {
            return 0;
        }
        self.dist[(y * self.w + x) as usize]
    }
}

const N8: [(i32, i32); 8] = [(1, 0), (-1, 0), (0, 1), (0, -1), (1, 1), (-1, 1), (1, -1), (-1, -1)];

/// The bodies of water of a course (4-connected water tiles) and each tile's distance from the land. The land outside the
/// property (type 20) is not land: water running into it is the sea.
pub fn build_ponds(t: &Terrain) -> Ponds {
    let (w, h) = (t.w.max(0), t.h.max(0));
    let n = (w * h) as usize;
    let water_ty = sg_core::course::t::WATER as i32;
    let out_ty = sg_core::course::t::OUT as i32;
    let is_water = |x: i32, y: i32| t.type_at(x, y) == water_ty;
    let mut dist = vec![255u8; n];
    let mut queue = std::collections::VecDeque::new();
    for y in 0..h {
        for x in 0..w {
            let ty = t.type_at(x, y);
            if ty != water_ty && ty != out_ty {
                dist[(y * w + x) as usize] = 0;
                queue.push_back((x, y));
            }
        }
    }
    while let Some((x, y)) = queue.pop_front() {
        let d = dist[(y * w + x) as usize];
        if d >= 254 {
            continue;
        }
        for (dx, dy) in N8 {
            let (nx, ny) = (x + dx, y + dy);
            if nx < 0 || ny < 0 || nx >= w || ny >= h {
                continue;
            }
            let k = (ny * w + nx) as usize;
            if dist[k] == 255 && is_water(nx, ny) {
                dist[k] = d + 1;
                queue.push_back((nx, ny));
            }
        }
    }
    let mut body = vec![u32::MAX; n];
    let mut bodies = Vec::new();
    for y0 in 0..h {
        for x0 in 0..w {
            if !is_water(x0, y0) || body[(y0 * w + x0) as usize] != u32::MAX {
                continue;
            }
            let id = bodies.len() as u32;
            let mut b = Body::default();
            let mut stack = vec![(x0, y0)];
            body[(y0 * w + x0) as usize] = id;
            let mut off_map = false;
            while let Some((x, y)) = stack.pop() {
                b.tiles.push((x, y));
                for (dx, dy) in [(1, 0), (-1, 0), (0, 1), (0, -1)] {
                    let (nx, ny) = (x + dx, y + dy);
                    if nx < 0 || ny < 0 || nx >= w || ny >= h || t.type_at(nx, ny) == out_ty {
                        off_map = true;
                        continue;
                    }
                    let k = (ny * w + nx) as usize;
                    if is_water(nx, ny) && body[k] == u32::MAX {
                        body[k] = id;
                        stack.push((nx, ny));
                    }
                }
            }
            let size = b.tiles.len() as u32;
            b.sea = size >= SEA_TILES || off_map && size >= SEA_TILES / 3;
            b.tiles.sort_unstable();
            let inner =
                |x: i32, y: i32| (-2..=2).all(|dy| (-2..=2).all(|dx| t.inside(x + dx, y + dy) && t.type_at(x + dx, y + dy) != out_ty));
            b.open = b.tiles.iter().copied().filter(|&(x, y)| dist[(y * w + x) as usize] >= 2 && inner(x, y)).collect();
            bodies.push(b);
        }
    }
    Ponds { w, h, body, dist, bodies }
}

fn smoothstep(a: f32, b: f32, x: f32) -> f32 {
    let t = ((x - a) / (b - a)).clamp(0.0, 1.0);
    t * t * (3.0 - 2.0 * t)
}

/// How strong the painterly waves are (0.08 glassy .. 1 choppy): with the day's wind (a middling breeze without the wind
/// option), calmer at dawn and dusk when the wind drops with the low sun (`phase`: the time of day, None without it), and
/// rougher in rain.
pub fn wave_strength(wind: Option<f32>, phase: Option<f32>, rain: f32) -> f32 {
    let base = wind.map(|k| 0.12 + 0.8 * k.clamp(0.0, 1.2)).unwrap_or(0.45);
    let calm = phase
        .map(|p| {
            let morning = if p > 0.95 { 0.0 } else { smoothstep(0.0, 0.24, p) };
            (0.3 + 0.7 * morning) * (1.0 - 0.45 * smoothstep(0.84, 0.95, p))
        })
        .unwrap_or(1.0);
    (base * calm + 0.25 * rain).clamp(0.08, 1.0)
}

/// The water shader's uniforms for the newer options (fx_glsl.rs `water_fs`).
pub(crate) struct WaterUniforms {
    /// (rain rings, waves, ice, caustics), each 0 while off.
    pub fx1: [f32; 4],
    /// The wind's heading (radians, world x towards z).
    pub heading: f32,
    /// The waves' size in world units, negative while the truer colour is off.
    pub size: f32,
    /// The distance haze the truer colour makes up for.
    pub haze: f32,
}

impl App {
    /// The course's bodies of water, after its ground changed (fx.rs `fx_terrain_built`).
    pub(crate) fn fx_ponds(&mut self) {
        self.fx.ponds = build_ponds(&self.terrain);
        if std::env::var_os("SG_FX_LOG").is_some() {
            // where to look for the spray and the life in stills: the bodies of water, the fountains and the waterfalls
            let n = sg_core::course::n();
            let depth = &self.water_depth;
            let (mut fountains, mut falls) = (Vec::new(), Vec::new());
            for b in 0..n {
                for a in 0..n {
                    let i = sg_core::course::idx(a, b);
                    if self.course.ty[i] != sg_core::course::t::WATER {
                        continue;
                    }
                    let fl = self.course.flags[i];
                    if fl & sg_core::course::f::SCENIC != 0 && fl & sg_core::course::f::PATH == 0 && depth.get(i) == Some(&0) {
                        fountains.push((a, b));
                    }
                    if !sg_core::decor::waterfalls(&self.course, a, b, 0, 4).is_empty() {
                        falls.push((a, b));
                    }
                }
            }
            let bodies: Vec<(usize, bool)> = self.fx.ponds.bodies.iter().map(|b| (b.tiles.len(), b.sea)).collect();
            println!("fx: water bodies (tiles, sea) {bodies:?}; fountains at {fountains:?}; waterfalls at {falls:?}");
        }
    }

    /// This frame's uniforms for the water's newer options (`upp`: world units per pixel of the target, as render.rs has it).
    pub(crate) fn fx_water_uniforms(&self, a: &crate::fx_light::Atmos, upp: f32) -> WaterUniforms {
        let s = crate::fx::settings();
        let phase = s.time_of_day.then(|| self.fx_day().1);
        // the waves' size: about 28 of the art's pixels at any zoom, never under half a tile
        let art_px = 2.0 / self.view_zoom().max(1e-3);
        let size = (28.0 * art_px).max(45.0);
        let sunny = (a.shadow * (1.0 - 1.6 * a.rain).max(0.0) * (1.0 - a.night)).clamp(0.0, 1.0);
        WaterUniforms {
            fx1: [
                if s.rain_rings { a.rain } else { 0.0 },
                if s.waves { wave_strength(s.wind.then_some(a.wind.k), phase, a.rain) } else { 0.0 },
                if s.ice { a.ice } else { 0.0 },
                if s.caustics { sunny } else { 0.0 },
            ],
            heading: if a.wind.k > 0.0 { a.wind.dir[1].atan2(a.wind.dir[0]) } else { 0.6 },
            size: if s.water_colour { size } else { -size },
            haze: if s.water_colour { crate::fx_light::haze_strength(self, a, upp) } else { 0.0 },
        }
    }

    /// The life on and round the water, and the spray of the fountains and waterfalls: on the ground, under the sprites.
    pub(crate) fn fx_water_life(&mut self, g: &mut Gfx, u: &Uniforms) {
        let s = crate::fx::settings();
        if !(s.water_life || s.spray) || self.land.is_none() || self.exe_zoom() < 2.0 || self.fx.ponds.bodies.is_empty() {
            return;
        }
        let a = self.fx.atmos;
        let t = self.fx.time;
        let mv = self.mv;
        let vscale = crate::ui::view_scale(self.draw_w, self.draw_h);
        // sizes in the art's pixels as the view draws them (the exe's zoom 4 draws them one to one; Enhanced's closer
        // smooth zoom magnifies them, whole pixels from there on)
        let z = self.view_zoom() / crate::app::ZOOM_UNIT / 4.0;
        let z = if z >= 1.0 { z.round() } else { z };
        let tint = self.fx_sprite_tint(0.0, 0.0, false).map(|t| [t[0], t[1], t[2]]).unwrap_or([1.0; 3]);
        let tc = |c: [f32; 3], al: f32| [c[0] * tint[0], c[1] * tint[1], c[2] * tint[2], al];
        let mut d = Dots { v: Vec::new(), axes: [mv[0], mv[4], mv[8], mv[1], mv[5], mv[9]], px: self.upp * vscale };
        let mut rings: Vec<([f32; 3], f32, f32)> = Vec::new();
        let seed = self.fx_seed();
        let theme = self.exe_theme().min(3);
        let month = self.fx_month();
        let ice = if s.ice { a.ice } else { 0.0 };
        let daylight = (1.0 - a.night * 1.5).clamp(0.0, 1.0);
        let water_ty = sg_core::course::t::WATER as i32;
        let (cx, cy) = self.terrain.tile_of(self.cam_x, self.cam_z);
        let reach = ((self.draw_w.max(self.draw_h) * self.upp / TILE_SIZE) * 0.75).ceil() as i32 + 1;
        let (x0, x1, y0, y1) =
            ((cx - reach).max(0), (cx + reach).min(self.terrain.w), (cy - reach).max(0), (cy + reach).min(self.terrain.h));
        let ponds = std::mem::take(&mut self.fx.ponds);
        let level = |app: &App, x: f32, z: f32| [x, app.terrain.height_at(x, z), z];
        let wind = a.wind;

        if s.water_life {
            for ty in y0..y1 {
                for tx in x0..x1 {
                    if self.terrain.type_at(tx, ty) != water_ty {
                        continue;
                    }
                    let Some(body) = ponds.body_at(tx, ty) else { continue };
                    if body.sea {
                        continue;
                    }
                    let hs = ((tx as u32) << 16 | ty as u32) ^ seed;
                    let dist = ponds.dist_at(tx, ty);
                    let (mx, mz) = self.terrain.tile_centre(tx, ty);
                    if dist == 1 && hash(hs, 1) < 0.34 {
                        // reeds at the edge: a clump on the bank side of a shore tile, swaying with the wind
                        let Some(&(lx, ly)) =
                            N8[..4].iter().find(|&&(dx, dy)| ponds.dist_at(tx + dx, ty + dy) == 0 && self.terrain.inside(tx + dx, ty + dy))
                        else {
                            continue;
                        };
                        // at the water's edge, so the clump stands in the shallows against the bank
                        let off = 0.3 + 0.1 * hash(hs, 2);
                        let side = (hash(hs, 3) - 0.5) * 0.6;
                        let (x, zz) =
                            (mx + (lx as f32 * off + ly as f32 * side) * TILE_SIZE, mz + (ly as f32 * off + lx as f32 * side) * TILE_SIZE);
                        let c = level(self, x, zz);
                        let n = 5 + (hash(hs, 4) * 4.0) as u32;
                        let (dark, light) = match theme {
                            1 => ([0.36, 0.42, 0.18], [0.55, 0.60, 0.26]),
                            3 => ([0.40, 0.44, 0.24], [0.58, 0.60, 0.34]),
                            _ => ([0.20, 0.38, 0.14], [0.38, 0.56, 0.20]),
                        };
                        let autumn = a.season.autumn;
                        let tone = |k: f32| {
                            let b = [
                                dark[0] + (light[0] - dark[0]) * k,
                                dark[1] + (light[1] - dark[1]) * k,
                                dark[2] + (light[2] - dark[2]) * k,
                            ];
                            tc([b[0] + 0.25 * autumn * (0.75 - b[0]), b[1] + 0.1 * autumn, b[2]], 0.95)
                        };
                        // the clump's own little shadow on the water
                        d.pixel(c, -(n as f32 * 0.5).round() - 1.0, -1.0, n as f32 + 2.0, 1.0, z, tc([0.05, 0.16, 0.20], 0.35));
                        // a tuft of stems fanning out from one root, the middle ones tallest, each darker at the foot and
                        // lit towards the tip (upright bars in alternating colours read as a fence)
                        let half = (n as f32 - 1.0) * 0.5;
                        for b in 0..n {
                            let rel = (b as f32 - half) / half.max(1.0);
                            let root = (rel * half * 0.5).round();
                            let h = ((5.0 + 4.0 * (1.0 - rel.abs()) + 2.0 * hash(hs, 20 + b)).round() * if ice > 0.0 { 0.7 } else { 1.0 })
                                .max(3.0);
                            let sway = wind.k * wind.screen_x * 1.4 + 0.7 * (t * 1.9 + hash(hs, 30 + b) * 6.3).sin() * wind.k.max(0.15);
                            let lean = rel * h * 0.4 + sway;
                            let lit = hash(hs, 40 + b) * 0.3 + if b % 3 == 1 { 0.4 } else { 0.1 };
                            for y in 0..h as u32 {
                                let u = y as f32 / h;
                                // the stem bends more towards its tip
                                d.pixel(c, (root + lean * u * u).round(), y as f32, 1.0, 1.0, z, tone((lit + u * 0.6).min(1.0)));
                            }
                            let top = (root + lean).round();
                            // a bulrush's brown head on a tall stem in summer and autumn (papyrus tufts in the desert)
                            if h >= 8.0 && month >= 6.0 && theme != 1 && b % 2 == 0 {
                                d.pixel(c, top, h - 2.0, 1.0, 2.0, z, tc([0.36, 0.22, 0.12], 1.0));
                            } else if h >= 8.0 && theme == 1 {
                                d.pixel(c, top - 1.0, h, 3.0, 1.0, z, tc([0.56, 0.62, 0.30], 1.0));
                            }
                        }
                        // the leaves at the foot, arching out to both sides
                        for (k, dir) in [(0u32, -1.0f32), (1, 1.0)] {
                            let x0 = dir * (half * 0.5).round();
                            d.pixel(c, x0 + dir, 2.0, 1.0, 1.0, z, tone(0.5));
                            d.pixel(c, x0 + 2.0 * dir, 2.0 + (hash(hs, 60 + k) < 0.5) as i32 as f32, 1.0, 1.0, z, tone(0.7));
                            d.pixel(c, x0 + 3.0 * dir, 1.0, 1.0, 1.0, z, tone(0.6));
                        }
                    } else if (1..=2).contains(&dist) && hash(hs, 5) < 0.16 && (4.3..10.6).contains(&month) && ice <= 0.0 {
                        // lily pads in the sheltered shallows, flowering in high summer
                        let k = 2 + (hash(hs, 6) * 3.0) as u32;
                        for p in 0..k {
                            let (ox, oz) = ((hash(hs, 40 + p) - 0.5) * 0.7, (hash(hs, 50 + p) - 0.5) * 0.7);
                            let c = level(self, mx + ox * TILE_SIZE, mz + oz * TILE_SIZE);
                            let big = hash(hs, 60 + p) < 0.5;
                            let leaf = tc([0.16, 0.40, 0.15], 1.0);
                            let lit = tc([0.32, 0.58, 0.22], 1.0);
                            // a flat leaf seen at a slant: a wide row over a narrower one, a notch cut in its top
                            let w = if big { 4.0 } else { 3.0 };
                            d.pixel(c, -1.0, -1.0, w, 1.0, z, tc([0.04, 0.14, 0.18], 0.35));
                            d.pixel(c, -1.0, 0.0, w, 1.0, z, leaf);
                            d.pixel(c, -1.0 + 1.0, 1.0, w - 2.0, 1.0, z, leaf);
                            d.pixel(c, -1.0, 1.0, 1.0, 1.0, z, lit);
                            if (6.0..8.8).contains(&month) && hash(hs, 70 + p) < 0.35 {
                                let petal = if hash(hs, 80 + p) < 0.5 { [1.0, 0.98, 0.95] } else { [0.98, 0.70, 0.80] };
                                d.pixel(c, w - 2.0, 1.0, 1.0, 1.0, z, tc(petal, 1.0));
                                d.pixel(c, w - 2.0, 2.0, 1.0, 1.0, z, tc([1.0, 0.86, 0.30], 1.0));
                            }
                        }
                    }
                }
            }
            // ducks and swans gliding round the open water of the larger lakes (not in the desert's oases), and fish jumping
            for (bi, body) in ponds.bodies.iter().enumerate() {
                if body.sea || body.tiles.is_empty() {
                    continue;
                }
                let bs = seed ^ (bi as u32).wrapping_mul(0x9e37_79b9) ^ body.tiles[0].0 as u32 ^ (body.tiles[0].1 as u32) << 12;
                let on_screen = |&(x, y): &(i32, i32)| x >= x0 && x < x1 && y >= y0 && y < y1;
                if theme != 1 && body.tiles.len() as u32 >= DUCK_TILES && !body.open.is_empty() && daylight > 0.2 {
                    let groups = (body.tiles.len() / 40).clamp(1, 4);
                    for gi in 0..groups as u32 {
                        let home = body.open[(hash(bs, 100 + gi) * body.open.len() as f32) as usize % body.open.len()];
                        if !on_screen(&home) {
                            continue;
                        }
                        // away from any ice at the edges
                        if ice > 0.0 && (ponds.dist_at(home.0, home.1) as f32) < ice * 1.4 + 2.0 {
                            continue;
                        }
                        let swan = theme != 2 && hash(bs, 110 + gi) < 0.3;
                        // a mother with ducklings in May and June
                        let young = !swan && (5.0..7.0).contains(&month) && hash(bs, 120 + gi) < 0.6;
                        let n = if swan {
                            1 + (hash(bs, 130 + gi) < 0.5) as u32
                        } else if young {
                            4
                        } else {
                            2 + (hash(bs, 130 + gi) < 0.4) as u32
                        };
                        let (hx, hz) = self.terrain.tile_centre(home.0, home.1);
                        let r = TILE_SIZE * (0.35 + 0.2 * hash(bs, 140 + gi));
                        let dir = if hash(bs, 150 + gi) < 0.5 { 1.0 } else { -1.0 };
                        let speed = if swan { 9.0 } else { 13.0 };
                        for k in 0..n {
                            let lag = k as f32
                                * if young {
                                    0.22
                                } else if swan {
                                    0.8
                                } else {
                                    0.45
                                };
                            let ang = dir * (t * speed / r) + hash(bs, 160 + gi) * std::f32::consts::TAU - dir * lag;
                            let (x, zz) = (hx + r * ang.cos(), hz + r * 0.75 * ang.sin());
                            let c = level(self, x, zz);
                            // which way it faces on the screen: its velocity along the screen's right
                            let (vx, vz) = (-dir * ang.sin(), dir * 0.75 * ang.cos());
                            let right = vx * mv[0] + vz * mv[8] >= 0.0;
                            let bob = ((t * 1.7 + k as f32 * 1.3 + gi as f32).sin() * 0.6).round();
                            if swan {
                                draw_swan(&mut d, c, z, right, bob, &tc);
                            } else {
                                draw_duck(&mut d, c, z, right, bob, young && k > 0, hash(bs, 170 + gi * 8 + k) < 0.5, &tc);
                            }
                        }
                    }
                }
                // a fish jumping now and then: up out of the water and back, a splash each time and a ring spreading
                if body.tiles.len() >= 6 && ice <= 0.0 && a.rain < 0.6 && daylight > 0.3 {
                    let slot = (t / FISH_EVERY).floor();
                    let mut f = t - slot * FISH_EVERY;
                    let ks = bs ^ (slot as u32).wrapping_mul(0x85eb_ca6b);
                    // SG_FX_FISH=P (stills): every lake's fish at that point (0..1) of its jump and rings
                    let held = std::env::var("SG_FX_FISH").ok().and_then(|v| v.parse::<f32>().ok());
                    if let Some(p) = held {
                        f = p.clamp(0.0, 1.0) * (FISH_JUMP + FISH_RING);
                    } else if hash(ks, 1) > 0.55 || f > FISH_JUMP + FISH_RING {
                        continue;
                    }
                    let cand: &[(i32, i32)] = if body.open.is_empty() { &body.tiles } else { &body.open };
                    let tile = cand[(hash(ks, 2) * cand.len() as f32) as usize % cand.len()];
                    if !on_screen(&tile) {
                        continue;
                    }
                    let (fx, fz) = self.terrain.tile_centre(tile.0, tile.1);
                    let c = level(self, fx + (hash(ks, 3) - 0.5) * 0.6 * TILE_SIZE, fz + (hash(ks, 4) - 0.5) * 0.6 * TILE_SIZE);
                    let way = if hash(ks, 5) < 0.5 { 1.0 } else { -1.0 };
                    let silver = tc([0.78, 0.82, 0.86], 1.0);
                    let splash = tc([0.92, 0.96, 1.0], 0.9);
                    if f < FISH_JUMP {
                        let p = f / FISH_JUMP;
                        let x = (way * (p - 0.5) * 6.0).round();
                        let y = (8.0 * (std::f32::consts::PI * p).sin()).round();
                        // the fish, head first: rising, level at the top, diving
                        let (hx, hy) = if p < 0.4 {
                            (way, 1.0)
                        } else if p > 0.6 {
                            (way, -1.0)
                        } else {
                            (way, 0.0)
                        };
                        d.pixel(c, x, y, 1.0, 1.0, z, silver);
                        d.pixel(c, x + hx, y + hy, 1.0, 1.0, z, silver);
                        d.pixel(c, x - hx, y - hy, 1.0, 1.0, z, tc([0.45, 0.50, 0.56], 1.0));
                        for (q, at) in [(p / 0.2, -3.0 * way), ((p - 0.8) / 0.2, 3.0 * way)] {
                            if (0.0..1.0).contains(&q) {
                                let up = (3.0 * (std::f32::consts::PI * q).sin()).round();
                                d.pixel(c, at - 1.0, up, 1.0, 1.0, z, splash);
                                d.pixel(c, at + 1.0, up + 1.0, 1.0, 1.0, z, splash);
                                d.pixel(c, at, 0.0, 1.0, 1.0, z, splash);
                            }
                        }
                    }
                    let since = f - FISH_JUMP * 0.85;
                    if since > 0.0 && since < FISH_RING {
                        let e = since / FISH_RING;
                        rings.push((c, (0.06 + 0.30 * e) * TILE_SIZE, 0.75 * (1.0 - e)));
                        if e > 0.25 {
                            rings.push((c, (0.06 + 0.30 * (e - 0.25)) * TILE_SIZE, 0.45 * (1.0 - e)));
                        }
                    }
                }
            }
        }

        if s.spray {
            self.fx_spray(&mut d, &mut rings, z, &tc, (x0, x1, y0, y1));
        }
        self.fx.ponds = ponds;
        if !d.v.is_empty() {
            g.tris(Mode::Flat, None, u, &d.v);
        }
        // the rings on the water's surface, thin lines in the plane of the water
        for (c, r, al) in rings {
            let pts: Vec<[f32; 3]> = (0..20)
                .map(|i| {
                    let a = i as f32 / 20.0 * std::f32::consts::TAU;
                    [c[0] + r * a.cos(), c[1] + 0.5, c[2] + r * a.sin()]
                })
                .collect();
            self.draw_lines(g, u, &pts, true, 0.55 * vscale.max(1.0), tc([0.92, 0.96, 1.0], al));
        }
    }

    /// Spray where the original's fountains stand in the water (foam churning round the jet's foot, drops falling back,
    /// rings spreading) and foam and mist at the foot of its waterfalls.
    fn fx_spray(
        &self,
        d: &mut Dots,
        rings: &mut Vec<([f32; 3], f32, f32)>,
        z: f32,
        tc: &dyn Fn([f32; 3], f32) -> [f32; 4],
        (x0, x1, y0, y1): (i32, i32, i32, i32),
    ) {
        let t = self.fx.time;
        let theme = self.exe_theme().min(3);
        let depth = &self.water_depth;
        if depth.len() != (sg_core::course::n() * sg_core::course::n()) as usize {
            return;
        }
        let foam = tc([0.94, 0.97, 1.0], 0.9);
        let ice = if crate::fx::settings().ice { self.fx.atmos.ice } else { 0.0 };
        // the foam flickers in steps, as the art's own frames do
        let step = (t * 8.0).floor() as u32;
        let rot = 2 * ((self.rot / 90.0).round() as i32).rem_euclid(4);
        let ez = self.exe_zoom();
        let zl = if ez >= 3.0 {
            4
        } else if ez >= 1.5 {
            2
        } else {
            1
        };
        for b in y0..y1 {
            for a in x0..x1 {
                let i = sg_core::course::idx(a, b);
                if i >= depth.len() || self.course.ty[i] != sg_core::course::t::WATER {
                    continue;
                }
                let (cx, cz) = self.terrain.tile_centre(a, b);
                let c = [cx, self.terrain.height_at(cx, cz), cz];
                let hs = (a as u32) << 16 | b as u32;
                let fl = self.course.flags[i];
                // the fountain: wild_ui.rs puts the theme's spray sprite on scenic water by the shore (a rock in the desert)
                if fl & sg_core::course::f::SCENIC != 0 && fl & sg_core::course::f::PATH == 0 && depth[i] == 0 && theme != 1 && ice < 0.5 {
                    for k in 0..22u32 {
                        let ang = k as f32 / 22.0 * std::f32::consts::TAU + hash(hs, k) * 0.4;
                        let r = 5.0 + 6.0 * hash(hs ^ step, k);
                        let hk = hash(hs ^ step, 50 + k);
                        if hk < 0.75 {
                            let w = if hk < 0.25 { 2.0 } else { 1.0 };
                            d.pixel(c, (ang.cos() * r).round(), (ang.sin() * r * 0.5).round(), w, 1.0, z, foam);
                        }
                    }
                    // drops falling back round the jet
                    for k in 0..16u32 {
                        let p = (t * 0.9 + hash(hs, 100 + k)).fract();
                        let ang = hash(hs, 120 + k) * std::f32::consts::TAU;
                        let out = 3.0 + 9.0 * p;
                        let up = 22.0 * (1.0 - p * p) - 2.0;
                        d.pixel(
                            c,
                            (ang.cos() * out).round(),
                            (up + ang.sin() * out * 0.5).round(),
                            1.0,
                            1.0,
                            z,
                            tc([0.90, 0.95, 1.0], 0.8 * (1.0 - p * 0.6)),
                        );
                    }
                    for k in 0..2 {
                        let e = (t / 2.2 + k as f32 * 0.5).fract();
                        rings.push((c, (0.12 + 0.30 * e) * TILE_SIZE, 0.55 * (1.0 - e)));
                    }
                }
                // the waterfalls' sprays (0x23b, 0x23c): foam churning and mist rising where the falling water meets the pool
                for fall in sg_core::decor::waterfalls(&self.course, a, b, rot, zl) {
                    if fall.sprite != 0x23b && fall.sprite != 0x23c {
                        continue;
                    }
                    let tall = fall.sprite == 0x23c;
                    // the anchor as wild_ui.rs shifts the sprite (screen pixels at the exe's zoom), in units of `z`
                    let k = ez / zl as f32 / z;
                    let (sx, sy) = (fall.dx as f32 * k, -(fall.dy as f32) * k);
                    // foam churning round the spray's foot where it meets the pool (the sprite covers its middle), and rings
                    // spreading from it over the water
                    let (rx, n) = if tall { (32.0, 48u32) } else { (25.0, 36) };
                    for k in 0..n {
                        let hk = hash(hs ^ step ^ (fall.view as u32) << 8, k);
                        if hk > 0.7 {
                            continue;
                        }
                        let ang = k as f32 / n as f32 * std::f32::consts::TAU + hash(hs, 200 + k) * 0.3;
                        let r = rx * (0.85 + 0.3 * hash(hs ^ step, 220 + k));
                        let (x, y) = (sx + ang.cos() * r, sy - 2.0 + ang.sin() * r * 0.42);
                        d.pixel(c, x.round(), y.round(), 1.0 + (hk < 0.25) as u32 as f32, 1.0, z, foam);
                    }
                    // the anchor on the water's surface: moved along the screen, then back along the view to the water
                    let [rx_, ry_, rz_, ux, uy, uz] = d.axes;
                    let (ox, oy) = (sx * z * d.px, (sy - 2.0) * z * d.px);
                    let p = [c[0] + rx_ * ox + ux * oy, c[1] + ry_ * ox + uy * oy, c[2] + rz_ * ox + uz * oy];
                    let e = self.mv;
                    let (ex, ey, ez_) = (e[2], e[6], e[10]);
                    let back = if ey.abs() > 1e-3 { (p[1] - c[1]) / ey } else { 0.0 };
                    let foot = [p[0] - ex * back, c[1], p[2] - ez_ * back];
                    for k in 0..3 {
                        let e = (t / 1.8 + k as f32 / 3.0 + hash(hs, 300)).fract();
                        rings.push((foot, (0.30 + 0.35 * e) * TILE_SIZE * if tall { 1.2 } else { 1.0 }, 0.55 * (1.0 - e)));
                    }
                }
            }
        }
    }
}

/// A duck on the water, `right`: facing the screen's right; `young`: a duckling (two pixels); `drake`: the green head.
#[allow(clippy::too_many_arguments)]
fn draw_duck(d: &mut Dots, c: [f32; 3], z: f32, right: bool, bob: f32, young: bool, drake: bool, tc: &dyn Fn([f32; 3], f32) -> [f32; 4]) {
    let m = |x: f32, w: f32| if right { x } else { -x - w };
    let shade = tc([0.04, 0.16, 0.20], 0.35);
    if young {
        d.pixel(c, m(-1.0, 3.0), -1.0, 3.0, 1.0, z, shade);
        d.pixel(c, m(-1.0, 2.0), bob.max(0.0), 2.0, 1.0, z, tc([0.55, 0.45, 0.22], 1.0));
        d.pixel(c, m(0.0, 1.0), 1.0 + bob.max(0.0), 1.0, 1.0, z, tc([0.78, 0.68, 0.32], 1.0));
        return;
    }
    let body = if drake { [0.52, 0.44, 0.38] } else { [0.50, 0.38, 0.24] };
    let back = if drake { [0.70, 0.66, 0.60] } else { [0.62, 0.48, 0.30] };
    let head = if drake { [0.10, 0.40, 0.20] } else { [0.46, 0.34, 0.22] };
    d.pixel(c, m(-3.0, 7.0), -1.0, 7.0, 1.0, z, shade);
    d.pixel(c, m(-2.0, 5.0), bob, 5.0, 1.0, z, tc(body, 1.0));
    d.pixel(c, m(-2.0, 4.0), 1.0 + bob, 4.0, 1.0, z, tc(back, 1.0));
    d.pixel(c, m(-3.0, 1.0), 1.0 + bob, 1.0, 1.0, z, tc([0.20, 0.18, 0.16], 1.0));
    d.pixel(c, m(1.0, 2.0), 2.0 + bob, 2.0, 2.0, z, tc(head, 1.0));
    d.pixel(c, m(3.0, 1.0), 2.0 + bob, 1.0, 1.0, z, tc([0.95, 0.66, 0.16], 1.0));
    // a faint wake opening behind
    for i in 1..4 {
        let f = i as f32;
        d.pixel(c, m(-3.0 - 2.0 * f, 1.0), -1.0 + (f * 0.5).floor(), 1.0, 1.0, z, tc([0.85, 0.92, 0.98], 0.35 - 0.08 * f));
        d.pixel(c, m(-3.0 - 2.0 * f, 1.0), -1.0 - (f * 0.5).floor(), 1.0, 1.0, z, tc([0.85, 0.92, 0.98], 0.30 - 0.08 * f));
    }
}

/// A swan on the water, its neck raised and curved, its bill orange.
fn draw_swan(d: &mut Dots, c: [f32; 3], z: f32, right: bool, bob: f32, tc: &dyn Fn([f32; 3], f32) -> [f32; 4]) {
    let m = |x: f32, w: f32| if right { x } else { -x - w };
    let white = tc([0.97, 0.97, 0.95], 1.0);
    let grey = tc([0.80, 0.82, 0.84], 1.0);
    d.pixel(c, m(-4.0, 9.0), -1.0, 9.0, 1.0, z, tc([0.04, 0.16, 0.20], 0.35));
    d.pixel(c, m(-3.0, 7.0), bob, 7.0, 1.0, z, grey);
    d.pixel(c, m(-3.0, 6.0), 1.0 + bob, 6.0, 1.0, z, white);
    d.pixel(c, m(-2.0, 4.0), 2.0 + bob, 4.0, 1.0, z, white);
    d.pixel(c, m(2.0, 1.0), 2.0 + bob, 1.0, 3.0, z, white);
    d.pixel(c, m(1.0, 1.0), 5.0 + bob, 1.0, 1.0, z, white);
    d.pixel(c, m(2.0, 1.0), 6.0 + bob, 1.0, 1.0, z, white);
    d.pixel(c, m(3.0, 1.0), 6.0 + bob, 1.0, 1.0, z, tc([0.95, 0.52, 0.12], 1.0));
    d.pixel(c, m(2.0, 1.0), 5.0 + bob, 1.0, 1.0, z, tc([0.12, 0.10, 0.10], 1.0));
    for i in 1..4 {
        let f = i as f32;
        d.pixel(c, m(-4.0 - 2.0 * f, 1.0), -1.0 + (f * 0.5).floor(), 1.0, 1.0, z, tc([0.85, 0.92, 0.98], 0.3 - 0.07 * f));
        d.pixel(c, m(-4.0 - 2.0 * f, 1.0), -1.0 - (f * 0.5).floor(), 1.0, 1.0, z, tc([0.85, 0.92, 0.98], 0.26 - 0.07 * f));
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn waves_follow_the_wind_and_the_hour() {
        // calm at dawn, choppier at midday, rough in a storm
        let dawn = wave_strength(Some(0.5), Some(0.03), 0.0);
        let noon = wave_strength(Some(0.5), Some(0.5), 0.0);
        let storm = wave_strength(Some(1.0), Some(0.5), 1.0);
        assert!(dawn < noon && noon < storm, "{dawn} {noon} {storm}");
        assert!(wave_strength(Some(0.0), Some(0.0), 0.0) >= 0.08);
        assert!(storm <= 1.0);
        // without the wind option, a middling breeze
        assert_eq!(wave_strength(None, None, 0.0), 0.45);
    }

    #[test]
    fn ponds_and_their_distances() {
        let mut t = Terrain::demo_course(12, 12, 1);
        let w = sg_core::course::t::WATER;
        for y in 0..12 {
            for x in 0..12 {
                let k = t.tile_index(x, y);
                // a 7 x 7 lake and a single tile pond
                t.ty[k] = if (2..9).contains(&x) && (2..9).contains(&y) || (x, y) == (10, 10) { w } else { 2 };
            }
        }
        let p = build_ponds(&t);
        assert_eq!(p.bodies.len(), 2);
        let lake = p.body_at(5, 5).unwrap();
        assert_eq!(lake.tiles.len(), 49);
        assert!(!lake.sea);
        assert_eq!(p.dist_at(2, 5), 1);
        assert_eq!(p.dist_at(5, 5), 4);
        assert_eq!(p.dist_at(0, 0), 0);
        assert!(lake.open.contains(&(5, 5)) && lake.open.contains(&(3, 5)) && !lake.open.contains(&(2, 5)));
        assert_eq!(p.body_at(10, 10).unwrap().tiles, vec![(10, 10)]);
        // a stream running off the map is not the sea; a large water is
        t.ty[0] = w;
        assert!(!build_ponds(&t).body_at(0, 0).unwrap().sea);
        let mut sea = Terrain::demo_course(24, 24, 1);
        sea.ty.iter_mut().for_each(|ty| *ty = w);
        assert!(build_ponds(&sea).bodies[0].sea);
    }
}
