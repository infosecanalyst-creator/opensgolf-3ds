//! Enhanced graphics' climates and winter (docs/ENHANCED.md, "Climate" and "Winter"): each property has its own weather
//! (how often it rains, how likely a snowy end of winter), and each year rolls its own: wetter or drier, with a cold snap or
//! without. The game's year runs from March to October, so winter is only its first days: in a year with a cold snap, where
//! the place gets snow, the rain falls as snow, the morning frost is full, and on some days a light dusting lies at dawn
//! and melts by mid-morning (with the time of day). Drawing only: what varies
//! comes from hashes of the course and the day, never from the game's random generator; golfers play as ever.
use crate::app::App;
use crate::fx_light::Atmos;

/// A property's weather: how many days in a hundred it rains, and how likely a year is to end its winter with a snowy cold
/// snap in early March. By the place on the world map (land::RECORDS order); a property a mod adds takes its theme's.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct Climate {
    pub rain: f32,
    pub snow: f32,
}

const fn cl(rain: f32, snow: f32) -> Climate {
    Climate { rain, snow }
}

/// Monterey, San Diego, Rocky Mtns., Las Vegas, Phoenix, Hawaii, Oahu, Nova Scotia, Northeast, Carolina, Ireland, Scotland,
/// Wales, Spain, Florida, Jamaica. Deserts and the tropics never see snow; the desert's rain is a rare shower.
pub const CLIMATES: [Climate; 16] = [
    cl(0.20, 0.0),
    cl(0.12, 0.0),
    cl(0.25, 0.7),
    cl(0.04, 0.0),
    cl(0.05, 0.0),
    cl(0.30, 0.0),
    cl(0.30, 0.0),
    cl(0.35, 0.7),
    cl(0.30, 0.6),
    cl(0.28, 0.15),
    cl(0.45, 0.2),
    cl(0.45, 0.4),
    cl(0.45, 0.25),
    cl(0.12, 0.05),
    cl(0.33, 0.0),
    cl(0.28, 0.0),
];

/// By theme (the exe's order: parkland, desert, tropical, links), for a property not in the table.
const THEME_CLIMATES: [Climate; 4] = [cl(0.25, 0.3), cl(0.04, 0.0), cl(0.30, 0.0), cl(0.40, 0.3)];
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

/// How wintry month `m` (3.0 the start of March .. 11.0 the end of October) is: 1 in the first week of March, nothing by
/// the end of the month's first half (a late cold snap; late October is cold mornings, not snow).
pub fn cold(m: f32) -> f32 {
    1.0 - smoothstep(3.15, 3.6, m)
}

impl App {
    /// The course's climate: its property's, else its theme's.
    pub(crate) fn fx_climate(&self) -> Climate {
        let theme = self.exe_theme().min(3) as usize;
        let c = self.land.as_ref().and_then(|l| CLIMATES.get(l.slot.property)).copied().unwrap_or(THEME_CLIMATES[theme]);
        // never snow in the desert or the tropics, whatever a property says
        if theme == 1 || theme == 2 {
            Climate { snow: 0.0, ..c }
        } else {
            c
        }
    }

    /// The game year now (0 the first): March to October is one year.
    fn fx_year(&self) -> u32 {
        self.econ.tick / (crate::fx_light::DAY_TICKS * 8)
    }

    /// How wet this year is against the place's usual: from half as many rainy days to half as many again.
    pub(crate) fn fx_wet_year(&self) -> f32 {
        0.5 + hash(self.fx_seed() ^ 0x7e7, self.fx_year())
    }

    /// Whether this year ends its winter with a snowy cold snap (the place's chance, a fresh roll each year).
    pub(crate) fn fx_snowy_year(&self) -> bool {
        hash(self.fx_seed() ^ 0x5e0, self.fx_year()) < self.fx_climate().snow
    }

    /// The day's late-winter weather into the atmosphere, in early March only (late October keeps its frosty mornings,
    /// fx_nature.rs): the rain falls as snow, and on some days a light dusting lies at dawn and melts by mid-morning.
    pub(crate) fn fx_winter(&self, a: &mut Atmos, day: u32, phase: Option<f32>) {
        let k = cold(self.fx_month());
        if k <= 0.0 {
            return;
        }
        // SG_FX_SNOW=S (stills): that much snow lying whatever the hour or the year
        let forced = std::env::var("SG_FX_SNOW").ok().and_then(|v| v.parse::<f32>().ok());
        // only in a year with a cold snap, where the place gets snow at all; other years keep the frosty mornings
        if forced.is_none() && !self.fx_snowy_year() {
            return;
        }
        // a dusting on about a third of the cold days, only with the time of day: from dawn, gone by mid-morning, so the
        // course reads clearly while it is built
        let dusting = hash(self.fx_seed() ^ 0x5a0, day) < 0.35;
        let morning = phase.map(|p| if p > 0.95 { smoothstep(0.95, 1.0, p) } else { 1.0 - smoothstep(0.06, 0.22, p) }).unwrap_or(0.0);
        a.snow = match forced {
            Some(f) => f.clamp(0.0, 0.67),
            None if dusting => (k * morning * (0.35 + 0.25 * hash(self.fx_seed(), day))).clamp(0.0, 0.6),
            None => 0.0,
        };
        // the rain falls as snow, and the ground is not wet
        a.snowfall = a.rain * k;
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
        // no snow at the end of October
        assert_eq!(cold(10.95), 0.0);
    }
}
