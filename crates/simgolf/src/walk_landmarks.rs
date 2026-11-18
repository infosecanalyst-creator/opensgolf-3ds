//! Modelled landmarks for the walk camera: the lighthouses, standing stones, towers, statues and the rest.
//!
//! A landmark's picture is drawn for the overhead view, from above at an angle. Fitted to a box and projected onto its
//! faces (as small objects are, walk_objects.rs `fit_proxy`), a tall thin landmark shows its own top smeared down its
//! sides and its stripes leaning across the faces; stood up as a card it leans back. So, as for the buildings
//! (walk_models.rs) and the oil pump (walk_sky.rs), each landmark here is plain coloured geometry of the project's own
//! design, coloured after the free set's art, drawn in the walk camera's cartoon finish. A landmark with no model here is
//! drawn as its picture, upright and turned to the eye, never as a fitted box.
//!
//! Models are built in metres with y up, standing on their foot at the origin, facing +x; `Landmark::span` is the width
//! the model is scaled to match against the picture's width (walk_objects.rs), as the pump's is.

use crate::gfx::Vert;
use crate::walk_sky::{add, beam, block, lathe, mul, rod, Rgb, V};
use std::f32::consts::{PI, TAU};

/// The landmarks with a model, by the picture they stand for.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub enum Landmark {
    /// The tall striped lighthouse with its keeper's cottage.
    LighthouseB,
    /// The smaller lighthouse (its dirt base is a picture of its own, laid on the ground).
    LighthouseC,
    /// Two trilithons on a grassy mound.
    Stones,
    Sundial,
    Rock,
    WaterTower,
    RadioTower,
    Barn,
    Cannon,
    Statue,
    Buddha,
    Heads,
    Pagoda,
    Rider,
    /// The tower mill with its four sails.
    Windmill,
    /// The farm's wind pump: a lattice tower under a bladed wheel and its tail vane.
    WindPump,
}

impl Landmark {
    /// The model for a landmark picture's file name (relative to Flics/, without its extension), if it has one.
    pub fn of_picture(name: &str) -> Option<Landmark> {
        let n = name.to_ascii_lowercase();
        let n = n.strip_prefix("landmarks/")?;
        let n = n.strip_suffix(".flc").unwrap_or(n);
        if n.contains("shadow") {
            return None;
        }
        Some(match n {
            "lighthouseb" => Landmark::LighthouseB,
            "lighthousec" => Landmark::LighthouseC,
            "stone two" => Landmark::Stones,
            "sundial" | "sundialb" => Landmark::Sundial,
            "parklandrock" => Landmark::Rock,
            "wtow2" => Landmark::WaterTower,
            "radio tower" => Landmark::RadioTower,
            "barn" => Landmark::Barn,
            "civilwarcannon" => Landmark::Cannon,
            "civilwarstatue" => Landmark::Statue,
            "buddha" => Landmark::Buddha,
            "easter" => Landmark::Heads,
            "pagoda" => Landmark::Pagoda,
            "equestrian" => Landmark::Rider,
            "windmill" => Landmark::Windmill,
            "wmilla" => Landmark::WindPump,
            _ => return None,
        })
    }

    /// The model's width across, in its own metres: the picture's width is matched to it.
    pub fn span(self) -> f32 {
        match self {
            Landmark::LighthouseB => 11.0,
            Landmark::LighthouseC => 5.0,
            Landmark::Stones => 9.0,
            Landmark::Sundial => 2.0,
            Landmark::Rock => 6.0,
            Landmark::WaterTower => 7.0,
            Landmark::RadioTower => 8.0,
            Landmark::Barn => 11.0,
            Landmark::Cannon => 3.6,
            Landmark::Statue => 3.2,
            Landmark::Buddha => 3.4,
            Landmark::Heads => 8.0,
            Landmark::Pagoda => 9.0,
            Landmark::Rider => 3.6,
            Landmark::Windmill => 13.0,
            Landmark::WindPump => 5.0,
        }
    }

    /// The radius of its foot (metres, before scaling): for its soft shadow and for walking into it.
    pub fn foot(self) -> f32 {
        match self {
            Landmark::LighthouseB => 2.6,
            Landmark::LighthouseC => 1.8,
            Landmark::Stones => 3.6,
            Landmark::Sundial => 0.7,
            Landmark::Rock => 2.4,
            Landmark::WaterTower => 2.8,
            Landmark::RadioTower => 3.0,
            Landmark::Barn => 4.5,
            Landmark::Cannon => 1.2,
            Landmark::Statue => 1.2,
            Landmark::Buddha => 1.3,
            Landmark::Heads => 3.2,
            Landmark::Pagoda => 3.8,
            Landmark::Rider => 1.4,
            Landmark::Windmill => 3.4,
            Landmark::WindPump => 1.8,
        }
    }

    pub fn model(self) -> Vec<Vert> {
        match self {
            Landmark::LighthouseB => lighthouse(true),
            Landmark::LighthouseC => lighthouse(false),
            Landmark::Stones => stones(),
            Landmark::Sundial => sundial(),
            Landmark::Rock => rock(),
            Landmark::WaterTower => water_tower(),
            Landmark::RadioTower => radio_tower(),
            Landmark::Barn => barn(),
            Landmark::Cannon => cannon(),
            Landmark::Statue => statue(),
            Landmark::Buddha => buddha(),
            Landmark::Heads => heads(),
            Landmark::Pagoda => pagoda(),
            Landmark::Rider => rider(),
            Landmark::Windmill => windmill(),
            Landmark::WindPump => wind_pump(),
        }
    }
}

const UP: V = [0.0, 1.0, 0.0];
const X: V = [1.0, 0.0, 0.0];
const RED: Rgb = [0.78, 0.16, 0.14];
const WHITE: Rgb = [0.94, 0.93, 0.89];
const DARK: Rgb = [0.16, 0.16, 0.18];
const STONE: Rgb = [0.62, 0.61, 0.57];
const STONE_DARK: Rgb = [0.48, 0.47, 0.44];
const GRASS: Rgb = [0.36, 0.56, 0.24];
const BRONZE: Rgb = [0.34, 0.40, 0.36];
const GOLD: Rgb = [0.86, 0.66, 0.22];
const GLASS: Rgb = [0.98, 0.90, 0.55];
const ROOF: Rgb = [0.30, 0.32, 0.36];
const STEEL: Rgb = [0.58, 0.58, 0.60];

/// A gabled roof along x: `c` the middle of its eaves, `l` half its length, `w` half its width, `h` its rise.
fn gable(o: &mut Vec<Vert>, c: V, l: f32, w: f32, h: f32, col: Rgb) {
    let slope = (w * w + h * h).sqrt();
    for s in [-1.0f32, 1.0] {
        let up = [0.0, w / slope, s * h / slope];
        let mid = add(c, [0.0, h * 0.5, s * w * 0.5]);
        beam(o, add(mid, [-l, 0.0, 0.0]), add(mid, [l, 0.0, 0.0]), slope + 0.3, 0.18, up, col);
    }
    // the gable ends, filled under the roof as a stack of thin slabs
    let steps = 5;
    for i in 0..steps {
        let f = (i as f32 + 0.5) / steps as f32;
        let half = w * (1.0 - f);
        block(o, add(c, [0.0, h * f, 0.0]), [l - 0.05, h / steps as f32 * 0.5, half], X, UP, WHITE);
    }
}

/// A low round mound of grass, `r` across and `h` high.
fn mound(o: &mut Vec<Vert>, r: f32, h: f32, col: Rgb) {
    lathe(o, [0.0, 0.0, 0.0], UP, &[(-0.3, r), (0.0, r), (h * 0.6, r * 0.8), (h, r * 0.45), (h, 0.0)], 22, |_, _| col);
}

fn lighthouse(cottage: bool) -> Vec<Vert> {
    let mut o = Vec::with_capacity(6000);
    let (h, r0, r1) = if cottage { (17.0, 2.3, 1.5) } else { (12.0, 1.6, 1.05) };
    // the tower: a tapering round shaft in bands of red and white
    let bands = if cottage { 6 } else { 5 };
    let prof: Vec<(f32, f32)> = (0..=bands * 2)
        .map(|j| {
            let t = j as f32 / (bands * 2) as f32;
            (t * h, r0 + (r1 - r0) * t)
        })
        .collect();
    let mut p = vec![(-0.2, r0 + 0.3), (0.0, r0 + 0.3)];
    p.extend(prof.iter().copied());
    lathe(&mut o, [0.0, 0.0, 0.0], UP, &p, 20, |_, j| {
        if j < 2 {
            STONE
        } else if ((j - 2) / 2) % 2 == 0 {
            RED
        } else {
            WHITE
        }
    });
    // the gallery: a dark walkway with its rail round the lantern
    lathe(&mut o, [0.0, h, 0.0], UP, &[(0.0, 0.0), (0.0, r1 + 0.6), (0.3, r1 + 0.6), (0.3, 0.0)], 20, |_, _| DARK);
    for i in 0..12 {
        let a = TAU * i as f32 / 12.0;
        let (s, c) = a.sin_cos();
        let foot = [c * (r1 + 0.5), h + 0.3, s * (r1 + 0.5)];
        rod(&mut o, foot, add(foot, [0.0, 0.8, 0.0]), 0.05, 0.05, 4, DARK);
    }
    lathe(&mut o, [0.0, h + 1.1, 0.0], UP, &[(0.0, r1 + 0.55), (0.1, r1 + 0.55), (0.1, r1 + 0.45), (0.0, r1 + 0.45)], 20, |_, _| DARK);
    // the lantern: the lit glass and its domed cap with a vane
    lathe(&mut o, [0.0, h + 0.3, 0.0], UP, &[(0.0, 0.0), (0.0, r1 * 0.75), (1.8, r1 * 0.75), (1.8, 0.0)], 16, |_, _| GLASS);
    lathe(
        &mut o,
        [0.0, h + 2.1, 0.0],
        UP,
        &[(0.0, 0.0), (0.0, r1 * 0.95), (0.4, r1 * 0.85), (0.9, r1 * 0.5), (1.2, 0.15), (1.3, 0.0)],
        16,
        |_, _| DARK,
    );
    rod(&mut o, [0.0, h + 3.3, 0.0], [0.0, h + 4.0, 0.0], 0.06, 0.03, 4, DARK);
    // a door and a few windows up the shaft
    block(&mut o, [r0 + 0.05, 1.0, 0.0], [0.12, 1.0, 0.5], X, UP, DARK);
    for k in 1..4 {
        let y = h * k as f32 / 4.0;
        let r = r0 + (r1 - r0) * (y / h);
        block(&mut o, [r * 0.97, y, 0.0], [0.1, 0.45, 0.28], X, UP, DARK);
    }
    if cottage {
        // the keeper's cottage beside it: white walls under a grey gabled roof, a chimney
        let c = [-1.0, 0.0, 4.6];
        block(&mut o, add(c, [0.0, 1.5, 0.0]), [2.6, 1.5, 1.8], X, UP, WHITE);
        gable(&mut o, add(c, [0.0, 3.0, 0.0]), 2.8, 2.0, 1.6, ROOF);
        block(&mut o, add(c, [1.6, 4.2, 0.6]), [0.3, 0.8, 0.3], X, UP, STONE_DARK);
        block(&mut o, add(c, [2.62, 1.0, 0.0]), [0.06, 1.0, 0.45], X, UP, DARK);
        for z in [-1.0f32, 1.0] {
            block(&mut o, add(c, [0.0, 1.8, z * 1.82]), [0.5, 0.45, 0.05], X, UP, DARK);
        }
    }
    o
}

fn stones() -> Vec<Vert> {
    let mut o = Vec::with_capacity(3000);
    mound(&mut o, 4.4, 0.5, GRASS);
    // two trilithons: a pair of rough uprights and a lintel across them, each stone a little off true
    for (cx, cz, turn) in [(-1.4f32, -1.0f32, 0.3f32), (1.6, 1.2, -0.2)] {
        let along = [turn.cos(), 0.0, turn.sin()];
        let base = [cx, 0.45, cz];
        for (s, lean) in [(-1.0f32, 0.04f32), (1.0, -0.03)] {
            let foot = add(base, mul(along, s * 1.25));
            let top = add(foot, [lean, 3.6, 0.0]);
            beam(&mut o, foot, top, 0.75, 0.55, [along[2], 0.0, -along[0]], if s < 0.0 { STONE } else { STONE_DARK });
        }
        let y = 0.45 + 3.6 + 0.3;
        beam(&mut o, add([cx, y, cz], mul(along, -1.9)), add([cx, y, cz], mul(along, 1.9)), 0.7, 0.6, UP, STONE);
    }
    // and a fallen one between them
    beam(&mut o, [-0.6, 0.75, 2.0], [1.2, 0.7, 2.9], 0.8, 0.6, UP, STONE_DARK);
    o
}

fn sundial() -> Vec<Vert> {
    let mut o = Vec::with_capacity(1200);
    block(&mut o, [0.0, 0.1, 0.0], [0.9, 0.1, 0.9], X, UP, STONE_DARK);
    block(&mut o, [0.0, 0.28, 0.0], [0.65, 0.08, 0.65], X, UP, STONE);
    lathe(&mut o, [0.0, 0.36, 0.0], UP, &[(0.0, 0.3), (0.2, 0.22), (0.8, 0.16), (0.95, 0.3), (1.0, 0.3)], 12, |_, _| STONE);
    lathe(&mut o, [0.0, 1.36, 0.0], UP, &[(0.0, 0.0), (0.0, 0.42), (0.05, 0.42), (0.05, 0.0)], 18, |_, _| GOLD);
    // the gnomon
    beam(&mut o, [-0.3, 1.41, 0.0], [0.15, 1.75, 0.0], 0.04, 0.04, [0.0, 0.0, 1.0], GOLD);
    o
}

fn boulder(o: &mut Vec<Vert>, c: V, r: f32, squash: f32, col: Rgb) {
    let n = 7;
    let prof: Vec<(f32, f32)> = (0..=n)
        .map(|j| {
            let a = PI * j as f32 / n as f32;
            (-a.cos() * r * squash, a.sin() * r * (1.0 + 0.08 * ((j * 7) % 3) as f32))
        })
        .collect();
    lathe(o, add(c, [0.0, r * squash * 0.88, 0.0]), UP, &prof, 9, |i, j| {
        if ((i * 9.0) as usize).is_multiple_of(3) && j > 2 {
            STONE_DARK
        } else {
            col
        }
    });
}

fn rock() -> Vec<Vert> {
    let mut o = Vec::with_capacity(2400);
    boulder(&mut o, [0.0, 0.0, 0.0], 2.0, 0.75, STONE);
    boulder(&mut o, [1.8, 0.0, 1.2], 1.0, 0.7, STONE_DARK);
    boulder(&mut o, [-1.6, 0.0, 1.0], 0.7, 0.8, STONE);
    boulder(&mut o, [0.8, 0.0, -1.9], 0.6, 0.7, STONE_DARK);
    o
}

fn water_tower() -> Vec<Vert> {
    let mut o = Vec::with_capacity(3000);
    let tank_y = 9.0;
    // four legs with their cross braces
    let legs: Vec<V> = (0..4)
        .map(|i| {
            let a = PI * 0.25 + PI * 0.5 * i as f32;
            [a.cos() * 2.6, 0.0, a.sin() * 2.6]
        })
        .collect();
    for (i, &f) in legs.iter().enumerate() {
        let top = [f[0] * 0.75, tank_y, f[2] * 0.75];
        rod(&mut o, f, top, 0.18, 0.15, 6, STEEL);
        let g = legs[(i + 1) % 4];
        let gt = [g[0] * 0.75, tank_y, g[2] * 0.75];
        let mid = |p: V, q: V, t: f32| add(p, mul(add(q, mul(p, -1.0)), t));
        rod(&mut o, mid(f, top, 0.25), mid(g, gt, 0.6), 0.05, 0.05, 4, STEEL);
        rod(&mut o, mid(g, gt, 0.25), mid(f, top, 0.6), 0.05, 0.05, 4, STEEL);
    }
    // the tank, rust red with a pale band, its cone roof and a ladder
    lathe(&mut o, [0.0, tank_y, 0.0], UP, &[(0.0, 0.0), (0.0, 2.7), (1.6, 2.8), (3.2, 2.7), (3.2, 0.0)], 20, |_, j| {
        if j == 1 {
            [0.86, 0.82, 0.74]
        } else {
            [0.66, 0.30, 0.20]
        }
    });
    lathe(&mut o, [0.0, tank_y + 3.2, 0.0], UP, &[(0.0, 2.9), (1.3, 0.3), (1.5, 0.0)], 20, |_, _| [0.52, 0.52, 0.55]);
    rod(&mut o, [2.75, 0.0, 0.0], [2.75, tank_y + 3.0, 0.0], 0.05, 0.05, 4, DARK);
    o
}

fn radio_tower() -> Vec<Vert> {
    let mut o = Vec::with_capacity(6000);
    let h = 30.0;
    let sections = 8;
    let half = |y: f32| 2.4 * (1.0 - y / h) + 0.25;
    let corner = |y: f32, i: usize| {
        let s = [(1.0, 1.0), (-1.0, 1.0), (-1.0, -1.0), (1.0, -1.0)][i];
        [s.0 * half(y), y, s.1 * half(y)]
    };
    for k in 0..sections {
        let (y0, y1) = (h * k as f32 / sections as f32, h * (k + 1) as f32 / sections as f32);
        let col = if k % 2 == 0 { RED } else { WHITE };
        for i in 0..4 {
            let j = (i + 1) % 4;
            rod(&mut o, corner(y0, i), corner(y1, i), 0.14, 0.12, 4, col);
            rod(&mut o, corner(y0, i), corner(y1, j), 0.05, 0.05, 3, col);
            rod(&mut o, corner(y0, j), corner(y1, i), 0.05, 0.05, 3, col);
            rod(&mut o, corner(y1, i), corner(y1, j), 0.05, 0.05, 3, col);
        }
    }
    // the mast on top with its light, and the hut at its foot
    rod(&mut o, [0.0, h, 0.0], [0.0, h + 4.0, 0.0], 0.1, 0.06, 5, RED);
    lathe(&mut o, [0.0, h + 4.0, 0.0], UP, &[(0.0, 0.0), (0.0, 0.25), (0.4, 0.25), (0.45, 0.0)], 8, |_, _| RED);
    block(&mut o, [3.6, 1.2, 1.2], [1.3, 1.2, 1.0], X, UP, [0.70, 0.68, 0.62]);
    block(&mut o, [3.6, 2.5, 1.2], [1.45, 0.12, 1.15], X, UP, ROOF);
    o
}

fn barn() -> Vec<Vert> {
    let mut o = Vec::with_capacity(4000);
    let barn_red = [0.62, 0.16, 0.12];
    let (l, w, wall) = (4.8, 3.2, 3.2);
    block(&mut o, [0.0, wall * 0.5, 0.0], [l, wall * 0.5, w], X, UP, barn_red);
    // the rounded roof: a half barrel along its length, in panels round the arch, the end walls filled under it
    let n = 9;
    let arc = |j: usize| {
        let a = PI * j as f32 / n as f32;
        (-a.cos() * (w + 0.25), wall + a.sin() * 2.6)
    };
    for j in 0..n {
        let ((z0, y0), (z1, y1)) = (arc(j), arc(j + 1));
        let (zm, ym) = ((z0 + z1) * 0.5, (y0 + y1) * 0.5);
        let (dz, dy) = (z1 - z0, y1 - y0);
        let len = (dz * dz + dy * dy).sqrt();
        let up = [0.0, -dz / len, dy / len];
        let up = if up[1] < 0.0 { mul(up, -1.0) } else { up };
        beam(&mut o, [-(l + 0.25), ym, zm], [l + 0.25, ym, zm], len + 0.1, 0.16, up, ROOF);
    }
    for k in 0..6 {
        let f = (k as f32 + 0.5) / 6.0;
        let half = (w + 0.2) * (1.0 - f * f).sqrt();
        block(&mut o, [0.0, wall + 2.6 * f * 0.95, 0.0], [l, 2.6 / 12.0, half], X, UP, barn_red);
    }
    // the big doors with their white cross, and the loft door
    for s in [-1.0f32, 1.0] {
        block(&mut o, [s * (l + 0.03), 1.3, 0.0], [0.04, 1.3, 1.3], X, UP, [0.46, 0.12, 0.10]);
        beam(&mut o, [s * (l + 0.08), 0.1, -1.2], [s * (l + 0.08), 2.5, 1.2], 0.18, 0.05, [s, 0.0, 0.0], WHITE);
        beam(&mut o, [s * (l + 0.08), 0.1, 1.2], [s * (l + 0.08), 2.5, -1.2], 0.18, 0.05, [s, 0.0, 0.0], WHITE);
        block(&mut o, [s * (l + 0.03), 4.2, 0.0], [0.04, 0.6, 0.6], X, UP, DARK);
    }
    o
}

fn cannon() -> Vec<Vert> {
    let mut o = Vec::with_capacity(2000);
    let wood = [0.42, 0.28, 0.16];
    // the wheels: a rim and hub on each side of the carriage
    for s in [-1.0f32, 1.0] {
        let c = [0.0, 0.75, s * 0.55];
        lathe(&mut o, add(c, [0.0, 0.0, -0.06 * s]), [0.0, 0.0, s], &[(0.0, 0.62), (0.0, 0.75), (0.12, 0.75), (0.12, 0.62)], 18, |_, _| {
            wood
        });
        lathe(&mut o, add(c, [0.0, 0.0, -0.1 * s]), [0.0, 0.0, s], &[(0.0, 0.0), (0.0, 0.14), (0.25, 0.14), (0.25, 0.0)], 8, |_, _| DARK);
        for k in 0..6 {
            let a = TAU * k as f32 / 6.0;
            rod(&mut o, c, add(c, [a.cos() * 0.64, a.sin() * 0.64, 0.0]), 0.03, 0.03, 3, wood);
        }
    }
    // the trail down to the ground behind, and the barrel
    beam(&mut o, [-0.1, 0.75, 0.0], [-1.6, 0.1, 0.0], 0.35, 0.18, UP, wood);
    lathe(
        &mut o,
        [-0.6, 0.95, 0.0],
        norm2([1.0, 0.12, 0.0]),
        &[(0.0, 0.0), (0.0, 0.2), (0.15, 0.24), (1.9, 0.14), (2.0, 0.17), (2.05, 0.0)],
        12,
        |_, _| BRONZE,
    );
    // a few balls stacked by it
    for (x, z, y) in [(0.6f32, 1.2f32, 0.12f32), (0.85, 1.2, 0.12), (0.72, 1.2, 0.32)] {
        lathe(&mut o, [x, y - 0.12, z], UP, &[(0.0, 0.0), (0.04, 0.09), (0.12, 0.12), (0.2, 0.09), (0.24, 0.0)], 8, |_, _| DARK);
    }
    o
}

fn norm2(a: V) -> V {
    crate::walk_sky::norm(a)
}

/// A stepped stone plinth `h` high under a figure, returning the height of its top.
fn plinth(o: &mut Vec<Vert>, w: f32, h: f32) -> f32 {
    block(o, [0.0, 0.15, 0.0], [w + 0.3, 0.15, w + 0.3], X, UP, STONE_DARK);
    block(o, [0.0, 0.3 + (h - 0.5) * 0.5, 0.0], [w, (h - 0.5) * 0.5, w], X, UP, STONE);
    block(o, [0.0, h - 0.1, 0.0], [w + 0.12, 0.1, w + 0.12], X, UP, STONE_DARK);
    h
}

/// A standing figure in one colour, its feet at `at`, facing +x, `t` tall.
fn figure(o: &mut Vec<Vert>, at: V, t: f32, col: Rgb) {
    let k = t / 1.8;
    let p = |x: f32, y: f32, z: f32| add(at, [x * k, y * k, z * k]);
    for s in [-1.0f32, 1.0] {
        rod(o, p(0.0, 0.0, s * 0.12), p(0.0, 0.85, s * 0.1), 0.08 * k, 0.1 * k, 6, col);
    }
    lathe(o, p(0.0, 0.8, 0.0), UP, &[(0.0, 0.0), (0.0, 0.2 * k), (0.4 * k, 0.24 * k), (0.62 * k, 0.17 * k), (0.66 * k, 0.0)], 8, |_, _| {
        col
    });
    rod(o, p(0.0, 1.38, -0.22), p(0.05, 0.9, -0.28), 0.06 * k, 0.05 * k, 5, col);
    rod(o, p(0.0, 1.38, 0.22), p(0.25, 1.6, 0.3), 0.06 * k, 0.05 * k, 5, col);
    lathe(
        o,
        p(0.0, 1.46, 0.0),
        UP,
        &[(0.0, 0.0), (0.02 * k, 0.1 * k), (0.15 * k, 0.13 * k), (0.3 * k, 0.08 * k), (0.32 * k, 0.0)],
        8,
        |_, _| col,
    );
    // a hat's brim and crown
    lathe(
        o,
        p(0.0, 1.72, 0.0),
        UP,
        &[(0.0, 0.0), (0.0, 0.2 * k), (0.03 * k, 0.2 * k), (0.03 * k, 0.11 * k), (0.12 * k, 0.1 * k), (0.13 * k, 0.0)],
        10,
        |_, _| col,
    );
}

fn statue() -> Vec<Vert> {
    let mut o = Vec::with_capacity(2000);
    let top = plinth(&mut o, 0.9, 2.2);
    figure(&mut o, [0.0, top, 0.0], 2.0, BRONZE);
    o
}

fn buddha() -> Vec<Vert> {
    let mut o = Vec::with_capacity(2400);
    let top = plinth(&mut o, 1.0, 1.2);
    // a lotus seat, the crossed legs, the round body, the head with its topknot
    lathe(&mut o, [0.0, top, 0.0], UP, &[(0.0, 0.0), (0.0, 0.9), (0.3, 1.05), (0.35, 0.0)], 14, |_, _| [0.70, 0.22, 0.18]);
    lathe(&mut o, [0.0, top + 0.3, 0.0], UP, &[(0.0, 0.0), (0.0, 0.85), (0.35, 0.8), (0.4, 0.0)], 14, |_, _| GOLD);
    lathe(&mut o, [0.0, top + 0.6, 0.0], UP, &[(0.0, 0.62), (0.5, 0.66), (0.9, 0.5), (1.1, 0.28), (1.15, 0.0)], 14, |_, _| GOLD);
    lathe(
        &mut o,
        [0.0, top + 1.7, 0.0],
        UP,
        &[(0.0, 0.0), (0.05, 0.25), (0.3, 0.33), (0.55, 0.25), (0.62, 0.12), (0.8, 0.08), (0.85, 0.0)],
        12,
        |_, _| GOLD,
    );
    o
}

fn heads() -> Vec<Vert> {
    let mut o = Vec::with_capacity(3000);
    mound(&mut o, 4.0, 0.4, GRASS);
    let rock = [0.40, 0.38, 0.36];
    for (x, z, t) in [(-1.8f32, -1.0f32, 4.2f32), (0.2, 0.2, 4.8), (2.0, 1.3, 4.0)] {
        let b = [x, 0.35, z];
        // the long body, the deep brow and the long nose, all of the dark stone, facing +x
        block(&mut o, add(b, [0.0, t * 0.5, 0.0]), [0.6, t * 0.5, 0.75], X, UP, rock);
        block(&mut o, add(b, [0.45, t * 0.78, 0.0]), [0.3, 0.14, 0.7], X, UP, mul(rock, 0.85));
        block(&mut o, add(b, [0.68, t * 0.6, 0.0]), [0.12, t * 0.12, 0.16], X, UP, rock);
        block(&mut o, add(b, [0.58, t * 0.38, 0.0]), [0.06, 0.06, 0.45], X, UP, mul(rock, 0.7));
    }
    o
}

fn pagoda() -> Vec<Vert> {
    let mut o = Vec::with_capacity(5000);
    let wall = [0.70, 0.22, 0.16];
    let roof = [0.20, 0.36, 0.30];
    block(&mut o, [0.0, 0.3, 0.0], [3.6, 0.3, 3.6], X, UP, STONE);
    let mut y = 0.6;
    let mut w: f32 = 2.6;
    for tier in 0..5 {
        let h = if tier == 0 { 2.4 } else { 1.7 };
        block(&mut o, [0.0, y + h * 0.5, 0.0], [w, h * 0.5, w], X, UP, wall);
        y += h;
        // the eaves: a broad shallow roof with its corners turned up
        let r = w * 1.45 + 0.4;
        lathe(&mut o, [0.0, y, 0.0], UP, &[(-0.15, r * 0.98), (0.0, r), (0.5, r * 0.7), (0.75, w * 0.8), (0.75, 0.0)], 4, |_, _| roof);
        y += 0.75;
        w *= 0.8;
    }
    rod(&mut o, [0.0, y, 0.0], [0.0, y + 2.4, 0.0], 0.14, 0.05, 6, GOLD);
    for k in 0..4 {
        lathe(&mut o, [0.0, y + 0.4 + k as f32 * 0.45, 0.0], UP, &[(0.0, 0.0), (0.0, 0.28), (0.08, 0.28), (0.1, 0.0)], 8, |_, _| GOLD);
    }
    o
}

fn rider() -> Vec<Vert> {
    let mut o = Vec::with_capacity(3000);
    let top = plinth(&mut o, 1.0, 2.0);
    let b = [0.0, top, 0.0];
    let p = |x: f32, y: f32, z: f32| add(b, [x, y, z]);
    // the horse: legs, a barrel of a body, the neck up to the head, the tail
    for (x, z) in [(0.65f32, 0.22f32), (0.65, -0.22), (-0.65, 0.22), (-0.65, -0.22)] {
        rod(&mut o, p(x, 0.0, z), p(x * 0.95, 1.05, z * 0.9), 0.07, 0.1, 5, BRONZE);
    }
    lathe(&mut o, p(-0.95, 1.3, 0.0), X, &[(0.0, 0.0), (0.05, 0.28), (0.6, 0.38), (1.3, 0.36), (1.85, 0.28), (1.9, 0.0)], 10, |_, _| {
        BRONZE
    });
    rod(&mut o, p(0.8, 1.4, 0.0), p(1.15, 2.1, 0.0), 0.2, 0.13, 6, BRONZE);
    rod(&mut o, p(1.05, 2.15, 0.0), p(1.55, 1.85, 0.0), 0.15, 0.1, 6, BRONZE);
    rod(&mut o, p(-0.95, 1.45, 0.0), p(-1.3, 0.75, 0.0), 0.08, 0.04, 4, BRONZE);
    // the rider astride it
    for s in [-1.0f32, 1.0] {
        rod(&mut o, p(0.0, 1.6, s * 0.32), p(0.15, 1.05, s * 0.35), 0.07, 0.06, 5, BRONZE);
    }
    figure(&mut o, p(0.0, 0.85, 0.0), 1.6, BRONZE);
    o
}

fn windmill() -> Vec<Vert> {
    let mut o = Vec::with_capacity(5000);
    let cap = 10.0;
    // the tower: a tapering round body of white-washed stone on a dark plinth, a door and a window
    lathe(&mut o, [0.0, 0.0, 0.0], UP, &[(-0.2, 3.5), (0.0, 3.5), (0.6, 3.4), (0.6, 3.2), (cap, 2.3), (cap, 0.0)], 18, |_, j| {
        if j < 2 {
            STONE_DARK
        } else {
            [0.90, 0.88, 0.80]
        }
    });
    block(&mut o, [3.25, 1.2, 0.0], [0.2, 1.1, 0.55], X, UP, [0.36, 0.22, 0.14]);
    block(&mut o, [2.8, 5.5, 0.0], [0.2, 0.5, 0.4], X, UP, DARK);
    // the cap: a dark boat-shaped roof turned to the wind
    lathe(&mut o, [0.0, cap, 0.0], UP, &[(0.0, 2.6), (0.9, 2.4), (2.0, 1.3), (2.6, 0.3), (2.7, 0.0)], 14, |_, _| ROOF);
    // the sails on their shaft out of the cap's front, a cross of lattice blades
    let hub = [2.6, cap + 1.0, 0.0];
    rod(&mut o, [0.5, cap + 1.0, 0.0], add(hub, [0.5, 0.0, 0.0]), 0.25, 0.3, 8, DARK);
    let wood = [0.48, 0.32, 0.20];
    let canvas = [0.92, 0.88, 0.78];
    for k in 0..4 {
        let a = PI * 0.25 + PI * 0.5 * k as f32;
        let dir = [0.0, a.sin(), a.cos()];
        let tip = add(hub, mul(dir, 6.5));
        beam(&mut o, add(hub, [0.4, 0.0, 0.0]), add(tip, [0.4, 0.0, 0.0]), 0.25, 0.2, X, wood);
        // the canvas on one side of the stock
        let side = [0.0, -dir[2], dir[1]];
        let c0 = add(add(hub, mul(dir, 1.2)), mul(side, 0.6));
        let c1 = add(add(hub, mul(dir, 6.3)), mul(side, 0.6));
        beam(&mut o, add(c0, [0.45, 0.0, 0.0]), add(c1, [0.45, 0.0, 0.0]), 1.1, 0.06, X, canvas);
    }
    o
}

fn wind_pump() -> Vec<Vert> {
    let mut o = Vec::with_capacity(4000);
    let h = 9.0;
    let half = |y: f32| 1.6 * (1.0 - y / h) + 0.3;
    let corner = |y: f32, i: usize| {
        let s = [(1.0, 1.0), (-1.0, 1.0), (-1.0, -1.0), (1.0, -1.0)][i];
        [s.0 * half(y), y, s.1 * half(y)]
    };
    for k in 0..4 {
        let (y0, y1) = (h * k as f32 / 4.0, h * (k + 1) as f32 / 4.0);
        for i in 0..4 {
            let j = (i + 1) % 4;
            rod(&mut o, corner(y0, i), corner(y1, i), 0.09, 0.08, 4, STEEL);
            rod(&mut o, corner(y0, i), corner(y1, j), 0.03, 0.03, 3, STEEL);
            rod(&mut o, corner(y1, i), corner(y1, j), 0.03, 0.03, 3, STEEL);
        }
    }
    // the wheel of blades facing +x, its hub and the tail vane behind
    let hub = [0.6, h + 0.4, 0.0];
    rod(&mut o, [-0.4, h + 0.4, 0.0], hub, 0.15, 0.15, 6, DARK);
    for k in 0..16 {
        let a = TAU * k as f32 / 16.0;
        let dir = [0.0, a.sin(), a.cos()];
        let side = [0.0, -dir[2], dir[1]];
        beam(&mut o, add(hub, mul(dir, 0.4)), add(hub, mul(dir, 1.9)), 0.32, 0.04, add(X, mul(side, 0.4)), STEEL);
    }
    lathe(&mut o, hub, X, &[(0.0, 0.0), (0.0, 1.9), (0.06, 1.9), (0.06, 1.8), (0.0, 1.8)], 20, |_, _| RED);
    rod(&mut o, [-0.4, h + 0.4, 0.0], [-2.6, h + 0.5, 0.0], 0.05, 0.05, 4, DARK);
    block(&mut o, [-2.6, h + 0.6, 0.0], [0.6, 0.45, 0.03], X, UP, RED);
    // the pump rod down the middle to a little trough
    rod(&mut o, [0.0, 0.3, 0.0], [0.0, h, 0.0], 0.04, 0.04, 4, DARK);
    block(&mut o, [1.8, 0.3, 0.0], [0.9, 0.3, 0.35], X, UP, [0.40, 0.30, 0.22]);
    o
}

#[cfg(test)]
mod tests {
    use super::*;

    const ALL: [Landmark; 16] = [
        Landmark::Windmill,
        Landmark::WindPump,
        Landmark::LighthouseB,
        Landmark::LighthouseC,
        Landmark::Stones,
        Landmark::Sundial,
        Landmark::Rock,
        Landmark::WaterTower,
        Landmark::RadioTower,
        Landmark::Barn,
        Landmark::Cannon,
        Landmark::Statue,
        Landmark::Buddha,
        Landmark::Heads,
        Landmark::Pagoda,
        Landmark::Rider,
    ];

    #[test]
    fn each_model_stands_on_its_foot_and_fits_its_span() {
        for l in ALL {
            let v = l.model();
            assert!(!v.is_empty() && v.len() % 3 == 0, "{l:?}");
            let (mut lo, mut hi) = ([f32::MAX; 3], [f32::MIN; 3]);
            for p in &v {
                for k in 0..3 {
                    lo[k] = lo[k].min(p.pos[k]);
                    hi[k] = hi[k].max(p.pos[k]);
                }
                let n = p.normal;
                assert!(((n[0] * n[0] + n[1] * n[1] + n[2] * n[2]).sqrt() - 1.0).abs() < 1e-3, "{l:?} {n:?}");
            }
            assert!(lo[1] > -0.35 && lo[1] < 0.2, "{l:?} foot {}", lo[1]);
            let across = (hi[0] - lo[0]).max(hi[2] - lo[2]);
            assert!(across < l.span() * 1.3 && across > l.span() * 0.4, "{l:?} {across} vs {}", l.span());
        }
    }

    #[test]
    fn the_pictures_find_their_models() {
        assert_eq!(Landmark::of_picture("Landmarks/LighthouseB"), Some(Landmark::LighthouseB));
        assert_eq!(Landmark::of_picture("Landmarks/Stone Two"), Some(Landmark::Stones));
        assert_eq!(Landmark::of_picture("Landmarks/LighthouseB.flc"), Some(Landmark::LighthouseB));
        assert_eq!(Landmark::of_picture("Landmarks/LighthouseBShadow"), None);
        assert_eq!(Landmark::of_picture("Landmarks/Windmill"), Some(Landmark::Windmill));
        assert_eq!(Landmark::of_picture("Landmarks/TarPit"), None);
        assert_eq!(Landmark::of_picture("Scenic/blimp"), None);
    }
}
