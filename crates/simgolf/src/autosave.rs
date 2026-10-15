//! The Autosave interface extra (extras.rs; docs/PLAYING.md "OpenSGolf extras"): every five minutes of play the game is
//! saved to "&Latest.sgs" in Saved Games, which the Load list shows as "autosave (Latest)". Not in the original.

use crate::app::{App, Screen};

/// Seconds of play between autosaves.
pub const INTERVAL: f64 = 300.0;

/// The autosave's file name ('&' marks an autosave in the Load list, files_ui.rs).
pub const FILE: &str = "&Latest.sgs";

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
            let path = crate::files_ui::saved_games_dir().join(FILE);
            match self.save_game(&path) {
                Ok(()) => println!("autosaved {}", path.display()),
                Err(e) => eprintln!("autosave: {e}"),
            }
        }
    }
}
