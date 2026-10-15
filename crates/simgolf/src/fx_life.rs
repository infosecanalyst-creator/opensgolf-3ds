//! Enhanced graphics' life on the course (docs/ENHANCED.md, "Life on the course"): the theme's bird of prey (the original's
//! fly-over sprite) wheeling over the course now and then, butterflies over flower beds in the warm months, and leaves
//! dropping from broadleaf trees in autumn, the last two as a few crisp pixels at the art's own pixel size. Drawing only:
//! nothing touches the simulation, and what varies comes from hashes of the course and the effects'
//! clock, never from the game's random generator.
use crate::app::App;
use crate::fx_nature::Kind;
use crate::gfx::{Gfx, Mode, Uniforms, Vert};
use sg_core::terrain::{TILE_SIZE, TT_DEEP_ROUGH, TT_FLOWER_BED, TT_ROUGH};

/// Seconds between the birds of prey wheeling over, and the share of that time they are up.
const SOAR_EVERY: f32 = 70.0;
const SOAR_SHARE: f32 = 0.55;
/// The most butterflies and leafy trees drawn.
const MAX_BUTTERFLIES: usize = 40;
const MAX_LEAF_TREES: usize = 80;

fn hash(a: u32, b: u32) -> f32 {
    let mut h = a.wrapping_mul(0x9e37_79b9) ^ b.wrapping_mul(0x85eb_ca6b);
    h ^= h >> 15;
    h = h.wrapping_mul(0x2c1b_3c6d);
    h ^= h >> 12;
    (h >> 8) as f32 / (1u32 << 24) as f32
}

/// Crisp squares facing the camera, in the art's pixels: a world point, then art pixels to the right and up (snapped to
/// whole art pixels, as the sprites are drawn).
struct Dots {
    v: Vec<Vert>,
    axes: [f32; 6],
    px: f32,
}

impl Dots {
    /// A block `w` x `h` art pixels whose bottom left is `side`, `up` art pixels from a world point; `z` art pixel size.
    #[allow(clippy::too_many_arguments)]
    fn pixel(&mut self, c: [f32; 3], side: f32, up: f32, w: f32, h: f32, z: f32, col: [f32; 4]) {
        let [rx, ry, rz, ux, uy, uz] = self.axes;
        let px = self.px * z;
        let (x0, y0) = (side.round(), up.round());
        let p = |x: f32, y: f32| {
            let (x, y) = (x * px, y * px);
            Vert::new(c[0] + rx * x + ux * y, c[1] + ry * x + uy * y, c[2] + rz * x + uz * y, 0.0, 0.0).col(col)
        };
        let (a, b, cc, d) = (p(x0, y0 + h), p(x0 + w, y0 + h), p(x0 + w, y0), p(x0, y0));
        self.v.extend_from_slice(&[a, b, cc, a, cc, d]);
    }
}

impl App {
    /// The birds of prey wheeling over the course now (wild_ui.rs draws them with the original's fly-over sprite): their
    /// places in map units and their headings (0..7, the fly-overs' compass). None in the dark or the rain.
    /// `SG_FX_BIRDS=1` keeps two up over the middle of the view (stills).
    pub(crate) fn fx_soarers(&self) -> Vec<(i32, i32, usize)> {
        let a = self.fx.atmos;
        if a.night > 0.5 || a.rain > 0.4 {
            return Vec::new();
        }
        let t = self.fx.time;
        let test = std::env::var("SG_FX_BIRDS").is_ok_and(|v| v == "1");
        let k = (t / SOAR_EVERY).floor();
        let ks = k as u32 ^ self.fx_seed();
        if !test && ((t / SOAR_EVERY - k) > SOAR_SHARE || hash(ks, 1) > 0.6) {
            return Vec::new();
        }
        let n = sg_core::mapsize::n() as f32 * 1024.0;
        let centre = if test {
            // units_to_world's inverse
            let (x0, z0) = self.units_to_world(0, 0);
            let k = TILE_SIZE / sg_core::staff::UNIT as f32;
            // the fly-over sprite flies high above its ground point: SG_FX_BIRDS_AT=dx,dy (tiles) moves the circle
            let off: Vec<f32> = std::env::var("SG_FX_BIRDS_AT").unwrap_or_default().split(',').filter_map(|v| v.parse().ok()).collect();
            let (ox, oy) = (off.first().copied().unwrap_or(0.0) * 1024.0, off.get(1).copied().unwrap_or(0.0) * 1024.0);
            ((self.cam_x - x0) / k + ox, (self.cam_z - z0) / k + oy)
        } else {
            (n * (0.25 + 0.5 * hash(ks, 2)), n * (0.25 + 0.5 * hash(ks, 3)))
        };
        let count = if test || hash(ks, 4) < 0.5 { 2 } else { 1 };
        (0..count)
            .map(|b| {
                let r = (3.0 + 2.0 * hash(ks, 5 + b)) * 1024.0;
                let dir = if hash(ks, 7) < 0.5 { 1.0 } else { -1.0 };
                let ang = dir * t * 0.22 + b as f32 * 2.4 + hash(ks, 8) * 6.3;
                let (x, y) = (centre.0 + r * ang.cos(), centre.1 + r * ang.sin());
                // the heading along the circle, to the nearest of the eight
                let (tx, ty) = (-ang.sin() * dir, ang.cos() * dir);
                const DX: [f32; 8] = [0.0, 1.0, 1.0, 1.0, 0.0, -1.0, -1.0, -1.0];
                const DY: [f32; 8] = [-1.0, -1.0, 0.0, 1.0, 1.0, 1.0, 0.0, -1.0];
                let h = (0..8).max_by(|&i, &j| (DX[i] * tx + DY[i] * ty).total_cmp(&(DX[j] * tx + DY[j] * ty))).unwrap_or(0);
                (x as i32, y as i32, h)
            })
            .collect()
    }

    /// The butterflies and falling leaves, over the course.
    pub(crate) fn fx_life(&mut self, g: &mut Gfx, u: &Uniforms) {
        if !crate::fx::settings().life || self.land.is_none() {
            return;
        }
        let a = self.fx.atmos;
        let t = self.fx.time;
        let mv = self.mv;
        let vscale = (self.draw_w / 800.0).min(self.draw_h / 600.0);
        // sizes below are in the art's pixels: drawn one to one at the closest zoom (the exe's 4), smaller further out
        let z = self.exe_zoom() / 4.0;
        let tint = self.fx_sprite_tint(0.0, 0.0, false).map(|t| [t[0], t[1], t[2]]).unwrap_or([1.0; 3]);
        let mut d = Dots { v: Vec::new(), axes: [mv[0], mv[4], mv[8], mv[1], mv[5], mv[9]], px: self.upp * vscale };
        let seed = self.fx_seed();
        let daylight = (1.0 - a.night * 1.5).clamp(0.0, 1.0);
        let dry = (1.0 - a.rain * 2.0).clamp(0.0, 1.0);
        let month = self.fx_month();
        let ground = |app: &App, x: f32, z: f32| [x, app.terrain.height_at(x, z), z];

        // butterflies: one over each of the first flower beds, May to September, in the day and the dry
        let warm = ((month - 4.5).min(9.7 - month) * 2.0).clamp(0.0, 1.0);
        if warm * daylight * dry > 0.05 {
            let al = warm * daylight * dry;
            const COLS: [[f32; 3]; 4] = [[1.0, 1.0, 0.95], [1.0, 0.9, 0.3], [1.0, 0.6, 0.2], [0.65, 0.8, 1.0]];
            let mut n = 0;
            'tiles: for ty in 0..self.terrain.h {
                for tx in 0..self.terrain.w {
                    if self.terrain.ty[self.terrain.tile_index(tx, ty)] != TT_FLOWER_BED {
                        continue;
                    }
                    let h = |k: u32| hash((tx as u32) << 16 | ty as u32, k ^ seed);
                    if h(1) > 0.6 {
                        continue;
                    }
                    let ph = h(2) * 50.0;
                    let (cx, cz) = self.terrain.tile_centre(tx, ty);
                    let x = cx + (t * 0.45 + ph).sin() * 0.45 * TILE_SIZE + (t * 1.3 + ph).sin() * 0.1 * TILE_SIZE;
                    let zz = cz + (t * 0.37 + ph * 1.3).cos() * 0.45 * TILE_SIZE;
                    let c = ground(self, x, zz);
                    let up = (4.0 + 2.0 * (t * 1.9 + ph).sin()) * 1.0;
                    let open = (t * 9.0 + ph).sin() > 0.0;
                    let cl = COLS[(h(3) * 4.0) as usize % 4];
                    let col = [cl[0] * tint[0], cl[1] * tint[1], cl[2] * tint[2], al];
                    let dark = [0.15 * tint[0], 0.12 * tint[1], 0.1 * tint[2], al];
                    // a dark body pixel with a wing pixel either side when open, both wings up when closed
                    d.pixel(c, 0.0, up, 1.0, 1.0, z, dark);
                    if open {
                        d.pixel(c, -1.0, up, 1.0, 1.0, z, col);
                        d.pixel(c, 1.0, up, 1.0, 1.0, z, col);
                    } else {
                        d.pixel(c, 0.0, up + 1.0, 1.0, 1.0, z, col);
                    }
                    n += 1;
                    if n >= MAX_BUTTERFLIES {
                        break 'tiles;
                    }
                }
            }
        }

        // falling leaves: September and October, from broadleaf trees, drifting with the wind
        let autumn = a.season.autumn;
        if autumn > 0.05 {
            let w = a.wind;
            let mut n = 0;
            for i in 0..self.props.len() {
                let p = &self.props[i];
                let Some(si) = p.body.filter(|_| p.golfer.is_none() && p.employee.is_none()) else { continue };
                let (px, pz) = (p.x, p.z);
                if !matches!(self.fx_kind(si), Kind::Deciduous | Kind::Blossom) {
                    continue;
                }
                let hs = px.to_bits() ^ pz.to_bits().rotate_left(13) ^ seed;
                if hash(hs, 1) > 0.5 * autumn + 0.1 {
                    continue;
                }
                let c = ground(self, px, pz);
                for l in 0..2u32 {
                    let period = 5.0 + 3.0 * hash(hs, 2 + l);
                    let ph = (t / period + hash(hs, 4 + l)).fract();
                    let top = 18.0 + 10.0 * hash(hs, 6 + l);
                    let up = top * (1.0 - ph);
                    let drift = (w.screen_x * 8.0 * w.k + 2.5 * (t * 2.1 + l as f32 * 3.0).sin()) * ph;
                    let start = (hash(hs, 8 + l) - 0.5) * 16.0;
                    let shade = hash(hs, 10 + l);
                    let al = (1.0 - ph * ph) * autumn;
                    let cl = [(0.85 + 0.15 * shade) * tint[0], (0.35 + 0.4 * shade) * tint[1], 0.1 * tint[2], al];
                    // tumbling: two pixels wide, then one
                    let wide = (t * 5.0 + l as f32 * 2.0 + shade * 6.0).sin() > 0.0;
                    d.pixel(c, start + drift, up, if wide { 2.0 } else { 1.0 }, 1.0, z, cl);
                }
                n += 1;
                if n >= MAX_LEAF_TREES {
                    break;
                }
            }
        }

        if !d.v.is_empty() {
            g.tris(Mode::Flat, None, u, &d.v);
        }
    }

    /// Grass tufts: a few blades of a pixel each standing up from the rough round the view, the tips swaying with the wind.
    /// Only at the closest zoom level, where the art is drawn one to one.
    pub(crate) fn fx_grass(&mut self, g: &mut Gfx, u: &Uniforms) {
        if !crate::fx::settings().grass || self.land.is_none() || self.exe_zoom() < 3.0 {
            return;
        }
        let a = self.fx.atmos;
        let t = self.fx.time;
        let mv = self.mv;
        let vscale = (self.draw_w / 800.0).min(self.draw_h / 600.0);
        let z = self.exe_zoom() / 4.0;
        let tint = self.fx_sprite_tint(0.0, 0.0, false).map(|t| [t[0], t[1], t[2]]).unwrap_or([1.0; 3]);
        let mut d = Dots { v: Vec::new(), axes: [mv[0], mv[4], mv[8], mv[1], mv[5], mv[9]], px: self.upp * vscale };
        let seed = self.fx_seed();
        let desert = self.exe_theme() == 1;
        // the tiles round the view
        let (cx, cy) = self.terrain.tile_of(self.cam_x, self.cam_z);
        let reach = ((self.draw_w.max(self.draw_h) * self.upp / TILE_SIZE) * 0.75).ceil() as i32 + 1;
        let (dark, light) = if desert { ([0.42, 0.40, 0.22], [0.58, 0.55, 0.32]) } else { ([0.20, 0.36, 0.12], [0.36, 0.55, 0.20]) };
        let snow = a.snow;
        for ty in (cy - reach).max(0)..(cy + reach).min(self.terrain.h) {
            for tx in (cx - reach).max(0)..(cx + reach).min(self.terrain.w) {
                let n = match self.terrain.ty[self.terrain.tile_index(tx, ty)] {
                    TT_ROUGH => 3,
                    TT_DEEP_ROUGH => 6,
                    _ => continue,
                };
                let (mx, mz) = self.terrain.tile_centre(tx, ty);
                for k in 0..n {
                    let hs = ((tx as u32) << 16 | ty as u32) ^ seed;
                    let (ox, oz) = (hash(hs, 1 + k) - 0.5, hash(hs, 20 + k) - 0.5);
                    let (x, zz) = (mx + ox * TILE_SIZE * 0.9, mz + oz * TILE_SIZE * 0.9);
                    let c = [x, self.terrain.height_at(x, zz), zz];
                    let sway = (a.wind.k * a.wind.screen_x * 1.6 + 0.6 * (t * 2.3 + hash(hs, 40 + k) * 6.3).sin() * a.wind.k).round();
                    // five blades fanning out from the root: (where, height, which way the tip leans)
                    let size = 0.7 + 0.5 * hash(hs, 80 + k);
                    for (b, &(bx, h, lean)) in
                        [(-2.0f32, 3.0f32, -1.0f32), (-1.0, 4.0, 0.0), (0.0, 5.0, 0.0), (1.0, 4.0, 0.0), (2.0, 3.0, 1.0)].iter().enumerate()
                    {
                        let h = (h * size).round().max(2.0);
                        let shade = hash(hs, 60 + k * 5 + b as u32);
                        let base = if b % 2 == 0 { dark } else { light };
                        let m = 0.85 + 0.3 * shade;
                        let mut col = [base[0] * m * tint[0], base[1] * m * tint[1], base[2] * m * tint[2], 0.95];
                        if snow > 0.0 {
                            let w = snow * 0.5;
                            col = [col[0] + (0.9 - col[0]) * w, col[1] + (0.92 - col[1]) * w, col[2] + (0.95 - col[2]) * w, col[3]];
                        }
                        // the blade's lower part straight, the top two pixels leaning out and with the wind
                        d.pixel(c, bx, 0.0, 1.0, h - 2.0, z, col);
                        d.pixel(c, bx + (lean * 0.5).round(), h - 2.0, 1.0, 1.0, z, col);
                        d.pixel(c, bx + lean + sway, h - 1.0, 1.0, 1.0, z, col);
                    }
                }
            }
        }
        if !d.v.is_empty() {
            g.tris(Mode::Flat, None, u, &d.v);
        }
    }
}
