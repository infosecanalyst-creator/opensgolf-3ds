//! Terrain textures: 64 x 64 tiles per terrain type and theme that repeat without seams, in the nine shapes the terrain
//! renderer blends with (docs/TERRAIN.md, "Edge blending": 1 the plain fill, then the border pieces: a straight collar,
//! corner notches, a rounded patch and a pinched patch; 6..9 repeat 1..4 with a lighter collar), and the path, wall and
//! cliff overlays.
//!
//! Each surface is painted from periodic noise (gradient noise, cellular noise and per-pixel grain, all wrapping at 64
//! pixels so the tiles join without seams) as a colour and a height; the height is then lit from the upper left, which
//! gives tufts, clumps, stones and the cut lip of a mown area their relief. Mown grass has wide mowing stripes, rough has
//! blades and clumps, sand has grain and soft ripples, water has a moving sheen. Every number here is our own.
use super::paint::{mix, rgb, sd_rect, shade, Canvas, Col};
use super::{hash, theme_of};

const N: usize = 64;

// ---- periodic noise -----------------------------------------------------------------------------------------------------

fn h32(x: i32, y: i32, seed: u32) -> u32 {
    let mut h = (x as u32).wrapping_mul(0x8da6_b343) ^ (y as u32).wrapping_mul(0xd816_3841) ^ seed.wrapping_mul(0xcb1a_b31f);
    h ^= h >> 15;
    h = h.wrapping_mul(0x2c1b_3c6d);
    h ^= h >> 12;
    h = h.wrapping_mul(0x297a_2d39);
    h ^ (h >> 15)
}

/// A hash in 0..1.
fn rnd(x: i32, y: i32, seed: u32) -> f32 {
    (h32(x, y, seed) & 0xFFFFFF) as f32 / 16777215.0
}

fn fade(t: f32) -> f32 {
    t * t * t * (t * (t * 6.0 - 15.0) + 10.0)
}

/// Gradient noise in about -1..1 with `cx` x `cy` cells across the 64 pixel tile (each must divide into whole cells; the
/// noise wraps at the tile edge).
fn perlin(x: f32, y: f32, cx: i32, cy: i32, seed: u32) -> f32 {
    let (fx, fy) = (x / N as f32 * cx as f32, y / N as f32 * cy as f32);
    let (ix, iy) = (fx.floor() as i32, fy.floor() as i32);
    let (tx, ty) = (fx - ix as f32, fy - iy as f32);
    let g = |i: i32, j: i32, dx: f32, dy: f32| {
        let a = rnd(i.rem_euclid(cx), j.rem_euclid(cy), seed) * std::f32::consts::TAU;
        a.cos() * dx + a.sin() * dy
    };
    let (u, v) = (fade(tx), fade(ty));
    let a = g(ix, iy, tx, ty) + (g(ix + 1, iy, tx - 1.0, ty) - g(ix, iy, tx, ty)) * u;
    let b = g(ix, iy + 1, tx, ty - 1.0) + (g(ix + 1, iy + 1, tx - 1.0, ty - 1.0) - g(ix, iy + 1, tx, ty - 1.0)) * u;
    (a + (b - a) * v) * 1.41
}

/// Fractal noise in about -1..1: `oct` octaves from `c` cells across, each twice as fine and half as strong.
fn fbm(x: f32, y: f32, c: i32, oct: u32, seed: u32) -> f32 {
    let (mut sum, mut amp, mut norm, mut cells) = (0.0, 1.0, 0.0, c);
    for o in 0..oct {
        sum += perlin(x, y, cells, cells, seed.wrapping_add(o * 7919)) * amp;
        norm += amp;
        amp *= 0.5;
        cells = (cells * 2).min(64);
    }
    sum / norm
}

/// Cellular noise with `c` x `c` cells (wrapping): the distances to the nearest and second nearest feature point (in
/// cell units) and a number for the nearest point's cell.
fn cells(x: f32, y: f32, c: i32, seed: u32) -> (f32, f32, u32) {
    let (fx, fy) = (x / N as f32 * c as f32, y / N as f32 * c as f32);
    let (ix, iy) = (fx.floor() as i32, fy.floor() as i32);
    let (mut d1, mut d2, mut id) = (9.0f32, 9.0f32, 0u32);
    for j in -1..=1 {
        for i in -1..=1 {
            let (cx, cy) = (ix + i, iy + j);
            let (wx, wy) = (cx.rem_euclid(c), cy.rem_euclid(c));
            let px = cx as f32 + 0.15 + 0.7 * rnd(wx, wy, seed);
            let py = cy as f32 + 0.15 + 0.7 * rnd(wx, wy, seed ^ 0x5bd1);
            let d = ((px - fx).powi(2) + (py - fy).powi(2)).sqrt();
            if d < d1 {
                d2 = d1;
                d1 = d;
                id = h32(wx, wy, seed ^ 0x77);
            } else if d < d2 {
                d2 = d;
            }
        }
    }
    (d1, d2, id)
}

fn smooth(e0: f32, e1: f32, x: f32) -> f32 {
    let t = ((x - e0) / (e1 - e0)).clamp(0.0, 1.0);
    t * t * (3.0 - 2.0 * t)
}

/// Short grass blades: streaks that are long along y and thin along x, -1..1.
fn blades(x: f32, y: f32, seed: u32, density: i32) -> f32 {
    perlin(x, y, density, density / 4, seed) * 0.6 + perlin(x, y, density * 2, density / 2, seed ^ 0x33) * 0.4
}

// ---- surfaces -------------------------------------------------------------------------------------------------------------

#[derive(Clone, Copy, PartialEq)]
enum Kind {
    Fairway,
    Firm,
    Green,
    Tricky,
    Tee,
    Rough,
    Deep,
    GrassBunker,
    Cracks,
    Scrub,
    Brush,
    Sand,
    Zen,
    GrassySand,
    WaterShallow,
    WaterMiddle,
    WaterDeep,
    Marsh,
    Rock,
    Woods,
    Paving,
    Ravine,
    Flowers,
}

/// The colours of a theme.
struct Theme {
    /// mown grass (fairway), its light and dark stripe
    mown: Col,
    green: Col,
    rough: Col,
    deep: Col,
    /// the bare ground of the theme (earth, desert sand)
    ground: Col,
    sand: Col,
    water: Col,
    shallow: Col,
    /// the low grass collar around mown areas
    collar: Col,
    scrub: Col,
    rock: Col,
    flower: [Col; 3],
    /// the desert's woods and ground are sand, not soil
    desert: bool,
    /// how much of the bare ground is in the sand of the water's edge (a beach 0, an earthy bank more)
    bank: f32,
    /// the scrub is heather (the links)
    heather: bool,
}

fn theme(t: usize) -> Theme {
    match t {
        // desert: irrigated blue-green turf on warm orange sand, sage scrub
        1 => Theme {
            mown: rgb(104, 166, 72),
            green: rgb(122, 186, 84),
            rough: rgb(86, 140, 62),
            deep: rgb(70, 118, 52),
            ground: rgb(222, 160, 104),
            sand: rgb(240, 222, 178),
            water: rgb(46, 112, 150),
            shallow: rgb(70, 150, 165),
            collar: rgb(84, 148, 64),
            scrub: rgb(150, 156, 98),
            rock: rgb(178, 120, 84),
            flower: [rgb(236, 120, 60), rgb(250, 210, 80), rgb(210, 80, 120)],
            desert: true,
            bank: 0.15,
            heather: false,
        },
        // tropical: saturated greens, white sand, turquoise water
        2 => Theme {
            mown: rgb(80, 172, 64),
            green: rgb(108, 196, 80),
            rough: rgb(56, 140, 50),
            deep: rgb(40, 112, 44),
            ground: rgb(150, 120, 80),
            sand: rgb(248, 240, 214),
            water: rgb(30, 128, 160),
            shallow: rgb(60, 182, 190),
            collar: rgb(66, 152, 56),
            scrub: rgb(48, 120, 46),
            rock: rgb(140, 132, 120),
            flower: [rgb(240, 70, 120), rgb(250, 160, 40), rgb(170, 90, 220)],
            desert: false,
            bank: 0.05,
            heather: false,
        },
        // links: pale, muted fescue and heather by a grey sea
        3 => Theme {
            mown: rgb(116, 164, 76),
            green: rgb(126, 178, 84),
            rough: rgb(124, 156, 78),
            deep: rgb(118, 146, 74),
            ground: rgb(150, 128, 90),
            sand: rgb(228, 212, 170),
            water: rgb(48, 100, 132),
            shallow: rgb(84, 140, 152),
            collar: rgb(112, 152, 76),
            scrub: rgb(128, 96, 120),
            rock: rgb(146, 142, 132),
            flower: [rgb(186, 112, 196), rgb(244, 214, 80), rgb(244, 244, 236)],
            desert: false,
            bank: 0.35,
            heather: true,
        },
        // parkland: fresh temperate greens
        _ => Theme {
            mown: rgb(96, 168, 66),
            green: rgb(118, 190, 78),
            rough: rgb(72, 136, 52),
            deep: rgb(54, 110, 42),
            ground: rgb(136, 104, 70),
            sand: rgb(236, 224, 184),
            water: rgb(40, 104, 156),
            shallow: rgb(66, 142, 170),
            collar: rgb(78, 146, 58),
            scrub: rgb(60, 112, 46),
            rock: rgb(146, 140, 128),
            flower: [rgb(226, 60, 80), rgb(250, 210, 70), rgb(160, 90, 210)],
            desert: false,
            bank: 0.4,
            heather: false,
        },
    }
}

fn kind_of(name: &str, theme: usize) -> Kind {
    match name.to_lowercase().as_str() {
        "fairway" => Kind::Fairway,
        "firmfairway" => Kind::Firm,
        "puttinggreen" => Kind::Green,
        "trickygreen" => Kind::Tricky,
        "tee" => Kind::Tee,
        // the desert's rough is its natural ground: sand with scrub
        "rough" if theme == 1 => Kind::Scrub,
        "rough" => Kind::Rough,
        "vegetation" | "overgrowth" if theme == 1 => Kind::Scrub,
        // the desert's brush is sun-cracked earth, its marsh a dry eroded wash
        "brush" if theme == 1 => Kind::Cracks,
        "marsh" if theme == 1 => Kind::Ravine,
        "natural" if theme == 1 => Kind::Rough,
        "grassbunker" => Kind::GrassBunker,
        "deeprough" | "overgrowth" | "natural" | "vegetation" => Kind::Deep,
        "grassysand" => Kind::GrassySand,
        "zensand" => Kind::Zen,
        "brush" => Kind::Brush,
        "rock" | "cliff" => Kind::Rock,
        "woods" => Kind::Woods,
        "watershallow" | "watershallowdesert" => Kind::WaterShallow,
        "watermiddle" => Kind::WaterMiddle,
        "waterdeep" => Kind::WaterDeep,
        "marsh" => Kind::Marsh,
        "building" => Kind::Paving,
        "ravine" => Kind::Ravine,
        "flowerbed" => Kind::Flowers,
        n if n.contains("sand") || n.contains("bunker") || n.contains("bunke") => Kind::Sand,
        _ => Kind::Rough,
    }
}

/// A painted surface: colour and height (0..1) per pixel.
struct Surf {
    col: Vec<Col>,
    h: Vec<f32>,
}

impl Surf {
    fn new() -> Surf {
        Surf { col: vec![[0.0, 0.0, 0.0, 1.0]; N * N], h: vec![0.0; N * N] }
    }
}

fn grain(x: f32, y: f32, seed: u32) -> f32 {
    rnd(x as i32, y as i32, seed) - 0.5
}

/// Mown turf: base colour with mowing stripes (`stripes` bands across, 0 none), mottling and fine grain.
fn turf(x: f32, y: f32, base: Col, stripes: u32, contrast: f32, seed: u32) -> (Col, f32) {
    let band = if stripes > 0 {
        let s = (x / N as f32 * stripes as f32 * std::f32::consts::PI).sin();
        // soft-edged bands
        smooth(-0.35, 0.35, s) * 2.0 - 1.0
    } else {
        0.0
    };
    let mottle = fbm(x, y, 4, 3, seed);
    let fine = blades(x, y, seed ^ 0xa1, 32);
    let k = band * contrast + mottle * 0.05 + fine * 0.035 + grain(x, y, seed ^ 0x1f) * 0.05;
    let c = shade(base, k);
    // a little yellow where the mottling is high, a little blue where it is low
    let c = mix(c, shade(mix(base, rgb(200, 190, 90), 0.5), k), (mottle * 0.6).max(0.0) * 0.25);
    (c, 0.5 + fine * 0.08 + mottle * 0.05)
}

/// Longer grass: blades and clumps.
fn grass(x: f32, y: f32, base: Col, clump: f32, seed: u32) -> (Col, f32) {
    let b = blades(x, y, seed, 32);
    let b2 = blades(x + 7.0, y + 3.0, seed ^ 0x51, 16);
    let cl = fbm(x, y, 8, 3, seed ^ 0x99);
    let tuft = smooth(0.05, 0.45, cl) * clump;
    let k = b * 0.10 + b2 * 0.06 + cl * 0.08 + grain(x, y, seed) * 0.06 - tuft * 0.05;
    let warm = mix(base, rgb(196, 186, 96), 0.18);
    let c = mix(shade(base, k), shade(warm, k), (fbm(x, y, 2, 2, seed ^ 0x3c) * 0.8 + 0.2).clamp(0.0, 1.0) * 0.6);
    (c, 0.4 + b * 0.07 + tuft * 0.4 + cl * 0.1)
}

/// Sand: warm grain, soft ripples and a few darker grains.
fn sand(x: f32, y: f32, base: Col, seed: u32) -> (Col, f32) {
    let ripple = (((x * 0.21 + y * 0.55) + fbm(x, y, 4, 2, seed) * 6.0) * 0.9).sin();
    let dunes = fbm(x, y, 2, 3, seed ^ 0x21);
    let g = grain(x, y, seed ^ 0x4d);
    let speck = if rnd(x as i32, y as i32, seed ^ 0x66) > 0.985 { -0.18 } else { 0.0 };
    let k = dunes * 0.05 + ripple * 0.02 + g * 0.07 + speck;
    (shade(base, k), 0.45 + dunes * 0.1 + ripple * 0.04)
}

/// Bare ground (earth or desert sand) with small pebbles.
fn ground(x: f32, y: f32, base: Col, seed: u32) -> (Col, f32) {
    let (c, h) = sand(x, y, base, seed);
    let (d1, _, id) = cells(x, y, 16, seed ^ 0x2b);
    let pebble = if id % 5 == 0 { smooth(0.32, 0.18, d1) } else { 0.0 };
    let crack = fbm(x, y, 4, 3, seed ^ 0x81).abs();
    let c = shade(c, -smooth(0.06, 0.0, crack) * 0.1);
    (mix(c, shade(base, -0.25 + (id % 7) as f32 * 0.04), pebble * 0.6), h + pebble * 0.35)
}

/// A field of rounded plants (bushes, scrub, tussocks) over `under`: `n` cells across, `share` of them grown.
#[allow(clippy::too_many_arguments)]
fn plants(x: f32, y: f32, under: (Col, f32), leaf: Col, n: i32, share: u32, size: f32, seed: u32) -> (Col, f32) {
    let (d1, _, id) = cells(x, y, n, seed);
    if id % 100 >= share {
        return under;
    }
    let r = size * (0.75 + (id >> 8) as f32 % 50.0 / 100.0);
    // the leafy edge
    let edge = fbm(x, y, 16, 2, seed ^ 0x6d) * 0.12;
    let t = smooth(r + 0.04, r - 0.04, d1 + edge);
    if t <= 0.0 {
        return under;
    }
    let dome = (1.0 - (d1 / r).powi(2)).max(0.0).sqrt();
    let leafy = blades(x * 1.7, y * 1.7, seed ^ 0x12, 32) * 0.12 + grain(x, y, seed ^ 0x9) * 0.08;
    let tint = ((id >> 4) % 9) as f32 / 9.0 - 0.5;
    let c = shade(leaf, leafy + tint * 0.12);
    (mix(under.0, c, t), under.1.max(0.5 + dome * 0.5 + leafy))
}

/// Stones over `under`: `n` cells across, `share` percent of them with a stone, lit as small domes.
#[allow(clippy::too_many_arguments)]
fn stones(x: f32, y: f32, under: (Col, f32), rock: Col, n: i32, share: u32, size: f32, seed: u32) -> (Col, f32) {
    let (d1, _, id) = cells(x, y, n, seed);
    if id % 100 >= share {
        return under;
    }
    let r = size * (0.55 + (id >> 8) as f32 % 45.0 / 100.0);
    let edge = fbm(x, y, 16, 1, seed ^ 0x6d) * 0.06;
    let t = smooth(r + 0.03, r - 0.03, d1 + edge);
    // a soft contact shadow just outside the stone
    let shadow = smooth(r + 0.12, r, d1) * 0.25;
    if t <= 0.0 {
        return (shade(under.0, -shadow), under.1);
    }
    let dome = (1.0 - (d1 / r).powi(2)).max(0.0).sqrt();
    let tint = ((id >> 4) % 9) as f32 / 9.0 - 0.5;
    let c = shade(rock, tint * 0.2 + grain(x, y, seed) * 0.08 + fbm(x, y, 16, 2, seed ^ 0x2) * 0.06);
    (mix(under.0, c, t), under.1.max(0.4 + dome * 0.6))
}

fn water(x: f32, y: f32, base: Col, seed: u32, sheen: f32) -> (Col, f32) {
    let w = fbm(x, y, 2, 3, seed);
    // ripples: thin bright crests where a warped stripe field crosses zero
    let warp = fbm(x, y, 4, 2, seed ^ 0x7) * 2.5;
    let crest = |phase: f32, fx: f32, fy: f32| {
        let v = ((x * fx + y * fy) / N as f32 * std::f32::consts::TAU + warp + phase).sin();
        smooth(0.86, 1.0, v)
    };
    let r = crest(0.0, 3.0, 5.0) * 0.7 + crest(1.7, -4.0, 6.0) * 0.5;
    let spark = smooth(0.55, 0.9, fbm(x, y, 8, 2, seed ^ 0x51)) * 0.5;
    let c = shade(base, w * 0.07 + (r * (0.18 + spark * 0.4)) * sheen);
    (c, 0.0)
}

/// Open water by its depth: `level` 0 the shore line, 1 shallow, 2 middle, 3 deep (fractions blend). Below 1 the sandy bed
/// shows through, a broken line of foam laps at about 0.3, and under 0.1 it is wet, then dry beach sand. The ripples and
/// glints use one seed for every depth, so neighbouring depths join without a seam.
fn water_at(th: &Theme, x: f32, y: f32, level: f32, seed: u32) -> Col {
    let deep = mix(shade(th.water, -0.16), rgb(20, 50, 96), 0.2);
    let shallow = mix(th.shallow, th.water, 0.3);
    let base = if level >= 2.0 {
        mix(th.water, deep, level - 2.0)
    } else if level >= 1.0 {
        mix(shallow, th.water, smooth(0.0, 1.0, level - 1.0))
    } else {
        mix(mix(th.shallow, th.sand, 0.3), shallow, smooth(0.15, 1.0, level))
    };
    // the bed: sandy patches and a few dark weed beds in the shallows
    let bedn = fbm(x, y, 4, 3, seed ^ 0x13);
    let clear = smooth(1.8, 0.4, level);
    let c = mix(base, mix(th.shallow, th.sand, 0.45), smooth(0.0, 0.5, bedn) * clear * 0.35);
    let c = shade(c, -smooth(-0.1, -0.5, bedn) * clear * 0.12);
    // large soft swells and fine ripples with glints, stronger in open water
    let swell = fbm(x, y, 2, 3, seed);
    let warp = fbm(x, y, 4, 2, seed ^ 0x7) * 2.5;
    let crest = |phase: f32, fx: f32, fy: f32| {
        let v = ((x * fx + y * fy) / N as f32 * std::f32::consts::TAU + warp + phase).sin();
        smooth(0.82, 1.0, v)
    };
    let r = crest(0.0, 3.0, 5.0) * 0.7 + crest(1.7, -4.0, 6.0) * 0.5;
    let spark = smooth(0.5, 0.9, fbm(x, y, 8, 2, seed ^ 0x51));
    let open = smooth(0.4, 1.5, level);
    let mut c = shade(c, swell * 0.06 + r * (0.12 + spark * 0.35) * open);
    // foam: a broken lapping line along the shore, and a fainter second one
    if level < 0.9 {
        let brk = fbm(x, y, 16, 2, seed ^ 0x2f) * 0.18;
        let line = |at: f32, w: f32| smooth(w, 0.0, (level - at + brk * 0.6).abs());
        let foam = (line(0.3, 0.12) * (0.75 + brk * 2.0) + line(0.62, 0.07) * 0.45).clamp(0.0, 1.0);
        c = mix(c, rgb(238, 246, 244), foam * 0.85);
    }
    // the beach: wet sand at the water's edge, drying away from it
    if level < 0.16 {
        let sand = shade(mix(th.sand, th.ground, th.bank), grain(x, y, seed ^ 0x4d) * 0.08 + fbm(x, y, 8, 2, seed ^ 0x21) * 0.04);
        let wet = shade(mix(sand, th.ground, 0.3), -0.18);
        let s = mix(sand, wet, smooth(-0.6, 0.12, level));
        c = mix(c, s, smooth(0.16, 0.06, level));
    }
    c
}

fn paint(kind: Kind, th: &Theme, x: f32, y: f32, seed: u32, set: u32, var: u32) -> (Col, f32) {
    match kind {
        Kind::Fairway => turf(x, y, shade(th.mown, set as f32 * 0.015), 4, 0.07, seed),
        Kind::Firm => turf(x, y, mix(th.mown, th.sand, 0.22 + set as f32 * 0.03), 4, 0.05, seed),
        Kind::Green => {
            let (c, h) = turf(x, y, th.green, 8, 0.035, seed);
            (c, h * 0.3 + 0.35)
        }
        Kind::Tricky => {
            // a diagonal crosscut on the fast greens
            let d1 = ((x + y) / N as f32 * 4.0 * std::f32::consts::PI).sin();
            let d2 = ((x - y) / N as f32 * 4.0 * std::f32::consts::PI).sin();
            let (c, h) = turf(x, y, shade(th.green, -0.02 - set as f32 * 0.02), 0, 0.0, seed);
            (shade(c, (smooth(-0.3, 0.3, d1) + smooth(-0.3, 0.3, d2) - 1.0) * 0.045), h * 0.3 + 0.35)
        }
        Kind::Tee => {
            // tees use their number as a look: stripes one way or the other, slightly lighter or darker
            let base = shade(th.green, -0.03 + (var % 5) as f32 * 0.012);
            let (c, h) = if var.is_multiple_of(2) { turf(y, x, base, 4, 0.05, seed) } else { turf(x, y, base, 4, 0.05, seed) };
            (c, h * 0.4 + 0.3)
        }
        Kind::Rough => grass(x, y, shade(th.rough, (set as f32 - 2.0) * 0.012), 0.6, seed),
        Kind::Deep => {
            // long grass in tussocks with sun-bleached tips
            let under = grass(x, y, shade(th.deep, -0.06), 0.6, seed);
            let tips = mix(th.deep, rgb(196, 178, 96), 0.3);
            let t1 = plants(x, y, under, shade(th.deep, 0.05), 4, 60, 0.36, seed ^ 0x1d);
            let (c, h) = plants(x, y, t1, tips, 8, 22, 0.3, seed ^ 0x2d);
            let tall = blades(x, y, seed ^ 0xfe, 32);
            (shade(c, tall * 0.07), h * 0.6 + tall * 0.05)
        }
        Kind::Scrub => {
            let g = ground(x, y, shade(th.ground, (set as f32 - 2.0) * 0.02), seed);
            let share = [10, 18, 26, 6, 32][set as usize % 5];
            let tufts = plants(x, y, g, th.scrub, 8, share, 0.32, seed ^ 0x31);
            plants(x, y, tufts, shade(th.scrub, -0.15), 16, share / 2, 0.28, seed ^ 0x47)
        }
        Kind::Brush => {
            let under = grass(x, y, th.rough, 0.5, seed);
            // low shrubs among the grass (heather on the links)
            let shrub = if th.heather { mix(th.scrub, th.rough, 0.35) } else { mix(th.scrub, rgb(150, 140, 60), 0.35) };
            plants(x, y, under, shrub, 8, 50 + set * 6, 0.36, seed ^ 0x5)
        }
        Kind::Sand => sand(x, y, th.sand, seed),
        Kind::Cracks => {
            let (c, h) = ground(x, y, shade(th.ground, -0.04), seed);
            // branching cracks: where two noise fields cross zero
            let a = fbm(x, y, 2, 4, seed ^ 0x5a).abs();
            let b = fbm(x, y, 4, 3, seed ^ 0x77).abs();
            let line = smooth(0.05, 0.0, a) * (0.6 + set as f32 * 0.1) + smooth(0.035, 0.0, b) * 0.5;
            let dark = mix(th.ground, rgb(90, 50, 30), 0.7);
            (mix(c, dark, line.min(1.0) * 0.85), h - line * 0.5)
        }
        Kind::GrassBunker => {
            // long, sun-bleached grass in a hollow
            let base = mix(th.deep, rgb(184, 170, 84), 0.45);
            let (c, h) = grass(x, y, base, 0.8, seed);
            let tall = blades(x, y, seed ^ 0xfe, 16);
            (shade(c, tall * 0.08), h + smooth(0.1, 0.6, tall) * 0.3)
        }
        Kind::Zen => {
            let (c, h) = sand(x, y, shade(th.sand, 0.04), seed);
            let rake = ((y / N as f32 * 16.0 + fbm(x, y, 2, 2, seed) * 0.8) * std::f32::consts::TAU).sin();
            (shade(c, rake * 0.06), h + rake * 0.12)
        }
        Kind::GrassySand => {
            let s = sand(x, y, mix(th.sand, th.ground, 0.15), seed);
            plants(x, y, s, shade(th.rough, 0.05), 8, 45, 0.35, seed ^ 0x2)
        }
        Kind::WaterShallow => (water_at(th, x, y, 1.0, seed), 0.0),
        Kind::WaterMiddle => (water_at(th, x, y, 2.0, seed), 0.0),
        Kind::WaterDeep => (water_at(th, x, y, 3.0, seed), 0.0),
        Kind::Marsh => {
            let mud = grass(x, y, mix(th.deep, th.ground, 0.35), 0.8, seed);
            let pool = smooth(0.12, 0.2, fbm(x, y, 4, 3, seed ^ 0x8));
            let (wc, _) = water(x, y, mix(th.water, th.deep, 0.35), seed ^ 0x4, 0.6);
            let c = mix(mud.0, wc, pool);
            let reeds = smooth(0.5, 0.8, blades(x, y, seed ^ 0x77, 32)) * (1.0 - pool);
            (mix(c, shade(th.scrub, 0.1), reeds * 0.6), mud.1 * (1.0 - pool) + reeds * 0.3)
        }
        Kind::Rock => {
            // grass strewn with stones and boulders (more of them in the later sets)
            let under = grass(x, y, th.rough, 0.4, seed);
            let share = [16, 8, 22, 12, 30][set as usize % 5];
            let big = stones(x, y, under, th.rock, 4, share, 0.36, seed ^ 0x3);
            stones(x, y, big, th.rock, 8, share / 2 + 4, 0.3, seed ^ 0x17)
        }
        Kind::Woods => {
            // the forest floor in the trees' shade: moss and low leafy cover, drifts of fallen leaves, dappled light (in the
            // desert: sand with litter and a little scrub)
            let litter_col = mix(th.ground, rgb(156, 104, 54), 0.45);
            let (mut c, mut h) = if th.desert {
                ground(x, y, mix(th.ground, rgb(150, 110, 70), 0.25), seed)
            } else {
                grass(x, y, shade(mix(th.deep, th.ground, 0.22), -0.1), 0.8, seed)
            };
            let drift = smooth(0.0, 0.3, fbm(x, y, 4, 3, seed ^ 0x45));
            let (d1, _, id) = cells(x, y, 32, seed ^ 0x5c);
            let leaf = smooth(0.5, 0.3, d1);
            let tint = (id % 5) as f32 * 0.05 - 0.1;
            let lc = shade(mix(litter_col, rgb(176, 120, 44), ((id >> 3) % 3) as f32 * 0.25), tint);
            let amount = if th.desert { 0.35 } else { 0.8 };
            c = mix(c, mix(shade(litter_col, -0.12), lc, leaf), drift * amount);
            h += leaf * drift * 0.15;
            let dapple = fbm(x, y, 2, 2, seed ^ 0x93);
            c = shade(c, smooth(0.05, 0.4, dapple) * 0.07 - smooth(-0.05, -0.4, dapple) * 0.08);
            let cover = if th.desert { mix(th.scrub, th.ground, 0.2) } else { shade(th.deep, 0.08) };
            let share = if th.desert { 12 } else { 34 };
            let ferns = plants(x, y, (c, h), cover, 8, share, 0.3, seed ^ 0x81);
            plants(x, y, ferns, shade(cover, -0.12), 16, share / 2, 0.26, seed ^ 0x8f)
        }
        Kind::Paving => {
            // the grounds of a building: a light, even lawn
            let (c, h) = turf(x, y, mix(th.green, th.rough, 0.35), 0, 0.0, seed);
            (c, h * 0.5 + 0.3)
        }
        Kind::Ravine => {
            let (c, h) = ground(x, y, shade(th.ground, -0.2), seed);
            let strata = ((y / 6.0 + fbm(x, y, 4, 2, seed) * 2.0) * std::f32::consts::PI).sin();
            let crack = smooth(0.08, 0.0, fbm(x, y, 2, 4, seed ^ 0x5a).abs());
            (shade(c, strata * 0.08 - crack * 0.35), h - crack * 0.4)
        }
        Kind::Flowers => {
            // mulched soil with leafy clumps, each clump in flower in one colour
            let soil = ground(x, y, mix(th.ground, rgb(84, 58, 40), 0.55), seed);
            let (d1, _, id) = cells(x, y, 4, seed ^ 0x9);
            let r = 0.42 + (id % 7) as f32 * 0.015;
            let t = smooth(r + 0.05, r - 0.03, d1 + fbm(x, y, 16, 2, seed ^ 0x6d) * 0.1);
            if t <= 0.0 {
                return soil;
            }
            let dome = (1.0 - (d1 / r).powi(2)).max(0.0).sqrt();
            let leafc = shade(mix(th.rough, th.deep, 0.4), blades(x * 1.7, y * 1.7, seed ^ 0x12, 32) * 0.12 - 0.05);
            let f = th.flower[(id % 3) as usize];
            let (e1, _, fid) = cells(x, y, 16, seed ^ 0x3);
            let petal = smooth(0.5, 0.26, e1) * smooth(r * 1.0, r * 0.55, d1) * if fid % 5 != 0 { 1.0 } else { 0.0 };
            let c = mix(leafc, shade(f, 0.1 - e1 * 0.5), petal);
            (mix(soil.0, c, t), soil.1.max(0.5 + dome * 0.4 + petal * 0.2))
        }
    }
}

/// Fills the surface and lights its height from the upper left.
fn render(s: &mut Surf, f: impl Fn(f32, f32) -> (Col, f32), relief: f32) {
    for y in 0..N {
        for x in 0..N {
            let (c, h) = f(x as f32 + 0.5, y as f32 + 0.5);
            s.col[y * N + x] = c;
            s.h[y * N + x] = h;
        }
    }
    light(s, relief);
}

fn light(s: &mut Surf, relief: f32) {
    if relief <= 0.0 {
        return;
    }
    let at = |x: i32, y: i32| s.h[(y.rem_euclid(N as i32) as usize) * N + x.rem_euclid(N as i32) as usize];
    let mut out = s.col.clone();
    for y in 0..N as i32 {
        for x in 0..N as i32 {
            let gx = at(x + 1, y) - at(x - 1, y);
            let gy = at(x, y + 1) - at(x, y - 1);
            // light from the upper left: slopes facing up-left are lit
            let k = (-(gx + gy) * relief).clamp(-0.35, 0.3);
            out[y as usize * N + x as usize] = shade(s.col[y as usize * N + x as usize], k);
        }
    }
    s.col = out;
}

/// The signed distance to the edge of a border piece's shape (negative inside), shape 1..5, on the 64 x 64 tile.
fn shape_sd(shape: u32, x: f32, y: f32) -> f32 {
    match shape {
        2 => sd_rect(x, y, 6.0, 6.0, 52.0, 52.0, 3.0),
        3 => {
            // the fill with small collar notches in the four corners
            let corner = |cx: f32, cy: f32| 8.0 - ((x - cx).powi(2) + (y - cy).powi(2)).sqrt();
            corner(0.0, 0.0).max(corner(64.0, 0.0)).max(corner(0.0, 64.0)).max(corner(64.0, 64.0))
        }
        4 => sd_rect(x, y, 6.0, 6.0, 52.0, 52.0, 18.0),
        5 => {
            // pinched: the collar bulges in from the middle of every side
            let bulge = |cx: f32, cy: f32| 26.0 - ((x - cx).powi(2) + (y - cy).powi(2)).sqrt();
            bulge(32.0, -16.0).max(bulge(32.0, 80.0)).max(bulge(-16.0, 32.0)).max(bulge(80.0, 32.0))
        }
        _ => -100.0,
    }
}

/// Bunker shapes for the four dished sand sets: lobed outlines (1..4) cut from the tile.
fn bunker_sd(set: u32, shape: u32, x: f32, y: f32) -> f32 {
    let base = shape_sd(shape, x, y);
    if shape == 1 {
        return base;
    }
    // a few soft lobes push the edge in and out
    let a = (y - 32.0).atan2(x - 32.0);
    let wob = (a * (2 + set % 3) as f32 + set as f32).sin() * 1.6 + (a * 5.0 + set as f32 * 1.3).sin() * 0.7;
    base + wob * smooth(-12.0, 0.0, base)
}

/// Does the kind keep crisp edges (mown, raked, built) or ragged ones (grass, scrub, water banks)?
fn crisp(k: Kind) -> bool {
    matches!(k, Kind::Fairway | Kind::Firm | Kind::Green | Kind::Tricky | Kind::Tee | Kind::Sand | Kind::Zen | Kind::Paving)
}

/// A terrain texture by its file name (`<Type><Set A..E><0001..>.bmp`, or one of the cliff and wall pictures).
pub fn texture(path: &str) -> Vec<u8> {
    let low = path.to_lowercase();
    let th_i = theme_of(&low);
    let th = theme(th_i);
    let file = path.rsplit('/').next().unwrap_or(path);
    let stem = &file[..file.len().saturating_sub(4)];
    let seed = hash(&low);
    let special = stem.to_lowercase();
    if ["strat", "strata", "clifftest", "retainwalla", "retainingwalla"].contains(&special.as_str()) {
        let mut c = Canvas::new(N, N, [0.0, 0.0, 0.0, 1.0]);
        wall(&mut c, &special, &th, seed);
        return super::enc::bmp24(&c.to_rgba());
    }
    // <type><set letter><4 digits>
    let digits = &stem[stem.len().saturating_sub(4)..];
    let var: u32 = digits.parse().unwrap_or(1);
    let set = stem.as_bytes().get(stem.len().saturating_sub(5)).map(|b| (b.to_ascii_uppercase() - b'A') as u32).unwrap_or(0);
    let name = &stem[..stem.len().saturating_sub(5)];
    let kind = kind_of(name, th_i);
    let is_tee = kind == Kind::Tee;
    // tees use the number as their look (one of 25), everything else as the shape
    let is_water = matches!(kind, Kind::WaterShallow | Kind::WaterMiddle | Kind::WaterDeep);
    if is_water {
        return water_piece(kind, &th, th_i, set, var);
    }
    // tees are always the plain fill
    let shape = if is_tee {
        1
    } else if var <= 5 {
        var
    } else {
        var - 5
    };
    let second = !is_tee && var > 5;
    // a building's lawn has no border pieces of its own in the second set
    let shape = if second && kind == Kind::Paving { 1 } else { shape };
    // the look depends on type, theme and set, not on the shape, so the pieces of one area match
    let look = hash(&format!("{}{}{}", name.to_lowercase(), th_i, set));
    let relief = match kind {
        Kind::Fairway | Kind::Firm | Kind::Tee => 0.8,
        Kind::Green | Kind::Tricky => 0.5,
        Kind::WaterShallow | Kind::WaterMiddle | Kind::WaterDeep => 0.0,
        Kind::Sand | Kind::Zen | Kind::Paving => 0.9,
        // long grass: soft relief, or it reads as hammered metal
        Kind::Rough | Kind::GrassBunker => 0.45,
        Kind::Deep | Kind::Woods | Kind::Flowers | Kind::Brush | Kind::Marsh => 0.6,
        _ => 0.9,
    };
    let mut inner = Surf::new();
    render(&mut inner, |x, y| paint(kind, &th, x, y, look, set, var), relief);
    let mut out = Canvas::new(N, N, [0.0, 0.0, 0.0, 1.0]);
    if shape == 1 {
        for (i, c) in inner.col.iter().enumerate() {
            out.px[i] = *c;
        }
        let _ = seed;
        return super::enc::bmp24(&out.to_rgba());
    }
    // the outside: a low grass collar (lighter on the second set of shapes); desert ground and scrub have none in their
    // second set, they show the bare ground with fewer plants
    let ground_kind = kind == Kind::Scrub || kind == Kind::Rock;
    let collar_col = if second { shade(th.collar, 0.04) } else { th.collar };
    let mut outer = Surf::new();
    if second && ground_kind {
        // the bare base of the same surface, so the shape fades into it without a seam
        if kind == Kind::Rock {
            render(&mut outer, |x, y| grass(x, y, th.rough, 0.4, look), relief);
        } else {
            render(&mut outer, |x, y| ground(x, y, shade(th.ground, (set as f32 - 2.0) * 0.02), look), relief);
        }
    } else if second && th_i == 1 {
        // in the desert the second set of pieces stands in the bare sand
        render(&mut outer, |x, y| ground(x, y, th.ground, look ^ 0x55), 0.9);
    } else {
        render(&mut outer, |x, y| grass(x, y, collar_col, 0.3, look ^ 0x99), 0.45);
    }
    let rag = !crisp(kind);
    let is_wet = is_water || kind == Kind::Marsh;
    let is_sand = matches!(kind, Kind::Sand | Kind::Zen);
    let sd_at = |x: f32, y: f32| {
        let mut d =
            if is_sand && name.to_lowercase().starts_with("sandbunk") { bunker_sd(set + var, shape, x, y) } else { shape_sd(shape, x, y) };
        if rag {
            d += fbm(x, y, 8, 3, look ^ 0x3e) * 4.0 + fbm(x, y, 32, 1, look ^ 0x4f) * 1.2;
        }
        d
    };
    // compose colour and a height with the cut lip, then light the edge
    let mut s = Surf::new();
    for y in 0..N {
        for x in 0..N {
            let (fx, fy) = (x as f32 + 0.5, y as f32 + 0.5);
            let d = sd_at(fx, fy);
            let t = smooth(0.8, -0.8, d);
            let i = y * N + x;
            let mut c = mix(outer.col[i], inner.col[i], t);
            // water: a band of wet sand and pebbles along the bank
            if is_wet && d > -5.0 && d < 1.5 {
                let bank = smooth(-5.0, -1.0, d) * smooth(1.5, 0.0, d);
                let peb = rnd(x as i32 / 2, y as i32 / 2, look ^ 0x6) * 0.3;
                c = mix(c, shade(mix(th.sand, th.ground, 0.4), peb - 0.15), bank * 0.75);
            }
            // sand: a soft shadow under the lip
            if is_sand && d < 0.0 {
                c = shade(c, -smooth(-4.0, 0.0, d) * 0.18);
            }
            s.col[i] = c;
            // mown areas sit a little above the collar, water and sand below it
            // (a ragged edge of long grass only a little: a strong lip there reads as a stitched seam round every patch)
            let lift = if is_wet || is_sand {
                -1.0
            } else if rag {
                0.22
            } else {
                0.5
            };
            s.h[i] = smooth(1.5, -1.5, d) * lift;
        }
    }
    // light only the step (the surfaces were lit on their own)
    let at = |x: i32, y: i32| s.h[(y.rem_euclid(N as i32) as usize) * N + x.rem_euclid(N as i32) as usize];
    for y in 0..N as i32 {
        for x in 0..N as i32 {
            let gx = at(x + 1, y) - at(x - 1, y);
            let gy = at(x, y + 1) - at(x, y - 1);
            let k = (-(gx + gy) * 0.5).clamp(-0.26, 0.22);
            out.px[y as usize * N + x as usize] = shade(s.col[y as usize * N + x as usize], k);
        }
    }
    super::enc::bmp24(&out.to_rgba())
}

/// A water piece. The terrain renderer gives water the border pieces by depth (TERRAIN.md): pieces 2..5 face land or
/// shallower water, 6..9 deeper water. So the outside of a shallow piece is the shore (foam, wet sand, the beach), and the
/// outside of a deeper piece the next depth, blended wide, so a lake shades from its banks down to its deep middle.
fn water_piece(kind: Kind, th: &Theme, th_i: usize, set: u32, var: u32) -> Vec<u8> {
    let shape = if var <= 5 { var } else { var - 5 };
    let second = var > 5;
    let inner: f32 = match kind {
        Kind::WaterShallow => 1.0,
        Kind::WaterMiddle => 2.0,
        _ => 3.0,
    };
    let outer = match (kind, second) {
        (Kind::WaterShallow, false) => -0.8,
        (Kind::WaterDeep, _) => 2.0,
        (_, true) => inner + 1.0,
        _ => inner - 1.0,
    };
    // one seed per theme and set for every depth: the ripples run on across the depths
    let seed = hash(&format!("water{th_i}{set}"));
    let shore = outer < 0.0;
    let mut c = Canvas::new(N, N, [0.0, 0.0, 0.0, 1.0]);
    for y in 0..N {
        for x in 0..N {
            let (fx, fy) = (x as f32 + 0.5, y as f32 + 0.5);
            let level = if shape == 1 {
                inner
            } else {
                let d = shape_sd(shape, fx, fy) + fbm(fx, fy, 8, 3, seed ^ 0x3e) * 3.5 + fbm(fx, fy, 32, 1, seed ^ 0x4f) * 0.8;
                // the next depth is only met half way at the tile's edge (the tile beyond may not blend back)
                let t = if shore { smooth(-9.0, 9.0, d) } else { smooth(-6.0, 8.0, d) * 0.5 };
                inner + (outer - inner) * t
            };
            c.px[y * N + x] = water_at(th, fx, fy, level, seed);
        }
    }
    super::enc::bmp24(&c.to_rgba())
}

/// Wall and cliff faces: layered rock (Strat, CliffTest) or a timber retaining wall, with grass along the top.
fn wall(c: &mut Canvas, kind: &str, th: &Theme, seed: u32) {
    let mut s = Surf::new();
    let timber = kind.contains("wall");
    render(
        &mut s,
        |x, y| {
            if timber {
                // horizontal sleepers with grain and dark joints
                let row = (y / 12.8).floor();
                let fy = y.rem_euclid(12.8);
                let joint = smooth(0.4, 1.4, fy.min(12.8 - fy));
                let grainv = fbm(x, y * 6.0 % 64.0, 8, 2, seed ^ row as u32) * 0.08;
                let base = shade(rgb(128, 96, 62), (h32(row as i32, 0, seed) % 7) as f32 * 0.025 - 0.07 + grainv);
                (mix(rgb(60, 44, 30), base, joint), joint * 0.6)
            } else {
                let strata = ((y / 7.0 + fbm(x, y, 2, 2, seed) * 1.5) * std::f32::consts::PI).sin();
                let surf = fbm(x, y, 8, 3, seed ^ 0x8);
                let base = shade(th.rock, strata * 0.1 + surf * 0.1 + grain(x, y, seed) * 0.06);
                (base, strata * 0.3 + surf * 0.3)
            }
        },
        1.4,
    );
    for y in 0..N {
        for x in 0..N {
            let (fx, fy) = (x as f32 + 0.5, y as f32 + 0.5);
            let top = 6.0 + (fbm(fx, 0.0, 8, 2, seed ^ 9) * 0.5 + 0.5) * 6.0;
            let col = if fy < top {
                shade(th.collar, -(fy / top) * 0.25 + grain(fx, fy, seed) * 0.08)
            } else {
                // a shadow under the grass lip
                shade(s.col[y * N + x], -smooth(top + 4.0, top, fy) * 0.25)
            };
            c.set(x as i32, y as i32, col);
        }
    }
}

/// A path overlay (gravel without X in the name, paving with X): Path a crossing, PathCap a small square, PathCurve a disc,
/// PathInside a crossing with filled corners; RedRingThin a thin red ring.
pub fn overlay(low: &str, w: u32, h: u32) -> Vec<u8> {
    let (w, h) = (w as usize, h as usize);
    let mut c = Canvas::new(w, h, [0.0; 4]);
    let name = low.rsplit('/').next().unwrap_or(low);
    let stem = name.trim_end_matches(".tga");
    let (cx, cy) = (w as f32 / 2.0, h as f32 / 2.0);
    if stem.starts_with("redring") {
        c.fill(
            (0.0, 0.0, w as f32, h as f32),
            |x, y| (((x - cx).powi(2) + (y - cy).powi(2)).sqrt() - cx * 0.92).abs() - 1.6,
            |_, _| rgb(200, 60, 60),
        );
        return super::enc::tga32(&c.to_rgba());
    }
    let th = theme(theme_of(low));
    let paved = stem.ends_with('x');
    let kind = stem.trim_end_matches('x');
    let half = w as f32 * 0.17;
    let sd = |x: f32, y: f32| -> f32 {
        let arm_h = sd_rect(x, y, -1.0, cy - half, w as f32 + 2.0, 2.0 * half, 0.0);
        let arm_v = sd_rect(x, y, cx - half, -1.0, 2.0 * half, h as f32 + 2.0, 0.0);
        match kind {
            "pathcap" => sd_rect(x, y, cx - half * 0.8, cy - half * 0.8, half * 1.6, half * 1.6, 3.0),
            "pathcurve" => ((x - cx).powi(2) + (y - cy).powi(2)).sqrt() - half * 1.15,
            "pathinside" => {
                let cross = arm_h.min(arm_v);
                let corner = sd_rect(x, y, cx - half * 1.8, cy - half * 1.8, half * 3.6, half * 3.6, half * 0.9);
                cross.min(corner)
            }
            _ => arm_h.min(arm_v),
        }
    };
    let seed = hash(low);
    // gravel in the theme's ground colour, or pale paving stones with joints
    let base = if paved { rgb(206, 200, 186) } else { mix(th.sand, th.ground, 0.35) };
    c.fill(
        (0.0, 0.0, w as f32, h as f32),
        |x, y| {
            let rough = if paved { 0.0 } else { fbm(x % 64.0, y % 64.0, 16, 2, seed) * 1.2 };
            sd(x, y) + rough
        },
        |x, y| {
            let d = sd(x, y);
            // a darker kerb along the edge
            let edge = smooth(-2.5, -0.5, d);
            let fill = if paved {
                let (fx, fy) = (x.rem_euclid(8.0), y.rem_euclid(8.0));
                let joint = smooth(0.3, 1.0, fx.min(8.0 - fx).min(fy.min(8.0 - fy)));
                let id = h32((x / 8.0) as i32, (y / 8.0) as i32, seed);
                shade(base, -(1.0 - joint) * 0.18 + (id % 5) as f32 * 0.015 - 0.03)
            } else {
                let g = rnd(x as i32, y as i32, seed ^ 5);
                let peb = if g > 0.86 {
                    0.12
                } else if g < 0.12 {
                    -0.14
                } else {
                    0.0
                };
                shade(base, peb + (g - 0.5) * 0.06)
            };
            shade(fill, -edge * 0.22)
        },
    );
    super::enc::tga32(&c.to_rgba())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn the_fill_tiles_without_a_seam() {
        // the plain fill's left and right columns (and top and bottom rows) differ no more than neighbouring columns do
        let bmp = texture("Data/Textures/Parkland/RoughA0001.bmp");
        let img = crate::assets::decode_bmp(&bmp).unwrap();
        let px = |x: u32, y: u32| {
            let i = ((y * 64 + x) * 4) as usize;
            img.px[i] as i32 + img.px[i + 1] as i32 + img.px[i + 2] as i32
        };
        let (mut edge, mut inside) = (0, 0);
        for y in 0..64 {
            edge += (px(0, y) - px(63, y)).abs();
            inside += (px(31, y) - px(32, y)).abs();
        }
        assert!(edge < inside * 2 + 64 * 30, "edge {edge} inside {inside}");
    }

    /// For artists: `TEX_SHEET=out.png cargo test -p sg-core tex_sheet -- --ignored` writes a contact sheet of every
    /// theme's surfaces (a row per surface: the five shapes of set A, then the fill tiled 3 x 3 to show any repetition).
    #[test]
    #[ignore]
    fn tex_sheet() {
        let Ok(out) = std::env::var("TEX_SHEET") else { return };
        let names = [
            "Fairway",
            "FirmFairway",
            "PuttingGreen",
            "TrickyGreen",
            "Tee",
            "Rough",
            "DeepRough",
            "Overgrowth",
            "Brush",
            "Rock",
            "Woods",
            "SandBunker1",
            "GrassySand",
            "ZenSand",
            "GrassBunker",
            "WaterShallow",
            "WaterMiddle",
            "WaterDeep",
            "Marsh",
            "Ravine",
            "FlowerBed",
            "Building",
        ];
        let themes = ["Parkland", "Desert", "Tropical", "Links"];
        let (cw, ch) = (64 * 8 + 16, names.len() * 196);
        let mut img = crate::assets::Rgba::new((cw * themes.len()) as u32, ch as u32);
        for (ti, th) in themes.iter().enumerate() {
            for (ri, n) in names.iter().enumerate() {
                let mut put = |file: &str, x0: usize, y0: usize| {
                    let bmp = texture(&format!("Data/Textures/{th}/{file}"));
                    let t = crate::assets::decode_bmp(&bmp).unwrap();
                    for y in 0..64 {
                        for x in 0..64 {
                            let s = (y * 64 + x) * 4;
                            let d = ((y0 + y) * img.w as usize + x0 + x) * 4;
                            img.px[d..d + 4].copy_from_slice(&t.px[s..s + 4]);
                        }
                    }
                };
                let y0 = ri * 196;
                for v in 1..=5 {
                    put(&format!("{n}A{v:04}.bmp"), ti * cw + (v - 1) * 64, y0);
                }
                for v in 6..=9 {
                    put(&format!("{n}B{v:04}.bmp"), ti * cw + (v - 6) * 64, y0 + 66);
                }
                for k in 0..9 {
                    let set = ["A", "B", "C", "A", "D", "B", "E", "C", "A"][k];
                    put(&format!("{n}{set}0001.bmp"), ti * cw + 5 * 64 + 8 + (k % 3) * 64 - 64 * 2 + 128, y0 + (k / 3) * 64);
                }
            }
        }
        assert!(crate::png::write_png(&out, &img));
    }
}
