//! The save self-check (the port's own; docs/PLAYING.md "Save self-check"): each time the game is saved, the saved file is
//! read back in memory into a scratch copy of the game and its simulation fingerprint (fingerprint.rs) compared with the
//! fingerprint the live game had when it was saved. They are the same whenever the save keeps everything that decides how
//! the game plays on, so a difference is a save fidelity bug: the game then keeps a small report in the save folder's
//! `Reports` and, at the next start, tells the player once on the ticker. Nothing is shown otherwise, nothing is sent
//! anywhere, and the live game, the screen, the sounds and the ticker are never touched by the check.
//!
//! When: after every save (the Autosave extra's, the player's, the quick save, the career's end ...) but at most once
//! every [`MIN_GAP`] seconds, never for the course editor's saves. The live game's fingerprint is taken as the file is written
//! (a few milliseconds); the file is read back over the next two drawn frames (unpacked on the first, loaded into the
//! scratch copy and fingerprinted on the second), so no frame takes the whole check, also in the browser.
//!
//! The scratch copy is a game of its own (`App::with_clock`), loaded by the game's own load (`App::load_save_into` without
//! art: no textures, portraits, scripts or messages). Before the comparison it takes from the live game what a load in the
//! same session keeps or what the game hands the simulation before each tick, which a save rightly does not hold: each
//! golfer's screen position (`sx`, `sy`: set from the camera on every drawn frame), the ticker's busy and message-up flags, the
//! advisor's and the close view's flags, the snapshots still to queue, the pointer's tile, the scripted runs' aiming, the tiles
//! retyped since the wildlife looked, the staff code's golfer copies and the cosmetic generator when the save has none.
//! The live game's course copy of the staff's weeds and growth, which every game tick refreshes from the staff's tiles
//! before anything reads it (`app::weeds_into`; the load does the same), is compared as the next tick will see it. The
//! scripts are not run in the scratch copy: their saved state (with the goals) is compared with the live game's. A save
//! while the hole being built has its tee and cup is not checked: loading it plans that hole again with the generator as
//! loaded, as the original's load does (0x42f7a0 clears game flag 0x40000 after 0x40b9b0 restores it), so the loaded game
//! rightly differs from the running one there (tools/determinism_check.sh checks that two loads of it agree).
//!
//! Test hooks: `SG_SELFCHECK=1` prints each check's result and time, and checks every save (no gap); `SG_SELFCHECK=0`
//! turns the check off; `SG_SELFCHECK_BREAK=1` changes the money in the scratch copy, so a check finds a difference and
//! the report path can be tried headless (tools/selfcheck_test.sh).
//!
//! For the "Report a problem" button: [`latest_report_files`] gives the newest report's files (its text, then the save).

use crate::app::{read_save_bytes, App, SaveGame, Screen};
use sg_core::fingerprint::{hash, Fingerprint};
use sg_core::fsutil;
use sg_core::land::ExeRng;
use sg_core::lang::tr;
use std::cell::RefCell;
use std::path::PathBuf;

/// Seconds of wall time between two checks at least (`SG_SELFCHECK=1`: none).
pub const MIN_GAP: f64 = 120.0;

/// The reports kept: the newest five, each a text and the save it was found with.
pub const KEPT: usize = 5;

/// A report's files' names start with this, then the date and time ("selfcheck-20261024-231500.txt" and ".sgs").
const PREFIX: &str = "selfcheck-";

/// The name of the newest report the player has been told about, in `Reports`.
const TOLD: &str = "told.txt";

/// What the scratch copy takes from the live game (see the module's notes), as it stood when the game was saved.
struct Kept {
    screen: Vec<(i32, i32)>,
    ticker_busy: bool,
    advisor: bool,
    view_close: bool,
    message_up: bool,
    show_snapshots: bool,
    award_snaps: Vec<(usize, (i32, i32))>,
    hover_tile: Option<(i32, i32)>,
    auto_aim: u8,
    retyped: Vec<(i32, i32)>,
    staff_golfers: Vec<sg_core::staff::StaffGolfer>,
    deco_rng: ExeRng,
    game_dir: PathBuf,
    seed: u32,
}

impl Kept {
    fn of(a: &App) -> Kept {
        let c = &a.club;
        Kept {
            screen: c.g.iter().map(|g| (g.sx, g.sy)).collect(),
            ticker_busy: c.ticker_busy,
            advisor: c.advisor,
            view_close: c.view_close,
            message_up: c.message_up,
            show_snapshots: c.show_snapshots,
            award_snaps: c.award_snaps.clone(),
            hover_tile: a.hover_tile,
            auto_aim: a.auto_aim,
            retyped: a.retyped.clone(),
            staff_golfers: a.staff_golfers.clone(),
            deco_rng: a.deco_rng,
            game_dir: a.game_dir.clone(),
            seed: a.seed,
        }
    }
}

enum Stage {
    /// The save's bytes, read back on the next frame.
    Bytes(Vec<u8>),
    /// The save read back, loaded and fingerprinted on the frame after.
    Read(Box<SaveGame>, Vec<u8>),
}

/// A check under way: the live game's fingerprint at the save, and what the scratch copy takes from it.
struct Pending {
    stage: Stage,
    live: Fingerprint,
    live_fields: Vec<(String, u64)>,
    kept: Kept,
    tick: u32,
    /// Milliseconds spent at the save, reading back and loading.
    ms: Vec<f64>,
}

#[derive(Default)]
struct State {
    pending: Option<Pending>,
    /// When the last check began (seconds), none yet.
    last: Option<f64>,
    /// The report from an earlier session the player is still to be told of (looked up at the first frame).
    tell: Option<Option<String>>,
}

thread_local! {
    static STATE: RefCell<State> = RefCell::new(State::default());
}

fn env() -> Option<String> {
    std::env::var("SG_SELFCHECK").ok().map(|v| v.trim().to_string())
}

fn logging() -> bool {
    env().as_deref() == Some("1")
}

fn now() -> f64 {
    miniquad::date::now()
}

/// Where the reports go: `Reports` in the save folder (in the browser its own storage, as the saved games; on Android the
/// app's external files folder, as the photos).
pub fn reports_dir() -> PathBuf {
    #[cfg(target_os = "android")]
    if let Some(p) = crate::android::game_dir().parent() {
        return within(p, "Reports");
    }
    within(&crate::app::save_dir(), "Reports")
}

/// A file in a folder.
fn within(dir: &std::path::Path, name: &str) -> PathBuf {
    let mut p = dir.to_path_buf();
    p.push(name);
    p
}

/// The words one after another with `sep` between them.
fn spaced(words: &[String], sep: &str) -> String {
    let mut s = String::new();
    for (i, w) in words.iter().enumerate() {
        if i > 0 {
            s.push_str(sep);
        }
        s.push_str(w);
    }
    s
}

/// The reports' names (without extension), oldest first.
fn report_stems(dir: &std::path::Path) -> Vec<String> {
    let mut stems: Vec<String> = fsutil::list_dir(dir)
        .iter()
        .filter_map(|p| p.file_name()?.to_str().map(str::to_string))
        .filter(|n| n.starts_with(PREFIX) && n.ends_with(".txt"))
        .map(|n| n.trim_end_matches(".txt").to_string())
        .collect();
    stems.sort();
    stems
}

/// The hook for "Report a problem": the newest report's files, its text and the save it was found with (empty when there
/// is none). Nothing here sends them; the files stay where they are.
#[allow(dead_code)] // for the Report a problem button, which attaches it
pub fn latest_report_files() -> Vec<PathBuf> {
    let dir = reports_dir();
    let Some(stem) = report_stems(&dir).pop() else { return Vec::new() };
    [".txt", ".sgs"].iter().map(|e| within(&dir, &format!("{stem}{e}"))).filter(|p| fsutil::exists(p)).collect()
}

/// The sections, and the fields of the club, the economy and the course, whose hashes differ between two fingerprints.
pub fn differences(a: &[(&'static str, u64)], b: &[(&'static str, u64)], fa: &[(String, u64)], fb: &[(String, u64)]) -> Vec<String> {
    let mut out: Vec<String> = Vec::new();
    for (x, y) in a.iter().zip(b) {
        if x != y {
            out.push(x.0.to_string());
        }
    }
    if a.len() != b.len() {
        out.push("(section count)".into());
    }
    for (x, y) in fa.iter().zip(fb) {
        if x != y {
            out.push(x.0.clone());
        }
    }
    out
}

/// A report's text: what differs, the build, the platform and the date; nothing about the player or their computer.
fn report_text(p: &Pending, scratch: &Fingerprint, differ: &[String], secs: i64) -> String {
    let (y, mo, d, h, mi, s) = crate::photo::civil(secs);
    let platform =
        if cfg!(target_arch = "wasm32") { "web".to_string() } else { format!("{} {}", std::env::consts::OS, std::env::consts::ARCH) };
    format!(
        "OpenSGolf save self-check report\n\
         version: {}\n\
         platform: {platform}\n\
         date: {y:04}-{mo:02}-{d:02} {h:02}:{mi:02}:{s:02} UTC\n\
         game tick: {}\n\
         course size: {}\n\
         found: the saved game, read back, does not give the game it was saved from\n\
         differ: {}\n\
         saved from: {}\n\
         read back:  {}\n\
         the saved game: the .sgs file of the same name\n",
        env!("CARGO_PKG_VERSION"),
        p.tick,
        sg_core::mapsize::n(),
        spaced(differ, " "),
        p.live.text(true),
        scratch.text(true),
    )
}

/// Writes a report (its text and the save) and keeps only the newest `KEPT`. Returns the report's name.
fn write_report(p: &Pending, bytes: &[u8], scratch: &Fingerprint, differ: &[String]) -> Option<String> {
    let dir = reports_dir();
    let secs = now() as i64;
    let (y, mo, d, h, mi, s) = crate::photo::civil(secs);
    let base = format!("{PREFIX}{y:04}{mo:02}{d:02}-{h:02}{mi:02}{s:02}");
    let have = report_stems(&dir);
    let mut stem = base.clone();
    let mut n = 2;
    while have.contains(&stem) {
        stem = format!("{base}-{n}");
        n += 1;
    }
    let text = report_text(p, scratch, differ, secs);
    if !fsutil::write_file(within(&dir, &format!("{stem}.sgs")), bytes)
        || !fsutil::write_file(within(&dir, &format!("{stem}.txt")), text.as_bytes())
    {
        return None;
    }
    let all = report_stems(&dir);
    for old in prune(&all) {
        fsutil::remove_file(within(&dir, &format!("{old}.txt")));
        fsutil::remove_file(within(&dir, &format!("{old}.sgs")));
    }
    Some(stem)
}

/// The reports to delete so that the newest `KEPT` stay (names oldest first).
pub fn prune(stems: &[String]) -> Vec<String> {
    stems.iter().take(stems.len().saturating_sub(KEPT)).cloned().collect()
}

/// Whether a report newer than the one last told about is there.
fn untold() -> Option<String> {
    let dir = reports_dir();
    let newest = report_stems(&dir).pop()?;
    let told = fsutil::read_text(within(&dir, TOLD)).unwrap_or_default();
    (newest.as_str() > told.trim()).then_some(newest)
}

impl App {
    /// After the game was saved as `bytes`: begins a check of the save (see the module's notes).
    pub(crate) fn selfcheck_after_save(&self, bytes: &[u8]) {
        let env = env();
        // (a game before its first tick, such as the title screens' course, has not had the simulation's inputs yet)
        if env.as_deref() == Some("0") || self.editor.is_some() || self.land.is_none() || self.game_tick == 0 {
            return;
        }
        let t0 = now();
        let due = STATE.with(|s| {
            let s = s.borrow();
            s.pending.is_none() && (env.as_deref() == Some("1") || s.last.is_none_or(|l| t0 - l >= MIN_GAP || t0 < l))
        });
        if !due {
            return;
        }
        // a hole being built with its tee and cup: the load plans it again, as the original's load does (0x42f7a0 clears
        // game flag 0x40000 after 0x40b9b0 restores it), so the loaded game differs from the running one by design
        if building(&self.club) {
            if env.as_deref() == Some("1") {
                println!(
                    "selfcheck: skipped at tick {} (a hole being built is planned again on load, as the original does)",
                    self.game_tick
                );
            }
            return;
        }
        let kept = Kept::of(self);
        let mut live = self.fingerprint();
        // the course's copy of the staff's weeds and growth is refreshed at the start of every game tick (weeds_into) and
        // by the load; the live game's is compared as the next tick will see it
        let mut course = self.course.clone();
        crate::app::weeds_into(&self.staff_tiles, &mut course);
        for part in live.parts.iter_mut().filter(|p| p.0 == "course") {
            part.1 = hash(&course);
        }
        let live_fields = self.fingerprint_field_list_with(&course);
        let ms = vec![(now() - t0) * 1000.0];
        let p = Pending { stage: Stage::Bytes(bytes.to_vec()), live, live_fields, kept, tick: self.game_tick, ms };
        STATE.with(|s| {
            let mut s = s.borrow_mut();
            s.pending = Some(p);
            s.last = Some(t0);
        });
    }

    /// Each frame: the next step of a check under way, and the once-a-session word to the player about a report from an
    /// earlier session.
    pub fn selfcheck_step(&mut self) {
        let tell = STATE.with(|s| s.borrow_mut().tell.get_or_insert_with(untold).clone());
        if let Some(stem) = tell.filter(|_| self.screen == Screen::Play && self.land.is_some() && self.popup.is_none()) {
            if self.post_message(tr("STR_SELFCHECK_FOUND"), 1, crate::message_ui::NOBODY) {
                fsutil::write_file(within(&reports_dir(), TOLD), stem.as_bytes());
                STATE.with(|s| s.borrow_mut().tell = Some(None));
            }
        }
        let Some(mut p) = STATE.with(|s| s.borrow_mut().pending.take()) else { return };
        let t0 = now();
        match std::mem::replace(&mut p.stage, Stage::Bytes(Vec::new())) {
            Stage::Bytes(bytes) => match read_save_bytes(&bytes, "self-check") {
                Ok(s) => {
                    p.stage = Stage::Read(Box::new(s), bytes);
                    p.ms.push((now() - t0) * 1000.0);
                    STATE.with(|s| s.borrow_mut().pending = Some(p));
                }
                Err(e) => self.selfcheck_done(p, &bytes, Err(e), t0),
            },
            Stage::Read(s, bytes) => {
                let r = scratch_fingerprint(*s, &p.kept);
                self.selfcheck_done(p, &bytes, r, t0);
            }
        }
    }

    /// Finishes a check under way at once (before the program ends: the --save-game test hook).
    pub fn selfcheck_flush(&mut self) {
        for _ in 0..2 {
            if STATE.with(|s| s.borrow().pending.is_none()) {
                break;
            }
            let tell = STATE.with(|s| s.borrow_mut().tell.replace(None));
            self.selfcheck_step();
            STATE.with(|s| s.borrow_mut().tell = tell);
        }
    }

    fn selfcheck_done(&mut self, mut p: Pending, bytes: &[u8], r: Result<(Fingerprint, Vec<(String, u64)>), String>, t0: f64) {
        p.ms.push((now() - t0) * 1000.0);
        let (scratch, differ) = match r {
            Ok((f, fields)) => {
                let d = differences(&p.live.parts, &f.parts, &p.live_fields, &fields);
                (f, d)
            }
            Err(e) => (Fingerprint::new(), vec![format!("(not read back: {e})")]),
        };
        if differ.is_empty() {
            if logging() {
                println!("selfcheck: ok at tick {} ({})", p.tick, times(&p.ms));
            }
            return;
        }
        let name = write_report(&p, bytes, &scratch, &differ);
        eprintln!("selfcheck: the save read back differs ({}); report {}", spaced(&differ, " "), name.as_deref().unwrap_or("not written"));
        if logging() {
            println!("selfcheck: differs at tick {} ({})", p.tick, times(&p.ms));
        }
    }
}

/// "12.3 ms: 4.0 + 3.1 + 5.2": the check's time in all and on each frame it took.
fn times(ms: &[f64]) -> String {
    let parts: Vec<String> = ms.iter().map(|m| format!("{m:.1}")).collect();
    format!("{:.1} ms: {}", ms.iter().sum::<f64>(), spaced(&parts, " + "))
}

/// Whether the hole being built has its tee and cup (sg_core::holetool `layout` plans it).
fn building(c: &sg_core::golfer::Club) -> bool {
    let h = c.next_hole;
    (1..=18).contains(&h) && c.holes.get(h as usize).is_some_and(|r| r.back.0 != 0 && r.pin.0 != 0)
}

/// The fingerprint of a saved game read back into a scratch copy of the game, with what the copy takes from the live game.
fn scratch_fingerprint(s: SaveGame, k: &Kept) -> Result<(Fingerprint, Vec<(String, u64)>), String> {
    let scripts = hash(&(&s.goals, &s.scripts));
    let mut a = Box::new(App::with_clock(k.game_dir.clone(), 0));
    a.seed = k.seed;
    a.deco_rng = k.deco_rng;
    a.staff_golfers = k.staff_golfers.clone();
    a.hover_tile = k.hover_tile;
    a.auto_aim = k.auto_aim;
    a.retyped = k.retyped.clone();
    a.load_save_into(None, s, false)?;
    for (g, &(sx, sy)) in a.club.g.iter_mut().zip(&k.screen) {
        g.sx = sx;
        g.sy = sy;
    }
    let c = &mut a.club;
    c.ticker_busy = k.ticker_busy;
    c.advisor = k.advisor;
    c.view_close = k.view_close;
    c.message_up = k.message_up;
    c.show_snapshots = k.show_snapshots;
    c.award_snaps = k.award_snaps.clone();
    if std::env::var("SG_SELFCHECK_BREAK").is_ok_and(|v| v.trim() == "1") {
        a.econ.cash += 1.0;
    }
    let mut f = a.fingerprint();
    for part in f.parts.iter_mut().filter(|p| p.0 == "scripts") {
        part.1 = scripts;
    }
    Ok((f, a.fingerprint_field_list()))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn the_newest_five_stay() {
        let stems: Vec<String> = (0..8).map(|i| format!("{PREFIX}2026010{i}-000000")).collect();
        let gone = prune(&stems);
        assert_eq!(gone, stems[..3].to_vec());
        assert!(prune(&stems[..5]).is_empty());
        assert!(prune(&[]).is_empty());
    }

    #[test]
    fn differences_name_sections_and_fields() {
        let a = [("club", 1), ("econ", 2), ("course", 3)];
        let b = [("club", 1), ("econ", 9), ("course", 3)];
        let fa = vec![("econ.cash".to_string(), 4), ("econ.debt".to_string(), 5)];
        let fb = vec![("econ.cash".to_string(), 7), ("econ.debt".to_string(), 5)];
        assert_eq!(differences(&a, &b, &fa, &fb), ["econ", "econ.cash"]);
        assert!(differences(&a, &a, &fa, &fa).is_empty());
    }

    #[test]
    fn a_hole_being_built_is_seen() {
        let mut c = sg_core::golfer::Club::default();
        assert!(!building(&c));
        let h = c.next_hole as usize;
        c.holes[h].back = (5, 6);
        assert!(!building(&c));
        c.holes[h].pin = (9, 9);
        assert!(building(&c));
        c.next_hole = 19;
        assert!(!building(&c));
    }

    #[test]
    fn times_add_up() {
        assert_eq!(times(&[1.0, 2.25, 3.0]), "6.2 ms: 1.0 + 2.2 + 3.0");
    }
}
