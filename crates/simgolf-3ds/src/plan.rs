//! The bottom screen: the course as a flat plan you draw on with the stylus, and the tools along its right edge.
//!
//! DS and 3DS building games keep the 3D view on top and give the touch screen a flat, readable map to draw on (SimCity
//! DS's zoning map, Theme Park DS's ride placing, Etrian Odyssey's hand-drawn map); moving a cursor over the top screen is
//! what a port without a touch screen has to do. So here:
//!   * The plan (x 0..256) shows the ground the top screen shows, from straight above and turned the same 45 degrees, so up
//!     on the plan is up on the top screen. A white frame marks the top screen's edges. One texel per tile, coloured like
//!     the tile's own texture and shaded by its height and slope, so hills read on the flat map.
//!   * The tool strip (x 256..320): eight terrain brushes, raise, lower, brush size, and a hand that slides the view.
//!   * While the pen is down, the top screen shows the brush's footprint on the 3D land under it, and each stroke is built
//!     into the course as it is drawn.
//! The painting is the demo's: types and heights go straight into sg-core's `Terrain`. Milestone 2 hands the same strokes to
//! the game's `App::paint_tiles` and `edit_elevation`, with their prices, rules and undo.

use crate::gfx::{Gfx, Mat4, Mode, TextureId, Uniforms, Vert};
use crate::shim::TouchPhase;
use crate::app::App;
use crate::{Demo, CLUBHOUSE};
use sg_core::assets::Rgba;
use sg_core::terrain::{self as t, TILE_SIZE};

/// The plan's width; the tool strip fills the rest of the 320-pixel screen.
pub const PLAN_W: f32 = 256.0;
const PLAN_H: f32 = 240.0;
const BTN_W: f32 = 32.0;
const BTN_H: f32 = 40.0;
/// Plan world units per pixel against the top screen's: the top screen's ground fits inside the plan with some of the
/// land round it showing.
const PLAN_ZOOM: f32 = 1.9;
const R: f32 = std::f32::consts::FRAC_1_SQRT_2;
/// The band along the plan's edges where a held pen slides the view, and the fastest it slides (plan pixels a frame).
const EDGE: f32 = 14.0;
const EDGE_SPEED: f32 = 4.0;

#[derive(Clone, Copy, PartialEq, Debug)]
pub enum Tool {
    /// Paints this tile type.
    Paint(u8),
    Raise,
    Lower,
    /// Not armed: a tap cycles the brush through 1, 2 and 3 tiles.
    Size,
    /// Drag to slide the view.
    Hand,
}

/// The strip, two columns by six rows, read left to right.
pub const TOOLS: [Tool; 12] = [
    Tool::Paint(t::TT_FAIRWAY),
    Tool::Paint(t::TT_PUTTING_GREEN),
    Tool::Paint(t::TT_TEE),
    Tool::Paint(t::TT_ROUGH),
    Tool::Paint(t::TT_SAND),
    Tool::Paint(t::TT_WATER_SHALLOW),
    Tool::Paint(t::TT_WOODS),
    Tool::Paint(t::TT_DEEP_ROUGH),
    Tool::Raise,
    Tool::Lower,
    Tool::Size,
    Tool::Hand,
];

/// What a pen event did, for main.rs: the tiles whose look changed (x0, y0, x1, y1 inclusive) and a sound to play.
#[derive(Default)]
pub struct Stroke {
    pub changed: Option<[i32; 4]>,
    pub sound: Option<&'static str>,
    /// The view slid (the pen held at the plan's edge): redraw.
    pub scrolled: bool,
}

impl Stroke {
    fn grow(&mut self, x0: i32, y0: i32, x1: i32, y1: i32) {
        self.changed = Some(match self.changed {
            None => [x0, y0, x1, y1],
            Some(c) => [c[0].min(x0), c[1].min(y0), c[2].max(x1), c[3].max(y1)],
        });
    }
}

pub struct Plan {
    tex: Option<TextureId>,
    /// The terrain changed: the plan's picture is redrawn at the next frame (`refresh`).
    pub stale: bool,
    pub tool: Tool,
    /// The brush's edge in tiles, 1..3.
    pub size: i32,
    /// A message for the status line in place of the tool's name (main.rs: a hole ready to open).
    pub hint: Option<String>,
    /// The pen on the plan while it is down (None on the strip or lifted).
    pub pen: Option<(f32, f32)>,
    /// The press started on the tool strip: the rest of it does nothing.
    on_strip: bool,
    /// The stroke so far: where the brush was last put down (world x, z), and the corner an elevation tool last changed.
    last: Option<(f32, f32)>,
    last_corner: Option<(i32, i32)>,
    /// Each tile type's colour: its texture averaged (`Demo::prepare`), so the plan looks like the land.
    rgb: Vec<[u8; 3]>,
    /// A stroke is under way (its undo point is taken, `App::undo_press`).
    in_stroke: bool,
}


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

impl Tool {
    /// The label under its button (five letters at most) and its full name for the status line.
    fn names(self) -> (&'static str, &'static str) {
        match self {
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
            Tool::Size => ("SIZE", "BRUSH SIZE"),
            Tool::Hand => ("MOVE", "MOVE VIEW"),
        }
    }
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

impl Plan {
    /// `rgb`: per tile type, its texture's average colour where there is one.
    pub fn new(rgb: Vec<Option<[u8; 3]>>) -> Plan {
        let rgb = (0..t::TYPE_COUNT).map(|ty| rgb.get(ty).copied().flatten().unwrap_or_else(|| fallback_rgb(ty as i32))).collect();
        Plan {
            tex: None,
            stale: true,
            tool: Tool::Paint(t::TT_FAIRWAY),
            size: 1,
            hint: None,
            pen: None,
            on_strip: false,
            last: None,
            last_corner: None,
            rgb,
            in_stroke: false,
        }
    }

    fn colour(&self, ty: i32) -> [u8; 3] {
        self.rgb.get(ty as usize).copied().unwrap_or([0, 0, 0])
    }

    // ---- the plan's geometry -------------------------------------------------------------------------------------------

    /// World units per plan pixel.
    fn upp(d: &Demo) -> f32 {
        d.upp * PLAN_ZOOM
    }

    /// Ground depth per plan pixel against width: the plan is squashed up and down as the top screen's tilted view is
    /// (1 / sin of the camera's pitch), so shapes on it look as they do on the top screen and the frame is its shape.
    fn tilt() -> f32 {
        1.0 / (crate::PITCH as f32).to_radians().sin()
    }

    /// Plan pixel to world (x, z): the plan's centre is the camera's ground point; right is world +x+z, down +z-x (the
    /// top screen's 45 degree turn, as main.rs pans), down squashed by the tilt.
    pub fn to_world(d: &Demo, x: f32, y: f32) -> (f32, f32) {
        let (dx, dy) = ((x - PLAN_W * 0.5) * Self::upp(d), (y - PLAN_H * 0.5) * Self::upp(d) * Self::tilt());
        (d.cam_x + (dx - dy) * R, d.cam_z + (dx + dy) * R)
    }

    pub fn to_plan(d: &Demo, wx: f32, wz: f32) -> (f32, f32) {
        let (ex, ez) = (wx - d.cam_x, wz - d.cam_z);
        (PLAN_W * 0.5 + (ex + ez) * R / Self::upp(d), PLAN_H * 0.5 + (ez - ex) * R / (Self::upp(d) * Self::tilt()))
    }

    /// World position of terrain corner (cx, cy).
    fn corner_world(d: &Demo, cx: i32, cy: i32) -> (f32, f32) {
        (
            cx as f32 * TILE_SIZE - d.terrain.w as f32 * TILE_SIZE * 0.5,
            cy as f32 * TILE_SIZE - d.terrain.h as f32 * TILE_SIZE * 0.5,
        )
    }

    /// The tiles the brush covers with its middle at world (wx, wz): the first tile and the edge length.
    fn brush_tiles(&self, d: &Demo, wx: f32, wz: f32) -> (i32, i32, i32) {
        let fx = (wx + d.terrain.w as f32 * TILE_SIZE * 0.5) / TILE_SIZE;
        let fy = (wz + d.terrain.h as f32 * TILE_SIZE * 0.5) / TILE_SIZE;
        let s = self.size;
        // a 1-tile brush is the tile under the pen; a 2-tile one the four round the nearest corner; a 3-tile one centred
        (((fx - s as f32 * 0.5).round()) as i32, ((fy - s as f32 * 0.5).round()) as i32, s)
    }

    fn tool_at(x: f32, y: f32) -> Option<usize> {
        if x < PLAN_W || !(0.0..PLAN_H).contains(&y) {
            return None;
        }
        let (col, row) = (((x - PLAN_W) / BTN_W) as usize, (y / BTN_H) as usize);
        let i = row * 2 + col.min(1);
        (i < TOOLS.len()).then_some(i)
    }

    // ---- input ---------------------------------------------------------------------------------------------------------

    /// A pen event on the bottom screen (its pixels).
    pub fn pen(&mut self, d: &mut Demo, app: &mut App, phase: TouchPhase, x: f32, y: f32) -> Stroke {
        let mut out = Stroke::default();
        match phase {
            TouchPhase::Started => {
                self.last = None;
                self.last_corner = None;
                if let Some(i) = Self::tool_at(x, y) {
                    self.on_strip = true;
                    self.pen = None;
                    match TOOLS[i] {
                        Tool::Size => self.size = self.size % 3 + 1,
                        tool => self.tool = tool,
                    }
                    out.sound = Some("Interface/Button1.wav");
                    return out;
                }
                self.on_strip = false;
                // a tap on a new hole's cup green opens the hole (the desktop's click on it, or H), whatever tool is armed
                let (wx, wz) = Self::to_world(d, x, y);
                if Self::open_at(app, wx, wz) {
                    self.on_strip = true; // the rest of this press does nothing
                    self.pen = None;
                    self.stale = true;
                    return out;
                }
                self.pen = Some((x, y));
                if self.tool != Tool::Hand {
                    self.remember(app);
                    let (wx, wz) = Self::to_world(d, x, y);
                    self.apply(d, app, wx, wz, &mut out);
                    out.sound = self.sound();
                }
            }
            TouchPhase::Moved => {
                if self.on_strip {
                    return out;
                }
                let x = x.min(PLAN_W - 1.0); // a drag onto the strip stays at the plan's edge
                let prev = self.pen.unwrap_or((x, y));
                self.pen = Some((x, y));
                if self.tool == Tool::Hand {
                    // the land follows the pen
                    let (dx, dy) = ((x - prev.0) * Self::upp(d), (y - prev.1) * Self::upp(d) * Self::tilt());
                    d.cam_x -= (dx - dy) * R;
                    d.cam_z -= (dx + dy) * R;
                    return out;
                }
                // the brush is put down every half tile along the pen's path, so a quick stroke leaves no gaps
                let (wx, wz) = Self::to_world(d, x, y);
                let (lx, lz) = self.last.unwrap_or((wx, wz));
                let dist = ((wx - lx).powi(2) + (wz - lz).powi(2)).sqrt();
                let steps = (dist / (TILE_SIZE * 0.5)).ceil().max(1.0) as i32;
                for i in 1..=steps {
                    let f = i as f32 / steps as f32;
                    self.apply(d, app, lx + (wx - lx) * f, lz + (wz - lz) * f, &mut out);
                }
            }
            TouchPhase::Ended | TouchPhase::Cancelled => {
                if self.pen.is_some() {
                    self.finish(app);
                }
                self.pen = None;
                self.last = None;
                self.last_corner = None;
            }
        }
        out
    }

    /// Called every frame: with the pen held near the plan's edge (and a brush armed), the view slides that way and the brush
    /// keeps painting under the pen, so a stroke can run on past what the screens show.
    pub fn tick(&mut self, d: &mut Demo, app: &mut App) -> Stroke {
        let mut out = Stroke::default();
        let Some((x, y)) = self.pen else { return out };
        if self.tool == Tool::Hand {
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
        let (ex, ey) = (band(x, PLAN_W), band(y, PLAN_H));
        if ex == 0.0 && ey == 0.0 {
            return out;
        }
        // up to EDGE_SPEED plan pixels a frame, faster the nearer the edge
        let (dx, dy) = (ex * EDGE_SPEED * Self::upp(d), ey * EDGE_SPEED * Self::upp(d) * Self::tilt());
        d.cam_x += (dx - dy) * R;
        d.cam_z += (dx + dy) * R;
        out.scrolled = true;
        // the land moved under the pen: paint from where the brush last was to where the pen now is
        let (wx, wz) = Self::to_world(d, x, y);
        let (lx, lz) = self.last.unwrap_or((wx, wz));
        let dist = ((wx - lx).powi(2) + (wz - lz).powi(2)).sqrt();
        let steps = (dist / (TILE_SIZE * 0.5)).ceil().max(1.0) as i32;
        for i in 1..=steps {
            let f = i as f32 / steps as f32;
            self.apply(d, app, lx + (wx - lx) * f, lz + (wz - lz) * f, &mut out);
        }
        out
    }

    /// The armed tool once at world (wx, wz) (A on the top screen's cursor).
    pub fn tap_world(&mut self, d: &mut Demo, app: &mut App, wx: f32, wz: f32) -> Stroke {
        let mut out = Stroke::default();
        self.last = None;
        self.last_corner = None;
        if Self::open_at(app, wx, wz) {
            self.stale = true;
        } else if self.tool != Tool::Hand {
            self.remember(app);
            self.apply(d, app, wx, wz, &mut out);
            out.sound = self.sound();
            self.finish(app);
        }
        self.last = None;
        self.last_corner = None;
        out
    }

    /// Opens the hole whose cup green is at this spot, when it is built (a tee and a green) but not yet open.
    fn open_at(app: &mut App, wx: f32, wz: f32) -> bool {
        let (ta, tb) = app.terrain.tile_of(wx, wz);
        // the tapped tile or one near it (a green painted with a big brush has its cup at one corner; a finger is not exact)
        let mut found = None;
        'look: for r in 0..=2 {
            for b in tb - r..=tb + r {
                for a in ta - r..=ta + r {
                    if !sg_core::course::inside(a, b) || app.terrain.ty[app.terrain.tile_index(a, b)] != 1 {
                        continue;
                    }
                    let fl = app.course.flags[sg_core::course::idx(a, b)];
                    let h = (fl & 0x1f) as usize;
                    if fl & sg_core::course::f::CUP != 0 && (1..19).contains(&h) && app.club.holes[h].par == 0 {
                        found = Some(h);
                        break 'look;
                    }
                }
            }
        }
        let Some(h) = found else { return false };
        let _ = h;
        app.open_hole()
    }

    /// The start of a stroke: the game keeps the whole club as it is (money and all) for its undo (undo.rs).
    fn remember(&mut self, app: &mut App) {
        app.undo_press();
        self.in_stroke = true;
    }

    /// The end of a stroke: the game keeps that undo point if the stroke changed the course. B steps back through them
    /// (`App::undo_step`, up to 30).
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
            // the game's elevation edit plays its own
            _ => return None,
        })
    }

    /// Puts the brush down once, through the game: its terrain brush (prices, the property's edge, obstacles, the hole
    /// tool's tees and greens: `App::paint_tiles`) and its Elevation panel's tools (`App::edit_elevation`).
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
                if tiles.is_empty() {
                    return;
                }
                if let Some(i) = crate::app::PAINT.iter().position(|p| p.ty == ty as i32 && p.vbyte == 0) {
                    app.paint_idx = i;
                }
                app.paint_tiles(&tiles);
                out.grow(x0, y0, x0 + s - 1, y0 + s - 1);
                self.stale = true;
            }
            Tool::Raise | Tool::Lower => {
                let (cx, cy) = d.terrain.corner_of(wx, wz);
                if self.last_corner == Some((cx, cy)) || cx < 0 || cy < 0 || cx > d.terrain.w || cy > d.terrain.h {
                    return;
                }
                self.last_corner = Some((cx, cy));
                // the Elevation panel's three tools: one corner, the 2 x 2 block, the corner with the land round it
                app.pstate.elev_tool = match self.size {
                    1 => 0,
                    2 => 1,
                    _ => 2,
                };
                app.edit_elevation(cx, cy, if self.tool == Tool::Raise { 1 } else { -1 });
                // the golfers' land follows the new heights
                if let Some(land) = app.land.as_mut() {
                    land.sync_from_terrain(&app.terrain);
                }
                out.grow(cx - 3, cy - 3, cx + 2, cy + 2);
                self.stale = true;
            }
            Tool::Size | Tool::Hand => {}
        }
    }

    // ---- drawing -------------------------------------------------------------------------------------------------------

    /// Redraws the plan's picture if the course changed. Call after `begin_frame` (the GPU is then done with it).
    pub fn refresh(&mut self, g: &mut Gfx, d: &Demo) {
        if !self.stale && self.tex.is_some() {
            return;
        }
        self.stale = false;
        let tr = &d.terrain;
        let (w, h) = (tr.w as u32, tr.h as u32);
        let mut px = vec![255u8; (w * h * 4) as usize];
        for y in 0..tr.h {
            for x in 0..tr.w {
                let ty = tr.type_at(x, y);
                let c = if ty == 20 { [24, 28, 24] } else { self.colour(ty) };
                // height and slope: higher land lighter, slopes facing the top of the plan lighter, facing down darker
                let l = tr.tile_levels(x, y).map(|v| v as f32); // north-west, north-east, south-west, south-east
                let avg = (l[0] + l[1] + l[2] + l[3]) * 0.25;
                // the plan's up is world -z+x, so the north-east corner is the one toward the top
                let facing = (l[1] - l[2]) * 0.5;
                let shade = (0.74 + 0.06 * avg + 0.2 * facing).clamp(0.4, 1.45);
                let o = ((y as u32 * w + x as u32) * 4) as usize;
                for k in 0..3 {
                    px[o + k] = (c[k] as f32 * shade).min(255.0) as u8;
                }
                px[o + 3] = 255;
            }
        }
        let img = Rgba { w, h, px };
        match self.tex {
            Some(id) => g.update_texture(id, &img),
            None => self.tex = Some(g.texture_crisp(&img)),
        }
    }

    /// The bottom screen: the plan, the top screen's frame, the brush and the tool strip. `cursor` is the top screen's
    /// cursor (shown on the plan while the pen is up).
    pub fn render(&self, g: &mut Gfx, d: &Demo, cursor: (f32, f32), hud: &str) {
        let ui = Uniforms::flat(&Mat4::ortho(0.0, 320.0, 240.0, 0.0, -1.0, 1.0), &Mat4::identity());
        let v = |x: f32, y: f32, c: [f32; 4]| Vert::new(x, y, 0.0, 0.0, 0.0).col(c);
        let quad = |g: &mut Gfx, p: [(f32, f32); 4], c: [f32; 4]| {
            g.quad(Mode::Flat, None, &ui, [v(p[0].0, p[0].1, c), v(p[1].0, p[1].1, c), v(p[2].0, p[2].1, c), v(p[3].0, p[3].1, c)])
        };
        let rect = |g: &mut Gfx, x: f32, y: f32, w: f32, h: f32, c: [f32; 4]| quad(g, [(x, y), (x + w, y), (x + w, y + h), (x, y + h)], c);
        let line = |g: &mut Gfx, a: (f32, f32), b: (f32, f32), w: f32, c: [f32; 4]| {
            let (dx, dy) = (b.0 - a.0, b.1 - a.1);
            let n = (dx * dx + dy * dy).sqrt().max(1e-3);
            let (ox, oy) = (-dy / n * w * 0.5, dx / n * w * 0.5);
            quad(g, [(a.0 + ox, a.1 + oy), (b.0 + ox, b.1 + oy), (b.0 - ox, b.1 - oy), (a.0 - ox, a.1 - oy)], c)
        };
        let outline = |g: &mut Gfx, p: [(f32, f32); 4], w: f32, c: [f32; 4]| {
            for i in 0..4 {
                line(g, p[i], p[(i + 1) % 4], w, c);
            }
        };
        let world = |wx: f32, wz: f32| Self::to_plan(d, wx, wz);

        // the course, one texel per tile
        if let Some(tex) = self.tex {
            let (cw, ch) = (d.terrain.w, d.terrain.h);
            let c = [
                Self::corner_world(d, 0, 0),
                Self::corner_world(d, cw, 0),
                Self::corner_world(d, cw, ch),
                Self::corner_world(d, 0, ch),
            ]
            .map(|(x, z)| world(x, z));
            let uv = [(0.0, 0.0), (1.0, 0.0), (1.0, 1.0), (0.0, 1.0)];
            let q = [0, 1, 2, 3].map(|i| Vert::new(c[i].0, c[i].1, 0.0, uv[i].0, uv[i].1));
            g.quad(Mode::Flat, Some(tex), &ui, q);
        }
        // the clubhouse stand-in's footprint (main.rs render_clubhouse)
        let (hx, hz) = (90.0, 60.0);
        let (cx, cz) = CLUBHOUSE;
        let house = [(cx - hx, cz - hz), (cx + hx, cz - hz), (cx + hx, cz + hz), (cx - hx, cz + hz)].map(|(x, z)| world(x, z));
        quad(g, house, [0.66, 0.26, 0.2, 1.0]);
        outline(g, house, 1.0, [0.95, 0.9, 0.8, 1.0]);
        // what the top screen shows: a 400 x 240 rectangle at the plan's scale (the plan is squashed as the top screen is,
        // so the frame is the top screen's shape; tracing the land under each corner bent it on hills)
        let hw = 200.0 / PLAN_ZOOM;
        let hh = 120.0 / PLAN_ZOOM;
        let (cx, cy) = (PLAN_W * 0.5, PLAN_H * 0.5);
        let top = [(cx - hw, cy - hh), (cx + hw, cy - hh), (cx + hw, cy + hh), (cx - hw, cy + hh)];
        outline(g, top, 1.0, [1.0, 1.0, 1.0, 0.7]);
        // the brush under the pen, or the top screen's cursor
        match self.pen {
            Some((x, y)) if self.tool != Tool::Hand => {
                let (wx, wz) = Self::to_world(d, x, y);
                let c = [1.0, 0.9, 0.2, 1.0];
                match self.tool {
                    Tool::Raise | Tool::Lower => {
                        let (kx, ky) = d.terrain.corner_of(wx, wz);
                        let (px, py) = { let (a, b) = Self::corner_world(d, kx, ky); world(a, b) };
                        rect(g, px - 2.5, py - 2.5, 5.0, 5.0, c);
                    }
                    _ => {
                        let (x0, y0, s) = self.brush_tiles(d, wx, wz);
                        let p = [(x0, y0), (x0 + s, y0), (x0 + s, y0 + s), (x0, y0 + s)].map(|(a, b)| {
                            let (u, w) = Self::corner_world(d, a, b);
                            world(u, w)
                        });
                        outline(g, p, 1.5, c);
                    }
                }
            }
            None => {
                let p = d.pick(cursor.0, cursor.1);
                let (px, py) = world(p[0], p[2]);
                rect(g, px - 1.5, py - 1.5, 3.0, 3.0, [1.0, 1.0, 1.0, 1.0]);
            }
            _ => {}
        }

        // the tool strip
        rect(g, PLAN_W, 0.0, 320.0 - PLAN_W, 240.0, [0.12, 0.16, 0.11, 1.0]);
        for (i, &tool) in TOOLS.iter().enumerate() {
            let (bx, by) = (PLAN_W + (i % 2) as f32 * BTN_W, (i / 2) as f32 * BTN_H);
            // the icon over a one-line label
            let (x, y, w, h) = (bx + 3.0, by + 3.0, BTN_W - 6.0, BTN_H - 13.0);
            let (mx, my) = (x + w * 0.5, y + h * 0.5);
            let armed = tool == self.tool;
            if armed {
                rect(g, bx + 1.0, by + 1.0, BTN_W - 2.0, BTN_H - 2.0, [1.0, 0.85, 0.2, 1.0]);
            }
            let rgb = |c: [u8; 3]| [c[0] as f32 / 255.0, c[1] as f32 / 255.0, c[2] as f32 / 255.0, 1.0];
            match tool {
                Tool::Paint(ty) => {
                    rect(g, x, y, w, h, rgb(self.colour(ty as i32)));
                    match ty {
                        t::TT_TEE => {
                            // two tee markers
                            rect(g, mx - 7.0, my - 2.0, 4.0, 4.0, [1.0, 1.0, 1.0, 1.0]);
                            rect(g, mx + 3.0, my - 2.0, 4.0, 4.0, [1.0, 1.0, 1.0, 1.0]);
                        }
                        t::TT_PUTTING_GREEN => {
                            // the flag
                            rect(g, mx - 1.0, my - 9.0, 2.0, 17.0, [0.95, 0.95, 0.95, 1.0]);
                            g.tris(Mode::Flat, None, &ui, &[v(mx + 1.0, my - 9.0, [0.9, 0.15, 0.1, 1.0]), v(mx + 9.0, my - 5.5, [0.9, 0.15, 0.1, 1.0]), v(mx + 1.0, my - 2.0, [0.9, 0.15, 0.1, 1.0])]);
                        }
                        t::TT_WOODS => {
                            let c = [0.1, 0.32, 0.12, 1.0];
                            for ox in [-5.0, 5.0] {
                                g.tris(Mode::Flat, None, &ui, &[v(mx + ox, my - 8.0, c), v(mx + ox + 6.0, my + 7.0, c), v(mx + ox - 6.0, my + 7.0, c)]);
                            }
                        }
                        t::TT_DEEP_ROUGH => {
                            // tufts
                            let c = [0.18, 0.3, 0.1, 1.0];
                            for (ox, oy) in [(-6.0, -5.0), (4.0, -2.0), (-3.0, 4.0), (6.0, 6.0)] {
                                rect(g, mx + ox, my + oy - 3.0, 2.0, 6.0, c);
                            }
                        }
                        _ => {}
                    }
                }
                Tool::Raise | Tool::Lower => {
                    rect(g, x, y, w, h, [0.3, 0.36, 0.28, 1.0]);
                    let c = if tool == Tool::Raise { [0.75, 0.95, 0.6, 1.0] } else { [0.95, 0.7, 0.5, 1.0] };
                    let s = if tool == Tool::Raise { -1.0 } else { 1.0 };
                    g.tris(Mode::Flat, None, &ui, &[v(mx, my + s * 8.0, c), v(mx + 8.0, my - s * 5.0, c), v(mx - 8.0, my - s * 5.0, c)]);
                }
                Tool::Size => {
                    rect(g, x, y, w, h, [0.3, 0.36, 0.28, 1.0]);
                    // the brush as tiles: 1, 2 x 2 or 3 x 3
                    let n = self.size;
                    let cell = 5.0;
                    let span = n as f32 * cell + (n - 1) as f32;
                    for j in 0..n {
                        for i in 0..n {
                            rect(g, mx - span * 0.5 + i as f32 * (cell + 1.0), my - span * 0.5 + j as f32 * (cell + 1.0), cell, cell, [1.0, 1.0, 1.0, 1.0]);
                        }
                    }
                }
                Tool::Hand => {
                    rect(g, x, y, w, h, [0.3, 0.36, 0.28, 1.0]);
                    let c = [1.0, 1.0, 1.0, 1.0];
                    rect(g, mx - 6.0, my - 1.0, 12.0, 2.0, c);
                    rect(g, mx - 1.0, my - 6.0, 2.0, 12.0, c);
                    for (dx, dy) in [(1.0, 0.0), (-1.0, 0.0), (0.0, 1.0), (0.0, -1.0)] {
                        let (tx, ty) = (mx + dx * 10.0, my + dy * 10.0);
                        let (px, py) = (-dy * 3.5, dx * 3.5);
                        g.tris(Mode::Flat, None, &ui, &[v(tx, ty, c), v(tx - dx * 4.0 + px, ty - dy * 4.0 + py, c), v(tx - dx * 4.0 - px, ty - dy * 4.0 - py, c)]);
                    }
                }
            }
            let label = tool.names().0;
            let lc = if armed { [0.12, 0.1, 0.02, 1.0] } else { [0.88, 0.92, 0.84, 1.0] };
            draw_text(g, &ui, bx + (BTN_W - text_width(label, 1.0)) * 0.5, by + BTN_H - 9.0, label, 1.0, lc);
        }

        // the status line over the plan's top-left corner: the armed tool and the brush
        let status = match (&self.hint, self.tool) {
            (Some(h), _) => h.clone(),
            (None, Tool::Hand) => self.tool.names().1.to_string(),
            (None, tool) => format!("{}  {}X{}", tool.names().1, self.size, self.size),
        };
        let sw = text_width(&status, 2.0);
        rect(g, 0.0, 0.0, sw + 8.0, 14.0, [0.0, 0.0, 0.0, 0.55]);
        let sc = if self.hint.is_some() { [1.0, 0.86, 0.3, 1.0] } else { [1.0, 1.0, 1.0, 1.0] };
        draw_text(g, &ui, 4.0, 2.0, &status, 2.0, sc);
        // the money and the date along the plan's foot (main.rs hud_line)
        if !hud.is_empty() {
            let hw = text_width(hud, 2.0);
            rect(g, PLAN_W - hw - 8.0, PLAN_H - 14.0, hw + 8.0, 14.0, [0.0, 0.0, 0.0, 0.55]);
            let red = hud.starts_with('-');
            draw_text(g, &ui, PLAN_W - hw - 4.0, PLAN_H - 12.0, hud, 2.0, if red { [1.0, 0.45, 0.4, 1.0] } else { [1.0, 0.95, 0.6, 1.0] });
        }
    }

    /// On the top screen while the pen is down: the brush's footprint lying on the 3D land (or the corner an elevation tool
    /// will move), drawn in the world so it has the land's depth in 3D.
    pub fn render_top(&self, g: &mut Gfx, d: &Demo) {
        let Some((x, y)) = self.pen else { return };
        if self.tool == Tool::Hand {
            return;
        }
        let (wx, wz) = Self::to_world(d, x, y);
        let u = Uniforms::flat(&d.proj(), &d.view());
        let lift = 3.0;
        let at = |cx: i32, cy: i32| {
            let (a, b) = Self::corner_world(d, cx, cy);
            Vert::new(a, d.terrain.height_at(a, b) + lift, b, 0.0, 0.0)
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
                    [
                        Vert::new(a - s, h, b, 0.0, 0.0).col(c),
                        Vert::new(a, h, b - s, 0.0, 0.0).col(c),
                        Vert::new(a + s, h, b, 0.0, 0.0).col(c),
                        Vert::new(a, h, b + s, 0.0, 0.0).col(c),
                    ],
                );
            }
            Tool::Paint(ty) => {
                let (x0, y0, s) = self.brush_tiles(d, wx, wz);
                let k = self.colour(ty as i32);
                let c = [k[0] as f32 / 255.0, k[1] as f32 / 255.0, k[2] as f32 / 255.0, 0.55];
                for row in y0..y0 + s {
                    for tx in x0..x0 + s {
                        if !d.terrain.inside(tx, row) {
                            continue;
                        }
                        let q = [at(tx, row), at(tx + 1, row), at(tx + 1, row + 1), at(tx, row + 1)].map(|v| v.col(c));
                        g.quad(Mode::Flat, None, &u, q);
                    }
                }
                // its edge, bright, so it reads against any ground
                let e = [1.0, 0.9, 0.2, 1.0];
                let w = 1.2 * d.upp;
                let corners = [(x0, y0), (x0 + s, y0), (x0 + s, y0 + s), (x0, y0 + s)];
                for i in 0..4 {
                    let (a, b) = (corners[i], corners[(i + 1) % 4]);
                    let (pa, pb) = (at(a.0, a.1), at(b.0, b.1));
                    // a thin strip along the edge, on the ground (the edge's own heights; good enough over a brush's span)
                    let (dx, dz) = (pb.pos[0] - pa.pos[0], pb.pos[2] - pa.pos[2]);
                    let n = (dx * dx + dz * dz).sqrt().max(1e-3);
                    let (ox, oz) = (-dz / n * w, dx / n * w);
                    let p = |v: Vert, sx: f32| Vert::new(v.pos[0] + ox * sx, v.pos[1], v.pos[2] + oz * sx, 0.0, 0.0).col(e);
                    g.quad(Mode::Flat, None, &u, [p(pa, 1.0), p(pb, 1.0), p(pb, -1.0), p(pa, -1.0)]);
                }
            }
            _ => {}
        }
        g.stereo_world(false);
    }
}
