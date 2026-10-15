//! Enhanced graphics after the rain (docs/ENHANCED.md, "After the rain"): wet ground, darker and a little glossy with the
//! sky in it, during a rainy day and drying through the next; puddles on the flat, low fairways and rough that grow while it
//! rains and shrink as the ground dries; now and then a rainbow when the sun comes out after rain; and, as its own option,
//! lightning in heavy rain on parkland and links courses. Drawing only: which days rain is fx_light.rs's render-side hash,
//! whether a rainbow shows and when the lightning strikes are hashes of the course, the day and the effects' clock, never
//! the game's random generator.
//!
//! The ground's part is in the terrain shader (fx_glsl.rs, `wetted`), driven by the detail map made here (the puddle
//! potential of each spot: flat, low grass ground, broken into blobs by a noise) and the uniforms set in fx.rs.
use crate::app::App;
use crate::fx_light::Atmos;
use crate::gfx::{Gfx, Mode, Uniforms, Vert};
use sg_core::assets::Rgba;
use sg_core::terrain::{Terrain, HEIGHT_STEP, TILE_SIZE};

fn smoothstep(a: f32, b: f32, x: f32) -> f32 {
    let t = ((x - a) / (b - a)).clamp(0.0, 1.0);
    t * t * (3.0 - 2.0 * t)
}

fn unit(h: u32) -> f32 {
    (h >> 8) as f32 / (1u32 << 24) as f32
}

/// Smooth value noise on a lattice of `cell` world units (0..1), the same every run.
fn vnoise(x: f32, z: f32, cell: f32, salt: u32) -> f32 {
    let (fx, fz) = (x / cell, z / cell);
    let (ix, iz) = (fx.floor(), fz.floor());
    let (u, v) = (fx - ix, fz - iz);
    let (u, v) = (u * u * (3.0 - 2.0 * u), v * v * (3.0 - 2.0 * v));
    let at = |a: f32, b: f32| unit(crate::fx_light::hash_of(&[a as i32 as u32, b as i32 as u32, salt]));
    let a = at(ix, iz) + (at(ix + 1.0, iz) - at(ix, iz)) * u;
    let b = at(ix, iz + 1.0) + (at(ix + 1.0, iz + 1.0) - at(ix, iz + 1.0)) * u;
    a + (b - a) * v
}

/// The detail map, `n` texels a tile: r the puddle potential (0 never .. 1 the first to fill: flat, low fairway and rough,
/// broken into blobs), g grass ground (the seasons' tint and the frost go only there), b and a the ground's smooth normal
/// (world x and z, 0.5 level): the normals of the corner grid by central differences, spread bilinearly, so the per-pixel
/// light does not step from tile to tile as the meshes' flat faces do (fx.rs, the terrain shader's `light`).
pub fn build_detail(t: &Terrain, n: usize) -> Rgba {
    let (tw, th) = (t.w.max(1) as usize, t.h.max(1) as usize);
    let (w, h) = (tw * n, th * n);
    let mut img = Rgba::new(w as u32, h as u32);
    // per tile: how level it is and how low it sits against the ground round it
    let level: Vec<f32> = (0..tw * th)
        .map(|k| {
            let (x, y) = ((k % tw) as i32, (k / tw) as i32);
            let c = [t.corner_at(x, y), t.corner_at(x + 1, y), t.corner_at(x, y + 1), t.corner_at(x + 1, y + 1)];
            let span = c.iter().max().unwrap_or(&0) - c.iter().min().unwrap_or(&0);
            match span {
                0 => 1.0,
                1 => 0.45,
                _ => 0.0,
            }
        })
        .collect();
    let mean = |x: i32, y: i32| -> f32 {
        let c = [t.corner_at(x, y), t.corner_at(x + 1, y), t.corner_at(x, y + 1), t.corner_at(x + 1, y + 1)];
        c.iter().sum::<i32>() as f32 / 4.0
    };
    let low: Vec<f32> = (0..tw * th)
        .map(|k| {
            let (x, y) = ((k % tw) as i32, (k / tw) as i32);
            let (mut s, mut cnt) = (0.0f32, 0.0f32);
            for dy in -2..=2 {
                for dx in -2..=2 {
                    if t.inside(x + dx, y + dy) {
                        s += mean(x + dx, y + dy);
                        cnt += 1.0;
                    }
                }
            }
            ((s / cnt.max(1.0) - mean(x, y)) * 0.6).clamp(-0.5, 0.6)
        })
        .collect();
    let kind = |ty: i32| -> (f32, bool) {
        match ty {
            2..=5 => (1.0, true),
            0 => (0.5, true),
            1 => (0.25, true),
            // sand, water, marsh, out of bounds and buildings are not grass; the rest (rocks, brush, woods) is grassy ground
            7..=9 | 17..=22 => (0.0, false),
            _ => (0.0, true),
        }
    };
    // the corners' normals (central differences over the corner grid), world x and z
    let (cw, ch) = (tw + 1, th + 1);
    let hgt = |x: i32, y: i32| t.corner_at(x.clamp(0, tw as i32), y.clamp(0, th as i32)) as f32 * HEIGHT_STEP;
    let mut cn = vec![[0f32; 2]; cw * ch];
    for y in 0..ch as i32 {
        for x in 0..cw as i32 {
            let dx = (hgt(x + 1, y) - hgt(x - 1, y)) / (2.0 * TILE_SIZE);
            let dz = (hgt(x, y + 1) - hgt(x, y - 1)) / (2.0 * TILE_SIZE);
            let l = (dx * dx + 1.0 + dz * dz).sqrt();
            cn[y as usize * cw + x as usize] = [-dx / l, -dz / l];
        }
    }
    let normal_at = |fx: f32, fz: f32| -> [f32; 2] {
        let (fx, fz) = (fx.clamp(0.0, tw as f32 - 0.001), fz.clamp(0.0, th as f32 - 0.001));
        let (x0, z0) = (fx as usize, fz as usize);
        let (u, v) = (fx - x0 as f32, fz - z0 as f32);
        let c = |x: usize, z: usize| cn[z * cw + x];
        let mut o = [0f32; 2];
        for (k, o) in o.iter_mut().enumerate() {
            let a = c(x0, z0)[k] * (1.0 - u) + c(x0 + 1, z0)[k] * u;
            let b = c(x0, z0 + 1)[k] * (1.0 - u) + c(x0 + 1, z0 + 1)[k] * u;
            *o = a * (1.0 - v) + b * v;
        }
        o
    };
    for j in 0..h {
        for i in 0..w {
            let (tx, ty) = (i / n, j / n);
            let k = ty * tw + tx;
            let (mask, grass) = kind(t.type_at(tx as i32, ty as i32));
            // world position of the texel
            let (x, z) = ((i as f32 + 0.5) / n as f32 * TILE_SIZE, (j as f32 + 0.5) / n as f32 * TILE_SIZE);
            // puddles gather only at the noise's peaks (and more in hollows), so they stay small and scattered
            let blob = 0.65 * vnoise(x, z, 1.1 * TILE_SIZE, 41) + 0.35 * vnoise(x, z, 0.4 * TILE_SIZE, 42);
            let pot = (mask * level[k] * (0.95 * blob.powf(2.5) + 0.4 * low[k].max(0.0))).clamp(0.0, 1.0);
            let p = &mut img.px[(j * w + i) * 4..(j * w + i) * 4 + 4];
            p[0] = (pot * 255.0).round() as u8;
            p[1] = if grass { 255 } else { 0 };
            let nrm = normal_at((i as f32 + 0.5) / n as f32, (j as f32 + 0.5) / n as f32);
            p[2] = ((nrm[0] * 0.5 + 0.5) * 255.0).round().clamp(0.0, 255.0) as u8;
            p[3] = ((nrm[1] * 0.5 + 0.5) * 255.0).round().clamp(0.0, 255.0) as u8;
        }
    }
    img
}

impl App {
    /// Sets the wet ground, the puddles, a rainbow and a lightning flash of this moment into `a` (rain: today's).
    /// `SG_FX_WET=x` makes the ground that wet with puddles that far spread, `SG_FX_RAINBOW=x` shows a rainbow, and
    /// `SG_FX_LIGHTNING=x` a flash of that brightness, for stills.
    pub(crate) fn fx_weather_after(&self, a: &mut Atmos, day: u32, rain: f32) {
        let s = crate::fx::settings();
        let env = |k: &str| std::env::var(k).ok().and_then(|v| v.parse::<f32>().ok());
        // how far through the day (one game month) we are
        let f = self.fx_day_fraction();
        if s.after_rain {
            let yesterday = if day > 0 && s.rain { self.fx_rain_on(day - 1) } else { 0.0 };
            if rain > 0.0 {
                a.wet = rain * (0.6 + 0.4 * smoothstep(0.0, 0.25, f));
                a.puddles = rain * if yesterday > 0.0 { 1.0 } else { smoothstep(0.0, 0.45, f) };
            } else if yesterday > 0.0 {
                a.wet = yesterday * (1.0 - smoothstep(0.0, 0.7, f));
                a.puddles = yesterday * (1.0 - smoothstep(0.0, 0.9, f));
                // a rainbow on about half the days after rain, early on, while the sun is out
                let h = crate::fx_light::hash_of(&[self.fx_seed(), day, 0x0062_6f77]);
                if unit(h) < 0.5 {
                    a.rainbow = smoothstep(0.02, 0.07, f) * (1.0 - smoothstep(0.22, 0.38, f));
                }
            }
            if let Some(w) = env("SG_FX_WET") {
                a.wet = w.clamp(0.0, 1.0);
                a.puddles = a.wet;
            }
            if let Some(b) = env("SG_FX_RAINBOW") {
                a.rainbow = b.clamp(0.0, 1.0);
            }
            // only with the sun out
            if a.sun_w[1] <= 0.05 || a.shadow < 0.4 {
                a.rainbow = 0.0;
            }
        }
        if s.lightning {
            // storms where the climate is wet (the tropics' thunderstorms too), never in the desert
            if rain >= 0.7 && self.exe_theme() != 1 && self.fx_climate().rain >= 0.2 {
                a.flash = self.fx_lightning_now(rain);
            }
            if let Some(v) = env("SG_FX_LIGHTNING") {
                a.flash = v.clamp(0.0, 1.0);
            }
        }
    }

    /// The lightning's brightness now: a near strike now and then (a bright flash and two after-flickers) and fainter,
    /// more frequent flickers in the distance.
    fn fx_lightning_now(&self, rain: f32) -> f32 {
        let t = self.fx.time;
        let seed = self.fx_seed();
        let pulse = |x: f32, c: f32, w: f32| (-((x - c) / w).powi(2)).exp();
        let strike = |slot_len: f32, chance: f32, salt: u32, k: f32| -> f32 {
            let slot = (t / slot_len).floor();
            let h = crate::fx_light::hash_of(&[seed, slot as i32 as u32, salt]);
            if unit(h) >= chance {
                return 0.0;
            }
            let at = slot * slot_len + unit(crate::fx_light::hash_of(&[h])) * (slot_len - 0.6);
            let d = t - at;
            k * (pulse(d, 0.0, 0.05) + 0.55 * pulse(d, 0.16, 0.05) + 0.3 * pulse(d, 0.34, 0.06))
        };
        let heavy = ((rain - 0.6) / 0.4).clamp(0.0, 1.0);
        (strike(9.0, 0.35 * heavy + 0.1, 0x626f_6c74, 1.0) + strike(3.0, 0.45, 0x6661_7221, 0.22)).min(1.0)
    }

    /// The rainbow: a faint arc across the upper screen (at infinity, so it stays put while the view pans), opposite the
    /// sun, with a fainter second bow outside it.
    pub(crate) fn fx_rainbow_draw(&mut self, g: &mut Gfx, u: &Uniforms, k: f32) {
        let (w, h) = (self.draw_w, self.draw_h);
        let rot = crate::fx::view_rotation(self);
        let sun = self.fx.atmos.sun_w;
        let (rx, rz) = (rot[0], rot[8]);
        let side = (sun[0] * rx + sun[2] * rz) / (sun[0].hypot(sun[2]).max(1e-3) * rx.hypot(rz).max(1e-3));
        let cx = w * (0.5 - 0.28 * side);
        let big = h.max(w * 0.6) * 1.25;
        let cy = h * 0.4 + big;
        let (sx, sy) = (2.0 / w, 2.0 / h);
        let mut v: Vec<Vert> = Vec::with_capacity(12000);
        let spectrum = |t: f32| -> [f32; 3] {
            // inside violet .. outside red
            let hue = (1.0 - t) * 275.0;
            let hh = hue / 60.0;
            let x = 1.0 - (hh % 2.0 - 1.0).abs();
            match hh as i32 {
                0 => [1.0, x, 0.0],
                1 => [x, 1.0, 0.0],
                2 => [0.0, 1.0, x],
                3 => [0.0, x, 1.0],
                _ => [x, 0.0, 1.0],
            }
        };
        let mut bow = |r0: f32, r1: f32, strength: f32, reverse: bool| {
            let rings = 18;
            let segs = 90;
            for i in 0..rings {
                let (t0, t1) = (i as f32 / rings as f32, (i + 1) as f32 / rings as f32);
                let col = |t: f32| {
                    let c = spectrum(if reverse { 1.0 - t } else { t });
                    (c, (std::f32::consts::PI * t).sin().powf(0.8) * strength)
                };
                let (c0, a0) = col(t0);
                let (c1, a1) = col(t1);
                for j in 0..segs {
                    let (p0, p1) = (j as f32 / segs as f32, (j + 1) as f32 / segs as f32);
                    let ang = |p: f32| std::f32::consts::PI * (0.08 + 0.84 * p);
                    let pt = |rr: f32, p: f32, c: [f32; 3], al: f32| {
                        let (x, y) = (cx - rr * ang(p).cos(), cy - rr * ang(p).sin());
                        // fading towards the ground at both ends
                        let fade = smoothstep(h * 0.7, h * 0.25, y) * smoothstep(-0.02 * h, 0.12 * h, y);
                        Vert::new(x * sx - 1.0, 1.0 - y * sy, 0.0, 0.0, 0.0).col([c[0], c[1], c[2], al * fade])
                    };
                    let (ra, rb) = (r0 + (r1 - r0) * t0, r0 + (r1 - r0) * t1);
                    let (a, b, c, d) = (pt(ra, p0, c0, a0), pt(ra, p1, c0, a0), pt(rb, p1, c1, a1), pt(rb, p0, c1, a1));
                    v.extend_from_slice(&[a, b, c, a, c, d]);
                }
            }
        };
        bow(big * 0.90, big * 0.95, 0.24 * k, false);
        bow(big * 1.03, big * 1.075, 0.08 * k, true);
        g.tris(Mode::Flat, None, u, &v);
        // the sky inside a rainbow is a little brighter
        g.flush();
    }

    /// A lightning flash: the course lit pale blue-white, stronger towards the top (the sky).
    pub(crate) fn fx_flash_draw(&mut self, g: &mut Gfx, u: &Uniforms, k: f32) {
        let c = [0.86, 0.9, 1.0];
        let v = |x: f32, y: f32, a: f32| Vert::new(x, y, 0.0, 0.0, 0.0).col([c[0], c[1], c[2], a]);
        let (top, bottom) = ((0.42 * k).min(0.6), 0.24 * k);
        g.quad(Mode::Flat, None, u, [v(-1.0, 1.0, top), v(1.0, 1.0, top), v(1.0, -1.0, bottom), v(-1.0, -1.0, bottom)]);
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn puddles_only_on_level_grass() {
        let mut t = Terrain::demo_course(8, 8, 1);
        for y in 0..8 {
            for x in 0..8 {
                t.paint(x, y, if x < 4 { 4 } else { 7 }, 0);
                t.flatten_tile(x, y);
            }
        }
        let img = build_detail(&t, 2);
        let at = |i: usize, j: usize, c: usize| img.px[(j * img.w as usize + i) * 4 + c];
        // sand never puddles and is not grass; rough is grass and may puddle
        assert_eq!((at(12, 4, 0), at(12, 4, 1)), (0, 0));
        assert_eq!(at(2, 4, 1), 255);
        assert!((0..8).any(|i| at(i, 6, 0) > 0));
    }
}
