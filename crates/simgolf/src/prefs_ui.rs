//! Preferences and its OpenSGolf boxes (the port's own layout; docs/PLAYING.md, "Preferences"). Preferences keeps the
//! original's four ticks first, exactly as the original has them, then a divider and one line per group of the port's own
//! options: Game options... (the OpenSGolf extras box), Graphics... (HD, Enhanced, pixel-perfect and the Enhanced graphics
//! settings), Controls and keys... (keys_ui.rs), Text size (stepped at once), Game files... (your SimGolf files or the
//! free graphics, Content packs, Language) and More... (the bug fixes, the update checks, Search, Report a problem, the
//! launcher, What's new, Support OpenSGolf, the play-time count). Each port line shows a one-line description at the box's foot when pointed at (on a touch screen when
//! tapped: a line that opens something opens on the second tap). The settings are the same ones, kept in the same places of settings.json, as before.

use crate::popup_ui::PopupKind;
use sg_core::lang::{tr, trf};

/// A line of Preferences or of one of its boxes.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Line {
    // the original's ticks (Preferences)
    Thoughts,
    Advisor,
    Animals,
    Sound,
    // the port's lines of Preferences
    GameOptions,
    Graphics,
    Controls,
    TextSize,
    GameFiles,
    More,
    // Preferences > Graphics
    Hd,
    /// Make HD graphics from my SimGolf files... (hd_make.rs)
    MakeHd,
    /// Remove HD graphics... (hd_make.rs): back to Classic and the pack deleted
    RemoveHd,
    Enhanced,
    PixelPerfect,
    EnhancedSettings,
    // Preferences > Game files
    Files,
    Content,
    Language,
    // Preferences > More
    FixBugs,
    UpdateCheck,
    UpdateNow,
    Search,
    /// Report a problem (the port's own helper, bugreport.rs)
    Report,
    /// Support OpenSGolf (the port's own, support.rs): the project's Buy Me a Coffee page
    Support,
    /// What's new... (the port's own, welcome.rs): the changelog in the browser
    WhatsNew,
    /// Launcher... (the port's own, launcher.rs): the screen shown before the title menu
    Launcher,
    /// Count my play time (anonymous) (the port's own, playcount.rs; not in store builds)
    PlayCount,
}

/// What the boxes' lines depend on besides the settings.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub struct Ctx {
    /// The "Use my SimGolf files..." / "Use the free graphics" line's label (gamefiles.rs; none in a browser) and whether
    /// it is the free graphics'.
    pub files: Option<(&'static str, bool)>,
    /// A translation is installed (the Language line).
    pub languages: bool,
    /// This build can check for updates itself (update.rs AVAILABLE: Check for updates now).
    pub update_now: bool,
    /// This build has the play-time count (playcount.rs AVAILABLE).
    pub play_count: bool,
    /// Seconds played on the course (support.rs's counter), shown under Support OpenSGolf.
    pub played: u64,
    /// What the Make HD graphics line offers (hd_make.rs; none on the free graphics).
    pub make_hd: Option<crate::hd_make::Offer>,
    /// There is an HD pack to remove (and no run making one).
    pub remove_hd: bool,
}

/// The rows at the foot of each box kept for the description (as the Extras box's, extras_ui::FOOT_LINES).
pub const FOOT_LINES: usize = 2;

/// The first character of the divider line between the original's ticks and the port's lines (drawn as a rule with its
/// words in the middle; no option).
pub const DIVIDER: char = '\u{b}';

/// The note under the bug fixes' tick (a line starting with a tab: popup_ui's note).
const NOTE: char = '\t';

/// The boxes laid out here.
pub fn is_prefs_box(kind: PopupKind) -> bool {
    matches!(kind, PopupKind::Prefs | PopupKind::PrefsGraphics | PopupKind::PrefsFiles | PopupKind::PrefsMore)
}

/// A box's options, in order.
pub fn items(kind: PopupKind, c: &Ctx) -> Vec<Line> {
    use Line::*;
    match kind {
        PopupKind::Prefs => vec![Thoughts, Advisor, Animals, Sound, GameOptions, Graphics, Controls, TextSize, GameFiles, More],
        PopupKind::PrefsGraphics => [Hd]
            .into_iter()
            .chain(c.make_hd.map(|_| MakeHd))
            .chain(c.remove_hd.then_some(RemoveHd))
            .chain([Enhanced, PixelPerfect, EnhancedSettings])
            .collect(),
        PopupKind::PrefsFiles => c.files.map(|_| Files).into_iter().chain([Content]).chain(c.languages.then_some(Language)).collect(),
        PopupKind::PrefsMore => [FixBugs, UpdateCheck]
            .into_iter()
            .chain(c.update_now.then_some(UpdateNow))
            .chain([Search, Report, Launcher, WhatsNew, Support])
            .chain(c.play_count.then_some(PlayCount))
            .collect(),
        _ => Vec::new(),
    }
}

/// The box's title.
fn title_key(kind: PopupKind) -> &'static str {
    match kind {
        PopupKind::PrefsGraphics => "STR_PREFS_GRAPHICS_TITLE",
        PopupKind::PrefsFiles => "STR_PREFS_FILES_TITLE",
        PopupKind::PrefsMore => "STR_PREFS_MORE_TITLE",
        _ => "STR_MENU_PREFS_TITLE",
    }
}

/// An option's text (with its leading space), `text` the text size's percent.
pub fn label(l: Line, c: &Ctx, text: u32) -> String {
    use Line::*;
    let key = match l {
        Thoughts => "STR_MENU_PREFS_THOUGHTS",
        Advisor => "STR_MENU_PREFS_ADVISOR",
        Animals => "STR_MENU_PREFS_ANIMALS",
        Sound => "STR_MENU_PREFS_SOUND",
        GameOptions => "STR_PREFS_GAME_OPTIONS",
        Graphics => "STR_PREFS_GRAPHICS",
        Controls => "STR_PREFS_CONTROLS",
        TextSize => {
            return trf("STR_MENU_PREFS_TEXT_SIZE", &[("SIZE", &tr(crate::textsize::name_key(text)))]);
        }
        GameFiles => "STR_PREFS_GAME_FILES",
        More => "STR_PREFS_MORE",
        Hd => "STR_MENU_PREFS_HD",
        MakeHd => {
            use crate::hd_make::Offer;
            return match c.make_hd {
                Some(Offer::Running(p)) => trf("STR_PREFS_MAKE_HD_RUNNING", &[("PCT", &p)]),
                Some(Offer::Again) => tr("STR_PREFS_MAKE_HD_AGAIN").to_string(),
                Some(Offer::Resume) => tr("STR_PREFS_MAKE_HD_RESUME").to_string(),
                _ => tr("STR_PREFS_MAKE_HD").to_string(),
            };
        }
        RemoveHd => "STR_PREFS_REMOVE_HD",
        Enhanced => "STR_MENU_PREFS_ENHANCED_ON",
        PixelPerfect => "STR_ENHANCED_PIXEL_PERFECT",
        EnhancedSettings => "STR_MENU_PREFS_ENHANCED",
        Files => return format!(" {}", c.files.map_or("", |f| f.0)),
        Content => "STR_MENU_PREFS_CONTENT",
        Language => {
            return trf("STR_MENU_PREFS_LANGUAGE", &[("LANGUAGE", &crate::language::current_name())]);
        }
        FixBugs => "STR_MENU_PREFS_FIX_BUGS",
        UpdateCheck => "STR_MENU_PREFS_UPDATE_CHECK",
        UpdateNow => "STR_PREFS_UPDATE_NOW",
        Search => "STR_MENU_PREFS_SEARCH",
        Report => "STR_MENU_PREFS_REPORT",
        Support => "STR_MENU_PREFS_SUPPORT",
        WhatsNew => "STR_MENU_PREFS_WHATSNEW",
        Launcher => "STR_MENU_PREFS_LAUNCHER",
        PlayCount => "STR_MENU_PREFS_PLAYCOUNT",
    };
    tr(key).to_string()
}

/// A box's lines: the title, its options (Preferences: the divider after the original's four; More: the note under the
/// bug fixes), and the empty rows of the foot.
pub fn lines(kind: PopupKind, c: &Ctx, text: u32) -> Vec<String> {
    let mut v = vec![tr(title_key(kind)).to_string()];
    for l in items(kind, c) {
        if l == Line::GameOptions {
            v.push(format!("{DIVIDER}{}", tr("STR_PREFS_DIVIDER")));
        }
        v.push(label(l, c, text));
        if l == Line::FixBugs {
            v.push(format!("{NOTE}{}", tr("STR_MENU_PREFS_FIX_BUGS_NOTE")));
        }
        if l == Line::Support {
            v.push(format!("{NOTE}{}", trf("STR_MENU_PREFS_PLAYED_NOTE", &[("TIME", &crate::support::time_words(c.played))])));
        }
    }
    v.extend(std::iter::repeat_n(String::new(), FOOT_LINES));
    v
}

/// A tick (kept in the box until its OK) rather than a line that does something at once.
pub fn is_tick(l: Line) -> bool {
    use Line::*;
    matches!(l, Thoughts | Advisor | Animals | Sound | Hd | Enhanced | PixelPerfect | FixBugs | UpdateCheck | PlayCount)
}

/// A line that opens a box or screen, or does something there is no stepping back from (the files line, the update check):
/// on a touch screen the first tap shows its description and the second does it.
pub fn opens(l: Line) -> bool {
    !is_tick(l) && !matches!(l, Line::TextSize | Line::Language)
}

/// The option bits drawn without a tick box (the lines that do something).
pub fn plain_bits(items: &[Line]) -> u32 {
    items.iter().enumerate().filter(|(_, &l)| !is_tick(l)).fold(0, |m, (k, _)| m | 1 << k)
}

/// The description at the foot (none for the original's own four).
pub fn desc_key(l: Line, c: &Ctx) -> Option<&'static str> {
    use Line::*;
    Some(match l {
        Thoughts | Advisor | Animals | Sound => return None,
        GameOptions => "STR_PREFS_DESC_GAME_OPTIONS",
        Graphics => "STR_PREFS_DESC_GRAPHICS",
        Controls => "STR_PREFS_DESC_CONTROLS",
        TextSize => "STR_PREFS_DESC_TEXT_SIZE",
        GameFiles => "STR_PREFS_DESC_GAME_FILES",
        More => "STR_PREFS_DESC_MORE",
        Hd => "STR_PREFS_DESC_HD",
        MakeHd => "STR_PREFS_DESC_MAKE_HD",
        RemoveHd => "STR_PREFS_DESC_REMOVE_HD",
        Enhanced => "STR_PREFS_DESC_ENHANCED",
        PixelPerfect => "STR_PREFS_DESC_PIXEL_PERFECT",
        EnhancedSettings => "STR_PREFS_DESC_ENHANCED_SETTINGS",
        Files if c.files.is_some_and(|f| f.1) => "STR_PREFS_DESC_FILES_FREE",
        Files => "STR_PREFS_DESC_FILES_MINE",
        Content => "STR_PREFS_DESC_CONTENT",
        Language => "STR_PREFS_DESC_LANGUAGE",
        FixBugs => "STR_PREFS_DESC_FIX_BUGS",
        UpdateCheck => "STR_EXTRAS_DESC_UPDATE_CHECK",
        UpdateNow => "STR_PREFS_DESC_UPDATE_NOW",
        Search => "STR_PREFS_DESC_SEARCH",
        Report => "STR_PREFS_DESC_REPORT",
        Support => "STR_PREFS_DESC_SUPPORT",
        WhatsNew => "STR_PREFS_DESC_WHATSNEW",
        Launcher => "STR_PREFS_DESC_LAUNCHER",
        PlayCount => "STR_PREFS_DESC_PLAYCOUNT",
    })
}

/// The foot's text for the option `k` lit (or last tapped): its description (true), or the box's hint (false). `armed`:
/// the line waits for a second tap.
pub fn foot_text(kind: PopupKind, c: &Ctx, k: Option<usize>, armed: bool) -> (String, bool) {
    let l = k.and_then(|k| items(kind, c).get(k).copied());
    match l.and_then(|l| desc_key(l, c)) {
        Some(d) if armed => (trf("STR_PREFS_TAP_AGAIN", &[("DESC", &tr(d))]), true),
        Some(d) => (tr(d).to_string(), true),
        None => {
            let hint = match kind {
                PopupKind::Prefs => "STR_PREFS_HINT",
                PopupKind::PrefsGraphics => "STR_PREFS_HINT_GRAPHICS",
                // every line of the other boxes is the port's
                _ => "STR_EXTRAS_HINT_PLAIN",
            };
            (tr(hint).to_string(), false)
        }
    }
}

/// The box an option of the port's own is in, as the search box names it (its Preferences line, "Preferences" for the
/// lines on Preferences itself).
pub fn place_key(l: Line) -> &'static str {
    use Line::*;
    match l {
        Hd | MakeHd | RemoveHd | Enhanced | PixelPerfect | EnhancedSettings => "STR_SEARCH_PLACE_GRAPHICS",
        Files | Content | Language => "STR_SEARCH_PLACE_FILES",
        FixBugs | UpdateCheck | UpdateNow | Search | Report | Launcher | WhatsNew | Support | PlayCount => "STR_SEARCH_PLACE_MORE",
        _ => "STR_SEARCH_PLACE_PREFS",
    }
}

impl crate::app::App {
    /// What Preferences' boxes depend on here.
    pub fn prefs_ctx(&self) -> Ctx {
        Ctx {
            files: self.files_option().map(|(a, l)| (l, a == crate::gamefiles::Action::Free)),
            languages: crate::language::available().len() > 1,
            update_now: crate::update::AVAILABLE,
            play_count: crate::playcount::AVAILABLE,
            played: self.support.record.played.max(0.0) as u64,
            make_hd: self.hd_make_offer(),
            remove_hd: self.hd_remove_offer(),
        }
    }

    /// A tick's state as in use.
    pub fn prefs_tick(&self, l: Line) -> bool {
        use Line::*;
        match l {
            Thoughts => self.show_thoughts,
            Advisor => self.show_advisor,
            Animals => self.club.wildlife.enabled,
            Sound => !self.mute,
            Hd => self.hd.enabled,
            Enhanced => crate::enhanced::settings().enabled,
            PixelPerfect => crate::enhanced::settings().pixel_perfect,
            FixBugs => crate::enhanced::fix_bugs(),
            UpdateCheck => crate::extras::update_check(),
            PlayCount => self.playcount.on(),
            _ => false,
        }
    }

    /// Opens a box from Preferences or one of its boxes (`from`), which it goes back to when done.
    pub fn open_prefs_child(&mut self, from: PopupKind, kind: PopupKind) {
        self.prefs_stack.push(from);
        self.open_popup(kind);
    }

    /// A box or screen opened from Preferences is done: back to the box it was opened from, if any.
    pub fn return_to_prefs(&mut self) {
        if let Some(kind) = self.prefs_stack.pop() {
            self.open_popup(kind);
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use Line::*;

    const ALL: Ctx = Ctx {
        files: Some(("Use my SimGolf files...", false)),
        languages: true,
        update_now: true,
        play_count: true,
        played: 0,
        make_hd: Some(crate::hd_make::Offer::Make),
        remove_hd: true,
    };

    #[test]
    fn preferences_keeps_the_originals_four_first_then_the_divider() {
        let it = items(PopupKind::Prefs, &ALL);
        assert_eq!(it[..4], [Thoughts, Advisor, Animals, Sound]);
        assert_eq!(it[4..], [GameOptions, Graphics, Controls, TextSize, GameFiles, More]);
        let l = lines(PopupKind::Prefs, &ALL, 100);
        assert_eq!(l[0], tr("STR_MENU_PREFS_TITLE"));
        assert_eq!(
            l[1..5],
            ["STR_MENU_PREFS_THOUGHTS", "STR_MENU_PREFS_ADVISOR", "STR_MENU_PREFS_ANIMALS", "STR_MENU_PREFS_SOUND"].map(tr)
        );
        assert!(l[5].starts_with(DIVIDER) && !l[5].starts_with(' '), "the divider is no option");
        // the options are the lines starting with a space, in the items' order; the foot is empty rows
        let opts: Vec<&String> = l.iter().filter(|s| s.starts_with(' ')).collect();
        assert_eq!(opts.len(), it.len());
        assert!(l[l.len() - FOOT_LINES..].iter().all(|s| s.is_empty()));
        // the original's four are the only ticks, each with no description of the port's
        assert_eq!(plain_bits(&it), 0b11_1111_0000);
        assert!(it[..4].iter().all(|&l| desc_key(l, &ALL).is_none()));
        assert!(it[4..].iter().all(|&l| desc_key(l, &ALL).is_some()));
    }

    #[test]
    fn every_box_has_every_line_and_a_description() {
        let none = Ctx::default();
        assert_eq!(items(PopupKind::PrefsGraphics, &none), [Hd, Enhanced, PixelPerfect, EnhancedSettings]);
        // Make HD graphics only with the original files in use
        assert_eq!(items(PopupKind::PrefsGraphics, &ALL), [Hd, MakeHd, RemoveHd, Enhanced, PixelPerfect, EnhancedSettings]);
        assert!(opens(RemoveHd));
        let running = Ctx { make_hd: Some(crate::hd_make::Offer::Running(42)), ..ALL };
        assert!(label(MakeHd, &running, 100).contains("42"));
        assert!(opens(MakeHd));
        // no files line in a browser, the language only with a translation, the update check now only where it can ask
        assert_eq!(items(PopupKind::PrefsFiles, &none), [Content]);
        assert_eq!(items(PopupKind::PrefsFiles, &ALL), [Files, Content, Language]);
        assert_eq!(items(PopupKind::PrefsMore, &none), [FixBugs, UpdateCheck, Search, Report, Launcher, WhatsNew, Support]);
        assert_eq!(
            items(PopupKind::PrefsMore, &ALL),
            [FixBugs, UpdateCheck, UpdateNow, Search, Report, Launcher, WhatsNew, Support, PlayCount]
        );
        for kind in [PopupKind::PrefsGraphics, PopupKind::PrefsFiles, PopupKind::PrefsMore] {
            for l in items(kind, &ALL) {
                assert!(desc_key(l, &ALL).is_some(), "{l:?}");
                assert_ne!(place_key(l), "STR_SEARCH_PLACE_PREFS", "{l:?} is in a box");
            }
        }
        // the note under the bug fixes is no option
        let l = lines(PopupKind::PrefsMore, &ALL, 100);
        assert!(l[2].starts_with(NOTE));
        assert_eq!(l.iter().filter(|s| s.starts_with(' ')).count(), 9);
        // and the time played under Support OpenSGolf is a note too
        let support = l.iter().position(|s| s == &label(Support, &ALL, 100)).unwrap();
        assert_eq!(l[support + 1], format!("{NOTE}Your time played: 0 minutes."));
        // the files line's description follows what it does
        let free = Ctx { files: Some(("Use the free graphics", true)), ..ALL };
        assert_ne!(desc_key(Files, &free), desc_key(Files, &ALL));
    }

    #[test]
    fn the_foot_says_what_the_lit_line_does() {
        let (t, desc) = foot_text(PopupKind::Prefs, &ALL, Some(5), false);
        assert!(desc && t == tr("STR_PREFS_DESC_GRAPHICS"));
        // the original's lines and none lit: the hint
        assert!(!foot_text(PopupKind::Prefs, &ALL, Some(0), false).1);
        assert!(!foot_text(PopupKind::Prefs, &ALL, None, false).1);
        assert_eq!(foot_text(PopupKind::PrefsGraphics, &ALL, None, false).0, tr("STR_PREFS_HINT_GRAPHICS"));
        // waiting for the second tap
        assert!(foot_text(PopupKind::Prefs, &ALL, Some(5), true).0.starts_with(tr("STR_PREFS_DESC_GRAPHICS")));
        // ticks and steppers act on the first tap, the rest on the second
        assert!(!opens(Hd) && !opens(TextSize) && !opens(Language) && opens(Graphics) && opens(Files) && opens(UpdateNow));
    }
}
