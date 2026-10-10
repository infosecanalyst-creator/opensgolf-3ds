//! Drawing: the course (terrain, paths, walls), sprites in painter's order, the ball, the edit cursor, and the 2D screens (title
//! menu, select difficulty, heads-up display with the dock). The property chooser and land screen are in world_ui, the
//! course report in info_ui.
use crate::app::*;
use crate::gfx::{Gfx, Mat4, Mode, Uniforms, Vert};
use crate::ui::{money, rgb, rgba, text_width, wrap_text, Screen as Ui};
use sg_core::economy::{Economy, STAFF_KINDS};
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
        // world units per drawable pixel: the zoom is in the 800 x 600 screen's pixels, so a bigger window shows the same view
        let vscale = (self.draw_w / 800.0).min(self.draw_h / 600.0);
        let upp = 2.0 / (self.zoom * vscale);
        let (hw, hh) = (self.draw_w * upp * 0.5, self.draw_h * upp * 0.5);
        let proj = Mat4::ortho(-hw, hw, -hh, hh, -DEPTH_RANGE, DEPTH_RANGE);
        let mv = Mat4::rotate(pitch_for(self.draw_w, self.draw_h), 1.0, 0.0, 0.0)
            .mul(&Mat4::rotate((45.0 + self.rot) as f64, 0.0, 1.0, 0.0))
            .mul(&Mat4::translate(-self.cam_x, 0.0, -self.cam_z));
        // Terrain.dll's light 0: the theme's Lighting.txt colours, a direction fixed in eye space, OpenGL's default material
        // with a white specular of shininess 13 (sg_core::terrain::Lighting). The water's texture is not animated (the
        // renderer has no clock); the exe animates water with its ripple, rock and waterfall sprites (wild_ui).
        let mut lit = Uniforms::flat(&proj, &mv);
        lit.lit = 1.0;
        // CALIBRATED: the decoded light leaves flat ground at about 58% of its texture, but the game's own rendered terrain
        // icons (Data/<theme>.pcx, e.g. desert sand 223,161,96 against its texture's 234,171,110) show about 95%, so
        // something in the original's setup brightens it further (not yet found). Ambient and diffuse are scaled to match.
        const BRIGHTNESS: f32 = 1.74;
        lit.light_amb = self.light.ambient.map(|v| v * BRIGHTNESS);
        lit.light_dif = self.light.diffuse.map(|v| v * BRIGHTNESS);
        lit.light_spec = self.light.specular;
        lit.light_dir = sg_core::terrain::light_direction();
        for b in &self.batches {
            g.draw_mesh(Mode::Solid, &b.mesh, b.tex, &lit);
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
        self.draw_bank_rocks(g, &Uniforms::flat(&proj, &mv));
        self.draw_props(g, &Uniforms::flat(&proj, &mv));
        if self.edit && self.has_hit {
            self.draw_cursor_preview(g, &Uniforms::flat(&proj, &mv));
        }
        self.draw_aim(g, &Uniforms::flat(&proj, &mv));
        self.draw_layout_path(g, &Uniforms::flat(&proj, &mv));
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
        let zoom = self.exe_zoom();
        let lifts = sg_core::pro::aim_lifts(p.distance, opt, zoom);
        let mv = self.mv;
        let (ux, uy, uz) = (mv[1], mv[5], mv[9]);
        let vscale = (self.draw_w / 800.0).min(self.draw_h / 600.0);
        let px = self.upp * vscale; // world units per 800 x 600 pixel
                                    // a ground point moved `up` pixels up the screen
        let lifted = |p: [f32; 3], up: f32| [p[0] + ux * up * px, p[1] + uy * up * px, p[2] + uz * up * px];
        let ground: Vec<[f32; 3]> = line
            .iter()
            .map(|&(x, y)| {
                let (wx, wz) = self.units_to_world(x, y);
                [wx, self.terrain.height_at(wx, wz), wz]
            })
            .collect();
        let shadow: Vec<[f32; 3]> = ground.iter().map(|&q| lifted(q, -1.0)).collect();
        let arc: Vec<[f32; 3]> = ground.iter().zip(&lifts).map(|(&q, &l)| lifted(q, l)).collect();
        // line width 2 pixels at the nearest zoom, 1 below it
        let half = if zoom >= 4.0 { 1.0 } else { 0.5 } * vscale;
        self.draw_lines(g, u, &shadow, false, half, [0.0, 0.0, 0.0, 1.0]);
        self.draw_lines(g, u, &arc, false, half, [1.0, 1.0, 1.0, 1.0]);
        let (wx, wz) = self.units_to_world(p.x, p.y);
        let r = TILE_SIZE * 0.5;
        let corners = [(wx - r, wz - r), (wx + r, wz - r), (wx + r, wz + r), (wx - r, wz + r)];
        let bx: Vec<[f32; 3]> = corners.iter().map(|&(x, z)| [x, self.terrain.height_at(x, z) + 3.0, z]).collect();
        self.draw_lines(g, u, &bx, true, 1.0, [1.0, 0.95, 0.3, 1.0]);
    }

    /// A person's frame in the palette of an outfit, recoloured on first use and cached.
    pub(crate) fn outfit_texture(
        &mut self,
        g: &mut Gfx,
        si: usize,
        view: i32,
        frame: i32,
        o: sg_core::bodies::Outfit,
    ) -> Option<miniquad::TextureId> {
        let i = self.sprites[si].s.frame_index(view, frame);
        if self.sprites[si].s.indexed.is_empty() {
            return None;
        }
        if let Some(&t) = self.outfit_tex.get(&(si, i, o)) {
            return Some(t);
        }
        if !self.outfit_pals.contains_key(&o) {
            let pal = self.swaps.compose(&o)?;
            self.outfit_pals.insert(o, pal);
        }
        let pal = self.outfit_pals[&o];
        let img = sg_core::sprites::recolour(&self.sprites[si].s, i, &pal)?;
        if self.outfit_tex.len() > 6000 {
            for (_, t) in self.outfit_tex.drain() {
                g.ctx.delete_texture(t);
            }
        }
        let t = g.texture(&img, false);
        self.outfit_tex.insert((si, i, o), t);
        Some(t)
    }

    pub(crate) fn sprite_texture(&mut self, g: &mut Gfx, si: usize, view: i32, frame: i32) -> miniquad::TextureId {
        let i = self.sprites[si].s.frame_index(view, frame);
        if let Some(t) = self.sprites[si].tex[i] {
            return t;
        }
        if crate::hd::active() {
            if let Some(img) = self.hd.sprites.get(&si).and_then(|h| h.frame(i)) {
                let t = g.texture_hd(&img);
                self.sprites[si].tex[i] = Some(t);
                return t;
            }
        }
        let t = g.texture(&self.sprites[si].s.frames[i], false);
        self.sprites[si].tex[i] = Some(t);
        t
    }

    /// Props as camera-facing quads. Sprites are pre-rendered, so they ignore lighting and depth: ground overlays first, then
    /// each sprite's shadow and body together, far to near (painter's order).
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
        let px = self.upp * (self.draw_w / 800.0).min(self.draw_h / 600.0); // world units per 800 x 600 pixel
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
            if let Some(v) = p.view {
                view = v.rem_euclid(views.max(1));
            }
            let (w, h, ax, ay) = (sp.w as f32, sp.h as f32, sp.anchor_x as f32, sp.anchor_y as f32);
            let tex = match p.outfit.filter(|_| !shadow) {
                Some(o) => app.outfit_texture(g, si, view, p.frame, o).unwrap_or_else(|| app.sprite_texture(g, si, view, p.frame)),
                None => app.sprite_texture(g, si, view, p.frame),
            };
            let y = app.terrain.height_at(p.x, p.z) + if shadow { 0.0 } else { p.lift };
            // the exe's screen offset, in world units along the eye's axes
            let (ox, oy) = (p.shift.0 * px, -p.shift.1 * px);
            let p = Prop { x: p.x + rx * ox + ux * oy, z: p.z + rz * ox + uz * oy, ..p };
            let y = y + ry * ox + uy * oy;
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
        // the exe composites each sprite's shadow under its body when it loads them (0x43d740 draws NameShadow.flc, then
        // Name.flc over it, into the same frames), so a sprite's shadow is drawn with it in the depth order: a nearer
        // sprite's shadow falls over a farther sprite
        for &(_, i) in &items {
            let sh = self.props[i].shadow;
            quad(self, g, sh, i, true);
            if !self.props[i].flat {
                let b = self.props[i].body;
                quad(self, g, b, i, false);
            }
        }
        self.draw_balls(g, u);
    }

    /// The balls (0x41557f, for every golfer with a ball): the 2 x 2 white cut at (192, 0) of BLDG.PCX with the 2 x 2 black
    /// cut under it (192, 2) as its shadow, a pixel below the ground point; a ball resting on ground whose hazard byte is
    /// above 0 (rough, sand and the like) is a white dash two pixels long instead, without a shadow. Both cuts are queued at
    /// scale 4, so they are 2 pixels at the closest zoom and shrink with it. A sweet shot (golfer flag 0x400000) leaves a
    /// yellow streak 2 pixels wide from where the ball was drawn last frame. A ball in the air is drawn `ball_lift` pixels
    /// straight up the screen from its ground point.
    fn draw_balls(&mut self, g: &mut Gfx, u: &Uniforms) {
        use sg_core::golfer::flag;
        let mv = self.mv;
        let (rx, ry, rz) = (mv[0], mv[4], mv[8]);
        let (ux, uy, uz) = (mv[1], mv[5], mv[9]);
        let vscale = (self.draw_w / 800.0).min(self.draw_h / 600.0);
        let px = self.upp * vscale; // world units per 800 x 600 pixel
        let side = (2.0 * self.exe_zoom() / 4.0).max(1.0f32);
        if self.ball_trail.len() != self.club.g.len() {
            self.ball_trail = vec![None; self.club.g.len()];
        }
        let white = [1.0; 4];
        let black = [0.0, 0.0, 0.0, 1.0];
        let yellow = crate::info_ui::c15(0x7f9c);
        // a w x h pixel box, its top left (dx, dy) pixels from a world point (screen y down)
        let quad = |g: &mut Gfx, p: [f32; 3], dx: f32, dy: f32, w: f32, h: f32, col: [f32; 4]| {
            let c = |cx: f32, cy: f32| {
                let (cx, cy) = (cx * px, -cy * px);
                Vert::new(p[0] + rx * cx + ux * cy, p[1] + ry * cx + uy * cy, p[2] + rz * cx + uz * cy, 0.0, 0.0).col(col)
            };
            let (a, b, cc, d) = (c(dx, dy), c(dx + w, dy), c(dx + w, dy + h), c(dx, dy + h));
            g.tris(Mode::Flat, None, u, &[a, b, cc, a, cc, d]);
        };
        for i in 0..self.club.g.len() {
            let gl = &self.club.g[i];
            if gl.bx == 0 || gl.hole <= 0 || gl.hole >= 19 || gl.flags & flag::PENALTY != 0 {
                self.ball_trail[i] = None;
                continue;
            }
            let (bx, bz) = self.units_to_world(gl.bx, gl.by);
            let gy = self.terrain.height_at(bx, bz);
            let ground = [bx, gy, bz];
            let resting = (gl.flags & flag::BALL_MOVING == 0 || gl.speed <= 0x400) && gl.bz == 0;
            let hazard = self.course.hazard[self.course.tile_type(gl.bx, gl.by).min(22) as usize];
            if resting && hazard > 0 {
                quad(g, ground, 0.0, 0.0, 2.0, 1.0, white);
                self.ball_trail[i] = Some(ground);
                continue;
            }
            let half = side / 2.0;
            quad(g, ground, -half, 1.0 - half, side, side, black);
            let lift = sg_core::flight::ball_lift(gl.bz, self.exe_zoom()) * px;
            let ball = [bx + ux * lift, gy + uy * lift, bz + uz * lift];
            quad(g, ball, -half, -half, side, side, white);
            if gl.flags & flag::SWEET != 0 {
                if let Some(prev) = self.ball_trail[i] {
                    self.draw_lines(g, u, &[prev, ball], false, vscale, yellow);
                }
            }
            self.ball_trail[i] = Some(ball);
        }
    }

    /// A line through world points, `px` pixels either side, closed into a loop or not.
    pub(crate) fn draw_lines(&self, g: &mut Gfx, u: &Uniforms, pts: &[[f32; 3]], closed: bool, px: f32, col: [f32; 4]) {
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
/// The folders under Themes/ offered as theme packs (the exe leaves out Championship, which holds the championship files).
pub const THEME_PACKS: [&str; 4] = ["Standard", "Firaxis", "More Stories", "The Sims"];
/// The title menu's labels (0x420e.. and the menu's opening 0x40f5c0): Klepto ITC 18 (0x51a028) in 0x6318, centred on the
/// label's point with the text's top 8 below it; "Start New Game" has the theme pack's name in brackets 24 further down.
const MENU_LABEL: [&str; 5] = ["Continue Saved Game", "Start New Game", "Sandbox Mode", "Select A Theme", "Play a Championship"];
const MENU_LABEL_X: [f32; 5] = [182.0, 573.0, 141.0, 631.0, 415.0];
const MENU_LABEL_Y: [f32; 5] = [66.0, 88.0, 431.0, 409.0, 525.0];

/// Select Difficulty (0x43a400, docs/DECODE_TITLE2.md 5): each item's lit cut in TitleSelDiffMO.pcx (x, y, w, h), where it
/// goes, its hit centre (an ellipse twice as wide as tall, octagonal distance under 80) and its label position.
pub const DIFF_CUTS: [(f32, f32, f32, f32, f32, f32); 4] = [
    (0.0, 0.0, 338.0, 146.0, 193.0, 32.0),
    (400.0, 0.0, 338.0, 146.0, 150.0, 161.0),
    (0.0, 300.0, 332.0, 114.0, 161.0, 320.0),
    (400.0, 300.0, 332.0, 130.0, 200.0, 436.0),
];
pub const DIFF_CENTRES: [(f32, f32); 4] = [(348.0, 107.0), (319.0, 218.0), (320.0, 367.0), (363.0, 495.0)];
pub const DIFF_LABELS: [(f32, f32); 4] = [(388.0, 113.0), (360.0, 227.0), (366.0, 346.0), (402.0, 464.0)];

/// The octagonal distance the exe's hit tests use: max + min / 2.
pub fn oct_dist(dx: f32, dy: f32) -> f32 {
    let (a, b) = (dx.abs(), dy.abs());
    a.max(b) + a.min(b) / 2.0
}

/// Which Select Difficulty item a point is on: 0..3, 100 the back button, -1 nothing.
pub fn difficulty_hit(vx: f32, vy: f32) -> i32 {
    if oct_dist(vx - 767.0, vy - 557.0) < 25.0 {
        return 100;
    }
    DIFF_CENTRES.iter().position(|&(cx, cy)| oct_dist((vx - cx) / 2.0, vy - cy) < 80.0).map(|i| i as i32).unwrap_or(-1)
}
/// The four difficulties the manual names, in the exe's order (0 easiest).
pub const DIFFICULTY_NAMES: [&str; 4] = ["Easy", "Moderate", "Difficult", "Impossible"];

/// The lower left dock: centre and radius of each button's hit disc.
pub struct DockBtn {
    pub cx: f32,
    pub cy: f32,
    pub r: f32,
}
const fn db(cx: f32, cy: f32, r: f32) -> DockBtn {
    DockBtn { cx, cy, r }
}
/// Measured from Interface/3mainLowerLeft.pcx (the assembled dock sits in the bottom left of the sheet).
pub const DOCK: [DockBtn; 10] = [
    db(43.0, 473.0, 33.0),  // Build Course
    db(117.0, 497.0, 30.0), // Add Buildings
    db(177.0, 536.0, 26.0), // People
    db(32.0, 543.0, 11.0),  // zoom in
    db(31.0, 585.0, 11.0),  // zoom out
    db(17.0, 565.0, 11.0),  // rotate right
    db(47.0, 565.0, 11.0),  // rotate left
    db(75.0, 552.0, 14.0),  // information (course report)
    db(107.0, 568.0, 14.0), // pause
    db(133.0, 583.0, 13.0), // tools (save the course)
];
/// The dock's lit looks (0x432ba0, cuts at 0x5873a8 from the loader 0x447000): sheet cut (x, y, w, h) of the bright blue
/// hover look and where it is drawn (the exe's table at 0x4c7960, picked through 0x4c7994), in our button order. The big
/// buttons' gold look is the same cut 100 to the right, the small ones' 50 to the right.
pub const DOCK_LIT: [(f32, f32, f32, f32, f32, f32); 10] = [
    (0.0, 0.0, 78.0, 81.0, 7.0, 436.0),
    (0.0, 100.0, 78.0, 81.0, 84.0, 463.0),
    (0.0, 200.0, 78.0, 81.0, 146.0, 503.0),
    (600.0, 50.0, 36.0, 38.0, 19.0, 530.0),
    (600.0, 100.0, 36.0, 38.0, 19.0, 573.0),
    (600.0, 150.0, 36.0, 38.0, 4.0, 551.0),
    (600.0, 200.0, 36.0, 38.0, 33.0, 552.0),
    (600.0, 250.0, 36.0, 38.0, 59.0, 535.0),
    (600.0, 300.0, 36.0, 38.0, 94.0, 553.0),
    (600.0, 350.0, 36.0, 38.0, 121.0, 570.0),
];
/// Where the gold trail of dock mode 0, 1 and 2 (cut (300, 100 * mode, 214, 91)) is drawn (0x432d01..0x432db7).
const DOCK_TRAIL: [(f32, f32); 3] = [(34.0, 509.0), (95.0, 525.0), (142.0, 555.0)];

/// The dock's tooltip captions (0x432ba0), in our button order.
pub const DOCK_HELP: [&str; 10] = [
    "Build Course",
    "Add Buildings",
    "People",
    "Zoom Map",
    "Unzoom Map",
    "Rotate Map",
    "Rotate Map",
    "Information",
    "Pause or Unpause",
    "System Functions",
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
        if self.panel == 3 || self.panel == 5 {
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
            // 3 People (this list only when the panel art is missing), 5 the player's pro
            3 | 5 if self.club.pro_aiming().is_some() => {
                // the pro waits for the aim: the five shot buttons (panel buttons 4..8)
                for (i, name) in SHOT_NAMES.iter().enumerate() {
                    v.push(PanelItem { label: format!("{name}  ({})", SHOT_KEYS[i]), kind: 7, arg: i });
                }
                v.push(PanelItem { label: "Click the course to aim and swing.  N twice cancels the round.".into(), kind: 8, arg: 0 });
            }
            3 | 5 => {
                let pro = self.pro_name();
                if self.club.gary == -1 {
                    v.push(PanelItem { label: format!("{pro}: Practice Round"), kind: 6, arg: 0 });
                    let m = if self.club.game & sg_core::pro::CHALLENGE != 0 && self.club.challenge_pro >= 0 {
                        let who = self.club.pros.get(self.club.challenge_pro as usize).map(|p| p.name.clone()).unwrap_or_default();
                        format!("{pro}: Match vs. {who}  (\u{a7}{}/hole)", self.club.wager_level * 2000)
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
                if self.panel == 5 {
                    // hiring and firing are on the Employee panel
                    v.push(PanelItem { label: "Employees".into(), kind: 10, arg: 0 });
                    return v;
                }
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
        // the logo and the lit buttons live in the highlight picture, split into their own shapes at load time
        if let Some(logo) = self.title_mo_parts.get(6) {
            s.image(g, logo, 0.0, 0.0);
        }
        s.image(g, &self.title_un, 0.0, 0.0);
        if let Some(lit) = self.title_mo_parts.get(self.hover.max(0) as usize).filter(|_| (0..6).contains(&self.hover)) {
            s.image(g, lit, 0.0, 0.0);
        }
        let grey = crate::info_ui::c15(0x6318);
        for b in 0..5 {
            s.put_centered(g, crate::ui::F_KLEPTO18, MENU_LABEL_X[b], MENU_LABEL_Y[b] + 8.0, MENU_LABEL[b], grey);
        }
        let theme = format!("({})", THEME_PACKS.get(self.theme_pack).copied().unwrap_or("Standard"));
        s.put_centered(g, crate::ui::F_KLEPTO18, MENU_LABEL_X[1], MENU_LABEL_Y[1] + 24.0, &theme, grey);
        if !self.toast.is_empty() && self.clock < self.toast_until {
            let w = text_width(&self.toast, 18.0) + 24.0;
            s.fill(g, 400.0 - w / 2.0, 560.0, w, 30.0, rgba(0.1, 0.1, 0.3, 0.9));
            s.text_centered(g, 400.0, 581.0, &self.toast, 18.0, rgb(1.0, 1.0, 0.8));
        }
        g.flush();
    }

    /// Select Difficulty: the screen, the hovered item lit from the sheet, the four names (black when hovered, grey a little
    /// lower otherwise) and the title.
    pub fn draw_difficulty(&mut self, g: &mut Gfx) {
        let s = Ui::new(self.draw_w, self.draw_h);
        self.view = s.view;
        s.image(g, &self.diff_base, 0.0, 0.0);
        if let Some(&(sx, sy, w, h, dx, dy)) = DIFF_CUTS.get(self.hover.max(0) as usize).filter(|_| (0..4).contains(&self.hover)) {
            s.image_part(g, &self.diff_mo, dx, dy, sx, sy, w, h);
        }
        if self.hover == 100 {
            s.image_part(g, &self.diff_mo, 732.0, 532.0, 732.0, 532.0, 68.0, 68.0);
        }
        // 0x43a400: all in Klepto ITC 24 (0x519a40), centred with the given tops; the hovered name black, the others in
        // 0x4210, two pixels lower
        let k24 = crate::ui::F_KLEPTO24;
        s.put_centered(g, k24, 602.0, 42.0, "Select Difficulty", rgb(0.0, 0.0, 0.0));
        for (i, &(x, y)) in DIFF_LABELS.iter().enumerate() {
            if self.hover == i as i32 {
                s.put_centered(g, k24, x, y, DIFFICULTY_NAMES[i], rgb(0.0, 0.0, 0.0));
            } else {
                s.put_centered(g, k24, x, y + 2.0, DIFFICULTY_NAMES[i], crate::info_ui::c15(0x4210));
            }
        }
        g.flush();
    }

    /// Heads-up display over the course: the exe's face strip, badge and rating pills (hud_ui), then the dock, advisor and
    /// story.
    pub fn draw_hud(&mut self, g: &mut Gfx) {
        if !self.ui_ok {
            return;
        }
        let s = Ui::new(self.draw_w, self.draw_h);
        self.view = s.view;
        // the golfers' names and thoughts belong to the course, under the HUD
        self.draw_names(g, &s);
        self.draw_thoughts(g, &s);
        // footage of the original: the End of Year report hides the HUD (dock, name plate, rating pills, leader board) and
        // leaves the golfers' names and words on the course
        if self.screen == Screen::YearEnd {
            g.flush();
            return;
        }
        if self.hud_art.tex.is_some() {
            // the exe's face strip, badge and rating pills (hud_ui)
            self.draw_exe_hud(g, &s);
        } else {
            // a plain stand-in without the art
            let mi = self.econ.month_index();
            let date = format!("{} {}", MONTHS[((2 + mi % 8) % 12) as usize], 2001 + mi / 8);
            let panel_bg = rgba(0.12, 0.1, 0.3, 0.78);
            let red = self.econ.cash < 0.0 && !self.econ.sandbox;
            let cash = if self.econ.sandbox { "Sandbox".to_string() } else { money(self.econ.cash as i64) };
            let rt = self.club.ratings;
            let fun = format!("{}", rt.fun);
            let skill = format!("{}.{:02}", rt.skill / 100, (rt.skill % 100).abs());
            s.fill(g, 8.0, 8.0, 230.0, 46.0, panel_bg);
            s.text(g, 18.0, 28.0, &self.course_name, 17.0, rgb(1.0, 1.0, 1.0));
            s.text(g, 18.0, 47.0, &date, 14.0, rgb(0.85, 0.85, 1.0));
            s.fill(g, 560.0, 8.0, 232.0, 52.0, panel_bg);
            s.text(g, 572.0, 30.0, &cash, 19.0, if red { rgb(1.0, 0.5, 0.5) } else { rgb(1.0, 1.0, 0.7) });
            s.text(g, 572.0, 52.0, &format!("Fun {fun}  Skill {skill}"), 15.0, rgb(0.9, 0.9, 1.0));
        }
        self.draw_floats(g, &s);
        self.draw_advisor(g, &s);
        self.draw_dock(g, &s);
        self.draw_leaderboard(g, &s);
        self.draw_card(g, &s);
        self.draw_simfoto(g, &s);
        self.draw_paused(g, &s);
        // the ticker goes over the golfer card, as in the exe's frame
        self.draw_ticker(g, &s);
        // without the Player panel's art the aim's lines go in a box of their own (with it they are in the panel)
        if !self.player_panel_ready() || self.panel != 5 {
            if let Some((lines, skills)) = self.aim_text() {
                let rows: Vec<(String, bool)> = lines.into_iter().map(|l| (l, true)).chain(skills).collect();
                let h = 12.0 + 16.0 * rows.len() as f32;
                s.fill(g, 600.0, 440.0 - h, 192.0, h, rgba(0.12, 0.1, 0.3, 0.85));
                for (i, (l, on)) in rows.iter().enumerate() {
                    let c = if *on { rgb(1.0, 1.0, 1.0) } else { rgb(0.55, 0.55, 0.65) };
                    s.text(g, 608.0, 440.0 - h + 20.0 + 16.0 * i as f32, l, 13.0, c);
                }
            }
        }
        g.flush();
    }

    /// The exe's dock mode (0x567afc) for the open panel: 0 Build Course, 1 Add Buildings, 2 People (golfers, player and the
    /// employee overlay), -1 none.
    pub fn dock_mode(&self) -> i32 {
        match self.panel {
            1 => 0,
            2 => 1,
            3..=5 => 2,
            _ => -1,
        }
    }

    /// The exe's raw dock mode: the Golfers panel is mode 2, the Player panel mode 4 (the port has no separate mode 3), and
    /// the Employee overlay keeps the mode it was opened from.
    pub fn exe_dock_mode(&self) -> i32 {
        let p = if self.panel == 3 { self.pstate.emp_base } else { self.panel };
        match p {
            4 => 2,
            5 => 4,
            _ => self.dock_mode(),
        }
    }

    fn draw_dock(&mut self, g: &mut Gfx, s: &Ui) {
        if self.dock_art.tex.is_some() {
            s.image_part(g, &self.dock_art, 0.0, 430.0, 0.0, 430.0, 215.0, 170.0);
            // 0x432ba0: the hovered button in its bright blue look (column 0 of the sheet), unless it is the open mode's (the
            // raw mode, so People lights over the Player panel, mode 4)
            let mode = self.exe_dock_mode();
            if self.dock_hover >= 0 && self.dock_hover != mode {
                let (sx, sy, w, h, dx, dy) = DOCK_LIT[self.dock_hover as usize];
                s.image_part(g, &self.dock_art, dx, dy, sx, sy, w, h);
            }
            // while paused the Pause button blinks gold (0x5a9f5c counts the paused frames, lit while bit 2 is set)
            if self.paused {
                self.dock_blink = self.dock_blink.wrapping_add(1);
                if self.dock_blink & 4 != 0 {
                    let (sx, sy, w, h, dx, dy) = DOCK_LIT[8];
                    s.image_part(g, &self.dock_art, dx, dy, sx + 50.0, sy, w, h);
                }
            }
            // the open mode's button in gold (column 100) and its gold trail along the dock towards the panel (0x587534 +
            // mode), as footage of the original shows: Build Course gold with the trail while the terrain panel is open
            if (0..3).contains(&mode) {
                let m = mode as usize;
                let (sx, sy, w, h, dx, dy) = DOCK_LIT[m];
                s.image_part(g, &self.dock_art, dx, dy + if m == 2 { 1.0 } else { 0.0 }, sx + 100.0, sy, w, h);
                let (tx, ty) = DOCK_TRAIL[m];
                s.image_part(g, &self.dock_art, tx, ty, 300.0, 100.0 * m as f32, 214.0, 91.0);
            }
            // the tooltip bubble after 11 still frames (0x432620): a translucent black bar 6 pixels a letter wide at the
            // pointer, the caption centred 5 pixels above it in white
            if self.dock_hover >= 0 && self.dock_hover == self.dock_tip.0 {
                self.dock_tip.1 += 1;
            } else {
                self.dock_tip = (self.dock_hover, 0);
            }
            if self.dock_hover >= 0 && self.dock_tip.1 > 11 {
                let tip = DOCK_HELP[self.dock_hover as usize];
                let n = tip.chars().count() as f32;
                let (px, py) = self.info.pointer;
                let x = px.clamp(3.0 * n, 800.0 - 3.0 * n);
                let y = py - 5.0;
                // APPROXIMATION: the bar is a 10 pixel line through y; its blend is not decoded
                s.fill(g, x - 3.0 * n, y - 5.0, 6.0 * n, 10.0, rgba(0.0, 0.0, 0.0, 0.5));
                s.text_centered(g, x, y + 4.0, tip, 11.0, rgb(1.0, 1.0, 1.0));
            }
        }
        if self.art_panel_open() {
            self.draw_panel(g, s);
        } else if self.panel == 4 {
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
        // Story lines (port-only, until the exe's portrait bubbles for story talk are drawn), below the ticker's place.
        if self.show_advisor {
            let y = 124.0;
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
        if self.pstate.hire_open && self.panel == 3 && self.panel_art_ready() {
            self.draw_hire_dialog(g, s);
        }
    }
}
