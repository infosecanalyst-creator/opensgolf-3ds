//! OpenSGolf extras (docs/PLAYING.md, "OpenSGolf extras"): the port's own additions to the interface, none of them in the
//! original. Each is purely additive (it adds a screen, a tag or a strip; nothing of the original's is taken away or
//! changed); a new player starts with them off (the original game) and Select Difficulty turns them all on in one click; Preferences > *OpenSGolf extras...* has a tick for each, kept in settings.json under
//! "ui_extras", and `--no-extras` turns them all off for one run (not saved).
//!
//! * the People screen (people_ui.rs): every golfer on the course and every employee, sortable, with Locate, Follow, the
//!   golfer card, the golfer's profile page, and for the staff Hire, Fire, Move and Rename, and the player's pro's screens;
//! * the message history (history_ui.rs): every ticker message kept with its date and speaker, saved with the game;
//! * name tags under the pointer (tags_ui.rs): the name and a mood face of the golfer or employee under the pointer;
//! * tabbed reports (report_tabs.rs): a strip of tabs over the F1 to F10 reports to flip between them;
//! * path help (pathview_ui.rs): a path view marking buildings and holes the paths do not reach, and a Show button on
//!   messages about a place;
//! * autosave (autosave.rs): the game saved every five minutes of play, as "autosave (Latest)" in the Load list;
//! * save pictures (files_ui.rs): a map of the course beside the Load list's saves.
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
}

impl Settings {
    pub const ALL_ON: Settings =
        Settings { people: true, history: true, name_tags: true, report_tabs: true, path_help: true, autosave: true, save_pictures: true };
    pub const ALL_OFF: Settings = Settings {
        people: false,
        history: false,
        name_tags: false,
        report_tabs: false,
        path_help: false,
        autosave: false,
        save_pictures: false,
    };

    /// The ticks as the Extras box's checkbox bits (in `LINES` order).
    pub fn bits(&self) -> u32 {
        [self.people, self.history, self.name_tags, self.report_tabs, self.path_help, self.autosave, self.save_pictures]
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
        }
    }
}

impl Default for Settings {
    fn default() -> Self {
        Settings::ALL_ON
    }
}

/// The Extras box's lines under the gameplay extras (popup_ui.rs): an empty line (a heading under options is drawn at the
/// options' row height, so it needs the room), the heading, then one tick per extra (text keys in lang/en.txt), then an
/// empty line that keeps the OK tick clear of the last one.
pub const LINES: [&str; 10] = [
    "STR_EXTRAS_END",
    "STR_EXTRAS_UI_HEAD",
    "STR_EXTRAS_PEOPLE",
    "STR_EXTRAS_HISTORY",
    "STR_EXTRAS_NAME_TAGS",
    "STR_EXTRAS_REPORT_TABS",
    "STR_EXTRAS_PATH_HELP",
    "STR_EXTRAS_AUTOSAVE",
    "STR_EXTRAS_SAVE_PICTURES",
    "STR_EXTRAS_END",
];

thread_local! {
    static SETTINGS: Cell<Settings> = const { Cell::new(Settings::ALL_ON) };
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

fn settings_json() -> serde_json::Value {
    sg_core::fsutil::read_file(crate::app::save_dir().join(crate::hd::SETTINGS_FILE))
        .and_then(|d| serde_json::from_slice::<serde_json::Value>(&d).ok())
        .filter(|v| v.is_object())
        .unwrap_or_else(|| serde_json::json!({}))
}

/// The ticks saved in settings.json: all on for a player whose settings predate them, all off on a first start (a new
/// player starts with the original game; Select Difficulty switches to the extras in one click).
pub fn saved() -> Settings {
    if sg_core::fsutil::read_file(crate::app::save_dir().join(crate::hd::SETTINGS_FILE)).is_none() {
        return Settings::ALL_OFF;
    }
    let v = settings_json()[SETTINGS_KEY].clone();
    let mut s: Settings = serde_json::from_value(v.clone()).unwrap_or_default();
    // ticks saved before autosave and save pictures existed: those two follow the People screen's tick (all on or all off)
    if v.is_object() && v.get("autosave").is_none() {
        s.autosave = s.people;
        s.save_pictures = s.people;
    }
    s
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
    set(if off { Settings::ALL_OFF } else { saved() });
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
}

impl Kept {
    pub fn is_empty(&self) -> bool {
        self.messages.is_empty() && self.staff_names.is_empty() && self.said.is_empty()
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
    pub path_marks: Option<crate::pathview_ui::Marks>,
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
    let x = (r.x + (r.w - f.width(label)) * 0.5).floor();
    let y = r.y + ((r.h - f.px) * 0.5).floor();
    s.put(g, f, x, y, label, c15(0x0018));
}

/// A command button in the look of the Keyboard controls screen's page buttons (keys_ui): navy on the pale blue, lit under
/// the pointer, grey when it cannot be used.
pub fn draw_button(g: &mut Gfx, s: &Ui, r: Rect, label: &str, enabled: bool, hover: bool) {
    let f = crate::ui::F_ARIAL10;
    let lit = enabled && hover;
    s.fill(g, r.x - 1.0, r.y - 1.0, r.w + 2.0, r.h + 2.0, c15(if lit { 0x7fff } else { 0x2d6b }));
    s.fill(g, r.x, r.y, r.w, r.h, if lit { crate::ui::rgb(0.85, 0.85, 1.0) } else { crate::ui::rgb(0.47, 0.47, 0.70) });
    let y = r.y + ((r.h - 10.0) * 0.5).floor();
    s.put_centered(g, f, r.x + r.w / 2.0, y, label, c15(if enabled { 0x0018 } else { 0x6318 }));
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

    /// Shift+H: the name tags off or on (saved with the other ticks).
    pub fn toggle_name_tags(&mut self) {
        let mut s = settings();
        s.name_tags = !s.name_tags;
        set(s);
        save(&s);
        self.show_toast(sg_core::lang::tr(if s.name_tags { "STR_TOAST_NAME_TAGS_ON" } else { "STR_TOAST_NAME_TAGS_OFF" }));
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
        let tick = self.game_tick;
        self.xs.kept.push_said(roster, Said { tick, text, tone });
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn bits_round_trip_and_old_settings_read_as_on() {
        for m in 0..32 {
            assert_eq!(Settings::from_bits(m).bits(), m);
        }
        let s: Settings = serde_json::from_str("{\"people\": false}").unwrap();
        assert_eq!(s, Settings { people: false, ..Settings::ALL_ON });
    }
}
