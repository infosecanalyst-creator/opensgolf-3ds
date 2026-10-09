//! The dock's panels drawn from the disc's own interface art: Build Course (the terrain buttons, with the Amenities panel behind
//! its round button), Add Buildings (the building lots, with the Elevation panel behind its round button) and People (the
//! Employee panel and its hire dialog). Each sheet is read from the game folder at run time; the tables here only say where to
//! cut and where to draw. Positions, hit regions, sprite states and tooltips are the exe's, from docs/UI_ART_MAP.md (Build
//! Course), docs/DECODE_PANELS.md and docs/UI_PANELS.md (the rest); the C++ port's include/sg/panels.h and ui_panels.h hold the
//! same tables.
//!
//! The idle look of every round button, tool button and lot pad is baked into a panel's body; only the hover, selected, armed
//! and disabled looks are separate cuts drawn on top. Hit codes: 0 and up are a panel's own slots, -2 and -3 its round buttons
//! (back or to the other panel, and Undo), -1 nothing.

use crate::app::*;
use crate::gfx::Gfx;
use crate::render::Rect;
use crate::ui::{load_pcx, load_pcx_alpha, money, rgb, rgba, text_width, Image, Screen as Ui};
use sg_core::economy::{self, Economy, WAGE_UNITS};
use sg_core::land;
use sg_core::terrain::Terrain;

/// A hit region of the exe (0x467170): with a = |dx * xs| and b = |dy * ys| the distance is (b + 2a) / 2 when b < a, else
/// (a + 2b) / 2 (an octagon), and the point is inside when that is below r.
#[derive(Clone, Copy)]
pub struct Hit {
    cx: i32,
    cy: i32,
    r: i32,
    xs: i32,
    ys: i32,
}

pub(crate) const fn hit(cx: i32, cy: i32, r: i32) -> Hit {
    Hit { cx, cy, r, xs: 1, ys: 1 }
}

impl Hit {
    pub fn has(&self, px: f32, py: f32) -> bool {
        let a = ((px.floor() as i32 - self.cx) * self.xs).abs();
        let b = ((py.floor() as i32 - self.cy) * self.ys).abs();
        let d = if b < a { (b + 2 * a) / 2 } else { (a + 2 * b) / 2 };
        d < self.r
    }
}

/// A rectangle on a sheet: x, y, w, h.
pub(crate) type Cut = (f32, f32, f32, f32);

pub(crate) fn blit(g: &mut Gfx, s: &Ui, im: &Image, c: Cut, dx: f32, dy: f32) {
    s.image_part(g, im, dx, dy, c.0, c.1, c.2, c.3);
}

/// The panel bodies: cut from their sheet and drawn at the same place.
const TERRAIN_BODY: Cut = (216.0, 482.0, 584.0, 118.0);
const AMENITIES_BODY: Cut = (214.0, 482.0, 586.0, 118.0);
const BUILDINGS_BODY: Cut = (216.0, 482.0, 584.0, 118.0);
const ELEVATION_BODY: Cut = (215.0, 482.0, 585.0, 118.0);
const EMPLOYEE_BODY: Cut = (215.0, 474.0, 585.0, 126.0);

/// Tooltips show once the pointer has stayed on the same hit for more than 10 frames.
const TIP_DELAY_FRAMES: u32 = 10;

/// What the exe shows when Undo is clicked (the port takes the click on the map instead of the right button; the money comes
/// back the same way).
const UNDO_HELP: &str = "Undo: click the map to take back what was built there. Your money will be refunded!";

// ---- Build Course: the terrain panel -------------------------------------------------------------------------------------------

/// The 13 tile buttons: top left corner and the exe's tile id, which is also the port's terrain type. Slot k's art is cell k
/// of the theme's TerrainButtons sheet: 62 x 54 at x = (k > 6 ? 248 : 0) + state * 62, y = (k % 7) * 54, states normal,
/// hover, selected and disabled.
const TERRAIN_SLOTS: [(f32, f32, i32); 13] = [
    (270.0, 508.0, 0),
    (332.0, 508.0, 1),
    (394.0, 508.0, 7),
    (456.0, 508.0, 4),
    (518.0, 508.0, 9),
    (580.0, 508.0, 10),
    (642.0, 508.0, 17),
    (301.0, 546.0, 2),
    (363.0, 546.0, 3),
    (425.0, 546.0, 5),
    (487.0, 546.0, 8),
    (549.0, 546.0, 11),
    (611.0, 546.0, 12),
];
/// The three tree buttons (slots 13..15, tile ids 13..15) sit at a position per theme (exe theme order: Parkland, Desert,
/// Tropical, Links) and have a size per theme; their art is at x = 520 + state * 80, y 0, 150 and 300, three states.
const TREE_POS: [[(f32, f32); 3]; 4] = [
    [(704.0, 477.0), (673.0, 530.0), (735.0, 511.0)],
    [(704.0, 488.0), (673.0, 533.0), (735.0, 496.0)],
    [(704.0, 483.0), (673.0, 519.0), (735.0, 496.0)],
    [(704.0, 478.0), (673.0, 512.0), (735.0, 524.0)],
];
const TREE_SIZE: [[(f32, f32); 3]; 4] = [
    [(65.0, 84.0), (62.0, 70.0), (62.0, 88.0)],
    [(75.0, 74.0), (62.0, 67.0), (65.0, 104.0)],
    [(72.0, 79.0), (62.0, 81.0), (65.0, 104.0)],
    [(67.0, 84.0), (62.0, 88.0), (67.0, 76.0)],
];
const TERRAIN_TO_AMENITIES: (Hit, Cut, f32, f32) = (hit(237, 525, 20), (150.0, 480.0, 34.0, 34.0), 220.0, 510.0);
/// The terrain panel's Undo has a hover look only.
const TERRAIN_UNDO: (Hit, Cut, f32, f32) = (hit(261, 578, 15), (40.0, 550.0, 34.0, 40.0), 246.0, 562.0);

/// Name of a tile button in the theme (exe theme order), from the exe's per-theme tile table.
fn tile_name(id: i32, theme: usize) -> &'static str {
    match (theme, id) {
        (1, 4) => "Desert",
        (1, 5) => "Rough",
        (1, 10) => "Ravine",
        // the desert's tree is drawn as a Joshua tree and its pine as a cactus (the button art and sg_core::decor agree)
        (1, 13) => "Joshua tree",
        (1, 14) => "Cactus",
        (2, 11) => "Tropical bush",
        (2, 13) => "Tropical tree",
        // the Links names are inferred
        (3, 11) => "Gorse",
        _ => [
            "Tees",
            "Green",
            "Fairway",
            "Firm fairway",
            "Rough",
            "Deep rough",
            "Mound",
            "Sand trap",
            "Waste bunker",
            "Pot bunker",
            "Stream",
            "Brush",
            "Rocks",
            "Tree",
            "Pine tree",
            "Palm tree",
            "Elm tree",
            "Water",
        ]
        .get(id as usize)
        .copied()
        .unwrap_or("?"),
    }
}

/// Top left and size of terrain slot i.
fn terrain_slot_rect(i: usize, theme: usize) -> Rect {
    if i < 13 {
        Rect::new(TERRAIN_SLOTS[i].0, TERRAIN_SLOTS[i].1, 62.0, 54.0)
    } else {
        let (x, y) = TREE_POS[theme][i - 13];
        let (w, h) = TREE_SIZE[theme][i - 13];
        Rect::new(x, y, w, h)
    }
}

/// The terrain slot under the pointer. The exe's own test for these buttons is not in the decode: the tile buttons take the
/// diamond of their isometric slab, the trees their rectangle, trees first since they overlap the first row.
fn terrain_slot_at(theme: usize, px: f32, py: f32) -> Option<usize> {
    if let Some(i) = (13..16).rev().find(|&i| terrain_slot_rect(i, theme).has(px, py)) {
        return Some(i);
    }
    (0..13).rev().find(|&i| {
        let r = terrain_slot_rect(i, theme);
        r.has(px, py) && (px - (r.x + 31.0)).abs() / 31.0 + (py - (r.y + 24.0)).abs() / 24.0 <= 1.0
    })
}

fn tile_id(slot: usize) -> i32 {
    if slot < 13 {
        TERRAIN_SLOTS[slot].2
    } else {
        slot as i32
    }
}

// ---- Build Course: the amenities panel -----------------------------------------------------------------------------------------

struct AmenitySlot {
    tip: &'static str,
    /// The building kind the slot arms (land::BUILDINGS; 0 is the path tool), -1 for Undo.
    kind: i32,
    hit: Hit,
    x: f32,
    y: f32,
    /// The hover cut; the selected look is 100 pixels right of it, the disabled look 200.
    hover: Cut,
}

const fn amenity(tip: &'static str, kind: i32, hit: Hit, x: f32, y: f32, hover: Cut) -> AmenitySlot {
    AmenitySlot { tip, kind, hit, x, y, hover }
}

/// Slots by the exe's hit result (slot 8 is art the panel code never tests, so it is left out of the hit test).
const AMENITIES: [AmenitySlot; 10] = [
    amenity("Ballwasher", 3, hit(302, 563, 20), 274.0, 536.0, (0.0, 0.0, 60.0, 56.0)),
    amenity("Pathway", 0, hit(333, 526, 20), 302.0, 504.0, (0.0, 100.0, 64.0, 50.0)),
    amenity("Building Lot", 5, hit(363, 563, 20), 334.0, 538.0, (0.0, 200.0, 62.0, 50.0)),
    amenity("Benches", 1, hit(470, 575, 18), 449.0, 553.0, (300.0, 0.0, 51.0, 47.0)),
    amenity("Landmarks", 4, hit(591, 575, 18), 571.0, 553.0, (300.0, 100.0, 51.0, 45.0)),
    amenity("Flower Bed", 2, hit(652, 575, 18), 632.0, 553.0, (300.0, 150.0, 51.0, 47.0)),
    amenity("Scenic Trees", 16, hit(714, 575, 18), 693.0, 553.0, (300.0, 200.0, 51.0, 45.0)),
    amenity("Undo", -1, hit(248, 567, 12), 234.0, 553.0, (0.0, 450.0, 36.0, 36.0)),
    amenity("", -1, hit(0, 0, 0), 762.0, 545.0, (0.0, 500.0, 38.0, 36.0)),
    amenity("Scenic Bridge", 19, hit(531, 580, 12), 510.0, 553.0, (300.0, 50.0, 51.0, 45.0)),
];
/// "Course Terrain": back to the terrain panel.
const AMENITIES_BACK: (Hit, Cut, f32, f32) = (hit(269, 500, 16), (0.0, 400.0, 34.0, 34.0), 254.0, 483.0);

fn shifted(c: Cut, dx: f32) -> Cut {
    (c.0 + dx, c.1, c.2, c.3)
}

/// The strip of designs the Amenities panel opens over itself for the item being placed (0x434cf0 calls 0x432200): the
/// item's designs side by side, the even ones on a row of buttons and the odd ones on a row 27 pixels lower, each the last
/// frame of view 0 of its sprite.
#[derive(Clone, Copy)]
struct Strip {
    /// Where the strip sits: its left end is dx + 544 - count * 47 / 4.
    dx: i32,
    count: i32,
    /// Size of the pictures (the queue's scale, with the panel's zoom 4: positive s draws s / 4, negative -s / 8).
    scale: f32,
}

/// The strip of the building tool's kind: benches (sprites 0x208 + i, palettes 0xaa + i), landmarks (0x168 + type, only
/// those the club owns), scenic bridges (0x226 + i, all in palette 0xa9), flower beds (three shapes, 0x1a3, 0x1a9 and 0x1af,
/// in the five colours 0x2d..0x31) and scenic trees (0x12f + i, palettes 0xb4 + i).
fn design_strip(kind: i32) -> Option<Strip> {
    let (dx, count, scale) = match kind {
        land::K_BENCH => (-84, 5, 1.0),
        land::K_LANDMARK => (0, 14, 0.5),
        land::K_BRIDGE => (-16, 8, 1.0),
        land::K_FLOWERS => (13, 15, 0.75),
        land::K_WILLOW => (100, 7, 0.5),
        _ => return None,
    };
    Some(Strip { dx, count, scale })
}

/// The exe sprite and palette of design i of a strip.
fn strip_sprite(kind: i32, i: i32) -> (u16, u8) {
    let n = i as u16;
    match kind {
        land::K_BENCH => (0x208 + n, 0xaa + i as u8),
        land::K_BRIDGE => (0x226 + n, 0xa9),
        land::K_FLOWERS => ([0x1a3, 0x1a9, 0x1af][(i / 5) as usize], 0x2d + (i % 5) as u8),
        land::K_WILLOW => (0x12f + n, 0xb4 + i as u8),
        _ => (0x168 + n, 100 + i as u8),
    }
}

impl Strip {
    fn left(&self) -> i32 {
        self.dx - self.count * 47 / 4 + 0x220
    }

    /// The anchor of design i.
    fn at(&self, i: i32) -> (i32, i32) {
        (self.left() + 0x2c + 47 * i / 2, 500 + if i & 1 != 0 { 27 } else { 0 })
    }

    /// The design under a point (the last one whose octagon, squashed to half height, is within 20), shown designs only.
    fn hit(&self, mask: u32, px: f32, py: f32) -> i32 {
        let mut found = -1;
        for i in (0..self.count).filter(|&i| mask == 0 || mask & (1 << i) != 0) {
            let (cx, cy) = self.at(i);
            if (Hit { cx, cy, r: 20, xs: 1, ys: 2 }).has(px, py) {
                found = i;
            }
        }
        found
    }
}

/// The strip's pieces on AmenitiesPanel (0x447461): the left end, the middle repeated, and the right end for an even or an
/// odd count; drawn along y 477.
const STRIP_LEFT: Cut = (380.0, 250.0, 68.0, 79.0);
const STRIP_MID: Cut = (450.0, 250.0, 47.0, 79.0);
const STRIP_END_EVEN: Cut = (600.0, 250.0, 67.0, 79.0);
const STRIP_END_ODD: Cut = (500.0, 250.0, 67.0, 79.0);

/// A design's button (0x44773d): x 300 when the design is under the pointer, 400 for the current design, 600 otherwise; the
/// even designs' row at y 350 (40 tall), the odd ones' at y 400 (41 tall), 47 wide, drawn 22 left of and 20 above the anchor.
fn strip_button(i: i32, state: i32) -> Cut {
    (300.0 + 100.0 * state as f32, if i & 1 != 0 { 400.0 } else { 350.0 }, 47.0, if i & 1 != 0 { 41.0 } else { 40.0 })
}

// ---- Add Buildings: the lots ---------------------------------------------------------------------------------------------------

/// Lot i places building kind 6 + i. The hover text (third line of the info box) is the exe's.
const LOT_EFFECT: [&str; 9] = [
    "Helps imaginative golfers",
    "Feeds hungry golfers",
    "Improves accurate golfers",
    "Golfers become members",
    "Helps long hitters",
    "Golfers play faster",
    "Increases property values",
    "Golfers stay happier",
    "Higher greens fees",
];

/// Hit region of lot i: an ellipse twice as wide as tall over its isometric slab.
fn lot_hit(i: usize) -> Hit {
    let (cx, cy) = if i < 5 { (383 + 80 * i as i32, 531) } else { (423 + 80 * (i as i32 - 5), 571) };
    Hit { cx, cy, r: 40, xs: 1, ys: 2 }
}
/// Where lot i's pad is drawn, and its hover and selected cuts (lots 5..8 use the second row of pads).
fn lot_pad(i: usize) -> (f32, f32, Cut, Cut) {
    if i < 5 {
        (343.0 + 80.0 * i as f32, 501.0, (0.0, 0.0, 74.0, 64.0), (100.0, 0.0, 77.0, 64.0))
    } else {
        (384.0 + 80.0 * (i - 5) as f32, 542.0, (0.0, 100.0, 77.0, 58.0), (100.0, 100.0, 77.0, 58.0))
    }
}
/// Lot i's picture on the theme's layout sheet (75 x 100 cells, column i): the exe loads a column's cuts in the order y 0, 200,
/// 100, 300 and draws number 2 * upgraded + buildable, that is grey, colour, grey upgraded, colour upgraded. It is drawn one
/// pixel higher while hovered and buildable.
fn lot_icon(i: usize, upgraded: bool, buildable: bool, lifted: bool) -> (Cut, f32, f32) {
    const ROW_Y: [f32; 4] = [0.0, 200.0, 100.0, 300.0];
    let row = ROW_Y[2 * upgraded as usize + buildable as usize];
    let (x, y) = if i < 5 { (344.0 + 80.0 * i as f32, 453.0) } else { (384.0 + 80.0 * (i - 5) as f32, 493.0) };
    ((75.0 * i as f32, row, 75.0, 100.0), x, y - lifted as i32 as f32)
}
const BUILDINGS_TO_ELEVATION: (Hit, Cut, f32, f32) = (hit(236, 526, 16), (150.0, 480.0, 34.0, 34.0), 220.0, 510.0);
/// Undo on the Buildings panel: hover and armed cuts at the same place.
const BUILDINGS_UNDO: (Hit, Cut, Cut, f32, f32) = (hit(277, 567, 12), (40.0, 550.0, 34.0, 40.0), (80.0, 550.0, 34.0, 40.0), 261.0, 553.0);

const MSG_UPGRADE_EARLY: &str = "Sorry, upgraded buildings are not available until you build beyond 9 holes.";
const MSG_MAX_LEVEL: &str = "You cannot upgrade this building any further.";
// the typo is the exe's
const MSG_LOCKED: &str = "This building is will become available as you build additional holes.";

// ---- Add Buildings: the elevation panel ----------------------------------------------------------------------------------------

/// Tools 0 vertex, 1 square, 2 area, 3 Analyze Golf Shot: tooltip, hit, where drawn, hover cut, selected cut.
const ELEVATION_TOOLS: [(&str, Hit, f32, f32, Cut, Cut); 4] = [
    ("Raise/lower vertex", hit(352, 547, 25), 312.0, 501.0, (0.0, 0.0, 88.0, 99.0), (100.0, 0.0, 88.0, 99.0)),
    ("Raise/lower square", hit(467, 547, 25), 429.0, 501.0, (0.0, 100.0, 88.0, 99.0), (100.0, 100.0, 88.0, 99.0)),
    ("Raise/lower area", hit(587, 547, 25), 546.0, 501.0, (0.0, 200.0, 88.0, 99.0), (100.0, 200.0, 88.0, 99.0)),
    ("Analyze Golf Shot", hit(715, 547, 16), 688.0, 501.0, (400.0, 0.0, 72.0, 99.0), (500.0, 0.0, 72.0, 99.0)),
];
/// "Buildings": back to the Buildings panel.
const ELEVATION_BACK: (Hit, Cut, f32, f32) = (hit(270, 499, 16), (0.0, 400.0, 41.0, 44.0), 254.0, 483.0);
const ELEVATION_UNDO: (Hit, Cut, Cut, f32, f32) = (hit(267, 561, 12), (0.0, 450.0, 34.0, 40.0), (50.0, 450.0, 34.0, 40.0), 253.0, 546.0);
/// The area tool's smoothing kernel over the 5 x 5 vertices around the edited one (the odd 3 in row 3 is the exe's).
const AREA_KERNEL: [[i32; 5]; 5] = [[6, 4, 3, 4, 6], [4, 2, 1, 2, 4], [3, 1, 0, 1, 3], [4, 2, 1, 2, 3], [6, 4, 3, 4, 6]];

// ---- People: the employee panel ------------------------------------------------------------------------------------------------

const MAX_EMPLOYEES: usize = 20;
const HIRE: (Hit, f32, f32) = (hit(270, 558, 16), 254.0, 542.0);
const HIRE_HOVER: Cut = (0.0, 500.0, 42.0, 42.0);
const HIRE_DISABLED: Cut = (100.0, 500.0, 42.0, 42.0);
/// Scroll arrows: narrow hit regions (xs 4); hover cut and the enabled look, drawn while scrolling that way is possible.
const SCROLL: [(Hit, f32, f32, Cut, Cut); 2] = [
    (Hit { cx: 321, cy: 549, r: 30, xs: 4, ys: 1 }, 313.0, 519.0, (0.0, 200.0, 16.0, 62.0), (100.0, 200.0, 16.0, 62.0)),
    (Hit { cx: 604, cy: 549, r: 30, xs: 4, ys: 1 }, 596.0, 519.0, (0.0, 300.0, 16.0, 62.0), (100.0, 300.0, 16.0, 62.0)),
];
/// Move, Fire and Rename for the selected employee: tooltip, hit, where drawn, row on the sheet. Their cuts are 52 x 48 at
/// x 400 hover, 500 armed (Move only), 600 disabled (nobody selected) and 700 idle.
const EMP_ACTIONS: [(&str, Hit, f32, f32, f32); 3] = [
    ("Move this employee", hit(647, 513, 16), 631.0, 497.0, 0.0),
    ("Fire this employee", hit(699, 510, 16), 683.0, 494.0, 50.0),
    ("Rename this employee", hit(756, 513, 16), 740.0, 497.0, 100.0),
];
/// The eight portraits, column major; each hits round (x + 32, y + 16).
const PORTRAITS: [(f32, f32); 8] =
    [(336.0, 507.0), (336.0, 550.0), (399.0, 507.0), (399.0, 550.0), (462.0, 507.0), (462.0, 550.0), (525.0, 507.0), (525.0, 550.0)];
/// The tabs out of the panel: Golfers (the golfers panel) and the player's pro.
const EMP_TAB_GOLFERS: (Hit, Cut, f32, f32) = (hit(286, 492, 16), (0.0, 400.0, 33.0, 33.0), 270.0, 475.0);
const EMP_TAB_PLAYER: (Hit, Cut, f32, f32) = (hit(256, 510, 16), (0.0, 450.0, 33.0, 33.0), 240.0, 494.0);
/// What the counter under a selected employee counts (regular, skilled).
pub(crate) const COUNTERS: [[&str; 2]; 4] = [
    ["Players greeted:", "Players cheered:"],
    ["Players rushed:", "Slackers intimidated:"],
    ["Weeds destroyed:", "Weeds eradicated:"],
    ["Beverages served:", "Satisfied customers:"],
];
/// The hire dialog: four kinds in bands 40 pixels tall, the upper half the regular hire, the lower the skilled one (the exe
/// tests only the pointer's y).
const HIRE_BAND_Y: [f32; 4] = [171.0, 242.0, 313.0, 384.0];
const HIRE_BLURB: [&str; 4] = ["Greeters...", "Speed up play...", "Weed Killers...", "Thirst quenchers..."];
const SKILLED_REFUSAL: &str = "You need to build up to a Daily Fee course (6 or more holes) before you can hire skilled employees.";

fn hire_choice_at(px: f32, py: f32) -> Option<usize> {
    if !(224.0..523.0).contains(&px) {
        return None;
    }
    HIRE_BAND_Y.iter().position(|&y| py >= y && py < y + 40.0).map(|k| 2 * k + (py > HIRE_BAND_Y[k] + 19.0) as usize)
}

// ---- art and state -------------------------------------------------------------------------------------------------------------

/// The panel sheets, from the game folder. Per-theme sheets are kept for all four themes, in the exe's theme order.
#[derive(Default)]
pub struct PanelArt {
    pub terrain: Image,
    pub terrain_buttons: [Image; 4],
    pub amenities: Image,
    pub buildings: Image,
    pub layouts: [Image; 4],
    pub elevation: Image,
    pub employees: Image,
    /// The Player panel (crate::player_panel).
    pub player: Image,
    pub hire: Image,
    pub frame: Image,
}

impl PanelArt {
    pub fn load(g: &mut Gfx, app: &App) -> PanelArt {
        let p = |rel: &str| app.game_path(&format!("Interface/{rel}"));
        let alpha = |g: &mut Gfx, rel: &str| load_pcx_alpha(g, &p(&format!("{rel}.pcx")), &p(&format!("{rel}_A.pcx"))).unwrap_or_default();
        // the button and lot sheets have no alpha sheet: magenta is clear (the lot sheets use 248,0,248)
        let keyed = |g: &mut Gfx, rel: &str| load_pcx(g, &p(rel), true, Some(0xF800F8)).unwrap_or_default();
        PanelArt {
            terrain: alpha(g, "BaseTerrainPanel"),
            terrain_buttons: [
                keyed(g, "ParkLandTerrainButtons.pcx"),
                keyed(g, "DesertTerrainButtons.pcx"),
                keyed(g, "TropicalTerrainButtons.pcx"),
                keyed(g, "LinksTerrainButtons.pcx"),
            ],
            amenities: alpha(g, "AmenitiesPanel"),
            buildings: alpha(g, "BuildingPanel"),
            layouts: [
                keyed(g, "parklands layout.pcx"),
                keyed(g, "desert layout.pcx"),
                keyed(g, "trop layout.pcx"),
                keyed(g, "links layout.pcx"),
            ],
            elevation: alpha(g, "ElevationPanel"),
            employees: alpha(g, "EmployeePanel"),
            player: alpha(g, "JoeCoolPanel"),
            hire: load_pcx_alpha(g, &p("infoscreens/hire.pcx"), &p("infoscreens/hire_alpha.pcx")).unwrap_or_default(),
            frame: alpha(g, "Pop_UpOk"),
        }
    }

    /// All panel bodies are on the disc (otherwise the dock falls back to its text lists).
    pub fn ready(&self) -> bool {
        [&self.terrain, &self.amenities, &self.buildings, &self.elevation, &self.employees].iter().all(|i| i.tex.is_some())
    }
}

/// What the panels remember between frames.
pub struct PanelState {
    /// The other panel of a dock mode is shown: Amenities instead of the terrain panel, Elevation instead of Buildings.
    pub alt: bool,
    /// The Elevation panel's tool: 0 vertex, 1 square, 2 area, 3 Analyze Golf Shot.
    pub elev_tool: usize,
    /// The selected employee (index into `employees`), the roster's scroll (a multiple of 2) and the hire dialog.
    pub emp_sel: Option<usize>,
    pub emp_off: usize,
    pub hire_open: bool,
    /// Pointer in 800 x 600 units, the hit under it and for how many frames it has been there.
    pub mouse: (f32, f32),
    pub hot: i32,
    pub hot_frames: u32,
    /// A scripted still: tooltips show at once.
    pub still: bool,
}

impl Default for PanelState {
    fn default() -> Self {
        PanelState {
            alt: false,
            elev_tool: 0,
            emp_sel: None,
            emp_off: 0,
            hire_open: false,
            mouse: (-1.0, -1.0),
            hot: -1,
            hot_frames: 0,
            still: false,
        }
    }
}

/// The exe's tooltip bar (0x432620): half transparent black behind white text, centred on the pointer a few pixels up and
/// kept on the screen.
pub(crate) fn tip_bar(g: &mut Gfx, s: &Ui, text: &str, px: f32, py: f32) {
    let w = text_width(text, 13.0) + 12.0;
    let cx = px.clamp(w * 0.5, 800.0 - w * 0.5);
    s.fill(g, cx - w * 0.5, py - 22.0, w, 17.0, rgba(0.0, 0.0, 0.0, 0.5));
    s.text_centered(g, cx, py - 9.0, text, 13.0, rgb(1.0, 1.0, 1.0));
}

/// The terrain and amenity tooltip: a box 160 wide over the slot at y 402 with the name and the price. The exe's box is 112
/// tall with a strip of translucent pieces along its top; here it is a plain translucent box sized to its two lines
/// (placeholder look).
fn tip_box(g: &mut Gfx, s: &Ui, slot_x: f32, name: &str, price: &str) {
    let x = slot_x.clamp(80.0, 720.0) - 80.0;
    s.fill(g, x, 402.0, 160.0, 42.0, rgba(0.0, 0.0, 0.0, 0.55));
    s.text_centered(g, x + 80.0, 419.0, name, 14.0, rgb(1.0, 1.0, 1.0));
    s.text_centered(g, x + 80.0, 437.0, price, 13.0, rgb(1.0, 0.95, 0.6));
}

impl App {
    pub fn panel_art_ready(&self) -> bool {
        self.panel_art.ready()
    }

    /// The open dock panel is drawn from its art (1..3 and the Player panel, 5); otherwise the dock shows its text list.
    pub fn art_panel_open(&self) -> bool {
        match self.panel {
            1..=3 => self.panel_art_ready(),
            5 => self.player_panel_ready(),
            _ => false,
        }
    }

    /// Opens a dock panel from its dock button (1 Build Course, 2 Add Buildings, 3 People); pressing the button of the open one
    /// closes it (the golfers and pro panels count as People).
    pub fn open_panel(&mut self, want: i32) {
        let family = if self.panel >= 3 { 3 } else { self.panel };
        self.pstate.hire_open = false;
        self.pstate.alt = false;
        if family == want {
            self.panel = 0;
            self.edit = false;
            return;
        }
        self.panel = want;
        self.edit = want == 1 || want == 2;
        match want {
            1 => self.tool = 0,
            2 if self.panel_art_ready() => {
                // a lot stays armed only if it still can be built; else the map is free until a lot is clicked
                let k = self.build_idx as i32;
                self.edit = self.tool == 4 && (6..=14).contains(&k) && self.lot_buildable((k - 6) as usize);
            }
            2 => {
                self.tool = 4;
                if !self.build_available(self.build_idx) {
                    self.build_idx = OFFERED_KINDS[0] as usize;
                }
            }
            _ => {}
        }
    }

    /// The level the next building of a kind would have: 0 when none stands, 1 for an upgrade, 2 when it is upgraded already.
    pub(crate) fn lot_level(&self, kind: i32) -> i32 {
        self.land.as_ref().and_then(|l| l.objects.iter().find(|o| o.kind == kind)).map(|o| o.sub + 1).unwrap_or(0)
    }

    /// Whether lot i is buildable now: unlocked, not upgraded twice, and upgrades only beyond 9 holes.
    fn lot_buildable(&self, i: usize) -> bool {
        let kind = 6 + i as i32;
        let level = self.lot_level(kind);
        self.build_available(kind as usize)
            && kind + level < self.unlocked()
            && level < 2
            && (level == 0 || economy::rank(self.holes.len()) >= 2)
    }

    /// The hired employees (indices into `employees`) in hiring order.
    fn hired(&self) -> Vec<usize> {
        self.employees.iter().enumerate().filter(|(_, e)| e.active && (-5..=-2).contains(&e.job)).map(|(i, _)| i).collect()
    }

    /// The Tees button looks disabled while the hole being built has its tee, and once eighteen holes are built.
    fn tee_disabled(&self) -> bool {
        let h = self.club.next_hole;
        if !(0..19).contains(&h) {
            return true;
        }
        let Some(rec) = self.club.holes.get(h as usize) else { return false };
        let two = self.club.game & sg_core::golfer::game::TWO_TEES != 0;
        rec.back.0 != 0 && !(two && rec.fwd.0 == 0)
    }

    fn paint_index_of(id: i32) -> Option<usize> {
        PAINT.iter().position(|p| p.ty == id && p.vbyte == 0)
    }

    /// The hit under the pointer in the open panel (see the module notes for the codes).
    pub fn panel_hit(&self, px: f32, py: f32) -> i32 {
        let theme = self.exe_theme() as usize;
        match (self.panel, self.pstate.alt) {
            (1, false) => {
                if let Some(i) = terrain_slot_at(theme, px, py) {
                    return i as i32;
                }
                if TERRAIN_TO_AMENITIES.0.has(px, py) {
                    return -2;
                }
                if TERRAIN_UNDO.0.has(px, py) {
                    return -3;
                }
                -1
            }
            (1, true) => {
                // a later test overrides an earlier one in the exe: the bridge, Undo, then the rest from the right
                if AMENITIES[9].hit.has(px, py) {
                    return 9;
                }
                if let Some(i) = (0..8).rev().find(|&i| AMENITIES[i].hit.has(px, py)) {
                    return i as i32;
                }
                if AMENITIES_BACK.0.has(px, py) {
                    return -2;
                }
                -1
            }
            (2, false) => {
                if BUILDINGS_TO_ELEVATION.0.has(px, py) {
                    return -2;
                }
                if BUILDINGS_UNDO.0.has(px, py) {
                    return -3;
                }
                (0..9).find(|&i| lot_hit(i).has(px, py)).map(|i| i as i32).unwrap_or(-1)
            }
            (2, true) => {
                if ELEVATION_UNDO.0.has(px, py) {
                    return -3;
                }
                if let Some(i) = (0..4).rev().find(|&i| ELEVATION_TOOLS[i].1.has(px, py)) {
                    return i as i32;
                }
                if ELEVATION_BACK.0.has(px, py) {
                    return -2;
                }
                -1
            }
            (3, _) => {
                // the first matching slot wins, and slots win over the tabs
                if HIRE.0.has(px, py) {
                    return 0;
                }
                if let Some(k) = (0..2).find(|&k| SCROLL[k].0.has(px, py)) {
                    return 2 + k as i32;
                }
                if let Some(a) = (0..3).find(|&a| EMP_ACTIONS[a].1.has(px, py)) {
                    return 4 + a as i32;
                }
                if let Some(p) = (0..8).find(|&p| hit(PORTRAITS[p].0 as i32 + 32, PORTRAITS[p].1 as i32 + 16, 16).has(px, py)) {
                    return 8 + p as i32;
                }
                if EMP_TAB_PLAYER.0.has(px, py) {
                    return -3;
                }
                if EMP_TAB_GOLFERS.0.has(px, py) {
                    return -2;
                }
                -1
            }
            (5, _) => self.player_hit(px, py),
            _ => -1,
        }
    }

    /// Draws the open panel (1..3, 5) from its sheet, with the hover and selected looks and the tooltip.
    pub fn draw_panel(&mut self, g: &mut Gfx, s: &Ui) {
        let (mx, my) = self.pstate.mouse;
        let h = if self.pstate.hire_open { -1 } else { self.panel_hit(mx, my) };
        if h != self.pstate.hot {
            self.pstate.hot = h;
            self.pstate.hot_frames = 0;
        } else {
            self.pstate.hot_frames = self.pstate.hot_frames.saturating_add(1);
        }
        let tip = h != -1 && (self.pstate.hot_frames > TIP_DELAY_FRAMES || self.pstate.still);
        match (self.panel, self.pstate.alt) {
            (1, false) => self.draw_terrain_panel(g, s, h, tip),
            (1, true) => self.draw_amenities_panel(g, s, h, tip),
            (2, false) => self.draw_buildings_panel(g, s, h, tip),
            (2, true) => self.draw_elevation_panel(g, s, h, tip),
            (3, _) => self.draw_employee_panel(g, s, h, tip),
            (5, _) => self.draw_player_panel(g, s, h, tip),
            _ => {}
        }
    }

    fn draw_terrain_panel(&self, g: &mut Gfx, s: &Ui, h: i32, tip: bool) {
        let art = &self.panel_art;
        let theme = self.exe_theme() as usize;
        let buttons = &art.terrain_buttons[theme];
        blit(g, s, &art.terrain, TERRAIN_BODY, TERRAIN_BODY.0, TERRAIN_BODY.1);
        let current = (self.tool == 0).then(|| PAINT[self.paint_idx].ty);
        for i in 0..16 {
            let r = terrain_slot_rect(i, theme);
            let id = tile_id(i);
            let state = if current == Some(id) {
                2
            } else if h == i as i32 {
                1
            } else if i == 0 && self.tee_disabled() {
                3
            } else {
                0
            };
            if i < 13 {
                let sx = if i > 6 { 248.0 } else { 0.0 } + state as f32 * 62.0;
                blit(g, s, buttons, (sx, (i % 7) as f32 * 54.0, 62.0, 54.0), r.x, r.y);
            } else {
                // trees have no disabled look
                let state = state.min(2) as f32;
                blit(g, s, buttons, (520.0 + state * 80.0, [0.0, 150.0, 300.0][i - 13], r.w, r.h), r.x, r.y);
            }
        }
        // The exe also animates a hint over the Green button while the hole has no green yet (sprite 0x189 + theme); its file is
        // not mapped, so it is left out.
        for (code, (_, cut, x, y)) in [(-2, TERRAIN_TO_AMENITIES), (-3, TERRAIN_UNDO)] {
            if h == code {
                blit(g, s, &art.terrain, cut, x, y);
            }
        }
        if !tip {
            return;
        }
        let (mx, my) = self.pstate.mouse;
        match h {
            -2 => tip_bar(g, s, "Amenities", mx, my),
            -3 => tip_bar(g, s, "Undo", mx, my),
            i if i >= 0 => {
                let id = tile_id(i as usize);
                let r = terrain_slot_rect(i as usize, theme);
                let price = format!("Cost per tile: {}", money(Economy::terrain_cost_units(id) as i64 * 100));
                tip_box(g, s, r.x + 32.0, tile_name(id, theme), &price);
            }
            _ => {}
        }
    }

    /// The amenity a slot shows as selected for the current tool.
    fn amenity_selected(&self, slot: usize) -> bool {
        match slot {
            1 => self.tool == 2,
            7 => self.tool == 5,
            8 => false,
            _ => self.tool == 4 && self.build_idx as i32 == AMENITIES[slot].kind,
        }
    }

    /// A Home Site needs a free lot (one per Silver or better member); a Landmark needs one the club owns (the exe only greys
    /// the Home Site; greying the Landmark when there is none to place is the port's).
    fn amenity_disabled(&self, slot: usize) -> bool {
        matches!(slot, 2 | 4) && !self.build_available(AMENITIES[slot].kind as usize)
    }

    /// The landmark the next placement uses and its price in units (0: free), as edit_building charges it: the chosen one,
    /// else (the port's, for the slot's tooltip before a choice) the first owned type still free to place, else the first
    /// owned one.
    fn landmark_price(&self) -> Option<i32> {
        let t = self.chosen_landmark().or_else(|| {
            (0..14)
                .find(|t| self.club.free_landmarks & (1 << t) != 0)
                .or_else(|| (0..14).find(|t| self.club.landmarks_owned & (1 << t) != 0))
        })?;
        Some(if self.club.free_landmarks & (1 << t) != 0 { 0 } else { (t * 5 + 25) * 2 })
    }

    /// The strip of the armed garden item, if it has one: its kind, layout and the designs shown (a mask, 0 for all).
    fn open_strip(&self) -> Option<(i32, Strip, u32)> {
        if self.tool != 4 {
            return None;
        }
        let kind = self.build_idx as i32;
        let st = design_strip(kind)?;
        let mask = if kind == land::K_LANDMARK { self.club.landmarks_owned & 0x3fff } else { 0 };
        // the landmark strip only opens once the club owns one
        (kind != land::K_LANDMARK || mask != 0).then_some((kind, st, mask))
    }

    /// The strip of designs (0x432200), drawn with the panel every frame; the design under the pointer becomes the one a
    /// click in the panel picks. Over a landmark, two tooltip bars give its name with its price in dollars (or "FREE!") and
    /// what it does (by type & 3: Happy Golfers, No Dandelions, Skill Upgrade, Happy Endings).
    fn draw_design_strip(&mut self, g: &mut Gfx, s: &Ui) {
        let Some((kind, st, mask)) = self.open_strip() else { return };
        let art = self.panel_art.amenities;
        let mut x = st.left() as f32;
        for k in 0..st.count / 2 {
            let c = if k == 0 { STRIP_LEFT } else { STRIP_MID };
            blit(g, s, &art, c, x, 477.0);
            x += c.2;
        }
        blit(g, s, &art, if st.count & 1 != 0 { STRIP_END_ODD } else { STRIP_END_EVEN }, x, 477.0);
        let (mx, my) = self.pstate.mouse;
        let hot = st.hit(mask, mx, my);
        let current = self.design.filter(|d| d.0 == kind).map(|d| d.1).unwrap_or(-1);
        for i in (0..st.count).filter(|&i| mask == 0 || mask & (1 << i) != 0) {
            let (cx, cy) = st.at(i);
            let state = if current == i {
                1
            } else if hot == i {
                0
            } else {
                3
            };
            blit(g, s, &art, strip_button(i, state), (cx - 22) as f32, (cy - 20) as f32);
            let si = if kind == land::K_LANDMARK {
                // in the landmark's palette, 100 + type
                let pal = sg_core::decor::palette_file(strip_sprite(kind, i).1, self.exe_theme());
                sg_core::objects::LANDMARKS.get(i as usize).and_then(|f| self.sprite_for(&format!("{f}.flc"), false, pal))
            } else {
                let (id, pal) = strip_sprite(kind, i);
                self.decor_sprite(id, pal).0
            };
            let Some(si) = si else { continue };
            let sp = &self.sprites[si].s;
            let f = sp.frames_per_view - 1;
            let (w, h, ax, ay) = (sp.w as f32, sp.h as f32, sp.anchor_x as f32, sp.anchor_y as f32);
            let tex = self.sprite_texture(g, si, 0, f);
            let k = st.scale;
            let im = Image { tex: Some(tex), w, h };
            s.image_scaled(g, &im, (cx as f32 - ax * k, cy as f32 - ay * k, w * k, h * k), (0.0, 0.0, w, h));
        }
        if hot == -1 {
            return;
        }
        self.design_hover = hot;
        if kind == land::K_LANDMARK {
            let name = sg_core::vips::landmark_short_name(hot);
            let mut ch = name.chars();
            let name = ch.next().map(|f| f.to_uppercase().collect::<String>() + ch.as_str()).unwrap_or_default();
            let price = if self.club.free_landmarks & (1 << hot) != 0 { "FREE!".to_string() } else { ((hot * 5 + 25) * 200).to_string() };
            // the exe's bars at the pointer less 12 and less 2 (tip_bar's point is the exe's plus 5)
            tip_bar(g, s, &format!("{name}, {price}"), mx, my - 7.0);
            let effect = ["Happy Golfers", "No Dandelions", "Skill Upgrade", "Happy Endings"][(hot & 3) as usize];
            tip_bar(g, s, effect, mx, my + 3.0);
        }
    }

    fn draw_amenities_panel(&mut self, g: &mut Gfx, s: &Ui, h: i32, tip: bool) {
        let art = &self.panel_art;
        blit(g, s, &art.amenities, AMENITIES_BODY, AMENITIES_BODY.0, AMENITIES_BODY.1);
        if h == -2 {
            blit(g, s, &art.amenities, AMENITIES_BACK.1, AMENITIES_BACK.2, AMENITIES_BACK.3);
        }
        for (i, sl) in AMENITIES.iter().enumerate().filter(|&(i, _)| i != 8) {
            if self.amenity_disabled(i) {
                blit(g, s, &art.amenities, shifted(sl.hover, 200.0), sl.x, sl.y);
            } else if self.amenity_selected(i) {
                blit(g, s, &art.amenities, shifted(sl.hover, 100.0), sl.x, sl.y);
            } else if h == i as i32 {
                blit(g, s, &art.amenities, sl.hover, sl.x, sl.y);
            }
        }
        self.draw_design_strip(g, s);
        if !tip {
            return;
        }
        let (mx, my) = self.pstate.mouse;
        match h {
            -2 => tip_bar(g, s, "Course Terrain", mx, my),
            7 => tip_bar(g, s, "Undo", mx, my),
            i if i >= 0 => {
                let sl = &AMENITIES[i as usize];
                let units = |k: i32| land::BUILDINGS[k as usize].2 as i64 * 100;
                let price = match sl.kind {
                    // the exe has one Pathway; gravel and paved are the port's (clicking Pathway again switches)
                    0 => format!("{}: {} per tile", if self.path_kind == 2 { "Paved" } else { "Gravel" }, money(units(0))),
                    4 => match self.landmark_price() {
                        Some(0) => "FREE!".to_string(),
                        Some(p) => format!("Cost: {}", money(p as i64 * 100)),
                        None => "None to place yet".to_string(),
                    },
                    5 if self.amenity_disabled(2) => "No free lots".to_string(),
                    k => format!("Cost: {}", money(units(k))),
                };
                tip_box(g, s, sl.hit.cx as f32, sl.tip, &price);
            }
            _ => {}
        }
    }

    fn draw_buildings_panel(&self, g: &mut Gfx, s: &Ui, h: i32, tip: bool) {
        let art = &self.panel_art;
        let theme = self.exe_theme() as usize;
        blit(g, s, &art.buildings, BUILDINGS_BODY, BUILDINGS_BODY.0, BUILDINGS_BODY.1);
        if h == -2 {
            blit(g, s, &art.buildings, BUILDINGS_TO_ELEVATION.1, BUILDINGS_TO_ELEVATION.2, BUILDINGS_TO_ELEVATION.3);
        }
        let (_, hover, armed, ux, uy) = BUILDINGS_UNDO;
        if self.tool == 5 {
            blit(g, s, &art.buildings, armed, ux, uy);
        } else if h == -3 {
            blit(g, s, &art.buildings, hover, ux, uy);
        }
        let can: Vec<bool> = (0..9).map(|i| self.lot_buildable(i)).collect();
        for i in 0..9 {
            let (x, y, hover, selected) = lot_pad(i);
            if can[i] && h == i as i32 {
                blit(g, s, &art.buildings, hover, x, y);
            }
            if self.tool == 4 && self.build_idx == 6 + i {
                blit(g, s, &art.buildings, selected, x, y);
            }
        }
        for (i, &ok) in can.iter().enumerate() {
            let (c, x, y) = lot_icon(i, self.lot_level(6 + i as i32) != 0, ok, ok && h == i as i32);
            blit(g, s, &art.layouts[theme], c, x, y);
        }
        if !tip {
            return;
        }
        let (mx, my) = self.pstate.mouse;
        match h {
            -2 => tip_bar(g, s, "Elevation", mx, my),
            -3 => tip_bar(g, s, "Undo", mx, my),
            i if i >= 0 => {
                // the info box: the frame from Pop_UpOk over a translucent fill, three centred lines
                let i = i as usize;
                let level = self.lot_level(6 + i as i32);
                let cx = ((i % 5) as f32 * 80.0 + if i > 4 { 431.0 } else { 391.0 }).clamp(80.0, 720.0);
                self.popup_frame(g, s, cx - 80.0, 460.0, 160.0, 100.0);
                // the Snack Bar never says "Upgraded" nor shows the upgrade price
                let up = level != 0 && i != 1;
                let name = land::BUILDINGS[6 + i].0;
                let units = land::BUILDINGS[6 + i].2 as i64;
                let dollars = if up { units * 300 / 2 } else { units * 100 };
                let ink = rgb(0.12, 0.1, 0.3);
                let title = if up { format!("Upgraded {name}") } else { name.to_string() };
                // the exe's y positions are the tops of the lines; ours are baselines
                s.text_centered(g, cx - 5.0, 474.0, &title, 12.0, ink);
                s.text_centered(g, cx - 5.0, 487.0, &format!("Cost: {}", money(dollars)), 12.0, ink);
                s.text_centered(g, cx - 5.0, 500.0, LOT_EFFECT[i], 11.0, rgb(0.3, 0.15, 0.45));
            }
            _ => {}
        }
    }

    /// The exe's pop-up frame (0x40d0b0): corner and edge pieces from the theme's row of Pop_UpOk around a translucent cream
    /// fill. The pieces are 16 pixels with one pixel gutters; the right column and bottom row are 26 wide or tall (they carry
    /// the shadow). The exe repeats each edge piece every 16 pixels; the edge pieces are uniform along their length, so here
    /// each edge is one stretched piece, and every piece is sampled half a pixel inside its gutters so the scaled screen shows
    /// no seams. The fill is more opaque than the exe's half (0x80) so the larger text here stays readable over the lots.
    fn popup_frame(&self, g: &mut Gfx, s: &Ui, x: f32, y: f32, w: f32, h: f32) {
        let im = &self.panel_art.frame;
        s.fill(g, x + 6.0, y + 6.0, w - 18.0, h - 18.0, rgba(1.0, 0.97, 0.9, 0.8));
        let oy = match self.exe_theme() {
            1 => 200.0,
            2 => 100.0,
            _ => 0.0,
        };
        let (r, b) = (x + w - 26.0, y + h - 26.0);
        // (source x, width) of the three columns and (source y, height) of the three rows, inset from the gutters
        let cols = [(0.0, 15.5), (17.5, 15.0), (34.5, 25.5)];
        let rows = [(oy, 15.5), (oy + 17.5, 15.0), (oy + 34.5, 25.5)];
        let dcols = [(x, 16.0), (x + 16.0, r - x - 16.0), (r, 26.0)];
        let drows = [(y, 16.0), (y + 16.0, b - y - 16.0), (b, 26.0)];
        for (j, &(sy, sh)) in rows.iter().enumerate() {
            for (i, &(sx, sw)) in cols.iter().enumerate() {
                if i == 1 && j == 1 {
                    continue;
                }
                let ((dx, dw), (dy, dh)) = (dcols[i], drows[j]);
                s.image_scaled(g, im, (dx, dy, dw, dh), (sx, sy, sw, sh));
            }
        }
    }

    fn draw_elevation_panel(&self, g: &mut Gfx, s: &Ui, h: i32, tip: bool) {
        let im = &self.panel_art.elevation;
        blit(g, s, im, ELEVATION_BODY, ELEVATION_BODY.0, ELEVATION_BODY.1);
        if h == -2 {
            blit(g, s, im, ELEVATION_BACK.1, ELEVATION_BACK.2, ELEVATION_BACK.3);
        }
        let (_, hover, armed, ux, uy) = ELEVATION_UNDO;
        if self.tool == 5 {
            blit(g, s, im, armed, ux, uy);
        }
        let cur = self.pstate.elev_tool;
        if h == -3 {
            blit(g, s, im, hover, ux, uy);
        } else if h >= 0 && h as usize != cur {
            let (_, _, x, y, hv, _) = ELEVATION_TOOLS[h as usize];
            blit(g, s, im, hv, x, y);
        }
        let (_, _, x, y, _, sel) = ELEVATION_TOOLS[cur];
        blit(g, s, im, sel, x, y);
        if tip {
            let (mx, my) = self.pstate.mouse;
            let t = match h {
                -2 => "Buildings",
                -3 => "Undo",
                i => ELEVATION_TOOLS[i as usize].0,
            };
            tip_bar(g, s, t, mx, my);
        }
    }

    fn draw_employee_panel(&mut self, g: &mut Gfx, s: &Ui, h: i32, tip: bool) {
        let rows = self.hired();
        let n = rows.len();
        if self.pstate.emp_sel.is_some_and(|i| !rows.contains(&i)) {
            self.pstate.emp_sel = None;
        }
        while self.pstate.emp_off > 0 && self.pstate.emp_off + 7 > n {
            self.pstate.emp_off -= 2;
        }
        let off = self.pstate.emp_off;
        let sel = self.pstate.emp_sel;
        {
            let im = &self.panel_art.employees;
            blit(g, s, im, EMPLOYEE_BODY, EMPLOYEE_BODY.0, EMPLOYEE_BODY.1);
            let can_scroll = [off > 0, n > off + 8];
            for (k, &(_, x, y, hover, enabled)) in SCROLL.iter().enumerate() {
                if can_scroll[k] {
                    blit(g, s, im, enabled, x, y);
                    if h == 2 + k as i32 {
                        blit(g, s, im, hover, x, y);
                    }
                }
            }
            if n >= MAX_EMPLOYEES {
                blit(g, s, im, HIRE_DISABLED, HIRE.1, HIRE.2);
            } else if h == 0 {
                blit(g, s, im, HIRE_HOVER, HIRE.1, HIRE.2);
            }
            for (a, &(_, _, x, y, row)) in EMP_ACTIONS.iter().enumerate() {
                let state = if sel.is_none() { 2.0 } else { 3.0 };
                blit(g, s, im, (400.0 + state * 100.0, row, 52.0, 48.0), x, y);
                if sel.is_some() && h == 4 + a as i32 {
                    blit(g, s, im, (400.0, row, 52.0, 48.0), x, y);
                }
                if a == 0 && sel.is_some() && self.moving_employee == sel {
                    blit(g, s, im, (500.0, row, 52.0, 48.0), x, y);
                }
            }
            for (code, (_, cut, x, y)) in [(-2, EMP_TAB_GOLFERS), (-3, EMP_TAB_PLAYER)] {
                if h == code {
                    blit(g, s, im, cut, x, y);
                }
            }
            for (p, &(x, y)) in PORTRAITS.iter().enumerate() {
                let Some(&ei) = rows.get(off + p) else { break };
                let yc = 50.0 * (p & 1) as f32;
                if sel == Some(ei) {
                    blit(g, s, im, (100.0, yc, 63.0, 43.0), x, y);
                } else if h == 8 + p as i32 {
                    blit(g, s, im, (0.0, yc, 63.0, 43.0), x, y);
                }
            }
        }
        // each portrait shows the employee's current clip, frame and facing (0x436e50), queued at full size with its ground
        // point at the slot's (30, 24)
        let mut queue = Vec::new();
        for (p, &(x, y)) in PORTRAITS.iter().enumerate() {
            let Some(&ei) = rows.get(off + p) else { break };
            let e = self.employees[ei];
            if let Some((si, view, f)) = self.staff_figure(&e) {
                queue.push((y + 24.0, si, view, f, x + 30.0, y + 24.0, 1.0));
            }
        }
        self.draw_queued(g, s, &mut queue);
        if let Some(ei) = sel {
            let e = self.employees[ei];
            let kind = (-2 - e.job as i32).clamp(0, 3) as usize;
            let up = e.upgraded as usize;
            let ink = crate::screens_ui::c15(0);
            // centred at x 700, black, placed by their tops: the name (in the face the frame last used, taken to be the body
            // face 0x51b360, derived), "Hired: <Month> <Year>", "Paid: <dollars>" (grouped, no currency sign) and the counter
            // label in Arial Bold 10 (0x519fd8), the count in the body face
            let month = ["March", "April", "May", "June", "July", "August", "September", "October"][(e.hired & 7) as usize];
            let paid = e.paid as i64 * 100;
            let lines = [
                (534.0, self.employee_name(&e), crate::ui::Face::Manual, 15.0),
                (546.0, format!("Hired: {month} {}", 2001 + (e.hired >> 3)), crate::ui::Face::Arial, 10.0),
                (
                    558.0,
                    format!("Paid: {}{}", if paid < 0 { "-" } else { "" }, crate::ui::group(paid.unsigned_abs())),
                    crate::ui::Face::Arial,
                    10.0,
                ),
                (572.0, COUNTERS[kind][up].to_string(), crate::ui::Face::Arial, 10.0),
                (584.0, format!("{}", e.served), crate::ui::Face::Manual, 15.0),
            ];
            for (y, t, face, size) in lines {
                crate::ui::set_face(Some(face));
                s.text_centered(g, 700.0, crate::screens_ui::top(y, size), &t, size, ink);
            }
            crate::ui::set_face(None);
        }
        if tip {
            let (mx, my) = self.pstate.mouse;
            let pro = self.pro_name();
            let t = match h {
                0 => "Hire employees",
                4..=6 => EMP_ACTIONS[(h - 4) as usize].0,
                -2 => "Golfers",
                -3 => pro.as_str(),
                _ => "",
            };
            if !t.is_empty() {
                tip_bar(g, s, t, mx, my);
            }
        }
    }

    /// An employee's walking clip (sprite 0x20e + set, in the set's palette 0x82 + set) in a view, its ground point at
    /// (ax, ay). The exe steps the frame once per redraw of its modal loop; APPROXIMATION: the port runs it at the clip's
    /// own frame time.
    pub(crate) fn draw_walking(&mut self, g: &mut Gfx, s: &Ui, set: usize, view: i32, ax: f32, ay: f32) {
        let Some(si) = self.staff_clips.get(set).and_then(|c| c[0].0) else { return };
        let sp = &self.sprites[si].s;
        let n = sp.frames_per_view.max(1);
        let f = if sp.frame_ms > 0 { (self.clock * 1000.0 / sp.frame_ms as f64) as i64 % n as i64 } else { 0 } as i32;
        let view = view.rem_euclid(sp.views.max(1));
        let (w, h) = {
            let fr = &sp.frames[sp.frame_index(view, f)];
            (fr.w as f32, fr.h as f32)
        };
        let (axx, ayy) = (sp.anchor_x as f32, sp.anchor_y as f32);
        let tex = self.sprite_texture(g, si, view, f);
        s.image(g, &Image { tex: Some(tex), w, h }, ax - axx, ay - ayy);
    }

    /// The modal hire dialog (sheet infoscreens/hire), drawn over everything.
    pub fn draw_hire_dialog(&mut self, g: &mut Gfx, s: &Ui) {
        s.fill(g, 0.0, 0.0, 800.0, 600.0, rgba(0.0, 0.0, 0.0, 0.35));
        let im = self.panel_art.hire;
        s.image(g, &im, 0.0, 0.0);
        let ink = rgb(0.15, 0.12, 0.35);
        s.text_centered(g, 377.0, 123.0, "HIRE AN EMPLOYEE", 17.0, ink);
        let (mx, my) = self.pstate.mouse;
        let hot = hire_choice_at(mx, my);
        let skilled_ok = economy::rank(self.holes.len()) >= 1;
        for k in 0..4 {
            let y = HIRE_BAND_Y[k];
            s.text(g, 232.0, y - 9.0, HIRE_BLURB[k], 12.0, ink);
            for sk in 0..2 {
                let c = 2 * k + sk;
                let ty = y + sk as f32 * 20.0;
                if hot == Some(c) {
                    s.fill(g, 226.0, ty + 1.0, 296.0, 18.0, rgba(1.0, 0.85, 0.4, 0.55));
                }
                let text = format!("{}: {} per week", STAFF_NAMES[k][sk], money(WAGE_UNITS[k][sk] as i64 * 100));
                let c = if sk == 1 && !skilled_ok { rgb(0.5, 0.48, 0.55) } else { ink };
                s.text(g, 246.0, ty + 15.0, &text, 14.0, c);
            }
            // 0x459400: the box beside each kind shows it walking, the skilled version while its line is hovered, in view k,
            // at (0x240, 0xc0 + 0x46 k)
            let skilled = hot.is_some_and(|c| c == 2 * k + 1);
            self.draw_walking(g, s, k + 4 * skilled as usize, k as i32, 576.0, (0xc0 + 0x46 * k) as f32);
        }
    }

    /// A click on the open panel (1..3, 5). Returns true when the panel took it.
    pub fn panel_click(&mut self, px: f32, py: f32) -> bool {
        if self.pstate.hire_open {
            self.hire_click(px, py);
            return true;
        }
        let h = self.panel_hit(px, py);
        let took = match (self.panel, self.pstate.alt) {
            (1, false) => self.terrain_click(h),
            (1, true) => self.amenities_click(h),
            (2, false) => self.buildings_click(h),
            (2, true) => self.elevation_click(h),
            (3, _) => self.employee_click(h),
            (5, _) => self.player_click(h),
            _ => false,
        };
        if took {
            self.snd("Interface/Button2.wav", 1.0, false);
            return true;
        }
        // the panel's own area swallows the rest
        let body = match self.panel {
            3 => EMPLOYEE_BODY,
            5 => crate::player_panel::PLAYER_BODY,
            _ => AMENITIES_BODY,
        };
        Rect::new(body.0, body.1 - 8.0, body.2, body.3 + 8.0).has(px, py)
    }

    fn arm_undo(&mut self) {
        self.tool = 5;
        self.edit = true;
        self.show_toast(UNDO_HELP);
    }

    fn terrain_click(&mut self, h: i32) -> bool {
        match h {
            -2 => self.pstate.alt = true,
            -3 => self.arm_undo(),
            i if i >= 0 => {
                // the three tree buttons paint the theme's tree, pine and palm tiles
                let Some(k) = Self::paint_index_of(tile_id(i as usize)) else { return false };
                self.tool = 0;
                self.paint_idx = k;
                // picking a brush rolls its variant (0x41eaa0); the green always starts as a plain green
                self.paint_variant = if PAINT[k].ty == 1 { Some((k, 0)) } else { None };
                self.edit = true;
            }
            _ => return false,
        }
        true
    }

    fn amenities_click(&mut self, h: i32) -> bool {
        match h {
            -2 => {
                self.pstate.alt = false;
                if self.tool != 0 {
                    self.tool = 0;
                    self.edit = true;
                }
            }
            7 => self.arm_undo(),
            1 => {
                // the exe has one Pathway; the port keeps gravel and paved, and a second click switches between them
                if self.tool == 2 {
                    self.path_kind = 3 - self.path_kind.clamp(1, 2);
                }
                self.path_kind = self.path_kind.clamp(1, 2);
                self.tool = 2;
                self.edit = true;
            }
            i if i >= 0 => {
                let slot = i as usize;
                if self.amenity_disabled(slot) {
                    self.ui_sound(0x18);
                    let why = if slot == 2 {
                        "No home sites are free: each Silver (or better) member buys one lot."
                    } else {
                        "The club has no landmark to place yet."
                    };
                    self.show_toast(why);
                    return true;
                }
                self.tool = 4;
                self.build_idx = AMENITIES[slot].kind as usize;
                self.edit = true;
                self.arm_design(AMENITIES[slot].kind);
            }
            _ => {
                // any other click in the panel picks the design last under the pointer on the strip (0x434ac0's default);
                // the click itself is not a button press
                if let Some((kind, st, mask)) = self.open_strip() {
                    let (px, py) = self.pstate.mouse;
                    let hot = st.hit(mask, px, py);
                    if hot != -1 {
                        self.design_hover = hot;
                    }
                    if self.design_hover != -1 || kind == land::K_LANDMARK {
                        self.design = (self.design_hover != -1).then_some((kind, self.design_hover));
                    }
                }
                return false;
            }
        }
        true
    }

    fn buildings_click(&mut self, h: i32) -> bool {
        match h {
            -2 => {
                self.pstate.alt = true;
                self.tool = 1;
                self.edit = self.pstate.elev_tool < 3;
            }
            -3 => self.arm_undo(),
            i if i >= 0 => {
                // refused in the exe's order, with the error sound
                let kind = 6 + i;
                let level = self.lot_level(kind);
                let refusal = if (level != 0 && economy::rank(self.holes.len()) < 2) || level > 1 {
                    Some(if level < 2 { MSG_UPGRADE_EARLY } else { MSG_MAX_LEVEL })
                } else if !self.build_available(kind as usize) {
                    Some(MSG_LOCKED)
                } else {
                    None
                };
                if let Some(m) = refusal {
                    self.ui_sound(0x18);
                    self.show_toast(m);
                    if self.tool == 4 && self.build_idx == kind as usize {
                        self.edit = false;
                    }
                    return true;
                }
                self.tool = 4;
                self.build_idx = kind as usize;
                self.edit = true;
            }
            _ => return false,
        }
        true
    }

    fn elevation_click(&mut self, h: i32) -> bool {
        match h {
            -2 => {
                self.pstate.alt = false;
                self.edit = false;
            }
            -3 => {
                // as in the exe, Undo also goes back to the Buildings panel
                self.pstate.alt = false;
                self.arm_undo();
            }
            3 => {
                // Analyze Golf Shot: what the exe's map click does with it is not decoded; the tool is only selected and the
                // map is left alone (placeholder)
                self.pstate.elev_tool = 3;
                self.edit = false;
                self.show_toast("Analyze Golf Shot is not available yet");
            }
            i if i >= 0 => {
                self.pstate.elev_tool = i as usize;
                self.tool = 1;
                self.edit = true;
            }
            _ => return false,
        }
        true
    }

    fn employee_click(&mut self, h: i32) -> bool {
        let rows = self.hired();
        let sel = self.pstate.emp_sel;
        match h {
            0 => {
                if rows.len() >= MAX_EMPLOYEES {
                    self.ui_sound(0x18);
                } else {
                    self.pstate.hire_open = true;
                }
            }
            2 if self.pstate.emp_off > 0 => self.pstate.emp_off -= 2,
            3 if rows.len() > self.pstate.emp_off + 8 => self.pstate.emp_off += 2,
            4 => {
                let Some(i) = sel else { return false };
                let e = self.employees[i];
                self.moving_employee = Some(i);
                let name = STAFF_NAMES[(-2 - e.job as i32).clamp(0, 3) as usize][e.upgraded as usize];
                self.show_toast(&format!("Click the course where the {name} should work"));
            }
            5 => {
                let Some(i) = sel else { return false };
                if self.fire_employee(i) {
                    self.pstate.emp_sel = None;
                }
            }
            6 => {
                // the port keeps no names for employees yet (placeholder for the exe's "Rename Employee..." prompt)
                if sel.is_none() {
                    return false;
                }
                self.show_toast("Employees cannot be renamed yet");
            }
            8..=15 => {
                let Some(&i) = rows.get(self.pstate.emp_off + (h - 8) as usize) else { return false };
                self.pstate.emp_sel = Some(i);
            }
            -2 => {
                self.panel = 4;
                self.golfer_page = 0;
            }
            -3 => self.panel = 5,
            _ => return false,
        }
        true
    }

    /// A click while the hire dialog is open: a choice hires (skilled ones need a Daily Fee course), anywhere off the dialog
    /// closes it.
    fn hire_click(&mut self, px: f32, py: f32) {
        if let Some(c) = hire_choice_at(px, py) {
            let (kind, skilled) = (c / 2, c % 2 == 1);
            if skilled && economy::rank(self.holes.len()) < 1 {
                self.ui_sound(0x18);
                self.show_toast(SKILLED_REFUSAL);
                return;
            }
            if self.hire_employee(kind, skilled) {
                self.pstate.emp_sel = self.hired().last().copied();
                let n = self.hired().len();
                self.pstate.emp_off = n.saturating_sub(7) & !1;
                self.snd("Interface/Button2.wav", 1.0, false);
            }
            self.pstate.hire_open = false;
        } else if !Rect::new(185.0, 95.0, 432.0, 375.0).has(px, py) {
            self.pstate.hire_open = false;
        }
    }

    /// The Elevation panel's edits at vertex (cx, cy), as the exe does them: the vertex tool moves one vertex; the square tool
    /// raises the lowest (or lowers the highest) corners of the 2 x 2 block x..x+1, y-1..y; the area tool moves the vertex and
    /// then pulls the 5 x 5 vertices around it along by the smoothing kernel. The exe's heights run 3..13; the port's own
    /// range is kept. No money is charged (none is in the exe's edit routines).
    pub fn edit_elevation(&mut self, cx: i32, cy: i32, delta: i32) {
        let t = &mut self.terrain;
        let at = |t: &Terrain, x: i32, y: i32| -> Option<i32> {
            (x >= 0 && y >= 0 && x <= t.w && y <= t.h).then(|| t.corner[(y * (t.w + 1) + x) as usize] as i32)
        };
        match self.pstate.elev_tool {
            1 => {
                let block = [(cx, cy - 1), (cx + 1, cy - 1), (cx, cy), (cx + 1, cy)];
                let hs: Vec<(i32, i32, i32)> = block.iter().filter_map(|&(x, y)| at(t, x, y).map(|v| (x, y, v))).collect();
                let pick = if delta > 0 { hs.iter().map(|v| v.2).min() } else { hs.iter().map(|v| v.2).max() };
                if let Some(m) = pick {
                    for &(x, y, _) in hs.iter().filter(|v| v.2 == m) {
                        t.raise_corner(x, y, delta);
                    }
                }
            }
            2 => {
                t.raise_corner(cx, cy, delta);
                if let Some(c) = at(t, cx, cy) {
                    for (i, row) in AREA_KERNEL.iter().enumerate() {
                        for (j, &k) in row.iter().enumerate() {
                            let (x, y) = (cx + i as i32 - 2, cy + j as i32 - 2);
                            let Some(v) = at(t, x, y) else { continue };
                            if (delta > 0 && v < c - k) || (delta < 0 && v > c + k) {
                                t.raise_corner(x, y, delta);
                            }
                        }
                    }
                }
            }
            _ => t.raise_corner(cx, cy, delta),
        }
        self.dirty = true;
        self.snd(if delta > 0 { "Interface/Bass Up 2.wav" } else { "Interface/Bass Down 2.wav" }, 0.6, false);
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn design_strips() {
        // benches: five designs from x 402, the odd ones a row lower
        let st = design_strip(land::K_BENCH).unwrap();
        assert_eq!(st.left(), 402);
        assert_eq!((st.at(0), st.at(1)), ((446, 500), (469, 527)));
        assert_eq!(st.hit(0, 469.0, 527.0), 1);
        assert_eq!(st.hit(0, 469.0, 516.0), -1);
        // landmarks show only the club's
        let st = design_strip(land::K_LANDMARK).unwrap();
        assert_eq!(st.hit(1 << 3, st.at(3).0 as f32, st.at(3).1 as f32), 3);
        assert_eq!(st.hit(1 << 2, st.at(3).0 as f32, st.at(3).1 as f32), -1);
        // flower beds: three shapes in five colours
        assert_eq!(strip_sprite(land::K_FLOWERS, 7), (0x1a9, 0x2f));
        assert_eq!(strip_sprite(land::K_BRIDGE, 5), (0x22b, 0xa9));
    }

    #[test]
    fn octagon_hits() {
        let h = hit(237, 525, 20);
        assert!(h.has(237.0, 525.0));
        assert!(h.has(256.0, 525.0));
        assert!(!h.has(257.0, 525.0));
        // on the diagonal the octagon reaches (b + 2a) / 2 < 20: 13 pixels each way
        assert!(h.has(250.0, 538.0));
        assert!(!h.has(251.0, 539.0));
        // the scroll arrows are narrow: x counts four times
        assert!(SCROLL[0].0.has(321.0, 520.0));
        assert!(!SCROLL[0].0.has(329.0, 549.0));
    }

    #[test]
    fn slots_and_choices() {
        assert_eq!(terrain_slot_at(0, 301.0, 532.0), Some(0));
        assert_eq!(terrain_slot_at(0, 332.0, 570.0), Some(7));
        assert_eq!(terrain_slot_at(0, 735.0, 500.0), Some(13));
        assert_eq!(terrain_slot_at(0, 230.0, 590.0), None);
        assert_eq!(hire_choice_at(300.0, 175.0), Some(0));
        assert_eq!(hire_choice_at(300.0, 195.0), Some(1));
        assert_eq!(hire_choice_at(300.0, 390.0), Some(6));
        assert_eq!(hire_choice_at(300.0, 225.0), None);
        assert_eq!(hire_choice_at(600.0, 175.0), None);
    }
}
