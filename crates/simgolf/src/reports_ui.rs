//! The report screens behind the F keys (docs/PUBLISHER_EXE_NOTES.md, "Report screens"): F2 Player Comments, F3 Histograph,
//! F4 Financial Report, F5 Routing Map (course routing, employees, course aura, home site value) and F8 Keyboard Shortcuts.
//! Layouts follow the exe's 800 x 600 screens; the art is the disc's.

use crate::app::*;
use crate::gfx::Gfx;
use crate::panels_ui::COUNTERS;
use crate::screens_ui::top;
use crate::ui::{load_pcx_alpha, rgb, rgba, wrap_text, Image, Screen as Ui};
use sg_core::course::{idx, N, TYPES};
use sg_core::economy::LEDGER_LABELS;
use sg_core::land;
use sg_core::staff;

/// The report art.
#[derive(Default)]
pub struct ReportArt {
    pub comments: Image,
    pub histo: Image,
    pub finance: Image,
    pub shortcuts: Image,
    pub route: [Image; 4],
    pub route_bottom: Image,
}

impl ReportArt {
    pub fn load(g: &mut Gfx, app: &App) -> ReportArt {
        let p = |rel: &str| app.game_path(&format!("Interface/infoscreens/{rel}"));
        let a = |g: &mut Gfx, c: &str, al: &str| load_pcx_alpha(g, &p(c), &p(al)).unwrap_or_default();
        let plain = |g: &mut Gfx, c: &str| crate::ui::load_pcx(g, &p(c), true, None).unwrap_or_default();
        ReportArt {
            comments: a(g, "PlayComt.pcx", "PlayComt_alpha.pcx"),
            histo: a(g, "histograph.pcx", "histograph_alpha.pcx"),
            finance: a(g, "FINANCEreport.pcx", "FINANCEreport_alpha.pcx"),
            shortcuts: a(g, "shortcuts.pcx", "shortcuts_alpha.pcx"),
            route: [
                plain(g, "route screens_course.pcx"),
                plain(g, "route screens_employ.pcx"),
                plain(g, "route screens_aura.pcx"),
                plain(g, "route screens_value.pcx"),
            ],
            route_bottom: plain(g, "route screens_bottom.pcx"),
        }
    }
}

/// A sprite queued for the end of the frame: depth, sprite, view, frame, ground point x and y, scale.
pub(crate) type Queued = (f32, usize, i32, i32, f32, f32, f32);

fn black() -> [f32; 4] {
    rgb(0.0, 0.0, 0.0)
}

/// 15-bit exe colour to RGBA.
fn c15(v: u32) -> [f32; 4] {
    rgb(((v >> 10) & 31) as f32 / 31.0, ((v >> 5) & 31) as f32 / 31.0, (v & 31) as f32 / 31.0)
}

/// The histograph's height of a sample (docs/UI_SCREENS2.md): linear to 500, then ten to one; a sample below 501 counts by
/// its absolute value, so a negative fun rating or balance is drawn mirrored above the baseline.
fn squash(v: i32) -> i32 {
    if v < 501 {
        v.abs() / 2
    } else {
        v / 10 + 200
    }
}

const SHORTCUTS: [(&str, &str); 16] = [
    ("F1", "Course status report."),
    ("F2", "Player comments report."),
    ("F3", "Histograph."),
    ("F4", "Financial Report."),
    ("F5", "Course Overview Map."),
    ("F6", "World Map."),
    ("F7", "SGA Evaluation."),
    ("F8", "Handy keyboard commands screen."),
    ("F9", "Membership Roster."),
    ("F10", "Professional Accomplishments."),
    ("?", "Repeat last message."),
    ("shift+b", "Sell a building lot."),
    ("shift+r", "Routing, Aura, and lot values"),
    ("shift+w", "World map, change courses"),
    ("Tab", "Rotate buildings/trees before placing."),
    ("Esc", "Quit the game."),
];
const SHORTCUTS_RIGHT: [(&str, &str); 15] = [
    ("g", "Select GREEN/TEES tool."),
    ("f", "Select FAIRWAY tool."),
    ("r", "Select ROUGH tool."),
    ("s", "Select SAND TRAP tool."),
    ("t", "Select TREES tool."),
    ("w", "Select WATER tool."),
    ("p", "Select PATH tool."),
    ("b", "Select BENCHES tool."),
    ("z", "Zoom the view."),
    ("x", "Unzoom the view."),
    ("shift+p", "Pause/Unpause the game."),
    ("shift+t", "Turn trees off/on."),
    ("shift+n", "Toggle names display."),
    ("e", "Toggle elevation mode."),
    ("/", "Instant shot analysis."),
];

impl App {
    pub fn open_report_screen(&mut self, s: Screen) {
        self.screen = if self.screen == s { Screen::Play } else { s };
        self.hover = -1;
    }

    fn dim(&self, g: &mut Gfx, s: &Ui) {
        s.fill(g, -400.0, -400.0, 1600.0, 1400.0, rgba(0.0, 0.0, 0.0, 0.75));
    }

    // ---- F2 ---------------------------------------------------------------------------------------------------------

    /// The comments report rows: event id, text, tone, the hole it happens on most, frequency in percent of tee shots.
    pub fn comment_rows(&self) -> Vec<(u32, sg_core::thoughts::Line, usize, i32)> {
        let mut total = [0i32; 64];
        let mut best = [1usize; 64];
        let mut plays = 0;
        for h in 1..19 {
            let hr = self.club.holes[h].clone();
            plays += hr.tee_shots;
            for c in 0..64 {
                let v = hr.events.get(c).copied().unwrap_or(0);
                total[c] += v;
                if v > self.club.holes[best[c]].events.get(c).copied().unwrap_or(0) {
                    best[c] = h;
                }
            }
        }
        let mut rows = Vec::new();
        if plays == 0 {
            return rows;
        }
        for _ in 0..20 {
            let (c, t) = (0..50).map(|c| (c, total[c])).fold((0, 0), |a, b| if b.1 > a.1 { b } else { a });
            if t == 0 {
                break;
            }
            total[c] = 0;
            let arg = self.club.holes[best[c]].event_args.get(c).copied().unwrap_or(0) & 0x3fff;
            let line = self.club.thought(&self.course, c as u32, arg, 0x98);
            rows.push((c as u32, line, best[c], t * 100 / plays));
        }
        rows
    }

    pub fn draw_comments(&mut self, g: &mut Gfx) {
        let s = Ui::new(self.draw_w, self.draw_h);
        self.view = s.view;
        self.dim(g, &s);
        let im = &self.reports.comments;
        let has = im.tex.is_some();
        if has {
            s.image_part(g, im, 148.0, 45.0, 148.0, 45.0, 505.0, 102.0);
        } else {
            s.fill(g, 148.0, 45.0, 505.0, 102.0, rgb(0.9, 0.88, 0.8));
        }
        // fonts (0x4546b0): the title in 0x821020 (Klepto 24) centred at (389, 61), the rest in 0x821ee8 (Manual SSi 14): the
        // headings with their tops at 96, "Comments" left at 182, the hole and frequency centred on 504 and 598
        use crate::ui::{F_INFO14, F_INFO_TITLE};
        s.put_centered(g, F_INFO_TITLE, 389.0, 61.0, "PLAYER COMMENTS REPORT", black());
        s.put(g, F_INFO14, 182.0, 96.0, "Comments", black());
        s.put_centered(g, F_INFO14, 504.0, 96.0, "Hole", black());
        s.put_centered(g, F_INFO14, 598.0, 96.0, "Frequency", black());
        let rows = self.comment_rows();
        let y0 = 124.0;
        for (n, (_, line, hole, freq)) in rows.iter().enumerate() {
            let y = y0 + 15.0 * n as f32;
            if has {
                s.image_part(g, im, 148.0, y, 148.0, 224.0, 505.0, 17.0);
            } else {
                s.fill(g, 148.0, y, 505.0, 15.0, rgb(0.92, 0.9, 0.84));
            }
            let c = match line.tone {
                sg_core::thoughts::Tone::Good => c15(0x1284),
                sg_core::thoughts::Tone::Bad => c15(0x7d08),
                _ => black(),
            };
            // PLACEHOLDER: the rows' first top (the exe's header piece height plus 24) is not decoded; 3 below the row piece
            s.put(g, F_INFO14, 182.0, y + 3.0, &line.text, c);
            s.put_centered(g, F_INFO14, 504.0, y + 3.0, &format!("{hole}"), c);
            s.put_centered(g, F_INFO14, 598.0, y + 3.0, &format!("{freq}%"), c);
        }
        if rows.is_empty() {
            s.put_centered(g, F_INFO14, 400.0, y0 + 3.0, "No comments yet.", black());
        }
        let fy = y0 + 15.0 * rows.len().max(1) as f32;
        if has {
            s.image_part(g, im, 148.0, fy, 148.0, 321.0, 505.0, 61.0);
            // the art's own lit tick over its baked one
            if self.over_ok(593.0, fy + 14.0) {
                s.image_part(g, im, 593.0, fy + 14.0, 593.0, 434.0, 44.0, 44.0);
            }
        }
        g.flush();
    }

    // ---- F3 ---------------------------------------------------------------------------------------------------------

    pub fn draw_histograph(&mut self, g: &mut Gfx) {
        let s = Ui::new(self.draw_w, self.draw_h);
        self.view = s.view;
        self.dim(g, &s);
        if self.reports.histo.tex.is_some() {
            s.image(g, &self.reports.histo, 0.0, 0.0);
        } else {
            s.fill(g, 100.0, 50.0, 620.0, 490.0, rgb(0.95, 0.95, 0.9));
        }
        // fonts (0x455ed0): the title in 0x821020 (Klepto 24) centred at (375, 20); the legend and the scales in 0x821ee8
        // (Manual SSi 14), the legend centred with its top at 542
        use crate::ui::{F_INFO14, F_INFO_TITLE};
        s.put_centered(g, F_INFO_TITLE, 375.0, 20.0, "HISTOGRAPH", black());
        for (x, t) in [(177.0, "Skill"), (332.0, "Cash"), (488.0, "Fun"), (644.0, "Event")] {
            s.put_centered(g, F_INFO14, x, 542.0, t, black());
        }
        let skill_s = if self.club.ratings.skill > 2500 { 2 } else { 1 };
        let cash = self.club.cash;
        let cdiv = match cash {
            c if c > 100000 => 80,
            c if c > 50000 => 40,
            c if c > 25000 => 20,
            c if c > 10000 => 10,
            c if c > 5000 => 4,
            c if c > 2500 => 2,
            _ => 1,
        };
        for k in 0..10 {
            let y = 0x202 as f32 - 50.0 * k as f32;
            let lv = if k <= 5 { 100 * skill_s * k } else { skill_s * (500 * k - 2000) };
            s.put_centered(g, F_INFO14, 77.0, y, &format!("{lv}"), c15(0x4010));
            let n = if k <= 5 { 10 * cdiv * k } else { cdiv * (50 * k - 200) };
            s.put_centered(g, F_INFO14, 727.0, y, &format!("\u{a7}{n}k"), black());
        }
        let months = (self.club.tick >> 10) as usize;
        let step = (600 / (months as i32 + 1)).clamp(1, 4) as f32;
        let h = &self.club.history;
        let base = 0x209 as f32;
        let mut prev = [base; 4];
        let mut offset = 0;
        for i in 1..=months.min(499) {
            let x0 = 0x6b as f32 + (i - 1) as f32 * step;
            let x1 = x0 + step;
            let r = h.get(i).copied().unwrap_or([0; 4]);
            let ys = [
                base - squash(r[2] / skill_s) as f32,
                base - squash(r[0] / cdiv) as f32,
                base - squash(r[1]) as f32,
                base - 4.0 * r[3] as f32,
            ];
            let cols = [c15(0x4010), if r[0] < 0 { c15(0x7d08) } else { black() }, c15(0x03e0), c15(0x0210)];
            for k in 0..4 {
                // the plot's clip rectangle (106, 72, 598, 450)
                let (a, b) = (prev[k].clamp(72.0, 522.0), ys[k].clamp(72.0, 522.0));
                s.fill(g, x0, a.min(b), (x1 - x0).max(1.0), (a - b).abs().max(1.0), cols[k]);
                prev[k] = ys[k];
            }
            let v = self.club.event_log.get(i).copied().unwrap_or(0);
            if v != 0 {
                let t = self.club.event_text(v);
                if !t.is_empty() {
                    offset = offset % 450 + 10;
                    let top = base - offset as f32;
                    s.fill(g, x1 + 10.0, ys[0].min(top), 1.0, (ys[0] - top).abs(), c15(0x03ff));
                    s.fill(g, x1 + 9.0, top - 1.0, 3.0, 3.0, c15(0x0210));
                    // the events in Arial Bold 10 (0x519fd8); PLACEHOLDER: their offset from the mark
                    s.put(g, crate::ui::F_ARIAL10, x1 + 14.0, top - 5.0, &t, c15(0x0210));
                }
            }
        }
        self.ok_tick(g, &s, 704.0, 551.0, false);
        g.flush();
    }

    // ---- F4 ---------------------------------------------------------------------------------------------------------

    pub fn draw_finance(&mut self, g: &mut Gfx) {
        let s = Ui::new(self.draw_w, self.draw_h);
        self.view = s.view;
        self.dim(g, &s);
        if self.reports.finance.tex.is_some() {
            s.image_part(g, &self.reports.finance, 8.0, 7.0, 8.0, 7.0, 784.0, 289.0);
        } else {
            s.fill(g, 8.0, 7.0, 784.0, 289.0, rgb(0.9, 0.88, 0.8));
        }
        // fonts (0x44f6b0): the title in 0x821020 (Klepto 24) left at (218, 21), the rest in 0x821ee8 (Manual SSi 14): the
        // labels centred on 98 with tops 87 + 17 i (7 more for the total), the years centred on 222 + 75 j at 60, the
        // figures right aligned on 256 + 75 j with tops 86 + 17 i and the total 24 below the last
        use crate::ui::{F_INFO14, F_INFO_TITLE};
        s.put(g, F_INFO_TITLE, 218.0, 21.0, "FINANCIAL REPORT", black());
        for (i, l) in LEDGER_LABELS.iter().enumerate() {
            s.put_centered(g, F_INFO14, 98.0, 87.0 + 17.0 * i as f32, l, black());
        }
        s.put_centered(g, F_INFO14, 98.0, 87.0 + 136.0 + 7.0, "Total (\u{a7})", black());
        let last = self.econ.year_index().min(99);
        // eight columns fit the art (the exe draws a ninth off the screen)
        let first = last.saturating_sub(7);
        for (j, y) in (first..=last).enumerate() {
            let x = 75.0 * j as f32;
            s.put_centered(g, F_INFO14, x + 222.0, 60.0, &format!("{}", 2001 + y), black());
            let row = self.econ.ledger.get(y).copied().unwrap_or_default();
            let mut total = 0.0;
            for (i, v) in row.iter().enumerate() {
                total += v;
                if *v != 0.0 {
                    let t = crate::screens_ui::digits(*v as i64);
                    let c = if *v < 0.0 { c15(0x6000) } else { black() };
                    s.put_right(g, F_INFO14, x + 256.0, 86.0 + 17.0 * i as f32, &t, c);
                }
            }
            let t = crate::screens_ui::digits(total as i64);
            let c = if total < 0.0 { c15(0x6000) } else { black() };
            s.put_right(g, F_INFO14, x + 256.0, 86.0 + 17.0 * 7.0 + 24.0, &t, c);
        }
        self.ok_tick(g, &s, 726.0, 249.0, false);
        g.flush();
    }

    // ---- F5 ---------------------------------------------------------------------------------------------------------

    /// A tile's colour on the routing map (0x456be0); out of bounds tiles are not drawn. Aura: red from the unhappy map, green
    /// from the happy one, blue from their byte sum over 16, water blue. Home site value: a quarter of the lot value (0x42ef40)
    /// less the site work of a home site there (0x40db90, side 2), times 1.5, as green; dark red where none would fit. The
    /// other tabs colour by the tile's class.
    fn tile_colour(&self, a: i32, b: i32, aura: Option<&(Vec<u8>, Vec<u8>)>) -> Option<[f32; 4]> {
        let i = idx(a, b);
        let t = self.course.ty[i];
        if t == sg_core::course::t::OUT {
            return None;
        }
        match self.route_tab {
            2 => {
                if t == sg_core::course::t::WATER {
                    return Some(c15(0x0218));
                }
                let (happy, unhappy) = aura.map(|m| (m.0[i], m.1[i])).unwrap_or((0, 0));
                let blue = happy.wrapping_add(unhappy) >> 4;
                Some(c15(((unhappy as u32 >> 3) << 10) | ((happy as u32 >> 3) << 5) | blue as u32))
            }
            3 => {
                let v = sg_core::homes::lot_value(&self.course, &self.club.holes, self.difficulty, a, b);
                let site = self.land.as_ref().map(|l| l.fits(a, b, 2, land::K_HOME_SITE, self.exe_theme()));
                match site {
                    Some(None) => Some(c15(0x2000)),
                    s => {
                        let q = sg_core::geom::clamp((v / 4 - s.flatten().unwrap_or(0)) * 3 / 2, 0, 255);
                        Some(c15(((q as u32) >> 3) << 5))
                    }
                }
            }
            _ => Some(c15(match TYPES[(t as usize).min(22)].class {
                0 | 1 => 0x3394,
                2 => 0x1310,
                4 => 0x0204,
                7 => 0x6310,
                10 => 0x5304,
                13 => 0x1184,
                17 => 0x0218,
                18 => 0x10c8,
                _ => 0x318c,
            })),
        }
    }

    fn mini(a: f32, b: f32) -> (f32, f32) {
        ((a + b) * 6.0 + 106.0, (b - a) * 3.0 + 441.0)
    }

    /// An employee as the exe draws it off the course (the panel's portraits and the routing map's list): the clip of its
    /// state (walk below 11, stand at 11, action at 12), its frame modulo the clip's length and the view (camera - facing - 2)
    /// & 7, as (sprite, view, frame).
    pub(crate) fn staff_figure(&self, e: &staff::Employee) -> Option<(usize, i32, i32)> {
        let (set, state) = self.staff_clip_of(e);
        let si = self.staff_clips[set][state].0?;
        let n = self.sprites[si].s.frames_per_view.max(1);
        let camera = 2 * (((self.rot / 90.0).round() as i32 % 4 + 4) % 4);
        Some((si, (camera - e.dir as i32 - 2) & 7, e.frame as i32 % n))
    }

    /// Draws the sprites queued this frame (0x4628d0) in depth order, each with its ground point at (x, y). The scale is the
    /// queue's zoom times the global zoom over 16, the global zoom taken as 4 (its value in play, derived), so zoom 4 draws at
    /// full size and 2 at half.
    pub(crate) fn draw_queued(&mut self, g: &mut Gfx, s: &Ui, queue: &mut Vec<Queued>) {
        queue.sort_by(|a, b| a.0.total_cmp(&b.0));
        for &(_, si, view, f, x, y, k) in queue.iter() {
            let (w, h, ax, ay) = {
                let sp = &self.sprites[si].s;
                let fr = &sp.frames[sp.frame_index(view, f)];
                (fr.w as f32, fr.h as f32, sp.anchor_x as f32, sp.anchor_y as f32)
            };
            let tex = self.sprite_texture(g, si, view, f);
            s.image(g, &Image { tex: Some(tex), w: w * k, h: h * k }, x - ax * k, y - ay * k);
        }
        queue.clear();
    }

    /// Opens or closes the routing map; it opens on hole 1 with the employee list at its top, in the tab used last.
    pub fn open_routing(&mut self) {
        if self.screen != Screen::Routing {
            self.route_hole = 1;
            self.route_scroll = 0;
        }
        self.open_report_screen(Screen::Routing);
    }

    /// The employees listed on the routing map, in record order (the player's own pro is left out).
    fn routing_staff(&self) -> Vec<usize> {
        (0..self.employees.len())
            .filter(|&i| {
                let e = &self.employees[i];
                e.active && e.job < 0 && e.job != staff::job::OWNER
            })
            .collect()
    }

    /// What is under the pointer on the routing map (0x456be0's hover test, in its order): the tab buttons 0 routing,
    /// 1 employees, 2 aura, 3 home site value; -2 the OK tick; 9 and 10 the employee list's arrows (employees tab with more
    /// than eight employees); -1 nothing.
    fn routing_spot(&self, vx: f32, vy: f32) -> i32 {
        let (x, y) = (vx.floor() as i32, vy.floor() as i32);
        let within = |x0: i32, x1: i32, y0: i32, y1: i32| (x0..=x1).contains(&x) && (y0..=y1).contains(&y);
        if within(62, 141, 315, 394) {
            1
        } else if within(183, 262, 254, 333) {
            0
        } else if within(536, 615, 254, 333) {
            2
        } else if within(659, 738, 315, 394) {
            3
        } else if within(662, 741, 532, 596) {
            -2
        } else if self.route_tab == 1 && self.routing_staff().len() > 8 && within(775, 792, 64, 100) {
            9
        } else if self.route_tab == 1 && self.routing_staff().len() > 8 && within(775, 792, 205, 241) {
            10
        } else {
            -1
        }
    }

    /// A click on the routing map, as the exe handles it: below y 280 the tab buttons switch the tab and the tick closes the
    /// map (elsewhere nothing happens); the list arrows scroll the employees; a click between y 257 and 280 closes the map;
    /// above that, in every tab, the row under the pointer (left list holes 1 to 9, right list 10 to 18, by the row the y
    /// falls in, clamped) is selected by a left click, and a right click moves the selected hole there: the holes between
    /// shift by one (the help line says "swap"), and the selection stays on the same number.
    pub fn routing_click(&mut self, vx: f32, vy: f32, right: bool) {
        let spot = self.routing_spot(vx, vy);
        let (x, y) = (vx.floor() as i32, vy.floor() as i32);
        if y > 280 {
            if (0..=3).contains(&spot) {
                self.route_tab = spot as usize;
            } else if spot == -2 {
                self.close_info();
            }
            return;
        }
        if spot == 9 {
            self.route_scroll = self.route_scroll.saturating_sub(1);
        } else if spot == 10 && self.routing_staff().len() > self.route_scroll + 8 {
            self.route_scroll += 1;
        }
        if y > 256 {
            return self.close_info();
        }
        // C division truncates toward zero, so the rows above the list all clamp to the first
        let row = ((y - 0x68) / 17 + 1).clamp(1, 19) + if x > 399 { 9 } else { 0 };
        let h = row.clamp(1, 18) as usize;
        if right {
            let from = self.route_hole;
            self.move_hole(from, h);
        } else {
            self.route_hole = h;
        }
    }

    pub fn draw_routing(&mut self, g: &mut Gfx) {
        let s = Ui::new(self.draw_w, self.draw_h);
        self.view = s.view;
        crate::info_ui::dim(g, &s);
        let tab = self.route_tab.min(3);
        if self.reports.route[tab].tex.is_some() {
            s.image(g, &self.reports.route[tab], 0.0, 0.0);
        }
        let bottom = self.reports.route_bottom;
        let bottom = &bottom;
        if bottom.tex.is_some() {
            s.image_part(g, bottom, 0.0, 253.0, 0.0, 253.0, 800.0, 347.0);
        }
        // the minimap: one 12 x 6 diamond per tile, the polygon (x - 6, y), (x, y - 3), (x + 6, y), (x, y + 3) (0x475df0)
        let aura = (tab == 2).then(|| self.course.aura());
        for a in 0..N {
            for b in 0..N {
                let Some(c) = self.tile_colour(a, b, aura.as_ref()) else { continue };
                let (x, y) = Self::mini(a as f32, b as f32);
                s.diamond(g, x, y, 6.0, 3.0, c);
            }
        }
        let ink = c15(0);
        // the course name in the 16 point face (0x821f28), 10 pixels higher with " Employees" on the employees tab
        let name = if tab == 1 { format!("{} Employees", self.course_name) } else { self.course_name.clone() };
        s.text_centered(g, 400.0, top(if tab == 1 { 48.0 } else { 58.0 }, 16.0), &name, 16.0, ink);
        s.text_centered(g, 400.0, top(15.0, 24.0), "ROUTING MAP", 24.0, ink);
        let row = |y: f32| top(y, 14.0);
        match tab {
            0 => {
                s.text_centered(g, 400.0, row(85.0), "COURSE ROUTING", 14.0, ink);
                s.text_centered(g, 400.0, row(108.0), "Left click to select hole.", 14.0, ink);
                s.text_centered(g, 400.0, row(126.0), "Right click to swap holes.", 14.0, ink);
                for (x, t) in [(71.0, "Hole #"), (132.0, "PAR"), (182.0, "YDS"), (239.0, "Time")] {
                    s.text_centered(g, x, row(85.0), t, 14.0, ink);
                }
                for (x, t) in [(564.0, "Hole #"), (630.0, "PAR"), (680.0, "YDS"), (737.0, "Time")] {
                    s.text_centered(g, x, row(85.0), t, 14.0, ink);
                }
            }
            2 => {
                s.text_centered(g, 400.0, row(188.0), "COURSE AURA", 14.0, ink);
                s.text_centered(g, 400.0, row(85.0), "AURA", 14.0, ink);
                s.text(g, 624.0, row(206.0), "Happy", 14.0, ink);
                s.text(g, 136.0, row(206.0), "Unhappy", 14.0, ink);
                // the paragraph wrapped to 300 pixels from (250, 110) (0x478530); the line pitch is a PLACEHOLDER
                let text = "Course AURA indicates where on your course players have made mostly happy comments and where they \
                            have made unhappy comments.";
                for (i, l) in wrap_text(text, 14.0, 300.0).iter().enumerate() {
                    s.text(g, 250.0, row(110.0 + 16.0 * i as f32), l, 14.0, ink);
                }
            }
            3 => {
                s.text_centered(g, 400.0, row(188.0), "HOME SITE VALUE", 14.0, ink);
                s.text(g, 631.0, row(206.0), "High", 14.0, ink);
                s.text(g, 142.0, row(206.0), "Low", 14.0, ink);
                let inc = [
                    "Things which INCREASE home value:",
                    "Close to water and trees.",
                    "Close to a fun golf hole.",
                    "Close to a top 100 or top 18 hole.",
                    "Building a Marina",
                ];
                let dec = [
                    "Things which DECREASE home value:",
                    "Close to an unfun hole.",
                    "Close to another building.",
                    "Too close to green, fairway, or OB.",
                    "Far away from the golf course.",
                ];
                for (i, (a, b)) in inc.iter().zip(dec.iter()).enumerate() {
                    let y = [86.0, 104.0, 121.0, 138.0, 156.0][i];
                    s.text(g, 51.0, row(y), a, 14.0, ink);
                    s.text(g, 494.0, row(y), b, 14.0, ink);
                }
            }
            _ => {}
        }
        // the holes: rows of the routing list (routing tab), the tee to green line and the number on the map (every tab)
        let mut queue = Vec::new();
        for h in 1..19 {
            let hr = self.club.holes[h].clone();
            if tab == 0 {
                let shift = if h <= 9 { 0.0 } else { 498.0 };
                let y = 104.0 + 17.0 * ((h - 1) % 9) as f32;
                if h == self.route_hole {
                    // a green frame one pixel wide round a white row
                    s.fill(g, shift + 32.0, y - 3.0, 238.0, 17.0, c15(0x23e8));
                    s.fill(g, shift + 33.0, y - 2.0, 236.0, 15.0, c15(0x7fff));
                }
                let x = 71.0 + shift;
                let c = if hr.par == 0 { c15(0x6318) } else { ink };
                s.text_centered(g, x, row(y), &format!("{h}"), 14.0, c);
                if hr.par != 0 {
                    s.text_centered(g, x + 61.0, row(y), &format!("{}", hr.par), 14.0, ink);
                    s.text_centered(g, x + 111.0, row(y), &format!("{}", hr.length), 14.0, ink);
                    if hr.tee_shots > 0 {
                        s.text_centered(g, x + 168.0, row(y), &format!("{}m", hr.time / hr.tee_shots / 40), 14.0, ink);
                    }
                }
            }
            if hr.par == 0 {
                continue;
            }
            // the tee marker of the hole's par (its last pop frame) on the back tee; a white line two wide from there through
            // the 250 yard marker (else the 200 one) to the pin, where the theme's flag stands at half size; the hole number in
            // yellow, 4 pixels above the marker or, with neither marker, halfway along (not on the employees tab)
            let (tx, ty) = Self::mini(hr.back.0 as f32, hr.back.1 as f32);
            let tee = 0x18d + (hr.par - 3).clamp(0, 2) as u16;
            if let (Some(si), _) = self.decor_sprite(tee, 0x5e) {
                let f = self.sprites[si].s.frames_per_view - 1;
                queue.push((ty, si, 0, f, tx, ty, 1.0));
            }
            let (gx, gy) = Self::mini(hr.pin.0 as f32, hr.pin.1 as f32);
            let mark = [hr.markers[2], hr.markers[1]].into_iter().find(|m| m.0 != -1);
            let label = match mark {
                Some((mx, my)) => {
                    let (x, y) = Self::mini((mx >> 10) as f32, (my >> 10) as f32);
                    s.line(g, tx, ty, x, y, 2.0, c15(0x7fff));
                    s.line(g, x, y, gx, gy, 2.0, c15(0x7fff));
                    (x, y)
                }
                None => {
                    s.line(g, tx, ty, gx, gy, 2.0, c15(0x7fff));
                    (((tx + gx) / 2.0).trunc(), ((ty + gy) / 2.0).trunc())
                }
            };
            if tab != 1 {
                // centred, by its top, with the palette's black a pixel below (0x404bc0)
                let (x, y) = (label.0, top(label.1 - 4.0, 14.0));
                s.text_centered(g, x, y + 1.0, &format!("{h}"), 14.0, ink);
                s.text_centered(g, x, y, &format!("{h}"), 14.0, c15(0x7ff0));
            }
            let flag = 0x189 + self.exe_theme().min(3) as u16;
            // the exe's palette 0x60 + theme; the port's 0x63 entry is that same per-theme flag palette
            if let (Some(si), _) = self.decor_sprite(flag, 0x63) {
                queue.push((gy, si, 3, 0, gx, gy, 0.5));
            }
        }
        if tab == 1 {
            self.draw_routing_staff(g, &s, &mut queue);
        }
        self.draw_queued(g, &s, &mut queue);
        // the tab under the pointer lights up (its pale cut), the open tab is yellow; the compass sits in the round well at
        // the lower left (buy_land_buttons cut 20); the tick lights up under the pointer
        let spot = self.routing_spot(self.info.pointer.0, self.info.pointer.1);
        if bottom.tex.is_some() {
            const HOVER: [(f32, f32, f32, f32, f32, f32); 4] = [
                (261.0, 0.0, 134.0, 117.0, 182.0, 254.0),
                (0.0, 0.0, 129.0, 113.0, 60.0, 313.0),
                (396.0, 0.0, 139.0, 117.0, 483.0, 254.0),
                (130.0, 0.0, 130.0, 113.0, 612.0, 313.0),
            ];
            const OPEN: [(f32, f32, f32, f32, f32, f32); 4] = [
                (261.0, 118.0, 134.0, 117.0, 182.0, 254.0),
                (0.0, 114.0, 129.0, 113.0, 60.0, 313.0),
                (396.0, 118.0, 139.0, 117.0, 482.0, 254.0),
                (130.0, 114.0, 130.0, 113.0, 612.0, 313.0),
            ];
            if (0..=3).contains(&spot) {
                let (sx, sy, w, h, dx, dy) = HOVER[spot as usize];
                s.image_part(g, bottom, dx, dy, sx, sy, w, h);
            } else if spot == -2 {
                s.image_part(g, bottom, 662.0, 532.0, 536.0, 0.0, 65.0, 65.0);
            }
            if tab == 1 && spot == 9 {
                s.image_part(g, bottom, 775.0, 64.0, 713.0, 1.0, 18.0, 37.0);
            } else if tab == 1 && spot == 10 {
                s.image_part(g, bottom, 775.0, 205.0, 732.0, 1.0, 18.0, 37.0);
            }
            let (sx, sy, w, h, dx, dy) = OPEN[tab];
            s.image_part(g, bottom, dx, dy, sx, sy, w, h);
        }
        let lb = &self.info.art.land_buttons;
        if lb.tex.is_some() {
            s.image_part(g, lb, 71.0, 533.0, 259.0, 127.0, 72.0, 61.0);
        }
        g.flush();
    }

    /// The employees tab (mode 0 of 0x456be0): eight employees at a time in two columns of four, each with its figure, its
    /// number and name, the month hired and the wages paid, and its work counter; the scroll track with more than eight; on
    /// the map every employee's work area (an ellipse 24 pixels across the diagonal, 48 for groundskeepers, half again when
    /// experienced) in the job's colour with the number and the name.
    fn draw_routing_staff(&mut self, g: &mut Gfx, s: &Ui, queue: &mut Vec<Queued>) {
        let list = self.routing_staff();
        let ink = c15(0);
        let n = list.len();
        let off = self.route_scroll.min(n.saturating_sub(1));
        if n > 8 {
            let bottom = &self.reports.route_bottom;
            if bottom.tex.is_some() {
                s.image_part(g, bottom, 775.0, 64.0, 751.0, 0.0, 18.0, 178.0);
            }
            let ty = (off * 95 / n) as f32;
            let th = (95.0 - ty).min((760 / n) as f32);
            s.fill(g, 781.0, 106.0 + ty, 6.0, th, c15(0x7fff));
        }
        let row = |y: f32| top(y, 14.0);
        for (k, &i) in list.iter().enumerate().skip(off).take(8) {
            let e = self.employees[i];
            let vis = k - off + 1;
            let x0 = if vis > 4 { 0.0 } else { -374.0 };
            let y0 = 45.0 * ((vis - 1) & 3) as f32;
            let kind = (-2 - e.job as i32).clamp(0, 3) as usize;
            // the figure: the employee's current clip, frame and facing, queued at full size (depth 32 above its feet)
            if let Some((si, view, f)) = self.staff_figure(&e) {
                queue.push((y0 + 71.0, si, view, f, x0 + 746.0, y0 + 103.0, 1.0));
            }
            s.text(g, x0 + 415.0, row(y0 + 71.0), &format!("{}. {}", k + 1, self.employee_name(&e)), 14.0, ink);
            let month = ["March", "April", "May", "June", "July", "August", "September", "October"][(e.hired & 7) as usize];
            let paid = crate::screens_ui::digits(e.paid as i64 * 100);
            let hired = format!("Hired: {month} {}, paid: \u{a7}{paid}", 2001 + (e.hired >> 3));
            s.text(g, x0 + 430.0, row(y0 + 86.0), &hired, 14.0, ink);
            let counter = format!("{} {}", COUNTERS[kind][e.upgraded as usize], e.served);
            s.text(g, x0 + 430.0, row(y0 + 101.0), &counter, 14.0, ink);
        }
        let mut number = 1;
        for e in self.employees.iter().filter(|e| e.active && e.job < 0) {
            let Some((pa, pb)) = e.post else { continue };
            let (cx, cy) = Self::mini(pa as f32, pb as f32);
            let (col, mut r) = match e.job {
                staff::job::SODA_VENDOR => (c15(0x03ff), 24.0),
                staff::job::GROUNDSKEEPER => (c15(0x7ff0), 48.0),
                staff::job::RANGER => (c15(0x0018), 24.0),
                staff::job::CLUB_PRO => (c15(0x6318), 24.0),
                _ => (c15(0x7fff), 24.0),
            };
            if e.upgraded {
                r += (r / 2.0f32).floor();
            }
            // 24 segments round the circle, r across and r / 2 down
            let pt = |k: i32| {
                let a = k as f32 / 24.0 * std::f32::consts::TAU;
                (cx + r * a.sin(), cy + (r / 2.0).floor() * a.cos())
            };
            for k in 0..24 {
                let (a, b) = (pt(k), pt(k + 1));
                s.line(g, a.0, a.1, b.0, b.1, 2.0, col);
            }
            let name = if e.job == staff::job::OWNER {
                self.pro_name()
            } else {
                s.text_centered(g, cx, top(cy - 10.0, 14.0), &format!("{number}. "), 14.0, col);
                number += 1;
                self.employee_name(e)
            };
            // the name in the small face (0x519fd8, Arial Bold 10)
            crate::ui::set_face(Some(crate::ui::Face::Arial));
            s.text_centered(g, cx, top(cy, 10.0), &name, 10.0, col);
            crate::ui::set_face(Some(crate::ui::Face::Info));
        }
    }

    // ---- F8 ---------------------------------------------------------------------------------------------------------

    pub fn draw_shortcuts(&mut self, g: &mut Gfx) {
        let s = Ui::new(self.draw_w, self.draw_h);
        self.view = s.view;
        self.dim(g, &s);
        if self.reports.shortcuts.tex.is_some() {
            s.image_part(g, &self.reports.shortcuts, 0.0, 0.0, 0.0, 0.0, 800.0, 409.0);
        } else {
            s.fill(g, 0.0, 0.0, 800.0, 409.0, rgb(0.9, 0.88, 0.8));
        }
        // fonts (0x44e770): the title in 0x821020 (Klepto 24) centred at (287, 28); the keys centred and the descriptions
        // left, in 0x821ee8 (Manual SSi 14), tops as listed
        use crate::ui::{F_INFO14, F_INFO_TITLE};
        s.put_centered(g, F_INFO_TITLE, 287.0, 28.0, "KEYBOARD SHORTCUTS", black());
        let ys = [0x51, 0x62, 0x73, 0x84, 0x95, 0xa6, 0xb7, 200, 0xd9, 0xea, 0xfb, 0x10c, 0x11d, 0x12e, 0x13f, 0x157];
        for (i, (k, d)) in SHORTCUTS.iter().enumerate() {
            let y = ys[i] as f32;
            let c = if i == 7 { c15(0x0200) } else { black() };
            s.put_centered(g, F_INFO14, 0x44 as f32, y, k, c);
            s.put(g, F_INFO14, 0x6c as f32, y, d, c);
        }
        for (i, (k, d)) in SHORTCUTS_RIGHT.iter().enumerate() {
            let y = ys[i] as f32;
            s.put_centered(g, F_INFO14, 0x1bc as f32, y, k, black());
            s.put(g, F_INFO14, 0x1e3 as f32, y, d, black());
        }
        self.ok_tick(g, &s, 710.0, 362.0, false);
        g.flush();
    }
}

impl App {
    /// Routing map, right click (0x456be0): the selected hole moves to the clicked place in the order; the holes between
    /// shift by one, the tee and green tiles are renumbered and golfers on the course follow their hole. The selection keeps
    /// its number, so it now shows the hole that moved into the old place.
    pub fn move_hole(&mut self, from: usize, to: usize) {
        if !(1..=18).contains(&from) || !(1..=18).contains(&to) || from == to {
            return;
        }
        // order[k] = the old hole that becomes hole k
        let mut order: Vec<usize> = (1..=18).collect();
        let h = order.remove(from - 1);
        order.insert(to - 1, h);
        let mut new_of = [0usize; 19];
        for (k, &old) in order.iter().enumerate() {
            new_of[old] = k + 1;
        }
        let old = self.club.holes.clone();
        for (k, &o) in order.iter().enumerate() {
            self.club.holes[k + 1] = old[o].clone();
        }
        if let Some(l) = self.land.as_mut() {
            for i in 0..l.flags.len() {
                let t = l.ty[i];
                let n = (l.flags[i] & 0x1f) as usize;
                if (t == land::T_TEE || t == land::T_GREEN) && (1..=18).contains(&n) {
                    l.flags[i] = (l.flags[i] & !0x1f) | new_of[n] as u16;
                }
            }
        }
        for g in self.club.g.iter_mut().take(sg_core::golfer::SLOTS) {
            if (1..=18).contains(&g.hole) {
                g.hole = new_of[g.hole as usize] as i32;
            }
        }
        self.club.next_hole = (1..=18).find(|&k| self.club.holes[k].par == 0).unwrap_or(19) as i32;
        if let Some(l) = &self.land {
            l.write_area(&mut self.terrain, 0, 0, land::N - 1, land::N - 1);
        }
        self.sync_course();
        self.dirty = true;
        println!("hole {from} moved to {to}");
    }

    /// F6, the world map: the property chooser, to move the club to another property. Cash, the calendar and the pro's
    /// career go along; the course starts afresh (0x407d30).
    pub fn open_world_map(&mut self) {
        if self.club.game & sg_core::golfer::game::REPEAT != 0 {
            return self.ui_sound(0x18);
        }
        self.world_move = true;
        self.deal_offer(self.econ.sandbox);
        self.screen = Screen::Property;
        self.hover = -1;
    }

    pub fn move_to_property(&mut self, g: &mut Gfx, prop: usize) {
        let price = self.offer_for(prop).1 as f64;
        if !self.econ.sandbox && self.econ.cash < price {
            return self.show_toast("You need more money before you can purchase this property.");
        }
        let cash = self.econ.cash - if self.econ.sandbox { 0.0 } else { price };
        let tick = self.game_tick;
        let ledger = self.econ.ledger.clone();
        let c = self.club.clone();
        let bought = std::mem::take(&mut self.bought);
        self.world_move = false;
        self.start_game(g, prop, self.econ.sandbox);
        self.econ.cash = cash;
        self.econ.tick = tick;
        // the properties bought before stay bought; the new one is bought this year
        self.bought = bought;
        self.mark_bought(prop);
        self.econ.ledger = ledger;
        let y = self.econ.year_index();
        if let Some(row) = self.econ.ledger.get_mut(y) {
            row[sg_core::economy::LEDGER_OTHER] = cash;
        }
        self.game_tick = tick;
        let k = &mut self.club;
        k.tick = tick;
        k.pro_skill = c.pro_skill;
        k.pro_mask = c.pro_mask;
        k.skill_points = c.skill_points;
        k.awards = c.awards;
        k.earned = c.earned;
        k.trophies = c.trophies;
        k.wager_level = c.wager_level;
        k.event_log = c.event_log;
        k.history = c.history;
        k.roster[0] = c.roster[0].clone();
        println!("moved to {}, cash {:.0}", self.course_name, self.econ.cash);
    }
}

#[cfg(test)]
mod tests {
    use super::squash;

    #[test]
    fn histograph_heights_mirror_negative_samples() {
        assert_eq!(squash(300), 150);
        assert_eq!(squash(-300), 150);
        assert_eq!(squash(500), 250);
        assert_eq!(squash(501), 250);
        assert_eq!(squash(2500), 450);
        assert_eq!(squash(-2000), 1000);
    }
}
