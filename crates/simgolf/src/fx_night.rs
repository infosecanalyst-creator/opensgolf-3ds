//! Enhanced graphics' evening lights (docs/ENHANCED.md, "Evening lights"), with the time of day: from sunset the windows of
//! the clubhouse and the other buildings glow warm (their own tick, *Lit windows*), lamps light up beside the buildings and along the paths with soft pools
//! of light on the ground, and on parkland and tropical courses fireflies drift over the rough and near the water at dusk.
//! The lights fade out again at dawn. Without the time of day there is no evening, and nothing here draws.
//!
//! The windows are found in the art itself: a building frame's dark glass panes (dark, neutral or bluish pixels framed by
//! light wall or frame pixels, in small patches) and its turquoise stained glass make a mask, cached per frame, which is
//! drawn warm over the building with a soft halo (the bloom picks it up on the desktop). Which windows are lit, where the
//! path lamps stand and where the fireflies fly come from render-side hashes, the same each time.
use crate::app::App;
use crate::fx_light::Atmos;
use crate::gfx::{Gfx, Mode, Uniforms, Vert};
use miniquad::*;
use sg_core::assets::Rgba;
use sg_core::terrain::TILE_SIZE;
use std::collections::HashMap;

/// Transparent pixels round a window mask, room for its halo.
pub const GLOW_PAD: usize = 5;

/// The windows' glow per (sprite, frame index): the classic texture it was made from and the glow (None: no windows).
#[derive(Default)]
pub struct NightState {
    glow: HashMap<(usize, usize), (TextureId, Option<TextureId>)>,
}

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

/// The evening's lights at day phase `p` for `theme` (fireflies on parkland and tropical courses, more in summer when the
/// month is known).
pub fn evening(a: &mut Atmos, p: f32, theme: u8, month: Option<f32>) {
    a.night = night_at(p);
    if theme == 0 || theme == 2 {
        let dusk = if p > 0.5 { smoothstep(0.885, 0.95, p) } else { 1.0 - smoothstep(0.0, 0.015, p) };
        let summer = month.map(|m| 0.25 + 0.75 * smoothstep(3.8, 5.0, m) * (1.0 - smoothstep(9.3, 10.4, m))).unwrap_or(1.0);
        a.fireflies = dusk * summer * (1.0 - a.rain);
    }
}

/// How far into the night day phase `p` is (0 by day, 1 at night): from sunset to just after dawn.
pub fn night_at(p: f32) -> f32 {
    if p > 0.5 {
        smoothstep(0.84, 0.935, p)
    } else {
        1.0 - smoothstep(0.0, 0.06, p)
    }
}

/// The windows of a building frame as a warm glow (padded by GLOW_PAD), or None when it has none. `salt` picks which
/// windows are lit (about four in five).
pub fn window_glow(img: &Rgba, salt: u32) -> Option<Rgba> {
    let (w, h) = (img.w as usize, img.h as usize);
    let px = |x: usize, y: usize| -> [f32; 4] {
        let p = &img.px[(y * w + x) * 4..(y * w + x) * 4 + 4];
        [p[0] as f32 / 255.0, p[1] as f32 / 255.0, p[2] as f32 / 255.0, p[3] as f32 / 255.0]
    };
    let mut glass = vec![false; w * h];
    let mut light = vec![false; w * h];
    let mut cyan = vec![false; w * h];
    for y in 0..h {
        for x in 0..w {
            let [r, g, b, a] = px(x, y);
            if a < 0.5 {
                continue;
            }
            let lum = 0.299 * r + 0.587 * g + 0.114 * b;
            let (mx, mn) = (r.max(g).max(b), r.min(g).min(b));
            let sat = if mx > 0.0 { (mx - mn) / mx } else { 0.0 };
            let k = y * w + x;
            glass[k] = lum < 0.40 && b >= r - 0.04 && g <= r.max(b) + 0.05;
            light[k] = lum > 0.52 && sat < 0.5;
            cyan[k] = b > 0.55 && g > 0.5 && r < 0.45;
        }
    }
    // a light pixel within three to one side
    let near = |x: usize, y: usize, dx: i32, dy: i32| -> bool {
        (1..=3).any(|s| {
            let (xx, yy) = (x as i32 + dx * s, y as i32 + dy * s);
            xx >= 0 && yy >= 0 && (xx as usize) < w && (yy as usize) < h && light[yy as usize * w + xx as usize]
        })
    };
    let mut win = vec![false; w * h];
    for y in 0..h {
        for x in 0..w {
            let k = y * w + x;
            if cyan[k] {
                win[k] = true;
            } else if glass[k] {
                let (l, r, u, d) = (near(x, y, -1, 0), near(x, y, 1, 0), near(x, y, 0, -1), near(x, y, 0, 1));
                win[k] = (l && r) || (u && d) || ((l || r) && (u || d));
            }
        }
    }
    // small patches only (a pane, not a dark wall); each lit or not
    let mut core = vec![0f32; w * h];
    let mut seen = vec![false; w * h];
    let mut any = false;
    for start in 0..w * h {
        if !win[start] || seen[start] {
            continue;
        }
        let mut stack = vec![start];
        seen[start] = true;
        let mut pts = Vec::new();
        while let Some(k) = stack.pop() {
            pts.push(k);
            let (x, y) = ((k % w) as i32, (k / w) as i32);
            for (dx, dy) in [(1, 0), (-1, 0), (0, 1), (0, -1)] {
                let (nx, ny) = (x + dx, y + dy);
                if nx >= 0 && ny >= 0 && (nx as usize) < w && (ny as usize) < h {
                    let n = ny as usize * w + nx as usize;
                    if win[n] && !seen[n] {
                        seen[n] = true;
                        stack.push(n);
                    }
                }
            }
        }
        let (x0, x1) = pts.iter().map(|k| k % w).fold((usize::MAX, 0), |(a, b), x| (a.min(x), b.max(x)));
        let (y0, y1) = pts.iter().map(|k| k / w).fold((usize::MAX, 0), |(a, b), y| (a.min(y), b.max(y)));
        // a pane is a compact patch: thin streaks (roof edges, gutters, the outlines of a dark trim) are not windows
        let area = ((x1 - x0 + 1) * (y1 - y0 + 1)) as f32;
        if pts.len() > 80 || x1 - x0 > 16 || y1 - y0 > 16 || (pts.len() > 2 && (pts.len() as f32) < 0.4 * area) {
            continue;
        }
        let lit = unit(hash(&[salt, x0 as u32, y0 as u32])) < 0.82;
        if !lit {
            continue;
        }
        // a warmer or a paler light per window
        let v = 0.8 + 0.2 * unit(hash(&[salt, y0 as u32, x0 as u32, 1]));
        for k in pts {
            core[k] = v;
        }
        any = true;
    }
    if !any {
        return None;
    }
    let (pw, ph) = (w + 2 * GLOW_PAD, h + 2 * GLOW_PAD);
    let mut a = vec![0f32; pw * ph];
    for y in 0..h {
        for x in 0..w {
            a[(y + GLOW_PAD) * pw + x + GLOW_PAD] = core[y * w + x];
        }
    }
    // the halo: the panes blurred (two box passes each way, radius 2)
    let mut halo = a.clone();
    let mut tmp = vec![0f32; pw * ph];
    for _ in 0..2 {
        for y in 0..ph {
            for x in 0..pw {
                let (lo, hi) = (x.saturating_sub(2), (x + 2).min(pw - 1));
                tmp[y * pw + x] = (lo..=hi).map(|xx| halo[y * pw + xx]).sum::<f32>() / 5.0;
            }
        }
        for y in 0..ph {
            for x in 0..pw {
                let (lo, hi) = (y.saturating_sub(2), (y + 2).min(ph - 1));
                halo[y * pw + x] = (lo..=hi).map(|yy| tmp[yy * pw + x]).sum::<f32>() / 5.0;
            }
        }
    }
    let mut out = Rgba::new(pw as u32, ph as u32);
    for k in 0..pw * ph {
        let c = a[k];
        let h = (halo[k] * 1.6).min(0.55);
        let al = c.max(h);
        if al <= 0.0 {
            continue;
        }
        // the panes a pale warm yellow, the halo deeper orange
        let m = if al > 0.0 { c / al } else { 0.0 };
        let col = [255.0, 186.0 + 46.0 * m, 96.0 + 70.0 * m];
        let p = &mut out.px[k * 4..k * 4 + 4];
        p[0] = col[0] as u8;
        p[1] = col[1] as u8;
        p[2] = col[2] as u8;
        p[3] = (al.min(1.0) * 255.0).round() as u8;
    }
    Some(out)
}

impl App {
    /// The glow of a building frame's lit windows, cached (None when it has none, or it is not a building). The texture is
    /// padded by GLOW_PAD pixels each way.
    pub(crate) fn fx_window_glow(&mut self, g: &mut Gfx, si: usize, frame_index: usize, classic: TextureId) -> Option<TextureId> {
        if self.fx.atmos.night <= 0.0 || crate::hd::active() || self.pack_hires(si) || self.fx_kind(si) != crate::fx_nature::Kind::Building
        {
            return None;
        }
        if let Some(&(src, t)) = self.fx.night.glow.get(&(si, frame_index)) {
            if src == classic {
                return t;
            }
            if let Some(t) = t {
                g.ctx.delete_texture(t);
            }
        }
        let img = window_glow(&self.sprites[si].s.frame(frame_index), si as u32 * 7919 + frame_index as u32);
        let t = img.map(|i| {
            let t = g.texture(&i, false);
            crate::fx::plain_texture(t);
            t
        });
        self.fx.night.glow.insert((si, frame_index), (classic, t));
        t
    }

    /// Where the lamps stand (world x, z): at the two outer corners (on the screen) of each building and the clubhouse, and
    /// every few tiles along the paths.
    fn fx_lamp_spots(&self) -> Vec<(f32, f32)> {
        let mut out = Vec::new();
        let t = &self.terrain;
        let m = self.mv;
        if let Some(l) = &self.land {
            for o in l.objects.iter().filter(|o| o.kind >= 6) {
                let s = l.footprint_size(o) as f32;
                let (x0, z0) = t.tile_centre(o.a, o.b);
                let (x0, z0) = (x0 - TILE_SIZE * 0.5, z0 - TILE_SIZE * 0.5);
                let e = 0.3 * TILE_SIZE;
                let corners = [
                    (x0 - e, z0 - e),
                    (x0 + s * TILE_SIZE + e, z0 - e),
                    (x0 - e, z0 + s * TILE_SIZE + e),
                    (x0 + s * TILE_SIZE + e, z0 + s * TILE_SIZE + e),
                ];
                let sx = |c: &(f32, f32)| m[0] * c.0 + m[8] * c.1;
                let left = corners.iter().min_by(|a, b| sx(a).total_cmp(&sx(b)));
                let right = corners.iter().max_by(|a, b| sx(a).total_cmp(&sx(b)));
                out.extend(left.into_iter().chain(right).copied());
            }
        }
        let seed = self.fx_seed();
        let mut last: HashMap<(i32, i32), ()> = HashMap::new();
        for y in 0..t.h {
            for x in 0..t.w {
                if t.path_at(x, y) == 0 || unit(hash(&[seed, x as u32, y as u32, 0x6c61])) > 0.3 {
                    continue;
                }
                if (-2..=2).any(|dy| (-2..=2).any(|dx| last.contains_key(&(x + dx, y + dy)))) {
                    continue;
                }
                last.insert((x, y), ());
                let (cx, cz) = t.tile_centre(x, y);
                let a = unit(hash(&[seed, x as u32, y as u32, 0x6c62])) * std::f32::consts::TAU;
                out.push((cx + a.cos() * 0.38 * TILE_SIZE, cz + a.sin() * 0.38 * TILE_SIZE));
            }
        }
        out
    }

    /// The lamps' soft pools of light on the ground and the windows' light spilling round the buildings: called after the
    /// ground and the cast shadows, before the sprites.
    pub(crate) fn fx_light_pools(&mut self, g: &mut Gfx, u: &Uniforms) {
        let k = self.fx.atmos.night;
        if k <= 0.0 {
            return;
        }
        let s = crate::fx::settings();
        let dot = crate::fx::gpu(g).dot;
        let mut v: Vec<Vert> = Vec::new();
        let pool = |v: &mut Vec<Vert>, x: f32, z: f32, y: f32, rad: f32, a: f32| {
            let c = [1.0, 0.78, 0.45, a];
            let p = |dx: f32, dz: f32, tu: f32, tv: f32| Vert::new(x + dx, y, z + dz, tu, tv).col(c);
            v.extend_from_slice(&[
                p(-rad, -rad, 0.0, 0.0),
                p(rad, -rad, 1.0, 0.0),
                p(rad, rad, 1.0, 1.0),
                p(-rad, -rad, 0.0, 0.0),
                p(rad, rad, 1.0, 1.0),
                p(-rad, rad, 0.0, 1.0),
            ]);
        };
        let lamps = if s.evening_lights { self.fx_lamp_spots() } else { Vec::new() };
        for (x, z) in lamps {
            let y = self.terrain.height_at(x, z) + 0.5;
            pool(&mut v, x, z, y, 1.2 * TILE_SIZE, 0.42 * k);
            pool(&mut v, x, z, y, 0.45 * TILE_SIZE, 0.22 * k);
        }
        // the Path styles and lamps extra's lamp posts (paths_lamps.rs): a wider, warmer pool each, in this same pass
        for (x, z) in self.lamp_spots() {
            let y = self.terrain.height_at(x, z) + 0.5;
            pool(&mut v, x, z, y, 1.6 * TILE_SIZE, 0.5 * k);
            pool(&mut v, x, z, y, 0.6 * TILE_SIZE, 0.28 * k);
        }
        // the windows' light spilling round the buildings (with the lit windows)
        if let Some(l) = self.land.as_ref().filter(|_| s.window_lights) {
            for o in l.objects.iter().filter(|o| o.kind >= 6) {
                let s = l.footprint_size(o) as f32;
                let (x0, z0) = self.terrain.tile_centre(o.a, o.b);
                let (x, z) = (x0 + (s - 1.0) * 0.5 * TILE_SIZE, z0 + (s - 1.0) * 0.5 * TILE_SIZE);
                let y = self.terrain.height_at(x, z) + 0.5;
                pool(&mut v, x, z, y, (s * 0.5 + 1.3) * TILE_SIZE, 0.24 * k);
            }
        }
        g.tris(Mode::Flat, Some(dot), u, &v);
        g.flush();
    }

    /// The lamps themselves, as (depth key, triangles) to be drawn in the sprites' order: a slim dark post and a bright
    /// head with a halo. `px` is world units per 800 x 600 pixel.
    pub(crate) fn fx_lamp_quads(&self, px: f32) -> Vec<(f32, Vec<Vert>)> {
        let k = self.fx.atmos.night;
        if k <= 0.0 || !crate::fx::settings().evening_lights {
            return Vec::new();
        }
        let m = self.mv;
        let (rx, ry, rz) = (m[0], m[4], m[8]);
        let (ux, uy, uz) = (m[1], m[5], m[9]);
        let mut out = Vec::new();
        for (x, z) in self.fx_lamp_spots() {
            let y = self.terrain.height_at(x, z);
            let key = m[2] * x + m[6] * y + m[10] * z;
            let at = |cx: f32, cy: f32, tu: f32, tv: f32, c: [f32; 4]| {
                let (cx, cy) = (cx * px, cy * px);
                Vert::new(x + rx * cx + ux * cy, y + ry * cx + uy * cy, z + rz * cx + uz * cy, tu, tv).col(c)
            };
            let post = [0.16, 0.15, 0.17, k];
            let (w, hgt) = (0.7, 15.0);
            let mut v = vec![
                at(-w, hgt, 0.5, 0.5, post),
                at(w, hgt, 0.5, 0.5, post),
                at(w, 0.0, 0.5, 0.5, post),
                at(-w, hgt, 0.5, 0.5, post),
                at(w, 0.0, 0.5, 0.5, post),
                at(-w, 0.0, 0.5, 0.5, post),
            ];
            let mut blob = |r: f32, c: [f32; 4]| {
                let (a, b, cc, d) = (
                    at(-r, hgt + 1.0 + r, 0.0, 0.0, c),
                    at(r, hgt + 1.0 + r, 1.0, 0.0, c),
                    at(r, hgt + 1.0 - r, 1.0, 1.0, c),
                    at(-r, hgt + 1.0 - r, 0.0, 1.0, c),
                );
                v.extend_from_slice(&[a, b, cc, a, cc, d]);
            };
            blob(9.0, [1.0, 0.75, 0.40, 0.40 * k]);
            blob(2.6, [1.0, 0.95, 0.80, k]);
            out.push((key, v));
        }
        out
    }

    /// Fireflies: small green-gold lights drifting and blinking over the rough, the woods and near the water, a few per
    /// tile where they live (drawn over the course; `k` how many are out).
    pub(crate) fn fx_fireflies_draw(&mut self, g: &mut Gfx, k: f32) {
        let t = &self.terrain;
        let (w, h) = (self.draw_w, self.draw_h);
        let m = self.mv;
        let upp = self.upp.max(1e-6);
        let vscale = crate::ui::view_scale(w, h);
        let seed = self.fx_seed();
        let time = self.fx.time;
        let tw = t.w.max(0) as usize;
        let mut v: Vec<Vert> = Vec::new();
        let (sx, sy) = (2.0 / w, 2.0 / h);
        for y in 0..t.h {
            for x in 0..t.w {
                let ty = t.type_at(x, y);
                let near_water = self.fx.water_near.get(y as usize * tw + x as usize).copied().flatten().is_some();
                let home = matches!(ty, 4 | 5 | 11 | 13..=16 | 18 | 19) || (near_water && ty != 17 && ty != 20);
                if !home {
                    continue;
                }
                let h0 = hash(&[seed, x as u32, y as u32, 0x6666]);
                let n = if unit(h0) < 0.16 * k { 1 + (h0 & 1) } else { 0 };
                let (cx, cz) = t.tile_centre(x, y);
                for i in 0..n {
                    let r = |j: u32| unit(hash(&[h0, i, j]));
                    let (f1, f2) = (0.18 + 0.2 * r(1), 0.15 + 0.2 * r(2));
                    let wx = cx + ((time * f1 + r(3) * 6.3).sin() * 0.45 + (time * f2 * 2.3 + r(4) * 6.3).sin() * 0.15) * TILE_SIZE;
                    let wz = cz + ((time * f2 + r(5) * 6.3).cos() * 0.45 + (time * f1 * 1.7 + r(6) * 6.3).cos() * 0.15) * TILE_SIZE;
                    let gy = t.height_at(wx, wz);
                    let ex = m[0] * wx + m[4] * gy + m[8] * wz + m[12];
                    let ey = m[1] * wx + m[5] * gy + m[9] * wz + m[13];
                    let lift = (8.0 + 10.0 * r(7) + 3.0 * (time * 0.7 + r(8) * 6.3).sin()) * vscale;
                    let (px, py) = (ex / upp + w * 0.5, h * 0.5 - ey / upp - lift);
                    if px < -20.0 || py < -20.0 || px > w + 20.0 || py > h + 20.0 {
                        continue;
                    }
                    // a slow glow up and down, dark most of the time
                    let blink = smoothstep(0.35, 1.0, (time * (0.7 + 0.6 * r(9)) + r(10) * 6.3).sin());
                    if blink <= 0.01 {
                        continue;
                    }
                    let mut put = |rad: f32, c: [f32; 4]| {
                        let p =
                            |dx: f32, dy: f32, tu: f32, tv: f32| Vert::new((px + dx) * sx - 1.0, 1.0 - (py + dy) * sy, 0.0, tu, tv).col(c);
                        v.extend_from_slice(&[
                            p(-rad, -rad, 0.0, 0.0),
                            p(rad, -rad, 1.0, 0.0),
                            p(rad, rad, 1.0, 1.0),
                            p(-rad, -rad, 0.0, 0.0),
                            p(rad, rad, 1.0, 1.0),
                            p(-rad, rad, 0.0, 1.0),
                        ]);
                    };
                    put(6.5 * vscale, [0.70, 1.0, 0.30, 0.45 * blink * k]);
                    put(1.6 * vscale, [0.95, 1.0, 0.65, blink * k]);
                }
            }
        }
        if v.is_empty() {
            return;
        }
        let dot = crate::fx::gpu(g).dot;
        let u = Uniforms::flat(&crate::gfx::Mat4::identity(), &crate::gfx::Mat4::identity());
        g.tris(Mode::Flat, Some(dot), &u, &v);
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn a_framed_pane_glows() {
        // a light wall with a dark 2 x 2 pane in it
        let mut img = Rgba::new(10, 10);
        for (k, p) in img.px.chunks_exact_mut(4).enumerate() {
            let (x, y) = (k % 10, k / 10);
            let pane = (4..6).contains(&x) && (4..6).contains(&y);
            p.copy_from_slice(&if pane { [30, 34, 44, 255] } else { [230, 215, 170, 255] });
        }
        // some salt lights it (four windows in five are lit)
        let lit = (0..20).filter_map(|s| window_glow(&img, s)).next().expect("a lit window");
        let a = |x: usize, y: usize| lit.px[((y + GLOW_PAD) * lit.w as usize + x + GLOW_PAD) * 4 + 3];
        assert!(a(4, 4) > 180);
        assert!(a(0, 0) < 40);
        assert_eq!(lit.px[3], 0);
        // a plain wall has no windows
        let mut wall = Rgba::new(6, 6);
        for p in wall.px.chunks_exact_mut(4) {
            p.copy_from_slice(&[230, 215, 170, 255]);
        }
        assert!(window_glow(&wall, 1).is_none());
    }
}
