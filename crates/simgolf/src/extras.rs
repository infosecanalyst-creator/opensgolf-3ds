//! OpenSGolf extras (docs/PLAYING.md, "OpenSGolf extras"): the port's own additions to the interface, none of them in the
//! original. Each is purely additive (it adds a screen, a tag or a strip; nothing of the original's is taken away or
//! changed); a new player starts with them off (the original game) and the presets (Select Difficulty's Game box, the top
//! line of the Extras box: extras_ui.rs `Preset`) turn them on in one click; Preferences > *OpenSGolf extras...* has a
//! tick for each (the Info page groups some into one line), kept in settings.json under "ui_extras", and `--no-extras`
//! turns them all off for one run (not saved). The update check is a plain Preferences line, not in the Extras box.
//!
//! * the People screen (people_ui.rs): every golfer on the course and every employee, sortable, with Locate, Follow, the
//!   golfer card, the golfer's profile page, and for the staff Hire, Fire, Move and Rename, and the player's pro's screens;
//! * the message history (history_ui.rs): every ticker message kept with its date and speaker, saved with the game;
//! * name tags under the pointer (tags_ui.rs): the name and a mood face of the golfer or employee under the pointer;
//! * tabbed reports (report_tabs.rs): a strip of tabs over the F1 to F10 reports to flip between them;
//! * path help (pathview_ui.rs): a path view marking buildings and holes the paths do not reach, and a Show button on
//!   messages about a place;
//! * autosave (autosave.rs): the game saved every five minutes of play and as its window goes to the background, as
//!   "autosave (Latest)" in the Load list;
//! * save pictures (files_ui.rs): a map of the course beside the Load list's saves;
//! * course sharing (share.rs): course codes and shared files for Play a Championship, with the sharer's record to beat;
//! * pause in the background and pause on big events (comfort.rs): the game and its sound stop while its window is in the
//!   background, or the game stops when a VIP arrives, a famous golfer challenges, the SGA offers a tournament, the board warns or cash goes below
//!   zero. The presets leave these two as the player has them (`Settings::with_kept`): some players leave the game
//!   running in the background on purpose.
//! * the landing map (insight.rs): where golfers' tee shots and approaches on a hole come to rest, as dots on the course;
//! * comments by hole (insight.rs): each hole's most frequent thoughts this month and last month, on a page of their own.
//! * check for updates when the game starts (update.rs): the title menu's Check for updates, once at start, saying
//!   something only when a newer version is out. A plain Preferences line, which the presets leave alone: it goes
//!   online, which a player chooses for themselves.
//!
//! * history graphs (graphs.rs): the club's and each hole's months as line graphs on a page of their own (Shift+Y).
//! * zoom towards the pointer (enhanced.rs zoom_anchor): the wheel keeps the ground under the pointer where it is. Set by
//!   the Change keys' controls presets (controls.rs `Preset`), and left alone by the extras presets: it changes how the
//!   original's zoom feels.
//! * drag to place (build_tools.rs row_*): a drag with a garden item or a ball washer lays a row of them from the press to
//!   the release. Set by the Change keys' controls presets, and left alone by the extras presets, like zooming towards
//!   the pointer: it changes what the original's drag does.
//!
//! * live scorecards (scorecard_ui.rs): the golfer card's golfer and partner on the Player panel's round scorecard, with
//!   pars and the score against par so far;
//! * the clickable ticker (message_ui.rs): a click on a message about a golfer or a place takes the camera there (and
//!   follows the golfer);
//! * the year in numbers (year_ui.rs): a page after the original's End of Year report, the year's figures against the year
//!   before;
//! * home-site details (homes_ui.rs): what a home site under the pointer cost, brought in and is worth, and the hole that
//!   lifts its value.
//!
//! * right-drag panning (main.rs right_down): a drag with the right button moves the course, as OpenSGolf did up to 0.4.7;
//!   a right click that does not move is still the original's (undo, putting a tool away). Set by the controls presets
//!   like zooming towards the pointer: it changes what the original's right button does.
//! * random trees (app.rs painted_tile, place_tile_item): each tree the woods, pine and palm brushes plant takes a random
//!   one of its type's looks (size, and for pines the species), and each scenic tree placed a random one of the seven,
//!   instead of the tool's one look until Tab steps it. A look (the Extras box's Looks page): only which of the original's
//!   pictures each tree gets changes. The facing is not chosen: the original turns every tree by its tile's place, and nothing
//!   per tile holds a facing.
//! * import and export saved games (files_ui.rs file_buttons): Load Previous Game's *Import a save file...* and *Export
//!   this save...*, for moving games between devices (issue #49). Up to 0.4.9 they came with Course sharing, so ticks
//!   saved before it follow that tick.
//!
//! * retired: the desert's Ravine and Brush art (0.5.6 to 0.5.13, issue #89), a look that traded the two tiles' pictures
//!   in the desert theme. The pictures were the original's all along: the desert's tile 10 is its Brush (the dry weeds)
//!   and tile 11 its Ravine (the crack), the names the exe's theme loader gives them, which the port had the other way
//!   round (issue #117, panels_ui.rs `tile_key`). Its bit (23) and `settings.json` key stay so that older ticks load; the
//!   key is read and ignored, and no page shows it.
//!
//! * mouse aim and swing (touch.rs): with the mouse, the pro's shot is aimed and taken as by touch: a click sets the
//!   target, a drag from it moves it, a drag elsewhere looks round, and the round Swing button shoots. Off, a click on the
//!   course aims and shoots at once as in the original. Left alone by the presets, as the controls presets' ticks are: it
//!   changes what the original's click does.
//!
//! * right click puts the tool away (main.rs right_click): with a tool in hand (a terrain brush, the elevation tools,
//!   a building), a right click on the course puts it down, as Space does, instead of the original's: a terrain brush's
//!   right click is the undo's, which twice on a hole's tee or green takes the hole away. Left alone by the presets.
//!
//! * the interface at the screen's edges (enhanced.rs widescreen): on a wide screen with Classic graphics, the HUD goes
//!   to the window's edges as Enhanced graphics' Widescreen tick puts it, instead of staying in the 800 x 600 middle. A
//!   look: the art and the game are the same. On by default and left alone by the presets (the original only ran at
//!   4:3, where it changes nothing).
//!
//! * home names on hover (homes_ui.rs draw_home_labels): the celebrities' names over their home sites show only while the
//!   pointer is on the lot, instead of the original's taking turns over every lot at the closest zoom (issue #160). A look,
//!   on the Looks page; left alone by the presets, as it takes the original's labels away.
//!
//! With every extra off the game draws and plays exactly as before they existed (each hook asks its switch first).
use serde::{Deserialize, Serialize};
use std::cell::Cell;

/// The settings.json key of the extras' ticks.
pub const SETTINGS_KEY: &str = "ui_extras";

#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(default)]
pub struct Settings {
    pub people: bool,
    pub history: bool,
    pub name_tags: bool,
    pub report_tabs: bool,
    pub path_help: bool,
    pub autosave: bool,
    pub save_pictures: bool,
    pub sharing: bool,
    pub pause_background: bool,
    pub pause_events: bool,
    pub landing_map: bool,
    pub hole_comments: bool,
    /// Check for updates when the game starts: on from 0.6 (stored as "check_updates", which reads as on when missing, so
    /// every player gets it once; the old "update_check" key, off by default before, is no longer read).
    #[serde(rename = "check_updates", default = "on")]
    pub update_check: bool,
    pub graphs: bool,
    pub zoom_pointer: bool,
    pub drag_place: bool,
    pub live_scorecards: bool,
    pub ticker_jump: bool,
    pub year_summary: bool,
    pub home_details: bool,
    pub right_drag: bool,
    pub random_trees: bool,
    pub save_files: bool,
    pub desert_art: bool,
    pub mouse_swing: bool,
    /// A right click puts the tool in hand away (OpenSGolf extras; the original's right click with a terrain brush is the
    /// undo's, main.rs right_click).
    #[serde(default)]
    pub right_drop: bool,
    /// The HUD at the window's edges on a wide screen with Classic graphics too (enhanced.rs widescreen; Enhanced graphics
    /// has its own Widescreen tick). On by default (missing reads as on, as the update check): the original only ran at
    /// 4:3, where nothing moves, so there is no original wide-screen layout to keep.
    #[serde(default = "on")]
    pub wide_ui: bool,
    /// The pointer at the screen's very edge scrolls the view, as in the original (main.rs edge_scroll; full screen only,
    /// where the pointer cannot leave the window). On by default (missing reads as on): it is the original's.
    #[serde(default = "on")]
    pub edge_scroll: bool,
    /// The celebrities' home names only under the pointer (homes_ui.rs draw_home_labels; the port's own, issue #160).
    #[serde(default)]
    pub home_labels_hover: bool,
    /// Hole Stats' Fly over button: a short camera flight down the hole (showcase.rs). Off by default.
    pub hole_flyover: bool,
}

impl Settings {
    /// The interface extras of the "Original + helpers" and "OpenSGolf" presets (extras_ui.rs `Preset`): every helper and
    /// look, without the ticks the presets leave as the player has them (`Settings::kept`: the two pauses, which change when
    /// the game runs, the update check, which goes online, the three the Change keys' controls presets set: zoom towards
    /// the pointer, drag to place, right-drag panning, mouse aim and swing, right click puts the tool away, and the interface at
    /// the screen's edges, on by default). Those are false here.
    pub const HELPERS: Settings = Settings {
        people: true,
        history: true,
        name_tags: true,
        report_tabs: true,
        path_help: true,
        autosave: true,
        save_pictures: true,
        sharing: true,
        pause_background: false,
        pause_events: false,
        landing_map: true,
        hole_comments: true,
        update_check: false,
        graphs: true,
        zoom_pointer: false,
        drag_place: false,
        live_scorecards: true,
        ticker_jump: true,
        year_summary: true,
        home_details: true,
        right_drag: false,
        random_trees: true,
        save_files: true,
        desert_art: false,
        mouse_swing: false,
        right_drop: false,
        wide_ui: false,
        edge_scroll: false,
        home_labels_hover: false,
        hole_flyover: true,
    };
    pub const ALL_OFF: Settings = Settings {
        people: false,
        history: false,
        name_tags: false,
        report_tabs: false,
        path_help: false,
        autosave: false,
        save_pictures: false,
        sharing: false,
        pause_background: false,
        pause_events: false,
        landing_map: false,
        hole_comments: false,
        update_check: false,
        graphs: false,
        zoom_pointer: false,
        drag_place: false,
        live_scorecards: false,
        ticker_jump: false,
        year_summary: false,
        home_details: false,
        right_drag: false,
        random_trees: false,
        save_files: false,
        desert_art: false,
        mouse_swing: false,
        right_drop: false,
        wide_ui: false,
        edge_scroll: false,
        home_labels_hover: false,
        hole_flyover: false,
    };

    /// The ticks as the Extras box's checkbox bits (in `UI_KEYS` order).
    pub fn bits(&self) -> u32 {
        [
            self.people,
            self.history,
            self.name_tags,
            self.report_tabs,
            self.path_help,
            self.autosave,
            self.save_pictures,
            self.sharing,
            self.pause_background,
            self.pause_events,
            self.landing_map,
            self.hole_comments,
            self.update_check,
            self.graphs,
            self.zoom_pointer,
            self.drag_place,
            self.live_scorecards,
            self.ticker_jump,
            self.year_summary,
            self.home_details,
            self.right_drag,
            self.random_trees,
            self.save_files,
            self.desert_art,
            self.mouse_swing,
            self.right_drop,
            self.wide_ui,
            self.edge_scroll,
            self.home_labels_hover,
            self.hole_flyover,
        ]
        .iter()
        .enumerate()
        .fold(0, |m, (i, &on)| m | (on as u32) << i)
    }

    pub fn from_bits(m: u32) -> Settings {
        let on = |i: u32| m & 1 << i != 0;
        Settings {
            people: on(0),
            history: on(1),
            name_tags: on(2),
            report_tabs: on(3),
            path_help: on(4),
            autosave: on(5),
            save_pictures: on(6),
            sharing: on(7),
            pause_background: on(8),
            pause_events: on(9),
            landing_map: on(10),
            hole_comments: on(11),
            update_check: on(12),
            graphs: on(13),
            zoom_pointer: on(14),
            drag_place: on(15),
            live_scorecards: on(16),
            ticker_jump: on(17),
            year_summary: on(18),
            home_details: on(19),
            right_drag: on(20),
            random_trees: on(21),
            save_files: on(22),
            desert_art: on(23),
            mouse_swing: on(24),
            right_drop: on(25),
            wide_ui: on(26),
            edge_scroll: on(27),
            home_labels_hover: on(28),
            hole_flyover: on(29),
        }
    }

    /// The ticks a preset sets (extras_ui.rs `Preset`): all but the kept ones (`with_kept`), which are false here.
    pub fn preset_part(&self) -> Settings {
        self.with_kept(&Settings::ALL_OFF)
    }

    /// These ticks with the ones a preset leaves alone taken from `from`: the two pauses, the update check (a plain
    /// Preferences line, not in the Extras box) and the controls presets' ticks (zoom towards the pointer, drag to place,
    /// right-drag panning), and mouse aim and swing, which changes what the original's click does, like them.
    pub fn with_kept(&self, from: &Settings) -> Settings {
        Settings {
            pause_background: from.pause_background,
            pause_events: from.pause_events,
            update_check: from.update_check,
            zoom_pointer: from.zoom_pointer,
            drag_place: from.drag_place,
            right_drag: from.right_drag,
            mouse_swing: from.mouse_swing,
            right_drop: from.right_drop,
            wide_ui: from.wide_ui,
            edge_scroll: from.edge_scroll,
            home_labels_hover: from.home_labels_hover,
            ..*self
        }
    }
}

/// Every tick off, as a new player starts and as ticks missing from settings.json read (`saved`).
impl Default for Settings {
    fn default() -> Self {
        Settings::ALL_OFF
    }
}

/// The interface extras' lines on the Extras box's Interface page (extras_ui.rs), one tick per extra in `Settings::bits`
/// order (text keys in lang/en.txt).
pub const UI_KEYS: [&str; 30] = [
    "STR_EXTRAS_PEOPLE",
    "STR_EXTRAS_HISTORY",
    "STR_EXTRAS_NAME_TAGS",
    "STR_EXTRAS_REPORT_TABS",
    "STR_EXTRAS_PATH_HELP",
    "STR_EXTRAS_AUTOSAVE",
    "STR_EXTRAS_SAVE_PICTURES",
    "STR_EXTRAS_SHARING",
    "STR_EXTRAS_PAUSE_BACKGROUND",
    "STR_EXTRAS_PAUSE_EVENTS",
    "STR_EXTRAS_LANDING_MAP",
    "STR_EXTRAS_HOLE_COMMENTS",
    "STR_EXTRAS_UPDATE_CHECK",
    "STR_EXTRAS_GRAPHS",
    "STR_EXTRAS_ZOOM_POINTER",
    "STR_EXTRAS_DRAG_PLACE",
    "STR_EXTRAS_LIVE_SCORECARDS",
    "STR_EXTRAS_TICKER_JUMP",
    "STR_EXTRAS_YEAR_SUMMARY",
    "STR_EXTRAS_HOME_DETAILS",
    "STR_EXTRAS_RIGHT_DRAG",
    "STR_EXTRAS_RANDOM_TREES",
    "STR_EXTRAS_SAVE_FILES",
    "STR_EXTRAS_DESERT_ART",
    "STR_EXTRAS_MOUSE_SWING",
    "STR_EXTRAS_RIGHT_DROP",
    "STR_EXTRAS_WIDE_UI",
    "STR_EXTRAS_EDGE_SCROLL",
    "STR_EXTRAS_HOME_LABELS_HOVER",
    "STR_EXTRAS_HOLE_FLYOVER",
];

/// The update check's value when settings.json has none (`Settings::update_check`).
fn on() -> bool {
    true
}

thread_local! {
    static SETTINGS: Cell<Settings> = const { Cell::new(Settings::ALL_OFF) };
}

pub fn settings() -> Settings {
    SETTINGS.with(|s| s.get())
}

pub fn set(s: Settings) {
    SETTINGS.with(|c| c.set(s));
}

pub fn people() -> bool {
    settings().people
}
pub fn history() -> bool {
    settings().history
}
pub fn name_tags() -> bool {
    settings().name_tags
}
pub fn report_tabs() -> bool {
    settings().report_tabs
}
pub fn path_help() -> bool {
    settings().path_help
}
pub fn autosave() -> bool {
    settings().autosave
}
pub fn save_pictures() -> bool {
    settings().save_pictures
}
pub fn sharing() -> bool {
    settings().sharing
}
pub fn pause_background() -> bool {
    settings().pause_background
}
pub fn pause_events() -> bool {
    settings().pause_events
}
pub fn landing_map() -> bool {
    settings().landing_map
}
pub fn hole_comments() -> bool {
    settings().hole_comments
}
pub fn update_check() -> bool {
    settings().update_check
}
pub fn graphs() -> bool {
    settings().graphs
}
pub fn zoom_pointer() -> bool {
    settings().zoom_pointer
}
pub fn drag_place() -> bool {
    settings().drag_place
}
pub fn live_scorecards() -> bool {
    settings().live_scorecards
}
pub fn ticker_jump() -> bool {
    settings().ticker_jump
}
pub fn year_summary() -> bool {
    settings().year_summary
}
pub fn home_details() -> bool {
    settings().home_details
}
pub fn right_drag() -> bool {
    settings().right_drag
}
/// Mouse aim and swing: the mouse aims the pro's shot as a finger does (touch.rs).
pub fn mouse_swing() -> bool {
    settings().mouse_swing
}

/// Whether the HUD goes to the window's edges with Classic graphics too (enhanced.rs widescreen).
pub fn wide_ui() -> bool {
    settings().wide_ui
}

pub fn edge_scroll() -> bool {
    settings().edge_scroll
}

/// Whether the celebrities' home names show only under the pointer (homes_ui.rs draw_home_labels).
pub fn home_labels_hover() -> bool {
    settings().home_labels_hover
}

/// Whether a right click puts the tool in hand away (main.rs right_click).
pub fn right_drop() -> bool {
    settings().right_drop
}
pub fn hole_flyover() -> bool {
    settings().hole_flyover
}
pub fn random_trees() -> bool {
    settings().random_trees
}
pub fn save_files() -> bool {
    settings().save_files
}

fn settings_json() -> serde_json::Value {
    sg_core::fsutil::read_file(crate::app::save_dir().join(crate::hd::SETTINGS_FILE))
        .and_then(|d| serde_json::from_slice::<serde_json::Value>(&d).ok())
        .filter(|v| v.is_object())
        .unwrap_or_else(|| serde_json::json!({}))
}

/// The ticks saved in settings.json: all off on a first start (a new player starts with the original game; the presets
/// switch to the extras in one click) and when settings.json has no ticks at all (settings written before the extras
/// existed, or only by other screens: 0.4.9 read those as most extras on).
pub fn saved() -> Settings {
    if sg_core::fsutil::read_file(crate::app::save_dir().join(crate::hd::SETTINGS_FILE)).is_none() {
        return Settings::ALL_OFF;
    }
    from_json(&settings_json()[SETTINGS_KEY])
}

/// The ticks in settings.json's "ui_extras" value: a tick missing there reads as off (`Settings::default`), except that
/// autosave, save pictures and sharing, which came after the People screen, follow its tick in ticks saved before them.
fn from_json(v: &serde_json::Value) -> Settings {
    if !v.is_object() {
        return Settings { update_check: true, wide_ui: true, edge_scroll: true, ..Settings::ALL_OFF };
    }
    let mut s: Settings = serde_json::from_value(v.clone()).unwrap_or_default();
    if v.get("autosave").is_none() {
        s.autosave = s.people;
        s.save_pictures = s.people;
    }
    if v.get("sharing").is_none() {
        s.sharing = s.people;
    }
    follow_sharing(v, &mut s);
    upgrade_off(v, &mut s);
    s
}

/// Ticks saved before Import and export saved games (0.5), whose buttons came with Course sharing: it follows that tick.
fn follow_sharing(v: &serde_json::Value, s: &mut Settings) {
    if v.get("save_files").is_none() {
        s.save_files = s.sharing;
    }
}

/// The extras added in 0.4 read as off from ticks saved before them.
fn upgrade_off(v: &serde_json::Value, s: &mut Settings) {
    if v.is_object() && v.get("landing_map").is_none() {
        s.landing_map = false;
    }
    if v.is_object() && v.get("hole_comments").is_none() {
        s.hole_comments = false;
    }
    // the update check (0.6: "check_updates", on unless unticked since; `Settings::update_check`) is not upgraded here
    // the history graphs keep more with the game, and zooming towards the pointer changes the original's zoom: both read
    // as off from ticks saved before them
    if v.get("graphs").is_none() {
        s.graphs = false;
    }
    if v.get("zoom_pointer").is_none() {
        s.zoom_pointer = false;
    }
    // drag to place changes what the original's drag does: off from ticks saved before it
    if v.get("drag_place").is_none() {
        s.drag_place = false;
    }
    // the 0.4.2 helpers (live scorecards, the clickable ticker, the year in numbers, home-site details): off from ticks
    // saved before them
    for (k, t) in [
        ("live_scorecards", &mut s.live_scorecards),
        ("ticker_jump", &mut s.ticker_jump),
        ("year_summary", &mut s.year_summary),
        ("home_details", &mut s.home_details),
    ] {
        if v.get(k).is_none() {
            *t = false;
        }
    }
    // right-drag panning (0.4.8): off from ticks saved before it, so a 0.4.7 player's right button is the original's
    if v.get("right_drag").is_none() {
        s.right_drag = false;
    }
    // random trees (0.4.8): off from ticks saved before it
    if v.get("random_trees").is_none() {
        s.random_trees = false;
    }
    // the desert's Ravine and Brush art (0.5.6 to 0.5.13): retired, its key read and ignored (issue #117)
    s.desert_art = false;
}

/// Saves the ticks into settings.json, keeping its other keys.
pub fn save(s: &Settings) {
    let mut v = settings_json();
    if let Ok(e) = serde_json::to_value(s) {
        v[SETTINGS_KEY] = e;
    }
    if let Ok(t) = serde_json::to_string_pretty(&v) {
        sg_core::fsutil::write_file(crate::app::save_dir().join(crate::hd::SETTINGS_FILE), t.as_bytes());
    }
}

/// At start up: the saved ticks, or all off for this run with `--no-extras`.
pub fn init(off: bool) {
    let first = sg_core::fsutil::read_file(crate::app::save_dir().join(crate::hd::SETTINGS_FILE)).is_none();
    let s = if off { Settings::ALL_OFF } else { saved() };
    // a first start writes its choice down at once (a file without the key reads as all off too)
    if first && !off {
        save(&s);
    }
    set(s);
}

// ---- what the extras keep with the game -----------------------------------------------------------------------------------

/// A ticker message as the history keeps it: the game tick it was posted at, its text, who said it (the ticker's speaker
/// code), the golfer's person record and face then (for a golfer), and the map point it is about when known.
#[derive(Clone, Debug, Default, PartialEq, Serialize, Deserialize)]
pub struct Message {
    pub tick: u32,
    pub text: String,
    pub speaker: i32,
    /// The speaking golfer's person record (-1 none).
    #[serde(default = "minus_one")]
    pub roster: i32,
    /// The golfer's face then: 0 happy, 1 neutral, 2 angry.
    #[serde(default)]
    pub face: u8,
    /// The map point (units) the message is about: the golfer's place then, or a building's.
    #[serde(default)]
    pub at: Option<(i32, i32)>,
}

fn minus_one() -> i32 {
    -1
}

/// Something a golfer said (a thought over the golfer), for the golfer's profile page: the tick, the text and its tone
/// (0 neutral, 1 a good thought, 2 a bad one).
#[derive(Clone, Debug, Default, PartialEq, Serialize, Deserialize)]
pub struct Said {
    pub tick: u32,
    pub text: String,
    #[serde(default)]
    pub tone: u8,
}

/// The most messages the history keeps (the oldest go first).
pub const MAX_MESSAGES: usize = 1000;
/// The most things kept per golfer, and the most golfers kept.
pub const MAX_SAID: usize = 60;
pub const MAX_SAYERS: usize = 600;

/// What the extras keep in a saved game (one backward-compatible field of the save: an older game reads as nothing kept,
/// and a game with nothing kept writes no field, so its save is the same as before the extras existed).
#[derive(Clone, Debug, Default, PartialEq, Serialize, Deserialize)]
pub struct Kept {
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub messages: Vec<Message>,
    /// Employees' names given by Rename, by index in the employee list.
    #[serde(default, skip_serializing_if = "BTreeMap::is_empty")]
    pub staff_names: BTreeMap<usize, String>,
    /// What each golfer said, by person record (the bios are the person records' own, as Customise's Update Bio edits).
    #[serde(default, skip_serializing_if = "BTreeMap::is_empty")]
    pub said: BTreeMap<i32, Vec<Said>>,
    /// The landing map's shots and the comments by hole (insight.rs), only while one of them is on.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub insight: Option<crate::insight::Kept>,
    /// The history graphs' months (graphs.rs), only while their tick is on.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub graphs: Option<crate::graphs::Kept>,
    /// The camera bookmarks (views.rs), once one has been kept.
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub views: Vec<Option<crate::views::View>>,
    /// The year in numbers' year-end marks and tournament wins (year_ui.rs), only while its tick is on.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub years: Option<crate::year_ui::Kept>,
    /// Home sites placed while the home-site details are on (homes_ui.rs): what each cost and sold for.
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub homes: Vec<crate::homes_ui::Sale>,
}

impl Kept {
    pub fn is_empty(&self) -> bool {
        self.messages.is_empty()
            && self.staff_names.is_empty()
            && self.said.is_empty()
            && self.insight.is_none()
            && self.graphs.is_none()
            && self.views.is_empty()
            && self.years.is_none()
            && self.homes.is_empty()
    }

    pub fn push_message(&mut self, m: Message) {
        self.messages.push(m);
        if self.messages.len() > MAX_MESSAGES {
            let over = self.messages.len() - MAX_MESSAGES;
            self.messages.drain(..over);
        }
    }

    pub fn push_said(&mut self, roster: i32, s: Said) {
        if !self.said.contains_key(&roster) && self.said.len() >= MAX_SAYERS {
            // the golfer heard from longest ago makes room
            if let Some(&old) = self.said.iter().min_by_key(|(_, v)| v.last().map(|s| s.tick).unwrap_or(0)).map(|(k, _)| k) {
                self.said.remove(&old);
            }
        }
        let v = self.said.entry(roster).or_default();
        // the same thought again at once (partners, repeats within a tick) is kept once
        if v.last().is_some_and(|l| l.text == s.text && s.tick.saturating_sub(l.tick) < 40) {
            return;
        }
        v.push(s);
        if v.len() > MAX_SAID {
            v.remove(0);
        }
    }
}

/// Who the camera follows (People > Follow): a golfer by person record, or an employee by index.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Follow {
    Golfer(i32),
    Staff(usize),
}

/// What the one-line prompt (popup_ui's rename prompt) edits besides the course name.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum RenameFor {
    Employee(usize),
    Bio(i32),
}

/// The extras' state in the game (`App::xs`).
#[derive(Default)]
pub struct State {
    pub kept: Kept,
    /// The map point the next ticker message is about (set just before posting it).
    pub next_place: Option<(i32, i32)>,
    /// The person record and map point of the message on the ticker (for its Show button).
    pub ticker_place: Option<(i32, (i32, i32))>,
    /// Money each golfer slot spent this round (units of $100) and the person record it was counted for.
    pub spent: Vec<(i32, i64)>,
    pub follow: Option<Follow>,
    pub rename_for: Option<RenameFor>,
    pub people: crate::people_ui::PeopleUi,
    pub history: crate::history_ui::HistoryUi,
    pub profile: crate::profile_ui::ProfileUi,

    /// The path view (Shift+V) and its marks, made again when the course changes.
    pub path_view: bool,
    /// The --screen test hook's extras screen, opened once the game has run (main.rs).
    pub test_open: Option<String>,
    /// The screen has been touched (touch.rs): no pointer to hover with, so no name tags.
    pub touched: bool,
    /// Shift+H hid the name tags (until the game is closed; not saved).
    pub tags_hidden: bool,
    pub path_marks: Option<crate::pathview_ui::Marks>,
    /// The landing map's balls in play, the map and the comments page (insight.rs).
    pub insight: crate::insight::State,
    /// The Graphs page's choices (graphs.rs).
    pub graphs: crate::graphs::State,
    /// The person record of the golfer who says the message on the ticker (the clickable ticker, message_ui.rs).
    pub ticker_golfer: Option<i32>,
    /// The End of Year report shows the year in numbers (year_ui.rs), the page after the original's.
    pub year_page: bool,
}

// ---- drawing pieces shared by the extras' screens ---------------------------------------------------------------------------

use crate::gfx::Gfx;
use crate::info_ui::c15;
use crate::render::Rect;
use crate::ui::{Fnt, Screen as Ui};
use std::collections::BTreeMap;

/// A tab: the command buttons' navy on pale blue (lit under the pointer), the chosen one on the yellow the original's hire
/// dialog lights its choice with (0x7f40).
pub fn draw_tab(g: &mut Gfx, s: &Ui, r: Rect, f: Fnt, label: &str, on: bool, hover: bool) {
    // a label too wide for its tab (a translation): smaller, on two lines or cut (ui.rs fit_in; issue #164)
    let fitted = crate::ui::fit_in(f, label, (r.x, r.y, r.w, r.h));
    if crate::glass_ui::active() {
        crate::glass_ui::tab(g, s, r.x, r.y, r.w, r.h + 1.0, on, hover);
        let put = |g: &mut Gfx, f: Fnt, x: f32, y: f32, t: &str| {
            if on {
                s.put(g, f, x, y, t, crate::glass_ui::INK);
            } else {
                s.put_shadowed(g, f, x, y, t, c15(0x7fff), false);
            }
        };
        match fitted {
            Some((f, lines)) => lines.iter().for_each(|(t, cx, y)| put(g, f, (cx - f.width(t) * 0.5).floor(), *y, t)),
            None => put(g, f, (r.x + (r.w - f.width(label)) * 0.5).floor(), r.y + ((r.h - f.px) * 0.5).floor(), label),
        }
        return;
    }
    let edge = if on || hover { c15(0x7fff) } else { c15(0x2d6b) };
    s.fill(g, r.x - 1.0, r.y - 1.0, r.w + 2.0, r.h + 2.0, edge);
    let fill = if on {
        c15(0x7f40)
    } else if hover {
        crate::ui::rgb(0.85, 0.85, 1.0)
    } else {
        crate::ui::rgb(0.47, 0.47, 0.70)
    };
    s.fill(g, r.x, r.y, r.w, r.h, fill);
    match fitted {
        Some((f, lines)) => {
            for (t, cx, y) in &lines {
                s.put(g, f, (cx - f.width(t) * 0.5).floor(), *y, t, c15(0x0018));
            }
        }
        None => {
            let x = (r.x + (r.w - f.width(label)) * 0.5).floor();
            let y = r.y + ((r.h - f.px) * 0.5).floor();
            s.put(g, f, x, y, label, c15(0x0018));
        }
    }
}

/// A command button in the look of the Keyboard controls screen's page buttons (keys_ui): navy on the pale blue, lit under
/// the pointer, grey when it cannot be used.
pub fn draw_button(g: &mut Gfx, s: &Ui, r: Rect, label: &str, enabled: bool, hover: bool) {
    let f = crate::ui::F_ARIAL10;
    let lit = enabled && hover;
    let c = if crate::glass_ui::active() {
        crate::glass_ui::button(g, s, r.x, r.y, r.w, r.h + 1.0, enabled, lit);
        if enabled {
            crate::glass_ui::INK
        } else {
            crate::glass_ui::DIM_INK
        }
    } else {
        s.fill(g, r.x - 1.0, r.y - 1.0, r.w + 2.0, r.h + 2.0, c15(if lit { 0x7fff } else { 0x2d6b }));
        s.fill(g, r.x, r.y, r.w, r.h, if lit { crate::ui::rgb(0.85, 0.85, 1.0) } else { crate::ui::rgb(0.47, 0.47, 0.70) });
        c15(if enabled { 0x0018 } else { 0x6318 })
    };
    // a label too wide for its button (a translation): smaller, on two lines or cut (ui.rs fit_in; issue #164)
    match crate::ui::fit_in(f, label, (r.x, r.y, r.w, r.h)) {
        Some((f, lines)) => lines.iter().for_each(|(t, cx, y)| s.put_centered(g, f, *cx, *y, t, c)),
        None => s.put_centered(g, f, r.x + r.w / 2.0, r.y + ((r.h - 10.0) * 0.5).floor(), label, c),
    }
}

/// A scroll arrow in the look of the roster's: a small triangle in the box's navy, pointing up or down.
pub fn draw_arrow(g: &mut Gfx, s: &Ui, cx: f32, cy: f32, up: bool, c: [f32; 4]) {
    for i in 0..5 {
        let w = 1.0 + 2.0 * i as f32;
        let y = if up { cy - 2.0 + i as f32 } else { cy + 2.0 - i as f32 };
        s.fill(g, cx - w / 2.0, y, w, 1.0, c);
    }
}

/// Lines of `t` no wider than `w` in font `f` (a word longer than the line stays whole).
pub fn wrap(f: Fnt, t: &str, w: f32) -> Vec<String> {
    let mut out = Vec::new();
    let mut line = String::new();
    for word in t.split(' ') {
        let next = if line.is_empty() { word.to_string() } else { format!("{line} {word}") };
        if !line.is_empty() && f.width(&next) > w {
            out.push(std::mem::take(&mut line));
            line = word.to_string();
        } else {
            line = next;
        }
    }
    out.push(line);
    out
}

/// The game date of a tick as the extras print it: "March 3, 2001" (8 months a year from March, 30 days a month, as the
/// SimFoto's date stamp counts them).
pub fn date(tick: u32) -> String {
    use sg_core::lang::{tr, trf};
    let month = tr(crate::hud_ui::MONTH_KEYS[((tick >> 10) & 7) as usize]);
    let day = (((tick & 0x3ff) * 30) >> 10) + 1;
    trf("STR_EXTRAS_DATE", &[("MONTH", &month), ("DAY", &day), ("YEAR", &(2001 + (tick >> 13)))])
}

impl crate::app::App {
    /// A golfer's ball portrait at any size (the ticker's 140 x 140 ball portrait scaled to `size`).
    #[allow(clippy::too_many_arguments)]
    pub fn small_head(&self, g: &mut Gfx, s: &Ui, roster: i32, face: usize, x: f32, y: f32, size: f32) {
        let art = &self.art;
        if art.trans.tex.is_none() || roster < 0 {
            return;
        }
        let id = roster as usize;
        let p = self.club.roster.get(id).cloned().unwrap_or_default();
        let (female, head) = (p.female(), p.head_index(id));
        s.image_scaled(g, &art.trans, (x, y, size, size), (0.0, 300.0, 140.0, 140.0));
        let gi = (!female) as usize;
        let e = face.min(2) as f32;
        if head < 19 {
            let (sx, sy) = ((head / 2) as f32 * 140.0, (head & 1) as f32 * 420.0 + e * 140.0);
            s.image_scaled(g, &art.halo[gi], (x, y, size, size), (sx, sy, 140.0, 140.0));
        } else if let Some(img) = art.heads[gi].get(head as usize - 19) {
            s.image_scaled(g, img, (x, y, size, size), (0.0, e * 140.0, 140.0, 140.0));
        }
    }

    /// Where a person record's golfer is now (on the course), with the slot.
    pub fn golfer_of(&self, roster: i32) -> Option<usize> {
        if roster < 0 {
            return None;
        }
        // on a hole, else walking off the course (hole 19): the People screen lists both
        let find = |holes: std::ops::RangeInclusive<i32>| {
            (0..sg_core::golfer::SLOTS).find(|&g| self.club.g[g].roster == roster && holes.contains(&self.club.g[g].hole))
        };
        find(1..=18).or_else(|| find(19..=19))
    }

    /// The camera to a map point (units).
    pub fn look_at(&mut self, at: (i32, i32)) {
        let (x, z) = self.units_to_world(at.0, at.1);
        self.cam_x = x;
        self.cam_z = z;
    }

    /// The camera follows its target (People > Follow) until it leaves the course; called once a frame.
    pub fn follow_tick(&mut self) {
        let Some(f) = self.xs.follow else { return };
        let at = match f {
            Follow::Golfer(r) => self.golfer_of(r).map(|g| (self.club.g[g].x, self.club.g[g].y)),
            Follow::Staff(i) => self.employees.get(i).filter(|e| e.active).map(|e| (e.x, e.y)),
        };
        match at {
            Some(p) => self.look_at(p),
            None => self.xs.follow = None,
        }
    }

    /// Shift+H with the name tags extra ticked: hides the tags or shows them again until the game is closed (the tick
    /// itself stays; with it unticked the key does nothing, as the other extras' keys).
    pub fn toggle_name_tags(&mut self) {
        if !name_tags() {
            return;
        }
        self.xs.tags_hidden = !self.xs.tags_hidden;
        self.show_toast(sg_core::lang::tr(if self.xs.tags_hidden { "STR_TOAST_NAME_TAGS_OFF" } else { "STR_TOAST_NAME_TAGS_ON" }));
    }

    /// "Following <name>" under the Paused caption while the camera follows someone.
    pub fn draw_following(&self, g: &mut Gfx, s: &Ui) {
        use sg_core::lang::trf;
        let Some(f) = self.xs.follow else { return };
        let name = match f {
            Follow::Golfer(r) => self.golfer_of(r).map(|g| self.club.vip_name(g)),
            Follow::Staff(i) => self.employees.get(i).map(|e| self.employee_name(e)),
        };
        if let Some(n) = name {
            // above the panel's place, clear of the ticker at the top
            let t = trf("STR_EXTRAS_FOLLOWING", &[("NAME", &n)]);
            s.put_shadowed(g, crate::ui::F_MANUAL15, 512.0, 428.0, &t, crate::ui::rgb(1.0, 1.0, 1.0), true);
        }
    }

    /// The --screen test hook's extras screens that need golfers on the course first (main.rs open_screen).
    pub fn extras_test_hook(&mut self) {
        let Some(t) = self.xs.test_open.clone() else { return };
        let first = (0..sg_core::golfer::SLOTS)
            .filter(|&g| (1..19).contains(&self.club.g[g].hole) && self.club.g[g].roster >= 0)
            .find(|&g| self.screen_of(self.club.g[g].x, self.club.g[g].y).is_some());
        match t.as_str() {
            "profile" => {
                let Some(g) = first else { return };
                // the golfer's latest thoughts, kept as the game would have kept them
                for i in (0..10).rev() {
                    let id = self.club.g[g].thoughts[i];
                    if id != 0 {
                        self.keep_thought(g, id as u32, (self.club.g[g].args[i] & 0x3fff) as i32);
                    }
                }
                self.open_profile(self.club.g[g].roster);
            }
            "tags" => {
                // the pointer on the first golfer on screen, every frame
                let Some(g) = first else { return };
                if let Some((x, y)) = self.screen_of(self.club.g[g].x, self.club.g[g].y) {
                    self.info.pointer = (x, y - 6.0);
                }
                return;
            }
            "ticker" => {
                // after the course's welcome story (posted at tick 0x20), which would take the ticker back
                if self.game_tick <= 0x20 {
                    return;
                }
                // a building off the paths, or (a course without one) the clubhouse standing in for a snack bar
                let Some(o) =
                    self.course.objects.iter().skip(1).find(|o| (6..=14).contains(&o.kind)).or(self.course.objects.first()).copied()
                else {
                    return;
                };
                self.xs.next_place = Some((o.a * 1024 + 1024, o.b * 1024 + 1024));
                let name = sg_core::land::building_name(if (6..=14).contains(&o.kind) { o.kind } else { 7 });
                self.show_toast(&sg_core::lang::trf("STR_TOAST_NOT_OPERATIONAL", &[("BUILDING", &name)]));
            }
            "people" if first.is_some() => {
                let lines_first = first.map(|g| self.club.g[g].roster);
                self.xs.people.golfer = lines_first;
            }
            "card" => {
                let Some(g) = first else { return };
                self.card = Some(g);
            }
            "staff" => self.xs.people.staff = self.people_staff().first().copied(),
            _ => {}
        }
        self.xs.test_open = None;
    }

    /// Money spent by each golfer this round, from the club's earnings at the golfer's own place (greens fees, food and
    /// drink, wagers).
    pub fn count_spent(&mut self, units: i32, at: (i32, i32)) {
        if units <= 0 {
            return;
        }
        let n = sg_core::golfer::SLOTS;
        if self.xs.spent.len() != n {
            self.xs.spent = vec![(-1, 0); n];
        }
        if let Some(g) = (0..n).find(|&g| self.club.g[g].hole > 0 && (self.club.g[g].x, self.club.g[g].y) == at) {
            let r = self.club.g[g].roster;
            let e = &mut self.xs.spent[g];
            if e.0 != r {
                *e = (r, 0);
            }
            e.1 += units as i64;
        }
    }

    /// Money a golfer slot has spent this round (dollars).
    pub fn spent_by(&self, g: usize) -> i64 {
        self.xs.spent.get(g).filter(|e| e.0 == self.club.g[g].roster).map(|e| e.1 * 100).unwrap_or(0)
    }

    /// A thought a golfer had, kept for the profile page.
    pub fn keep_thought(&mut self, g: usize, id: u32, arg: i32) {
        let gg = &self.club.g[g];
        let roster = gg.roster;
        if roster < 0 || id == 0 {
            return;
        }
        let text = if id == 0x32 {
            let owner = gg.flags & sg_core::golfer::flag::STORY != 0;
            let t = self.club.stories.line(gg.story, arg & 15, (arg >> 4) & 15, owner).trim().to_string();
            t.replacen("PARTNER", &self.club.name(g ^ 1), 1).replacen("MYNAME", &self.club.name(g), 1)
        } else {
            self.club.thought(&self.course, id, arg & 0x3fff, g).text
        };
        if text.trim().is_empty() {
            return;
        }
        let tone = match gg.thoughts.iter().position(|&t| t as u32 == id).map(|i| gg.args[i] & 0xc000) {
            Some(0x4000) => 1,
            Some(0xc000) => 2,
            _ => 0,
        };
        let tick = self.date();
        self.xs.kept.push_said(roster, Said { tick, text, tone });
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn bits_round_trip_and_missing_ticks_read_as_off() {
        for m in 0..1024 {
            assert_eq!(Settings::from_bits(m).bits(), m);
        }
        // settings.json without the ticks at all (written before the extras, or only by other screens): all off but the
        // update check, the interface at the screen's edges (both on from 0.6) and edge scrolling (the original's)
        assert_eq!(Settings::default(), Settings::ALL_OFF);
        let checking = Settings { update_check: true, wide_ui: true, edge_scroll: true, ..Settings::ALL_OFF };
        assert_eq!(from_json(&serde_json::Value::Null), checking);
        let file: serde_json::Value = serde_json::from_str("{\"hd\": {\"enabled\": false}, \"controls\": {}}").unwrap();
        assert_eq!(from_json(&file[SETTINGS_KEY]), checking);
        // the first ticks (People to path help) as the first version saved them: as saved, the later ones off but for the
        // three that follow the People screen
        let v: serde_json::Value = serde_json::from_str(
            "{\"people\": true, \"history\": false, \"name_tags\": true, \"report_tabs\": true, \"path_help\": false}",
        )
        .unwrap();
        let s = from_json(&v);
        let want = Settings {
            people: true,
            name_tags: true,
            report_tabs: true,
            // import and export follows sharing, which follows the People screen
            save_files: true,
            autosave: true,
            save_pictures: true,
            sharing: true,
            // on from 0.6 for every file without their own ticks (edge scrolling from 0.6.4)
            update_check: true,
            wide_ui: true,
            edge_scroll: true,
            ..Settings::ALL_OFF
        };
        assert_eq!(s, want);
        // ticks saved now read back as saved
        for t in [Settings::HELPERS, Settings::ALL_OFF, Settings { update_check: true, random_trees: true, ..Settings::ALL_OFF }] {
            assert_eq!(from_json(&serde_json::to_value(t).unwrap()), t);
        }
        let s: Settings = serde_json::from_str("{\"people\": false}").unwrap();
        // the pauses are off unless saved on, and they come after the older ticks' bits
        assert!(!s.pause_background && !s.pause_events);
        let p = Settings { pause_background: true, pause_events: true, ..Settings::ALL_OFF };
        assert_eq!(p.bits(), 0b11 << 8);
        assert_eq!(Settings::from_bits(p.bits()), p);
        assert_eq!(p.preset_part(), Settings::ALL_OFF);
        for m in [0, 1 << 8, 1 << 9, 1 << 10, 1 << 11, 1 << 12, 1 << 13, 1 << 14, 1 << 15, 0xffff] {
            assert_eq!(Settings::from_bits(m).bits(), m);
        }
    }

    #[test]
    fn the_update_check_is_on_once_from_0_6_and_stays_as_the_player_sets_it() {
        // an older file's update tick (off by default then) is not read: on
        let v: serde_json::Value = serde_json::from_str("{\"people\": true, \"update_check\": false}").unwrap();
        assert!(from_json(&v).update_check);
        // unticked since: off, and saved as such
        let v: serde_json::Value = serde_json::from_str("{\"check_updates\": false}").unwrap();
        let s = from_json(&v);
        assert!(!s.update_check);
        assert_eq!(serde_json::to_value(s).unwrap()["check_updates"], serde_json::Value::Bool(false));
    }

    #[test]
    fn ticks_saved_before_the_insight_extras_read_them_as_off() {
        let v: serde_json::Value = serde_json::from_str("{\"people\": true, \"autosave\": true, \"sharing\": true}").unwrap();
        let mut s: Settings = serde_json::from_value(v.clone()).unwrap();
        upgrade_off(&v, &mut s);
        assert!(s.people && s.sharing && !s.landing_map && !s.hole_comments);
        // ticks saved with them keep them
        let v: serde_json::Value = serde_json::from_str("{\"landing_map\": true, \"hole_comments\": true}").unwrap();
        let mut s: Settings = serde_json::from_value(v.clone()).unwrap();
        upgrade_off(&v, &mut s);
        assert!(s.landing_map && s.hole_comments);
        assert!(s.update_check, "the update check reads as on from ticks saved before 0.6");
        assert!(!s.graphs && !s.zoom_pointer, "the graphs and zooming to the pointer read as off from older ticks");
        let v: serde_json::Value = serde_json::from_str("{\"graphs\": true, \"zoom_pointer\": true}").unwrap();
        let mut g: Settings = serde_json::from_value(v.clone()).unwrap();
        upgrade_off(&v, &mut g);
        assert!(g.graphs && g.zoom_pointer);
        assert!(!Settings::HELPERS.zoom_pointer && Settings::HELPERS.graphs);
        assert_eq!(Settings { zoom_pointer: true, ..Settings::ALL_OFF }.preset_part(), Settings::ALL_OFF);
        let v: serde_json::Value = serde_json::from_str("{\"update_check\": true}").unwrap();
        let mut s: Settings = serde_json::from_value(v.clone()).unwrap();
        upgrade_off(&v, &mut s);
        assert!(s.update_check);
        assert!(!Settings::HELPERS.update_check, "not in the presets");
        assert_eq!(Settings { update_check: true, ..Settings::ALL_OFF }.preset_part(), Settings::ALL_OFF);
        // drag to place: off from older ticks, kept when saved, out of the one-click extras and the game style
        let v: serde_json::Value = serde_json::from_str("{\"people\": true, \"zoom_pointer\": true}").unwrap();
        let mut d: Settings = serde_json::from_value(v.clone()).unwrap();
        upgrade_off(&v, &mut d);
        assert!(!d.drag_place && d.zoom_pointer);
        let v: serde_json::Value = serde_json::from_str("{\"drag_place\": true}").unwrap();
        let mut d: Settings = serde_json::from_value(v.clone()).unwrap();
        upgrade_off(&v, &mut d);
        assert!(d.drag_place);
        assert!(!Settings::HELPERS.drag_place);
        assert_eq!(Settings { drag_place: true, ..Settings::ALL_OFF }.bits(), 1 << 15);
        assert_eq!(Settings { drag_place: true, ..Settings::ALL_OFF }.preset_part(), Settings::ALL_OFF);
        // the 0.4.2 helpers: off from older ticks, kept when saved, after the older bits, and in the one-click extras
        let v: serde_json::Value = serde_json::from_str("{\"people\": true, \"graphs\": true}").unwrap();
        let mut s: Settings = serde_json::from_value(v.clone()).unwrap();
        upgrade_off(&v, &mut s);
        assert!(!s.live_scorecards && !s.ticker_jump && !s.year_summary && !s.home_details);
        let v: serde_json::Value =
            serde_json::from_str("{\"live_scorecards\": true, \"ticker_jump\": true, \"year_summary\": true, \"home_details\": true}")
                .unwrap();
        let mut s: Settings = serde_json::from_value(v.clone()).unwrap();
        upgrade_off(&v, &mut s);
        assert!(s.live_scorecards && s.ticker_jump && s.year_summary && s.home_details);
        let n = Settings { live_scorecards: true, ticker_jump: true, year_summary: true, home_details: true, ..Settings::ALL_OFF };
        assert_eq!(n.bits(), 0b1111 << 16);
        assert_eq!(Settings::from_bits(n.bits()), n);
        assert_eq!(n.preset_part(), n, "the presets set them");
        assert_eq!(Settings::HELPERS.bits() & n.bits(), n.bits());
        // a save with nothing of theirs is the same as before
        assert!(Kept::default().is_empty());
        assert_eq!(serde_json::to_string(&Kept::default()).unwrap(), "{}");
    }

    #[test]
    fn right_drag_is_off_unless_ticked() {
        // ticks as 0.4.7 saved them (every extra of its day on): the right button stays the original's
        let v: serde_json::Value = serde_json::from_str(
            "{\"people\": true, \"zoom_pointer\": true, \"drag_place\": true, \"live_scorecards\": true, \"home_details\": true}",
        )
        .unwrap();
        let mut s: Settings = serde_json::from_value(v.clone()).unwrap();
        upgrade_off(&v, &mut s);
        assert!(!s.right_drag && s.drag_place && s.zoom_pointer);
        let v: serde_json::Value = serde_json::from_str("{\"right_drag\": true}").unwrap();
        let mut s: Settings = serde_json::from_value(v.clone()).unwrap();
        upgrade_off(&v, &mut s);
        assert!(s.right_drag);
        assert!(!Settings::HELPERS.right_drag && !Settings::ALL_OFF.right_drag && !Settings::default().right_drag);
        let r = Settings { right_drag: true, ..Settings::ALL_OFF };
        assert_eq!(r.bits(), 1 << 20);
        assert_eq!(Settings::from_bits(r.bits()), r);
        assert_eq!(r.preset_part(), Settings::ALL_OFF);
    }

    #[test]
    fn mouse_swing_is_off_unless_ticked_and_the_presets_leave_it() {
        // ticks saved before it (every extra of 0.5 on): the mouse aims and shoots with one click, as in the original
        let v: serde_json::Value = serde_json::json!({ "people": true, "right_drag": true, "desert_art": true });
        assert!(!from_json(&v).mouse_swing && from_json(&v).right_drag);
        assert!(from_json(&serde_json::json!({ "mouse_swing": true })).mouse_swing);
        assert!(!Settings::ALL_OFF.mouse_swing && !Settings::default().mouse_swing && !Settings::HELPERS.mouse_swing);
        let m = Settings { mouse_swing: true, ..Settings::ALL_OFF };
        assert_eq!(m.bits(), 1 << 24, "after every older tick");
        assert_eq!(Settings::from_bits(m.bits()), m);
        // the presets leave it as the player has it
        assert_eq!(m.preset_part(), Settings::ALL_OFF);
        assert!(Settings::HELPERS.with_kept(&m).mouse_swing && !Settings::HELPERS.with_kept(&Settings::ALL_OFF).mouse_swing);
    }

    #[test]
    fn save_files_follow_course_sharing_from_older_ticks() {
        for sharing in [false, true] {
            let v: serde_json::Value = serde_json::json!({ "people": true, "sharing": sharing, "random_trees": true });
            let mut s: Settings = serde_json::from_value(v.clone()).unwrap();
            follow_sharing(&v, &mut s);
            assert_eq!(s.save_files, sharing);
        }
        let v: serde_json::Value = serde_json::json!({ "sharing": false, "save_files": true });
        let mut s: Settings = serde_json::from_value(v.clone()).unwrap();
        follow_sharing(&v, &mut s);
        assert!(s.save_files && !s.sharing);
        assert!(Settings::HELPERS.save_files && !Settings::ALL_OFF.save_files);
        assert_eq!(Settings { save_files: true, ..Settings::ALL_OFF }.preset_part().bits(), 1 << 22, "the presets set it");
        let f = Settings { save_files: true, ..Settings::ALL_OFF };
        assert_eq!(f.bits(), 1 << 22);
        assert_eq!(Settings::from_bits(f.bits()), f);
        assert_eq!(UI_KEYS.len(), 30);
    }

    #[test]
    fn the_interface_at_the_edges_is_on_unless_switched_off_and_the_presets_leave_it() {
        assert!(from_json(&serde_json::json!({ "people": true })).wide_ui, "missing: on");
        assert!(!from_json(&serde_json::json!({ "wide_ui": false })).wide_ui, "switched off: stays off");
        let off = Settings { wide_ui: false, ..Settings::HELPERS };
        assert!(!Settings::HELPERS.with_kept(&off).wide_ui && Settings::ALL_OFF.with_kept(&checking_on()).wide_ui);
    }

    fn checking_on() -> Settings {
        Settings { wide_ui: true, ..Settings::ALL_OFF }
    }

    #[test]
    fn home_names_on_hover_are_off_unless_ticked_and_the_presets_leave_them() {
        assert!(!from_json(&serde_json::json!({ "people": true, "edge_scroll": true })).home_labels_hover, "missing: off");
        assert!(from_json(&serde_json::json!({ "home_labels_hover": true })).home_labels_hover);
        assert!(!Settings::ALL_OFF.home_labels_hover && !Settings::default().home_labels_hover && !Settings::HELPERS.home_labels_hover);
        let h = Settings { home_labels_hover: true, ..Settings::ALL_OFF };
        assert_eq!(h.bits(), 1 << 28, "after every older tick");
        assert_eq!(Settings::from_bits(h.bits()), h);
        assert_eq!(h.preset_part(), Settings::ALL_OFF);
        assert!(Settings::HELPERS.with_kept(&h).home_labels_hover);
    }

    #[test]
    fn random_trees_are_off_unless_ticked() {
        let v: serde_json::Value = serde_json::from_str("{\"people\": true, \"drag_place\": true, \"right_drag\": true}").unwrap();
        let mut s: Settings = serde_json::from_value(v.clone()).unwrap();
        upgrade_off(&v, &mut s);
        assert!(!s.random_trees && s.right_drag);
        let v: serde_json::Value = serde_json::from_str("{\"random_trees\": true}").unwrap();
        let mut s: Settings = serde_json::from_value(v.clone()).unwrap();
        upgrade_off(&v, &mut s);
        assert!(s.random_trees);
        assert!(Settings::HELPERS.random_trees && !Settings::ALL_OFF.random_trees, "a look: the helper presets tick it");
        let r = Settings { random_trees: true, ..Settings::ALL_OFF };
        assert_eq!(r.bits(), 1 << 21);
        assert_eq!(Settings::from_bits(r.bits()), r);
        assert_eq!(r.preset_part(), r);
    }

    #[test]
    fn the_retired_desert_art_tick_loads_and_is_ignored() {
        // settings saved by 0.5.6 to 0.5.13 with it ticked still load, the other ticks as they were, and it reads as off
        let v: serde_json::Value = serde_json::json!({ "people": true, "random_trees": true, "desert_art": true });
        let s = from_json(&v);
        assert!(s.people && s.random_trees && !s.desert_art);
        assert!(!Settings::ALL_OFF.desert_art && !Settings::default().desert_art && !Settings::HELPERS.desert_art);
        // its bit stays where it was, so the later ticks keep theirs
        let d = Settings { desert_art: true, ..Settings::ALL_OFF };
        assert_eq!(d.bits(), 1 << 23);
    }
}
