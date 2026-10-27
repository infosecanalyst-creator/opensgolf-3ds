//! The tools: the course on the bottom screen, played with the stylus as the PC game is with the mouse, and the tool
//! palette on the top screen, chosen with the D-pad (hud.rs draws the rest of the top screen's dashboard).
//!
//! The DS building games' rule (the touch screen is where you act, the top screen is where you read), with the course
//! itself under the stylus rather than a stand-in map of it: what you touch is what changes.
//!
//!   * Bottom screen, all of it: the course in the game's own view. A drag paints (or builds) along the pen; a tap does it
//!     once; holding still until a ring round the stylus fills (0.8 s) inspects instead (golfer, hole, building or ground,
//!     on the top screen). The MOVE tool slides the view with the pen; a stroke into the screen's edge scrolls on.
//!     The brush's footprint, the hole being laid out and the tiles the game turned down are drawn on the land.
//!   * Top screen: four categories (COURSE, BUILD, STAFF, INFO) and the chosen one's tools in a row. L and R choose a
//!     category, left and right on the D-pad a tool, up and down the brush size. Tools you use on the course arm as soon as they are chosen; buttons
//!     that act (hire, fire, save, a page) act on A. Under the row, the chosen tool's price and what it does.
//!   * A mini-map of the whole course (the flat plan this module used to draw on the bottom screen) with a frame for what
//!     the bottom screen shows.
//!   * The HOLE tool builds a hole in two taps (tee, then green) or one drag, showing the length and par as you go, and
//!     opens it at once.

use crate::app::App;
use crate::gfx::{Gfx, Mat4, Mode, TextureId, Uniforms, Vert};
use crate::shim::TouchPhase;
use crate::text::{money, Size, Text};
use crate::Demo;
use sg_core::assets::Rgba;
use sg_core::land;
use sg_core::terrain::{self as t, TILE_SIZE};
use std::collections::HashMap;
use std::time::{Duration, Instant};

/// The course view: the whole bottom screen.
pub const VIEW_W: f32 = 320.0;
pub const VIEW_H: f32 = 240.0;
/// The tool row on the top screen: a button's size, and how many show at once.
const BTN_W: f32 = 40.0;
const BTN_H: f32 = 30.0;
const ROW_SHOWN: usize = 10;
const R: f32 = std::f32::consts::FRAC_1_SQRT_2;
/// The band along the plan's edges where a held pen slides the view, and the fastest it slides (plan pixels a frame).
const EDGE: f32 = 14.0;
const EDGE_SPEED: f32 = 4.0;
/// A press that moves less than this (pixels) is a tap or a hold, not a drag.
const SLOP: f32 = 4.0;
/// Holding still this long inspects.
const HOLD: Duration = Duration::from_millis(800);
/// The ring under a still stylus starts filling after this, so a careful aim is not mistaken for a look.
const RING_FROM: Duration = Duration::from_millis(250);

#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub enum Tab {
    Course,
    Build,
    Staff,
    Info,
}
const TABS: [(Tab, &str); 4] = [(Tab::Course, "COURSE"), (Tab::Build, "BUILD"), (Tab::Staff, "STAFF"), (Tab::Info, "INFO")];

/// The top screen's information panels (hud.rs).
#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub enum Panel {
    Money,
    Holes,
    Golfers,
    Staff,
    Help,
}

#[derive(Clone, Copy, PartialEq, Debug)]
pub enum Tool {
    None,
    /// Tee, then green: a whole hole.
    Hole,
    /// Paints this tile type.
    Paint(u8),
    Raise,
    Lower,
    /// Drag to slide the view.
    Hand,
    /// Gravel (1) or paved (2) path.
    Path(i32),
    /// A one-tile item or building of this exe kind (bench, flower bed, ball washer, willow, scenic bridge).
    Item(i32),
    /// A building of this exe kind (chosen from the BLDG list).
    Building(i32),
    /// Takes back what is on a tile (items refunded, buildings demolished), through the game's undo records.
    Remove,
    // buttons that act at once rather than arm
    /// Opens the building list.
    Pick,
    /// Turns the next building a quarter.
    Turn,
    Hire(usize),
    Fire(usize),
    Show(Panel),
    Save,
    Load,
    Thoughts,
}

impl Tool {
    /// Buttons that do their thing when tapped instead of arming.
    fn momentary(self) -> bool {
        matches!(self, Tool::Pick | Tool::Turn | Tool::Hire(_) | Tool::Fire(_) | Tool::Show(_) | Tool::Save | Tool::Load | Tool::Thoughts)
    }
    /// Whether a drag applies it along the pen (else once, at the start).
    fn strokes(self) -> bool {
        matches!(self, Tool::Paint(_) | Tool::Raise | Tool::Lower | Tool::Path(_) | Tool::Item(_) | Tool::Remove)
    }
    fn sized(self) -> bool {
        matches!(self, Tool::Paint(_) | Tool::Raise | Tool::Lower)
    }
    fn key(self) -> u32 {
        match self {
            Tool::Paint(ty) => 100 + ty as u32,
            Tool::Raise => 1,
            Tool::Lower => 2,
            _ => 0,
        }
    }
}

const STAFF_SHORT: [&str; 4] = ["PRO", "RANGER", "KEEPER", "VENDOR"];

/// Each category's tools, in the order the D-pad goes through them.
fn slots(tab: Tab) -> Vec<Tool> {
    use Tool::*;
    match tab {
        Tab::Course => vec![
            Hole,
            Paint(t::TT_TEE),
            Paint(t::TT_PUTTING_GREEN),
            Paint(t::TT_FAIRWAY),
            Paint(t::TT_ROUGH),
            Paint(t::TT_DEEP_ROUGH),
            Paint(t::TT_SAND),
            Paint(t::TT_WATER_SHALLOW),
            Paint(t::TT_WOODS),
            Raise,
            Lower,
            Hand,
        ],
        Tab::Build => {
            let mut v = vec![Path(1), Path(2), Item(land::K_BENCH), Item(land::K_FLOWERS), Item(land::K_WASHER), Item(land::K_WILLOW), Item(land::K_BRIDGE)];
            v.extend(LIST_KINDS.iter().map(|&k| Building(k)));
            v.extend([Turn, Remove]);
            v
        }
        Tab::Staff => vec![Hire(0), Fire(0), Hire(1), Fire(1), Hire(2), Fire(2), Hire(3), Fire(3)],
        Tab::Info => vec![Show(Panel::Money), Show(Panel::Holes), Show(Panel::Golfers), Show(Panel::Staff), Save, Load, Thoughts, Show(Panel::Help)],
    }
}

fn tab_index(tab: Tab) -> usize {
    TABS.iter().position(|t| t.0 == tab).unwrap_or(0)
}

/// What the strip and the plan asked of main.rs this event.
#[derive(Clone, Copy, PartialEq, Debug)]
pub enum Action {
    Undo,
    Pause,
    Speed,
    Panel(Panel),
    Save,
    Load,
    Thoughts,
    /// A long press at this world point: show what is there.
    Inspect(f32, f32),
    /// A short line for the top screen (a hole opened, why something was refused).
    Note(&'static str),
    /// Someone was hired or fired: the staff panel comes up.
    Staffed,
}

/// What a pen event did, for main.rs: the tiles whose look changed (x0, y0, x1, y1 inclusive), a sound to play, an action.
#[derive(Default)]
pub struct Stroke {
    pub changed: Option<[i32; 4]>,
    pub sound: Option<&'static str>,
    /// The view slid: redraw.
    pub scrolled: bool,
    pub action: Option<Action>,
    /// A building or path went in: the whole course is rebuilt.
    pub rebuilt: bool,
}

impl Stroke {
    fn grow(&mut self, x0: i32, y0: i32, x1: i32, y1: i32) {
        self.changed = Some(match self.changed {
            None => [x0, y0, x1, y1],
            Some(c) => [c[0].min(x0), c[1].min(y0), c[2].max(x1), c[3].max(y1)],
        });
    }
}

/// A press on the plan.
struct Press {
    x0: f32,
    y0: f32,
    /// The world point it began on (x, z): a hole's tee, a look, a stroke's first dab stay put if the view moves.
    w0: (f32, f32),
    t0: Instant,
    /// It moved past SLOP: a drag.
    stroked: bool,
    /// Used up (it inspected, opened a hole, or was a view drag).
    done: bool,
    /// The view follows the pen (Hand, or L / R held).
    pan: bool,
}

pub struct Plan {
    tex: Option<TextureId>,
    tex_size: (u32, u32),
    /// The terrain changed: the plan's picture is redrawn at the next frame (`refresh`).
    pub stale: bool,
    pub tab: Tab,
    pub tool: Tool,
    /// The brush's edge in tiles (1..3) per sized tool.
    sizes: HashMap<u32, i32>,
    /// A message for the status line in place of the tool's (main.rs: a hole ready to open).
    pub hint: Option<String>,
    /// The pen on the plan while it is down.
    pub pen: Option<(f32, f32)>,
    press: Option<Press>,
    /// The press started on the strip, a button or the list: the rest of it does nothing.
    on_strip: bool,
    /// The stroke so far: where the brush was last put down (world x, z), and the corner an elevation tool last changed.
    last: Option<(f32, f32)>,
    last_corner: Option<(i32, i32)>,
    last_tile: Option<(i32, i32)>,
    /// Each tile type's colour: its texture averaged (`Demo::prepare`), so the plan looks like the land.
    rgb: Vec<[u8; 3]>,
    /// A stroke is under way (its undo point is taken, `App::undo_press`), and the cash when it began.
    in_stroke: bool,
    cash0: f64,
    /// The HOLE tool's tee, placed and waiting for its green.
    hole_tee: Option<(i32, i32)>,
    /// Tiles the game turned down, flashed red until the instant.
    flash: Vec<(i32, i32)>,
    flash_until: Option<Instant>,
    /// The building list is open (no longer: the buildings are in the BUILD row).
    picking: bool,
    /// The D-pad's place in each category's row.
    sel: [usize; 4],
    /// Why the game last turned tiles down, shown on the status line for a moment.
    refusal: Option<(&'static str, Instant)>,
    /// Golfers' positions on the plan, world x, z (main.rs, each frame).
    pub golfers: Vec<(f32, f32)>,
    /// The top screen shows thoughts (INFO's THINK button).
    pub thoughts: bool,
}

/// Buildings for the BLDG list: everything the building tool places but the one-tile items on the BUILD tab, landmarks
/// (their designs need the desktop's strip) and the clubhouse.

const LIST_KINDS: [i32; 10] = [5, 6, 7, 8, 9, 10, 11, 12, 13, 14];


/// A 3 x 5 pixel font for the tool labels and the status line: each glyph is five rows of three bits, the high bit on the
/// left. Upper case, digits and a little punctuation (anything else draws as a space).
fn glyph(c: char) -> [u8; 5] {
    match c.to_ascii_uppercase() {
        'A' => [2, 5, 7, 5, 5],
        'B' => [6, 5, 6, 5, 6],
        'C' => [3, 4, 4, 4, 3],
        'D' => [6, 5, 5, 5, 6],
        'E' => [7, 4, 6, 4, 7],
        'F' => [7, 4, 6, 4, 4],
        'G' => [3, 4, 5, 5, 3],
        'H' => [5, 5, 7, 5, 5],
        'I' => [7, 2, 2, 2, 7],
        'J' => [1, 1, 1, 5, 2],
        'K' => [5, 5, 6, 5, 5],
        'L' => [4, 4, 4, 4, 7],
        'M' => [5, 7, 7, 5, 5],
        'N' => [6, 5, 5, 5, 5],
        'O' => [2, 5, 5, 5, 2],
        'P' => [6, 5, 6, 4, 4],
        'Q' => [2, 5, 5, 6, 3],
        'R' => [6, 5, 6, 5, 5],
        'S' => [3, 4, 2, 1, 6],
        'T' => [7, 2, 2, 2, 2],
        'U' => [5, 5, 5, 5, 7],
        'V' => [5, 5, 5, 5, 2],
        'W' => [5, 5, 7, 7, 5],
        'X' => [5, 5, 2, 5, 5],
        'Y' => [5, 5, 2, 2, 2],
        'Z' => [7, 1, 2, 4, 7],
        '0' => [7, 5, 5, 5, 7],
        '1' => [2, 6, 2, 2, 7],
        '2' => [6, 1, 2, 4, 7],
        '3' => [6, 1, 2, 1, 6],
        '4' => [5, 5, 7, 1, 1],
        '5' => [7, 4, 6, 1, 6],
        '6' => [3, 4, 7, 5, 7],
        '7' => [7, 1, 2, 2, 2],
        '8' => [7, 5, 7, 5, 7],
        '9' => [7, 5, 7, 1, 6],
        '-' => [0, 0, 7, 0, 0],
        ':' => [0, 2, 0, 2, 0],
        '.' => [0, 0, 0, 0, 2],
        '/' => [1, 1, 2, 4, 4],
        '$' => [3, 6, 2, 3, 6],
        ',' => [0, 0, 0, 2, 4],
        '+' => [0, 2, 7, 2, 0],
        '?' => [6, 1, 2, 0, 2],
        '>' => [4, 2, 1, 2, 4],
        '!' => [2, 2, 2, 0, 2],
        _ => [0; 5],
    }
}

/// Width of `text` in the 3 x 5 font at `scale` (4 pixels a glyph, less the last one's gap).
fn text_width(text: &str, scale: f32) -> f32 {
    (text.chars().count() as f32 * 4.0 - 1.0).max(0.0) * scale
}

/// Draws `text` in the 3 x 5 font with its top-left at (x, y); each row's runs of lit pixels are one rectangle.
fn draw_text(g: &mut Gfx, ui: &Uniforms, x: f32, y: f32, text: &str, scale: f32, c: [f32; 4]) {
    let mut verts: Vec<Vert> = Vec::new();
    let v = |x: f32, y: f32| Vert::new(x, y, 0.0, 0.0, 0.0).col(c);
    for (i, ch) in text.chars().enumerate() {
        let gx = x + i as f32 * 4.0 * scale;
        for (row, bits) in glyph(ch).iter().enumerate() {
            let mut col = 0;
            while col < 3 {
                if bits & (4 >> col) == 0 {
                    col += 1;
                    continue;
                }
                let start = col;
                while col < 3 && bits & (4 >> col) != 0 {
                    col += 1;
                }
                let (x0, x1) = (gx + start as f32 * scale, gx + col as f32 * scale);
                let (y0, y1) = (y + row as f32 * scale, y + (row + 1) as f32 * scale);
                verts.extend_from_slice(&[v(x0, y0), v(x1, y0), v(x1, y1), v(x0, y0), v(x1, y1), v(x0, y1)]);
            }
        }
    }
    g.tris(Mode::Flat, None, ui, &verts);
}

/// A colour for a type with no texture to average.
fn fallback_rgb(ty: i32) -> [u8; 3] {
    match ty as u8 {
        t::TT_TEE => [96, 168, 72],
        t::TT_PUTTING_GREEN | t::TT_TRICKY_GREEN => [72, 176, 80],
        t::TT_FAIRWAY | t::TT_FIRM_FAIRWAY => [104, 160, 64],
        t::TT_ROUGH => [80, 128, 48],
        t::TT_DEEP_ROUGH => [64, 104, 40],
        t::TT_WOODS => [32, 72, 32],
        t::TT_ROCK | t::TT_CLIFF | t::TT_RAVINE => [120, 112, 104],
        t::TT_BUILDING => [140, 80, 64],
        t::TT_MARSH => [72, 96, 64],
        _ if t::is_water(ty) => [48, 96, 168],
        _ if t::is_sand(ty) => [216, 200, 144],
        _ => [96, 120, 64],
    }
}

/// Averages a picture's colour.
pub fn average_rgb(img: &Rgba) -> [u8; 3] {
    let mut sum = [0u64; 3];
    let n = (img.w * img.h).max(1) as u64;
    for p in img.px.chunks_exact(4) {
        for c in 0..3 {
            sum[c] += p[c] as u64;
        }
    }
    [(sum[0] / n) as u8, (sum[1] / n) as u8, (sum[2] / n) as u8]
}

impl Tool {
    /// The label under its button and its name for the status line.
    fn names(self) -> (&'static str, &'static str) {
        match self {
            Tool::None => ("", ""),
            Tool::Hole => ("HOLE", "NEW HOLE"),
            Tool::Paint(t::TT_FAIRWAY) => ("FAIR", "FAIRWAY"),
            Tool::Paint(t::TT_PUTTING_GREEN) => ("GREEN", "GREEN"),
            Tool::Paint(t::TT_TEE) => ("TEE", "TEE"),
            Tool::Paint(t::TT_ROUGH) => ("ROUGH", "ROUGH"),
            Tool::Paint(t::TT_SAND) => ("SAND", "SAND TRAP"),
            Tool::Paint(t::TT_WATER_SHALLOW) => ("WATER", "WATER"),
            Tool::Paint(t::TT_WOODS) => ("TREES", "TREES"),
            Tool::Paint(t::TT_DEEP_ROUGH) => ("DEEP", "DEEP ROUGH"),
            Tool::Paint(_) => ("PAINT", "PAINT"),
            Tool::Raise => ("RAISE", "RAISE LAND"),
            Tool::Lower => ("LOWER", "LOWER LAND"),
            Tool::Hand => ("MOVE", "MOVE VIEW"),
            Tool::Path(1) => ("PATH", "GRAVEL PATH"),
            Tool::Path(_) => ("PAVED", "PAVED PATH"),
            Tool::Item(land::K_BENCH) => ("BENCH", "BENCH"),
            Tool::Item(land::K_FLOWERS) => ("FLOWER", "FLOWER BED"),
            Tool::Item(land::K_WASHER) => ("WASHER", "BALL WASHER"),
            Tool::Item(land::K_WILLOW) => ("WILLOW", "WILLOW TREE"),
            Tool::Item(land::K_BRIDGE) => ("BRIDGE", "SCENIC BRIDGE"),
            Tool::Item(_) => ("ITEM", "ITEM"),
            Tool::Building(_) => ("BLDG", "BUILDING"),  // label() and the status line use the game's name
            Tool::Remove => ("CLEAR", "CLEAR"),
            Tool::Pick => ("BLDG", "BUILDINGS"),
            Tool::Turn => ("TURN", "TURN"),
            Tool::Hire(k) => (STAFF_SHORT[k.min(3)], ["HIRE A CLUB PRO", "HIRE A RANGER", "HIRE A GROUNDSKEEPER", "HIRE A SODA VENDOR"][k.min(3)]),
            Tool::Fire(k) => ("FIRE", ["FIRE A CLUB PRO", "FIRE A RANGER", "FIRE A GROUNDSKEEPER", "FIRE A SODA VENDOR"][k.min(3)]),
            Tool::Show(Panel::Money) => ("MONEY", "MONEY"),
            Tool::Show(Panel::Holes) => ("HOLES", "HOLES"),
            Tool::Show(Panel::Golfers) => ("GOLF", "GOLFERS"),
            Tool::Show(Panel::Staff) => ("STAFF", "STAFF"),
            Tool::Show(Panel::Help) => ("HELP", "MANUAL"),
            Tool::Save => ("SAVE", "SAVE"),
            Tool::Load => ("LOAD", "LOAD"),
            Tool::Thoughts => ("THINK", "THOUGHTS"),
        }
    }
}

/// A building kind's name (the game's own text) and price in dollars.
pub fn building_name(kind: i32) -> String {
    let (name, _, _) = sg_core::data::buildings()[kind.clamp(0, 19) as usize];
    sg_core::lang::text(name).to_string()
}
fn building_price(kind: i32) -> i64 {
    sg_core::data::buildings()[kind.clamp(0, 19) as usize].2 as i64 * 100
}

/// Course length in yards between two tiles and the par the game will give it (holetool::open_hole's rule: 25 yards a
/// tile, a quarter more past 250, then the par bands).
pub fn hole_length(tee: (i32, i32), green: (i32, i32)) -> (i32, i32) {
    let (dx, dy) = ((green.0 - tee.0) as f64, (green.1 - tee.1) as f64);
    let mut l = ((dx * dx + dy * dy) * 625.0).sqrt() as i32;
    if l > 250 {
        l += (l - 250) / 4;
    }
    let par = match l {
        e if e > 625 => 6,
        e if e > 474 => 5,
        e if e > 249 => 4,
        e if e < 51 => 2,
        _ => 3,
    };
    (l, par)
}

impl Plan {
    /// `rgb`: per tile type, its texture's average colour where there is one.
    pub fn new(rgb: Vec<Option<[u8; 3]>>) -> Plan {
        let rgb = (0..t::TYPE_COUNT).map(|ty| rgb.get(ty).copied().flatten().unwrap_or_else(|| fallback_rgb(ty as i32))).collect();
        Plan {
            tex: None,
            tex_size: (0, 0),
            stale: true,
            tab: Tab::Course,
            tool: Tool::Hole,
            sizes: HashMap::new(),
            hint: None,
            pen: None,
            press: None,
            on_strip: false,
            last: None,
            last_corner: None,
            last_tile: None,
            rgb,
            in_stroke: false,
            cash0: 0.0,
            hole_tee: None,
            flash: Vec::new(),
            flash_until: None,
            picking: false,
            sel: [0; 4],
            refusal: None,
            golfers: Vec::new(),
            thoughts: true,
        }
    }

    fn colour(&self, ty: i32) -> [u8; 3] {
        self.rgb.get(ty as usize).copied().unwrap_or([0, 0, 0])
    }

    fn size(&self) -> i32 {
        if self.tool.sized() {
            self.sizes.get(&self.tool.key()).copied().unwrap_or(1)
        } else {
            1
        }
    }

    // ---- geometry ------------------------------------------------------------------------------------------------------

    /// Bottom-screen pixel to the world (x, z) of the land under it, as you see it (hills in front picked first).
    pub fn to_world(d: &Demo, x: f32, y: f32) -> (f32, f32) {
        let p = d.pick(x, y);
        (p[0], p[2])
    }

    /// Slides the view so the ground under bottom-screen pixel `from` comes under `to` (a drag, an edge scroll).
    fn slide(d: &mut Demo, from: (f32, f32), to: (f32, f32)) {
        let (a, b) = (d.ground(from.0, from.1), d.ground(to.0, to.1));
        d.cam_x -= b.0 - a.0;
        d.cam_z -= b.1 - a.1;
    }

    /// World position of terrain corner (cx, cy).
    fn corner_world(d: &Demo, cx: i32, cy: i32) -> (f32, f32) {
        (
            cx as f32 * TILE_SIZE - d.terrain.w as f32 * TILE_SIZE * 0.5,
            cy as f32 * TILE_SIZE - d.terrain.h as f32 * TILE_SIZE * 0.5,
        )
    }

    fn tile_of(d: &Demo, wx: f32, wz: f32) -> (i32, i32) {
        (
            ((wx + d.terrain.w as f32 * TILE_SIZE * 0.5) / TILE_SIZE).floor() as i32,
            ((wz + d.terrain.h as f32 * TILE_SIZE * 0.5) / TILE_SIZE).floor() as i32,
        )
    }

    /// The tiles the brush covers with its middle at world (wx, wz): the first tile and the edge length.
    fn brush_tiles(&self, d: &Demo, wx: f32, wz: f32) -> (i32, i32, i32) {
        let fx = (wx + d.terrain.w as f32 * TILE_SIZE * 0.5) / TILE_SIZE;
        let fy = (wz + d.terrain.h as f32 * TILE_SIZE * 0.5) / TILE_SIZE;
        let s = self.size();
        // a 1-tile brush is the tile under the pen; a 2-tile one the four round the nearest corner; a 3-tile one centred
        (((fx - s as f32 * 0.5).round()) as i32, ((fy - s as f32 * 0.5).round()) as i32, s)
    }

    /// Arms a tool (from a button or the building list).
    fn arm(&mut self, app: &mut App, tool: Tool) {
        self.tool = tool;
        self.hole_tee = None;
        self.picking = false;
        if let Tool::Paint(ty) = tool {
            // the price preview asks the game with its armed brush (App::paint_cost_at)
            if let Some(i) = crate::app::PAINT.iter().position(|p| p.ty == ty as i32 && p.vbyte == 0) {
                app.paint_idx = i;
            }
        }
    }

    // ---- input ---------------------------------------------------------------------------------------------------------

    /// A pen event on the bottom screen (its pixels). `pan`: L or R is held, so a drag slides the view.
    pub fn pen(&mut self, d: &mut Demo, app: &mut App, phase: TouchPhase, x: f32, y: f32, pan: bool) -> Stroke {
        let mut out = Stroke::default();
        match phase {
            TouchPhase::Started => {
                self.last = None;
                self.last_corner = None;
                self.last_tile = None;
                self.pen = None;
                self.on_strip = false;
                self.pen = Some((x, y));
                let pan = pan || self.tool == Tool::Hand;
                let w0 = Self::to_world(d, x, y);
                self.press = Some(Press { x0: x, y0: y, w0, t0: Instant::now(), stroked: pan, done: pan, pan });
            }
            TouchPhase::Moved => {
                if self.on_strip {
                    return out;
                }
                let (x, y) = (x.clamp(0.0, VIEW_W - 1.0), y.clamp(0.0, VIEW_H - 1.0));
                let prev = self.pen.unwrap_or((x, y));
                self.pen = Some((x, y));
                let Some(p) = self.press.as_mut() else { return out };
                if p.pan {
                    // the land follows the pen: the spot under it stays under it
                    Self::slide(d, prev, (x, y));
                    out.scrolled = true;
                    return out;
                }
                if p.done {
                    return out;
                }
                if !p.stroked {
                    if (x - p.x0).hypot(y - p.y0) < SLOP {
                        return out;
                    }
                    p.stroked = true;
                    let w0 = p.w0;
                    if self.tool == Tool::Hole {
                        // the drag runs from the tee to the green: nothing is built until the pen comes up
                        return out;
                    }
                    self.remember(app);
                    let (wx, wz) = w0;
                    self.apply(d, app, wx, wz, &mut out);
                    out.sound = self.sound();
                }
                if self.tool.strokes() {
                    self.stroke_to(d, app, x, y, &mut out);
                }
            }
            TouchPhase::Ended | TouchPhase::Cancelled => {
                if let Some(p) = self.press.take() {
                    if !p.done {
                        if !p.stroked {
                            // a tap: a new hole's green opens it; else the tool once
                            let (wx, wz) = p.w0;
                            if Self::open_at(app, wx, wz) {
                                self.stale = true;
                                out.action = Some(Action::Note("Hole open"));
                            } else if self.tool == Tool::Hole {
                                self.hole_tap(d, app, wx, wz, &mut out);
                            } else if self.tool != Tool::None {
                                self.remember(app);
                                self.apply(d, app, wx, wz, &mut out);
                                out.sound = self.sound();
                                self.finish(app);
                            }
                        } else if self.tool == Tool::Hole {
                            let tee = p.w0;
                            let green = self.pen.map(|(x, y)| Self::to_world(d, x, y)).unwrap_or(tee);
                            let (tee, green) = (Self::tile_of(d, tee.0, tee.1), Self::tile_of(d, green.0, green.1));
                            self.build_hole(app, tee, green, &mut out);
                        }
                    }
                }
                self.finish(app);
                self.pen = None;
                self.last = None;
                self.last_corner = None;
                self.last_tile = None;
            }
        }
        out
    }

    /// A tap on one of the tab's twelve buttons.
    fn press_slot(&mut self, app: &mut App, tool: Tool, out: &mut Stroke) {
        match tool {
            Tool::None => out.sound = None,
            Tool::Pick => {}
            Tool::Building(k) if !app.build_available(k as usize) => {
                out.sound = Some("Interface/Wrong.wav");
                out.action = Some(Action::Note("Build more holes to unlock that"));
            }
            Tool::Turn => {
                app.turn = (app.turn + 1) & 3;
                out.action = Some(Action::Note("Next building turned"));
            }
            Tool::Hire(k) => {
                if app.hire_staff(k) {
                    out.action = Some(Action::Staffed);
                } else {
                    out.sound = Some("Interface/Wrong.wav");
                    out.action = Some(Action::Note("Can't hire: no money or staff limit"));
                }
            }
            Tool::Fire(k) => {
                if !app.fire_staff(k) {
                    out.sound = Some("Interface/Wrong.wav");
                    out.action = Some(Action::Note("Nobody of that kind to fire"));
                } else {
                    out.action = Some(Action::Staffed);
                }
            }
            Tool::Show(p) => out.action = Some(Action::Panel(p)),
            Tool::Save => out.action = Some(Action::Save),
            Tool::Load => out.action = Some(Action::Load),
            Tool::Thoughts => {
                self.thoughts = !self.thoughts;
                out.action = Some(Action::Thoughts);
            }
            t => self.arm(app, t),
        }
    }

    /// The D-pad on the top screen's palette: left and right choose a tool, up and down a category. A tool used on the course
    /// is armed at once; a button that acts waits for A.
    pub fn nav(&mut self, app: &mut App, dx: i32, dy: i32) -> Stroke {
        let mut out = Stroke { sound: Some("Interface/Button2.wav"), ..Default::default() };
        if dy != 0 {
            let i = (tab_index(self.tab) as i32 + dy).rem_euclid(TABS.len() as i32) as usize;
            self.tab = TABS[i].0;
        }
        let ti = tab_index(self.tab);
        let row = slots(self.tab);
        if dx != 0 {
            self.sel[ti] = (self.sel[ti] as i32 + dx).rem_euclid(row.len() as i32) as usize;
        }
        self.sel[ti] = self.sel[ti].min(row.len() - 1);
        let tool = row[self.sel[ti]];
        let locked = matches!(tool, Tool::Building(k) if !app.build_available(k as usize));
        if !tool.momentary() && tool != self.tool && !(dy != 0 && locked) {
            self.press_slot(app, tool, &mut out);
            if out.sound == Some("Interface/Button1.wav") {
                out.sound = Some("Interface/Button2.wav");
            }
        }
        out
    }

    /// The tool the D-pad is on in the shown category.
    pub fn chosen(&self) -> Tool {
        let row = slots(self.tab);
        row[self.sel[tab_index(self.tab)].min(row.len() - 1)]
    }

    /// A: the chosen button acts (hire, fire, save, load, a page, thoughts, turn), or the chosen tool is armed.
    pub fn activate(&mut self, app: &mut App) -> Stroke {
        let mut out = Stroke { sound: Some("Interface/Button1.wav"), ..Default::default() };
        let tool = self.chosen();
        if tool == self.tool && !tool.momentary() {
            out.action = Some(Action::Note("Touch the course to use it"));
            return out;
        }
        self.press_slot(app, tool, &mut out);
        out
    }

    /// Drops a press under way (the pause menu opened over it, or another game is opening): its stroke ends as if the
    /// stylus had come up, so nothing paints on by itself and the next stroke takes its own undo point.
    pub fn drop_press(&mut self, app: &mut App) {
        self.finish(app);
        self.press = None;
        self.pen = None;
        self.last = None;
        self.last_corner = None;
        self.last_tile = None;
    }

    /// Another game is opening: nothing of this one's tools carries over.
    pub fn reset(&mut self, app: &mut App) {
        self.drop_press(app);
        self.hole_tee = None;
        self.flash.clear();
        self.flash_until = None;
        self.refusal = None;
        self.hint = None;
        self.stale = true;
    }

    /// B with a tee placed and no green yet: the tee is taken back (true); else B is undo.
    pub fn cancel_tee(&mut self) -> bool {
        self.hole_tee.take().is_some()
    }

    /// D-pad up / down: the brush size, 1 to 3, for the tools that have one.
    pub fn change_size(&mut self, by: i32) -> Stroke {
        let mut out = Stroke::default();
        if self.tool.sized() {
            let s = (self.size() + by).clamp(1, 3);
            self.sizes.insert(self.tool.key(), s);
            out.sound = Some("Interface/Button2.wav");
        } else {
            out.sound = Some("Interface/Wrong.wav");
            out.action = Some(Action::Note("Brush size is for painting and raising or lowering land"));
        }
        out
    }

    /// The brush put down every half tile from where it last was to the pen, so a quick stroke leaves no gaps.
    fn stroke_to(&mut self, d: &Demo, app: &mut App, x: f32, y: f32, out: &mut Stroke) {
        let (wx, wz) = Self::to_world(d, x, y);
        let (lx, lz) = self.last.unwrap_or((wx, wz));
        let dist = ((wx - lx).powi(2) + (wz - lz).powi(2)).sqrt();
        let steps = (dist / (TILE_SIZE * 0.5)).ceil().max(1.0) as i32;
        for i in 1..=steps {
            let f = i as f32 / steps as f32;
            self.apply(d, app, lx + (wx - lx) * f, lz + (wz - lz) * f, out);
        }
    }

    /// Called every frame: a held press long enough inspects; a stroke held at the plan's edge slides the view and paints on.
    pub fn tick(&mut self, d: &mut Demo, app: &mut App) -> Stroke {
        let mut out = Stroke::default();
        if let Some(until) = self.flash_until {
            if Instant::now() >= until {
                self.flash.clear();
                self.flash_until = None;
            }
            out.scrolled = true; // keeps the red tiles drawn, then gone
        }
        let Some((x, y)) = self.pen else { return out };
        let Some(p) = self.press.as_mut() else { return out };
        if !p.stroked && !p.done && p.t0.elapsed() > RING_FROM {
            out.scrolled = true; // the ring fills: redraw
        }
        if !p.stroked && !p.done && p.t0.elapsed() >= HOLD {
            p.done = true;
            let (wx, wz) = p.w0;
            out.action = Some(Action::Inspect(wx, wz));
            out.sound = Some("Interface/Button2.wav");
            return out;
        }
        if !p.stroked || p.pan || p.done || !self.tool.strokes() {
            return out;
        }
        // how far into the edge band the pen is, -1..1 on each axis
        let band = |v: f32, max: f32| {
            if v < EDGE {
                -(EDGE - v) / EDGE
            } else if v > max - EDGE {
                (v - (max - EDGE)) / EDGE
            } else {
                0.0
            }
        };
        let (ex, ey) = (band(x, VIEW_W), band(y, VIEW_H));
        if ex == 0.0 && ey == 0.0 {
            return out;
        }
        let c = (VIEW_W * 0.5, VIEW_H * 0.5);
        Self::slide(d, (c.0 + ex * EDGE_SPEED, c.1 + ey * EDGE_SPEED), c);
        out.scrolled = true;
        self.stroke_to(d, app, x, y, &mut out);
        out
    }

    /// Opens the hole whose cup green is at this spot (or a tile or two from it), when it is built but not yet open.
    fn open_at(app: &mut App, wx: f32, wz: f32) -> bool {
        let (ta, tb) = app.terrain.tile_of(wx, wz);
        let mut found = false;
        'look: for r in 0..=2 {
            for b in tb - r..=tb + r {
                for a in ta - r..=ta + r {
                    if !sg_core::course::inside(a, b) || app.terrain.ty[app.terrain.tile_index(a, b)] != 1 {
                        continue;
                    }
                    let fl = app.course.flags[sg_core::course::idx(a, b)];
                    let h = (fl & 0x1f) as usize;
                    if fl & sg_core::course::f::CUP != 0 && (1..19).contains(&h) && app.club.holes[h].par == 0 {
                        found = true;
                        break 'look;
                    }
                }
            }
        }
        found && app.open_hole()
    }

    /// The HOLE tool's tap: the first places the tee, the second the green (and builds the hole).
    fn hole_tap(&mut self, d: &Demo, app: &mut App, wx: f32, wz: f32, out: &mut Stroke) {
        let tile = Self::tile_of(d, wx, wz);
        match self.hole_tee {
            None => {
                if !app.terrain.inside(tile.0, tile.1) || !app.land.as_ref().is_none_or(|l| l.in_play(tile.0, tile.1)) {
                    out.sound = Some("Interface/Wrong.wav");
                    out.action = Some(Action::Note("The tee has to be on your land"));
                    self.flash_tiles(vec![tile]);
                    return;
                }
                self.hole_tee = Some(tile);
                out.sound = Some("Interface/Button1.wav");
            }
            Some(tee) => {
                self.hole_tee = None;
                self.build_hole(app, tee, tile, out);
            }
        }
    }

    /// A whole hole: the tee, a 3 x 3 green with its cup in the middle, then opened, as one undo step.
    fn build_hole(&mut self, app: &mut App, tee: (i32, i32), green: (i32, i32), out: &mut Stroke) {
        // both ends on the map (a drag can start off it, at the course's edge)
        if !app.terrain.inside(tee.0, tee.1) || !app.terrain.inside(green.0, green.1) {
            out.sound = Some("Interface/Wrong.wav");
            out.action = Some(Action::Note("The tee and the green have to be on the course"));
            return;
        }
        let (yards, _) = hole_length(tee, green);
        if (green.0 - tee.0).abs().max((green.1 - tee.1).abs()) < 3 || yards < 75 {
            out.sound = Some("Interface/Wrong.wav");
            out.action = Some(Action::Note("Too short: put the green farther from the tee"));
            return;
        }
        let h = app.club.next_hole;
        if !(1..19).contains(&h) {
            out.sound = Some("Interface/Wrong.wav");
            out.action = Some(Action::Note("The course has 18 holes already"));
            return;
        }
        self.remember(app);
        let mut refused = self.paint(app, &[tee], t::TT_TEE);
        // the cup goes on the first green tile painted (holetool), so the middle first
        let mut tiles = vec![green];
        for dy in -1..=1 {
            for dx in -1..=1 {
                if (dx, dy) != (0, 0) {
                    tiles.push((green.0 + dx, green.1 + dy));
                }
            }
        }
        tiles.retain(|&(x, y)| app.terrain.inside(x, y) && app.terrain.ty[app.terrain.tile_index(x, y)] != t::TT_PUTTING_GREEN);
        refused.extend(self.paint(app, &tiles, t::TT_PUTTING_GREEN));
        self.finish(app);
        out.grow(tee.0.min(green.0) - 1, tee.1.min(green.1) - 1, tee.0.max(green.0) + 1, tee.1.max(green.1) + 1);
        self.stale = true;
        let r = &app.club.holes[h as usize];
        if r.back.0 != 0 && r.pin.0 != 0 && app.open_hole() {
            out.sound = Some("Interface/Place GreenTee.wav");
            out.action = Some(Action::Note("Hole open"));
        } else {
            out.sound = Some("Interface/Wrong.wav");
            out.action = Some(Action::Note(if refused.is_empty() {
                "The game would not open that hole"
            } else {
                "Couldn't build there (land, money or obstacle)"
            }));
        }
        if !refused.is_empty() {
            self.flash_tiles(refused);
        }
    }

    fn flash_tiles(&mut self, tiles: Vec<(i32, i32)>) {
        self.flash = tiles;
        self.flash_until = Some(Instant::now() + Duration::from_millis(700));
    }

    /// The start of a stroke: the game keeps the whole club as it is (money and all) for its undo (undo.rs).
    fn remember(&mut self, app: &mut App) {
        if !self.in_stroke {
            app.undo_press();
            self.in_stroke = true;
            self.cash0 = app.econ.cash;
        }
    }

    /// The end of a stroke: the game keeps that undo point if the stroke changed the course (UNDO / B step back, up to 30).
    fn finish(&mut self, app: &mut App) {
        if self.in_stroke {
            app.undo_release();
            self.in_stroke = false;
        }
    }

    fn sound(&self) -> Option<&'static str> {
        Some(match self.tool {
            Tool::Paint(t::TT_FAIRWAY) => "Interface/Place Fairway.wav",
            Tool::Paint(t::TT_PUTTING_GREEN) | Tool::Paint(t::TT_TEE) => "Interface/Place GreenTee.wav",
            Tool::Paint(t::TT_SAND) => "Interface/Place Bunker.wav",
            Tool::Paint(t::TT_WATER_SHALLOW) => "Interface/Place Water.wav",
            Tool::Paint(_) => "Interface/Place Rough.wav",
            // the game's own edits play theirs
            _ => return None,
        })
    }

    /// Paints tiles through the game; the ones it turned down (off the land, too dear, an obstacle).
    fn paint(&mut self, app: &mut App, tiles: &[(i32, i32)], ty: u8) -> Vec<(i32, i32)> {
        if tiles.is_empty() {
            return Vec::new();
        }
        if let Some(i) = crate::app::PAINT.iter().position(|p| p.ty == ty as i32 && p.vbyte == 0) {
            app.paint_idx = i;
        }
        app.paint_tiles(tiles);
        tiles.iter().copied().filter(|&(x, y)| !app.terrain.inside(x, y) || app.terrain.ty[app.terrain.tile_index(x, y)] != ty).collect()
    }

    /// Puts the tool down once at world (wx, wz), through the game.
    fn apply(&mut self, d: &Demo, app: &mut App, wx: f32, wz: f32, out: &mut Stroke) {
        self.last = Some((wx, wz));
        match self.tool {
            Tool::Paint(ty) => {
                let (x0, y0, s) = self.brush_tiles(d, wx, wz);
                let mut tiles = Vec::new();
                for y in y0..y0 + s {
                    for x in x0..x0 + s {
                        // tiles already of this type are left alone (and not charged again)
                        if app.terrain.inside(x, y) && app.terrain.ty[app.terrain.tile_index(x, y)] != ty {
                            tiles.push((x, y));
                        }
                    }
                }
                let cash = app.econ.cash;
                let refused = self.paint(app, &tiles, ty);
                if refused.len() < tiles.len() {
                    out.grow(x0, y0, x0 + s - 1, y0 + s - 1);
                    self.stale = true;
                }
                if let Some(&(rx, ry)) = refused.first() {
                    // why: not the club's land, not enough money, or something in the way
                    let why = if !app.land.as_ref().is_none_or(|l| l.in_play(rx, ry)) {
                        "NOT YOUR LAND: BUY MORE TO BUILD THERE"
                    } else if app.paint_cost_at(rx, ry).is_some_and(|c| c as f64 * 100.0 > cash) {
                        "NOT ENOUGH MONEY"
                    } else {
                        "SOMETHING IS IN THE WAY"
                    };
                    self.refusal = Some((why, Instant::now() + Duration::from_millis(2500)));
                }
                if !refused.is_empty() {
                    let mut f = std::mem::take(&mut self.flash);
                    f.extend(refused);
                    f.dedup();
                    self.flash_tiles(f);
                }
            }
            Tool::Raise | Tool::Lower => {
                let (cx, cy) = d.terrain.corner_of(wx, wz);
                if self.last_corner == Some((cx, cy)) || cx < 0 || cy < 0 || cx > d.terrain.w || cy > d.terrain.h {
                    return;
                }
                self.last_corner = Some((cx, cy));
                // the Elevation panel's three tools: one corner, the 2 x 2 block, the corner with the land round it
                app.pstate.elev_tool = match self.size() {
                    1 => 0,
                    2 => 1,
                    _ => 2,
                };
                app.edit_elevation(cx, cy, if self.tool == Tool::Raise { 1 } else { -1 });
                if let Some(land) = app.land.as_mut() {
                    land.sync_from_terrain(&app.terrain);
                }
                out.grow(cx - 3, cy - 3, cx + 2, cy + 2);
                self.stale = true;
            }
            Tool::Path(_) | Tool::Item(_) | Tool::Building(_) | Tool::Remove => {
                let tile = Self::tile_of(d, wx, wz);
                if self.last_tile == Some(tile) || !app.terrain.inside(tile.0, tile.1) {
                    return;
                }
                // a building goes in once per press
                if matches!(self.tool, Tool::Building(_)) && self.last_tile.is_some() {
                    return;
                }
                self.last_tile = Some(tile);
                let cash = app.econ.cash;
                match self.tool {
                    Tool::Path(k) => {
                        app.path_kind = k;
                        app.edit_path(tile.0, tile.1, false);
                    }
                    Tool::Item(k) | Tool::Building(k) => {
                        app.build_idx = k as usize;
                        app.edit_building(tile.0, tile.1, false);
                    }
                    _ => app.undo_tile(tile.0, tile.1),
                }
                if app.dirty {
                    out.rebuilt = true;
                    self.stale = true;
                } else if self.tool != Tool::Remove && app.econ.cash == cash {
                    self.flash_tiles(vec![tile]);
                }
            }
            _ => {}
        }
    }
}

#[derive(Clone, Copy)]
enum StripHit {
    Tab(usize),
    Slot(usize),
    Undo,
    Pause,
    Speed,
    Size,
}

// ---- drawing -------------------------------------------------------------------------------------------------------------

/// Flat-colour drawing helpers over one set of 2D uniforms.
struct Pen2d<'a> {
    u: &'a Uniforms,
}

impl Pen2d<'_> {
    fn v(x: f32, y: f32, c: [f32; 4]) -> Vert {
        Vert::new(x, y, 0.0, 0.0, 0.0).col(c)
    }
    fn quad(&self, g: &mut Gfx, p: [(f32, f32); 4], c: [f32; 4]) {
        g.quad(Mode::Flat, None, self.u, p.map(|(x, y)| Self::v(x, y, c)));
    }
    fn rect(&self, g: &mut Gfx, x: f32, y: f32, w: f32, h: f32, c: [f32; 4]) {
        self.quad(g, [(x, y), (x + w, y), (x + w, y + h), (x, y + h)], c);
    }
    fn tri(&self, g: &mut Gfx, a: (f32, f32), b: (f32, f32), d: (f32, f32), c: [f32; 4]) {
        g.tris(Mode::Flat, None, self.u, &[Self::v(a.0, a.1, c), Self::v(b.0, b.1, c), Self::v(d.0, d.1, c)]);
    }
    fn line(&self, g: &mut Gfx, a: (f32, f32), b: (f32, f32), w: f32, c: [f32; 4]) {
        let (dx, dy) = (b.0 - a.0, b.1 - a.1);
        let n = (dx * dx + dy * dy).sqrt().max(1e-3);
        let (ox, oy) = (-dy / n * w * 0.5, dx / n * w * 0.5);
        self.quad(g, [(a.0 + ox, a.1 + oy), (b.0 + ox, b.1 + oy), (b.0 - ox, b.1 - oy), (a.0 - ox, a.1 - oy)], c);
    }
    fn outline(&self, g: &mut Gfx, p: [(f32, f32); 4], w: f32, c: [f32; 4]) {
        for i in 0..4 {
            self.line(g, p[i], p[(i + 1) % 4], w, c);
        }
    }
}

const WHITE: [f32; 4] = [1.0, 1.0, 1.0, 1.0];
const YELLOW: [f32; 4] = [1.0, 0.9, 0.2, 1.0];
const RED: [f32; 4] = [1.0, 0.2, 0.15, 0.85];

impl Plan {
    /// Redraws the plan's picture if the course changed: the ground, dimmed off the club's land, with its buildings, items
    /// and paths. Call after `begin_frame` (the GPU is then done with it).
    pub fn refresh(&mut self, g: &mut Gfx, d: &Demo, app: &App) {
        if !self.stale && self.tex.is_some() {
            return;
        }
        self.stale = false;
        let tr = &d.terrain;
        let (w, h) = (tr.w as u32, tr.h as u32);
        let mut px = vec![255u8; (w * h * 4) as usize];
        let owned = |x: i32, y: i32| app.land.as_ref().is_none_or(|l| l.in_play(x, y));
        for y in 0..tr.h {
            for x in 0..tr.w {
                let ty = tr.type_at(x, y);
                let c = if ty == 20 { [24, 28, 24] } else { self.colour(ty) };
                // height and slope: higher land lighter, slopes facing the top of the plan lighter, facing down darker
                let l = tr.tile_levels(x, y).map(|v| v as f32); // north-west, north-east, south-west, south-east
                let avg = (l[0] + l[1] + l[2] + l[3]) * 0.25;
                let facing = (l[1] - l[2]) * 0.5;
                let mut shade = (0.74 + 0.06 * avg + 0.2 * facing).clamp(0.4, 1.45);
                // land the club does not own: dimmed, so the property line reads at a glance
                if !owned(x, y) {
                    shade *= 0.42;
                }
                let o = ((y as u32 * w + x as u32) * 4) as usize;
                for k in 0..3 {
                    px[o + k] = (c[k] as f32 * shade).min(255.0) as u8;
                }
            }
        }
        // what stands on the land
        if let Some(l) = app.land.as_ref() {
            for o in l.objects.iter() {
                let (c, size) = match o.kind {
                    land::K_PATH => ([168, 160, 140], 1),
                    land::K_CLUBHOUSE => ([200, 70, 55], l.footprint_size(o)),
                    land::K_BENCH | land::K_FLOWERS | land::K_WILLOW | land::K_BRIDGE | land::K_WASHER => ([220, 200, 120], 1),
                    land::K_LANDMARK => ([150, 150, 170], 1),
                    _ => ([176, 104, 76], l.footprint_size(o)),
                };
                for b in o.b..o.b + size {
                    for a in o.a..o.a + size {
                        if tr.inside(a, b) {
                            let i = ((b as u32 * w + a as u32) * 4) as usize;
                            px[i..i + 3].copy_from_slice(&c);
                        }
                    }
                }
            }
        }
        let img = Rgba { w, h, px };
        match self.tex {
            Some(id) if self.tex_size == (w, h) => g.update_texture(id, &img),
            old => {
                // first time, or a course of another size: a texture of its own size
                if let Some(id) = old {
                    g.delete_texture(id);
                }
                self.tex = Some(g.texture_crisp(&img));
                self.tex_size = (w, h);
            }
        }
    }

    /// What the status line says.
    fn status(&self, d: &Demo, app: &App, at: (f32, f32)) -> (String, [f32; 4]) {
        // the chosen button when it is not the tool in hand: what A would do, or why it is locked
        let chosen = self.chosen();
        if chosen != self.tool {
            match chosen {
                Tool::Building(k) if !app.build_available(k as usize) => {
                    let n = building_name(k).to_uppercase();
                    let why = if k == land::K_HOME_SITE {
                        "OPENS WHEN MEMBERS WANT HOMES".to_string()
                    } else {
                        match (0..=18).find(|&h| land::unlocked_kinds(h, false) > k) {
                            Some(h) => format!("OPENS AT {h} HOLES"),
                            None => "LOCKED".to_string(),
                        }
                    };
                    return (format!("{n}: {why}"), [0.75, 0.78, 0.72, 1.0]);
                }
                Tool::Hire(k) => {
                    let n = app.employees.iter().filter(|e| e.active && e.job == -2 - k as i8).count();
                    return (format!("{} (YOU HAVE {n})", chosen.names().1), WHITE);
                }
                Tool::Fire(k) => {
                    let n = app.employees.iter().filter(|e| e.active && e.job == -2 - k as i8).count();
                    return (format!("{} (YOU HAVE {n})", chosen.names().1), WHITE);
                }
                t if t.momentary() => return (format!("{}: PRESS A", t.names().1), WHITE),
                _ => {}
            }
        }
        if let (Some(h), Tool::Hole) = (&self.hint, self.tool) {
            return (h.clone(), [1.0, 0.86, 0.3, 1.0]);
        }
        if let Some((s, until)) = &self.refusal {
            if Instant::now() < *until {
                return (s.to_string(), [1.0, 0.6, 0.5, 1.0]);
            }
        }
        let name = self.tool.names().1;
        if self.in_stroke {
            let spent = (self.cash0 - app.econ.cash).round() as i64;
            if spent > 0 {
                return (format!("{name}   -{}", money(spent)), [1.0, 0.75, 0.6, 1.0]);
            }
        }
        let tile = Self::tile_of(d, at.0, at.1);
        let s = match self.tool {
            Tool::Hole => match (self.hole_tee, &self.press) {
                (_, Some(p)) if p.stroked && !p.pan => {
                    let tee = Self::tile_of(d, p.w0.0, p.w0.1);
                    let (y, par) = hole_length(tee, tile);
                    format!("HOLE {}   {y} YDS   PAR {par}", app.club.next_hole)
                }
                (Some(tee), _) if self.pen.is_some() => {
                    let (y, par) = hole_length(tee, tile);
                    format!("GREEN HERE: {y} YDS, PAR {par}")
                }
                (Some(_), _) => "TEE PLACED: NOW TAP WHERE THE GREEN GOES".into(),
                _ => format!("HOLE {}: TAP TEE, THEN GREEN", app.club.next_hole),
            },
            Tool::Paint(ty) => {
                let n = self.size();
                // under the pen: what this tile costs (clearing what is there included); else the paint's own price
                match (self.pen.is_some(), app.paint_cost_at(tile.0, tile.1)) {
                    (true, Some(c)) => format!("{name} {n}x{n}   {}/TILE HERE", money(c as i64 * 100)),
                    (true, None) => format!("{name} {n}x{n}   CAN'T PAINT HERE"),
                    _ => {
                        let c = sg_core::economy::Economy::terrain_cost_units(ty as i32) as i64 * 100;
                        format!("{name} {n}x{n}   {}/TILE", money(c))
                    }
                }
            }
            Tool::Raise | Tool::Lower => format!("{name} {}", ["CORNER", "2x2", "HILL"][(self.size() - 1).clamp(0, 2) as usize]),
            Tool::Item(k) | Tool::Building(k) => {
                let n = if matches!(self.tool, Tool::Building(_)) { building_name(k).to_uppercase() } else { name.to_string() };
                format!("{n}   {}", money(building_price(k)))
            }
            Tool::Remove => "CLEAR: TAP AN ITEM, PATH OR BUILDING".into(),
            Tool::Hand => "MOVE: DRAG THE COURSE (OR USE THE CIRCLE PAD)".into(),
            _ => name.to_string(),
        };
        (s, WHITE)
    }

    /// A tool's label under its button: buildings by the game's own name, the first word of it.
    fn label(tool: Tool) -> String {
        match tool {
            Tool::Fire(_) => "FIRE".to_string(),
            Tool::Building(k) => {
                let n = building_name(k).to_uppercase();
                let w = n.split_whitespace().next().unwrap_or("BLDG").to_string();
                match w.as_str() {
                    "DRIVING" => "RANGE".to_string(),
                    "PUTTING" => "PUTT".to_string(),
                    _ => w.chars().take(6).collect(),
                }
            }
            t => t.names().0.to_string(),
        }
    }

    /// The top screen's palette at y0 (the tabs, then the shown category's row of buttons, then the status line); drawn
    /// with `ui` (which may carry the 3D slider's offset). Returns the y below it.
    /// `speaker`: the golfer behind the newest thought, whom A goes to while a course tool is in hand.
    pub fn render_palette(&self, g: &mut Gfx, ui: &Uniforms, d: &Demo, app: &App, text: &Text, y0: f32, speaker: Option<&str>) -> f32 {
        let p2 = Pen2d { u: ui };
        const W: f32 = 400.0;
        // the categories
        let tw = W / TABS.len() as f32;
        for (i, (tab, label)) in TABS.iter().enumerate() {
            let x = i as f32 * tw;
            let on = *tab == self.tab;
            p2.rect(g, x + 1.0, y0, tw - 2.0, 14.0, if on { [0.3, 0.42, 0.26, 1.0] } else { [0.08, 0.11, 0.08, 0.92] });
            if on {
                p2.rect(g, x + 1.0, y0 + 12.0, tw - 2.0, 2.0, YELLOW);
            }
            let lw = text.width(label, Size::Small);
            text.draw(g, ui, x + (tw - lw) * 0.5, y0 + 1.0, label, Size::Small, if on { WHITE } else { [0.62, 0.68, 0.6, 1.0] });
        }
        // the row: ten buttons at a time, scrolled to keep the chosen one in view
        let ry = y0 + 15.0;
        let row = slots(self.tab);
        let sel = self.sel[tab_index(self.tab)].min(row.len() - 1);
        let first = if row.len() <= ROW_SHOWN { 0 } else { sel.saturating_sub(ROW_SHOWN / 2).min(row.len() - ROW_SHOWN) };
        p2.rect(g, 0.0, ry, W, BTN_H + 1.0, [0.1, 0.14, 0.1, 0.94]);
        for (k, &tool) in row.iter().enumerate().skip(first).take(ROW_SHOWN) {
            let bx = (k - first) as f32 * BTN_W;
            let armed = tool == self.tool || (tool == Tool::Thoughts && self.thoughts);
            self.draw_tool(g, ui, app, tool, bx, ry, armed);
            if k == sel && !armed {
                p2.outline(g, [(bx + 1.0, ry + 0.5), (bx + BTN_W - 1.0, ry + 0.5), (bx + BTN_W - 1.0, ry + BTN_H - 0.5), (bx + 1.0, ry + BTN_H - 0.5)], 1.5, WHITE);
            }
        }
        // more to either side
        if first > 0 {
            p2.tri(g, (1.0, ry + BTN_H * 0.5), (6.0, ry + BTN_H * 0.5 - 5.0), (6.0, ry + BTN_H * 0.5 + 5.0), YELLOW);
        }
        if first + ROW_SHOWN < row.len() {
            p2.tri(g, (W - 1.0, ry + BTN_H * 0.5), (W - 6.0, ry + BTN_H * 0.5 - 5.0), (W - 6.0, ry + BTN_H * 0.5 + 5.0), YELLOW);
        }
        // the status line: the armed tool's price and what to do; the chosen button's key on the right
        let sy = ry + BTN_H + 2.0;
        let at = self.pen.map(|(x, y)| Self::to_world(d, x, y)).unwrap_or((d.cam_x, d.cam_z));
        let (status, sc) = self.status(d, app, at);
        let chosen = row[sel];
        let key = if chosen.momentary() {
            format!("A: {}", chosen.names().1)
        } else {
            let see = speaker.map(|n| format!("A: SEE {}", n.to_uppercase()));
            let brush = self.tool.sized().then(|| {
                let n = self.size();
                format!("BRUSH {n}x{n}")
            });
            match (see, brush) {
                (Some(a), Some(b)) => format!("{a}   {b}"),
                (Some(a), None) => a,
                (None, Some(b)) => format!("{b} (D-PAD UP/DOWN)"),
                (None, None) => String::new(),
            }
        };
        let kw = text.width(&key, Size::Small);
        p2.rect(g, 0.0, sy - 1.0, W, 13.0, [0.0, 0.0, 0.0, 0.55]);
        let line = text.wrap(&status, Size::Normal, W - kw - 16.0).into_iter().next().unwrap_or_default();
        text.draw(g, ui, 5.0, sy - 1.0, &line, Size::Normal, sc);
        text.draw(g, ui, W - kw - 5.0, sy + 0.5, &key, Size::Small, [0.75, 0.82, 0.72, 1.0]);
        sy + 13.0
    }

    /// One palette button at (bx, by), BTN_W x BTN_H: its picture and label, yellow when armed.
    fn draw_tool(&self, g: &mut Gfx, ui: &Uniforms, app: &App, tool: Tool, bx: f32, by: f32, armed: bool) {
        let p2 = Pen2d { u: ui };
        let rgb = |c: [u8; 3]| [c[0] as f32 / 255.0, c[1] as f32 / 255.0, c[2] as f32 / 255.0, 1.0];
        let panel = [0.3, 0.36, 0.28, 1.0];
        let (x, y, w, h) = (bx + 3.0, by + 2.0, BTN_W - 6.0, BTN_H - 11.0);
        let (mx, my) = (x + w * 0.5, y + h * 0.5);
        if armed {
            p2.rect(g, bx + 1.0, by + 0.5, BTN_W - 2.0, BTN_H - 1.0, YELLOW);
        }
        let locked = matches!(tool, Tool::Building(k) if !app.build_available(k as usize));
            match tool {
                Tool::Paint(ty) => {
                    p2.rect(g, x, y, w, h, rgb(self.colour(ty as i32)));
                    match ty {
                        t::TT_TEE => {
                            p2.rect(g, mx - 7.0, my - 2.0, 4.0, 4.0, WHITE);
                            p2.rect(g, mx + 3.0, my - 2.0, 4.0, 4.0, WHITE);
                        }
                        t::TT_PUTTING_GREEN => {
                            p2.rect(g, mx - 1.0, my - 7.0, 2.0, 14.0, [0.95, 0.95, 0.95, 1.0]);
                            p2.tri(g, (mx + 1.0, my - 7.0), (mx + 8.0, my - 4.0), (mx + 1.0, my - 1.0), [0.9, 0.15, 0.1, 1.0]);
                        }
                        t::TT_WOODS => {
                            let c = [0.1, 0.32, 0.12, 1.0];
                            for ox in [-5.0, 5.0] {
                                p2.tri(g, (mx + ox, my - 7.0), (mx + ox + 5.0, my + 6.0), (mx + ox - 5.0, my + 6.0), c);
                            }
                        }
                        t::TT_DEEP_ROUGH => {
                            for (ox, oy) in [(-6.0, -4.0), (4.0, -2.0), (-3.0, 3.0), (6.0, 4.0)] {
                                p2.rect(g, mx + ox, my + oy - 3.0, 2.0, 5.0, [0.18, 0.3, 0.1, 1.0]);
                            }
                        }
                        _ => {}
                    }
                }
                Tool::Hole => {
                    p2.rect(g, x, y, w, h, rgb(self.colour(t::TT_FAIRWAY as i32)));
                    p2.rect(g, x + 3.0, my + 1.0, 3.0, 3.0, WHITE);
                    p2.rect(g, x + 8.0, my + 1.0, 3.0, 3.0, WHITE);
                    p2.line(g, (x + 9.0, my + 2.0), (mx + 7.0, my - 1.0), 1.0, [1.0, 1.0, 1.0, 0.7]);
                    p2.rect(g, mx + 7.0, y + 1.0, 1.5, h - 4.0, WHITE);
                    p2.tri(g, (mx + 8.5, y + 1.0), (mx + 14.0, y + 3.5), (mx + 8.5, y + 6.0), [0.9, 0.15, 0.1, 1.0]);
                }
                Tool::Raise | Tool::Lower => {
                    p2.rect(g, x, y, w, h, panel);
                    let c = if tool == Tool::Raise { [0.75, 0.95, 0.6, 1.0] } else { [0.95, 0.7, 0.5, 1.0] };
                    let s = if tool == Tool::Raise { -1.0 } else { 1.0 };
                    p2.tri(g, (mx, my + s * 7.0), (mx + 8.0, my - s * 5.0), (mx - 8.0, my - s * 5.0), c);
                }
                Tool::Hand => {
                    p2.rect(g, x, y, w, h, panel);
                    p2.rect(g, mx - 6.0, my - 0.75, 12.0, 1.5, WHITE);
                    p2.rect(g, mx - 0.75, my - 6.0, 1.5, 12.0, WHITE);
                }
                Tool::Path(k) => {
                    p2.rect(g, x, y, w, h, rgb(self.colour(t::TT_FAIRWAY as i32)));
                    let c = if k == 1 { [0.72, 0.68, 0.6, 1.0] } else { [0.45, 0.45, 0.48, 1.0] };
                    p2.quad(g, [(x + 4.0, y + h), (x + 12.0, y), (x + 20.0, y), (x + 12.0, y + h)], c);
                }
                Tool::Item(k) => {
                    p2.rect(g, x, y, w, h, rgb(self.colour(t::TT_ROUGH as i32)));
                    match k {
                        land::K_BENCH => {
                            p2.rect(g, mx - 8.0, my - 2.0, 16.0, 3.0, [0.55, 0.36, 0.2, 1.0]);
                            p2.rect(g, mx - 7.0, my + 1.0, 2.0, 4.0, [0.4, 0.26, 0.14, 1.0]);
                            p2.rect(g, mx + 5.0, my + 1.0, 2.0, 4.0, [0.4, 0.26, 0.14, 1.0]);
                        }
                        land::K_FLOWERS => {
                            for (ox, oy, c) in [(-6.0, -2.0, [0.95, 0.3, 0.4, 1.0]), (0.0, 2.0, [1.0, 0.85, 0.2, 1.0]), (5.0, -3.0, [0.7, 0.4, 0.95, 1.0])] {
                                p2.rect(g, mx + ox - 2.0, my + oy - 2.0, 4.0, 4.0, c);
                            }
                        }
                        land::K_WASHER => {
                            p2.rect(g, mx - 1.0, my - 2.0, 2.0, 8.0, [0.3, 0.3, 0.32, 1.0]);
                            p2.rect(g, mx - 4.0, my - 6.0, 8.0, 5.0, [0.2, 0.45, 0.85, 1.0]);
                        }
                        land::K_WILLOW => {
                            p2.rect(g, mx - 1.0, my, 2.0, 6.0, [0.4, 0.26, 0.14, 1.0]);
                            p2.quad(g, [(mx - 8.0, my + 3.0), (mx - 4.0, my - 7.0), (mx + 4.0, my - 7.0), (mx + 8.0, my + 3.0)], [0.35, 0.6, 0.25, 1.0]);
                        }
                        _ => {
                            p2.rect(g, x, my - 1.0, w, 6.0, [0.25, 0.45, 0.8, 1.0]);
                            p2.quad(g, [(x + 4.0, my + 1.0), (mx, my - 5.0), (mx, my - 3.0), (x + 6.0, my + 1.0)], [0.75, 0.6, 0.4, 1.0]);
                            p2.quad(g, [(mx, my - 5.0), (x + w - 4.0, my + 1.0), (x + w - 6.0, my + 1.0), (mx, my - 3.0)], [0.75, 0.6, 0.4, 1.0]);
                        }
                    }
                }
                Tool::Pick | Tool::Building(_) => {
                    p2.rect(g, x, y, w, h, panel);
                    p2.rect(g, mx - 7.0, my - 1.0, 14.0, 8.0, [0.8, 0.5, 0.35, 1.0]);
                    p2.tri(g, (mx - 9.0, my - 1.0), (mx, my - 8.0), (mx + 9.0, my - 1.0), [0.55, 0.25, 0.2, 1.0]);
                }
                Tool::Turn => {
                    p2.rect(g, x, y, w, h, panel);
                    p2.rect(g, mx - 6.0, my - 6.0, 12.0, 1.5, WHITE);
                    p2.rect(g, mx + 4.5, my - 6.0, 1.5, 10.0, WHITE);
                    p2.tri(g, (mx + 1.0, my + 3.0), (mx + 9.5, my + 3.0), (mx + 5.25, my + 8.0), WHITE);
                }
                Tool::Remove => {
                    p2.rect(g, x, y, w, h, panel);
                    p2.line(g, (mx - 6.0, my - 6.0), (mx + 6.0, my + 6.0), 2.0, [1.0, 0.4, 0.35, 1.0]);
                    p2.line(g, (mx + 6.0, my - 6.0), (mx - 6.0, my + 6.0), 2.0, [1.0, 0.4, 0.35, 1.0]);
                }
                Tool::Hire(k) | Tool::Fire(k) => {
                    let hire = matches!(tool, Tool::Hire(_));
                    p2.rect(g, x, y, w, h, if hire { [0.22, 0.4, 0.24, 1.0] } else { [0.42, 0.22, 0.2, 1.0] });
                    let n = app.employees.iter().filter(|e| e.active && e.job == -2 - k as i8).count();
                    let s = if hire { format!("+ {n}") } else { "-".to_string() };
                    draw_text(g, ui, mx - text_width(&s, 2.0) * 0.5, my - 5.0, &s, 2.0, WHITE);
                }
                Tool::Show(_) | Tool::Save | Tool::Load | Tool::Thoughts => {
                    p2.rect(g, x, y, w, h, panel);
                    let glyph = match tool {
                        Tool::Show(Panel::Money) => "$",
                        Tool::Show(Panel::Holes) => "18",
                        Tool::Show(Panel::Golfers) => "::",
                        Tool::Show(Panel::Staff) => "+",
                        Tool::Show(Panel::Help) => "?",
                        Tool::Save => "S",
                        Tool::Load => "L",
                        _ => "...",
                    };
                    draw_text(g, ui, mx - text_width(glyph, 2.0) * 0.5, my - 5.0, glyph, 2.0, WHITE);
                }
                _ => p2.rect(g, x, y, w, h, panel),
            }
        if locked {
            p2.rect(g, x, y, w, h, [0.0, 0.0, 0.0, 0.55]);
        }
        let label = Self::label(tool);
        let lc = if armed { [0.12, 0.1, 0.02, 1.0] } else if locked { [0.5, 0.52, 0.5, 1.0] } else { [0.88, 0.92, 0.84, 1.0] };
        draw_text(g, ui, bx + (BTN_W - text_width(&label, 1.0)) * 0.5, by + BTN_H - 8.0, &label, 1.0, lc);
    }

    /// The whole course in (x, y, w, h) on the top screen, turned and squashed as the course view is, land off the club's
    /// property dimmed, golfers as dots, and a frame round what the bottom screen shows.
    pub fn render_minimap(&self, g: &mut Gfx, ui: &Uniforms, d: &Demo, (rx, ry, rw, rh): (f32, f32, f32, f32)) {
        let Some(tex) = self.tex else { return };
        let p2 = Pen2d { u: ui };
        let half = (d.terrain.w.max(d.terrain.h) as f32) * TILE_SIZE * 0.5;
        let sinp = (crate::PITCH as f32).to_radians().sin();
        // world to the map: screen right is world +x+z, down +z-x (the view's 45 degree turn), down squashed by the pitch
        let span_u = 2.0 * half * std::f32::consts::SQRT_2;
        let scale = (rw / span_u).min(rh / (span_u * sinp));
        let (cx, cy) = (rx + rw * 0.5, ry + rh * 0.5);
        let mm = |wx: f32, wz: f32| (cx + (wx + wz) * R * scale, cy + (wz - wx) * R * sinp * scale);
        p2.rect(g, rx, ry, rw, rh, [0.03, 0.06, 0.04, 0.85]);
        let (cw, ch) = (d.terrain.w, d.terrain.h);
        let c = [Self::corner_world(d, 0, 0), Self::corner_world(d, cw, 0), Self::corner_world(d, cw, ch), Self::corner_world(d, 0, ch)].map(|(x, z)| mm(x, z));
        let uv = [(0.0, 0.0), (1.0, 0.0), (1.0, 1.0), (0.0, 1.0)];
        let q = [0, 1, 2, 3].map(|i| Vert::new(c[i].0, c[i].1, 0.0, uv[i].0, uv[i].1));
        g.quad(Mode::Flat, Some(tex), ui, q);
        for &(wx, wz) in &self.golfers {
            let (x, y) = mm(wx, wz);
            p2.rect(g, x - 1.0, y - 1.0, 2.0, 2.0, WHITE);
        }
        // what the bottom screen shows: its 320 x 240 pixels at the view's scale, round the camera
        let (fx, fy) = mm(d.cam_x, d.cam_z);
        let (hw, hh) = (VIEW_W * 0.5 * d.upp * scale, VIEW_H * 0.5 * d.upp * scale);
        p2.outline(g, [(fx - hw, fy - hh), (fx + hw, fy - hh), (fx + hw, fy + hh), (fx - hw, fy + hh)], 1.0, [1.0, 0.95, 0.5, 0.9]);
    }

    /// On the course (bottom screen): tiles the game just turned down, a hole's tee waiting for its green, and while the pen
    /// is down the brush's footprint lying on the land (or the corner an elevation tool will move, or the hole being laid
    /// out).
    pub fn render_overlay(&self, g: &mut Gfx, d: &Demo) {
        let u = Uniforms::flat(&d.proj(), &d.view());
        {
            let lift = 3.0;
            let at = |cx: i32, cy: i32| {
                let (a, b) = Self::corner_world(d, cx, cy);
                Vert::new(a, d.terrain.height_at(a, b) + lift, b, 0.0, 0.0)
            };
            for &(tx, ty) in &self.flash {
                let q = [at(tx, ty), at(tx + 1, ty), at(tx + 1, ty + 1), at(tx, ty + 1)].map(|v| v.col(RED));
                g.quad(Mode::Flat, None, &u, q);
            }
            if let (Some((tx, ty)), Tool::Hole) = (self.hole_tee, self.tool) {
                let q = [at(tx, ty), at(tx + 1, ty), at(tx + 1, ty + 1), at(tx, ty + 1)].map(|v| v.col([1.0, 1.0, 1.0, 0.85]));
                g.quad(Mode::Flat, None, &u, q);
            }
        }
        // a still stylus: a ring filling round it until the press becomes a look (Inspect)
        if let (Some(p), Some((px, py))) = (&self.press, self.pen) {
            let held = p.t0.elapsed();
            if !p.stroked && !p.done && held > RING_FROM {
                let f = ((held - RING_FROM).as_secs_f32() / (HOLD - RING_FROM).as_secs_f32()).clamp(0.0, 1.0);
                let ui = Uniforms::flat(&Mat4::ortho(0.0, VIEW_W, VIEW_H, 0.0, -1.0, 1.0), &Mat4::identity());
                let p2 = Pen2d { u: &ui };
                let (r, segs) = (13.0, 24);
                for i in 0..segs {
                    let (a0, a1) = (i as f32 / segs as f32, (i + 1) as f32 / segs as f32);
                    let c = if a1 <= f { [1.0, 0.9, 0.3, 0.95] } else { [0.0, 0.0, 0.0, 0.35] };
                    let ang = |t: f32| (t * std::f32::consts::TAU - std::f32::consts::FRAC_PI_2).sin_cos();
                    let ((s0, c0), (s1, c1)) = (ang(a0), ang(a1));
                    p2.line(g, (px + c0 * r, py + s0 * r), (px + c1 * r, py + s1 * r), 2.5, c);
                }
            }
        }
        let Some((x, y)) = self.pen else { return };
        if self.tool == Tool::Hand || self.press.as_ref().is_some_and(|p| p.pan) {
            return;
        }
        let (wx, wz) = Self::to_world(d, x, y);
        let lift = 3.0;
        let at = |cx: i32, cy: i32| {
            let (a, b) = Self::corner_world(d, cx, cy);
            Vert::new(a, d.terrain.height_at(a, b) + lift, b, 0.0, 0.0)
        };
        let tile_quad = |g: &mut Gfx, x0: i32, y0: i32, s: i32, c: [f32; 4]| {
            let q = [at(x0, y0), at(x0 + s, y0), at(x0 + s, y0 + s), at(x0, y0 + s)].map(|v| v.col(c));
            g.quad(Mode::Flat, None, &u, q);
        };
        g.stereo_world(true);
        match self.tool {
            Tool::Raise | Tool::Lower => {
                let (kx, ky) = d.terrain.corner_of(wx, wz);
                let (a, b) = Self::corner_world(d, kx, ky);
                let h = d.terrain.height_at(a, b) + lift;
                let s = 4.0 * d.upp;
                let c = [1.0, 0.9, 0.2, 0.9];
                g.quad(
                    Mode::Flat,
                    None,
                    &u,
                    [Vert::new(a - s, h, b, 0.0, 0.0).col(c), Vert::new(a, h, b - s, 0.0, 0.0).col(c), Vert::new(a + s, h, b, 0.0, 0.0).col(c), Vert::new(a, h, b + s, 0.0, 0.0).col(c)],
                );
            }
            Tool::Paint(ty) => {
                let (x0, y0, s) = self.brush_tiles(d, wx, wz);
                let k = self.colour(ty as i32);
                let c = [k[0] as f32 / 255.0, k[1] as f32 / 255.0, k[2] as f32 / 255.0, 0.55];
                for row in y0..y0 + s {
                    for tx in x0..x0 + s {
                        if d.terrain.inside(tx, row) {
                            tile_quad(g, tx, row, 1, c);
                        }
                    }
                }
            }
            Tool::Hole => {
                let green = Self::tile_of(d, wx, wz);
                let tee = match (&self.press, self.hole_tee) {
                    (Some(p), _) if p.stroked => Some(p.w0).map(|(x, z)| Self::tile_of(d, x, z)),
                    (_, t) => t,
                };
                if let Some(tee) = tee {
                    tile_quad(g, tee.0, tee.1, 1, [1.0, 1.0, 1.0, 0.8]);
                    let k = self.colour(t::TT_PUTTING_GREEN as i32);
                    tile_quad(g, green.0 - 1, green.1 - 1, 3, [k[0] as f32 / 255.0, k[1] as f32 / 255.0, k[2] as f32 / 255.0, 0.6]);
                } else {
                    tile_quad(g, green.0, green.1, 1, [1.0, 1.0, 1.0, 0.8]);
                }
            }
            Tool::Building(k) => {
                let s = sg_core::data::buildings()[k.clamp(0, 19) as usize].1;
                let t = Self::tile_of(d, wx, wz);
                tile_quad(g, t.0, t.1, s, [1.0, 0.9, 0.2, 0.45]);
            }
            Tool::None => {}
            _ => {
                let t = Self::tile_of(d, wx, wz);
                tile_quad(g, t.0, t.1, 1, [1.0, 0.9, 0.2, 0.45]);
            }
        }
        g.stereo_world(false);
    }
}
