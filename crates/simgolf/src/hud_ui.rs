//! The main screen's heads-up display as the exe draws it every frame (main frame 0x418d09 to 0x41975d): the golfer face
//! strip along the bottom, the course badge top left (emblem, name, date, a row of accomplishment icons and the home site
//! waiting list) and the three rating pills top right (cash, fun, skill). Positions are the exe's, in 800x600 pixels; text
//! positions are the tops of the text (ours are baselines, see `top`).
//!
//! Text styles (0x476310 sets colour, shadow colour, shadow dx, dy): the centred and left calls 0x404b70 / 0x4049d0 pass a
//! shadow colour of -1, i.e. no shadow; 0x404bc0 / 0x404ad0 pass shadow colour 1 (`ui::SHADOW_1`, dark red) one pixel below.

use crate::app::*;
use crate::gfx::Gfx;
use crate::info_ui::{c15, hundredths};
use crate::screens_ui::top;
use crate::ui::{self, group, load_pcx, rgba, Face, Image, Screen as Ui};
use sg_core::economy::{self, Economy};
use sg_core::golfer::{flag, game};
use sg_core::land;

/// courseinfo.pcx: the badge plate and the three pills, sheet x, y, w, h as cut by the loader 0x446943, then where the
/// main frame draws each one (the pills two pixels right of their place on the sheet, the plate one pixel up).
const PLATES: [[f32; 6]; 4] = [
    [48.0, 6.0, 183.0, 58.0, 48.0, 5.0],
    [647.0, 13.0, 138.0, 36.0, 649.0, 13.0],
    [675.0, 55.0, 110.0, 37.0, 677.0, 55.0],
    [697.0, 99.0, 88.0, 36.0, 699.0, 99.0],
];

/// The club emblem of each property (byte 0 of the 0x82 byte site records at 0x4c1ea8): 0..7 are the cuts of
/// tropdesert.pcx, 8..15 those of parklink.pcx.
const SITE_EMBLEM: [usize; 16] = [9, 4, 11, 3, 0, 7, 6, 8, 15, 10, 14, 12, 13, 2, 5, 1];

/// The date's months (0x4c2908): a year is eight months of 1024 ticks.
const MONTHS: [&str; 8] = ["March", "April", "May", "June", "July", "August", "September", "October"];

/// The short course rank suffix the badge appends (0x40daa0 with argument 0), by 0x44faf0's rank of the open holes:
/// Municipal, Golf Club, Country Club, Championship. The Championship one points into zeroed data, so it is empty.
const RANK_SUFFIX: [&str; 4] = [" MC", " GC", " CC", ""];
/// The long class words (0x40daa0 with argument 1).
const RANK_LONG: [&str; 4] = ["Municipal", "Golf Club", "Country Club", "Championship"];

/// GBUBBLES.pcx icons used by the badge: 16 x 16 cuts at (16 * i, 324) (array 0x59b050).
const ICON_BALL: usize = 13;
const ICON_STAR: usize = 17;
const ICON_SIGN: usize = 20;
const ICON_HEART: usize = 30;
const ICON_TROPHY: usize = 33;

/// The face strip: golfer slots scanned, first face x, pitch, last face x, and the strip's y.
const STRIP_SLOTS: i32 = 0x98;
const STRIP_X: f32 = 204.0;
const STRIP_PITCH: f32 = 16.0;
const STRIP_LAST_X: f32 = 784.0;
const STRIP_Y: f32 = 575.0;

const ARIAL: f32 = 10.0; // font object 0x519fd8, Arial Bold 10
const MANUAL: f32 = 15.0; // font object 0x51b360, Manual SSi Bold 15
/// Text tops of the badge's name and date (footage of the original, see `draw_badge`).
const BADGE_NAME_Y: f32 = 11.0;
const BADGE_DATE_Y: f32 = 25.0;

/// The art the HUD needs besides courseinfo.pcx (App::hud_art), and the golfer of each face drawn this frame.
#[derive(Default)]
pub struct HudArt {
    /// s_courseinfo.pcx: where the plates darken the course (green on magenta).
    pub shadow: Image,
    /// MemberPanel.pcx: the ten 16 x 16 mood faces.
    pub faces: Image,
    /// GBUBBLES.pcx: the small icons.
    pub icons: Image,
    /// The golfer slot under each face of the strip, left to right (0x567ff4).
    pub strip: Vec<usize>,
}

impl HudArt {
    pub fn load(g: &mut Gfx, app: &App) -> HudArt {
        let i = |rel: &str| app.game_path(rel);
        HudArt {
            shadow: load_pcx(g, &i("Interface/s_courseinfo.pcx"), true, None).unwrap_or_default(),
            // the cuts are colour keyed on each sheet's background (DERIVED: the colour at the sheet's corner)
            faces: load_pcx(g, &i("Interface/MemberPanel.pcx"), true, Some(0x6b6b9c)).unwrap_or_default(),
            icons: load_pcx(g, &i("GBUBBLES.pcx"), false, Some(0x9be7ff)).unwrap_or_default(),
            strip: Vec::new(),
        }
    }
}

/// "x12" left aligned with no shadow, as the badge's counts.
fn count(s: &Ui, g: &mut Gfx, x: f32, y: f32, n: i32) {
    s.text(g, x, top(y, ARIAL), &format!("x{n}"), ARIAL, c15(0x7fff));
}

/// Centred text with the exe's pill shadow (0x404bc0): palette colour 1 one pixel below (`ui::SHADOW_1`, dark red in
/// footage of the original).
fn shadowed(s: &Ui, g: &mut Gfx, x: f32, y: f32, t: &str, c: [f32; 4]) {
    shadowed_in(s, g, x, y, t, c, ui::SHADOW_1);
}

/// Centred Manual SSi 15 text over a shadow of the given colour one pixel below.
fn shadowed_in(s: &Ui, g: &mut Gfx, x: f32, y: f32, t: &str, c: [f32; 4], shadow: [f32; 4]) {
    s.text_centered(g, x, top(y, MANUAL) + 1.0, t, MANUAL, shadow);
    s.text_centered(g, x, top(y, MANUAL), t, MANUAL, c);
}

/// A row of n icons centred near x 148 at y: one icon per count while there are few, else one icon and "xN".
/// `spread` is the width the row may use (160 for the accomplishments, 150 for the waiting list).
fn icon_step(n: i32, spread: i32) -> (f32, f32) {
    let step = (spread / (n + 1)).clamp(2, 12);
    (148.0 - (step * n / 2) as f32, step as f32)
}

impl App {
    fn icon(&self, g: &mut Gfx, s: &Ui, i: usize, x: f32, y: f32) {
        s.image_part(g, &self.hud.icons, x, y, 16.0 * i as f32, 324.0, 16.0, 16.0);
    }

    /// The course name as the badge shows it (0x40daa0 with argument 0): the club's name and the short rank suffix, e.g.
    /// "Ocean Grove MC" on the Florida property (as in footage of the original). The exe adds the suffix of the current
    /// rank to any name.
    pub fn hud_course_name(&self) -> String {
        let site = self.land.as_ref().and_then(|l| sg_core::properties::PROPERTIES.get(l.slot.property));
        let base = match site {
            // an old save's default "<property> GC": the property's course name
            Some(p) if self.course_name == format!("{} GC", p.name) => p.course,
            _ => self.course_name.as_str(),
        };
        format!("{base}{}", RANK_SUFFIX[economy::rank(self.hole_numbers.len()) as usize])
    }

    /// The course name with the long class word (0x40daa0 with argument 1): "Dolphin Coast Municipal" on the routing map in
    /// footage of the original.
    pub fn long_course_name(&self) -> String {
        format!("{} {}", self.base_course_name(), RANK_LONG[economy::rank(self.hole_numbers.len()) as usize])
    }

    /// The course name with no class at all (0x40daa0 with argument -1): the championship leader board's "at <name>".
    pub fn base_course_name(&self) -> String {
        let short = self.hud_course_name();
        let rank = economy::rank(self.hole_numbers.len()) as usize;
        short.strip_suffix(RANK_SUFFIX[rank]).unwrap_or(&short).to_string()
    }

    /// The face strip (0x418d09): every golfer on a hole (1 to 18, or 19 going home), newest first, a mood face 16 pixels
    /// apart from x 204 to 784 along the bottom, the hole number under it on a white box (lavender for a pro or VIP), its
    /// colour the golfer's worst need. With ten or more holes open, holding Shift shows only the golfers on the back nine.
    fn draw_face_strip(&mut self, g: &mut Gfx, s: &Ui) {
        self.hud.strip.clear();
        let back_nine = self.shift_held && self.hole_numbers.len() >= 10;
        let mut x = STRIP_X;
        ui::set_face(Some(Face::Arial));
        for i in 0..STRIP_SLOTS {
            let slot = (self.club.last_created - i).rem_euclid(STRIP_SLOTS) as usize;
            let Some(gg) = self.club.g.get(slot) else { continue };
            if gg.hole <= 0 || (back_nine && gg.hole <= 9) {
                continue;
            }
            let k = (gg.mood + 2).clamp(1, 10);
            s.image_part(g, &self.hud.faces, x, STRIP_Y, 594.0 - 16.0 * (k - 1) as f32, 100.0, 16.0, 16.0);
            s.fill(g, x + 1.0, 591.0, 14.0, 9.0, c15(if gg.kind != 0 { 0x631f } else { 0x7fff }));
            // later rules win: tired grey, thirsty blue, hungry red, leaving orange
            let mut c = 0x0000;
            if gg.fatigue > 160 {
                c = 0x4210;
            }
            if gg.thirst > 16 {
                c = 0x0018;
            }
            if gg.hunger > 16 {
                c = 0x6000;
            }
            if gg.flags & flag::LEAVING != 0 {
                c = 0x7d08;
            }
            s.text_centered(g, x + 8.0, top(591.0, ARIAL), &gg.hole.to_string(), ARIAL, c15(c));
            self.hud.strip.push(slot);
            x += STRIP_PITCH;
            if x > STRIP_LAST_X {
                break;
            }
        }
        ui::set_face(None);
    }

    /// A click on the face strip while no dock panel is open (0x41eefe): the golfer under the face gets the golfer card and
    /// the view centres on it; a click past the last face closes the card.
    pub fn strip_click(&mut self, vx: f32, vy: f32) -> bool {
        if self.hud_art.tex.is_none() || self.panel != 0 || vx < STRIP_X || vy <= STRIP_Y || self.club.game & game::TOURNAMENT != 0 {
            return false;
        }
        let k = ((vx - STRIP_X) / STRIP_PITCH) as usize;
        match self.hud.strip.get(k) {
            Some(&gi) => {
                let (x, z) = self.units_to_world(self.club.g[gi].x, self.club.g[gi].y);
                self.cam_x = x;
                self.cam_z = z;
                if self.card != Some(gi) {
                    self.card_ui = crate::screens_ui::CardUi { mouse: self.card_ui.mouse, ..Default::default() };
                }
                self.card = Some(gi);
            }
            None => self.card = None,
        }
        true
    }

    /// The badge (0x418ee9): the darkened course under the plate, the plate, the club emblem, the name and date, then the
    /// accomplishment icons and the home site waiting list.
    fn draw_badge(&self, g: &mut Gfx, s: &Ui) {
        let [sx, sy, w, h, dx, dy] = PLATES[0];
        s.image_part_tint(g, &self.hud.shadow, dx, dy, sx, sy, w, h, rgba(0.0, 0.0, 0.0, 0.5));
        s.image_part(g, &self.hud_art, dx, dy, sx, sy, w, h);
        if let Some(&e) = self.land.as_ref().and_then(|l| SITE_EMBLEM.get(l.slot.property)) {
            let (sheet, cx, cy) = if e < 8 { (1, e, if e < 4 { 481.0 } else { 480.0 }) } else { (0, e - 8, 395.0) };
            s.image_part(g, &self.title.art.emblems[sheet], 3.0, 3.0, 3.0 + 88.0 * cx as f32, cy, 80.0, 80.0);
        }
        // Footage of the original (Dolphin Coast at May 2001, June 2003 and July 2007) shows the name in the pills' font,
        // Manual SSi Bold 15, and the date in Arial Bold 10, both white with the black shadow one pixel below; the name's
        // capitals span y 11 to 20 and the date's y 26 to 32, centred on x 143. (The decode had both in Arial without a
        // shadow at 13 and 27.)
        let white = c15(0x7fff);
        ui::set_face(Some(Face::Manual));
        let black = rgba(0.0, 0.0, 0.0, 1.0);
        shadowed_in(s, g, 143.0, BADGE_NAME_Y, &self.hud_course_name(), white, black);
        ui::set_face(Some(Face::Arial));
        let t = self.econ.tick;
        let date = format!("{} {}", MONTHS[((t >> 10) & 7) as usize], 2001 + (t >> 13));
        s.text_centered(g, 143.0, top(BADGE_DATE_Y, ARIAL) + 1.0, &date, ARIAL, black);
        s.text_centered(g, 143.0, top(BADGE_DATE_Y, ARIAL), &date, ARIAL, white);
        // holes named Top 100 and Top 18, homes sold to celebrities, tournament fame, Happy Endings (0x418bc7)
        let ranked = self.club.holes.iter().skip(1).take(18).map(|h| (h.flags & 1 != 0) as i32 + (h.flags & 2 != 0) as i32).sum();
        let sold =
            self.land.as_ref().map(|l| l.objects.iter().filter(|o| o.kind == land::K_HOME_SITE && o.sub != 0).count() as i32).unwrap_or(0);
        let rows = [(ICON_BALL, ranked), (ICON_STAR, sold), (ICON_TROPHY, self.club.trophies), (ICON_HEART, self.club.happy_endings)];
        let n: i32 = rows.iter().map(|r| r.1).sum();
        if n < 20 {
            let (mut x, step) = icon_step(n, 160);
            for (icon, k) in rows {
                for _ in 0..k {
                    self.icon(g, s, icon, x, 38.0);
                    x += step;
                }
            }
        } else {
            for (i, (icon, k)) in rows.into_iter().enumerate() {
                let x = 80.0 + 36.0 * i as f32;
                self.icon(g, s, icon, x, 38.0);
                count(s, g, x + 14.0, 38.0, k);
            }
        }
        // the home site waiting list (0x4193b0): Silver and better members without a home site
        let n = self.club.homesite_demand;
        if n > 0 {
            if n < 10 {
                let (mut x, step) = icon_step(n, 150);
                for _ in 0..n {
                    self.icon(g, s, ICON_SIGN, x, 50.0);
                    x += step;
                }
            } else {
                self.icon(g, s, ICON_SIGN, 136.0, 50.0);
                count(s, g, 150.0, 50.0, n);
            }
        }
        ui::set_face(None);
    }

    /// The rating pills (0x4194b5), drawn in every mode: each over its own darkened course; cash in green (red and blinking
    /// every 4 ticks while in debt, nothing in a sandbox), the fun rating in pale yellow, the skill rating in cyan.
    fn draw_pills(&self, g: &mut Gfx, s: &Ui) {
        for &[sx, sy, w, h, dx, dy] in &PLATES[1..] {
            s.image_part_tint(g, &self.hud.shadow, dx, dy, sx, sy, w, h, rgba(0.0, 0.0, 0.0, 0.5));
            s.image_part(g, &self.hud_art, dx, dy, sx, sy, w, h);
        }
        ui::set_face(Some(Face::Manual));
        if !self.econ.sandbox {
            let units = (self.econ.cash / Economy::UNIT) as i64;
            // 0x42dc00: the sign goes after the money sign; the exe adds "*" after cash from the money cheat (not in the port)
            let text = format!("\u{a7}{}{}", if units < 0 { "-" } else { "" }, group(units.unsigned_abs() * 100));
            if units >= 0 {
                shadowed(s, g, 708.0, 35.0, &text, c15(0x23e8));
            } else if self.econ.tick & 4 != 0 {
                shadowed(s, g, 708.0, 35.0, &text, c15(0x7d08));
            }
        }
        let rt = self.club.ratings;
        shadowed(s, g, 722.0, 78.0, &rt.fun.to_string(), c15(0x7ff0));
        shadowed(s, g, 732.0, 121.0, &hundredths(rt.skill), c15(0x03ff));
        ui::set_face(None);
    }

    /// The exe's HUD over the course, before the dock (which covers the face strip while a panel is open).
    pub fn draw_exe_hud(&mut self, g: &mut Gfx, s: &Ui) {
        // a tournament hides the strip and the badge (game flag 0x200000)
        if self.club.game & game::TOURNAMENT == 0 {
            self.draw_face_strip(g, s);
            self.draw_badge(g, s);
        } else {
            self.hud.strip.clear();
        }
        self.draw_pills(g, s);
    }
}
