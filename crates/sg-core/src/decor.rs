//! What the exe draws on each tile besides the ground: trees and weeds so far (main frame tile loops at 0x410604 and
//! 0x413da3). Sprite and palette ids are the exe's; `sprite_file` and `palette_file` name the files the theme loads for them.
//!
//! Screen offsets are converted to tile units at the default view: one tile step along a is (8, -5) zoom units on screen and
//! along b (8, 5), so a screen offset (dx, dy) in zoom units is (dx / 8 - dy / 5) / 2 tiles along a and (dx / 8 + dy / 5) / 2
//! along b.

use crate::course::{N, TYPES};
use crate::geom::{DX, DY};

/// One sprite to draw on a tile.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct Draw {
    pub sprite: u16,
    /// Frame, or None for the last frame (fully grown).
    pub frame: Option<i32>,
    /// View 0..3 of a four-view sprite.
    pub view: i32,
    /// Palette id (0: the sprite's own colours).
    pub pal: u8,
    /// Offset from the tile centre in tiles along a and b.
    pub da: f32,
    pub db: f32,
}

/// A screen offset in zoom units (one unit is one pixel at the closest zoom divided by the zoom level) as tile offsets.
fn screen(dx: f32, dy: f32) -> (f32, f32) {
    ((dx / 8.0 - dy / 5.0) / 2.0, (dx / 8.0 + dy / 5.0) / 2.0)
}

/// Tile type at (a, b); off the map counts as out of bounds.
fn ty_at(ty: &[u8], a: i32, b: i32) -> u8 {
    if (0..N).contains(&a) && (0..N).contains(&b) {
        ty[(a * N + b) as usize]
    } else {
        crate::course::t::OUT
    }
}

fn group(t: u8) -> u8 {
    TYPES[(t as usize).min(22)].group
}

/// The trees of a tile of type tree (13), pine (14) or palm (15) (0x41407a..0x41491d). `grown` is false while the tile's
/// growth counter runs (a newly planted tree); `field` is the land's height noise (0x42dba0). Elms (16) and scenic tiles are
/// drawn elsewhere.
#[allow(clippy::too_many_arguments)]
pub fn trees(ty: &[u8], var: u8, scenic: bool, a: i32, b: i32, theme: u8, grown: bool, field: &dyn Fn(i32, i32) -> i32) -> Vec<Draw> {
    let own = ty_at(ty, a, b);
    if TYPES[(own as usize).min(22)].class != 13 || own == 16 || scenic {
        return Vec::new();
    }
    // which types stand around the tile, and on how many of the four axes both sides agree on being this kind of woods
    let mut mask = 0u32;
    for d in 0..8 {
        mask |= 1 << (ty_at(ty, a + DX[d], b + DY[d]) & 0x1f);
    }
    let g0 = group(own);
    let mut axes = 0;
    for d in 0..4 {
        let g1 = group(ty_at(ty, a + DX[d], b + DY[d]));
        let g2 = group(ty_at(ty, a + DX[d | 4], b + DY[d | 4]));
        if (g1 == g0) == (g2 == g0) {
            axes += 1;
        }
    }
    let size = match axes {
        0 => 0,
        1 | 2 => 1,
        _ => 2,
    };
    let v = var as i32;
    let view = (a - 2 * b) & 3;
    let pal_noise = |shift: i32| ((field(a << shift, b << shift) >> 5) & 3) as u8;
    let single = |sprite: u16, pal: u8| {
        let (da, db) = screen((((a + 3 * b) % 5) - 2) as f32, 0.0);
        Draw { sprite, frame: if grown { None } else { Some(0) }, view, pal, da, db }
    };
    // two small trees share a tile
    let pair = |sprite: u16, pal: u8| {
        let z = 8.0;
        let (a1, b1) = screen(z / if a & 1 != 0 { 3.0 } else { 5.0 }, 0.0);
        let (a2, b2) = screen(-z / if b & 1 != 0 { 3.0 } else { 5.0 }, -z / if a & 2 != 0 { 4.0 } else { -6.0 });
        let f = if grown { None } else { Some(0) };
        vec![Draw { sprite, frame: f, view, pal, da: a1, db: b1 }, Draw { sprite, frame: f, view, pal, da: a2, db: b2 }]
    };
    match own {
        13 => {
            let mut sprite = 0x19b + size;
            if v != 0 {
                sprite = 0x19b + (v % 3) as u16;
            }
            // a lone grown broadleaf in Parkland or Links is the scenic elm
            if grown && mask & (1 << 13) == 0 && (theme == 0 || theme == 3) {
                return vec![Draw { sprite: 0x131, frame: None, view, pal: 0xa6, da: 0.0, db: 0.0 }];
            }
            vec![single(sprite, 0x25 + pal_noise(8))]
        }
        14 => {
            let mut sprite = 0x192 + size;
            let n = field(a << 8, b << 8);
            let mut pal = 0x29u8;
            let desert_or_tropic = theme == 1 || theme == 2;
            let mut fixed = false;
            if (n / 13) & 4 != 0 {
                sprite += 6;
                if desert_or_tropic {
                    pal = 0x2a;
                    fixed = true;
                }
            }
            if !fixed && (theme == 0 || theme == 3) {
                pal = 0x29 + pal_noise(8);
            }
            if v != 0 {
                sprite = 0x192 + (v % 3) as u16;
                if desert_or_tropic {
                    pal = 0x29;
                }
                if (v / 3) & 1 != 0 {
                    sprite += 6;
                    if desert_or_tropic {
                        pal += 1;
                    }
                }
            }
            if sprite == 0x192 {
                pair(sprite, pal)
            } else {
                vec![single(sprite, pal)]
            }
        }
        _ => {
            let mut sprite = 0x195 + size;
            if v != 0 {
                sprite = 0x195 + (v % 3) as u16;
            }
            let pal = 0x32 + pal_noise(7);
            if sprite == 0x195 {
                pair(sprite, pal)
            } else {
                vec![single(sprite, pal)]
            }
        }
    }
}

/// Sound slot played when a newly planted tree of this type starts growing.
pub fn tree_sound(t: u8) -> Option<(i32, i32)> {
    match t {
        13 => Some((0x93, 100)),
        14 => Some((0x91, 50)),
        15 => Some((0x92, 50)),
        _ => None,
    }
}

/// The weed sprite of a tile (0x413f46): crabgrass where the type is no hazard, the theme's weed elsewhere; the frame steps
/// back from the last by the tile's position, never below its variant, and follows the growth counter while it grows.
pub fn weed(t: u8, a: i32, b: i32, frames: i32, counter: Option<i32>) -> Draw {
    let hazard = TYPES[(t as usize).min(22)].hazard;
    let (sprite, pal) = if hazard <= 0 { (0x184, 0x4c) } else { (0x190, 0x4d) };
    let lo = (a + 2 * b) & 3;
    let hi = counter.map(|c| c.clamp(0, 0x63)).unwrap_or(999);
    let v = (frames - 1 - (b & 2) - (a & 1)).max(lo);
    let frame = if v > hi && lo <= hi { hi } else { v };
    Draw { sprite, frame: Some(frame), view: 0, pal, da: 0.0, db: 0.0 }
}

/// The flag on a hole's cup tile (0x411f1c): it pops up while the tile grows, then waves in one of four views by how much
/// golfers enjoy the hole (mood sum per tee shot and half planned shot), the second view for a hole not yet open.
#[allow(clippy::too_many_arguments)]
pub fn flag(
    theme: u8,
    counter: Option<i32>,
    pop_frames: i32,
    open: bool,
    mood_sum: i32,
    tee_shots: i32,
    plans: i32,
    anim: i32,
    open_frames: i32,
) -> Draw {
    let t = theme.min(3) as u16;
    if let Some(c) = counter {
        return Draw { sprite: 0x185 + t, frame: Some(c.min(pop_frames - 1)), view: 0, pal: 0x63, da: 0.0, db: 0.0 };
    }
    let view = if open { crate::geom::clamp(mood_sum * 100 / (plans / 2 + tee_shots + 4) / 10, 0, 3) } else { 1 };
    Draw { sprite: 0x189 + t, frame: Some(anim.rem_euclid(open_frames.max(1))), view, pal: 0x63, da: 0.0, db: 0.0 }
}

/// The two tee markers of an open hole's tee (0x4120ae): red for par 3, white for par 4, blue above, set either side of the
/// tee across its facing.
pub fn tee_markers(par: i32, facing: i32, frame: Option<i32>) -> [Draw; 2] {
    let sprite = 0x18d + (par - 3).clamp(0, 2) as u16;
    let e = ((facing + 1) & 7) as usize;
    let (dx, dy) = (DX[e] as f32, DY[e] as f32);
    let (sx, sy) = if e & 1 != 0 { (4.0, 2.0) } else { (5.0, 3.0) };
    let (a1, b1) = screen(sx * dx, sy * dy);
    let (a2, b2) = screen(-sx * dx, -sy * dy);
    [Draw { sprite, frame, view: 0, pal: 0x5e, da: a1, db: b1 }, Draw { sprite, frame, view: 0, pal: 0x5e, da: a2, db: b2 }]
}

/// The bench seats of a bench tile (0x411e17): one per heading d (0, 2, 4, 6) a golfer can sit facing (`ok(d)`), pulled
/// toward that side of the tile. Empty means the tile has lost its bench.
pub fn benches(var: u8, ok: &dyn Fn(usize) -> bool) -> Vec<Draw> {
    let v = (var % 7) as u16;
    let mut out = Vec::new();
    for d in [0usize, 2, 4, 6] {
        if !ok(d) {
            continue;
        }
        let e = (d + 3) & 7;
        let (da, db) = screen(-4.0 * DX[e] as f32, -2.5 * DY[e] as f32);
        out.push(Draw { sprite: 0x208 + v, frame: None, view: (-2 - (e as i32) / 2) & 3, pal: 0xaa + v as u8, da, db });
    }
    out
}

/// An ornamental tree on a scenic elm tile (0x412bf6), over a bed of scenic flowers: the variant's low bits pick plum,
/// dogwood, elm, Japanese maple, cypress, scenic tree or peach; 0 is the willow, coloured by the noise.
pub fn ornamental(var: u8, a: i32, b: i32, grown: bool, field: &dyn Fn(i32, i32) -> i32) -> [Draw; 2] {
    let k = (var & 7) as u16;
    let view = (b - a) & 3;
    let (sprite, pal) = if k == 0 { (0xf9, 0x36 + ((field(a << 8, b << 8) >> 5) & 3) as u8) } else { (0x12e + k, 0xb3 + k as u8) };
    [
        Draw { sprite: 0xfc, frame: None, view, pal: 0x59, da: 0.0, db: 0.0 },
        Draw { sprite, frame: if grown { None } else { Some(0) }, view, pal, da: 0.0, db: 0.0 },
    ]
}

/// A flower bed tile (flag 0x1000) as the main frame draws it (0x41266b). The tile's design byte (`var`, 0..14) is shape
/// var / 5 (plain, sinuous, walled: sprites 0x1a2, 0x1a8, 0x1ae on) and colour var % 5 (palettes 0x2d..0x31, or 0xbb while
/// weeds grow in it). The bed joins the four neighbours that are beds of the same shape on the same building level
/// (0x543018): bit k of the mask for the neighbour at heading 2k, turned with the view (the mask rotated right one place
/// per quarter turn of the camera, `rot` = 0x5685f4 / 2, so it describes the neighbours as they lie on the screen), picks
/// the piece and its screen view from the table at 0x4c2f28 (single, one, two, three and four sides, corner); a one-sided
/// piece turns two views, a three-sided one view, and the single and four-sided pieces take view (a - 2b) & 3 whatever the
/// camera. The returned view is that screen view less `rot`, as the renderer adds the camera's quarter turns to every
/// four-view sprite. The bed shows its growth counter as the frame while it grows (flag
/// 0x4000), else its last frame. Where the beds fill the 2 x 2 block with the tile up and left of this one, on level
/// corners, and (b + 2a) % 5 == 0, a gazebo (a even, palette 0x96) or a topiary (palette 0xbc) stands at the block's
/// middle in view b & 3. `bed(a, b)` is the design and level of a bed tile, None for other tiles; `corner` is 0x40c170.
/// The bed is drawn straight onto the frame in the tile loop (0x4628d0), under every queued sprite; the gazebo is queued
/// (0x462a30) by the tile centre's screen height.
pub fn flower_bed(
    a: i32,
    b: i32,
    bed: &dyn Fn(i32, i32) -> Option<(u8, i32)>,
    weedy: bool,
    growth: Option<i32>,
    corner: &dyn Fn(i32, i32) -> i32,
    rot: i32,
) -> Vec<Draw> {
    const PIECES: [(u16, i32); 16] = [
        (0x1a2, -1),
        (0x1a3, 3),
        (0x1a3, 2),
        (0x1a7, 2),
        (0x1a3, 1),
        (0x1a4, 1),
        (0x1a7, 1),
        (0x1a5, 2),
        (0x1a3, 0),
        (0x1a7, 3),
        (0x1a4, 0),
        (0x1a5, 3),
        (0x1a7, 0),
        (0x1a5, 0),
        (0x1a5, 1),
        (0x1a6, -1),
    ];
    let Some((var, level)) = bed(a, b) else { return Vec::new() };
    let mut mask = 0usize;
    for k in 0..4 {
        if let Some((v, l)) = bed(a + DX[2 * k], b + DY[2 * k]) {
            if v / 5 == var / 5 && l == level {
                mask |= 1 << k;
            }
        }
    }
    for _ in 0..rot.rem_euclid(4) {
        mask = (mask >> 1) | ((mask & 1) << 3);
    }
    let (piece, mut view) = PIECES[mask];
    if piece == 0x1a3 {
        view ^= 2;
    }
    if piece == 0x1a5 {
        view = (view + 1) & 3;
    }
    if view == -1 {
        view = (a - 2 * b) & 3;
    }
    let view = (view - rot) & 3;
    let sprite = piece + 6 * (var / 5).min(2) as u16;
    let pal = if weedy { 0xbb } else { 0x2d + var % 5 };
    let frame = if weedy { None } else { growth };
    let mut out = vec![Draw { sprite, frame, view, pal, da: 0.0, db: 0.0 }];
    if (b + 2 * a) % 5 == 0 && mask & 9 == 9 && bed(a - 1, b - 1).is_some() && corner(a - 1, b - 1) == corner(a, b) {
        let (da, db) = screen(-8.0, 0.0);
        let odd = a & 1;
        out.push(Draw { sprite: 0x1b4 + odd as u16, frame, view: b & 3, pal: if odd != 0 { 0xbc } else { 0x96 }, da, db });
    }
    out
}

/// The bridge pieces of a path tile on water (0x41192c). `path` has bit k set when the neighbour at heading 2k carries a
/// path, `land` the same for neighbours that are not water, `water_path` whether any path neighbour is on water; `scenic`
/// and `style` are the scenic bridge flag and the tile's low flag bits (the bridge tool's look). Anim is the frame counter.
pub fn bridge(path: u8, land: u8, water_path: bool, scenic: bool, style: u8, anim: i32, reflect_frames: i32) -> Vec<Draw> {
    let d = |sprite: u16, view: i32, dx: f32, dy: f32, frame: Option<i32>, pal: u8| {
        let (da, db) = screen(dx, dy);
        Draw { sprite, frame, view, pal, da, db }
    };
    if !water_path {
        // a bridge on its own: the scenic bridge, or a deck with a cap at each end
        let along = (path & 5 != 0) as i32;
        if scenic {
            return vec![d(0x226 + (style & 0x1f) as u16, along, 0.0, 0.0, Some(0), 0xa9)];
        }
        let s = if along != 0 { -1.0 } else { 1.0 };
        return vec![d(0x1ff, along, 4.0 * s, -2.5, Some(0), 0x4e), d(0x1ff, along + 2, -4.0 * s, 2.5, Some(0), 0x4e)];
    }
    const PIECE: [u16; 16] = [0, 0x1fe, 0x1fe, 0x200, 0x1fe, 0x1fe, 0x200, 0x201, 0x1fe, 0x200, 0x1fe, 0x201, 0x200, 0x201, 0x201, 0x202];
    const VIEW: [i32; 16] = [0, 1, 0, 2, 1, 1, 1, 2, 0, 3, 0, 3, 0, 0, 1, 0];
    let m = (path & 15) as usize;
    let piece = PIECE[m];
    // a straight deck that reaches land on one side ends in a ramp (view by side: inferred from the order of the exe's cases)
    if piece == 0x1fe {
        if let Some(view) = match land & 15 {
            1 => Some(3),
            2 => Some(2),
            4 => Some(1),
            8 => Some(0),
            _ => None,
        } {
            return vec![d(0x205, view, 0.0, 0.0, Some(0), 0x4e)];
        }
    }
    let mut out = Vec::new();
    for k in 0..4 {
        if land & (1 << k) != 0 {
            out.push(d(0x1ff, (-1 - k) & 3, 0.0, 0.0, Some(0), 0x4e));
        }
    }
    if m != 0 {
        out.push(d(piece, VIEW[m], 0.0, 0.0, Some(0), 0x4e));
        if piece == 0x1fe {
            out.push(d(0x204, VIEW[m], 0.0, 0.0, Some(anim.rem_euclid(reflect_frames.max(1))), 0x4e));
        }
    }
    out
}

/// The file (under Flics, without extension) the theme loads for a decoration sprite id.
pub fn sprite_file(id: u16, theme: u8) -> Option<&'static str> {
    let t = theme.min(3) as usize;
    let pick = |v: [&'static str; 4]| Some(v[t]);
    match id {
        0x1fe => pick(["Bridges/bridgeTILE", "Bridges/DESbridgeTILE", "Bridges/TROPbridgeTILE", "Bridges/LinksBridgeTILE"]),
        0x1ff => pick(["Bridges/bridgeCAP", "Bridges/DESbridgeCAP", "Bridges/TROPbridgeCAP", "Bridges/LinksBridgeCAP"]),
        0x200 => pick(["Bridges/bridgeL", "Bridges/DESbridgeL", "Bridges/TROPbridgeL", "Bridges/LinksBridgeL"]),
        0x201 => pick(["Bridges/bridgeT", "Bridges/DESbridgeT", "Bridges/TROPbridgeT", "Bridges/LinksBridgeT"]),
        0x202 => pick(["Bridges/bridgeX", "Bridges/DESbridgeX", "Bridges/TROPbridgeX", "Bridges/LinksBridgeX"]),
        0x204 => pick(["Bridges/bridgeREFLECT", "Bridges/DESbridgeREFLECT", "Bridges/TROPbridgeREFLECT", "Bridges/LinksBridgeREFLECT"]),
        0x205 => pick(["Bridges/bridgeXtra", "Bridges/DESbridgeXtra", "Bridges/TROPbridgeXtra", "Bridges/LinksBridgeXtra"]),
        0x226..=0x22d => Some(
            [
                "Bridges/SCENICgen03",
                "Bridges/SCENICgen04",
                "Bridges/SCENICgen05",
                "Bridges/SCENICgen06",
                "Bridges/SCENICgen07",
                "Bridges/SCENICgen08",
                "Bridges/SCENICgen09",
                "Bridges/SCENICgen10",
            ][(id - 0x226) as usize],
        ),
        0x12f => Some("Scenic/Plum"),
        0x130 => Some("Scenic/Dogwood"),
        0x131 => Some("Scenic/ScenicElm"),
        0x132 => Some("Scenic/JapaneseMaple"),
        0x133 => Some("Scenic/Cypress"),
        0x134 => Some("Scenic/Scenic_Tree"),
        0x135 => Some("Scenic/PeachTree"),
        0xf9 => Some("Trees/WillowTree"),
        0xfc => Some("Scenic/Scenic_Flowers"),
        0x185 => pick(["Tees/FlagPARK_pop", "Tees/FlagDESERT_pop", "Tees/FlagTROP_pop", "Tees/FlagLINKS_pop"]),
        0x189 => pick(["Tees/FlagPARK_open", "Tees/FlagDESERT_open", "Tees/FlagTROP_open", "Tees/FlagLINKS_open"]),
        0x186..=0x188 => sprite_file(0x185, (id - 0x185) as u8),
        0x18a..=0x18c => sprite_file(0x189, (id - 0x189) as u8),
        0x18d => Some("Tees/TeeMarkerRed_Pop"),
        0x18e => Some("Tees/TeeMarkerWhite_Pop"),
        0x18f => Some("Tees/TeeMarkerBlue_Pop"),
        0x208 => Some("Scenic/benW01"),
        0x209 => Some("Flowers/box bench"),
        0x20a => Some("Flowers/red bench"),
        0x20b => Some("Flowers/round wood bench"),
        0x20c => Some("Flowers/backless bench"),
        0x20d => Some("Flowers/lovers bench"),
        0x184 => Some("Flowers/crabgrass"),
        // the flower beds' pieces (single, one, two, three and four sides, corner) in the plain, sinuous and walled shapes
        // (0x440bc2; Links loads the Parkland set, and the walled four-sided piece is the plain one)
        0x1a2..=0x1b3 => {
            const SETS: [[&str; 18]; 3] = [
                [
                    "Flowers/Flowers_Single",
                    "Flowers/Flowers_1Side",
                    "Flowers/Flowers_2Side",
                    "Flowers/Flowers_3Side",
                    "Flowers/Flowers_4Side",
                    "Flowers/Flowers_Corner",
                    "Flowers/SinFlowers_Single",
                    "Flowers/SinFlowers_1Side",
                    "Flowers/SinFlowers_2Side",
                    "Flowers/SinFlowers_3Side",
                    "Flowers/SinFlowers_4Side",
                    "Flowers/SinFlowers_Corner",
                    "Flowers/WalledFlowers_Single",
                    "Flowers/WalledFlowers_1Side",
                    "Flowers/WalledFlowers_2Side",
                    "Flowers/WalledFlowers_3Side",
                    "Flowers/Flowers_4Side",
                    "Flowers/WalledFlowers_Corner",
                ],
                [
                    "Flowers/DesFlowers_Single",
                    "Flowers/DesFlowers_1Side",
                    "Flowers/DesFlowers_2Side",
                    "Flowers/DesFlowers_3Side",
                    "Flowers/DesFlowers_4Side",
                    "Flowers/DesFlowers_Corner",
                    "Flowers/DesSin_Single",
                    "Flowers/DesSin_1Side",
                    "Flowers/DesSin_2Side",
                    "Flowers/DesSin_3Side",
                    "Flowers/DesSin_4Side",
                    "Flowers/DesSin_Corner",
                    "Flowers/DesFlowers_Single_Wall",
                    "Flowers/DesFlowers_1Side_Wall",
                    "Flowers/DesFlowers_2Side_Wall",
                    "Flowers/DesFlowers_3Side_Wall",
                    "Flowers/DesFlowers_4Side",
                    "Flowers/DesFlowers_Corner_Wall",
                ],
                [
                    "Flowers/TropFlowers_Single",
                    "Flowers/TropFlowers_1Side",
                    "Flowers/TropFlowers_2Side",
                    "Flowers/TropFlowers_3Side",
                    "Flowers/TropFlowers_4Side",
                    "Flowers/TropFlowers_Corner",
                    "Flowers/SinTrop_Single",
                    "Flowers/SinTrop_1Side",
                    "Flowers/SinTrop_2Side",
                    "Flowers/SinTrop_3Side",
                    "Flowers/SinTrop_4Side",
                    "Flowers/SinTrop_Corner",
                    "Flowers/TropFlowers_Single_Wall",
                    "Flowers/TropFlowers_1Side_Wall",
                    "Flowers/TropFlowers_2Side_Wall",
                    "Flowers/TropFlowers_3Side_Wall",
                    "Flowers/TropFlowers_4Side",
                    "Flowers/TropFlowers_Corner_Wall",
                ],
            ];
            Some(SETS[if t == 1 || t == 2 { t } else { 0 }][(id - 0x1a2) as usize])
        }
        0x1b4 => Some("Flowers/Gazebo"),
        0x1b5 => Some("Flowers/Topiary"),
        0x190 => pick(["Flowers/dandelion_01", "Flowers/OilSlick", "Flowers/DryGrass", "Flowers/dandelion_01"]),
        0x192 => {
            pick(["Trees/TreePineSpruceSm", "Trees/Desert/CactusA_Sm", "Trees/Tropic/Tree_Cerc/Cerc_Small", "Trees/Links/LinksPine_Small"])
        }
        0x193 => {
            pick(["Trees/TreePineSpruceMed", "Trees/Desert/CactusA_Md", "Trees/Tropic/Tree_Cerc/Cerc_Med", "Trees/Links/LinksPine_Med"])
        }
        0x194 => {
            pick(["Trees/TreePineSpruceLg", "Trees/Desert/CactusA_Lg", "Trees/Tropic/Tree_Cerc/Cerc_Large", "Trees/Links/LinksPine_Tall"])
        }
        0x198 => {
            pick(["Trees/TreePineFirSm", "Trees/Desert/CactusC_Sm", "Trees/Tropic/Tree_Drac/Drac_Small", "Trees/Links/LinksPine_Small"])
        }
        0x199 => pick(["Trees/TreePineFirMed", "Trees/Desert/CactusC_Md", "Trees/Tropic/Tree_Drac/Drac_Med", "Trees/Links/LinksPine_Med"]),
        0x19a => {
            pick(["Trees/TreePineFirLg", "Trees/Desert/CactusC_Lg", "Trees/Tropic/Tree_Drac/Drac_Large", "Trees/Links/LinksPine_Tall"])
        }
        0x195 => pick(["Trees/TreePalmSm", "Trees/Desert/TallPalm_Small", "Trees/Desert/TallPalm_Small", "Trees/Links/ScotsPine_Small"]),
        0x196 => pick(["Trees/TreePalmMed", "Trees/Desert/TallPalm_Med", "Trees/Desert/TallPalm_Med", "Trees/Links/ScotsPine_Med"]),
        0x197 => pick(["Trees/TreePalmLg", "Trees/Desert/TallPalm_Large", "Trees/Desert/TallPalm_Large", "Trees/Links/ScotsPine_Lg"]),
        0x19b => pick(["Trees/TreeMapleSmall", "Trees/Desert/JoshuaTree_Sm", "Trees/Tropic/TropicalBrush_Sm", "Trees/TreeMapleSmall"]),
        0x19c => pick(["Trees/TreeMapleMedium", "Trees/Desert/JoshuaTree_Md", "Trees/Tropic/TropicalBrush_Med", "Trees/TreeMapleMedium"]),
        0x19d => pick(["Trees/TreeMapleLarge", "Trees/Desert/JoshuaTree_Lg", "Trees/Tropic/TropicalBrush_Lg", "Trees/TreeMapleLarge"]),
        _ => None,
    }
}

/// The palette file (under Flics) the theme loads into a palette id, or None for the sprite's own colours.
pub fn palette_file(pal: u8, theme: u8) -> Option<&'static str> {
    let t = theme.min(3) as usize;
    let pick = |v: [&'static str; 4]| Some(v[t]);
    match pal {
        0x25 => {
            pick(["Trees/PalGreenMaple.pcx", "Trees/Desert/JoshuaTreePal.pcx", "Trees/Tropic/TropBrushPal.pcx", "Trees/PalGreenMaple.pcx"])
        }
        0x26 => {
            pick(["Trees/PalRedMaple.pcx", "Trees/Desert/JoshuaTreePal.pcx", "Trees/Tropic/TropBrushPal2.pcx", "Trees/PalRedMaple.pcx"])
        }
        0x27 => {
            pick(["Trees/PalBrownMaple.pcx", "Trees/Desert/JoshuaTreePal.pcx", "Trees/Tropic/TropBrushPal3.pcx", "Trees/PalBrownMaple.pcx"])
        }
        0x28 => {
            pick(["Trees/PalBlueMaple.pcx", "Trees/Desert/JoshuaTreePal.pcx", "Trees/Tropic/TropBrushPal.pcx", "Trees/PalBlueMaple.pcx"])
        }
        0x29 => pick([
            "Trees/PalPineBlue.pcx",
            "Trees/Desert/CactusAPal.pcx",
            "Trees/Tropic/Tree_Cerc/PalCerc.pcx",
            "Trees/Links/LinksPinePal.pcx",
        ]),
        0x2a => pick([
            "Trees/PalPineDarkGreen.pcx",
            "Trees/Desert/CactusCPal.pcx",
            "Trees/Tropic/Tree_Drac/PalDrac.pcx",
            "Trees/Links/LinksPinePal2.pcx",
        ]),
        0x2b => pick(["Trees/PalPineOriginal.pcx", "", "", "Trees/Links/LinksPinePal3.pcx"]).filter(|s| !s.is_empty()),
        0x2c => pick(["Trees/PalPineYellow.pcx", "", "", "Trees/Links/LinksPinePal4.pcx"]).filter(|s| !s.is_empty()),
        0x32..=0x35 => {
            let k = (pal - 0x32) as usize;
            match t {
                0 => Some(["Trees/PalOrangePalm.pcx", "Trees/PalOriginalPalm.pcx", "Trees/PalPlumPalm.pcx", "Trees/PalGreenPalm.pcx"][k]),
                1 => Some("Trees/Desert/TallPalmDesertPal.pcx"),
                2 => Some("Trees/Tropic/Tree_Tall_Palm/PalTallPalm.pcx"),
                _ => Some(
                    [
                        "Trees/Links/ScotsPinePal.pcx",
                        "Trees/Links/ScotsPinePal2.pcx",
                        "Trees/Links/ScotsPinePal3.pcx",
                        "Trees/Links/ScotsPinePal4.pcx",
                    ][k],
                ),
            }
        }
        // the flower beds' five colours (0x440bc2: Desert and Tropical their own, the others the FlowerBedA set)
        0x2d..=0x31 => {
            let k = (pal - 0x2d) as usize;
            Some(match t {
                1 => [
                    "Flowers/DesertFlowersRedPal.pcx",
                    "Flowers/DesertFlowersOrgPal.pcx",
                    "Flowers/DesertFlowersPurpPal.pcx",
                    "Flowers/DesertFlowersBluePal.pcx",
                    "Flowers/DesertFlowersPinkPal.pcx",
                ][k],
                2 => [
                    "Flowers/TropicalFlowers_AquaPal.pcx",
                    "Flowers/TropicalFlowers_OrgPal.pcx",
                    "Flowers/TropicalFlowers_PurpPal.pcx",
                    "Flowers/TropicalFlowers_BluePal.pcx",
                    "Flowers/TropicalFlowers_RedPal.pcx",
                ][k],
                _ => [
                    "Flowers/FlowerBedA_YelPal.pcx",
                    "Flowers/FlowerBedA_OrgPal.pcx",
                    "Flowers/FlowerBedA_PurpPal.pcx",
                    "Flowers/FlowerBedA_WhitePal.pcx",
                    "Flowers/FlowerBedA_RedPal.pcx",
                ][k],
            })
        }
        0x4e => {
            pick(["Bridges/PARKbridgepal.pcx", "Bridges/DESbridgepal.pcx", "Bridges/TROPbridgepal.pcx", "Bridges/LinksBridgePalette.pcx"])
        }
        0xa9 => Some("Bridges/SCENICgenpal.pcx"),
        // a weedy flower bed: palette object 0xbb (0x81ca10 + 0xbb * 0x58 = 0x820a58) is loaded only from
        // "flics\bldgs\flowers\<name>IckyPal" (0x4cc0f4, 0x4cc500, 0x4ccd1c by theme, in the theme loader 0x43dbe0), a
        // folder the disc does not have, so the exe's load (0x475840) fails before it touches the palette and the bed is
        // drawn with a never-loaded palette, whose look is decided inside jgl.dll (not decoded). The files themselves ship
        // under Flics/Flowers with the same names; PLACEHOLDER: the port uses those
        0xbb => pick([
            "Flowers/FlowerbedA_IckyPal.pcx",
            "Flowers/DesertFlowersIckyPal.pcx",
            "Flowers/TropicalFlowers_IckyPal.pcx",
            "Flowers/FlowerbedA_IckyPal.pcx",
        ]),
        // the landmarks' palettes, 100 + type (0x440bc2)
        100..=118 => Some(
            [
                "Landmarks/SundialPal.pcx",
                "Landmarks/BarnPal.pcx",
                "Landmarks/CivilWarCannonPal.pcx",
                "Landmarks/StonetwoPal.pcx",
                "Landmarks/wmillAPal.pcx",
                "Landmarks/ParklandRockPal.pcx",
                "Landmarks/CivilWarStatuePal.pcx",
                "Landmarks/LighthouseCPal.pcx",
                "Landmarks/BuddhaPal.pcx",
                "Landmarks/WindmillPal.pcx",
                "Landmarks/equestrianPal.pcx",
                "Landmarks/EasterPal.pcx",
                "Landmarks/PagodaPal.pcx",
                "Landmarks/LighthouseBPal.pcx",
                "Landmarks/ChineseHousePal.pcx",
                "Landmarks/TarpitPal.pcx",
                "Landmarks/wtow2Pal.pcx",
                "Landmarks/Radio TowerPal.pcx",
                "Landmarks/Red Oil Pump Pal.pcx",
            ][(pal - 100) as usize],
        ),
        0x96 => Some("Flowers/GazeboPal.pcx"),
        0xbc => Some("Flowers/TopiaryPal.pcx"),
        0x4c => Some("Flowers/CrabgrassPal.pcx"),
        0x4d => pick(["Flowers/dandelionPal.pcx", "Flowers/OilSlickPal.pcx", "Flowers/DryGrassPal.pcx", "Flowers/dandelionPal.pcx"]),
        0xa6 | 0xb6 => Some("Scenic/ScenicElmPal.pcx"),
        0x36 => Some("Trees/WillowWhite.pcx"),
        0x37 => Some("Trees/WillowLemon.pcx"),
        0x38 => Some("Trees/WillowPink.pcx"),
        0x39 => Some("Trees/WillowGreen.pcx"),
        0x59 => Some("Scenic/ScenicFlowersPal.pcx"),
        0x5e => Some("Tees/TeeMarkerPal.pcx"),
        // the theme loader (0x43dbe0) keeps its regular house palette in 0x60 (flics\homes\regular\parkland\A\
        // ParkHouseA_palette and so on); 0x61 and 0x62 are never loaded
        0x60 => pick([
            "Homes/Regular/Parkland/A/ParkHouseA_palette.pcx",
            "Homes/Regular/Desert/A/DesHouseA_palette.pcx",
            "Homes/Regular/Tropical/B/TropHouseB_palette.pcx",
            "Homes/Regular/Links/B/LinksHouseB_palette.pcx",
        ]),
        0x63 => pick(["Tees/Flag_PARKpal.pcx", "Tees/Flag_DESERTpal.pcx", "Tees/Flag_TROPpal.pcx", "Tees/Flag_LINKSpal.pcx"]),
        0xaa => Some("Scenic/benWPAL.pcx"),
        0xab => Some("Flowers/box benchPal.pcx"),
        0xac => Some("Flowers/redbenchPal.pcx"),
        0xad => Some("Flowers/round wood benchPal.pcx"),
        0xae => Some("Flowers/backless benchPal.pcx"),
        0xaf => Some("Flowers/LoversBenchPal.pcx"),
        0xb4 => Some("Scenic/PlumPal.pcx"),
        0xb5 => Some("Scenic/DogwoodPal.pcx"),
        0xb7 => Some("Scenic/JapaneseMaple_OrgPal.pcx"),
        0xb8 => Some("Scenic/CypressPal.pcx"),
        0xb9 => Some("Scenic/Scenic_TreePal.pcx"),
        0xba => Some("Scenic/PeachTreePal.pcx"),
        _ => None,
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::course::NN;

    #[test]
    fn rock_cuts_follow_the_sheet() {
        // Parkland set 1 (top right), the first column; Desert set 2 (lower half), column 6; empty slots and blocks past Links
        assert_eq!(rock_cut(0), Some(((539, 13, 17, 46), 5)));
        assert_eq!(rock_cut(56 + 7 + 6), Some(((359 + 0xa1, 261 + 60 + 1, 12, 46), 6)));
        assert_eq!(rock_cut(49), None);
        assert_eq!(rock_cut(4 * 56), None);
        // Links set 7, column 2, at the sheet's left
        assert_eq!(rock_cut(3 * 56 + 42 + 2), Some(((0x2c + 1, 12 + 180 + 1, 37, 46), 19)));
    }

    #[test]
    fn woods_sizes() {
        let mut ty = vec![4u8; NN];
        // a pine wood: the inner tile has pines all round, the edge one only on some axes
        for a in 10..15 {
            for b in 10..15 {
                ty[(a * N + b) as usize] = 14;
            }
        }
        let flat = |_: i32, _: i32| 0;
        let inner = trees(&ty, 0, false, 12, 12, 0, true, &flat);
        assert_eq!(inner.len(), 1);
        assert_eq!(inner[0].sprite, 0x194);
        let corner = trees(&ty, 0, false, 10, 10, 0, true, &flat);
        assert_eq!(corner[0].sprite, 0x193);
        // a lone pine stands tall; one with woods on one side of every axis is two small ones
        let mut lone = vec![4u8; NN];
        lone[(5 * N + 5) as usize] = 14;
        assert_eq!(trees(&lone, 0, false, 5, 5, 0, true, &flat)[0].sprite, 0x194);
        let mut edge = lone.clone();
        for (a, b) in [(5, 4), (6, 4), (6, 5), (6, 6)] {
            edge[(a * N + b) as usize] = 14;
        }
        let d = trees(&edge, 0, false, 5, 5, 0, true, &flat);
        assert_eq!(d.len(), 2);
        assert!(d.iter().all(|d| d.sprite == 0x192));
        // a lone broadleaf in Parkland is a scenic elm
        lone[(5 * N + 5) as usize] = 13;
        assert_eq!(trees(&lone, 0, false, 5, 5, 0, true, &flat)[0].sprite, 0x131);
        assert_eq!(trees(&lone, 0, false, 5, 5, 1, true, &flat)[0].sprite, 0x19d);
    }

    #[test]
    fn waterfall_into_lower_pool() {
        let mut c = crate::course::Course::default();
        let (lo, hi) = (crate::course::idx(10, 10), crate::course::idx(10, 9));
        c.ty[lo] = crate::course::t::WATER;
        c.ty[hi] = crate::course::t::WATER;
        // the -y neighbour stands three steps higher: its corners all at 6, this tile's near ones at sea level
        for (x, y) in [(10, 9), (11, 9), (11, 8), (10, 8)] {
            c.height[x * 51 + y] = 6;
        }
        // the wall bit says the neighbour that way stands higher: only the low tile has one, toward -y
        c.walls[lo] = 1;
        let falls = waterfalls(&c, 10, 10, 0, 4);
        assert_eq!(falls, vec![Fall { sprite: 0x237, view: 1, dx: -16, dy: -15 }, Fall { sprite: 0x23b, view: 1, dx: -16, dy: -6 }]);
        // seen from the opposite side the higher tile shows the fall going away down its far side
        assert_eq!(waterfalls(&c, 10, 9, 4, 4), vec![Fall { sprite: 0x237, view: 3, dx: -16, dy: 6 }]);
        assert!(waterfalls(&c, 10, 9, 0, 4).is_empty());
        // a tall fall when the step is more than three
        for (x, y) in [(10, 9), (11, 9), (11, 8), (10, 8)] {
            c.height[x * 51 + y] = 10;
        }
        assert_eq!(waterfalls(&c, 10, 10, 0, 4)[0], Fall { sprite: 0x238, view: 1, dx: -16, dy: -10 - 2 });
    }

    #[test]
    fn flower_beds_join_up() {
        // a 2 x 2 block of sinuous red beds at (5..6, 10..11), a lone plain bed at (20, 20)
        let bed = |a: i32, b: i32| match (a, b) {
            (5..=6, 10..=11) => Some((9, 0)),
            (20, 20) => Some((0, 0)),
            _ => None,
        };
        let flat = |_: i32, _: i32| 3;
        let lone = flower_bed(20, 20, &bed, false, None, &flat, 0);
        assert_eq!(lone, vec![Draw { sprite: 0x1a2, frame: None, view: (20 - 40) & 3, pal: 0x2d, da: 0.0, db: 0.0 }]);
        // (6, 11) has beds at headings 0 (6, 10) and 6 (5, 11): a corner, and (11 + 12) % 5 != 0 so no gazebo
        let c = flower_bed(6, 11, &bed, false, None, &flat, 0);
        assert_eq!((c.len(), c[0].sprite, c[0].view, c[0].pal), (1, 0x1a7 + 6, 3, 0x31));
        // (5, 10) has beds at headings 2 and 4: a corner the other way, weedy
        let w = flower_bed(5, 10, &bed, true, Some(2), &flat, 0);
        assert_eq!((w[0].sprite, w[0].view, w[0].pal, w[0].frame), (0x1ad, 1, 0xbb, None));
        // a block at (12..13, 38..39): (13, 39) has (39 + 26) % 5 == 0, so a topiary (a odd) at the block's middle
        let block = |a: i32, b: i32| ((12..=13).contains(&a) && (38..=39).contains(&b)).then_some((2u8, 0));
        let t = flower_bed(13, 39, &block, false, Some(1), &flat, 0);
        assert_eq!(t.len(), 2);
        assert_eq!((t[1].sprite, t[1].pal, t[1].view, t[1].frame, t[1].da, t[1].db), (0x1b5, 0xbc, 3, Some(1), -0.5, -0.5));
    }

    #[test]
    fn flower_beds_turn_with_the_view() {
        let bed = |a: i32, b: i32| match (a, b) {
            (5..=6, 10..=11) => Some((9, 0)),
            (20, 20) => Some((0, 0)),
            _ => None,
        };
        let flat = |_: i32, _: i32| 3;
        // the corner at (6, 11) (headings 0 and 6, mask 9): a quarter turn makes it mask 12, the corner piece in screen view
        // 0, which with the renderer's added quarter is view 3 here
        let c = flower_bed(6, 11, &bed, false, None, &flat, 1);
        assert_eq!((c[0].sprite, c[0].view), (0x1a7 + 6, (0 - 1) & 3));
        // after a full turn of four quarters the piece is the unturned one again
        let c4 = flower_bed(6, 11, &bed, false, None, &flat, 4);
        assert_eq!(c4, flower_bed(6, 11, &bed, false, None, &flat, 0));
        // a lone bed keeps its screen view (a - 2b) & 3 whatever the camera
        for rot in 0..4 {
            let lone = flower_bed(20, 20, &bed, false, None, &flat, rot);
            assert_eq!((lone[0].view + rot) & 3, (20 - 40) & 3);
        }
    }

    #[test]
    fn weed_frames() {
        // a grown weed shows a frame near the end; a growing one its counter
        assert_eq!(weed(4, 0, 0, 10, None).frame, Some(9));
        assert_eq!(weed(4, 1, 2, 10, None).frame, Some(6));
        assert_eq!(weed(4, 0, 0, 10, Some(3)).frame, Some(3));
        assert_eq!(weed(2, 0, 0, 10, None).sprite, 0x184);
        assert_eq!(weed(4, 0, 0, 10, None).sprite, 0x190);
    }
}

/// A rock stood on a water bank (main frame 0x41094f): the cut of cliffs01.pcx (`sheet` x, y, w, h), its anchor in the cut
/// (`ax`, `ay`) and where the anchor goes, in pixels from the tile's centre on the screen at the closest zoom (`dx`, `dy`;
/// everything scales with zoom / 4, the queue's scale 4).
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct BankRock {
    pub sheet: (i32, i32, i32, i32),
    pub ax: i32,
    pub ay: i32,
    pub dx: i32,
    pub dy: i32,
}

/// Left edges of the seven rock columns on cliffs01.pcx and the end of the last (0x445970), and the columns' anchors (the
/// tile loop's table at 0x41097d).
const ROCK_X: [i32; 8] = [1, 0x13, 0x2c, 0x52, 0x71, 0x8e, 0xa1, 0xae];
const ROCK_ANCHOR: [i32; 7] = [6, 0x20, 0x3f, 0x61, 0x81, 0x9b, 0xa7];

/// The rock cut with flat index `n` (0x445970 cuts 4 blocks by theme, 56 slots each: set s = 1..7 of 7 columns, the last 7
/// slots empty). Set s is a bank s steps high: odd sets on the sheet's upper half, even ones on the lower, right to left.
pub fn rock_cut(n: i32) -> Option<((i32, i32, i32, i32), i32)> {
    if !(0..4 * 56).contains(&n) || n % 56 >= 49 {
        return None;
    }
    let (block, k) = (n / 56, n % 56);
    let (set, col) = (k / 7 + 1, (k % 7) as usize);
    let x0 = 0x21a - (set / 2) * 179;
    let y0 = if set & 1 != 0 { 12 } else { 261 } + block * 60 + 1;
    Some(((ROCK_X[col] + x0, y0, ROCK_X[col + 1] - ROCK_X[col] - 1, 46), ROCK_ANCHOR[col] - ROCK_X[col]))
}

/// The rocks along the banks of water tile (a, b), as the tile loop stands them (0x41094f to 0x410ea1): where a wall bit
/// (the neighbour that way stands higher, 0x5619a0) faces the camera, a cut of the bank's height in steps (the higher
/// neighbour's corner, held to 0..10, less the tile's own corner) is put at the tile's back, right or left point. `rot` is
/// the exe's view (0, 2, 4, 6), `theme` its theme (0 Parkland, 1 Desert, 2 Tropical, 3 Links).
pub fn bank_rocks(c: &crate::course::Course, a: i32, b: i32, rot: i32, theme: u8) -> Vec<BankRock> {
    use crate::course::{idx, inside, t};
    let mut out = Vec::new();
    if c.ty_at(a, b) != t::WATER {
        return out;
    }
    let walls = |a: i32, b: i32| if inside(a, b) { c.walls[idx(a, b)] as i32 } else { 0 };
    let near = |d: i32| (a + DX[(d & 7) as usize], b + DY[(d & 7) as usize]);
    // the wall bit facing each of the four view directions (j = 0..3: heading rot + 2j)
    let m = |j: i32| 1 << (2 * ((j + rot / 2) & 3));
    // a neighbour's corner, held to 0..10
    let high = |d: i32, k: i32| {
        let (na, nb) = near(d);
        c.corner(na, nb, k & 7).clamp(0, 10)
    };
    let here = walls(a, b);
    let base = 56 * theme as i32;
    let mut rock = |n: i32, dx: i32, dy: i32| {
        if let Some((sheet, ax)) = rock_cut(n) {
            out.push(BankRock { sheet, ax, ay: 40, dx, dy });
        }
    };
    let own = c.corner(a, b, (rot + 1) & 7);
    if here & m(1) != 0 {
        let (na, nb) = near(rot);
        let col = if here & m(0) != 0 {
            3
        } else if walls(na, nb) & m(1) != 0 {
            1
        } else {
            0
        };
        let e = high(rot + 2, rot - 1);
        if e > own {
            rock(base + 7 * (e - own - 1) + col, 0, -20);
        }
        let (na, nb) = near(rot + 3);
        if here & m(2) == 0 && walls(na, nb) & m(0) != 0 {
            let e = high(rot + 2, rot - 3);
            if e > own {
                rock(base + 7 * (e - own - 1) + 2, 32, 0);
            }
        }
    }
    if here & m(0) != 0 && here & m(1) == 0 {
        let e = high(rot, rot - 3);
        let (na, nb) = near(rot - 2);
        if walls(na, nb) & m(0) != 0 && e > own {
            rock(base + 7 * (e - own - 1) + 4, -32, 0);
        }
        // the exe takes the neighbour at heading 2 - rot here (rot - 2 would mirror the test above)
        let (na, nb) = near(2 - rot);
        if walls(na, nb) & m(0) == 0 {
            let e = high(rot, rot + 3);
            if e > own {
                rock(base + 7 * (e - own - 1) + 5, 0, -20);
            }
        }
    }
    let (na, nb) = near(rot - 1);
    if here & m(3) != 0 && walls(na, nb) & m(2) != 0 {
        // the last column by the neighbour's own height, not the bank's
        let e = high(rot - 2, rot + 1);
        if e > 3 {
            rock(base + 7 * (e - 4) + 6, -32, 0);
        }
    }
    out
}

/// A waterfall or its spray on a water tile (main frame 0x410ea4 to 0x4114e2): sprite 0x237 (short fall), 0x238 (tall fall),
/// 0x23b (short spray) or 0x23c (tall spray) in the theme's water palette (0xb2), in a fixed view whatever the camera, its
/// anchor (`dx`, `dy`) screen pixels from the tile's centre at the exe zoom it was laid out for. The exe runs every one's
/// frames from (7 * a + tick) and draws it straight onto the frame as the tile loop reaches the tile (0x4628d0, which
/// paints at once; only 0x462a30 queues a sprite, keyed by the tile centre's screen height), so it lies under every queued
/// sprite.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct Fall {
    pub sprite: u16,
    pub view: i32,
    pub dx: i32,
    pub dy: i32,
}

/// The falls and sprays of water tile (a, b) at the exe's view `rot` (0, 2, 4, 6) and zoom `zoom` (1, 2, 4), in the exe's
/// order. Wall bits (0x5619a0: the neighbour that way stands higher) between two water tiles make them:
/// * the neighbour at heading rot (rot + 2) is water and lower than this tile (its wall bit back to this tile is set): a
///   short fall in view 3 (2), half a tile width left (right) of the centre and a third of the half height below it;
/// * the neighbour at heading rot + 2 (rot) is water and higher (this tile's wall bit that way): with d its corner rot - 3
///   (rot + 3) less this tile's corner rot + 1, a short fall lifted (d - 1) * 5 * zoom / 8 pixels when that is above 0 and a
///   short spray, or for d above 3 a tall fall lifted (2d - 13) * 5 * zoom / 8 and a tall spray; view 0 (1), half a tile
///   width right (left) of the centre. The 5 is the exe's height step (0x4c2e00, never written).
pub fn waterfalls(c: &crate::course::Course, a: i32, b: i32, rot: i32, zoom: i32) -> Vec<Fall> {
    use crate::course::{idx, inside, t};
    let mut out = Vec::new();
    if c.ty_at(a, b) != t::WATER {
        return out;
    }
    let walls = |(a, b): (i32, i32)| if inside(a, b) { c.walls[idx(a, b)] as i32 } else { 0 };
    let near = |d: i32| (a + DX[(d & 7) as usize], b + DY[(d & 7) as usize]);
    let m = |j: i32| 1 << (2 * ((j + rot / 2) & 3));
    let water = |(na, nb): (i32, i32)| c.ty_at(na, nb) == t::WATER;
    // the tile's half width (0x4c2840) and half height (the tile loop's 20 * zoom / 4) in pixels
    let (hw, hh) = (8 * zoom, 5 * zoom);
    let fall = |sprite: u16, view: i32, dx: i32, dy: i32| Fall { sprite, view, dx, dy };
    for (k, j, view, dx) in [(rot, 2, 3, -hw / 2), (rot + 2, 3, 2, hw / 2)] {
        let n = near(k);
        if walls(n) & m(j) != 0 && water(n) {
            out.push(fall(0x237, view, dx, hh / 3));
        }
    }
    let own = c.corner(a, b, (rot + 1) & 7);
    for (k, j, side, view, dx) in [(rot + 2, 1, rot - 3, 0, hw / 2), (rot, 0, rot + 3, 1, -hw / 2)] {
        let n = near(k);
        if walls((a, b)) & m(j) == 0 || !water(n) {
            continue;
        }
        let d = c.corner(n.0, n.1, side & 7) - own;
        if d <= 3 {
            let lift = (d - 1) * 5 * zoom / 8;
            if lift > 0 {
                out.push(fall(0x237, view, dx, -hh / 2 - lift));
            }
            out.push(fall(0x23b, view, dx, -hh / if lift != 0 { 3 } else { 2 }));
        } else {
            let lift = (2 * d - 13) * 5 * zoom / 8;
            out.push(fall(0x238, view, dx, -hh / 2 - lift));
            out.push(fall(0x23c, view, dx, -hh / 3));
        }
    }
    out
}

/// The file (under Flics) the theme loads for a waterfall sprite id.
pub fn fall_file(sprite: u16, theme: u8) -> Option<&'static str> {
    let pick = |v: [&'static str; 4]| Some(v[theme.min(3) as usize]);
    match sprite {
        0x237 => pick(["Water/WaterfallShortA", "Water/DesWaterfallShortA", "Water/TropWaterfallShortA", "Water/LinksWaterfallShortA"]),
        0x238 => pick(["Water/WaterfallTallA", "Water/DesWaterfallTallA", "Water/TropWaterfallTallA", "Water/LinksWaterfallTallA"]),
        0x23b => pick(["Water/sprayShortA", "Water/DesSprayShortA", "Water/TropSprayShortA", "Water/LinksSprayShortA"]),
        0x23c => pick(["Water/sprayTallA", "Water/DesSprayTallA", "Water/TropSprayTallA", "Water/LinksSprayTallA"]),
        _ => None,
    }
}
