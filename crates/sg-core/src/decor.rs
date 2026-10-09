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
        0x4e => {
            pick(["Bridges/PARKbridgepal.pcx", "Bridges/DESbridgepal.pcx", "Bridges/TROPbridgepal.pcx", "Bridges/LinksBridgePalette.pcx"])
        }
        0xa9 => Some("Bridges/SCENICgenpal.pcx"),
        0x4c => Some("Flowers/CrabgrassPal.pcx"),
        0x4d => pick(["Flowers/dandelionPal.pcx", "Flowers/OilSlickPal.pcx", "Flowers/DryGrassPal.pcx", "Flowers/dandelionPal.pcx"]),
        0xa6 | 0xb6 => Some("Scenic/ScenicElmPal.pcx"),
        0x36 => Some("Trees/WillowWhite.pcx"),
        0x37 => Some("Trees/WillowLemon.pcx"),
        0x38 => Some("Trees/WillowPink.pcx"),
        0x39 => Some("Trees/WillowGreen.pcx"),
        0x59 => Some("Scenic/ScenicFlowersPal.pcx"),
        0x5e => Some("Tees/TeeMarkerPal.pcx"),
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
    fn weed_frames() {
        // a grown weed shows a frame near the end; a growing one its counter
        assert_eq!(weed(4, 0, 0, 10, None).frame, Some(9));
        assert_eq!(weed(4, 1, 2, 10, None).frame, Some(6));
        assert_eq!(weed(4, 0, 0, 10, Some(3)).frame, Some(3));
        assert_eq!(weed(2, 0, 0, 10, None).sprite, 0x184);
        assert_eq!(weed(4, 0, 0, 10, None).sprite, 0x190);
    }
}
