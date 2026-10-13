//! The player's own copy of SimGolf (docs/PLAYING.md): where the game looks for its files when it starts, the folder the
//! player chose (`game_dir` in settings.json), and the port's own "Use my SimGolf files..." and "Use the free graphics"
//! on the title screen and in Preferences.
//!
//! The folder is looked for in this order: the `--game` option; the saved `game_dir` while it still holds the files; a
//! `SimGolf` or `game` folder next to the program (inside or beside a macOS .app, beside an AppImage); `game` in the save
//! folder; the usual install folders of the system; `game` in the folder the game was started from. With none found the
//! game plays on the free base set (content.rs) and the title screen says once how to use the original files.
//!
//! Choosing a folder checks it for the files the game cannot start without, saves it, and starts the program again on it:
//! the original's art, sounds, fonts and tables then load from the start exactly as they do on a normal start, which no
//! in-place swap could promise. A browser gets its files from the page (web/opensgolf.js) and has no such option; on
//! Android the option says which folder to copy the files to.
use crate::app::{save_dir, App};
use sg_core::lang::{tr, trf};
use std::path::{Path, PathBuf};
use std::sync::atomic::{AtomicBool, Ordering};

/// The settings.json key of the folder the player chose.
pub const SETTINGS_KEY: &str = "game_dir";
/// The settings.json key that records that the free graphics hint was shown.
const HINT_KEY: &str = "free_graphics_hint_shown";
/// Test hook: a folder to take as the one picked in the system's dialog, chosen once the game has started (or "free" to
/// choose the free graphics). Not passed on to the restarted program.
pub const PICK_ENV: &str = "SG_PICK_FOLDER";
/// A line for the restarted program to show (what the player chose), passed in its environment.
const NOTICE_ENV: &str = "OPENSGOLF_NOTICE";

/// What the game cannot start without, in the folder of the original's data (Program_Files_(ENGLISH) on the CD): the title
/// art, the terrain textures, the flics and the theme packs. Matched without case, as the loader matches names.
pub const REQUIRED: [&str; 4] = ["Interface/TitleBASE.pcx", "Data/Textures", "Flics", "Themes"];

/// Set when the game draws to a file (--png): a restart then waits for the new program and exits with its code.
static HEADLESS: AtomicBool = AtomicBool::new(false);

pub fn set_headless(on: bool) {
    HEADLESS.store(on, Ordering::Relaxed);
}

/// The required entries `dir` lacks (empty: it holds the game's data).
pub fn missing(dir: &Path) -> Vec<&'static str> {
    REQUIRED.iter().copied().filter(|r| !sg_core::fsutil::exists(sg_core::fsutil::resolve(dir, r))).collect()
}

/// A folder name that may hold the game a level further down (the CD's Program_Files_(ENGLISH), an install folder).
fn worth_a_look(p: &Path) -> bool {
    let n = p.file_name().map(|n| n.to_string_lossy().to_lowercase()).unwrap_or_default();
    ["simgolf", "program", "game", "maxis", "ea games", "electronic arts"].iter().any(|w| n.contains(w))
}

/// The folder of the game's data in or under `dir`: `dir` itself, or a sub-folder up to two levels down (the copied CD's
/// Program_Files_(ENGLISH), an install folder's SimGolf), sub-folders with a likely name first. Err lists what `dir`
/// itself lacks.
pub fn locate(dir: &Path) -> Result<PathBuf, Vec<&'static str>> {
    let top = missing(dir);
    if top.is_empty() {
        return Ok(dir.to_path_buf());
    }
    let mut level = vec![dir.to_path_buf()];
    for depth in 0..2 {
        let mut next = Vec::new();
        for d in &level {
            let mut kids: Vec<PathBuf> = sg_core::fsutil::list_dir(d).into_iter().filter(|p| sg_core::fsutil::is_dir(p)).collect();
            kids.sort_by_key(|p| (!worth_a_look(p), p.clone()));
            for k in kids {
                if missing(&k).is_empty() {
                    return Ok(k);
                }
                if depth == 0 && worth_a_look(&k) {
                    next.push(k);
                }
            }
        }
        level = next;
    }
    Err(top)
}

// ---- settings.json ------------------------------------------------------------------------------------------------------------

fn settings_path() -> PathBuf {
    save_dir().join(crate::hd::SETTINGS_FILE)
}

fn read_settings() -> serde_json::Value {
    sg_core::fsutil::read_file(settings_path())
        .and_then(|d| serde_json::from_slice::<serde_json::Value>(&d).ok())
        .filter(|v| v.is_object())
        .unwrap_or_else(|| serde_json::json!({}))
}

fn write_setting(key: &str, value: serde_json::Value) {
    let mut v = read_settings();
    v[key] = value;
    if let Ok(t) = serde_json::to_string_pretty(&v) {
        sg_core::fsutil::write_file(settings_path(), t.as_bytes());
    }
}

/// The folder the player chose, as saved.
pub fn saved_dir() -> Option<PathBuf> {
    read_settings()[SETTINGS_KEY].as_str().filter(|s| !s.trim().is_empty()).map(PathBuf::from)
}

/// Saves the folder the player chose.
pub fn save_game_dir(dir: &Path) {
    write_setting(SETTINGS_KEY, serde_json::Value::from(dir.to_string_lossy().into_owned()));
}

/// True the first time it is asked (and records that it was): the free graphics hint is shown once.
pub fn take_hint() -> bool {
    if read_settings()[HINT_KEY].as_bool() == Some(true) {
        return false;
    }
    write_setting(HINT_KEY, serde_json::Value::Bool(true));
    true
}

/// True when `text` differs from what settings.json records under `key` (and records it): a message shown once until it
/// changes.
pub fn take_once(key: &str, text: &str) -> bool {
    if read_settings()[key].as_str() == Some(text) {
        return false;
    }
    write_setting(key, serde_json::Value::from(text));
    true
}

// ---- finding the folder at start up -------------------------------------------------------------------------------------------

/// How the game folder was found.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum How {
    /// The --game option.
    Given,
    /// The folder the player chose (settings.json).
    Saved,
    /// One of the usual places.
    Found,
    /// Nowhere: the default path, which holds nothing (the free base set is played).
    NotFound,
}

/// Where the original game's files are looked for when nothing else names a folder. In a browser the page puts the player's
/// game folder at "game" (web/opensgolf.js); on Android it is the app's external files folder (android.rs).
pub fn default_dir() -> PathBuf {
    #[cfg(target_os = "android")]
    {
        crate::android::game_dir()
    }
    #[cfg(not(target_os = "android"))]
    PathBuf::from(if cfg!(target_arch = "wasm32") { "game" } else { "game/Program_Files_(ENGLISH)" })
}

/// The usual places of a SimGolf install on this system, and the folders next to the program, in the order they are tried.
pub fn usual_places() -> Vec<PathBuf> {
    let mut v = Vec::new();
    if cfg!(any(target_arch = "wasm32", target_os = "android")) {
        return v;
    }
    let env = |k: &str| std::env::var_os(k).filter(|s| !s.is_empty()).map(PathBuf::from);
    let names = |d: PathBuf| [d.join("SimGolf"), d.join("game")];
    // next to the program: a portable copy, or the files put beside OpenSGolf.app / the AppImage / opensgolf.exe
    if let Some(exe_dir) = std::env::current_exe().ok().and_then(|e| e.parent().map(Path::to_path_buf)) {
        v.extend(names(exe_dir.clone()));
        if cfg!(target_os = "macos") {
            // OpenSGolf.app/Contents/MacOS/opensgolf: the app's Resources, then the folder the app is in
            if let Some(contents) = exe_dir.parent() {
                v.extend(names(contents.join("Resources")));
                if let Some(beside) = contents.parent().and_then(Path::parent) {
                    v.extend(names(beside.to_path_buf()));
                }
            }
        }
    }
    if let Some(beside) = env("APPIMAGE").and_then(|a| a.parent().map(Path::to_path_buf)) {
        v.extend(names(beside));
    }
    v.push(save_dir().join("game"));
    let home = env("HOME").or_else(|| env("USERPROFILE"));
    if cfg!(target_os = "windows") {
        let mut roots: Vec<PathBuf> = ["ProgramFiles(x86)", "ProgramFiles", "ProgramW6432"].iter().filter_map(|k| env(k)).collect();
        roots.extend([PathBuf::from("C:\\Program Files (x86)"), PathBuf::from("C:\\Program Files")]);
        for r in roots {
            for vendor in ["Maxis", "EA GAMES", "Electronic Arts"] {
                v.push(r.join(vendor).join("SimGolf"));
            }
        }
        v.push(PathBuf::from("C:\\SimGolf"));
    } else if cfg!(target_os = "macos") {
        if let Some(h) = &home {
            v.extend([h.join("Games/SimGolf"), h.join("SimGolf")]);
        }
        v.push(PathBuf::from("/Applications/SimGolf"));
    } else {
        if let Some(h) = &home {
            v.extend([h.join(".local/share/opensgolf/game"), h.join("SimGolf"), h.join("Games/SimGolf")]);
            // installed under Wine
            for pf in ["Program Files (x86)", "Program Files"] {
                for vendor in ["Maxis", "EA GAMES"] {
                    v.push(h.join(".wine/drive_c").join(pf).join(vendor).join("SimGolf"));
                }
            }
        }
    }
    // the folder the game was started from (a developer's checkout)
    v.push(PathBuf::from("game"));
    v.dedup();
    v
}

/// The game folder for this run: the --game option (a folder above the data is followed down to it), else the saved folder
/// while it still holds the files, else the first usual place that does, else the default (which holds nothing).
pub fn find(given: Option<PathBuf>) -> (PathBuf, How) {
    if let Some(g) = given {
        return (locate(&g).unwrap_or(g), How::Given);
    }
    if cfg!(not(any(target_arch = "wasm32", target_os = "android"))) {
        if let Some(d) = saved_dir().and_then(|d| locate(&d).ok()) {
            return (d, How::Saved);
        }
        if let Some(d) = usual_places().iter().find_map(|p| locate(p).ok()) {
            return (d, How::Found);
        }
    }
    let d = default_dir();
    let how = if crate::content::has_original(&d) { How::Found } else { How::NotFound };
    (d, how)
}

/// The line the restarted program shows (what the player chose), once.
pub fn take_notice() -> Option<String> {
    std::env::var(NOTICE_ENV).ok().filter(|s| !s.is_empty())
}

// ---- choosing a folder ----------------------------------------------------------------------------------------------------------

/// What the title button and the Preferences line do here.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Action {
    /// Pick a folder with the system's dialog.
    Pick,
    /// Go back to the free base set.
    Free,
    /// Say where to copy the files (Android, or a build without the picker).
    Where,
    /// A saved game made with other content packs (content_profiles.rs): switch to its packs and load it after a restart.
    SavePacksSwitch,
    /// The same: load it with the packs in use.
    SavePacksLoad,
}

/// Opens the system's folder picker (blocking until the player chooses or cancels). The test hook stands in for it.
fn pick_folder(start: Option<PathBuf>) -> Option<PathBuf> {
    if let Some(p) = std::env::var_os(PICK_ENV) {
        return Some(PathBuf::from(p));
    }
    #[cfg(all(feature = "picker", not(any(target_arch = "wasm32", target_os = "android"))))]
    {
        let mut d = rfd::FileDialog::new().set_title(tr("STR_GAME_FILES_PICK_TITLE"));
        if let Some(s) = start.filter(|s| s.is_dir()) {
            d = d.set_directory(s);
        }
        d.pick_folder()
    }
    #[cfg(not(all(feature = "picker", not(any(target_arch = "wasm32", target_os = "android")))))]
    {
        let _ = start;
        None
    }
}

/// The folder to copy the files to when there is no picker: the app's folder on Android, else `game` in the save folder.
fn copy_target() -> PathBuf {
    if cfg!(target_os = "android") {
        default_dir()
    } else {
        save_dir().join("game")
    }
}

/// Starts this program again with the same options (less --game and --base, so the saved choice applies), showing `note`.
/// On Unix the program replaces itself (same process: an AppImage stays mounted, the Dock icon stays); on Windows it starts
/// a new one and leaves. Returns only when that failed.
fn restart(note: &str) -> std::io::Error {
    restart_with(note, None)
}

/// The variable that names a saved game to load once the program has started again (content_profiles.rs).
const LOAD_ENV: &str = "OPENSGOLF_LOAD_AFTER_RESTART";

/// The saved game the program was started again to load, once (the variable is cleared, so a later restart does not load it
/// again).
pub fn take_load_after_restart() -> Option<PathBuf> {
    let p = std::env::var_os(LOAD_ENV).filter(|s| !s.is_empty()).map(PathBuf::from);
    // (a browser has no environment: removing a variable there panics)
    if p.is_some() {
        std::env::remove_var(LOAD_ENV);
    }
    p
}

/// `restart`, and with `load` the saved game the new program loads once it has started.
pub fn restart_with(note: &str, load: Option<&Path>) -> std::io::Error {
    let exe = match std::env::current_exe() {
        Ok(e) => e,
        Err(e) => return e,
    };
    let mut args = Vec::new();
    let mut it = std::env::args_os().skip(1);
    while let Some(a) = it.next() {
        // a switch of content packs keeps the game folder: only the base set is the saved choice
        if (a == "--game" && load.is_none()) || a == "--base" {
            it.next();
            continue;
        }
        args.push(a);
    }
    println!("restarting on the chosen graphics: {note}");
    let mut cmd = std::process::Command::new(exe);
    cmd.args(args).env(NOTICE_ENV, note).env_remove(PICK_ENV);
    if let Some(l) = load {
        cmd.env(LOAD_ENV, l);
    }
    #[cfg(unix)]
    {
        use std::os::unix::process::CommandExt;
        cmd.exec()
    }
    #[cfg(not(unix))]
    {
        match cmd.spawn() {
            Ok(mut child) => {
                let code = if HEADLESS.load(Ordering::Relaxed) { child.wait().ok().and_then(|s| s.code()).unwrap_or(1) } else { 0 };
                std::process::exit(code)
            }
            Err(e) => e,
        }
    }
}

/// A message in the game's own box over the screen: lines, then its options (OK; or go on and not now, for a question).
pub struct Notice {
    pub lines: Vec<String>,
    /// A question: what the first option does.
    pub confirm: Option<Action>,
    /// A question with three options: what the second does (the last option cancels).
    pub alt: Option<Action>,
    /// The option lit.
    pub sel: Option<usize>,
}

impl Notice {
    /// The generic popup's box, centred on the screen.
    fn choice_box(&self) -> crate::popup_ui::ChoiceBox {
        let h = (self.lines.len() as f32 * 3.0 + 3.0) * 8.0;
        let top = (((600.0 - h) / 2.0) as i32 & !1) as f32;
        crate::popup_ui::ChoiceBox::new(self.lines.clone(), 400.0, top)
    }
}

/// Splits a message into lines that fit `width` pixels in the choice box's font.
pub fn wrap(text: &str, width: f32) -> Vec<String> {
    let f = crate::popup_ui::CHOICE_FONT;
    let mut lines = Vec::new();
    for para in text.split('\n') {
        let mut cur = String::new();
        for w in para.split_whitespace() {
            let next = if cur.is_empty() { w.to_string() } else { format!("{cur} {w}") };
            if f.width(&next) <= width {
                cur = next;
                continue;
            }
            if !cur.is_empty() {
                lines.push(std::mem::take(&mut cur));
            }
            // a word too long for a line (a folder's path) is cut where it must
            for c in w.chars() {
                cur.push(c);
                if f.width(&cur) > width {
                    cur.pop();
                    lines.push(std::mem::replace(&mut cur, c.to_string()));
                }
            }
        }
        lines.push(cur);
    }
    lines
}

impl App {
    /// True while the free base set is the one loaded.
    pub fn on_free_set(&self) -> bool {
        self.content.loaded.first().is_some_and(|id| id == sg_core::packs::FREE)
    }

    /// The title button's and Preferences line's action and label here (None in a browser, which has its own picker on the
    /// page).
    pub fn files_option(&self) -> Option<(Action, &'static str)> {
        if cfg!(target_arch = "wasm32") {
            return None;
        }
        if cfg!(target_os = "android") {
            // only while the original files are not there yet
            return self.on_free_set().then(|| (Action::Where, tr("STR_GAME_FILES_WHERE")));
        }
        if !self.on_free_set() {
            return Some((Action::Free, tr("STR_GAME_FILES_USE_FREE")));
        }
        let pick = if cfg!(feature = "picker") { Action::Pick } else { Action::Where };
        Some((pick, tr("STR_GAME_FILES_USE_MINE")))
    }

    /// Does what the title button or the Preferences line offers.
    pub fn files_action(&mut self, a: Action) {
        match a {
            Action::Pick => self.choose_game_folder(),
            Action::Free => self.use_free_graphics(),
            Action::Where => {
                let msg = trf("STR_GAME_FILES_COPY_HERE", &[("DIR", &copy_target().display())]);
                self.show_notice(&msg);
            }
            Action::SavePacksSwitch | Action::SavePacksLoad => self.save_packs_answer(a == Action::SavePacksSwitch),
        }
    }

    /// "Use my SimGolf files...": the system's folder picker, then the folder is checked; a good one is saved and the game
    /// starts again on it, a wrong one is explained.
    pub fn choose_game_folder(&mut self) {
        let start = saved_dir().or_else(|| Some(self.game_dir.clone()));
        let Some(picked) = pick_folder(start) else { return };
        if picked.as_os_str() == "free" {
            self.use_free_graphics();
            return;
        }
        if let Err(msg) = self.use_game_folder(&picked) {
            println!("game folder: {} refused", picked.display());
            self.show_notice(&msg);
        }
    }

    /// Checks a chosen folder; a good one is saved, the original files become the base set, and the game starts again on
    /// them (this returns only when that failed). Err is the message for the player.
    pub fn use_game_folder(&mut self, picked: &Path) -> Result<(), String> {
        let dir = locate(picked).map_err(|lacks| {
            let lacks: Vec<String> = lacks.iter().map(|l| l.replace('/', std::path::MAIN_SEPARATOR_STR)).collect();
            let name = picked.file_name().map(|n| n.to_string_lossy().into_owned()).unwrap_or_else(|| picked.display().to_string());
            trf("STR_GAME_FILES_NOT_FOUND", &[("DIR", &name), ("MISSING", &lacks.join(", "))])
        })?;
        save_game_dir(&dir);
        crate::content::save_base(None);
        println!("game folder: {}", dir.display());
        let e = restart(tr("STR_GAME_FILES_USING"));
        eprintln!("restart: {e}");
        Err(tr("STR_GAME_FILES_RESTART").to_string())
    }

    /// "Use the free graphics": the free base set becomes the saved choice and the game starts again on it.
    pub fn use_free_graphics(&mut self) {
        crate::content::save_base(Some(sg_core::packs::FREE));
        let e = restart(tr("STR_GAME_FILES_USING_FREE"));
        eprintln!("restart: {e}");
        self.show_notice(tr("STR_GAME_FILES_RESTART"));
    }

    /// Preferences' line during a game: the game starts again for the change, so the player is asked first.
    pub fn files_action_in_game(&mut self, a: Action) {
        if a == Action::Where {
            self.files_action(a);
            return;
        }
        let mut lines = wrap(tr("STR_GAME_FILES_ASK_RESTART"), 560.0);
        lines.push(format!(" {}", tr("STR_GAME_FILES_GO_ON")));
        lines.push(format!(" {}", tr("STR_GAME_FILES_NOT_NOW")));
        self.notice = Some(Notice { lines, confirm: Some(a), alt: None, sel: None });
    }

    /// A message in the game's own box (the generic popup's frame) with an OK, over whatever screen is up.
    pub fn show_notice(&mut self, msg: &str) {
        let mut lines = wrap(msg, 560.0);
        lines.push(format!(" {}", tr("STR_OK")));
        self.notice = Some(Notice { lines, confirm: None, alt: None, sel: Some(0) });
    }

    /// The notice, when one is up: the choice box centred on the screen.
    pub fn draw_notice(&mut self, g: &mut crate::gfx::Gfx) {
        let Some(n) = &self.notice else { return };
        let (b, sel) = (n.choice_box(), n.sel);
        let s = crate::ui::Screen::new(self.draw_w, self.draw_h);
        self.view = s.view;
        crate::ui::set_face(None);
        self.draw_choice_box(g, &s, &b, sel);
        g.flush();
    }

    /// The pointer over the notice (in 800 x 600 units): it lights the option under it; a click on an option chooses it,
    /// a click outside the box cancels. A plain notice goes on any click.
    pub fn notice_pointer(&mut self, vx: f32, vy: f32, click: bool) {
        let Some(n) = self.notice.as_mut() else { return };
        let b = n.choice_box();
        let hit = b.option_at(vx, vy);
        if n.confirm.is_some() {
            n.sel = hit;
        }
        if !click {
            return;
        }
        let Some(a) = n.confirm else {
            self.notice = None;
            return;
        };
        if hit.is_none() && b.frame().has(vx, vy) {
            return;
        }
        let alt = n.alt;
        self.notice = None;
        match (hit, alt) {
            (Some(0), _) => self.files_action(a),
            (Some(1), Some(b)) => self.files_action(b),
            _ => {}
        }
    }

    /// Keys on the notice: Enter or Space take the lit option (a plain notice: close it), Esc cancels, Up and Down move.
    pub fn notice_key(&mut self, k: miniquad::KeyCode) {
        use miniquad::KeyCode;
        let Some(n) = self.notice.as_mut() else { return };
        let last = 1 + n.alt.is_some() as usize;
        match k {
            KeyCode::Up => n.sel = Some(n.sel.map_or(0, |s| s.saturating_sub(1))),
            KeyCode::Down if n.confirm.is_some() => n.sel = Some(n.sel.map_or(0, |s| (s + 1).min(last))),
            KeyCode::Escape => self.notice = None,
            KeyCode::Enter | KeyCode::KpEnter | KeyCode::Space => {
                let go = match n.sel {
                    Some(0) => n.confirm,
                    Some(1) => n.alt,
                    _ => None,
                };
                self.notice = None;
                if let Some(a) = go {
                    self.files_action(a);
                }
            }
            _ => {}
        }
    }

    /// The title screen's button (top left) and, once, the hint beside it that the free graphics are in use.
    pub fn draw_files_button(&self, g: &mut crate::gfx::Gfx, s: &crate::ui::Screen) {
        use crate::ui::{rgb, rgba};
        let Some((_, label)) = self.files_option() else { return };
        let r = files_button_rect(label);
        let lit = self.hover == FILES_HOVER;
        s.fill(g, r.x - 1.0, r.y - 1.0, r.w + 2.0, r.h + 2.0, rgba(1.0, 1.0, 1.0, 0.85));
        s.fill(g, r.x, r.y, r.w, r.h, if lit { rgba(0.13, 0.32, 0.55, 0.95) } else { rgba(0.08, 0.16, 0.32, 0.88) });
        s.put_centered(g, crate::ui::F_MANUAL15, r.x + r.w / 2.0, r.y + 4.0, label, rgb(1.0, 1.0, if lit { 1.0 } else { 0.8 }));
        if self.free_hint {
            let hint = tr("STR_GAME_FILES_HINT");
            let x = r.x + r.w + 10.0;
            let w = crate::ui::F_ARIAL10.width(hint) + 16.0;
            s.fill(g, x, r.y, w.min(790.0 - x), r.h, rgba(1.0, 0.98, 0.85, 0.92));
            s.put(g, crate::ui::F_ARIAL10, x + 8.0, r.y + 6.0, hint, rgb(0.1, 0.1, 0.2));
        }
    }
}

/// The hover value of the title's files button.
pub const FILES_HOVER: i32 = 7;

/// The title button: top left, where the original's art and the free set's leave the background bare.
pub fn files_button_rect(label: &str) -> crate::render::Rect {
    crate::render::Rect::new(8.0, 6.0, crate::ui::F_MANUAL15.width(label).max(120.0) + 24.0, 24.0)
}

#[cfg(test)]
mod tests {
    use super::*;

    fn tmp(name: &str) -> PathBuf {
        let d = std::env::temp_dir().join(format!("opensgolf-gamefiles-{}-{name}", std::process::id()));
        let _ = std::fs::remove_dir_all(&d);
        std::fs::create_dir_all(&d).unwrap();
        d
    }

    fn fake_game(d: &Path) {
        for r in ["Interface", "Data/Textures", "Flics", "Themes"] {
            std::fs::create_dir_all(d.join(r)).unwrap();
        }
        std::fs::write(d.join("Interface/TitleBASE.pcx"), b"x").unwrap();
    }

    #[test]
    fn locate_finds_the_data_folder() {
        let root = tmp("locate");
        // an empty folder: everything is reported
        assert_eq!(locate(&root).unwrap_err(), REQUIRED.to_vec());
        // a copied CD: the data is in Program_Files_(ENGLISH), found from the CD's folder
        let data = root.join("CD").join("Program_Files_(ENGLISH)");
        fake_game(&data);
        assert_eq!(locate(&root.join("CD")).unwrap(), data);
        // from two levels up through a likely name ("SimGolf/Program_Files_(ENGLISH)")
        let up = root.join("Games");
        let inner = up.join("SimGolf").join("Program_Files_(ENGLISH)");
        fake_game(&inner);
        assert_eq!(locate(&up).unwrap(), inner);
        // a folder with some of it: what it lacks is named
        let half = root.join("half");
        std::fs::create_dir_all(half.join("Flics")).unwrap();
        std::fs::create_dir_all(half.join("Themes")).unwrap();
        assert_eq!(locate(&half).unwrap_err(), vec!["Interface/TitleBASE.pcx", "Data/Textures"]);
        // names are matched without case
        let lower = root.join("lower");
        for r in ["interface", "data/textures", "flics", "themes"] {
            std::fs::create_dir_all(lower.join(r)).unwrap();
        }
        std::fs::write(lower.join("interface/titlebase.pcx"), b"x").unwrap();
        assert_eq!(locate(&lower).unwrap(), lower);
        let _ = std::fs::remove_dir_all(&root);
    }

    #[test]
    fn wrap_keeps_lines_short() {
        let lines = wrap("one two three four five six seven eight nine ten eleven twelve thirteen fourteen fifteen", 120.0);
        assert!(lines.len() > 2);
        assert!(lines.iter().all(|l| crate::popup_ui::CHOICE_FONT.width(l) <= 120.0));
        let path = "/a/very/long/folder/path/that/does/not/fit/on/one/line/of/the/box/at/all";
        let lines = wrap(&format!("In {path} nothing"), 120.0);
        assert!(lines.iter().all(|l| crate::popup_ui::CHOICE_FONT.width(l) <= 120.0));
        assert_eq!(lines.concat().replace(' ', ""), format!("In{path}nothing"));
    }
}
