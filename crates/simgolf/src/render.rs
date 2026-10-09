//! Drawing: the course (terrain, paths, walls), sprites in painter's order, the ball, the edit cursor, and the 2D screens (title
//! menu, property chooser, heads-up display with the dock, course report).
use crate::app::*;
use crate::gfx::{Gfx, Mat4, Mode, Uniforms, Vert};
use crate::ui::{money, rgb, rgba, text_width, wrap_text, Screen as Ui};
use sg_core::economy::{Economy, STAFF_KINDS};
use sg_core::land;
use sg_core::properties::{PROPERTIES, START_FUNDS};
use sg_core::sprites::SPRITE_UNITS_PER_PIXEL;
use sg_core::terrain::{DEPTH_RANGE, TILE_SIZE};

#[derive(Clone, Copy)]
pub struct Rect {
    pub x: f32,
    pub y: f32,
    pub w: f32,
    pub h: f32,
}

impl Rect {
    pub const fn new(x: f32, y: f32, w: f32, h: f32) -> Rect {
        Rect { x, y, w, h }
    }
    pub fn has(&self, px: f32, py: f32) -> bool {
        px >= self.x && py >= self.y && px < self.x + self.w && py < self.y + self.h
    }
}

// ---- the course ---------------------------------------------------------------------------------------------------------------

impl App {
    /// Draws the course and everything on it. Camera as the original: ortho +-5000 depth, pitch about X, then yaw 45 degrees.
    pub fn render_world(&mut self, g: &mut Gfx) {
        let upp = 2.0 / (self.zoom * self.dpi); // world units per drawable pixel
        let (hw, hh) = (self.draw_w * upp * 0.5, self.draw_h * upp * 0.5);
        let proj = Mat4::ortho(-hw, hw, -hh, hh, -DEPTH_RANGE, DEPTH_RANGE);
        let mv = Mat4::rotate(pitch_for(self.draw_w, self.draw_h), 1.0, 0.0, 0.0)
            .mul(&Mat4::rotate((45.0 + self.rot) as f64, 0.0, 1.0, 0.0))
            .mul(&Mat4::translate(-self.cam_x, 0.0, -self.cam_z));
        // Directional light fixed in eye space. APPROXIMATION: the original's light setup is not decoded.
        let mut lit = Uniforms::flat(&proj, &mv);
        lit.lit = 1.0;
        lit.light_amb = self.light.ambient.map(|v| v * 0.6);
        lit.light_dif = self.light.diffuse.map(|v| v * 0.55);
        for i in 0..self.batches.len() {
            let mut u = lit;
            if self.batches[i].water {
                // APPROXIMATION: water shimmers by drifting its texture a pixel or so; the original's water animation is not decoded.
                u.uv_offset = [0.014 * (self.time as f32 * 1.3).sin(), 0.014 * (self.time as f32 * 0.9).cos()];
            }
            let b = &self.batches[i];
            g.draw_mesh(Mode::Solid, &b.mesh, b.tex, &u);
        }
        for b in &self.wall_batches {
            g.draw_mesh(Mode::Solid, &b.mesh, b.tex, &lit);
        }
        let mut overlay = lit;
        overlay.alpha_ref = 0.04;
        for b in self.path_batches.iter().chain(&self.mud_batches) {
            g.draw_mesh(Mode::Overlay, &b.mesh, b.tex, &overlay);
        }
        self.mv = mv.0;
        self.upp = upp;
        self.draw_props(g, &Uniforms::flat(&proj, &mv));
        if self.edit && self.has_hit {
            self.draw_cursor(g, &Uniforms::flat(&proj, &mv));
        }
        self.draw_aim(g, &Uniforms::flat(&proj, &mv));
        g.flush();
    }

    /// The pro's aim line (0x41bbe1): from the ball, bending with the shot type, drawn white over a dark copy; and a box on
    /// the aimed tile.
    fn draw_aim(&mut self, g: &mut Gfx, u: &Uniforms) {
        let (Some(gi), Some(p)) = (self.club.pro_aiming(), self.aim) else { return };
        let gg = &self.club.g[gi];
        let ball = (gg.bx, gg.by);
        let opt = self.club.planner.option;
        let line = sg_core::pro::aim_line(ball, &p, opt);
        let n = line.len().max(2) as f32 - 1.0;
        let lift = match opt {
            3 => 3.0,
            4 => 0.0,
            _ => 2.0,
        } * p.distance as f32
            / 24.0;
        let pts: Vec<[f32; 3]> = line
            .iter()
            .enumerate()
            .map(|(i, &(x, y))| {
                let (wx, wz) = self.units_to_world(x, y);
                let t = i as f32 / n;
                // the arc's height: zero at both ends (an approximation of the exe's per-point screen lift)
                [wx, self.terrain.height_at(wx, wz) + 4.0 + lift * 4.0 * t * (1.0 - t) * TILE_SIZE / 8.0, wz]
            })
            .collect();
        self.draw_lines(g, u, &pts, false, 2.5, [0.0, 0.0, 0.0, 0.6]);
        self.draw_lines(g, u, &pts, false, 1.2, [1.0, 1.0, 1.0, 1.0]);
        let (wx, wz) = self.units_to_world(p.x, p.y);
        let r = TILE_SIZE * 0.5;
        let corners = [(wx - r, wz - r), (wx + r, wz - r), (wx + r, wz + r), (wx - r, wz + r)];
        let bx: Vec<[f32; 3]> = corners.iter().map(|&(x, z)| [x, self.terrain.height_at(x, z) + 3.0, z]).collect();
        self.draw_lines(g, u, &bx, true, 1.0, [1.0, 0.95, 0.3, 1.0]);
    }

    fn sprite_texture(&mut self, g: &mut Gfx, si: usize, view: i32, frame: i32) -> miniquad::TextureId {
        let i = self.sprites[si].s.frame_index(view, frame);
        if let Some(t) = self.sprites[si].tex[i] {
            return t;
        }
        let t = g.texture(&self.sprites[si].s.frames[i], false);
        self.sprites[si].tex[i] = Some(t);
        t
    }

    /// Props as camera-facing quads. Sprites are pre-rendered, so they ignore lighting and depth: ground overlays first, then
    /// shadows, then bodies far to near (painter's order).
    fn draw_props(&mut self, g: &mut Gfx, u: &Uniforms) {
        if !self.show_props || self.props.is_empty() {
            return;
        }
        self.update_props();
        if self.follow {
            if let Some(gl) = self.club.g.iter().take(sg_core::golfer::SLOTS).find(|g| g.hole > 0 && g.hole < 19) {
                let (x, z) = self.units_to_world(gl.x, gl.y);
                self.cam_x = x;
                self.cam_z = z;
            }
        }
        let mv = self.mv;
        let (rx, ry, rz) = (mv[0], mv[4], mv[8]); // eye X axis in world space
        let (ux, uy, uz) = (mv[1], mv[5], mv[9]); // eye Y axis in world space
                                                  // The sprites were rendered at 4 camera yaws, 90 degrees apart; 8 view sprites (people) face 45 degree steps.
        let quarter = ((self.rot / 90.0).round() as i32 % 4 + 4) % 4;
        let s = SPRITE_UNITS_PER_PIXEL;
        let mut items: Vec<(f32, usize)> = Vec::new();
        for (i, p) in self.props.iter().enumerate() {
            if p.hidden {
                continue;
            }
            let y = self.terrain.height_at(p.x, p.z);
            items.push((mv[2] * p.x + mv[6] * y + mv[10] * p.z, i));
        }
        items.sort_by(|a, b| a.0.partial_cmp(&b.0).unwrap_or(std::cmp::Ordering::Equal));
        let quad = |app: &mut App, g: &mut Gfx, si: Option<usize>, pi: usize, shadow: bool| {
            let Some(si) = si else { return };
            let p = app.props[pi].clone();
            let sp = &app.sprites[si].s;
            let views = sp.views;
            let mut view = if views >= 8 {
                (quarter * 2) % 8
            } else if views >= 4 {
                (quarter + p.facing).rem_euclid(views)
            } else if views == 2 {
                quarter % 2
            } else {
                0
            };
            if views >= 8 && p.heading > -999.0 {
                // View 0 faces world -X (heading 180); each further view turns 45 degrees counter clockwise on screen, i.e.
                // heading - 45; a 90 degree camera turn adds 2 views.
                let k = ((180.0 - p.heading) / 45.0).round() as i32;
                view = ((k + 2 * quarter) % 8 + 8) % 8;
            }
            let (w, h, ax, ay) = (sp.w as f32, sp.h as f32, sp.anchor_x as f32, sp.anchor_y as f32);
            let tex = app.sprite_texture(g, si, view, p.frame);
            let y = app.terrain.height_at(p.x, p.z) + if shadow { 0.0 } else { p.lift };
            let s = s * p.scale;
            let (l, r) = (-ax * s, (w - ax) * s);
            let (t, b) = (ay * s, -(h - ay) * s); // up is positive
            let c = |cx: f32, cy: f32, tu: f32, tv: f32| {
                Vert::new(p.x + rx * cx + ux * cy, y + ry * cx + uy * cy, p.z + rz * cx + uz * cy, tu, tv)
            };
            g.quad(Mode::Flat, Some(tex), u, [c(l, t, 0.0, 0.0), c(r, t, 1.0, 0.0), c(r, b, 1.0, 1.0), c(l, b, 0.0, 1.0)]);
        };
        for &(_, i) in &items {
            if self.props[i].flat {
                let b = self.props[i].body;
                quad(self, g, b, i, false);
            }
        }
        for &(_, i) in &items {
            let sh = self.props[i].shadow;
            quad(self, g, sh, i, true);
        }
        for &(_, i) in &items {
            if !self.props[i].flat {
                let b = self.props[i].body;
                quad(self, g, b, i, false);
            }
        }
        // The ball: a small white disc with a dark disc on the ground below it.
        // Height above the ground is in the exe's z units, 16 to a height step (15 world units here).
        let balls: Vec<(f32, f32, f32)> = self
            .club
            .g
            .iter()
            .filter(|gl| gl.bx != 0 && gl.hole > 0 && gl.hole < 19 && gl.flags & sg_core::golfer::flag::PENALTY == 0)
            .map(|gl| {
                let (x, z) = self.units_to_world(gl.bx, gl.by);
                (x, z, gl.bz as f32 * sg_core::terrain::HEIGHT_STEP / 16.0)
            })
            .collect();
        for (bx, bz, bh) in balls {
            let gy = self.terrain.height_at(bx, bz);
            let disc = |g: &mut Gfx, x: f32, y: f32, z: f32, rpx: f32, col: [f32; 4], upright: bool| {
                let pt = |i: i32| {
                    let a = i as f32 * std::f32::consts::TAU / 12.0;
                    let (cx, cy) = (a.cos() * rpx, a.sin() * rpx);
                    if upright {
                        Vert::new(x + rx * cx + ux * cy, y + ry * cx + uy * cy, z + rz * cx + uz * cy, 0.0, 0.0).col(col)
                    } else {
                        Vert::new(x + cx, y, z + cy * 0.8, 0.0, 0.0).col(col)
                    }
                };
                let centre = Vert::new(x, y, z, 0.0, 0.0).col(col);
                let mut v = Vec::with_capacity(36);
                for i in 0..12 {
                    v.extend_from_slice(&[centre, pt(i), pt(i + 1)]);
                }
                g.tris(Mode::Flat, None, u, &v);
            };
            disc(g, bx, gy + 1.0, bz, 4.5, [0.0, 0.0, 0.0, 0.45], false);
            disc(g, bx, gy + bh + 4.0, bz, 5.0, [1.0; 4], true);
        }
    }

    /// The brush outline on the ground, two pixels wide.
    fn draw_cursor(&mut self, g: &mut Gfx, u: &Uniforms) {
        let t = &self.terrain;
        let (ox, oz) = (-t.w as f32 * TILE_SIZE * 0.5, -t.h as f32 * TILE_SIZE * 0.5);
        let r = self.brush as f32;
        let (x0, z0, x1, z1) = if self.tool != 1 {
            let (tx, ty) = t.tile_of(self.hit_x, self.hit_z);
            (
                ox + (tx as f32 - r) * TILE_SIZE,
                oz + (ty as f32 - r) * TILE_SIZE,
                ox + (tx as f32 + r + 1.0) * TILE_SIZE,
                oz + (ty as f32 + r + 1.0) * TILE_SIZE,
            )
        } else {
            let (cx, cy) = t.corner_of(self.hit_x, self.hit_z);
            (
                ox + (cx as f32 - r - 0.5) * TILE_SIZE,
                oz + (cy as f32 - r - 0.5) * TILE_SIZE,
                ox + (cx as f32 + r + 0.5) * TILE_SIZE,
                oz + (cy as f32 + r + 0.5) * TILE_SIZE,
            )
        };
        const N: i32 = 8;
        let pt = |x: f32, z: f32| {
            let (x, z) = (x.clamp(ox, -ox), z.clamp(oz, -oz));
            [x, t.height_at(x, z) + 3.0, z]
        };
        let mut pts = Vec::new();
        for i in 0..N {
            pts.push(pt(x0 + (x1 - x0) * i as f32 / N as f32, z0));
        }
        for i in 0..N {
            pts.push(pt(x1, z0 + (z1 - z0) * i as f32 / N as f32));
        }
        for i in 0..N {
            pts.push(pt(x1 - (x1 - x0) * i as f32 / N as f32, z1));
        }
        for i in 0..N {
            pts.push(pt(x0, z1 - (z1 - z0) * i as f32 / N as f32));
        }
        self.draw_lines(g, u, &pts, true, 1.0, [1.0, 0.95, 0.3, 1.0]);
    }

    /// A line through world points, `px` pixels either side, closed into a loop or not.
    fn draw_lines(&self, g: &mut Gfx, u: &Uniforms, pts: &[[f32; 3]], closed: bool, px: f32, col: [f32; 4]) {
        let mv = self.mv;
        let (rx, ry, rz, ux, uy, uz) = (mv[0], mv[4], mv[8], mv[1], mv[5], mv[9]);
        let half = self.upp * px;
        let mut v = Vec::new();
        let n = if closed { pts.len() } else { pts.len().saturating_sub(1) };
        for i in 0..n {
            let (a, b) = (pts[i], pts[(i + 1) % pts.len()]);
            let d = [b[0] - a[0], b[1] - a[1], b[2] - a[2]];
            let (sx, sy) = (rx * d[0] + ry * d[1] + rz * d[2], ux * d[0] + uy * d[1] + uz * d[2]);
            let l = (sx * sx + sy * sy).sqrt().max(1e-6);
            let (px, py) = (-sy / l * half, sx / l * half);
            let off = [rx * px + ux * py, ry * px + uy * py, rz * px + uz * py];
            let q = |p: [f32; 3], s: f32| Vert::new(p[0] + off[0] * s, p[1] + off[1] * s, p[2] + off[2] * s, 0.0, 0.0).col(col);
            let (a0, a1, b0, b1) = (q(a, -1.0), q(a, 1.0), q(b, -1.0), q(b, 1.0));
            v.extend_from_slice(&[a0, b0, b1, a0, b1, a1]);
        }
        g.tris(Mode::Flat, None, u, &v);
    }
}

// ---- screens ------------------------------------------------------------------------------------------------------------------

/// Regions of the 800x600 title art (read off TitleBASE / TitleUnSel / TitleMO in the disc's Interface folder).
pub const MENU_BTN: [Rect; 6] = [
    Rect::new(40.0, 25.0, 325.0, 120.0),
    Rect::new(415.0, 40.0, 360.0, 135.0),
    Rect::new(12.0, 395.0, 285.0, 100.0),
    Rect::new(508.0, 380.0, 270.0, 95.0),
    Rect::new(285.0, 478.0, 285.0, 105.0),
    Rect::new(718.0, 528.0, 64.0, 64.0),
];
/// The folders under Themes/.
pub const THEME_PACKS: [&str; 5] = ["Standard", "Firaxis", "More Stories", "The Sims", "Championship"];
const MENU_LABEL: [&str; 5] = ["Continue Saved Game", "Start New Game (Standard)", "Sandbox Mode", "Select A Theme", "Play a Championship"];
const MENU_LABEL_X: [f32; 5] = [180.0, 595.0, 150.0, 650.0, 418.0];
const MENU_LABEL_Y: [f32; 5] = [83.0, 112.0, 450.0, 427.0, 543.0];
const INK: [f32; 4] = [0.12, 0.12, 0.38, 1.0];

pub fn property_card(i: usize) -> Rect {
    const LX: [f32; 10] = [250.0, 301.0, 345.0, 383.0, 416.0, 437.0, 453.0, 464.0, 468.0, 462.0];
    const LW: [f32; 10] = [175.0, 173.0, 174.0, 174.0, 173.0, 173.0, 174.0, 176.0, 173.0, 174.0];
    const LY: [f32; 10] = [12.0, 64.0, 115.0, 167.0, 220.0, 277.0, 337.0, 398.0, 457.0, 520.0];
    const RX: [f32; 6] = [465.0, 508.0, 545.0, 579.0, 605.0, 622.0];
    const RY: [f32; 6] = [12.0, 64.0, 115.0, 167.0, 220.0, 277.0];
    let p = &PROPERTIES[i];
    let idx = PROPERTIES[..i].iter().filter(|q| q.column == p.column).count();
    if p.column == 0 {
        Rect::new(LX[idx], LY[idx], LW[idx], 48.0)
    } else {
        Rect::new(RX[idx], RY[idx], 175.0, 48.0)
    }
}

pub const BACK_BUTTON: Rect = Rect::new(748.0, 538.0, 46.0, 46.0);
/// Our own control on the property chooser (the original picks the difficulty at the start of a game; its screen is not drawn yet).
pub const DIFFICULTY_BUTTON: Rect = Rect::new(20.0, 46.0, 220.0, 22.0);
/// The four difficulties the manual names, in the exe's order (0 easiest).
pub const DIFFICULTY_NAMES: [&str; 4] = ["Easy", "Moderate", "Difficult", "Impossible"];

/// The lower left dock: centre and radius of each button; its normal sprite on the sheet; the offset to the highlighted one.
pub struct DockBtn {
    pub cx: f32,
    pub cy: f32,
    pub r: f32,
    pub sx: f32,
    pub sy: f32,
    pub sw: f32,
    pub sh: f32,
    pub hover_dx: f32,
}
const fn db(cx: f32, cy: f32, r: f32, sx: f32, sy: f32, sw: f32, sh: f32, hover_dx: f32) -> DockBtn {
    DockBtn { cx, cy, r, sx, sy, sw, sh, hover_dx }
}
/// Measured from Interface/3mainLowerLeft.pcx (the assembled dock sits in the bottom left of the sheet, with the highlighted
/// versions of each button elsewhere on it). The panels, advisor text and layout are our own.
pub const DOCK: [DockBtn; 10] = [
    db(43.0, 473.0, 33.0, 0.0, 0.0, 76.0, 80.0, 100.0),     // Build Course
    db(117.0, 497.0, 30.0, 0.0, 100.0, 70.0, 78.0, 100.0),  // Add Buildings
    db(177.0, 536.0, 26.0, 0.0, 200.0, 64.0, 76.0, 100.0),  // People
    db(32.0, 543.0, 11.0, 598.0, 48.0, 30.0, 30.0, 50.0),   // zoom in
    db(31.0, 585.0, 11.0, 598.0, 98.0, 30.0, 30.0, 50.0),   // zoom out
    db(17.0, 565.0, 11.0, 598.0, 148.0, 30.0, 30.0, 50.0),  // rotate right
    db(47.0, 565.0, 11.0, 598.0, 198.0, 30.0, 30.0, 50.0),  // rotate left
    db(75.0, 552.0, 14.0, 598.0, 248.0, 34.0, 34.0, 50.0),  // information (course report)
    db(107.0, 568.0, 14.0, 598.0, 298.0, 34.0, 34.0, 50.0), // pause
    db(133.0, 583.0, 13.0, 598.0, 348.0, 34.0, 34.0, 50.0), // tools (save the course)
];
pub const DOCK_HELP: [&str; 10] = [
    "Build Course",
    "Add Buildings",
    "People",
    "Zoom in",
    "Zoom out",
    "Rotate right",
    "Rotate left",
    "Course report",
    "Pause",
    "Save the course",
];

/// The pro's shot buttons (panel buttons 4..8) and their option values and keys.
pub const SHOT_NAMES: [&str; 5] = ["Straight shot", "Fade shot (L to R)", "Draw shot (R to L)", "High backspin shot", "Low punch shot"];
pub const SHOT_OPTS: [i32; 5] = [0, -1, 1, 3, 4];
pub const SHOT_KEYS: [&str; 5] = ["S", "F", "D", "H", "L"];

/// Items in the open panel. kind: 0 paint (arg PAINT index), 1 path, 2 raise, 3 lower, 4 building (arg BUILD index), 5 staff.
pub struct PanelItem {
    pub label: String,
    pub kind: i32,
    pub arg: usize,
}

pub fn dock_hit(vx: f32, vy: f32) -> i32 {
    DOCK.iter().position(|b| (vx - b.cx).powi(2) + (vy - b.cy).powi(2) <= b.r * b.r).map(|i| i as i32).unwrap_or(-1)
}

pub fn panel_item_rect(i: usize, cols: usize) -> Rect {
    let cw = if cols == 1 { 540.0 } else { 182.0 };
    Rect::new(232.0 + (i % cols) as f32 * cw, 462.0 + (i / cols) as f32 * 18.5, cw - 4.0, 17.0)
}

const MONTHS: [&str; 12] =
    ["January", "February", "March", "April", "May", "June", "July", "August", "September", "October", "November", "December"];

impl App {
    pub fn panel_cols(&self) -> usize {
        if self.panel == 3 {
            1
        } else {
            3
        }
    }

    pub fn panel_items(&self) -> Vec<PanelItem> {
        let mut v = Vec::new();
        match self.panel {
            1 => {
                for (i, p) in PAINT.iter().enumerate().take(15) {
                    v.push(PanelItem {
                        label: format!("{}  {}", p.name, money(Economy::terrain_cost_units(p.ty) as i64 * 100)),
                        kind: 0,
                        arg: i,
                    });
                }
                v.push(PanelItem { label: "Path, gravel".into(), kind: 1, arg: 1 });
                v.push(PanelItem { label: "Path, paved".into(), kind: 1, arg: 2 });
                v.push(PanelItem { label: "Raise ground".into(), kind: 2, arg: 0 });
                v.push(PanelItem { label: "Lower ground".into(), kind: 3, arg: 0 });
            }
            2 => {
                for &k in OFFERED_KINDS.iter() {
                    let k = k as usize;
                    if self.build_available(k) {
                        let (name, _, price) = sg_core::land::BUILDINGS[k];
                        v.push(PanelItem { label: format!("{name}  {}", money(price as i64 * 100)), kind: 4, arg: k });
                    }
                }
            }
            3 if self.club.pro_aiming().is_some() => {
                // the pro waits for the aim: the five shot buttons (panel buttons 4..8)
                for (i, name) in SHOT_NAMES.iter().enumerate() {
                    v.push(PanelItem { label: format!("{name}  ({})", SHOT_KEYS[i]), kind: 7, arg: i });
                }
                v.push(PanelItem { label: "Click the course to aim and swing.  N twice cancels the round.".into(), kind: 8, arg: 0 });
            }
            3 => {
                let pro = self.pro_name();
                if self.club.gary == -1 {
                    v.push(PanelItem { label: format!("{pro}: Practice Round"), kind: 6, arg: 0 });
                    let m = if self.club.game & sg_core::pro::CHALLENGE != 0 && self.club.challenge_pro >= 0 {
                        let who = self.club.pros.get(self.club.challenge_pro as usize).map(|p| p.name.clone()).unwrap_or_default();
                        format!("{pro}: Match vs. {who}  (${}/hole)", self.club.wager_level * 2000)
                    } else {
                        format!("{pro}: Match vs. a pro  (waiting for a challenge)")
                    };
                    v.push(PanelItem { label: m, kind: 6, arg: 1 });
                    if self.club.can_begin_tournament() {
                        v.push(PanelItem { label: format!("{pro}: Begin Tournament"), kind: 6, arg: 4 });
                    }
                } else if self.club.game & sg_core::golfer::game::TOURNAMENT != 0 {
                    v.push(PanelItem { label: "Tournament under way: cancel it (no prizes)".into(), kind: 6, arg: 3 });
                } else {
                    v.push(PanelItem { label: format!("{pro} is playing: cancel the round"), kind: 6, arg: 3 });
                }
                let pts = if self.club.skill_points > 0 { format!("  ({} points to add)", self.club.skill_points) } else { String::new() };
                v.push(PanelItem { label: format!("{pro}'s skills{pts}"), kind: 6, arg: 2 });
                v.push(PanelItem { label: "Golfers on the course".into(), kind: 9, arg: 0 });
                for k in 0..STAFF_KINDS {
                    v.push(PanelItem {
                        label: format!("{}: {}  (click to hire, right click to fire)", Economy::staff_name(k), self.econ.staff[k]),
                        kind: 5,
                        arg: k,
                    });
                }
            }
            _ => {}
        }
        v
    }

    pub fn draw_menu(&mut self, g: &mut Gfx) {
        let s = Ui::new(self.draw_w, self.draw_h);
        self.view = s.view;
        s.image(g, &self.title_base, 0.0, 0.0);
        s.image_part(g, &self.title_mo, 170.0, 190.0, 170.0, 190.0, 480.0, 165.0); // the logo sits in the highlight layer
        s.image(g, &self.title_un, 0.0, 0.0);
        if (0..6).contains(&self.hover) {
            let r = MENU_BTN[self.hover as usize];
            s.image_part(g, &self.title_mo, r.x, r.y, r.x, r.y, r.w, r.h);
        }
        for b in 0..5 {
            s.text_centered(g, MENU_LABEL_X[b], MENU_LABEL_Y[b], MENU_LABEL[b], 19.0, INK);
        }
        s.text_centered(g, MENU_LABEL_X[3], MENU_LABEL_Y[3] + 17.0, &format!("Theme: {}", THEME_PACKS[self.theme_pack]), 13.0, INK);
        if !self.toast.is_empty() && self.clock < self.toast_until {
            let w = text_width(&self.toast, 18.0) + 24.0;
            s.fill(g, 400.0 - w / 2.0, 560.0, w, 30.0, rgba(0.1, 0.1, 0.3, 0.9));
            s.text_centered(g, 400.0, 581.0, &self.toast, 18.0, rgb(1.0, 1.0, 0.8));
        }
        g.flush();
    }

    pub fn can_afford(&self, i: usize) -> bool {
        self.sandbox_choice || self.offer_for(i).1 <= START_FUNDS
    }

    pub fn draw_property(&mut self, g: &mut Gfx) {
        let s = Ui::new(self.draw_w, self.draw_h);
        self.view = s.view;
        s.image(g, &self.world_base, 0.0, 0.0);
        s.text(g, 24.0, 36.0, "Where will you build your golf course?", 14.0, rgb(0.1, 0.1, 0.35));
        let d = DIFFICULTY_BUTTON;
        s.fill(g, d.x, d.y, d.w, d.h, if self.hover == 101 { rgba(1.0, 1.0, 0.4, 0.35) } else { rgba(1.0, 1.0, 1.0, 0.25) });
        let label = format!("Difficulty: {} (click to change)", DIFFICULTY_NAMES[self.difficulty.clamp(0, 3) as usize]);
        s.text(g, d.x + 6.0, d.y + 16.0, &label, 13.0, rgb(0.1, 0.1, 0.35));
        let funds = if self.sandbox_choice { "Unlimited \u{a7}".to_string() } else { money(START_FUNDS as i64) };
        s.text_centered(g, 737.0, 32.0, &funds, 17.0, rgb(0.1, 0.1, 0.35));
        for (i, p) in PROPERTIES.iter().enumerate() {
            let r = property_card(i);
            let (acres, price) = self.offer_for(i);
            let ok = self.can_afford(i);
            let a = if ok { 1.0 } else { 0.55 };
            s.image_part(g, &self.theme_icons[p.theme], r.x + 1.0, r.y - 1.0, if ok { 200.0 } else { 0.0 }, 0.0, 52.0, 52.0);
            let cx = r.x + 62.0 + (r.w - 74.0) * 0.5;
            s.text_centered(g, cx, r.y + 15.0, p.name, 16.0, rgba(0.08, 0.08, 0.3, a));
            s.text_centered(g, cx, r.y + 26.0, p.bonus, 11.0, rgba(0.2, 0.2, 0.35, a));
            if !self.sandbox_choice {
                s.text_centered(g, cx, r.y + 43.0, &format!("{} acres: {}", acres, money(price as i64)), 12.0, rgba(0.25, 0.18, 0.1, a));
            }
            if self.hover == i as i32 {
                s.fill(g, r.x, r.y, r.w, r.h, rgba(1.0, 1.0, 0.4, 0.22));
            }
        }
        if (0..16).contains(&self.hover) {
            let p = &PROPERTIES[self.hover as usize];
            let (acres, _) = self.offer_for(self.hover as usize);
            s.text(g, 26.0, 568.0, &format!("{}, {} acres. Bonus: {}.", p.name, acres, p.bonus), 14.0, rgb(0.1, 0.1, 0.3));
            if !self.can_afford(self.hover as usize) {
                s.text(g, 26.0, 586.0, "Not enough funds.", 12.0, rgb(0.55, 0.1, 0.1));
            }
        }
        g.flush();
    }

    /// The exe's TRACTS FOR SALE screen: a minimap of the course with each tract's number, the nine tract descriptions and
    /// prices in a 3 x 3 grid, the cash reserve and a cancel button (positions from the exe; the minimap is drawn as flat
    /// tile diamonds coloured by type, not with the exe's minimap art).
    pub fn draw_land(&mut self, g: &mut Gfx) {
        let s = Ui::new(self.draw_w, self.draw_h);
        self.view = s.view;
        s.fill(g, 0.0, 0.0, 800.0, 600.0, rgb(0.13, 0.16, 0.12));
        let theme = self.exe_theme();
        if let Some(land) = self.land.as_ref() {
            for a in 0..land::N {
                for b in 0..land::N {
                    let t = land.ty[(a * land::N + b) as usize];
                    let c = match t {
                        0 | 1 => rgb(0.45, 0.85, 0.4),
                        2 | 3 => rgb(0.35, 0.72, 0.3),
                        7..=9 => rgb(0.9, 0.85, 0.6),
                        13..=16 => rgb(0.12, 0.38, 0.14),
                        17 => rgb(0.25, 0.45, 0.8),
                        20 => rgb(0.07, 0.08, 0.07),
                        21 | 22 => rgb(0.6, 0.45, 0.35),
                        _ => rgb(0.25, 0.55, 0.22),
                    };
                    let (x, y) = ((a + b) as f32 * 6.0 + 106.0, (b - a) as f32 * 3.0 + 439.0);
                    s.fill(g, x - 6.0, y - 3.0, 12.0, 6.0, c);
                }
            }
            for h in 1..19 {
                let rec = &self.club.holes[h];
                if rec.par == 0 {
                    continue;
                }
                for (a, b) in [rec.back, rec.pin] {
                    let (x, y) = ((a + b) as f32 * 6.0 + 106.0, (b - a) as f32 * 3.0 + 439.0);
                    s.fill(g, x - 3.0, y - 3.0, 6.0, 6.0, rgb(1.0, 1.0, 1.0));
                }
            }
            for i in 0..9 {
                let (a0, b0) = sg_core::tracts::origin(i);
                if land.ty[((a0 + 8) * land::N + b0 + 8) as usize] == land::T_OUT {
                    let (x, y) = ((a0 + b0 + 16) as f32 * 6.0 + 106.0, (b0 - a0) as f32 * 3.0 + 439.0);
                    s.text(g, x - 7.0, y + 4.0, &format!("{}", i + 1), 16.0, rgb(1.0, 0.95, 0.5));
                }
            }
        }
        s.text_centered(g, 400.0, 28.0, "TRACTS FOR SALE", 22.0, rgb(1.0, 0.95, 0.75));
        for i in 0..9usize {
            let (col, row) = ((i / 3) as f32, (i % 3) as f32);
            let x = 88.0 + 256.0 * col - if i <= 5 { 10.0 } else { 0.0 };
            let y = 62.0 + 68.0 * row;
            let tr = self.tracts[i];
            if self.land_hover == i as i32 || tr.oob == 0 {
                let fx = [15.0, 272.0, 538.0][i / 3];
                let fy = [54.0, 122.0, 190.0][i % 3];
                s.fill(g, fx, fy, 236.0, 66.0, rgba(1.0, 1.0, 0.6, if tr.oob == 0 { 0.08 } else { 0.18 }));
            }
            let text = sg_core::tracts::describe(&tr, i, theme);
            let lines = wrap_text(&text, 12.0, 165.0);
            for (k, l) in lines.iter().enumerate() {
                s.text(g, x, y + 12.0 + k as f32 * 13.0, l, 12.0, rgb(0.95, 0.95, 0.9));
            }
            if tr.oob != 0 {
                let p = format!("Price: {}", money(tr.price as i64 * 100));
                let w = text_width(&p, 12.0);
                s.text(g, x + 165.0 - w, y + 12.0 + (lines.len() + 1) as f32 * 13.0, &p, 12.0, rgb(1.0, 0.9, 0.5));
            }
        }
        s.text(g, 548.0, 275.0, "Cash Reserve", 14.0, rgb(0.95, 0.95, 0.9));
        s.text_centered(g, 720.0, 276.0, &money(self.econ.cash as i64), 14.0, rgb(1.0, 0.9, 0.5));
        if self.land_hover == 9 {
            s.fill(g, 662.0, 533.0, 64.0, 64.0, rgba(1.0, 1.0, 0.6, 0.25));
        }
        s.fill(g, 668.0, 539.0, 52.0, 52.0, rgba(0.6, 0.15, 0.1, 0.8));
        s.text_centered(g, 694.0, 570.0, "X", 20.0, rgb(1.0, 1.0, 1.0));
        s.text(g, 71.0, 560.0, "I don't think I'll buy any land.", 14.0, rgb(0.95, 0.95, 0.9));
        g.flush();
    }

    /// The Course Report (the original's Information menu, Course Report), drawn from the disc's coursereport.pcx pieces: a header
    /// band, repeated row strips, a total strip and a legend. Column layout read off that art. Yards use 0.15 yards per world unit:
    /// a placeholder scale.
    pub fn draw_report(&mut self, g: &mut Gfx) {
        if !self.ui_ok || self.report_art.tex.is_none() {
            return;
        }
        let s = Ui::new(self.draw_w, self.draw_h);
        self.view = s.view;
        const CX: [f32; 12] = [112.0, 157.0, 190.0, 233.0, 283.0, 330.0, 375.0, 422.0, 468.0, 571.0, 641.0, 716.0];
        const CW: [f32; 12] = [41.0, 29.0, 39.0, 46.0, 43.0, 41.0, 43.0, 42.0, 99.0, 66.0, 71.0, 74.0];
        const HEAD: [&str; 12] = ["Yds", "Par", "Avg", "Time", "Fun", "+Len", "+Acc", "+Img", "Type", "Avg.Fee", "Revenue", "Profit"];
        let ink = rgb(0.1, 0.1, 0.3);
        let n = self.holes.len();
        let (row_h, top, body_y) = (22.0, 104.0, 108.0);
        let band = rgb(148.0 / 255.0, 150.0 / 255.0, 198.0 / 255.0);
        s.fill(g, 0.0, 100.0, 800.0, body_y + n as f32 * row_h + 6.0 - 100.0, band);
        s.image_part(g, &self.report_art, 0.0, 0.0, 0.0, 0.0, 800.0, top);
        s.text_centered(g, 323.0, 53.0, "COURSE REPORT", 26.0, rgb(0.15, 0.12, 0.3));
        for c in 0..12 {
            s.text_centered(g, CX[c] + CW[c] / 2.0, 89.0, HEAD[c], 12.0, ink);
        }
        // the exe's Course Report (0x44fb30): every number from the hole records (sg_core::ratings::report_row)
        let all = self.club.game & 0x40 != 0;
        let rows: Vec<_> = self
            .hole_numbers
            .iter()
            .map(|&h| sg_core::ratings::report_row(h as usize, &self.club.holes[h as usize], self.difficulty, all))
            .collect();
        let open = rows.len() as i32;
        let (mut t_yds, mut t_par, mut t_avg, mut t_min, mut t_fun, mut t_len, mut t_acc, mut t_img, mut t_fee, mut t_rev, mut t_prof) =
            (0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0);
        let mut m_last = 0;
        let hund = |v: i32| format!("{}{}.{:02}", if v < 0 { "-" } else { "" }, v.abs() / 100, v.abs() % 100);
        let dollars = |v: i32| money(v as i64);
        for (i, r) in rows.iter().enumerate() {
            let y = body_y + i as f32 * row_h;
            s.image_part(g, &self.report_art, 0.0, y, 0.0, if i % 2 == 1 { 168.0 } else { 128.0 }, 800.0, 19.0);
            t_yds += r.yards;
            t_par += r.par;
            t_avg += r.avg;
            t_min += r.minutes;
            m_last = r.minutes;
            t_fun += r.fun;
            t_len += r.len;
            t_acc += r.acc;
            t_img += r.img;
            t_fee += r.avg_fee;
            t_rev += r.revenue;
            t_prof += r.profit;
            // tint: 0 none, 1 good, 2 bad
            let cell = |g: &mut Gfx, c: usize, txt: &str, tint: i32| {
                if tint != 0 {
                    s.image_part(g, &self.report_art, CX[c], y + 2.0, CX[c], if tint == 2 { 205.0 } else { 240.0 }, CW[c], 15.0);
                }
                let col = if tint != 0 { rgb(1.0, 1.0, 1.0) } else { ink };
                s.text_centered(g, CX[c] + CW[c] / 2.0, y + 14.0, txt, 12.0, col);
            };
            let demand = |v: i32| {
                if v < 0 {
                    2
                } else if v >= 50 {
                    1
                } else {
                    0
                }
            };
            let mark = match (r.flags & 3, r.scenic) {
                (0, false) | (2, false) => "",
                (1, false) => " *",
                (_, false) => " **",
                (0, true) | (2, true) => " s",
                _ => " s*",
            };
            s.text(g, 16.0, y + 14.0, &format!("Hole {}{}", r.hole, mark), 12.0, ink);
            cell(g, 0, &r.yards.to_string(), 0);
            cell(g, 1, &r.par.to_string(), 0);
            cell(g, 2, &hund(r.avg), if r.flags & 0xc != 0 { 2 } else { 0 });
            cell(
                g,
                3,
                &format!("{}m", r.minutes),
                if r.minutes >= 5 * r.par {
                    2
                } else if r.minutes <= 3 * r.par {
                    1
                } else {
                    0
                },
            );
            cell(
                g,
                4,
                &format!("{}%", r.fun),
                if r.fun < 10 {
                    2
                } else if r.fun >= 50 {
                    1
                } else {
                    0
                },
            );
            cell(g, 5, &hund(r.len), demand(r.len));
            cell(g, 6, &hund(r.acc), demand(r.acc));
            cell(g, 7, &hund(r.img), demand(r.img));
            cell(
                g,
                8,
                r.type_name,
                if r.variety >= 3 {
                    2
                } else if r.variety < 2 {
                    1
                } else {
                    0
                },
            );
            cell(g, 9, &dollars(r.avg_fee), 0);
            cell(g, 10, &dollars(r.revenue), 0);
            cell(g, 11, &dollars(r.profit), if r.profit < 0 { 2 } else { 0 });
        }
        let total_y = body_y + rows.len() as f32 * row_h + 6.0;
        s.image_part(g, &self.report_art, 0.0, total_y, 0.0, 420.0, 800.0, 92.0);
        s.fill(g, 0.0, total_y + 28.0, 24.0, 26.0, band); // the art carries some layout numbers in its margin
        let y = total_y + 8.0 + 14.0;
        let cell = |g: &mut Gfx, c: usize, txt: &str| s.text_centered(g, CX[c] + CW[c] / 2.0, y, txt, 12.0, ink);
        if open > 0 {
            s.text(g, 16.0, y, "Total", 12.0, ink);
            cell(g, 0, &t_yds.to_string());
            cell(g, 1, &t_par.to_string());
            cell(g, 2, &hund(t_avg));
            // the exe shows the last hole's minutes past the hour, not the total's
            let time = if t_min >= 60 { format!("{}h {}m", t_min / 60, m_last % 60) } else { format!("{}m", m_last % 60) };
            cell(g, 3, &time);
            cell(g, 4, &format!("{}%", t_fun / open));
            cell(g, 5, &hund(t_len / open));
            cell(g, 6, &hund(t_acc / open));
            cell(g, 7, &hund(t_img / open));
            cell(g, 9, &dollars(t_fee / open));
            cell(g, 10, &dollars(t_rev));
            cell(g, 11, &dollars(t_prof));
        }
        let ly = total_y + 55.0 + 6.0;
        s.text(g, 138.0, ly, "Top 100 Hole", 11.0, ink);
        s.text(g, 296.0, ly, "Top 18 Hole", 11.0, ink);
        s.text(g, 448.0, ly, "Scenic Hole", 11.0, ink);
        g.flush();
    }

    /// Heads-up display over the course: club name and date, money, fun and skill, the dock, advisor and story. Layout is our own,
    /// from the screenshots.
    pub fn draw_hud(&mut self, g: &mut Gfx) {
        if !self.ui_ok {
            return;
        }
        let s = Ui::new(self.draw_w, self.draw_h);
        self.view = s.view;
        // The exe's date stamp routine counts months in blocks of 1024 ticks and shows month numbers 3..10, so a year here is eight
        // months, March to October (medium confidence; the start year 2001 is a placeholder). One economy day counts as one month.
        let mi = self.econ.month_index();
        let date = format!("{} {}", MONTHS[((2 + mi % 8) % 12) as usize], 2001 + mi / 8);
        let panel_bg = rgba(0.12, 0.1, 0.3, 0.78);
        s.fill(g, 8.0, 8.0, 230.0, 46.0, panel_bg);
        s.text(g, 18.0, 28.0, &self.course_name, 17.0, rgb(1.0, 1.0, 1.0));
        s.text(g, 18.0, 47.0, &date, 14.0, rgb(0.85, 0.85, 1.0));
        s.fill(g, 560.0, 8.0, 232.0, 70.0, panel_bg);
        let red = self.econ.cash < 0.0 && !self.econ.sandbox;
        let cash = if self.econ.sandbox { "Sandbox".to_string() } else { money(self.econ.cash as i64) };
        s.text(g, 572.0, 30.0, &cash, 19.0, if red { rgb(1.0, 0.5, 0.5) } else { rgb(1.0, 1.0, 0.7) });
        let rt = self.club.ratings;
        s.text(
            g,
            572.0,
            52.0,
            &format!("Fun {}  Skill {}.{:02}", rt.fun, rt.skill / 100, (rt.skill % 100).abs()),
            15.0,
            rgb(0.9, 0.9, 1.0),
        );
        s.text(g, 572.0, 71.0, &format!("Golfers {}, holes {}", self.golfers_on_course(), self.holes.len()), 13.0, rgb(0.75, 0.75, 0.95));
        if self.edit {
            let st = self.edit_status();
            let w = (text_width(&st, 12.0) + 16.0).min(230.0);
            s.fill(g, 8.0, 58.0, w, 20.0, panel_bg);
            s.text(g, 16.0, 72.0, &st, 12.0, rgb(1.0, 0.95, 0.6));
        }
        if self.speed > 1 && !self.paused {
            s.text(g, 170.0, 42.0, &format!("Speed x{}", self.speed), 14.0, rgb(1.0, 1.0, 0.8));
        }
        if self.paused {
            s.text_centered(g, 400.0, 120.0, "PAUSED", 24.0, rgb(1.0, 1.0, 0.8));
        }
        self.draw_thoughts(g, &s);
        self.draw_card(g, &s);
        self.draw_dock(g, &s);
        self.draw_leaderboard(g, &s);
        let lines = self.aim_text();
        if !lines.is_empty() {
            let h = 12.0 + 16.0 * lines.len() as f32;
            s.fill(g, 600.0, 440.0 - h, 192.0, h, rgba(0.12, 0.1, 0.3, 0.85));
            for (i, (l, on)) in lines.iter().enumerate() {
                let c = if *on { rgb(1.0, 1.0, 1.0) } else { rgb(0.55, 0.55, 0.65) };
                s.text(g, 608.0, 440.0 - h + 20.0 + 16.0 * i as f32, l, 13.0, c);
            }
        }
        if self.econ.game_over {
            s.fill(g, 200.0, 250.0, 400.0, 80.0, rgba(0.5, 0.05, 0.05, 0.9));
            s.text_centered(g, 400.0, 300.0, "GAME OVER", 40.0, rgb(1.0, 1.0, 1.0));
        }
        g.flush();
    }

    fn draw_dock(&self, g: &mut Gfx, s: &Ui) {
        if self.dock_art.tex.is_some() {
            s.image_part(g, &self.dock_art, 0.0, 430.0, 0.0, 430.0, 215.0, 170.0);
            if self.dock_hover >= 0 {
                let b = &DOCK[self.dock_hover as usize];
                let (dx, dy) = (b.cx - (b.sx + b.sw * 0.5), b.cy - (b.sy + b.sh * 0.5));
                s.image_part(g, &self.dock_art, b.sx + dx, b.sy + dy, b.sx + b.hover_dx, b.sy, b.sw, b.sh);
                s.text(g, 228.0, 448.0, DOCK_HELP[self.dock_hover as usize], 14.0, rgb(1.0, 1.0, 0.7));
            }
        }
        if self.panel == 4 {
            s.fill(g, 226.0, 452.0, 570.0, 144.0, rgba(0.16, 0.14, 0.34, 0.88));
            self.draw_golfers_panel(g, s);
        } else if self.panel != 0 {
            s.fill(g, 226.0, 452.0, 570.0, 144.0, rgba(0.16, 0.14, 0.34, 0.88));
            let cols = self.panel_cols();
            for (i, it) in self.panel_items().iter().enumerate() {
                let r = panel_item_rect(i, cols);
                let sel = match it.kind {
                    0 => self.tool == 0 && self.paint_idx == it.arg,
                    1 => self.tool == 2 && self.path_kind as usize == it.arg,
                    2 => self.tool == 1 && self.raise_sign > 0,
                    3 => self.tool == 1 && self.raise_sign < 0,
                    4 => self.tool == 4 && self.build_idx == it.arg,
                    7 => self.club.planner.option == SHOT_OPTS[it.arg],
                    _ => false,
                };
                if sel {
                    s.fill(g, r.x, r.y, r.w, r.h, rgba(0.9, 0.75, 0.2, 0.55));
                } else if i as i32 == self.panel_hover {
                    s.fill(g, r.x, r.y, r.w, r.h, rgba(0.5, 0.5, 0.9, 0.45));
                }
                s.text(g, r.x + 4.0, r.y + 13.0, &it.label, 13.0, rgb(1.0, 1.0, 1.0));
            }
        }
        // Advisor and story, top centre.
        if self.show_advisor {
            let lines = wrap_text(self.advisor_text(), 14.0, 290.0);
            let h = 10.0 + 17.0 * lines.len() as f32;
            s.fill(g, 244.0, 8.0, 306.0, h, rgba(0.12, 0.1, 0.3, 0.82));
            for (i, l) in lines.iter().enumerate() {
                s.text(g, 252.0, 25.0 + 17.0 * i as f32, l, 14.0, rgb(1.0, 0.95, 0.7));
            }
            let y = 8.0 + h + 6.0;
            if !self.story_lines.is_empty() && self.game_tick < self.story_until {
                // the latest story lines (the exe shows them as thought bubbles over the two golfers)
                let mut rows: Vec<String> = Vec::new();
                for l in self.story_lines.iter() {
                    let (who, text) = l.split_once('|').unwrap_or(("", l));
                    rows.extend(wrap_text(&format!("{who}: {text}"), 14.0, 290.0));
                }
                let sh = 26.0 + 17.0 * rows.len() as f32;
                s.fill(g, 244.0, y, 306.0, sh, rgba(0.12, 0.2, 0.25, 0.85));
                s.text(g, 252.0, y + 16.0, &self.story_title, 12.0, rgb(0.8, 0.9, 1.0));
                for (i, l) in rows.iter().enumerate() {
                    s.text(g, 252.0, y + 33.0 + 17.0 * i as f32, l, 14.0, rgb(1.0, 1.0, 1.0));
                }
            }
        }
        if !self.toast.is_empty() && self.clock < self.toast_until {
            let w = text_width(&self.toast, 16.0) + 24.0;
            s.fill(g, 400.0 - w / 2.0, 410.0, w, 28.0, rgba(0.5, 0.1, 0.1, 0.88));
            s.text_centered(g, 400.0, 430.0, &self.toast, 16.0, rgb(1.0, 1.0, 1.0));
        }
    }
}
