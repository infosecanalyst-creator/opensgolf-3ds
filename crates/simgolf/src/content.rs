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
use sg_core::lang::{tr, trf};
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
    /// Why the base set differs from the player's choice (no original files found), shown once at start up.
    pub notice: Option<String>,
    /// The base set used whatever the saved choice says (--base, or the free set when the game folder has no original
    /// files): the Content screen shows the order the next start will really load.
    pub forced_base: Option<String>,
    /// Find more: the content index and its downloads (None: the list of installed packs is shown).
    pub index: Option<IndexView>,
    /// The packs loaded at start up with their versions, as saves record them (content_profiles.rs).
    pub stamps: Vec<crate::content_profiles::PackStamp>,
    /// The profiles list while it is open, and the name box of a profile being saved (content_profiles.rs).
    pub profiles: Option<crate::content_profiles::ProfilesView>,
    pub profile_name: Option<String>,
    /// A saved game made with other packs, waiting for the player's answer; and "load it anyway" chosen.
    pub pending_load: Option<crate::content_profiles::PendingLoad>,
    pub load_anyway: bool,
}

impl ContentState {
    /// Looks for the packs again (after a download or an export), keeping the choices; the packs loaded stay as they are
    /// until the next start.
    pub fn rediscover(&mut self) {
        self.found = packs::discover(&pack_dirs());
        self.settings = saved_settings().complete(&self.found);
        let mut s = self.settings.clone();
        if s.base.is_none() {
            s.base = self.forced_base.clone();
        }
        let mut plan = packs::resolve(&self.found, &s);
        packs::attach_warnings(&mut plan, &self.warnings);
        self.plan = Some(plan);
    }
}

/// The settings.json key of the content index's address (docs/CONTENT_INDEX.md).
const INDEX_URL_KEY: &str = "content_index_url";

/// The community index (github.com/crspytopgn/opensgolf-content), used when settings.json does not name one.
pub const DEFAULT_INDEX_URL: &str = "https://raw.githubusercontent.com/crspytopgn/opensgolf-content/main/index.json";

/// Find more: the index as read, the packs it offers, and a download in progress.
#[derive(Default)]
pub struct IndexView {
    pub url: String,
    /// Fetching the index.
    pub loading: Option<crate::net::Request>,
    pub offers: Vec<sg_core::index::Offer>,
    /// The index's problems (entries left out) or why it could not be read.
    pub problems: Vec<String>,
    pub scroll: usize,
    pub picked: Option<usize>,
    /// The pack being downloaded (an index into `offers`) and its request.
    pub download: Option<(usize, crate::net::Request)>,
    /// The last thing that happened, for the status line (and whether it is an error).
    pub status: String,
    pub failed: bool,
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

/// Saves the content choices (content_profiles.rs loads a profile or a save's packs into them).
pub fn save_choices(s: &Settings) {
    save_settings(s);
}

/// Saves the base set chosen (None: the original files), keeping the other content choices; it applies from the next
/// start (gamefiles.rs restarts the game for it).
pub fn save_base(base: Option<&str>) {
    let mut s = saved_settings();
    s.base = base.map(str::to_string);
    save_settings(&s);
}

/// Finds the packs, loads the enabled ones over the game folder at `game_dir`, and installs their data. Runs once, before
/// anything else of the game is read. With no packs enabled nothing is mounted and the built-in tables stay.
///
/// `base` (the --base option) picks the base set for this run without saving it. With no base set chosen and no SimGolf
/// files in `game_dir`, the free base set (`opensgolf-free`) is used instead of the original, with a message.
pub fn startup(game_dir: &Path, base: Option<&str>) -> ContentState {
    let found = packs::discover(&pack_dirs());
    let saved = saved_settings().complete(&found);
    // the choices for this run: the saved ones, unless --base or a missing game folder overrides the base set (not saved)
    let mut settings = saved.clone();
    let mut notice = None;
    let mut forced_base = None;
    if let Some(b) = base {
        settings.base = (b != packs::ORIGINAL).then(|| b.to_string());
    } else if settings.base.is_none() && !has_original(game_dir) {
        settings.base = Some(packs::FREE.to_string());
        forced_base = settings.base.clone();
        let msg = trf("STR_CONTENT_NO_GAME_FILES", &[("DIR", &game_dir.display())]);
        println!("{msg}");
        notice = Some(msg);
    }
    let mut plan = packs::resolve(&found, &settings);
    let loaded = packs::load(&found, &plan);
    let warnings = loaded.warnings.clone();
    packs::attach_warnings(&mut plan, &warnings);
    let stamps = crate::content_profiles::stamps(&plan);
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
    ContentState {
        found,
        loaded: ids,
        settings: saved,
        plan: Some(plan),
        warnings,
        scroll: 0,
        picked: None,
        notice,
        forced_base,
        index: None,
        stamps,
        ..Default::default()
    }
}

/// True when `game_dir` holds the original game's files (its Interface and Flics folders), read before any pack is mounted.
pub fn has_original(game_dir: &Path) -> bool {
    ["Interface", "Flics"].iter().all(|d| sg_core::fsutil::is_dir(sg_core::fsutil::resolve(game_dir, d)))
}

/// What the screen does with a click.
enum Hit {
    Toggle(usize),
    Up(usize),
    Down(usize),
    Row(usize),
    More(bool),
    FindMore,
    Done,
}

/// The Find more... button (and, on the index, the Back to the list button).
fn find_more_button() -> Rect {
    Rect::new(BOX.x + BOX.w - 200.0, BOX.y + BOX.h - 50.0, 116.0, 22.0)
}

impl App {
    pub fn open_content(&mut self) {
        self.content.index = None;
        self.content.profiles = None;
        self.content.profile_name = None;
        self.content.scroll = 0;
        self.content.picked = None;
        self.replan_content();
        self.screen = Screen::Content;
    }

    /// The plan the saved choices give, keeping the data warnings of the loaded packs on their rows.
    fn replan_content(&mut self) {
        let c = &mut self.content;
        let mut s = c.settings.clone();
        if s.base.is_none() {
            s.base = c.forced_base.clone();
        }
        let mut plan = packs::resolve(&c.found, &s);
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
        if find_more_button().has(x, y) {
            return Some(Hit::FindMore);
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
        if self.content.index.is_some() {
            return self.index_click(x, y);
        }
        // profiles (content_profiles.rs)
        if self.content.profiles.is_some() || self.content.profile_name.is_some() {
            return self.profiles_click(x, y);
        }
        if self.profile_buttons_click(x, y) {
            return;
        }
        match self.content_hit(x, y) {
            Some(Hit::FindMore) => self.open_content_index(),
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
        if self.content.index.is_some() {
            return self.index_key(k);
        }
        if self.profile_name_key(k) {
            return;
        }
        if self.content.profiles.is_some() {
            return self.profiles_key(k);
        }
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
        if self.content.index.is_some() {
            self.index_poll();
            return self.draw_index(g);
        }
        if self.content.profiles.is_some() {
            self.draw_profiles(g);
            return self.draw_profile_name(g);
        }
        use crate::info_ui::c15;
        let s = Ui::new(self.draw_w, self.draw_h);
        self.view = s.view;
        crate::info_ui::dim(g, &s);
        self.draw_info_frame(g, &s, BOX.x, BOX.y, BOX.w, BOX.h);
        let (white, grey, yellow) = (c15(0x7fff), c15(0x6318), c15(0x7ff0));
        let (green, dark) = (c15(0x0180), c15(0x2108));
        let red = rgb(0.85, 0.1, 0.1);
        let navy = c15(0x0018);
        s.put_shadowed(g, F_MANUAL20, 400.0, BOX.y + 14.0, tr("STR_CONTENT_TITLE"), yellow, true);
        s.put(g, F_ARIAL10, BOX.x + 24.0, BOX.y + 40.0, tr("STR_CONTENT_HELP"), navy);
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
            let line = pack_line(m, &row.status, self.content.loaded.contains(&m.id), row.pack.is_none() && m.id != packs::ORIGINAL);
            let colour = match &row.status {
                Status::Active => green,
                Status::Error(_) => red,
                _ => dark,
            };
            s.put(g, F_ARIAL10, BOX.x + 54.0, y + 24.0, &fit(&line, BOX.w - 190.0), colour);
            // a problem with its data, else with its scripts this session (scenario_ui)
            let script_errors: Vec<&String> = self.scen.errors.iter().filter(|(id, _)| *id == m.id).map(|(_, e)| e).collect();
            if let Some(w) = row.warnings.first() {
                let more =
                    if row.warnings.len() > 1 { trf("STR_CONTENT_MORE", &[("N", &(row.warnings.len() - 1))]) } else { String::new() };
                let t = trf("STR_CONTENT_WARNING", &[("WARNING", &w), ("MORE", &more)]);
                s.put(g, F_ARIAL10, BOX.x + 54.0, y + 34.0, &fit(&t, BOX.w - 190.0), c15(0x5200));
            } else if let Some(e) = script_errors.last() {
                let more =
                    if script_errors.len() > 1 { trf("STR_CONTENT_MORE", &[("N", &(script_errors.len() - 1))]) } else { String::new() };
                let t = trf("STR_CONTENT_SCRIPT_ERROR", &[("ERROR", e), ("MORE", &more)]);
                s.put(g, F_ARIAL10, BOX.x + 54.0, y + 34.0, &fit(&t, BOX.w - 190.0), red);
            }
            if m.kind == Kind::Mod && row.pack.is_some() {
                for (dx, label) in [(BOX.w - 112.0, tr("STR_CONTENT_UP")), (BOX.w - 62.0, tr("STR_CONTENT_DOWN"))] {
                    let r = Rect::new(BOX.x + dx, y + 4.0, 44.0, 20.0);
                    crate::glass_ui::button_fill(g, &s, r.x, r.y, r.w, r.h, r.has(px, py));
                    s.put_centered(g, F_ARIAL10, r.x + r.w / 2.0, r.y + 4.0, label, navy);
                }
            }
        }
        let n = plan.rows.len();
        let below = TOP + ROWS as f32 * ROW_H;
        if n > ROWS {
            let more = [
                (24.0, tr("STR_CONTENT_PREVIOUS"), self.content.scroll > 0),
                (110.0, tr("STR_CONTENT_NEXT"), self.content.scroll + ROWS < n),
            ];
            for (dx, label, ok) in more {
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
        let note = if changed { tr("STR_CONTENT_NOTE_CHANGED") } else { tr("STR_CONTENT_NOTE") };
        s.put(g, F_ARIAL10, BOX.x + 24.0, BOX.y + BOX.h - 26.0, note, if changed { red } else { navy });
        let b = find_more_button();
        crate::glass_ui::button_fill(g, &s, b.x, b.y, b.w, b.h, b.has(px, py));
        s.put_centered(g, F_ARIAL10, b.x + b.w / 2.0, b.y + 5.0, tr("STR_CONTENT_FIND_MORE"), navy);
        // profiles (content_profiles.rs)
        for (b, label) in crate::content_profiles::profile_buttons().into_iter().zip(["STR_PROFILE_SAVE", "STR_PROFILE_LIST"]) {
            crate::glass_ui::button_fill(g, &s, b.x, b.y, b.w, b.h, b.has(px, py));
            s.put_centered(g, F_ARIAL10, b.x + b.w / 2.0, b.y + 4.0, tr(label), navy);
        }
        if self.art.trans.tex.is_some() {
            s.image_part(g, &self.art.trans, BOX.x + BOX.w - 70.0, BOX.y + BOX.h - 58.0, 350.0, 140.0, 50.0, 50.0);
        }
        g.flush();
        self.draw_profile_name(g);
    }
}

/// A pack's second line: its kind, authors and licence, and whether it is on.
fn pack_line(m: &packs::Manifest, status: &Status, loaded_now: bool, broken: bool) -> String {
    let who = if m.authors.is_empty() { String::new() } else { trf("STR_CONTENT_BY", &[("AUTHORS", &m.authors.join(", "))]) };
    let lic = if m.license.is_empty() { String::new() } else { trf("STR_CONTENT_LICENCE", &[("LICENCE", &m.license)]) };
    let state = match status {
        Status::Active if !loaded_now => tr("STR_CONTENT_ON_AFTER_RESTART").to_string(),
        Status::Active => tr("STR_CONTENT_ON").to_string(),
        Status::Off if loaded_now => tr("STR_CONTENT_OFF_AFTER_RESTART").to_string(),
        Status::Off => tr("STR_CONTENT_OFF").to_string(),
        Status::Error(e) => trf("STR_CONTENT_ERROR", &[("ERROR", e)]),
        Status::NotLoaded(_) if m.kind == Kind::Scenario => tr("STR_CONTENT_SCENARIO_CHOSEN").to_string(),
        Status::NotLoaded(e) => e.clone(),
    };
    let kind = if broken { "?" } else { kind_name(m.kind) };
    trf("STR_CONTENT_PACK_LINE", &[("KIND", &kind), ("BY", &who), ("LICENCE", &lic), ("STATE", &state)])
}

/// A pack kind's name.
fn kind_name(k: Kind) -> &'static str {
    match k {
        Kind::Base => tr("STR_CONTENT_KIND_BASE"),
        Kind::Mod => tr("STR_CONTENT_KIND_MOD"),
        Kind::Translation => tr("STR_CONTENT_KIND_TRANSLATION"),
        Kind::Scenario => tr("STR_CONTENT_KIND_SCENARIO"),
    }
}

/// `t` cut to fit `w` pixels of the small font, with "..." when cut.
pub fn fit(t: &str, w: f32) -> String {
    if F_ARIAL10.width(t) <= w {
        return t.to_string();
    }
    let mut s: String = t.to_string();
    while !s.is_empty() && F_ARIAL10.width(&format!("{s}...")) > w {
        s.pop();
    }
    format!("{s}...")
}

// ---- Find more: the content index (docs/MODDING.md 7, docs/CONTENT_INDEX.md) --------------------------------------------------

/// The address of the content index from settings.json (`content_index_url`): the community index when the key is
/// missing, none when it is set to "".
pub fn index_url() -> String {
    read_settings_json()[INDEX_URL_KEY].as_str().unwrap_or(DEFAULT_INDEX_URL).trim().to_string()
}

impl App {
    /// Preferences > Content > Find more: reads the content index (nothing to read when no address is set).
    pub fn open_content_index(&mut self) {
        let url = index_url();
        let mut v = IndexView { url: url.clone(), ..Default::default() };
        if url.is_empty() {
            v.status = tr("STR_INDEX_NO_URL").to_string();
        } else {
            v.status = tr("STR_INDEX_LOADING").to_string();
            v.loading = Some(crate::net::get(&url, 16 * 1024 * 1024));
        }
        self.content.index = Some(v);
        self.index_poll();
    }

    /// Looks at the requests in progress (each frame while the screen is open).
    pub fn index_poll(&mut self) {
        let Some(v) = self.content.index.as_mut() else { return };
        if let Some(r) = v.loading.as_mut().and_then(|r| r.poll()) {
            v.loading = None;
            match r.and_then(|d| sg_core::index::Index::parse(&d)) {
                Ok((idx, warnings)) => {
                    for w in &warnings {
                        eprintln!("content index: {w}");
                    }
                    v.offers = sg_core::index::offers(&idx, &self.content.found);
                    v.problems = warnings;
                    v.status = if v.offers.is_empty() {
                        tr("STR_INDEX_NOTHING_NEW").to_string()
                    } else {
                        trf("STR_INDEX_LISTED", &[("N", &v.offers.len())])
                    };
                    v.failed = false;
                }
                Err(e) => {
                    eprintln!("content index {}: {e}", v.url);
                    v.status = trf("STR_INDEX_FAILED", &[("ERROR", &e)]);
                    v.failed = true;
                }
            }
        }
        let done = v.download.as_mut().and_then(|(k, r)| r.poll().map(|r| (*k, r)));
        let Some((k, reply)) = done else { return };
        v.download = None;
        let Some(offer) = v.offers.get(k).cloned() else { return };
        let dir = save_dir().join("packs");
        let r = reply.and_then(|data| sg_core::index::install(&offer.entry, &data, &dir, &self.content.found));
        match r {
            Ok(path) => {
                println!("content index: installed {} {} as {}", offer.entry.id, offer.entry.version, path.display());
                self.content.rediscover();
                let v = self.content.index.as_mut().expect("open");
                v.offers.remove(k);
                v.picked = None;
                v.status = trf("STR_INDEX_INSTALLED", &[("PACK", &offer.entry.name), ("VERSION", &offer.entry.version)]);
                v.failed = false;
            }
            Err(e) => {
                eprintln!("content index: {}: {e}", offer.entry.id);
                let v = self.content.index.as_mut().expect("open");
                v.status = trf("STR_INDEX_DOWNLOAD_FAILED", &[("PACK", &offer.entry.name), ("ERROR", &e)]);
                v.failed = true;
            }
        }
    }

    /// Downloads offer `k` (one at a time).
    pub fn index_download(&mut self, k: usize) {
        let Some(v) = self.content.index.as_mut() else { return };
        if v.download.is_some() || v.loading.is_some() {
            return self.ui_sound(0x18);
        }
        let Some(o) = v.offers.get(k) else { return };
        let url = sg_core::index::resolve_url(&v.url, &o.entry.url);
        println!("content index: downloading {} from {url}", o.entry.id);
        v.status = trf("STR_INDEX_DOWNLOADING", &[("PACK", &o.entry.name)]);
        v.failed = false;
        v.download = Some((k, crate::net::get(&url, o.entry.size.min(sg_core::index::MAX_PACK_SIZE))));
        self.index_poll();
    }

    fn index_click(&mut self, x: f32, y: f32) {
        if Rect::new(BOX.x + BOX.w - 70.0, BOX.y + BOX.h - 58.0, 50.0, 50.0).has(x, y) {
            return self.close_content();
        }
        if find_more_button().has(x, y) {
            self.content.index = None;
            return;
        }
        let Some(v) = self.content.index.as_mut() else { return };
        let n = v.offers.len();
        if n > ROWS {
            if Rect::new(BOX.x + 24.0, TOP + ROWS as f32 * ROW_H + 4.0, 80.0, 20.0).has(x, y) {
                v.scroll = v.scroll.saturating_sub(ROWS);
                return;
            }
            if Rect::new(BOX.x + 110.0, TOP + ROWS as f32 * ROW_H + 4.0, 80.0, 20.0).has(x, y) {
                v.scroll = (v.scroll + ROWS).min(n.saturating_sub(ROWS));
                return;
            }
        }
        if y < TOP || !(BOX.x + 10.0..=BOX.x + BOX.w - 10.0).contains(&x) {
            return;
        }
        let k = ((y - TOP) / ROW_H) as usize;
        let i = v.scroll + k;
        if k >= ROWS || i >= n {
            return;
        }
        if Rect::new(BOX.x + BOX.w - 112.0, TOP + k as f32 * ROW_H + 4.0, 94.0, 20.0).has(x, y) {
            return self.index_download(i);
        }
        v.picked = Some(i);
    }

    fn index_key(&mut self, k: miniquad::KeyCode) {
        use miniquad::KeyCode;
        let Some(v) = self.content.index.as_mut() else { return };
        let n = v.offers.len();
        match k {
            KeyCode::Escape | KeyCode::Backspace => self.content.index = None,
            KeyCode::Up | KeyCode::Down if n > 0 => {
                let p = match (v.picked, k == KeyCode::Down) {
                    (None, _) => 0,
                    (Some(p), true) => (p + 1).min(n - 1),
                    (Some(p), false) => p.saturating_sub(1),
                };
                v.picked = Some(p);
                if p < v.scroll {
                    v.scroll = p;
                } else if p >= v.scroll + ROWS {
                    v.scroll = p + 1 - ROWS;
                }
            }
            KeyCode::Enter | KeyCode::KpEnter | KeyCode::Space => {
                if let Some(p) = v.picked {
                    self.index_download(p);
                }
            }
            _ => {}
        }
    }

    fn draw_index(&mut self, g: &mut Gfx) {
        use crate::info_ui::c15;
        let s = Ui::new(self.draw_w, self.draw_h);
        self.view = s.view;
        crate::info_ui::dim(g, &s);
        self.draw_info_frame(g, &s, BOX.x, BOX.y, BOX.w, BOX.h);
        let (white, grey, yellow, navy, green) = (c15(0x7fff), c15(0x6318), c15(0x7ff0), c15(0x0018), c15(0x0180));
        let red = rgb(0.85, 0.1, 0.1);
        let Some(v) = self.content.index.as_ref() else { return };
        s.put_shadowed(g, F_MANUAL20, 400.0, BOX.y + 14.0, tr("STR_INDEX_TITLE"), yellow, true);
        let source = if v.url.is_empty() { tr("STR_INDEX_SOURCE_NONE").to_string() } else { trf("STR_INDEX_SOURCE", &[("URL", &v.url)]) };
        s.put(g, F_ARIAL10, BOX.x + 24.0, BOX.y + 40.0, &fit(&source, BOX.w - 48.0), navy);
        let (px, py) = self.info.pointer;
        let busy = v.download.as_ref().map(|d| d.0);
        for (k, o) in v.offers.iter().enumerate().skip(v.scroll).take(ROWS) {
            let y = TOP + (k - v.scroll) as f32 * ROW_H;
            if v.picked == Some(k) || Rect::new(BOX.x + 10.0, y, BOX.w - 20.0, ROW_H - 2.0).has(px, py) {
                s.fill(g, BOX.x + 12.0, y, BOX.w - 24.0, ROW_H - 2.0, rgb(0.66, 0.66, 0.84));
            }
            let e = &o.entry;
            s.put(g, F_MANUAL15, BOX.x + 24.0, y + 4.0, &fit(&format!("{}  {}", e.name, e.version), BOX.w - 160.0), white);
            let who = if e.authors.is_empty() { String::new() } else { trf("STR_CONTENT_BY", &[("AUTHORS", &e.authors.join(", "))]) };
            let lic = trf("STR_CONTENT_LICENCE", &[("LICENCE", &e.licence)]);
            let state = match &o.installed {
                Some(old) => trf("STR_INDEX_UPDATE", &[("VERSION", old)]),
                None => tr("STR_INDEX_NEW").to_string(),
            };
            let size = trf("STR_INDEX_SIZE", &[("KB", &e.size.div_ceil(1024))]);
            let line = trf(
                "STR_INDEX_PACK_LINE",
                &[("KIND", &kind_name(e.kind)), ("BY", &who), ("LICENCE", &lic), ("STATE", &state), ("SIZE", &size)],
            );
            s.put(g, F_ARIAL10, BOX.x + 24.0, y + 24.0, &fit(&line, BOX.w - 160.0), green);
            let r = Rect::new(BOX.x + BOX.w - 112.0, y + 4.0, 94.0, 20.0);
            let label = if busy == Some(k) { tr("STR_INDEX_DOWNLOADING_BUTTON") } else { tr("STR_INDEX_DOWNLOAD") };
            crate::glass_ui::button_fill(g, &s, r.x, r.y, r.w, r.h, r.has(px, py));
            s.put_centered(g, F_ARIAL10, r.x + r.w / 2.0, r.y + 4.0, label, navy);
        }
        let n = v.offers.len();
        let below = TOP + ROWS as f32 * ROW_H;
        if n > ROWS {
            let more = [(24.0, tr("STR_CONTENT_PREVIOUS"), v.scroll > 0), (110.0, tr("STR_CONTENT_NEXT"), v.scroll + ROWS < n)];
            for (dx, label, ok) in more {
                s.fill(g, BOX.x + dx, below + 4.0, 80.0, 20.0, rgb(0.47, 0.47, 0.70));
                s.put_centered(g, F_ARIAL10, BOX.x + dx + 40.0, below + 8.0, label, if ok { navy } else { grey });
            }
        }
        // the picked pack's description, then what happened last and the index's problems
        let mut ty = below + 30.0;
        let mut lines: Vec<(String, [f32; 4])> = Vec::new();
        if let Some(o) = v.picked.and_then(|k| v.offers.get(k)) {
            lines.extend(crate::ui::wrap_text(&o.entry.description, 10.0, BOX.w - 120.0).into_iter().take(2).map(|l| (l, navy)));
        }
        lines.extend(
            crate::ui::wrap_text(&v.status, 10.0, BOX.w - 120.0).into_iter().take(3).map(|l| (l, if v.failed { red } else { navy })),
        );
        if let Some(p) = v.problems.first() {
            let more = if v.problems.len() > 1 { trf("STR_CONTENT_MORE", &[("N", &(v.problems.len() - 1))]) } else { String::new() };
            lines.push((trf("STR_INDEX_PROBLEM", &[("PROBLEM", p), ("MORE", &more)]), c15(0x5200)));
        }
        for (l, c) in lines.into_iter().take(5) {
            s.put(g, F_ARIAL10, BOX.x + 24.0, ty, &fit(&l, BOX.w - 230.0), c);
            ty += 12.0;
        }
        s.put(g, F_ARIAL10, BOX.x + 24.0, BOX.y + BOX.h - 26.0, tr("STR_CONTENT_NOTE"), navy);
        let b = find_more_button();
        crate::glass_ui::button_fill(g, &s, b.x, b.y, b.w, b.h, b.has(px, py));
        s.put_centered(g, F_ARIAL10, b.x + b.w / 2.0, b.y + 5.0, tr("STR_INDEX_BACK"), navy);
        if self.art.trans.tex.is_some() {
            s.image_part(g, &self.art.trans, BOX.x + BOX.w - 70.0, BOX.y + BOX.h - 58.0, 350.0, 140.0, 50.0, 50.0);
        }
        g.flush();
    }
}
