//! Enhanced graphics' light and weather (docs/ENHANCED.md, "Time of day and weather"): the sun and sky through the day
//! following the game's clock, drifting cloud shadows, light rain on some days, morning mist in low ground and a haze in the
//! distance. None of it touches the simulation: the time of day is read from the game's calendar tick, and what is random
//! (which days rain, where the drops fall) comes from a render-side hash seeded from the save, never from the game's
//! random generator.
//!
//! With the time of day and rain off, [`App::fx_atmos`] gives the classic midday (the decoded light, the theme's sky) and
//! nothing here changes a pixel; clouds, mist and haze each draw only when their option is on.
use crate::app::App;
use crate::gfx::{Gfx, Mat4, Mode, Uniforms, Vert};
use miniquad::*;
use sg_core::terrain::TILE_SIZE;
use std::sync::OnceLock;

/// The clouds' noise repeats every 26 tiles (texture coordinates per world unit).
pub const CLOUD_REPEAT: f32 = 1.0 / (26.0 * TILE_SIZE);
/// The noise's size in texels (a power of two, so it tiles and mipmaps in WebGL 1).
const CLOUD_N: usize = 128;
/// The game's calendar ticks in one day of light: a game month (1024 ticks, about a minute and a half at normal speed).
pub const DAY_TICKS: u32 = 1024;
/// The art's own shadow frames (pre-rendered with the sprites) fall to the right on the screen and a little up it, about
/// two thirds of the height of what casts them (measured on the tree and building frames against their bodies): the cast
/// shadows follow that at midday, so the two kinds agree, and turn and stretch with the sun from there.
const ART_SHADOW_UP_DEG: f32 = 50.0;
const ART_SHADOW_LEN: f32 = 0.75;

/// A new game (tick 0) starts in the morning.
const DAY_START: f32 = 0.12;

/// This frame's sun, sky and weather.
#[derive(Clone, Copy, Debug)]
pub struct Atmos {
    /// The light differs from the classic midday (time of day or rain): the colours and direction below are used.
    pub tinted: bool,
    /// Towards the sun, in world space and in eye space.
    pub sun_w: [f32; 3],
    pub sun_e: [f32; 3],
    /// Multipliers of the decoded light's diffuse and specular (`sun`) and ambient (`amb`) colours.
    pub sun: [f32; 3],
    pub amb: [f32; 3],
    /// How strongly the sun casts shadows (1 in full sun, 0 at dusk).
    pub shadow: f32,
    /// The sky's top, its horizon and the haze below the map.
    pub sky_top: [f32; 3],
    pub sky_low: [f32; 3],
    pub below: [f32; 3],
    /// The sprites' tint (they are pre-lit for midday).
    pub sprite: [f32; 3],
    /// Cloud shadows: strength (0 off), the noise's edge and its drift (world units).
    pub cloud: f32,
    pub cover: f32,
    pub cloud_off: [f32; 2],
    /// Rain (0 none to 1), mist density and colour, haze strength and colour.
    pub rain: f32,
    pub mist: f32,
    pub mist_col: [f32; 3],
    pub haze: f32,
    pub haze_col: [f32; 3],
    /// Where the sprites' cast shadows fall (a unit direction along the ground, world x and z) and how far per unit of
    /// height (fx_shadow.rs).
    pub shadow_dir: [f32; 2],
    pub shadow_len: f32,
}

impl Default for Atmos {
    fn default() -> Atmos {
        Atmos {
            tinted: false,
            sun_w: [0.0, 1.0, 0.0],
            sun_e: sg_core::terrain::light_direction(),
            sun: [1.0; 3],
            amb: [1.0; 3],
            shadow: 1.0,
            sky_top: [0.0; 3],
            sky_low: [0.0; 3],
            below: [0.0; 3],
            sprite: [1.0; 3],
            cloud: 0.0,
            cover: 0.5,
            cloud_off: [0.0; 2],
            rain: 0.0,
            mist: 0.0,
            mist_col: [1.0; 3],
            haze: 0.0,
            haze_col: [1.0; 3],
            shadow_dir: [1.0, 0.0],
            shadow_len: ART_SHADOW_LEN,
        }
    }
}

/// A moment of the day: the sun's turn from the decoded light's direction (degrees round the vertical) and its height
/// (degrees; None: the decoded light's own), the colours as multipliers, the sky's colours and how far they replace the
/// theme's, and the shadows' strength.
struct Key {
    at: f32,
    az: f32,
    el: Option<f32>,
    sun: [f32; 3],
    amb: [f32; 3],
    sprite: [f32; 3],
    top: [f32; 3],
    low: [f32; 3],
    sky: f32,
    shadow: f32,
}

const W: [f32; 3] = [1.0, 1.0, 1.0];

/// The day: a short dawn, morning, a long midday (the classic light), a golden afternoon, sunset and a short blue dusk.
/// Dusk keeps a cool light from the sky so the course stays easy to read. The sun swings round the front of the view (from
/// the left in the morning to the right at sunset), so its shadows fall sideways and away from the viewer, never in long
/// streaks down the screen.
const DAY: [Key; 9] = [
    Key {
        at: 0.00,
        az: -40.0,
        el: Some(5.0),
        sun: [0.86, 0.60, 0.50],
        amb: [0.96, 0.90, 1.00],
        sprite: [0.86, 0.80, 0.82],
        top: [0.32, 0.38, 0.64],
        low: [0.98, 0.74, 0.64],
        sky: 0.85,
        shadow: 0.35,
    },
    Key {
        at: 0.07,
        az: -32.0,
        el: Some(10.0),
        sun: [1.00, 0.84, 0.68],
        amb: [0.98, 0.95, 0.97],
        sprite: [0.98, 0.92, 0.86],
        top: [0.40, 0.56, 0.84],
        low: [1.00, 0.87, 0.76],
        sky: 0.5,
        shadow: 0.85,
    },
    Key {
        at: 0.22,
        az: -15.0,
        el: Some(19.0),
        sun: [1.00, 0.96, 0.90],
        amb: W,
        sprite: [1.00, 0.98, 0.95],
        top: [0.40, 0.56, 0.84],
        low: [1.00, 0.92, 0.82],
        sky: 0.15,
        shadow: 1.0,
    },
    Key { at: 0.40, az: 0.0, el: None, sun: W, amb: W, sprite: W, top: W, low: W, sky: 0.0, shadow: 1.0 },
    Key { at: 0.60, az: 0.0, el: None, sun: W, amb: W, sprite: W, top: W, low: W, sky: 0.0, shadow: 1.0 },
    Key {
        at: 0.74,
        az: -45.0,
        el: Some(15.0),
        sun: [1.00, 0.93, 0.82],
        amb: W,
        sprite: [1.00, 0.97, 0.91],
        top: [0.40, 0.54, 0.82],
        low: [1.00, 0.90, 0.76],
        sky: 0.2,
        shadow: 1.0,
    },
    Key {
        at: 0.83,
        az: -95.0,
        el: Some(8.0),
        sun: [1.00, 0.78, 0.55],
        amb: [0.97, 0.90, 0.88],
        sprite: [1.00, 0.88, 0.74],
        top: [0.36, 0.44, 0.74],
        low: [1.00, 0.76, 0.52],
        sky: 0.65,
        shadow: 0.9,
    },
    Key {
        at: 0.895,
        az: -150.0,
        el: Some(4.0),
        sun: [1.00, 0.62, 0.40],
        amb: [0.92, 0.78, 0.86],
        sprite: [0.94, 0.75, 0.68],
        top: [0.28, 0.30, 0.58],
        low: [1.00, 0.58, 0.36],
        sky: 0.92,
        shadow: 0.6,
    },
    Key {
        at: 0.95,
        az: -165.0,
        el: Some(40.0),
        sun: [0.34, 0.40, 0.66],
        amb: [0.88, 0.96, 1.30],
        sprite: [0.66, 0.72, 0.92],
        top: [0.13, 0.17, 0.38],
        low: [0.38, 0.42, 0.64],
        sky: 1.0,
        shadow: 0.0,
    },
];

fn lerp3(a: [f32; 3], b: [f32; 3], t: f32) -> [f32; 3] {
    [a[0] + (b[0] - a[0]) * t, a[1] + (b[1] - a[1]) * t, a[2] + (b[2] - a[2]) * t]
}

fn mul3(a: [f32; 3], b: [f32; 3]) -> [f32; 3] {
    [a[0] * b[0], a[1] * b[1], a[2] * b[2]]
}

fn grey(c: [f32; 3], k: f32) -> [f32; 3] {
    let l = c[0] * 0.299 + c[1] * 0.587 + c[2] * 0.114;
    lerp3(c, [l, l, l], k)
}

/// The day's state at phase `p` (0..1): (azimuth, elevation or None, sun, ambient, sprite, sky top, sky low, sky weight,
/// shadow), between the two keys around it with a smooth step; the last key runs on into the first (dusk to dawn).
#[allow(clippy::type_complexity)]
fn day_at(p: f32, el0: f32) -> (f32, f32, [f32; 3], [f32; 3], [f32; 3], [f32; 3], [f32; 3], f32, f32) {
    let p = p.rem_euclid(1.0);
    let n = DAY.len();
    let i = DAY.iter().rposition(|k| k.at <= p).unwrap_or(0);
    let (a, b) = (&DAY[i], &DAY[(i + 1) % n]);
    let span = if i + 1 == n { 1.0 - a.at + b.at } else { b.at - a.at };
    let t = ((p - a.at) / span).clamp(0.0, 1.0);
    let t = t * t * (3.0 - 2.0 * t);
    let el = |k: &Key| k.el.unwrap_or(el0);
    let lerp = |x: f32, y: f32| x + (y - x) * t;
    (
        lerp(a.az, b.az),
        lerp(el(a), el(b)),
        lerp3(a.sun, b.sun, t),
        lerp3(a.amb, b.amb, t),
        lerp3(a.sprite, b.sprite, t),
        lerp3(a.top, b.top, t),
        lerp3(a.low, b.low, t),
        lerp(a.sky, b.sky),
        lerp(a.shadow, b.shadow),
    )
}

/// A direction (towards the sun) quantised to 3 degree steps round and 2 up, for rebuilding the light map's shadows.
pub fn sun_step(w: [f32; 3]) -> (i32, i32) {
    let az = w[2].atan2(w[0]).to_degrees();
    let el = w[1].clamp(-1.0, 1.0).asin().to_degrees();
    ((az / 3.0).round() as i32, (el / 2.0).round() as i32)
}

/// A hash for the render-side randomness (FNV-1a over the words, then mixed).
fn hash(words: &[u32]) -> u32 {
    let mut h: u32 = 0x811c_9dc5;
    for w in words {
        for b in w.to_le_bytes() {
            h ^= b as u32;
            h = h.wrapping_mul(0x0100_0193);
        }
    }
    h ^= h >> 16;
    h = h.wrapping_mul(0x7feb_352d);
    h ^= h >> 15;
    h
}

fn unit(h: u32) -> f32 {
    (h >> 8) as f32 / (1u32 << 24) as f32
}

// ---- the clouds' noise ------------------------------------------------------------------------------------------------------

/// A tiling noise for the clouds and the mist (fractal value noise on lattices that divide the texture, so it repeats
/// seamlessly), the same every run.
fn cloud_noise() -> &'static [u8] {
    static N: OnceLock<Vec<u8>> = OnceLock::new();
    N.get_or_init(|| {
        let mut v = vec![0f32; CLOUD_N * CLOUD_N];
        let mut amp = 0.5f32;
        let mut total = 0.0;
        for (o, cells) in [4usize, 8, 16, 32].into_iter().enumerate() {
            let lat = |x: usize, y: usize| unit(hash(&[o as u32, (x % cells) as u32, (y % cells) as u32]));
            let step = CLOUD_N / cells;
            for y in 0..CLOUD_N {
                for x in 0..CLOUD_N {
                    let (cx, cy) = (x / step, y / step);
                    let (fx, fy) = ((x % step) as f32 / step as f32, (y % step) as f32 / step as f32);
                    let (fx, fy) = (fx * fx * (3.0 - 2.0 * fx), fy * fy * (3.0 - 2.0 * fy));
                    let a = lat(cx, cy) + (lat(cx + 1, cy) - lat(cx, cy)) * fx;
                    let b = lat(cx, cy + 1) + (lat(cx + 1, cy + 1) - lat(cx, cy + 1)) * fx;
                    v[y * CLOUD_N + x] += (a + (b - a) * fy) * amp;
                }
            }
            total += amp;
            amp *= 0.5;
        }
        // stretched to the full range
        let (lo, hi) = v.iter().fold((f32::MAX, f32::MIN), |(l, h), &x| (l.min(x), h.max(x)));
        let _ = total;
        v.iter().map(|&x| (((x - lo) / (hi - lo).max(1e-6)) * 255.0).round() as u8).collect()
    })
}

/// The noise as a repeating texture (bilinear, no mipmaps), its value in all three colours.
pub fn cloud_texture(ctx: &mut dyn RenderingBackend) -> TextureId {
    let px: Vec<u8> = cloud_noise().iter().flat_map(|&v| [v, v, v, 255]).collect();
    ctx.new_texture(
        TextureAccess::Static,
        TextureSource::Bytes(&px),
        TextureParams {
            kind: TextureKind::Texture2D,
            width: CLOUD_N as u32,
            height: CLOUD_N as u32,
            format: TextureFormat::RGBA8,
            wrap: TextureWrap::Repeat,
            min_filter: FilterMode::Linear,
            mag_filter: FilterMode::Linear,
            mipmap_filter: MipmapFilterMode::None,
            allocate_mipmaps: false,
            sample_count: 1,
        },
    )
}

/// The noise at texture coordinates (u, v), bilinear and repeating as the GPU samples it (0..1).
fn noise_at(u: f32, v: f32) -> f32 {
    let n = cloud_noise();
    let (x, y) = (u * CLOUD_N as f32 - 0.5, v * CLOUD_N as f32 - 0.5);
    let (x0, y0) = (x.floor(), y.floor());
    let (fx, fy) = (x - x0, y - y0);
    let at = |i: f32, j: f32| {
        let (i, j) = ((i as i64).rem_euclid(CLOUD_N as i64) as usize, (j as i64).rem_euclid(CLOUD_N as i64) as usize);
        n[j * CLOUD_N + i] as f32 / 255.0
    };
    let a = at(x0, y0) + (at(x0 + 1.0, y0) - at(x0, y0)) * fx;
    let b = at(x0, y0 + 1.0) + (at(x0 + 1.0, y0 + 1.0) - at(x0, y0 + 1.0)) * fx;
    a + (b - a) * fy
}

fn smoothstep(a: f32, b: f32, x: f32) -> f32 {
    let t = ((x - a) / (b - a)).clamp(0.0, 1.0);
    t * t * (3.0 - 2.0 * t)
}

/// The sky colour below the map of this moment (what the post-processing's target is cleared to).
pub fn below_now(app: &App) -> [f32; 3] {
    app.fx.atmos.below
}

// ---- the day and the weather ------------------------------------------------------------------------------------------------

impl App {
    /// The seed of the render-side randomness: the course's name and size, so a save keeps its weather.
    fn fx_seed(&self) -> u32 {
        let name = self.course_name.bytes().fold(0u32, |h, b| h.wrapping_mul(31).wrapping_add(b as u32));
        hash(&[name, self.terrain.w as u32, self.terrain.h as u32])
    }

    /// The day (one game month) and the phase of its light (0..1, 0 dawn, 0.4..0.6 midday, 0.9 sunset). `SG_FX_HOUR`
    /// fixes the phase for stills.
    pub(crate) fn fx_day(&self) -> (u32, f32) {
        let t = self.econ.tick as f32 + self.tick_fraction();
        let day = self.econ.tick / DAY_TICKS;
        let p = std::env::var("SG_FX_HOUR").ok().and_then(|v| v.parse::<f32>().ok());
        (day, p.unwrap_or(t / DAY_TICKS as f32 + DAY_START).rem_euclid(1.0))
    }

    /// How hard it rains on a day (0 dry): about one day in four on parkland, fewer in the desert, more on the links.
    /// `SG_FX_RAIN=1` (or 0) makes every day wet (dry) for stills.
    fn fx_rain_on(&self, day: u32) -> f32 {
        if let Some(v) = std::env::var("SG_FX_RAIN").ok().and_then(|v| v.parse::<f32>().ok()) {
            return v.clamp(0.0, 1.0);
        }
        let chance = [0.25, 0.08, 0.22, 0.35][self.exe_theme().min(3) as usize];
        let h = hash(&[self.fx_seed(), day, 0x7261_696e]);
        if unit(h) < chance {
            0.6 + 0.4 * unit(hash(&[h]))
        } else {
            0.0
        }
    }

    /// This moment's sun, sky and weather.
    pub(crate) fn fx_atmos(&self) -> Atmos {
        let s = crate::fx::settings();
        let look = self.look();
        let mut a = Atmos { sky_top: look.sky_top, sky_low: look.sky_low, below: look.below, ..Atmos::default() };
        let rot = crate::fx::view_rotation(self);
        let base = crate::fx::to_world(&rot, sg_core::terrain::light_direction());
        a.sun_w = base;
        a.haze_col = look.sky_low;
        // the art's shadow direction along the ground: the screen's right turned a little up it
        let (r, u) = ([rot[0], rot[8]], [rot[1], rot[9]]);
        let ul = u[0].hypot(u[1]).max(1e-4);
        let (c, sn) = (ART_SHADOW_UP_DEG.to_radians().cos(), ART_SHADOW_UP_DEG.to_radians().sin());
        let d = [r[0] * c + u[0] / ul * sn, r[1] * c + u[1] / ul * sn];
        let dl = d[0].hypot(d[1]).max(1e-4);
        a.shadow_dir = [d[0] / dl, d[1] / dl];
        let (day, phase) = self.fx_day();
        let rain = if s.rain { self.fx_rain_on(day) } else { 0.0 };
        if s.time_of_day {
            let el0 = base[1].clamp(-1.0, 1.0).asin().to_degrees();
            let (az, el, sun, amb, sprite, top, low, w, shadow) = day_at(phase, el0);
            let a0 = base[2].atan2(base[0]);
            // the cast shadows turn with the sun and lengthen as it sinks
            let (sa, ca) = az.to_radians().sin_cos();
            a.shadow_dir = [a.shadow_dir[0] * ca - a.shadow_dir[1] * sa, a.shadow_dir[0] * sa + a.shadow_dir[1] * ca];
            let el_r = el.to_radians().max(0.02);
            a.shadow_len = (ART_SHADOW_LEN * el0.to_radians().tan() / el_r.tan()).min(2.4);
            let (az, el) = (a0 + az.to_radians(), el.to_radians());
            a.sun_w = [el.cos() * az.cos(), el.sin(), el.cos() * az.sin()];
            // a low sun lights flat ground at a grazing angle: its light is raised so the course keeps at least three
            // quarters of its midday brightness (playable), while slopes turned to the sun still glow and the shadows grow
            let comp = (el0.to_radians().sin() * 0.75 / el.sin().max(0.05)).clamp(1.0, 3.0);
            a.sun = sun.map(|v| v * comp);
            a.amb = amb;
            a.sprite = sprite;
            a.shadow = shadow;
            a.sky_top = lerp3(look.sky_top, top, w);
            a.sky_low = lerp3(look.sky_low, low, w);
            a.below = lerp3(look.below, mul3(low, [0.55, 0.55, 0.62]), w * 0.8);
            a.haze_col = a.sky_low;
            a.tinted = true;
        }
        if rain > 0.0 {
            // overcast: the sun dimmed and its shadows faint, the sky and the colours greyer
            let k = rain;
            a.sun = a.sun.map(|v| v * (1.0 - 0.5 * k));
            a.amb = grey(a.amb, 0.3 * k).map(|v| v * (1.0 - 0.08 * k));
            a.sprite = grey(a.sprite, 0.15 * k).map(|v| v * (1.0 - 0.17 * k));
            a.shadow *= 1.0 - 0.7 * k;
            a.sky_top = lerp3(a.sky_top, grey(a.sky_top, 1.0).map(|v| v * 0.8), 0.7 * k);
            a.sky_low = lerp3(a.sky_low, grey(a.sky_low, 1.0).map(|v| v * 0.85), 0.7 * k);
            a.below = lerp3(a.below, grey(a.below, 1.0), 0.6 * k);
            a.haze_col = a.sky_low;
            a.rain = k;
            a.tinted = true;
        }
        a.sun_e = crate::fx::to_eye(&rot, a.sun_w);
        if a.sun_w[1] <= 0.0 {
            a.shadow = 0.0;
        }
        if s.clouds {
            // the cover changes from day to day; rainy days are overcast
            let h = hash(&[self.fx_seed(), day, 0x636c_6f75]);
            a.cover = 0.62 - 0.14 * unit(h) - 0.25 * rain;
            a.cloud = (0.42 * a.shadow.max(0.3)).min(0.42);
            let t = self.fx.time;
            let dir = unit(hash(&[h])) * std::f32::consts::TAU;
            // the clouds drift about 0.6 tiles a second; each day starts somewhere else in the noise
            let (ox, oz) = (unit(hash(&[h, 1])) / CLOUD_REPEAT, unit(hash(&[h, 2])) / CLOUD_REPEAT);
            // kept within one period of the noise, so the shader's coordinates stay small (mediump on some phones)
            let period = 1.0 / CLOUD_REPEAT;
            a.cloud_off = [(ox + dir.cos() * 60.0 * t).rem_euclid(period), (oz + dir.sin() * 60.0 * t).rem_euclid(period)];
        }
        if s.mist {
            // thick at dawn, gone by late morning with the time of day; a light mist all day without it
            a.mist = if s.time_of_day {
                let m = if phase > 0.9 { smoothstep(0.9, 0.99, phase) } else { 1.0 - smoothstep(0.04, 0.26, phase) };
                0.8 * m
            } else {
                0.35
            };
            a.mist += 0.15 * rain;
            a.mist_col = lerp3(a.sky_low, [0.94, 0.96, 1.0], 0.55).map(|v| v * (0.55 + 0.45 * a.amb[1].min(1.0)));
        }
        if s.haze {
            a.haze = 0.30 + 0.25 * rain;
        }
        a
    }

    /// The colour a sprite is drawn with at (x, z): its tint for the time of day and rain, darker under a cloud; for its
    /// shadow frame, how strong the shadow is there. None when nothing changes it (the classic colour).
    pub(crate) fn fx_sprite_tint(&self, x: f32, z: f32, shadow: bool) -> Option<[f32; 4]> {
        let a = &self.fx.atmos;
        if !a.tinted && a.cloud <= 0.0 {
            return None;
        }
        let sun = if a.cloud > 0.0 { self.fx_cloud_sun(x, z) } else { 1.0 };
        if shadow {
            return Some([1.0, 1.0, 1.0, a.shadow * sun]);
        }
        // a sprite is pre-lit: under a cloud it loses part of its sunlit brightness
        let k = 1.0 - 0.55 * (1.0 - sun);
        Some([a.sprite[0] * k, a.sprite[1] * k, a.sprite[2] * k, 1.0])
    }

    /// How much sun the clouds let through at (x, z), as the ground's shader works it out.
    pub(crate) fn fx_cloud_sun(&self, x: f32, z: f32) -> f32 {
        let a = &self.fx.atmos;
        if a.cloud <= 0.0 {
            return 1.0;
        }
        let n = noise_at((x + a.cloud_off[0]) * CLOUD_REPEAT, (z + a.cloud_off[1]) * CLOUD_REPEAT);
        1.0 - a.cloud * smoothstep(a.cover, a.cover + 0.22, n)
    }

    /// Over the course once the sprites are drawn: the haze in the distance and the rain.
    pub(crate) fn fx_overlay(&mut self, g: &mut Gfx) {
        let a = self.fx.atmos;
        if a.haze <= 0.0 && a.rain <= 0.0 {
            return;
        }
        let u = Uniforms::flat(&Mat4::identity(), &Mat4::identity());
        if a.haze > 0.0 {
            // the camera looks down at the course from the side, so the screen's top is far: a haze growing up the screen,
            // stronger the more of the course the view takes in
            let depth = self.draw_h * self.upp / (36.0 * TILE_SIZE);
            let k = a.haze * depth.clamp(0.25, 1.0);
            let c = a.haze_col;
            let strips = 10;
            for i in 0..strips {
                let (y0, y1) = (i as f32 / strips as f32, (i + 1) as f32 / strips as f32);
                let al = |y: f32| k * smoothstep(0.3, 1.0, y).powf(1.4);
                let (a0, a1) = (al(y0), al(y1));
                if a1 <= 0.0 {
                    continue;
                }
                let v = |x: f32, y: f32, al: f32| Vert::new(x, y * 2.0 - 1.0, 0.0, 0.0, 0.0).col([c[0], c[1], c[2], al]);
                g.quad(Mode::Flat, None, &u, [v(-1.0, y1, a1), v(1.0, y1, a1), v(1.0, y0, a0), v(-1.0, y0, a0)]);
            }
        }
        if a.rain > 0.0 {
            self.fx_rain_draw(g, &u, a.rain);
        }
        g.flush();
    }

    /// The rain: thin slanted streaks falling over the course and a small splash where each lands, drawn on the screen
    /// (their places from the render-side hash, their fall from the effects' clock).
    fn fx_rain_draw(&mut self, g: &mut Gfx, u: &Uniforms, k: f32) {
        let (w, h) = (self.draw_w, self.draw_h);
        let vscale = (w / 800.0).min(h / 600.0);
        let n = ((w * h) / (1920.0 * 1080.0) * 700.0 * k).clamp(40.0, 2500.0) as u32;
        let seed = self.fx_seed();
        let t = self.fx.time;
        let mut v: Vec<Vert> = Vec::with_capacity(n as usize * 12);
        let dot = crate::fx::gpu(g).dot;
        let mut splash: Vec<Vert> = Vec::new();
        // pixels to clip space
        let (sx, sy) = (2.0 / w, 2.0 / h);
        let slant = 0.22;
        for i in 0..n {
            let r = |j: u32| unit(hash(&[seed, i, j]));
            let speed = 0.9 + 0.5 * r(3);
            let f = (r(2) + t * speed).fract();
            let (x, ground) = (r(0) * w, r(1) * h);
            let len = (14.0 + 12.0 * r(4)) * vscale;
            let fall = 0.42 * h;
            if f < 0.85 {
                // falling: from `fall` pixels above where it lands
                let y = ground - fall * (1.0 - f / 0.85);
                let (x0, y0) = (x - slant * (ground - y), y);
                let (x1, y1) = (x0 + slant * len, y0 + len);
                let half = 0.55 * vscale.max(1.0);
                let col_top = [0.80, 0.84, 0.92, 0.0];
                let col = [0.82, 0.86, 0.94, 0.34 * (0.6 + 0.4 * r(5))];
                let p = |px: f32, py: f32, c: [f32; 4]| Vert::new(px * sx - 1.0, 1.0 - py * sy, 0.0, 0.0, 0.0).col(c);
                let (a0, a1, b0, b1) = (p(x0 - half, y0, col_top), p(x0 + half, y0, col_top), p(x1 - half, y1, col), p(x1 + half, y1, col));
                v.extend_from_slice(&[a0, b0, b1, a0, b1, a1]);
            } else {
                // landed: a ring opening and fading
                let e = (f - 0.85) / 0.15;
                let rad = (2.0 + 5.0 * e) * vscale;
                let al = 0.35 * (1.0 - e);
                let c = [0.85, 0.9, 0.98, al];
                let p = |px: f32, py: f32, tu: f32, tv: f32| Vert::new(px * sx - 1.0, 1.0 - py * sy, 0.0, tu, tv).col(c);
                let (l, r, t, b) = (x - rad, x + rad, ground - rad * 0.45, ground + rad * 0.45);
                splash.extend_from_slice(&[
                    p(l, t, 0.0, 0.0),
                    p(r, t, 1.0, 0.0),
                    p(r, b, 1.0, 1.0),
                    p(l, t, 0.0, 0.0),
                    p(r, b, 1.0, 1.0),
                    p(l, b, 0.0, 1.0),
                ]);
            }
        }
        g.tris(Mode::Flat, None, u, &v);
        g.tris(Mode::Flat, Some(dot), u, &splash);
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn noise_tiles_and_spans() {
        let n = cloud_noise();
        assert_eq!(n.len(), CLOUD_N * CLOUD_N);
        assert_eq!(n.iter().copied().min(), Some(0));
        assert_eq!(n.iter().copied().max(), Some(255));
        // repeats: the sample one period over is the same
        assert!((noise_at(0.3, 0.7) - noise_at(1.3, -0.3)).abs() < 1e-4);
    }

    #[test]
    fn the_day_runs_round() {
        // midday is the decoded light, dusk has no shadows, the phase wraps from dusk to dawn smoothly
        let mid = day_at(0.5, 25.0);
        assert_eq!((mid.0, mid.1, mid.2, mid.8), (0.0, 25.0, [1.0; 3], 1.0));
        assert_eq!(day_at(0.95, 25.0).8, 0.0);
        let (end, start) = (day_at(0.99999, 25.0), day_at(0.0, 25.0));
        assert!((end.0 - start.0).abs() < 0.1 && (end.1 - start.1).abs() < 0.1);
        // never darker than a third of the classic light, even at dusk
        for i in 0..100 {
            let d = day_at(i as f32 / 100.0, 25.0);
            assert!(d.3.iter().all(|&v| v >= 0.75) && d.4.iter().all(|&v| v >= 0.6), "{i}");
        }
    }

    #[test]
    fn sun_steps() {
        assert_eq!(sun_step([1.0, 0.0, 0.0]), (0, 0));
        assert_ne!(sun_step([0.9, 0.1, 0.0]), sun_step([0.9, 0.2, 0.0]));
    }
}
