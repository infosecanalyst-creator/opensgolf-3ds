//! Enhanced graphics' winter (docs/ENHANCED.md, "Winter"): the game's year runs from March to October, and on parkland and
//! links courses its first and last weeks are cold: the rain falls as snow and the morning frost is full, and on some days
//! of the last week a light snow lies on the level ground (thinning in the midday sun with the time of day). Drawing only: what varies
//! comes from hashes of the course and the day, never from the game's random generator; golfers play as ever.
use crate::app::App;
use crate::fx_light::Atmos;
use crate::gfx::{Gfx, Mode, Uniforms, Vert};

fn hash(a: u32, b: u32) -> f32 {
    let mut h = a.wrapping_mul(0x9e37_79b9) ^ b.wrapping_mul(0x85eb_ca6b);
    h ^= h >> 15;
    h = h.wrapping_mul(0x2c1b_3c6d);
    h ^= h >> 12;
    (h >> 8) as f32 / (1u32 << 24) as f32
}

fn smoothstep(e0: f32, e1: f32, x: f32) -> f32 {
    let t = ((x - e0) / (e1 - e0)).clamp(0.0, 1.0);
    t * t * (3.0 - 2.0 * t)
}

/// How cold month `m` (3.0 the start of March .. 11.0 the end of October) is: 1 in the first and last week of the year.
pub fn cold(m: f32) -> f32 {
    (1.0 - smoothstep(3.15, 3.6, m)).max(smoothstep(10.4, 10.85, m))
}

impl App {
    /// The day's winter weather into the atmosphere: snow cover, snow instead of rain, full frost.
    pub(crate) fn fx_winter(&self, a: &mut Atmos, day: u32, phase: Option<f32>) {
        let theme = self.exe_theme();
        if theme != 0 && theme != 3 {
            return;
        }
        let k = cold(self.fx_month());
        if k <= 0.0 {
            return;
        }
        // snow lies only at the year's end: a new game starts in early March, and its first holes are built on a course
        // that has to read clearly (March keeps the frost and the odd snow shower)
        let end = self.fx_month() > 10.0;
        // SG_FX_SNOW=S (stills): that much snow lying whatever the day
        let forced = std::env::var("SG_FX_SNOW").ok().and_then(|v| v.parse::<f32>().ok());
        // about half the cold days have snow lying
        let lying =
            forced.unwrap_or_else(|| if hash(self.fx_seed() ^ 0x5a0, day) < 0.55 { 0.5 + 0.5 * hash(self.fx_seed(), day) } else { 0.0 });
        // thinner when the sun is up
        let sun = phase.map(|p| 1.0 - 0.45 * smoothstep(0.25, 0.55, p) * (1.0 - smoothstep(0.75, 0.95, p))).unwrap_or(1.0);
        // at most two thirds cover, so the fairways, greens and paths always show through
        a.snow = if end || forced.is_some() { (k * lying * sun * 0.67).clamp(0.0, 0.67) } else { 0.0 };
        // the rain falls as snow, and the ground is not wet
        a.snowfall = (a.rain * k).max(if forced.is_some() && a.rain > 0.0 { a.rain } else { 0.0 });
        a.rain *= 1.0 - k;
        a.wet *= 1.0 - k;
        a.puddles *= 1.0 - k;
        if a.snow > 0.0 || a.snowfall > 0.0 {
            a.season.frost = a.season.frost.max(k * 0.6);
        }
    }

    /// Falling snow: small square flakes drifting down the screen and swaying, pale, in the art's pixel style.
    pub(crate) fn fx_snow_draw(&mut self, g: &mut Gfx, u: &Uniforms, k: f32) {
        let (w, h) = (self.draw_w, self.draw_h);
        let vscale = (w / 800.0).min(h / 600.0);
        let n = ((w * h) / (1920.0 * 1080.0) * 900.0 * k).clamp(30.0, 2500.0) as u32;
        let seed = self.fx_seed();
        let t = self.fx.time;
        let (sx, sy) = (2.0 / w, 2.0 / h);
        let wind = self.fx.atmos.wind;
        let drift = if wind.k > 0.0 { 40.0 * wind.k * wind.screen_x } else { 8.0 };
        let mut v: Vec<Vert> = Vec::with_capacity(n as usize * 6);
        for i in 0..n {
            let r = |j: u32| hash(seed ^ i.wrapping_mul(0x27d4_eb2d), j);
            let speed = 0.08 + 0.06 * r(3);
            let f = (r(2) + t * speed).fract();
            let y = f * (h + 20.0) - 10.0;
            let x = (r(0) * w + drift * vscale * f * 10.0 + (t * (0.6 + r(5)) + r(6) * 9.0).sin() * 6.0 * vscale).rem_euclid(w);
            // whole pixels of the art: one or two of them
            let size = if r(4) < 0.3 { 2.0 } else { 1.0 } * vscale.max(1.0).round();
            let (x, y) = ((x / size).round() * size, (y / size).round() * size);
            let c = [0.97, 0.98, 1.0, 0.85];
            let p = |px: f32, py: f32| Vert::new(px * sx - 1.0, 1.0 - py * sy, 0.0, 0.0, 0.0).col(c);
            let (a, b, cc, d) = (p(x, y), p(x + size, y), p(x + size, y + size), p(x, y + size));
            v.extend_from_slice(&[a, b, cc, a, cc, d]);
        }
        g.tris(Mode::Flat, None, u, &v);
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn cold_weeks() {
        assert_eq!(cold(3.0), 1.0);
        assert_eq!(cold(6.0), 0.0);
        assert_eq!(cold(10.95), 1.0);
    }
}
