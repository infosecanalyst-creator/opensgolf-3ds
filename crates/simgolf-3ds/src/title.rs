//! The title screen: what the game opens on, and where Start's "Title screen" comes back to.
//!
//! The handheld convention rather than the arcade one: no "press Start" splash, straight to a menu, with the likeliest
//! choice (going on with the last game) chosen already. The top screen is for reading: the game's name over a slow flight
//! across a course, and a line about whatever is chosen below. The bottom screen is for choosing: big rows a fingertip or the
//! stylus can hit, the D-pad and A as well, B to go back.
//!
//!   * Continue      the newest save (the autosave written when the game is left, or any later one)
//!   * New Course    the plots of land on offer, with their size and price and what is left to build with
//!   * Load Game     every save, newest first
//!   * Settings      golfers' thoughts on the top screen, sound volume (kept on the card)
//!   * Controls      the controls page (hud.rs)
//!   * Manual        the instruction manual (manual.rs, run by main.rs)
//!   * Quit          back to the Homebrew Launcher
//!
//! `splash` draws the same look while the game's files are read at start-up, in place of the text console.

use crate::app::App;
use crate::gfx::{Eye, Gfx, Mat4, Mode, Uniforms, Vert};
use crate::hud::{month_text, Hud};
use crate::input::Event;
use crate::plan::Panel;
use crate::screens::Screen;
use crate::shim::{KeyCode, MouseButton, TouchPhase};
use crate::text::{money, Size, Text};
use std::path::PathBuf;
use std::time::Instant;

const W: f32 = 400.0;
const H: f32 = 240.0;
const NAME: &str = "OpenSGolf";
const TAGLINE: &str = "Build a golf club, on two screens";

const GOLD: [f32; 4] = [1.0, 0.86, 0.35, 1.0];
const WHITE: [f32; 4] = [1.0, 1.0, 1.0, 1.0];
const DIM: [f32; 4] = [0.72, 0.78, 0.72, 1.0];
const ROW: [f32; 4] = [0.12, 0.3, 0.16, 1.0];
const ROW_OFF: [f32; 4] = [0.1, 0.16, 0.11, 1.0];
const ROW_ON: [f32; 4] = [0.95, 0.8, 0.25, 1.0];
const BG_TOP: [u8; 3] = [0x14, 0x2a, 0x1a];
const BG_BOTTOM: [u8; 3] = [0x0c, 0x16, 0x0e];

/// What the player picked: a save to open, a plot to start a new course on, or leaving.
pub enum Choice {
    Open(PathBuf, String),
    New(usize, String),
    /// The instruction manual (main.rs opens it over the title).
    Manual,
    Quit,
}

#[derive(Clone, Copy, PartialEq, Debug)]
enum Page {
    Main,
    Plots,
    Saves,
    Settings,
    Controls,
}

#[derive(Clone, Copy, PartialEq, Debug)]
enum Item {
    Continue,
    New,
    Load,
    Settings,
    Controls,
    Manual,
    Quit,
}

struct SaveEntry {
    path: PathBuf,
    /// "Autosave", "Quick save", or the file's name.
    label: String,
    course: String,
    /// "Jul 2002   $45,210"
    detail: String,
    /// When it was written, as "Oct 25, 7:03 PM".
    when: String,
}

struct Plot {
    prop: usize,
    place: String,
    course: String,
    acres: i32,
    price: i64,
    left: i64,
}

/// The title screen's settings, kept on the card.
pub struct Settings {
    pub thoughts: bool,
    /// An index into VOLUMES.
    pub volume: usize,
}

const VOLUMES: [(&str, f32); 4] = [("Off", 0.0), ("Low", 0.3), ("Medium", 0.55), ("High", 0.8)];
const SETTINGS_FILE: &str = "sdmc:/3ds/opensgolf/settings.txt";

impl Settings {
    pub fn load() -> Settings {
        let mut s = Settings { thoughts: true, volume: 3 };
        if let Ok(t) = std::fs::read_to_string(SETTINGS_FILE) {
            for line in t.lines() {
                match line.split_once('=') {
                    Some(("thoughts", v)) => s.thoughts = v.trim() != "0",
                    Some(("volume", v)) => s.volume = v.trim().parse::<usize>().unwrap_or(3).min(VOLUMES.len() - 1),
                    _ => {}
                }
            }
        }
        s
    }

    pub fn save(&self) {
        let _ = std::fs::write(SETTINGS_FILE, format!("thoughts={}\nvolume={}\n", self.thoughts as u8, self.volume));
    }

    /// The mixer's master volume.
    pub fn master(&self) -> f32 {
        VOLUMES[self.volume].1
    }
}

/// One row of the bottom screen: a title, an optional second line, whether it can be picked.
struct Row {
    title: String,
    sub: Option<String>,
    enabled: bool,
}

pub struct Title {
    page: Page,
    sel: usize,
    saves: Vec<SaveEntry>,
    plots: Vec<Plot>,
    pub settings: Settings,
    /// A line in place of the chosen row's description (a save that would not open).
    note: Option<String>,
    t0: Instant,
    /// A game is being opened: its name, shown over everything for the frame drawn before the work starts.
    pub loading: Option<String>,
    /// The press began on this row (a tap picks it on release only if it ends on the same row).
    pressed: Option<usize>,
}

impl Title {
    pub fn new(app: &App) -> Title {
        let mut t = Title {
            page: Page::Main,
            sel: 0,
            saves: Vec::new(),
            plots: Vec::new(),
            settings: Settings::load(),
            note: None,
            t0: Instant::now(),
            loading: None,
            pressed: None,
        };
        t.refresh(app);
        t
    }

    /// Reads the saves and the plots on offer again (at start, and on coming back from a game), and goes to the main page.
    pub fn refresh(&mut self, app: &App) {
        self.saves = scan_saves();
        self.plots = plots(app);
        self.page = Page::Main;
        self.sel = 0;
        self.note = None;
        self.loading = None;
        self.pressed = None;
    }

    /// A save that would not open: back to the menu with the reason on the top screen.
    pub fn failed(&mut self, e: &str) {
        self.loading = None;
        self.note = Some(format!("That game would not open: {e}"));
    }

    /// Seconds since the title came up: the fly-over's clock.
    pub fn clock(&self) -> f32 {
        self.t0.elapsed().as_secs_f32()
    }

    fn items(&self) -> Vec<Item> {
        let mut v = Vec::new();
        if !self.saves.is_empty() {
            v.push(Item::Continue);
        }
        v.push(Item::New);
        if !self.saves.is_empty() {
            v.push(Item::Load);
        }
        v.extend([Item::Settings, Item::Controls, Item::Manual, Item::Quit]);
        v
    }

    fn rows(&self) -> Vec<Row> {
        let row = |t: &str| Row { title: t.to_string(), sub: None, enabled: true };
        let back = row("Back");
        match self.page {
            Page::Main => self
                .items()
                .into_iter()
                .map(|i| {
                    row(match i {
                        Item::Continue => "Continue",
                        Item::New => "New Course",
                        Item::Load => "Load Game",
                        Item::Settings => "Settings",
                        Item::Controls => "Controls",
                        Item::Manual => "Manual",
                        Item::Quit => "Quit",
                    })
                })
                .collect(),
            Page::Plots => {
                let mut v: Vec<Row> = self
                    .plots
                    .iter()
                    .map(|p| Row {
                        title: p.course.clone(),
                        sub: Some(format!("{} acres   {}", p.acres, money(p.price))),
                        enabled: p.left > 0,
                    })
                    .collect();
                v.push(back);
                v
            }
            Page::Saves => {
                let mut v: Vec<Row> = self
                    .saves
                    .iter()
                    .take(5)
                    .map(|s| Row { title: format!("{}: {}", s.label, s.course), sub: Some(format!("{}   {}", s.detail, s.when).trim_end().to_string()), enabled: true })
                    .collect();
                v.push(back);
                v
            }
            Page::Settings => vec![
                row(&format!("Golfer thoughts: {}", if self.settings.thoughts { "On" } else { "Off" })),
                row(&format!("Sound: {}", VOLUMES[self.settings.volume].0)),
                back,
            ],
            Page::Controls => vec![back],
        }
    }

    /// Two columns when a page has more than five choices (the plots), so each stays big enough for a fingertip.
    fn grid(&self, n: usize) -> bool {
        n > 6 && self.page != Page::Main
    }

    /// Each row's rectangle on the bottom screen (x, y, w, h). In a grid the last row (Back) spans both columns.
    fn layout(&self, n: usize) -> Vec<(f32, f32, f32, f32)> {
        let (top, bottom, x0, full) = (8.0, 232.0, 24.0, 272.0);
        if self.grid(n) {
            let cells = n - 1;
            let lines = cells.div_ceil(2) + 1;
            let pitch = ((bottom - top) / lines as f32).min(46.0);
            let mut v: Vec<_> = (0..cells)
                .map(|i| (x0 + (i % 2) as f32 * (full * 0.5), top + (i / 2) as f32 * pitch, full * 0.5 - 4.0, pitch - 5.0))
                .collect();
            v.push((x0, top + (lines - 1) as f32 * pitch, full, pitch - 5.0));
            v
        } else {
            let pitch = ((bottom - top) / n.max(1) as f32).min(44.0);
            let used = pitch * n as f32;
            let y0 = top + ((bottom - top) - used) * 0.5;
            (0..n).map(|i| (x0, y0 + i as f32 * pitch, full, pitch - 5.0)).collect()
        }
    }

    fn hit(&self, x: f32, y: f32) -> Option<usize> {
        let n = self.rows().len();
        self.layout(n).iter().position(|&(rx, ry, rw, rh)| x >= rx && x < rx + rw && y >= ry && y < ry + rh)
    }

    fn go(&mut self, page: Page) {
        self.page = page;
        self.sel = 0;
        self.note = None;
    }

    /// One input event. Returns what was picked, if anything.
    pub fn event(&mut self, ev: &Event, mixer: &sg_core::mixer::Mixer) -> Option<Choice> {
        if self.loading.is_some() {
            return None;
        }
        let n = self.rows().len();
        let cols: usize = if self.grid(n) { 2 } else { 1 };
        let moved = |sel: usize, d: i32| -> usize { (sel as i32 + d).rem_euclid(n as i32) as usize };
        let before = self.sel;
        let mut pick = None;
        match *ev {
            // up and down a line (a grid's line is two cells; Back is the last line, on its own)
            Event::KeyDown(KeyCode::Up, _) => {
                self.sel = if self.sel + 1 == n {
                    n.saturating_sub(2)
                } else if self.sel >= cols {
                    self.sel - cols
                } else {
                    n - 1
                }
            }
            Event::KeyDown(KeyCode::Down, _) => {
                self.sel = if self.sel + 1 == n {
                    0
                } else if self.sel + cols >= n - 1 {
                    n - 1
                } else {
                    self.sel + cols
                }
            }
            Event::KeyDown(KeyCode::Left, _) if cols > 1 => self.sel = moved(self.sel, -1),
            Event::KeyDown(KeyCode::Right, _) if cols > 1 => self.sel = moved(self.sel, 1),
            // A, or Start on the main page (where "press Start" is what people try first)
            Event::MouseDown(MouseButton::Left, _, _) => pick = Some(self.sel),
            Event::KeyDown(KeyCode::Escape, _) if self.page == Page::Main => pick = Some(self.sel),
            // B or Start: back a page
            Event::MouseDown(MouseButton::Right, _, _) | Event::KeyDown(KeyCode::Escape, _) => {
                if self.page != Page::Main {
                    mixer.play("Interface/Button2.wav", 0.6, false);
                    self.go(Page::Main);
                }
                return None;
            }
            Event::Pen(phase, x, y) => {
                let at = self.hit(x, y);
                match phase {
                    TouchPhase::Started => {
                        self.pressed = at;
                        if let Some(i) = at {
                            self.sel = i;
                        }
                    }
                    TouchPhase::Moved => {
                        if let Some(i) = at {
                            self.sel = i;
                        }
                    }
                    _ => {
                        if at.is_some() && at == self.pressed {
                            pick = at;
                        }
                        self.pressed = None;
                    }
                }
            }
            _ => {}
        }
        if self.sel != before && pick.is_none() {
            mixer.play("Interface/Button2.wav", 0.4, false);
            self.note = None;
        }
        let i = pick?;
        let rows = self.rows();
        if !rows.get(i).is_some_and(|r| r.enabled) {
            mixer.play("Interface/Wrong.wav", 0.7, false);
            self.note = Some("Not enough money to build on that plot".into());
            return None;
        }
        mixer.play("Interface/Button1.wav", 0.7, false);
        let last = rows.len() - 1;
        match self.page {
            Page::Main => match self.items()[i] {
                Item::Continue => {
                    let s = &self.saves[0];
                    return Some(Choice::Open(s.path.clone(), s.course.clone()));
                }
                Item::New => self.go(Page::Plots),
                Item::Load => self.go(Page::Saves),
                Item::Settings => self.go(Page::Settings),
                Item::Controls => self.go(Page::Controls),
                Item::Manual => return Some(Choice::Manual),
                Item::Quit => return Some(Choice::Quit),
            },
            Page::Plots if i < last => {
                let p = &self.plots[i];
                return Some(Choice::New(p.prop, p.course.clone()));
            }
            Page::Saves if i < last => {
                let s = &self.saves[i];
                return Some(Choice::Open(s.path.clone(), s.course.clone()));
            }
            Page::Settings if i == 0 => {
                self.settings.thoughts = !self.settings.thoughts;
                self.settings.save();
            }
            Page::Settings if i == 1 => {
                self.settings.volume = (self.settings.volume + 1) % VOLUMES.len();
                mixer.set_master(self.settings.master());
                self.settings.save();
                mixer.play("Interface/Button1.wav", 0.7, false);
            }
            // Back
            _ => {
                let from = self.page;
                self.go(Page::Main);
                // back on the row that led there
                if let Some(k) = self.items().iter().position(|&it| {
                    matches!((from, it), (Page::Plots, Item::New) | (Page::Saves, Item::Load) | (Page::Settings, Item::Settings) | (Page::Controls, Item::Controls))
                }) {
                    self.sel = k;
                }
            }
        }
        None
    }

    /// A line about the chosen row, for the top screen.
    fn describe(&self) -> String {
        if let Some(n) = &self.note {
            return n.clone();
        }
        let n = self.rows().len();
        let back = self.sel + 1 == n;
        match self.page {
            Page::Main => match self.items().get(self.sel) {
                Some(Item::Continue) => {
                    let s = &self.saves[0];
                    format!("Go on with {}: {}. {}{}.", s.course, s.detail, s.label, saved_when(&s.when))
                }
                Some(Item::New) => "Choose a plot of land and build a new course on it.".into(),
                Some(Item::Load) => format!("Choose from your {} saved games, newest first.", self.saves.len()),
                Some(Item::Settings) => "Golfers' thoughts on the top screen, and the sound volume.".into(),
                Some(Item::Controls) => "How to play with the stylus, a fingertip and the buttons.".into(),
                Some(Item::Manual) => "The instruction manual: building holes, staff, golfers, money and tips.".into(),
                Some(Item::Quit) => "Back to the Homebrew Launcher.".into(),
                None => String::new(),
            },
            Page::Plots if !back => {
                let p = &self.plots[self.sel];
                if p.left > 0 {
                    format!("{} ({}): {} acres for {}. That leaves {} to build with.", p.course, p.place, p.acres, money(p.price), money(p.left))
                } else {
                    format!("{} ({}): {} acres for {}, more than the club has.", p.course, p.place, p.acres, money(p.price))
                }
            }
            Page::Saves if !back => {
                let s = &self.saves[self.sel];
                format!("{}: {}, {}{}.", s.label, s.course, s.detail, saved_when(&s.when))
            }
            Page::Settings if self.sel == 0 => "Golfers say what they think of the course along the foot of the top screen.".into(),
            Page::Settings if self.sel == 1 => "Sound effects and the course's ambience. Tap to change.".into(),
            _ => "Back to the title.".into(),
        }
    }

    /// The top screen, over the fly-over (drawn first by main.rs).
    pub fn render_top(&self, g: &mut Gfx, text: &Text, app: &App, hud: &Hud) {
        let u = ui(W);
        if self.page == Page::Controls {
            hud.draw_panel(g, text, app, Panel::Help, "B: back");
            return;
        }
        // the name, on a dark band
        rect(g, &u, 0.0, 0.0, W, 60.0, [0.0, 0.05, 0.02, 0.62]);
        rect(g, &u, 0.0, 60.0, W, 1.5, [1.0, 0.86, 0.35, 0.8]);
        let nw = text.width(NAME, Size::Title);
        text.draw(g, &u, (W - nw) * 0.5 + 2.0, 8.0 + 2.0, NAME, Size::Title, [0.0, 0.0, 0.0, 0.6]);
        text.draw(g, &u, (W - nw) * 0.5, 8.0, NAME, Size::Title, GOLD);
        let tw = text.width(TAGLINE, Size::Normal);
        text.draw_shadowed(g, &u, (W - tw) * 0.5, 42.0, TAGLINE, Size::Normal, WHITE);

        {
            // the chosen row's line along the foot
            let lines = text.wrap(&self.describe(), Size::Normal, W - 24.0);
            let lh = text.line_height(Size::Normal);
            let top = H - lh * lines.len().max(1) as f32 - 10.0;
            rect(g, &u, 0.0, top - 4.0, W, H - top + 4.0, [0.0, 0.05, 0.02, 0.62]);
            let c = if self.note.is_some() { [1.0, 0.6, 0.5, 1.0] } else { WHITE };
            for (i, l) in lines.iter().enumerate() {
                text.draw_shadowed(g, &u, 12.0, top + i as f32 * lh, l, Size::Normal, c);
            }
        }

        if let Some(name) = &self.loading {
            rect(g, &u, 0.0, 61.5, W, H - 61.5, [0.0, 0.0, 0.0, 0.55]);
            let l = format!("Opening {name}...");
            let lw = text.width(&l, Size::Large);
            text.draw_shadowed(g, &u, (W - lw) * 0.5, 140.0, &l, Size::Large, WHITE);
        }
    }

    /// The bottom screen: the page's rows.
    pub fn render_bottom(&self, g: &mut Gfx, text: &Text) {
        let u = ui(320.0);
        let rows = self.rows();
        let lay = self.layout(rows.len());
        let grid = self.grid(rows.len());
        for (i, (row, &(x, y, w, h))) in rows.iter().zip(lay.iter()).enumerate() {
            let on = i == self.sel;
            rect(g, &u, x, y, w, h, if on { ROW_ON } else if row.enabled { ROW } else { ROW_OFF });
            rect(g, &u, x, y + h - 2.0, w, 2.0, [0.0, 0.0, 0.0, 0.35]);
            let c = if on {
                [0.08, 0.08, 0.05, 1.0]
            } else if row.enabled {
                WHITE
            } else {
                [0.5, 0.55, 0.5, 1.0]
            };
            match &row.sub {
                None => {
                    let size = if h >= 26.0 { Size::Large } else { Size::Normal };
                    let lw = text.width(&row.title, size);
                    let ly = y + (h - text.line_height(size)) * 0.5;
                    text.draw(g, &u, x + (w - lw) * 0.5, ly, &row.title, size, c);
                }
                Some(sub) => {
                    let fit = |s: &str, size: Size| text.wrap(s, size, w - 14.0).into_iter().next().unwrap_or_default();
                    let (t, s) = (fit(&row.title, Size::Normal), fit(sub, Size::Small));
                    let total = text.line_height(Size::Normal) + text.line_height(Size::Small);
                    let ly = y + (h - total) * 0.5;
                    let tx = if grid { x + 7.0 } else { x + 10.0 };
                    text.draw(g, &u, tx, ly, &t, Size::Normal, c);
                    let sc = if on { [0.2, 0.18, 0.08, 1.0] } else { DIM };
                    text.draw(g, &u, tx, ly + text.line_height(Size::Normal), &s, Size::Small, sc);
                }
            }
        }
    }
}

fn ui(w: f32) -> Uniforms {
    Uniforms::flat(&Mat4::ortho(0.0, w, H, 0.0, -1.0, 1.0), &Mat4::identity())
}

fn rect(g: &mut Gfx, u: &Uniforms, x: f32, y: f32, w: f32, h: f32, c: [f32; 4]) {
    let v = |x: f32, y: f32| Vert::new(x, y, 0.0, 0.0, 0.0).col(c);
    g.quad(Mode::Flat, None, u, [v(x, y), v(x + w, y), v(x + w, y + h), v(x, y + h)]);
}

/// One frame of the start-up screen: the name, what is being read, and the time so far.
pub fn splash(g: &mut Gfx, text: &Text, line: &str, secs: f32) {
    g.begin_frame();
    let eyes: &[Eye] = if g.slider() > 0.0 { &[Eye::Left, Eye::Right] } else { &[Eye::Left] };
    for &eye in eyes {
        g.begin_screen(Screen::Top, eye, BG_TOP);
        let u = ui(W);
        rect(g, &u, 0.0, 96.0, W, 1.5, [1.0, 0.86, 0.35, 0.8]);
        let nw = text.width(NAME, Size::Title);
        text.draw(g, &u, (W - nw) * 0.5, 52.0, NAME, Size::Title, GOLD);
        let tw = text.width(TAGLINE, Size::Normal);
        text.draw(g, &u, (W - tw) * 0.5, 106.0, TAGLINE, Size::Normal, WHITE);
    }
    g.begin_screen(Screen::Bottom, Eye::Left, BG_BOTTOM);
    let u = ui(320.0);
    let l = "Getting the course ready";
    let lw = text.width(l, Size::Large);
    text.draw(g, &u, (320.0 - lw) * 0.5, 92.0, l, Size::Large, WHITE);
    let lines = text.wrap(line, Size::Normal, 290.0);
    for (i, s) in lines.iter().take(2).enumerate() {
        let sw = text.width(s, Size::Normal);
        text.draw(g, &u, (320.0 - sw) * 0.5, 124.0 + i as f32 * 14.0, s, Size::Normal, DIM);
    }
    // a bar that fills over the usual start-up time, never quite to the end
    let f = 1.0 - (-secs / 6.0).exp();
    rect(g, &u, 60.0, 170.0, 200.0, 6.0, [0.18, 0.26, 0.2, 1.0]);
    rect(g, &u, 60.0, 170.0, 200.0 * f * 0.95, 6.0, GOLD);
    g.end_frame();
}

/// The saves in Saved Games, newest first.
fn scan_saves() -> Vec<SaveEntry> {
    let dir = crate::files_ui::saved_games_dir();
    let Ok(rd) = std::fs::read_dir(&dir) else { return Vec::new() };
    // (when written, a rank for when that is unknown: the latest autosave, then the quick save, then the rest)
    let mut v: Vec<(u64, u8, SaveEntry)> = Vec::new();
    for e in rd.flatten() {
        let path = e.path();
        if !path.extension().is_some_and(|x| x.eq_ignore_ascii_case("sgs")) {
            continue;
        }
        let secs = modified(&path);
        let Ok(s) = crate::app::read_save(&path) else { continue };
        let stem = path.file_stem().map(|s| s.to_string_lossy().to_string()).unwrap_or_default();
        let label = match stem.as_str() {
            "&Latest" => "Autosave".to_string(),
            "&Latest-2" => "Earlier autosave".to_string(),
            "&Latest-3" => "Oldest autosave".to_string(),
            "&Quick" => "Quick save".to_string(),
            "&QuitSave" => "End of career".to_string(),
            other => other.trim_start_matches('&').to_string(),
        };
        let detail = format!("{}   {}", month_text(s.econ.month_index() as i32), money(s.econ.cash.round() as i64));
        let rank = match stem.as_str() {
            "&Latest" => 3,
            "&Quick" => 2,
            _ => 1,
        };
        v.push((secs, rank, SaveEntry { path, label, course: s.course_name.clone(), detail, when: when_text(secs) }));
    }
    v.sort_by(|a, b| (b.0, b.1).cmp(&(a.0, a.1)));
    v.into_iter().map(|(_, _, e)| e).collect()
}

/// The Parkland plots on offer (the theme this port's ground textures are prepared for), cheapest first.
fn plots(app: &App) -> Vec<Plot> {
    let props = sg_core::data::properties();
    let funds = sg_core::mapsize::scale_money_at(app.course_size, sg_core::data::economy().start_funds as i64);
    let mut v: Vec<Plot> = (0..props.len())
        .filter(|&i| props[i].theme == 0)
        .map(|i| {
            let (acres, price) = app.offer_for(i);
            Plot { prop: i, place: props[i].name().to_string(), course: props[i].course().to_string(), acres, price: price as i64, left: funds - price as i64 }
        })
        .collect();
    v.sort_by_key(|p| p.price);
    v
}

/// ", saved Oct 25, 7:03 PM" (nothing when the time is not known).
fn saved_when(when: &str) -> String {
    if when.is_empty() {
        String::new()
    } else {
        format!(", saved {when}")
    }
}

/// When a file on the card was last written, in seconds since 1970 (0 if unknown). The standard library's file times are
/// not filled in on the 3DS; libctru asks the SD card's archive for the file's timestamp.
fn modified(path: &std::path::Path) -> u64 {
    if let Ok(c) = std::ffi::CString::new(path.to_string_lossy().as_bytes()) {
        let mut t: u64 = 0;
        let rc = unsafe { ctru_sys::archive_getmtime(c.as_ptr(), &mut t) };
        // an archive that keeps no times (an emulator's card) answers with the 3DS clock's start, 1 Jan 2000
        if rc >= 0 && t > 946_684_800 + 86_400 {
            return t;
        }
    }
    std::fs::metadata(path)
        .and_then(|m| m.modified())
        .ok()
        .and_then(|t| t.duration_since(std::time::UNIX_EPOCH).ok())
        .map(|d| d.as_secs())
        .unwrap_or(0)
}

/// The console's clock is local time kept as if it were UTC: "Oct 25, 7:03 PM".
fn when_text(secs: u64) -> String {
    if secs == 0 {
        return String::new();
    }
    const MONTHS: [&str; 12] = ["Jan", "Feb", "Mar", "Apr", "May", "Jun", "Jul", "Aug", "Sep", "Oct", "Nov", "Dec"];
    let days = (secs / 86400) as i64;
    let rem = secs % 86400;
    // days since 1970 to a civil date (H. Hinnant's algorithm)
    let z = days + 719468;
    let era = z.div_euclid(146097);
    let doe = z - era * 146097;
    let yoe = (doe - doe / 1460 + doe / 36524 - doe / 146096) / 365;
    let doy = doe - (365 * yoe + yoe / 4 - yoe / 100);
    let mp = (5 * doy + 2) / 153;
    let d = doy - (153 * mp + 2) / 5 + 1;
    let m = if mp < 10 { mp + 3 } else { mp - 9 };
    let (h, min) = (rem / 3600, rem % 3600 / 60);
    let (h12, ampm) = match h {
        0 => (12, "AM"),
        1..=11 => (h, "AM"),
        12 => (12, "PM"),
        _ => (h - 12, "PM"),
    };
    format!("{} {}, {}:{:02} {}", MONTHS[(m - 1) as usize], d, h12, min, ampm)
}
