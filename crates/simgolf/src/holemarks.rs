//! What the course view shows of holes besides their flags and tee markers: the numbered signposts on the tees of Top 100 and
//! Top 18 holes (Flics/Bldgs/holemark.pcx), the hole being built (the trial golfer's shots from tee to pin, and its number,
//! length and par over the green), and the instant shot analysis of the '/' and '.' keys.
//!
//! Facts are from the publisher's golf.exe main routine (0x40f5c0), restated in our own words.

use crate::app::*;
use crate::gfx::{Gfx, Uniforms};
use crate::screens_ui::{c15, top, BODY, LARGE};
use crate::ui::{rgb, Screen as Ui};
use sg_core::analysis::KINDS;
use sg_core::assets::decode_pcx;
use sg_core::sprites::Sprite;

/// Exe colour words used here (0x8000 plus 15-bit RGB): the analysis title, its rule, the black of shadows and circles.
const CYAN: u32 = 0x3ff;
const RULE: u32 = 0x210;
const WHITE: u32 = 0x7fff;

/// The signpost of hole h (1..18) cut from holemark.pcx, as the loader at 0x448012 cuts the sheet: three sets of 18, nine to a
/// row, 17 x 14 at (i%9*17+3, i/9*14+4), 18 x 17 at (i%9*18+4, i/9*17+36) and 19 x 21 at (i%9*19+4, i/9*21+73). The tee loop
/// (0x4122a9) draws only the second set (Top 100) and the third (Top 18); the first is never drawn. Returns the cut and the
/// point that sits on the ground: (9, 16 + 3 * set).
fn holemark_cut(h: usize, set: usize) -> (u32, u32, u32, u32, i32, i32) {
    let i = (h - 1) as u32;
    let (w, ht, x0, y0) = match set {
        0 => (17, 14, 3, 4),
        1 => (18, 17, 4, 0x24),
        _ => (19, 21, 4, 0x49),
    };
    (i % 9 * w + x0, i / 9 * ht + y0, w, ht, 9, 16 + 3 * set as i32)
}

impl App {
    /// A signpost as a one-frame sprite, cut and colour keyed on first use. `up` lifts it 12 pixels instead of lowering it.
    fn holemark_sprite(&mut self, h: usize, set: usize, up: bool) -> Option<usize> {
        let key = format!("holemark|{h}|{set}|{up}");
        if let Some(v) = self.sprite_index.get(&key) {
            return *v;
        }
        let sheet = sg_core::fsutil::read_file(self.game_path("Flics/Bldgs/holemark.pcx")).and_then(|d| decode_pcx(&d).ok());
        let r = sheet.map(|sheet| {
            let (x0, y0, w, ht, ax, ay) = holemark_cut(h, set);
            let mut img = sg_core::Rgba::new(w, ht);
            for y in 0..ht {
                for x in 0..w {
                    let (sx, sy) = ((x0 + x).min(sheet.w - 1), (y0 + y).min(sheet.h - 1));
                    let s = ((sy * sheet.w + sx) * 4) as usize;
                    let d = ((y * w + x) * 4) as usize;
                    let p = &sheet.px[s..s + 4];
                    let key = p[0] == 255 && p[1] == 0 && p[2] == 255;
                    // the pure green parallelogram beside each post is its shadow; PLACEHOLDER: how the renderer draws
                    // that colour is not decoded, so it is half-strength black here
                    let shadow = p[0] == 0 && p[1] == 255 && p[2] == 0;
                    let px = if shadow { [0, 0, 0, 128] } else { [p[0], p[1], p[2], if key { 0 } else { 255 }] };
                    img.px[d..d + 4].copy_from_slice(&px);
                }
            }
            let s = Sprite {
                views: 1,
                frames_per_view: 1,
                anchor_x: ax,
                anchor_y: ay + if up { 12 } else { -12 },
                w,
                h: ht,
                frame_ms: 0,
                view_mask: 0,
                shadow: false,
                frames: vec![img],
                indexed: Vec::new(),
            };
            self.sprites.push(GlSprite { s, tex: vec![None] });
            self.sprites.len() - 1
        });
        self.sprite_index.insert(key, r);
        r
    }

    /// The signposts on the tees of open Top 100 and Top 18 holes (0x4122a9): the Top 18 set when the hole has that flag, the
    /// Top 100 set otherwise, drawn at the tee 3 pixels per zoom step above it when the tee faces away from the viewer
    /// ((facing - view rotation) & 4), below it otherwise, so it stands behind the tee. The sprite scales with the zoom, so the
    /// 3 pixels a step are always 12 of its own.
    pub(crate) fn push_holemarks(&mut self) {
        let quarter = ((self.rot / 90.0).round() as i32).rem_euclid(4);
        for h in 1..19 {
            let rec = &self.club.holes[h];
            if rec.par == 0 || rec.flags & 3 == 0 {
                continue;
            }
            let set = if rec.flags & 2 != 0 { 2 } else { 1 };
            // the exe's view rotation steps by 2 of 8 per quarter turn
            let up = (rec.tee_facing - 2 * quarter) & 4 != 0;
            let mut tees = vec![rec.back];
            if rec.fwd != rec.back && rec.fwd.0 != 0 {
                tees.push(rec.fwd);
            }
            let Some(si) = self.holemark_sprite(h, set, up) else { continue };
            for (a, b) in tees {
                let (x, z) = self.terrain.tile_centre(a, b);
                self.props.push(Prop { x, z, body: Some(si), decor: true, ..Default::default() });
            }
        }
    }

    /// Virtual pixels to world units at the current zoom, and the eye's right and up axes in world space.
    fn eye_axes(&self) -> (f32, [f32; 3], [f32; 3]) {
        let vs = (self.draw_w / 800.0).min(self.draw_h / 600.0);
        let m = self.mv;
        (self.upp * vs, [m[0], m[4], m[8]], [m[1], m[5], m[9]])
    }

    /// The trial golfer's shots over the hole being built (0x41370e, every frame once it has a tee and a green): the golfer
    /// with all three skills from the back tee, a segment per shot to the tile it lands on, up to five, stopping at a landing on
    /// a hazard (not drawn) or the pin. Each is a white line 2 pixels wide over a black one a pixel lower, both 6 pixels right
    /// of the true line (3 per trial golfer, this being the second). The first golfer's tee shot only sets the tee facing.
    pub(crate) fn draw_layout_path(&mut self, g: &mut Gfx, u: &Uniforms) {
        let h = self.club.next_hole;
        if !(1..19).contains(&h) {
            return;
        }
        let rec = &self.club.holes[h as usize];
        let (back, pin) = (rec.back, rec.pin);
        if back.0 == 0 || pin.0 == 0 || self.club.layout_land.len() < 2 {
            return;
        }
        let (px, r, up) = self.eye_axes();
        let shift = |p: [f32; 3], dx: f32, dy: f32| {
            let (dx, dy) = (dx * px, -dy * px);
            [p[0] + r[0] * dx + up[0] * dy, p[1] + r[1] * dx + up[1] * dy, p[2] + r[2] * dx + up[2] * dy]
        };
        let at = |app: &App, (a, b): (i32, i32)| {
            let (x, z) = app.terrain.tile_centre(a, b);
            [x, app.terrain.height_at(x, z), z]
        };
        let vs = (self.draw_w / 800.0).min(self.draw_h / 600.0);
        let mut start = at(self, back);
        let lands: Vec<(i32, i32)> = self.club.layout_land[1..].to_vec();
        for (n, l) in lands.into_iter().enumerate() {
            let ty =
                if sg_core::course::inside(l.0, l.1) { self.course.ty[sg_core::course::idx(l.0, l.1)] } else { sg_core::course::t::OUT };
            if self.course.h(ty) > 1 {
                break;
            }
            let end = at(self, l);
            self.draw_lines(g, u, &[shift(start, 6.0, 1.0), shift(end, 6.0, 1.0)], false, vs, [0.0, 0.0, 0.0, 1.0]);
            self.draw_lines(g, u, &[shift(start, 6.0, 0.0), shift(end, 6.0, 0.0)], false, vs, c15(WHITE));
            if n + 1 >= 5 || l == pin {
                break;
            }
            start = end;
        }
    }

    /// Over the green of the hole being built (0x417277), when the green is on screen: "Hole N", "N yards" and "Par N" in
    /// white body text over a black shadow a pixel lower (0x404bc0), centred, their tops 26, 13 and 0 pixels above a point 5 pixels per zoom step over the green; and over
    /// the back tee, blinking on the game tick's bits 2 and 3, "Press 'h' to open hole".
    pub fn draw_building_label(&mut self, g: &mut Gfx) {
        let h = self.club.next_hole;
        if !(1..19).contains(&h) || self.edit_hidden_label() {
            return;
        }
        let rec = &self.club.holes[h as usize];
        let (back, pin) = (rec.back, rec.pin);
        if back.0 == 0 || pin.0 == 0 {
            return;
        }
        let centre = |(a, b): (i32, i32)| (a * 0x400 + 0x200, b * 0x400 + 0x200);
        let (gx, gy) = centre(pin);
        let Some((x, y)) = self.screen_of(gx, gy) else { return };
        let Some((h, yards, par)) = self.club.building_figures(&self.course) else { return };
        let s = Ui::new(self.draw_w, self.draw_h);
        let lift = 5.0 * (self.zoom / ZOOM_UNIT).round().max(1.0);
        // 0x404bc0: white with the palette's black one pixel below
        let label = |g: &mut Gfx, x: f32, y: f32, t: &str| {
            s.text_centered(g, x, top(y + 1.0, BODY), t, BODY, rgb(0.0, 0.0, 0.0));
            s.text_centered(g, x, top(y, BODY), t, BODY, c15(WHITE));
        };
        label(g, x, y - lift - 26.0, &format!("Hole {h}"));
        label(g, x, y - lift - 13.0, &format!("{yards} yards"));
        label(g, x, y - lift, &format!("Par {par}"));
        let (tx, ty) = centre(back);
        if let Some((x, y)) = self.screen_of(tx, ty) {
            if self.game_tick & 0xc != 0 {
                label(g, x, y - lift, "Press 'h' to open hole");
            }
        }
        g.flush();
    }

    /// The label is part of the course view; screens drawn over the course hide it.
    fn edit_hidden_label(&self) -> bool {
        self.screen != Screen::Play
    }

    /// '/' (0x2f) analyses the hole nearest the tile under the pointer and keeps it; '.' (0x2e) analyses the kept hole again
    /// from the pointer's tile. `at` overrides the pointer (scripted runs).
    pub fn start_analysis(&mut self, at: Option<(i32, i32)>, nearest: bool) {
        let Some((a, b)) = at.or(self.hover_tile) else { return };
        self.sync_course();
        if nearest {
            self.analysis_hole = self.club.nearest_hole(a, b);
        }
        let an = self.club.shot_analysis(&mut self.course, &mut self.exe_rng, a, b, self.analysis_hole);
        self.analysis = Some(an);
        self.analysis_shown = 0;
    }

    /// A key or click while the analysis is up: once all twelve shots are drawn it closes it (the exe waits at 0x45c0c0 and
    /// then puts the screen back). Returns whether the input was taken.
    pub fn analysis_input(&mut self) -> bool {
        if self.analysis.is_none() {
            return false;
        }
        if self.analysis_shown >= 12 {
            self.analysis = None;
        }
        true
    }

    /// The analysis over the frozen screen (0x41d078), one more shot each frame as the exe presents one per shot: a black
    /// circle of 10 yards round the tile the shots start from; the panel (0x40cef0 at 282, 26, 256 x 148, grown to whole
    /// 16 pixel pieces) with its rule at y 64 and the four kinds' labels at x 296, y 144 - 20 * kind; per shot a circle of
    /// 5 + kind yards round where it stopped, the ball there (a short white dash when it stopped on a hazard) and its ground
    /// track a pixel wide, raised by its kind's number of pixels, in the kind's colour; with the last shot the title, the hole
    /// line and each kind's figure against the all-skills golfers.
    pub fn draw_analysis(&mut self, g: &mut Gfx) {
        let Some(an) = self.analysis.clone() else { return };
        self.analysis_shown = (self.analysis_shown + 1).min(an.shots.len());
        let s = Ui::new(self.draw_w, self.draw_h);
        let black = rgb(0.0, 0.0, 0.0);
        let circle = |app: &App, g: &mut Gfx, (x, y): (i32, i32), yards: i32, c: [f32; 4]| {
            // 0x407c60: 24 steps round, a line 2 pixels wide, broken where it leaves the screen
            let r = ((yards << 10) / 25) as f32;
            let mut last: Option<(f32, f32)> = None;
            for k in 0..25 {
                let a = k as f32 * std::f32::consts::TAU / 24.0;
                let p = app.screen_of(x + (a.sin() * r) as i32, y - (a.cos() * r) as i32);
                if let (Some(p0), Some(p1)) = (last, p) {
                    s.line(g, p0.0, p0.1, p1.0, p1.1, 2.0, c);
                }
                last = p;
            }
        };
        let (fa, fb) = an.from;
        circle(self, g, (fa * 0x400 + 0x200, fb * 0x400 + 0x200), 10, black);
        self.art.trans_frame(g, &s, 282.0, 21.0, 256.0, 159.0);
        s.line(g, 282.0, 64.5, 538.0, 64.5, 1.0, c15(RULE));
        for (k, &(_, label, col)) in KINDS.iter().enumerate().take(self.analysis_shown.min(4)) {
            s.text(g, 296.0, top(144.0 - 20.0 * k as f32, LARGE), label, LARGE, c15(col));
        }
        for shot in an.shots.iter().take(self.analysis_shown) {
            let k = shot.kind;
            let col = c15(KINDS[k].2);
            let end = shot.end();
            circle(self, g, end, k as i32 + 5, col);
            if let Some((x, y)) = self.screen_of(end.0, end.1) {
                if shot.hazard <= 0 {
                    s.fill(g, x - 2.0, y - 1.0, 4.0, 2.0, rgb(0.1, 0.1, 0.1));
                    s.fill(g, x - 1.5, y - 2.5, 3.0, 3.0, c15(WHITE));
                } else {
                    s.line(g, x, y + 0.5, x + 2.0, y + 0.5, 1.0, c15(WHITE));
                }
            }
            let mut last: Option<(f32, f32)> = None;
            for &(x, y) in &shot.trace {
                if let Some((sx, sy)) = self.screen_of(x, y) {
                    let p = (sx, sy + 1.0 - k as f32);
                    if let Some(p0) = last {
                        s.line(g, p0.0, p0.1, p.0, p.1, 1.0, col);
                    }
                    last = Some(p);
                }
            }
        }
        if self.analysis_shown >= an.shots.len() {
            s.text_centered(g, 410.0, top(34.0, LARGE), "Shot Analysis:", LARGE, c15(CYAN));
            s.text_centered(g, 410.0, top(50.0, BODY), &format!("Golfers playing hole {}...", an.hole), BODY, c15(CYAN));
            for k in 0..3 {
                s.text_centered(g, 490.0, top(147.0 - 20.0 * k as f32, BODY), &an.figure(k), BODY, c15(KINDS[k].2));
            }
        }
        g.flush();
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn cuts_follow_the_loader() {
        assert_eq!(holemark_cut(1, 0), (3, 4, 17, 14, 9, 16));
        assert_eq!(holemark_cut(10, 1), (4, 0x24 + 17, 18, 17, 9, 19));
        assert_eq!(holemark_cut(18, 2), (8 * 19 + 4, 0x49 + 21, 19, 21, 9, 22));
    }
}
