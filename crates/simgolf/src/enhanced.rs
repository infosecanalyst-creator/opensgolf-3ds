//! Enhanced graphics (docs/ENHANCED.md): an optional, non-original way of drawing the game for today's large and wide
//! screens, without changing any art. Text drawn from the TrueType fonts at the screen's own pixel size, interface art
//! with premultiplied alpha and mipmaps, the HUD held to the window's corners on wide windows, anisotropic filtering and
//! multisampling for the course, a smooth camera (continuous zoom, animated turns, panning with inertia) and the golfers,
//! balls, animals and fly-overs moved smoothly between the simulation's 87 ms ticks.
//!
//! Classic is the default and stays exactly as it is: every hook elsewhere asks one of this module's switches first
//! ([`native_text`], [`smooth_ui`], [`widescreen`], ...), which are all false while the master switch is off, so with
//! Enhanced off none of the new code runs and every frame is the same, pixel for pixel, as before Enhanced existed.
//! Nothing here touches the simulation: saves, the random generator and the game's rules are the same either way (the
//! rules that depend on the zoom read the nearest of the original's three levels, `App::exe_zoom`).
//!
//! Adding an option: give [`Settings`] a field (with `#[serde(default)]` it reads old settings files), add an
//! [`OptionEntry`] to [`OPTIONS`] with a text key in lang/en.txt, and ask for it through a switch like [`smooth_ui`]. The
//! Preferences screen (System Functions, Preferences, *Enhanced graphics...*) lists every entry of [`OPTIONS`] in order.
use crate::app::App;
use crate::gfx::Gfx;
use crate::ui::{rgba, Screen as Ui};
use serde::{Deserialize, Serialize};
use std::cell::{Cell, RefCell};
use std::collections::HashMap;

/// The settings.json key of the Enhanced settings.
pub const SETTINGS_KEY: &str = "enhanced";

/// The Enhanced settings: the master switch and the options under it. An option only counts while `enabled` is on.
#[derive(Clone, Copy, Debug, PartialEq, Serialize, Deserialize)]
#[serde(default)]
pub struct Settings {
    /// The master switch (off: Classic).
    pub enabled: bool,
    /// Text rasterised from the TrueType fonts at the screen's pixel size (layout unchanged).
    pub native_text: bool,
    /// Interface and sprite art with premultiplied alpha and mipmaps, colour-keyed art with smoothed outlines (edges.rs):
    /// clean edges at any scale.
    pub smooth_ui: bool,
    /// On windows wider (or taller) than 4:3 the HUD goes to the window's corners and edges; full-screen pages are
    /// framed.
    pub widescreen: bool,
    /// Anisotropic filtering of the course's textures (where the graphics driver offers it).
    pub terrain_filter: bool,
    /// 4x multisampling of the frame (desktop).
    pub msaa: bool,
    /// Continuous zoom, animated quarter turns, panning with inertia.
    pub smooth_camera: bool,
    /// Golfers, balls, animals, carts and fly-overs moved smoothly between the simulation's ticks.
    pub smooth_motion: bool,
    /// Photo mode may be opened (the C key, the camera button; photo.rs).
    pub photo_mode: bool,
    // the shader effects (fx.rs), each read through fx::sync
    /// Pixel-art scaling of magnified sprites and interface pictures (fx.rs).
    pub fx_scaler: bool,
    /// Settings files from before `fx_scaler_mode`: the sharp-bilinear filter was chosen (read, cleared when the filter is
    /// chosen again).
    pub fx_scaler_sharp: bool,
    /// The scaler's filter: 0 sharp bilinear, 1 edge-directed, 2 smooth (fx_smooth.rs).
    pub fx_scaler_mode: u8,
    /// Animated water: reflections, sparkle, shore foam.
    pub fx_water: bool,
    /// Per-pixel terrain light, ambient occlusion and soft relief shadows.
    pub fx_terrain_light: bool,
    /// The art's sprite shadows softened.
    pub fx_sprite_shadows: bool,
    /// A sky instead of black beyond the map.
    pub fx_sky: bool,
    /// Tilt-shift blur towards the screen's top and bottom.
    pub fx_tilt_shift: bool,
    /// Colour grading and vignette, per theme.
    pub fx_grading: bool,
    /// Bloom on the brightest highlights.
    pub fx_bloom: bool,
    /// The effects' quality: 0 low, 1 medium, 2 high.
    pub fx_quality: u8,
    /// Buildings, trees and the like cast shadows along the sun (fx_shadow.rs).
    pub fx_cast_shadows: bool,
    /// A soft fading trail behind balls in flight (fx_ball.rs).
    pub fx_ball_trail: bool,
    /// A puff of sand, a splash ring or a flick of grass where a ball lands (fx_ball.rs).
    pub fx_ball_impacts: bool,
    /// Morning, midday, sunset and dusk light following the game's clock (fx_light.rs).
    pub fx_time_of_day: bool,
    /// Drifting cloud shadows (fx_light.rs).
    pub fx_clouds: bool,
    /// Distance haze towards the top of the screen.
    pub fx_haze: bool,
    /// Light rain and darker light on some days.
    pub fx_rain: bool,
    /// Mist pooling in low ground in the morning.
    pub fx_mist: bool,
    /// Trees, buildings and golfers mirrored in the water, sun glitter, shore lapping (fx_reflect.rs).
    pub fx_reflections: bool,
    /// A wind each day: trees, palms and flags sway (fx_nature.rs).
    pub fx_wind: bool,
    /// Lit windows, lamps and fireflies at dusk (fx_night.rs).
    pub fx_evening_lights: bool,
    /// Spring blossom and autumn colours through the game's year (fx_nature.rs).
    pub fx_seasons: bool,
    /// Wet ground, puddles and rainbows after rain (fx_wet.rs).
    pub fx_after_rain: bool,
    /// Lightning in heavy rain (fx_wet.rs).
    pub fx_lightning: bool,
    /// How long a day of light lasts: 0 short (a game month), 1 normal (three months), 2 long (six) (fx_light.rs).
    pub fx_day_length: u8,
    /// Fix the original game's own bugs (docs/FIXES.md): on by default and, unlike every other option here, in Classic too
    /// (the master switch does not gate it); off plays the original's behaviour, bugs included. Crashes are fixed either way.
    pub fix_original_bugs: bool,
    /// The frame rate cap (framerate.rs CAPS: 30, 60, 120, 144, 240, uncapped). Counts in Classic too.
    pub frame_cap: u8,
    /// V-sync (the window's swap interval, from the next start; browsers always have it). Counts in Classic too.
    pub vsync: bool,
}

impl Settings {
    /// Enhanced off, every option ready for when it is turned on.
    pub const DEFAULT: Settings = Settings {
        enabled: false,
        native_text: true,
        smooth_ui: true,
        widescreen: true,
        terrain_filter: true,
        msaa: true,
        smooth_camera: true,
        smooth_motion: true,
        photo_mode: true,
        fx_scaler: true,
        fx_scaler_sharp: false,
        fx_scaler_mode: 2,
        fx_water: true,
        fx_terrain_light: true,
        fx_sprite_shadows: true,
        fx_sky: true,
        fx_tilt_shift: false,
        fx_grading: true,
        fx_bloom: true,
        fx_quality: 1,
        fx_cast_shadows: true,
        fx_ball_trail: true,
        fx_ball_impacts: true,
        fx_clouds: true,
        fx_haze: true,
        // the three that change the mood most are offered, not on by default
        fx_time_of_day: false,
        fx_rain: false,
        fx_mist: false,
        fx_reflections: true,
        fx_wind: true,
        fx_evening_lights: true,
        fx_seasons: true,
        fx_after_rain: true,
        // a strong effect: offered, not on by default
        fx_lightning: false,
        fx_day_length: 1,
        fix_original_bugs: true,
        frame_cap: crate::framerate::UNCAPPED,
        vsync: true,
    };
}

impl Settings {
    /// The scaler's filter (0 sharp bilinear, 1 edge-directed, 2 smooth), an older file's sharp-bilinear choice included.
    pub fn scaler_mode(&self) -> u8 {
        if self.fx_scaler_sharp {
            0
        } else {
            self.fx_scaler_mode.min(2)
        }
    }
}

impl Default for Settings {
    fn default() -> Self {
        Settings::DEFAULT
    }
}

thread_local! {
    static SETTINGS: Cell<Settings> = const { Cell::new(Settings::DEFAULT) };
}

/// The settings in use.
pub fn settings() -> Settings {
    SETTINGS.with(|s| s.get())
}

/// Puts settings in use (not saved; see `App::apply_enhanced`).
pub fn set(s: Settings) {
    SETTINGS.with(|c| c.set(s));
    // the simulation's own fixes follow the same setting (sg_core::fixes)
    sg_core::fixes::set(s.fix_original_bugs);
}

fn on(f: impl Fn(&Settings) -> bool) -> bool {
    let s = settings();
    s.enabled && f(&s)
}

pub fn native_text() -> bool {
    on(|s| s.native_text)
}
pub fn smooth_ui() -> bool {
    on(|s| s.smooth_ui)
}
/// Colour-keyed art (interface pictures, sprites) with smooth outlines at any size (edges.rs): part of the smooth interface
/// art option.
pub fn smooth_edges() -> bool {
    smooth_ui()
}
pub fn widescreen() -> bool {
    on(|s| s.widescreen)
}
pub fn terrain_filter() -> bool {
    on(|s| s.terrain_filter)
}
pub fn msaa() -> bool {
    on(|s| s.msaa)
}
pub fn smooth_camera() -> bool {
    on(|s| s.smooth_camera)
}
pub fn smooth_motion() -> bool {
    on(|s| s.smooth_motion)
}
pub fn photo_mode() -> bool {
    on(|s| s.photo_mode)
}
/// "Fix original bugs" (docs/FIXES.md): not under the master switch, so Classic gets the fixes too unless it is turned off.
pub fn fix_bugs() -> bool {
    settings().fix_original_bugs
}

/// What a change of an option needs before it shows.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Reload {
    /// Nothing: the option is read every frame.
    Nothing,
    /// The art is made again (textures carry the option).
    Art,
}

/// The pages of the Enhanced graphics screen (tabs along its top), by text key.
pub const PAGES: [&str; 4] =
    ["STR_ENHANCED_PAGE_DISPLAY", "STR_ENHANCED_PAGE_EFFECTS", "STR_ENHANCED_PAGE_ATMOSPHERE", "STR_ENHANCED_PAGE_WEATHER"];
pub const DISPLAY: u8 = 0;
pub const EFFECTS: u8 = 1;
pub const ATMOSPHERE: u8 = 2;
pub const WEATHER: u8 = 3;

/// One line of the Enhanced graphics screen.
pub struct OptionEntry {
    /// The settings.json field (for docs and tests).
    pub key: &'static str,
    /// The line's text key (lang/en.txt); the text starts with a space, as every option line of a pop-up box does. A
    /// setting with choices has a {VALUE} in it.
    pub label: &'static str,
    /// The page (tab) it is on: DISPLAY, EFFECTS, ATMOSPHERE or WEATHER.
    pub page: u8,
    /// The text keys of a setting's values when it has more than on and off (a click steps to the next); empty for a tick.
    pub choices: &'static [&'static str],
    pub get: fn(&Settings) -> u8,
    pub set: fn(&mut Settings, u8),
    pub reload: Reload,
}

macro_rules! tick {
    ($key:literal, $label:literal, $page:expr, $field:ident, $reload:expr) => {
        OptionEntry {
            key: $key,
            label: $label,
            page: $page,
            choices: &[],
            get: |s| s.$field as u8,
            set: |s, v| s.$field = v != 0,
            reload: $reload,
        }
    };
    ($key:literal, $label:literal, $page:expr, $field:ident) => {
        tick!($key, $label, $page, $field, Reload::Nothing)
    };
}

/// The options under the master switch, in the order the Enhanced graphics screen lists them on their pages.
pub const OPTIONS: &[OptionEntry] = &[
    tick!("native_text", "STR_ENHANCED_NATIVE_TEXT", DISPLAY, native_text),
    tick!("smooth_ui", "STR_ENHANCED_SMOOTH_UI", DISPLAY, smooth_ui, Reload::Art),
    tick!("widescreen", "STR_ENHANCED_WIDESCREEN", DISPLAY, widescreen),
    tick!("terrain_filter", "STR_ENHANCED_TERRAIN_FILTER", DISPLAY, terrain_filter, Reload::Art),
    tick!("msaa", "STR_ENHANCED_MSAA", DISPLAY, msaa),
    tick!("smooth_camera", "STR_ENHANCED_SMOOTH_CAMERA", DISPLAY, smooth_camera),
    tick!("smooth_motion", "STR_ENHANCED_SMOOTH_MOTION", DISPLAY, smooth_motion),
    tick!("photo_mode", "STR_ENHANCED_PHOTO_MODE", DISPLAY, photo_mode),
    tick!("fix_original_bugs", "STR_ENHANCED_FIX_BUGS", DISPLAY, fix_original_bugs),
    // the port's frame rate settings (framerate.rs): they count with Enhanced graphics off too
    OptionEntry {
        key: "frame_cap",
        label: "STR_ENHANCED_FRAME_CAP",
        page: DISPLAY,
        choices: &[
            "STR_FRAME_CAP_30",
            "STR_FRAME_CAP_60",
            "STR_FRAME_CAP_120",
            "STR_FRAME_CAP_144",
            "STR_FRAME_CAP_240",
            "STR_FRAME_CAP_NONE",
        ],
        get: |s| s.frame_cap.min(crate::framerate::UNCAPPED),
        set: |s, v| s.frame_cap = v.min(crate::framerate::UNCAPPED),
        reload: Reload::Nothing,
    },
    OptionEntry {
        key: "vsync",
        label: if cfg!(target_arch = "wasm32") { "STR_ENHANCED_VSYNC_BROWSER" } else { "STR_ENHANCED_VSYNC" },
        page: DISPLAY,
        choices: &[],
        get: |s| s.vsync as u8,
        // a browser always draws on the display's refresh
        set: |s, v| s.vsync = v != 0 || cfg!(target_arch = "wasm32"),
        reload: Reload::Nothing,
    },
    tick!("fx_scaler", "STR_ENHANCED_FX_SCALER", EFFECTS, fx_scaler),
    OptionEntry {
        key: "fx_scaler_mode",
        label: "STR_ENHANCED_FX_SCALER_MODE",
        page: EFFECTS,
        choices: &["STR_FX_SCALER_SHARP", "STR_FX_SCALER_EDGE", "STR_FX_SCALER_SMOOTH"],
        get: |s| s.scaler_mode(),
        set: |s, v| {
            s.fx_scaler_mode = v.min(2);
            s.fx_scaler_sharp = false;
        },
        reload: Reload::Nothing,
    },
    tick!("fx_water", "STR_ENHANCED_FX_WATER", EFFECTS, fx_water),
    tick!("fx_terrain_light", "STR_ENHANCED_FX_TERRAIN_LIGHT", EFFECTS, fx_terrain_light),
    tick!("fx_sprite_shadows", "STR_ENHANCED_FX_SPRITE_SHADOWS", EFFECTS, fx_sprite_shadows),
    tick!("fx_cast_shadows", "STR_ENHANCED_FX_CAST_SHADOWS", EFFECTS, fx_cast_shadows),
    tick!("fx_ball_trail", "STR_ENHANCED_FX_BALL_TRAIL", EFFECTS, fx_ball_trail),
    tick!("fx_ball_impacts", "STR_ENHANCED_FX_BALL_IMPACTS", EFFECTS, fx_ball_impacts),
    OptionEntry {
        key: "fx_quality",
        label: "STR_ENHANCED_FX_QUALITY",
        page: EFFECTS,
        choices: &["STR_FX_QUALITY_LOW", "STR_FX_QUALITY_MEDIUM", "STR_FX_QUALITY_HIGH"],
        get: |s| s.fx_quality.min(2),
        set: |s, v| s.fx_quality = v.min(2),
        reload: Reload::Nothing,
    },
    tick!("fx_sky", "STR_ENHANCED_FX_SKY", ATMOSPHERE, fx_sky),
    tick!("fx_time_of_day", "STR_ENHANCED_FX_TIME_OF_DAY", ATMOSPHERE, fx_time_of_day),
    OptionEntry {
        key: "fx_day_length",
        label: "STR_ENHANCED_FX_DAY_LENGTH",
        page: ATMOSPHERE,
        choices: &["STR_FX_DAY_SHORT", "STR_FX_DAY_NORMAL", "STR_FX_DAY_LONG"],
        get: |s| s.fx_day_length.min(2),
        set: |s, v| s.fx_day_length = v.min(2),
        reload: Reload::Nothing,
    },
    tick!("fx_evening_lights", "STR_ENHANCED_FX_EVENING_LIGHTS", ATMOSPHERE, fx_evening_lights),
    tick!("fx_clouds", "STR_ENHANCED_FX_CLOUDS", ATMOSPHERE, fx_clouds),
    tick!("fx_haze", "STR_ENHANCED_FX_HAZE", ATMOSPHERE, fx_haze),
    tick!("fx_reflections", "STR_ENHANCED_FX_REFLECTIONS", ATMOSPHERE, fx_reflections),
    tick!("fx_tilt_shift", "STR_ENHANCED_FX_TILT_SHIFT", ATMOSPHERE, fx_tilt_shift),
    tick!("fx_grading", "STR_ENHANCED_FX_GRADING", ATMOSPHERE, fx_grading),
    tick!("fx_bloom", "STR_ENHANCED_FX_BLOOM", ATMOSPHERE, fx_bloom),
    tick!("fx_rain", "STR_ENHANCED_FX_RAIN", WEATHER, fx_rain),
    tick!("fx_after_rain", "STR_ENHANCED_FX_AFTER_RAIN", WEATHER, fx_after_rain),
    tick!("fx_lightning", "STR_ENHANCED_FX_LIGHTNING", WEATHER, fx_lightning),
    tick!("fx_mist", "STR_ENHANCED_FX_MIST", WEATHER, fx_mist),
    tick!("fx_wind", "STR_ENHANCED_FX_WIND", WEATHER, fx_wind),
    tick!("fx_seasons", "STR_ENHANCED_FX_SEASONS", WEATHER, fx_seasons),
];

// ---- the settings file ------------------------------------------------------------------------------------------------------

fn settings_json() -> serde_json::Value {
    sg_core::fsutil::read_file(crate::app::save_dir().join(crate::hd::SETTINGS_FILE))
        .and_then(|d| serde_json::from_slice::<serde_json::Value>(&d).ok())
        .filter(|v| v.is_object())
        .unwrap_or_else(|| serde_json::json!({}))
}

/// The settings saved in settings.json (Enhanced off when there are none).
pub fn saved() -> Settings {
    serde_json::from_value(settings_json()[SETTINGS_KEY].clone()).unwrap_or_default()
}

/// Saves the settings into settings.json, keeping its other keys.
pub fn save(s: &Settings) {
    let mut v = settings_json();
    if let Ok(e) = serde_json::to_value(s) {
        v[SETTINGS_KEY] = e;
    }
    if let Ok(t) = serde_json::to_string_pretty(&v) {
        sg_core::fsutil::write_file(crate::app::save_dir().join(crate::hd::SETTINGS_FILE), t.as_bytes());
    }
}

/// At start up, before any art is loaded: the saved settings, with the master switch from `--enhanced` or
/// `--classic-render` for this run when given (not saved).
pub fn init(force: Option<bool>) {
    let mut s = saved();
    if let Some(on) = force {
        s.enabled = on;
    }
    // test hook (docs/FIXES.md): SG_ORIGINAL_BUGS=1 turns "Fix original bugs" off for this run (not saved), for stills
    if std::env::var_os("SG_ORIGINAL_BUGS").is_some() {
        s.fix_original_bugs = false;
    }
    set(s);
    crate::fx::sync(&s);
    if s.enabled {
        let on: Vec<&str> = OPTIONS.iter().filter(|o| (o.get)(&s) != 0).map(|o| o.key).collect();
        println!("Enhanced graphics: on ({})", on.join(", "));
    }
}

// ---- the Preferences screen -------------------------------------------------------------------------------------------------

/// The options on a page, in order.
pub fn page_options(page: u8) -> impl Iterator<Item = &'static OptionEntry> {
    OPTIONS.iter().filter(move |o| o.page == page)
}

/// The most option lines a page has (every page's box is made this tall, so the box keeps its size when the tab changes).
fn most_options() -> usize {
    (0..PAGES.len() as u8).map(|p| page_options(p).count()).max().unwrap_or(0)
}

/// An option's line as shown: its text, with the current value of a setting with choices.
fn option_line(o: &OptionEntry, s: &Settings) -> String {
    use sg_core::lang::{tr, trf};
    match o.choices.get((o.get)(s) as usize) {
        Some(v) => trf(o.label, &[("VALUE", &tr(v))]),
        None => tr(o.label).to_string(),
    }
}

/// Heading lines of the box above the options: the title, then two empty lines the page tabs are drawn on.
pub const TAB_LINES: usize = 2;

/// The lines of a page of the Enhanced graphics box: the heading, room for the tabs, the master switch, then the page's
/// options, padded with empty lines to the longest page.
pub fn popup_lines(page: u8, s: &Settings) -> Vec<String> {
    use sg_core::lang::tr;
    let mut v = vec![tr("STR_ENHANCED_TITLE").to_string()];
    v.extend((0..TAB_LINES).map(|_| String::new()));
    v.push(tr("STR_ENHANCED_MASTER").to_string());
    v.extend(page_options(page).map(|o| option_line(o, s)));
    let n = 1 + TAB_LINES + 1 + most_options();
    while v.len() < n {
        v.push(String::new());
    }
    v
}

/// The widest line of every page (the box keeps one width whatever the page).
pub fn popup_widest(s: &Settings) -> f32 {
    let f = crate::popup_ui::CHOICE_FONT;
    let lines = (0..PAGES.len() as u8).flat_map(|p| popup_lines(p, s)).map(|l| f.width(&l).round()).fold(0.0, f32::max);
    // wide enough for every tab to hold its name with some room (the tabs share the box's width equally)
    let tab = PAGES.iter().map(|k| f.width(sg_core::lang::tr(k))).fold(0.0, f32::max) + 24.0;
    lines.max(tab * PAGES.len() as f32)
}

/// The box's ticks on a page: bit 0 the master switch, bit 1 + i the page's option i.
pub fn popup_checks(page: u8, s: &Settings) -> u32 {
    page_options(page).enumerate().fold(s.enabled as u32, |m, (i, o)| m | (((o.get)(s) != 0) as u32) << (i + 1))
}

/// The page's lines that are settings with choices, drawn without a tick box (bit 1 + i).
pub fn popup_plain(page: u8) -> u32 {
    page_options(page).enumerate().filter(|(_, o)| !o.choices.is_empty()).fold(0, |m, (i, _)| m | 1 << (i + 1))
}

/// A click on option `k` of a page (0 the master switch): a tick is flipped, a setting with choices steps to the next.
pub fn toggle(page: u8, k: usize, s: &mut Settings) {
    if k == 0 {
        s.enabled = !s.enabled;
        return;
    }
    if let Some(o) = page_options(page).nth(k - 1) {
        let v = (o.get)(s);
        let n = o.choices.len().max(2) as u8;
        (o.set)(s, (v + 1) % n);
    }
}

impl App {
    /// The player's choice from the Enhanced graphics screen: puts the settings in use, saves them, and makes the art
    /// again when an option the textures carry changed. Returns true when the caller has to load the interface art again
    /// (main.rs, load_art).
    pub fn apply_enhanced(&mut self, g: &mut Gfx, new: Settings) -> bool {
        let old = settings();
        if new == old {
            return false;
        }
        set(new);
        save(&new);
        // the water's heights follow "Fix original bugs" (docs/FIXES.md 1)
        if old.fix_original_bugs != new.fix_original_bugs {
            self.rebuild_batches(g);
        }
        // the shader effects follow their ticks (what the terrain's meshes depend on is rebuilt)
        self.fx_set(crate::fx::from_enhanced(&new));
        let art = |s: &Settings| {
            let mut k: Vec<bool> = OPTIONS.iter().filter(|o| o.reload == Reload::Art).map(|o| s.enabled && (o.get)(s) != 0).collect();
            k.push(s.enabled);
            k
        };
        if art(&old) == art(&new) {
            return false;
        }
        self.reload_textures(g);
        true
    }

    /// Drops the course's and the sprites' textures so they are made again with the current settings (the interface art is
    /// loaded again by the caller).
    pub fn reload_textures(&mut self, g: &mut Gfx) {
        crate::hd::free_tracked(g);
        for t in self.textures.drain().filter_map(|(_, t)| t) {
            g.ctx.delete_texture(t);
        }
        self.rebuild_batches(g);
        for s in self.sprites.iter_mut() {
            for t in s.tex.iter_mut().filter_map(Option::take) {
                g.ctx.delete_texture(t);
            }
        }
        for (_, t) in self.red_tex.drain() {
            g.ctx.delete_texture(t);
        }
        for (_, t) in self.outfit_tex.drain() {
            g.ctx.delete_texture(t);
        }
    }
}

// ---- widescreen: the HUD at the window's edges ------------------------------------------------------------------------------

/// Where a piece of the 800 x 600 interface goes on a window of another shape (Enhanced widescreen only; Classic keeps
/// everything in the centred 800 x 600).
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
#[allow(dead_code)] // every edge is offered, not every one is used yet
pub enum Anchor {
    Centre,
    Top,
    Bottom,
    Left,
    Right,
    TopLeft,
    TopRight,
    BottomLeft,
    BottomRight,
}

impl Anchor {
    /// How far (in 800 x 600 units) the anchor moves the interface, for the window's visible area `ext` (left, top, right,
    /// bottom in the same units; the centred 800 x 600 is 0, 0, 800, 600).
    pub fn shift(self, ext: (f32, f32, f32, f32)) -> (f32, f32) {
        use Anchor::*;
        let (l, t, r, b) = ext;
        let x = match self {
            Left | TopLeft | BottomLeft => l,
            Right | TopRight | BottomRight => r - 800.0,
            _ => 0.0,
        };
        let y = match self {
            Top | TopLeft | TopRight => t,
            Bottom | BottomLeft | BottomRight => b - 600.0,
            _ => 0.0,
        };
        (x, y)
    }
}

/// A rectangle drawn this frame (in 800 x 600 units as it shows on the window) and the shift it was drawn with.
#[derive(Clone, Copy, Debug)]
struct Region {
    x0: f32,
    y0: f32,
    x1: f32,
    y1: f32,
    dx: f32,
    dy: f32,
}

thread_local! {
    /// The interface rectangles of the frame being drawn and of the last whole frame, in drawing order.
    static REGIONS: RefCell<(Vec<Region>, Vec<Region>)> = const { RefCell::new((Vec::new(), Vec::new())) };
}

/// Starts a frame's list of interface rectangles.
pub fn begin_frame() {
    REGIONS.with(|r| r.borrow_mut().0.clear());
}

/// Ends a frame: its rectangles become the ones the pointer is tested against until the next frame ends.
pub fn end_frame() {
    REGIONS.with(|r| {
        let mut r = r.borrow_mut();
        let (cur, last) = &mut *r;
        std::mem::swap(cur, last);
    });
}

/// Remembers an interface rectangle drawn with a shift (widescreen only).
pub fn record(x0: f32, y0: f32, x1: f32, y1: f32, shift: (f32, f32)) {
    let (x0, x1) = (x0.min(x1), x0.max(x1));
    let (y0, y1) = (y0.min(y1), y0.max(y1));
    REGIONS.with(|r| {
        let v = &mut r.borrow_mut().0;
        // a run of pieces with the same shift that touch is kept as one rectangle (a panel's many cuts)
        if let Some(l) = v.last_mut() {
            if l.dx == shift.0 && l.dy == shift.1 && x0 >= l.x0 && x1 <= l.x1 && y0 >= l.y0 && y1 <= l.y1 {
                return;
            }
        }
        if v.len() < 20000 {
            v.push(Region { x0, y0, x1, y1, dx: shift.0, dy: shift.1 });
        }
    });
}

/// The 800 x 600 point a pointer at (vx, vy) (centred 800 x 600 units) means: under a piece of the interface drawn at a
/// window edge, the point in that piece's own layout; elsewhere the point itself. The topmost piece (drawn last) wins.
pub fn unshift(vx: f32, vy: f32) -> (f32, f32) {
    REGIONS.with(|r| {
        let r = r.borrow();
        r.1.iter().rev().find(|g| vx >= g.x0 && vx < g.x1 && vy >= g.y0 && vy < g.y1).map(|g| (vx - g.dx, vy - g.dy)).unwrap_or((vx, vy))
    })
}

/// The frame round a full-screen 800 x 600 page on a wider (or taller) window: a dark green gradient in the bars (over the
/// course, translucent so the course shows dimly through) and a soft shadow along the page's edges.
pub fn backdrop(g: &mut Gfx, s: &Ui, over_world: bool) {
    let (l, t, r, b) = s.extent();
    if l > -0.5 && t > -0.5 && r < 800.5 && b < 600.5 {
        return;
    }
    let a = if over_world { 0.86 } else { 1.0 };
    let top = rgba(0.09, 0.15, 0.11, a);
    let bottom = rgba(0.02, 0.04, 0.03, a);
    let mut bar = |x: f32, y: f32, w: f32, h: f32| {
        if w <= 0.0 || h <= 0.0 {
            return;
        }
        // the gradient runs down the whole window
        let k0 = (y - t) / (b - t).max(1.0);
        let k1 = (y + h - t) / (b - t).max(1.0);
        let mix = |k: f32| {
            let mut c = [0.0; 4];
            for i in 0..4 {
                c[i] = top[i] + (bottom[i] - top[i]) * k;
            }
            c
        };
        s.gradient(g, x, y, w, h, mix(k0), mix(k1));
    };
    bar(l, t, -l, b - t);
    bar(800.0, t, r - 800.0, b - t);
    bar(0.0, t, 800.0, -t);
    bar(0.0, 600.0, 800.0, b - 600.0);
    // the page's shadow: 16 units fading out from its edges
    for i in 0..8 {
        let k = i as f32 * 2.0;
        let c = rgba(0.0, 0.0, 0.0, 0.07 * (8 - i) as f32 / 8.0 * a);
        if l < -0.5 {
            s.fill(g, -k - 2.0, 0.0, 2.0, 600.0, c);
            s.fill(g, 800.0 + k, 0.0, 2.0, 600.0, c);
        }
        if t < -0.5 {
            s.fill(g, 0.0, -k - 2.0, 800.0, 2.0, c);
            s.fill(g, 0.0, 600.0 + k, 800.0, 2.0, c);
        }
    }
}

// ---- the smooth camera and smooth motion ------------------------------------------------------------------------------------

/// Where a golfer, animal or fly-over was at the last two simulation ticks.
#[derive(Clone, Copy, Debug)]
struct Track {
    prev: [f32; 3],
    cur: [f32; 3],
    seen: u32,
}

/// The App's Enhanced state: the camera as shown, and the positions of the moving things at the last two ticks.
#[derive(Default)]
pub struct EnhState {
    /// The zoom as shown (eases towards `App::zoom`); 0 until the first frame.
    pub view_zoom: f32,
    /// Degrees still to turn to reach `App::rot` (an animated quarter turn).
    pub rot_left: f32,
    /// The pan still to make (right, up in world units) from an arrow key.
    pub pan_left: (f32, f32),
    /// The drag's speed (right, up in world units per second), carried on after the button is let go.
    pub pan_vel: (f32, f32),
    pub dragging: bool,
    last_drag: f64,
    last_frame: f64,
    /// The tick the tracks were taken at.
    tick: u32,
    tracks: HashMap<u32, Track>,
    balls: Vec<Option<Track>>,
}

/// Seconds for an eased move to get about two thirds of the way.
const EASE: f32 = 0.09;

fn ease(dt: f32) -> f32 {
    1.0 - (-dt / EASE).exp()
}

impl App {
    /// Whether the camera eases: with the smooth camera, and always in photo mode (photo.rs).
    fn smooth_cam(&self) -> bool {
        smooth_camera() || self.photo.is_some()
    }

    /// The zoom the course is drawn at: the eased one with the smooth camera, else the zoom itself.
    pub fn view_zoom(&self) -> f32 {
        if self.smooth_cam() && self.enh.view_zoom > 0.0 {
            self.enh.view_zoom
        } else {
            self.zoom
        }
    }

    /// The camera's turn as drawn (degrees): part way through an animated quarter turn with the smooth camera.
    pub fn view_rot(&self) -> f32 {
        if self.smooth_cam() {
            self.rot - self.enh.rot_left
        } else {
            self.rot
        }
    }

    /// Moves the shown camera towards the real one, once a frame (smooth camera only).
    pub fn camera_frame(&mut self, now: f64) {
        let dt = if self.enh.last_frame > 0.0 { (now - self.enh.last_frame).clamp(0.0, 0.1) as f32 } else { 0.0 };
        self.enh.last_frame = now;
        if !self.smooth_cam() {
            self.enh.view_zoom = self.zoom;
            self.enh.rot_left = 0.0;
            self.enh.pan_left = (0.0, 0.0);
            self.enh.pan_vel = (0.0, 0.0);
            return;
        }
        let k = ease(dt);
        // zoom: eased in log space, so each doubling takes the same time
        if self.enh.view_zoom <= 0.0 {
            self.enh.view_zoom = self.zoom;
        }
        let (a, b) = (self.enh.view_zoom.ln(), self.zoom.ln());
        self.enh.view_zoom = if (a - b).abs() < 1e-3 { self.zoom } else { (a + (b - a) * k).exp() };
        // the turn
        self.enh.rot_left -= self.enh.rot_left * k;
        if self.enh.rot_left.abs() < 0.05 {
            self.enh.rot_left = 0.0;
        }
        // an arrow key's scroll
        let (pr, pu) = self.enh.pan_left;
        if pr != 0.0 || pu != 0.0 {
            let (sr, su) = (pr * k, pu * k);
            self.pan(sr, su);
            self.enh.pan_left = (pr - sr, pu - su);
            if (pr - sr).hypot(pu - su) < 0.05 {
                self.pan(pr - sr, pu - su);
                self.enh.pan_left = (0.0, 0.0);
            }
        }
        // the drag's inertia, once let go
        if !self.enh.dragging {
            let (vr, vu) = self.enh.pan_vel;
            if vr != 0.0 || vu != 0.0 {
                self.pan(vr * dt, vu * dt);
                let f = (-dt * 4.0).exp();
                self.enh.pan_vel = (vr * f, vu * f);
                if self.enh.pan_vel.0.hypot(self.enh.pan_vel.1) < 2.0 {
                    self.enh.pan_vel = (0.0, 0.0);
                }
            }
        }
    }

    /// A drag of the course by (right, up) world units: panned at once, its speed kept for the inertia.
    pub fn drag_pan(&mut self, right: f32, up: f32, now: f64) {
        self.pan(right, up);
        if !self.smooth_cam() {
            return;
        }
        let dt = (now - self.enh.last_drag).max(1e-3) as f32;
        self.enh.last_drag = now;
        if dt > 0.1 {
            self.enh.pan_vel = (0.0, 0.0);
            return;
        }
        let v = (right / dt, up / dt);
        // smoothed over the last few motion events
        self.enh.pan_vel = (self.enh.pan_vel.0 * 0.5 + v.0 * 0.5, self.enh.pan_vel.1 * 0.5 + v.1 * 0.5);
    }

    /// The drag starts (true) or ends (false); a drag that stopped before it ended carries no inertia.
    pub fn drag_state(&mut self, down: bool, now: f64) {
        self.enh.dragging = down;
        if down || now - self.enh.last_drag > 0.08 {
            self.enh.pan_vel = (0.0, 0.0);
        }
    }

    /// An arrow key's scroll: eased with the smooth camera, at once otherwise.
    pub fn key_pan(&mut self, right: f32, up: f32) {
        if self.smooth_cam() {
            self.enh.pan_left.0 += right;
            self.enh.pan_left.1 += up;
        } else {
            self.pan(right, up);
        }
    }

    /// A quarter turn has just been made: the shown camera starts from where it was.
    pub fn turned(&mut self, degrees: f32) {
        if self.smooth_cam() {
            // the short way round
            self.enh.rot_left = (self.enh.rot_left + degrees + 180.0).rem_euclid(360.0) - 180.0;
        }
    }

    /// The mouse wheel or a pinch with the smooth camera: the zoom moves continuously between the original's closest and
    /// farthest levels (a wheel notch is a quarter of a doubling); in photo mode further both ways (photo.rs).
    pub fn zoom_by(&mut self, doublings: f32) {
        use crate::app::ZOOM_UNIT;
        let (lo, hi) = if self.photo.is_some() { crate::photo::ZOOM_RANGE } else { (ZOOM_UNIT, ZOOM_UNIT * 4.0) };
        self.zoom = (self.zoom * doublings.exp2()).clamp(lo, hi);
    }

    /// The simulation advanced: the positions at this tick become the newest of the two kept (smooth motion only). Called
    /// after `update_props` has pointed the props at the simulation.
    pub fn track_motion(&mut self) {
        if !smooth_motion() {
            self.enh.tracks.clear();
            self.enh.balls.clear();
            return;
        }
        let tick = self.game_tick;
        let advanced = tick != self.enh.tick;
        self.enh.tick = tick;
        // a jump of more than two tiles in a tick is a move to a new place, not a walk
        let far = 2.0 * sg_core::terrain::TILE_SIZE;
        let mut seen = HashMap::with_capacity(self.enh.tracks.len());
        for p in &self.props {
            let Some(key) = track_key(p) else { continue };
            if p.hidden {
                continue;
            }
            let now = [p.x, p.z, p.lift];
            let t = match self.enh.tracks.get(&key) {
                Some(t) if advanced => {
                    let jump = (now[0] - t.cur[0]).hypot(now[1] - t.cur[1]);
                    Track { prev: if jump > far { now } else { t.cur }, cur: now, seen: tick }
                }
                Some(t) => Track { cur: now, ..*t },
                None => Track { prev: now, cur: now, seen: tick },
            };
            seen.insert(key, t);
        }
        self.enh.tracks = seen;
        // the balls, by golfer (in the simulation's units: x, y along the ground and the height)
        let n = self.club.g.len();
        self.enh.balls.resize(n, None);
        for i in 0..n {
            let gl = &self.club.g[i];
            if gl.bx == 0 || gl.hole <= 0 || gl.hole >= 19 {
                self.enh.balls[i] = None;
                continue;
            }
            let now = [gl.bx as f32, gl.by as f32, gl.bz as f32];
            self.enh.balls[i] = Some(match self.enh.balls[i] {
                Some(t) if advanced => {
                    let jump = (now[0] - t.cur[0]).hypot(now[1] - t.cur[1]);
                    Track { prev: if jump > 8.0 * 1024.0 { now } else { t.cur }, cur: now, seen: tick }
                }
                Some(t) => Track { cur: now, ..t },
                None => Track { prev: now, cur: now, seen: tick },
            });
        }
    }

    /// How far the simulation is between its last tick and the next (0 to 1).
    pub(crate) fn tick_fraction(&self) -> f32 {
        let tick_s = sg_core::flight::TICK_MS as f64 / 1000.0;
        (self.tick_acc / tick_s).clamp(0.0, 1.0) as f32
    }

    /// Where a prop is drawn: between its places at the last two ticks with smooth motion, else where it is.
    pub fn shown_at(&self, p: &crate::app::Prop) -> (f32, f32, f32) {
        if smooth_motion() {
            if let Some(t) = track_key(p).and_then(|k| self.enh.tracks.get(&k)) {
                let f = self.tick_fraction();
                let l = |i: usize| t.prev[i] + (t.cur[i] - t.prev[i]) * f;
                return (l(0), l(1), l(2));
            }
        }
        (p.x, p.z, p.lift)
    }

    /// Where golfer `i`'s ball is drawn, in the simulation's units (x, y, height).
    pub fn shown_ball(&self, i: usize) -> Option<(f32, f32, f32)> {
        let t = self.enh.balls.get(i).copied().flatten().filter(|_| smooth_motion())?;
        let f = self.tick_fraction();
        let l = |k: usize| t.prev[k] + (t.cur[k] - t.prev[k]) * f;
        let _ = t.seen;
        Some((l(0), l(1), l(2)))
    }
}

/// The identity a moving prop keeps from tick to tick: its golfer, employee, resident, or the ambient animal or fly-over
/// it was made for.
fn track_key(p: &crate::app::Prop) -> Option<u32> {
    if let Some(g) = p.golfer {
        Some(0x1000_0000 | g as u32)
    } else if let Some(e) = p.employee {
        Some(0x2000_0000 | e as u32)
    } else if let Some(r) = p.resident {
        Some(0x3000_0000 | r as u32)
    } else if p.track != 0 {
        Some(p.track)
    } else {
        None
    }
}

// ---- texture filtering ------------------------------------------------------------------------------------------------------

/// Turns on anisotropic filtering (up to 8x) for a mipmapped texture where the driver has GL_EXT_texture_filter_anisotropic
/// (or OpenGL 4.6). Skipped in browsers and on phones, and wherever the driver does not know the setting.
pub fn anisotropy(g: &mut Gfx, tex: miniquad::TextureId) {
    #[cfg(not(any(target_arch = "wasm32", target_os = "android", target_os = "ios")))]
    {
        use miniquad::gl;
        const TEXTURE_2D: u32 = 0x0DE1;
        const TEXTURE_BINDING_2D: u32 = 0x8069;
        const MAX_ANISOTROPY: u32 = 0x84FE;
        const MAX_MAX_ANISOTROPY: u32 = 0x84FF;
        thread_local! {
            static MAX: Cell<i32> = const { Cell::new(-1) };
        }
        #[allow(irrefutable_let_patterns)]
        let miniquad::RawId::OpenGl(id) = (unsafe { g.ctx.texture_raw_id(tex) }) else {
            return;
        };
        unsafe {
            let mut max = MAX.with(|m| m.get());
            if max < 0 {
                while gl::glGetError() != 0 {}
                let mut v = 0;
                gl::glGetIntegerv(MAX_MAX_ANISOTROPY, &mut v);
                max = if gl::glGetError() == 0 { v } else { 0 };
                MAX.with(|m| m.set(max));
                if max > 1 {
                    println!("Enhanced graphics: anisotropic filtering {}x", max.min(8));
                }
            }
            if max <= 1 {
                return;
            }
            // miniquad keeps track of the bound texture: put back what was bound
            let mut prev = 0;
            gl::glGetIntegerv(TEXTURE_BINDING_2D, &mut prev);
            gl::glBindTexture(TEXTURE_2D, id);
            gl::glTexParameterf(TEXTURE_2D, MAX_ANISOTROPY, max.min(8) as f32);
            gl::glBindTexture(TEXTURE_2D, prev as u32);
        }
    }
    #[cfg(any(target_arch = "wasm32", target_os = "android", target_os = "ios"))]
    let _ = (g, tex);
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn pages_toggle() {
        let mut s = Settings::DEFAULT;
        s.msaa = false;
        // every option is on one page, and its tick is the bit the box shows
        for p in 0..PAGES.len() as u8 {
            for (i, o) in page_options(p).enumerate() {
                let before = (o.get)(&s);
                toggle(p, i + 1, &mut s);
                assert_ne!((o.get)(&s), before, "{}", o.key);
                if o.choices.is_empty() {
                    assert_eq!(popup_checks(p, &s) >> (i + 1) & 1, ((o.get)(&s) != 0) as u32);
                }
            }
            assert_eq!(popup_lines(p, &s).len(), popup_lines(0, &s).len());
        }
        toggle(ATMOSPHERE, 0, &mut s);
        assert!(s.enabled);
        // quality steps round its three values
        let q = page_options(EFFECTS).position(|o| o.key == "fx_quality").unwrap() + 1;
        let mut t = Settings::DEFAULT;
        for want in [2, 0, 1] {
            toggle(EFFECTS, q, &mut t);
            assert_eq!(t.fx_quality, want);
        }
    }

    #[test]
    fn off_means_every_option_off() {
        set(Settings::DEFAULT);
        assert!(!native_text() && !smooth_ui() && !widescreen() && !terrain_filter() && !msaa());
        assert!(!smooth_camera() && !smooth_motion());
        // the bug fixes are not under the master switch
        assert!(fix_bugs());
    }

    #[test]
    fn anchors_reach_the_edges() {
        // 16:9 at 1920 x 1080: the visible area is 133.3 units wider each side
        let ext = (-133.0, 0.0, 933.0, 600.0);
        assert_eq!(Anchor::TopLeft.shift(ext), (-133.0, 0.0));
        assert_eq!(Anchor::BottomRight.shift(ext), (133.0, 0.0));
        assert_eq!(Anchor::Top.shift(ext), (0.0, 0.0));
    }

    #[test]
    fn option_keys_are_settings_fields() {
        let v = serde_json::to_value(Settings::DEFAULT).unwrap();
        for o in OPTIONS {
            assert!(v.get(o.key).is_some(), "{}", o.key);
        }
    }

    #[test]
    fn old_settings_files_read() {
        let s: Settings = serde_json::from_str(r#"{"enabled": true}"#).unwrap();
        assert!(s.enabled && s.native_text);
    }
}
