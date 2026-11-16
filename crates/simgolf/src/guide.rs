//! The reference half of the web site's feature guide (web/guide.html, published with the browser version on GitHub
//! Pages): every option of Preferences, Game options and the Enhanced graphics settings, with where it is, what it does,
//! whether it is on for a new player and which presets switch it on, and every key of Change keys. It is made here from
//! the same lists, texts and defaults the game draws its boxes from, so it cannot drift from the game: the test below
//! fails when web/guide.html's generated part differs from what this makes.
//!
//! After adding or changing an option, a key or an option's English text, run
//!
//! ```text
//! OPENSGOLF_WRITE_GUIDE=1 cargo test -p opensgolf guide
//! ```
//!
//! to write the new reference into web/guide.html, and give a new Enhanced graphics option a description (enhanced.rs
//! `desc_key`, lang/en.txt). The hand-written half of the page (the feature sections above the reference)
//! is edited in web/guide.html itself.

use crate::extras_ui::{self, ExtrasLine, Preset, Tag};
use crate::prefs_ui::{self, Line};
use crate::{controls, enhanced};
use sg_core::extras::Extras;
use sg_core::lang::tr;

/// The lines around the generated part of web/guide.html.
pub const BEGIN: &str =
    "<!-- reference:begin (made by crates/simgolf/src/guide.rs: OPENSGOLF_WRITE_GUIDE=1 cargo test -p opensgolf guide) -->";
pub const END: &str = "<!-- reference:end -->";

/// When an option of Preferences' own boxes is there (none: always).
fn pref_note(l: Line) -> Option<&'static str> {
    Some(match l {
        Line::MakeHd => "With your own SimGolf files in use; not in browsers yet.",
        Line::RemoveHd => "Once you have made an HD pack.",
        Line::Files => "Not in the browser: there you choose the files on the start page.",
        Line::Language => "When a translation is installed.",
        Line::UpdateNow => "Not in the browser, which is always the newest version.",
        Line::PlayCount => "Not in store builds.",
        _ => return None,
    })
}

/// Escapes text for HTML.
fn esc(s: &str) -> String {
    s.trim().replace('&', "&amp;").replace('<', "&lt;").replace('>', "&gt;").replace('"', "&quot;")
}

/// An anchor id from a text key ("STR_EXTRAS_WEATHER" -> "opt-extras-weather").
fn anchor(key: &str) -> String {
    format!("opt-{}", key.split_once('_').map_or(key, |(_, rest)| rest).to_lowercase().replace('_', "-"))
}

/// The status pill: on, off or a value.
fn pill(on: bool, text: &str) -> String {
    format!("<span class=\"pill {}\">{}</span>", if on { "on" } else { "off" }, esc(text))
}

fn on_off(on: bool) -> String {
    pill(on, if on { "On" } else { "Off" })
}

/// The interface extras a new player has on: those of a settings.json written without them (extras.rs `from_json`).
fn new_player_ui() -> crate::extras::Settings {
    crate::extras::new_player()
}

/// The rows of the Preferences boxes (the port's lines that are a setting or do something; not the lines that only open
/// another box).
fn prefs_rows(out: &mut String) {
    use crate::popup_ui::PopupKind;
    let ctx = prefs_ui::Ctx {
        files: Some(("Use my SimGolf files... / Use the free graphics", false)),
        languages: true,
        update_now: true,
        play_count: true,
        played: 0,
        make_hd: Some(crate::hd_make::Offer::Make),
        remove_hd: true,
    };
    let e = enhanced::Settings::DEFAULT;
    let boxes = [
        (PopupKind::Prefs, "Preferences"),
        (PopupKind::PrefsGraphics, "Preferences › Graphics"),
        (PopupKind::PrefsFiles, "Preferences › Game files"),
        (PopupKind::PrefsMore, "Preferences › More"),
    ];
    out.push_str("<h3 id=\"ref-preferences\">Preferences and its boxes</h3>\n");
    out.push_str("<p class=\"lede\">In a game: <b>System Functions › Preferences</b> (the dock's System Functions button). The original's four ticks come first; these are OpenSGolf's own lines under them.</p>\n");
    out.push_str("<div class=\"tw\"><table>\n<thead><tr><th>Option</th><th>What it does</th><th>New player</th></tr></thead>\n<tbody>\n");
    for (kind, place) in boxes {
        for l in prefs_ui::items(kind, &ctx) {
            // the original's ticks, and the lines that only open the next box (their box has its own rows)
            if matches!(
                l,
                Line::Thoughts
                    | Line::Advisor
                    | Line::Animals
                    | Line::Sound
                    | Line::GameOptions
                    | Line::Graphics
                    | Line::Controls
                    | Line::GameFiles
                    | Line::More
            ) {
                continue;
            }
            let label = prefs_ui::label(l, &ctx, 100);
            // a stepped line's name without its value ("Text size: normal" -> "Text size"), the value in its own column
            let label = match l {
                Line::TextSize | Line::Language => label.split(':').next().unwrap_or(&label).to_string(),
                _ => label,
            };
            let desc = prefs_ui::desc_key(l, &ctx).map(tr).unwrap_or("");
            let default = match l {
                Line::Hd => on_off(false),
                Line::Enhanced => on_off(e.enabled),
                Line::PixelPerfect => on_off(e.pixel_perfect),
                Line::FixBugs => on_off(e.fix_original_bugs),
                Line::UpdateCheck => on_off(new_player_ui().update_check),
                Line::PlayCount => on_off(true),
                Line::TextSize => pill(false, tr(crate::textsize::name_key(100))),
                Line::Language => pill(false, "English"),
                _ => "<span class=\"pill act\">Opens</span>".to_string(),
            };
            let note = pref_note(l).map_or(String::new(), |n| format!(" <span class=\"when\">{}</span>", esc(n)));
            out.push_str(&format!(
                "<tr><td><b>{}</b><span class=\"where\">{}</span></td><td>{}{}</td><td>{}</td></tr>\n",
                esc(&label),
                esc(place),
                esc(desc),
                note,
                default
            ));
        }
    }
    out.push_str("</tbody></table></div>\n");
}

/// The rows of the Game options box, page by page.
fn extras_rows(out: &mut String) {
    use crate::extras::Settings;
    let off = (Extras::default(), Settings::ALL_OFF);
    let ui0 = new_player_ui();
    let helpers = Preset::Helpers.values().unwrap();
    let osg = Preset::OpenSGolf.values().unwrap();
    // a tick the presets leave as the player has it (extras.rs `with_kept`)
    let kept = |l: ExtrasLine| match l {
        ExtrasLine::Ui(i) => Settings::ALL_OFF.with_kept(&Settings::from_bits(1 << i)).bits() & 1 << i != 0,
        _ => false,
    };
    out.push_str("<h3 id=\"ref-game-options\">Game options</h3>\n");
    out.push_str(&format!(
        "<p class=\"lede\"><b>Preferences › Game options</b>. The first line steps through the presets: <b>{}</b>, <b>{}</b>, <b>{}</b> and <b>{}</b>. \
         The same choice is the <b>Game</b> box on Select Difficulty and the launcher's <b>How to play</b> tab. \
         Lines that change how the game plays carry a tag: <i>{}</i>, <i>{}</i>, <i>{}</i> or <i>{}</i>.</p>\n",
        esc(tr(Preset::Original.key())),
        esc(tr(Preset::Helpers.key())),
        esc(tr(Preset::OpenSGolf.key())),
        esc(tr(Preset::Custom.key())),
        esc(tr(Tag::Light.key())),
        esc(tr(Tag::Noticeable.key())),
        esc(tr(Tag::Big.key())),
        esc(tr(Tag::Experimental.key())),
    ));
    for page in 0..extras_ui::PAGES.len() as u8 {
        let name = tr(extras_ui::PAGES[page as usize]);
        out.push_str(&format!(
            "<h4 id=\"ref-game-options-{}\">{} <span class=\"banner\">{}</span></h4>\n",
            name.to_lowercase().replace(' ', "-"),
            esc(name),
            esc(tr(extras_ui::BANNERS[page as usize]))
        ));
        out.push_str(&format!(
            "<div class=\"tw\"><table>\n<thead><tr><th>Option</th><th>What it does</th><th>New player</th><th>{}</th><th>{}</th></tr></thead>\n<tbody>\n",
            esc(tr(Preset::Helpers.key())),
            esc(tr(Preset::OpenSGolf.key()))
        ));
        // every line, the cut content's too (listed only with the original's files)
        extras_ui::show_cut_content(true);
        for l in extras_ui::page_lines(page) {
            let (text, _) = extras_ui::line_text(l, &off.0, &off.1);
            // a line's name without its value ("Calendar: normal" -> "Calendar")
            let text = match text.split_once(':') {
                Some((a, _)) if extras_ui::is_plain(l) => a.to_string(),
                _ => text,
            };
            let key = match l {
                ExtrasLine::Game(i) => sg_core::extras::OPTION_KEYS[i],
                ExtrasLine::Ui(i) => crate::extras::UI_KEYS[i],
                ExtrasLine::Calendar => "STR_EXTRAS_CALENDAR",
                ExtrasLine::Group(g) => extras_ui::GROUPS[g].0,
                ExtrasLine::Preset => "STR_EXTRAS_PRESET",
            };
            let desc = tr(extras_ui::desc_key(l, &off.0, &off.1));
            let tag = extras_ui::tag(l).map_or(String::new(), |t| format!(" <span class=\"tag\">{}</span>", esc(tr(t.key()))));
            let value = |e: &Extras, ui: &Settings| {
                let v = extras_ui::line_value(l, e, ui);
                if extras_ui::is_plain(l) {
                    let (t, _) = extras_ui::line_text(l, e, ui);
                    let t = t.split_once(':').map_or(t.as_str(), |(_, b)| b).trim().to_string();
                    pill(v != 0, &t)
                } else {
                    on_off(v == 1)
                }
            };
            let new = value(&off.0, &ui0);
            let (h, o) = if kept(l) {
                let k = "<span class=\"pill kept\">Your choice</span>".to_string();
                (k.clone(), k)
            } else {
                (value(&helpers.0, &helpers.1), value(&osg.0, &osg.1))
            };
            let when = if l == ExtrasLine::Game(1) { " <span class=\"when\">Listed only with your own SimGolf files.</span>" } else { "" };
            out.push_str(&format!(
                "<tr id=\"{}\"><td><b>{}</b>{}</td><td>{}{}</td><td>{}</td><td>{}</td><td>{}</td></tr>\n",
                anchor(key),
                esc(&text),
                tag,
                esc(desc),
                when,
                new,
                h,
                o
            ));
        }
        out.push_str("</tbody></table></div>\n");
    }
}

/// The rows of the Enhanced graphics settings, page by page.
fn enhanced_rows(out: &mut String) {
    let d = enhanced::Settings::DEFAULT;
    out.push_str("<h3 id=\"ref-enhanced\">Enhanced graphics settings</h3>\n");
    out.push_str(&format!(
        "<p class=\"lede\"><b>Preferences › Graphics › Enhanced graphics settings</b>. Every page starts with the master switch, <b>{}</b>, which is <b>off</b> for a new player: \
         with it off you play in Classic and none of these draw, except the three marked as counting with Classic too. \
         The last column is what each option is set to once you switch Enhanced graphics on. {}</p>\n",
        esc(tr("STR_ENHANCED_MASTER")),
        esc(tr("STR_ENHANCED_BANNER"))
    ));
    for page in 0..enhanced::PAGES.len() as u8 {
        let name = tr(enhanced::PAGES[page as usize]);
        out.push_str(&format!("<h4 id=\"ref-enhanced-{}\">{}</h4>\n", name.to_lowercase(), esc(name)));
        out.push_str(
            "<div class=\"tw\"><table>\n<thead><tr><th>Option</th><th>What it does</th><th>With Enhanced on</th></tr></thead>\n<tbody>\n",
        );
        for o in enhanced::OPTIONS.iter().filter(|o| o.page == page) {
            let label = tr(o.label);
            let label = if o.choices.is_empty() { label.to_string() } else { label.split(':').next().unwrap_or(label).to_string() };
            let v = (o.get)(&d);
            let value = match o.choices.get(v as usize) {
                Some(c) => pill(true, tr(c)),
                None => on_off(v != 0),
            };
            out.push_str(&format!(
                "<tr id=\"opt-enhanced-{}\"><td><b>{}</b></td><td>{}</td><td>{}</td></tr>\n",
                o.key.replace('_', "-"),
                esc(&label),
                esc(enhanced::desc_key(o.key).map(tr).unwrap_or("")),
                value
            ));
        }
        out.push_str("</tbody></table></div>\n");
    }
}

/// A key's name as the page shows it: letters as on the keyboard (the game's own list writes a letter without Shift in
/// lower case).
fn key_label(b: controls::Binding) -> String {
    let l = b.label();
    if l.chars().count() == 1 {
        l.to_uppercase()
    } else {
        l
    }
}

/// The keys of Change keys.
fn key_rows(out: &mut String) {
    let keys = controls::Settings::default();
    out.push_str("<h3 id=\"ref-keys\">Every key</h3>\n");
    out.push_str("<p class=\"lede\">Each can be given another key on <b>Preferences › Controls and keys › Change keys</b>, which also has the <b>Original</b> and <b>Modern controls</b> presets and the <b>WASD camera</b>. F8 in a game lists them too.</p>\n");
    out.push_str("<div class=\"tw\"><table class=\"keys\">\n<thead><tr><th>Action</th><th>Key</th></tr></thead>\n<tbody>\n");
    for (i, a) in controls::ACTIONS.iter().enumerate() {
        let key = keys.binding(i).map(key_label).unwrap_or_else(|| "-".into());
        let wasd = match a.wasd {
            controls::Wasd::None => String::new(),
            controls::Wasd::Also(k) => format!(
                " <span class=\"when\">WASD camera: also {}</span>",
                esc(&key_label(controls::Binding { key: k, shift: false, ctrl: false }))
            ),
            controls::Wasd::Moves(k) => format!(
                " <span class=\"when\">WASD camera: {}</span>",
                esc(&key_label(controls::Binding { key: k, shift: false, ctrl: false }))
            ),
        };
        let aim = if a.ctx == controls::Ctx::Aim { " <span class=\"when\">While aiming your pro</span>" } else { "" };
        out.push_str(&format!("<tr><td>{}{}</td><td><kbd>{}</kbd>{}</td></tr>\n", esc(tr(a.text)), aim, esc(&key), wasd));
    }
    out.push_str("</tbody></table></div>\n");
}

/// The generated part of web/guide.html, markers included.
pub fn reference() -> String {
    let mut out = String::new();
    out.push_str(BEGIN);
    out.push('\n');
    prefs_rows(&mut out);
    extras_rows(&mut out);
    enhanced_rows(&mut out);
    key_rows(&mut out);
    out.push_str(END);
    out
}

#[cfg(test)]
mod tests {
    use super::*;

    fn page_path() -> std::path::PathBuf {
        std::path::Path::new(env!("CARGO_MANIFEST_DIR")).join("../../web/guide.html")
    }

    #[test]
    fn every_enhanced_option_has_a_note() {
        let missing: Vec<_> = enhanced::OPTIONS.iter().filter(|o| enhanced::desc_key(o.key).is_none()).map(|o| o.key).collect();
        assert!(missing.is_empty(), "add a STR_ENHANCED_DESC_ line for {missing:?} to enhanced.rs desc_key and lang/en.txt");
    }

    #[test]
    fn guide_reference_is_current() {
        let path = page_path();
        let page = std::fs::read_to_string(&path).expect("web/guide.html");
        let (Some(a), Some(b)) = (page.find(BEGIN), page.find(END)) else {
            panic!("web/guide.html has lost its reference markers ({BEGIN} ... {END})");
        };
        let now = &page[a..b + END.len()];
        let want = reference();
        if now == want {
            return;
        }
        if std::env::var_os("OPENSGOLF_WRITE_GUIDE").is_some() {
            let new = format!("{}{}{}", &page[..a], want, &page[b + END.len()..]);
            std::fs::write(&path, new).expect("writing web/guide.html");
            return;
        }
        panic!(
            "web/guide.html's option reference is out of date (an option, key or text changed). \
             Run OPENSGOLF_WRITE_GUIDE=1 cargo test -p opensgolf guide to write it, and check the page's hand-written sections too"
        );
    }
}
