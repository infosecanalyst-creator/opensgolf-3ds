//! Drawing: the course (terrain, paths, walls), sprites in painter's order, the ball, the edit cursor, and the 2D screens (title
//! menu, property chooser, heads-up display with the dock, course report).
use crate::app::*;
use crate::gfx::{Gfx, Mat4, Mode, Uniforms, Vert};
use crate::ui::{money, num, rgb, rgba, text_width, wrap_text, Screen as Ui};
use sg_core::economy::{Economy, STAFF_KINDS};
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
        g.flush();
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
            if let Some(gl) = self.golfers.iter().find(|g| g.active) {
                self.cam_x = gl.sim.golfer_x;
                self.cam_z = gl.sim.golfer_z;
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
        let quad = |app: &mut App, g: &mut Gfx, si: Option<usize>, pi: usize| {
            let Some(si) = si else { return };
            let p = app.props[pi].clone();
            let sp = &app.sprites[si].s;
            let views = sp.views;
            let mut view = if views >= 8 {
                (quarter * 2) % 8
            } else if views >= 4 {
                quarter % views
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
            let y = app.terrain.height_at(p.x, p.z);
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
                quad(self, g, b, i);
            }
        }
        for &(_, i) in &items {
            let sh = self.props[i].shadow;
            quad(self, g, sh, i);
        }
        for &(_, i) in &items {
            if !self.props[i].flat {
                let b = self.props[i].body;
                quad(self, g, b, i);
            }
        }
        // The ball: a small white disc with a dark disc on the ground below it.
        for gl in &self.golfers {
            if !gl.active || gl.sim.ball_h < 0.0 {
                continue;
            }
            let sm = &gl.sim;
            let gy = self.terrain.height_at(sm.ball_x, sm.ball_z);
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
            disc(g, sm.ball_x, gy + 1.0, sm.ball_z, 4.5, [0.0, 0.0, 0.0, 0.45], false);
            disc(g, sm.ball_x, gy + sm.ball_h + 4.0, sm.ball_z, 5.0, [1.0; 4], true);
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
        let mv = self.mv;
        let (rx, ry, rz, ux, uy, uz) = (mv[0], mv[4], mv[8], mv[1], mv[5], mv[9]);
        let half = self.upp; // one pixel either side
        let col = [1.0, 0.95, 0.3, 1.0];
        let mut v = Vec::new();
        for i in 0..pts.len() {
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
                for (i, b) in BUILD.iter().enumerate() {
                    if self.build_available(i) {
                        v.push(PanelItem { label: format!("{}  {}", b.name, money(b.cost as i64 * 100)), kind: 4, arg: i });
                    }
                }
            }
            3 => {
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
        self.sandbox_choice || PROPERTIES[i].price <= START_FUNDS
    }

    pub fn draw_property(&mut self, g: &mut Gfx) {
        let s = Ui::new(self.draw_w, self.draw_h);
        self.view = s.view;
        s.image(g, &self.world_base, 0.0, 0.0);
        s.text(g, 24.0, 36.0, "Where will you build your golf course?", 14.0, rgb(0.1, 0.1, 0.35));
        let funds = if self.sandbox_choice { "Unlimited \u{a7}".to_string() } else { money(START_FUNDS as i64) };
        s.text_centered(g, 737.0, 32.0, &funds, 17.0, rgb(0.1, 0.1, 0.35));
        for (i, p) in PROPERTIES.iter().enumerate() {
            let r = property_card(i);
            let ok = self.can_afford(i);
            let a = if ok { 1.0 } else { 0.55 };
            s.image_part(g, &self.theme_icons[p.theme], r.x + 1.0, r.y - 1.0, if ok { 200.0 } else { 0.0 }, 0.0, 52.0, 52.0);
            let cx = r.x + 62.0 + (r.w - 74.0) * 0.5;
            s.text_centered(g, cx, r.y + 15.0, p.name, 16.0, rgba(0.08, 0.08, 0.3, a));
            s.text_centered(g, cx, r.y + 26.0, p.bonus, 11.0, rgba(0.2, 0.2, 0.35, a));
            if !self.sandbox_choice {
                s.text_centered(
                    g,
                    cx,
                    r.y + 43.0,
                    &format!("{} acres: {}", p.acres, money(p.price as i64)),
                    12.0,
                    rgba(0.25, 0.18, 0.1, a),
                );
            }
            if self.hover == i as i32 {
                s.fill(g, r.x, r.y, r.w, r.h, rgba(1.0, 1.0, 0.4, 0.22));
            }
        }
        if (0..16).contains(&self.hover) {
            let p = &PROPERTIES[self.hover as usize];
            s.text(g, 26.0, 568.0, &format!("{}, {} acres. Bonus: {}.", p.name, p.acres, p.bonus), 14.0, rgb(0.1, 0.1, 0.3));
            if !self.can_afford(self.hover as usize) {
                s.text(g, 26.0, 586.0, "Not enough funds.", 12.0, rgb(0.55, 0.1, 0.1));
            }
        }
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
        let total_y = body_y + n as f32 * row_h + 6.0;
        let band = rgb(148.0 / 255.0, 150.0 / 255.0, 198.0 / 255.0);
        s.fill(g, 0.0, 100.0, 800.0, total_y - 100.0, band);
        s.image_part(g, &self.report_art, 0.0, 0.0, 0.0, 0.0, 800.0, top);
        s.text_centered(g, 323.0, 53.0, "COURSE REPORT", 26.0, rgb(0.15, 0.12, 0.3));
        for c in 0..12 {
            s.text_centered(g, CX[c] + CW[c] / 2.0, 89.0, HEAD[c], 12.0, ink);
        }
        self.ensure_ratings(40);
        let (mut t_y, mut t_par, mut t_str, mut t_sec, mut t_mood, mut t_rev, mut t_plays, mut t_prof, mut t_len, mut t_acc, mut t_img) =
            (0.0f64, 0.0f64, 0.0f64, 0.0f64, 0.0f64, 0.0f64, 0.0f64, 0.0f64, 0.0f64, 0.0f64, 0.0f64);
        let upkeep_share = if n > 0 { (self.econ.upkeep_paid + self.econ.wages_paid) / n as f64 } else { 0.0 };
        for i in 0..n {
            let y = body_y + i as f32 * row_h;
            s.image_part(g, &self.report_art, 0.0, y, 0.0, if i % 2 == 1 { 168.0 } else { 128.0 }, 800.0, 19.0);
            let r = &self.holes[i];
            let rt = &self.ratings[i];
            let hs = self.hole_stats.get(i).copied().unwrap_or_default();
            let plays = hs.plays as f64;
            let yds = r.length as f64 * 0.15;
            let avg = if hs.plays > 0 { hs.strokes / plays } else { 0.0 };
            let mins = if hs.plays > 0 { hs.seconds / plays * 0.35 } else { 0.0 }; // 0.35 game minutes per sim second: placeholder
            let fun = if hs.plays > 0 { hs.mood / plays } else { 0.0 };
            let fee = if hs.plays > 0 { hs.revenue / plays } else { 0.0 };
            let profit = hs.revenue - upkeep_share;
            t_y += yds;
            t_par += r.par as f64;
            t_str += avg;
            t_sec += mins;
            t_mood += fun;
            t_rev += hs.revenue;
            t_plays += plays;
            t_prof += profit;
            t_len += rt.len as f64;
            t_acc += rt.acc as f64;
            t_img += rt.img as f64;
            // tint: 0 none, 1 green, 2 red
            let cell = |g: &mut Gfx, c: usize, txt: &str, tint: i32| {
                if tint != 0 {
                    s.image_part(g, &self.report_art, CX[c], y + 2.0, CX[c], if tint == 2 { 205.0 } else { 240.0 }, CW[c], 15.0);
                }
                let col = if tint != 0 { rgb(1.0, 1.0, 1.0) } else { ink };
                s.text_centered(g, CX[c] + CW[c] / 2.0, y + 14.0, txt, 12.0, col);
            };
            let played = hs.plays > 0;
            s.text(g, 16.0, y + 14.0, &format!("Hole {}", i + 1), 12.0, ink);
            cell(g, 0, &num(yds, 0), 0);
            cell(g, 1, &r.par.to_string(), 0);
            cell(g, 2, &if played { num(avg, 2) } else { "-".into() }, 0);
            cell(g, 3, &if played { format!("{}m", num(mins, 0)) } else { "-".into() }, 0);
            let fun_tint = if !played {
                0
            } else if fun >= 70.0 {
                1
            } else if fun < 45.0 {
                2
            } else {
                0
            };
            cell(g, 4, &if played { format!("{}%", num(fun, 0)) } else { "-".into() }, fun_tint);
            cell(g, 5, &num(rt.len as f64, 2), 0);
            cell(g, 6, &num(rt.acc as f64, 2), 0);
            cell(g, 7, &num(rt.img as f64, 2), 0);
            cell(g, 8, rt.ty, 0);
            cell(g, 9, &if played { num(fee, 0) } else { "-".into() }, 0);
            cell(g, 10, &num(hs.revenue, 0), 0);
            cell(
                g,
                11,
                &num(profit, 0),
                if profit < 0.0 {
                    2
                } else if profit > 0.0 {
                    1
                } else {
                    0
                },
            );
        }
        s.image_part(g, &self.report_art, 0.0, total_y, 0.0, 420.0, 800.0, 92.0);
        s.fill(g, 0.0, total_y + 28.0, 24.0, 26.0, band); // the art carries some layout numbers in its margin
        let y = total_y + 8.0 + 14.0;
        let cell = |g: &mut Gfx, c: usize, txt: &str| s.text_centered(g, CX[c] + CW[c] / 2.0, y, txt, 12.0, ink);
        s.text(g, 16.0, y, "Total", 12.0, ink);
        let k = if n > 0 { 1.0 / n as f64 } else { 0.0 };
        let any = t_plays > 0.0;
        cell(g, 0, &num(t_y, 0));
        cell(g, 1, &num(t_par, 0));
        cell(g, 2, &if any { num(t_str, 2) } else { "-".into() });
        cell(g, 3, &if any { format!("{}m", num(t_sec, 0)) } else { "-".into() });
        cell(g, 4, &if any { format!("{}%", num(t_mood * k, 0)) } else { "-".into() });
        cell(g, 5, &num(t_len * k, 2));
        cell(g, 6, &num(t_acc * k, 2));
        cell(g, 7, &num(t_img * k, 2));
        cell(g, 9, &if any { num(t_rev / t_plays, 0) } else { "-".into() });
        cell(g, 10, &num(t_rev, 0));
        cell(g, 11, &num(t_prof, 0));
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
        let mi = self.econ.day - 1;
        let date = format!("{} {}", MONTHS[((2 + mi % 8) % 12) as usize], 2001 + mi / 8);
        let panel_bg = rgba(0.12, 0.1, 0.3, 0.78);
        s.fill(g, 8.0, 8.0, 230.0, 46.0, panel_bg);
        s.text(g, 18.0, 28.0, &self.course_name, 17.0, rgb(1.0, 1.0, 1.0));
        s.text(g, 18.0, 47.0, &date, 14.0, rgb(0.85, 0.85, 1.0));
        s.fill(g, 560.0, 8.0, 232.0, 70.0, panel_bg);
        let red = self.econ.cash < 0.0 && !self.econ.sandbox;
        let cash = if self.econ.sandbox { "Sandbox".to_string() } else { money(self.econ.cash as i64) };
        s.text(g, 572.0, 30.0, &cash, 19.0, if red { rgb(1.0, 0.5, 0.5) } else { rgb(1.0, 1.0, 0.7) });
        self.ensure_ratings(20);
        s.text(g, 572.0, 52.0, &format!("Fun {}  Skill {:.2}", self.club_fun(), self.club_skill()), 15.0, rgb(0.9, 0.9, 1.0));
        s.text(g, 572.0, 71.0, &format!("Golfers {}, holes {}", self.golfers_on_course(), self.holes.len()), 13.0, rgb(0.75, 0.75, 0.95));
        if self.edit {
            let st = self.edit_status();
            let w = (text_width(&st, 12.0) + 16.0).min(230.0);
            s.fill(g, 8.0, 58.0, w, 20.0, panel_bg);
            s.text(g, 16.0, 72.0, &st, 12.0, rgb(1.0, 0.95, 0.6));
        }
        if self.paused {
            s.text_centered(g, 400.0, 120.0, "PAUSED", 24.0, rgb(1.0, 1.0, 0.8));
        }
        self.draw_dock(g, &s);
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
        if self.panel != 0 {
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
            if !self.story_lines.is_empty() && self.golfers_on_course() >= 2 {
                let line = &self.story_lines[self.story_pos % self.story_lines.len()];
                let a = line.starts_with('A');
                let sl = wrap_text(line.get(2..).unwrap_or(""), 14.0, 290.0);
                let sh = 12.0 + 17.0 * (sl.len() + 1) as f32;
                let bg = if a { rgba(0.25, 0.12, 0.12, 0.82) } else { rgba(0.12, 0.22, 0.12, 0.82) };
                s.fill(g, 244.0, y, 306.0, sh, bg);
                s.text(
                    g,
                    252.0,
                    y + 16.0,
                    &format!("{}{}", self.story_title, if a { " (golfer one)" } else { " (golfer two)" }),
                    12.0,
                    rgb(0.8, 0.9, 1.0),
                );
                for (i, l) in sl.iter().enumerate() {
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
