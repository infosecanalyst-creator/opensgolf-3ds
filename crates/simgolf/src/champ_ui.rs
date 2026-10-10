//! Play a Championship in the app: saving a course or the pro for championship play, and the mode's start. The course
//! chooser and Pick A Pro are the Load Previous Game screen's other modes (files_ui); the rules are in sg_core::championship.

use crate::app::*;
use crate::files_ui::ListKind;
use crate::gfx::Gfx;
use sg_core::championship::{self, ProFile};
use sg_core::economy::Economy;
use std::path::{Path, PathBuf};

/// Extension of our courses saved for championship play (a whole-game save with the club emptied at start).
pub const COURSE_EXT: &str = "sgch";

impl App {
    /// Where our championship courses and pros are saved: a Championship folder next to the saved game.
    pub fn championship_dir(&self) -> PathBuf {
        self.course_file.parent().map(|p| p.to_path_buf()).unwrap_or_default().join("Championship")
    }

    /// Title menu "Play a Championship", after the difficulty box (0x46ddd0): the course chooser, then Pick A Pro. The
    /// difficulty chosen in the box is kept whatever the course was saved with.
    pub fn open_championship(&mut self) {
        self.title.files = None;
        self.open_files(ListKind::Course, false);
    }

    /// Saves the course for championship play (File menu, command 0x43): refused during a tournament.
    pub fn save_championship_course(&mut self) {
        if self.club.game & sg_core::golfer::game::TOURNAMENT != 0 {
            self.ui_sound(24);
            return self.show_toast("Cannot save course during a tournament.");
        }
        let dir = self.championship_dir();
        let path = dir.join(format!("{}.{COURSE_EXT}", self.course_name));
        match self.save_game(&path) {
            Ok(()) => self.show_toast(&format!("{} saved for championship play.", self.course_name)),
            Err(e) => self.show_toast(&format!("Could not save: {e}")),
        }
    }

    /// Saves the player's pro for championship play.
    pub fn save_championship_pro(&mut self) {
        let dir = self.championship_dir();
        let p = self.club.pro_file();
        let path = dir.join(format!("{}.pro", p.person.name));
        if sg_core::fsutil::write_file(&path, &championship::pro_bytes(&p)) {
            self.show_toast(&format!("{} saved for championship play.", p.person.name));
        } else {
            self.show_toast("Could not save the pro.");
        }
    }

    /// Starts the championship (0x46ddd0): the course is loaded, cash $100,000, June of the second year, only the
    /// championship flag, the menu's difficulty, and the field goes out at once.
    pub fn start_championship(&mut self, g: &mut Gfx, course: &Path, pro: &ProFile) -> Result<(), String> {
        let diff = self.difficulty;
        self.load_game(g, course)?;
        self.difficulty = diff;
        self.club.difficulty = diff;
        self.econ.cash = 1000.0 * Economy::UNIT;
        self.econ.sandbox = false;
        self.game_tick = 0x2c00;
        self.econ.tick = 0x2c00;
        self.club.tick = 0x2c00;
        self.club.start_championship(&self.course, &mut self.exe_rng, pro, 1000);
        self.register_portraits(g);
        self.show_player_panel();
        self.edit = false;
        self.screen = Screen::Play;
        println!(
            "championship: {} with {} on {} ({} players)",
            championship::EVENTS[diff.clamp(0, 3) as usize],
            pro.person.name,
            self.course_name,
            self.club.leaderboard().0.len()
        );
        Ok(())
    }
}
