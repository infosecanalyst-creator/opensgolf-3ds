//! Saves newest first, with dates and backups (an interface extra, extras.rs `save_newest`; the port's own, not in
//! the original, which lists the saves by name and keeps only the last of each):
//!
//! * Load Previous Game lists the saved games newest first, each with how long ago it was saved ("5 min ago", "3 days
//!   ago", then the date).
//! * Saving over a game first keeps the game that was there: up to `KEEP` of each, as "My Club~1.sgs" (the newest) to
//!   "My Club~3.sgs" in `Saved Games/Backups`, listed at the foot of the Load list as "My Club (backup 1)". Loading one
//!   plays it as a game of its own: saving it makes a new save beside the original rather than replacing it.
//!
//! When each save was made is kept in `Saved Games/saves.json` (file name to Unix seconds), written through fsutil so it
//! is in the browser's storage as well, where files have no dates of their own. A save the index does not know (one
//! copied in, or made with the extra off) takes its file's date on the desktop, and sorts last, by name, in the browser.
//! Autosaves are not backed up: they keep their own three (autosave.rs).
//!
//! With the extra on, the title menu also gets a **Continue** button under Check for updates, naming the newest save
//! (autosaves included) and how long ago it was made; a click loads it as the Load list's tick would.

use sg_core::fsutil;
use std::collections::BTreeMap;
use std::path::{Path, PathBuf};

/// The index of when each save was made, in Saved Games.
pub const INDEX: &str = "saves.json";
/// The backups' folder, in Saved Games.
pub const DIR: &str = "Backups";
/// Backups kept of each save.
pub const KEEP: usize = 3;

/// A file's name as the index keeps it (as on disk; looked up without regard to case, as the Load list's names are).
fn key(p: &Path) -> String {
    p.file_name().map(|n| n.to_string_lossy().into_owned()).unwrap_or_default()
}

/// The index's time for a key, its case ignored.
fn lookup(idx: &BTreeMap<String, f64>, k: &str) -> Option<f64> {
    idx.get(k).copied().or_else(|| idx.iter().find(|(n, _)| n.eq_ignore_ascii_case(k)).map(|(_, &t)| t))
}

/// The index of a Saved Games folder: file name (a backup's as "Backups/<name>") to Unix seconds.
pub fn index(dir: &Path) -> BTreeMap<String, f64> {
    fsutil::read_file(dir.join(INDEX)).and_then(|d| serde_json::from_slice(&d).ok()).unwrap_or_default()
}

fn write_index(dir: &Path, idx: &BTreeMap<String, f64>) {
    if let Ok(d) = serde_json::to_vec_pretty(idx) {
        fsutil::write_file(dir.join(INDEX), &d);
    }
}

/// The folder whose index holds `path`'s time: Saved Games for a save, Saved Games for a backup too (its key is
/// "backups/<name>").
fn index_place(path: &Path) -> Option<(PathBuf, String)> {
    let parent = path.parent()?;
    if parent.file_name().is_some_and(|n| n.to_string_lossy().eq_ignore_ascii_case(DIR)) {
        return Some((parent.parent()?.to_path_buf(), format!("{DIR}/{}", key(path))));
    }
    Some((parent.to_path_buf(), key(path)))
}

/// Records that `path` was saved at `when` (Unix seconds).
pub fn stamp(path: &Path, when: f64) {
    let Some((dir, k)) = index_place(path) else { return };
    let mut idx = index(&dir);
    idx.retain(|n, _| !n.eq_ignore_ascii_case(&k));
    idx.insert(k, when);
    // forget the saves that are gone (deleted from the Load list or by hand), so the index stays small
    idx.retain(|k, _| {
        let p = match k.strip_prefix(&format!("{DIR}/")) {
            Some(b) => dir.join(DIR).join(b),
            None => dir.join(k),
        };
        fsutil::exists(&p)
    });
    write_index(&dir, &idx);
}

/// When `path` was saved, in Unix seconds: the index's, or the file's own date on the desktop; None when neither is known.
pub fn saved_at(path: &Path, idx: &BTreeMap<String, f64>) -> Option<f64> {
    let k = index_place(path).map(|(_, k)| k)?;
    if let Some(t) = lookup(idx, &k) {
        return Some(t);
    }
    #[cfg(not(target_arch = "wasm32"))]
    {
        let m = std::fs::metadata(path).and_then(|m| m.modified()).ok()?;
        return m.duration_since(std::time::UNIX_EPOCH).ok().map(|d| d.as_secs_f64());
    }
    #[allow(unreachable_code)]
    None
}

/// The backup `n` (1 is the newest) of a save's file name stem.
pub fn backup_path(dir: &Path, stem: &str, n: usize) -> PathBuf {
    dir.join(DIR).join(format!("{stem}~{n}.sgs"))
}

/// Whether `path` is one of the backups (in the Backups folder), with its save's name and number.
pub fn backup_of(path: &Path) -> Option<(String, usize)> {
    let parent = path.parent()?.file_name()?.to_string_lossy().to_string();
    if !parent.eq_ignore_ascii_case(DIR) {
        return None;
    }
    let stem = path.file_stem()?.to_string_lossy().to_string();
    let (name, n) = stem.rsplit_once('~')?;
    Some((name.to_string(), n.parse().ok()?))
}

/// Before a save is written over: the game there now becomes backup 1, the older backups move one along and the oldest
/// is dropped. Nothing for an autosave ('&' names) or a save not there yet. True when a backup was made.
pub fn keep_backup(path: &Path) -> bool {
    let Some(stem) = path.file_stem().map(|s| s.to_string_lossy().to_string()) else { return false };
    let Some(dir) = path.parent() else { return false };
    if stem.starts_with('&') || backup_of(path).is_some() {
        return false;
    }
    let Some(now) = fsutil::read_raw(path) else { return false };
    let mut idx = index(dir);
    let bkey = |n: usize| format!("{DIR}/{}", key(&backup_path(dir, &stem, n)));
    for n in (1..KEEP).rev() {
        let (from, to) = (backup_path(dir, &stem, n), backup_path(dir, &stem, n + 1));
        if let Some(d) = fsutil::read_raw(&from) {
            if fsutil::write_file(&to, &d) {
                match idx.remove(&bkey(n)) {
                    Some(t) => idx.insert(bkey(n + 1), t),
                    None => idx.remove(&bkey(n + 1)),
                };
            }
        }
    }
    if !fsutil::write_file(backup_path(dir, &stem, 1), &now) {
        return false;
    }
    match saved_at(path, &idx) {
        Some(t) => idx.insert(bkey(1), t),
        None => idx.remove(&bkey(1)),
    };
    write_index(dir, &idx);
    true
}

/// The backups in a Saved Games folder.
pub fn list(dir: &Path) -> Vec<PathBuf> {
    fsutil::list_dir(dir.join(DIR))
        .into_iter()
        .filter(|p| p.extension().is_some_and(|x| x.eq_ignore_ascii_case("sgs")) && backup_of(p).is_some())
        .collect()
}

/// The Load list with the extra on: the saves newest first (the ones without a known time after them, by name), then
/// the backups newest first.
pub fn order(saves: Vec<PathBuf>, backups: Vec<PathBuf>, idx: &BTreeMap<String, f64>) -> Vec<PathBuf> {
    let sort = |mut v: Vec<PathBuf>| {
        v.sort_by(|a, b| {
            let (ta, tb) = (saved_at(a, idx), saved_at(b, idx));
            tb.partial_cmp(&ta).unwrap_or(std::cmp::Ordering::Equal).then_with(|| key(a).to_lowercase().cmp(&key(b).to_lowercase()))
        });
        v
    };
    let mut out = sort(saves);
    out.extend(sort(backups));
    out
}

/// How long ago `then` was, at `now` (both Unix seconds), in a few words: within a week as minutes, hours or days ago, then
/// as a date (UTC: the game knows no time zone in the browser, and a day is close enough).
pub fn age_words(now: f64, then: f64) -> String {
    use sg_core::lang::{tr, trf};
    let s = (now - then).max(0.0);
    if s < 60.0 {
        tr("STR_FILES_AGE_NOW").to_string()
    } else if s < 3600.0 {
        trf("STR_FILES_AGE_MIN", &[("N", &((s / 60.0) as i64))])
    } else if s < 86400.0 {
        trf("STR_FILES_AGE_HOURS", &[("N", &((s / 3600.0) as i64))])
    } else if s < 7.0 * 86400.0 {
        let d = (s / 86400.0) as i64;
        if d == 1 {
            tr("STR_FILES_AGE_YESTERDAY").to_string()
        } else {
            trf("STR_FILES_AGE_DAYS", &[("N", &d)])
        }
    } else {
        let (y, mo, d, ..) = crate::photo::civil(then as i64);
        format!("{y:04}-{mo:02}-{d:02}")
    }
}

/// The title menu's Continue button's hover id (main.rs; beside update.rs's and the others').
pub const CONTINUE_HOVER: i32 = 11;

/// The newest save in a Saved Games folder (autosaves too, not the backups) and when it was made.
pub fn newest(dir: &Path) -> Option<(PathBuf, f64)> {
    let idx = index(dir);
    fsutil::list_dir(dir)
        .into_iter()
        .filter(|p| p.extension().is_some_and(|x| x.eq_ignore_ascii_case("sgs")))
        .filter_map(|p| saved_at(&p, &idx).map(|t| (p, t)))
        .max_by(|a, b| a.1.partial_cmp(&b.1).unwrap_or(std::cmp::Ordering::Equal))
}

impl crate::app::App {
    /// The Continue button's save and label (None with the extra off or no dated save).
    pub fn continue_save(&self) -> Option<(PathBuf, String)> {
        if !crate::extras::save_newest() {
            return None;
        }
        let (p, t) = newest(&crate::files_ui::saved_games_dir())?;
        let label = sg_core::lang::trf(
            "STR_TITLE_CONTINUE_SAVE",
            &[("NAME", &crate::files_ui::row_name(&p)), ("AGE", &age_words(miniquad::date::now(), t))],
        );
        Some((p, label))
    }

    /// The Continue button's rectangle: under Check for updates at the top right (the update button's own size rule).
    pub fn continue_button(&self) -> Option<(crate::render::Rect, PathBuf, String)> {
        let (p, label) = self.continue_save()?;
        let w = crate::ui::F_MANUAL15.width(&label).clamp(120.0, 420.0) + 24.0;
        let y = if self.update_button().is_some() { 36.0 } else { 6.0 };
        Some((crate::render::Rect::new(792.0 - w, y, w, 24.0), p, label))
    }

    /// Draws the Continue button, in the update button's style.
    pub fn draw_continue_button(&self, g: &mut crate::gfx::Gfx, s: &crate::ui::Screen) {
        use crate::ui::{rgb, rgba};
        let Some((r, _, label)) = self.continue_button() else { return };
        let lit = self.hover == CONTINUE_HOVER;
        if crate::glass_ui::active() {
            crate::glass_ui::title_button(g, s, r.x, r.y, r.w, r.h, &label, lit);
            return;
        }
        s.fill(g, r.x - 1.0, r.y - 1.0, r.w + 2.0, r.h + 2.0, rgba(1.0, 1.0, 1.0, 0.85));
        s.fill(g, r.x, r.y, r.w, r.h, if lit { rgba(0.13, 0.32, 0.55, 0.95) } else { rgba(0.08, 0.16, 0.32, 0.88) });
        s.put_centered_fit(
            g,
            crate::ui::F_MANUAL15,
            r.x + r.w / 2.0,
            r.y + 4.0,
            r.w - 12.0,
            &label,
            rgb(1.0, 1.0, if lit { 1.0 } else { 0.8 }),
        );
    }

    /// A click on Continue: Load Previous Game opens with the save chosen and loads it, as its tick does (so a game
    /// saved with other content packs still asks first, content_profiles.rs).
    pub fn continue_click(&mut self) {
        let Some((_, p, _)) = self.continue_button() else { return };
        self.open_files(crate::files_ui::ListKind::Load, false);
        let row = self.title.files.as_ref().and_then(|f| f.files.iter().position(|q| q == &p));
        if let Some(row) = row {
            self.files_select_and_load(row);
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn backups_are_named_and_recognised() {
        let dir = Path::new("Saved Games");
        let b = backup_path(dir, "My Club", 2);
        assert_eq!(b, Path::new("Saved Games/Backups/My Club~2.sgs"));
        assert_eq!(backup_of(&b), Some(("My Club".to_string(), 2)));
        assert_eq!(backup_of(Path::new("Saved Games/My Club~2.sgs")), None);
    }

    #[test]
    fn newest_first_then_unknown_by_name_then_backups() {
        let mut idx = BTreeMap::new();
        idx.insert("b.sgs".to_string(), 100.0);
        idx.insert("c.sgs".to_string(), 300.0);
        idx.insert("Backups/c~1.sgs".to_string(), 200.0);
        let p = |s: &str| PathBuf::from(format!("/nonexistent-dir-for-test/{s}"));
        let saves = vec![p("a.sgs"), p("b.sgs"), p("c.sgs"), p("d.sgs")];
        let backups = vec![PathBuf::from("/nonexistent-dir-for-test/Backups/c~1.sgs")];
        let out = order(saves, backups, &idx);
        let names: Vec<String> = out.iter().map(|p| p.file_name().unwrap().to_string_lossy().to_string()).collect();
        assert_eq!(names, ["c.sgs", "b.sgs", "a.sgs", "d.sgs", "c~1.sgs"]);
    }

    #[test]
    fn ages_read_as_words_then_dates() {
        assert_eq!(age_words(1000.0, 990.0), "just now");
        assert_eq!(age_words(10_000.0, 10_000.0 - 300.0), "5 min ago");
        assert_eq!(age_words(100_000.0, 100_000.0 - 7300.0), "2 hours ago");
        assert_eq!(age_words(1_000_000.0, 1_000_000.0 - 90_000.0), "yesterday");
        assert_eq!(age_words(1_000_000.0, 1_000_000.0 - 3.0 * 86400.0), "3 days ago");
        assert_eq!(age_words(1_700_000_000.0, 0.0), "1970-01-01");
    }
}
