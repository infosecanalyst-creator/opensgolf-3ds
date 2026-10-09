//! Play a Championship in the app: the course chooser (courses saved for championship play), the Pick A Pro chooser (pro
//! files in the championship folders), saving a course or the pro for championship play, and the mode's start. The chooser
//! layout is our own; the rules are in sg_core::championship.

use crate::app::*;
use crate::gfx::Gfx;
use crate::render::Rect;
use crate::ui::{rgb, rgba, Screen as Ui};
use sg_core::championship::{self, ProFile};
use sg_core::economy::Economy;
use std::path::{Path, PathBuf};

/// Extension of our courses saved for championship play (a whole-game save with the club emptied at start).
pub const COURSE_EXT: &str = "sgch";

const LIST: Rect = Rect::new(300.0, 100.0, 460.0, 16.0 * 20.0);
const OK: Rect = Rect::new(560.0, 520.0, 100.0, 34.0);
const CANCEL: Rect = Rect::new(680.0, 520.0, 100.0, 34.0);
const DIFF: Rect = Rect::new(40.0, 520.0, 300.0, 34.0);

#[derive(Clone, Debug, Default)]
pub struct ChampScreen {
    /// 0 choosing the course, 1 choosing the pro.
    pub step: u8,
    pub files: Vec<PathBuf>,
    pub sel: Option<usize>,
    pub course: Option<PathBuf>,
    pub pro: Option<ProFile>,
}

fn list(dir: &Path, ext: &str) -> Vec<PathBuf> {
    let mut v: Vec<PathBuf> = std::fs::read_dir(dir)
        .map(|r| {
            r.filter_map(|e| e.ok().map(|e| e.path())).filter(|p| p.extension().is_some_and(|x| x.eq_ignore_ascii_case(ext))).collect()
        })
        .unwrap_or_default();
    v.sort();
    v
}

impl App {
    /// Where our championship courses and pros are saved: a Championship folder next to the saved game.
    pub fn championship_dir(&self) -> PathBuf {
        self.course_file.parent().map(|p| p.to_path_buf()).unwrap_or_default().join("Championship")
    }

    fn pro_files(&self) -> Vec<PathBuf> {
        let mut v = list(&self.championship_dir(), "pro");
        v.extend(list(&self.game_path("Themes/Championship"), "pro"));
        v
    }

    /// Title menu "Play a Championship".
    pub fn open_championship(&mut self) {
        let files = list(&self.championship_dir(), COURSE_EXT);
        self.champ = Some(ChampScreen { files, ..Default::default() });
        self.screen = Screen::Champ;
        self.hover = -1;
    }

    /// Saves the course for championship play (File menu, command 0x43): refused during a tournament.
    pub fn save_championship_course(&mut self) {
        if self.club.game & sg_core::golfer::game::TOURNAMENT != 0 {
            self.slot_sound(24, self.cam_x, self.cam_z);
            return self.show_toast("Cannot save course during a tournament.");
        }
        let dir = self.championship_dir();
        let _ = std::fs::create_dir_all(&dir);
        let path = dir.join(format!("{}.{COURSE_EXT}", self.course_name));
        match self.save_game(&path) {
            Ok(()) => self.show_toast(&format!("{} saved for championship play.", self.course_name)),
            Err(e) => self.show_toast(&format!("Could not save: {e}")),
        }
    }

    /// Saves the player's pro for championship play.
    pub fn save_championship_pro(&mut self) {
        let dir = self.championship_dir();
        let _ = std::fs::create_dir_all(&dir);
        let p = self.club.pro_file();
        let path = dir.join(format!("{}.pro", p.person.name));
        match std::fs::write(&path, championship::pro_bytes(&p)) {
            Ok(()) => self.show_toast(&format!("{} saved for championship play.", p.person.name)),
            Err(e) => self.show_toast(&format!("Could not save: {e}")),
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
        self.panel = 3;
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

    pub fn champ_click(&mut self, g: &mut Gfx, vx: f32, vy: f32) {
        let Some(mut c) = self.champ.take() else { return };
        if DIFF.has(vx, vy) {
            self.difficulty = (self.difficulty + 1) % 4;
        } else if LIST.has(vx, vy) {
            let i = ((vy - LIST.y) / 16.0) as usize;
            if i < c.files.len() {
                c.sel = Some(i);
                if c.step == 1 {
                    c.pro = std::fs::read(&c.files[i]).ok().and_then(|b| championship::parse_pro(&b));
                }
            }
        } else if CANCEL.has(vx, vy) || vx < 0.0 {
            self.screen = Screen::Menu;
            return;
        } else if OK.has(vx, vy) {
            if let Some(i) = c.sel {
                if c.step == 0 {
                    c.course = Some(c.files[i].clone());
                    c.step = 1;
                    c.sel = None;
                    c.files = self.pro_files();
                } else if let (Some(course), Some(pro)) = (c.course.clone(), c.pro.clone()) {
                    if let Err(e) = self.start_championship(g, &course, &pro) {
                        self.show_toast(&format!("Could not load the course: {e}"));
                        self.screen = Screen::Menu;
                    }
                    return;
                }
            }
        }
        self.champ = Some(c);
    }

    pub fn draw_champ(&mut self, g: &mut Gfx) {
        let Some(c) = self.champ.clone() else { return };
        let s = Ui::new(self.draw_w, self.draw_h);
        self.view = s.view;
        s.image(g, &self.title_base, 0.0, 0.0);
        s.fill(g, 20.0, 20.0, 760.0, 560.0, rgba(0.06, 0.08, 0.25, 0.9));
        let title = if c.step == 0 { "Select Championship Course" } else { "Pick A Pro" };
        s.text(g, LIST.x, 60.0, title, 22.0, rgb(1.0, 1.0, 0.8));
        if c.files.is_empty() {
            let msg = if c.step == 0 {
                "No courses yet. In a game, press F7 to save your course for championship play."
            } else {
                "No pros found. In a game, press F8 to save your pro for championship play."
            };
            s.text(g, LIST.x, LIST.y + 14.0, msg, 13.0, rgb(1.0, 0.8, 0.7));
        }
        for (i, f) in c.files.iter().take(20).enumerate() {
            let y = LIST.y + 16.0 * i as f32;
            if c.sel == Some(i) {
                s.fill(g, LIST.x, y, LIST.w, 16.0, rgba(0.9, 0.75, 0.2, 0.5));
            }
            let name = f.file_stem().map(|n| n.to_string_lossy().into_owned()).unwrap_or_default();
            s.text(g, LIST.x + 4.0, y + 13.0, &name, 14.0, rgb(1.0, 1.0, 1.0));
        }
        if let Some(p) = c.pro.as_ref().filter(|_| c.step == 1) {
            s.text(g, 40.0, 100.0, &format!("{}'s skills", p.person.name), 16.0, rgb(1.0, 1.0, 0.8));
            for k in 0..10 {
                s.text(
                    g,
                    40.0,
                    126.0 + 22.0 * k as f32,
                    &format!("{}: {}%", sg_core::pro::SKILL_NAMES[k], p.skills[k] as i32 * 10),
                    14.0,
                    rgb(1.0, 1.0, 1.0),
                );
            }
        }
        let label = format!("Difficulty: {} (click to change)", crate::render::DIFFICULTY_NAMES[self.difficulty.clamp(0, 3) as usize]);
        for (r, t) in [(DIFF, label.as_str()), (OK, "OK"), (CANCEL, "Cancel")] {
            s.fill(g, r.x, r.y, r.w, r.h, rgba(0.3, 0.4, 0.7, 0.85));
            s.text_centered(g, r.x + r.w / 2.0, r.y + 23.0, t, 14.0, rgb(1.0, 1.0, 1.0));
        }
        g.flush();
    }
}
