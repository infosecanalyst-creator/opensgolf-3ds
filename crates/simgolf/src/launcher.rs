//! The launcher (the port's own, none of it in the original; docs/PLAYING.md "The launcher"): a screen of its own before
//! the title menu, in the look of the browser version's page (web/index.html: the sky masthead with the gold OpenSGolf
//! heading, the dark blue pills, the green tab bar, the panel, the glossy buttons and the yellow Buy Me a Coffee button).
//! Everything on it is drawn by the engine from shapes and the bundled free fonts (Fredoka and Nunito, SIL Open Font
//! License), so it looks the same with or without the player's SimGolf files and shows none of the original's art. Its
//! layout is its own, in units of about a point (`unit`), so it fills any window from a phone held sideways to 4K.
//!
//! It shows at every real start while *Show this at start* is ticked (settings.json `launcher_at_start`, on until the
//! player unticks it), and always at a new player's first start, where it takes the place of the "How would you like to
//! play?" box (welcome.rs, recorded as shown; that box still shows when the launcher does not). After an update it opens
//! on its News tab, in place of the What's new box. It never shows in scripted runs (--png, --time, --screen) nor over a
//! game being loaded. Preferences > More > Launcher... and the title menu's Launcher... button bring it back. Four tabs:
//!
//! * **Game files**: what is in use and what was found (gamefiles.rs), with "Use my SimGolf files" (the title button's
//!   picker: a folder, a .zip or the CD) and "Play with the free graphics", where this system offers them.
//! * **How to play**: the four presets of "How would you like to play?" (welcome.rs `CHOICES`), the one in use marked; a
//!   choice is put in use exactly as that box and Game options put it.
//! * **Display & sound**: a window or full screen and the window's size (desktop), Classic / Enhanced / HD graphics (HD
//!   where a pack is made), the text size, the sound and the language: the same settings as Preferences, saved the same way.
//! * **News & updates**: the version, its What's new highlights (welcome.rs, whatsnew.json), Check for updates (where
//!   update.rs has it), the changelog and the Buy Me a Coffee page (support.rs).
//!
//! Play (and Enter or Esc) goes on to the title menu, or back to the game it was opened over. Mouse, touch (a tap is a
//! click) and a controller (gamepad.rs moves the pointer, A clicks) all work its buttons.
//!
//! Test hooks (for stills; nothing is written to settings.json): --screen launcher or SG_LAUNCHER=1 show it;
//! SG_LAUNCHER_TAB=files|play|display|news picks the tab; SG_LAUNCHER_FILES=mine:N shows the game files as the player's
//! own with N sounds.
use crate::app::App;
use crate::extras_ui::Preset;
use crate::gamefiles::Action;
use crate::gfx::Gfx;
use crate::render::Rect;
use crate::ui::{Face, Fnt, Image, Screen as Ui};
use sg_core::freeset::paint::{mix, sd_poly, sd_rect, Canvas, Col};
use sg_core::lang::{tr, trf};
use std::cell::RefCell;
use std::collections::HashMap;

/// settings.json's key for the "Show this at start" tick (missing: on).
pub const SHOW_KEY: &str = "launcher_at_start";
/// settings.json's key for the desktop window: full screen or not, and its size (the launcher's Display & sound, F11).
#[cfg_attr(any(target_arch = "wasm32", target_os = "android"), allow(dead_code))]
pub const WINDOW_KEY: &str = "window";
/// The test hooks.
pub const FORCE_ENV: &str = "SG_LAUNCHER";
const TAB_ENV: &str = "SG_LAUNCHER_TAB";

/// The window sizes Display & sound steps through (desktop), in the system's points.
#[cfg_attr(any(target_arch = "wasm32", target_os = "android"), allow(dead_code))]
pub const SIZES: [(u32, u32); 5] = [(1024, 640), (1280, 800), (1440, 900), (1680, 1050), (1920, 1200)];

/// The tabs.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub enum Tab {
    Files,
    Play,
    Display,
    News,
}

impl Tab {
    pub const ALL: [Tab; 4] = [Tab::Files, Tab::Play, Tab::Display, Tab::News];

    fn key(self) -> &'static str {
        match self {
            Tab::Files => "STR_LAUNCHER_TAB_FILES",
            Tab::Play => "STR_LAUNCHER_TAB_PLAY",
            Tab::Display => "STR_LAUNCHER_TAB_DISPLAY",
            Tab::News => "STR_LAUNCHER_TAB_NEWS",
        }
    }

    /// The tab a test hook names.
    fn named(s: &str) -> Option<Tab> {
        Some(match s.trim().to_lowercase().as_str() {
            "files" => Tab::Files,
            "play" | "presets" => Tab::Play,
            "display" | "sound" => Tab::Display,
            "news" => Tab::News,
            _ => return None,
        })
    }

    /// The next (`step` 1) or the previous (-1) tab, round.
    pub fn step(self, step: i32) -> Tab {
        let i = Tab::ALL.iter().position(|&t| t == self).unwrap_or(0) as i32;
        Tab::ALL[(i + step).rem_euclid(Tab::ALL.len() as i32) as usize]
    }
}

/// The graphics Display & sound offers (Preferences > Graphics' ticks: HD, Enhanced, neither for Classic).
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub enum Look {
    Classic,
    Enhanced,
    Hd,
}

/// What a click on the launcher does.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Hit {
    Tab(Tab),
    Play,
    ShowAtStart,
    /// The masthead's What's new pill: the News tab.
    NewsPill,
    /// Buy Me a Coffee (support.rs).
    Coffee,
    /// The title button's action (gamefiles.rs: the picker, where to copy the files, the free graphics).
    Files(Action),
    /// In a browser the files are chosen on the page: back to it.
    BackToPage,
    /// A preset of "How would you like to play?" (welcome.rs `CHOICES`).
    Preset(usize),
    /// Game options... (the extras box).
    GameOptions,
    Fullscreen(bool),
    /// The next (1) or the previous (-1) window size.
    WindowSize(i32),
    Graphics(Look),
    TextSize(i32),
    Sound(bool),
    Language(i32),
    Update,
    Changelog,
}

/// The launcher's state (`App::launcher`).
#[derive(Clone)]
pub struct State {
    pub tab: Tab,
    pub hover: Option<Hit>,
    /// The "Show this at start" tick.
    pub at_start: bool,
    /// A test hook showed it: nothing is written.
    pub forced: bool,
    /// The last frame's buttons and the window pixels per unit, for the pointer.
    hits: Vec<(Rect, Hit)>,
    k: f32,
}

// ---- when it shows ------------------------------------------------------------------------------------------------------------

/// Whether the launcher shows at this start, from settings.json as it was when the game started: always when a test hook
/// asks (`forced`), never in a scripted run, always for a new player (no settings yet, or a first start closed before its
/// first box), else while "Show this at start" is ticked.
pub fn decide(at_start: &serde_json::Value, scripted: bool, forced: bool) -> bool {
    if forced {
        return true;
    }
    if scripted {
        return false;
    }
    new_player(at_start) || at_start[SHOW_KEY].as_bool() != Some(false)
}

/// A new player: no settings.json (or an empty one), or a first start closed before its first box (welcome.rs).
pub fn new_player(v: &serde_json::Value) -> bool {
    v.as_object().is_none_or(|o| o.is_empty()) || v[crate::welcome::WELCOME_KEY] == serde_json::Value::Bool(false)
}

/// A test hook (SG_LAUNCHER=1) asks for the launcher.
pub fn forced_by_env() -> bool {
    std::env::var_os(FORCE_ENV).is_some_and(|v| !v.is_empty() && v != "0")
}

fn settings_path() -> std::path::PathBuf {
    crate::app::save_dir().join(crate::hd::SETTINGS_FILE)
}

fn settings_json() -> serde_json::Value {
    sg_core::fsutil::read_file(settings_path())
        .and_then(|d| serde_json::from_slice::<serde_json::Value>(&d).ok())
        .filter(|v| v.is_object())
        .unwrap_or_else(|| serde_json::json!({}))
}

fn save(key: &str, val: serde_json::Value) {
    let mut v = settings_json();
    v[key] = val;
    if let Ok(t) = serde_json::to_string_pretty(&v) {
        sg_core::fsutil::write_file(settings_path(), t.as_bytes());
    }
}

/// The tick as saved (on unless unticked).
pub fn show_at_start() -> bool {
    settings_json()[SHOW_KEY].as_bool() != Some(false)
}

/// The desktop window as saved: its size in points (none: the default) and whether it is full screen.
#[cfg_attr(any(target_arch = "wasm32", target_os = "android"), allow(dead_code))]
pub fn saved_window() -> (Option<(u32, u32)>, bool) {
    window_of(&settings_json()[WINDOW_KEY])
}

/// The window setting `v` (settings.json's `window`) read: a size between 640 x 400 and 8K, and full screen.
#[cfg_attr(any(target_arch = "wasm32", target_os = "android"), allow(dead_code))]
pub fn window_of(v: &serde_json::Value) -> (Option<(u32, u32)>, bool) {
    let n = |k: &str| v[k].as_u64().filter(|&n| (400..=8192).contains(&n)).map(|n| n as u32);
    let size = n("width").zip(n("height")).filter(|&(w, _)| w >= 640);
    (size, v["fullscreen"].as_bool() == Some(true))
}

/// Saves the desktop window's full screen (F11, Alt+Enter and the launcher) and, when given, its size in points.
#[cfg_attr(any(target_arch = "wasm32", target_os = "android"), allow(dead_code))]
pub fn save_window(fullscreen: bool, size: Option<(u32, u32)>) {
    let mut w = settings_json()[WINDOW_KEY].clone();
    if !w.is_object() {
        w = serde_json::json!({});
    }
    w["fullscreen"] = fullscreen.into();
    if let Some((x, y)) = size {
        w["width"] = x.into();
        w["height"] = y.into();
    }
    save(WINDOW_KEY, w);
}

/// The size in `SIZES` after (`step` 1) or before (-1) a window `w` points wide; none past the ends.
#[cfg_attr(any(target_arch = "wasm32", target_os = "android"), allow(dead_code))]
pub fn next_size(w: f32, step: i32) -> Option<(u32, u32)> {
    if step > 0 {
        SIZES.iter().copied().find(|s| s.0 as f32 > w + 8.0)
    } else {
        SIZES.iter().rev().copied().find(|s| (s.0 as f32) < w - 8.0)
    }
}

/// `s` with its first letter a capital (a text size's name as Preferences words it in a line).
fn capital(s: &str) -> String {
    let mut c = s.chars();
    c.next().map(|f| f.to_uppercase().chain(c).collect()).unwrap_or_default()
}

// ---- what it shows ------------------------------------------------------------------------------------------------------------

/// Everything the launcher shows, read from the game (`App::launcher_model`), so its layout can be made and tested alone.
#[derive(Clone, Debug)]
pub struct Model {
    pub tab: Tab,
    pub hover: Option<Hit>,
    pub at_start: bool,
    /// The game files: the line in use, a line under it, and whether it is a problem.
    pub files_line: String,
    pub files_detail: String,
    pub files_problem: bool,
    /// The player's own files are in use.
    pub mine_in_use: bool,
    /// The buttons the two cards have (the title button's action where this system offers it).
    pub mine_action: Option<Action>,
    pub free_action: Option<Action>,
    /// The browser version (the files come from the page).
    pub browser: bool,
    pub preset: Preset,
    /// A window of its own (desktop): full screen and the window's size can be chosen.
    pub desktop: bool,
    pub fullscreen: bool,
    pub window: String,
    pub look: Look,
    /// An HD pack is there to switch on.
    pub hd: bool,
    pub enhanced_on: bool,
    pub text_size: String,
    pub sound: bool,
    pub language: String,
    pub languages: usize,
    pub version: String,
    pub news_title: String,
    pub highlights: Vec<(String, String)>,
    /// Check for updates (where the build has it): whether a check is under way.
    pub update: Option<bool>,
}

impl Model {
    /// A model for tests: every part shown.
    #[cfg(test)]
    pub fn sample() -> Model {
        Model {
            tab: Tab::Files,
            hover: None,
            at_start: true,
            files_line: "Free graphics: OpenSGolf's own art and sounds".into(),
            files_detail: "No SimGolf files were found on this computer.".into(),
            files_problem: false,
            mine_in_use: false,
            mine_action: Some(Action::Pick),
            free_action: None,
            browser: false,
            preset: Preset::Original,
            desktop: true,
            fullscreen: false,
            window: "1280 x 800".into(),
            look: Look::Classic,
            hd: true,
            enhanced_on: false,
            text_size: "Normal".into(),
            sound: true,
            language: "English".into(),
            languages: 3,
            version: "OpenSGolf 0.7.3".into(),
            news_title: "What's new in 0.7".into(),
            highlights: (0..6).map(|i| (format!("Highlight number {i}"), "A note about it, a line or so long at most.".into())).collect(),
            update: Some(false),
        }
    }
}

// ---- colours (web/index.html's light palette) ----------------------------------------------------------------------------------

/// A colour 0xRRGGBB for the engine (0..1).
const fn c(v: u32) -> [f32; 4] {
    [((v >> 16) & 255) as f32 / 255.0, ((v >> 8) & 255) as f32 / 255.0, (v & 255) as f32 / 255.0, 1.0]
}
const fn ca(v: u32, a: f32) -> [f32; 4] {
    let mut x = c(v);
    x[3] = a;
    x
}
/// A colour 0xRRGGBB for the painter (0..255).
fn pc(v: u32, a: f32) -> Col {
    [((v >> 16) & 255) as f32, ((v >> 8) & 255) as f32, (v & 255) as f32, a]
}

const INK: [f32; 4] = c(0x1c2a1f);
const INK_SOFT: [f32; 4] = c(0x3d4f41);
const GOLD: [f32; 4] = c(0xffd23f);
const CARD_HEAD: [f32; 4] = c(0x1d6b2f);
const WHITE: [f32; 4] = [1.0; 4];
const PANEL: u32 = 0xf5f9ee;

// ---- the baked pieces --------------------------------------------------------------------------------------------------------

/// Texels per unit in the baked pieces (crisp up to a 4x window).
const K: f32 = 4.0;

/// The pieces drawn from baked textures: the page's buttons, boxes and pills.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
enum Skin {
    /// The page's gel buttons: yellow (.btn), green (.btn.green), light blue (.theme) and Buy Me a Coffee (.btn.coffee).
    Gold,
    Green,
    Sky,
    Coffee,
    /// The masthead's links (.toplinks a) and its coffee link.
    Pill,
    CoffeePill,
    /// The tab bar's tabs (nav.tabs a), and the open one.
    Tab,
    TabOn,
    /// The masthead's sky (.mast), rounded at the top as the panel.
    Mast,
    /// The site panel (.site), a card (.card), the main card (.card.primary), the files summary (#summary and its .problem)
    /// and the news box (.news).
    Site,
    Card,
    Primary,
    Summary,
    Problem,
    News,
    /// A tick box, ticked, and a value field.
    Check,
    CheckOn,
    Field,
    /// The sounds tip (.card .tip).
    Tip,
    /// A round choice mark, and the one chosen.
    Radio,
    RadioOn,
}

/// How a piece is painted: a vertical gradient (stops 0..1 down it), its border, corner radius (negative: half the
/// height), the page's inset highlight, inner shade, drop shadow and outer ring.
struct Spec {
    stops: &'static [(f32, u32)],
    fill_a: f32,
    border: (u32, f32),
    bw: f32,
    r: f32,
    top: bool,
    hi: f32,
    lo: (u32, f32),
    shadow: (u32, f32, f32, f32),
    ring: (u32, f32, f32),
}

const NO_SHADOW: (u32, f32, f32, f32) = (0, 0.0, 0.0, 0.0);
const NO_RING: (u32, f32, f32) = (0, 0.0, 0.0);
const DROP: (u32, f32, f32, f32) = (0x000000, 0.22, 3.0, 2.0);

fn spec(k: Skin) -> Spec {
    let gel = |stops, border, hi, lo| Spec {
        stops,
        fill_a: 1.0,
        border: (border, 1.0),
        bw: 1.0,
        r: -1.0,
        top: false,
        hi,
        lo,
        shadow: DROP,
        ring: NO_RING,
    };
    let flat = |fill: &'static [(f32, u32)], border: u32, bw: f32, r: f32, ring| Spec {
        stops: fill,
        fill_a: 1.0,
        border: (border, 1.0),
        bw,
        r,
        top: false,
        hi: 0.0,
        lo: (0, 0.0),
        shadow: NO_SHADOW,
        ring,
    };
    match k {
        Skin::Gold => gel(&[(0.0, 0xfff6c4), (0.45, 0xffe066), (0.52, 0xffcc1f), (1.0, 0xffd84a)], 0xa87400, 0.9, (0x965a00, 0.25)),
        Skin::Green => {
            gel(&[(0.0, 0x8fdc98), (0.22, 0x3d9e4e), (0.5, 0x1f7532), (0.52, 0x18672a), (1.0, 0x1f7632)], 0x0e4a1c, 0.6, (0x000000, 0.25))
        }
        Skin::Sky => gel(&[(0.0, 0xffffff), (0.45, 0xe6eff9), (0.52, 0xc9daef), (1.0, 0xdde9f6)], 0x5d7fa8, 1.0, (0x285082, 0.22)),
        Skin::Coffee => gel(&[(0.0, 0xfff59e), (0.45, 0xffe600), (0.52, 0xffd500), (1.0, 0xffe233)], 0xa38c00, 0.9, (0x965a00, 0.22)),
        Skin::Pill => Spec { fill_a: 0.62, border: (0xffffff, 0.35), shadow: NO_SHADOW, ..flat(&[(0.0, 0x082450)], 0, 1.0, 5.0, NO_RING) },
        Skin::CoffeePill => flat(&[(0.0, 0xffdd00)], 0xb39b00, 1.0, 5.0, NO_RING),
        Skin::Tab => Spec {
            top: true,
            r: 8.0,
            hi: 0.45,
            ..gel(&[(0.0, 0x4fae5e), (0.5, 0x2b8a3c), (0.51, 0x237a33), (1.0, 0x2c8a3d)], 0x0f4a1c, 0.45, (0, 0.0))
        },
        Skin::TabOn => Spec { top: true, shadow: NO_SHADOW, ..flat(&[(0.0, PANEL)], 0x0f4a1c, 1.0, 8.0, NO_RING) },
        Skin::Mast => {
            Spec { top: true, bw: 0.0, ..flat(&[(0.0, 0x1b5cae), (0.55, 0x3a86d4), (1.0, 0x8cc4ee)], 0x1b5cae, 0.0, 13.0, NO_RING) }
        }
        Skin::Site => Spec { shadow: (0x05193c, 0.45, 22.0, 12.0), ..flat(&[(0.0, PANEL)], 0x0d3b6e, 1.0, 14.0, (0xffffff, 0.55, 4.0)) },
        Skin::Card => flat(&[(0.0, 0xffffff)], 0xb4c6ab, 1.0, 8.0, (0xc8d6bf, 1.0, 1.0)),
        Skin::Primary => {
            Spec { shadow: (0x143c1e, 0.18, 10.0, 5.0), ..flat(&[(0.0, 0xffffff)], 0x36b04a, 3.0, 14.0, (0x1d6b2f, 1.0, 1.0)) }
        }
        Skin::Summary => flat(&[(0.0, 0xffffff)], 0x36b04a, 3.0, 12.0, (0x1d6b2f, 1.0, 1.0)),
        Skin::Problem => flat(&[(0.0, 0xfff4c2)], 0xd89a00, 3.0, 12.0, (0x8a5a00, 1.0, 1.0)),
        Skin::News => flat(&[(0.0, 0xeef5fb)], 0x8fb3d6, 1.0, 10.0, (0xc8d6bf, 1.0, 1.0)),
        Skin::Check => Spec { hi: 0.0, shadow: NO_SHADOW, ..flat(&[(0.0, 0xffffff), (1.0, 0xe3e8ec)], 0x7d8fa3, 1.5, 4.0, NO_RING) },
        Skin::CheckOn => flat(&[(0.0, 0x2f8f44), (1.0, 0x1f7532)], 0x0e4a1c, 1.0, 4.0, NO_RING),
        Skin::Field => flat(&[(0.0, 0xffffff)], 0xb4c6ab, 1.0, 6.0, NO_RING),
        Skin::Radio => Spec { r: -1.0, ..spec(Skin::Check) },
        Skin::RadioOn => Spec { r: -1.0, ..spec(Skin::CheckOn) },
        Skin::Tip => Spec { hi: 0.5, ..flat(&[(0.0, 0xfff4c2)], 0xd89a00, 2.0, 10.0, NO_RING) },
    }
}

impl Spec {
    /// The room round the piece its shadow and ring take, in units.
    fn margin(&self) -> f32 {
        let s = if self.shadow.1 > 0.0 { self.shadow.2 + self.shadow.3.abs() } else { 0.0 };
        s.max(self.ring.2).ceil()
    }

    /// Its corner radius at height `h`.
    fn radius(&self, h: f32) -> f32 {
        if self.r < 0.0 {
            h / 2.0
        } else {
            self.r.min(h / 2.0)
        }
    }

    /// Baked with its gradient at its height: drawn three-sliced (its ends kept, its middle stretched).
    fn sliced3(&self) -> bool {
        self.stops.len() > 1 || self.r < 0.0 || self.top
    }

    /// The colour at `t` down the gradient.
    fn at(&self, t: f32) -> Col {
        let s = self.stops;
        let col = |v: u32| pc(v, 1.0);
        if t <= s[0].0 {
            return col(s[0].1);
        }
        for w in s.windows(2) {
            if t <= w[1].0 {
                let k = (t - w[0].0) / (w[1].0 - w[0].0).max(1e-4);
                return mix(col(w[0].1), col(w[1].1), k);
            }
        }
        col(s[s.len() - 1].1)
    }
}

/// Paints a piece `w` x `h` units (with its margin round it), lit (under the pointer: the page's brightness(1.06)).
fn bake(sp: &Spec, w: f32, h: f32, lit: bool) -> Canvas {
    let m = sp.margin();
    let (cw, ch) = (((w + 2.0 * m) * K).ceil() as usize, ((h + 2.0 * m) * K).ceil() as usize);
    let mut cv = Canvas::new(cw, ch, [0.0; 4]);
    let (x, y, bw, bh) = (m * K, m * K, w * K, h * K);
    let r = sp.radius(h) * K;
    let top = sp.top;
    // the shape, `inset` texels in from its edge (a piece rounded only at the top goes on below its bottom edge)
    let shape = move |px: f32, py: f32, inset: f32| -> f32 {
        if top {
            let d = sd_rect(px, py, x + inset, y + inset, bw - 2.0 * inset, bh + r * 2.0, (r - inset).max(0.0));
            d.max(py - (y + bh))
        } else {
            sd_rect(px, py, x + inset, y + inset, bw - 2.0 * inset, bh - 2.0 * inset, (r - inset).max(0.0))
        }
    };
    let all = (0.0, 0.0, cw as f32, ch as f32);
    let (sc, sa, blur, dy) = sp.shadow;
    if sa > 0.0 {
        let (blur, dy) = (blur * K, dy * K);
        cv.fill(
            all,
            |px, py| shape(px, py - dy, 0.0) - blur,
            |px, py| {
                let d = shape(px, py - dy, 0.0).max(0.0) / blur;
                let t = (1.0 - d).clamp(0.0, 1.0);
                pc(sc, sa * t * t)
            },
        );
    }
    let (rc, ra, rw) = sp.ring;
    if rw > 0.0 {
        let rw = rw * K;
        cv.fill(all, |px, py| (shape(px, py, 0.0) - rw).max(-shape(px, py, 0.0)), |_, _| pc(rc, ra));
    }
    let b = sp.bw * K;
    if b > 0.0 {
        cv.fill(all, |px, py| shape(px, py, 0.0).max(-shape(px, py, b)), |_, _| pc(sp.border.0, sp.border.1));
    }
    let white = pc(0xffffff, 1.0);
    let (lo, lo_a) = sp.lo;
    cv.fill(
        all,
        |px, py| shape(px, py, b),
        |_, py| {
            let t = ((py - y) / bh.max(1.0)).clamp(0.0, 1.0);
            let mut col = sp.at(t);
            if lit {
                col = mix(col, white, 0.2);
            }
            // the inset highlight under the top edge and the shade along the bottom (the page's box-shadow insets)
            if sp.hi > 0.0 && py - y - b < K * 1.2 {
                col = mix(col, white, sp.hi);
            }
            if lo_a > 0.0 && !top {
                let up = y + bh - b - py;
                if up < 3.0 * K {
                    col = mix(col, pc(lo, 1.0), lo_a * (1.0 - up / (3.0 * K)));
                }
            }
            col[3] = sp.fill_a;
            col
        },
    );
    cv
}

/// The pictures made from shapes: the page's clouds, hills, logo, backdrop pattern and the summary's ball.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
enum Art {
    /// A soft white blob (the masthead's clouds, the backdrop's glow).
    Blob,
    /// The dark round the window's edges (#backdrop::after).
    Vignette,
    /// The backdrop's dimples, a tile of 8 x 8 (16 x 14 units each).
    Dimples,
    /// The masthead's hills (.mast .hills, 800 x 56).
    Hills,
    /// The logo (the page's header svg, 64 x 64).
    Logo,
    /// The summary's green ball with a tick, and its yellow one with a "!".
    BallOk,
    BallProblem,
    /// The page's CD (.cd: a silver disc with a rainbow sheen and the port's own green label, no logos), 100 x 100.
    Cd,
    /// The page's framed picture (.shot), tilted: here the free graphics' own title picture (the port's art, not the
    /// original's), 244 x 174 with its shadow.
    Shot,
}

/// The size of an art piece in units and its texels per unit.
fn art_size(a: Art) -> (f32, f32, f32) {
    match a {
        Art::Blob => (64.0, 32.0, 2.0),
        Art::Vignette => (64.0, 48.0, 1.0),
        Art::Dimples => (128.0, 112.0, 2.0),
        Art::Hills => (800.0, 56.0, 2.0),
        Art::Logo => (64.0, 64.0, 4.0),
        Art::BallOk | Art::BallProblem => (30.0, 30.0, 4.0),
        Art::Cd => (108.0, 108.0, 3.0),
        Art::Shot => (244.0, 186.0, 3.0),
    }
}

/// Points along a cubic Bézier curve from `p0` (not included).
fn bezier(out: &mut Vec<(f32, f32)>, p0: (f32, f32), p1: (f32, f32), p2: (f32, f32), p3: (f32, f32)) {
    for i in 1..=24 {
        let t = i as f32 / 24.0;
        let u = 1.0 - t;
        let f = |a: f32, b: f32, c: f32, d: f32| u * u * u * a + 3.0 * u * u * t * b + 3.0 * u * t * t * c + t * t * t * d;
        out.push((f(p0.0, p1.0, p2.0, p3.0), f(p0.1, p1.1, p2.1, p3.1)));
    }
}

/// The free graphics' title picture (the port's own art, baked into the program), for the framed picture.
fn free_title() -> Option<sg_core::assets::Rgba> {
    sg_core::png::decode_png(&sg_core::freeset::read("gfx/Interface/TitleBASE.png")?)
}

/// `img` sampled at (x, y) in its pixels, smoothly.
fn sample(img: &sg_core::assets::Rgba, x: f32, y: f32) -> Col {
    let (w, h) = (img.w as i32, img.h as i32);
    let px = |x: i32, y: i32| {
        let o = ((y.clamp(0, h - 1) * w + x.clamp(0, w - 1)) * 4) as usize;
        [img.px[o] as f32, img.px[o + 1] as f32, img.px[o + 2] as f32, img.px[o + 3] as f32 / 255.0]
    };
    let (x, y) = (x - 0.5, y - 0.5);
    let (x0, y0) = (x.floor() as i32, y.floor() as i32);
    let (tx, ty) = (x - x0 as f32, y - y0 as f32);
    mix(mix(px(x0, y0), px(x0 + 1, y0), tx), mix(px(x0, y0 + 1), px(x0 + 1, y0 + 1), tx), ty)
}

fn paint_art(a: Art) -> Canvas {
    let (w, h, k) = art_size(a);
    let (cw, ch) = ((w * k) as usize, (h * k) as usize);
    let mut cv = Canvas::new(cw, ch, [0.0; 4]);
    let all = (0.0, 0.0, cw as f32, ch as f32);
    let (fw, fh) = (cw as f32, ch as f32);
    match a {
        Art::Blob => {
            // radial-gradient(white, transparent 70%)
            cv.fill(
                all,
                |_, _| -1.0,
                |px, py| {
                    let d = ((px / fw - 0.5) * 2.0).hypot((py / fh - 0.5) * 2.0) / 0.7;
                    let t = (1.0 - d).clamp(0.0, 1.0);
                    pc(0xffffff, t * t * (3.0 - 2.0 * t))
                },
            );
        }
        Art::Vignette => {
            // radial-gradient(ellipse 120% 90% at 50% 40%, transparent 55%, rgba(0, 0, 0, 0.35))
            cv.fill(
                all,
                |_, _| -1.0,
                |px, py| {
                    let d = ((px / fw - 0.5) / 0.6).hypot((py / fh - 0.4) / 0.45 * 0.5);
                    let t = ((d - 0.55) / 0.45).clamp(0.0, 1.0);
                    pc(0x000000, 0.35 * t)
                },
            );
        }
        Art::Dimples => {
            // two offset layers of a golf ball's dimples: a dark dot with a faint light ring
            for j in 0..16 {
                for i in 0..16 {
                    let (ox, oy) = if j % 2 == 0 { (0.0, 0.0) } else { (8.0, 7.0) };
                    let (cx, cy) = ((((i / 2) as f32) * 16.0 + ox + 7.2) * k, (((j / 2) as f32) * 14.0 + oy + 5.6) * k);
                    if i % 2 == 1 {
                        continue;
                    }
                    cv.fill(
                        (cx - 3.0 * k, cy - 3.0 * k, cx + 3.0 * k, cy + 3.0 * k),
                        |px, py| (px - cx).hypot(py - cy) - 1.4 * k,
                        |_, _| pc(0x00140a, 0.16),
                    );
                    cv.fill(
                        (cx - 3.0 * k, cy - 3.0 * k, cx + 3.0 * k, cy + 3.0 * k),
                        |px, py| ((px - cx).hypot(py - cy) - 2.1 * k).abs() - 0.2 * k,
                        |_, _| pc(0xffffff, 0.05),
                    );
                }
            }
        }
        Art::Hills => {
            let s = |x: f32, y: f32| (x * k, y * k);
            let mut back = vec![s(0.0, 30.0)];
            bezier(&mut back, s(0.0, 30.0), s(140.0, 6.0), s(260.0, 18.0), s(400.0, 24.0));
            bezier(&mut back, s(400.0, 24.0), s(540.0, 30.0), s(660.0, 8.0), s(800.0, 18.0));
            back.extend([s(800.0, 56.0), s(0.0, 56.0)]);
            cv.poly(&back, pc(0x7cbf86, 1.0));
            let mut front = vec![s(0.0, 40.0)];
            bezier(&mut front, s(0.0, 40.0), s(180.0, 22.0), s(360.0, 46.0), s(520.0, 38.0));
            bezier(&mut front, s(520.0, 38.0), s(640.0, 32.0), s(720.0, 26.0), s(800.0, 32.0));
            front.extend([s(800.0, 56.0), s(0.0, 56.0)]);
            // the mown stripes: 20 units wide, turned 30 degrees
            let (sn, cs) = (30f32.to_radians().sin(), 30f32.to_radians().cos());
            cv.fill(
                all,
                |px, py| sd_poly(px, py, &front),
                |px, py| {
                    let u = (px * cs + py * sn) / k;
                    if (u / 20.0).floor() as i64 % 2 == 0 {
                        pc(0x3f9e4e, 1.0)
                    } else {
                        pc(0x37944a, 1.0)
                    }
                },
            );
            cv.ellipse(300.0 * k, 44.0 * k, 46.0 * k, 5.0 * k, pc(0xf1dda2, 1.0));
            cv.rect(612.0 * k, 12.0 * k, 2.0 * k, 24.0 * k, pc(0xffffff, 1.0));
            cv.poly(&[s(614.0, 12.0), s(627.0, 16.5), s(614.0, 21.0)], pc(0xe8423b, 1.0));
        }
        Art::Logo => {
            let s = |x: f32, y: f32| (x * k, y * k);
            // drop-shadow(0 3px 4px rgba(0, 20, 50, 0.5))
            cv.fill(
                all,
                |px, py| (px - 32.0 * k).hypot(py - 35.0 * k) - 34.0 * k,
                |px, py| {
                    let d = ((px - 32.0 * k).hypot(py - 35.0 * k) - 30.0 * k).max(0.0) / (4.0 * k);
                    pc(0x001432, 0.5 * (1.0 - d).clamp(0.0, 1.0).powi(2))
                },
            );
            cv.circle(32.0 * k, 32.0 * k, 31.25 * k, pc(0xffffff, 1.0));
            cv.circle(32.0 * k, 32.0 * k, 28.75 * k, pc(0x2f8f4e, 1.0));
            cv.fill(
                all,
                |px, py| {
                    sg_core::freeset::paint::sd_ellipse(px, py, 32.0 * k, 47.0 * k, 20.0 * k, 7.0 * k)
                        .max((px - 32.0 * k).hypot(py - 32.0 * k) - 28.75 * k)
                },
                |_, _| pc(0x58c27a, 1.0),
            );
            cv.round_rect(30.5 * k, 11.0 * k, 3.0 * k, 36.0 * k, 1.5 * k, pc(0xffffff, 1.0));
            cv.poly(&[s(33.5, 11.0), s(50.5, 17.5), s(33.5, 24.0)], pc(0xe8423b, 1.0));
            cv.circle(22.0 * k, 46.0 * k, 5.5 * k, pc(0xffffff, 1.0));
            for (x, y) in [(20.5, 44.5), (23.5, 45.0), (21.5, 47.5)] {
                cv.circle(x * k, y * k, 0.9 * k, pc(0xd5dcd6, 1.0));
            }
        }
        Art::BallOk | Art::BallProblem => {
            let ok = a == Art::BallOk;
            let (lo, hi) = if ok { (0x1f7532, 0x8fdc98) } else { (0xf0b400, 0xfff3a8) };
            let r = 15.0 * k;
            cv.fill(
                all,
                |px, py| (px - r).hypot(py - r - 0.6 * k) - r + 0.4 * k,
                |_, py| pc(0x000000, 0.35 * ((py - r) / r).clamp(0.0, 1.0)),
            );
            cv.fill(
                all,
                |px, py| (px - r).hypot(py - r) - r + 1.0 * k,
                |px, py| {
                    let d = (px - 0.35 * 2.0 * r).hypot(py - 0.3 * 2.0 * r) / (0.7 * 2.0 * r);
                    mix(pc(hi, 1.0), pc(lo, 1.0), d.clamp(0.0, 1.0))
                },
            );
            if ok {
                let t = 2.2 * k;
                let pts = [(9.0, 15.5), (13.0, 19.5), (21.0, 10.5)].map(|(x, y)| (x * k, y * k));
                cv.line(pts[0].0, pts[0].1, pts[1].0, pts[1].1, t, pc(0xffffff, 1.0));
                cv.line(pts[1].0, pts[1].1, pts[2].0, pts[2].1, t, pc(0xffffff, 1.0));
            } else {
                cv.round_rect(13.6 * k, 7.0 * k, 2.8 * k, 11.0 * k, 1.4 * k, pc(0x2b1d00, 1.0));
                cv.circle(15.0 * k, 21.8 * k, 1.7 * k, pc(0x2b1d00, 1.0));
            }
        }
        Art::Cd => {
            // the page's disc (an svg 200 across, here 100 units with a margin of 4 for its shadow)
            let u = 0.5 * k;
            let (cx, cy) = (54.0 * k, 54.0 * k);
            let rr = 100.0 * u;
            let hole = 21.0 * u;
            let ring = |px: f32, py: f32| (px - cx).hypot(py - cy);
            cv.fill(
                all,
                |px, py| (ring(px, py - 3.0 * k) - rr - 3.0 * k).max(hole - ring(px, py)),
                |px, py| pc(0x000000, 0.45 * (1.0 - ((ring(px, py - 3.0 * k) - rr).max(0.0) / (3.0 * k))).clamp(0.0, 1.0)),
            );
            // the silver, brushed round (a conic gradient), the clear hub and its ring, the hole cut out
            let silver = [0xf7f9fa, 0xb4bfc6, 0xeef2f4, 0x9fabb3, 0xf9fbfc, 0xaeb9c0, 0xe9eef1, 0xa5b0b8, 0xf7f9fa];
            cv.fill(
                all,
                |px, py| (ring(px, py) - rr).max(hole - ring(px, py)),
                |px, py| {
                    let t = ((px - cx).atan2(-(py - cy)) / std::f32::consts::TAU).rem_euclid(1.0) * 8.0;
                    let i = (t.floor() as usize).min(7);
                    let mut col = mix(pc(silver[i], 1.0), pc(silver[i + 1], 1.0), t - i as f32);
                    let d = ring(px, py) / rr;
                    if d < 0.42 {
                        col = mix(col, pc(0xe6f0f6, 1.0), 0.55);
                    }
                    if (0.43..0.446).contains(&d) {
                        col = mix(col, pc(0xffffff, 1.0), 0.75);
                    }
                    if d > 0.98 {
                        col = mix(col, pc(0x78828c, 1.0), 0.9);
                    }
                    col
                },
            );
            // the rainbow on the bare silver of the rim and the hub
            let rainbow = [0xff4fa8, 0xffd23a, 0x4ff59a, 0x3fb2ff, 0xb46bff];
            cv.fill(
                all,
                |px, py| {
                    let d = ring(px, py);
                    (d - rr * 0.98).max(rr * 0.775 - d).min((d - rr * 0.21).max(rr * 0.16 - d)).max(hole - d)
                },
                |px, py| {
                    let a = ((px - cx).atan2(-(py - cy)).to_degrees() - 290.0).rem_euclid(180.0);
                    if a > 100.0 {
                        return [0.0; 4];
                    }
                    let t = a / 100.0 * 4.0;
                    let i = (t.floor() as usize).min(3);
                    let col = mix(pc(rainbow[i], 1.0), pc(rainbow[i + 1], 1.0), t - i as f32);
                    [col[0], col[1], col[2], 0.45 * (1.0 - (a / 100.0 - 0.5).abs() * 1.6).clamp(0.0, 1.0)]
                },
            );
            // the printed label: a green ring with dark edges, a flag and a ball on it
            let (lx, ly, lr) = (cx + (84.8 - 100.0) * u, cy + (69.6 - 100.0) * u, 121.6 * u);
            cv.fill(all, |px, py| (ring(px, py) - 76.6 * u).max(43.4 * u - ring(px, py)), |_, _| pc(0x0e4a1c, 1.0));
            cv.fill(
                all,
                |px, py| (ring(px, py) - 76.0 * u).max(44.0 * u - ring(px, py)),
                |px, py| {
                    let t = (px - lx).hypot(py - ly) / lr;
                    if t < 0.6 {
                        mix(pc(0x3fae55, 1.0), pc(0x1f7532, 1.0), t / 0.6)
                    } else {
                        mix(pc(0x1f7532, 1.0), pc(0x145a24, 1.0), (t - 0.6) / 0.4)
                    }
                },
            );
            let p = |x: f32, y: f32| (cx + (x - 100.0) * u, cy + (y - 100.0) * u);
            let (ex, ey) = p(160.0, 110.0);
            cv.ellipse(ex, ey, 10.0 * u, 3.0 * u, pc(0x58c27a, 1.0));
            let (fx, fy) = p(159.0, 92.0);
            cv.rect(fx, fy, 2.0 * u, 18.0 * u, pc(0xffffff, 1.0));
            cv.poly(&[p(161.0, 92.0), p(171.0, 95.6), p(161.0, 99.2)], pc(0xe8423b, 1.0));
            let (ex, ey) = p(40.0, 106.0);
            cv.ellipse(ex, ey, 9.0 * u, 3.0 * u, pc(0x58c27a, 1.0));
            let (bx, by) = p(40.0, 101.0);
            cv.circle(bx, by, 5.0 * u, pc(0xffffff, 1.0));
            // the glint, still while the disc would turn
            cv.fill(
                all,
                |px, py| (ring(px, py) - rr).max(hole - ring(px, py)),
                |px, py| {
                    let t = ((px - cx) * 0.57 + (py - cy) * 0.82) / rr;
                    let g = (1.0 - ((t + 0.1) / 0.16).abs()).clamp(0.0, 1.0);
                    pc(0xffffff, 0.32 * g)
                },
            );
        }
        Art::Shot => {
            // the frame (4 units of the page's green, a white inner line, a dark outer one) round the picture, tilted 2.5
            // degrees, with its shadow; the picture covers it as the page's background-size: cover does
            let (fw0, fh0) = (220.0 * k, 150.0 * k);
            let (cx, cy) = (fw / 2.0, (fh - 8.0 * k) / 2.0);
            let (sn, cs) = (2.5f32.to_radians().sin(), 2.5f32.to_radians().cos());
            let local = move |px: f32, py: f32| {
                let (dx, dy) = (px - cx, py - cy);
                (dx * cs + dy * sn + fw0 / 2.0, -dx * sn + dy * cs + fh0 / 2.0)
            };
            let frame = move |px: f32, py: f32, inset: f32| {
                let (u, v) = local(px, py);
                sd_rect(u, v, inset, inset, fw0 - 2.0 * inset, fh0 - 2.0 * inset, (14.0 * k - inset).max(2.0))
            };
            cv.fill(
                all,
                |px, py| frame(px, py - 8.0 * k, 0.0) - 14.0 * k,
                |px, py| pc(0x001432, 0.4 * (1.0 - frame(px, py - 8.0 * k, 0.0).max(0.0) / (14.0 * k)).clamp(0.0, 1.0).powi(2)),
            );
            cv.fill(all, |px, py| frame(px, py, -k), |_, _| pc(0x1d6b2f, 1.0));
            cv.fill(all, |px, py| frame(px, py, 0.0), |_, _| pc(0x36b04a, 1.0));
            if let Some(img) = free_title() {
                // the course, the clubhouse and the pond of the title picture, clear of its buttons
                let (sx, sy, sw) = (0.0, 222.0, 230.0);
                let sh = sw * 142.0 / 212.0;
                cv.fill(
                    all,
                    |px, py| frame(px, py, 4.0 * k),
                    |px, py| {
                        let (u, v) = local(px, py);
                        let (fu, fv) = ((u - 4.0 * k) / (212.0 * k), (v - 4.0 * k) / (142.0 * k));
                        sample(&img, sx + fu * sw, sy + fv * sh)
                    },
                );
            }
            cv.fill(all, |px, py| frame(px, py, 4.0 * k).max(-frame(px, py, 5.0 * k)), |_, _| pc(0xffffff, 0.5));
        }
    }
    cv
}

thread_local! {
    /// The baked pieces made so far: the skins by (skin, height in quarter units, lit) and the pictures.
    static SKINS: RefCell<HashMap<(Skin, u32, bool), Image>> = RefCell::new(HashMap::new());
    static ARTS: RefCell<HashMap<Art, Image>> = RefCell::new(HashMap::new());
}

fn texture(g: &mut Gfx, cv: &Canvas) -> Image {
    let tex = g.texture_classic(&cv.to_rgba());
    // not pixel art: the effects' scaler leaves it alone
    crate::fx::plain_texture(tex);
    Image { tex: Some(tex), w: cv.w as f32, h: cv.h as f32 }
}

fn skin_image(g: &mut Gfx, k: Skin, h: f32, lit: bool) -> (Image, Spec) {
    let sp = spec(k);
    let key_h = if sp.sliced3() { (h * 4.0).round() as u32 } else { 0 };
    let im = SKINS.with(|m| {
        *m.borrow_mut().entry((k, key_h, lit)).or_insert_with(|| {
            let r = sp.radius(h);
            let cv = if sp.sliced3() { bake(&sp, 2.0 * r + 8.0, h, lit) } else { bake(&sp, 2.0 * r + 8.0, 2.0 * r + 8.0, lit) };
            texture(g, &cv)
        })
    });
    (im, sp)
}

fn art_image(g: &mut Gfx, a: Art) -> Image {
    ARTS.with(|m| *m.borrow_mut().entry(a).or_insert_with(|| texture(g, &paint_art(a))))
}

// ---- drawing and laying out at once ------------------------------------------------------------------------------------------

/// Where the launcher is drawn (none: it is only laid out, for the pointer and the tests).
pub struct Dr<'a> {
    pub g: &'a mut Gfx,
    pub s: &'a Ui,
}

/// One pass over the launcher: its buttons (`hits`), each piece's rectangle (`pieces`, for the tests) and whether anything
/// had to be left out for lack of room (`squeezed`).
pub struct Out<'a> {
    pub hits: Vec<(Rect, Hit)>,
    pub pieces: Vec<Rect>,
    pub squeezed: Vec<&'static str>,
    pub dr: Option<Dr<'a>>,
    hover: Option<Hit>,
}

impl<'a> Out<'a> {
    pub fn new(dr: Option<Dr<'a>>, hover: Option<Hit>) -> Out<'a> {
        Out { hits: Vec::new(), pieces: Vec::new(), squeezed: Vec::new(), dr, hover }
    }

    fn lit(&self, h: Hit) -> bool {
        self.hover == Some(h)
    }

    fn hit(&mut self, r: Rect, h: Hit) {
        self.hits.push((r, h));
    }

    fn squeeze(&mut self, what: &'static str) {
        self.squeezed.push(what);
    }

    fn skin(&mut self, k: Skin, r: Rect, lit: bool) {
        self.skin_tint(k, r, lit, WHITE);
    }

    /// A baked piece over `r` (its shadow and ring round it), tinted.
    fn skin_tint(&mut self, k: Skin, r: Rect, lit: bool, tint: [f32; 4]) {
        self.pieces.push(r);
        let Some(d) = self.dr.as_mut() else { return };
        if r.w <= 0.0 || r.h <= 0.0 {
            return;
        }
        let (im, sp) = skin_image(d.g, k, r.h, lit);
        let m = sp.margin();
        let rad = sp.radius(r.h);
        let (x, y, w, h) = (r.x - m, r.y - m, r.w + 2.0 * m, r.h + 2.0 * m);
        let e = (rad + m + 2.0).min(w / 2.0);
        let se = (rad + m + 2.0) * K;
        let s = d.s;
        if sp.sliced3() {
            let k2 = e / (rad + m + 2.0);
            s.image_scaled_tint(d.g, &im, (x, y, e, h), (0.0, 0.0, se * k2, im.h), tint);
            if w > 2.0 * e {
                s.image_scaled_tint(d.g, &im, (x + e, y, w - 2.0 * e, h), (se, 0.0, im.w - 2.0 * se, im.h), tint);
            }
            s.image_scaled_tint(d.g, &im, (x + w - e, y, e, h), (im.w - se * k2, 0.0, se * k2, im.h), tint);
        } else {
            let ev = (rad + m + 2.0).min(h / 2.0);
            let xs = [(x, e, 0.0, se), (x + e, w - 2.0 * e, se, im.w - 2.0 * se), (x + w - e, e, im.w - se, se)];
            let ys = [(y, ev, 0.0, se), (y + ev, h - 2.0 * ev, se, im.h - 2.0 * se), (y + h - ev, ev, im.h - se, se)];
            for &(dx, dw, sx, sw) in &xs {
                for &(dy, dh, sy, sh) in &ys {
                    if dw > 0.0 && dh > 0.0 {
                        s.image_scaled_tint(d.g, &im, (dx, dy, dw, dh), (sx, sy, sw, sh), tint);
                    }
                }
            }
        }
    }

    fn art(&mut self, a: Art, r: Rect, tint: [f32; 4]) {
        // (the soft clouds and glow may reach past the window: only the pictures that matter are kept for the tests)
        if a != Art::Blob {
            self.pieces.push(r);
        }
        let Some(d) = self.dr.as_mut() else { return };
        let im = art_image(d.g, a);
        d.s.image_scaled_tint(d.g, &im, (r.x, r.y, r.w, r.h), (0.0, 0.0, im.w, im.h), tint);
    }

    fn fill(&mut self, r: Rect, col: [f32; 4]) {
        if let Some(d) = self.dr.as_mut() {
            d.s.fill(d.g, r.x, r.y, r.w, r.h, col);
        }
    }

    fn gradient(&mut self, r: Rect, top: [f32; 4], bottom: [f32; 4]) {
        if let Some(d) = self.dr.as_mut() {
            d.s.gradient(d.g, r.x, r.y, r.w, r.h, top, bottom);
        }
    }

    fn tri(&mut self, p: [(f32, f32); 3], col: [f32; 4]) {
        if let Some(d) = self.dr.as_mut() {
            d.s.quad(d.g, [p[0], p[1], p[2], p[2]], col);
        }
    }

    fn line(&mut self, a: (f32, f32), b: (f32, f32), w: f32, col: [f32; 4]) {
        if let Some(d) = self.dr.as_mut() {
            d.s.line(d.g, a.0, a.1, b.0, b.1, w, col);
        }
    }

    /// Text with its top at y.
    fn text(&mut self, f: Fnt, x: f32, y: f32, s: &str, col: [f32; 4]) {
        if let Some(d) = self.dr.as_mut() {
            d.s.put(d.g, f, x, y, s, col);
        }
    }

    /// Text in a room `w` wide from x (smaller or cut when too long: ui.rs fit_line).
    fn text_fit(&mut self, f: Fnt, x: f32, y: f32, w: f32, s: &str, col: [f32; 4]) {
        if let Some(d) = self.dr.as_mut() {
            d.s.put_fit(d.g, f, x, y, w, s, col);
        }
    }

    /// Text centred on cx in a room `w` wide.
    fn text_center(&mut self, f: Fnt, cx: f32, y: f32, w: f32, s: &str, col: [f32; 4]) {
        if let Some(d) = self.dr.as_mut() {
            d.s.put_centered_fit(d.g, f, cx, y, w, s, col);
        }
    }

    /// Text with the page's shadows (a list of offsets and colours) under it.
    fn text_shadowed(&mut self, f: Fnt, x: f32, y: f32, s: &str, col: [f32; 4], shadows: &[(f32, f32, [f32; 4])]) {
        for &(dx, dy, sc) in shadows {
            self.text(f, x + dx, y + dy, s, sc);
        }
        self.text(f, x, y, s, col);
    }

    /// A note wrapped to `w` in at most `lines` lines (the last cut short when it has to be): the height it took.
    fn note(&mut self, f: Fnt, x: f32, y: f32, w: f32, lines: usize, s: &str, col: [f32; 4]) -> f32 {
        if lines == 0 || s.is_empty() {
            return 0.0;
        }
        let mut ls = crate::extras::wrap(f, s, w);
        let mut font = f;
        if ls.iter().any(|l| f.width(l) > w) {
            // a word wider than the room: ui.rs fit_box (a little smaller, then cut)
            if let Some(t) = crate::ui::fit_box(f, s, w, f.px + f.line() * (lines - 1) as f32) {
                font = t.font;
                ls = t.lines;
            }
        } else if ls.len() > lines {
            // more than there is room for: the size kept, the last line cut short
            self.squeeze("note cut");
            let rest = ls[lines - 1..].join(" ");
            ls.truncate(lines - 1);
            let mut t: Vec<char> = rest.chars().collect();
            loop {
                let cut: String = t.iter().collect::<String>().trim_end().to_string() + "\u{2026}";
                if t.is_empty() || f.width(&cut) <= w {
                    ls.push(cut);
                    break;
                }
                t.pop();
            }
        }
        let pitch = (font.px * 1.32).round();
        for (i, l) in ls.iter().take(lines).enumerate() {
            self.text(font, x, y + pitch * i as f32, l, col);
        }
        pitch * ls.len().min(lines) as f32
    }
}

fn body(px: f32) -> Fnt {
    Fnt { face: Face::Body, px }
}
fn display(px: f32) -> Fnt {
    Fnt { face: Face::Display, px }
}

/// The top of a line of `f` centred in a box from `y` `h` high.
fn mid(f: Fnt, y: f32, h: f32) -> f32 {
    (y + (h - f.px) / 2.0 - f.px * 0.08).round()
}

/// The window pixels one unit of the launcher's layout takes in a `w` x `h` drawable at `dpi` (the system's pixels per
/// point): about a point, so a phone's buttons are a finger wide, but at least 600 units high and 640 wide fit in.
pub fn unit(w: f32, h: f32, dpi: f32) -> f32 {
    dpi.max(1.0).max(h / 600.0).min(w / 640.0).max(0.25)
}

/// The launcher's sizes at a window `w` x `h` units.
#[derive(Clone, Copy)]
pub struct Frame {
    pub compact: bool,
    pub panel: Rect,
    pub mast: Rect,
    pub tabs: Rect,
    pub content: Rect,
    pub foot: Rect,
}

pub fn frame(w: f32, h: f32) -> Frame {
    let compact = h < 470.0;
    let (mx, my) = if compact { (6.0, 6.0) } else { (16.0, (h * 0.03).clamp(10.0, 24.0)) };
    let pw = (w - 2.0 * mx).min(1040.0);
    let panel = Rect::new(((w - pw) / 2.0).round(), my, pw, h - 2.0 * my);
    let mast_h = if compact { 62.0 } else { (panel.h * 0.22).clamp(100.0, 136.0).round() };
    let tabs_h = if compact { 36.0 } else { 42.0 };
    let foot_h = if compact { 52.0 } else { 66.0 };
    let pad = if compact { 10.0 } else { 16.0 };
    let mast = Rect::new(panel.x, panel.y, panel.w, mast_h);
    let tabs = Rect::new(panel.x, mast.y + mast_h, panel.w, tabs_h);
    let foot = Rect::new(panel.x, panel.y + panel.h - foot_h, panel.w, foot_h);
    let top = tabs.y + tabs_h + pad;
    let content = Rect::new(panel.x + pad, top, panel.w - 2.0 * pad, foot.y - pad * 0.5 - top);
    Frame { compact, panel, mast, tabs, content, foot }
}

/// Lays the launcher out at a window `w` x `h` units and, with `out.dr`, draws it.
pub fn layout(m: &Model, w: f32, h: f32, out: &mut Out) {
    let fr = frame(w, h);
    backdrop(w, h, out);
    out.skin(Skin::Site, fr.panel, false);
    masthead(&fr, out);
    tabs(m, &fr, out);
    match m.tab {
        Tab::Files => files_tab(m, fr.content, fr.compact, out),
        Tab::Play => play_tab(m, fr.content, fr.compact, out),
        Tab::Display => display_tab(m, fr.content, fr.compact, out),
        Tab::News => news_tab(m, fr.content, fr.compact, out),
    }
    footer(m, &fr, out);
}

/// The page's backdrop (#backdrop): deep green into navy, a soft light behind the panel, the dimples and the dark edges.
fn backdrop(w: f32, h: f32, out: &mut Out) {
    let mid_y = h * 0.48;
    out.gradient(Rect::new(0.0, 0.0, w, mid_y), c(0x1f6b3f), c(0x164a42));
    out.gradient(Rect::new(0.0, mid_y, w, h - mid_y), c(0x164a42), c(0x102a4f));
    if out.dr.is_some() {
        out.art(Art::Blob, Rect::new(w / 2.0 - 540.0, 340.0 - 440.0, 1080.0, 880.0), ca(0x96e1a0, 0.5));
        let (tw, th) = (128.0, 112.0);
        let mut y = 0.0;
        while y < h {
            let mut x = 0.0;
            while x < w {
                out.art(Art::Dimples, Rect::new(x, y, tw, th), WHITE);
                x += tw;
            }
            y += th;
        }
        out.art(Art::Vignette, Rect::new(0.0, 0.0, w, h), WHITE);
    }
    out.pieces.clear();
}

/// The masthead (.mast): the sky, its clouds and hills, the logo, the heading and kicker, and the links at the top right.
fn masthead(fr: &Frame, out: &mut Out) {
    let r = fr.mast;
    let inner = Rect::new(r.x + 1.0, r.y + 1.0, r.w - 2.0, r.h - 1.0);
    out.skin(Skin::Mast, inner, false);
    let k = r.h / 136.0;
    // the clouds (the page's three soft ellipses)
    for (fx, fy, cw, ch, a) in [(0.72, 0.28, 180.0, 68.0, 0.9), (0.78, 0.22, 140.0, 52.0, 0.85), (0.18, 0.78, 240.0, 60.0, 0.55)] {
        let (cx, cy) = (r.x + r.w * fx, r.y + r.h * fy);
        out.art(Art::Blob, Rect::new(cx - cw / 2.0, cy - ch * k / 2.0, cw, ch * k), [1.0, 1.0, 1.0, a]);
    }
    let hills_h = if fr.compact { 20.0 } else { (56.0 * k).round() };
    out.art(Art::Hills, Rect::new(inner.x, r.y + r.h - hills_h, inner.w, hills_h), WHITE);
    // the links: What's new and Buy me a coffee (the page's Downloads, GitHub and coffee)
    let pf = body(if fr.compact { 12.0 } else { 13.0 });
    let ph = if fr.compact { 26.0 } else { 30.0 };
    let pills = [(Hit::NewsPill, tr("STR_LAUNCHER_NEWS_PILL"), Skin::Pill), (Hit::Coffee, tr("STR_LAUNCHER_COFFEE"), Skin::CoffeePill)];
    let mut x = r.x + r.w - if fr.compact { 10.0 } else { 16.0 };
    let py = r.y + if fr.compact { 8.0 } else { 10.0 };
    let mut left_end = x;
    for (hit, label, sk) in pills.iter().rev() {
        let coffee = *sk == Skin::CoffeePill;
        let icon = if coffee { 18.0 } else { 0.0 };
        let pw = (pf.width(label) + 22.0 + icon).min(240.0);
        x -= pw;
        let pr = Rect::new(x, py, pw, ph);
        out.skin(*sk, pr, out.lit(*hit));
        let ink = if coffee { c(0x1b1b1b) } else { WHITE };
        if coffee {
            cup(out, pr.x + 11.0, pr.y + ph / 2.0, ph * 0.5, ink);
        }
        out.text_center(pf, pr.x + icon + (pw - icon) / 2.0, mid(pf, pr.y, ph), pw - 12.0 - icon, label, ink);
        out.hit(pr, *hit);
        left_end = x;
        x -= 6.0;
    }
    // the logo and the heading: "Open" and "Golf" gold, the S white, with the page's stacked shadow
    let (logo, h1, kick) = if fr.compact { (40.0, 32.0, 14.0) } else { ((58.0 * k).round(), (50.0 * k).round(), (21.0 * k).round()) };
    let lx = r.x + if fr.compact { 10.0 } else { 18.0 };
    let ly = if fr.compact { r.y + 8.0 } else { r.y + 12.0 * k };
    out.art(Art::Logo, Rect::new(lx, ly, logo, logo), WHITE);
    let f = display(h1);
    let hx = lx + logo + if fr.compact { 8.0 } else { 12.0 };
    let hy = ly + (logo - h1) / 2.0 - h1 * 0.12;
    let sh = [(0.0, 4.0, ca(0x001432, 0.45)), (0.0, 3.0, c(0x6b4400)), (0.0, 2.0, c(0x8a5a00))];
    let parts = [("Open", GOLD), ("S", WHITE), ("Golf", GOLD)];
    let mut tx = hx;
    for (s, col) in parts {
        out.text_shadowed(f, tx, hy, s, col, &sh);
        tx += f.width(s);
    }
    let kicker = tr("STR_LAUNCHER_KICKER");
    let kf = display(kick);
    let ksh = [(0.0, 2.0, ca(0x001432, 0.5)), (0.0, 1.0, c(0x0b2f63))];
    if fr.compact {
        // beside the heading, as room allows
        let kx = tx + 10.0;
        let room = left_end - 10.0 - kx;
        if room > 80.0 {
            let (kf2, ks, _) = crate::ui::fit_line(kf, kicker, room);
            out.text_shadowed(kf2, kx, hy + (h1 - kf2.px) * 0.62, &ks, WHITE, &ksh);
        } else {
            out.squeeze("kicker");
        }
    } else {
        let room = (r.x + r.w - 20.0 - hx).max(60.0);
        let (kf2, ks, _) = crate::ui::fit_line(kf, kicker, room);
        out.text_shadowed(kf2, hx + 2.0, hy + h1 * 1.08, &ks, WHITE, &ksh);
    }
}

/// The tab bar (nav.tabs): glossy green tabs, the open one in the panel's colour, joined to it.
fn tabs(m: &Model, fr: &Frame, out: &mut Out) {
    let r = fr.tabs;
    out.gradient(Rect::new(r.x + 1.0, r.y, r.w - 2.0, r.h), c(0x2e7d3d), c(0x1b5f2b));
    out.fill(Rect::new(r.x + 1.0, r.y, r.w - 2.0, 1.0), c(0x5cb86b));
    out.fill(Rect::new(r.x + 1.0, r.y + r.h - 3.0, r.w - 2.0, 3.0), c(PANEL));
    let (gap, pad) = (3.0, 8.0);
    let tw = (r.w - 2.0 * pad - gap * 3.0) / 4.0;
    let f = body(if fr.compact { 13.0 } else { 14.0 });
    for (i, t) in Tab::ALL.iter().enumerate() {
        let x = r.x + pad + (tw + gap) * i as f32;
        let on = m.tab == *t;
        let tr_ = if on { Rect::new(x, r.y + 6.0, tw, r.h - 6.0) } else { Rect::new(x, r.y + 6.0, tw, r.h - 9.0) };
        let hit = Hit::Tab(*t);
        out.skin(if on { Skin::TabOn } else { Skin::Tab }, tr_, !on && out.lit(hit));
        let label = tr(t.key());
        let ty = mid(f, tr_.y, r.h - 9.0);
        if on {
            out.text_center(f, x + tw / 2.0, ty, tw - 12.0, label, INK);
        } else {
            out.text_center(f, x + tw / 2.0, ty + 1.0, tw - 12.0, label, ca(0x000000, 0.45));
            out.text_center(f, x + tw / 2.0, ty, tw - 12.0, label, WHITE);
        }
        out.hit(Rect::new(x, r.y, tw, r.h), hit);
    }
}

/// The foot: the "Show this at start" tick and the big Play button.
fn footer(m: &Model, fr: &Frame, out: &mut Out) {
    let r = fr.foot;
    out.fill(Rect::new(r.x + 1.0, r.y, r.w - 2.0, 1.0), c(0xc8d6bf));
    out.gradient(Rect::new(r.x + 1.0, r.y + 1.0, r.w - 2.0, r.h - 14.0), ca(0x000000, 0.0), ca(0x000000, 0.03));
    let pad = if fr.compact { 10.0 } else { 16.0 };
    // Play: the page's main yellow button, with a ▶ drawn
    let pf = display(if fr.compact { 20.0 } else { 23.0 });
    let ph = if fr.compact { 40.0 } else { 48.0 };
    let label = tr("STR_LAUNCHER_PLAY");
    let pw = (pf.width(label) + 74.0).clamp(150.0, 260.0);
    let pr = Rect::new(r.x + r.w - pad - pw, r.y + (r.h - ph) / 2.0, pw, ph);
    out.skin(Skin::Gold, pr, out.lit(Hit::Play));
    let tw = pf.width(label).min(pw - 64.0);
    let x0 = pr.x + (pw - tw - 26.0) / 2.0;
    let cy = pr.y + ph / 2.0;
    let ink = c(0x2b1d00);
    let s = ph * 0.2;
    out.tri([(x0, cy - s), (x0 + s * 1.6, cy), (x0, cy + s)], ink);
    out.text_fit(pf, x0 + 26.0, mid(pf, pr.y, ph), pw - 64.0, label, ink);
    out.hit(pr, Hit::Play);
    // the tick (.remember)
    let bf = body(if fr.compact { 13.0 } else { 14.0 });
    let tick = tr("STR_LAUNCHER_SHOW_AT_START");
    let bs = 22.0;
    let bx = r.x + pad;
    let by = r.y + (r.h - bs) / 2.0;
    let room = (pr.x - 16.0 - (bx + bs + 10.0)).max(40.0);
    let lw = bf.width(tick).min(room);
    out.skin(if m.at_start { Skin::CheckOn } else { Skin::Check }, Rect::new(bx, by, bs, bs), out.lit(Hit::ShowAtStart));
    if m.at_start {
        out.line((bx + 5.5, by + 11.5), (bx + 9.5, by + 15.5), 2.6, WHITE);
        out.line((bx + 9.5, by + 15.5), (bx + 17.0, by + 6.5), 2.6, WHITE);
    }
    out.text_fit(bf, bx + bs + 10.0, mid(bf, by, bs), room, tick, INK_SOFT);
    out.hit(Rect::new(bx - 4.0, r.y + 4.0, bs + 14.0 + lw, r.h - 8.0), Hit::ShowAtStart);
    // unticked: how to get it back, where there is room
    if !m.at_start {
        let nf = body(if fr.compact { 11.0 } else { 12.0 });
        let nx = bx + bs + 10.0 + lw + 18.0;
        let room = pr.x - 16.0 - nx;
        let note = tr("STR_LAUNCHER_BRING_BACK");
        if room > 120.0 {
            let n = crate::extras::wrap(nf, note, room).len().min(2);
            let y = r.y + (r.h - (nf.px * 1.32).round() * n as f32) / 2.0;
            out.note(nf, nx, y, room, 2, note, INK_SOFT);
        } else {
            out.squeeze("bring back");
        }
    }
}

/// A cup of coffee `s` high with its left at x, centred on cy (the page's ☕ on its coffee links).
fn cup(out: &mut Out, x: f32, cy: f32, s: f32, ink: [f32; 4]) {
    let (w, h) = (s * 0.78, s * 0.62);
    let top = cy - h / 2.0 + s * 0.12;
    // the cup, narrower at its foot, its handle and the saucer
    out.tri([(x, top), (x + w, top), (x + w * 0.86, top + h)], ink);
    out.tri([(x, top), (x + w * 0.86, top + h), (x + w * 0.14, top + h)], ink);
    let (hx, hy, hr) = (x + w, top + h * 0.38, h * 0.26);
    for i in 0..6 {
        let a0 = (-90.0 + 30.0 * i as f32).to_radians();
        let a1 = (-90.0 + 30.0 * (i + 1) as f32).to_radians();
        out.line((hx + hr * a0.cos(), hy + hr * a0.sin()), (hx + hr * a1.cos(), hy + hr * a1.sin()), s * 0.1, ink);
    }
    out.line((x - s * 0.08, top + h + s * 0.08), (x + w + s * 0.08, top + h + s * 0.08), s * 0.09, ink);
    // the steam
    for k in 0..2 {
        let sx = x + w * (0.32 + 0.36 * k as f32);
        out.line((sx, top - s * 0.08), (sx + s * 0.06, top - s * 0.22), s * 0.07, ink);
        out.line((sx + s * 0.06, top - s * 0.22), (sx, top - s * 0.36), s * 0.07, ink);
    }
}

/// A gel button: its label fitted in it, the light blue one's dark ink, the green one's white, a disabled one faded.
fn button(out: &mut Out, k: Skin, r: Rect, f: Fnt, label: &str, hit: Option<Hit>, enabled: bool) {
    let lit = enabled && hit.is_some_and(|h| out.lit(h));
    out.skin_tint(k, r, lit, if enabled { WHITE } else { [1.0, 1.0, 1.0, 0.55] });
    let ink = match k {
        Skin::Green => WHITE,
        Skin::Gold => c(0x2b1d00),
        Skin::Coffee => c(0x1b1b1b),
        _ => c(0x0d2a52),
    };
    let ink = if enabled { ink } else { ca(0x0d2a52, 0.45) };
    let y = mid(f, r.y, r.h);
    if k == Skin::Green {
        out.text_center(f, r.x + r.w / 2.0, y - 1.0, r.w - 14.0, label, ca(0x000000, 0.4));
    }
    if k == Skin::Coffee {
        // the page's coffee button has its cup
        let icon = r.h * 0.5;
        let tw = f.width(label).min(r.w - 24.0 - icon);
        let x0 = r.x + (r.w - tw - icon - 8.0) / 2.0;
        cup(out, x0, r.y + r.h / 2.0, icon, ink);
        out.text_fit(f, x0 + icon + 8.0, y, r.w - 14.0 - icon - 8.0, label, ink);
    } else {
        out.text_center(f, r.x + r.w / 2.0, y, r.w - 14.0, label, ink);
    }
    if let Some(h) = hit.filter(|_| enabled) {
        out.hit(r, h);
    }
}

/// A gel button as wide as its label (at least `min`, at most `max`).
fn button_w(f: Fnt, label: &str, min: f32, max: f32) -> f32 {
    (f.width(label) + 36.0).clamp(min, max.max(min))
}

/// Game files: the summary of what is in use, then a card for each choice.
fn files_tab(m: &Model, r: Rect, compact: bool, out: &mut Out) {
    let lf = body(if compact { 14.0 } else { 16.0 });
    let df = body(if compact { 12.0 } else { 13.0 });
    let two = !m.files_detail.is_empty() && r.h >= 200.0;
    let sh = if two { (lf.px + df.px * 1.32 + 24.0).round() } else { (lf.px + 22.0).round() };
    let sr = Rect::new(r.x, r.y, r.w, sh);
    out.skin(if m.files_problem { Skin::Problem } else { Skin::Summary }, sr, false);
    let ball = 28.0;
    out.art(
        if m.files_problem { Art::BallProblem } else { Art::BallOk },
        Rect::new(sr.x + 12.0, sr.y + (sh - ball) / 2.0, ball, ball),
        WHITE,
    );
    let tx = sr.x + 12.0 + ball + 12.0;
    let room = sr.x + sr.w - 14.0 - tx;
    let ink = if m.files_problem { c(0x2b2100) } else { INK };
    if two {
        let y = sr.y + (sh - lf.px - df.px * 1.32) / 2.0 - 1.0;
        out.text_fit(lf, tx, y, room, &m.files_line, ink);
        out.text_fit(df, tx, y + lf.px + 4.0, room, &m.files_detail, if m.files_problem { ink } else { INK_SOFT });
    } else {
        if !m.files_detail.is_empty() {
            out.squeeze("files detail");
        }
        out.text_fit(lf, tx, mid(lf, sr.y, sh), room, &m.files_line, ink);
    }
    // the two choices
    let top = sr.y + sh + if compact { 10.0 } else { 14.0 };
    let avail = r.y + r.h - top;
    let side = r.w >= 520.0;
    let gap = 12.0;
    // the sounds tip under the cards, on the free graphics, where the cards keep room enough
    let tip_h = if compact { 46.0 } else { 58.0 };
    let tip = !m.mine_in_use && avail - tip_h - 10.0 >= if side { 150.0 } else { 2.0 * 110.0 + gap };
    let avail = if tip { avail - tip_h - 10.0 } else { avail };
    let (cw, ch) = if side { ((r.w - gap) / 2.0, avail.min(184.0)) } else { (r.w, ((avail - gap) / 2.0).min(150.0)) };
    let cards = [
        (true, "STR_LAUNCHER_MINE_TITLE", if m.browser { "STR_LAUNCHER_MINE_NOTE_WEB" } else { "STR_LAUNCHER_MINE_NOTE" }),
        (false, "STR_LAUNCHER_FREE_TITLE", "STR_LAUNCHER_FREE_NOTE"),
    ];
    for (i, &(mine, title, note)) in cards.iter().enumerate() {
        let (x, y) = if side { (r.x + (cw + gap) * i as f32, top) } else { (r.x, top + (ch + gap) * i as f32) };
        let cr = Rect::new(x, y, cw, ch);
        let in_use = mine == m.mine_in_use;
        out.skin(if in_use { Skin::Primary } else { Skin::Card }, cr, false);
        let pad = if compact { 12.0 } else { 14.0 };
        let tf = display(if compact { 17.0 } else { 19.0 });
        let nf = body(if compact { 12.0 } else { 13.0 });
        let bh = if compact { 34.0 } else { 40.0 };
        let bf = body(if compact { 13.0 } else { 14.0 });
        let action = if mine { m.mine_action } else { m.free_action };
        // the page's pictures beside the words where the card is wide enough: the CD (a click on it picks the files, as
        // on the page) and the framed picture of the free graphics
        let mut tw = cr.w - 2.0 * pad;
        let cap = if compact { 84.0 } else { 112.0 };
        if cr.w - 2.0 * pad - cap - 12.0 >= 250.0 && ch >= 110.0 {
            let ar = if mine {
                let a = (ch - 2.0 * pad).min(cap);
                Rect::new(cr.x + cr.w - pad - a, cr.y + (ch - a) / 2.0, a, a)
            } else {
                let a = (cr.w * 0.36).min(cap * 1.5).min((ch - 2.0 * pad) * 244.0 / 186.0);
                Rect::new(cr.x + cr.w - pad - a + 6.0, cr.y + (ch - a * 186.0 / 244.0) / 2.0 + 4.0, a, a * 186.0 / 244.0)
            };
            out.art(if mine { Art::Cd } else { Art::Shot }, ar, WHITE);
            if let Some(a) = action.filter(|_| mine && !in_use && !m.browser) {
                out.hit(Rect::new(ar.x + 4.0, ar.y + 4.0, ar.w - 8.0, ar.h - 8.0), Hit::Files(a));
            }
            tw = ar.x - 12.0 - (cr.x + pad);
        }
        out.text_fit(tf, cr.x + pad, cr.y + pad - 2.0, tw, tr(title), CARD_HEAD);
        let ny = cr.y + pad + tf.px + 6.0;
        let room_lines = ((cr.y + cr.h - pad - bh - 8.0 - ny) / (nf.px * 1.32).round()).floor().max(0.0) as usize;
        let lines = room_lines.min(4);
        if lines == 0 {
            out.squeeze("card note");
        }
        out.note(nf, cr.x + pad, ny, tw, lines, tr(note), INK_SOFT);
        let by = cr.y + cr.h - pad - bh;
        if in_use {
            // the choice in use: a green tag with a tick
            let label = tr("STR_LAUNCHER_IN_USE");
            let w = button_w(bf, label, 100.0, tw - 18.0) + 18.0;
            let br = Rect::new(cr.x + pad, by, w, bh);
            out.skin(Skin::Green, br, false);
            let cx = br.x + 18.0;
            let cy = br.y + bh / 2.0;
            out.line((cx - 5.0, cy), (cx - 1.0, cy + 4.0), 2.4, WHITE);
            out.line((cx - 1.0, cy + 4.0), (cx + 6.0, cy - 5.0), 2.4, WHITE);
            out.text_center(bf, br.x + 10.0 + w / 2.0, mid(bf, br.y, bh), w - 34.0, label, WHITE);
        } else if m.browser && mine {
            let label = tr("STR_LAUNCHER_BACK_TO_PAGE");
            let w = button_w(bf, label, 120.0, tw);
            button(out, Skin::Sky, Rect::new(cr.x + pad, by, w, bh), bf, label, Some(Hit::BackToPage), true);
        } else if let Some(a) = action {
            let label = tr(if mine { "STR_LAUNCHER_MINE_BUTTON" } else { "STR_LAUNCHER_FREE_BUTTON" });
            let w = button_w(bf, label, 120.0, tw);
            button(out, Skin::Sky, Rect::new(cr.x + pad, by, w, bh), bf, label, Some(Hit::Files(a)), true);
        } else {
            // nothing to choose here (Android with the player's files: the free graphics are not offered)
            out.text_fit(nf, cr.x + pad, mid(nf, by, bh), tw, tr("STR_LAUNCHER_NOT_HERE"), INK_SOFT);
        }
    }
    if tip {
        let below = if side { top + ch } else { top + 2.0 * ch + gap };
        sound_tip(Rect::new(r.x, below + 10.0, r.w, tip_h), compact, out);
    }
}

/// The page's sounds tip under the files' cards, where there is room: how to get the original sounds.
fn sound_tip(r: Rect, compact: bool, out: &mut Out) {
    out.skin(Skin::Tip, r, false);
    // a loudspeaker
    let ink = c(0x2b2100);
    let (x, cy) = (r.x + 16.0, r.y + r.h / 2.0);
    out.fill(Rect::new(x, cy - 4.0, 6.0, 8.0), ink);
    out.tri([(x + 4.0, cy - 4.0), (x + 13.0, cy - 10.0), (x + 13.0, cy + 10.0)], ink);
    out.tri([(x + 4.0, cy - 4.0), (x + 13.0, cy + 10.0), (x + 4.0, cy + 4.0)], ink);
    for (k, rad) in [(0, 6.0), (1, 11.0)] {
        let a = (x + 15.0 + k as f32, cy);
        for i in 0..6 {
            let t0 = (-50.0 + 20.0 * i as f32).to_radians();
            let t1 = (-50.0 + 20.0 * (i + 1) as f32).to_radians();
            out.line((a.0 + rad * t0.cos(), a.1 + rad * t0.sin()), (a.0 + rad * t1.cos(), a.1 + rad * t1.sin()), 1.8, ink);
        }
    }
    let tx = r.x + 48.0;
    let w = r.x + r.w - 14.0 - tx;
    let hf = display(if compact { 14.0 } else { 16.0 });
    let nf = body(if compact { 12.0 } else { 13.0 });
    let two = r.h >= hf.px + nf.px * 1.32 + 14.0;
    let y = if two { r.y + (r.h - hf.px - nf.px * 1.32 - 2.0) / 2.0 - 1.0 } else { mid(hf, r.y, r.h) };
    out.text_fit(hf, tx, y, w, tr("STR_LAUNCHER_TIP_TITLE"), ink);
    if two {
        out.note(nf, tx, y + hf.px + 3.0, w, 1, tr("STR_LAUNCHER_TIP"), ink);
    }
}

/// How to play: the four presets, the one in use framed and its dot filled.
fn play_tab(m: &Model, r: Rect, compact: bool, out: &mut Out) {
    let hf = display(if compact { 17.0 } else { 20.0 });
    let bf = body(if compact { 12.0 } else { 13.0 });
    let bh = if compact { 28.0 } else { 32.0 };
    let opts = tr("STR_LAUNCHER_GAME_OPTIONS");
    let bw = button_w(bf, opts, 110.0, r.w * 0.4);
    let head_h = bh;
    button(out, Skin::Sky, Rect::new(r.x + r.w - bw, r.y, bw, bh), bf, opts, Some(Hit::GameOptions), true);
    out.text_fit(hf, r.x, mid(hf, r.y, head_h), r.w - bw - 12.0, tr("STR_WELCOME_TITLE"), INK);
    let top = r.y + head_h + if compact { 8.0 } else { 12.0 };
    let cols = if r.w >= 520.0 { 2 } else { 1 };
    let rows = 4 / cols;
    let gap = if compact { 8.0 } else { 12.0 };
    let cw = (r.w - gap * (cols - 1) as f32) / cols as f32;
    let nf = body(if compact { 12.0 } else { 13.0 });
    // where it can be changed later, under the choices where there is room
    let change_h = (nf.px * 1.32).round() + 10.0;
    let room = r.y + r.h - top - gap * (rows - 1) as f32;
    let change = room - change_h >= rows as f32 * 84.0;
    let ch = ((if change { room - change_h } else { room }) / rows as f32).min(104.0);
    if change {
        let y = top + (ch + gap) * rows as f32 - gap + 10.0;
        out.note(nf, r.x, y, r.w, 1, tr("STR_WELCOME_CHANGE"), INK_SOFT);
    }
    let tf = display(if compact { 16.0 } else { 18.0 });
    for (i, (p, line, note)) in crate::welcome::CHOICES.iter().enumerate() {
        let (col, row) = (i % cols, i / cols);
        let cr = Rect::new(r.x + (cw + gap) * col as f32, top + (ch + gap) * row as f32, cw, ch);
        let on = m.preset == *p;
        let hit = Hit::Preset(i);
        out.skin(if on { Skin::Primary } else { Skin::Card }, cr, false);
        if out.lit(hit) && !on {
            // under the pointer: a faint green frame
            out.skin_tint(Skin::Summary, cr, false, [1.0, 1.0, 1.0, 0.5]);
        }
        let pad = if compact { 10.0 } else { 14.0 };
        // the radio dot
        let rd = 20.0;
        let (rx, ry) = (cr.x + pad, cr.y + pad);
        out.skin(if on { Skin::RadioOn } else { Skin::Radio }, Rect::new(rx, ry, rd, rd), false);
        if on {
            out.skin(Skin::Radio, Rect::new(rx + 6.0, ry + 6.0, rd - 12.0, rd - 12.0), false);
        }
        let tx = rx + rd + 10.0;
        out.text_fit(tf, tx, ry + (rd - tf.px) / 2.0 - 2.0, cr.x + cr.w - pad - tx, tr(line).trim(), if on { CARD_HEAD } else { INK });
        let ny = ry + rd + 6.0;
        let lines = ((cr.y + cr.h - pad + 2.0 - ny) / (nf.px * 1.32).round()).floor().max(0.0) as usize;
        if lines == 0 {
            out.squeeze("preset note");
        }
        out.note(nf, tx, ny, cr.x + cr.w - pad - tx, lines.min(3), tr(note), INK_SOFT);
        out.hit(cr, hit);
    }
}

/// One row of Display & sound: a choice of buttons (the one in use green) or a stepper.
enum Ctl {
    Seg(Vec<(String, Hit, bool)>),
    Step(String, Hit, Hit, bool),
}

/// Display & sound: two columns of labelled rows.
fn display_tab(m: &Model, r: Rect, compact: bool, out: &mut Out) {
    let mut rows: Vec<(&'static str, Ctl)> = Vec::new();
    if m.desktop {
        rows.push((
            "STR_LAUNCHER_DISPLAY",
            Ctl::Seg(vec![
                (tr("STR_LAUNCHER_WINDOW").into(), Hit::Fullscreen(false), !m.fullscreen),
                (tr("STR_LAUNCHER_FULLSCREEN").into(), Hit::Fullscreen(true), m.fullscreen),
            ]),
        ));
        rows.push(("STR_LAUNCHER_WINDOW_SIZE", Ctl::Step(m.window.clone(), Hit::WindowSize(-1), Hit::WindowSize(1), !m.fullscreen)));
    }
    rows.push(("STR_LAUNCHER_TEXT_SIZE", Ctl::Step(m.text_size.clone(), Hit::TextSize(-1), Hit::TextSize(1), true)));
    let mut looks = vec![
        (tr("STR_LAUNCHER_CLASSIC").to_string(), Hit::Graphics(Look::Classic), m.look == Look::Classic),
        (tr("STR_LAUNCHER_ENHANCED").to_string(), Hit::Graphics(Look::Enhanced), m.enhanced_on),
    ];
    if m.hd {
        looks.push((tr("STR_LAUNCHER_HD").to_string(), Hit::Graphics(Look::Hd), m.look == Look::Hd));
    }
    rows.push(("STR_LAUNCHER_GRAPHICS", Ctl::Seg(looks)));
    rows.push((
        "STR_LAUNCHER_SOUND",
        Ctl::Seg(vec![
            (tr("STR_LAUNCHER_ON").into(), Hit::Sound(true), m.sound),
            (tr("STR_LAUNCHER_OFF").into(), Hit::Sound(false), !m.sound),
        ]),
    ));
    rows.push(("STR_LAUNCHER_LANGUAGE", Ctl::Step(m.language.clone(), Hit::Language(-1), Hit::Language(1), m.languages > 1)));
    let cols = if r.w >= 560.0 { 2 } else { 1 };
    let per = rows.len().div_ceil(cols);
    let gap_x = if compact { 10.0 } else { 14.0 };
    let colw = (r.w - gap_x * (cols - 1) as f32) / cols as f32;
    let vgap = if compact { 6.0 } else { 10.0 };
    let note_h = if compact || r.h < 300.0 { 0.0 } else { 40.0 };
    let cp = if compact { 8.0 } else { 14.0 };
    let rh = ((r.h - note_h - 2.0 * cp - vgap * (per - 1) as f32) / per as f32).min(if compact { 42.0 } else { 46.0 });
    if rh < 28.0 {
        out.squeeze("display rows");
    }
    // each column in a card, the page's .card
    for col in 0..cols {
        let n = (rows.len() - col * per).min(per) as f32;
        let x = r.x + (colw + gap_x) * col as f32;
        out.skin(Skin::Card, Rect::new(x, r.y, colw, n * rh + (n - 1.0) * vgap + 2.0 * cp), false);
        for k in 1..n as usize {
            let y = r.y + cp + (rh + vgap) * k as f32 - vgap / 2.0;
            out.fill(Rect::new(x + cp, y, colw - 2.0 * cp, 1.0), c(0xe3ebdc));
        }
    }
    let lf = body(if compact { 13.0 } else { 14.0 });
    let bf = body(if compact { 12.0 } else { 13.0 });
    for (i, (label, ctl)) in rows.iter().enumerate() {
        let (col, row) = (i / per, i % per);
        let x = r.x + (colw + gap_x) * col as f32 + cp;
        let y = r.y + cp + (rh + vgap) * row as f32;
        let colw = colw - 2.0 * cp;
        let lw = (colw * 0.34).clamp(90.0, 170.0);
        out.text_fit(lf, x, mid(lf, y, rh), lw - 8.0, tr(label), INK);
        let cx = x + lw;
        let cw = colw - lw;
        let bh = (rh - 6.0).min(36.0);
        let by = y + (rh - bh) / 2.0;
        match ctl {
            Ctl::Seg(opts) => {
                let n = opts.len() as f32;
                let g = 6.0;
                let w = (cw - g * (n - 1.0)) / n;
                for (k, (t, hit, on)) in opts.iter().enumerate() {
                    let br = Rect::new(cx + (w + g) * k as f32, by, w, bh);
                    button(out, if *on { Skin::Green } else { Skin::Sky }, br, bf, t, Some(*hit), true);
                }
            }
            Ctl::Step(value, prev, next, enabled) => {
                let aw = bh;
                let fr = Rect::new(cx + aw + 6.0, by, cw - 2.0 * aw - 12.0, bh);
                out.skin_tint(Skin::Field, fr, false, if *enabled { WHITE } else { [1.0, 1.0, 1.0, 0.6] });
                out.text_center(bf, fr.x + fr.w / 2.0, mid(bf, fr.y, bh), fr.w - 10.0, value, if *enabled { INK } else { INK_SOFT });
                for (k, h) in [(0, prev), (1, next)] {
                    let ar = Rect::new(if k == 0 { cx } else { cx + cw - aw }, by, aw, bh);
                    button(out, Skin::Sky, ar, bf, "", Some(*h), *enabled);
                    let (mx, my) = (ar.x + aw / 2.0, ar.y + bh / 2.0);
                    let s = bh * 0.18;
                    let ink = if *enabled { c(0x0d2a52) } else { ca(0x0d2a52, 0.35) };
                    if k == 0 {
                        out.tri([(mx + s * 0.6, my - s), (mx - s * 0.8, my), (mx + s * 0.6, my + s)], ink);
                    } else {
                        out.tri([(mx - s * 0.6, my - s), (mx + s * 0.8, my), (mx - s * 0.6, my + s)], ink);
                    }
                }
            }
        }
    }
    if note_h > 0.0 {
        let nf = body(12.0);
        let y = r.y + per as f32 * rh + (per - 1) as f32 * vgap + 2.0 * cp + 10.0;
        out.note(nf, r.x + 2.0, y, r.w - 4.0, 2, tr("STR_LAUNCHER_DISPLAY_NOTE"), INK_SOFT);
    }
}

/// News & updates: the version, this version's highlights, and the buttons.
fn news_tab(m: &Model, r: Rect, compact: bool, out: &mut Out) {
    let hf = display(if compact { 18.0 } else { 22.0 });
    let sf = body(if compact { 12.0 } else { 14.0 });
    let head_h = if compact { 24.0 } else { 30.0 };
    out.text(hf, r.x, mid(hf, r.y, head_h), &m.version, CARD_HEAD);
    let vx = r.x + hf.width(&m.version) + 14.0;
    out.text_fit(sf, vx, mid(sf, r.y, head_h) + 1.0, r.x + r.w - vx, &m.news_title, INK_SOFT);
    // the buttons, at the foot
    let bf = body(if compact { 12.0 } else { 14.0 });
    let bh = if compact { 32.0 } else { 40.0 };
    let mut btns: Vec<(Skin, &str, Hit, bool)> = Vec::new();
    if let Some(checking) = m.update {
        btns.push((Skin::Sky, tr(if checking { "STR_UPDATE_CHECKING_BUTTON" } else { "STR_LAUNCHER_UPDATE" }), Hit::Update, !checking));
    }
    btns.push((Skin::Sky, tr("STR_LAUNCHER_CHANGELOG"), Hit::Changelog, true));
    btns.push((Skin::Coffee, tr("STR_LAUNCHER_COFFEE_BUTTON"), Hit::Coffee, true));
    let gap = 10.0;
    let mut ws: Vec<f32> =
        btns.iter().map(|b| button_w(bf, b.1, 90.0, 320.0) + if b.0 == Skin::Coffee { bh * 0.5 + 8.0 } else { 0.0 }).collect();
    let total: f32 = ws.iter().sum::<f32>() + gap * (ws.len() - 1) as f32;
    if total > r.w {
        let k = (r.w - gap * (ws.len() - 1) as f32) / (total - gap * (ws.len() - 1) as f32);
        ws.iter_mut().for_each(|w| *w *= k);
    }
    let by = r.y + r.h - bh;
    let mut x = r.x;
    for ((sk, label, hit, en), w) in btns.iter().zip(&ws) {
        button(out, *sk, Rect::new(x, by, *w, bh), bf, label, Some(*hit), *en);
        x += w + gap;
    }
    // the highlights in the page's news box
    let top = r.y + head_h + if compact { 6.0 } else { 10.0 };
    let br = Rect::new(r.x, top, r.w, by - (if compact { 8.0 } else { 12.0 }) - top);
    out.skin(Skin::News, br, false);
    let n = m.highlights.len().max(1);
    let cols = if br.w >= 560.0 && n > 1 { 2 } else { 1 };
    let rows = n.div_ceil(cols);
    let pad = if compact { 8.0 } else { 12.0 };
    let ih = (br.h - 2.0 * pad) / rows as f32;
    let cw = (br.w - 2.0 * pad - 16.0 * (cols - 1) as f32) / cols as f32;
    let lf = display(if compact { 14.0 } else { 16.0 });
    let nf = body(if compact { 11.0 } else { 12.5 });
    if ih < lf.px + 2.0 {
        out.squeeze("highlights");
    }
    for (i, (line, note)) in m.highlights.iter().enumerate() {
        let (col, row) = (i / rows, i % rows);
        let x = br.x + pad + (cw + 16.0) * col as f32;
        let y = br.y + pad + ih * row as f32;
        // a gold bullet, as the page's news list
        let dot = 7.0;
        out.skin(Skin::Gold, Rect::new(x, y + (lf.px - dot) / 2.0 + 2.0, dot, dot), false);
        out.text_fit(lf, x + dot + 8.0, y, cw - dot - 8.0, line.trim(), INK);
        let lines = ((ih - lf.px - 6.0) / (nf.px * 1.32).round()).floor().max(0.0) as usize;
        if lines > 0 {
            out.note(nf, x + dot + 8.0, y + lf.px + 3.0, cw - dot - 8.0, lines.min(2), note, INK_SOFT);
        }
    }
}

// ---- the game's side -----------------------------------------------------------------------------------------------------------

/// The title menu's Launcher... button's hover value (after editor::EDITOR_HOVER).
pub const TITLE_HOVER: i32 = 10;

/// The title menu's Launcher... button: at the top left beside the files button (`files`: its label; none: in its place).
pub fn title_button_rect(files: Option<&str>) -> Rect {
    let x = files.map_or(8.0, |l| {
        let f = crate::gamefiles::files_button_rect(l);
        f.x + f.w + 8.0
    });
    let w = crate::ui::F_MANUAL15.width(tr("STR_LAUNCHER_TITLE_BUTTON")).max(80.0) + 24.0;
    Rect::new(x, 6.0, w, 24.0)
}

impl State {
    pub fn new(tab: Tab, forced: bool) -> State {
        State { tab, hover: None, at_start: show_at_start(), forced, hits: Vec::new(), k: 1.0 }
    }
}

impl App {
    /// Shows the launcher (`at_start`: as the game starts; `forced`: a test hook, nothing written). At a new player's first
    /// start it takes the place of "How would you like to play?", and after an update of What's new, opening on its News
    /// tab (both recorded as shown, as those boxes record themselves).
    pub fn open_launcher(&mut self, at_start: bool, forced: bool) {
        let news = at_start && self.welcome.launcher_shown(!forced);
        let tab = std::env::var(TAB_ENV).ok().and_then(|t| Tab::named(&t)).unwrap_or(if news { Tab::News } else { Tab::Files });
        self.launcher = Some(State::new(tab, forced));
        self.hover = -1;
    }

    /// The title menu's Launcher... button.
    pub fn launcher_button(&self) -> Rect {
        title_button_rect(self.files_option().map(|o| o.1))
    }

    /// Draws the title menu's Launcher... button, in the files button's style.
    pub fn draw_launcher_button(&self, g: &mut Gfx, s: &Ui) {
        use crate::ui::{rgb, rgba};
        let r = self.launcher_button();
        let lit = self.hover == TITLE_HOVER;
        let label = tr("STR_LAUNCHER_TITLE_BUTTON");
        if crate::glass_ui::active() {
            crate::glass_ui::title_button(g, s, r.x, r.y, r.w, r.h, label, lit);
            return;
        }
        s.fill(g, r.x - 1.0, r.y - 1.0, r.w + 2.0, r.h + 2.0, rgba(1.0, 1.0, 1.0, 0.85));
        s.fill(g, r.x, r.y, r.w, r.h, if lit { rgba(0.13, 0.32, 0.55, 0.95) } else { rgba(0.08, 0.16, 0.32, 0.88) });
        s.put_centered(g, crate::ui::F_MANUAL15, r.x + r.w / 2.0, r.y + 4.0, label, rgb(1.0, 1.0, if lit { 1.0 } else { 0.8 }));
    }

    /// What the launcher shows now (`fullscreen`: the desktop window is full screen).
    pub fn launcher_model(&self, fullscreen: bool) -> Model {
        let st = self.launcher.clone().unwrap_or_else(|| State::new(Tab::Files, true));
        let report = self.files_report();
        // (the test hook's own files: the title button's choice as with them)
        let option = if report.mine && self.on_free_set() {
            cfg!(not(any(target_arch = "wasm32", target_os = "android"))).then_some((Action::Free, ""))
        } else {
            self.files_option()
        };
        let mine_in_use = report.mine;
        let mine_action = option.filter(|o| o.0 != Action::Free).map(|o| o.0);
        let free_action = option.filter(|o| o.0 == Action::Free).map(|o| o.0);
        let enh = crate::enhanced::settings();
        let look = if self.hd.enabled {
            Look::Hd
        } else if enh.enabled {
            Look::Enhanced
        } else {
            Look::Classic
        };
        let (sw, sh) = ((self.draw_w / self.dpi.max(1.0)).round(), (self.draw_h / self.dpi.max(1.0)).round());
        let langs = crate::language::available();
        let news = crate::welcome::highlights().last().copied();
        Model {
            tab: st.tab,
            hover: st.hover,
            at_start: st.at_start,
            files_line: report.line,
            files_detail: report.detail,
            files_problem: report.problem,
            mine_in_use,
            mine_action,
            free_action,
            browser: cfg!(target_arch = "wasm32"),
            preset: Preset::of(&self.extras_shown(), &crate::extras::settings()),
            desktop: cfg!(not(any(target_arch = "wasm32", target_os = "android"))),
            fullscreen,
            window: format!("{sw} \u{d7} {sh}"),
            look,
            hd: self.hd.enabled || (!self.on_free_set() && self.hd_remove_offer()),
            enhanced_on: enh.enabled,
            text_size: capital(tr(crate::textsize::name_key(crate::textsize::pct()))),
            sound: !self.mute,
            language: crate::language::current_name(),
            languages: langs.len(),
            version: trf("STR_TITLE_VERSION", &[("VERSION", &crate::update::VERSION)]),
            news_title: news.map(|(v, _)| trf("STR_WHATSNEW_TITLE", &[("VERSION", &v)])).unwrap_or_default(),
            highlights: news
                .map(|(_, h)| h.iter().map(|x| (tr(x.line).trim().to_string(), tr(x.note).to_string())).collect())
                .unwrap_or_default(),
            update: crate::update::AVAILABLE.then_some(self.update_check.is_some()),
        }
    }

    /// Draws the launcher over the whole window (`fullscreen` as `launcher_model`).
    pub fn draw_launcher(&mut self, g: &mut Gfx, fullscreen: bool) {
        crate::ui::Font::load_bundled(g);
        let k = unit(self.draw_w, self.draw_h, self.dpi);
        let (w, h) = (self.draw_w / k, self.draw_h / k);
        let model = self.launcher_model(fullscreen);
        let s = Ui::with_unit(self.draw_w, self.draw_h, k);
        crate::ui::set_crisp(true);
        let mut out = Out::new(Some(Dr { g: &mut *g, s: &s }), model.hover);
        layout(&model, w, h, &mut out);
        let hits = std::mem::take(&mut out.hits);
        drop(out);
        crate::ui::set_crisp(false);
        g.flush();
        if let Some(st) = self.launcher.as_mut() {
            st.hits = hits;
            st.k = k;
        }
    }

    /// The pointer at window pixel (x, y): the button under it lit; with `click`, what it does.
    pub fn launcher_pointer(&mut self, x: f32, y: f32, click: bool) -> Option<Hit> {
        let st = self.launcher.as_mut()?;
        let (ux, uy) = (x / st.k, y / st.k);
        let hit = st.hits.iter().rev().find(|(r, _)| r.has(ux, uy)).map(|(_, h)| *h);
        st.hover = hit;
        hit.filter(|_| click)
    }

    /// A key on the launcher: Enter, Esc (Android's back) or Space play; Left and Right (Tab) change the tab.
    pub fn launcher_key(&mut self, k: miniquad::KeyCode) -> Option<Hit> {
        use miniquad::KeyCode;
        let st = self.launcher.as_mut()?;
        match k {
            KeyCode::Enter | KeyCode::KpEnter | KeyCode::Escape | KeyCode::Space => Some(Hit::Play),
            KeyCode::Right | KeyCode::Tab => Some(Hit::Tab(st.tab.step(1))),
            KeyCode::Left => Some(Hit::Tab(st.tab.step(-1))),
            _ => None,
        }
    }

    /// What a click does that needs nothing but the game's state (the rest: main.rs `launcher_act`). False when it is not
    /// one of those.
    pub fn launcher_simple(&mut self, h: Hit) -> bool {
        let Some(st) = self.launcher.as_mut() else { return false };
        match h {
            Hit::Tab(t) => st.tab = t,
            Hit::NewsPill => st.tab = Tab::News,
            Hit::Play => {
                self.launcher = None;
                self.hover = -1;
            }
            Hit::ShowAtStart => {
                st.at_start = !st.at_start;
                if !st.forced {
                    save(SHOW_KEY, st.at_start.into());
                }
            }
            Hit::Coffee => self.open_support_page(),
            Hit::Changelog => self.open_changelog(),
            Hit::Update => self.check_for_updates(),
            Hit::TextSize(step) => {
                let p = crate::textsize::pct();
                let n = crate::textsize::SIZES.len();
                let i = crate::textsize::SIZES.iter().position(|&s| s == p).unwrap_or(0) as i32;
                crate::textsize::save(crate::textsize::SIZES[(i + step).rem_euclid(n as i32) as usize]);
            }
            Hit::Language(step) => {
                let all = crate::language::available();
                let code = sg_core::lang::language();
                let i = all.iter().position(|(c, _)| *c == code).unwrap_or(0) as i32;
                let next = all[(i + step).rem_euclid(all.len().max(1) as i32) as usize].0.clone();
                if crate::language::apply(&next) {
                    crate::language::save(&next);
                }
            }
            _ => return false,
        }
        true
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::json;

    /// The window sizes the launcher is laid out at in the tests, in units: phones held sideways to 4K's 600 units high,
    /// and a narrow window.
    const SIZES_U: [(f32, f32); 8] =
        [(640.0, 360.0), (667.0, 375.0), (844.0, 390.0), (800.0, 600.0), (1067.0, 600.0), (1280.0, 600.0), (960.0, 600.0), (640.0, 853.0)];

    #[test]
    fn it_shows_for_new_players_and_while_ticked_never_in_scripted_runs() {
        // a new player: always, even with the tick off (a settings.json written by a first start closed early)
        for v in [json!({}), json!(null), json!({ "welcome_shown": false, SHOW_KEY: false })] {
            assert!(decide(&v, false, false), "{v}");
            assert!(!decide(&v, true, false), "scripted: {v}");
        }
        // a player who has been here before: as the tick says (missing: on)
        assert!(decide(&json!({ "welcome_shown": true }), false, false));
        assert!(decide(&json!({ "welcome_shown": true, SHOW_KEY: true }), false, false));
        assert!(!decide(&json!({ "welcome_shown": true, SHOW_KEY: false }), false, false));
        // a player of an older version (settings without the welcome key) keeps it unless unticked
        assert!(decide(&json!({ "ui_extras": {} }), false, false));
        assert!(!decide(&json!({ "ui_extras": {}, SHOW_KEY: false }), false, false));
        // scripted runs never, unless a test hook asks
        assert!(!decide(&json!({ "welcome_shown": true }), true, false));
        assert!(decide(&json!({ "welcome_shown": true, SHOW_KEY: false }), true, true));
    }

    #[test]
    fn the_window_setting_reads_back() {
        assert_eq!(window_of(&json!({ "fullscreen": true, "width": 1440, "height": 900 })), (Some((1440, 900)), true));
        assert_eq!(window_of(&json!({ "fullscreen": false })), (None, false));
        assert_eq!(window_of(&json!(null)), (None, false));
        // nonsense sizes are left out
        assert_eq!(window_of(&json!({ "width": 20, "height": 900 })), (None, false));
        assert_eq!(window_of(&json!({ "width": 100000, "height": 900 })).0, None);
        assert_eq!(next_size(1280.0, 1), Some((1440, 900)));
        assert_eq!(next_size(1280.0, -1), Some((1024, 640)));
        assert_eq!(next_size(800.0, 1), Some((1024, 640)));
        assert_eq!(next_size(1920.0, 1), None);
        assert_eq!(next_size(1024.0, -1), None);
        assert_eq!(capital("normal"), "Normal");
    }

    #[test]
    fn tabs_step_round() {
        assert_eq!(Tab::Files.step(1), Tab::Play);
        assert_eq!(Tab::News.step(1), Tab::Files);
        assert_eq!(Tab::Files.step(-1), Tab::News);
        assert_eq!(Tab::named("news"), Some(Tab::News));
        assert_eq!(Tab::named("nope"), None);
    }

    #[test]
    fn the_unit_is_about_a_point_and_fits_600_by_640() {
        assert_eq!(unit(800.0, 600.0, 1.0), 1.0);
        assert_eq!(unit(1920.0, 1080.0, 1.0), 1.8);
        assert_eq!(unit(3840.0, 2160.0, 2.0), 3.6);
        // a phone held sideways: its points
        assert_eq!(unit(2532.0, 1170.0, 3.0), 3.0);
        assert_eq!(unit(844.0, 390.0, 1.0), 1.0);
        // a small window: 640 units wide at the least
        assert_eq!(unit(400.0, 300.0, 1.0), 400.0 / 640.0);
    }

    fn overlap(a: &Rect, b: &Rect) -> bool {
        a.x < b.x + b.w - 0.5 && b.x < a.x + a.w - 0.5 && a.y < b.y + b.h - 0.5 && b.y < a.y + a.h - 0.5
    }

    /// Every button inside the window and the panel, none over another, and nothing left out for lack of room at a
    /// window of 800 x 600 units or more.
    fn check(m: &Model, w: f32, h: f32) {
        let mut out = Out::new(None, None);
        layout(m, w, h, &mut out);
        let fr = frame(w, h);
        let what = format!("{:?} at {w}x{h}", m.tab);
        assert!(out.hits.iter().any(|(_, h)| *h == Hit::Play), "{what}");
        for (r, hit) in &out.hits {
            assert!(r.w >= 20.0 && r.h >= 20.0, "{what}: {hit:?} too small {r:?}");
            let p = fr.panel;
            assert!(
                r.x >= p.x - 0.5 && r.y >= p.y - 0.5 && r.x + r.w <= p.x + p.w + 0.5 && r.y + r.h <= p.y + p.h + 0.5,
                "{what}: {hit:?} {r:?}"
            );
        }
        for (i, (a, ha)) in out.hits.iter().enumerate() {
            for (b, hb) in &out.hits[i + 1..] {
                assert!(!overlap(a, b), "{what}: {ha:?} {a:?} over {hb:?} {b:?}");
            }
        }
        for p in &out.pieces {
            assert!(p.x >= -0.5 && p.y >= -0.5 && p.x + p.w <= w + 0.5 && p.y + p.h <= h + 0.5, "{what}: piece {p:?}");
        }
        // the tab's own buttons lie inside its room
        let content: Vec<_> = out
            .hits
            .iter()
            .filter(|(_, h)| !matches!(h, Hit::Tab(_) | Hit::Play | Hit::ShowAtStart | Hit::NewsPill | Hit::Coffee))
            .collect();
        for (r, hit) in content {
            let c = fr.content;
            assert!(r.y >= c.y - 0.5 && r.y + r.h <= c.y + c.h + 0.5, "{what}: {hit:?} {r:?} outside {c:?}");
        }
        if w >= 800.0 && h >= 600.0 {
            assert!(out.squeezed.is_empty(), "{what}: {:?}", out.squeezed);
        }
    }

    #[test]
    fn every_tab_fits_from_a_phone_to_a_big_screen() {
        let mut m = Model::sample();
        for (w, h) in SIZES_U {
            for t in Tab::ALL {
                m.tab = t;
                check(&m, w, h);
            }
        }
        // the other cases: the player's own files in use, a browser, a phone (no window), no update check, the tick off
        m = Model {
            mine_in_use: true,
            mine_action: None,
            free_action: Some(Action::Free),
            at_start: false,
            update: None,
            ..Model::sample()
        };
        for (w, h) in SIZES_U {
            for t in Tab::ALL {
                m.tab = t;
                check(&m, w, h);
            }
        }
        m = Model { browser: true, desktop: false, mine_action: None, hd: false, languages: 1, ..Model::sample() };
        for (w, h) in SIZES_U {
            for t in Tab::ALL {
                m.tab = t;
                check(&m, w, h);
            }
        }
    }

    #[test]
    fn the_buttons_are_the_choices_shown() {
        let mut out = Out::new(None, None);
        let m = Model { tab: Tab::Play, ..Model::sample() };
        layout(&m, 800.0, 600.0, &mut out);
        let presets: Vec<usize> = out.hits.iter().filter_map(|(_, h)| if let Hit::Preset(k) = h { Some(*k) } else { None }).collect();
        assert_eq!(presets, vec![0, 1, 2, 3]);
        // the files: the picker on the free graphics; the free graphics with the player's own in use
        let mut out = Out::new(None, None);
        layout(&Model::sample(), 800.0, 600.0, &mut out);
        assert!(out.hits.iter().any(|(_, h)| *h == Hit::Files(Action::Pick)));
        assert!(!out.hits.iter().any(|(_, h)| *h == Hit::Files(Action::Free)));
        // the display: HD only where there is a pack; the window only on the desktop
        let mut out = Out::new(None, None);
        layout(&Model { tab: Tab::Display, hd: false, desktop: false, ..Model::sample() }, 800.0, 600.0, &mut out);
        assert!(!out.hits.iter().any(|(_, h)| matches!(h, Hit::Graphics(Look::Hd) | Hit::Fullscreen(_) | Hit::WindowSize(_))));
        assert!(out.hits.iter().any(|(_, h)| *h == Hit::Graphics(Look::Enhanced)));
        // news: Check for updates only where the build has it
        let mut out = Out::new(None, None);
        layout(&Model { tab: Tab::News, update: None, ..Model::sample() }, 800.0, 600.0, &mut out);
        assert!(!out.hits.iter().any(|(_, h)| *h == Hit::Update));
        assert!(out.hits.iter().any(|(_, h)| *h == Hit::Changelog));
        assert!(out.hits.iter().filter(|(_, h)| *h == Hit::Coffee).count() == 2, "the masthead's link and the news tab's button");
    }

    #[test]
    fn a_preset_button_puts_its_choice_in_use_as_the_welcome_box_does() {
        use crate::extras::Settings;
        use sg_core::extras::Extras;
        // the How to play tab's buttons are the welcome box's choices, in its order
        let mut out = Out::new(None, None);
        layout(&Model { tab: Tab::Play, ..Model::sample() }, 1067.0, 600.0, &mut out);
        let k: Vec<usize> = out.hits.iter().filter_map(|(_, h)| if let Hit::Preset(k) = h { Some(*k) } else { None }).collect();
        assert_eq!(k.len(), crate::welcome::CHOICES.len());
        // and each is put in use as that box puts it (main.rs launcher_act: welcome::pick, then Game options' apply)
        let ui = Settings { update_check: true, ..Settings::ALL_OFF };
        for (i, (p, _, _)) in crate::welcome::CHOICES.iter().enumerate().take(3) {
            let Some(crate::welcome::Pick::Put(e, u)) = crate::welcome::pick(Some(i), Extras::default(), ui, None) else { panic!("{p:?}") };
            assert_eq!(Preset::of(&e, &u), *p);
            assert!(u.update_check, "the kept ticks stay");
        }
        assert_eq!(crate::welcome::pick(Some(3), Extras::default(), ui, None), Some(crate::welcome::Pick::OwnMix));
    }

    #[test]
    fn the_launcher_takes_the_place_of_the_box_due() {
        // a new player's first box: recorded, nothing more to show; the launcher opens on its files
        let mut w = crate::welcome::State::default();
        w.welcome_due = true;
        assert!(!w.launcher_shown(false));
        assert!(!w.welcome_due && !w.news_due);
        // an update's What's new: the launcher opens on its News tab
        let news = crate::welcome::news(None, "0.7.3");
        let mut w = crate::welcome::State::default();
        (w.news_due, w.news) = (true, news);
        assert!(w.launcher_shown(false));
        assert!(!w.news_due);
        // nothing due: nothing
        let mut w = crate::welcome::State::default();
        assert!(!w.launcher_shown(false));
    }

    #[test]
    fn the_english_strings_are_there() {
        for t in Tab::ALL {
            assert_ne!(tr(t.key()), t.key());
        }
        for k in [
            "STR_LAUNCHER_KICKER",
            "STR_LAUNCHER_PLAY",
            "STR_LAUNCHER_SHOW_AT_START",
            "STR_LAUNCHER_BRING_BACK",
            "STR_LAUNCHER_MINE_TITLE",
            "STR_LAUNCHER_MINE_NOTE",
            "STR_LAUNCHER_MINE_NOTE_WEB",
            "STR_LAUNCHER_MINE_BUTTON",
            "STR_LAUNCHER_FREE_TITLE",
            "STR_LAUNCHER_FREE_NOTE",
            "STR_LAUNCHER_FREE_BUTTON",
            "STR_LAUNCHER_BACK_TO_PAGE",
            "STR_LAUNCHER_IN_USE",
            "STR_LAUNCHER_NOT_HERE",
            "STR_LAUNCHER_GAME_OPTIONS",
            "STR_LAUNCHER_DISPLAY_NOTE",
            "STR_LAUNCHER_UPDATE",
            "STR_LAUNCHER_CHANGELOG",
            "STR_LAUNCHER_COFFEE",
            "STR_LAUNCHER_COFFEE_BUTTON",
            "STR_LAUNCHER_NEWS_PILL",
            "STR_LAUNCHER_TITLE_BUTTON",
            "STR_LAUNCHER_FILES_MINE",
            "STR_LAUNCHER_FILES_MINE_QUIET",
            "STR_LAUNCHER_FILES_FREE",
            "STR_LAUNCHER_FILES_FREE_CHOSEN",
            "STR_LAUNCHER_FILES_NONE_FOUND",
            "STR_LAUNCHER_FILES_FOUND",
            "STR_LAUNCHER_FILES_FROM",
            "STR_LAUNCHER_FILES_FROM_PAGE",
        ] {
            assert_ne!(tr(k), k, "lang/en.txt lacks {k}");
        }
    }
}
