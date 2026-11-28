//! Saves per course (SG-Q1). Every course has its own set of save files in Saved Games, so one course's autosaves and
//! quick save can never push another course's out:
//!
//!   &<set>~auto.sgs, &<set>~auto2.sgs, &<set>~auto3.sgs   the three rolling autosaves (newest first)
//!   &<set>~quick.sgs (+ .bak, the one before it)           SAVE GAME's save
//!   &<set>~view.txt                                        where the view was (SG-Q8)
//!
//! <set> is the course's name; a second course begun on the same plot becomes "<name> 2", and so on. The older builds'
//! shared files (&Latest.sgs, &Latest-2.sgs, &Latest-3.sgs, &Quick.sgs) are never written or removed any more: they
//! stay in the Load list and open as before, and a game opened from one of them goes on in a set of its own.

use std::path::{Path, PathBuf};
use std::sync::Mutex;

/// The set of the game in play ("" before a game is open).
static SET: Mutex<String> = Mutex::new(String::new());

/// The autosaves' kinds, newest first.
pub const AUTO: [&str; 3] = ["auto", "auto2", "auto3"];
const KINDS: [&str; 4] = ["auto", "auto2", "auto3", "quick"];

fn dir() -> PathBuf {
    crate::files_ui::saved_games_dir()
}

/// A course name as a set name: letters, digits, spaces and hyphens only (the SD card's FAT names), at most 24 long.
pub fn clean(name: &str) -> String {
    let mut s: String = name.chars().filter(|c| c.is_ascii_alphanumeric() || *c == ' ' || *c == '-').collect();
    s = s.split_whitespace().collect::<Vec<_>>().join(" ");
    s.truncate(24);
    let s = s.trim().to_string();
    if s.is_empty() {
        "Course".to_string()
    } else {
        s
    }
}

/// A save's file stem as (set, kind) when it is one of this module's: "&Pebble Creek~auto2" -> ("Pebble Creek", "auto2").
pub fn parse(stem: &str) -> Option<(String, &'static str)> {
    let (set, kind) = stem.strip_prefix('&')?.rsplit_once('~')?;
    let kind = KINDS.iter().find(|k| k.eq_ignore_ascii_case(kind))?;
    (!set.is_empty()).then(|| (set.to_string(), *kind))
}

/// The file of a set's save of one kind.
pub fn file(set: &str, kind: &str) -> PathBuf {
    dir().join(format!("&{set}~{kind}.sgs"))
}

/// The sets that have a file on the card (lower case: the card's names ignore case).
fn sets_on_card() -> Vec<String> {
    let mut v = Vec::new();
    if let Ok(rd) = std::fs::read_dir(dir()) {
        for e in rd.flatten() {
            let p = e.path();
            let stem = p.file_stem().map(|s| s.to_string_lossy().to_string()).unwrap_or_default();
            if let Some((set, _)) = parse(&stem) {
                v.push(set.to_ascii_lowercase());
            }
        }
    }
    v
}

fn set_current(s: String) {
    crate::gfx::note(&format!("saves: this course's set is \"{s}\""));
    if let Ok(mut g) = SET.lock() {
        *g = s;
    }
}

/// A new course was started: it gets a set no other course on the card has.
pub fn begin_new(course: &str) {
    let base = clean(course);
    let taken = sets_on_card();
    let mut name = base.clone();
    let mut n = 2;
    while taken.contains(&name.to_ascii_lowercase()) && n < 1000 {
        name = format!("{base} {n}");
        n += 1;
    }
    set_current(name);
}

/// A save was opened: the game goes on in that save's set; a save from an older build (or any other file) starts a set of
/// its own, so nothing is ever written over the file it came from.
pub fn begin_open(path: &Path, course: &str) {
    let stem = path.file_stem().map(|s| s.to_string_lossy().to_string()).unwrap_or_default();
    match parse(&stem) {
        Some((set, _)) => set_current(set),
        None => begin_new(course),
    }
}

/// The set of the game in play.
pub fn current() -> String {
    let s = SET.lock().map(|g| g.clone()).unwrap_or_default();
    if s.is_empty() {
        "Course".to_string()
    } else {
        s
    }
}

/// This course's SAVE GAME file.
pub fn quick() -> PathBuf {
    file(&current(), "quick")
}

fn view_file(set: &str) -> PathBuf {
    dir().join(format!("&{set}~view.txt"))
}

/// SG-Q8: where the view was when the course was last saved (camera x, z and zoom), kept beside the set's saves.
pub fn write_view(x: f32, z: f32, zoom: f32) {
    let p = view_file(&current());
    if let Err(e) = std::fs::write(&p, format!("{x} {z} {zoom}\n")) {
        crate::gfx::note(&format!("view not saved: {e}"));
    }
}

/// The view kept for the game in play's set, if there is one and it reads as three numbers.
pub fn read_view() -> Option<(f32, f32, f32)> {
    let t = std::fs::read_to_string(view_file(&current())).ok()?;
    let mut it = t.split_whitespace().map(|v| v.parse::<f32>());
    let (x, z, zoom) = (it.next()?.ok()?, it.next()?.ok()?, it.next()?.ok()?);
    (x.is_finite() && z.is_finite() && zoom.is_finite() && zoom > 0.0).then_some((x, z, zoom))
}
