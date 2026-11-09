//! The support reminder (the port's own, none of it in the original; docs/PLAYING.md "Supporting OpenSGolf"): the game
//! counts the time played on the course (the game running, not paused and not in the background) on the player's own
//! device, in settings.json (the browser's storage on the web, the app's folder on Android). The counter is never sent;
//! the play-time count (playcount.rs) sends only a mark for each 15 minutes of it.
//!
//! After about an hour of play, at the next natural pause (the title menu, or just after the year-end report closes, never
//! while building, while the pro is out, in a tournament or over another box) a small box asks once: Support on Buy Me a
//! Coffee, Maybe later (Esc, a right-click or a click outside the box too) or Don't ask again. Maybe later asks once more
//! about five hours of play later; then never again. Preferences > More > Support OpenSGolf opens the same page at any time.
//! With the counter under an hour nothing at all changes on screen; saved games never hold the counter.
//!
//! Test hooks: SG_PLAYTIME=SECONDS starts the counter there; SG_SUPPORT=1 shows the box at the first title menu or course
//! frame, whatever the counter (for stills).
use crate::app::{App, Screen};
use serde::{Deserialize, Serialize};
use sg_core::lang::{tr, trf};

/// The project's Buy Me a Coffee page.
pub const URL: &str = "https://buymeacoffee.com/crspytopgn";
/// settings.json's key.
pub const SETTINGS_KEY: &str = "support";
/// The play time before the first ask, and between the first and the second, in seconds.
pub const FIRST_ASK: f64 = 3600.0;
pub const SECOND_ASK: f64 = 5.0 * 3600.0;
/// The most the box ever asks.
pub const MAX_ASKS: u32 = 2;
/// The play time counted between two writes of settings.json.
const SAVE_EVERY: f64 = 60.0;
/// How long the title menu shows before the box comes up there.
const TITLE_SETTLE: f64 = 1.5;

/// What is kept in settings.json.
#[derive(Clone, Debug, Default, PartialEq, Serialize, Deserialize)]
#[serde(default)]
pub struct Record {
    /// Seconds played on the course.
    pub played: f64,
    /// Times the box was shown.
    pub asks: u32,
    /// `played` when it was last shown.
    pub asked_at: f64,
    /// Support or Don't ask again was chosen: never again.
    pub done: bool,
}

/// The box's answers, in the order of its options.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Answer {
    Support,
    Later,
    Never,
}

impl Answer {
    /// The answer for the box's option `k` (None: Esc, a right-click or a click outside the box).
    pub fn from_option(k: Option<usize>) -> Answer {
        match k {
            Some(0) => Answer::Support,
            Some(2) => Answer::Never,
            _ => Answer::Later,
        }
    }
}

impl Record {
    /// Whether the box is due: an hour played before the first ask, five more before the second, never after two asks or
    /// after Support or Don't ask again.
    pub fn due(&self) -> bool {
        if self.done {
            return false;
        }
        match self.asks {
            0 => self.played >= FIRST_ASK,
            1 => self.played >= self.asked_at + SECOND_ASK,
            _ => false,
        }
    }

    /// The box is shown.
    pub fn asked(&mut self) {
        self.asks += 1;
        self.asked_at = self.played;
    }

    /// The box's answer.
    pub fn answer(&mut self, a: Answer) {
        if a != Answer::Later || self.asks >= MAX_ASKS {
            self.done = true;
        }
    }
}

/// Where the game is when the box might come up.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Spot {
    /// The title menu.
    Title,
    /// The course, back from the year-end report (and what followed it: the year in numbers, the Top 10).
    AfterYearEnd,
    /// The course otherwise.
    Course,
    /// Any other screen or box.
    Elsewhere,
}

/// What else is going on.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub struct Busy {
    /// Building: a tool or a building in hand.
    pub building: bool,
    /// The pro is out on a round (aiming, walking).
    pub pro_out: bool,
    /// A tournament is on.
    pub tournament: bool,
    /// Another box or message is up (a notice, the rename or save prompt, a photo).
    pub other_box: bool,
}

/// Whether the box may come up at `at`: only at a natural pause; `forced` (SG_SUPPORT) on the course too.
pub fn natural_pause(at: Spot, busy: Busy, forced: bool) -> bool {
    if busy.other_box {
        return false;
    }
    let playing = busy.building || busy.pro_out || busy.tournament;
    match at {
        Spot::Title => true,
        Spot::AfterYearEnd => !playing,
        Spot::Course => forced && !playing,
        Spot::Elsewhere => false,
    }
}

/// The reminder's state (`App::support`).
#[derive(Default)]
pub struct State {
    pub record: Record,
    /// Play time counted since settings.json was last written.
    unsaved: f64,
    /// The window is in the background.
    pub hidden: bool,
    /// The screen of the last frame.
    last: Option<Screen>,
    /// The year-end report closed; the box may come up when the game is back on the course.
    after_year_end: bool,
    /// When the title menu came up.
    title_since: f64,
    /// SG_SUPPORT: the box once, whatever the counter.
    forced: bool,
}

fn settings_json() -> serde_json::Value {
    sg_core::fsutil::read_file(crate::app::save_dir().join(crate::hd::SETTINGS_FILE))
        .and_then(|d| serde_json::from_slice::<serde_json::Value>(&d).ok())
        .filter(|v| v.is_object())
        .unwrap_or_else(|| serde_json::json!({}))
}

/// At start up: the saved record (SG_PLAYTIME sets its play time) and the SG_SUPPORT hook.
pub fn init() -> State {
    let mut record: Record = serde_json::from_value(settings_json()[SETTINGS_KEY].clone()).unwrap_or_default();
    if let Some(s) = std::env::var("SG_PLAYTIME").ok().and_then(|v| v.trim().parse::<f64>().ok()) {
        record.played = s.max(0.0);
    }
    let forced = std::env::var_os("SG_SUPPORT").is_some_and(|v| !v.is_empty() && v != "0");
    State { record, forced, ..Default::default() }
}

/// Saves the record into settings.json, keeping its other keys.
fn save(r: &Record) {
    let mut v = settings_json();
    let mut r = r.clone();
    r.played = r.played.floor();
    r.asked_at = r.asked_at.floor();
    if let Ok(e) = serde_json::to_value(&r) {
        v[SETTINGS_KEY] = e;
    }
    if let Ok(t) = serde_json::to_string_pretty(&v) {
        sg_core::fsutil::write_file(crate::app::save_dir().join(crate::hd::SETTINGS_FILE), t.as_bytes());
    }
}

/// Opens the support page (a new tab in the browser, the system's browser elsewhere); false when none could be opened.
pub fn open_page() -> bool {
    crate::bugreport::open_link(URL)
}

impl App {
    /// The frame's play time: `dt` seconds counted when `playing` (the club's clock ran) and the window is in front.
    pub fn support_tick(&mut self, dt: f64, playing: bool) {
        let s = &mut self.support;
        if !playing || s.hidden || dt <= 0.0 {
            return;
        }
        s.record.played += dt;
        s.unsaved += dt;
        if s.unsaved >= SAVE_EVERY {
            s.unsaved = 0.0;
            save(&s.record);
        }
    }

    /// Writes the play time now (the play-time count's 15-minute mark: a quit soon after must not count it again).
    pub fn support_save_now(&mut self) {
        self.support.unsaved = 0.0;
        save(&self.support.record);
    }

    /// Once a frame: the box at a natural pause when it is due (`auto` false: scripted stills, which only SG_SUPPORT
    /// shows it in).
    pub fn support_frame(&mut self, auto: bool) {
        let now = self.screen;
        let s = &mut self.support;
        if s.last == Some(Screen::YearEnd) && now != Screen::YearEnd {
            s.after_year_end = true;
        }
        if now == Screen::Menu && s.last != Some(Screen::Menu) {
            s.title_since = self.clock;
        }
        s.last = Some(now);
        let at = match now {
            Screen::Menu if self.clock - s.title_since >= TITLE_SETTLE || s.forced => Spot::Title,
            Screen::Play if std::mem::take(&mut s.after_year_end) => Spot::AfterYearEnd,
            Screen::Play => Spot::Course,
            _ => Spot::Elsewhere,
        };
        if !(s.forced || auto && s.record.due()) {
            return;
        }
        let busy = Busy {
            building: self.edit,
            pro_out: self.club.gary != -1,
            tournament: self.club.game & sg_core::golfer::game::TOURNAMENT != 0,
            other_box: self.other_box_up(),
        };
        if !natural_pause(at, busy, self.support.forced) {
            return;
        }
        self.support.forced = false;
        self.support.record.asked();
        save(&self.support.record);
        self.hover = -1;
        self.open_popup(crate::popup_ui::PopupKind::Support);
    }

    /// Another box or message is up (a popup, a notice, the rename or save prompt, a photo, the launcher), or the interface
    /// art is missing: no box of the port's own comes up over it (this one, the welcome boxes).
    pub fn other_box_up(&self) -> bool {
        self.popup.is_some()
            || self.launcher.is_some()
            || self.notice.is_some()
            || self.rename.is_some()
            || self.title.save_name.is_some()
            || self.great_shot.is_some()
            || self.simfoto.is_some()
            || self.photo.is_some()
            || self.analysis.is_some()
            || !self.ui_ok
    }

    /// The box's answer (`k` its option, None for Esc, a right-click or a click outside it).
    pub fn support_answer(&mut self, k: Option<usize>) {
        let a = Answer::from_option(k);
        self.support.record.answer(a);
        save(&self.support.record);
        if a == Answer::Support {
            self.open_support_page();
        }
    }

    /// Preferences > More > Support OpenSGolf, and the box's Support: the page in the browser (the box asks no more).
    pub fn open_support_page(&mut self) {
        if !self.support.record.done {
            self.support.record.done = true;
            save(&self.support.record);
        }
        if !open_page() {
            #[cfg(not(target_arch = "wasm32"))]
            miniquad::window::clipboard_set(URL);
            self.show_toast(&trf("STR_SUPPORT_NO_BROWSER", &[("URL", &URL)]));
        }
    }
}

/// A time played in hours and minutes ("6 hours 12 minutes", "45 minutes"), for the note in Preferences > More.
pub fn time_words(secs: u64) -> String {
    let (h, m) = (secs / 3600, secs / 60 % 60);
    let hours = match h {
        1 => tr("STR_TIME_HOUR").to_string(),
        h => trf("STR_TIME_HOURS", &[("N", &h)]),
    };
    let minutes = match m {
        1 => tr("STR_TIME_MINUTE").to_string(),
        m => trf("STR_TIME_MINUTES", &[("N", &m)]),
    };
    match (h, m) {
        (0, _) => minutes,
        (_, 0) => hours,
        _ => format!("{hours} {minutes}"),
    }
}

/// How long the player has played, in whole hours (an hour at the least: the box never shows sooner).
fn played_words(played: f64) -> String {
    match (played / 3600.0).floor() as u64 {
        0 | 1 => tr("STR_SUPPORT_PLAYED_1").to_string(),
        h => trf("STR_SUPPORT_PLAYED", &[("HOURS", &h)]),
    }
}

/// The box's lines: the title, its words as notes (popup_ui's lines starting with a tab), then the three options.
pub fn box_lines(played: f64) -> Vec<String> {
    // an empty note under the title keeps the words clear of the free set's header band
    let mut v = vec![tr("STR_SUPPORT_TITLE").to_string(), "\t".to_string()];
    let words = format!("{} {}", played_words(played), tr("STR_SUPPORT_BODY"));
    v.extend(crate::extras::wrap(crate::popup_ui::NOTE_FONT, &words, 340.0).into_iter().map(|l| format!("\t{l}")));
    v.push("\t".to_string());
    v.extend([tr("STR_SUPPORT_YES"), tr("STR_SUPPORT_LATER"), tr("STR_SUPPORT_NEVER")].map(str::to_string));
    v
}

#[cfg(test)]
mod tests {
    use super::*;

    fn at(played: f64) -> Record {
        Record { played, ..Default::default() }
    }

    #[test]
    fn the_first_ask_comes_after_an_hour() {
        assert!(!at(0.0).due());
        assert!(!at(FIRST_ASK - 1.0).due());
        assert!(at(FIRST_ASK).due());
        assert!(at(FIRST_ASK * 3.0).due());
    }

    #[test]
    fn maybe_later_asks_once_more_five_hours_on_then_never() {
        let mut r = at(FIRST_ASK + 10.0);
        r.asked();
        assert!(!r.due(), "not twice in a row");
        r.answer(Answer::Later);
        assert!(!r.done);
        r.played += SECOND_ASK - 1.0;
        assert!(!r.due());
        r.played += 1.0;
        assert!(r.due(), "five hours after the first ask");
        r.asked();
        r.answer(Answer::Later);
        assert!(r.done, "two asks at most");
        r.played += 100.0 * 3600.0;
        assert!(!r.due());
        // the window closed with the second box up: no third ask either
        let mut r = Record { played: FIRST_ASK + SECOND_ASK, asks: 2, asked_at: FIRST_ASK + SECOND_ASK, done: false };
        r.played += 100.0 * 3600.0;
        assert!(!r.due());
    }

    #[test]
    fn support_or_dont_ask_again_ends_it() {
        for a in [Answer::Support, Answer::Never] {
            let mut r = at(FIRST_ASK);
            r.asked();
            r.answer(a);
            assert!(r.done);
            r.played += 100.0 * 3600.0;
            assert!(!r.due(), "{a:?}");
        }
        assert!(!Record { done: true, ..at(1e9) }.due());
    }

    #[test]
    fn the_options_map_to_answers_and_cancel_is_later() {
        assert_eq!(Answer::from_option(Some(0)), Answer::Support);
        assert_eq!(Answer::from_option(Some(1)), Answer::Later);
        assert_eq!(Answer::from_option(Some(2)), Answer::Never);
        assert_eq!(Answer::from_option(None), Answer::Later);
    }

    #[test]
    fn only_at_a_natural_pause() {
        let calm = Busy::default();
        assert!(natural_pause(Spot::Title, calm, false));
        assert!(natural_pause(Spot::AfterYearEnd, calm, false));
        assert!(!natural_pause(Spot::Course, calm, false), "not in the middle of play");
        assert!(!natural_pause(Spot::Elsewhere, calm, false));
        for busy in [
            Busy { building: true, ..calm },
            Busy { pro_out: true, ..calm },
            Busy { tournament: true, ..calm },
            Busy { other_box: true, ..calm },
        ] {
            assert!(!natural_pause(Spot::AfterYearEnd, busy, false), "{busy:?}");
            assert!(!natural_pause(Spot::Course, busy, true), "{busy:?}");
        }
        // never over another box, on the title menu either
        assert!(!natural_pause(Spot::Title, Busy { other_box: true, ..calm }, true));
        // the test hook on a quiet course
        assert!(natural_pause(Spot::Course, calm, true));
    }

    #[test]
    fn the_record_reads_back_and_old_settings_read_as_nothing_played() {
        let r = Record { played: 4000.0, asks: 1, asked_at: 3700.0, done: false };
        let v = serde_json::to_value(&r).unwrap();
        assert_eq!(serde_json::from_value::<Record>(v).unwrap(), r);
        let none: Record = serde_json::from_value(serde_json::json!({})["support"].clone()).unwrap_or_default();
        assert_eq!(none, Record::default());
    }

    #[test]
    fn the_box_has_a_title_words_and_three_options() {
        let l = box_lines(FIRST_ASK);
        assert_eq!(l[0], tr("STR_SUPPORT_TITLE"));
        let opts: Vec<&String> = l.iter().filter(|s| s.starts_with(' ')).collect();
        assert_eq!(opts, [tr("STR_SUPPORT_YES"), tr("STR_SUPPORT_LATER"), tr("STR_SUPPORT_NEVER")]);
        assert!(l.iter().any(|s| s.starts_with('\t') && s.len() > 1));
    }

    #[test]
    fn the_box_says_how_long_was_played() {
        assert_eq!(played_words(FIRST_ASK), tr("STR_SUPPORT_PLAYED_1"));
        assert_eq!(played_words(FIRST_ASK + SECOND_ASK + 1800.0), "You've played for about 6 hours.");
        assert!(box_lines(FIRST_ASK).concat().contains("£90 a month"));
    }

    #[test]
    fn the_time_played_reads_in_hours_and_minutes() {
        assert_eq!(time_words(0), "0 minutes");
        assert_eq!(time_words(60), "1 minute");
        assert_eq!(time_words(45 * 60 + 59), "45 minutes");
        assert_eq!(time_words(3600), "1 hour");
        assert_eq!(time_words(3600 + 60), "1 hour 1 minute");
        assert_eq!(time_words(6 * 3600 + 12 * 60), "6 hours 12 minutes");
    }
}
