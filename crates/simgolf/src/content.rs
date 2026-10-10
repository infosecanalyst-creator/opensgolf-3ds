//! Content packs in the game (docs/MODDING.md): loading the enabled packs at start up, and Preferences > Content, the screen
//! that lists the installed packs and lets the player switch them on and off and order them.
//!
//! Packs are found in `<save folder>/packs` and in `packs` next to the program (in the browser: the packs the player added
//! on the page, which keeps them under `packs/`). The player's choices are kept under "content" in settings.json. The
//! enabled packs are loaded once, when the game starts, before any art, sound or data is read; a change made on the
//! Content screen is saved at once and takes effect the next time the game starts.
//!
//! The screen is the port's own (the original had no packs): the System Functions box's frame and fonts, one row per pack.
use crate::app::*;
use crate::gfx::Gfx;
use crate::render::Rect;
use crate::ui::{rgb, Screen as Ui, F_ARIAL10, F_MANUAL15, F_MANUAL20};
use sg_core::packs::{self, Found, Kind, Plan, Settings, Status};
use std::path::{Path, PathBuf};

/// The settings.json key of the content choices.
const SETTINGS_KEY: &str = "content";

/// Rows shown at once.
const ROWS: usize = 8;
const ROW_H: f32 = 46.0;
const TOP: f32 = 96.0;
const BOX: Rect = Rect { x: 40.0, y: 36.0, w: 720.0, h: 528.0 };

#[derive(Default)]
pub struct ContentState {
    pub found: Found,
    /// The packs loaded when the game started (ids in load order, the base set first).
    pub loaded: Vec<String>,
    /// The player's choices as saved (they apply at the next start).
    pub settings: Settings,
    /// The load order those choices give.
    pub plan: Option<Plan>,
    /// Data problems of the loaded packs: (pack id or "", message).
    pub warnings: Vec<(String, String)>,
    /// First row shown, and the row picked (its description is shown).
    pub scroll: usize,
    pub picked: Option<usize>,
}

/// Where packs are looked for: the save folder's `packs`, then `packs` next to the program.
pub fn pack_dirs() -> Vec<PathBuf> {
    let saved = save_dir().join("packs");
    // in a browser there is no program folder: the page keeps the packs the player added under packs/
    let program = if cfg!(target_arch = "wasm32") {
        None
    } else {
        std::env::current_exe().ok().and_then(|e| e.parent().map(|p| p.join("packs"))).filter(|d| *d != saved)
    };
    std::iter::once(saved).chain(program).collect()
}

fn settings_path() -> PathBuf {
    save_dir().join(crate::hd::SETTINGS_FILE)
}

fn read_settings_json() -> serde_json::Value {
    sg_core::fsutil::read_file(settings_path())
        .and_then(|d| serde_json::from_slice::<serde_json::Value>(&d).ok())
        .filter(|v| v.is_object())
        .unwrap_or_else(|| serde_json::json!({}))
}

/// The player's content choices from settings.json (none saved: the original alone).
pub fn saved_settings() -> Settings {
    serde_json::from_value(read_settings_json()[SETTINGS_KEY].clone()).unwrap_or_default()
}

/// Saves the content choices into settings.json, keeping its other keys.
fn save_settings(s: &Settings) {
    let mut v = read_settings_json();
    if let Ok(c) = serde_json::to_value(s) {
        v[SETTINGS_KEY] = c;
    }
    if let Ok(t) = serde_json::to_string_pretty(&v) {
        sg_core::fsutil::write_file(settings_path(), t.as_bytes());
    }
}

/// Finds the packs, loads the enabled ones over the game folder at `game_dir`, and installs their data. Runs once, before
/// anything else of the game is read. With no packs enabled nothing is mounted and the built-in tables stay.
pub fn startup(game_dir: &Path) -> ContentState {
    let found = packs::discover(&pack_dirs());
    let settings = saved_settings().complete(&found);
    let mut plan = packs::resolve(&found, &settings);
    let loaded = packs::load(&found, &plan);
    let warnings = loaded.warnings.clone();
    packs::attach_warnings(&mut plan, &warnings);
    let ids = plan.active_ids();
    if ids.len() > 1 {
        println!("content packs: {}", ids.join(", "));
    }
    for r in &plan.rows {
        if let Status::Error(e) = &r.status {
            eprintln!("content pack {} ({}): {e}", r.manifest.name, r.location);
        }
    }
    for (id, w) in &warnings {
        eprintln!("content pack {}: {w}", if id.is_empty() { "data" } else { id });
    }
    packs::apply(game_dir, loaded);
    ContentState { found, loaded: ids, settings, plan: Some(plan), warnings, scroll: 0, picked: None }
}

/// What the screen does with a click.
enum Hit {
    Toggle(usize),
    Up(usize),
    Down(usize),
    Row(usize),
    More(bool),
    Done,
}

impl App {
    pub fn open_content(&mut self) {
        self.content.scroll = 0;
        self.content.picked = None;
        self.replan_content();
        self.screen = Screen::Content;
    }

    /// The plan the saved choices give, keeping the data warnings of the loaded packs on their rows.
    fn replan_content(&mut self) {
        let c = &mut self.content;
        let mut plan = packs::resolve(&c.found, &c.settings);
        packs::attach_warnings(&mut plan, &c.warnings);
        c.plan = Some(plan);
    }

    fn content_rows(&self) -> usize {
        self.content.plan.as_ref().map(|p| p.rows.len()).unwrap_or(0)
    }

    /// True when the saved choices load other packs (or another order) than the ones in use.
    fn content_changed(&self) -> bool {
        self.content.plan.as_ref().map(|p| p.active_ids() != self.content.loaded).unwrap_or(false)
    }

    fn content_hit(&self, x: f32, y: f32) -> Option<Hit> {
        if Rect::new(BOX.x + BOX.w - 70.0, BOX.y + BOX.h - 58.0, 50.0, 50.0).has(x, y) {
            return Some(Hit::Done);
        }
        let n = self.content_rows();
        if n > ROWS {
            if Rect::new(BOX.x + 24.0, TOP + ROWS as f32 * ROW_H + 4.0, 80.0, 20.0).has(x, y) {
                return Some(Hit::More(false));
            }
            if Rect::new(BOX.x + 110.0, TOP + ROWS as f32 * ROW_H + 4.0, 80.0, 20.0).has(x, y) {
                return Some(Hit::More(true));
            }
        }
        if y < TOP || !(BOX.x + 10.0..=BOX.x + BOX.w - 10.0).contains(&x) {
            return None;
        }
        let k = ((y - TOP) / ROW_H) as usize;
        let i = self.content.scroll + k;
        if k >= ROWS || i >= n {
            return None;
        }
        let ry = TOP + k as f32 * ROW_H;
        if Rect::new(BOX.x + 16.0, ry + 2.0, 30.0, 30.0).has(x, y) {
            return Some(Hit::Toggle(i));
        }
        if Rect::new(BOX.x + BOX.w - 112.0, ry + 4.0, 44.0, 20.0).has(x, y) {
            return Some(Hit::Up(i));
        }
        if Rect::new(BOX.x + BOX.w - 62.0, ry + 4.0, 44.0, 20.0).has(x, y) {
            return Some(Hit::Down(i));
        }
        Some(Hit::Row(i))
    }

    /// A click on the Content screen.
    pub fn content_click(&mut self, x: f32, y: f32) {
        match self.content_hit(x, y) {
            Some(Hit::Done) => self.close_content(),
            Some(Hit::Toggle(i)) => self.content_toggle(i),
            Some(Hit::Up(i)) => self.content_move(i, true),
            Some(Hit::Down(i)) => self.content_move(i, false),
            Some(Hit::Row(i)) => self.content.picked = Some(i),
            Some(Hit::More(down)) => self.content_scroll(down),
            None => {}
        }
    }

    /// Keys: Up and Down pick a row, Space or Enter switches it, PageUp and PageDown (with the picked row) move it in the
    /// order, Esc closes.
    pub fn content_key(&mut self, k: miniquad::KeyCode) {
        use miniquad::KeyCode;
        let n = self.content_rows();
        match k {
            KeyCode::Escape => self.close_content(),
            KeyCode::Up | KeyCode::Down if n > 0 => {
                let p = match (self.content.picked, k == KeyCode::Down) {
                    (None, _) => 0,
                    (Some(p), true) => (p + 1).min(n - 1),
                    (Some(p), false) => p.saturating_sub(1),
                };
                self.content.picked = Some(p);
                if p < self.content.scroll {
                    self.content.scroll = p;
                } else if p >= self.content.scroll + ROWS {
                    self.content.scroll = p + 1 - ROWS;
                }
            }
            KeyCode::Space | KeyCode::Enter | KeyCode::KpEnter => {
                if let Some(p) = self.content.picked {
                    self.content_toggle(p);
                }
            }
            KeyCode::PageUp | KeyCode::PageDown => {
                if let Some(p) = self.content.picked {
                    self.content_move(p, k == KeyCode::PageUp);
                }
            }
            _ => {}
        }
    }

    fn content_scroll(&mut self, down: bool) {
        let n = self.content_rows();
        let c = &mut self.content;
        c.scroll = if down { (c.scroll + ROWS).min(n.saturating_sub(ROWS)) } else { c.scroll.saturating_sub(ROWS) };
    }

    fn close_content(&mut self) {
        self.screen = Screen::Play;
        self.hover = -1;
    }

    /// Switches a row: a mod on or off; a base set or translation chosen (a translation chosen again: none).
    fn content_toggle(&mut self, i: usize) {
        let Some(row) = self.content.plan.as_ref().and_then(|p| p.rows.get(i)).cloned() else { return };
        let s = &mut self.content.settings;
        let id = row.manifest.id.clone();
        let ok = match row.manifest.kind {
            _ if row.pack.is_none() && id != packs::ORIGINAL => false, // a broken pack
            Kind::Base => {
                s.base = (id != packs::ORIGINAL).then_some(id);
                true
            }
            Kind::Mod => match s.mods.iter_mut().find(|m| m.id == id) {
                Some(m) => {
                    m.enabled = !m.enabled;
                    true
                }
                None => false,
            },
            Kind::Translation => {
                s.translation = if s.translation.as_deref() == Some(id.as_str()) { None } else { Some(id) };
                true
            }
            Kind::Scenario => false,
        };
        if !ok {
            self.ui_sound(0x18);
            return;
        }
        self.content.picked = Some(i);
        save_settings(&self.content.settings);
        self.replan_content();
    }

    /// Moves a mod up or down in the load order (only among the mods).
    fn content_move(&mut self, i: usize, up: bool) {
        let Some(row) = self.content.plan.as_ref().and_then(|p| p.rows.get(i)).cloned() else { return };
        let mods = &mut self.content.settings.mods;
        let Some(k) = mods.iter().position(|m| m.id == row.manifest.id).filter(|_| row.manifest.kind == Kind::Mod) else {
            self.ui_sound(0x18);
            return;
        };
        let to = if up { k.checked_sub(1) } else { (k + 1 < mods.len()).then_some(k + 1) };
        let Some(to) = to else {
            self.ui_sound(0x18);
            return;
        };
        mods.swap(k, to);
        save_settings(&self.content.settings);
        self.replan_content();
        // keep the moved pack picked
        let ni = self.content.plan.as_ref().and_then(|p| p.rows.iter().position(|r| r.manifest.id == row.manifest.id));
        self.content.picked = ni;
        if let Some(ni) = ni {
            if ni < self.content.scroll {
                self.content.scroll = ni;
            } else if ni >= self.content.scroll + ROWS {
                self.content.scroll = ni + 1 - ROWS;
            }
        }
    }

    pub fn draw_content(&mut self, g: &mut Gfx) {
        use crate::info_ui::c15;
        let s = Ui::new(self.draw_w, self.draw_h);
        self.view = s.view;
        crate::info_ui::dim(g, &s);
        self.draw_info_frame(g, &s, BOX.x, BOX.y, BOX.w, BOX.h);
        let (white, grey, yellow) = (c15(0x7fff), c15(0x6318), c15(0x7ff0));
        let (green, dark) = (c15(0x0180), c15(0x2108));
        let red = rgb(0.85, 0.1, 0.1);
        let navy = c15(0x0018);
        s.put_shadowed(g, F_MANUAL20, 400.0, BOX.y + 14.0, "Content packs", yellow, true);
        s.put(
            g,
            F_ARIAL10,
            BOX.x + 24.0,
            BOX.y + 40.0,
            "Base set first, then the mods in this order (later ones win), then the translation.",
            navy,
        );
        let Some(plan) = self.content.plan.clone() else { return };
        let (px, py) = self.info.pointer;
        let changed = self.content_changed();
        for (k, row) in plan.rows.iter().enumerate().skip(self.content.scroll).take(ROWS) {
            let y = TOP + (k - self.content.scroll) as f32 * ROW_H;
            let picked = self.content.picked == Some(k);
            if picked || Rect::new(BOX.x + 10.0, y, BOX.w - 20.0, ROW_H - 2.0).has(px, py) {
                s.fill(g, BOX.x + 12.0, y, BOX.w - 24.0, ROW_H - 2.0, rgb(0.66, 0.66, 0.84));
            }
            let m = &row.manifest;
            let on = row.status == Status::Active;
            // the InfoButtons check box: ticked when the pack loads (or is chosen), crossed otherwise
            if self.art.info_buttons.tex.is_some() {
                let cut = if on { 350.0 } else { 400.0 };
                s.image_part(g, &self.art.info_buttons, BOX.x + 18.0, y + 4.0, cut, 0.0, 26.0, 26.0);
            }
            let title = if m.version == "?" { m.name.clone() } else { format!("{}  {}", m.name, m.version) };
            s.put(g, F_MANUAL15, BOX.x + 54.0, y + 4.0, &title, if row.pack.is_some() || m.id == packs::ORIGINAL { white } else { grey });
            let who = if m.authors.is_empty() { String::new() } else { format!(" by {}", m.authors.join(", ")) };
            let lic = if m.license.is_empty() { String::new() } else { format!(", {}", m.license) };
            let loaded_now = self.content.loaded.contains(&m.id);
            let (state, colour) = match &row.status {
                Status::Active if !loaded_now => ("on after a restart".to_string(), green),
                Status::Active => ("on".to_string(), green),
                Status::Off if loaded_now => ("off after a restart".to_string(), dark),
                Status::Off => ("off".to_string(), dark),
                Status::Error(e) => (format!("error: {e}"), red),
                Status::NotLoaded(e) => (e.clone(), dark),
            };
            let kind = if row.pack.is_none() && m.id != packs::ORIGINAL { "?" } else { m.kind.name() };
            let line = format!("{kind}{who}{lic}  -  {state}");
            s.put(g, F_ARIAL10, BOX.x + 54.0, y + 24.0, &fit(&line, BOX.w - 190.0), colour);
            if let Some(w) = row.warnings.first() {
                let more = if row.warnings.len() > 1 { format!(" (+{} more)", row.warnings.len() - 1) } else { String::new() };
                s.put(g, F_ARIAL10, BOX.x + 54.0, y + 34.0, &fit(&format!("warning: {w}{more}"), BOX.w - 190.0), c15(0x5200));
            }
            if m.kind == Kind::Mod && row.pack.is_some() {
                for (dx, label) in [(BOX.w - 112.0, "Up"), (BOX.w - 62.0, "Down")] {
                    let r = Rect::new(BOX.x + dx, y + 4.0, 44.0, 20.0);
                    s.fill(g, r.x, r.y, r.w, r.h, if r.has(px, py) { rgb(0.85, 0.85, 1.0) } else { rgb(0.47, 0.47, 0.70) });
                    s.put_centered(g, F_ARIAL10, r.x + r.w / 2.0, r.y + 4.0, label, navy);
                }
            }
        }
        let n = plan.rows.len();
        let below = TOP + ROWS as f32 * ROW_H;
        if n > ROWS {
            for (dx, label, ok) in [(24.0, "Previous", self.content.scroll > 0), (110.0, "Next", self.content.scroll + ROWS < n)] {
                s.fill(g, BOX.x + dx, below + 4.0, 80.0, 20.0, rgb(0.47, 0.47, 0.70));
                s.put_centered(g, F_ARIAL10, BOX.x + dx + 40.0, below + 8.0, label, if ok { navy } else { grey });
            }
        }
        // the picked pack's description and where it is; the restart note
        let mut ty = below + 30.0;
        if let Some(r) = self.content.picked.and_then(|k| plan.rows.get(k)) {
            let text = if r.manifest.description.is_empty() {
                r.location.clone()
            } else {
                format!("{}  ({})", r.manifest.description, r.location)
            };
            for l in crate::ui::wrap_text(&text, 10.0, BOX.w - 120.0).into_iter().take(3) {
                s.put(g, F_ARIAL10, BOX.x + 24.0, ty, &l, navy);
                ty += 12.0;
            }
        }
        let note = if changed {
            "Your changes are saved. They take effect when you restart the game."
        } else {
            "Changes are saved at once and take effect when you restart the game."
        };
        s.put(g, F_ARIAL10, BOX.x + 24.0, BOX.y + BOX.h - 26.0, note, if changed { red } else { navy });
        if self.art.trans.tex.is_some() {
            s.image_part(g, &self.art.trans, BOX.x + BOX.w - 70.0, BOX.y + BOX.h - 58.0, 350.0, 140.0, 50.0, 50.0);
        }
        g.flush();
    }
}

/// `t` cut to fit `w` pixels of the small font, with "..." when cut.
fn fit(t: &str, w: f32) -> String {
    if F_ARIAL10.width(t) <= w {
        return t.to_string();
    }
    let mut s: String = t.to_string();
    while !s.is_empty() && F_ARIAL10.width(&format!("{s}...")) > w {
        s.pop();
    }
    format!("{s}...")
}
