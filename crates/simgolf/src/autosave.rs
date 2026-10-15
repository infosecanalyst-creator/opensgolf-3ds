//! The Autosave interface extra (extras.rs; docs/PLAYING.md "OpenSGolf extras"): every five minutes of play the game is
//! saved to "&Latest.sgs" in Saved Games, which the Load list shows as "autosave (Latest)". The two autosaves before it
//! are kept as "&Latest-2.sgs" and "&Latest-3.sgs" (each autosave moves them one along). Not in the original.

use crate::app::{App, Screen};

/// Seconds of play between autosaves.
pub const INTERVAL: f64 = 300.0;

/// The autosave's file name ('&' marks an autosave in the Load list, files_ui.rs).
pub const FILE: &str = "&Latest.sgs";

/// The autosaves kept, newest first: the latest and the two before it.
pub const KEPT: [&str; 3] = [FILE, "&Latest-2.sgs", "&Latest-3.sgs"];

/// Before an autosave: each kept autosave moves one along (the oldest is dropped), so the new one finds its name free.
/// Works through fsutil, so in the browser's store as well. Returns the moves made (from, to).
pub fn rotate(dir: &std::path::Path) -> Vec<(usize, usize)> {
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
    /// Each frame, with the clock `t` in seconds: counts the time spent playing (the course on screen, not the reports,
    /// menus or other screens) and saves the game each time it reaches the interval. Loading or starting a game starts
    /// the count again.
    pub fn autosave_tick(&mut self, t: f64) {
        let dt = if self.autosave_clock > 0.0 { (t - self.autosave_clock).clamp(0.0, 1.0) } else { 0.0 };
        self.autosave_clock = t;
        if !crate::extras::autosave() || self.land.is_none() {
            self.autosave_due = 0.0;
            return;
        }
        if self.screen != Screen::Play {
            return;
        }
        self.autosave_due += dt;
        // SG_AUTOSAVE_SECS=N (tests): every N seconds of play instead
        let every = std::env::var("SG_AUTOSAVE_SECS").ok().and_then(|v| v.parse::<f64>().ok()).unwrap_or(INTERVAL);
        if self.autosave_due >= every {
            self.autosave_due = 0.0;
            let dir = crate::files_ui::saved_games_dir();
            rotate(&dir);
            let path = dir.join(FILE);
            match self.save_game(&path) {
                Ok(()) => println!("autosaved {}", path.display()),
                Err(e) => eprintln!("autosave: {e}"),
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
}
