//! Enhanced graphics' wind and seasons (docs/ENHANCED.md, "Wind" and "Seasons"). Drawing only: nothing here touches the
//! simulation, and what varies (the day's wind, which leaves turn first) comes from render-side hashes seeded from the save,
//! never from the game's random generator.
//!
//! **Wind.** Each day (one game month, fx_light.rs) has a wind: a direction and a strength from a hash of the course and the
//! day, with gusts running across the course along it. Trees, palms, bushes and tall grasses lean and sway with it: the
//! sprite is drawn as a few horizontal strips whose corners are moved along the screen's right, more the higher they are
//! (the ground point stays where it is), so the art bends like a stem instead of sliding. Flags fly downwind: the art
//! (whose flag always points right) is mirrored about its pole when the wind blows to the left of the screen. The rain's
//! slant and the clouds' drift follow the same wind (fx_light.rs). Hit-testing and the drawing order are untouched: only the
//! drawn corners move, a few pixels at most.
//!
//! **Seasons.** The game's year runs from March to October. Deciduous trees get blossom flecks in March and April, fresher
//! greens in spring and turn orange, red and gold in September and October (the leaves turn patchily, a few at a time);
//! conifers only freshen in spring, palms and cacti do not change. That is a recoloured copy of each tree frame (foliage
//! picked by hue), made when the season moves on by an eighth and cached. The grass gets fresher or a little golden in the
//! ground's shader, which also lays frost and dew sparkle on early mornings in spring and autumn with the time of day.
use crate::app::App;
use crate::gfx::Gfx;
use miniquad::*;
use sg_core::assets::Rgba;
use sg_core::terrain::TILE_SIZE;
use std::collections::HashMap;

/// What a sprite is, for the wind, the seasons and the evening lights (from its file's name).
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Kind {
    Other,
    /// Broadleaf trees: they blossom a little in spring and turn in autumn.
    Deciduous,
    /// Flowering trees (dogwood, peach, plum): full of blossom in spring.
    Blossom,
    Conifer,
    Palm,
    Cactus,
    Bush,
    /// Tall grasses, reeds, bamboo, small flowers.
    Grass,
    Flag,
    Building,
}

/// A sprite's kind from its file (the path under Flics/, any case).
pub fn kind_of(rel: &str) -> Kind {
    let n = rel.to_ascii_lowercase();
    let has = |k: &str| n.contains(k);
    if has("shadow") {
        return Kind::Other;
    }
    if n.starts_with("tees/") {
        return if has("flag") { Kind::Flag } else { Kind::Other };
    }
    if n.starts_with("bldgs/") || n.starts_with("homes/") {
        return if has("flag") {
            Kind::Flag
        } else if has("sign") || has("fence") || has("wall") || has("grill") || has("bag") || has("cars") {
            Kind::Other
        } else {
            Kind::Building
        };
    }
    if n.starts_with("landmarks/") {
        return if ["chapel", "chinesehouse", "lighthouse", "windmill", "barn", "pagoda", "wmill"].iter().any(|k| has(k)) {
            Kind::Building
        } else {
            Kind::Other
        };
    }
    if has("cactus") || has("joshua") {
        return Kind::Cactus;
    }
    if has("palm") || has("drac_") {
        return Kind::Palm;
    }
    if has("pine") || has("fir") || has("spruce") || has("cypress") || has("aerie") {
        return Kind::Conifer;
    }
    if has("dogwood") || has("peach") || has("plum") {
        return Kind::Blossom;
    }
    if has("maple")
        || has("willow")
        || has("elm")
        || has("aspen")
        || has("japanesetree")
        || has("scenic_tree")
        || has("mesquite")
        || has("linkstree")
        || has("cerc_")
        || has("mangrove")
    {
        return Kind::Deciduous;
    }
    if has("brush") || has("rosebush") || has("rhody") || has("bougainvillea") || has("vineyard") || has("heather") {
        return Kind::Bush;
    }
    if has("bamboo")
        || has("cattails")
        || has("ograss")
        || has("drygrass")
        || has("dandelion")
        || has("purpleflowers")
        || has("whiteflowers")
        || has("crabgrass")
    {
        return Kind::Grass;
    }
    Kind::Other
}

/// The kinds of the loaded sprites, and the recoloured (seasonal) tree frames.
#[derive(Default)]
pub struct NatureState {
    kinds: Vec<Kind>,
    /// (sprites, sprite_index) lengths the kinds were made for.
    made_for: (usize, usize),
    /// (sprite, frame index) -> (the classic texture it was made from, the season it shows, the recoloured texture).
    season_tex: HashMap<(usize, usize), (TextureId, [u8; 4], TextureId)>,
    /// (sprite, frame index) -> the flag's art points right of its pole.
    flag_right: HashMap<(usize, usize), bool>,
}

/// The day's wind: towards where it blows (unit, world x and z), its strength (0 calm to 1 a strong breeze) and how much of
/// it is across the screen (-1 to the left .. 1 to the right).
#[derive(Clone, Copy, Debug, Default)]
pub struct Wind {
    pub dir: [f32; 2],
    pub k: f32,
    pub screen_x: f32,
}

/// The seasons' weights now (0..1 each, already scaled for the theme).
#[derive(Clone, Copy, Debug, Default, PartialEq)]
pub struct Season {
    /// Fresh spring greens.
    pub fresh: f32,
    /// Blossom on the trees.
    pub blossom: f32,
    /// Autumn colours.
    pub autumn: f32,
    /// Frost on the grass (early mornings, with the time of day).
    pub frost: f32,
    /// Dew sparkle (early mornings, with the time of day).
    pub dew: f32,
    /// Snow lying on the trees and roofs (fx_winter.rs).
    pub snow: f32,
}

/// How a sprite is bent this frame: its top moved `lean` times its height along the screen's right (the bend grows as
/// height^curve), and mirrored about its ground point (a flag flying the other way).
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct Bend {
    pub lean: f32,
    pub curve: f32,
    pub flip: bool,
}

/// A sprite's corners in strips: each quad's four corners (x right, y up, both relative to the ground point; texture u, v)
/// for a picture spanning l..r and b..t (b below the ground point, t above it). Without a bend, the plain quad.
pub fn strips(l: f32, r: f32, t: f32, b: f32, bend: Bend) -> Vec<[(f32, f32, f32, f32); 4]> {
    let (l, r, ul, ur) = if bend.flip { (-r, -l, 1.0, 0.0) } else { (l, r, 0.0, 1.0) };
    let tv = |y: f32| if t - b > 0.0 { (t - y) / (t - b) } else { 0.0 };
    let off = |y: f32| if y > 0.0 && t > 0.0 { bend.lean * t * (y / t).powf(bend.curve) } else { 0.0 };
    let mut rows: Vec<f32> = Vec::with_capacity(7);
    if bend.lean != 0.0 && t > 0.0 {
        for k in [1.0, 0.72, 0.47, 0.24] {
            rows.push(t * k);
        }
        if b < 0.0 {
            rows.push(0.0);
        }
    } else {
        rows.push(t);
    }
    rows.push(b);
    rows.windows(2)
        .map(|w| {
            let (y0, y1) = (w[0], w[1]);
            let (o0, o1) = (off(y0), off(y1));
            [(l + o0, y0, ul, tv(y0)), (r + o0, y0, ur, tv(y0)), (r + o1, y1, ur, tv(y1)), (l + o1, y1, ul, tv(y1))]
        })
        .collect()
}

fn hash3(a: u32, b: u32, c: u32) -> u32 {
    let mut h = a.wrapping_mul(0x9e37_79b9) ^ b.wrapping_mul(0x85eb_ca6b) ^ c.wrapping_mul(0xc2b2_ae35);
    h ^= h >> 15;
    h = h.wrapping_mul(0x2c1b_3c6d);
    h ^= h >> 12;
    h = h.wrapping_mul(0x297a_2d39);
    h ^= h >> 15;
    h
}

fn unit(h: u32) -> f32 {
    (h >> 8) as f32 / (1u32 << 24) as f32
}

/// Smooth 1D value noise (0..1).
fn vnoise(x: f32, salt: u32) -> f32 {
    let i = x.floor();
    let f = x - i;
    let f = f * f * (3.0 - 2.0 * f);
    let a = unit(hash3(i as i32 as u32, salt, 7));
    let b = unit(hash3((i as i32 + 1) as u32, salt, 7));
    a + (b - a) * f
}

fn smoothstep(a: f32, b: f32, x: f32) -> f32 {
    let t = ((x - a) / (b - a)).clamp(0.0, 1.0);
    t * t * (3.0 - 2.0 * t)
}

/// How a kind moves in the wind: (lean at full wind, sway, sway frequency in Hz, bend curve).
fn sway_params(k: Kind) -> Option<(f32, f32, f32, f32)> {
    Some(match k {
        Kind::Deciduous | Kind::Blossom => (0.040, 0.012, 0.55, 1.6),
        Kind::Conifer => (0.024, 0.008, 0.45, 1.8),
        Kind::Palm => (0.075, 0.024, 0.38, 2.2),
        Kind::Bush => (0.035, 0.018, 0.85, 1.3),
        Kind::Grass => (0.090, 0.040, 1.20, 1.2),
        _ => return None,
    })
}

/// The season's weights at calendar month `m` (3.0 the start of March .. 11.0 the end of October) for `theme` (the exe's
/// order: parkland, desert, tropical, links), and the mornings' frost and dew at day phase `phase` (None: no time of day).
pub fn season_at(m: f32, theme: u8, phase: Option<f32>) -> Season {
    // per theme: autumn colours, blossom, freshness, frost
    let (ka, kb, kf, kc) = match theme {
        1 => (0.35, 0.3, 0.5, 0.5),
        2 => (0.0, 0.0, 0.4, 0.0),
        3 => (0.85, 0.6, 1.0, 1.0),
        _ => (1.0, 1.0, 1.0, 1.0),
    };
    let fresh = 1.0 - smoothstep(4.6, 5.8, m);
    let blossom = smoothstep(2.9, 3.25, m) * (1.0 - smoothstep(4.3, 5.0, m));
    let autumn = smoothstep(8.7, 10.3, m);
    let mut s = Season { fresh: fresh * kf, blossom: blossom * kb, autumn: autumn * ka, frost: 0.0, dew: 0.0, snow: 0.0 };
    if let Some(p) = phase {
        // early morning: from dawn to about mid morning
        let morning = if p > 0.95 { smoothstep(0.95, 1.0, p) } else { 1.0 - smoothstep(0.07, 0.2, p) };
        let cold = (1.0 - smoothstep(3.6, 4.6, m)).max(smoothstep(9.4, 10.6, m));
        let mild = (1.0 - smoothstep(4.6, 5.6, m)).max(smoothstep(8.4, 9.4, m));
        s.frost = cold * morning * kc;
        s.dew = mild * morning;
    }
    s
}

/// The season as cache keys of eighths: (fresh, blossom, autumn, snow), for a frame of `kind`.
fn season_key(s: &Season, kind: Kind) -> [u8; 4] {
    let q = |v: f32| (v.clamp(0.0, 1.0) * 8.0).round() as u8;
    match kind {
        Kind::Deciduous | Kind::Blossom => [q(s.fresh), q(s.blossom), q(s.autumn), q(s.snow)],
        Kind::Conifer => [q(s.fresh * 0.5), 0, 0, q(s.snow)],
        Kind::Bush | Kind::Building => [0, 0, 0, q(s.snow)],
        _ => [0; 4],
    }
}

/// Snow on a frame (`snow` 0..1): every column's top pixels turn white (a cap on a crown, a ridge on a roof), deeper with
/// more snow, and on trees a dusting over the lit, upper part of the foliage. White keeps the art's own shading, a little
/// blue in its darker pixels, so the sprite still reads.
fn snow_on(img: &mut Rgba, kind: Kind, snow: f32, salt: u32) {
    if snow <= 0.0 {
        return;
    }
    let (w, h) = (img.w as usize, img.h as usize);
    let tree = matches!(kind, Kind::Deciduous | Kind::Blossom | Kind::Conifer | Kind::Bush);
    let depth = (1.0 + snow * if tree { 3.0 } else { 2.0 }).round() as usize;
    let whiten = |p: &mut [u8], k: f32| {
        let l = (0.299 * p[0] as f32 + 0.587 * p[1] as f32 + 0.114 * p[2] as f32) / 255.0;
        let v = (0.62 + 0.55 * l).min(1.0);
        let c = [0.93 + 0.07 * l, 0.95 + 0.05 * l, 1.0];
        for i in 0..3 {
            p[i] = (p[i] as f32 + (c[i] * v * 255.0 - p[i] as f32) * k).round().clamp(0.0, 255.0) as u8;
        }
    };
    // the top of each column, and of each run of pixels below a gap (the tiers of a pine, a roof's lower eaves)
    for x in 0..w {
        let mut run = 0usize;
        let mut above_clear = true;
        for y in 0..h {
            let k = (y * w + x) * 4;
            if img.px[k + 3] == 0 {
                above_clear = true;
                run = 0;
                continue;
            }
            if above_clear {
                run = depth;
                above_clear = false;
            }
            if run > 0 {
                let edge = if run == depth { 1.0 } else { 0.85 };
                whiten(&mut img.px[k..k + 4], edge * snow.clamp(0.6, 1.0));
                run -= 1;
            }
        }
    }
    // roofs: the art paints them in darker, greyer or browner shades than the light walls; in the upper part of the
    // building those pixels take the snow, as do the tops of each column (the ridges, spires and eaves)
    if kind == Kind::Building {
        let limit = h * 55 / 100;
        for y in 0..limit {
            for x in 0..w {
                let k = (y * w + x) * 4;
                if img.px[k + 3] == 0 {
                    continue;
                }
                let (r, g, b) = (img.px[k] as f32 / 255.0, img.px[k + 1] as f32 / 255.0, img.px[k + 2] as f32 / 255.0);
                let (_, sat, v) = rgb_to_hsv(r, g, b);
                if v < 0.62 && sat < 0.5 {
                    let k2 = (1.0 - v / 0.62).min(0.6) + 0.4;
                    whiten(&mut img.px[k..k + 4], (0.9 * snow.clamp(0.5, 1.0) * k2).min(0.9));
                }
            }
        }
    }
    // a dusting over the upper two thirds of a tree's lit foliage
    if tree {
        for y in 0..h * 2 / 3 {
            for x in 0..w {
                let k = (y * w + x) * 4;
                if img.px[k + 3] == 0 {
                    continue;
                }
                let l = (0.299 * img.px[k] as f32 + 0.587 * img.px[k + 1] as f32 + 0.114 * img.px[k + 2] as f32) / 255.0;
                let hp = unit(hash3(x as u32 >> 1, y as u32, salt ^ 0x5a0));
                if l > 0.32 && hp < snow * 0.45 {
                    whiten(&mut img.px[k..k + 4], 0.8);
                }
            }
        }
    }
}

fn rgb_to_hsv(r: f32, g: f32, b: f32) -> (f32, f32, f32) {
    let mx = r.max(g).max(b);
    let mn = r.min(g).min(b);
    let d = mx - mn;
    let h = if d <= 1e-6 {
        0.0
    } else if mx == r {
        60.0 * ((g - b) / d).rem_euclid(6.0)
    } else if mx == g {
        60.0 * ((b - r) / d + 2.0)
    } else {
        60.0 * ((r - g) / d + 4.0)
    };
    (h, if mx > 0.0 { d / mx } else { 0.0 }, mx)
}

fn hsv_to_rgb(h: f32, s: f32, v: f32) -> (f32, f32, f32) {
    let h = h.rem_euclid(360.0) / 60.0;
    let c = v * s;
    let x = c * (1.0 - (h % 2.0 - 1.0).abs());
    let (r, g, b) = match h as i32 {
        0 => (c, x, 0.0),
        1 => (x, c, 0.0),
        2 => (0.0, c, x),
        3 => (0.0, x, c),
        4 => (x, 0.0, c),
        _ => (c, 0.0, x),
    };
    let m = v - c;
    (r + m, g + m, b + m)
}

/// A tree frame in the season `key` (eighths of freshness, blossom and autumn); `salt` varies the colours between kinds of
/// tree. Only foliage (green hues) changes; trunks, shadows and the transparent key stay.
pub fn recolour(img: &Rgba, kind: Kind, key: [u8; 4], salt: u32) -> Rgba {
    let mut out = recolour_leaves(img, kind, [key[0], key[1], key[2]], salt);
    snow_on(&mut out, kind, key[3] as f32 / 8.0, salt);
    out
}

fn recolour_leaves(img: &Rgba, kind: Kind, key: [u8; 3], salt: u32) -> Rgba {
    let (fresh, blossom, autumn) = (key[0] as f32 / 8.0, key[1] as f32 / 8.0, key[2] as f32 / 8.0);
    let tsalt = unit(hash3(salt, 3, 9));
    let mut out = img.clone();
    let w = img.w as usize;
    for (k, p) in out.px.chunks_exact_mut(4).enumerate() {
        if p[3] == 0 {
            continue;
        }
        let (x, y) = ((k % w) as u32, (k / w) as u32);
        let (r, g, b) = (p[0] as f32 / 255.0, p[1] as f32 / 255.0, p[2] as f32 / 255.0);
        let (mut h, mut s, mut v) = rgb_to_hsv(r, g, b);
        if !(48.0..=178.0).contains(&h) || s < 0.12 || v < 0.04 {
            continue;
        }
        let hp = unit(hash3(x, y, salt));
        // blossom: flecks in small clusters, shaded by the leaf they sit on
        if blossom > 0.0 {
            let dens = match kind {
                Kind::Blossom => 0.32,
                Kind::Deciduous => 0.05 + 0.08 * tsalt,
                _ => 0.0,
            };
            let hc = unit(hash3(x >> 1, y >> 1, salt ^ 0x5bd1));
            if hc < dens * blossom && v > 0.18 {
                let c = if (kind == Kind::Blossom && salt & 1 == 0) || tsalt < 0.5 { [1.0, 0.72, 0.84] } else { [1.0, 0.96, 0.95] };
                let shade = 0.62 + 0.55 * v.min(0.7) + 0.08 * hp;
                p[0] = (c[0] * shade * 255.0).clamp(0.0, 255.0) as u8;
                p[1] = (c[1] * shade * 255.0).clamp(0.0, 255.0) as u8;
                p[2] = (c[2] * shade * 255.0).clamp(0.0, 255.0) as u8;
                continue;
            }
        }
        // spring: a fresher, slightly yellower green
        h -= 9.0 * fresh;
        s = (s * (1.0 + 0.12 * fresh)).min(1.0);
        v = (v * (1.0 + 0.07 * fresh)).min(1.0);
        // autumn: the leaves turn a few at a time, darker ones red, lighter ones gold
        if autumn > 0.0 && matches!(kind, Kind::Deciduous | Kind::Blossom) {
            let turn = smoothstep(hp * 0.75, hp * 0.75 + 0.25, autumn);
            let pick = (0.35 * v + 0.25 * unit(hash3(x >> 1, y >> 1, salt ^ 0x77)) + 0.75 * tsalt - 0.22).clamp(0.0, 1.0);
            let target = 3.0 + 50.0 * pick;
            // the short way round the hue circle
            let d = (target - h + 540.0).rem_euclid(360.0) - 180.0;
            h += d * turn;
            s = (s * (1.0 + 0.45 * turn) + 0.08 * turn).min(1.0);
            v = (v * (1.0 + 0.32 * turn) + 0.05 * turn).min(1.0);
        }
        let (r, g, b) = hsv_to_rgb(h, s, v);
        p[0] = (r * 255.0).round().clamp(0.0, 255.0) as u8;
        p[1] = (g * 255.0).round().clamp(0.0, 255.0) as u8;
        p[2] = (b * 255.0).round().clamp(0.0, 255.0) as u8;
    }
    out
}

/// A hash of a string (a sprite's file), for per-tree colour variation.
fn name_hash(s: &str) -> u32 {
    s.bytes().fold(0x811c_9dc5u32, |h, b| (h ^ b as u32).wrapping_mul(0x0100_0193))
}

impl App {
    /// The kind of sprite `si` (its file's name, cached).
    pub(crate) fn fx_kind(&mut self, si: usize) -> Kind {
        let n = &mut self.fx.nature;
        if n.made_for != (self.sprites.len(), self.sprite_index.len()) {
            n.kinds = vec![Kind::Other; self.sprites.len()];
            for (key, v) in &self.sprite_index {
                if let Some(i) = *v {
                    if i < n.kinds.len() {
                        n.kinds[i] = kind_of(key.split('|').next().unwrap_or(""));
                    }
                }
            }
            n.made_for = (self.sprites.len(), self.sprite_index.len());
        }
        n.kinds.get(si).copied().unwrap_or(Kind::Other)
    }

    /// The colour salt of sprite `si` (its file and palette).
    fn fx_salt(&self, si: usize) -> u32 {
        self.sprite_index.iter().find(|(_, v)| **v == Some(si)).map(|(k, _)| name_hash(k)).unwrap_or(si as u32)
    }

    /// The day's wind (render-side hash of the course and the day; rainy days are windier). `SG_FX_WIND=strength` or
    /// `SG_FX_WIND=strength,degrees` (0 blows to the screen's right, 90 away from the viewer) fixes it for stills.
    pub(crate) fn fx_wind_of(&self, day: u32, rain: f32, rot: &[f32; 16]) -> Wind {
        let h = crate::fx_light::hash_of(&[self.fx_seed(), day, 0x7769_6e64]);
        let mut k = 0.12 + 0.75 * unit(h).powf(1.4) + 0.25 * rain;
        let (r, u) = ([rot[0], rot[8]], [rot[1], rot[9]]);
        let mut dir = {
            let a = unit(hash3(h, 1, 2)) * std::f32::consts::TAU;
            [a.cos(), a.sin()]
        };
        if let Ok(v) = std::env::var("SG_FX_WIND") {
            let p: Vec<f32> = v.split(',').filter_map(|x| x.trim().parse().ok()).collect();
            if let Some(&s) = p.first() {
                k = s;
            }
            if let Some(&deg) = p.get(1) {
                // relative to the screen: along its right, turned towards its top
                let (c, s) = (deg.to_radians().cos(), deg.to_radians().sin());
                let ul = u[0].hypot(u[1]).max(1e-4);
                let rl = r[0].hypot(r[1]).max(1e-4);
                let d = [r[0] / rl * c + u[0] / ul * s, r[1] / rl * c + u[1] / ul * s];
                let l = d[0].hypot(d[1]).max(1e-4);
                dir = [d[0] / l, d[1] / l];
            }
        }
        let rl = r[0].hypot(r[1]).max(1e-4);
        let screen_x = (dir[0] * r[0] + dir[1] * r[1]) / rl;
        Wind { dir, k: k.clamp(0.0, 1.2), screen_x }
    }

    /// The calendar month now (3.0 the start of March .. 11.0 the end of October), from the game's calendar tick.
    /// `SG_FX_MONTH` fixes it for stills (e.g. 10 for early October, 4.5 for mid April).
    pub(crate) fn fx_month(&self) -> f32 {
        if let Some(m) = std::env::var("SG_FX_MONTH").ok().and_then(|v| v.parse::<f32>().ok()) {
            return m.clamp(3.0, 10.999);
        }
        let t = self.econ.tick as f32 + self.tick_fraction();
        3.0 + (t / crate::fx_light::DAY_TICKS as f32).rem_euclid(8.0)
    }

    /// How sprite `si` (of kind `kind`) at (x, z) bends in the wind this frame, or None when it does not move.
    pub(crate) fn fx_sway(&mut self, si: usize, frame_index: usize, x: f32, z: f32) -> Option<Bend> {
        let a = self.fx.atmos;
        if a.wind.k <= 0.0 {
            return None;
        }
        let kind = self.fx_kind(si);
        if kind == Kind::Flag {
            // the art's flag points right of the pole: mirrored when the wind blows to the screen's left
            let right = *self.fx.nature.flag_right.entry((si, frame_index)).or_insert_with(|| {
                let sp = &self.sprites[si].s;
                let img = &sp.frames[frame_index];
                let (mut m, mut n) = (0.0f32, 0.0f32);
                for (k, p) in img.px.chunks_exact(4).enumerate() {
                    if p[3] > 0 {
                        m += (k % img.w as usize) as f32;
                        n += 1.0;
                    }
                }
                n == 0.0 || m / n >= sp.anchor_x as f32
            });
            let flip = (a.wind.screen_x < -0.2) == right && a.wind.screen_x.abs() > 0.2 && a.wind.k > 0.08;
            return flip.then_some(Bend { lean: 0.0, curve: 1.0, flip: true });
        }
        let (lean_k, osc_k, freq, curve) = sway_params(kind)?;
        let w = a.wind;
        let t = self.fx.time;
        // gusts run across the course along the wind, about one every few seconds
        let along = (x * w.dir[0] + z * w.dir[1]) / (7.0 * TILE_SIZE);
        let gust = 0.5 + 0.5 * vnoise(t * 0.35 - along, 11) + 0.25 * vnoise(t * 1.1 - along * 2.0, 12);
        let ph = unit(hash3(x.to_bits(), z.to_bits(), 5)) * std::f32::consts::TAU;
        let f = freq * (0.85 + 0.3 * unit(hash3(z.to_bits(), x.to_bits(), 6)));
        let osc = (t * f * std::f32::consts::TAU + ph).sin() + 0.35 * (t * f * 2.3 * std::f32::consts::TAU + ph * 1.7).sin();
        let side = w.screen_x;
        let lean = w.k * gust * lean_k * side + w.k * osc_k * osc * (0.45 + 0.55 * gust) * (0.4 + 0.6 * side.abs());
        Some(Bend { lean, curve, flip: false })
    }

    /// What the newer effects change about a sprite's body as draw_props draws it: its texture (the season's colours), its
    /// bend in the wind and its windows' glow with that glow's strength.
    #[allow(clippy::type_complexity)]
    pub(crate) fn fx_sprite_extras(
        &mut self,
        g: &mut Gfx,
        si: usize,
        view: i32,
        p: &crate::app::Prop,
        tex: TextureId,
    ) -> (TextureId, Option<Bend>, Option<(TextureId, f32)>) {
        let s = crate::fx::settings();
        let fi = self.sprites[si].s.frame_index(view, p.frame);
        let seasonal = s.seasons || self.fx.atmos.season.snow > 0.0;
        let tex2 = if seasonal && p.outfit.is_none() { self.fx_season_texture(g, si, fi, tex).unwrap_or(tex) } else { tex };
        let bend = if s.wind { self.fx_sway(si, fi, p.x, p.z) } else { None };
        let night = self.fx.atmos.night;
        let glow = if s.evening_lights && night > 0.0 { self.fx_window_glow(g, si, fi, tex).map(|t| (t, night)) } else { None };
        (tex2, bend, glow)
    }

    /// The seasonal colours of a tree frame (cached), or None for the classic art.
    pub(crate) fn fx_season_texture(&mut self, g: &mut Gfx, si: usize, frame_index: usize, classic: TextureId) -> Option<TextureId> {
        if crate::hd::active() {
            return None;
        }
        let season = self.fx.atmos.season;
        if season == Season::default() {
            return None;
        }
        let kind = self.fx_kind(si);
        let key = season_key(&season, kind);
        if key == [0; 4] {
            return None;
        }
        if let Some(&(src, k, t)) = self.fx.nature.season_tex.get(&(si, frame_index)) {
            if src == classic && k == key {
                return Some(t);
            }
            g.ctx.delete_texture(t);
            self.fx.nature.season_tex.remove(&(si, frame_index));
        }
        if self.fx.nature.season_tex.len() > 3000 {
            for (_, (_, _, t)) in self.fx.nature.season_tex.drain() {
                g.ctx.delete_texture(t);
            }
        }
        let salt = self.fx_salt(si);
        let img = recolour(&self.sprites[si].s.frames[frame_index], kind, key, salt);
        let t = g.texture(&img, false);
        self.fx.nature.season_tex.insert((si, frame_index), (classic, key, t));
        Some(t)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn kinds_from_names() {
        assert_eq!(kind_of("Trees/TreeMapleLarge.flc"), Kind::Deciduous);
        assert_eq!(kind_of("Trees/TreePineFirLg.flc"), Kind::Conifer);
        assert_eq!(kind_of("Trees/Tropic/TreePalm/TreePalmLg.flc"), Kind::Palm);
        assert_eq!(kind_of("Trees/Desert/CactusA_Lg.flc"), Kind::Cactus);
        assert_eq!(kind_of("Scenic/Dogwood.flc"), Kind::Blossom);
        assert_eq!(kind_of("Tees/FlagPARK_open.flc"), Kind::Flag);
        assert_eq!(kind_of("Bldgs/Park/clubL1.flc"), Kind::Building);
        assert_eq!(kind_of("Bldgs/Park/clubL1Shadow.flc"), Kind::Other);
    }

    #[test]
    fn strips_keep_the_ground_point() {
        let b = Bend { lean: 0.1, curve: 1.5, flip: false };
        let q = strips(-10.0, 10.0, 40.0, -5.0, b);
        assert_eq!(q.len(), 5);
        // the top moved by lean times the height, the ground row and below not at all
        assert!((q[0][0].0 - (-10.0 + 4.0)).abs() < 1e-4);
        assert_eq!(q[4][0].0, -10.0);
        assert_eq!(q[4][3].0, -10.0);
        // the strips meet
        for w in q.windows(2) {
            assert_eq!(w[0][3], w[1][0]);
        }
        // a flip mirrors about the ground point and swaps u
        let f = strips(-2.0, 8.0, 10.0, 0.0, Bend { lean: 0.0, curve: 1.0, flip: true });
        assert_eq!(f.len(), 1);
        assert_eq!((f[0][0].0, f[0][0].2), (-8.0, 1.0));
    }

    #[test]
    fn seasons_through_the_year() {
        let spring = season_at(3.5, 0, None);
        assert!(spring.blossom > 0.9 && spring.fresh > 0.9 && spring.autumn == 0.0);
        let summer = season_at(7.0, 0, None);
        assert_eq!((summer.blossom, summer.fresh, summer.autumn), (0.0, 0.0, 0.0));
        assert!(season_at(10.5, 0, None).autumn > 0.95);
        // the tropics have no autumn
        assert_eq!(season_at(10.5, 2, None).autumn, 0.0);
        // frost at dawn in March, none at midday
        assert!(season_at(3.2, 0, Some(0.03)).frost > 0.9);
        assert_eq!(season_at(3.2, 0, Some(0.5)).frost, 0.0);
    }

    #[test]
    fn autumn_turns_green_leaves_only() {
        let mut img = Rgba::new(4, 1);
        img.px.copy_from_slice(&[40, 120, 30, 255, 120, 80, 40, 255, 40, 120, 30, 0, 50, 140, 40, 255]);
        let o = recolour(&img, Kind::Deciduous, [0, 0, 8, 0], 1);
        // the leaf is now warm, the trunk and the transparent pixel untouched
        assert!(o.px[0] > o.px[1]);
        assert_eq!(&o.px[4..12], &img.px[4..12]);
    }
}
