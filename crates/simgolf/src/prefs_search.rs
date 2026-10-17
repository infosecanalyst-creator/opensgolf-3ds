//! Preferences > Search (the port's own; docs/PLAYING.md, "Searching the Preferences"): one box that finds an option on any
//! of the Preferences' pages by its words: the Enhanced graphics options (and its master switch), the OpenSGolf extras
//! (gameplay, sandbox, looks, info, helpers) and the keyboard controls. Typing narrows the list, a click on a found option
//! ticks it (or steps a setting with choices; a key opens Change keys), "Show only changed" keeps the options that differ
//! from the original game, and "Reset to original..." asks, then puts every one of them back (`Prefs::original`).
//! Nothing changes until the box's OK, as in the other Preferences boxes; Esc leaves everything as it was.

use crate::extras_ui::ExtrasLine;
use crate::{controls, enhanced, extras};
use sg_core::extras::Extras;
use sg_core::lang::{tr, trf};

/// The settings the search box shows and changes (put in use by its OK).
#[derive(Clone, Debug, PartialEq)]
pub struct Prefs {
    pub enh: enhanced::Settings,
    pub ex: Extras,
    pub ui: extras::Settings,
    pub keys: controls::Settings,
}

impl Prefs {
    /// The original game, as a new player starts: Enhanced graphics off with its options at their defaults (the bug fixes
    /// on, as they are by default; the "Original game" of Select Difficulty keeps them too), every OpenSGolf extra off, the
    /// original's keys.
    pub fn original() -> Prefs {
        Prefs {
            enh: enhanced::Settings::DEFAULT,
            ex: Extras::default(),
            ui: extras::Settings::ALL_OFF,
            keys: controls::Settings::default(),
        }
    }
}

/// An option the search can find.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Hit {
    /// Enhanced graphics' master switch.
    EnhancedOn,
    /// enhanced::OPTIONS[i].
    Enhanced(usize),
    /// A line of the Extras box.
    Extra(ExtrasLine),
    /// controls::ACTIONS[i]'s key.
    Key(usize),
}

/// The most found options the box lists at once (more: "type more to narrow").
pub const MAX_HITS: usize = 10;

/// The width the box's lines are cut to (pixels of the popup's font), so it keeps one size.
pub const LINE_W: f32 = 600.0;

/// Every option, in the Preferences' order: Enhanced graphics, the extras' pages, the keys.
pub fn all() -> Vec<Hit> {
    let mut v = vec![Hit::EnhancedOn];
    for page in 0..enhanced::PAGES.len() as u8 {
        v.extend(enhanced::OPTIONS.iter().enumerate().filter(|(_, o)| o.page == page).map(|(i, _)| Hit::Enhanced(i)));
    }
    for page in 0..crate::extras_ui::PAGES.len() as u8 {
        v.extend(crate::extras_ui::page_lines(page).into_iter().map(Hit::Extra));
    }
    v.extend((0..controls::ACTIONS.len()).map(Hit::Key));
    v
}

/// Where an option is: its category (the extras box's page, "Graphics" and the Enhanced graphics page, or "Keys"), with how
/// much it changes the game for a gameplay or sandbox extra ("Gameplay · Noticeable").
pub fn place(h: Hit) -> String {
    let two = |a: &str, b: &str| trf("STR_SEARCH_PLACE_TWO", &[("A", &a), ("B", &b)]);
    match h {
        Hit::EnhancedOn => tr("STR_SEARCH_PLACE_ENHANCED").to_string(),
        Hit::Enhanced(i) => two(tr("STR_SEARCH_PLACE_ENHANCED"), tr(enhanced::PAGES[enhanced::OPTIONS[i].page as usize])),
        Hit::Extra(l) => {
            let page = tr(crate::extras_ui::PAGES[crate::extras_ui::page_of(l) as usize]);
            match crate::extras_ui::tag(l) {
                Some(t) => two(page, tr(t.key())),
                None => page.to_string(),
            }
        }
        Hit::Key(_) => tr("STR_SEARCH_PLACE_KEYS").to_string(),
    }
}

/// An option's text (with its value for a setting with choices and a key) and whether it is on.
pub fn text(h: Hit, p: &Prefs) -> (String, bool) {
    match h {
        Hit::EnhancedOn => (tr("STR_ENHANCED_MASTER").trim().to_string(), p.enh.enabled),
        Hit::Enhanced(i) => {
            let o = &enhanced::OPTIONS[i];
            (enhanced::option_line(o, &p.enh).trim().to_string(), (o.get)(&p.enh) != 0)
        }
        Hit::Extra(l) => {
            let (t, on) = crate::extras_ui::line_text(l, &p.ex, &p.ui);
            (t.trim().to_string(), on)
        }
        Hit::Key(i) => {
            let key = p.keys.binding(i).map(controls::Binding::label).unwrap_or_else(|| "-".into());
            (trf("STR_SEARCH_KEY", &[("ACTION", &tr(controls::ACTIONS[i].text)), ("KEY", &key)]), false)
        }
    }
}

/// Whether an option differs from the original game (`Prefs::original`).
pub fn changed(h: Hit, p: &Prefs) -> bool {
    let o = Prefs::original();
    match h {
        Hit::EnhancedOn => p.enh.enabled != o.enh.enabled,
        Hit::Enhanced(i) => (enhanced::OPTIONS[i].get)(&p.enh) != (enhanced::OPTIONS[i].get)(&o.enh),
        Hit::Extra(l) => crate::extras_ui::line_text(l, &p.ex, &p.ui).1 != crate::extras_ui::line_text(l, &o.ex, &o.ui).1,
        Hit::Key(i) => p.keys.keys_of(i).collect::<Vec<_>>() != o.keys.keys_of(i).collect::<Vec<_>>(),
    }
}

/// Whether an option is drawn without a tick box: a setting with choices (a click steps it) or a key (a click opens Change
/// keys).
pub fn plain(h: Hit) -> bool {
    match h {
        Hit::EnhancedOn => false,
        Hit::Enhanced(i) => !enhanced::OPTIONS[i].choices.is_empty(),
        Hit::Extra(ExtrasLine::Game(i)) => Extras::choices(i) > 2,
        Hit::Extra(ExtrasLine::Ui(_)) => false,
        Hit::Key(_) => true,
    }
}

/// Whether an option matches the typed words: each of them starts a word of its page's name or its text (any case; "rain"
/// finds "Rain on some days" and "rainbows", not "Terrain").
pub fn matches(h: Hit, p: &Prefs, query: &str) -> bool {
    let hay = format!("{} {}", place(h), text(h, p).0).to_lowercase();
    let starts_word = |w: &str| hay.match_indices(w).any(|(i, _)| !hay[..i].chars().next_back().is_some_and(|c| c.is_alphanumeric()));
    query.to_lowercase().split_whitespace().all(starts_word)
}

/// The options found: matching the words, and differing from the original when `changed_only`.
pub fn search(query: &str, changed_only: bool, p: &Prefs) -> Vec<Hit> {
    all().into_iter().filter(|&h| (!changed_only || changed(h, p)) && matches(h, p, query)).collect()
}

/// A click on a found option (a key is not changed here: the caller opens Change keys).
pub fn toggle(h: Hit, p: &mut Prefs) {
    match h {
        Hit::EnhancedOn => p.enh.enabled = !p.enh.enabled,
        Hit::Enhanced(i) => {
            let o = &enhanced::OPTIONS[i];
            let n = o.choices.len().max(2) as u8;
            let v = ((o.get)(&p.enh) + 1) % n;
            (o.set)(&mut p.enh, v);
        }
        Hit::Extra(l) => crate::extras_ui::toggle_line(l, &mut p.ex, &mut p.ui),
        Hit::Key(_) => {}
    }
}

impl crate::app::App {
    /// A character typed while a popup is open: the search box takes it into its words; on Preferences a letter or digit
    /// opens the search box on it (the Preferences' "small search box": type to find).
    pub fn search_char(&mut self, c: char) {
        use crate::popup_ui::PopupKind;
        let Some(p) = self.popup.as_mut().filter(|_| self.screen == crate::app::Screen::Popup) else { return };
        match p.kind {
            PopupKind::Search if !c.is_control() && p.query.chars().count() < MAX_QUERY => {
                p.query.push(c);
                p.refresh();
            }
            PopupKind::Prefs if c.is_alphanumeric() => {
                self.pref_query = c.to_string();
                self.open_popup(PopupKind::Search);
            }
            _ => {}
        }
    }
}

/// The most characters the search box takes.
pub const MAX_QUERY: usize = 24;

/// A line cut to `LINE_W` with "..." when it is longer.
pub fn fit(s: &str) -> String {
    let f = crate::popup_ui::CHOICE_FONT;
    if f.width(s) <= LINE_W {
        return s.to_string();
    }
    let mut t: String = s.to_string();
    while !t.is_empty() && f.width(&format!("{t}...")) > LINE_W {
        t.pop();
    }
    format!("{}...", t.trim_end())
}

/// The search box's lines: the title, the typed words, the "Show only changed" tick, up to `MAX_HITS` found options
/// (padded to that many), a line that says how many more there are or that nothing was found, the reset line, and an empty
/// line that keeps the OK tick clear.
pub fn popup_lines(query: &str, caret: bool, hits: &[Hit], p: &Prefs) -> Vec<String> {
    let mut v = vec![tr("STR_SEARCH_TITLE").to_string()];
    // an empty search shows a hint after the caret, so it is plain where the typing goes
    let hint = if query.is_empty() { tr("STR_SEARCH_HINT").to_string() } else { String::new() };
    v.push(trf("STR_SEARCH_FIND", &[("QUERY", &format!("{query}{}{hint}", if caret { "|" } else { " " }))]));
    v.push(tr("STR_SEARCH_CHANGED").to_string());
    for &h in hits.iter().take(MAX_HITS) {
        v.push(fit(&format!(" {}: {}", place(h), text(h, p).0)));
    }
    for _ in hits.len().min(MAX_HITS)..MAX_HITS {
        v.push(String::new());
    }
    v.push(if hits.is_empty() {
        tr("STR_SEARCH_NONE").to_string()
    } else if hits.len() > MAX_HITS {
        trf("STR_SEARCH_MORE", &[("N", &(hits.len() - MAX_HITS))])
    } else {
        String::new()
    });
    v.push(tr("STR_SEARCH_RESET").to_string());
    v.push(String::new());
    v
}

/// The box's ticks: bit 0 "Show only changed", bit 1 + i found option i.
pub fn popup_checks(changed_only: bool, hits: &[Hit], p: &Prefs) -> u32 {
    hits.iter().take(MAX_HITS).enumerate().fold(changed_only as u32, |m, (i, &h)| m | (text(h, p).1 as u32) << (i + 1))
}

/// The box's lines without a tick box: plain found options and the reset line.
pub fn popup_plain(hits: &[Hit]) -> u32 {
    let n = hits.len().min(MAX_HITS);
    hits.iter().take(MAX_HITS).enumerate().filter(|(_, &h)| plain(h)).fold(1 << (n + 1), |m, (i, _)| m | 1 << (i + 1))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn finds_options_on_every_page_by_their_words() {
        let p = Prefs::original();
        let rain = search("rain", false, &p);
        assert!(rain.iter().any(|&h| matches!(h, Hit::Enhanced(i) if enhanced::OPTIONS[i].key == "fx_rain")));
        let sandbox = search("sandbox:", false, &p);
        assert_eq!(sandbox, (7..=9).map(|i| Hit::Extra(ExtrasLine::Game(i))).collect::<Vec<_>>(), "the three sandbox switches");
        // words in any order and case, each must be there
        assert_eq!(search("CALENDAR pause", false, &p), vec![Hit::Extra(ExtrasLine::Game(7))]);
        assert!(search("zoom", false, &p).contains(&Hit::Key(controls::action("zoom_in"))));
        assert!(search("no such option at all", false, &p).is_empty());
        // words are matched from their start
        assert!(!search("rain", false, &p)
            .iter()
            .any(|&h| matches!(h, Hit::Enhanced(i) if enhanced::OPTIONS[i].key == "fx_terrain_light")));
        assert!(search("rainb", false, &p).iter().any(|&h| matches!(h, Hit::Enhanced(i) if enhanced::OPTIONS[i].key == "fx_after_rain")));
        assert_eq!(search("", false, &p).len(), all().len());
    }

    #[test]
    fn show_only_changed_and_reset() {
        let mut p = Prefs::original();
        assert!(search("", true, &p).is_empty(), "the original game has nothing changed");
        p.ex.freeze_moods = true;
        p.ex.season_speed = 2;
        p.enh.fx_rain = true;
        p.ui.people = true;
        p.keys.bind(controls::action("zoom_in"), Some(controls::Binding { key: miniquad::KeyCode::Q, shift: false, ctrl: false }));
        let c = search("", true, &p);
        assert_eq!(c.len(), 5, "{c:?}");
        assert!(c.contains(&Hit::Extra(ExtrasLine::Game(sg_core::extras::SEASON_SPEED))));
        assert!(c.contains(&Hit::Key(controls::action("zoom_in"))));
        // the Enhanced master switch off is the original, its options at their defaults are not changes
        assert!(!c.contains(&Hit::EnhancedOn));
        assert_eq!(Prefs::original(), Prefs::original());
        assert!(search("", true, &Prefs::original()).is_empty());
    }

    #[test]
    fn a_click_ticks_or_steps() {
        let mut p = Prefs::original();
        let speed = Hit::Extra(ExtrasLine::Game(sg_core::extras::SEASON_SPEED));
        for want in [1, 2, 0] {
            toggle(speed, &mut p);
            assert_eq!(p.ex.season_speed, want);
        }
        toggle(Hit::Extra(ExtrasLine::Game(9)), &mut p);
        assert!(p.ex.unlock_all);
        toggle(Hit::Extra(ExtrasLine::Ui(0)), &mut p);
        assert!(p.ui.people && !p.ui.history);
        toggle(Hit::EnhancedOn, &mut p);
        assert!(p.enh.enabled);
        assert!(plain(speed) && plain(Hit::Key(0)) && !plain(Hit::EnhancedOn));
        assert!(plain(Hit::Extra(ExtrasLine::Game(sg_core::extras::PATHS))) && !plain(Hit::Extra(ExtrasLine::Game(9))));
    }

    #[test]
    fn found_options_say_their_category_and_tag() {
        let tagged = |a: &'static str, b: &'static str| trf("STR_SEARCH_PLACE_TWO", &[("A", &tr(a)), ("B", &tr(b))]);
        let weather = Hit::Extra(ExtrasLine::Game(sg_core::extras::WEATHER));
        assert_eq!(place(weather), tagged("STR_EXTRAS_PAGE_GAMEPLAY", "STR_EXTRAS_TAG_NOTICEABLE"));
        assert_eq!(place(Hit::Extra(ExtrasLine::Game(9))), tagged("STR_EXTRAS_PAGE_SANDBOX", "STR_EXTRAS_TAG_BIG"));
        assert_eq!(place(Hit::Extra(ExtrasLine::Game(sg_core::extras::PATHS))), tr("STR_EXTRAS_PAGE_LOOKS"));
        assert_eq!(place(Hit::Extra(ExtrasLine::Ui(0))), tr("STR_EXTRAS_PAGE_INFO"));
        // the tag's words find the options so tagged
        let p = Prefs::original();
        assert_eq!(search("big change", false, &p).len(), 4, "hardcore and three sandbox switches");
    }

    #[test]
    fn the_box_keeps_its_size() {
        let p = Prefs::original();
        let few = search("sandbox:", false, &p);
        let many = search("", false, &p);
        assert_eq!(popup_lines("sandbox", true, &few, &p).len(), popup_lines("", false, &many, &p).len());
        assert_eq!(popup_lines("x", false, &[], &p).len(), MAX_HITS + 6);
        // the reset line is the option after the found ones, drawn without a box
        assert_eq!(popup_plain(&few) >> (few.len() + 1) & 1, 1);
    }
}
