//! The Autosave interface extra (extras.rs; docs/PLAYING.md "OpenSGolf extras"): every five minutes of play the game is
//! saved to "&Latest.sgs" in Saved Games, which the Load list shows as "autosave (Latest)". The two autosaves before it
//! are kept as "&Latest-2.sgs" and "&Latest-3.sgs" (each autosave moves them one along). The game is saved there as well
//! when its window goes to the background (`autosave_on_hide`), so a phone's browser that throws away a page left in the
//! background (iOS does) loses nothing of the game. Not in the original.

use crate::app::{App, Screen};
use std::path::Path;

/// Seconds of play between autosaves.
pub const INTERVAL: f64 = 300.0;

/// The least play (seconds, counted as for `INTERVAL`) since the last autosave, or since the game was started or loaded,
/// before the window going to the background saves the game: a window left and come back to at once has nothing new to
/// keep. The port's own choice.
pub const HIDE_GAP: f64 = 30.0;

/// The autosave's file name ('&' marks an autosave in the Load list, files_ui.rs).
pub const FILE: &str = "&Latest.sgs";

/// The autosaves kept, newest first: the latest and the two before it.
pub const KEPT: [&str; 3] = [FILE, "&Latest-2.sgs", "&Latest-3.sgs"];

/// Before an autosave: each kept autosave moves one along (the oldest is dropped), so the new one finds its name free.
/// Works through fsutil, so in the browser's store as well. Returns the moves made (from, to).
pub fn rotate(dir: &Path) -> Vec<(usize, usize)> {
    use sg_core::fsutil;
    let mut moved = Vec::new();
    for i in (0..KEPT.len() - 1).rev() {
        let (from, to) = (dir.join(KEPT[i]), dir.join(KEPT[i + 1]));
        let Some(data) = fsutil::read_raw(&from) else { continue };
        if fsutil::write_file(&to, &data) {
            fsutil::remove_file(&from);
            moved.push((i, i + 1));
        }
    }
    moved
}

impl App {
    /// The count towards the next autosave starts again (a game started or loaded, an autosave made, the extra off).
    pub fn autosave_restart(&mut self) {
        self.autosave_due = 0.0;
        self.autosave_hidden_at = None;
    }

    /// Each frame, with the clock `t` in seconds: counts the time spent playing (the course on screen, not the reports,
    /// menus or other screens) and saves the game each time it reaches the interval. Loading or starting a game starts
    /// the count again.
    pub fn autosave_tick(&mut self, t: f64) {
        let dt = if self.autosave_clock > 0.0 { (t - self.autosave_clock).clamp(0.0, 1.0) } else { 0.0 };
        self.autosave_clock = t;
        if !crate::extras::autosave() || self.land.is_none() {
            self.autosave_restart();
            return;
        }
        if self.screen != Screen::Play {
            return;
        }
        self.autosave_due += dt;
        // SG_AUTOSAVE_SECS=N (tests): every N seconds of play instead
        let every = std::env::var("SG_AUTOSAVE_SECS").ok().and_then(|v| v.parse::<f64>().ok()).unwrap_or(INTERVAL);
        if self.autosave_due >= every {
            self.autosave_restart();
            self.autosave_to(&crate::files_ui::saved_games_dir(), false);
        }
    }

    /// The window went to the background (main.rs window_minimized_event, which a browser's gl.js sends from inside the
    /// page's visibilitychange and blur events, so the save is in the browser's storage before the page can be frozen or
    /// thrown away): with the Autosave extra on, a game on screen is saved as autosave (Latest) once `HIDE_GAP` of play
    /// has gone by since the last autosave. Not during a tournament, where the quick save is refused too (comfort.rs): the
    /// player can make this save at any moment by leaving the window. The first such save after a five-minute autosave
    /// (or the game's start or load) moves the kept autosaves along, as that one does; the next ones replace it, so leaving
    /// the window often (alt-tabbing on a desktop) never pushes out more than one older autosave between two five-minute
    /// ones.
    pub fn autosave_on_hide(&mut self) {
        self.autosave_on_hide_in(&crate::files_ui::saved_games_dir());
    }

    /// `autosave_on_hide` with the autosaves in `dir`. True when the game was saved.
    fn autosave_on_hide_in(&mut self, dir: &Path) -> bool {
        let tournament = self.club.game & sg_core::golfer::game::TOURNAMENT != 0;
        // only on the course itself, where the five-minute autosave and the quick save are made: not over a report, the
        // year's end or a tournament's results, nor the Load Previous Game list, whose autosaves would move along under it
        if !crate::extras::autosave() || self.land.is_none() || self.screen != Screen::Play || tournament {
            return false;
        }
        if self.autosave_due - self.autosave_hidden_at.unwrap_or(0.0) < HIDE_GAP {
            return false;
        }
        let saved = self.autosave_to(dir, self.autosave_hidden_at.is_some());
        // tried, saved or not: a save that failed has moved the kept autosaves along once already, and the next one only
        // replaces Latest, so failing saves do not wear the older ones away
        self.autosave_hidden_at = Some(self.autosave_due);
        saved
    }

    /// Saves the game as autosave (Latest) in `dir`: over the one there with `replace`, else after the kept autosaves have
    /// moved along. True when it was saved.
    fn autosave_to(&self, dir: &Path, replace: bool) -> bool {
        if !replace {
            rotate(dir);
        }
        let path = dir.join(FILE);
        match self.save_game(&path) {
            Ok(()) => {
                println!("autosaved {}", path.display());
                true
            }
            Err(e) => {
                crate::bugreport::warn(&format!("autosave: {e}"));
                false
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn autosaves_move_along_and_the_oldest_goes() {
        let dir = std::env::temp_dir().join(format!("opensgolf-autosave-test-{}", std::process::id()));
        let _ = std::fs::remove_dir_all(&dir);
        std::fs::create_dir_all(&dir).unwrap();
        let read = |n: &str| std::fs::read(dir.join(n)).ok();
        // nothing to move yet
        assert!(rotate(&dir).is_empty());
        std::fs::write(dir.join(FILE), b"one").unwrap();
        assert_eq!(rotate(&dir), vec![(0, 1)]);
        assert_eq!((read(FILE), read(KEPT[1])), (None, Some(b"one".to_vec())));
        std::fs::write(dir.join(FILE), b"two").unwrap();
        rotate(&dir);
        std::fs::write(dir.join(FILE), b"three").unwrap();
        rotate(&dir);
        std::fs::write(dir.join(FILE), b"four").unwrap();
        assert_eq!(rotate(&dir), vec![(1, 2), (0, 1)]);
        assert_eq!(read(FILE), None);
        assert_eq!(read(KEPT[1]), Some(b"four".to_vec()));
        assert_eq!(read(KEPT[2]), Some(b"three".to_vec()));
        // three kept at most: "one" and "two" are gone
        assert_eq!(std::fs::read_dir(&dir).unwrap().count(), 2);
        let _ = std::fs::remove_dir_all(&dir);
    }

    /// A game under way on the course, with the Autosave extra on.
    fn a_game() -> App {
        crate::extras::set(crate::extras::Settings { autosave: true, ..crate::extras::Settings::ALL_OFF });
        let mut a = App::with_clock(std::path::PathBuf::new(), 4242);
        a.land = Some(sg_core::land::Land::from_terrain(&a.terrain, 0));
        a
    }

    /// An empty folder of its own for a test's autosaves.
    fn folder(name: &str) -> std::path::PathBuf {
        let dir = std::env::temp_dir().join(format!("opensgolf-autosave-{name}-{}", std::process::id()));
        let _ = std::fs::remove_dir_all(&dir);
        std::fs::create_dir_all(&dir).unwrap();
        dir
    }

    #[test]
    fn the_background_autosaves_after_some_play_and_then_replaces_its_own_save() {
        let dir = folder("hide");
        let text = |n: &str| std::fs::read(dir.join(n)).ok().map(|b| String::from_utf8_lossy(&b).into_owned());
        let game = || crate::app::read_save(&dir.join(FILE)).is_ok();
        std::fs::write(dir.join(FILE), "five minutes").unwrap();
        let mut a = a_game();
        let mut hide = |due: f64| {
            a.autosave_due = due;
            a.autosave_on_hide_in(&dir)
        };
        // just started or loaded, or the five-minute autosave just made: nothing new to keep
        assert!(!hide(10.0));
        assert_eq!(text(FILE).as_deref(), Some("five minutes"));
        // half a minute of play later: saved, and the five-minute autosave moves along
        assert!(hide(40.0));
        assert!(game());
        assert_eq!(text(KEPT[1]).as_deref(), Some("five minutes"));
        // away again soon after coming back: nothing
        assert!(!hide(50.0));
        // later: the save replaces its own, and the autosaves before it stay where they are
        std::fs::write(dir.join(FILE), "hidden").unwrap();
        assert!(hide(75.0));
        assert!(game());
        assert_eq!((text(KEPT[1]).as_deref(), text(KEPT[2])), (Some("five minutes"), None));
        // after the next five-minute autosave the first save on going to the background moves them along again
        a.autosave_restart();
        std::fs::write(dir.join(FILE), "five minutes later").unwrap();
        let mut hide = |due: f64| {
            a.autosave_due = due;
            a.autosave_on_hide_in(&dir)
        };
        assert!(!hide(20.0));
        assert!(hide(30.0));
        assert_eq!(text(KEPT[1]).as_deref(), Some("five minutes later"));
        assert_eq!(text(KEPT[2]).as_deref(), Some("five minutes"));
        let _ = std::fs::remove_dir_all(&dir);
    }

    #[test]
    fn the_background_saves_nothing_where_no_autosave_belongs() {
        let dir = folder("never");
        let mut a = a_game();
        a.autosave_due = 100.0;
        // a tournament under way (no quick save either)
        a.club.game |= sg_core::golfer::game::TOURNAMENT;
        assert!(!a.autosave_on_hide_in(&dir));
        a.club.game &= !sg_core::golfer::game::TOURNAMENT;
        // the title menu, with the game left there
        a.screen = Screen::Menu;
        assert!(!a.autosave_on_hide_in(&dir));
        a.screen = Screen::Play;
        // no game
        let land = a.land.take();
        assert!(!a.autosave_on_hide_in(&dir));
        a.land = land;
        // the extra off
        crate::extras::set(crate::extras::Settings::ALL_OFF);
        assert!(!a.autosave_on_hide_in(&dir));
        assert_eq!(std::fs::read_dir(&dir).unwrap().count(), 0);
        // with the extra on, nor over a report or another of the game's screens
        crate::extras::set(crate::extras::Settings { autosave: true, ..crate::extras::Settings::ALL_OFF });
        a.screen = Screen::Finance;
        assert!(!a.autosave_on_hide_in(&dir));
        assert_eq!(std::fs::read_dir(&dir).unwrap().count(), 0);
        // the course
        a.screen = Screen::Play;
        assert!(a.autosave_on_hide_in(&dir));
        assert!(crate::app::read_save(&dir.join(FILE)).is_ok());
        let _ = std::fs::remove_dir_all(&dir);
    }
}
