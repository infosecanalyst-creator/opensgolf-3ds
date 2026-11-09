//! Sprites (FLC) and their palette files. Every sprite is drawn in the free set's own 256-colour palette (below) from a
//! small model chosen by the file's folder and name: people from a jointed figure posed for the animation and turned for
//! each of the 8 views, buildings as boxes with roofs, doors and windows turned for the 4 views, trees grown over their 16
//! frames, animals, flowers, benches, flags and the rest from a few shapes each. The frame size, views, frame counts and the
//! ground point (anchor) are the original file's, so the engine places every sprite exactly where it placed the original.
//!
//! The palette keeps the engine's colour slots: entries 0..96 are the golfer's clothes, skin, hat and hair ramps that the
//! swap palettes in Bodies/ recolour (crate::bodies), 255 is the transparent key, and shadow sprites use a ramp of four
//! entries whose red channel gives the darkness (crate::sprites).
use super::enc::{flc as write_flc, pcx_indexed, FlcHead};
use super::hash;
use super::paint::{noise, rgb, shade, Col};
use std::f32::consts::PI;

// ---- the palette --------------------------------------------------------------------------------------------------------------

/// A colour ramp in the palette: `n` entries from dark to light.
#[derive(Clone, Copy, Debug)]
pub struct Mat {
    pub base: u8,
    pub n: u8,
}

impl Mat {
    /// The entry for brightness t (0 dark .. 1 light).
    pub fn at(self, t: f32) -> u8 {
        self.base + ((t.clamp(0.0, 1.0) * (self.n - 1) as f32).round() as u8)
    }
}

pub const SHIRT: Mat = Mat { base: 0, n: 20 };
pub const PANTS: Mat = Mat { base: 20, n: 20 };
pub const SKIN: Mat = Mat { base: 40, n: 20 };
pub const HAT: Mat = Mat { base: 60, n: 20 };
/// Men: the hands; women: the hair.
pub const ALT: Mat = Mat { base: 80, n: 10 };
/// Women's hands.
pub const HANDS_F: Mat = Mat { base: 90, n: 7 };
pub const LEAF: Mat = Mat { base: 97, n: 16 };
pub const BARK: Mat = Mat { base: 113, n: 8 };
pub const WALL: Mat = Mat { base: 121, n: 16 };
pub const ROOF: Mat = Mat { base: 137, n: 16 };
pub const STONE: Mat = Mat { base: 153, n: 12 };
pub const WATER: Mat = Mat { base: 165, n: 12 };
pub const EARTH: Mat = Mat { base: 177, n: 8 };
pub const DARK: Mat = Mat { base: 185, n: 8 };
pub const WHITE: Mat = Mat { base: 193, n: 8 };
pub const RED: Mat = Mat { base: 201, n: 8 };
pub const YELLOW: Mat = Mat { base: 209, n: 8 };
pub const PURPLE: Mat = Mat { base: 217, n: 8 };
pub const WOOD: Mat = Mat { base: 225, n: 8 };
pub const GOLD: Mat = Mat { base: 233, n: 8 };
pub const SHOE: Mat = Mat { base: 241, n: 8 };
pub const KEY: u8 = 255;

/// The colours of the golfer's slots for swap file `n` (0..9): shirt, trousers, skin, hat, hair or men's hands.
fn person_colours(n: usize, female: bool) -> [Col; 6] {
    const SHIRTS: [Col; 10] = [
        rgb(60, 110, 200),
        rgb(200, 60, 60),
        rgb(60, 160, 80),
        rgb(235, 200, 60),
        rgb(235, 235, 235),
        rgb(140, 80, 180),
        rgb(240, 140, 50),
        rgb(40, 160, 160),
        rgb(240, 130, 170),
        rgb(50, 55, 65),
    ];
    const PANTS: [Col; 10] = [
        rgb(200, 180, 130),
        rgb(40, 50, 90),
        rgb(235, 235, 230),
        rgb(120, 120, 125),
        rgb(40, 40, 45),
        rgb(120, 85, 55),
        rgb(70, 110, 60),
        rgb(160, 50, 60),
        rgb(120, 160, 210),
        rgb(225, 215, 180),
    ];
    const SKINS: [Col; 4] = [rgb(240, 200, 165), rgb(205, 150, 105), rgb(150, 100, 65), rgb(100, 65, 45)];
    const HAIR: [Col; 10] = [
        rgb(90, 60, 35),
        rgb(30, 25, 25),
        rgb(225, 190, 110),
        rgb(170, 70, 35),
        rgb(170, 170, 170),
        rgb(60, 40, 25),
        rgb(240, 230, 200),
        rgb(120, 80, 50),
        rgb(200, 140, 70),
        rgb(70, 70, 80),
    ];
    let skin = SKINS[n % 4];
    [SHIRTS[n % 10], PANTS[n % 10], skin, SHIRTS[(n + 4) % 10], if female { HAIR[n % 10] } else { SKINS[n % 4] }, skin]
}

fn put_ramp(p: &mut [[u8; 3]; 256], m: Mat, c: Col) {
    for i in 0..m.n {
        let t = i as f32 / (m.n - 1).max(1) as f32;
        // dark shadow to a light highlight around the colour
        let k = if t < 0.6 { -0.62 * (1.0 - t / 0.6) } else { (t - 0.6) / 0.4 * 0.32 };
        let col = leaf_hue(m, shade(c, k), k);
        p[(m.base + i) as usize] = [col[0] as u8, col[1] as u8, col[2] as u8];
    }
}

/// Shades turn cool and highlights warm, as in sunlight, foliage most of all (the same as tools/freeset_art/mats.py).
fn leaf_hue(m: Mat, c: Col, k: f32) -> Col {
    if m.base != LEAF.base {
        // every material: a warm sunlit highlight and a slightly cool shade
        let (r, b) = if k < 0.0 { (0.0, -k * 8.0) } else { (k * 14.0, k * -16.0) };
        return [(c[0] + r).clamp(0.0, 255.0), c[1], (c[2] + b).clamp(0.0, 255.0), c[3]];
    }
    let (r, b) = if k < 0.0 { (-k * -10.0, -k * 22.0) } else { (k * 40.0, k * -40.0) };
    // and a little richer than the colour itself
    let g = (c[0] + c[1] + c[2]) / 3.0;
    let s = |v: f32| g + (v - g) * 1.1;
    [(s(c[0]) + r).clamp(0.0, 255.0), s(c[1]).clamp(0.0, 255.0), (s(c[2]) + b).clamp(0.0, 255.0), c[3]]
}

/// The free palette, with the golfer slots in the colours of swap `swap` and the roof, wall and leaves turned by `variant`.
pub fn palette(swap: usize, female: bool, variant: u32) -> [[u8; 3]; 256] {
    let mut p = [[0u8; 3]; 256];
    let pc = person_colours(swap, female);
    put_ramp(&mut p, SHIRT, pc[0]);
    put_ramp(&mut p, PANTS, pc[1]);
    put_ramp(&mut p, SKIN, pc[2]);
    put_ramp(&mut p, HAT, pc[3]);
    put_ramp(&mut p, ALT, pc[4]);
    put_ramp(&mut p, HANDS_F, pc[5]);
    let v = variant as usize;
    const LEAVES: [Col; 6] =
        [rgb(70, 140, 55), rgb(60, 120, 70), rgb(110, 150, 50), rgb(170, 120, 60), rgb(150, 70, 90), rgb(200, 200, 120)];
    const ROOFS: [Col; 6] = [rgb(170, 70, 55), rgb(70, 90, 130), rgb(90, 110, 80), rgb(120, 100, 85), rgb(185, 120, 70), rgb(80, 80, 90)];
    const WALLS: [Col; 6] =
        [rgb(235, 225, 200), rgb(220, 205, 170), rgb(240, 240, 235), rgb(215, 185, 150), rgb(200, 210, 220), rgb(225, 200, 190)];
    put_ramp(&mut p, LEAF, LEAVES[v % 6]);
    put_ramp(&mut p, BARK, rgb(115, 80, 50));
    put_ramp(&mut p, WALL, WALLS[(v / 6) % 6]);
    put_ramp(&mut p, ROOF, ROOFS[(v / 2) % 6]);
    put_ramp(&mut p, STONE, rgb(150, 148, 140));
    put_ramp(&mut p, WATER, rgb(70, 130, 190));
    put_ramp(&mut p, EARTH, rgb(150, 115, 75));
    put_ramp(&mut p, DARK, rgb(60, 70, 85));
    put_ramp(&mut p, WHITE, rgb(240, 240, 235));
    put_ramp(&mut p, RED, [rgb(210, 50, 55), rgb(240, 110, 160), rgb(250, 140, 40)][v % 3]);
    put_ramp(&mut p, YELLOW, [rgb(245, 210, 60), rgb(250, 240, 200), rgb(250, 180, 60)][v % 3]);
    put_ramp(&mut p, PURPLE, [rgb(140, 90, 200), rgb(80, 120, 220), rgb(220, 120, 220)][v % 3]);
    put_ramp(&mut p, WOOD, rgb(160, 115, 70));
    put_ramp(&mut p, GOLD, rgb(220, 180, 70));
    put_ramp(&mut p, SHOE, rgb(70, 60, 55));
    p[KEY as usize] = [255, 0, 255];
    p
}

/// Shadows: three darknesses (by the red channel, crate::sprites) and the key.
pub(crate) fn shadow_palette() -> [[u8; 3]; 256] {
    let mut p = [[255u8, 255, 255]; 256];
    p[1] = [170, 230, 170];
    p[2] = [90, 200, 90];
    p[3] = [0, 160, 0];
    p[KEY as usize] = [255, 0, 255];
    p
}

/// The leaf colour of a tree palette file by its name (lower case): the colour variants the game picks for its trees.
fn tree_leaf(name: &str) -> Option<Col> {
    let n = name.trim_end_matches(".pcx");
    let k = n.chars().rev().find(|c| c.is_ascii_digit()).and_then(|c| c.to_digit(10)).unwrap_or(1).max(1) as usize - 1;
    let pick = |v: [Col; 4]| Some(v[k.min(3)]);
    const TABLE: [(&str, Col); 31] = [
        ("greenmaple", rgb(88, 146, 56)),
        ("redmaple", rgb(170, 78, 52)),
        ("brownmaple", rgb(156, 142, 56)),
        ("bluemaple", rgb(66, 128, 94)),
        ("pineblue", rgb(64, 114, 112)),
        ("pinedarkgreen", rgb(48, 98, 60)),
        ("pineoriginal", rgb(66, 118, 62)),
        ("pineyellow", rgb(116, 140, 60)),
        ("greenpalm", rgb(70, 142, 56)),
        ("bluepalm", rgb(62, 126, 104)),
        ("orangepalm", rgb(150, 142, 62)),
        ("originalpalm", rgb(76, 140, 58)),
        ("plumpalm", rgb(112, 122, 62)),
        ("tallpalmdesert", rgb(108, 134, 70)),
        ("tallpalm", rgb(78, 138, 60)),
        ("cactusapal", rgb(86, 128, 70)),
        ("cactusbpal", rgb(92, 134, 74)),
        ("cactuscpal", rgb(104, 146, 82)),
        ("greenercactus", rgb(70, 138, 66)),
        ("orangecactus", rgb(146, 132, 70)),
        ("originalcactus", rgb(86, 128, 70)),
        ("yellercactus", rgb(132, 142, 70)),
        ("joshua", rgb(96, 120, 70)),
        ("palcerc", rgb(190, 104, 160)),
        ("paldrac", rgb(70, 120, 60)),
        ("willowgreen", rgb(118, 156, 74)),
        ("willowlemon", rgb(170, 178, 82)),
        ("willowpink", rgb(204, 140, 172)),
        ("willowwhite", rgb(222, 222, 204)),
        ("tropbrush", rgb(56, 128, 56)),
        ("scotspine", rgb(66, 116, 66)),
    ];
    const SCENIC: [(&str, Col); 22] = [
        ("aspenpal", rgb(196, 170, 64)),
        ("blackpinepal", rgb(62, 106, 66)),
        ("bougainvilleapal", rgb(64, 120, 52)),
        ("cypresspal", rgb(58, 104, 64)),
        ("dogwoodpal", rgb(80, 130, 60)),
        ("japanesemaple_greenpal", rgb(100, 150, 60)),
        ("japanesemaple_orgpal", rgb(220, 120, 50)),
        ("japanesemaple_redpal", rgb(190, 56, 48)),
        ("japanesemaple_yelpal", rgb(220, 190, 70)),
        ("japanesetreepal", rgb(56, 100, 60)),
        ("mangrovepal", rgb(66, 110, 60)),
        ("mesquitepal", rgb(112, 130, 72)),
        ("peachtreepal", rgb(90, 136, 60)),
        ("plumpal", rgb(120, 70, 96)),
        ("rhodypal", rgb(50, 100, 50)),
        ("scenic_treepal", rgb(78, 130, 56)),
        ("scenicelmpal", rgb(70, 124, 54)),
        ("heatherpal", rgb(146, 92, 146)),
        ("scenic_ograsspal", rgb(150, 166, 86)),
        ("cattailspal", rgb(110, 140, 70)),
        ("bamboopal", rgb(120, 164, 72)),
        ("vineyardpal", rgb(84, 130, 56)),
    ];
    if let Some((_, c)) = SCENIC.iter().find(|(k, _)| n.starts_with(k)) {
        return Some(*c);
    }
    if n.starts_with("cactusgroup") || n.starts_with("oasispal") {
        return Some(rgb(88, 130, 72));
    }
    if n.starts_with("tropbrushpal") {
        return pick([rgb(56, 128, 56), rgb(82, 142, 48), rgb(46, 112, 74), rgb(56, 128, 56)]);
    }
    if n.starts_with("linkspinepal") {
        return pick([rgb(56, 102, 56), rgb(46, 92, 64), rgb(72, 106, 56), rgb(60, 96, 72)]);
    }
    if n.starts_with("linkstree3pal") {
        return pick([rgb(96, 132, 62), rgb(122, 132, 58), rgb(154, 112, 52), rgb(86, 124, 70)]);
    }
    if n.starts_with("linkstree4pal") {
        return pick([rgb(74, 120, 60), rgb(100, 130, 56), rgb(140, 100, 50), rgb(66, 110, 72)]);
    }
    if n.starts_with("scotspinepal") {
        return pick([rgb(58, 104, 62), rgb(50, 94, 70), rgb(74, 108, 58), rgb(62, 98, 76)]);
    }
    TABLE.iter().find(|(k, _)| n.contains(k)).map(|(_, c)| *c)
}

/// The uniform of a member of staff by their palette file's name (lower case): shirt, trousers or skirt, skin, hat, the
/// men's hands or the women's hair, the women's hands.
fn staff_colours(name: &str) -> Option<[Col; 6]> {
    let skin = rgb(232, 190, 150);
    let n = name.trim_end_matches(".pcx");
    Some(match n {
        "gkpal" => [rgb(70, 124, 62), rgb(170, 150, 108), skin, rgb(70, 124, 62), skin, skin],
        "golfcelebpal" => [rgb(242, 204, 66), rgb(60, 62, 72), skin, rgb(244, 244, 240), skin, skin],
        "greeterpal" => [rgb(170, 44, 54), rgb(48, 50, 62), skin, rgb(170, 44, 54), skin, skin],
        "lawntechpal" => [rgb(232, 124, 44), rgb(78, 92, 116), skin, rgb(232, 124, 44), skin, skin],
        "marshallpal" => [rgb(42, 62, 112), rgb(186, 176, 146), skin, rgb(206, 186, 126), skin, skin],
        "rangerpal" => [rgb(150, 142, 92), rgb(112, 100, 72), skin, rgb(124, 100, 62), skin, skin],
        "sodavendorpal" => [rgb(244, 244, 240), rgb(200, 52, 52), skin, rgb(200, 52, 52), skin, skin],
        "traygirlpal" => [rgb(240, 150, 182), rgb(246, 246, 242), skin, rgb(240, 150, 182), rgb(110, 70, 40), skin],
        "bev_walkpal" | "bev_incartpal" => [rgb(80, 170, 170), rgb(246, 246, 242), skin, rgb(246, 246, 242), rgb(222, 186, 110), skin],
        _ => return None,
    })
}

/// The flower colour of a flowering scenic plant's palette file (lower case name).
fn scenic_flower(name: &str) -> Option<Col> {
    const F: [(&str, Col); 5] = [
        ("bougainvilleapal", rgb(214, 60, 150)),
        ("dogwoodpal", rgb(240, 236, 226)),
        ("peachtreepal", rgb(244, 166, 186)),
        ("plumpal", rgb(226, 150, 206)),
        ("rhodypal", rgb(222, 92, 170)),
    ];
    F.iter().find(|(k, _)| name.starts_with(k)).map(|(_, c)| *c)
}

/// The colours of a landmark's or a prop's palette file (lower case name) by material: they are rendered offline in these
/// ramps (tools/freeset_art/landmarks.py and props.py give the same colours).
fn landmark_colours(name: &str) -> Option<&'static [(Mat, Col)]> {
    const T: &[(&str, &[(Mat, Col)])] = &[
        ("flag_parkpal", &[(RED, rgb(226, 44, 44)), (WHITE, rgb(244, 244, 240)), (YELLOW, rgb(240, 200, 60)), (DARK, rgb(30, 30, 30))]),
        ("flag_desertpal", &[(RED, rgb(244, 140, 36)), (WHITE, rgb(244, 244, 240)), (YELLOW, rgb(240, 200, 60)), (DARK, rgb(30, 30, 30))]),
        ("flag_troppal", &[(RED, rgb(236, 70, 150)), (WHITE, rgb(244, 244, 240)), (YELLOW, rgb(240, 200, 60)), (DARK, rgb(30, 30, 30))]),
        ("flag_linkspal", &[(RED, rgb(244, 206, 48)), (WHITE, rgb(244, 244, 240)), (YELLOW, rgb(240, 200, 60)), (DARK, rgb(30, 30, 30))]),
        ("teemarkerpal", &[(RED, rgb(220, 40, 40)), (PURPLE, rgb(50, 90, 220)), (WHITE, rgb(244, 244, 240)), (DARK, rgb(70, 60, 50))]),
        (
            "benwpal",
            &[
                (WOOD, rgb(156, 110, 66)),
                (DARK, rgb(54, 58, 62)),
                (RED, rgb(196, 54, 46)),
                (WHITE, rgb(240, 236, 226)),
                (LEAF, rgb(80, 140, 60)),
            ],
        ),
        (
            "box benchpal",
            &[
                (WOOD, rgb(156, 110, 66)),
                (DARK, rgb(54, 58, 62)),
                (RED, rgb(196, 54, 46)),
                (WHITE, rgb(240, 236, 226)),
                (LEAF, rgb(80, 140, 60)),
            ],
        ),
        (
            "redbenchpal",
            &[
                (WOOD, rgb(156, 110, 66)),
                (DARK, rgb(54, 58, 62)),
                (RED, rgb(196, 54, 46)),
                (WHITE, rgb(240, 236, 226)),
                (LEAF, rgb(80, 140, 60)),
            ],
        ),
        (
            "round wood benchpal",
            &[
                (WOOD, rgb(156, 110, 66)),
                (DARK, rgb(54, 58, 62)),
                (RED, rgb(196, 54, 46)),
                (WHITE, rgb(240, 236, 226)),
                (LEAF, rgb(80, 140, 60)),
            ],
        ),
        (
            "backless benchpal",
            &[
                (WOOD, rgb(156, 110, 66)),
                (DARK, rgb(54, 58, 62)),
                (RED, rgb(196, 54, 46)),
                (WHITE, rgb(240, 236, 226)),
                (LEAF, rgb(80, 140, 60)),
            ],
        ),
        (
            "loversbenchpal",
            &[
                (WOOD, rgb(156, 110, 66)),
                (DARK, rgb(54, 58, 62)),
                (RED, rgb(196, 54, 46)),
                (WHITE, rgb(240, 236, 226)),
                (LEAF, rgb(80, 140, 60)),
            ],
        ),
        (
            "swingpal",
            &[
                (WOOD, rgb(156, 110, 66)),
                (DARK, rgb(54, 58, 62)),
                (RED, rgb(196, 54, 46)),
                (WHITE, rgb(240, 236, 226)),
                (LEAF, rgb(80, 140, 60)),
            ],
        ),
        (
            "upgrade bwasherpal",
            &[
                (STONE, rgb(170, 164, 150)),
                (DARK, rgb(60, 66, 70)),
                (RED, rgb(40, 110, 70)),
                (WHITE, rgb(244, 244, 240)),
                (LEAF, rgb(60, 110, 70)),
            ],
        ),
        (
            "parkbridgepal",
            &[
                (WOOD, rgb(150, 108, 70)),
                (WHITE, rgb(240, 236, 226)),
                (EARTH, rgb(186, 136, 92)),
                (YELLOW, rgb(220, 190, 120)),
                (STONE, rgb(160, 154, 142)),
            ],
        ),
        (
            "desbridgepal",
            &[
                (WOOD, rgb(150, 108, 70)),
                (WHITE, rgb(240, 236, 226)),
                (EARTH, rgb(186, 136, 92)),
                (YELLOW, rgb(220, 190, 120)),
                (STONE, rgb(160, 154, 142)),
            ],
        ),
        (
            "tropbridgepal",
            &[
                (WOOD, rgb(150, 108, 70)),
                (WHITE, rgb(240, 236, 226)),
                (EARTH, rgb(186, 136, 92)),
                (YELLOW, rgb(220, 190, 120)),
                (STONE, rgb(160, 154, 142)),
            ],
        ),
        (
            "linksbridgepalette",
            &[
                (WOOD, rgb(150, 108, 70)),
                (WHITE, rgb(240, 236, 226)),
                (EARTH, rgb(186, 136, 92)),
                (YELLOW, rgb(220, 190, 120)),
                (STONE, rgb(160, 154, 142)),
            ],
        ),
        ("scenicgenpal", &[(WOOD, rgb(150, 108, 70)), (STONE, rgb(168, 160, 146)), (RED, rgb(178, 64, 50))]),
        ("sail_movepal", &[(WHITE, rgb(246, 246, 240)), (WOOD, rgb(170, 124, 80)), (RED, rgb(210, 50, 50))]),
        (
            "iron benchpal",
            &[
                (WOOD, rgb(156, 110, 66)),
                (DARK, rgb(54, 58, 62)),
                (RED, rgb(196, 54, 46)),
                (WHITE, rgb(240, 236, 226)),
                (LEAF, rgb(80, 140, 60)),
            ],
        ),
        (
            "rainbow sail pal",
            &[
                (WHITE, rgb(246, 246, 240)),
                (DARK, rgb(60, 60, 60)),
                (RED, rgb(226, 50, 50)),
                (YELLOW, rgb(250, 210, 50)),
                (LEAF, rgb(60, 170, 80)),
                (PURPLE, rgb(80, 90, 220)),
                (SHOE, rgb(230, 180, 140)),
            ],
        ),
        ("sundialpal", &[(STONE, rgb(190, 184, 170)), (GOLD, rgb(196, 150, 70))]),
        (
            "barnpal",
            &[
                (WALL, rgb(176, 58, 46)),
                (ROOF, rgb(92, 94, 100)),
                (WHITE, rgb(238, 234, 222)),
                (DARK, rgb(60, 54, 50)),
                (YELLOW, rgb(220, 190, 100)),
            ],
        ),
        ("civilwarcannonpal", &[(WOOD, rgb(120, 84, 52)), (DARK, rgb(52, 52, 56)), (GOLD, rgb(150, 112, 64))]),
        ("stonetwopal", &[(STONE, rgb(150, 146, 136)), (LEAF, rgb(96, 140, 66))]),
        (
            "wmillapal",
            &[
                (WOOD, rgb(150, 120, 84)),
                (DARK, rgb(70, 66, 64)),
                (WHITE, rgb(232, 230, 220)),
                (RED, rgb(190, 60, 50)),
                (WATER, rgb(70, 130, 170)),
            ],
        ),
        (
            "parklandrockpal",
            &[
                (STONE, rgb(142, 138, 128)),
                (LEAF, rgb(90, 130, 60)),
                (EARTH, rgb(150, 110, 70)),
                (DARK, rgb(40, 32, 28)),
                (WHITE, rgb(244, 240, 230)),
            ],
        ),
        ("civilwarstatuepal", &[(STONE, rgb(184, 178, 166)), (GOLD, rgb(110, 128, 104))]),
        (
            "lighthousecpal",
            &[
                (WHITE, rgb(240, 238, 230)),
                (RED, rgb(196, 52, 48)),
                (DARK, rgb(54, 60, 70)),
                (YELLOW, rgb(255, 226, 120)),
                (STONE, rgb(150, 146, 138)),
                (ROOF, rgb(70, 74, 84)),
            ],
        ),
        ("buddhapal", &[(STONE, rgb(170, 162, 146)), (GOLD, rgb(214, 168, 70)), (RED, rgb(176, 64, 60))]),
        (
            "windmillpal",
            &[
                (STONE, rgb(150, 84, 64)),
                (WALL, rgb(226, 214, 190)),
                (WOOD, rgb(110, 80, 56)),
                (ROOF, rgb(96, 92, 88)),
                (WHITE, rgb(240, 234, 220)),
                (DARK, rgb(60, 60, 66)),
            ],
        ),
        ("equestrianpal", &[(STONE, rgb(186, 180, 170)), (GOLD, rgb(104, 122, 100))]),
        ("easterpal", &[(STONE, rgb(132, 126, 116)), (LEAF, rgb(96, 140, 66)), (RED, rgb(150, 76, 58)), (DARK, rgb(60, 56, 52))]),
        (
            "pagodapal",
            &[
                (STONE, rgb(170, 162, 150)),
                (WALL, rgb(178, 56, 46)),
                (ROOF, rgb(54, 92, 84)),
                (GOLD, rgb(212, 168, 70)),
                (DARK, rgb(60, 40, 36)),
            ],
        ),
        (
            "lighthousebpal",
            &[
                (WHITE, rgb(240, 238, 230)),
                (RED, rgb(196, 52, 48)),
                (DARK, rgb(54, 60, 70)),
                (YELLOW, rgb(255, 226, 120)),
                (STONE, rgb(150, 146, 138)),
                (ROOF, rgb(70, 74, 84)),
            ],
        ),
        (
            "chinesehousepal",
            &[
                (STONE, rgb(184, 176, 160)),
                (WALL, rgb(226, 210, 180)),
                (RED, rgb(180, 48, 40)),
                (ROOF, rgb(64, 78, 92)),
                (GOLD, rgb(220, 176, 70)),
                (WOOD, rgb(120, 70, 50)),
                (YELLOW, rgb(250, 210, 120)),
            ],
        ),
        (
            "tarpitpal",
            &[
                (DARK, rgb(34, 32, 36)),
                (EARTH, rgb(118, 92, 62)),
                (WHITE, rgb(232, 222, 196)),
                (STONE, rgb(138, 130, 118)),
                (LEAF, rgb(110, 120, 60)),
            ],
        ),
        ("wtow2pal", &[(STONE, rgb(150, 150, 152)), (WALL, rgb(226, 220, 206)), (ROOF, rgb(170, 70, 56)), (DARK, rgb(70, 72, 78))]),
        (
            "radio towerpal",
            &[
                (RED, rgb(200, 56, 48)),
                (WHITE, rgb(236, 236, 232)),
                (STONE, rgb(150, 148, 142)),
                (WALL, rgb(196, 190, 176)),
                (DARK, rgb(70, 72, 78)),
            ],
        ),
        ("red oil pump pal", &[(STONE, rgb(150, 146, 138)), (DARK, rgb(60, 62, 68)), (RED, rgb(196, 60, 46)), (WHITE, rgb(220, 220, 220))]),
    ];
    let n = name.trim_end_matches(".pcx");
    T.iter().find(|(k, _)| *k == n).map(|(_, c)| *c)
}

/// A flower bed's palette (lower case name): its blooms (the RED ramp, tools/freeset_art/props.py) in the colour the
/// name gives, on green leaves, dark mulch and a stone kerb.
fn flower_bed_colours(name: &str) -> Option<[(Mat, Col); 5]> {
    if !(name.contains("flowerbed") || name.contains("desertflowers") || name.contains("tropicalflowers") || name.contains("tropfb")) {
        return None;
    }
    const BLOOMS: [(&str, Col); 10] = [
        ("yel", rgb(250, 214, 56)),
        ("org", rgb(248, 140, 40)),
        ("purp", rgb(156, 84, 214)),
        ("white", rgb(248, 246, 238)),
        ("pink", rgb(244, 120, 172)),
        ("blue", rgb(76, 116, 232)),
        ("aqua", rgb(64, 196, 206)),
        ("green", rgb(170, 220, 80)),
        ("icky", rgb(150, 136, 90)),
        ("red", rgb(228, 44, 64)),
    ];
    let bloom = BLOOMS.iter().find(|(k, _)| name.contains(k)).map(|(_, c)| *c).unwrap_or(rgb(228, 44, 64));
    Some([(RED, bloom), (LEAF, rgb(70, 140, 56)), (EARTH, rgb(110, 76, 52)), (STONE, rgb(176, 168, 152)), (WHITE, rgb(246, 244, 236))])
}

/// A palette file (a PCX the engine reads for its palette): golfer swap palettes by their number, the others turn the
/// roof, wall and leaf colours by their name.
pub fn palette_file(low: &str, w: u32, h: u32) -> Vec<u8> {
    let name = low.rsplit('/').next().unwrap_or(low);
    let digits: String = name.chars().filter(|c| c.is_ascii_digit()).collect();
    let female = low.contains("female");
    let pal = if name.contains("swap") || name.contains("baspal") || name.contains("basepal") {
        let n = digits.parse::<usize>().unwrap_or(1).max(1) - 1;
        palette(n, female, 0)
    } else if let Some(c) = staff_colours(name) {
        // the staff's uniforms (their sprites are drawn in the golfer slots, baked.rs)
        let mut p = palette(0, false, 0);
        for (m, col) in [SHIRT, PANTS, SKIN, HAT, ALT, HANDS_F].into_iter().zip(c) {
            put_ramp(&mut p, m, col);
        }
        p
    } else if let Some(cols) = flower_bed_colours(name) {
        let mut p = palette(0, false, 0);
        for (m, c) in cols {
            put_ramp(&mut p, m, c);
        }
        p
    } else if let Some(cols) = landmark_colours(name) {
        let mut p = palette(0, false, 0);
        for &(m, c) in cols {
            put_ramp(&mut p, m, c);
        }
        p
    } else if let Some(leaf) = tree_leaf(name) {
        // the colour variants of the trees (the baked trees keep their leaves in the LEAF ramp, baked.rs), and the
        // flowers of the flowering scenic plants in the RED ramp
        let mut p = palette(0, false, 0);
        put_ramp(&mut p, LEAF, leaf);
        if let Some(f) = scenic_flower(name) {
            put_ramp(&mut p, RED, f);
        }
        if name.contains("palm") {
            put_ramp(&mut p, BARK, rgb(150, 120, 86));
        }
        p
    } else {
        let mut v = hash(name) % 36;
        // the flower and willow colour palettes by their colour name
        for (k, word) in ["green", "lemon", "yellow", "orange", "pink", "red", "purple", "blue", "white"].iter().enumerate() {
            if name.contains(word) {
                v = k as u32;
            }
        }
        palette(0, false, v)
    };
    // a swatch of the colours as the picture
    let (w, h) = (w.max(1), h.max(1));
    let idx = (0..(w * h)).map(|i| (((i % w) * 16 / w) + ((i / w) * 16 / h) * 16) as u8).collect();
    pcx_indexed(w, h, idx, &pal)
}

// ---- an indexed canvas ------------------------------------------------------------------------------------------------------

pub struct Ix {
    pub w: i32,
    pub h: i32,
    pub px: Vec<u8>,
}

/// The light, in screen terms: from the upper left, towards the viewer.
const LIGHT: (f32, f32, f32) = (-0.48, -0.62, 0.62);

impl Ix {
    fn new(w: u32, h: u32) -> Ix {
        Ix { w: w as i32, h: h as i32, px: vec![KEY; (w * h) as usize] }
    }
    fn set(&mut self, x: i32, y: i32, c: u8) {
        if x >= 0 && y >= 0 && x < self.w && y < self.h {
            self.px[(y * self.w + x) as usize] = c;
        }
    }
    /// Fills where `inside` holds (pixel centres), colouring with `col`.
    fn fill(&mut self, b: (f32, f32, f32, f32), inside: impl Fn(f32, f32) -> bool, col: impl Fn(f32, f32) -> u8) {
        let (x0, y0, x1, y1) = b;
        for y in (y0.floor() as i32).max(0)..=(y1.ceil() as i32).min(self.h - 1) {
            for x in (x0.floor() as i32).max(0)..=(x1.ceil() as i32).min(self.w - 1) {
                let (px, py) = (x as f32 + 0.5, y as f32 + 0.5);
                if inside(px, py) {
                    self.set(x, y, col(px, py));
                }
            }
        }
    }
    /// A shaded ball (or ellipse) of material m.
    fn ball(&mut self, cx: f32, cy: f32, rx: f32, ry: f32, m: Mat, lift: f32) {
        let (rx, ry) = (rx.max(0.6), ry.max(0.6));
        self.fill(
            (cx - rx, cy - ry, cx + rx, cy + ry),
            |x, y| ((x - cx) / rx).powi(2) + ((y - cy) / ry).powi(2) <= 1.0,
            |x, y| {
                let (nx, ny) = ((x - cx) / rx, (y - cy) / ry);
                let nz = (1.0 - nx * nx - ny * ny).max(0.0).sqrt();
                let l = nx * LIGHT.0 + ny * LIGHT.1 + nz * LIGHT.2;
                m.at(0.3 + 0.6 * l.max(0.0) + lift)
            },
        );
    }
    /// A shaded rod from a to b of radius r (round ends).
    fn rod(&mut self, a: (f32, f32), b: (f32, f32), r: f32, m: Mat, lift: f32) {
        let r = r.max(0.5);
        let (dx, dy) = (b.0 - a.0, b.1 - a.1);
        let len2 = (dx * dx + dy * dy).max(1e-4);
        let (nx, ny) = (-dy / len2.sqrt(), dx / len2.sqrt());
        self.fill(
            (a.0.min(b.0) - r, a.1.min(b.1) - r, a.0.max(b.0) + r, a.1.max(b.1) + r),
            |x, y| {
                let t = (((x - a.0) * dx + (y - a.1) * dy) / len2).clamp(0.0, 1.0);
                (x - a.0 - dx * t).powi(2) + (y - a.1 - dy * t).powi(2) <= r * r
            },
            |x, y| {
                let side = ((x - a.0) * nx + (y - a.1) * ny) / r;
                let l = side * (nx * LIGHT.0 + ny * LIGHT.1) + (1.0 - side * side).max(0.0).sqrt() * LIGHT.2;
                m.at(0.3 + 0.6 * l.max(0.0) + lift)
            },
        );
    }
    /// A flat polygon in one entry.
    fn poly(&mut self, pts: &[(f32, f32)], c: u8) {
        let (mut x0, mut y0, mut x1, mut y1) = (f32::MAX, f32::MAX, f32::MIN, f32::MIN);
        for &(x, y) in pts {
            x0 = x0.min(x);
            y0 = y0.min(y);
            x1 = x1.max(x);
            y1 = y1.max(y);
        }
        self.fill((x0, y0, x1, y1), |x, y| inside_poly(x, y, pts), |_, _| c);
    }
    fn line(&mut self, a: (f32, f32), b: (f32, f32), c: u8) {
        let n = ((b.0 - a.0).abs().max((b.1 - a.1).abs()).ceil() as i32).max(1);
        for k in 0..=n {
            let t = k as f32 / n as f32;
            self.set((a.0 + (b.0 - a.0) * t).floor() as i32, (a.1 + (b.1 - a.1) * t).floor() as i32, c);
        }
    }
}

fn inside_poly(px: f32, py: f32, pts: &[(f32, f32)]) -> bool {
    let mut inside = false;
    let n = pts.len();
    for i in 0..n {
        let (a, b) = (pts[i], pts[(i + 1) % n]);
        if (a.1 > py) != (b.1 > py) && px < (b.0 - a.0) * (py - a.1) / (b.1 - a.1) + a.0 {
            inside = !inside;
        }
    }
    inside
}

// ---- projection ---------------------------------------------------------------------------------------------------------------

/// How much the ground is foreshortened (sine of the camera's elevation) and heights (its cosine).
const SINP: f32 = 0.62;
const COSP: f32 = 0.78;

/// A view of the scene: the ground point in the frame and the turn of the object.
#[derive(Clone, Copy)]
struct View {
    ax: f32,
    ay: f32,
    yaw: f32,
}

impl View {
    /// World point (x right, y up, z towards the viewer, in sprite pixels) after the object's turn, on the frame.
    fn p(&self, x: f32, y: f32, z: f32) -> (f32, f32) {
        let (s, c) = self.yaw.sin_cos();
        let (xr, zr) = (x * c - z * s, x * s + z * c);
        (self.ax + xr, self.ay + zr * SINP - y * COSP)
    }
    /// The depth of a point after the turn (larger is nearer the viewer).
    fn depth(&self, x: f32, z: f32) -> f32 {
        let (s, c) = self.yaw.sin_cos();
        x * s + z * c
    }
}

// ---- the sprite files -------------------------------------------------------------------------------------------------------------

/// What one frame is asked to show.
struct Ctx<'a> {
    name: &'a str,
    low: &'a str,
    view: View,
    v: u32,
    views: u32,
    f: u32,
    fpv: u32,
    w: f32,
    h: f32,
    seed: u32,
    /// People: a scale on the figure, and no head (the body stills the customise screens put a head on).
    scale: f32,
    headless: bool,
}

impl Ctx<'_> {
    /// The animation's progress, 0..1.
    fn t(&self) -> f32 {
        if self.fpv <= 1 {
            0.0
        } else {
            self.f as f32 / self.fpv as f32
        }
    }
    fn has(&self, k: &str) -> bool {
        self.name.contains(k)
    }
}

pub fn flc(low: &str, head: &FlcHead) -> Vec<u8> {
    let shadow = low.ends_with("shadow.flc");
    let name = low.rsplit('/').next().unwrap_or(low).trim_end_matches(".flc").to_string();
    let views = head.views;
    let fpv = head.fpv.max(1);
    let (ax, ay) =
        if views > 0 { (240.0 - head.crop.0 as f32, 240.0 - head.crop.1 as f32) } else { (head.w as f32 / 2.0, head.h as f32 - 2.0) };
    let mut frames = Vec::new();
    let nv = views.max(1);
    let seed = hash(low);
    for v in 0..nv {
        for f in 0..fpv {
            let mut ix = Ix::new(head.w, head.h);
            let step = if nv >= 8 { PI / 4.0 } else { PI / 2.0 };
            let ctx = Ctx {
                name: &name,
                low,
                view: View { ax, ay, yaw: v as f32 * step },
                v,
                views: nv,
                f,
                fpv,
                w: head.w as f32,
                h: head.h as f32,
                seed,
                scale: 1.0,
                headless: false,
            };
            if shadow {
                draw_shadow(&mut ix, &ctx);
            } else {
                draw(&mut ix, &ctx);
            }
            frames.push(ix.px);
        }
    }
    let pal = if shadow { shadow_palette() } else { palette(0, low.contains("female"), seed % 36) };
    write_flc(head, &pal, &frames)
}

fn draw_shadow(ix: &mut Ix, c: &Ctx) {
    // a soft oval on the ground, cast away from the light (down and right on the screen), sized by the object
    let lowp = c.low;
    if lowp.contains("anim") || lowp.contains("flag") || lowp.contains("reflect") || lowp.contains("ripple") {
        return;
    }
    let people = ["/male/", "/female/", "/celebs/", "/employee/", "/animals/"].iter().any(|d| lowp.contains(d));
    let (ax, ay) = (c.view.ax, c.view.ay);
    let (rx, ry) = if people {
        (5.0f32.min(c.w / 3.0), 2.2)
    } else {
        let r = (ax.min(c.w - ax) * 0.85).max(2.0);
        (r, r * 0.45)
    };
    let (cx, cy) = (ax + rx * 0.35, ay + ry * 0.2);
    ix.fill(
        (cx - rx, cy - ry, cx + rx, cy + ry),
        |x, y| ((x - cx) / rx).powi(2) + ((y - cy) / ry).powi(2) <= 1.0,
        |x, y| {
            let d = ((x - cx) / rx).powi(2) + ((y - cy) / ry).powi(2);
            if d < 0.35 {
                3
            } else if d < 0.7 {
                2
            } else {
                1
            }
        },
    );
}

fn draw(ix: &mut Ix, c: &Ctx) {
    let l = c.low;
    let in_dir = |d: &str| l.contains(d);
    if in_dir("flics/male/") || in_dir("flics/female/") || in_dir("flics/celebs/") || in_dir("flics/employee/") {
        person(ix, c);
    } else if in_dir("flics/animals/") || ["eagle", "hawk", "pelican", "vulture", "ghog", "dolphin", "grouse"].iter().any(|k| c.has(k)) {
        animal(ix, c);
    } else if in_dir("flics/trees/") || is_tree_name(c.name) {
        tree(ix, c);
    } else if c.has("waterfall") {
        waterfall(ix, c);
    } else if c.has("anim")
        || c.has("status")
        || c.has("elev")
        || c.has("wheel")
        || c.has("lites")
        || c.has("underwater")
        || c.has("reflect")
        || c.has("ripple")
        || c.has("wave")
        || c.has("spray")
        || c.has("splash")
        || c.has("fountain")
    {
        water_fx(ix, c);
    } else if c.has("flag") || in_dir("flics/tees/") {
        flag(ix, c);
    } else if in_dir("flics/flowers/") {
        flowers(ix, c);
    } else if in_dir("flics/bridges/") || c.has("bridge") {
        bridge(ix, c);
    } else if in_dir("flics/homes/cars") || c.has("car") && !c.has("cart") {
        car(ix, c);
    } else if c.has("base") || c.has("dirt") || c.has("plat") {
        slab(ix, c);
    } else if c.has("sign")
        || c.has("00dr")
        || c.has("drivingr") && c.name.contains("00")
        || c.has("_100")
        || c.has("_200")
        || c.has("_300")
        || c.has("_400")
    {
        sign(ix, c);
    } else if c.has("wall") || c.has("fence") || c.has("rockwall") {
        wall(ix, c);
    } else if in_dir("flics/water/") && (c.has("coral") || c.has("sand") || c.name.starts_with("uw")) {
        water_piece(ix, c);
    } else if in_dir("flics/landmarks/") || in_dir("flics/scenic/") || in_dir("flics/water/") {
        landmark(ix, c);
    } else {
        building(ix, c);
    }
}

fn is_tree_name(n: &str) -> bool {
    [
        "tree",
        "pine",
        "palm",
        "cactus",
        "joshua",
        "maple",
        "elm",
        "aspen",
        "cypress",
        "dogwood",
        "plum",
        "peach",
        "mangrove",
        "mesquite",
        "bamboo",
        "bougainvillea",
        "rhody",
        "willow",
        "oak",
        "cerc",
        "drac",
        "birch",
    ]
    .iter()
    .any(|k| n.contains(k))
}

// ---- people ------------------------------------------------------------------------------------------------------------------------

/// A posed figure: joint positions in the figure's own space (x forwards, y up, z to its left), in pixels.
#[derive(Clone, Copy, Default)]
struct Pose {
    hip: (f32, f32, f32),
    chest: (f32, f32, f32),
    head: (f32, f32, f32),
    knee: [(f32, f32, f32); 2],
    foot: [(f32, f32, f32); 2],
    elbow: [(f32, f32, f32); 2],
    hand: [(f32, f32, f32); 2],
    /// The club's head, when one is held (from the hands).
    club: Option<(f32, f32, f32)>,
}

/// The pose for animation `name` at progress t (0..1), for a figure `tall` pixels high.
fn pose(name: &str, t: f32, tall: f32) -> Pose {
    let k = tall / 26.0;
    let s = |v: f32| v * k;
    let has = |w: &str| name.contains(w);
    let cyc = (t * 2.0 * PI).sin();
    let mut p = Pose {
        hip: (0.0, s(12.5), 0.0),
        chest: (0.0, s(19.5), 0.0),
        head: (0.0, s(23.3), 0.0),
        knee: [(0.0, s(6.5), s(1.6)), (0.0, s(6.5), s(-1.6))],
        foot: [(0.0, 0.0, s(1.7)), (0.0, 0.0, s(-1.7))],
        elbow: [(0.0, s(15.5), s(3.6)), (0.0, s(15.5), s(-3.6))],
        hand: [(s(0.5), s(11.5), s(3.6)), (s(0.5), s(11.5), s(-3.6))],
        club: None,
    };
    let walk = |p: &mut Pose, amp: f32, bob: f32| {
        for (i, sgn) in [1.0f32, -1.0].iter().enumerate() {
            let a = cyc * amp * sgn;
            p.foot[i].0 = s(5.0) * a;
            p.foot[i].1 = s(1.2) * (-a).max(0.0);
            p.knee[i].0 = s(3.0) * a + s(1.2);
            p.hand[i].0 = -s(4.0) * a;
            p.elbow[i].0 = -s(2.0) * a;
        }
        let b = (t * 4.0 * PI).cos().abs() * s(bob);
        p.hip.1 += b;
        p.chest.1 += b;
        p.head.1 += b;
    };
    let bend = |p: &mut Pose, fwd: f32, down: f32| {
        p.chest.0 += fwd;
        p.chest.1 -= down;
        p.head.0 += fwd * 1.5;
        p.head.1 -= down * 1.4;
        p.hip.0 -= fwd * 0.3;
        p.knee[0].0 += fwd * 0.4;
        p.knee[1].0 += fwd * 0.4;
        for i in 0..2 {
            p.elbow[i].0 += fwd * 1.2;
            p.elbow[i].1 -= down;
        }
    };
    // a club held in both hands at the front, swung through angle `a` (0 the ball, up and back as it grows)
    let swing = |p: &mut Pose, a: f32, len: f32| {
        let r = s(6.5);
        let (sa, ca) = a.sin_cos();
        let grip = (s(1.5) + r * 0.4 * sa.max(-0.5), p.chest.1 - r * ca, -r * sa * 0.9);
        p.hand = [grip, (grip.0, grip.1 + s(0.6), grip.2 + s(0.4))];
        p.elbow = [
            (grip.0 * 0.5, (grip.1 + p.chest.1) / 2.0, s(2.6) + grip.2 * 0.4),
            (grip.0 * 0.5, (grip.1 + p.chest.1) / 2.0, s(-2.0) + grip.2 * 0.4),
        ];
        let l = s(len);
        p.club = Some((grip.0 + l * 0.25, grip.1 - l * ca, grip.2 - l * sa * 0.95));
    };
    if has("walk") || has("stride") || has("run") {
        walk(&mut p, if has("tired") { 0.45 } else { 0.8 }, 0.6);
        if has("tired") {
            bend(&mut p, s(1.2), s(0.8));
        }
    } else if has("cart") {
        // seated, hands on the wheel
        p.hip = (s(-1.0), s(9.0), 0.0);
        p.chest = (s(-0.6), s(15.5), 0.0);
        p.head = (s(-0.3), s(19.3), 0.0);
        p.knee = [(s(3.0), s(9.5), s(1.6)), (s(3.0), s(9.5), s(-1.6))];
        p.foot = [(s(3.5), s(4.0), s(1.6)), (s(3.5), s(4.0), s(-1.6))];
        p.hand = [(s(4.0), s(13.0), s(1.5)), (s(4.0), s(13.0), s(-1.5))];
        p.elbow = [(s(1.5), s(13.0), s(3.2)), (s(1.5), s(13.0), s(-3.2))];
    } else if has("sit") {
        p.hip = (s(-1.0), s(7.0), 0.0);
        p.chest = (s(-0.8), s(13.5), 0.0);
        p.head = (s(-0.5), s(17.3 + cyc * 0.3), 0.0);
        p.knee = [(s(3.5), s(7.5), s(1.6)), (s(3.5), s(7.5), s(-1.6))];
        p.foot = [(s(3.8), 0.0, s(1.8)), (s(3.8), 0.0, s(-1.8))];
        p.hand = [(s(2.5), s(7.8), s(2.6)), (s(2.5), s(7.8), s(-2.6))];
        p.elbow = [(s(0.5), s(10.0), s(3.2)), (s(0.5), s(10.0), s(-3.2))];
    } else if has("swing") || (has("pitch") && !has("address")) {
        let big = if has("pitch") { 0.55 } else { 1.0 };
        // back slowly, down fast, through and up
        let a = if t < 0.45 {
            (t / 0.45) * 2.6 * big
        } else if t < 0.6 {
            2.6 * big * (1.0 - (t - 0.45) / 0.15)
        } else {
            -((t - 0.6) / 0.4) * 2.4 * big
        };
        bend(&mut p, s(1.0), s(0.6));
        swing(&mut p, a, 10.0);
    } else if has("putt") && !has("lineup") {
        bend(&mut p, s(2.0), s(1.6));
        let a = if has("address") { 0.0 } else { (t * 2.0 * PI).sin() * 0.45 };
        swing(&mut p, a, 7.0);
    } else if has("address") {
        bend(&mut p, s(1.6), s(1.0));
        swing(&mut p, (t * 2.0 * PI).sin() * 0.12, 10.0);
    } else if has("lineup") {
        // crouched behind the ball
        p.hip = (s(-1.5), s(6.0), 0.0);
        p.chest = (s(0.5), s(12.0), 0.0);
        p.head = (s(1.5), s(15.5), 0.0);
        p.knee = [(s(2.5), s(5.0), s(2.0)), (s(2.5), s(5.0), s(-2.0))];
        p.foot = [(0.0, 0.0, s(2.0)), (0.0, 0.0, s(-2.0))];
        p.hand = [(s(4.0), s(9.0 + cyc), s(1.0)), (s(3.0), s(7.0), s(-2.0))];
        p.elbow = [(s(2.0), s(10.0), s(2.6)), (s(1.5), s(9.0), s(-2.8))];
    } else if has("happy") || has("success") || has("celeb") {
        let up = (t * 4.0 * PI).sin().abs();
        let jump = s(2.0) * up;
        for q in [&mut p.hip, &mut p.chest, &mut p.head] {
            q.1 += jump;
        }
        for i in 0..2 {
            p.knee[i].1 += jump * 0.7;
            p.foot[i].1 += jump * 0.5;
            let z = if i == 0 { 1.0 } else { -1.0 };
            p.elbow[i] = (0.0, s(21.0) + jump, s(4.5) * z);
            p.hand[i] = (s(0.5), s(26.0) + jump, s(5.0) * z);
        }
    } else if has("sad") || has("failure") || has("mad") {
        bend(&mut p, s(1.5), s(1.4));
        let shake = (t * 6.0 * PI).sin();
        p.head.2 += s(0.6) * shake;
        if has("failure") {
            p.hand[0] = (s(1.0), s(22.5), s(1.5));
            p.elbow[0] = (s(1.5), s(19.0), s(3.5));
        }
    } else if has("point") || has("action") || has("greet") || has("hand") {
        let lift = (t * 2.0 * PI).sin() * 0.5 + 0.5;
        p.elbow[0] = (s(3.0), s(18.5), s(3.2));
        p.hand[0] = (s(6.0), s(18.0 + lift * 2.0), s(2.5));
    } else if has("lookup") {
        p.head.0 -= s(0.8);
        p.head.1 += s(0.3);
    } else if has("lean") {
        let z = if has("left") { 1.0 } else { -1.0 } * s(1.5) * (t * PI).sin();
        p.chest.2 += z;
        p.head.2 += z * 1.5;
    } else {
        // standing: a little weight shift and breathing
        let b = cyc * s(0.25);
        p.chest.1 += b;
        p.head.1 += b;
        p.hip.2 += cyc * s(0.3);
    }
    p
}

/// The body of a person file: (height, width factor, female, skirt, cap) from the file and folder name.
fn figure(c: &Ctx) -> (f32, f32, bool, bool, bool) {
    let female = c.low.contains("/female/")
        || c.has("female")
        || c.has("fem")
        || c.has("girl")
        || c.has("princess")
        || c.has("model")
        || c.has("popsinger")
        || c.has("bev");
    let wide = if c.has("fat") || c.has("pls") || c.has("kls") || c.has("aging") || c.has("politician") || c.has("soda") {
        1.25
    } else if c.has("sss") || c.has("model") || c.has("fitness") || c.has("sk") {
        0.9
    } else {
        1.0
    };
    let tall = if c.has("kls") {
        24.0
    } else if female {
        24.5
    } else {
        26.0
    };
    let skirt = female && (c.has("sk") || c.has("princess") || c.has("girl") || c.has("model"));
    let cap = !female || c.has("bev") || c.has("tray");
    (tall, wide, female, skirt, cap)
}

fn person(ix: &mut Ix, c: &Ctx) {
    let (tall, wide, female, skirt, cap) = figure(c);
    let tall = tall * c.scale;
    let p = pose(c.name, c.t(), tall);
    // the facing on the screen: view 0 down-left, each next view 45 degrees anticlockwise (4-view files: 90)
    let step = if c.views >= 8 { 45.0 } else { 90.0 };
    let phi = (135.0 - step * c.v as f32).to_radians();
    let (fx, fz) = (phi.cos(), phi.sin());
    // figure space (x forwards, z left) to the screen's ground axes, then up
    let to = |q: (f32, f32, f32)| -> ((f32, f32), f32) {
        let gx = q.0 * fx + q.2 * fz;
        let gz = q.0 * fz - q.2 * fx;
        ((c.view.ax + gx, c.view.ay + gz * SINP - q.1 * COSP), gz)
    };
    // a golf cart under a seated figure
    if c.has("cart") {
        cart(ix, c, phi);
    }
    if c.has("soda") {
        // the soda vendor's trolley
        let (a, _) = to((5.0, 6.0, 0.0));
        ix.ball(a.0, a.1, 5.0, 4.0, RED, 0.1);
        ix.ball(a.0, a.1 - 4.5, 4.0, 1.5, WHITE, 0.0);
    }
    if c.has("lawn") || c.has("gk") {
        let (a, _) = to((6.0, 2.5, 0.0));
        ix.ball(a.0, a.1, 4.0, 2.6, RED, 0.0);
        ix.ball(a.0, a.1 + 1.5, 4.5, 1.2, DARK, 0.0);
    }
    let hands = if female { HANDS_F } else { ALT };
    let k = tall / 26.0;
    // parts as (depth, draw)
    enum Part {
        Rod((f32, f32), (f32, f32), f32, Mat),
        Ball((f32, f32), f32, f32, Mat),
    }
    let mut parts: Vec<(f32, Part)> = Vec::new();
    let mut rod = |a: (f32, f32, f32), b: (f32, f32, f32), r: f32, m: Mat, bias: f32| {
        let (pa, da) = to(a);
        let (pb, db) = to(b);
        parts.push(((da + db) / 2.0 + bias, Part::Rod(pa, pb, r, m)));
    };
    let pants_m = PANTS;
    for i in 0..2 {
        rod(p.hip, p.knee[i], 1.45 * k * wide, pants_m, 0.0);
        rod(p.knee[i], p.foot[i], 1.25 * k, if skirt { hands } else { pants_m }, 0.0);
        let toe = (p.foot[i].0 + 1.4 * k, p.foot[i].1, p.foot[i].2);
        rod(p.foot[i], toe, 0.9 * k, SHOE, 0.1);
    }
    // torso
    rod(p.hip, p.chest, 2.5 * k * wide, SHIRT, 0.3);
    if skirt {
        let hem = (p.hip.0, p.hip.1 - 4.5 * k, p.hip.2);
        rod(p.hip, hem, 2.6 * k * wide, PANTS, 0.25);
    }
    for i in 0..2 {
        let z = if i == 0 { 1.0 } else { -1.0 };
        let shoulder = (p.chest.0, p.chest.1 + 0.5 * k, p.chest.2 + 2.7 * k * wide * z);
        rod(shoulder, p.elbow[i], 1.0 * k * wide, SHIRT, 0.5);
        rod(p.elbow[i], p.hand[i], 0.85 * k, hands, 0.6);
    }
    let (hp, hd) = to(p.head);
    if !c.headless {
        parts.push((hd + 0.8, Part::Ball(hp, 2.75 * k, 2.85 * k, SKIN)));
    }
    parts.sort_by(|a, b| a.0.partial_cmp(&b.0).unwrap_or(std::cmp::Ordering::Equal));
    for (_, part) in &parts {
        match *part {
            Part::Rod(a, b, r, m) => ix.rod(a, b, r, m, 0.0),
            Part::Ball(ctr, rx, ry, m) => ix.ball(ctr.0, ctr.1, rx, ry, m, 0.05),
        }
    }
    if c.headless {
        return;
    }
    // hair or cap, and a face towards the viewer
    let facing_viewer = fz > -0.2;
    if female && !cap {
        ix.ball(hp.0 - fx * 0.8 * k, hp.1 - 1.2 * k, 2.9 * k, 2.2 * k, ALT, 0.0);
        ix.rod((hp.0 - fx * 1.6 * k, hp.1 - 0.5 * k), (hp.0 - fx * 1.8 * k, hp.1 + 3.0 * k), 1.6 * k, ALT, 0.0);
    } else {
        ix.ball(hp.0, hp.1 - 1.9 * k, 2.5 * k, 1.2 * k, HAT, 0.1);
        ix.rod((hp.0, hp.1 - 0.9 * k), (hp.0 + fx * 3.6 * k, hp.1 - 0.6 * k + fz * 1.0 * k), 0.8 * k, HAT, 0.0);
    }
    if facing_viewer {
        let ex = hp.0 + fx * 1.2 * k;
        let ey = hp.1 + 0.2 * k;
        ix.set((ex - 0.8 * k * fz.abs().max(0.4)).floor() as i32, ey.floor() as i32, DARK.at(0.0));
        ix.set((ex + 0.8 * k * fz.abs().max(0.4)).floor() as i32, ey.floor() as i32, DARK.at(0.0));
    }
    if let Some(head) = p.club {
        let (a, _) = to(p.hand[0]);
        let (b, _) = to(head);
        ix.line(a, b, STONE.at(0.8));
        ix.rod(b, (b.0 + fx * 1.2, b.1 + 0.3), 0.6, STONE, 0.1);
    }
    // the celebrity's own touch: a gold medal, a guitar, a ball
    if c.low.contains("/celebs/") {
        let m = [GOLD, RED, PURPLE, YELLOW][(c.seed % 4) as usize];
        let (a, _) = to((p.chest.0 + 2.0, p.chest.1 - 1.0, 0.0));
        ix.ball(a.0, a.1, 1.2 * k, 1.2 * k, m, 0.2);
    }
}

/// A body still (Bodies/<body>.pcx): the figure from the front without its head, in the golfer colour slots, for the
/// customise screens to recolour and put a head on.
pub fn body_still(name: &str, w: u32, h: u32) -> Vec<u8> {
    let mut ix = Ix::new(w, h);
    let low = format!("bodies/{name}");
    let small = name.contains("_sm");
    let ctx = Ctx {
        name: &name.replace(".pcx", "_sq"),
        low: &low,
        view: View { ax: w as f32 / 2.0, ay: h as f32 - 3.0, yaw: 0.0 },
        v: 1,
        views: 8,
        f: 0,
        fpv: 1,
        w: w as f32,
        h: h as f32,
        seed: hash(name),
        scale: (h as f32 - 6.0) / (26.0 * COSP * 0.8) * if small { 0.85 } else { 1.0 }
            / if name.starts_with("female") { 24.5 / 26.0 } else { 1.0 },
        headless: true,
    };
    person(&mut ix, &ctx);
    let mut pal = palette(0, name.starts_with("female"), 0);
    pal[255] = [248, 0, 248];
    pcx_indexed(w, h, ix.px, &pal)
}

fn cart(ix: &mut Ix, c: &Ctx, phi: f32) {
    let v = View { ax: c.view.ax, ay: c.view.ay, yaw: -phi };
    // a box body along the facing, a white roof on posts and four wheels
    let (l, w) = (10.0, 5.5);
    let corners = |y: f32| [v.p(-l / 2.0, y, -w), v.p(l / 2.0, y, -w), v.p(l / 2.0, y, w), v.p(-l / 2.0, y, w)];
    for (x, z) in [(-l / 2.0 + 1.5, -w), (l / 2.0 - 1.5, -w), (-l / 2.0 + 1.5, w), (l / 2.0 - 1.5, w)] {
        let q = v.p(x, 1.2, z);
        ix.ball(q.0, q.1, 1.5, 1.4, SHOE, 0.0);
    }
    let lo = corners(2.0);
    let hi = corners(7.0);
    for i in 0..4 {
        let j = (i + 1) % 4;
        ix.poly(&[lo[i], lo[j], hi[j], hi[i]], WHITE.at(0.4 + 0.15 * i as f32));
    }
    ix.poly(&hi, WHITE.at(0.85));
    for (x, z) in [(-l / 2.0 + 1.0, -w + 0.5), (-l / 2.0 + 1.0, w - 0.5)] {
        ix.line(v.p(x, 7.0, z), v.p(x, 19.0, z), STONE.at(0.5));
    }
    let roof = [v.p(-l / 2.0, 19.0, -w), v.p(l / 2.0 - 2.0, 19.0, -w), v.p(l / 2.0 - 2.0, 19.0, w), v.p(-l / 2.0, 19.0, w)];
    ix.poly(&roof, WHITE.at(1.0));
}

// ---- animals -------------------------------------------------------------------------------------------------------------------------

/// A bird of prey of the fly-overs (Scenic/Eagle, Hawk, Vulture, Pelican: SQ gliding, Fidget a wingbeat). The frame is tall
/// because the bird flies high over its ground point (wild_ui.rs draws it there, with no lift), so it is drawn in the air
/// near the top of the frame with nothing under it: wings spread across its heading, body, tail and head.
fn flier(ix: &mut Ix, c: &Ctx, to: &impl Fn(f32, f32, f32) -> (f32, f32)) {
    let n = c.name;
    let t = c.t();
    // the body's height: its centre some 22 pixels under the frame's top
    let top = c.view.ay - c.h;
    let hy = (c.view.ay - (top + 22.0)) / COSP;
    // the bird's size by its frame (the eagle's 50 pixels wide: a wingspan of about 30)
    let s = c.w / 50.0 * 1.3;
    let span = 12.0 * s;
    let (wing, body, head, tail) = if n.contains("pelican") {
        (WHITE, WHITE, WHITE, WHITE)
    } else if n.contains("vulture") {
        (DARK, DARK, RED, DARK)
    } else if n.contains("hawk") {
        (BARK, EARTH, EARTH, WOOD)
    } else {
        // the bald eagle: a dark brown bird with a white head and tail
        (BARK, BARK, WHITE, WHITE)
    };
    // the wing tips' rise: a full beat over the Fidget clip, a slow sway while gliding
    let (lift, sway) = if c.has("fidget") { ((t * 2.0 * PI).cos() * span * 0.45, 0.0) } else { (span * 0.08, (t * 2.0 * PI).sin() * 1.2) };
    for side in [-1.0f32, 1.0] {
        // a wing: broad at the body, swept back a little to rounded, fingered tips, the tips lifted (or tilted the other
        // way on the sway)
        let tip = hy + lift + side * sway;
        let mid = hy + (tip - hy) * 0.5;
        let pts = [
            to(2.4 * s, hy, side * 1.2 * s),
            to(2.2 * s, mid, side * span * 0.55),
            to(1.0 * s, tip, side * span),
            to(-1.2 * s, tip, side * span * 0.97),
            to(-2.8 * s, mid, side * span * 0.6),
            to(-3.2 * s, hy, side * 1.2 * s),
        ];
        // lit from the upper left: the up-turned wing is lighter, the far one darker
        let k = 0.45 + 0.08 * side + if lift > 0.0 { 0.1 } else { 0.0 };
        ix.poly(&pts, wing.at(k));
        // the darker flight feathers along the trailing edge and at the tip
        let dark = if n.contains("pelican") { DARK.at(0.3) } else { wing.at(k - 0.3) };
        ix.line(pts[3], pts[4], dark);
        ix.line(pts[2], pts[3], dark);
    }
    // the fanned tail
    let tl =
        [to(-3.0 * s, hy, 1.0 * s), to(-7.0 * s, hy, 2.4 * s), to(-7.4 * s, hy, 0.0), to(-7.0 * s, hy, -2.4 * s), to(-3.0 * s, hy, -s)];
    ix.poly(&tl, tail.at(0.65));
    ix.rod(to(-3.4 * s, hy, 0.0), to(2.8 * s, hy, 0.0), 1.9 * s, body, 0.0);
    let h = to(4.2 * s, hy + 0.6, 0.0);
    ix.ball(h.0, h.1, 1.7 * s, 1.6 * s, head, 0.2);
    let beak = if n.contains("pelican") { 4.0 } else { 1.6 } * s;
    ix.rod(to(5.4 * s, hy + 0.5, 0.0), to(5.4 * s + beak, hy, 0.0), 0.5 * s, YELLOW, 0.1);
}

fn animal(ix: &mut Ix, c: &Ctx) {
    let step = if c.views >= 8 { 45.0 } else { 90.0 };
    let phi = (135.0 - step * c.v as f32).to_radians();
    let (fx, fz) = (phi.cos(), phi.sin());
    let to = |x: f32, y: f32, z: f32| (c.view.ax + x * fx + z * fz, c.view.ay + (x * fz - z * fx) * SINP - y * COSP);
    let t = c.t();
    let cyc = (t * 2.0 * PI).sin();
    let walking = c.has("walk") || c.has("run");
    let down = c.has("down") || c.has("eat") || c.has("drink") || c.has("dig");
    let n = c.name;
    let size = (c.view.ay * 0.9).min(c.w * 0.5).max(4.0);
    if n.contains("snake") || n.contains("croc") || n.contains("gila") {
        // a long low body in segments
        let (m, len, r) = if n.contains("snake") {
            (LEAF, size * 1.3, 1.2)
        } else if n.contains("croc") {
            (LEAF, c.w * 0.42, 3.2)
        } else {
            (RED, c.w * 0.32, 2.4)
        };
        let segs = 8;
        let mut prev = None;
        for i in 0..=segs {
            let u = i as f32 / segs as f32;
            let x = (0.5 - u) * len;
            let z = (u * 3.0 * PI + t * 2.0 * PI).sin()
                * if n.contains("snake") {
                    2.5
                } else if walking {
                    1.0
                } else {
                    0.3
                };
            let q = to(x, r, z);
            if let Some(pq) = prev {
                ix.rod(pq, q, r * (1.0 - u * 0.6), m, 0.0);
            }
            prev = Some(q);
        }
        if !n.contains("snake") {
            for (x, zz) in [(len * 0.3, 1.0), (len * 0.3, -1.0), (-len * 0.1, 1.0), (-len * 0.1, -1.0)] {
                let a = to(x, r, zz * r);
                let b = to(x + cyc * 1.2 * walking as i32 as f32, 0.0, zz * (r + 1.5));
                ix.rod(a, b, 0.8, m, 0.0);
            }
        }
        return;
    }
    if n.contains("eagle") || n.contains("hawk") || n.contains("pelican") || n.contains("vulture") {
        flier(ix, c, &to);
        return;
    }
    if n.contains("dolphin") {
        let arc = t * PI;
        let q = to(0.0, arc.sin() * size * 0.8, (t - 0.5) * size);
        ix.ball(q.0, q.1, 4.0, 2.5, STONE, 0.0);
        ix.rod((q.0 - 3.0, q.1), (q.0 - 6.0, q.1 + 2.0), 1.0, STONE, 0.0);
        let w = to(0.0, 0.0, 0.0);
        ix.ball(w.0, w.1, 6.0, 1.5, WATER, 0.3);
        return;
    }
    if n.contains("ghog") {
        let up = if c.has("popup") { (t * PI).sin() } else { 0.5 };
        let q = to(0.0, 2.0 + up * 3.0, 0.0);
        let g = to(0.0, 0.0, 0.0);
        ix.ball(g.0, g.1, 4.0, 1.6, EARTH, -0.1);
        ix.ball(q.0, q.1, 2.5, 3.0 * up.max(0.3), BARK, 0.1);
        return;
    }
    // birds on legs and four-legged animals
    let bird = ["crane", "flamingo", "duck", "rr_", "grouse"].iter().any(|k| n.contains(k));
    let m = if n.contains("sheep") {
        WHITE
    } else if n.contains("flamingo") {
        RED
    } else if n.contains("crane") {
        STONE
    } else if n.contains("duck") || n.contains("grouse") {
        BARK
    } else {
        EARTH
    };
    if bird {
        let leg = if n.contains("duck") || n.contains("grouse") { size * 0.25 } else { size * 0.5 };
        let body_y = leg + size * 0.12;
        for (i, z) in [(0, 0.8f32), (1, -0.8)] {
            let swing = if walking { cyc * 2.0 * if i == 0 { 1.0 } else { -1.0 } } else { 0.0 };
            ix.rod(to(0.0, body_y, z), to(swing, 0.0, z), 0.5, if n.contains("duck") { YELLOW } else { DARK }, 0.0);
        }
        let b = to(0.0, body_y, 0.0);
        ix.ball(b.0, b.1, size * 0.28, size * 0.2, m, 0.0);
        let neck_top = if down { to(size * 0.35, body_y * 0.3, 0.0) } else { to(size * 0.18, body_y + size * 0.35, 0.0) };
        ix.rod(b, neck_top, (size * 0.06).max(0.6), m, 0.1);
        ix.ball(neck_top.0, neck_top.1, size * 0.1 + 0.6, size * 0.1 + 0.6, if n.contains("duck") { LEAF } else { m }, 0.1);
        let beak = (neck_top.0 + fx * (size * 0.15 + 1.0), neck_top.1 + 0.5);
        ix.line(neck_top, beak, YELLOW.at(0.6));
        return;
    }
    let leg = size * 0.38;
    let body_y = leg + size * 0.12;
    for (x, z) in [(size * 0.3, 1.0f32), (size * 0.3, -1.0), (-size * 0.3, 1.0), (-size * 0.3, -1.0)] {
        let swing = if walking { cyc * size * 0.15 * z * x.signum() } else { 0.0 };
        ix.rod(
            to(x, body_y, z * size * 0.12),
            to(x + swing, 0.0, z * size * 0.12),
            (size * 0.05).max(0.6),
            if n.contains("sheep") { DARK } else { m },
            -0.1,
        );
    }
    let b = to(0.0, body_y, 0.0);
    ix.ball(b.0, b.1, size * 0.45, size * 0.24, m, 0.0);
    let head = if down { to(size * 0.55, size * 0.15, 0.0) } else { to(size * 0.55, body_y + size * 0.22, 0.0) };
    ix.rod(to(size * 0.35, body_y + size * 0.05, 0.0), head, size * 0.09, m, 0.05);
    ix.ball(head.0, head.1, size * 0.13, size * 0.11, if n.contains("sheep") { DARK } else { m }, 0.1);
    if n.contains("elk") {
        for z in [-1.0f32, 1.0] {
            let a = (head.0 + z * size * 0.08, head.1 - size * 0.1);
            ix.line(a, (a.0 + z * size * 0.2, a.1 - size * 0.3), BARK.at(0.8));
            ix.line((a.0 + z * size * 0.1, a.1 - size * 0.15), (a.0 + z * size * 0.02, a.1 - size * 0.35), BARK.at(0.8));
        }
    }
}

// ---- trees and plants ----------------------------------------------------------------------------------------------------------

fn tree(ix: &mut Ix, c: &Ctx) {
    let n = c.name;
    // growth over the frames of a tree (16 frames: seed to full crown), full for other plants
    let grow = if c.fpv >= 8 { 0.2 + 0.8 * (c.f as f32 / (c.fpv - 1) as f32) } else { 1.0 };
    let (ax, ay) = (c.view.ax, c.view.ay);
    let height = (ay - 2.0).max(4.0) * grow;
    let half_w = ax.min(c.w - ax).max(3.0) * grow;
    let sway = if c.fpv > 1 && c.fpv < 8 { (c.t() * 2.0 * PI).sin() * 0.8 } else { 0.0 };
    let seed = c.seed;
    let jitter = |k: u32| ((seed.wrapping_mul(2654435761).wrapping_add(k * 40503)) % 1000) as f32 / 1000.0 - 0.5;
    if n.contains("palm") || n.contains("drac") || n.contains("cerc") && n.contains("tall") {
        // a leaning trunk and a crown of fronds
        let top = (ax + height * 0.12 + sway, ay - height * 0.82);
        let segs = 6;
        for i in 0..segs {
            let u0 = i as f32 / segs as f32;
            let u1 = (i + 1) as f32 / segs as f32;
            let p0 = (ax + (top.0 - ax) * u0 * u0, ay + (top.1 - ay) * u0);
            let p1 = (ax + (top.0 - ax) * u1 * u1, ay + (top.1 - ay) * u1);
            ix.rod(p0, p1, (height * 0.035).max(0.8) * (1.2 - u0 * 0.5), BARK, (i % 2) as f32 * 0.1);
        }
        let fronds = 7;
        for k in 0..fronds {
            let a = k as f32 / fronds as f32 * 2.0 * PI + c.v as f32 * 0.4;
            let len = height * 0.35;
            let end = (top.0 + a.cos() * len, top.1 + a.sin() * len * 0.45 + len * 0.25);
            let mid = (top.0 + a.cos() * len * 0.5, top.1 - len * 0.12 + a.sin() * len * 0.2);
            ix.rod(top, mid, (len * 0.07).max(0.8), LEAF, 0.15);
            ix.rod(mid, end, (len * 0.05).max(0.6), LEAF, 0.0);
        }
        ix.ball(top.0, top.1 + 1.0, 1.6, 1.4, BARK, -0.1);
        return;
    }
    if n.contains("cactus") || n.contains("joshua") {
        let w = (half_w * 0.22).max(1.2);
        let top = (ax + sway * 0.2, ay - height);
        let m = if n.contains("joshua") { BARK } else { LEAF };
        ix.rod((ax, ay), top, w, m, 0.0);
        for (side, at, len) in [(-1.0f32, 0.45f32, 0.35f32), (1.0, 0.6, 0.3)] {
            let base = (ax, ay - height * at);
            let elbow = (ax + side * half_w * 0.55, base.1);
            let tip = (elbow.0, base.1 - height * len);
            ix.rod(base, elbow, w * 0.8, m, 0.0);
            ix.rod(elbow, tip, w * 0.8, m, 0.0);
            if n.contains("joshua") {
                ix.ball(tip.0, tip.1, w * 1.6, w * 1.4, LEAF, 0.0);
            }
        }
        if n.contains("joshua") {
            ix.ball(top.0, top.1, w * 1.8, w * 1.5, LEAF, 0.0);
        }
        return;
    }
    let trunk_top = (ax + sway * 0.3, ay - height * 0.45);
    ix.rod((ax, ay), trunk_top, (half_w * 0.09).max(0.8), BARK, 0.0);
    if n.contains("pine") || n.contains("fir") || n.contains("cypress") || n.contains("aspen") && false {
        // stacked cones
        let tiers = 4;
        for k in 0..tiers {
            let u = k as f32 / tiers as f32;
            let base_y = ay - height * (0.18 + u * 0.62);
            let top_y = base_y - height * 0.38;
            let wdt = half_w * (0.95 - u * 0.55) * if n.contains("cypress") { 0.5 } else { 1.0 };
            let cx = ax + sway * u;
            ix.fill(
                (cx - wdt, top_y, cx + wdt, base_y + 2.0),
                |x, y| {
                    let tt = ((y - top_y) / (base_y - top_y)).clamp(0.0, 1.0);
                    (x - cx).abs() <= wdt * tt && y <= base_y + 1.5 - ((x - cx).abs() / wdt.max(0.1)) * 1.5
                },
                |x, y| {
                    let side = (x - cx) / wdt.max(0.1);
                    LEAF.at(0.25 + 0.35 * (1.0 - (y - top_y) / (base_y - top_y + 0.1)) - side * 0.25)
                },
            );
        }
        return;
    }
    // a round crown of overlapping balls; willows droop, flowering trees get blossoms
    let crown_y = ay - height * 0.68;
    let blobs = 5;
    let rw = half_w * 0.62;
    let rh = height * 0.3;
    for k in 0..blobs {
        let a = k as f32 / blobs as f32 * 2.0 * PI;
        let cx = ax + a.cos() * rw * 0.45 + jitter(k) * rw * 0.2 + sway;
        let cy = crown_y + a.sin() * rh * 0.35 + jitter(k + 9) * rh * 0.2;
        ix.ball(cx, cy, rw * 0.7, rh * 0.7, LEAF, -0.05);
    }
    ix.ball(ax + sway, crown_y - rh * 0.2, rw * 0.75, rh * 0.75, LEAF, 0.05);
    if n.contains("willow") {
        for k in 0..9 {
            let x = ax - rw + 2.0 * rw * k as f32 / 8.0;
            ix.rod((x, crown_y), (x + sway, crown_y + rh * 1.3), 0.7, LEAF, (k % 3) as f32 * 0.1);
        }
    }
    if n.contains("plum")
        || n.contains("peach")
        || n.contains("dogwood")
        || n.contains("bougain")
        || n.contains("rhody")
        || n.contains("japanesemaple")
    {
        let m = if n.contains("dogwood") { WHITE } else { RED };
        for k in 0..14 {
            let x = ax + jitter(k * 3) * rw * 1.6;
            let y = crown_y + jitter(k * 3 + 1) * rh * 1.4;
            ix.ball(x, y, 0.9, 0.9, m, 0.3);
        }
    }
}

fn flowers(ix: &mut Ix, c: &Ctx) {
    let n = c.name;
    let (ax, ay) = (c.view.ax, c.view.ay);
    if n.contains("bench") || n.contains("swing") {
        let v = c.view;
        let (l, d) = ((ax.min(c.w - ax) * 0.75).max(4.0), 3.0);
        let seat = [v.p(-l, 5.0, -d), v.p(l, 5.0, -d), v.p(l, 5.0, d), v.p(-l, 5.0, d)];
        for (x, z) in [(-l + 1.0, -d), (l - 1.0, -d), (-l + 1.0, d), (l - 1.0, d)] {
            ix.line(v.p(x, 0.0, z), v.p(x, 5.0, z), DARK.at(0.3));
        }
        let m = if n.contains("red") {
            RED
        } else if n.contains("iron") {
            DARK
        } else {
            WOOD
        };
        ix.poly(&seat, m.at(0.7));
        let back = [v.p(-l, 5.0, -d), v.p(l, 5.0, -d), v.p(l, 11.0, -d), v.p(-l, 11.0, -d)];
        if !n.contains("backless") {
            ix.poly(&back, m.at(0.5));
        }
        return;
    }
    if n.contains("gazebo") || n.contains("topiary") {
        building(ix, c);
        return;
    }
    // a bed: a low mound of leaves dotted with flowers over the footprint (an edge or corner piece is still a bed)
    let rx = ax.min(c.w - ax) * 0.9;
    let ry = rx * SINP * 0.9;
    let cy = ay;
    ix.ball(ax, cy, rx, ry, LEAF, -0.1);
    if n.contains("wall") || n.contains("walled") {
        ix.fill(
            (ax - rx, cy, ax + rx, cy + ry + 3.0),
            |x, y| ((x - ax) / rx).powi(2) + ((y - cy - 3.0) / ry).powi(2) <= 1.0 && y > cy,
            |_, y| STONE.at(0.5 + (y - cy) * 0.05),
        );
    }
    let palette = [RED, YELLOW, PURPLE, WHITE];
    let grow = if c.fpv >= 8 { c.f as f32 / (c.fpv - 1) as f32 } else { 1.0 };
    let count = (rx * ry * 0.18 * grow) as u32;
    for k in 0..count {
        let a = ((c.seed.wrapping_add(k * 7919)) % 1000) as f32 / 1000.0 * 2.0 * PI;
        let r = ((c.seed.wrapping_mul(k + 3)) % 1000) as f32 / 1000.0;
        let x = ax + a.cos() * rx * r.sqrt() * 0.9;
        let y = cy + a.sin() * ry * r.sqrt() * 0.9 - 1.0;
        let m = palette[((k + c.seed) % 4) as usize];
        ix.ball(x, y, 1.0, 0.9, m, 0.3);
    }
}

// ---- buildings and things --------------------------------------------------------------------------------------------------------

/// The size of a building's footprint (half sides along x and z) and height that fit the frame.
fn fit(c: &Ctx, height_share: f32) -> (f32, f32, f32) {
    let (ax, ay) = (c.view.ax, c.view.ay);
    let side = ax.min(c.w - ax).max(4.0);
    // a turned square of half side s spans s * (|cos| + |sin|) either way: 1.41 s at 45 degrees
    let s = side / 1.42 * 0.95;
    let below = (c.h - ay).max(2.0);
    let s = s.min(below / (SINP * 1.42) * 1.05).max(3.0);
    let top_room = ay - s * 1.42 * SINP;
    let hgt = (top_room / COSP * height_share).max(3.0);
    (s, s, hgt)
}

/// A box with a roof, a door and windows, turned for the view.
fn building(ix: &mut Ix, c: &Ctx) {
    let n = c.name;
    let (sx, sz, hgt) = fit(c, 0.92);
    let v = View { yaw: c.view.yaw + PI / 4.0, ..c.view };
    let roof_share = if n.contains("castle") || n.contains("church") || n.contains("chapel") { 0.25 } else { 0.38 };
    let wall_h = hgt * (1.0 - roof_share);
    let roof_h = hgt * roof_share;
    let wall_m = WALL;
    // walls: the four sides, nearest last
    let corners = [(-sx, -sz), (sx, -sz), (sx, sz), (-sx, sz)];
    let mut sides: Vec<(f32, usize)> = (0..4)
        .map(|i| {
            let (a, b) = (corners[i], corners[(i + 1) % 4]);
            (v.depth((a.0 + b.0) / 2.0, (a.1 + b.1) / 2.0), i)
        })
        .collect();
    sides.sort_by(|a, b| a.0.partial_cmp(&b.0).unwrap_or(std::cmp::Ordering::Equal));
    for &(_, i) in &sides {
        let (a, b) = (corners[i], corners[(i + 1) % 4]);
        // face brightness by its outward direction against the light
        let (mx, mz) = ((a.0 + b.0) / 2.0, (a.1 + b.1) / 2.0);
        let (s, cc) = v.yaw.sin_cos();
        let (ox, oz) = (mx * cc - mz * s, mx * s + mz * cc);
        let lit = 0.5 + 0.3 * (-ox / sx.max(1.0)).clamp(-1.0, 1.0) * 0.6 + 0.15 * (oz / sz.max(1.0)).clamp(-1.0, 1.0);
        let quad = [v.p(a.0, 0.0, a.1), v.p(b.0, 0.0, b.1), v.p(b.0, wall_h, b.1), v.p(a.0, wall_h, a.1)];
        ix.poly(&quad, wall_m.at(lit));
        // windows in a row, a door on side 2
        let len = ((b.0 - a.0).powi(2) + (b.1 - a.1).powi(2)).sqrt();
        let count = ((len / 9.0) as i32).clamp(1, 8);
        let floors = ((wall_h / 11.0) as i32).clamp(1, 4);
        for fl in 0..floors {
            for k in 0..count {
                let u = (k as f32 + 0.5) / count as f32;
                let (x, z) = (a.0 + (b.0 - a.0) * u, a.1 + (b.1 - a.1) * u);
                let hw = (len / count as f32 * 0.22).min(2.5);
                let (dx, dz) = ((b.0 - a.0) / len * hw, (b.1 - a.1) / len * hw);
                let y0 = wall_h * (fl as f32 + 0.35) / floors as f32;
                let y1 = y0 + (wall_h / floors as f32 * 0.4).min(5.0);
                let door = i == 2 && fl == 0 && k == count / 2;
                let (y0, y1) = if door { (0.0, (wall_h * 0.55).min(7.0)) } else { (y0, y1) };
                let win = [v.p(x - dx, y0, z - dz), v.p(x + dx, y0, z - dz), v.p(x + dx, y1, z + dz), v.p(x - dx, y1, z + dz)];
                ix.poly(&win, if door { WOOD.at(0.4) } else { DARK.at(0.35 + 0.3 * lit) });
            }
        }
    }
    // the roof: a hip roof to a ridge along the longer side (a flat top for towers and pools)
    let flat =
        n.contains("swim") || n.contains("spa") || n.contains("air") || n.contains("hel") || n.contains("ten") && !n.contains("tent");
    if flat {
        let top = [v.p(-sx, wall_h, -sz), v.p(sx, wall_h, -sz), v.p(sx, wall_h, sz), v.p(-sx, wall_h, sz)];
        ix.poly(&top, STONE.at(0.7));
        if n.contains("swim") || n.contains("spa") {
            let pool = [
                v.p(-sx * 0.7, wall_h, -sz * 0.6),
                v.p(sx * 0.7, wall_h, -sz * 0.6),
                v.p(sx * 0.7, wall_h, sz * 0.6),
                v.p(-sx * 0.7, wall_h, sz * 0.6),
            ];
            ix.poly(&pool, WATER.at(0.7));
        }
        if n.contains("ten") {
            let net = [v.p(0.0, wall_h, -sz * 0.8), v.p(0.0, wall_h, sz * 0.8)];
            ix.line(net[0], net[1], WHITE.at(1.0));
        }
        return;
    }
    let ridge = sx * 0.45;
    let peak = [v.p(-ridge, wall_h + roof_h, 0.0), v.p(ridge, wall_h + roof_h, 0.0)];
    let eave = |x: f32, z: f32| v.p(x * 1.08, wall_h, z * 1.08);
    let faces = [
        ([eave(-sx, -sz), eave(sx, -sz), peak[1], peak[0]], (0.0, -sz)),
        ([eave(sx, -sz), eave(sx, sz), peak[1], peak[1]], (sx, 0.0)),
        ([eave(sx, sz), eave(-sx, sz), peak[0], peak[1]], (0.0, sz)),
        ([eave(-sx, sz), eave(-sx, -sz), peak[0], peak[0]], (-sx, 0.0)),
    ];
    let mut order: Vec<usize> = (0..4).collect();
    order.sort_by(|&a, &b| {
        v.depth(faces[a].1 .0, faces[a].1 .1).partial_cmp(&v.depth(faces[b].1 .0, faces[b].1 .1)).unwrap_or(std::cmp::Ordering::Equal)
    });
    for k in order {
        let (q, m) = faces[k];
        let (s, cc) = v.yaw.sin_cos();
        let ox = m.0 * cc - m.1 * s;
        let lit = 0.55 - 0.25 * (ox / sx.max(1.0)).clamp(-1.0, 1.0);
        ix.poly(&q, ROOF.at(lit));
    }
    if n.contains("castle") || n.contains("church") || n.contains("chapel") || n.contains("hotel") || n.contains("club") {
        // a tower or chimney
        let tw = sx * 0.22;
        let tx = sx * 0.55;
        let top_h = wall_h + roof_h * 1.5;
        let tower = [v.p(tx - tw, wall_h * 0.5, 0.0), v.p(tx + tw, wall_h * 0.5, 0.0), v.p(tx + tw, top_h, 0.0), v.p(tx - tw, top_h, 0.0)];
        ix.poly(&tower, if n.contains("club") || n.contains("hotel") { STONE.at(0.6) } else { WALL.at(0.6) });
        let spire = [v.p(tx - tw * 1.2, top_h, 0.0), v.p(tx + tw * 1.2, top_h, 0.0), v.p(tx, top_h + roof_h * 0.8, 0.0)];
        ix.poly(&spire, ROOF.at(0.4));
    }
    if n.contains("mar") && !n.contains("mart") {
        // a jetty and a boat
        let jetty = [v.p(sx, 0.5, -2.0), v.p(sx * 1.4, 0.5, -2.0), v.p(sx * 1.4, 0.5, 2.0), v.p(sx, 0.5, 2.0)];
        ix.poly(&jetty, WOOD.at(0.6));
    }
}

/// A flat ground slab over the footprint (a building's base or dirt layer, a platform).
fn slab(ix: &mut Ix, c: &Ctx) {
    let (sx, sz, _) = fit(c, 0.2);
    let v = View { yaw: c.view.yaw + PI / 4.0, ..c.view };
    let raised = if c.has("plat") { 3.0 } else { 0.0 };
    let m = if c.has("dirt") {
        EARTH
    } else if c.has("plat") {
        WOOD
    } else {
        STONE
    };
    let top = [v.p(-sx, raised, -sz), v.p(sx, raised, -sz), v.p(sx, raised, sz), v.p(-sx, raised, sz)];
    if raised > 0.0 {
        let low = [v.p(-sx, 0.0, -sz), v.p(sx, 0.0, -sz), v.p(sx, 0.0, sz), v.p(-sx, 0.0, sz)];
        for i in 0..4 {
            let j = (i + 1) % 4;
            ix.poly(&[low[i], low[j], top[j], top[i]], m.at(0.3 + 0.1 * i as f32));
        }
    }
    ix.poly(&top, m.at(0.6));
    // a border of paving or grass
    for i in 0..4 {
        ix.line(top[i], top[(i + 1) % 4], m.at(0.85));
    }
}

fn sign(ix: &mut Ix, c: &Ctx) {
    let (ax, ay) = (c.view.ax, c.view.ay);
    let hgt = (ay - 3.0).max(4.0);
    ix.rod((ax, ay), (ax, ay - hgt * 0.55), 0.8, WOOD, 0.0);
    let bw = (ax.min(c.w - ax) * 0.9).max(3.0);
    let board = [(ax - bw, ay - hgt), (ax + bw, ay - hgt), (ax + bw, ay - hgt * 0.5), (ax - bw, ay - hgt * 0.5)];
    ix.poly(&board, WHITE.at(0.8));
    ix.line(board[0], board[1], RED.at(0.5));
    ix.line(board[3], board[2], RED.at(0.5));
}

fn wall(ix: &mut Ix, c: &Ctx) {
    let (sx, _, _) = fit(c, 0.3);
    let v = View { yaw: c.view.yaw + PI / 4.0, ..c.view };
    let h = (c.view.ay * 0.5).max(3.0);
    let m = if c.has("rock") { STONE } else { WOOD };
    let q = [v.p(-sx, 0.0, 0.0), v.p(sx, 0.0, 0.0), v.p(sx, h, 0.0), v.p(-sx, h, 0.0)];
    ix.poly(&q, m.at(0.55));
    ix.line(q[3], q[2], m.at(0.9));
}

fn flag(ix: &mut Ix, c: &Ctx) {
    let (ax, ay) = (c.view.ax, c.view.ay);
    let n = c.name;
    if n.contains("tv") {
        building(ix, c);
        return;
    }
    if n.contains("teemarker") {
        let m = if n.contains("red") {
            RED
        } else if n.contains("blue") {
            PURPLE
        } else {
            WHITE
        };
        let pop = if c.fpv > 1 { (c.f as f32 / (c.fpv - 1) as f32).min(1.0) } else { 1.0 };
        ix.ball(ax, ay - 2.0 * pop, 2.5, 2.0 * pop.max(0.2), m, 0.2);
        return;
    }
    // a pole and a waving flag; "pop" files rise out of the ground
    let rise = if n.contains("pop") && c.fpv > 1 { (c.f as f32 / (c.fpv - 1) as f32).min(1.0) } else { 1.0 };
    let pole = (ay - 3.0).max(5.0) * rise;
    ix.line((ax, ay), (ax, ay - pole), WHITE.at(0.9));
    let m = if n.contains("yflag") || n.contains("yellow") {
        YELLOW
    } else if n.contains("white") {
        WHITE
    } else {
        RED
    };
    let fw = (c.w - ax - 1.0).clamp(4.0, 12.0) * rise;
    let fh = (pole * 0.3).max(3.0);
    let wave = c.t() * 2.0 * PI;
    ix.fill(
        (ax, ay - pole - 1.0, ax + fw, ay - pole + fh + 2.0),
        |x, y| {
            let u = (x - ax) / fw;
            let off = (wave + u * 4.0).sin() * 1.2 * u;
            y >= ay - pole + off && y <= ay - pole + fh * (1.0 - u * 0.5) + off
        },
        |x, _| m.at(0.5 + 0.3 * ((wave + (x - ax) * 0.5).sin())),
    );
}

/// Water effects and animated overlays: ripples on the ground under the frame's anchor (nothing floats in the air).
fn water_fx(ix: &mut Ix, c: &Ctx) {
    let watery = ["reflect", "ripple", "underwater", "wave", "spray", "splash", "fountain", "water"].iter().any(|k| c.has(k));
    if !watery {
        return;
    }
    let (ax, ay) = (c.view.ax, c.view.ay);
    let t = c.t();
    if c.low.contains("flics/water/") && c.has("spray") {
        // the foam at a waterfall's foot (Water/sprayShortA, sprayTallA and the themes'): white water churning in a low
        // cloud of droplets that rise and fall over the clip
        let (rx, ry) = ((c.w * 0.22).max(4.0), (c.w * 0.22).max(4.0) * SINP * 0.6);
        ix.fill(
            (ax - rx, ay - ry, ax + rx, ay + ry),
            |x, y| ((x - ax) / rx).powi(2) + ((y - ay) / ry).powi(2) <= 1.0,
            |x, y| if noise(x as i32, y as i32, c.seed ^ c.f) > 0.45 { WHITE.at(0.85) } else { WATER.at(0.8) },
        );
        for k in 0..10 {
            let a = noise(k, 1, c.seed) * 2.0 * PI;
            let (vx, vy) = (a.cos() * rx * 0.9, 2.0 + 4.0 * noise(k, 2, c.seed));
            let u = (t + noise(k, 3, c.seed)) % 1.0;
            let (x, y) = (ax + vx * u, ay - vy * 4.0 * u * (1.0 - u));
            ix.set(x as i32, y as i32, WHITE.at(0.9));
        }
        return;
    }
    if c.has("spray") || c.has("splash") || c.has("fountain") {
        let hgt = (ay - 2.0).max(3.0) * (0.6 + 0.4 * (t * 2.0 * PI).sin().abs());
        for k in 0..6 {
            let a = (k as f32 - 2.5) * 0.25;
            ix.rod((ax, ay), (ax + a * hgt * 0.6, ay - hgt * (1.0 - a.abs())), 0.6, WATER, 0.6);
        }
        return;
    }
    let r = ax.min(c.w - ax) * 0.7;
    for k in 0..2 {
        let rr = r * ((t + k as f32 / 2.0) % 1.0);
        if rr < r * 0.3 {
            continue;
        }
        ix.fill(
            (ax - rr - 1.0, ay - rr * SINP - 1.0, ax + rr + 1.0, ay + rr * SINP + 1.0),
            |x, y| {
                let d = (((x - ax) / rr.max(0.5)).powi(2) + ((y - ay) / (rr * SINP).max(0.5)).powi(2)).sqrt();
                (d - 1.0).abs() < 0.06 && ((x * 0.7 + y) as i32) % 3 != 0
            },
            |_, _| WATER.at(0.75),
        );
    }
}

/// The waterfalls where water meets higher water (Water/WaterfallShortA, WaterfallTallA and the themes' own; decor.rs
/// `waterfalls` places them on a tile's edges): a sheet of water pouring over the step, streaked white and moving down
/// over the clip, foam where it lands. Views 0 and 1 are the falls on the tile's far edges (the right one running down to
/// the right, the left one up), 2 and 3 the low lip on its near edges; the tall fall is a column between two mossy rocks.
fn waterfall(ix: &mut Ix, c: &Ctx) {
    let (ax, ay) = (c.view.ax, c.view.ay);
    let t = c.t();
    let seed = c.seed;
    // a falling streak's brightness at a pixel: light runs moving down the sheet, one loop over the clip
    let streak = |x: f32, y: f32| {
        let col = x.floor();
        let lane = noise(col as i32, 7, seed);
        let w = ((y * 0.45 - t * 2.0 * PI * 2.0 + lane * 9.0).sin() + 1.0) * 0.5;
        lane * 0.5 + w * 0.5
    };
    let paint = |k: f32, foam: bool| {
        if foam || k > 0.82 {
            WHITE.at(0.55 + 0.4 * k)
        } else {
            WATER.at(0.35 + 0.55 * k)
        }
    };
    if c.has("tall") {
        // the column, slightly wider where it lands, between rocks at its lip
        let (top, bot) = (8.0, (ay + 2.0).min(c.h - 3.0));
        let hw = |y: f32| 4.5 + 2.0 * ((y - top) / (bot - top)).clamp(0.0, 1.0);
        ix.fill(
            (ax - 8.0, top, ax + 8.0, bot),
            |x, y| (x - ax).abs() <= hw(y) + (noise(y as i32, 3, seed) - 0.5) * 1.5,
            |x, y| {
                let edge = (x - ax).abs() > hw(y) - 1.0;
                let k = streak(x, y) * if edge { 0.6 } else { 1.0 };
                paint(k, y > bot - 3.0 && noise(x as i32, y as i32, seed ^ c.f) > 0.4)
            },
        );
        for side in [-1.0f32, 1.0] {
            ix.ball(ax + side * 7.0, top + 1.0, 4.0, 3.0, STONE, -0.05);
            ix.ball(ax + side * 9.5, top + 4.0, 3.0, 2.5, LEAF, -0.1);
        }
        return;
    }
    // the short fall: a band along the tile's edge, 9 pixels high on the far edges, a 3 pixel foaming lip on the near ones
    let s = if c.v == 0 || c.v == 2 { 1.0 } else { -1.0 };
    let (mid, high) = if c.v < 2 { (ay - 6.0, 9.0) } else { (ay - 12.0, 3.0) };
    let half = (c.w * 0.5 - 6.0).max(4.0);
    let centre = |x: f32| mid + s * 0.6 * (x - ax);
    ix.fill(
        (ax - half - 1.0, mid - half * 0.6 - high, ax + half + 1.0, mid + half * 0.6 + high),
        |x, y| {
            let d = y - centre(x);
            let ragged = (noise(x as i32, 1, seed) - 0.5) * 2.0;
            (x - ax).abs() <= half && d.abs() <= high * 0.5 + ragged * 0.6
        },
        |x, y| {
            let d = (y - centre(x)) / (high * 0.5);
            // the lip lit on top, foam at the foot
            let foam = d > 0.55 && noise(x as i32, y as i32, seed ^ c.f) > 0.45;
            let k = streak(x, y - centre(x)) * (0.75 + 0.25 * (1.0 - d.abs()));
            paint(if d < -0.6 { k.max(0.7) } else { k }, foam)
        },
    );
}

/// The pieces in shallow, flat water (wild_ui.rs lays them on the water with its palette): the tropical sand bars
/// (Water/AStropsand*), and what shows under the surface (Water/UW*): rocks, dim and low, and the tropics' coral. Low
/// and flat, so nothing stands up out of the water.
fn water_piece(ix: &mut Ix, c: &Ctx) {
    let (ax, ay) = (c.view.ax, c.view.ay);
    let half = ax.min(c.w - ax).max(3.0);
    let seed = c.seed ^ c.v.wrapping_mul(0x9e37);
    if c.has("sand") {
        // a sand bar just breaking the surface: pale on top, darker and wet at its rim, a few shells
        let (rx, ry) = (half * 0.8, half * 0.8 * SINP * 0.7);
        let lump = |x: f32, y: f32| 1.0 + (noise((x / 3.0) as i32, (y / 3.0) as i32, seed) - 0.5) * 0.3;
        ix.fill(
            (ax - rx, ay - ry * 1.3, ax + rx, ay + ry * 1.3),
            |x, y| ((x - ax) / rx).powi(2) + ((y - ay) / ry).powi(2) <= lump(x, y),
            |x, y| {
                let d = ((x - ax) / rx).powi(2) + ((y - ay) / ry).powi(2);
                if d > 0.75 {
                    EARTH.at(0.45)
                } else if noise(x as i32, y as i32, seed ^ 5) > 0.93 {
                    WHITE.at(0.9)
                } else {
                    YELLOW.at(0.25 + 0.15 * (1.0 - d) + 0.1 * noise(x as i32, y as i32, seed))
                }
            },
        );
        return;
    }
    if c.has("coral") {
        // low clumps of branching coral seen through the water
        let k = 3 + (seed % 3) as i32;
        for i in 0..k {
            let ox = (noise(i, 1, seed) - 0.5) * half * 1.2;
            let oy = (noise(i, 2, seed) - 0.5) * half * 0.5;
            let m = [RED, PURPLE, YELLOW][(i as u32 + seed) as usize % 3];
            let (bx, by) = (ax + ox, ay + oy);
            ix.ball(bx, by, 2.5, 1.5, m, -0.2);
            for j in 0..4 {
                let a = (j as f32 - 1.5) * 0.45 + (noise(i, 10 + j, seed) - 0.5) * 0.4;
                let l = 2.5 + 2.0 * noise(i, 20 + j, seed);
                ix.rod((bx, by), (bx + a.sin() * l * 1.3, by - a.cos() * l * 0.8), 0.6, m, -0.1 + 0.1 * j as f32);
            }
        }
        return;
    }
    // a rock under the surface: dim, its top near the water
    let (rx, ry) = (half * 0.65, half * 0.65 * SINP);
    ix.ball(ax, ay - ry * 0.2, rx, ry, STONE, -0.25);
    let s = (noise(1, 1, seed) - 0.5) * rx;
    ix.ball(ax + s, ay - ry * 0.4, rx * 0.45, ry * 0.5, STONE, -0.15);
}

fn bridge(ix: &mut Ix, c: &Ctx) {
    if c.has("rock") {
        let (ax, ay) = (c.view.ax, c.view.ay);
        let r = ax.min(c.w - ax) * 0.6;
        ix.ball(ax, ay - r * 0.2, r, r * 0.55, STONE, 0.0);
        return;
    }
    let (sx, sz, _) = fit(c, 0.3);
    let v = View { yaw: c.view.yaw + PI / 4.0, ..c.view };
    let deck = [v.p(-sx, 3.0, -sz * 0.5), v.p(sx, 3.0, -sz * 0.5), v.p(sx, 3.0, sz * 0.5), v.p(-sx, 3.0, sz * 0.5)];
    let m = if c.has("des") {
        EARTH
    } else if c.has("links") || c.has("stone") || c.has("brick") {
        STONE
    } else {
        WOOD
    };
    ix.poly(&deck, m.at(0.6));
    for i in 0..8 {
        let x = -sx + 2.0 * sx * i as f32 / 7.0;
        ix.line(v.p(x, 3.0, -sz * 0.5), v.p(x, 3.0, sz * 0.5), m.at(0.4));
    }
    for z in [-sz * 0.5, sz * 0.5] {
        ix.line(v.p(-sx, 6.0, z), v.p(sx, 6.0, z), m.at(0.8));
        for i in 0..4 {
            let x = -sx + 2.0 * sx * i as f32 / 3.0;
            ix.line(v.p(x, 3.0, z), v.p(x, 6.0, z), m.at(0.7));
        }
    }
}

fn car(ix: &mut Ix, c: &Ctx) {
    let (sx, sz, _) = fit(c, 0.3);
    let v = View { yaw: c.view.yaw + PI / 4.0, ..c.view };
    let (l, w) = (sx, sz * 0.5);
    let m = [RED, YELLOW, DARK, PURPLE][(c.seed % 4) as usize];
    for (x, z) in [(-l * 0.6, -w), (l * 0.6, -w), (-l * 0.6, w), (l * 0.6, w)] {
        let q = v.p(x, 1.5, z);
        ix.ball(q.0, q.1, 2.0, 1.6, SHOE, 0.0);
    }
    let lo = [v.p(-l, 1.5, -w), v.p(l, 1.5, -w), v.p(l, 1.5, w), v.p(-l, 1.5, w)];
    let hi = [v.p(-l, 5.0, -w), v.p(l, 5.0, -w), v.p(l, 5.0, w), v.p(-l, 5.0, w)];
    for i in 0..4 {
        let j = (i + 1) % 4;
        ix.poly(&[lo[i], lo[j], hi[j], hi[i]], m.at(0.35 + 0.1 * i as f32));
    }
    ix.poly(&hi, m.at(0.75));
    let cab = [
        v.p(-l * 0.4, 5.0, -w * 0.8),
        v.p(l * 0.2, 5.0, -w * 0.8),
        v.p(l * 0.1, 7.5, w * 0.8 * 0.0 - w * 0.6),
        v.p(-l * 0.35, 7.5, -w * 0.6),
    ];
    ix.poly(&cab, DARK.at(0.5));
}

/// Landmarks, scenery and water features: a statue, tower, rock, boat or balloon by name, else a stone monument.
fn landmark(ix: &mut Ix, c: &Ctx) {
    let n = c.name;
    let (ax, ay) = (c.view.ax, c.view.ay);
    let half = ax.min(c.w - ax).max(3.0);
    let hgt = (ay - 2.0).max(4.0);
    let has = |k: &str| n.contains(k);
    if has("rock")
        || has("boulder")
        || has("skull")
        || has("stone")
        || has("tarpit")
        || has("hotspring")
        || has("koi")
        || has("oasis")
        || has("pond")
        || has("gold")
    {
        let water = has("pond") || has("koi") || has("hotspring") || has("tarpit") || has("oasis");
        let m = if water {
            WATER
        } else if has("gold") {
            GOLD
        } else if has("skull") {
            WHITE
        } else {
            STONE
        };
        ix.ball(ax, ay - half * 0.1, half * 0.85, half * 0.85 * SINP, if water { STONE } else { m }, -0.1);
        if water {
            ix.ball(ax, ay - half * 0.1, half * 0.65, half * 0.6 * SINP, if has("tarpit") { DARK } else { m }, 0.1);
        } else {
            ix.ball(ax - half * 0.2, ay - half * 0.5, half * 0.5, half * 0.45, m, 0.05);
        }
        return;
    }
    if has("tower") || has("wtow") || has("lighthouse") || has("radio") {
        let w0 = half.min(hgt * 0.18);
        let pts = [(ax - w0, ay), (ax + w0, ay), (ax + w0 * 0.4, ay - hgt), (ax - w0 * 0.4, ay - hgt)];
        if has("radio") {
            ix.line((ax - w0, ay), (ax, ay - hgt), STONE.at(0.4));
            ix.line((ax + w0, ay), (ax, ay - hgt), STONE.at(0.6));
            for k in 1..8 {
                let y = ay - hgt * k as f32 / 8.0;
                let wv = w0 * (1.0 - k as f32 / 8.0);
                ix.line((ax - wv, y), (ax + wv, y), STONE.at(0.5));
            }
            ix.ball(ax, ay - hgt, 1.5, 1.5, RED, 0.3);
        } else {
            ix.poly(&pts, WHITE.at(0.7));
            for k in 1..4 {
                let y = ay - hgt * k as f32 / 4.0;
                ix.line((ax - w0 * (1.0 - 0.6 * k as f32 / 4.0), y), (ax + w0 * (1.0 - 0.6 * k as f32 / 4.0), y), RED.at(0.5));
            }
            ix.ball(ax, ay - hgt, w0 * 0.6, w0 * 0.5, YELLOW, 0.3);
        }
        return;
    }
    if has("blimp") || has("hotair") || has("copter") {
        let cy = ay - hgt * 0.75;
        if has("hotair") {
            ix.ball(ax, cy - half * 0.3, half * 0.6, half * 0.7, RED, 0.0);
            ix.rod((ax - 3.0, cy + half * 0.3), (ax + 3.0, cy + half * 0.3), 2.0, WOOD, 0.0);
        } else if has("blimp") {
            ix.ball(ax, cy, half * 0.9, half * 0.32, STONE, 0.1);
            ix.ball(ax, cy + half * 0.3, half * 0.2, half * 0.08, DARK, 0.0);
        } else {
            ix.ball(ax, cy, half * 0.35, half * 0.22, RED, 0.1);
            ix.line((ax - half * 0.7, cy - half * 0.25), (ax + half * 0.7, cy - half * 0.25), DARK.at(0.3));
        }
        return;
    }
    if has("sail") || has("boat") {
        let t = c.t();
        let bx = ax + (t * 2.0 * PI).sin() * 2.0;
        ix.ball(bx, ay - 2.0, half * 0.55, 2.5, WHITE, 0.0);
        ix.line((bx, ay - 3.0), (bx, ay - hgt * 0.9), WOOD.at(0.5));
        ix.poly(
            &[(bx + 1.0, ay - hgt * 0.88), (bx + 1.0, ay - 5.0), (bx + half * 0.5, ay - 5.0)],
            if has("rainbow") { YELLOW.at(0.7) } else { WHITE.at(0.9) },
        );
        return;
    }
    if has("windmill") || has("wmill") || has("pump") {
        let t = c.t();
        building(ix, c);
        let hub = (ax, ay - hgt * 0.7);
        for k in 0..4 {
            let a = k as f32 * PI / 2.0 + t * PI / 2.0;
            ix.rod(hub, (hub.0 + a.cos() * half * 0.7, hub.1 + a.sin() * half * 0.7), 1.0, WOOD, 0.2);
        }
        return;
    }
    if has("pyramid") || has("volcano") {
        let m = if has("volcano") { EARTH } else { YELLOW };
        ix.poly(&[(ax - half, ay), (ax, ay - hgt), (ax, ay + half * SINP * 0.5)], m.at(0.75));
        ix.poly(&[(ax + half, ay), (ax, ay - hgt), (ax, ay + half * SINP * 0.5)], m.at(0.45));
        if has("volcano") {
            ix.ball(ax, ay - hgt, half * 0.12, half * 0.05, RED, 0.3);
        }
        return;
    }
    if has("chapel")
        || has("barn")
        || has("house")
        || has("pagoda")
        || has("ruins")
        || has("garden")
        || has("gazebo")
        || has("vineyard")
        || has("railroad")
        || has("rock form")
        || has("natural")
    {
        building(ix, c);
        return;
    }
    if has("cattail") || has("grass") || has("heather") || has("flowers") || has("wildflowers") || has("crabgrass") || has("dandelion") {
        flowers(ix, c);
        return;
    }
    // a monument: a plinth and a figure or obelisk
    let pw = half * 0.6;
    ix.poly(&[(ax - pw, ay), (ax + pw, ay), (ax + pw, ay - hgt * 0.25), (ax - pw, ay - hgt * 0.25)], STONE.at(0.5));
    ix.poly(
        &[(ax - pw, ay - hgt * 0.25), (ax + pw, ay - hgt * 0.25), (ax + pw * 0.8, ay - hgt * 0.3), (ax - pw * 0.8, ay - hgt * 0.3)],
        STONE.at(0.8),
    );
    let m = if has("buddha") || has("easter") || has("gargoyle") { STONE } else { GOLD };
    ix.rod((ax, ay - hgt * 0.3), (ax, ay - hgt * 0.8), pw * 0.45, m, 0.0);
    ix.ball(ax, ay - hgt * 0.86, pw * 0.4, pw * 0.4, m, 0.1);
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn birds_of_prey_fly_with_nothing_under_them() {
        // the fly-over's frame is tall because the bird is high over its ground point: nothing (issue #84's post) may be
        // drawn in the frame's lower half
        for (name, w, h, fpv, crop) in [("eagle_sq", 47, 124, 18, (224, 113)), ("eagle_fidget", 50, 133, 12, (223, 104))] {
            let head = FlcHead { w, h, views: 4, fpv, crop, view_ms: 1500, mask: 15, speed: 83 };
            let f = crate::flc::decode_flc(&flc(&format!("flics/scenic/{name}.flc"), &head)).unwrap();
            for fr in &f.frames {
                assert!(fr.idx.iter().any(|&i| i != KEY), "{name}: an empty frame");
                let low = &fr.idx[(h / 2 * w) as usize..];
                assert!(low.iter().all(|&i| i == KEY), "{name}: something under the bird");
            }
        }
    }

    #[test]
    fn waterfalls_and_water_pieces_are_water() {
        // drawn as water, sand and rock, not as the monument (gold) the unknown scenery falls back to
        for name in ["waterfallshorta", "tropwaterfalltalla", "uwtropcorala", "astropsanda", "uwparkrocka"] {
            let head = FlcHead { w: 40, h: 50, views: 4, fpv: 2, crop: (220, 200), view_ms: 1000, mask: 15, speed: 83 };
            let f = crate::flc::decode_flc(&flc(&format!("flics/water/{name}.flc"), &head)).unwrap();
            let used: std::collections::HashSet<u8> = f.frames.iter().flat_map(|fr| fr.idx.iter().copied()).collect();
            assert!(used.iter().any(|&i| i != KEY), "{name}: nothing drawn");
            assert!(!used.iter().any(|&i| (GOLD.base..GOLD.base + GOLD.n).contains(&i)), "{name}: drawn as a monument");
        }
    }

    #[test]
    fn people_keep_their_colour_slots() {
        let head = FlcHead { w: 35, h: 34, views: 8, fpv: 4, crop: (227, 213), view_ms: 330, mask: 0xff, speed: 83 };
        let d = flc("flics/male/malekls_normalwalk.flc", &head);
        let f = crate::flc::decode_flc(&d).unwrap();
        assert_eq!(f.frames.len(), 32);
        let used: std::collections::HashSet<u8> = f.frames.iter().flat_map(|fr| fr.idx.iter().copied()).collect();
        // the shirt, trousers and skin ramps are drawn, so the swap palettes recolour them
        assert!(used.iter().any(|&i| i < 20) && used.iter().any(|&i| (20..40).contains(&i)) && used.iter().any(|&i| (40..60).contains(&i)));
        assert!(used.contains(&KEY));
    }
}
