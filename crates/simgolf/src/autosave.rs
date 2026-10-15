//! The Autosave interface extra (extras.rs; docs/PLAYING.md "OpenSGolf extras"): every five minutes of play the game is
//! saved to "&Latest.sgs" in Saved Games, which the Load list shows as "autosave (Latest)". Not in the original.

use crate::app::{App, Screen};

/// Seconds of play between autosaves.
pub const INTERVAL: f64 = 300.0;

/// The autosave's file name ('&' marks an autosave in the Load list, files_ui.rs).
pub const FILE: &str = "&Latest.sgs";

impl App {
    /// Each frame, with the clock `t` in seconds: saves the game when the time comes (only while a game is on screen).
    pub fn autosave_tick(&mut self, t: f64) {
        if !crate::extras::autosave() || self.screen != Screen::Play || self.land.is_none() {
            self.autosave_due = 0.0;
            return;
        }
        if self.autosave_due == 0.0 {
            self.autosave_due = t + INTERVAL;
        } else if t >= self.autosave_due {
            self.autosave_due = t + INTERVAL;
            let path = crate::files_ui::saved_games_dir().join(FILE);
            match self.save_game(&path) {
                Ok(()) => println!("autosaved {}", path.display()),
                Err(e) => eprintln!("autosave: {e}"),
            }
        }
    }
}
