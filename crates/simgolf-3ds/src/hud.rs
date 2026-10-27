//! The top screen: the dashboard, the DS convention of reading on top and acting below (the course is on the bottom screen,
//! plan.rs). Nothing of the course is drawn twice: up here are the tools, the money and the club.
//!
//!   * a bar along the top (below);
//!   * the tool palette and its status line (plan.rs `render_palette`), chosen with the D-pad;
//!   * the overview: a mini-map of the whole course with a frame for the bottom screen's view (plan.rs `render_minimap`),
//!     this year's money (income, costs, net), the course (holes open, golfers, ratings) and the game's messages and
//!     golfers' thoughts; Select (or the INFO buttons) puts a page in its place: money, holes, golfers, staff;
//!
//!   * a bar along the top: cash (with what was just spent or earned flashing beside it), the date and game speed, golfers
//!     on the course and the club's fun and skill ratings;
//!   * along the foot: the game's own messages (its ticker) and golfers' thoughts as they have them, the game's main
//!     feedback on a course ("This hole is too short!"), plus short notes from the tools;
//!   * a card for whatever a long press on the course inspected (golfer, hole, building, ground);
//!   * the help page (INFO row), over the whole screen;
//!   * Start's pause menu: on the top screen for the buttons, and as big rows on the bottom screen for the stylus.
//! All of it sits at the screen's depth (no stereo shear), drawn after the world for each eye.

use crate::app::App;
use crate::gfx::{Gfx, Mat4, Mode, Uniforms, Vert};
use crate::plan::{building_name, hole_length, Panel, Plan};
use crate::Demo;
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
const WHITE: [f32; 4] = [1.0, 1.0, 1.0, 1.0];
const MONTHS: [&str; 12] = ["Jan", "Feb", "Mar", "Apr", "May", "Jun", "Jul", "Aug", "Sep", "Oct", "Nov", "Dec"];
const THOUGHT_FOR: Duration = Duration::from_secs(45);
/// Thoughts kept in the overview's feed.
const FEED: usize = 5;
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
    /// The game month now, and the cash when it began: this month's cash flow.
    month: Option<(i32, f64)>,
    /// Golfers who just had a thought: who (golfer slot), the thought's colour, when. A bubble shows over each on the course
    /// for a moment; the newest is who A goes to.
    spoke: Vec<(usize, [f32; 4], Instant)>,
}

/// How long a speech bubble stays over a golfer, and how long A can still go to the newest speaker.
const BUBBLE_FOR: Duration = Duration::from_millis(2500);
const SPEAKER_FOR: Duration = Duration::from_secs(20);

/// Start's pause menu, in order.
pub const MENU: [&str; 6] = ["Resume", "Save game", "Load game", "Controls", "Title screen", "Quit"];
/// The menu's rows on the bottom screen (320 x 240): x, first y, width, height, pitch.
const MB: (f32, f32, f32, f32, f32) = (40.0, 26.0, 240.0, 31.0, 35.0);
/// The panels Select steps through, then none.
const CYCLE: [Panel; 4] = [Panel::Money, Panel::Holes, Panel::Golfers, Panel::Staff];

fn rgba(c: [f32; 3], a: f32) -> [f32; 4] {
    [c[0], c[1], c[2], a]
}

/// "1 golfer", "3 golfers".
fn plural(n: usize, one: &str, many: &str) -> String {
    format!("{n} {}", if n == 1 { one } else { many })
}

/// The course's fun and challenge as the game's words (each hole's average), or None before a hole is open.
pub fn rating_words(app: &App) -> Option<(&'static str, &'static str)> {
    let open = (1..19).filter(|&h| app.club.holes[h].par > 0).count() as i32;
    if open == 0 {
        return None;
    }
    let r = app.club.ratings;
    Some((sg_core::ratings::fun_word(r.fun / open), sg_core::ratings::demand_word(r.skill / open)))
}

/// Golfers' average mood as a word and its colour.
fn mood_word(m: i32) -> (&'static str, [f32; 4]) {
    match m {
        m if m >= 5 => ("Happy", [0.6, 1.0, 0.55, 1.0]),
        m if m >= 0 => ("Content", [1.0, 1.0, 1.0, 1.0]),
        m if m >= -4 => ("Unhappy", [1.0, 0.75, 0.5, 1.0]),
        _ => ("Angry", [1.0, 0.5, 0.45, 1.0]),
    }
}

pub fn date_text(app: &App) -> String {
    month_text(app.econ.month_index() as i32)
}

/// The game's month count as "Jul 2002" (its year is eight playing months, March to October).
pub fn month_text(mi: i32) -> String {
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
        Hud { panel: None, card: None, note: None, thoughts: VecDeque::new(), seen: vec![(0, 0); SLOTS], last_cash: None, flash: None, menu: None, month: None, spoke: Vec::new() }
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

    /// Another game is opening: nothing of the last one's money flash, thoughts or notes carries over.
    pub fn reset(&mut self) {
        self.close();
        self.menu = None;
        self.note = None;
        self.thoughts.clear();
        self.seen = vec![(0, 0); SLOTS];
        self.last_cash = None;
        self.flash = None;
        self.month = None;
        self.spoke.clear();
    }

    /// Select: the next panel (money, holes, golfers, staff), then none; the help page closes.
    pub fn cycle(&mut self) {
        self.card = None;
        if self.panel == Some(Panel::Help) {
            self.panel = None;
            return;
        }
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

    /// The pause menu as big rows on the bottom screen, over the course.
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
        let mi = app.econ.month_index();
        if self.month.is_none_or(|(m, _)| m != mi) {
            self.month = Some((mi, cash));
        }
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
            // the hole they are on (19 is the walk back to the clubhouse)
            let tag = if (1..19).contains(&gg.hole) { format!("H{} ", gg.hole) } else { String::new() };
            let text = format!("{tag}{}: {}", app.club.name(speaker), line.text);
            if self.thoughts.back().is_some_and(|(t, _, _)| *t == text) {
                continue;
            }
            self.thoughts.push_back((text, c, now));
            self.spoke.retain(|(g, _, at)| *g != speaker && now.duration_since(*at) < SPEAKER_FOR);
            self.spoke.push((speaker, c, now));
            while self.thoughts.len() > FEED {
                self.thoughts.pop_front();
            }
        }
    }

    /// The golfer behind the newest thought, while it is recent and they are still on the course: (slot, name).
    pub fn speaker(&self, app: &App) -> Option<(usize, String)> {
        let &(gi, _, at) = self.spoke.last()?;
        let gg = app.club.g.get(gi)?;
        (at.elapsed() < SPEAKER_FOR && gg.hole > 0 && gg.hole <= 19).then(|| (gi, app.club.name(gi)))
    }

    /// On the course (bottom screen): a speech bubble over each golfer who just had a thought, in the thought's colour.
    pub fn render_bubbles(&self, g: &mut Gfx, app: &App, d: &Demo) {
        let now = Instant::now();
        let mvp = d.proj().mul(&d.view());
        let u = Uniforms::flat(&Mat4::ortho(0.0, 320.0, 240.0, 0.0, -1.0, 1.0), &Mat4::identity());
        let quad = |g: &mut Gfx, p: [(f32, f32); 4], c: [f32; 4]| {
            let v = |(x, y): (f32, f32)| Vert::new(x, y, 0.0, 0.0, 0.0).col(c);
            g.quad(Mode::Flat, None, &u, p.map(v));
        };
        for &(gi, c, at) in &self.spoke {
            let age = now.duration_since(at);
            let Some(gg) = app.club.g.get(gi) else { continue };
            if age > BUBBLE_FOR || gg.hole <= 0 || gg.hole > 19 {
                continue;
            }
            let (wx, wz) = app.units_to_world(gg.x, gg.y);
            let q = mvp.apply([wx, d.terrain.height_at(wx, wz), wz]);
            // above the golfer's head: a golfer stands about 45 world units tall
            let lift = (45.0 / d.upp).clamp(10.0, 60.0);
            let (x, y) = ((q[0] + 1.0) * 160.0, (1.0 - q[1]) * 120.0 - lift);
            if !(-10.0..330.0).contains(&x) || !(-10.0..250.0).contains(&y) {
                continue;
            }
            let fade = 1.0 - ((age.as_secs_f32() / BUBBLE_FOR.as_secs_f32()) - 0.8).max(0.0) * 5.0;
            let (bw, bh) = (16.0, 10.0);
            let edge = [0.05, 0.08, 0.05, 0.9 * fade];
            let fill = [1.0, 1.0, 1.0, 0.95 * fade];
            quad(g, [(x - bw * 0.5 - 1.0, y - bh - 1.0), (x + bw * 0.5 + 1.0, y - bh - 1.0), (x + bw * 0.5 + 1.0, y + 1.0), (x - bw * 0.5 - 1.0, y + 1.0)], edge);
            quad(g, [(x - bw * 0.5, y - bh), (x + bw * 0.5, y - bh), (x + bw * 0.5, y), (x - bw * 0.5, y)], fill);
            // the tail, pointing down at the golfer
            quad(g, [(x - 3.0, y), (x + 1.0, y), (x - 1.0, y + 4.0), (x - 1.0, y + 4.0)], fill);
            // three dots in the thought's colour (green good, red bad)
            let dot = [c[0] * 0.8, c[1] * 0.8, c[2] * 0.8, fade];
            for k in 0..3 {
                let dx = x - 4.0 + k as f32 * 4.0;
                quad(g, [(dx - 1.2, y - 6.2), (dx + 1.2, y - 6.2), (dx + 1.2, y - 3.8), (dx - 1.2, y - 3.8)], dot);
            }
        }
    }

    /// A long press on the course at world (wx, wz): the nearest golfer, else the hole, building or ground there.
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
    /// The whole top screen. `off`: the 3D slider's offset for this eye (pixels; the palette and pages stand out by it).
    pub fn render(&self, g: &mut Gfx, text: &Text, app: &App, plan: &Plan, demo: &Demo, off: f32) {
        let u = Uniforms::flat(&Mat4::ortho(0.0, W, H, 0.0, -1.0, 1.0), &Mat4::identity());
        // the same, moved by the eye's offset
        let uo = Uniforms::flat(&Mat4::ortho(-off, W - off, H, 0.0, -1.0, 1.0), &Mat4::identity());
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
            x += text.draw(g, &u, x, 1.0, &s, Size::Normal, c);
        }
        let speed = if app.paused { "PAUSED".to_string() } else { format!("{}x", app.speed.max(1)) };
        let right = match rating_words(app) {
            Some((f, c)) => format!("Fun: {f}   Challenge: {c}"),
            None => "No holes open".to_string(),
        };
        let rw = text.width(&right, Size::Small);
        text.draw(g, &u, W - rw - 4.0, 2.5, &right, Size::Small, [0.85, 0.92, 1.0, 1.0]);
        // the date and speed centred between the cash and the ratings, never over either
        let mid = format!("{}  {}", date_text(app), speed);
        let mw = text.width(&mid, Size::Normal);
        let mx = ((W - mw) * 0.5).min(W - rw - 12.0 - mw).max(x + 8.0);
        text.draw(g, &u, mx, 1.0, &mid, Size::Normal, if app.paused { [1.0, 0.85, 0.3, 1.0] } else { [1.0, 1.0, 1.0, 1.0] });

        // the palette, then the overview or a page
        let speaker = self.speaker(app).map(|(_, n)| n);
        let body = plan.render_palette(g, &uo, demo, app, text, BAR_H + 1.0, speaker.as_deref()) + 2.0;
        let lh = text.line_height(Size::Normal);
        let sl = text.line_height(Size::Small) + 1.0;
        match self.panel {
            Some(Panel::Help) => self.draw_panel_at(g, text, app, Panel::Help, "Select: close", (6.0, BAR_H + 2.0, W - 12.0, H - BAR_H - 4.0), &uo),
            Some(p) => {
                let next = match CYCLE.iter().position(|&c| c == p).and_then(|i| CYCLE.get(i + 1)) {
                    Some(Panel::Money) => "Select: Money",
                    Some(Panel::Holes) => "Select: Holes",
                    Some(Panel::Golfers) => "Select: Golfers",
                    Some(Panel::Staff) => "Select: Staff",
                    _ => "Select: overview",
                };
                self.draw_panel_at(g, text, app, p, next, (4.0, body, W - 8.0, H - body - 2.0), &uo)
            }
            None => {
                // left: a small mini-map at the screen's surface, the holes under it
                let mh = H - body - 2.0;
                let (mw, mmh) = (132.0, 88.0f32.min(mh));
                plan.render_minimap(g, &u, demo, (4.0, body, mw, mmh));
                let gold = [1.0, 0.86, 0.4, 1.0];
                let dimc = [0.78, 0.84, 0.78, 1.0];
                rect(g, 4.0, body + mmh + 1.0, mw, mh - mmh - 1.0, [0.02, 0.05, 0.03, 0.85]);
                let mut hy = body + mmh + 2.0;
                let open: Vec<usize> = (1..19).filter(|&h| app.club.holes[h].par > 0).collect();
                if open.is_empty() {
                    for l in text.wrap("No holes yet: choose HOLE, then tap a tee and a green on the course.", Size::Small, mw - 6.0) {
                        text.draw(g, &uo, 7.0, hy, &l, Size::Small, dimc);
                        hy += sl;
                    }
                } else {
                    let rows = ((body + mh - hy) / sl).floor().max(0.0) as usize;
                    // the newest holes when they do not all fit
                    for &h in open.iter().skip(open.len().saturating_sub(rows)) {
                        let row = sg_core::ratings::report_row(h, &app.club.holes[h], app.difficulty, false);
                        let l = format!("H{h} par {}  {}", row.par, row.type_name);
                        text.draw(g, &uo, 7.0, hy, &l, Size::Small, WHITE);
                        let fw = sg_core::ratings::fun_word(row.fun);
                        let fwd = text.width(fw, Size::Small);
                        text.draw(g, &uo, 4.0 + mw - fwd - 3.0, hy, fw, Size::Small, dimc);
                        hy += sl;
                    }
                }
                // right: the money, the golfers, then their thoughts and the game's messages
                let x0 = 4.0 + mw + 4.0;
                let cw = W - x0 - 4.0;
                rect(g, x0 - 2.0, body, cw + 4.0, mh, [0.02, 0.05, 0.03, 0.85]);
                let urect = |g: &mut Gfx, x: f32, y: f32, w: f32, h: f32, c: [f32; 4]| {
                    let v = |x: f32, y: f32| Vert::new(x, y, 0.0, 0.0, 0.0).col(c);
                    g.quad(Mode::Flat, None, &uo, [v(x, y), v(x + w, y), v(x + w, y + h), v(x, y + h)]);
                };
                let mut y = body + 2.0;
                let pair = |g: &mut Gfx, y: f32, l: &str, r: &str, c: [f32; 4]| {
                    text.draw(g, &uo, x0 + 2.0, y, l, Size::Small, dimc);
                    let rw = text.width(r, Size::Small);
                    text.draw(g, &uo, x0 + cw - rw - 2.0, y, r, Size::Small, c);
                };
                let signed = |v: f64| if v >= 0.5 { format!("+{}", money(v.round() as i64)) } else { money(v.round() as i64) };
                let up_down = |v: f64| if v >= 0.5 { [0.65, 1.0, 0.6, 1.0] } else if v <= -0.5 { [1.0, 0.6, 0.5, 1.0] } else { WHITE };
                let month = self.month.map(|(_, c0)| app.econ.cash - c0).unwrap_or(0.0);
                let year = app.econ.ledger.last().copied().unwrap_or([0.0; econ::LEDGER_COLUMNS]);
                // the ledger's columns: green fees, home sites, food and drink earn; building, facilities, salaries and
                // upkeep cost; "other" holds the starting funds, so only a loss there counts
                let net: f64 = year[..7].iter().sum::<f64>() + year[7].min(0.0);
                pair(g, y, "This month", &signed(month), up_down(month));
                y += sl;
                pair(g, y, "This year", &signed(net), up_down(net));
                y += sl;
                let fees = year[0];
                if fees.abs() < 0.5 && app.club.g.iter().any(|gg| gg.hole > 0 && gg.hole < 19) {
                    text.draw(g, &uo, x0 + 2.0, y, "Green fees come in as rounds finish", Size::Small, [0.6, 0.66, 0.6, 1.0]);
                    y += sl;
                }
                y += 3.0;
                let on = app.club.g.iter().filter(|gg| gg.hole > 0 && gg.hole < 19).count();
                let waiting = app.club.g.iter().filter(|gg| gg.hole == -1).count();
                pair(g, y, "Golfers", &format!("{} playing, {} waiting", on, waiting), WHITE);
                y += sl;
                if on > 0 {
                    let m: i32 = app.club.g.iter().filter(|gg| gg.hole > 0 && gg.hole < 19).map(|gg| gg.mood).sum::<i32>() / on as i32;
                    let (w, c) = mood_word(m);
                    pair(g, y, "Their mood", w, c);
                    y += sl;
                }
                if app.club.ratings.waitlist > 0 {
                    pair(g, y, "Membership waiting list", &plural(app.club.ratings.waitlist as usize, "person", "people"), WHITE);
                    y += sl;
                }
                y += 3.0;
                // the game's message, the golfers' thoughts (H = the hole they are on), a tool's note: newest at the foot
                let mut foot: Vec<(String, [f32; 4])> = Vec::new();
                if app.ticker.showing && !app.ticker.text.is_empty() {
                    for l in text.wrap(&app.ticker.text.replace('\n', " "), Size::Small, cw - 4.0).into_iter().take(3) {
                        foot.push((l, [1.0, 0.9, 0.45, 1.0]));
                    }
                }
                for (s, c, at) in self.thoughts.iter() {
                    let age = now.duration_since(*at).as_secs_f32() / THOUGHT_FOR.as_secs_f32();
                    let a = if age > 0.8 { (1.0 - age) * 5.0 } else { 1.0 };
                    for l in text.wrap(s, Size::Small, cw - 4.0).into_iter().take(2) {
                        foot.push((l, rgba([c[0], c[1], c[2]], a.clamp(0.25, 1.0))));
                    }
                }
                if let Some((s, _)) = &self.note {
                    for l in text.wrap(s, Size::Small, cw - 4.0) {
                        foot.push((l, WHITE));
                    }
                }
                let room = ((body + mh - y) / sl).floor().max(0.0) as usize;
                let n = foot.len();
                if n > 0 && room > 0 {
                    urect(g, x0, y - 2.0, cw, 1.0, [1.0, 1.0, 1.0, 0.15]);
                }
                for (i, (s, c)) in foot.into_iter().skip(n.saturating_sub(room)).enumerate() {
                    text.draw(g, &uo, x0 + 2.0, y + i as f32 * sl, &s, Size::Small, c);
                }
            }
        }

        // the inspect card, over the overview
        if let Some((lines, _)) = &self.card {
            let w = lines.iter().map(|l| text.width(l, Size::Normal)).fold(120.0, f32::max) + 16.0;
            let h = lines.len() as f32 * lh + 10.0;
            let w = w.min(W - 8.0);
            let (x, y) = (4.0, body + 2.0);
            let v = |x: f32, y: f32, c: [f32; 4]| Vert::new(x, y, 0.0, 0.0, 0.0).col(c);
            let card = [0.02, 0.06, 0.03, 0.94];
            g.quad(Mode::Flat, None, &uo, [v(x, y, card), v(x + w, y, card), v(x + w, y + h, card), v(x, y + h, card)]);
            let gl = [1.0, 0.9, 0.3, 1.0];
            g.quad(Mode::Flat, None, &uo, [v(x, y, gl), v(x + w, y, gl), v(x + w, y + 1.5, gl), v(x, y + 1.5, gl)]);
            for (i, l) in lines.iter().enumerate() {
                let c = if i == 0 { [1.0, 0.92, 0.5, 1.0] } else { [1.0, 1.0, 1.0, 1.0] };
                text.draw(g, &uo, x + 8.0, y + 5.0 + i as f32 * lh, l, Size::Normal, c);
            }
        }

        // Start's pause menu
        self.draw_menu_top(g, text);
    }

    /// A panel over the top screen below the bar, with a hint in its top-right corner (also the title screen's Controls).
    pub fn draw_panel(&self, g: &mut Gfx, text: &Text, app: &App, p: Panel, hint: &str) {
        let u = Uniforms::flat(&Mat4::ortho(0.0, W, H, 0.0, -1.0, 1.0), &Mat4::identity());
        self.draw_panel_at(g, text, app, p, hint, (8.0, BAR_H + 4.0, W - 16.0, H - BAR_H - 10.0), &u);
    }

    /// A page in (x, y, w, h), drawn with `u`.
    pub fn draw_panel_at(&self, g: &mut Gfx, text: &Text, app: &App, p: Panel, hint: &str, (x, y, w, h): (f32, f32, f32, f32), u: &Uniforms) {
        let u = *u;
        let rect = |g: &mut Gfx, x: f32, y: f32, w: f32, h: f32, c: [f32; 4]| {
            let v = |x: f32, y: f32| Vert::new(x, y, 0.0, 0.0, 0.0).col(c);
            g.quad(Mode::Flat, None, &u, [v(x, y), v(x + w, y), v(x + w, y + h), v(x, y + h)]);
        };
        {
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
            let hw = text.width(hint, Size::Small);
            text.draw(g, &u, x + w - hw - 8.0, y + 5.0, hint, Size::Small, [0.6, 0.65, 0.6, 1.0]);
        }
    }

    /// Start's pause menu on the top screen.
    fn draw_menu_top(&self, g: &mut Gfx, text: &Text) {
        let u = Uniforms::flat(&Mat4::ortho(0.0, W, H, 0.0, -1.0, 1.0), &Mat4::identity());
        let rect = |g: &mut Gfx, x: f32, y: f32, w: f32, h: f32, c: [f32; 4]| {
            let v = |x: f32, y: f32| Vert::new(x, y, 0.0, 0.0, 0.0).col(c);
            g.quad(Mode::Flat, None, &u, [v(x, y), v(x + w, y), v(x + w, y + h), v(x, y + h)]);
        };
        let lh = text.line_height(Size::Normal);
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
                    if r.par == 0 {
                        rows.push((format!("Hole {h}   not open: tap its green"), String::new(), [1.0, 0.85, 0.4, 1.0]));
                        continue;
                    }
                    // the Course Report's row: what kind of hole it plays as, how much fun, what it earns
                    let rep = sg_core::ratings::report_row(h, r, app.difficulty, false);
                    let right = if r.tee_shots > 0 {
                        format!("fun {}   fees {}", sg_core::ratings::fun_word(rep.fun).to_lowercase(), money(rep.revenue as i64))
                    } else {
                        "not played yet".to_string()
                    };
                    rows.push((format!("Hole {h}   par {}   {yards} yds   {}", r.par, rep.type_name), right, white));
                }
                if n == 0 {
                    rows.push(("No holes yet: HOLE (COURSE row), then tap a tee and a green on the course".into(), String::new(), dim));
                }
                let r = app.club.ratings;
                rows.push((String::new(), String::new(), white));
                let words = rating_words(app).map(|(f, c)| format!("   Fun: {f}   Challenge: {c}")).unwrap_or_default();
                rows.push((format!("Course par {}{words}", r.par), String::new(), dim));
                (format!("Holes ({n})"), rows)
            }
            Panel::Golfers => {
                let on: Vec<usize> = (0..SLOTS.min(app.club.g.len())).filter(|&gi| app.club.g[gi].hole > 0 && app.club.g[gi].hole < 19).collect();
                let waiting = app.club.g.iter().filter(|gg| gg.hole == -1).count();
                let avg = if on.is_empty() { 0 } else { on.iter().map(|&gi| app.club.g[gi].mood).sum::<i32>() / on.len() as i32 };
                rows.push((format!("{} on the course   {} waiting   mood: {}", on.len(), waiting, mood_word(avg).0.to_lowercase()), String::new(), dim));
                for &gi in on.iter().take(14) {
                    let gg = &app.club.g[gi];
                    let c = if gg.mood < 0 { [1.0, 0.65, 0.55, 1.0] } else { white };
                    rows.push((app.club.name(gi), format!("hole {}   {}", gg.hole, mood_word(gg.mood).0.to_lowercase()), c));
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
                rows.push(("STAFF row: choose + to hire one or - to fire one, then press A".into(), String::new(), dim));
                rows.push(("Groundskeepers keep the grass and weeds down; rangers keep play moving;".into(), String::new(), dim));
                rows.push(("soda vendors sell drinks; the club pro teaches.".into(), String::new(), dim));
                ("Staff".into(), rows)
            }
            Panel::Help => {
                for (l, r) in [
                    ("Choose a tool", "D-pad left / right"),
                    ("Change category", "L / R"),
                    ("Use it", "drag or tap on the course (bottom screen)"),
                    ("Brush size", "D-pad up / down"),
                    ("Look at something", "hold the stylus still until the ring fills"),
                    ("New hole", "HOLE: tap a tee, then a green"),
                    ("Hire, fire, save, a page", "choose it, then A"),
                    ("Scroll", "Circle Pad, C-stick, MOVE tool, or stroke into the edge"),
                    ("Zoom", "ZL (out) / ZR (in)"),
                    ("Undo (or clear a placed tee)", "B"),
                    ("Pause / game speed", "X / Y"),
                    ("Money, holes, golfers, staff pages", "Select"),
                    ("Menu: save, load, title screen, quit", "Start"),
                ] {
                    rows.push((l.to_string(), r.to_string(), white));
                }
                ("Controls".into(), rows)
            }
        }
    }
}
