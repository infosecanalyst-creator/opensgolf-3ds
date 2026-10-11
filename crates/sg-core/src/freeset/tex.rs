//! Terrain textures: flat-shaded 64 x 64 tiles per terrain type and theme that repeat without seams, in the nine shapes the
//! terrain renderer blends with (docs/TERRAIN.md, "Edge blending": 1 the plain fill, then the border pieces: a straight
//! collar, corner notches, a rounded patch and a pinched patch; 6..9 repeat 1..4 with a different look), and the path,
//! wall and cliff overlays.
use super::paint::{mix, rgb, sd_rect, shade, smooth_noise, Canvas, Col};
use super::{hash, theme_of};

/// The look of a terrain type: base colour, the colour of the speckle or pattern, and the pattern.
#[derive(Clone, Copy)]
enum Pattern {
    Grass,
    Stripes,
    Fine,
    Tufts,
    Sand,
    Raked,
    Water,
    Marsh,
    Bushes,
    Stones,
    Litter,
    Paving,
    Strata,
    Flowers,
}

fn look(name: &str, theme: usize) -> (Col, Col, Pattern) {
    // the theme's grass: parkland fresh, desert dry, tropical lush, links pale
    let grass = [rgb(92, 158, 64), rgb(150, 160, 80), rgb(70, 165, 70), rgb(120, 150, 85)][theme];
    let rough = [rgb(68, 122, 48), rgb(176, 150, 102), rgb(52, 130, 52), rgb(104, 124, 70)][theme];
    let sand = [rgb(232, 218, 170), rgb(236, 206, 150), rgb(245, 235, 200), rgb(222, 208, 165)][theme];
    let water = [rgb(52, 110, 170), rgb(64, 120, 160), rgb(40, 150, 175), rgb(60, 100, 140)][theme];
    let n = name.to_lowercase();
    match n.as_str() {
        "tee" => (shade(grass, 0.12), shade(grass, 0.02), Pattern::Fine),
        "puttinggreen" | "trickygreen" => (shade(grass, 0.2), shade(grass, 0.1), Pattern::Fine),
        "fairway" => (shade(grass, 0.08), shade(grass, -0.04), Pattern::Stripes),
        "firmfairway" => (mix(shade(grass, 0.1), sand, 0.25), mix(grass, sand, 0.15), Pattern::Stripes),
        "rough" => (rough, shade(rough, -0.18), Pattern::Grass),
        "deeprough" | "overgrowth" | "vegetation" | "natural" => (shade(rough, -0.18), shade(rough, -0.38), Pattern::Tufts),
        "grassysand" | "grassbunker" => (mix(sand, grass, 0.25), shade(grass, -0.1), Pattern::Tufts),
        "potsandbunker" | "potbunker" | "sandbunker" | "sandbunker1" | "sandbunker2" | "sandbunker3" | "sandbunker4" | "sandbunke4" => {
            (sand, shade(sand, -0.12), Pattern::Sand)
        }
        "zensand" => (shade(sand, 0.05), shade(sand, -0.15), Pattern::Raked),
        "brush" => (rough, shade(rough, -0.35), Pattern::Bushes),
        "rock" | "cliff" => (rgb(150, 140, 125), rgb(105, 98, 90), Pattern::Stones),
        "woods" => (mix(rgb(90, 75, 45), rough, 0.4), shade(rough, -0.4), Pattern::Litter),
        "watershallow" | "watershallowdesert" => (shade(water, 0.15), shade(water, 0.35), Pattern::Water),
        "watermiddle" => (water, shade(water, 0.2), Pattern::Water),
        "waterdeep" => (shade(water, -0.25), shade(water, -0.05), Pattern::Water),
        "marsh" => (mix(rough, water, 0.35), shade(water, 0.1), Pattern::Marsh),
        "building" => (rgb(170, 160, 140), rgb(140, 130, 112), Pattern::Paving),
        "ravine" => (rgb(130, 100, 70), rgb(95, 72, 50), Pattern::Strata),
        "flowerbed" => (rgb(110, 80, 55), rgb(90, 140, 60), Pattern::Flowers),
        _ => (grass, shade(grass, -0.1), Pattern::Grass),
    }
}

/// The surroundings a border piece shows outside its shape: the theme's grass (water lies in a grassy bank).
fn collar(name: &str, theme: usize) -> Col {
    let n = name.to_lowercase();
    let (rough, _, _) = look("rough", theme);
    if n.starts_with("water") {
        mix(rough, rgb(130, 120, 95), 0.4)
    } else {
        rough
    }
}

/// The colour of the fill pattern at (x, y), repeating every 64 pixels.
fn fill(p: Pattern, base: Col, accent: Col, x: f32, y: f32, seed: u32) -> Col {
    let n1 = smooth_noise(x, y, 8.0, seed, 8);
    let n2 = smooth_noise(x, y, 4.0, seed ^ 0x55, 16);
    let fine = smooth_noise(x, y, 2.0, seed ^ 0x99, 32);
    match p {
        Pattern::Grass => mix(base, accent, (n1 * 0.6 + fine * 0.5 - 0.3).clamp(0.0, 1.0)),
        Pattern::Stripes => {
            let band = if ((x / 16.0).floor() as i32) % 2 == 0 { 0.0 } else { 1.0 };
            mix(base, accent, band * 0.5 + fine * 0.25)
        }
        Pattern::Fine => mix(base, accent, fine * 0.5 + n1 * 0.2),
        Pattern::Tufts => mix(base, accent, ((n2 - 0.45) * 3.0).clamp(0.0, 1.0) * 0.8 + fine * 0.2),
        Pattern::Sand => mix(base, accent, (fine - 0.55).max(0.0) * 2.0 + n1 * 0.15),
        Pattern::Raked => mix(base, accent, (((y + n1 * 4.0) / 4.0 * std::f32::consts::PI).sin() * 0.5 + 0.5) * 0.6),
        Pattern::Water => {
            let wave = ((x * 0.2 + y * 0.35 + n1 * 6.0).sin() * 0.5 + 0.5).powi(6);
            mix(base, accent, wave * 0.7 + n2 * 0.15)
        }
        Pattern::Marsh => {
            let pool = ((n1 - 0.55) * 6.0).clamp(0.0, 1.0);
            mix(mix(base, shade(base, -0.2), fine), accent, pool)
        }
        Pattern::Bushes => {
            let bush = ((n2 - 0.55) * 5.0).clamp(0.0, 1.0);
            mix(base, shade(accent, (fine - 0.5) * 0.4), bush)
        }
        Pattern::Stones => {
            let stone = ((n2 - 0.5) * 5.0).clamp(0.0, 1.0);
            let lit = ((smooth_noise(x - 1.0, y - 1.0, 4.0, seed ^ 0x55, 16) - n2) * 8.0).clamp(-0.3, 0.3);
            shade(mix(accent, base, stone), lit * stone)
        }
        Pattern::Litter => mix(base, accent, ((n2 - 0.4) * 2.5).clamp(0.0, 1.0) * 0.7 + fine * 0.2),
        Pattern::Paving => {
            let seam = (x % 16.0) < 1.0 || (y % 16.0) < 1.0;
            if seam {
                accent
            } else {
                mix(base, shade(base, 0.1), fine * 0.4)
            }
        }
        Pattern::Strata => mix(base, accent, (((y + n1 * 6.0) / 6.0).sin() * 0.5 + 0.5) * 0.7),
        Pattern::Flowers => {
            let spot = smooth_noise(x, y, 3.0, seed ^ 0x77, 21);
            if spot > 0.72 {
                [rgb(220, 60, 70), rgb(240, 200, 60), rgb(170, 90, 200), rgb(250, 250, 250)]
                    [((x / 3.0) as i32 + (y / 3.0) as i32).rem_euclid(4) as usize]
            } else {
                mix(base, accent, ((n2 - 0.4) * 3.0).clamp(0.0, 1.0))
            }
        }
    }
}

/// The signed distance to the edge of a border piece's shape (negative inside), shape 1..5, on the 64 x 64 tile.
fn shape_sd(shape: u32, x: f32, y: f32) -> f32 {
    match shape {
        2 => sd_rect(x, y, 6.0, 6.0, 52.0, 52.0, 2.0),
        3 => {
            // the fill with small collar notches in the four corners
            let corner = |cx: f32, cy: f32| 9.0 - ((x - cx).powi(2) + (y - cy).powi(2)).sqrt();
            -(corner(0.0, 0.0).max(corner(64.0, 0.0)).max(corner(0.0, 64.0)).max(corner(64.0, 64.0)))
        }
        4 => sd_rect(x, y, 5.0, 5.0, 54.0, 54.0, 20.0),
        5 => {
            // pinched: the collar bulges in from the middle of every side
            let bulge = |cx: f32, cy: f32| 24.0 - ((x - cx).powi(2) + (y - cy).powi(2)).sqrt();
            bulge(32.0, -14.0).max(bulge(32.0, 78.0)).max(bulge(-14.0, 32.0)).max(bulge(78.0, 32.0))
        }
        _ => -100.0,
    }
}

/// A terrain texture by its file name (`<Type><Set A..E><0001..>.bmp`, or one of the cliff and wall pictures).
pub fn texture(path: &str) -> Vec<u8> {
    let low = path.to_lowercase();
    let theme = theme_of(&low);
    let file = path.rsplit('/').next().unwrap_or(path);
    let stem = &file[..file.len().saturating_sub(4)];
    let seed = hash(&low);
    let mut c = Canvas::new(64, 64, [0.0, 0.0, 0.0, 1.0]);
    let special = stem.to_lowercase();
    if ["strata", "clifftest", "retainwalla", "retainingwalla"].contains(&special.as_str()) {
        wall(&mut c, &special, theme, seed);
        return super::enc::bmp24(&c.to_rgba());
    }
    // <type><set letter><4 digits>
    let digits = &stem[stem.len().saturating_sub(4)..];
    let var: u32 = digits.parse().unwrap_or(1);
    let set = stem.as_bytes().get(stem.len().saturating_sub(5)).map(|b| (b.to_ascii_uppercase() - b'A') as u32).unwrap_or(0);
    let name = &stem[..stem.len().saturating_sub(5)];
    let (base, accent, pat) = look(name, theme);
    let is_tee = name.eq_ignore_ascii_case("tee");
    // tees use the number as their look (one of 25), everything else as the shape
    let shape = if is_tee {
        1
    } else if var <= 5 {
        var
    } else {
        var - 5
    };
    let look_seed = seed_for(name, theme, set + if var > 5 { 7 } else { 0 } + if is_tee { var } else { 0 });
    let base = shade(base, (set as f32 - 1.0) * 0.03 + if is_tee { (var % 5) as f32 * 0.02 - 0.04 } else { 0.0 });
    let edge = collar(name, theme);
    for y in 0..64 {
        for x in 0..64 {
            let (fx, fy) = (x as f32 + 0.5, y as f32 + 0.5);
            let inner = fill(pat, base, accent, fx, fy, look_seed);
            let col = if shape == 1 {
                inner
            } else {
                let d = shape_sd(shape, fx, fy);
                let outer = fill(Pattern::Grass, edge, shade(edge, -0.15), fx, fy, look_seed ^ 3);
                let t = (0.5 - d).clamp(0.0, 1.0);
                // a darker rim just inside the edge, like a cut lip
                let rim = if d > -2.5 && d <= 0.5 { 0.18 } else { 0.0 };
                mix(outer, shade(inner, -rim), t)
            };
            c.set(x, y, col);
        }
    }
    super::enc::bmp24(&c.to_rgba())
}

fn seed_for(name: &str, theme: usize, k: u32) -> u32 {
    hash(&name.to_lowercase()).wrapping_add(theme as u32 * 7919).wrapping_add(k * 104729)
}

fn wall(c: &mut Canvas, kind: &str, theme: usize, seed: u32) {
    let (rough, _, _) = look("rough", theme);
    let rock = [rgb(150, 138, 120), rgb(190, 140, 95), rgb(150, 140, 120), rgb(140, 138, 132)][theme];
    for y in 0..64 {
        for x in 0..64 {
            let (fx, fy) = (x as f32 + 0.5, y as f32 + 0.5);
            let col = if kind.contains("wall") {
                // wooden boards
                let board = ((fx / 8.0).floor() as i32 % 2) as f32;
                let grain = smooth_noise(fx * 3.0, fy * 0.3, 4.0, seed, 48);
                mix(rgb(120, 92, 60), rgb(90, 68, 44), board * 0.4 + grain * 0.4)
            } else {
                fill(Pattern::Strata, rock, shade(rock, -0.25), fx, fy, seed)
            };
            // grass along the top
            let top = 7.0 + smooth_noise(fx, 0.0, 6.0, seed ^ 9, 11) * 5.0;
            c.set(x, y, if fy < top { shade(rough, (fy / top - 1.0) * 0.3) } else { col });
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
    let paved = stem.ends_with('x');
    let kind = stem.trim_end_matches('x');
    let half = w as f32 * 0.17;
    let sd = |x: f32, y: f32| -> f32 {
        let arm_h = sd_rect(x, y, -1.0, cy - half, w as f32 + 2.0, 2.0 * half, 0.0);
        let arm_v = sd_rect(x, y, cx - half, -1.0, 2.0 * half, h as f32 + 2.0, 0.0);
        match kind {
            "pathcap" => sd_rect(x, y, cx - half * 0.8, cy - half * 0.8, half * 1.6, half * 1.6, 2.0),
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
    let base = if paved { rgb(178, 178, 172) } else { rgb(214, 190, 150) };
    c.fill(
        (0.0, 0.0, w as f32, h as f32),
        |x, y| {
            let rough = if paved { 0.0 } else { (smooth_noise(x, y, 2.0, seed, 32) - 0.5) * 3.0 };
            sd(x, y) + rough
        },
        |x, y| {
            let g = smooth_noise(x, y, 1.5, seed ^ 5, 43);
            let edge = (-sd(x, y) / 3.0).clamp(0.0, 1.0);
            shade(mix(base, shade(base, -0.2), g * if paved { 0.25 } else { 0.6 }), (edge - 1.0) * 0.25)
        },
    );
    super::enc::tga32(&c.to_rgba())
}
