//! The port's optional gameplay changes (sg_core::extras; docs/PLAYING.md, "OpenSGolf extras"): the Preferences box that
//! switches them, the default for new games kept in settings.json, and the game side of each: Tab turning a placed building
//! or stepping through an item's designs, the extra pages of the landmark strip (cut landmarks, animals), buying animals and
//! the Hole Stats' par buttons.
//! Nothing here runs while the extras are off.

use crate::app::App;
use crate::render::Rect;
use sg_core::extras::Extras;
use sg_core::land;
use sg_core::objects::{ANIMAL_BASE, CUT_BASE, CUT_LANDMARKS};

/// The settings.json key of the extras new games start with.
const SETTINGS_KEY: &str = "extras";

fn settings_json() -> serde_json::Value {
    sg_core::fsutil::read_file(crate::app::save_dir().join(crate::hd::SETTINGS_FILE))
        .and_then(|d| serde_json::from_slice::<serde_json::Value>(&d).ok())
        .filter(|v| v.is_object())
        .unwrap_or_else(|| serde_json::json!({}))
}

/// The extras new games start with (all off when none were saved).
pub fn saved() -> Extras {
    serde_json::from_value(settings_json()[SETTINGS_KEY].clone()).unwrap_or_default()
}

/// Saves the extras new games start with.
pub fn save_next(e: &Extras) {
    save(e);
}

fn save(e: &Extras) {
    let mut v = settings_json();
    if let Ok(x) = serde_json::to_value(e) {
        v[SETTINGS_KEY] = x;
    }
    if let Ok(t) = serde_json::to_string_pretty(&v) {
        sg_core::fsutil::write_file(crate::app::save_dir().join(crate::hd::SETTINGS_FILE), t.as_bytes());
    }
}

// ---- the Extras box (Preferences > OpenSGolf extras) ------------------------------------------------------------------------

/// The Extras box's pages (tabs along its top, drawn as the Enhanced graphics box's), sorted by the three classes of extra
/// so the player sees at a glance which ticks touch the game: GAMEPLAY changes how the game plays (the Gameplay and
/// Sandbox pages, each line tagged with how much), LOOKS changes only how it looks (the Looks page), HELPER shows
/// information or saves effort (the Info, Saves and Controls pages); the last three leave play exactly as it is. Only the page a line
/// is shown on follows this; the extras' bits, saves and settings keep their order.
pub const PAGES: [&str; 6] = [
    "STR_EXTRAS_PAGE_GAMEPLAY",
    "STR_EXTRAS_PAGE_SANDBOX",
    "STR_EXTRAS_PAGE_LOOKS",
    "STR_EXTRAS_PAGE_INFO",
    "STR_EXTRAS_PAGE_SAVES",
    "STR_EXTRAS_PAGE_CONTROLS",
];
/// The last page whose lines change how the game plays (Gameplay and Sandbox, before it): their lines carry a tag.
const LAST_TAGGED_PAGE: u8 = 1;

/// Each page's banner, the line under the tabs that says what its ticks change.
pub const BANNERS: [&str; 6] = [
    "STR_EXTRAS_BANNER_GAMEPLAY",
    "STR_EXTRAS_BANNER_SANDBOX",
    "STR_EXTRAS_BANNER_LOOKS",
    "STR_EXTRAS_BANNER_INFO",
    "STR_EXTRAS_BANNER_SAVES",
    "STR_EXTRAS_BANNER_CONTROLS",
];

/// The hint shown where the description of the line under the pointer goes while there is none: on the tagged pages, what
/// the tags mean too.
const HINTS: [&str; 2] = ["STR_EXTRAS_HINT_TAGGED", "STR_EXTRAS_HINT_PLAIN"];

/// What an option line of the Extras box is.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum ExtrasLine {
    /// Option i of sg_core::extras::OPTION_KEYS.
    Game(usize),
    /// Interface extra i (crate::extras::UI_KEYS' ticks, bit i of their Settings).
    Ui(usize),
    /// The Calendar line: Slower seasons and Pause the calendar as one setting (`Extras::calendar`).
    Calendar,
    /// A line of the Info page that ticks several interface extras together (`GROUPS`).
    Group(usize),
    /// The preset line at the top of the box, over the tabs (`Preset`).
    Preset,
}

/// How much a gameplay or sandbox extra changes the game: the coloured pill at the right end of its line.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Tag {
    Light,
    Noticeable,
    Big,
    /// Not finished: the cut content (decoration only).
    Experimental,
}

impl Tag {
    pub const ALL: [Tag; 4] = [Tag::Light, Tag::Noticeable, Tag::Big, Tag::Experimental];

    /// The pill's words.
    pub fn key(self) -> &'static str {
        match self {
            Tag::Light => "STR_EXTRAS_TAG_LIGHT",
            Tag::Noticeable => "STR_EXTRAS_TAG_NOTICEABLE",
            Tag::Big => "STR_EXTRAS_TAG_BIG",
            Tag::Experimental => "STR_EXTRAS_TAG_EXPERIMENTAL",
        }
    }

    /// The pill's fill and edge (15-bit colours, muted as the original's palette is): green, amber, red, slate, each dark
    /// enough for its white words to read (about 5 to 1 or better).
    pub fn colours(self) -> (u32, u32) {
        match self {
            Tag::Light => (0x11e6, 0x0d03),
            Tag::Noticeable => (0x4d62, 0x3922),
            Tag::Big => (0x58c6, 0x3063),
            Tag::Experimental => (0x4153, 0x20c9),
        }
    }
}

/// The Info page's grouped lines: (text key, description key, the interface extras' bits it ticks together). The ticks
/// stay one bit each in settings.json, so older and newer versions read them alike; a line whose ticks differ shows as
/// "some on" and a click turns them all on.
pub const GROUPS: [(&str, &str, &[usize]); 3] = [
    // the People screen, name tags, live scorecards
    ("STR_EXTRAS_GROUP_PEOPLE", "STR_EXTRAS_DESC_GROUP_PEOPLE", &[0, 2, 16]),
    // landing map, comments by hole, history graphs, the year in numbers, report tabs
    ("STR_EXTRAS_GROUP_REPORTS", "STR_EXTRAS_DESC_GROUP_REPORTS", &[10, 11, 13, 18, 3]),
    // the message history and the clickable ticker
    ("STR_EXTRAS_GROUP_MESSAGES", "STR_EXTRAS_DESC_GROUP_MESSAGES", &[1, 17]),
];

/// The option of `OPTION_KEYS` that is the cut content (shown only with the original game's files, `show_cut_content`).
const CUT: usize = 1;

/// The interface extra that is the update check: a tick of Preferences > More (prefs_ui.rs), not in the box.
#[cfg(test)]
const UPDATE_CHECK: usize = 12;

/// The interface extra that was the desert's Ravine and Brush art (0.5.6 to 0.5.13, issue #89): retired in 0.6, when the
/// desert's two tiles took the theme's own names (issue #117), so it is on no page; its `settings.json` key is still read
/// and ignored (extras.rs `from_json`).
#[cfg(test)]
const RETIRED_DESERT_ART: usize = 23;

/// The option lines of each page, in order (each page's most wanted first). Slower
/// seasons and Pause the calendar (options 6 and 7) are the Sandbox page's Calendar line; the Info page's groups stand for
/// their ticks (`GROUPS`); the update check is on Preferences itself.
const PAGE_LINES: [&[ExtrasLine]; 6] = {
    use sg_core::extras as x;
    use ExtrasLine::{Calendar, Game as G, Group, Ui as U};
    // each page's lines in the order players are likeliest to want them, the experimental cut content last
    [
        &[
            G(x::WEATHER),
            G(x::MORE_VISITORS),
            G(x::LEFTIES),
            G(3), // higher staff limits
            G(5), // set par
            G(x::TEES),
            G(4), // animals
            G(x::SECOND_COURSE),
            G(2), // hardcore
            G(CUT),
        ],
        // unlock all, no new dandelions, freeze moods, the calendar (slower seasons, pause), keeping the commissioner's
        // approval (moved here when the Gameplay page grew too long for the box)
        &[G(9), G(x::NO_WEEDS), G(8), Calendar, G(x::KEEP_LAND)],
        // the interface at the edges, path styles and lamps, random trees, Tab turns buildings (only the picture turns),
        // home names on hover, the hole flyover
        &[U(26), G(x::PATHS), U(21), G(0), U(28), U(29)],
        // People, extra reports, messages, home-site details, path help
        &[Group(0), Group(1), Group(2), U(19), U(4)],
        // autosave, import and export saved games, sharing, save pictures
        &[U(5), U(22), U(7), U(6)],
        // zoom towards the pointer, right click puts the tool away, drag to place, right-drag panning, mouse aim and swing
        // (the three the controls presets set and the two that change what the original's mouse does), then the two pauses
        &[U(14), U(25), U(15), U(20), U(27), U(24), U(8), U(9)],
    ]
};

thread_local! {
    static CUT_SHOWN: std::cell::Cell<bool> = const { std::cell::Cell::new(true) };
}

/// Whether the box lists the cut content: only with the original game's files (popup_ui sets it as a box opens). On the
/// free base set the line is hidden; a game that has it on keeps it, harmlessly.
pub fn show_cut_content(on: bool) {
    CUT_SHOWN.with(|c| c.set(on));
}

fn cut_shown() -> bool {
    CUT_SHOWN.with(|c| c.get())
}

/// The option lines of a page, in order.
pub fn page_lines(page: u8) -> Vec<ExtrasLine> {
    PAGE_LINES
        .get(page as usize)
        .map_or_else(Vec::new, |l| l.iter().copied().filter(|&l| l != ExtrasLine::Game(CUT) || cut_shown()).collect())
}

/// The box's option lines on a page: the preset line, then the page's.
pub fn box_lines(page: u8) -> Vec<ExtrasLine> {
    std::iter::once(ExtrasLine::Preset).chain(page_lines(page)).collect()
}

/// The page a line is on (none for the preset line, which is over the tabs, and for the options no page lists).
pub fn page_of(l: ExtrasLine) -> Option<u8> {
    PAGE_LINES.iter().position(|p| p.contains(&l)).map(|p| p as u8)
}

/// How much a line changes the game (none for the pages that leave play as it is).
pub fn tag(l: ExtrasLine) -> Option<Tag> {
    if page_of(l)? > LAST_TAGGED_PAGE {
        return None;
    }
    Some(match l {
        // animals, left-handed golfers, keeping the commissioner's approval, multiple tees (nothing changes until one is
        // placed)
        ExtrasLine::Game(i) if i == 4 || i == sg_core::extras::LEFTIES || i == sg_core::extras::KEEP_LAND || i == sg_core::extras::TEES => {
            Tag::Light
        }
        ExtrasLine::Game(CUT) => Tag::Experimental,
        // hardcore, and the sandbox's freeze and unlock all
        ExtrasLine::Game(2 | 8 | 9) => Tag::Big,
        // a second course of 18 holes
        ExtrasLine::Game(i) if i == sg_core::extras::SECOND_COURSE => Tag::Big,
        _ => Tag::Noticeable,
    })
}

/// The one-line descriptions shown at the foot of the box for the line under the pointer, by option.
const GAME_DESC: [&str; sg_core::extras::OPTION_KEYS.len()] = [
    "STR_EXTRAS_DESC_ROTATE",
    "STR_EXTRAS_DESC_CUT_CONTENT",
    "STR_EXTRAS_DESC_HARDCORE",
    "STR_EXTRAS_DESC_MORE_STAFF",
    "STR_EXTRAS_DESC_ANIMALS",
    "STR_EXTRAS_DESC_SET_PAR",
    "STR_EXTRAS_DESC_SEASON_SPEED",
    "STR_EXTRAS_DESC_PAUSE_CALENDAR",
    "STR_EXTRAS_DESC_FREEZE_MOODS",
    "STR_EXTRAS_DESC_UNLOCK_ALL",
    "STR_EXTRAS_DESC_LEFTIES",
    "STR_EXTRAS_DESC_WEATHER",
    "STR_EXTRAS_DESC_PATHS",
    "STR_EXTRAS_DESC_MORE_VISITORS",
    "STR_EXTRAS_DESC_NO_WEEDS",
    "STR_EXTRAS_DESC_KEEP_LAND",
    "STR_EXTRAS_DESC_SECOND_COURSE",
    "STR_EXTRAS_DESC_TEES",
];
const UI_DESC: [&str; crate::extras::UI_KEYS.len()] = [
    "STR_EXTRAS_DESC_PEOPLE",
    "STR_EXTRAS_DESC_HISTORY",
    "STR_EXTRAS_DESC_NAME_TAGS",
    "STR_EXTRAS_DESC_REPORT_TABS",
    "STR_EXTRAS_DESC_PATH_HELP",
    "STR_EXTRAS_DESC_AUTOSAVE",
    "STR_EXTRAS_DESC_SAVE_PICTURES",
    "STR_EXTRAS_DESC_SHARING",
    "STR_EXTRAS_DESC_PAUSE_BACKGROUND",
    "STR_EXTRAS_DESC_PAUSE_EVENTS",
    "STR_EXTRAS_DESC_LANDING_MAP",
    "STR_EXTRAS_DESC_HOLE_COMMENTS",
    "STR_EXTRAS_DESC_UPDATE_CHECK",
    "STR_EXTRAS_DESC_GRAPHS",
    "STR_EXTRAS_DESC_ZOOM_POINTER",
    "STR_EXTRAS_DESC_DRAG_PLACE",
    "STR_EXTRAS_DESC_LIVE_SCORECARDS",
    "STR_EXTRAS_DESC_TICKER_JUMP",
    "STR_EXTRAS_DESC_YEAR_SUMMARY",
    "STR_EXTRAS_DESC_HOME_DETAILS",
    "STR_EXTRAS_DESC_RIGHT_DRAG",
    "STR_EXTRAS_DESC_RANDOM_TREES",
    "STR_EXTRAS_DESC_SAVE_FILES",
    "STR_EXTRAS_DESC_DESERT_ART",
    "STR_EXTRAS_DESC_MOUSE_SWING",
    "STR_EXTRAS_DESC_RIGHT_DROP",
    "STR_EXTRAS_DESC_WIDE_UI",
    "STR_EXTRAS_DESC_EDGE_SCROLL",
    "STR_EXTRAS_DESC_HOME_LABELS_HOVER",
    "STR_EXTRAS_DESC_HOLE_FLYOVER",
];

/// A line's description (what ticking it changes; for the preset line, what the preset in use is).
pub fn desc_key(l: ExtrasLine, e: &Extras, ui: &crate::extras::Settings) -> &'static str {
    match l {
        ExtrasLine::Game(i) => GAME_DESC[i],
        ExtrasLine::Ui(i) => UI_DESC[i],
        ExtrasLine::Calendar => "STR_EXTRAS_DESC_CALENDAR",
        ExtrasLine::Group(g) => GROUPS[g].1,
        ExtrasLine::Preset => Preset::of(e, ui).desc(),
    }
}

/// The text at the foot of a page: the description of option `k` of the box (`box_lines`), or the page's hint with none.
pub fn foot_text(page: u8, k: Option<usize>, e: &Extras, ui: &crate::extras::Settings) -> (&'static str, bool) {
    match k.and_then(|k| box_lines(page).get(k).copied()) {
        Some(l) => (sg_core::lang::tr(desc_key(l, e, ui)), true),
        None => (sg_core::lang::tr(HINTS[(page > LAST_TAGGED_PAGE) as usize]), false),
    }
}

/// The most option lines a page has (every page is drawn that tall), counting lines a page hides (the cut content off the
/// original files), so the box keeps its size either way.
fn most_lines() -> usize {
    PAGE_LINES.iter().map(|p| p.len()).max().unwrap_or(0)
}

/// The widest tag pill (they are all drawn this wide, so they line up).
pub fn pill_w() -> f32 {
    Tag::ALL.iter().map(|t| PILL_FONT.width(sg_core::lang::tr(t.key()))).fold(0.0, f32::max) + 14.0
}

/// The tag pills' font: Manual SSi Bold 14, the info screens' body (0x821ee8).
pub const PILL_FONT: crate::ui::Fnt = crate::ui::F_INFO14;

/// Room a pill takes after its line's text (the gap before it and the pill).
fn pill_room() -> f32 {
    16.0 + pill_w()
}

/// Lines of the box under the options: the description of the line under the pointer (two rows of the box, so it may wrap
/// to two lines and the box keeps its size).
pub const FOOT_LINES: usize = 2;

/// Lines of the box between its title and the tabs: the preset line and a gap.
pub const HEAD_LINES: usize = 2;

/// A line's value: a tick's 0 or 1, a setting's value, a group's 0 (all off), 1 (all on) or 2 (some on), the preset's
/// index in `Preset::ALL`.
pub fn line_value(l: ExtrasLine, e: &Extras, ui: &crate::extras::Settings) -> u8 {
    match l {
        ExtrasLine::Game(i) => e.option(i),
        ExtrasLine::Ui(i) => (ui.bits() >> i & 1) as u8,
        ExtrasLine::Calendar => e.calendar(),
        ExtrasLine::Group(g) => {
            let on = GROUPS[g].2.iter().filter(|&&b| ui.bits() & 1 << b != 0).count();
            if on == 0 {
                0
            } else if on == GROUPS[g].2.len() {
                1
            } else {
                2
            }
        }
        ExtrasLine::Preset => Preset::of(e, ui) as u8,
    }
}

/// A line's text (with its value for a setting) and whether it is on (a group only with all its ticks on).
pub fn line_text(l: ExtrasLine, e: &Extras, ui: &crate::extras::Settings) -> (String, bool) {
    use sg_core::extras::{CALENDAR_KEYS, OPTION_KEYS, PATHS, PATH_STYLE_KEYS, SEASON_SPEED, SPEED_KEYS};
    use sg_core::lang::{tr, trf};
    let v = line_value(l, e, ui);
    match l {
        ExtrasLine::Game(i) if i == SEASON_SPEED => (trf(OPTION_KEYS[i], &[("VALUE", &tr(SPEED_KEYS[v as usize]))]), v != 0),
        ExtrasLine::Game(i) if i == PATHS => (trf(OPTION_KEYS[i], &[("VALUE", &tr(PATH_STYLE_KEYS[v as usize]))]), v != 0),
        ExtrasLine::Game(i) => (tr(OPTION_KEYS[i]).to_string(), v != 0),
        ExtrasLine::Ui(i) => (tr(crate::extras::UI_KEYS[i]).to_string(), v != 0),
        ExtrasLine::Calendar => (trf("STR_EXTRAS_CALENDAR", &[("VALUE", &tr(CALENDAR_KEYS[v as usize]))]), v != 0),
        ExtrasLine::Group(g) if v == 2 => (trf("STR_EXTRAS_SOME_ON", &[("LINE", &tr(GROUPS[g].0))]), false),
        ExtrasLine::Group(g) => (tr(GROUPS[g].0).to_string(), v != 0),
        ExtrasLine::Preset => (trf("STR_EXTRAS_PRESET", &[("VALUE", &tr(Preset::ALL[v as usize].key()))]), false),
    }
}

/// Whether a line is drawn without a tick box: a setting a click steps (the preset, the calendar, the path styles).
pub fn is_plain(l: ExtrasLine) -> bool {
    match l {
        ExtrasLine::Game(i) => Extras::choices(i) > 2,
        ExtrasLine::Calendar | ExtrasLine::Preset => true,
        ExtrasLine::Ui(_) | ExtrasLine::Group(_) => false,
    }
}

/// A click on a line: a tick flips, a setting steps on, a group's ticks all go off when all were on and else all on, the
/// preset line puts the next preset in use (`next_preset`).
pub fn toggle_line(l: ExtrasLine, e: &mut Extras, ui: &mut crate::extras::Settings) {
    match l {
        ExtrasLine::Game(i) => e.step_option(i),
        ExtrasLine::Ui(i) => *ui = crate::extras::Settings::from_bits(ui.bits() ^ 1 << i),
        ExtrasLine::Calendar => e.step_calendar(),
        ExtrasLine::Group(g) => {
            let all = line_value(l, e, ui) == 1;
            let m = GROUPS[g].2.iter().fold(0, |m, &b| m | 1 << b);
            *ui = crate::extras::Settings::from_bits(if all { ui.bits() & !m } else { ui.bits() | m });
        }
        ExtrasLine::Preset => next_preset(e, ui),
    }
}

/// The lines of a page of the Extras box: the title, the preset line and a gap, room for the tabs and the banner (popup_ui
/// draws them), the page's options, padded with empty lines to the longest page, and the rows of the description at the
/// foot (beside the OK tick).
pub fn popup_lines(page: u8, e: &Extras, ui: &crate::extras::Settings) -> Vec<String> {
    let mut v = vec![sg_core::lang::tr("STR_EXTRAS_TITLE").to_string(), line_text(ExtrasLine::Preset, e, ui).0, String::new()];
    v.extend((0..crate::enhanced::TAB_LINES + crate::popup_ui::BANNER_LINES).map(|_| String::new()));
    v.extend(page_lines(page).into_iter().map(|l| line_text(l, e, ui).0));
    let n = 1 + HEAD_LINES + crate::enhanced::TAB_LINES + crate::popup_ui::BANNER_LINES + most_lines() + FOOT_LINES;
    while v.len() < n {
        v.push(String::new());
    }
    v
}

/// The box's ticks on a page (bit i the box's option i: the preset line, then the page's).
pub fn popup_checks(page: u8, e: &Extras, ui: &crate::extras::Settings) -> u32 {
    box_lines(page).into_iter().enumerate().fold(0, |m, (i, l)| m | (line_text(l, e, ui).1 as u32) << i)
}

/// The box's lines drawn without a tick box on a page (`is_plain`).
pub fn popup_plain(page: u8) -> u32 {
    box_lines(page).into_iter().enumerate().filter(|&(_, l)| is_plain(l)).fold(0, |m, (i, _)| m | 1 << i)
}

/// The widest line of every page (a tagged one with its pill), the banners, and room for the tabs (the box keeps one width
/// whatever the page). The line texts are measured with every value they can show, so a click that steps a value does
/// not change the box's size.
pub fn popup_widest(e: &Extras, ui: &crate::extras::Settings) -> f32 {
    let f = crate::popup_ui::CHOICE_FONT;
    let lines = PAGE_LINES
        .iter()
        .flat_map(|p| p.iter().copied())
        .chain([ExtrasLine::Preset])
        .map(|l| {
            let w = line_texts(l, e, ui).iter().map(|t| f.width(t).round()).fold(0.0, f32::max);
            w + tag(l).map_or(0.0, |_| pill_room())
        })
        .fold(f.width(sg_core::lang::tr("STR_EXTRAS_TITLE")).round(), f32::max);
    let banners = BANNERS.iter().map(|k| crate::popup_ui::BANNER_FONT.width(sg_core::lang::tr(k)) + 24.0).fold(0.0, f32::max);
    let tab = PAGES.iter().map(|k| f.width(sg_core::lang::tr(k))).fold(0.0, f32::max) + 24.0;
    lines.max(banners).max(tab * PAGES.len() as f32)
}

/// Every text a line can show (each value of a setting, a group all on and some on, each preset).
fn line_texts(l: ExtrasLine, e: &Extras, ui: &crate::extras::Settings) -> Vec<String> {
    use sg_core::lang::{tr, trf};
    match l {
        ExtrasLine::Game(i) => (0..Extras::choices(i))
            .map(|v| {
                let mut x = *e;
                x.set_option(i, v);
                line_text(l, &x, ui).0
            })
            .collect(),
        ExtrasLine::Calendar => (0..sg_core::extras::CALENDAR_KEYS.len() as u8)
            .map(|v| {
                let mut x = *e;
                x.set_calendar(v);
                line_text(l, &x, ui).0
            })
            .collect(),
        ExtrasLine::Group(g) => vec![tr(GROUPS[g].0).to_string(), trf("STR_EXTRAS_SOME_ON", &[("LINE", &tr(GROUPS[g].0))])],
        ExtrasLine::Preset => Preset::ALL.iter().map(|p| trf("STR_EXTRAS_PRESET", &[("VALUE", &tr(p.key()))])).collect(),
        ExtrasLine::Ui(_) => vec![line_text(l, e, ui).0],
    }
}

// ---- the presets (Select Difficulty's Game box, the Extras box's top line) ------------------------------------------------

/// The presets of the OpenSGolf extras (docs/PLAYING.md, "Original game or OpenSGolf extras"): one click sets every
/// gameplay, look and helper extra. Each leaves the player's own pauses, update check and controls-preset ticks
/// (`Settings::with_kept`) and the bug fixes as they are.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Preset {
    /// Every extra off: the game exactly as it was.
    Original,
    /// Every helper and look on, nothing that changes play.
    Helpers,
    /// The helpers and looks with OpenSGolf's own gameplay additions (`Extras::OPENSGOLF`).
    OpenSGolf,
    /// Anything else: the player's own mix.
    Custom,
}

impl Preset {
    pub const ALL: [Preset; 4] = [Preset::Original, Preset::Helpers, Preset::OpenSGolf, Preset::Custom];

    /// Its name.
    pub fn key(self) -> &'static str {
        match self {
            Preset::Original => "STR_PRESET_ORIGINAL",
            Preset::Helpers => "STR_PRESET_HELPERS",
            Preset::OpenSGolf => "STR_PRESET_OPENSGOLF",
            Preset::Custom => "STR_PRESET_CUSTOM",
        }
    }

    /// Its one-line description (the Extras box's foot).
    pub fn desc(self) -> &'static str {
        match self {
            Preset::Original => "STR_PRESET_DESC_ORIGINAL",
            Preset::Helpers => "STR_PRESET_DESC_HELPERS",
            Preset::OpenSGolf => "STR_PRESET_DESC_OPENSGOLF",
            Preset::Custom => "STR_PRESET_DESC_CUSTOM",
        }
    }

    /// What it sets: the gameplay extras and the interface ticks (the kept ones off); none for Custom.
    pub fn values(self) -> Option<(Extras, crate::extras::Settings)> {
        use crate::extras::Settings;
        match self {
            Preset::Original => Some((Extras::default(), Settings::ALL_OFF)),
            Preset::Helpers => Some((Extras::LOOKS, Settings::HELPERS)),
            Preset::OpenSGolf => Some((Extras::OPENSGOLF, Settings::HELPERS)),
            Preset::Custom => None,
        }
    }

    /// The preset these extras are: any path style counts as path styles on (Tab with the Pathway tool steps it in a
    /// game), a stopped calendar whatever speed is stored as stopped; the kept ticks do not count.
    pub fn of(e: &Extras, ui: &crate::extras::Settings) -> Preset {
        let mut n = *e;
        n.path_style = n.path_style.min(1);
        n.set_calendar(e.calendar());
        let u = ui.preset_part();
        Preset::ALL.into_iter().find(|p| p.values() == Some((n, u))).unwrap_or(Preset::Custom)
    }

    /// Puts the preset's values (Custom: the mix `custom`) in `e` and `ui`, the kept ticks and a path style already chosen
    /// left as they are. Returns false when there is nothing to put (Custom without a mix).
    pub fn apply(self, e: &mut Extras, ui: &mut crate::extras::Settings, custom: Option<(Extras, crate::extras::Settings)>) -> bool {
        let Some((pe, pu)) = self.values().or(custom) else { return false };
        let style = e.path_style;
        let keep_style = e.lamps() && pe.lamps();
        *e = pe;
        if keep_style {
            e.path_style = style;
        }
        *ui = pu.with_kept(ui);
        true
    }
}

/// The preset after `now` (Original, Original + helpers, OpenSGolf, Custom, Original), put in `e` and `ui`. Custom is
/// skipped while there is no mix of the player's own to go back to (`custom`, a mix that is no preset).
pub fn cycle_preset(
    now: Preset,
    e: &mut Extras,
    ui: &mut crate::extras::Settings,
    custom: Option<(Extras, crate::extras::Settings)>,
) -> Preset {
    let custom = custom.filter(|(ce, cu)| Preset::of(ce, cu) == Preset::Custom);
    let i = Preset::ALL.iter().position(|&p| p == now).unwrap_or(0);
    for k in 1..=Preset::ALL.len() {
        let p = Preset::ALL[(i + k) % Preset::ALL.len()];
        if p.apply(e, ui, custom) {
            return p;
        }
    }
    now
}

/// The settings.json key of the player's own mix, kept when a preset replaces it so Custom can bring it back.
const CUSTOM_KEY: &str = "extras_custom";

/// The player's own mix last kept (none before one was).
pub fn custom_mix() -> Option<(Extras, crate::extras::Settings)> {
    let v = &settings_json()[CUSTOM_KEY];
    Some((serde_json::from_value(v["extras"].clone()).ok()?, serde_json::from_value(v["ui"].clone()).ok()?))
}

/// Keeps the extras as the player's own mix when they are no preset.
pub fn remember_custom(e: &Extras, ui: &crate::extras::Settings) {
    if Preset::of(e, ui) != Preset::Custom || custom_mix() == Some((*e, *ui)) {
        return;
    }
    let mut v = settings_json();
    v[CUSTOM_KEY] = serde_json::json!({ "extras": e, "ui": ui });
    if let Ok(t) = serde_json::to_string_pretty(&v) {
        sg_core::fsutil::write_file(crate::app::save_dir().join(crate::hd::SETTINGS_FILE), t.as_bytes());
    }
}

/// The preset line's click: the next preset (a mix of the player's own is kept first, for Custom to bring back).
pub fn next_preset(e: &mut Extras, ui: &mut crate::extras::Settings) {
    let now = Preset::of(e, ui);
    if now == Preset::Custom {
        remember_custom(e, ui);
    }
    cycle_preset(now, e, ui, custom_mix());
}

/// Pages of the landmark strip: the club's own landmarks (the original's strip), the cut ones, the animals.
pub const PAGE_OWNED: u8 = 0;
pub const PAGE_CUT: u8 = 1;
pub const PAGE_ANIMALS: u8 = 2;
/// The Path styles and lamps extra's page: the lamp post (paths_lamps.rs).
pub const PAGE_LAMPS: u8 = 3;

/// Building kinds Tab may turn once placed: landmarks and the buildings drawn as one object (the driving range is drawn
/// tile by tile and home sites by their lot, so they keep their way).
fn turnable(kind: i32) -> bool {
    kind == land::K_LANDMARK || (6..=15).contains(&kind) && kind != land::K_DRIVING_RANGE
}

impl App {
    /// The date now: the game tick, or with the Slower seasons and Pause the calendar extras the calendar's tick
    /// (sg_core::extras::Calendar).
    pub fn date(&self) -> u32 {
        self.club.cal.date(self.game_tick)
    }

    /// The preset Select Difficulty's Game box shows for what the next game starts with. Its Original is the original
    /// game, Classic graphics included: with every extra off but Enhanced graphics on it says Custom.
    pub fn game_style(&self) -> Preset {
        match Preset::of(&self.extras_next, &crate::extras::settings()) {
            Preset::Original if crate::enhanced::settings().enabled => Preset::Custom,
            p => p,
        }
    }

    /// A click on the Game box: the next preset (Original, Original + helpers, OpenSGolf, Custom when the player has a mix
    /// of their own, Original). The choice is saved like the Preferences ticks it sets; the original game also turns
    /// Enhanced graphics off (the others leave the graphics as they are). The pauses, the update check, the controls
    /// presets' ticks and the bug fixes stay as the player set them.
    pub fn next_game_style(&mut self, g: &mut crate::gfx::Gfx) {
        let (mut e, mut ui) = (self.extras_next, crate::extras::settings());
        if Preset::of(&e, &ui) == Preset::Custom {
            remember_custom(&e, &ui);
        }
        let p = cycle_preset(self.game_style(), &mut e, &mut ui, custom_mix());
        self.extras_next = e;
        save(&e);
        crate::extras::set(ui);
        crate::extras::save(&ui);
        if !ui.path_help {
            self.xs.path_view = false;
        }
        if p == Preset::Original {
            let mut enh = crate::enhanced::settings();
            if enh.enabled {
                enh.enabled = false;
                if self.apply_enhanced(g, enh) {
                    self.ui_ok &= crate::load_art(self, g);
                }
            }
        }
    }

    /// The Extras box's OK. In a game, the game takes the ticked extras (kept in its save) and new games still start with
    /// the extras chosen on the title screens (#137: a sandbox game's extras carried over into the next standard game). On
    /// the title screens they are what new games start with, saved for the next run.
    pub fn set_extras(&mut self, e: Extras) {
        if self.land.is_none() || self.on_title() {
            self.extras_next = e;
            save(&e);
            return;
        }
        // a game's own extras changed while it is played: an action (replay.rs)
        if self.rec.depth_zero() {
            self.act(sg_core::action::Action::Extras(e));
            return;
        }
        // a new path style redraws the paths (paths_lamps.rs)
        self.dirty |= self.club.extras.path_material() != e.path_material();
        self.club.extras = e;
        self.econ.extras = e;
        if !e.cut_content && !e.animals && !e.lamps() {
            self.strip_page = PAGE_OWNED;
        }
        if !e.set_par {
            self.club.restore_pars();
        }
    }

    /// The Set par extra's buttons on Hole Stats, right of the par: the par each one offers and its rectangle (none for a par
    /// out of `PAR_RANGE`, nothing at all when the hole's par may not be changed now).
    pub fn par_buttons(&self) -> Vec<(i32, Rect)> {
        let h = self.info.stats_hole.clamp(1, 18);
        // (Course A's holes only: the Second course extra's Course B keeps the game's pars, sg_core::second)
        if self.hole_b() || !self.club.can_set_par(h) {
            return Vec::new();
        }
        let par = self.club.holes[h].par;
        [(par - 1, 568.0), (par + 1, 589.0)]
            .into_iter()
            .filter(|(p, _)| sg_core::extras::PAR_RANGE.contains(p))
            .map(|(p, x)| (p, Rect::new(x, 122.0, 18.0, 17.0)))
            .collect()
    }

    /// Draws the par buttons ("-" and "+", lit under the pointer).
    pub fn draw_par_buttons(&self, g: &mut crate::gfx::Gfx, s: &crate::ui::Screen) {
        use crate::info_ui::c15;
        let par = self.club.holes[self.info.stats_hole.clamp(1, 18)].par;
        let (px, py) = self.info.pointer;
        for (p, r) in self.par_buttons() {
            let lit = r.has(px, py);
            s.fill(g, r.x - 1.0, r.y - 1.0, r.w + 2.0, r.h + 2.0, c15(0x0000));
            s.fill(g, r.x, r.y, r.w, r.h, c15(if lit { 0x5ade } else { 0x3dd4 }));
            let t = if p < par { "-" } else { "+" };
            s.text_centered(g, r.x + r.w * 0.5, r.y + 14.0, t, 14.0, c15(if lit { 0x7fff } else { 0x0000 }));
        }
    }

    /// The Hole flyover extra's button on Hole Stats (showcase.rs): only for an open hole.
    pub fn flyover_button(&self) -> Option<Rect> {
        let h = self.info.stats_hole.clamp(1, 18);
        (crate::extras::hole_flyover() && self.club.holes[h].par != 0).then(|| Rect::new(652.0, 36.0, 96.0, 22.0))
    }

    /// Draws the Fly over button (lit under the pointer), in the par buttons' colours.
    pub fn draw_flyover_button(&self, g: &mut crate::gfx::Gfx, s: &crate::ui::Screen) {
        use crate::info_ui::c15;
        let Some(r) = self.flyover_button() else { return };
        let (px, py) = self.info.pointer;
        let lit = r.has(px, py);
        s.fill(g, r.x - 1.0, r.y - 1.0, r.w + 2.0, r.h + 2.0, c15(0x0000));
        s.fill(g, r.x, r.y, r.w, r.h, c15(if lit { 0x5ade } else { 0x3dd4 }));
        s.text_centered(
            g,
            r.x + r.w * 0.5,
            r.y + 16.0,
            sg_core::lang::tr("STR_HOLESTATS_FLY_OVER"),
            14.0,
            c15(if lit { 0x7fff } else { 0x0000 }),
        );
    }

    /// A click on Hole Stats: a par button asks to confirm the change (true), anything else is the original's close.
    pub fn hole_stats_click(&mut self, x: f32, y: f32) -> bool {
        if self.flyover_button().is_some_and(|r| r.has(x, y)) {
            let h = self.info.stats_hole.clamp(1, 18);
            return self.start_flyover(h);
        }
        let Some((p, _)) = self.par_buttons().into_iter().find(|(_, r)| r.has(x, y)) else { return false };
        self.ask_par(p);
        true
    }

    /// The Set par extra's keys on Hole Stats: minus and plus ask for one par less or more. Returns whether the key was one.
    pub fn hole_stats_key(&mut self, k: miniquad::KeyCode) -> bool {
        use miniquad::KeyCode;
        let par = self.club.holes[self.info.stats_hole.clamp(1, 18)].par;
        let want = match k {
            KeyCode::Minus | KeyCode::KpSubtract => par - 1,
            KeyCode::Equal | KeyCode::KpAdd => par + 1,
            _ => return false,
        };
        if !self.par_buttons().iter().any(|&(p, _)| p == want) {
            return false;
        }
        self.ask_par(want);
        true
    }

    /// The confirmation box for hole Stats' hole getting par `p`.
    pub fn ask_par(&mut self, p: i32) {
        self.par_change = Some((self.info.stats_hole.clamp(1, 18), p));
        self.ui_sound(0x0b);
        self.open_popup(crate::popup_ui::PopupKind::SetPar);
    }

    /// The confirmation box's Yes: the hole gets the par asked for.
    pub fn confirm_par(&mut self) {
        if let Some((h, p)) = self.par_change.take() {
            self.act(sg_core::action::Action::SetPar { hole: h as i32, par: p });
        }
    }

    /// The gameplay extras the extras box shows: in a game, its own; on the title screens (a game left there too), what the
    /// next game starts with.
    pub fn extras_shown(&self) -> Extras {
        if self.land.is_some() && !self.on_title() {
            self.club.extras
        } else {
            self.extras_next
        }
    }

    /// Tab with the Rotate extra on: steps through the designs of the armed garden item or landmark strip, or, with no
    /// building tool in hand, turns the building under the pointer a quarter (the clubhouse too). Returns true when it did
    /// one of those (the original's Tab is not taken then).
    pub fn extras_tab(&mut self) -> bool {
        // Path styles and lamps: with the Pathway tool in hand, Tab steps the course's path style (paths_lamps.rs)
        if self.edit && self.tool == 2 && self.club.extras.lamps() && self.act(sg_core::action::Action::PathStyle) {
            return true;
        }
        if !self.club.extras.rotate {
            return false;
        }
        if self.edit && self.tool == 4 {
            let kind = self.build_idx as i32;
            let designs = self.strip_designs(kind);
            if designs.is_empty() {
                return false;
            }
            let cur = self.design.filter(|d| d.0 == kind).map(|d| d.1);
            let next = match cur.and_then(|c| designs.iter().position(|&d| d == c)) {
                Some(i) => designs[(i + 1) % designs.len()],
                None => designs[0],
            };
            self.design = Some((kind, next));
            self.design_hover = next;
            self.ui_sound(0x0b);
            return true;
        }
        if !self.has_hit {
            return false;
        }
        let (a, b) = self.terrain.tile_of(self.hit_x, self.hit_z);
        self.act(sg_core::action::Action::Turn { x: a, y: b })
    }

    /// The Rotate extra's turn of the building whose footprint holds tile a, b: the Turn action (replay.rs).
    pub(crate) fn turn_building(&mut self, a: i32, b: i32) -> bool {
        self.ensure_land();
        let Some(l) = self.land.as_mut() else { return false };
        // any object whose footprint holds the tile (the generator's clubhouse carries no footprint flag, so not object_on)
        let on = |o: &land::Object| {
            let s = l.footprint_size(o);
            (o.a..o.a + s).contains(&a) && (o.b..o.b + s).contains(&b)
        };
        let Some(i) = l.objects.iter().position(|o| o.kind >= 0 && turnable(o.kind) && on(o)) else { return false };
        l.objects[i].dir = (l.objects[i].dir + 1) & 3;
        self.after_object_change();
        self.snd("Interface/Building.wav", 0.5, false);
        true
    }

    /// The pages the landmark strip has: the club's own landmarks while it owns one, then the cut landmarks and the animals
    /// as their extras are on.
    pub fn strip_pages(&self) -> Vec<u8> {
        let mut v = Vec::new();
        if self.club.landmarks_shown() & 0x3fff != 0 {
            v.push(PAGE_OWNED);
        }
        // the cut landmarks' art is the original game's: no page on the free base set (a game with the extra on keeps it)
        if self.club.extras.cut_content && !self.on_free_set() {
            v.push(PAGE_CUT);
        }
        if self.club.extras.animals && !sg_core::wildlife::buyable_kinds(self.exe_theme()).is_empty() {
            v.push(PAGE_ANIMALS);
        }
        if self.club.extras.lamps() {
            v.push(PAGE_LAMPS);
        }
        v
    }

    /// The landmark strip's page shown (the first one there is when the one chosen is not there).
    pub fn strip_page_shown(&self) -> Option<u8> {
        let pages = self.strip_pages();
        if pages.contains(&self.strip_page) {
            Some(self.strip_page)
        } else {
            pages.first().copied()
        }
    }

    /// The landmark button pressed again while its strip is open: the next page.
    pub fn next_strip_page(&mut self) {
        let pages = self.strip_pages();
        if pages.len() > 1 {
            let cur = self.strip_page_shown().unwrap_or(PAGE_OWNED);
            let i = pages.iter().position(|&p| p == cur).unwrap_or(0);
            self.strip_page = pages[(i + 1) % pages.len()];
            self.design = None;
            self.design_hover = -1;
        }
    }

    /// The landmark types of an extra page in strip order (empty for the club's own page, which is the original's).
    pub fn extra_page_types(&self, page: u8) -> Vec<i32> {
        match page {
            PAGE_CUT => (0..CUT_LANDMARKS.len() as i32).map(|i| CUT_BASE + i).collect(),
            PAGE_ANIMALS => sg_core::wildlife::buyable_kinds(self.exe_theme()).into_iter().map(|k| ANIMAL_BASE + k).collect(),
            PAGE_LAMPS => vec![sg_core::extras::LAMP_TYPE],
            _ => Vec::new(),
        }
    }

    /// The designs Tab steps through for a kind: the strip's designs shown (for landmarks, the page shown).
    fn strip_designs(&self, kind: i32) -> Vec<i32> {
        match kind {
            land::K_BENCH => (0..5).collect(),
            land::K_BRIDGE => (0..8).collect(),
            land::K_FLOWERS => (0..15).collect(),
            land::K_WILLOW => (0..7).collect(),
            land::K_LANDMARK => match self.strip_page_shown() {
                Some(PAGE_OWNED) => (0..14).filter(|&t| self.club.landmarks_shown() & (1 << t) != 0).collect(),
                Some(p) => self.extra_page_types(p),
                None => Vec::new(),
            },
            _ => Vec::new(),
        }
    }

    /// Whether landmark type `t` may be placed: the club owns it (the original) or the Unlock all extra is on, or it is a cut
    /// landmark or an animal and that extra is on.
    pub fn landmark_allowed(&self, t: i32) -> bool {
        if t == sg_core::extras::LAMP_TYPE {
            return self.club.extras.lamps();
        }
        if (0..CUT_BASE).contains(&t) {
            return self.club.landmarks_shown() & (1 << t) != 0;
        }
        if (CUT_BASE..CUT_BASE + CUT_LANDMARKS.len() as i32).contains(&t) {
            return self.club.extras.cut_content;
        }
        self.club.extras.animals && sg_core::wildlife::buyable_kinds(self.exe_theme()).contains(&(t - ANIMAL_BASE))
    }

    /// The price in units of a landmark type the original does not have (a cut landmark, an animal pair).
    pub fn extra_landmark_units(t: i32) -> i32 {
        if t == sg_core::extras::LAMP_TYPE {
            sg_core::extras::LAMP_UNITS
        } else if t >= ANIMAL_BASE {
            sg_core::extras::ANIMAL_UNITS
        } else {
            sg_core::extras::CUT_LANDMARK_UNITS
        }
    }

    /// The landmark tool with an animal chosen: buys a pair of that animal on the tile (the Buyable animals extra).
    pub fn buy_animal(&mut self, a: i32, b: i32, t: i32) {
        let ok_tile = sg_core::course::inside(a, b) && {
            let ty = self.course.ty[sg_core::course::idx(a, b)];
            !self.course.oob(a, b) && ty != land::T_WATER && ty != land::T_BUILDING && ty != land::T_HOME_SITE
        };
        if !ok_tile {
            self.ui_sound(24);
            return self.show_toast(sg_core::lang::tr("STR_TOAST_ANIMAL_NOT_HERE"));
        }
        let cost = sg_core::extras::ANIMAL_UNITS as f64 * sg_core::economy::Economy::UNIT;
        if !self.econ.affordable(cost, self.hole_numbers.len()) {
            return self.show_toast(&sg_core::lang::trf(
                "STR_TOAST_CHANGE_COSTS",
                &[("COST", &crate::ui::money(cost as i64)), ("CASH", &crate::ui::money(self.econ.cash as i64))],
            ));
        }
        if !self.club.wildlife.buy(&mut self.exe_rng, a, b, t - ANIMAL_BASE) {
            self.ui_sound(24);
            return self.show_toast(sg_core::lang::tr("STR_TOAST_ANIMALS_FULL"));
        }
        let cash = self.econ.cash;
        self.econ.spend_to(sg_core::economy::LEDGER_FACILITIES, cost);
        self.float_change(cash, (a, b));
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::extras::Settings;

    #[test]
    fn every_option_is_on_exactly_one_page() {
        let all: Vec<ExtrasLine> = PAGE_LINES.iter().flat_map(|p| p.iter().copied()).collect();
        for l in &all {
            assert_eq!(all.iter().filter(|&m| m == l).count(), 1, "{l:?} must be on exactly one page");
        }
        // every gameplay option: on a page, or (Slower seasons, Pause the calendar) the Calendar line
        for i in 0..sg_core::extras::OPTION_KEYS.len() {
            let shown = all.contains(&ExtrasLine::Game(i));
            assert_eq!(shown, i != 6 && i != 7, "option {i}");
        }
        assert!(all.contains(&ExtrasLine::Calendar));
        // every interface tick: on a page, in exactly one group, or (the update check) on Preferences itself; the retired
        // desert art tick nowhere
        for b in 0..crate::extras::UI_KEYS.len() {
            let places = all.contains(&ExtrasLine::Ui(b)) as usize
                + GROUPS.iter().filter(|g| g.2.contains(&b)).count()
                + (b == UPDATE_CHECK || b == RETIRED_DESERT_ART) as usize;
            assert_eq!(places, 1, "interface extra {b}");
        }
        assert_eq!(PAGES.len(), PAGE_LINES.len());
        assert_eq!(BANNERS.len(), PAGES.len());
        assert_eq!(page_lines(3).len(), 5, "the Info page's five lines");
    }

    #[test]
    fn each_page_holds_one_class() {
        use sg_core::extras as x;
        for (p, lines) in PAGE_LINES.iter().enumerate() {
            for &l in lines.iter() {
                assert_eq!(tag(l).is_some(), p <= 1, "{l:?} on page {p}");
                assert_eq!(page_of(l), Some(p as u8));
            }
        }
        // looks: only the picture changes
        for l in [ExtrasLine::Game(0), ExtrasLine::Game(x::PATHS), ExtrasLine::Ui(21), ExtrasLine::Ui(26), ExtrasLine::Ui(28)] {
            assert_eq!(page_of(l), Some(2), "{l:?}");
        }
        // animals cost money and lift moods: gameplay, a light touch
        assert_eq!((page_of(ExtrasLine::Game(4)), tag(ExtrasLine::Game(4))), (Some(0), Some(Tag::Light)));
        assert_eq!(tag(ExtrasLine::Game(2)), Some(Tag::Big), "hardcore");
        assert_eq!(tag(ExtrasLine::Game(CUT)), Some(Tag::Experimental));
        assert_eq!(tag(ExtrasLine::Game(x::WEATHER)), Some(Tag::Noticeable));
        assert_eq!(tag(ExtrasLine::Calendar), Some(Tag::Noticeable));
        assert_eq!(tag(ExtrasLine::Game(x::LEFTIES)), Some(Tag::Light));
        assert_eq!((page_of(ExtrasLine::Game(x::KEEP_LAND)), tag(ExtrasLine::Game(x::KEEP_LAND))), (Some(1), Some(Tag::Light)));
        assert_eq!((page_of(ExtrasLine::Game(x::SECOND_COURSE)), tag(ExtrasLine::Game(x::SECOND_COURSE))), (Some(0), Some(Tag::Big)));
        assert_eq!((page_of(ExtrasLine::Game(x::TEES)), tag(ExtrasLine::Game(x::TEES))), (Some(0), Some(Tag::Light)));
        assert_eq!(page_of(ExtrasLine::Ui(5)), Some(4), "autosave");
        // the Controls page holds the mouse ticks, the most wanted first, then the two pauses
        assert_eq!(page_lines(5), [14, 25, 15, 20, 27, 24, 8, 9].map(ExtrasLine::Ui));
        // the Saves page holds the saving and sharing ticks only, autosave first
        assert_eq!(page_lines(4), [ExtrasLine::Ui(5), ExtrasLine::Ui(22), ExtrasLine::Ui(7), ExtrasLine::Ui(6)]);
        // the most wanted first: weather on Gameplay, the interface at the edges on Looks; the experimental line last
        assert_eq!(page_lines(0)[0], ExtrasLine::Game(x::WEATHER));
        assert_eq!(page_lines(2)[0], ExtrasLine::Ui(26));
        assert_eq!(page_lines(0).last(), Some(&ExtrasLine::Game(CUT)));
        assert_eq!((page_of(ExtrasLine::Preset), tag(ExtrasLine::Preset)), (None, None));
        assert_eq!(page_of(ExtrasLine::Ui(UPDATE_CHECK)), None);
    }

    #[test]
    fn the_cut_content_shows_only_with_the_original_files() {
        assert!(page_lines(0).contains(&ExtrasLine::Game(CUT)));
        show_cut_content(false);
        assert!(!page_lines(0).contains(&ExtrasLine::Game(CUT)));
        let n = popup_lines(0, &Extras::default(), &Settings::ALL_OFF).len();
        show_cut_content(true);
        // the box keeps its size either way (most_lines counts the hidden line)
        assert_eq!(popup_lines(0, &Extras::default(), &Settings::ALL_OFF).len(), n);
    }

    #[test]
    fn every_line_and_page_has_its_words() {
        let keys = PAGES.iter().chain(&BANNERS).chain(&HINTS).chain(&GAME_DESC).chain(&UI_DESC);
        let groups = GROUPS.iter().flat_map(|g| [g.0, g.1]);
        let presets = Preset::ALL.iter().flat_map(|p| [p.key(), p.desc()]);
        let more = ["STR_EXTRAS_CALENDAR", "STR_EXTRAS_DESC_CALENDAR", "STR_EXTRAS_SOME_ON", "STR_EXTRAS_PRESET"];
        for k in keys.copied().chain(Tag::ALL.map(Tag::key)).chain(groups).chain(presets).chain(more).chain(sg_core::extras::CALENDAR_KEYS)
        {
            assert!(sg_core::lang::has(k), "lang/en.txt has no {k}");
        }
        // the foot shows the line's description (the box's option 0 is the preset line), or the page's hint with none
        let (e, ui) = (Extras::default(), Settings::ALL_OFF);
        assert_eq!(foot_text(2, Some(2), &e, &ui), (sg_core::lang::tr("STR_EXTRAS_DESC_PATHS"), true));
        assert_eq!(foot_text(2, Some(0), &e, &ui), (sg_core::lang::tr("STR_PRESET_DESC_ORIGINAL"), true));
        assert_eq!(foot_text(0, Some(0), &Extras::OPENSGOLF, &Settings::HELPERS).0, sg_core::lang::tr("STR_PRESET_DESC_OPENSGOLF"));
        assert!(!foot_text(2, Some(7), &e, &ui).1);
        assert!(!foot_text(3, None, &e, &ui).1);
    }

    #[test]
    fn every_page_is_as_tall_and_the_preset_heads_it() {
        let (e, ui) = (Extras::default(), Settings::ALL_OFF);
        let n = popup_lines(0, &e, &ui).len();
        for p in 0..PAGES.len() as u8 {
            let lines = popup_lines(p, &e, &ui);
            assert_eq!(lines.len(), n);
            // the foot's rows are no options
            assert!(lines[n - FOOT_LINES..].iter().all(|l| l.is_empty()));
            // the preset line is the first option, over the tabs, and drawn without a tick box
            assert!(lines[1].starts_with(' ') && lines[2].is_empty());
            assert_eq!(box_lines(p)[0], ExtrasLine::Preset);
            assert_eq!(popup_plain(p) & 1, 1);
        }
        // the tallest box still fits an 800 x 600 screen from the box's top (popup_ui: 92)
        assert!(92.0 + (n as f32 * 3.0 + 3.0) * 8.0 <= 584.0, "{n} lines");
    }

    #[test]
    fn presets_match_their_ticks_and_keep_the_players_own() {
        use Preset::*;
        let none = Settings::ALL_OFF;
        assert_eq!(Preset::of(&Extras::default(), &none), Original);
        assert_eq!(Preset::of(&Extras::LOOKS, &Settings::HELPERS), Helpers);
        assert_eq!(Preset::of(&Extras::OPENSGOLF, &Settings::HELPERS), OpenSGolf);
        // the OpenSGolf preset: helpers, looks, more staff, animals, weather, tees, the second course; nothing else
        let o = Extras::OPENSGOLF;
        assert!(o.more_staff && o.animals && o.rotate && o.path_style == 1 && o.weather && o.tees && o.second_course);
        assert!(!o.more_visitors && !o.hardcore && !o.lefties && !o.set_par && !o.cut_content && !o.keep_land);
        assert!(o.calendar() == 0 && !o.freeze_moods && !o.unlock_all && !o.no_weeds);
        let fewer = Extras { more_staff: false, animals: false, weather: false, tees: false, second_course: false, ..o };
        assert_eq!(fewer, Extras::LOOKS);
        // the kept ticks (pauses, update check, controls presets) do not count
        let kept = Settings {
            pause_background: true,
            pause_events: true,
            update_check: true,
            zoom_pointer: true,
            drag_place: true,
            right_drag: true,
            mouse_swing: true,
            ..Settings::ALL_OFF
        };
        assert_eq!(Preset::of(&Extras::default(), &kept), Original);
        assert_eq!(Preset::of(&Extras::OPENSGOLF, &Settings { right_drag: true, ..Settings::HELPERS }), OpenSGolf);
        // any path style is the path styles of the presets; a stopped calendar is one whatever speed is stored
        assert_eq!(Preset::of(&Extras { path_style: 4, ..Extras::OPENSGOLF }, &Settings::HELPERS), OpenSGolf);
        // one tick more or less is the player's own mix
        assert_eq!(Preset::of(&Extras { weather: false, ..Extras::OPENSGOLF }, &Settings::HELPERS), Custom);
        assert_eq!(Preset::of(&Extras::default(), &Settings { people: true, ..none }), Custom);
        assert_eq!(Preset::of(&Extras::LOOKS, &Settings { random_trees: false, ..Settings::HELPERS }), Custom);
        assert_eq!(Preset::of(&Extras { pause_calendar: true, season_speed: 2, ..Extras::default() }, &none), Custom);
        // applying keeps the kept ticks and a chosen path style
        let (mut e, mut ui) = (Extras { path_style: 3, hardcore: true, ..Extras::default() }, kept);
        assert!(OpenSGolf.apply(&mut e, &mut ui, None));
        assert_eq!((e, ui), (Extras { path_style: 3, ..Extras::OPENSGOLF }, Settings::HELPERS.with_kept(&kept)));
        assert_eq!(Preset::of(&e, &ui), OpenSGolf);
        assert!(Original.apply(&mut e, &mut ui, None));
        assert_eq!((e, ui), (Extras::default(), kept));
        assert!(!Custom.apply(&mut e, &mut ui, None), "no mix of the player's own to go back to");
    }

    #[test]
    fn the_preset_line_cycles_and_custom_comes_back() {
        use Preset::*;
        let (mut e, mut ui) = (Extras::default(), Settings::ALL_OFF);
        let mut seen = Vec::new();
        for _ in 0..4 {
            seen.push(cycle_preset(Preset::of(&e, &ui), &mut e, &mut ui, None));
        }
        assert_eq!(seen, vec![Helpers, OpenSGolf, Original, Helpers], "no Custom without a mix");
        let mine = (Extras { weather: true, ..Extras::default() }, Settings { people: true, ..Settings::ALL_OFF });
        let mut seen = Vec::new();
        for _ in 0..4 {
            seen.push(cycle_preset(Preset::of(&e, &ui), &mut e, &mut ui, Some(mine)));
        }
        assert_eq!(seen, vec![OpenSGolf, Custom, Original, Helpers]);
        // Custom brings the mix back, the kept ticks as they are now
        let (mut e, mut ui) = (Extras::OPENSGOLF, Settings { pause_events: true, ..Settings::HELPERS });
        assert_eq!(cycle_preset(OpenSGolf, &mut e, &mut ui, Some(mine)), Custom);
        assert_eq!((e, ui), (mine.0, Settings { pause_events: true, ..mine.1 }));
        // a kept "mix" that is a preset is no mix
        assert_eq!(cycle_preset(OpenSGolf, &mut e, &mut ui, Some((Extras::default(), Settings::ALL_OFF))), Original);
    }

    #[test]
    fn the_calendar_and_the_groups_are_one_line_each() {
        let ui = Settings::ALL_OFF;
        let mut e = Extras::default();
        let mut shown = Vec::new();
        for _ in 0..4 {
            toggle_line(ExtrasLine::Calendar, &mut e, &mut ui.clone());
            shown.push(line_text(ExtrasLine::Calendar, &e, &ui));
        }
        assert_eq!(shown.iter().map(|t| t.1).collect::<Vec<_>>(), [true, true, true, false]);
        assert!(shown[2].0.contains(sg_core::lang::tr("STR_EXTRAS_CALENDAR_STOPPED")) && e.is_off());
        // a group: all on, mixed (shown as some on; a click turns all on), all off
        let people = ExtrasLine::Group(0);
        let mut ui = Settings { name_tags: true, ..Settings::ALL_OFF };
        assert_eq!(line_value(people, &e, &ui), 2);
        assert!(!line_text(people, &e, &ui).1 && line_text(people, &e, &ui).0.contains("("));
        toggle_line(people, &mut e, &mut ui);
        assert!(ui.people && ui.name_tags && ui.live_scorecards && line_text(people, &e, &ui).1);
        toggle_line(people, &mut e, &mut ui);
        assert_eq!(ui, Settings::ALL_OFF);
        // the groups' ticks stay single bits: the reports line ticks its five
        toggle_line(ExtrasLine::Group(1), &mut e, &mut ui);
        assert_eq!(
            ui,
            Settings { landing_map: true, hole_comments: true, graphs: true, year_summary: true, report_tabs: true, ..Settings::ALL_OFF }
        );
        // the box's lines measure every value they can show (no change of size on a click)
        let w = popup_widest(&e, &ui);
        assert_eq!(w, popup_widest(&Extras::OPENSGOLF, &Settings { people: true, ..Settings::HELPERS }));
    }
}
