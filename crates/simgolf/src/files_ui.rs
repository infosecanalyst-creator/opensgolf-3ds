//! The title's side screens, rebuilt on the game's own art (docs/DECODE_TITLE2.md 1, 2, 4 and 6, docs/DECODE_PICKAPRO.md):
//! Load Previous Game (also the championship course chooser and Pick A Pro, which share its layout and hit circles), Select a
//! Theme Pack and the credits; and the in-game Save dialog (docs/DECODE_MENUS.md 4).
//!
//! Positions, colours, hit circles and rules follow the decodes. The port's own parts: saves are this port's files
//! (`<save dir>/Saved Games/<name>.sgs`, championship courses and pros under `<save dir>/Championship`), so a row's preview reads
//! the port's save instead of the exe's course block; the left panel's oval shows the club emblem of the save's property (the
//! property to emblem table is DERIVED from the art); a mid-game visit does not need the exe's "While Browsing" save because
//! the preview never replaces the game in memory; only the port's own files can be deleted (the exe also deletes files on the
//! disc). Text sizes are ours (the exe's font object is hidden in the decompile).
//!
//! PLACEHOLDER: the OK button's tooltip word (a shared literal under five characters; "Load" here, as the task notes suggest),
//! the delete box's second option ("No, never mind."), the autosave label's punctuation, the Pick A Pro portrait when a pro
//! file carries no picture (a plain disc: the port cannot render faces from the face sprites yet), and which of the three
//! stored portraits Pick A Pro shows (the first).

use crate::app::*;
use crate::gfx::Gfx;
use crate::render::oct_dist;
use crate::ui::{load_pcx, load_pcx_alpha, money, rgb, rgba, text_width, Image, Screen as Ui};
use miniquad::KeyCode;
use sg_core::assets::Rgba;
use sg_core::championship::ProFile;
use std::path::{Path, PathBuf};

/// What the list screen lists.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub enum ListKind {
    /// Load Previous Game: the saved games.
    #[default]
    Load,
    /// Play a Championship, first step: courses saved for championship play ("Select Championship Course").
    Course,
    /// Play a Championship, second step: Pick A Pro.
    Pro,
}

/// The left panel of a saved game or course: what the exe reads from the previewed save.
#[derive(Clone, Debug, Default)]
pub struct SaveInfo {
    /// Open holes: number, par, yards.
    pub holes: Vec<(usize, i32, i32)>,
    pub par: i32,
    pub yards: i32,
    pub cash: i64,
    pub fun: i32,
    /// Hundredths of a stroke.
    pub len: i32,
    pub acc: i32,
    pub img: i32,
    pub theme: String,
    pub designer: String,
    pub record: i32,
    pub property: Option<usize>,
}

impl SaveInfo {
    pub fn of(s: &SaveGame) -> SaveInfo {
        let holes: Vec<(usize, i32, i32)> =
            (1..19).filter_map(|h| s.club.holes.get(h).filter(|r| r.par != 0).map(|r| (h, r.par, r.length))).collect();
        let r = &s.club.ratings;
        let theme = if s.theme_pack.is_empty() { "Standard".to_string() } else { s.theme_pack.replace('_', " ") };
        SaveInfo {
            par: holes.iter().map(|h| h.1).sum(),
            yards: holes.iter().map(|h| h.2).sum(),
            holes,
            cash: s.econ.cash as i64,
            fun: r.fun,
            len: r.len,
            acc: r.acc,
            img: r.img,
            theme,
            designer: s.club.roster.first().map(|p| p.name.clone()).unwrap_or_default(),
            record: s.club.top_rounds[0],
            property: s.land.as_ref().map(|l| l.slot.property),
        }
    }
}

#[derive(Default)]
pub struct FileList {
    pub kind: ListKind,
    pub files: Vec<PathBuf>,
    pub scroll: usize,
    pub sel: Option<usize>,
    /// Opened from a game in progress (System Functions, Shift+L): Cancel goes back to it.
    pub from_game: bool,
    pub info: Option<SaveInfo>,
    pub pro: Option<ProFile>,
    portrait_px: Option<Rgba>,
    portrait: Option<Image>,
    /// The delete box, with its highlighted option.
    pub confirm: Option<usize>,
    /// 1 "Loading..." is on screen, 2 the load runs on the next frame.
    pub loading: u8,
    /// Play a Championship: the course chosen in the first step.
    pub course: Option<PathBuf>,
}

pub struct Credits {
    pub lines: Vec<String>,
    /// Clock time of the first frame (negative until drawn), and seconds already run (the test hook starts part way).
    start: f64,
    preroll: f64,
    music: i32,
}

/// The art of the title's side screens, from the disc's Interface folder and game folder.
#[derive(Clone, Copy, Debug, Default)]
pub struct TitleArt {
    pub load: Image,
    pub load_mo: Image,
    pub pickapro: Image,
    pub themes: Image,
    pub themes_mo: Image,
    pub credits: Image,
    pub bink: Image,
    /// parklink.pcx and tropdesert.pcx (with their alpha pictures): the sixteen club emblems.
    pub emblems: [Image; 2],
}

impl TitleArt {
    pub fn load(g: &mut Gfx, app: &App) -> TitleArt {
        let i = |rel: &str| app.game_path(&format!("Interface/{rel}"));
        let alpha = |g: &mut Gfx, n: &str| load_pcx_alpha(g, &i(&format!("{n}.pcx")), &i(&format!("{n}_A.pcx"))).unwrap_or_default();
        TitleArt {
            load: load_pcx(g, &i("Title_LoadGame.pcx"), false, None).unwrap_or_default(),
            load_mo: load_pcx(g, &i("Title_LoadGame_MO.pcx"), true, None).unwrap_or_default(),
            pickapro: load_pcx(g, &i("Title_Pickapro.pcx"), false, None).unwrap_or_default(),
            themes: load_pcx(g, &i("Title_ThemePacks.pcx"), false, None).unwrap_or_default(),
            themes_mo: load_pcx(g, &i("Title_ThemePacks_MO.pcx"), true, None).unwrap_or_default(),
            credits: load_pcx(g, &app.game_path("creditsbckgrd.pcx"), false, None).unwrap_or_default(),
            bink: load_pcx(g, &app.game_path("bink64.pcx"), false, None).unwrap_or_default(),
            emblems: [alpha(g, "parklink"), alpha(g, "tropdesert")],
        }
    }
}

/// The title's side screens and the Save dialog.
#[derive(Default)]
pub struct TitleScreens {
    pub files: Option<FileList>,
    /// Theme Packs: the folders under Themes and which of the five kinds of file each holds.
    pub themes: Vec<(String, [bool; 5])>,
    pub credits: Option<Credits>,
    /// The name being typed in the Save dialog, while it is open.
    pub save_name: Option<String>,
    /// Play a Championship was chosen: the difficulty box leads to the course chooser instead of the property chooser.
    pub champ_pending: bool,
    pub art: TitleArt,
    hover_last: i32,
    hover_frames: u32,
}

impl TitleScreens {
    pub fn over_game(&self) -> bool {
        self.files.as_ref().is_some_and(|f| f.from_game)
    }
}

/// Where the Save dialog writes and Load Previous Game reads.
pub fn saved_games_dir() -> PathBuf {
    save_dir().join("Saved Games")
}

/// 15-bit exe colour to RGBA.
fn c15(v: u32) -> [f32; 4] {
    rgb(((v >> 10) & 31) as f32 / 31.0, ((v >> 5) & 31) as f32 / 31.0, (v & 31) as f32 / 31.0)
}
const BLACK: [f32; 4] = [0.0, 0.0, 0.0, 1.0];
const WHITE: [f32; 4] = [1.0, 1.0, 1.0, 1.0];

/// The exe's text calls take the top of the text; ours draw from the baseline.
fn top(s: &Ui, g: &mut Gfx, x: f32, y: f32, t: &str, size: f32, c: [f32; 4]) {
    s.text(g, x, y + size * 0.82, t, size, c);
}
fn top_c(s: &Ui, g: &mut Gfx, x: f32, y: f32, t: &str, size: f32, c: [f32; 4]) {
    s.text_centered(g, x, y + size * 0.82, t, size, c);
}

/// A row's name: the file name without its extension; names starting with '&' are autosaves.
pub fn row_name(p: &Path) -> String {
    let n = p.file_stem().map(|n| n.to_string_lossy().into_owned()).unwrap_or_default();
    match n.strip_prefix('&') {
        Some(rest) => format!("autosave ({rest})"),
        None => n,
    }
}

/// Files of a folder with an extension, without the exe's skipped names, sorted without regard to case.
fn list(dir: &Path, ext: &str) -> Vec<PathBuf> {
    let mut v: Vec<PathBuf> = sg_core::fsutil::list_dir(dir)
        .into_iter()
        .filter(|p| p.extension().is_some_and(|x| x.eq_ignore_ascii_case(ext)))
        .filter(|p| p.file_name().map(|n| n.to_string_lossy()).is_some_and(|n| !n.contains("Shadow") && !n.contains("While Browsing")))
        .collect();
    v.sort_by_key(|p| p.file_name().map(|n| n.to_string_lossy().to_lowercase()).unwrap_or_default());
    v
}

// The list screen's buttons (index = the exe's button number): centre, radius, the art's cut (top-left), its size, tooltip.
const BUTTONS: [(f32, f32, f32, f32, f32, f32, f32); 5] = [
    (633.0, 555.0, 26.0, 600.0, 520.0, 70.0, 70.0),
    (769.0, 556.0, 24.0, 740.0, 530.0, 60.0, 60.0),
    (784.0, 120.0, 20.0, 778.0, 100.0, 22.0, 40.0),
    (784.0, 400.0, 20.0, 778.0, 380.0, 22.0, 40.0),
    (704.0, 555.0, 23.0, 670.0, 520.0, 70.0, 70.0),
];
const OK_TIP: &str = "Load";
const TIPS: [&str; 5] = [OK_TIP, "Cancel", "", "", "Delete"];
const ROWS: usize = 16;

fn list_hit(vx: f32, vy: f32) -> i32 {
    BUTTONS.iter().position(|b| oct_dist(vx - b.0, vy - b.1) < b.2).map(|i| i as i32).unwrap_or(-1)
}

const THEME_HEADS: [&str; 5] = ["Stories", "Characters", "Celebrities", "Pro Golfers", "Courses"];
fn theme_hit(vx: f32, vy: f32) -> i32 {
    if oct_dist(vx - 615.0, vy - 555.0) < 26.0 {
        0
    } else if oct_dist(vx - 749.0, vy - 556.0) < 24.0 {
        1
    } else {
        -1
    }
}

/// Pick A Pro's ten skill rows (docs/DECODE_PICKAPRO.md 1).
const PRO_SKILLS: [&str; 10] = [
    "Power Hitter",
    "Long Driver",
    "Accurate Driver",
    "Accurate Irons",
    "Accurate Putter",
    "Draw Shot (R to L)",
    "Fade Shot (L to R)",
    "High Backspin Shot",
    "Recovery Skills",
    "Luck",
];

/// Club emblem of each property: (sheet 0 parklink / 1 tropdesert, cut). DERIVED from the art and real screenshots.
const EMBLEM: [(usize, usize); 16] =
    [(0, 1), (1, 4), (0, 3), (1, 3), (1, 0), (1, 7), (1, 6), (0, 0), (0, 7), (0, 2), (0, 6), (0, 4), (0, 5), (1, 2), (1, 5), (1, 1)];

const CONFIRM_TOP: f32 = 100.0;
const CONFIRM_PITCH: f32 = 24.0;

/// The delete box (the generic yes/no popup at (400, 100)): its lines and rectangle.
fn confirm_box(name: &str) -> ([String; 3], (f32, f32, f32, f32)) {
    let lines = [format!("Are you sure you want to delete {name}?"), " Yes, delete this file.".into(), " No, never mind.".into()];
    let w = lines.iter().map(|l| text_width(l.trim(), 15.0)).fold(0.0, f32::max) + 49.0 + 30.0;
    let h = (lines.len() as f32 * 3.0 + 3.0) * 8.0;
    (lines, (400.0 - w / 2.0, CONFIRM_TOP, w, h))
}

impl App {
    /// Opens Load Previous Game (kind Load), or a championship chooser.
    pub fn open_files(&mut self, kind: ListKind, from_game: bool) {
        let files = match kind {
            ListKind::Load => {
                let mut v = list(&saved_games_dir(), "sgs");
                // the single save of earlier versions of the port
                if sg_core::fsutil::exists(self.game_file()) {
                    v.push(self.game_file());
                }
                v
            }
            ListKind::Course => list(&self.championship_dir(), crate::champ_ui::COURSE_EXT),
            ListKind::Pro => {
                let mut v = list(&self.championship_dir(), "pro");
                v.extend(list(&self.game_path("Themes/Championship"), "pro"));
                v
            }
        };
        let course = self.title.files.take().and_then(|f| f.course);
        self.title.files = Some(FileList { kind, files, from_game, course, ..Default::default() });
        self.screen = Screen::Files;
        self.hover = -1;
    }

    /// Select a Theme Pack (0x4725b0): the folders under Themes, without names holding a dot and without Championship.
    pub fn open_themes(&mut self) {
        let root = self.game_path("Themes");
        let mut v: Vec<(String, [bool; 5])> = sg_core::fsutil::list_dir(&root)
            .into_iter()
            .filter(|p| sg_core::fsutil::is_dir(p))
            .filter_map(|p| {
                let n = p.file_name()?.to_string_lossy().into_owned();
                if n.contains('.') || n.eq_ignore_ascii_case("Championship") {
                    return None;
                }
                let files: Vec<String> = sg_core::fsutil::list_dir(&p)
                    .iter()
                    .filter_map(|f| f.file_name().map(|n| n.to_string_lossy().to_lowercase()))
                    .collect();
                let ext = |e: &str| files.iter().any(|f| f.ends_with(e));
                let has = |n: &str| files.iter().any(|f| f == n);
                Some((n, [ext(".txt"), ext(".chr"), has("celebrities.dta"), has("progolfers.dta"), ext(".cse")]))
            })
            .collect();
        v.sort_by_key(|t| t.0.to_lowercase());
        self.title.themes = v;
        self.screen = Screen::Themes;
        self.hover = -1;
    }

    /// The credits (0x44b9c0): the lines of credits.txt after "#CREDITS" up to the next '#' line.
    pub fn open_credits(&mut self) {
        let text = sg_core::fsutil::read_file(self.game_path("credits.txt")).map(|d| sg_core::formats::latin1(&d)).unwrap_or_default();
        let lines: Vec<String> = text
            .lines()
            .map(|l| l.trim_end().to_string())
            .skip_while(|l| l != "#CREDITS")
            .skip(1)
            .take_while(|l| !l.starts_with('#'))
            .take(512)
            .collect();
        if let Some(m) = self.mixer.clone() {
            if self.title_music >= 0 {
                m.fade_out(self.title_music, 500);
            }
        }
        // DERIVED: credit.wav is registered by the exe but its trigger is not found; it plays with the screen
        let music = self.snd("Music/Credit.wav", 0.8, false);
        self.title.credits = Some(Credits { lines, start: -1.0, preroll: 0.0, music });
        self.screen = Screen::Credits;
        self.hover = -1;
    }

    fn close_credits(&mut self) {
        if let Some(c) = self.title.credits.take() {
            if let (Some(m), true) = (&self.mixer, c.music >= 0) {
                m.fade_out(c.music, 500);
            }
        }
        self.screen = Screen::Menu;
        self.hover = -1;
    }

    /// The --screen test hook for these screens (`in_game`: over a game in progress). Returns false for other names.
    pub fn test_title_screen(&mut self, name: &str, in_game: bool) -> bool {
        match name {
            "load" => self.open_files(ListKind::Load, in_game),
            "champ" => self.open_files(ListKind::Course, false),
            "pickapro" => self.open_files(ListKind::Pro, false),
            "themes" => self.open_themes(),
            "credits" => {
                self.open_credits();
                if let Some(c) = self.title.credits.as_mut() {
                    c.preroll = 12.0;
                }
            }
            "save" => self.open_save(),
            _ => return false,
        }
        // SG_HOVER, and SG_SELECT picks a row, for stills of the hover art, tooltips and the left panel
        if let Some(h) = std::env::var("SG_HOVER").ok().and_then(|v| v.parse().ok()) {
            self.hover = h;
            self.title.hover_last = h;
            self.title.hover_frames = 30;
        }
        if let Some(r) = std::env::var("SG_SELECT").ok().and_then(|v| v.parse::<usize>().ok()) {
            self.select_row(r);
        }
        // SG_CONFIRM opens the delete box over the selected row
        if let Some(f) = self.title.files.as_mut().filter(|f| f.sel.is_some() && std::env::var_os("SG_CONFIRM").is_some()) {
            f.confirm = Some(0);
        }
        true
    }

    fn select_row(&mut self, row: usize) {
        let Some(f) = self.title.files.as_mut() else { return };
        if row >= f.files.len() {
            f.sel = None;
            f.info = None;
            f.pro = None;
            return;
        }
        f.sel = Some(row);
        let path = f.files[row].clone();
        match f.kind {
            ListKind::Load | ListKind::Course => f.info = read_save(&path).ok().map(|s| SaveInfo::of(&s)),
            ListKind::Pro => {
                let bytes = sg_core::fsutil::read_file(&path);
                f.pro = bytes.as_deref().and_then(sg_core::championship::parse_pro);
                f.portrait_px = bytes.and_then(|b| sg_core::formats::parse_character(&b).ok()).and_then(|c| c.portraits.into_iter().next());
                // the portraits' background is the art's magenta key
                if let Some(px) = f.portrait_px.as_mut() {
                    for p in px.px.as_chunks_mut::<4>().0 {
                        if p[0] == 255 && p[1] == 0 && p[2] == 255 {
                            p[3] = 0;
                        }
                    }
                }
                f.portrait = None;
            }
        }
    }

    fn files_cancel(&mut self) {
        let from_game = self.title.over_game();
        self.title.files = None;
        self.screen = if from_game { Screen::Play } else { Screen::Menu };
        self.hover = -1;
    }

    /// The OK button's work, a frame after "Loading..." went up.
    fn files_ok(&mut self, g: &mut Gfx) {
        let Some(f) = self.title.files.as_mut() else { return };
        let Some(path) = f.sel.and_then(|i| f.files.get(i)).cloned() else { return self.files_cancel() };
        let (kind, course, pro) = (f.kind, f.course.clone(), f.pro.clone());
        match kind {
            ListKind::Load => match self.load_game(g, &path) {
                Ok(()) => {
                    println!("loaded game {}", path.display());
                    self.title.files = None;
                }
                Err(e) => {
                    if let Some(f) = self.title.files.as_mut() {
                        f.loading = 0;
                    }
                    self.show_toast(&format!("Could not load the saved game: {e}"));
                }
            },
            ListKind::Course => {
                if let Some(f) = self.title.files.as_mut() {
                    f.course = Some(path);
                }
                self.open_files(ListKind::Pro, false);
            }
            ListKind::Pro => {
                self.title.files = None;
                self.screen = Screen::Menu;
                if let (Some(course), Some(pro)) = (course, pro) {
                    if let Err(e) = self.start_championship(g, &course, &pro) {
                        self.show_toast(&format!("Could not load the course: {e}"));
                        self.screen = Screen::Menu;
                    }
                }
            }
        }
    }

    /// Pointer over the side screens: hover lights the buttons, a click acts.
    pub fn title_pointer(&mut self, vx: f32, vy: f32, click: bool) {
        match self.screen {
            Screen::Files => self.files_pointer(vx, vy, click),
            Screen::Themes => self.themes_pointer(vx, vy, click),
            Screen::Credits if click => self.close_credits(),
            _ => {}
        }
    }

    pub fn title_key(&mut self, k: KeyCode) {
        match self.screen {
            Screen::Files => {
                let Some(f) = self.title.files.as_mut() else { return };
                if f.loading != 0 {
                    return;
                }
                if let Some(o) = f.confirm.as_mut() {
                    match k {
                        KeyCode::Up | KeyCode::Down => *o ^= 1,
                        KeyCode::Enter | KeyCode::KpEnter | KeyCode::Space => {
                            let yes = *o == 0;
                            self.confirm_done(yes);
                        }
                        KeyCode::Escape => f.confirm = None,
                        _ => {}
                    }
                } else if k == KeyCode::Escape {
                    self.files_cancel();
                }
            }
            // a key leaves Theme Packs with the Standard pack
            Screen::Themes => {
                self.theme_pack = 0;
                self.screen = Screen::Menu;
                self.hover = -1;
            }
            Screen::Credits => self.close_credits(),
            _ => {}
        }
    }

    fn confirm_done(&mut self, yes: bool) {
        let Some(f) = self.title.files.as_mut() else { return };
        f.confirm = None;
        let (kind, from_game, course) = (f.kind, f.from_game, f.course.clone());
        let Some(path) = f.sel.and_then(|i| f.files.get(i)).cloned() else { return };
        if !yes {
            return;
        }
        // only the port's own files: the original's disc files are left alone
        let ours = path.parent().is_some_and(|d| d == saved_games_dir() || d == self.championship_dir()) || path == self.game_file();
        if !ours || !sg_core::fsutil::remove_file(&path) {
            self.ui_sound(0x18);
            return;
        }
        self.open_files(kind, from_game);
        if let Some(f) = self.title.files.as_mut() {
            f.course = course;
        }
    }

    fn files_pointer(&mut self, vx: f32, vy: f32, click: bool) {
        let Some(f) = self.title.files.as_mut() else { return };
        if f.loading != 0 {
            return;
        }
        if let Some(o) = f.confirm.as_mut() {
            let name = f.sel.and_then(|i| f.files.get(i)).map(|p| row_name(p)).unwrap_or_default();
            let (_, (x, y, w, _)) = confirm_box(&name);
            let row = |k: usize| y + 26.0 + CONFIRM_PITCH * (k + 1) as f32;
            let hit = (0..2).find(|&k| vx >= x && vx < x + w && vy >= row(k) - 17.0 && vy < row(k) - 17.0 + CONFIRM_PITCH);
            if let Some(k) = hit {
                *o = k;
            }
            if click {
                match hit {
                    Some(k) => self.confirm_done(k == 0),
                    None => f.confirm = None,
                }
            }
            return;
        }
        let hit = list_hit(vx, vy);
        self.hover = hit;
        if !click {
            return;
        }
        let n = f.files.len();
        match hit {
            0 if f.sel.is_some() => {
                f.loading = 1;
            }
            0 | 1 => self.files_cancel(),
            2 => f.scroll = f.scroll.saturating_sub(4),
            3 => f.scroll = (f.scroll + 4).min(n.saturating_sub(ROWS)),
            4 if f.sel.is_some() => f.confirm = Some(0),
            4 => {}
            _ if vx > 320.0 && vy > 115.0 => {
                let row = ((vy - 116.0) / 16.0) as usize + f.scroll;
                self.select_row(row);
            }
            _ => {}
        }
    }

    fn themes_pointer(&mut self, vx: f32, vy: f32, click: bool) {
        let hit = theme_hit(vx, vy);
        self.hover = hit;
        if !click {
            return;
        }
        match hit {
            0 => self.screen = Screen::Menu,
            1 => {
                self.theme_pack = 0;
                self.screen = Screen::Menu;
            }
            _ => {
                let row = ((vy - 91.0) / 32.0).floor() as i32;
                if vy >= 91.0 && vx > 72.0 && vx < 752.0 && row >= 0 {
                    if let Some((folder, _)) = self.title.themes.get(row as usize) {
                        let label = folder.replace('_', " ");
                        match crate::render::THEME_PACKS.iter().position(|p| p.eq_ignore_ascii_case(&label)) {
                            Some(k) => self.theme_pack = k,
                            None => self.ui_sound(0x18),
                        }
                    }
                }
            }
        }
        if self.screen == Screen::Menu {
            self.hover = -1;
        }
    }

    /// Draws Load Previous Game, Theme Packs or the credits.
    pub fn draw_title_screen(&mut self, g: &mut Gfx) {
        let s = Ui::new(self.draw_w, self.draw_h);
        self.view = s.view;
        if self.hover == self.title.hover_last {
            self.title.hover_frames += 1;
        } else {
            self.title.hover_last = self.hover;
            self.title.hover_frames = 0;
        }
        match self.screen {
            Screen::Files => self.draw_files(g, &s),
            Screen::Themes => self.draw_themes(g, &s),
            Screen::Credits => self.draw_credits(g, &s),
            _ => {}
        }
        g.flush();
    }

    fn draw_files(&mut self, g: &mut Gfx, s: &Ui) {
        let Some(mut f) = self.title.files.take() else { return };
        if let Some(px) = f.portrait_px.take() {
            if let Some(old) = f.portrait.and_then(|i| i.tex) {
                g.ctx.delete_texture(old);
            }
            f.portrait = Some(Image { tex: Some(g.texture(&px, false)), w: px.w as f32, h: px.h as f32 });
        }
        let art = self.title.art;
        let pro = f.kind == ListKind::Pro;
        s.image(g, if pro { &art.pickapro } else { &art.load }, 0.0, 0.0);
        let sel = f.sel.filter(|&i| i < f.files.len());
        if sel.is_none() {
            // OK and Delete greyed
            s.image_part(g, &art.load_mo, 600.0, 520.0, 600.0, 435.0, 70.0, 70.0);
            s.image_part(g, &art.load_mo, 670.0, 520.0, 670.0, 435.0, 70.0, 70.0);
        }
        if let Some(b) = BUTTONS.get(self.hover.max(0) as usize).filter(|_| self.hover >= 0 && f.confirm.is_none()) {
            if sel.is_some() || !matches!(self.hover, 0 | 4) {
                s.image_part(g, &art.load_mo, b.3, b.4, b.3, b.4, b.5, b.6);
            }
        }
        let title = match f.kind {
            ListKind::Load => "Load Previous Game",
            ListKind::Course => "Select Championship Course",
            ListKind::Pro => "Pick A Pro",
        };
        top_c(s, g, 504.0, 42.0, title, 20.0, BLACK);
        let n = f.files.len();
        for r in 0..ROWS.min(n.saturating_sub(f.scroll)) {
            let i = f.scroll + r;
            let y = 116.0 + 16.0 * r as f32;
            let on = sel == Some(i);
            if on {
                s.fill(g, 310.0, y - 1.0, 456.0, 15.0, c15(if pro { 0x1284 } else { 0x7b20 }));
            }
            top(s, g, 320.0, y, &row_name(&f.files[i]), 13.0, if on && pro { WHITE } else { BLACK });
        }
        if n > f.scroll + ROWS {
            top(s, g, 320.0, 372.0, "(more...)", 13.0, BLACK);
        }
        if n == 0 && f.kind == ListKind::Course {
            // the port's hint: no course ships with the game
            top(s, g, 320.0, 116.0, "No course saved for championship play yet (System Functions in a game).", 13.0, c15(0x4210));
        }
        if n > 0 {
            let h = (16.0 * 228.0 / n as f32).clamp(4.0, 228.0);
            s.fill(g, 781.0, 147.0 + f.scroll as f32 * 228.0 / n as f32, 6.0, h, WHITE);
        }
        if sel.is_some() {
            if pro {
                self.draw_pro_panel(g, s, &f);
            } else if let Some(info) = &f.info {
                self.draw_save_panel(g, s, info);
            }
        }
        // a button's word after the pointer rests on it for 30 frames
        if self.title.hover_frames >= 30 && f.confirm.is_none() {
            if let (Some(b), Some(tip)) = (BUTTONS.get(self.hover.max(0) as usize), TIPS.get(self.hover.max(0) as usize)) {
                let tip = if pro && self.hover == 0 { "OK" } else { tip };
                if self.hover >= 0 && !tip.is_empty() && (sel.is_some() || !matches!(self.hover, 0 | 4)) {
                    s.fill(g, b.3 + 6.0, b.4 + 14.0, 48.0, 16.0, rgba(0.08, 0.08, 0.12, 0.9));
                    top_c(s, g, b.3 + 30.0, b.4 + 16.0, tip, 11.0, WHITE);
                }
            }
        }
        if f.loading != 0 {
            if pro {
                top(s, g, 388.0, 472.0, "Loading...", 14.0, WHITE);
            } else {
                top(s, g, 550.0, 456.0, "Loading...", 14.0, c15(0x7b20));
            }
        }
        if let (Some(o), Some(i)) = (f.confirm, sel) {
            let (lines, (x, y, w, h)) = confirm_box(&row_name(&f.files[i]));
            s.fill(g, x - 3.0, y - 3.0, w + 6.0, h + 6.0, rgba(0.55, 0.55, 0.85, 0.95));
            s.fill(g, x, y, w, h, rgba(0.92, 0.92, 1.0, 0.97));
            for (k, l) in lines.iter().enumerate() {
                let ly = y + 26.0 + CONFIRM_PITCH * k as f32;
                if k == 0 {
                    s.text_centered(g, x + w / 2.0, ly, l, 16.0, rgb(0.15, 0.1, 0.4));
                } else {
                    if k - 1 == o {
                        s.fill(g, x + 8.0, ly - 17.0, w - 16.0, CONFIRM_PITCH - 2.0, rgba(0.98, 0.85, 0.2, 0.9));
                    }
                    s.text(g, x + 36.0, ly, l.trim(), 15.0, rgb(0.08, 0.08, 0.25));
                }
            }
        }
        let go = f.loading == 2;
        if f.loading == 1 {
            f.loading = 2;
        }
        self.title.files = Some(f);
        if go {
            self.files_ok(g);
        }
    }

    /// Load Previous Game's left panel (docs/DECODE_PICKAPRO.md 3).
    fn draw_save_panel(&self, g: &mut Gfx, s: &Ui, i: &SaveInfo) {
        let art = &self.title.art;
        if let Some(&(sheet, cut)) = i.property.and_then(|p| EMBLEM.get(p)) {
            let sy = if sheet == 0 { 396.0 } else { 481.0 };
            s.image_part(g, &art.emblems[sheet], 84.0, 32.0, 4.0 + 88.0 * cut as f32, sy, 80.0, 80.0);
        }
        let (grey, label, fun, skill) = (c15(0x4210), c15(0x2108), c15(0x1284), c15(0x0210));
        const X: [f32; 3] = [66.0, 125.0, 184.0];
        let head = ["Holes", "Par", "Yards"];
        let vals = [i.holes.len().to_string(), i.par.to_string(), i.yards.to_string()];
        for k in 0..3 {
            top_c(s, g, X[k], 124.0, head[k], 12.0, grey);
            top_c(s, g, X[k], 146.0, &vals[k], 13.0, BLACK);
        }
        top_c(s, g, 75.0, 169.0, "Cash", 12.0, label);
        top_c(s, g, 161.0, 169.0, &money(i.cash), 13.0, BLACK);
        let hund = |v: i32| format!("{}{}.{:02}", if v < 0 { "-" } else { "" }, v.abs() / 100, v.abs() % 100);
        let rows = [
            ("Fun Rating", i.fun.to_string(), fun),
            ("Length Skill", hund(i.len), skill),
            ("Accuracy Skill", hund(i.acc), skill),
            ("Imagination", hund(i.img), skill),
        ];
        for (k, (l, v, c)) in rows.iter().enumerate() {
            let y = 192.0 + 18.0 * k as f32;
            top(s, g, 48.0, y, l, 12.0, label);
            top_c(s, g, 184.0, y, v, 13.0, *c);
        }
        for (k, h) in ["Hole", "Par", "Yards"].iter().enumerate() {
            top_c(s, g, X[k], 270.0, h, 12.0, grey);
        }
        for (r, &(h, par, yards)) in i.holes.iter().enumerate() {
            let y = 286.0 + 17.0 * r as f32;
            for (k, v) in [h as i32, par, yards].iter().enumerate() {
                top_c(s, g, X[k], y, &v.to_string(), 13.0, BLACK);
            }
        }
        top(s, g, 388.0, 442.0, &format!(" (Theme: {})", i.theme), 13.0, WHITE);
        top(s, g, 388.0, 458.0, &format!("Designed by {}", i.designer), 13.0, WHITE);
        if i.record > 0 {
            // the port keeps the best rounds but not who played them
            top(s, g, 388.0, 474.0, &format!("Course Record: {}", i.record), 13.0, WHITE);
        }
    }

    /// Pick A Pro's left panel (docs/DECODE_PICKAPRO.md 1).
    fn draw_pro_panel(&self, g: &mut Gfx, s: &Ui, f: &FileList) {
        let Some(p) = &f.pro else { return };
        match f.portrait {
            Some(im) => s.image(g, &im, 69.0, 61.0),
            None => {
                // PLACEHOLDER: no picture in the file and no face renderer yet; the art's golf ball shows through
                top_c(s, g, 139.0, 124.0, &p.person.name, 14.0, c15(0x4210));
            }
        }
        top(s, g, 56.0, 255.0, &format!("{}'s skills", p.person.name), 13.0, BLACK);
        let teal = c15(0x0210);
        for (k, name) in PRO_SKILLS.iter().enumerate() {
            let y = 279.0 + 24.0 * k as f32;
            top(s, g, 94.0, y, name, 12.0, teal);
            // "+" then skill*10 then "%"; the interface font has no plus, so it is drawn as two bars
            let t = format!("{}%", p.skills[k] as i32 * 10);
            let x0 = 59.0 - (text_width(&t, 12.0) + 9.0) / 2.0;
            s.fill(g, x0, y + 5.0, 7.0, 2.0, teal);
            s.fill(g, x0 + 2.5, y + 2.5, 2.0, 7.0, teal);
            top(s, g, x0 + 9.0, y, &t, 12.0, teal);
        }
        top(s, g, 56.0, 544.0, "Signature saying:", 13.0, BLACK);
        let male = p.person.b21 & 0x80 == 0;
        let say =
            p.sayings.first().filter(|t| !t.is_empty()).cloned().unwrap_or_else(|| sg_core::thoughts::signature_saying(0, male).into());
        top(s, g, 36.0, 562.0, &format!("\"{say}\""), 13.0, WHITE);
    }

    /// Select a Theme Pack (docs/DECODE_TITLE2.md 4).
    fn draw_themes(&mut self, g: &mut Gfx, s: &Ui) {
        let art = self.title.art;
        s.image(g, &art.themes, 0.0, 0.0);
        top(s, g, 78.0, 34.0, "Select a Theme Pack", 18.0, BLACK);
        for (k, h) in THEME_HEADS.iter().enumerate() {
            top_c(s, g, 346.0 + 90.0 * k as f32, if k % 2 == 1 { 37.0 } else { 62.0 }, h, 12.0, BLACK);
        }
        let current = crate::render::THEME_PACKS.get(self.theme_pack).copied().unwrap_or("Standard");
        for (i, (folder, has)) in self.title.themes.iter().take(13).enumerate() {
            let y = 92.0 + 32.0 * i as f32;
            let label = folder.replace('_', " ");
            let cur = label.eq_ignore_ascii_case(current);
            if cur {
                s.image_part(g, &art.themes_mo, 72.0, y, 1.0, 33.0, 221.0, 31.0);
            }
            top(s, g, 73.0, y + 11.0, &label, 14.0, if cur { BLACK } else { c15(0x4210) });
            for k in 0..5 {
                let x = 346.0 + 90.0 * k as f32 - 43.0;
                if cur {
                    s.image_part(g, &art.themes_mo, x, y, if has[k] { 89.0 } else { 1.0 }, 1.0, 87.0, 31.0);
                } else if has[k] {
                    s.image_part(g, &art.themes_mo, x, y, 177.0, 1.0, 87.0, 31.0);
                }
            }
        }
        match self.hover {
            0 => s.image_part(g, &art.themes_mo, 581.0, 520.0, 265.0, 1.0, 71.0, 80.0),
            1 => s.image_part(g, &art.themes_mo, 726.0, 533.0, 337.0, 1.0, 48.0, 49.0),
            _ => {}
        }
    }

    /// The credits (docs/DECODE_TITLE2.md 6): the lines rise from the bottom at one pixel per 30 ms.
    fn draw_credits(&mut self, g: &mut Gfx, s: &Ui) {
        let art = self.title.art;
        let clock = self.clock;
        let Some(c) = self.title.credits.as_mut() else { return };
        if c.start < 0.0 {
            c.start = clock - c.preroll;
        }
        s.image(g, &art.credits, 0.0, 0.0);
        const SIZE: f32 = 20.0;
        const PITCH: f32 = 24.0;
        let y0 = 600.0 - ((clock - c.start) * 1000.0 / 30.0) as f32;
        for (i, l) in c.lines.iter().enumerate() {
            let y = y0 + PITCH * i as f32;
            if !(-70.0..610.0).contains(&y) {
                continue;
            }
            if l.starts_with('$') {
                // DERIVED: the "$bink" line draws the logo centred
                s.image(g, &art.bink, 368.0, y);
                continue;
            }
            top(s, g, 52.0, y + 2.0, l, SIZE, BLACK);
            top(s, g, 50.0, y, l, SIZE, WHITE);
        }
        if y0 <= -(PITCH * c.lines.len() as f32 + 150.0) {
            self.close_credits();
        }
    }

    /// System Functions, Save the current game (0x405b10): a name prompt over the stopped game.
    pub fn open_save(&mut self) {
        self.title.save_name = Some(self.course_name.clone());
    }

    /// A key while the Save dialog is open: Enter saves to Saved Games/<name>.sgs (an invalid name is refused with the
    /// error sound), Esc cancels, Backspace deletes. Returns false when the dialog is not open.
    pub fn save_key(&mut self, k: KeyCode) -> bool {
        let Some(name) = self.title.save_name.as_mut() else { return false };
        match k {
            KeyCode::Escape => self.title.save_name = None,
            KeyCode::Backspace => {
                name.pop();
            }
            KeyCode::Enter | KeyCode::KpEnter => {
                let n = name.trim().to_string();
                if n.is_empty() || n.starts_with('.') || n.chars().any(|c| "\\/:*?\"<>|".contains(c)) {
                    self.ui_sound(0x18);
                    return true;
                }
                let path = saved_games_dir().join(format!("{n}.sgs"));
                self.title.save_name = None;
                match self.save_game(&path) {
                    Ok(()) => {
                        println!("saved game {}", path.display());
                        self.show_toast("Game Saved");
                    }
                    Err(e) => self.show_toast(&format!("Could not save: {e}")),
                }
            }
            _ => {}
        }
        true
    }

    pub fn save_char(&mut self, c: char) {
        if let Some(name) = self.title.save_name.as_mut() {
            if !c.is_control() && name.chars().count() < 32 {
                name.push(c);
            }
        }
    }

    pub fn draw_save_dialog(&self, g: &mut Gfx) {
        let Some(name) = &self.title.save_name else { return };
        let s = Ui::new(self.draw_w, self.draw_h);
        let head = "SAVE GAME, edit name then press Enter";
        let w = (text_width(head, 17.0) + 60.0).max(360.0);
        let (x, y, h) = (400.0 - w / 2.0, 240.0, 90.0);
        s.fill(g, x - 3.0, y - 3.0, w + 6.0, h + 6.0, rgba(0.55, 0.55, 0.85, 0.95));
        s.fill(g, x, y, w, h, rgba(0.92, 0.92, 1.0, 0.97));
        s.text_centered(g, 400.0, y + 28.0, head, 17.0, rgb(0.15, 0.1, 0.4));
        s.fill(g, x + 20.0, y + 42.0, w - 40.0, 28.0, WHITE);
        let caret = if (self.clock * 2.0) as i64 % 2 == 0 { "|" } else { "" };
        s.text(g, x + 26.0, y + 62.0, &format!("{name}{caret}"), 16.0, rgb(0.05, 0.05, 0.2));
        g.flush();
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn autosave_rows() {
        assert_eq!(row_name(Path::new("Saved Games/&Year 2002.sgs")), "autosave (Year 2002)");
        assert_eq!(row_name(Path::new("Saved Games/My Club.sgs")), "My Club");
    }

    #[test]
    fn list_buttons() {
        assert_eq!(list_hit(633.0, 555.0), 0);
        assert_eq!(list_hit(769.0, 556.0), 1);
        assert_eq!(list_hit(784.0, 120.0), 2);
        assert_eq!(list_hit(784.0, 400.0), 3);
        assert_eq!(list_hit(704.0, 555.0), 4);
        assert_eq!(list_hit(400.0, 300.0), -1);
    }
}
