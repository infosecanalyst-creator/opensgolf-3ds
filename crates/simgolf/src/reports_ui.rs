//! The report screens behind the F keys (docs/PUBLISHER_EXE_NOTES.md, "Report screens"): F2 Player Comments, F3 Histograph,
//! F4 Financial Report, F5 Routing Map (course routing, employees, course aura, home site value) and F8 Keyboard Shortcuts.
//! Layouts follow the exe's 800 x 600 screens; the art is the disc's.

use crate::app::*;
use crate::gfx::Gfx;
use crate::render::Rect;
use crate::ui::{load_pcx_alpha, money, rgb, rgba, text_width, wrap_text, Image, Screen as Ui};
use sg_core::course::{idx, inside, N, TYPES};
use sg_core::economy::LEDGER_LABELS;
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

fn black() -> [f32; 4] {
    rgb(0.0, 0.0, 0.0)
}

/// 15-bit exe colour to RGBA.
fn c15(v: u32) -> [f32; 4] {
    rgb(((v >> 10) & 31) as f32 / 31.0, ((v >> 5) & 31) as f32 / 31.0, (v & 31) as f32 / 31.0)
}

fn squash(v: i32) -> i32 {
    if v <= 500 {
        v / 2
    } else {
        v / 10 + 200
    }
}

const SHORTCUTS: [(&str, &str); 16] = [
    ("F1", "Course status report"),
    ("F2", "Player comments report"),
    ("F3", "Histograph"),
    ("F4", "Financial Report"),
    ("F5", "Course Overview Map"),
    ("F6", "Take a screenshot"),
    ("F7", "SGA Evaluation"),
    ("F8", "Handy keyboard commands screen"),
    ("F9", "Membership Roster"),
    ("F10", "Professional Accomplishments"),
    ("Shift+S / L", "Save / load the game"),
    ("Shift+P", "Pause/Unpause the game"),
    ("Shift+T", "Turn trees off/on"),
    ("Shift+F7 / F8", "Save course / pro for a championship"),
    ("J / Shift+J", "Practice round / match"),
    ("Esc", "Quit to the title menu"),
];
const SHORTCUTS_RIGHT: [(&str, &str); 14] = [
    ("G", "Select GREEN/TEES tool"),
    ("F", "Select FAIRWAY tool"),
    ("R", "Select ROUGH tool"),
    ("S", "Select SAND TRAP tool"),
    ("W", "Select WATER tool"),
    ("P", "Select PATH tool"),
    ("= / -", "Raise / lower the ground"),
    ("Z", "Zoom the view"),
    ("X", "Unzoom the view"),
    ("Q / E", "Rotate the view"),
    ("Tab", "Toggle edit mode"),
    ("H", "Open the hole being built"),
    ("Shift+H", "Hide the advisor"),
    ("N twice", "Cancel the pro's round"),
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
            let hr = &self.club.holes[h];
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
        s.text_centered(g, 389.0, 72.0, "PLAYER COMMENTS REPORT", 20.0, black());
        s.text(g, 182.0, 104.0, "Comments", 12.0, black());
        s.text_centered(g, 504.0, 104.0, "Hole", 12.0, black());
        s.text_centered(g, 598.0, 104.0, "Frequency", 12.0, black());
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
            s.text(g, 182.0, y + 13.0, &format!("\"{}\"", line.text), 11.0, c);
            s.text_centered(g, 504.0, y + 13.0, &format!("{hole}"), 11.0, c);
            s.text_centered(g, 598.0, y + 13.0, &format!("{freq}%"), 11.0, c);
        }
        if rows.is_empty() {
            s.text_centered(g, 400.0, y0 + 13.0, "No comments yet.", 12.0, black());
        }
        let fy = y0 + 15.0 * rows.len().max(1) as f32;
        if has {
            s.image_part(g, im, 148.0, fy, 148.0, 321.0, 505.0, 61.0);
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
        s.text_centered(g, 375.0, 34.0, "HISTOGRAPH", 22.0, black());
        for (x, t) in [(177.0, "Skill"), (332.0, "Cash"), (488.0, "Fun"), (644.0, "Event")] {
            s.text_centered(g, x, 551.0, t, 12.0, black());
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
            let y = 0x202 as f32 - 50.0 * k as f32 + 4.0;
            let lv = if k <= 5 { 100 * skill_s * k } else { skill_s * (500 * k - 2000) };
            s.text_centered(g, 77.0, y, &format!("{}.{:02}", lv / 100, lv % 100), 11.0, c15(0x4010));
            let n = if k <= 5 { 10 * cdiv * k } else { cdiv * (50 * k - 200) };
            s.text_centered(g, 727.0, y, &format!("${n}K"), 11.0, black());
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
                base - squash((r[0] / cdiv).abs()) as f32,
                base - squash(r[1]) as f32,
                base - 4.0 * r[3] as f32,
            ];
            let cols = [c15(0x4010), if r[0] < 0 { c15(0x7d08) } else { black() }, c15(0x03e0), c15(0x0210)];
            for k in 0..4 {
                let (a, b) = (prev[k].max(72.0), ys[k].max(72.0));
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
                    s.text(g, x1 + 14.0, top + 3.0, &t, 10.0, c15(0x0210));
                }
            }
        }
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
        s.text(g, 218.0, 40.0, "FINANCIAL REPORT", 22.0, black());
        for (i, l) in LEDGER_LABELS.iter().enumerate() {
            s.text_centered(g, 98.0, 99.0 + 17.0 * i as f32, l, 11.0, black());
        }
        s.text_centered(g, 98.0, 99.0 + 136.0 + 7.0, "Total", 11.0, black());
        let last = self.econ.year_index().min(99);
        // eight columns fit the art (the exe draws a ninth off the screen)
        let first = last.saturating_sub(7);
        for (j, y) in (first..=last).enumerate() {
            let x = 75.0 * j as f32;
            s.text_centered(g, x + 222.0, 66.0, &format!("{}", 2001 + y), 12.0, black());
            let row = self.econ.ledger.get(y).copied().unwrap_or_default();
            let mut total = 0.0;
            for (i, v) in row.iter().enumerate() {
                total += v;
                if *v != 0.0 {
                    let t = money(*v as i64);
                    let c = if *v < 0.0 { c15(0x6000) } else { black() };
                    s.text(g, x + 256.0 - text_width(&t, 11.0), 98.0 + 17.0 * i as f32, &t, 11.0, c);
                }
            }
            let t = money(total as i64);
            let c = if total < 0.0 { c15(0x6000) } else { black() };
            s.text(g, x + 256.0 - text_width(&t, 11.0), 98.0 + 17.0 * 7.0 + 24.0, &t, 11.0, c);
        }
        g.flush();
    }

    // ---- F5 ---------------------------------------------------------------------------------------------------------

    fn tile_colour(&self, a: i32, b: i32) -> Option<[f32; 4]> {
        let t = self.course.ty[idx(a, b)];
        if t == sg_core::course::t::OUT {
            return None;
        }
        match self.route_tab {
            2 => {
                if t == sg_core::course::t::WATER {
                    return Some(c15(0x0218));
                }
                let (mut good, mut bad) = (0i32, 0i32);
                for da in -2..=2 {
                    for db in -2..=2 {
                        if inside(a + da, b + db) {
                            let i = idx(a + da, b + db);
                            good += self.course.happy.get(i).copied().unwrap_or(0) as i32;
                            bad += self.course.unhappy.get(i).copied().unwrap_or(0) as i32;
                        }
                    }
                }
                let (gr, rd) = ((good * 12).min(255) as f32 / 255.0, (bad * 12).min(255) as f32 / 255.0);
                Some(rgb(rd, gr, ((gr + rd) / 2.0).min(1.0) * 0.5))
            }
            3 => {
                let v = sg_core::homes::lot_value(&self.course, &self.club.holes, self.difficulty, a, b);
                let g = ((v / 4) * 3 / 2).clamp(0, 255) as f32 / 255.0;
                Some(rgb(0.0, g, 0.0))
            }
            _ => Some(c15(match TYPES[(t as usize).min(22)].class {
                0 | 1 => 0x3394,
                2 => 0x1310,
                4 => 0x0204,
                7 => 0x6310,
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

    pub fn routing_click(&mut self, vx: f32, vy: f32) {
        let tabs = [
            (Rect::new(183.0, 254.0, 80.0, 80.0), 0),
            (Rect::new(62.0, 315.0, 80.0, 80.0), 1),
            (Rect::new(536.0, 254.0, 80.0, 80.0), 2),
            (Rect::new(659.0, 315.0, 80.0, 80.0), 3),
        ];
        for (r, t) in tabs {
            if r.has(vx, vy) {
                self.route_tab = t;
                return;
            }
        }
        if Rect::new(662.0, 532.0, 80.0, 64.0).has(vx, vy) {
            self.screen = Screen::Play;
            return;
        }
        if self.route_tab == 0 && vy > 0x60 as f32 {
            let row = ((vy - 0x68 as f32) / 17.0) as i32 + 1;
            let h = (row.clamp(1, 9) + if vx > 399.0 { 9 } else { 0 }).clamp(1, 18);
            self.route_hole = h as usize;
        }
    }

    pub fn draw_routing(&mut self, g: &mut Gfx) {
        let s = Ui::new(self.draw_w, self.draw_h);
        self.view = s.view;
        self.dim(g, &s);
        let tab = self.route_tab.min(3);
        if self.reports.route[tab].tex.is_some() {
            s.image(g, &self.reports.route[tab], 0.0, 0.0);
        }
        if self.reports.route_bottom.tex.is_some() {
            s.image_part(g, &self.reports.route_bottom, 0.0, 253.0, 0.0, 253.0, 800.0, 347.0);
        }
        // the minimap
        for a in 0..N {
            for b in 0..N {
                let Some(c) = self.tile_colour(a, b) else { continue };
                let (x, y) = Self::mini(a as f32, b as f32);
                s.fill(g, x - 4.0, y - 2.0, 8.0, 4.0, c);
                s.fill(g, x - 2.0, y - 3.0, 4.0, 6.0, c);
            }
        }
        s.text_centered(g, 400.0, 22.0, "ROUTING MAP", 22.0, black());
        let name = if tab == 1 { format!("{} Employees", self.course_name) } else { self.course_name.clone() };
        s.text_centered(g, 400.0, if tab == 1 { 54.0 } else { 64.0 }, &name, 14.0, black());
        // holes: tee to green with the hole number
        for h in 1..19 {
            let hr = &self.club.holes[h];
            if hr.par == 0 {
                continue;
            }
            let (tx, ty) = Self::mini(hr.back.0 as f32, hr.back.1 as f32);
            let (gx, gy) = Self::mini(hr.pin.0 as f32, hr.pin.1 as f32);
            let n = ((gx - tx).hypot(gy - ty) / 2.0).max(1.0) as i32;
            for k in 0..=n {
                let t = k as f32 / n as f32;
                s.fill(g, tx + (gx - tx) * t - 1.0, ty + (gy - ty) * t - 1.0, 2.0, 2.0, rgb(1.0, 1.0, 1.0));
            }
            if tab != 1 {
                s.text_centered(g, (tx + gx) / 2.0, (ty + gy) / 2.0 - 4.0, &format!("{h}"), 12.0, c15(0x7ff0));
            }
        }
        match tab {
            0 => {
                s.text_centered(g, 400.0, 92.0, "COURSE ROUTING", 13.0, black());
                s.text_centered(g, 400.0, 112.0, "Left click to select hole.", 11.0, black());
                for half in 0..2 {
                    let x = if half == 0 { 0x47 as f32 } else { 0x234 as f32 };
                    for (dx, t) in [(0.0, "Hole"), (61.0, "Par"), (111.0, "Yards"), (168.0, "Minutes")] {
                        s.text_centered(g, x + dx, 90.0, t, 11.0, black());
                    }
                }
                for h in 1..19 {
                    let x = if h <= 9 { 0x47 as f32 } else { 0x239 as f32 };
                    let y = 0x68 as f32 + 17.0 * ((h - 1) % 9) as f32;
                    let hr = &self.club.holes[h];
                    if h == self.route_hole {
                        s.fill(g, x - 39.0, y - 3.0, 238.0, 17.0, c15(0x23e8));
                    }
                    let c = if hr.par == 0 { c15(0x6318) } else { black() };
                    s.text_centered(g, x, y + 10.0, &format!("{h}"), 11.0, c);
                    if hr.par != 0 {
                        s.text_centered(g, x + 61.0, y + 10.0, &format!("{}", hr.par), 11.0, black());
                        s.text_centered(g, x + 111.0, y + 10.0, &format!("{}", hr.length), 11.0, black());
                        if hr.tee_shots > 0 {
                            s.text_centered(g, x + 168.0, y + 10.0, &format!("{}", hr.time / hr.tee_shots / 40), 11.0, black());
                        }
                    }
                }
            }
            1 => {
                let list: Vec<&staff::Employee> =
                    self.employees.iter().filter(|e| e.active && e.job < 0 && e.job != staff::job::OWNER).collect();
                for (k, e) in list.iter().take(8).enumerate() {
                    let r = (k % 4) as f32;
                    let x = if k < 4 { 0x38 as f32 } else { 0x1ae as f32 };
                    let jobn = (-2 - e.job as i32).clamp(0, 3) as usize;
                    let name = STAFF_NAMES[jobn][e.upgraded as usize];
                    s.text(g, x - 15.0, 0x47 as f32 + 45.0 * r, &format!("{}. {name}", k + 1), 12.0, black());
                    s.text(g, x, 0x56 as f32 + 45.0 * r + 2.0, &format!("Post: {:?}", e.post.unwrap_or((0, 0))), 11.0, black());
                }
                for e in self.employees.iter().filter(|e| e.active) {
                    let Some((pa, pb)) = e.post else { continue };
                    let (cx, cy) = Self::mini(pa as f32, pb as f32);
                    let mut rr = if e.job == staff::job::GROUNDSKEEPER { 48.0 } else { 24.0 };
                    if e.upgraded {
                        rr *= 1.5;
                    }
                    let col = match e.job {
                        staff::job::SODA_VENDOR => c15(0x03ff),
                        staff::job::GROUNDSKEEPER => c15(0x7ff0),
                        staff::job::RANGER => c15(0x0018),
                        staff::job::CLUB_PRO => c15(0x6318),
                        _ => rgb(1.0, 1.0, 1.0),
                    };
                    for k in 0..25 {
                        let a = k as f32 / 25.0 * std::f32::consts::TAU;
                        s.fill(g, cx + rr * 0.5 * a.cos(), cy + rr * 0.25 * a.sin(), 2.0, 2.0, col);
                    }
                }
            }
            2 => {
                s.text_centered(g, 400.0, 188.0, "COURSE AURA", 14.0, black());
                s.text(g, 0x88 as f32, 0xce as f32, "Unhappy", 11.0, black());
                s.text(g, 0x270 as f32, 0xce as f32, "Happy", 11.0, black());
                for (i, l) in
                    wrap_text("Course AURA indicates where on your course golfers have been happy (green) or unhappy (red).", 12.0, 300.0)
                        .iter()
                        .enumerate()
                {
                    s.text(g, 250.0, 110.0 + 15.0 * i as f32, l, 12.0, black());
                }
            }
            _ => {
                s.text_centered(g, 400.0, 188.0, "HOME SITE VALUE", 14.0, black());
                let inc = [
                    "Things which INCREASE home value",
                    "Close to water and trees.",
                    "Close to a fun golf hole.",
                    "Close to a top 100 or top 18 hole.",
                    "Building a Marina.",
                ];
                let dec = [
                    "Things which DECREASE home value",
                    "Close to an unfun hole.",
                    "Close to another building.",
                    "Too close to green, fairway, or OB.",
                    "Far away from the golf course.",
                ];
                for (i, (a, b)) in inc.iter().zip(dec.iter()).enumerate() {
                    let y = [0x56, 0x68, 0x79, 0x8a, 0x9c][i] as f32 + 10.0;
                    s.text(g, 0x33 as f32, y, a, 11.0, black());
                    s.text(g, 0x1ee as f32, y, b, 11.0, black());
                }
            }
        }
        g.flush();
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
        s.text_centered(g, 287.0, 40.0, "KEYBOARD SHORTCUTS", 22.0, black());
        let ys = [0x51, 0x62, 0x73, 0x84, 0x95, 0xa6, 0xb7, 200, 0xd9, 0xea, 0xfb, 0x10c, 0x11d, 0x12e, 0x13f, 0x157];
        for (i, (k, d)) in SHORTCUTS.iter().enumerate() {
            let y = ys[i] as f32 + 10.0;
            let c = if i == 7 { c15(0x0200) } else { black() };
            s.text_centered(g, 0x44 as f32, y, k, 11.0, c);
            s.text(g, 0x6c as f32, y, d, 11.0, c);
        }
        for (i, (k, d)) in SHORTCUTS_RIGHT.iter().enumerate() {
            let y = ys[i] as f32 + 10.0;
            s.text_centered(g, 0x1bc as f32, y, k, 11.0, black());
            s.text(g, 0x1e3 as f32, y, d, 11.0, black());
        }
        g.flush();
    }
}
