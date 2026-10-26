//! The top screen's information, over the 3D course: the DS convention of reading on top and acting below (plan.rs).
//!
//!   * a bar along the top: cash (with what was just spent or earned flashing beside it), the date and game speed, golfers
//!     on the course and the club's fun and skill ratings;
//!   * along the foot: the game's own messages (its ticker) and golfers' thoughts as they have them, the game's main
//!     feedback on a course ("This hole is too short!"), plus short notes from the tools;
//!   * a card for whatever a long press on the plan inspected (golfer, hole, building, ground);
//!   * the INFO tab's panels: money, holes, golfers, staff, help (Select steps through them);
//!   * Start's pause menu: on the top screen for the buttons, and as big rows on the bottom screen for the stylus.
//! All of it sits at the screen's depth (no stereo shear), drawn after the world for each eye.

use crate::app::App;
use crate::gfx::{Gfx, Mat4, Mode, Uniforms, Vert};
use crate::plan::{building_name, hole_length, Panel};
use crate::text::{money, Size, Text};
use sg_core::economy as econ;
use sg_core::golfer::SLOTS;
use sg_core::land;
use sg_core::terrain as t;
use std::collections::VecDeque;
use std::time::{Duration, Instant};

const W: f32 = 400.0;
const H: f32 = 240.0;
const BAR_H: f32 = 15.0;
const MONTHS: [&str; 12] = ["Jan", "Feb", "Mar", "Apr", "May", "Jun", "Jul", "Aug", "Sep", "Oct", "Nov", "Dec"];
const THOUGHT_FOR: Duration = Duration::from_secs(7);
const NOTE_FOR: Duration = Duration::from_millis(2600);
const CARD_FOR: Duration = Duration::from_secs(7);

pub struct Hud {
    pub panel: Option<Panel>,
    card: Option<(Vec<String>, Instant)>,
    note: Option<(String, Instant)>,
    /// The latest thoughts, newest last: text, colour, when.
    thoughts: VecDeque<(String, [f32; 4], Instant)>,
    /// Each golfer's thought last seen (thought id, stamp), so each is shown once.
    seen: Vec<(u8, u8)>,
    last_cash: Option<f64>,
    /// Money just spent or earned: the running sum and when it last changed.
    flash: Option<(f64, Instant)>,
    /// Start's pause menu is open, with this row chosen.
    pub menu: Option<usize>,
}

/// Start's pause menu, in order.
pub const MENU: [&str; 5] = ["Resume", "Save game", "Load game", "Controls", "Quit"];
/// The menu's rows on the bottom screen (320 x 240): x, first y, width, height, pitch.
const MB: (f32, f32, f32, f32, f32) = (40.0, 28.0, 240.0, 34.0, 41.0);
/// The panels Select steps through, then none.
const CYCLE: [Panel; 4] = [Panel::Money, Panel::Holes, Panel::Golfers, Panel::Staff];

fn rgba(c: [f32; 3], a: f32) -> [f32; 4] {
    [c[0], c[1], c[2], a]
}

pub fn date_text(app: &App) -> String {
    let mi = app.econ.month_index();
    format!("{} {}", MONTHS[((2 + mi % 8) % 12) as usize], 2001 + mi / 8)
}

fn ty_name(ty: i32) -> &'static str {
    match ty as u8 {
        t::TT_TEE => "Tee",
        t::TT_PUTTING_GREEN | t::TT_TRICKY_GREEN => "Green",
        t::TT_FAIRWAY | t::TT_FIRM_FAIRWAY => "Fairway",
        t::TT_ROUGH => "Rough",
        t::TT_DEEP_ROUGH => "Deep rough",
        t::TT_WOODS => "Trees",
        t::TT_ROCK => "Rock",
        t::TT_CLIFF => "Cliff",
        t::TT_MARSH => "Marsh",
        t::TT_BUILDING => "Building",
        _ if t::is_water(ty) => "Water",
        _ if t::is_sand(ty) => "Sand",
        _ => "Ground",
    }
}

impl Hud {
    pub fn new() -> Hud {
        Hud { panel: None, card: None, note: None, thoughts: VecDeque::new(), seen: vec![(0, 0); SLOTS], last_cash: None, flash: None, menu: None }
    }

    pub fn toggle(&mut self, p: Panel) {
        self.card = None;
        self.panel = if self.panel == Some(p) { None } else { Some(p) };
    }

    /// Shows `p` (without toggling it off if it is up).
    pub fn show(&mut self, p: Panel) {
        self.panel = Some(p);
    }

    /// Puts away the panel and the inspect card.
    pub fn close(&mut self) {
        self.panel = None;
        self.card = None;
    }

    /// Select: the next panel (money, holes, golfers, staff), then none.
    pub fn cycle(&mut self) {
        self.card = None;
        self.panel = match self.panel.and_then(|p| CYCLE.iter().position(|&c| c == p)) {
            Some(i) if i + 1 < CYCLE.len() => Some(CYCLE[i + 1]),
            Some(_) => None,
            None => Some(CYCLE[0]),
        };
    }

    /// The menu row under a bottom-screen point.
    pub fn menu_hit(x: f32, y: f32) -> Option<usize> {
        let (mx, my, w, h, pitch) = MB;
        if x < mx || x > mx + w || y < my {
            return None;
        }
        let i = ((y - my) / pitch) as usize;
        (i < MENU.len() && y - my - i as f32 * pitch <= h).then_some(i)
    }

    /// The pause menu as big rows on the bottom screen, over the plan.
    pub fn render_menu_bottom(&self, g: &mut Gfx, text: &Text) {
        let Some(sel) = self.menu else { return };
        let u = Uniforms::flat(&Mat4::ortho(0.0, 320.0, 240.0, 0.0, -1.0, 1.0), &Mat4::identity());
        let rect = |g: &mut Gfx, x: f32, y: f32, w: f32, h: f32, c: [f32; 4]| {
            let v = |x: f32, y: f32| Vert::new(x, y, 0.0, 0.0, 0.0).col(c);
            g.quad(Mode::Flat, None, &u, [v(x, y), v(x + w, y), v(x + w, y + h), v(x, y + h)]);
        };
        rect(g, 0.0, 0.0, 320.0, 240.0, [0.0, 0.0, 0.0, 0.9]);
        let t = "PAUSED";
        text.draw(g, &u, (320.0 - text.width(t, Size::Normal)) * 0.5, 8.0, t, Size::Normal, [1.0, 0.85, 0.3, 1.0]);
        let (mx, my, w, h, pitch) = MB;
        for (i, label) in MENU.iter().enumerate() {
            let y = my + i as f32 * pitch;
            let on = i == sel;
            rect(g, mx, y, w, h, if on { [0.95, 0.8, 0.25, 1.0] } else { [0.12, 0.3, 0.16, 1.0] });
            rect(g, mx, y + h - 2.0, w, 2.0, [0.0, 0.0, 0.0, 0.35]);
            let c = if on { [0.08, 0.08, 0.05, 1.0] } else { [1.0, 1.0, 1.0, 1.0] };
            let lw = text.width(label, Size::Normal);
            text.draw(g, &u, mx + (w - lw) * 0.5, y + (h - text.line_height(Size::Normal)) * 0.5, label, Size::Normal, c);
        }
    }

    pub fn note(&mut self, s: &str) {
        self.note = Some((s.to_string(), Instant::now()));
    }

    /// Whether anything on it changes over time (a redraw is due).
    pub fn animating(&self) -> bool {
        self.flash.is_some() || self.note.is_some() || self.card.is_some() || !self.thoughts.is_empty()
    }

    /// Once a frame: new thoughts, money moving, things timing out.
    pub fn update(&mut self, app: &App, thoughts_on: bool) {
        let now = Instant::now();
        // cash: what changed since the last frame, summed while it keeps changing
        let cash = app.econ.cash;
        if let Some(last) = self.last_cash {
            let d = cash - last;
            if d.abs() >= 1.0 {
                let sum = match self.flash {
                    Some((s, at)) if now.duration_since(at) < Duration::from_millis(1500) => s + d,
                    _ => d,
                };
                self.flash = Some((sum, now));
            }
        }
        self.last_cash = Some(cash);
        if self.flash.is_some_and(|(_, at)| now.duration_since(at) > Duration::from_millis(2200)) {
            self.flash = None;
        }
        if self.note.as_ref().is_some_and(|(_, at)| now.duration_since(*at) > NOTE_FOR) {
            self.note = None;
        }
        if self.card.as_ref().is_some_and(|(_, at)| now.duration_since(*at) > CARD_FOR) {
            self.card = None;
        }
        while self.thoughts.front().is_some_and(|(_, _, at)| now.duration_since(*at) > THOUGHT_FOR) {
            self.thoughts.pop_front();
        }
        // golfers' thoughts as they have them (the game shows each over the golfer for about 56 ticks: thoughts_ui.rs)
        for gi in 0..SLOTS.min(app.club.g.len()) {
            let gg = &app.club.g[gi];
            let key = (gg.thought, gg.thought_stamp);
            if gg.hole <= 0 || !(1..=7).contains(&gg.timer) || gg.thought == 0 || gg.thought == 0x32 {
                continue;
            }
            if self.seen[gi] == key {
                continue;
            }
            self.seen[gi] = key;
            if !thoughts_on {
                continue;
            }
            let line = app.club.thought(&app.course, gg.thought as u32, gg.thought_arg & 0x3fff, gi);
            if line.text.is_empty() {
                continue;
            }
            let speaker = if line.partner { gi ^ 1 } else { gi };
            let c = match line.tone {
                sg_core::thoughts::Tone::Good => [0.55, 1.0, 0.55, 1.0],
                sg_core::thoughts::Tone::Bad => [1.0, 0.6, 0.5, 1.0],
                _ => [1.0, 1.0, 1.0, 1.0],
            };
            let text = format!("{}: {}", app.club.name(speaker), line.text);
            if self.thoughts.back().is_some_and(|(t, _, _)| *t == text) {
                continue;
            }
            self.thoughts.push_back((text, c, now));
            while self.thoughts.len() > 3 {
                self.thoughts.pop_front();
            }
        }
    }

    /// A long press on the plan at world (wx, wz): the nearest golfer, else the hole, building or ground there.
    pub fn inspect(&mut self, app: &App, wx: f32, wz: f32) {
        self.panel = None;
        let mut lines = Vec::new();
        // a golfer within about a tile
        let mut best: Option<(usize, f32)> = None;
        for gi in 0..SLOTS.min(app.club.g.len()) {
            let gg = &app.club.g[gi];
            if gg.hole <= 0 || gg.hole >= 19 {
                continue;
            }
            let (x, z) = app.units_to_world(gg.x, gg.y);
            let d = (x - wx).hypot(z - wz);
            if d < t::TILE_SIZE * 1.3 && best.is_none_or(|(_, b)| d < b) {
                best = Some((gi, d));
            }
        }
        let (a, b) = app.terrain.tile_of(wx, wz);
        if let Some((gi, _)) = best {
            let gg = &app.club.g[gi];
            lines.push(app.club.name(gi));
            lines.push(format!("On hole {}, stroke {}", gg.hole, gg.strokes));
            lines.push(format!("Mood {}   Hunger {}   Thirst {}   Tired {}", gg.mood, gg.hunger, gg.thirst, gg.fatigue));
            if gg.thought != 0 && gg.thought != 0x32 {
                let line = app.club.thought(&app.course, gg.thought as u32, gg.thought_arg & 0x3fff, gi);
                if !line.text.is_empty() {
                    lines.push(format!("\"{}\"", line.text));
                }
            }
        } else if app.terrain.inside(a, b) {
            let ty = app.terrain.ty[app.terrain.tile_index(a, b)] as i32;
            let fl = if sg_core::course::inside(a, b) { app.course.flags[sg_core::course::idx(a, b)] } else { 0 };
            let h = (fl & 0x1f) as usize;
            let object = app.land.as_ref().and_then(|l| l.object_on(a, b).map(|o| l.objects[o]));
            if (ty == t::TT_TEE as i32 || ty == t::TT_PUTTING_GREEN as i32) && (1..19).contains(&h) {
                let r = &app.club.holes[h];
                lines.push(format!("Hole {h}"));
                if r.par == 0 {
                    lines.push("Not open yet: tap its green to open it".into());
                } else {
                    let yards = if r.length > 0 { r.length } else { hole_length(r.back, r.pin).0 };
                    lines.push(format!("Par {}   {} yards", r.par, yards));
                    lines.push(format!("Played {} times   Green fees {}", r.tee_shots, money(r.fees as i64 * 100)));
                    if r.tee_shots > 0 {
                        lines.push(format!("Average mood {}   Gave up {}", r.mood_sum / r.tee_shots.max(1), r.quits));
                    }
                }
            } else if let Some(o) = object {
                let name = if o.kind == land::K_PATH { "Path".to_string() } else { building_name(o.kind) };
                lines.push(name);
                if o.kind >= 6 && o.kind != land::K_CLUBHOUSE {
                    lines.push(format!("Level {}", o.sub + 1));
                }
            } else {
                lines.push(ty_name(ty).to_string());
                let owned = app.land.as_ref().is_none_or(|l| l.in_play(a, b));
                lines.push(if owned { "Your land".into() } else { "Not your land".into() });
                lines.push(format!("Height {}", app.terrain.tile_levels(a, b).iter().map(|&v| v as i32).max().unwrap_or(0)));
            }
        }
        if !lines.is_empty() {
            self.card = Some((lines, Instant::now()));
        }
    }

    /// The overlay for one eye (after the world, at the screen's depth).
    pub fn render(&self, g: &mut Gfx, text: &Text, app: &App) {
        let u = Uniforms::flat(&Mat4::ortho(0.0, W, H, 0.0, -1.0, 1.0), &Mat4::identity());
        let rect = |g: &mut Gfx, x: f32, y: f32, w: f32, h: f32, c: [f32; 4]| {
            let v = |x: f32, y: f32| Vert::new(x, y, 0.0, 0.0, 0.0).col(c);
            g.quad(Mode::Flat, None, &u, [v(x, y), v(x + w, y), v(x + w, y + h), v(x, y + h)]);
        };
        let now = Instant::now();

        // the bar
        rect(g, 0.0, 0.0, W, BAR_H, [0.0, 0.0, 0.0, 0.55]);
        let units = app.econ.cash.round() as i64;
        let cash = if app.econ.sandbox { "SANDBOX".to_string() } else { money(units) };
        let cc = if units < 0 { [1.0, 0.45, 0.4, 1.0] } else { [0.6, 1.0, 0.55, 1.0] };
        let mut x = 4.0 + text.draw(g, &u, 4.0, 1.0, &cash, Size::Normal, cc);
        if let Some((sum, at)) = self.flash {
            let fade = 1.0 - (now.duration_since(at).as_secs_f32() / 2.2).clamp(0.0, 1.0);
            let s = if sum < 0.0 { money(sum.round() as i64) } else { format!("+{}", money(sum.round() as i64)) };
            let c = if sum < 0.0 { [1.0, 0.55, 0.45, fade] } else { [0.6, 1.0, 0.6, fade] };
            x += 6.0;
            text.draw(g, &u, x, 1.0, &s, Size::Normal, c);
        }
        let _ = x;
        let speed = if app.paused {
            "PAUSED".to_string()
        } else {
            ">".repeat(match app.speed {
                1 => 1,
                2 => 2,
                _ => 3,
            })
        };
        let mid = format!("{}   {}", date_text(app), speed);
        let mw = text.width(&mid, Size::Normal);
        text.draw(g, &u, (W - mw) * 0.5, 1.0, &mid, Size::Normal, if app.paused { [1.0, 0.85, 0.3, 1.0] } else { [1.0, 1.0, 1.0, 1.0] });
        let golfers = app.club.g.iter().filter(|gg| gg.hole > 0 && gg.hole < 19).count();
        let r = app.club.ratings;
        let right = format!("{golfers} golfers   Fun {}   Skill {}.{:02}", r.fun, r.skill / 100, (r.skill % 100).abs());
        let rw = text.width(&right, Size::Small);
        text.draw(g, &u, W - rw - 4.0, 2.5, &right, Size::Small, [0.85, 0.92, 1.0, 1.0]);

        // the foot: the game's message, then the newest thoughts, then a tool's note
        let mut foot: Vec<(String, [f32; 4])> = Vec::new();
        if app.ticker.showing && !app.ticker.text.is_empty() {
            for l in text.wrap(&app.ticker.text.replace('\n', " "), Size::Normal, W - 12.0).into_iter().take(2) {
                foot.push((l, [1.0, 0.9, 0.45, 1.0]));
            }
        }
        for (s, c, at) in self.thoughts.iter().rev().take(2).collect::<Vec<_>>().into_iter().rev() {
            let age = now.duration_since(*at).as_secs_f32() / THOUGHT_FOR.as_secs_f32();
            let a = if age > 0.75 { (1.0 - age) * 4.0 } else { 1.0 };
            for l in text.wrap(s, Size::Normal, W - 12.0).into_iter().take(2) {
                foot.push((l, rgba([c[0], c[1], c[2]], a.clamp(0.0, 1.0))));
            }
        }
        if let Some((s, _)) = &self.note {
            foot.push((s.clone(), [1.0, 1.0, 1.0, 1.0]));
        }
        let lh = text.line_height(Size::Normal);
        let foot: Vec<_> = foot.into_iter().rev().take(4).collect::<Vec<_>>().into_iter().rev().collect();
        if !foot.is_empty() && self.panel.is_none() {
            let top = H - lh * foot.len() as f32 - 4.0;
            rect(g, 0.0, top - 2.0, W, H - top + 2.0, [0.0, 0.0, 0.0, 0.45]);
            for (i, (s, c)) in foot.iter().enumerate() {
                text.draw_shadowed(g, &u, 6.0, top + i as f32 * lh, s, Size::Normal, *c);
            }
        }

        // the inspect card
        if let Some((lines, _)) = &self.card {
            let w = lines.iter().map(|l| text.width(l, Size::Normal)).fold(120.0, f32::max) + 16.0;
            let h = lines.len() as f32 * lh + 10.0;
            let (x, y) = ((W - w) * 0.5, BAR_H + 8.0);
            rect(g, x, y, w, h, [0.02, 0.06, 0.03, 0.88]);
            rect(g, x, y, w, 1.5, [1.0, 0.9, 0.3, 1.0]);
            for (i, l) in lines.iter().enumerate() {
                let c = if i == 0 { [1.0, 0.92, 0.5, 1.0] } else { [1.0, 1.0, 1.0, 1.0] };
                text.draw(g, &u, x + 8.0, y + 5.0 + i as f32 * lh, l, Size::Normal, c);
            }
        }

        // a panel
        if let Some(p) = self.panel {
            let (x, y, w, h) = (8.0, BAR_H + 4.0, W - 16.0, H - BAR_H - 10.0);
            rect(g, x, y, w, h, [0.02, 0.05, 0.03, 0.9]);
            rect(g, x, y, w, 1.5, [1.0, 0.9, 0.3, 1.0]);
            let (title, rows) = self.panel_rows(app, p);
            text.draw(g, &u, x + 8.0, y + 4.0, &title, Size::Normal, [1.0, 0.92, 0.5, 1.0]);
            let sl = text.line_height(Size::Small) + 1.0;
            let max = ((h - 24.0) / sl) as usize;
            for (i, (l, r, c)) in rows.iter().take(max).enumerate() {
                let ry = y + 20.0 + i as f32 * sl;
                text.draw(g, &u, x + 8.0, ry, l, Size::Small, *c);
                if !r.is_empty() {
                    let rw = text.width(r, Size::Small);
                    text.draw(g, &u, x + w - rw - 8.0, ry, r, Size::Small, *c);
                }
            }
            let hint = "Select: next   tap the button again to close";
            let hw = text.width(hint, Size::Small);
            text.draw(g, &u, x + w - hw - 8.0, y + 5.0, hint, Size::Small, [0.6, 0.65, 0.6, 1.0]);
        }

        // Start's pause menu
        if let Some(sel) = self.menu {
            rect(g, 0.0, BAR_H, W, H - BAR_H, [0.0, 0.0, 0.0, 0.45]);
            let rh = lh + 6.0;
            let (w, h) = (190.0, 30.0 + MENU.len() as f32 * rh + 16.0);
            let (x, y) = ((W - w) * 0.5, (H - h) * 0.5 + 6.0);
            rect(g, x, y, w, h, [0.02, 0.06, 0.03, 0.94]);
            rect(g, x, y, w, 1.5, [1.0, 0.9, 0.3, 1.0]);
            let t = "PAUSED";
            text.draw(g, &u, x + (w - text.width(t, Size::Normal)) * 0.5, y + 6.0, t, Size::Normal, [1.0, 0.85, 0.3, 1.0]);
            for (i, label) in MENU.iter().enumerate() {
                let ry = y + 26.0 + i as f32 * rh;
                let on = i == sel;
                if on {
                    rect(g, x + 10.0, ry, w - 20.0, rh - 2.0, [0.95, 0.8, 0.25, 1.0]);
                }
                let c = if on { [0.08, 0.08, 0.05, 1.0] } else { [1.0, 1.0, 1.0, 1.0] };
                text.draw(g, &u, x + (w - text.width(label, Size::Normal)) * 0.5, ry + 2.0, label, Size::Normal, c);
            }
            let hint = "D-pad choose   A select   B back";
            text.draw(g, &u, x + (w - text.width(hint, Size::Small)) * 0.5, y + h - 14.0, hint, Size::Small, [0.65, 0.7, 0.65, 1.0]);
        }
    }

    /// A panel's title and rows (left text, right text, colour).
    fn panel_rows(&self, app: &App, p: Panel) -> (String, Vec<(String, String, [f32; 4])>) {
        let white = [1.0, 1.0, 1.0, 1.0];
        let dim = [0.7, 0.75, 0.7, 1.0];
        let mut rows = Vec::new();
        match p {
            Panel::Money => {
                let year = app.econ.ledger.last().copied().unwrap_or([0.0; econ::LEDGER_COLUMNS]);
                const LABELS: [&str; 8] = ["Green fees", "Home sites", "Food and drink", "Course building", "Facilities", "Salaries", "Maintenance", "Other"];
                let mut net = 0.0;
                for (k, label) in LABELS.iter().enumerate() {
                    let v = year[k];
                    net += v;
                    let c = if v < 0.0 { [1.0, 0.65, 0.55, 1.0] } else if v > 0.0 { [0.65, 1.0, 0.6, 1.0] } else { dim };
                    rows.push((label.to_string(), money(v.round() as i64), c));
                }
                rows.push((String::new(), String::new(), white));
                rows.push(("This year".into(), money(net.round() as i64), if net < 0.0 { [1.0, 0.6, 0.5, 1.0] } else { [0.6, 1.0, 0.6, 1.0] }));
                rows.push(("Cash".into(), money(app.econ.cash.round() as i64), white));
                ("Money: this year".into(), rows)
            }
            Panel::Holes => {
                let mut n = 0;
                for h in 1..19 {
                    let r = &app.club.holes[h];
                    if r.back.0 == 0 && r.pin.0 == 0 {
                        continue;
                    }
                    n += 1;
                    let yards = if r.length > 0 { r.length } else { hole_length(r.back, r.pin).0 };
                    let state = if r.par == 0 { "not open: tap its green".to_string() } else { format!("par {}   {} yds", r.par, yards) };
                    let mood = if r.tee_shots > 0 { format!("played {}   mood {}", r.tee_shots, r.mood_sum / r.tee_shots.max(1)) } else { String::new() };
                    rows.push((format!("Hole {h}   {state}"), mood, if r.par == 0 { [1.0, 0.85, 0.4, 1.0] } else { white }));
                }
                if n == 0 {
                    rows.push(("No holes yet: COURSE tab, HOLE, tap a tee then a green".into(), String::new(), dim));
                }
                let r = app.club.ratings;
                rows.push((String::new(), String::new(), white));
                rows.push((format!("Course par {}   Fun {}   Skill {}.{:02}", r.par, r.fun, r.skill / 100, (r.skill % 100).abs()), String::new(), dim));
                (format!("Holes ({n})"), rows)
            }
            Panel::Golfers => {
                let on: Vec<usize> = (0..SLOTS.min(app.club.g.len())).filter(|&gi| app.club.g[gi].hole > 0 && app.club.g[gi].hole < 19).collect();
                let waiting = app.club.g.iter().filter(|gg| gg.hole == -1).count();
                let avg = if on.is_empty() { 0 } else { on.iter().map(|&gi| app.club.g[gi].mood).sum::<i32>() / on.len() as i32 };
                rows.push((format!("{} on the course   {} waiting   average mood {}", on.len(), waiting, avg), String::new(), dim));
                for &gi in on.iter().take(14) {
                    let gg = &app.club.g[gi];
                    let c = if gg.mood < 0 { [1.0, 0.65, 0.55, 1.0] } else { white };
                    rows.push((app.club.name(gi), format!("hole {}   mood {}", gg.hole, gg.mood), c));
                }
                ("Golfers".into(), rows)
            }
            Panel::Staff => {
                const NAMES: [&str; 4] = ["Club pros", "Rangers", "Groundskeepers", "Soda vendors"];
                for (k, name) in NAMES.iter().enumerate() {
                    let n = app.employees.iter().filter(|e| e.active && e.job == -2 - k as i8).count();
                    rows.push((name.to_string(), n.to_string(), white));
                }
                rows.push((String::new(), String::new(), white));
                rows.push(("STAFF tab: the green button hires one, the red one fires one".into(), String::new(), dim));
                rows.push(("Groundskeepers keep the grass and weeds down; rangers keep play moving;".into(), String::new(), dim));
                rows.push(("soda vendors sell drinks; the club pro teaches.".into(), String::new(), dim));
                ("Staff".into(), rows)
            }
            Panel::Help => {
                for (l, r) in [
                    ("Paint", "drag on the bottom map"),
                    ("Do it once", "tap"),
                    ("Look at something", "hold the stylus still"),
                    ("New hole", "HOLE: tap a tee, then a green"),
                    ("Scroll", "C-stick, D-pad, or hold L or R and drag"),
                    ("Zoom", "tap L (out) or R (in)"),
                    ("Undo", "UNDO button or B"),
                    ("Pause", "PAUSE button or X"),
                    ("Game speed", "ZL slower, ZR faster (or SPEED)"),
                    ("Next tool tab", "Y"),
                    ("Money, holes, golfers, staff", "Select"),
                    ("Menu: save, load, quit", "Start"),
                    ("Brush size", "SIZE, under the map"),
                    ("Use tool at cursor", "Circle Pad moves it, A uses it"),
                ] {
                    rows.push((l.to_string(), r.to_string(), white));
                }
                ("Controls".into(), rows)
            }
        }
    }
}
