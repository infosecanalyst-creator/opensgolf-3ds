//! opensgolf: the OpenSGolf game. Runs on the free base set, or reads the data of your own copy of SimGolf at run time.
//!
//!   opensgolf --game "<dir>/Program_Files_(ENGLISH)" [--theme parkland|links|desert|tropical] [--seed N] [--size WxH] [--zoom Z]
//!           [--rot DEG] [--center TX,TY] [--time SECONDS] [--follow] [--png out.png] [--golfer NAME] [--sandbox] [--cash N]
//!           [--screen menu|property|play|report] [--course FILE] [--save FILE] [--edit SPEC] [--panel N] [--mute] [--sound-log]
//!           [--hd | --classic] [--hd-pack DIR] [--course-size N]
//!
//! Keys: arrows/WASD pan, Q/E rotate, +/- or mouse wheel zoom, 1-4 theme, R new demo course, P toggle scenery, F follow the golfer,
//! F6 screenshot, F1-F10 reports, M music, N mute, H open the new hole, Shift+H advisor, Tab edit mode, Esc menu. Left-drag pans.

// UI drawing takes source and destination rectangles as plain numbers; index loops mirror the original's tables.
#![allow(clippy::too_many_arguments, clippy::needless_range_loop)]
// Windows: a release build is a windowed program, so a double-click opens no console window; started from a console with
// options it writes its messages there (attach_console).
#![cfg_attr(all(windows, not(debug_assertions)), windows_subsystem = "windows")]

#[cfg(target_os = "android")]
mod android;
mod app;
mod audio;
#[cfg(target_os = "macos")]
mod autorelease;
mod autosave;
#[cfg(all(not(target_arch = "wasm32"), feature = "net"))]
mod autoupdate;
mod build_tools;
mod champ_ui;
mod comfort;
mod content;
mod content_profiles;
mod controls;
mod controls_ui;
mod cursor_ui;
mod cust_ui;
mod edges;
mod enhanced;
mod extras;
mod extras_ui;
mod files_ui;
mod framerate;
mod fx;
mod fx_ball;
mod fx_crowd;
mod fx_glsl;
mod fx_life;
mod fx_light;
mod fx_nature;
mod fx_night;
mod fx_reflect;
mod fx_shadow;
mod fx_smooth;
mod fx_wet;
mod fx_winter;
mod gamefiles;
mod gamepad;
mod gfx;
mod hd;
mod history_ui;
mod holemarks;
mod hud_ui;
mod import;
mod info_ui;
mod insight;
mod keys_ui;
mod language;
mod member_panel;
mod message_ui;
mod net;
mod panels_ui;
mod pathview_ui;
mod people_ui;
mod perf;
mod photo;
mod player_panel;
mod popup_ui;
mod prefs_search;
mod pro_ui;
mod profile_ui;
mod render;
mod report_tabs;
mod reports_ui;
mod scenario_ui;
mod screens_ui;
mod share;
mod staffview_ui;
mod sysdiag;
mod tags_ui;
mod thoughts_ui;
mod touch;
mod tourney_ui;
mod ui;
mod undo;
mod update;
mod wild_ui;
mod world_ui;

use app::*;
use gfx::Gfx;
use miniquad::*;
use render::*;
use sg_core::formats::{atoi, parse_pro_golfers};
use sg_core::lang::tr;
use sg_core::mixer::Mixer;
use sg_core::terrain::{Terrain, TT_FAIRWAY, TT_PUTTING_GREEN, TT_ROUGH, TT_SAND, TT_TEE, TT_WATER_SHALLOW};
use std::path::PathBuf;
use std::sync::Arc;

struct Options {
    game_dir: PathBuf,
    /// How the game folder was found (--game, the saved choice, a usual place, or not at all).
    game_how: gamefiles::How,
    theme: usize,
    seed: u32,
    win_w: i32,
    win_h: i32,
    zoom: f32,
    rot: f32,
    png_out: Option<PathBuf>,
    time: f64,
    follow: bool,
    golfer: Option<String>,
    mute: bool,
    sound_log: bool,
    sandbox: bool,
    /// --course-size N: the map edge in tiles of a new game (sg_core::mapsize; default the original's 50).
    course_size: i32,
    screen: Option<String>,
    cash: Option<f64>,
    /// Test hook: the extras a new game starts with (sg_core::extras bits), not saved to the settings.
    extras: Option<u32>,
    difficulty: Option<i32>,
    course: Option<PathBuf>,
    save: Option<PathBuf>,
    save_game: Option<PathBuf>,
    load_game: Option<PathBuf>,
    champ: Option<(PathBuf, PathBuf)>,
    edit_spec: String,
    panel: Option<i32>,
    center: Option<(i32, i32)>,
    clock: Option<u32>,
    property: Option<String>,
    /// --hd / --classic: the graphics mode for this run (else the saved choice).
    hd: Option<bool>,
    hd_pack: Option<PathBuf>,
    fx: bool,
    fx_set: Option<String>,
    /// --enhanced / --classic-render: Enhanced graphics on or off for this run (else the saved choice; enhanced.rs).
    enhanced: Option<bool>,
    /// --no-extras: the OpenSGolf extras off for this run (extras.rs), not saved.
    no_extras: bool,
    /// --fullscreen, or Steam's Game Mode (the Steam Deck): start full screen.
    fullscreen: bool,
    /// --base ID: the base set for this run (original, opensgolf-free or a base pack), not saved.
    base: Option<String>,
    /// --lang CODE: the language for this run (else the saved setting); --lang-file FILE: a translation file to try out.
    lang: Option<String>,
    lang_file: Option<PathBuf>,
    /// --scenario ID: starts the scenario pack with that id (docs/MODDING.md 5).
    scenario: Option<String>,
}

const USAGE: &str = "usage: opensgolf [--game DIR] [--theme T] [--seed N] [--size WxH] [--zoom Z] [--rot DEG] [--center TX,TY] [--png FILE] \
[--time S] [--follow] [--golfer NAME] [--sandbox] [--course-size N] [--cash N] [--difficulty 0-3] [--screen menu|property|play|report] [--course FILE] [--save FILE] \
[--edit SPEC] [--panel N] [--mute] [--sound-log] [--clock MS] [--property N|NAME] [--hd | --classic] [--hd-pack DIR] \
[--enhanced | --classic-render] [--no-extras] [--fullscreen] [--fx] [--fx-set LIST] \
[--lang CODE] [--lang-file FILE] [--base original|opensgolf-free|ID] [--scenario ID]";

/// Leading comma separated integers, like sscanf("%d,%d,...") (stops at the first one that does not parse).
fn ints(s: &str) -> Vec<i32> {
    let mut v = Vec::new();
    for part in s.split(',') {
        let p = part.trim_start();
        let ok = p.strip_prefix('-').unwrap_or(p).starts_with(|c: char| c.is_ascii_digit());
        if !ok {
            break;
        }
        v.push(atoi(p));
    }
    v
}

fn parse_args() -> Options {
    let mut given_game = None;
    let mut o = Options {
        game_dir: PathBuf::new(),
        game_how: gamefiles::How::NotFound,
        theme: 0,
        seed: 7,
        win_w: 1280,
        win_h: 800,
        // the exe starts at its closest zoom level, 4
        zoom: app::ZOOM_UNIT * 4.0,
        rot: 0.0,
        png_out: None,
        time: 0.0,
        follow: false,
        golfer: None,
        mute: false,
        sound_log: false,
        sandbox: false,
        course_size: sg_core::mapsize::ORIGINAL,
        screen: None,
        cash: None,
        extras: None,
        difficulty: None,
        course: None,
        save: None,
        save_game: None,
        load_game: None,
        champ: None,
        edit_spec: String::new(),
        panel: None,
        center: None,
        clock: None,
        property: None,
        hd: None,
        hd_pack: None,
        fx: false,
        fx_set: None,
        enhanced: None,
        no_extras: false,
        fullscreen: false,
        base: None,
        lang: None,
        lang_file: None,
        scenario: None,
    };
    let args: Vec<String> = std::env::args().skip(1).collect();
    let mut i = 0;
    while i < args.len() {
        let a = args[i].as_str();
        let mut next = || {
            i += 1;
            args.get(i).cloned().unwrap_or_default()
        };
        match a {
            "--game" => given_game = Some(PathBuf::from(next())),
            "--theme" => {
                let t = next().to_lowercase();
                if let Some(k) = THEMES.iter().position(|n| n.to_lowercase() == t) {
                    o.theme = k;
                }
            }
            "--seed" => o.seed = next().parse().unwrap_or(7),
            "--clock" => o.clock = next().parse().ok(),
            "--property" => o.property = Some(next()),
            "--size" => {
                let s = next();
                if let Some((w, h)) = s.split_once('x') {
                    o.win_w = w.parse().unwrap_or(1280);
                    o.win_h = h.parse().unwrap_or(800);
                }
            }
            "--zoom" => o.zoom = next().parse().unwrap_or(app::ZOOM_UNIT * 4.0),
            "--rot" => o.rot = next().parse().unwrap_or(0.0),
            "--png" => o.png_out = Some(PathBuf::from(next())),
            "--time" => o.time = next().parse().unwrap_or(0.0),
            "--follow" => o.follow = true,
            "--golfer" => o.golfer = Some(next()),
            "--mute" => o.mute = true,
            "--perf" => perf::enable(5.0), // the performance log (perf.rs)
            "--sound-log" => o.sound_log = true,
            "--sandbox" => o.sandbox = true,
            "--course-size" => o.course_size = sg_core::mapsize::valid(next().parse().unwrap_or(0)),
            "--screen" => o.screen = Some(next()),
            "--cash" => o.cash = next().parse().ok(),     // test hook: starting cash
            "--extras" => o.extras = next().parse().ok(), // test hook: the extras' bits for a new game (extras_ui)
            "--difficulty" => o.difficulty = next().parse().ok().map(|d: i32| d.clamp(0, 3)),
            "--course" => o.course = Some(PathBuf::from(next())),
            "--save" => o.save = Some(PathBuf::from(next())),
            "--save-game" => o.save_game = Some(PathBuf::from(next())),
            "--load-game" => o.load_game = Some(PathBuf::from(next())),
            "--champ" => {
                let c = PathBuf::from(next());
                o.champ = Some((c, PathBuf::from(next())));
            }
            "--edit" => o.edit_spec = next(),
            "--hd" => o.hd = Some(true),
            "--classic" => o.hd = Some(false),
            "--hd-pack" => o.hd_pack = Some(PathBuf::from(next())),
            "--fx" => o.fx = true,
            "--fx-set" => o.fx_set = Some(next()),
            "--fx-list" => {
                fx::configure(true, None);
                fx::print_options();
                std::process::exit(0);
            }
            "--enhanced" => o.enhanced = Some(true),
            "--classic-render" => o.enhanced = Some(false),
            "--no-extras" => o.no_extras = true,
            "--fullscreen" => o.fullscreen = true,
            "--base" => o.base = Some(next()),
            "--lang" => o.lang = Some(next()),
            "--lang-file" => o.lang_file = Some(PathBuf::from(next())),
            "--scenario" => o.scenario = Some(next()),
            "--panel" => o.panel = Some(atoi(&next())),
            "--center" => {
                let v = ints(&next());
                if v.len() >= 2 {
                    o.center = Some((v[0], v[1]));
                }
            }
            "--help" | "-h" => {
                println!("{USAGE}");
                std::process::exit(0);
            }
            // older macOS hands a program started from the Finder its process serial number
            a if a.starts_with("-psn_") => {}
            _ => {
                eprintln!("{USAGE}");
                std::process::exit(2);
            }
        }
        i += 1;
    }
    // the player's own SimGolf files: --game, else the folder chosen in the game, else the usual places (gamefiles.rs)
    (o.game_dir, o.game_how) = gamefiles::find(given_game);
    o
}

struct Stage {
    /// A controller (gamepad.rs).
    pad: gamepad::Pad,
    /// the desktop window is full screen (F11 or Alt+Enter)
    #[cfg_attr(any(target_arch = "wasm32", target_os = "android"), allow(dead_code))]
    fullscreen: bool,
    g: Gfx,
    app: App,
    png_out: Option<PathBuf>,
    save_game_out: Option<PathBuf>,
    frames: u32,
    editing: bool,
    dragging: bool,
    paused_total: f64,
    pause_start: f64,
    last_tick: f64,
    mouse: (f32, f32),
    shift: bool,
    ctrl: bool,
    /// Alt (Option on a Mac) is held: a click with the terrain brush fills (build_tools.rs).
    alt: bool,
    /// The mouse button held down was a Ctrl+click, taken as a right-click (its release is a right one too).
    ctrl_click: bool,
    /// A key just opened a name box (Shift+S the Save dialog): the character that key types comes next and is not the
    /// name's (it put an "S" in front of every typed save name).
    swallow_char: bool,
    /// The wheel's turn not yet spent on a zoom step (in notches) and the clock of its last event.
    wheel: (f32, f64),
    pause_toggle: bool,
    shown_version: u32,
    touch: touch::Touch,
}

fn load_ui(app: &mut App, g: &mut Gfx) -> bool {
    let font = ui::Font::load(g, |f| app.game_path(f));
    load_art(app, g) && font
}

/// The interface art (at start up, and again when the graphics mode changes). False when the title art is missing.
fn load_art(app: &mut App, g: &mut Gfx) -> bool {
    let mut img = |rel: &str, magenta: bool, key: Option<u32>| ui::load_pcx(g, &app.game_path(&format!("Interface/{rel}")), magenta, key);
    let base = img("TitleBASE.pcx", false, None);
    let un = img("TitleUnSel.pcx", true, None);
    let mo = img("TitleMO.pcx", true, None);
    let world = img("WorldBase.pcx", false, None);
    let diff_base = img("TitleSelDiffUnSel.pcx", false, None);
    let diff_mo = img("TitleSelDiffMO.pcx", true, None);
    let report = img("infoscreens/coursereport.pcx", true, None); // optional
    let dock = img("3mainLowerLeft.pcx", false, Some(0xF800F8)); // optional: the lower left dock
    const ICON: [&str; 4] = ["ChooseParklandButtons.pcx", "ChooseLinksButtons.pcx", "ChooseDesertButtons.pcx", "ChooseTropicalButtons.pcx"];
    let icons: Vec<Option<ui::Image>> = ICON.iter().map(|f| img(f, false, Some(0xFF0000))).collect(); // icons are optional
    let ok = base.is_some() && un.is_some() && mo.is_some() && world.is_some();
    app.title_base = base.unwrap_or_default();
    app.title_un = un.unwrap_or_default();
    app.title_mo = mo.unwrap_or_default();
    let mut rects: Vec<(f32, f32, f32, f32)> = render::MENU_BTN.iter().map(|r| (r.x, r.y, r.w, r.h)).collect();
    rects.push((170.0, 190.0, 480.0, 165.0)); // the logo
    app.title_mo_parts = ui::split_overlay(g, &app.game_path("Interface/TitleMO.pcx"), &rects);
    app.world_base = world.unwrap_or_default();
    app.diff_base = diff_base.unwrap_or_default();
    app.hud_art =
        ui::load_pcx_alpha(g, &app.game_path("Interface/courseinfo.pcx"), &app.game_path("Interface/courseinfo_A.pcx")).unwrap_or_default();
    app.diff_mo = diff_mo.unwrap_or_default();
    app.report_art = report.unwrap_or_default();
    app.dock_art = dock.unwrap_or_default();
    for (t, i) in icons.into_iter().enumerate() {
        app.theme_icons[t] = i.unwrap_or_default();
    }
    // the terrain brushes' tile pictures (0x445a43): 57 x 40 cells, eight to a row
    for (t, f) in ["parkland", "desert", "tropical", "links"].iter().enumerate() {
        app.tool_tiles[t] = ui::load_pcx(g, &app.game_path(&format!("Data/{f}.pcx")), true, None).unwrap_or_default();
    }
    // the bank rocks: in HD with the classic picture's key (hd::load_pcx_classic_key, docs/FIXES.md 1)
    let cliffs = app.game_path("cliffs01.pcx");
    app.cliffs = crate::hd::active()
        .then(|| crate::hd::load_pcx_classic_key(g, &cliffs))
        .flatten()
        .or_else(|| ui::load_pcx(g, &cliffs, true, None))
        .unwrap_or_default();
    app.ui_ok = ok;
    app.art = crate::screens_ui::Art::load(g, app);
    app.register_portraits(g);
    app.reports = crate::reports_ui::ReportArt::load(g, app);
    app.panel_art = crate::panels_ui::PanelArt::load(g, app);
    app.title.art = crate::files_ui::TitleArt::load(g, app);
    app.hud = crate::hud_ui::HudArt::load(g, app);
    app.info.art = crate::info_ui::InfoArt::load(g, app);
    ok
}

/// Scripted edits for tests: "p:x,y,type[,vbyte,radius];w:x,y,kind[,radius];b:x,y,building[,design];r:cx,cy,delta[,radius];
/// h:kind[,x,y];t:x,y,type;o" (t paints one tile through the hole tool, o opens the hole being built; h hires an employee: 0 Club Pro, 1 Ranger, 2 Groundskeeper, 3 Soda Vendor; x,y is the post tile).
/// The --screen test hook: opens a screen of a game in progress.
fn open_screen(app: &mut App, screen: Option<&str>) {
    match screen {
        Some("land") => app.open_land_screen(),
        Some("report") if app.ui_ok => app.screen = Screen::Report,
        Some("pair") if app.ui_ok => app.screen = Screen::Pair,
        // the great-shot SimFoto (0x407e00) of the player's pro, or of the first golfer on screen with the partner
        Some("greatshot") => app.test_great_shot = true,
        Some("board") if app.ui_ok => {
            // SG_BOARD_AWARDS="id,id,..": accomplishments marked earned (a month apart, no snapshot) for a still
            if let Ok(ids) = std::env::var("SG_BOARD_AWARDS") {
                let date = app.club.date();
                for (k, id) in ints(&ids).into_iter().enumerate() {
                    if let Some(e) = app.club.earned.get_mut(id.max(0) as usize) {
                        let tick = date + 1024 * k as u32;
                        *e = Some(sg_core::records::Earned { tick, course: app.club.course_name.clone() });
                    }
                }
            }
            app.screen = Screen::Board;
        }
        // the report as the coming year end will show it (the season in progress), whatever the date: the real report opens
        // only on the tick that ends a season
        Some("yearend") if app.ui_ok => {
            app.year_end_tick = (app.club.date() | 0x1fff) + 1;
            app.screen = Screen::YearEnd;
        }
        Some("roster") if app.ui_ok => {
            roster_fill(app);
            app.screen = Screen::Roster;
        }
        Some("comments") if app.ui_ok => app.screen = Screen::Comments,
        Some("histograph") if app.ui_ok => app.screen = Screen::Histograph,
        Some("finance") if app.ui_ok => app.screen = Screen::Finance,
        Some("routing") if app.ui_ok => {
            // SG_ROUTE_TAB picks the tab for a still: 0 routing, 1 employees, 2 aura, 3 home site value
            if let Some(t) = std::env::var("SG_ROUTE_TAB").ok().and_then(|t| t.parse::<usize>().ok()) {
                app.route_tab = t.min(3);
            }
            app.screen = Screen::Routing;
        }
        Some("shortcuts") if app.ui_ok => app.screen = Screen::Shortcuts,
        Some("content") if app.ui_ok => app.open_content(),
        // SG_KEYS_PAGE=N shows page N (from 0) of Preferences > Keyboard controls
        Some("keys") if app.ui_ok => {
            app.open_keys(Screen::Play);
            app.keys_page = std::env::var("SG_KEYS_PAGE").ok().and_then(|v| v.parse().ok()).unwrap_or(0);
        }
        Some("goals") if app.ui_ok => app.open_goals(),
        // the OpenSGolf extras' screens (extras.rs); SG_PEOPLE_SORT=COL[,1] sorts the list by column COL (downward with 1)
        Some(s @ ("people" | "staff" | "pro")) if app.ui_ok => {
            app.open_people();
            app.xs.people.tab = match s {
                "staff" => people_ui::Tab::Staff,
                "pro" => people_ui::Tab::Pro,
                _ => people_ui::Tab::Golfers,
            };
            if let Ok(v) = std::env::var("SG_PEOPLE_SORT") {
                let v = ints(&v);
                let t = (s == "staff") as usize;
                app.xs.people.sort[t] = (v.first().copied().unwrap_or(0).clamp(0, 5) as usize, v.get(1).is_some_and(|&d| d != 0));
            }
            app.xs.test_open = Some(s.to_string());
        }
        // the insight extras (insight.rs): comments by hole, and the landing map (of the first open hole until the pointer
        // finds another, or of SG_LANDING_HOLE)
        // SG_HOLECOMMENTS=P[,N]: the period P (0 this month, 1 last month, 2 this year), scrolled N holes down
        Some("holecomments") if app.ui_ok => {
            app.open_hole_comments();
            if let Ok(v) = std::env::var("SG_HOLECOMMENTS") {
                let v = ints(&v);
                app.xs.insight.period = v.first().copied().unwrap_or(0).clamp(0, 2) as usize;
                app.xs.insight.off = v.get(1).copied().unwrap_or(0).clamp(0, 17) as usize;
            }
        }
        // SG_LANDING_HOLE=N: hole N's map whatever the pointer is over
        Some("landing") if app.ui_ok => {
            app.xs.insight.map = true;
            app.xs.insight.hole = (1..19).find(|&h| app.club.holes[h].par != 0).unwrap_or(0);
            if let Some(h) = std::env::var("SG_LANDING_HOLE").ok().and_then(|v| v.parse::<usize>().ok()) {
                app.xs.insight.hole = h.clamp(1, 18);
                app.xs.insight.fixed = true;
            }
        }
        Some(s @ ("history" | "profile" | "pathview" | "tags" | "ticker" | "card")) if app.ui_ok => {
            if s == "history" {
                app.open_history();
            }
            if s == "pathview" {
                app.xs.path_view = true;
            }
            app.xs.test_open = Some(s.to_string());
        }
        Some("content-index") if app.ui_ok => {
            app.open_content();
            app.open_content_index();
            // SG_INDEX_DOWNLOAD=N downloads the index's offer N at once (a local index finishes before the still)
            if let Some(k) = std::env::var("SG_INDEX_DOWNLOAD").ok().and_then(|v| v.parse().ok()) {
                app.index_download(k);
            }
        }
        Some("golfers") if app.ui_ok => app.panel = 4,
        Some("holestats") if app.ui_ok => {
            app.info.stats_hole = (1..19).find(|&h| app.club.holes[h].par != 0).unwrap_or(1);
            app.screen = Screen::HoleStats;
        }
        Some("sgaoffer") if app.ui_ok => {
            // the offer over the report, whatever the course scores
            app.club.game |= sg_core::tournament::OFFERED;
            app.begin_tournament();
            if let Some(s) = app.sga.as_mut() {
                s.offer = true;
                s.report.purse = s.report.purse.max(20 * s.report.holes);
            }
        }
        Some("results") if app.ui_ok => {
            // a tournament played out at once: every field golfer scores par give or take a stroke
            app.club.game |= sg_core::tournament::OFFERED;
            app.accept_tournament();
            app.club.tourney_opts = 0;
            let mut rng = app.exe_rng;
            for s in 0..36 {
                let g = &mut app.club.g[s];
                g.hole = 0;
                for h in 1..19 {
                    let par = app.club.holes[h].par;
                    g.card[h] = if par == 0 { 0 } else { (par + rng.below(3) - 1) as i8 };
                }
            }
            if let Some(res) = app.club.tournament_tick(&mut rng, &app.course) {
                app.results = Some(res);
                app.screen = Screen::Results;
            }
        }
        Some("prep") if app.ui_ok => {
            // the tournament recommendations: a tournament begun at once and its checklist shown
            app.club.game |= sg_core::tournament::OFFERED;
            app.accept_tournament();
            let prep = app.club.tournament_prep();
            let n = prep.lines().len();
            app.prep = Some((prep, (1 << n) - 1));
            app.screen = Screen::Prep;
        }
        Some("bestscores") if app.ui_ok => app.screen = Screen::BestScores,
        Some("top10") if app.ui_ok => app.open_top10(None),
        Some("world") if app.ui_ok => app.open_world_map(),
        Some("sga") if app.ui_ok => {
            app.club.game |= sg_core::tournament::OFFERED;
            app.begin_tournament();
        }
        Some("skills") if app.ui_ok => {
            let pts = app.club.first_skill_points();
            app.open_skills(pts, Some(false));
            if let Some(d) = app.skill_dialog.as_mut() {
                d.intro = true;
            }
        }
        // an accomplishment's three points: the card over the trophy room
        Some("skillsaward") if app.ui_ok => {
            app.open_skills(3, None);
            if let Some(d) = app.skill_dialog.as_mut() {
                d.award = true;
            }
        }
        // the golfer card of the newest golfer on the course (card) or of the n-th listed (card:n); its story page
        // (cardstory); Customise Golfer (customise), with its face picker open (customisefaces)
        Some(s) if app.ui_ok && (s.starts_with("card") || s.starts_with("customise")) => {
            let n = s.split(':').nth(1).and_then(|v| v.parse::<usize>().ok()).unwrap_or(0);
            let mut list = app.listed_golfers();
            if s.starts_with("cardstory") {
                list.retain(|&g| app.club.g[g].story != -1);
            }
            if s.starts_with("customise") {
                // customise:n opens it from the n-th listed golfer's card instead of the Player panel
                match list.get(n).filter(|_| s.contains(':')) {
                    Some(&g) => app.open_customise(g),
                    None => app.open_customise_player(),
                }
                if let Some(c) = app.cust.as_mut().filter(|_| s == "customisefaces") {
                    c.picker = Some(0);
                }
            } else if let Some(&g) = list.get(n.min(list.len().saturating_sub(1))) {
                app.card = Some(g);
                app.card_ui.story = s.starts_with("cardstory");
                if s.starts_with("cardsnap") {
                    // the card's Take Snapshot: the SimFoto of that golfer
                    app.card = None;
                    let (x, z) = app.units_to_world(app.club.g[g].x, app.club.g[g].y);
                    app.cam_x = x;
                    app.cam_z = z;
                    app.simfoto = Some(screens_ui::SimFoto { slot: g, frames: 0, saved: false });
                }
            }
        }
        Some(s) if app.ui_ok => {
            app.test_title_screen(s, true);
        }
        _ => {}
    }
}

/// Test hooks for scripted stills of the dock panels: SG_PANEL_CLICKS="x,y;x,y" clicks the open panel at those 800 x 600
/// points in turn, SG_PANEL_MOUSE="x,y" leaves the pointer there with its tooltip showing at once.
fn panel_test_hooks(app: &mut App) {
    if let Ok(clicks) = std::env::var("SG_PANEL_CLICKS") {
        for c in clicks.split(';').map(ints).filter(|v| v.len() >= 2) {
            app.pstate.mouse = (c[0] as f32, c[1] as f32);
            app.panel_click(c[0] as f32, c[1] as f32);
        }
    }
    if let Some(v) = std::env::var("SG_PANEL_MOUSE").ok().map(|s| ints(&s)).filter(|v| v.len() >= 2) {
        app.pstate.mouse = (v[0] as f32, v[1] as f32);
        app.pstate.still = true;
        app.dock_hover = dock_hit(v[0] as f32, v[1] as f32);
        app.info.pointer = (v[0] as f32, v[1] as f32);
        app.dock_tip = (app.dock_hover, 12);
    }
    // SG_BUILD=kind arms the building tool with that exe object kind (stills of the pointer's previews), SG_DESIGN=n picks
    // design n of a garden item as its strip would
    if let Some(k) = std::env::var("SG_BUILD").ok().and_then(|v| v.parse::<usize>().ok()) {
        app.edit = true;
        app.tool = 4;
        app.build_idx = k.min(19);
        app.arm_design(app.build_idx as i32);
        if let Some(d) = std::env::var("SG_DESIGN").ok().and_then(|v| v.parse::<i32>().ok()) {
            app.design = Some((app.build_idx as i32, d));
            app.design_hover = d;
        }
    }
    // SG_PAINT=type arms the terrain brush that paints that tile type (0 the tee, 1 the green)
    if let Some(k) = std::env::var("SG_PAINT").ok().and_then(|v| v.parse::<i32>().ok()).and_then(|t| PAINT.iter().position(|p| p.ty == t)) {
        app.edit = true;
        app.tool = 0;
        app.paint_idx = k;
    }
}

/// Test hook for stills of the roster: SG_ROSTER_FILL="n[,scroll]" gives the first n golfer records a round, a level, scores
/// and a few hole marks (made-up values, for the layout only) and scrolls the list by `scroll` rows.
fn roster_fill(app: &mut App) {
    let Some(v) = std::env::var("SG_ROSTER_FILL").ok().map(|s| ints(&s)) else { return };
    let n = (v.first().copied().unwrap_or(0).max(0) as usize).min(app.club.members.len());
    for (i, m) in app.club.members.iter_mut().take(n).enumerate() {
        m.rounds = 1 + (i as i32 * 7) % 23;
        m.best = (70 + (i * 5) % 30) as u8;
        m.avg = ((i * 3) % 25) as i8;
        m.level = [1, 2, 3, 4, 2, 0][i % 6];
        m.gone = if i % 9 == 4 { 0xff } else { 0 };
        for h in 1..19 {
            m.holes[h] = match (i * 5 + h * 3) % 17 {
                0 => 1,
                1 => 2,
                2 if m.gone == 0xff => 4,
                3 => 5,
                _ => 0,
            };
        }
    }
    app.roster_offset = v.get(1).copied().unwrap_or(0).max(0) as usize;
}

/// Test hook for stills of the tool under the pointer: SG_CURSOR_TILE="x,y" (in tiles, fractions allowed: "10.5,12.5" is a tile
/// centre, "10,12" a corner) holds the course pointer there.
fn cursor_pin() -> Option<(f32, f32)> {
    static PIN: std::sync::OnceLock<Option<(f32, f32)>> = std::sync::OnceLock::new();
    *PIN.get_or_init(|| {
        let v: Vec<f32> = std::env::var("SG_CURSOR_TILE").ok()?.split(',').filter_map(|t| t.trim().parse().ok()).collect();
        (v.len() >= 2).then(|| (v[0], v[1]))
    })
}

/// The ground point SG_CURSOR_TILE holds the pointer on, as a world point.
fn pin_world(app: &App) -> Option<(f32, f32)> {
    let (x, y) = cursor_pin()?;
    let t = sg_core::terrain::TILE_SIZE;
    Some((x * t - app.terrain.w as f32 * t * 0.5, y * t - app.terrain.h as f32 * t * 0.5))
}

fn apply_edit_spec(app: &mut App, spec: &str) {
    for item in spec.split(';').filter(|s| !s.is_empty()) {
        let (kind, rest) = (item.as_bytes()[0], item.get(2..).unwrap_or(""));
        let v = ints(rest);
        let at = |k: usize| v.get(k).copied().unwrap_or(0);
        match kind {
            b'p' if item.len() > 2 && v.len() >= 3 => {
                let (a, b, c, d, r) = (at(0), at(1), at(2), at(3), at(4));
                app.adopt_holes = true;
                for dy in -r..=r {
                    for dx in -r..=r {
                        if dx * dx + dy * dy <= r * r + r {
                            app.terrain.paint(a + dx, b + dy, c, d);
                            if matches!(c, 0 | 1 | 17 | 22) {
                                app.terrain.flatten_tile(a + dx, b + dy);
                            }
                        }
                    }
                }
            }
            b'w' if item.len() > 2 && v.len() >= 3 => {
                let (a, b, c, d) = (at(0), at(1), at(2), at(3));
                if app.terrain.path_kind.len() != app.terrain.ty.len() {
                    app.terrain.path_kind = vec![0; app.terrain.ty.len()];
                }
                for dy in -d..=d {
                    for dx in -d..=d {
                        let (x, y) = (a + dx, b + dy);
                        if dx * dx + dy * dy <= d * d + d && app.terrain.inside(x, y) {
                            let i = app.terrain.tile_index(x, y);
                            app.terrain.path_kind[i] = c as u8;
                        }
                    }
                }
            }
            b'd' => {
                // ASCII map of the tile types: '#' out of bounds, 'C' the clubhouse door, '~' water, 'T' woods, '.' rough,
                // '=' fairway, 'o' green, 't' tee, '*' other
                let door = app.course.door;
                for y in 0..app.terrain.h {
                    let row: String = (0..app.terrain.w)
                        .map(|x| match ((x, y) == door, app.terrain.type_at(x, y)) {
                            (true, _) => 'C',
                            (_, 20) => '#',
                            (_, 17 | 23 | 24 | 25) => '~',
                            (_, 13) => 'T',
                            (_, 4 | 5) => '.',
                            (_, 2 | 3) => '=',
                            (_, 1) => 'o',
                            (_, 0) => 't',
                            (_, 22) => 'B',
                            _ => '*',
                        })
                        .collect();
                    println!("map {y:3} {row}");
                }
                for e in app.employees.iter().filter(|e| e.active) {
                    println!("employee job {} at tile {},{}", e.job, e.x >> 10, e.y >> 10);
                }
                if let Some(l) = &app.land {
                    for o in l.objects.iter().filter(|o| o.kind >= 0) {
                        println!("object kind {} at {},{} on type {}", o.kind, o.a, o.b, app.terrain.type_at(o.a, o.b));
                    }
                }
            }
            b'b' if item.len() > 2 && (3..=4).contains(&v.len()) => {
                app.build_idx = at(2).clamp(0, 19) as usize;
                if v.len() == 4 {
                    app.design = Some((app.build_idx as i32, at(3)));
                }
                app.edit_building(at(0), at(1), false);
                println!(
                    "building {} at {},{}: {} objects, toast '{}', cash {}",
                    sg_core::land::building_name(app.build_idx as i32),
                    at(0),
                    at(1),
                    app.land.as_ref().map(|l| l.objects.iter().filter(|o| o.kind >= 0).count()).unwrap_or(0),
                    app.toast,
                    app.econ.cash
                );
            }
            b'h' if item.len() > 2 && !v.is_empty() => {
                // a fourth value hires the experienced version (bit 1) and arms the Employee panel's Move (bit 2)
                let ok = app.hire_employee(at(0).clamp(0, 3) as usize, at(3) & 1 != 0);
                if ok && v.len() >= 3 {
                    if let Some(e) = app.employees.iter_mut().rev().find(|e| e.active) {
                        e.post = Some((at(1), at(2)));
                    }
                }
                if ok && at(3) & 2 != 0 {
                    app.moving_employee = app.employees.iter().rposition(|e| e.active);
                }
                println!("hired staff kind {}: {ok}", at(0));
            }
            b'e' if item.len() > 2 && v.len() >= 3 => {
                // raise (or lower) the vertices of a square of side 2r + 1 round x,y by n steps, for stills of the steps
                let (x, y, n, r) = (at(0), at(1), at(2), at(3));
                for dy in -r..=r {
                    for dx in -r..=r {
                        app.terrain.raise_corner(x + dx, y + dy, n);
                    }
                }
                app.dirty = true;
            }
            b't' if item.len() > 2 && v.len() >= 3 => {
                // paint one tile through the hole tool (type 0 tee, 1 green)
                app.paint_idx = PAINT.iter().position(|p| p.ty == at(2)).unwrap_or(app.paint_idx);
                let r = std::mem::replace(&mut app.brush, 0);
                app.sync_course();
                app.edit_paint(at(0), at(1));
                app.brush = r;
                app.sync_course();
            }
            b'u' if v.len() >= 2 => {
                // test hook: the undo tool's right click on tile x,y, then the open holes and the tiles' hole numbers
                app.sync_course();
                app.undo_tile(at(0), at(1));
                let numbered = app.course.flags.iter().filter(|&&f| f & 0x1f != 0 && f & 0x400 == 0).count();
                println!(
                    "undo {},{}: type {}, open holes {:?}, next hole {}, numbered tiles {numbered}",
                    at(0),
                    at(1),
                    app.terrain.type_at(at(0), at(1)),
                    app.hole_numbers,
                    app.club.next_hole
                );
            }
            b'l' if item.len() > 2 && !v.is_empty() => {
                // buy tract n (1..9) through the land screen
                app.open_land_screen();
                let (col, row) = ((at(0) - 1) / 3, (at(0) - 1) % 3);
                app.land_pointer(20.0 + 260.0 * col as f32, 60.0 + 68.0 * row as f32);
                app.land_click(false);
            }
            b'j' => {
                // the pro's round: j:0 practice, j:1 the match; scripted runs aim every full shot at the pin (j:0,1 only
                // shows the aim; j:0,3 leaves the aiming to the player)
                let ui = std::mem::replace(&mut app.ui_ok, false);
                app.auto_aim = match v.get(1).map(|_| at(1)) {
                    Some(1) => 2,
                    Some(3) => 0,
                    _ => 1,
                };
                app.pro_button(if at(0) == 1 { 1 } else { 0 });
                app.ui_ok = ui;
                println!("pro round requested: game flags {:#x}, skills {:?}", app.club.game, &app.club.pro_skill[..10]);
            }
            b'y' if v.len() >= 2 => {
                // test hook: a home site at x,y bought at once by a celebrity
                let theme = app.exe_theme();
                if let Some(l) = app.land.as_mut() {
                    let i = l.place(&mut app.exe_rng, at(0), at(1), sg_core::land::K_HOME_SITE, 0, theme);
                    l.objects[i].val = 1_000_000;
                    l.write_area(&mut app.terrain, at(0), at(1), at(0) + 1, at(1) + 1);
                    if let Some(o) = app.club.celebrity_home(&mut app.exe_rng, 1_000_000, 0, (at(0), at(1))) {
                        app.land.as_mut().unwrap().objects[i].sub = o;
                    }
                }
                app.after_object_change();
                println!("celebrity home at {},{}: {} residents", at(0), at(1), app.club.residents.len());
            }
            b'R' => {
                // test hook: the Set par extra asks for par N on the Hole Stats' hole (its confirmation box); "R:N,1" sets it
                if at(1) != 0 {
                    let ok = app.club.set_par(app.info.stats_hole.clamp(1, 18), at(0));
                    println!("set par {}: {ok}", at(0));
                } else {
                    app.ask_par(at(0));
                }
            }
            b'X' => {
                // test hook: the game's extras (sg_core::extras bits), not saved to the settings
                let e = sg_core::extras::Extras::from_bits(at(0) as u32);
                app.club.extras = e;
                app.econ.extras = e;
                println!("extras {e:?}");
            }
            b'S' => {
                // test hook: the landmark tool in hand with page P of its strip (extras_ui), and design D when given
                // the Amenities panel (the Build Course panel's other side)
                if app.panel != 1 {
                    app.open_panel(1);
                }
                app.pstate.alt = true;
                app.tool = 4;
                app.edit = true;
                app.build_idx = sg_core::land::K_LANDMARK as usize;
                app.strip_page = at(0) as u8;
                app.design = (v.len() >= 2).then(|| (sg_core::land::K_LANDMARK, at(1)));
            }
            b'T' => {
                // test hook: the Rotate extra's Tab with the pointer on tile x,y
                let (x, z) = app.terrain.tile_centre(at(0), at(1));
                app.has_hit = true;
                app.hit_x = x;
                app.hit_z = z;
                let done = app.extras_tab();
                println!("extras tab at {},{}: {done}", at(0), at(1));
            }
            b'x' => app.save_championship_course(),
            b'q' => app.save_championship_pro(),
            b'A' => app.auto_aim = 1, // scripted runs: the pro aims every full shot at the pin
            b'M' => {
                let kind = match at(0) {
                    0 => popup_ui::PopupKind::Info,
                    1 => popup_ui::PopupKind::System,
                    2 => popup_ui::PopupKind::Prefs,
                    4 => popup_ui::PopupKind::Retire,
                    5 => popup_ui::PopupKind::Enhanced,
                    6 => popup_ui::PopupKind::Extras,
                    // M:7 the Preferences' search box (SG_PREFS_SEARCH="words" types them, SG_PREFS_CHANGED=1 shows only
                    // what changed)
                    7 => {
                        app.pref_query = std::env::var("SG_PREFS_SEARCH").unwrap_or_default();
                        popup_ui::PopupKind::Search
                    }
                    8 => popup_ui::PopupKind::ResetOriginal,
                    _ => popup_ui::PopupKind::LandOffer,
                };
                app.open_popup(kind);
                // M:5,P opens the Enhanced graphics box on page P, M:6,P the Extras box
                if let Some(p) = app.popup.as_mut().filter(|_| v.len() >= 2) {
                    p.set_page(at(1).clamp(0, p.page_count() as i32 - 1) as u8);
                }
                if let Some(p) = app.popup.as_mut().filter(|p| p.kind == popup_ui::PopupKind::Search) {
                    p.changed_only = std::env::var_os("SG_PREFS_CHANGED").is_some();
                    p.refresh();
                }
            }
            b'm' if v.len() >= 2 => app.move_hole(at(0) as usize, at(1) as usize),
            // test hook: photo mode (photo.rs), P:LOOK,BLUR,VIGNETTE,ZOOM,HINT,SHOOT
            b'P' => photo::test_hook(app, &v),
            b'a' if v.len() >= 3 => {
                // test hook: an animal of kind v[2] and a follower at x,y
                app.club.wildlife.spawn(&mut app.exe_rng, at(0), at(1), at(2));
            }
            b'g' => {
                // a tournament: g:0 as the SGA offers it (the evaluation must pass), g:1 straight away with the default purse
                app.club.game |= sg_core::tournament::OFFERED;
                app.auto_aim = 1;
                if at(0) >= 1 {
                    // g:2 is for stills of the leader board and the Professional Tournament scorecard: a pro with no
                    // skills gets one point of Accurate Driver and every accomplishment counts as earned already, so no
                    // skill points card or trophy room covers the course
                    if at(0) == 2 {
                        if app.club.pro_skill.iter().all(|&v| v == 0) {
                            app.club.pro_skill[2] = 1;
                        }
                        app.club.awards = u32::MAX;
                    }
                    app.accept_tournament();
                } else {
                    let ui = std::mem::replace(&mut app.ui_ok, false);
                    app.begin_tournament();
                    app.ui_ok = ui;
                }
            }
            // test hooks: the instant shot analysis from tile a,b ('/' at the pointer), and hole h's flag word
            b'/' if v.len() >= 2 => {
                app.start_analysis(Some((at(0), at(1))), true);
                app.analysis_shown = 12;
                println!(
                    "shot analysis of hole {} from {},{}: {:?}",
                    app.analysis_hole,
                    at(0),
                    at(1),
                    app.analysis.as_ref().map(|a| a.sums)
                );
            }
            b'F' if v.len() >= 2 => app.club.holes[at(0).clamp(0, 19) as usize].flags = at(1) as u32,
            // test hook: the landmarks the club owns and those it may place free (bit masks by type)
            b'K' if v.len() >= 2 => {
                app.club.landmarks_owned = at(0) as u32;
                app.club.free_landmarks = at(1) as u32;
            }
            // test hook: the board's debt counter and the cash in units
            b'D' if item.len() > 2 && v.len() >= 2 => {
                app.econ.debt_stage = at(0);
                app.econ.cash = at(1) as f64 * sg_core::economy::Economy::UNIT;
            }
            b'B' | b'L' if item.len() > 2 && v.len() >= 3 => {
                // the terrain brush of type `type` (sg_core::terrain) over a rectangle B:x0,y0,x1,y1,type or the fill
                // L:x,y,type, through the brush's own paint (build_tools.rs), then what it cost
                let ty = if kind == b'B' { at(4) } else { at(2) };
                app.paint_idx = PAINT.iter().position(|p| p.ty == ty && p.vbyte == 0).unwrap_or(app.paint_idx);
                app.sync_course();
                let area = if kind == b'B' { app.rect_area((at(0), at(1)), (at(2), at(3))) } else { app.fill_area((at(0), at(1))) };
                let (n, cost) = build_tools::area_cost(&area.tiles, |x, y| app.paint_cost_at(x, y));
                let cash = app.econ.cash;
                app.paint_area(&area);
                app.sync_course();
                println!(
                    "area {item}: {} tiles, {n} to change for {} units, too large {}, cash {cash} -> {}",
                    area.tiles.len(),
                    cost,
                    area.too_large,
                    app.econ.cash
                );
            }
            b'o' => {
                let ok = app.open_hole();
                println!("open hole: {ok}, next hole {}", app.club.next_hole);
            }
            b'r' if item.len() > 2 && v.len() >= 3 => {
                let (a, b, c, d) = (at(0), at(1), at(2), at(3));
                for dy in -d..=d {
                    for dx in -d..=d {
                        if dx * dx + dy * dy <= d * d + d {
                            app.terrain.raise_corner(a + dx, b + dy, c);
                        }
                    }
                }
            }
            _ => eprintln!("bad --edit item: {item}"),
        }
    }
}

impl Stage {
    fn new(o: Options) -> Stage {
        let mut g = Gfx::new();
        // the content packs first: their art, sounds and data replace the game's before anything is read (an App deals
        // its first property offer from the data when it is made)
        gamefiles::set_headless(o.png_out.is_some());
        let mut content = content::startup(&o.game_dir, o.base.as_deref());
        // a game folder with some of the original's files but not the course textures (a copy cut short, on a phone most of
        // all): the free base set rather than closing at the first theme (load_theme below)
        let theme_dir = |t: usize| sg_core::fsutil::resolve(&o.game_dir, &format!("Data/Textures/{}", app::THEMES[t]));
        if o.base.is_none() && content.forced_base.is_none() && sg_core::terrain::TextureCatalog::load(&theme_dir(o.theme)).is_err() {
            eprintln!("game folder {}: no course textures, playing the free base set", o.game_dir.display());
            content = content::startup(&o.game_dir, Some(sg_core::packs::FREE));
            content.forced_base = Some(sg_core::packs::FREE.to_string());
        }
        let mut app = match o.clock {
            Some(c) => App::with_clock(o.game_dir.clone(), c),
            None => App::new(o.game_dir.clone()),
        };
        println!("game folder: {} ({:?})", o.game_dir.display(), o.game_how);
        // nothing found anywhere (no --game): no complaint about a folder the player never named, but once a friendly hint
        // on the title screen of how to use the original files (gamefiles.rs)
        if o.game_how != gamefiles::How::Given && content.forced_base.is_some() && cfg!(not(target_arch = "wasm32")) {
            content.notice = None;
            app.free_hint = cfg!(not(target_os = "android")) && gamefiles::take_hint();
        }
        app.content = content;
        app.theme = o.theme;
        app.seed = o.seed;
        app.zoom = o.zoom;
        app.rot = o.rot;
        app.time = o.time;
        app.follow = o.follow;
        app.mute = o.mute;
        app.sound_log = o.sound_log;
        app.econ.sandbox = o.sandbox;
        app.course_size = o.course_size;
        if let Some(d) = o.difficulty {
            app.difficulty = d;
        }
        if let Some(c) = o.cash {
            app.econ.start_cash = c;
        }
        if let Some(b) = o.extras {
            app.extras_next = sg_core::extras::Extras::from_bits(b);
        }
        if let Some(name) = &o.golfer {
            // Play as a golfer from progolfers.dta (case-insensitive name match).
            let pros = sg_core::fsutil::read_file(app.game_path("Themes/Standard/progolfers.dta"))
                .ok_or_else(|| "cannot read progolfers.dta".to_string())
                .and_then(|d| parse_pro_golfers(&sg_core::formats::latin1(&d)));
            let pros = pros.unwrap_or_else(|e| {
                eprintln!("error: cannot read progolfers.dta ({e})");
                std::process::exit(1)
            });
            let low = name.to_lowercase();
            let Some(hit) = pros.iter().find(|p| p.name.to_lowercase().contains(&low)) else {
                eprintln!("no golfer matching '{name}'");
                std::process::exit(1)
            };
            // the player's pro takes the famous golfer's ten skills (the shot planner reads them from the pro's record)
            for (k, &v) in hit.skill.iter().enumerate() {
                app.club.pro_skill[k] = v.clamp(0, 15) as u8;
            }
            println!("golfer: {}, skills {}", hit.name, hit.skill.iter().map(|s| format!("{s:X}")).collect::<Vec<_>>().join(" "));
        }
        let info = g.ctx.info();
        println!("GL: {:?}", info.backend);
        // a driver that cannot run the game (Windows' OpenGL 1.1 stand-in) ends the start with a message, not a dead window
        #[cfg(not(any(target_arch = "wasm32", target_os = "android")))]
        sysdiag::check_gl();

        let mut mixer = Mixer::new(&app.game_path("Sounds"));
        mixer.add_dir("simsfx", &app.game_path("SimsFX"));
        // the free set's sounds stand in for a copy whose own cannot be played; the player is told once (sound_note)
        mixer.with_fallback();
        let sound_note = mixer.check().err().filter(|_| content::has_original(&app.game_dir)).map(|e| {
            audio::log(&format!("sound: the game's own sounds cannot be played ({e}); playing the free set's sounds"));
            sg_core::lang::trf("STR_SOUND_CANNOT_PLAY", &[("WHY", &e)])
        });
        // what the sound plays from, in the log and in a browser's console (audio::log)
        let own = !app.on_free_set() && content::has_original(&app.game_dir);
        let count = mixer.list("").len();
        audio::log(&if mixer.is_empty() {
            format!("sound: no sounds at all ({}): the game is silent", mixer.last_error())
        } else if own && !mixer.all_free() {
            format!(
                "sound: {count} sounds from your SimGolf files{}",
                if sound_note.is_some() { " (some cannot be played: the free set's stand in)" } else { "" }
            )
        } else if own {
            format!("sound: your SimGolf files hold no sounds; playing the {count} free OpenSGolf sounds")
        } else {
            format!("sound: {count} free OpenSGolf sounds")
        });
        if mixer.is_empty() {
            eprintln!("sound: {}", mixer.last_error());
        } else {
            let mixer = Arc::new(mixer);
            app.mixer = Some(mixer.clone());
            if !app.mute && o.png_out.is_none() {
                app.audio = audio::open(mixer);
                if app.audio.is_none() {
                    audio::log("sound: no audio device, running silent");
                }
            }
        }
        app.terrain = Terrain::demo_course(40, 40, app.seed);
        app.land = None;
        // the graphics mode, before any art is loaded: Classic unless HD was chosen (--hd or the saved choice) and a pack exists
        if let Some(d) = &o.hd_pack {
            app.hd.pack_dir = d.clone();
        }
        let hd_msg = app.init_hd(o.hd.unwrap_or_else(hd::saved_choice));
        // Enhanced graphics (enhanced.rs), also before any art: the textures carry its filtering
        enhanced::init(o.enhanced);
        // the OpenSGolf extras' ticks (extras.rs)
        extras::init(o.no_extras);
        // the player's keys (controls.rs)
        controls::init();
        // the effects (fx.rs) follow Enhanced's ticks; --fx / --fx-set / OPENSGOLF_FX turn them on for this run
        fx::configure(o.fx, o.fx_set.as_deref());
        if !app.load_theme(&mut g, app.theme) {
            std::process::exit(1);
        }
        if !load_ui(&mut app, &mut g) {
            eprintln!("ui: could not load the Interface art or KLEPTO__.TTF, starting on the course");
        }
        app.load_story();
        app.swaps = sg_core::bodies::Swaps::load(&app.game_path("Bodies"));
        if let Some(h) = std::env::var("SG_HOVER").ok().and_then(|v| v.parse().ok()) {
            app.hover = h; // test hook: a scripted still with the pointer over a menu item
        }
        {
            let scripted = o.png_out.is_some()
                || o.course.is_some()
                || !o.edit_spec.is_empty()
                || o.save.is_some()
                || o.load_game.is_some()
                || o.golfer.is_some()
                || o.sandbox
                || o.follow
                || o.scenario.is_some();
            let s = o.screen.clone().unwrap_or_else(|| if scripted { "play".into() } else { "menu".into() });
            if app.ui_ok {
                match s.as_str() {
                    "report" => app.screen = Screen::Report,
                    "menu" => app.screen = Screen::Menu,
                    "property" => app.screen = Screen::Property,
                    "difficulty" => app.screen = Screen::Difficulty,
                    "scenarios" => app.open_scenarios(),
                    "land" => app.open_land_screen(),
                    s => {
                        app.test_title_screen(s, false);
                    }
                }
            }
        }
        if let Some(name) = &o.property {
            let k = name.parse::<usize>().ok().filter(|&k| k < 16).or_else(|| {
                let n = name.to_lowercase();
                sg_core::data::properties()
                    .iter()
                    .position(|p| [sg_core::lang::en(p.name_key), p.name()].iter().any(|s| s.to_lowercase().starts_with(&n)))
            });
            let Some(k) = k else {
                eprintln!("error: unknown property {name}");
                std::process::exit(1)
            };
            app.course_size = o.course_size;
            app.deal_offer(o.sandbox);
            if !o.sandbox {
                // the chooser only sells what the starting money covers: move the property into a cheap slot (slot 0 is
                // kept for Parkland) so a scripted start looks like a real one
                app.cheap_slot(k);
            }
            app.start_game(&mut g, k, o.sandbox);
            if let Some(c) = o.cash {
                // the property's price came out of the default funds; the test hook sets the cash after it
                app.econ.start_cash = c;
                app.econ.cash = c;
            }
            open_screen(&mut app, o.screen.as_deref());
        }
        if let Some(id) = &o.scenario {
            if let Err(e) = app.start_scenario_by_id(&mut g, id) {
                eprintln!("error: {e}");
                std::process::exit(1);
            }
            open_screen(&mut app, o.screen.as_deref());
        }
        if let Some(f) = &o.course {
            match Terrain::load(f, &app.terrain) {
                Ok(t) => app.terrain = t,
                Err(e) => {
                    eprintln!("error: {e}");
                    std::process::exit(1);
                }
            }
            app.rebuild_batches(&mut g);
            app.refresh_trees();
        }
        if let Some((course, pro)) = &o.champ {
            let p = sg_core::fsutil::read_file(pro).and_then(|b| sg_core::championship::parse_pro(&b)).unwrap_or_else(|| {
                eprintln!("error: {} is not a pro file", pro.display());
                std::process::exit(1)
            });
            if let Err(e) = app.start_championship(&mut g, course, &p) {
                eprintln!("error: {e}");
                std::process::exit(1);
            }
        }
        if let Some(f) = &o.load_game {
            if let Err(e) = app.load_game(&mut g, f) {
                eprintln!("error: {e}");
                std::process::exit(1);
            }
            println!("loaded game {} at tick {}", f.display(), app.game_tick);
            open_screen(&mut app, o.screen.as_deref());
        }
        // started again on a saved game's content packs (content_profiles.rs): that game is loaded now
        if let Some(f) = gamefiles::take_load_after_restart() {
            match app.load_game(&mut g, &f) {
                Ok(()) => println!("loaded game {} at tick {}", f.display(), app.game_tick),
                Err(e) => app.show_toast(&sg_core::lang::trf("STR_TOAST_COULD_NOT_LOAD_GAME", &[("ERROR", &e)])),
            }
        }
        apply_edit_spec(&mut app, &o.edit_spec);
        if !o.edit_spec.is_empty() {
            app.rebuild_batches(&mut g);
            app.refresh_trees();
        }
        // after the game is set up (starting one closes the panels)
        if let Some(p) = o.panel {
            app.panel = 0;
            if (1..=3).contains(&p) {
                app.open_panel(p);
            } else {
                app.panel = p;
            }
            app.dock_hover = if (1..=3).contains(&p) { p - 1 } else { -1 };
            panel_test_hooks(&mut app);
        } else if std::env::var("SG_PANEL_MOUSE").is_ok() {
            panel_test_hooks(&mut app);
        }
        // SG_MESSAGE="speaker;text" posts a ticker message (speaker -1 nobody, a golfer slot, or a picture code), for stills
        if let Some((who, text)) = std::env::var("SG_MESSAGE").ok().as_deref().and_then(|v| v.split_once(';')) {
            app.post_message(text, 1, who.trim().parse().unwrap_or(message_ui::NOBODY));
        }
        // SG_EXPORT_SCENARIO=1 exports the game as a scenario pack (the Save dialog's button), for tests
        if std::env::var_os("SG_EXPORT_SCENARIO").is_some() {
            app.export_scenario();
        }
        if std::env::var_os("SG_PAUSED").is_some() {
            app.paused = true;
        }
        // SG_TAPE and SG_AREA: the measuring tape and the brush's rectangle and fill, for stills (build_tools.rs)
        build_tools::test_hooks(&mut app);
        // SG_STAFF_VIEW=1 shows the staff areas (Shift+A, staffview_ui.rs), for stills
        if std::env::var_os("SG_STAFF_VIEW").is_some() {
            app.comfort.staff_view = true;
        }
        if let Some(f) = &o.save {
            if let Err(e) = app.terrain.save(f) {
                eprintln!("error: {e}");
                std::process::exit(1);
            }
            println!("saved course {}", f.display());
        }
        if let Some((cx, cy)) = o.center {
            let (x, z) = app.terrain.tile_centre(cx, cy);
            app.cam_x = x;
            app.cam_z = z;
            app.photo_rebase_camera();
        }
        app.report_course(true);
        let t = now();
        app.clock = t;
        if let Some(msg) = hd_msg {
            app.show_toast(&msg);
        }
        if let Some(msg) = app.content.notice.take() {
            app.show_toast(&msg);
        }
        // the program started again on the graphics the player chose (gamefiles.rs)
        if let Some(msg) = gamefiles::take_notice() {
            app.show_toast(&msg);
            // the player's files were chosen, but the game could not read them on this start: why, in a box
            if let Some(why) = app.files_not_loaded(&msg) {
                app.show_notice(&why);
            }
        }
        // the player's copy has sounds the game cannot play: said in a box once (until the reason changes), in the
        // ticker on every start
        if let Some(msg) = sound_note {
            app.show_toast(&msg);
            if o.png_out.is_none() && gamefiles::take_once("sound_problem_shown", &msg) {
                app.show_notice(&msg);
            }
        }
        // the last run crashed: the report, to copy and send (android.rs)
        #[cfg(target_os = "android")]
        if let Some(report) = android::take_crash_report() {
            app.show_crash_report(report);
        }
        // "Update now" started this version: the old files it kept are removed, and it says so (autoupdate.rs)
        #[cfg(all(not(target_arch = "wasm32"), feature = "net"))]
        if let Some(v) = autoupdate::finish_after_restart(&app::save_dir(), update::VERSION) {
            app.show_toast(&sg_core::lang::trf("STR_UPDATE_DONE", &[("VERSION", &v)]));
        }
        #[cfg(target_os = "android")]
        let _ = std::fs::remove_dir_all(android::files_dir().join("update"));
        // the interface extra "Check for updates when the game starts" (update.rs), not for scripted runs; test hook:
        // SG_UPDATE_URL answers "Check for updates" from a file, at once (as the extra's check when it is ticked)
        let test_url = std::env::var_os(update::TEST_URL_ENV).is_some();
        if test_url && !extras::update_check() {
            app.check_for_updates();
        } else if o.png_out.is_none() || test_url {
            app.check_for_updates_at_start();
        }
        // test hook: SG_PICK_FOLDER=DIR takes DIR as the folder picked in "Use my SimGolf files..." ("free": Use the free
        // graphics), as if chosen on the title screen
        if std::env::var_os(gamefiles::PICK_ENV).is_some() {
            app.choose_game_folder(false);
        }
        Stage {
            g,
            app,
            fullscreen: o.fullscreen,
            pad: Default::default(),
            png_out: o.png_out,
            save_game_out: o.save_game,
            frames: 0,
            editing: false,
            dragging: false,
            touch: Default::default(),
            paused_total: 0.0,
            pause_start: 0.0,
            last_tick: t,
            mouse: (0.0, 0.0),
            shift: false,
            ctrl: false,
            alt: false,
            ctrl_click: false,
            swallow_char: false,
            wheel: (0.0, 0.0),
            pause_toggle: false,
            shown_version: 0,
        }
    }

    fn toggle_pause(&mut self) {
        let t = now();
        self.app.paused = !self.app.paused;
        // the player's own pause or unpause: no longer one the comfort pauses made (comfort.rs)
        self.app.comfort.paused_for = None;
        if self.app.paused {
            self.pause_start = t;
        } else {
            self.paused_total += t - self.pause_start;
        }
    }

    /// What a popup menu's choice does (docs/DECODE_MENUS.md 3, 4, 5).
    fn popup_done(&mut self, r: popup_ui::PopupResult) {
        let Some(kind) = self.app.popup.as_ref().map(|p| p.kind) else { return };
        // the career's end takes any answer but the first (Esc too) as the main menu
        let r = if kind == popup_ui::PopupKind::CareerOver { Some(r.unwrap_or(1)) } else { r };
        let Some(k) = r else {
            self.app.close_popup();
            return;
        };
        if k == usize::MAX {
            return; // refused or toggled: the popup stays
        }
        // the OpenSGolf extras' lines of the Information menu go by their text (they follow the goals line when it is there)
        let line = self.app.popup.as_ref().and_then(|p| p.lines.get(k + 1).cloned()).unwrap_or_default();
        // the Enhanced graphics, Extras and search boxes' ticks, across their pages
        let pref = self.app.popup.as_ref().map(|p| p.pref.clone());
        let query = self.app.popup.as_ref().map(|p| p.query.clone()).unwrap_or_default();
        self.app.close_popup();
        use popup_ui::PopupKind;
        match kind {
            PopupKind::Info if line == tr("STR_MENU_INFO_PEOPLE") => self.app.open_people(),
            PopupKind::Info if line == tr("STR_MENU_INFO_HISTORY") => self.app.open_history(),
            PopupKind::Info if line == tr("STR_MENU_INFO_PATH_VIEW") => self.app.toggle_path_view(),
            PopupKind::Info if line == tr("STR_MENU_INFO_LANDING_MAP") => self.app.toggle_landing_map(),
            PopupKind::Info if line == tr("STR_MENU_INFO_HOLE_COMMENTS") => self.app.open_hole_comments(),
            PopupKind::Info => match k {
                0 => self.app.repeat_message(),
                1 => self.open_report(),
                2 => self.app.open_report_screen(Screen::Comments),
                3 => self.app.open_routing(),
                4 => self.app.open_report_screen(Screen::Histograph),
                5 => self.app.sga_report(),
                6 => self.app.open_report_screen(Screen::Finance),
                7 => {
                    self.app.roster_offset = 0;
                    self.app.screen = Screen::Roster;
                }
                8 => {
                    self.app.screen = Screen::Board;
                    self.app.screen_jingle(0x7e, Screen::Board);
                }
                9 => self.app.open_world_map(),
                10 => self.app.screen = Screen::BestScores,
                11 => self.app.open_top10(None),
                // the port's own line, shown when the game has goals
                12 => self.app.open_goals(),
                _ => {}
            },
            PopupKind::LandOffer => {
                if k == 0 {
                    self.app.open_land_screen();
                }
            }
            // the Set par extra's confirmation (extras_ui.rs): Yes changes the par, back on Hole Stats either way
            PopupKind::SetPar => {
                if k == 0 {
                    self.app.confirm_par();
                }
            }
            PopupKind::QuickLoad => {
                if k == 0 {
                    self.app.quick_load(&mut self.g);
                }
            }
            PopupKind::CareerOver => {
                if k == 0 {
                    self.app.econ.continue_in_sandbox();
                } else {
                    self.app.end_career();
                }
            }
            PopupKind::System => match k {
                0 => self.app.open_save(),
                1 => self.app.open_files(files_ui::ListKind::Load, true),
                2 => {
                    if self.app.club.game & sg_core::golfer::game::TOURNAMENT != 0 {
                        self.app.cancel_tournament();
                    } else {
                        let mut rng = self.app.exe_rng;
                        self.app.club.cancel_pro_round(&mut rng);
                        self.app.exe_rng = rng;
                    }
                }
                3 => self.app.save_championship_pro(),
                4 => self.app.open_rename(),
                5 => self.app.open_popup(PopupKind::Prefs),
                6 => self.app.save_championship_course(),
                7 => self.app.open_popup(PopupKind::Retire),
                8 if extras::sharing() => self.app.share_course(),
                _ => {}
            },
            // Preferences > Enhanced graphics: the ticks become the settings (enhanced.rs)
            PopupKind::Enhanced => {
                if let Some(p) = pref {
                    if self.app.apply_enhanced(&mut self.g, p.enh) {
                        self.app.ui_ok &= load_art(&mut self.app, &mut self.g);
                    }
                }
            }
            // Preferences > Search (prefs_search.rs): OK puts what was ticked in use; a key found opens Change keys on it, and
            // Reset to original asks first (the ticks so far kept either way)
            PopupKind::Search => {
                if let Some(p) = &pref {
                    self.apply_prefs(p);
                }
                if k == popup_ui::SEARCH_RESET {
                    self.app.pref_query = query;
                    self.app.open_popup(PopupKind::ResetOriginal);
                } else if k >= popup_ui::SEARCH_KEY {
                    // Change keys with the action found waiting for its new key
                    let back = self.app.screen;
                    self.app.open_keys(back);
                    self.app.open_remap_on(k - popup_ui::SEARCH_KEY);
                }
            }
            // the search box's Reset to original: Yes puts everything back as in the original game; the search box opens
            // again either way, on the words typed
            PopupKind::ResetOriginal => {
                if k == 0 {
                    println!("preferences reset to the original game");
                    self.apply_prefs(&prefs_search::Prefs::original());
                }
                self.app.open_popup(PopupKind::Search);
            }
            // the ordinary quit leaves the way the career's end does (DECODE_WORLD2 6.5); EXACT (main loop after
            // 0x420b4e): "save first" runs the Save dialog (0x405b10) and then quits whether or not a game was saved,
            // "So long" quits, "Wait" and Esc go back to the game
            PopupKind::Retire => match k {
                1 => {
                    self.app.open_save();
                    self.app.title.quit_after_save = true;
                }
                2 => self.app.end_career(),
                _ => {}
            },
            // Preferences > OpenSGolf extras: the gameplay ticks become the game's extras (extras_ui.rs), the interface ones the
            // interface extras' settings, saved (extras.rs)
            PopupKind::Extras => {
                if let Some(p) = pref {
                    self.apply_extras(p.ex, p.ui);
                }
            }
            PopupKind::Prefs => {
                let m = k as u32;
                if m & 1 << popup_ui::PREFS_CONTENT != 0 {
                    self.app.open_content();
                }
                self.app.show_thoughts = m & 1 != 0;
                self.app.show_advisor = m & 2 != 0;
                self.app.club.wildlife.enabled = m & 4 != 0;
                if self.app.switch_hd(&mut self.g, m & 16 != 0) {
                    self.app.ui_ok &= load_art(&mut self.app, &mut self.g);
                }
                // the port's own Enhanced graphics tick: its master switch, the options under it kept
                let mut enh = enhanced::settings();
                enh.enabled = m & 1 << popup_ui::PREFS_ENHANCED_ON != 0;
                if self.app.apply_enhanced(&mut self.g, enh) {
                    self.app.ui_ok &= load_art(&mut self.app, &mut self.g);
                }
                if m & 1 << popup_ui::PREFS_ENHANCED != 0 && m & 1 << popup_ui::PREFS_CONTENT == 0 {
                    self.app.open_popup(PopupKind::Enhanced);
                }
                // the port's own Keyboard controls line (keys_ui.rs)
                if m & 1 << popup_ui::PREFS_KEYS != 0 {
                    let back = self.app.screen;
                    self.app.open_keys(back);
                }
                // the port's own OpenSGolf extras line (extras_ui.rs, extras.rs)
                if m & 1 << popup_ui::PREFS_EXTRAS != 0 {
                    self.app.open_popup(PopupKind::Extras);
                }
                // the port's own Search line (prefs_search.rs), on no words
                if m & 1 << popup_ui::PREFS_SEARCH != 0 {
                    self.app.pref_query.clear();
                    self.app.open_popup(PopupKind::Search);
                }
                let sound = m & 8 != 0;
                if sound == self.app.mute {
                    self.app.mute = !sound;
                    if self.app.mute {
                        if let Some(mx) = &self.app.mixer {
                            mx.stop_all();
                        }
                        self.app.ambience = -1;
                        self.app.title_music = -1;
                    }
                    self.app.music_screen = None;
                }
                // the port's own "Use my SimGolf files..." line, after the ticks are kept (gamefiles.rs)
                if m & 1 << popup_ui::PREFS_FILES_FLAG != 0 {
                    if let Some((a, _)) = self.app.files_option() {
                        self.app.files_action_in_game(a);
                    }
                }
            }
        }
    }

    /// The OpenSGolf extras chosen in Preferences: the gameplay ones become the game's (and the next game's) extras, the
    /// interface ones are put in use; both are saved.
    fn apply_extras(&mut self, ex: sg_core::extras::Extras, ui: extras::Settings) {
        self.app.set_extras(ex);
        extras::set(ui);
        extras::save(&ui);
        if !ui.path_help {
            self.app.xs.path_view = false;
        }
        if !ui.landing_map {
            self.app.xs.insight.map = false;
        }
    }

    /// The search box's settings put in use and saved (prefs_search.rs): Enhanced graphics, the extras, the keys.
    fn apply_prefs(&mut self, p: &prefs_search::Prefs) {
        if self.app.apply_enhanced(&mut self.g, p.enh) {
            self.app.ui_ok &= load_art(&mut self.app, &mut self.g);
        }
        self.apply_extras(p.ex, p.ui);
        if p.keys != controls::settings() {
            controls::save(p.keys.clone());
        }
    }

    fn open_report(&mut self) {
        self.app.report_course(true);
        if self.app.ui_ok && self.app.report_art.tex.is_some() {
            self.app.screen = Screen::Report;
        }
    }

    /// Returns true when the click was on the dock or an open panel.
    fn dock_click(&mut self, vx: f32, vy: f32, right: bool) -> bool {
        let app = &mut self.app;
        if !app.ui_ok || app.dock_art.tex.is_none() {
            return false;
        }
        let art_panel = app.art_panel_open();
        // the hire dialog is modal
        if art_panel && app.pstate.hire_open {
            return app.panel_click(vx, vy);
        }
        let d = dock_hit(vx, vy);
        if d >= 0 {
            match d {
                0..=2 => app.open_panel(d + 1),
                3 => app.zoom_step(true),
                4 => app.zoom_step(false),
                // the Rotate Map buttons send Home and PageUp
                5 => app.rotate_view(-1),
                6 => app.rotate_view(1),
                7 => app.open_popup(popup_ui::PopupKind::Info),
                8 => self.pause_toggle = true,
                _ => app.open_popup(popup_ui::PopupKind::System),
            }
            self.app.snd("Interface/Button1.wav", 1.0, false);
            return true;
        }
        if app.panel == 4 && !art_panel {
            if app.golfers_click(vx, vy) {
                return true;
            }
            if Rect::new(226.0, 452.0, 570.0, 144.0).has(vx, vy) {
                return true;
            }
        }
        if art_panel {
            return app.panel_click(vx, vy);
        }
        if app.panel != 0 {
            let items = app.panel_items();
            let cols = app.panel_cols();
            for (i, it) in items.iter().enumerate() {
                if panel_item_rect(i, cols).has(vx, vy) {
                    match it.kind {
                        0 => {
                            app.tool = 0;
                            app.paint_idx = it.arg;
                            app.edit = true;
                        }
                        1 => {
                            app.tool = 2;
                            app.path_kind = it.arg as i32;
                            app.edit = true;
                        }
                        2 | 3 => {
                            app.tool = 1;
                            app.raise_sign = if it.kind == 2 { 1 } else { -1 };
                            app.edit = true;
                        }
                        4 => {
                            app.tool = 4;
                            app.build_idx = it.arg;
                            app.edit = true;
                        }
                        6 => app.pro_button(it.arg),
                        9 => {
                            app.panel = 4;
                            app.golfer_page = 0;
                        }
                        10 => app.open_panel(3),
                        7 => app.club.set_shot_option(SHOT_OPTS[it.arg]),
                        8 => {}
                        _ => {
                            if right {
                                app.fire_staff(it.arg);
                            } else {
                                app.hire_staff(it.arg);
                            }
                        }
                    }
                    app.snd("Interface/Button2.wav", 1.0, false);
                    return true;
                }
            }
            if Rect::new(226.0, 452.0, 570.0, 144.0).has(vx, vy) {
                return true; // the panel background swallows clicks
            }
        }
        false
    }

    fn dock_hover_update(&mut self, vx: f32, vy: f32) {
        let app = &mut self.app;
        app.dock_hover = dock_hit(vx, vy);
        app.pstate.mouse = (vx, vy);
        app.pstate.still = false;
        app.panel_hover = -1;
        if app.panel != 0 {
            let cols = app.panel_cols();
            for i in 0..app.panel_items().len() {
                if panel_item_rect(i, cols).has(vx, vy) {
                    app.panel_hover = i as i32;
                }
            }
        }
    }

    fn menu_pointer(&mut self, x: f32, y: f32, click: bool) {
        let (vx, vy) = self.app.view.to_virtual(x, y);
        let app = &mut self.app;
        let mut hit = -1;
        if app.screen == Screen::Difficulty {
            hit = difficulty_hit(vx, vy);
        } else if app.screen == Screen::Menu {
            for (b, r) in MENU_BTN.iter().enumerate() {
                if r.has(vx, vy) {
                    hit = b as i32;
                }
            }
            // the logo's hover region (docs/DECODE_MENUS.md 8) opens the credits (DERIVED)
            if hit < 0 && Rect::new(168.0, 198.0, 472.0, 176.0).has(vx, vy) {
                hit = 6;
            }
            // the port's own: "Use my SimGolf files..." / "Use the free graphics" (gamefiles.rs)
            if let Some((_, label)) = app.files_option() {
                if gamefiles::files_button_rect(label).has(vx, vy) {
                    hit = gamefiles::FILES_HOVER;
                }
            }
            // the port's own: "Check for updates" (update.rs)
            if app.update_button().is_some_and(|r| r.has(vx, vy)) {
                hit = update::UPDATE_HOVER;
            }
        } else {
            hit = app.world_hit(vx, vy);
        }
        app.hover = hit;
        if !click || (hit < 0 && app.screen != Screen::Difficulty) {
            return;
        }
        if app.screen == Screen::Difficulty {
            if hit == render::COURSE_SIZE_HIT {
                if app.course_size_choosable() {
                    app.next_course_size();
                    app.ui_sound(0x18);
                }
                return;
            }
            if hit == render::STYLE_HIT {
                app.next_game_style(&mut self.g);
                app.ui_sound(0x18);
                return;
            }
            if hit == 100 || hit < 0 {
                // the back button, or a click on nothing, goes back (0x43a400 returns -1)
                app.screen = Screen::Menu;
                app.scen.chosen = None;
            } else if let Some(k) = app.scen.chosen {
                // a scenario that names no difficulty (scenario_ui)
                app.difficulty = hit;
                app.start_scenario(&mut self.g, k);
            } else if std::mem::take(&mut app.title.champ_pending) {
                app.difficulty = hit;
                // the new game's Top 10 id is drawn right after the difficulty (0x421518)
                app.course_id = Some(app.exe_rng.below(0xffff));
                app.open_championship();
            } else {
                app.difficulty = hit;
                app.course_id = Some(app.exe_rng.below(0xffff));
                app.deal_offer(app.sandbox_choice);
                app.screen = Screen::Property;
            }
            app.hover = -1;
            return;
        }
        if app.screen == Screen::Menu {
            match hit {
                0 => app.open_files(files_ui::ListKind::Load, false),
                // with scenario packs installed, Start New Game first asks: a free game or a scenario (the port's own)
                1 if app.has_scenarios() => app.open_scenarios(),
                1 | 2 | 4 => {
                    // Start New Game, Sandbox Mode and Play a Championship ask for the difficulty first (0x43a400)
                    app.sandbox_choice = hit == 2;
                    app.title.champ_pending = hit == 4;
                    app.screen = Screen::Difficulty;
                    app.hover = -1;
                }
                3 => app.open_themes(),
                6 => app.open_credits(),
                gamefiles::FILES_HOVER => {
                    if let Some((a, _)) = app.files_option() {
                        app.hover = -1;
                        app.files_action(a);
                    }
                }
                update::UPDATE_HOVER => app.check_for_updates(),
                _ => quit_game(),
            }
        } else {
            app.world_click(&mut self.g, hit);
        }
    }

    /// The course screen's keys, as the exe's two key switches have them (character keys at 0x41c5b2, virtual keys at
    /// 0x41de29): lower-case letters pick tools, shifted letters open screens, digits jump to holes, arrows scroll four tiles,
    /// PageUp / Home turn the view a quarter, z / x zoom between the three levels.
    fn play_key(&mut self, k: KeyCode) {
        // the player's keys (controls.rs): the key pressed becomes the default key of the action bound to it
        let aiming = self.app.club.pro_aiming().is_some();
        let Some((k, shift, act)) = controls::translate(k, self.shift, aiming) else { return };
        // the port's own measuring tape (build_tools.rs)
        if act == Some(controls::action("tape")) {
            let at = self.pointer_ground();
            self.app.tape_key_down(at);
            return;
        }
        // Esc first puts the tape or a rectangle under way away
        if k == KeyCode::Escape && self.app.build_tools_cancel() {
            return;
        }
        // the port's own: the whole interface hidden or shown again (u; controls_ui.rs)
        if act == Some(controls::action("hide_interface")) {
            self.app.ctl.hud_hidden = !self.app.ctl.hud_hidden;
            return;
        }
        // Enhanced graphics' photo mode (photo.rs): c, a key the original leaves unused; nothing in Classic
        if k == KeyCode::C && !shift {
            self.photo_enter();
            return;
        }
        let app = &mut self.app;
        // a terrain key picks that brush on the Build Course panel (0x40d890 with the type itself)
        let pick = |app: &mut App, ty: u8| {
            if app.panel != 1 {
                app.open_panel(1);
            }
            app.edit = true;
            app.tool = 0;
            if let Some(i) = PAINT.iter().position(|p| p.ty == ty as i32 && p.vbyte == 0) {
                app.paint_idx = i;
            }
        };
        if act.map_or(aiming && !shift, controls::is_aim) {
            // while the pro aims, s f d h l pick the shot shape instead
            let opt = match k {
                KeyCode::S => Some(0),
                KeyCode::F => Some(-1),
                KeyCode::D => Some(1),
                KeyCode::H => Some(3),
                KeyCode::L => Some(4),
                _ => None,
            };
            if let Some(o) = opt {
                app.club.set_shot_option(o);
                return;
            }
        }
        if k == KeyCode::N && !shift && (app.club.gary > 0 || app.club.game & sg_core::golfer::game::TOURNAMENT != 0) {
            // 'n' cancels the pro's round after a question; here a second press answers it
            if app.game_tick <= app.cancel_until {
                app.pro_button(3);
                app.cancel_until = 0;
            } else {
                app.cancel_until = app.game_tick + 40;
                app.show_toast(tr("STR_TOAST_CANCEL_ROUND"));
            }
            return;
        }
        let hole_key = [
            KeyCode::Key1,
            KeyCode::Key2,
            KeyCode::Key3,
            KeyCode::Key4,
            KeyCode::Key5,
            KeyCode::Key6,
            KeyCode::Key7,
            KeyCode::Key8,
            KeyCode::Key9,
        ]
        .iter()
        .position(|&c| c == k);
        if let Some(n) = hole_key {
            // 1..9 and their shifted symbols (! @ # $ % ^ & * () jump to holes 1..18
            let h = n + 1 + if shift { 9 } else { 0 };
            let hr = &app.club.holes[h.min(18)];
            if hr.par != 0 {
                let (tx, ty) = (hr.back.0 * 1024 + 512, hr.back.1 * 1024 + 512);
                let (px, py) = (hr.pin.0 * 1024 + 512, hr.pin.1 * 1024 + 512);
                let (x, z) = app.units_to_world((tx + px) / 2, (ty + py) / 2);
                app.cam_x = x;
                app.cam_z = z;
            }
            return;
        }
        if shift {
            match k {
                KeyCode::S => app.open_save(),
                KeyCode::L => app.open_files(files_ui::ListKind::Load, true),
                KeyCode::P => self.toggle_pause(),
                KeyCode::T => app.show_props = !app.show_props,
                KeyCode::R => app.open_routing(),
                KeyCode::W if app.club.championship() => app.ui_sound(24),
                KeyCode::W => app.open_world_map(),
                KeyCode::J => app.begin_tournament(),
                KeyCode::C => app.save_championship_course(),
                KeyCode::N => app.show_names = !app.show_names,
                // the OpenSGolf extras' keys (extras.rs): People, the message history, name tags, the path view
                KeyCode::G if extras::people() => app.open_people(),
                KeyCode::M if extras::history() => app.open_history(),
                KeyCode::H => app.toggle_name_tags(),
                KeyCode::V if extras::path_help() => app.toggle_path_view(),
                // the port's own comfort keys (comfort.rs, staffview_ui.rs): quick save, quick load, the staff areas
                KeyCode::F5 => app.quick_save(),
                KeyCode::F9 => app.ask_quick_load(),
                KeyCode::A => app.toggle_staff_view(),
                // the insight extras (insight.rs): the landing map and comments by hole
                KeyCode::D if extras::landing_map() => app.toggle_landing_map(),
                KeyCode::I if extras::hole_comments() => app.open_hole_comments(),
                // ? repeats the last message
                KeyCode::Slash => app.repeat_message(),
                KeyCode::B => {
                    // the Home Site tool, "Sell lot for cash"
                    if app.panel != 2 {
                        app.open_panel(2);
                    }
                    app.edit = true;
                    app.tool = 4;
                    app.build_idx = sg_core::land::K_HOME_SITE as usize;
                }
                // into the save folder: an app started from the Finder or a desktop icon has "/" or a read-only folder
                // as its working directory
                KeyCode::F6 if self.screenshot(&save_dir().join("opensgolf-shot.png")) => {
                    println!("saved {}", save_dir().join("opensgolf-shot.png").display())
                }
                _ => {}
            }
            return;
        }
        match k {
            KeyCode::Escape if app.pstate.hire_open => app.pstate.hire_open = false,
            // the original's Esc opened the pause menu over an open golfer card; "Fix original bugs" closes the card
            // (docs/FIXES.md 7)
            KeyCode::Escape if app.card.is_some() && enhanced::fix_bugs() => app.card = None,
            KeyCode::Escape => {
                if app.ui_ok {
                    // the pause menu: System Functions over the stopped game
                    app.open_popup(popup_ui::PopupKind::System);
                    app.hover = -1;
                } else {
                    quit_game();
                }
            }
            // Space drops the tool and the selected golfer (and ends People > Follow)
            KeyCode::Space => {
                app.edit = false;
                app.card = None;
                app.xs.follow = None;
            }
            // the port's own game speed (the original has none): ] cycles normal, x2 and x4
            KeyCode::RightBracket => {
                app.speed = match app.speed {
                    1 => 2,
                    2 => 4,
                    _ => 1,
                };
                let name = match app.speed {
                    1 => tr("STR_TOAST_SPEED_NORMAL"),
                    2 => tr("STR_TOAST_SPEED_FAST"),
                    _ => tr("STR_TOAST_SPEED_FASTEST"),
                };
                app.show_toast(name);
            }
            // Tab turns the next building; with the elevation tool it also steps the brush (one point, 2x2, area)
            // the Rotate extra's Tab (extras_ui.rs) comes first; nothing when it is off
            KeyCode::Tab if app.extras_tab() => {}
            KeyCode::Tab => {
                app.turn = (app.turn + 1) & 3;
                // the exe's Tab adds one to the same variable that holds the paint brush's variant (0x5a34f0)
                if let Some((k, v)) = app.paint_variant {
                    app.paint_variant = Some((k, v + 1));
                }
                if app.edit && app.tool == 1 {
                    app.pstate.elev_tool = (app.pstate.elev_tool + 1) % 3;
                }
            }
            KeyCode::Minus | KeyCode::Equal | KeyCode::KpSubtract | KeyCode::KpAdd if app.has_hit => {
                let (a, b) = app.terrain.corner_of(app.hit_x, app.hit_z);
                let up = matches!(k, KeyCode::Equal | KeyCode::KpAdd);
                app.edit_elevation(a, b, if up { 1 } else { -1 });
            }
            KeyCode::Up | KeyCode::Down | KeyCode::Left | KeyCode::Right => {
                app.xs.follow = None;
                // four tiles along the view's diagonal
                let step = 4.0 * std::f32::consts::SQRT_2 * sg_core::terrain::TILE_SIZE;
                match k {
                    KeyCode::Up => app.key_pan(0.0, step),
                    KeyCode::Down => app.key_pan(0.0, -step),
                    KeyCode::Left => app.key_pan(-step, 0.0),
                    _ => app.key_pan(step, 0.0),
                }
            }
            // instant shot analysis: '/' of the hole nearest the pointer, '.' of the last one analysed
            KeyCode::Slash => app.start_analysis(None, true),
            KeyCode::Period => app.start_analysis(None, false),
            KeyCode::PageUp => app.rotate_view(1),
            KeyCode::Home => app.rotate_view(-1),
            KeyCode::Key0 => {
                // the view turned back; if it already was, centre on the clubhouse
                if app.rot.rem_euclid(360.0) == 0.0 {
                    let (a, b) = app.course.door;
                    let (x, z) = app.units_to_world(a * 1024 + 512, b * 1024 + 512);
                    app.cam_x = x;
                    app.cam_z = z;
                }
                let before = app.rot;
                app.rot = 0.0;
                app.turned(-before);
                app.dirty |= app.terrain.sand_phase != 0;
            }
            KeyCode::Z => app.zoom_step(true),
            KeyCode::X => app.zoom_step(false),
            KeyCode::F => pick(app, TT_FAIRWAY),
            KeyCode::R => pick(app, if app.exe_theme() == 1 { sg_core::terrain::TT_DEEP_ROUGH } else { TT_ROUGH }),
            KeyCode::G => {
                // green first, then the tee on a second press
                let on_green = app.edit && app.tool == 0 && PAINT[app.paint_idx].ty == TT_PUTTING_GREEN as i32;
                pick(app, if on_green { TT_TEE } else { TT_PUTTING_GREEN });
                app.turn = 0;
            }
            KeyCode::S => pick(app, TT_SAND),
            KeyCode::T => pick(app, sg_core::terrain::TT_WOODS),
            KeyCode::V => pick(app, sg_core::land::T_RAVINE),
            KeyCode::W => pick(app, TT_WATER_SHALLOW),
            KeyCode::P => {
                pick(app, TT_FAIRWAY);
                app.tool = 2;
            }
            KeyCode::B => {
                if app.panel != 2 {
                    app.open_panel(2);
                }
                app.edit = true;
                app.tool = 4;
                app.build_idx = sg_core::land::K_BENCH as usize;
            }
            KeyCode::E => {
                if app.panel != 2 {
                    app.open_panel(2);
                }
                app.pstate.alt = true;
                app.edit = true;
                app.tool = 1;
            }
            KeyCode::H => {
                if !app.open_hole() {
                    app.show_toast(tr("STR_TOAST_HOLE_NEEDS_TEE_GREEN"));
                }
            }
            KeyCode::M => app.toggle_music(),
            KeyCode::F1 => self.open_report(),
            KeyCode::F2 => app.open_report_screen(Screen::Comments),
            KeyCode::F3 => app.open_report_screen(Screen::Histograph),
            KeyCode::F4 => app.open_report_screen(Screen::Finance),
            KeyCode::F5 => app.open_routing(),
            KeyCode::F6 => app.open_world_map(),
            KeyCode::F7 => app.sga_report(),
            KeyCode::F8 => app.open_report_screen(Screen::Shortcuts),
            KeyCode::F9 => {
                app.roster_offset = 0;
                app.screen = Screen::Roster;
            }
            KeyCode::F10 => {
                app.screen = Screen::Board;
                app.screen_jingle(0x7e, Screen::Board);
            }
            _ => {}
        }
    }

    fn draw_frame(&mut self, target: Option<RenderPass>) {
        let app = &mut self.app;
        // black behind everything: the exe fills the frame with colour 0x80000000 before Terrain::render (0x4498a0), and the
        // land outside the property (type 20) is not drawn, so it shows black
        let clear = PassAction::clear_color(0.0, 0.0, 0.0, 1.0);
        self.g.ctx.begin_pass(target, clear);
        enhanced::begin_frame();
        // OpenSGolf extras: People > Follow keeps the camera on its target (extras.rs)
        app.follow_tick();
        app.camera_frame(app.clock);
        app.count_exe_frames();
        // the title screens set Klepto ITC (difficulty, the file lists and Pick A Pro, the theme packs); the rest of the game
        // draws in Manual SSi and Arial
        let title = app.ui_ok && matches!(app.screen, Screen::Menu | Screen::Difficulty | Screen::Files | Screen::Themes);
        // the info screens (reports, roster, SGA, land and the like) draw with their own two fonts
        let info = app.ui_ok
            && matches!(
                app.screen,
                Screen::Report
                    | Screen::HoleStats
                    | Screen::Comments
                    | Screen::Histograph
                    | Screen::Finance
                    | Screen::Routing
                    | Screen::Shortcuts
                    | Screen::Roster
                    | Screen::Board
                    | Screen::YearEnd
                    | Screen::Sga
                    | Screen::Results
                    | Screen::Land
                    | Screen::Pair
                    | Screen::BestScores
            );
        ui::set_face(title.then_some(ui::Face::Klepto));
        // Enhanced widescreen: the title screens framed on a wide window (enhanced.rs; nothing in Classic)
        let title_page = app.ui_ok
            && matches!(
                app.screen,
                Screen::Files | Screen::Themes | Screen::Credits | Screen::Menu | Screen::Property | Screen::Difficulty | Screen::Scenarios
            );
        if title_page && enhanced::widescreen() {
            enhanced::backdrop(&mut self.g, &ui::Screen::new(app.draw_w, app.draw_h), false);
        }
        if matches!(app.screen, Screen::Files | Screen::Themes | Screen::Credits) && app.ui_ok {
            app.draw_title_screen(&mut self.g);
        } else if app.screen == Screen::Menu && app.ui_ok {
            app.draw_menu(&mut self.g);
        } else if app.screen == Screen::Property && app.ui_ok {
            app.draw_property(&mut self.g);
        } else if app.screen == Screen::Difficulty && app.ui_ok {
            app.draw_difficulty(&mut self.g);
        } else if app.screen == Screen::Scenarios && app.ui_ok {
            app.draw_menu(&mut self.g);
            ui::set_face(None);
            app.draw_scenarios(&mut self.g);
        } else if app.photo.is_some() {
            // Enhanced graphics' photo mode: the course alone (photo.rs)
            self.draw_photo(target);
        } else {
            // with the effects' post-processing the course is drawn offscreen and composited before the interface (fx.rs)
            let post = fx::active() && app.fx_world_begin(&mut self.g, target);
            app.render_world(&mut self.g);
            if post {
                app.fx_world_end(&mut self.g, target);
            }
            app.draw_building_label(&mut self.g);
            if app.great_shot.is_none() {
                app.draw_cursor_overlay(&mut self.g);
            }
            if app.hud_hidden() {
                // the interface hidden with u (controls_ui.rs): only a note on how to bring it back
                app.draw_hud_hint(&mut self.g);
            } else {
                if !app.no_hud {
                    app.draw_hud(&mut self.g);
                }
                // Enhanced graphics: the camera button that opens photo mode (photo.rs; nothing in Classic)
                app.draw_photo_button(&mut self.g);
            }
            if app.screen == Screen::Popup {
                // the Set par extra's confirmation stays over the Hole Stats it came from (extras_ui.rs)
                if app.ui_ok && app.popup.as_ref().is_some_and(|p| p.kind == popup_ui::PopupKind::SetPar) {
                    ui::set_face(Some(ui::Face::Info));
                    app.draw_hole_stats(&mut self.g);
                    ui::set_face(title.then_some(ui::Face::Klepto));
                }
                app.draw_popup(&mut self.g);
            }
            if !matches!(app.screen, Screen::People | Screen::Profile) {
                app.draw_rename(&mut self.g);
            }
            app.draw_save_dialog(&mut self.g);
            // Enhanced widescreen: a full-screen page over the course is framed, the course dimmed round it
            if enhanced::widescreen()
                && app.ui_ok
                && matches!(app.screen, Screen::Board | Screen::Land | Screen::Routing | Screen::Pair | Screen::Top10 | Screen::Customise)
            {
                enhanced::backdrop(&mut self.g, &ui::Screen::new(app.draw_w, app.draw_h), true);
            }
            // only the screen over the course takes the info fonts, not the HUD under it
            ui::set_face(info.then_some(ui::Face::Info));
            if app.screen == Screen::Report {
                app.draw_report(&mut self.g);
            }
            if app.screen == Screen::Land {
                app.draw_land(&mut self.g);
            }
            if app.screen == Screen::Skills {
                app.draw_skills(&mut self.g);
            }
            if matches!(app.screen, Screen::Sga | Screen::Prep | Screen::Results) {
                app.draw_tourney_screen(&mut self.g);
            }
            match app.screen {
                Screen::Pair => app.draw_pair_screen(&mut self.g),
                Screen::Board => app.draw_board(&mut self.g),
                Screen::YearEnd => app.draw_year_end(&mut self.g),
                Screen::Roster => app.draw_roster(&mut self.g),
                Screen::Comments => app.draw_comments(&mut self.g),
                Screen::Histograph => app.draw_histograph(&mut self.g),
                Screen::Finance => app.draw_finance(&mut self.g),
                Screen::Routing => app.draw_routing(&mut self.g),
                Screen::Shortcuts => app.draw_shortcuts(&mut self.g),
                Screen::Content => app.draw_content(&mut self.g),
                Screen::Keys => app.draw_keys(&mut self.g),
                Screen::Goals => app.draw_goals(&mut self.g),
                Screen::HoleStats => app.draw_hole_stats(&mut self.g),
                Screen::BestScores => app.draw_best_scores(&mut self.g),
                Screen::Top10 => app.draw_top10(&mut self.g),
                Screen::Customise => app.draw_customise(&mut self.g),
                // the OpenSGolf extras' screens (extras.rs); the rename prompt (an employee's name, a bio) goes over them
                Screen::People => {
                    app.draw_people(&mut self.g);
                    app.draw_rename(&mut self.g);
                }
                Screen::History => app.draw_history(&mut self.g),
                Screen::HoleComments => app.draw_hole_comments(&mut self.g),
                Screen::Profile => {
                    app.draw_profile(&mut self.g);
                    app.draw_rename(&mut self.g);
                }
                _ => {}
            }
            app.draw_analysis(&mut self.g);
        }
        // the OpenSGolf extras' tab strip over the F1 to F10 reports (report_tabs.rs)
        self.app.draw_report_tabs(&mut self.g);
        self.app.draw_notice(&mut self.g);
        self.draw_touch_buttons();
        self.draw_gamepad_pointer();
        self.g.flush();
        self.g.ctx.end_render_pass();
        enhanced::end_frame();
    }

    /// Renders the current frame offscreen and writes it as a PNG.
    fn screenshot(&mut self, file: &PathBuf) -> bool {
        let img = self.grab();
        sg_core::png::write_png(file, &img)
    }

    /// An accomplishment's snapshot (0x46e810): 200 x 160 of the screen around the award's map point, kept for the board
    /// and written to snapshots/accomp<id>.png. When the point is off screen the view moves there first.
    fn take_snapshot(&mut self) {
        let Some(&(id, pt)) = self.app.snapshot_due.first() else { return };
        // any screen of a game in progress will do: the grab draws the course alone
        if matches!(
            self.app.screen,
            Screen::Menu | Screen::Scenarios | Screen::Difficulty | Screen::Property | Screen::Files | Screen::Themes | Screen::Credits
        ) || !self.app.ui_ok
        {
            return;
        }
        // When the point is off screen the view looks there for the grab only (a followed golfer would pull a lasting camera
        // move straight back, and the snapshot would never be taken).
        let cam = (self.app.cam_x, self.app.cam_z);
        if self.app.screen_of(pt.0, pt.1).is_none() {
            let (x, z) = self.app.units_to_world(pt.0, pt.1);
            self.app.cam_x = x;
            self.app.cam_z = z;
        }
        self.app.snapshot_due.remove(0);
        // the snapshot is of the course, so a board that opened first is set aside for the grab
        let shown = std::mem::replace(&mut self.app.screen, Screen::Play);
        self.app.no_hud = true;
        let img = self.grab();
        let at = self.app.screen_of(pt.0, pt.1);
        let v = self.app.view;
        self.app.no_hud = false;
        self.app.screen = shown;
        (self.app.cam_x, self.app.cam_z) = cam;
        let (vx, vy) = at.unwrap_or((400.0, 300.0));
        let (left, top) = ((vx - 100.0).clamp(0.0, 600.0), (vy - 100.0).clamp(0.0, 440.0));
        let mut out = sg_core::assets::Rgba::new(200, 160);
        for y in 0..160u32 {
            for x in 0..200u32 {
                let sx = ((left + x as f32) * v.scale + v.ox) as u32;
                let sy = ((top + y as f32) * v.scale + v.oy) as u32;
                if sx < img.w && sy < img.h {
                    let si = ((sy * img.w + sx) * 4) as usize;
                    let di = ((y * 200 + x) * 4) as usize;
                    out.px[di..di + 4].copy_from_slice(&img.px[si..si + 4]);
                }
            }
        }
        sg_core::png::write_png(self.app.snapshot_dir().join(format!("accomp{id}.png")), &out);
        let tex = self.g.texture(&out, false);
        self.app.snapshots.insert(id, ui::Image { tex: Some(tex), w: 200.0, h: 160.0 });
    }

    /// Writes the SimFoto once it is on screen: the photo's inside and a little of its frame ((left - 2, top - 2) and 6 more
    /// than the inside each way, as 0x431d20 is passed) to snapshots/<golfer>-<date>.png, with the shutter sound (0x95).
    fn save_simfoto(&mut self) {
        let Some(f) = self.app.simfoto else { return };
        if f.saved || f.frames < 2 {
            return;
        }
        let Some((l, t, r, b)) = self.app.simfoto_rect(&f) else { return };
        let out = self.grab_rect(l - 2, t - 2, (r - l + 6).max(1) as u32, (b - t + 6).max(1) as u32);
        let name = format!("{}-{}.png", self.app.club.name(f.slot), self.app.simfoto_date());
        let name: String = name.chars().map(|c| if "/\\:*?\"<>|".contains(c) { '_' } else { c }).collect();
        let dir = self.app.snapshot_dir();
        sg_core::png::write_png(dir.join(name), &out);
        self.app.ui_sound(0x95);
        if let Some(s) = self.app.simfoto.as_mut() {
            s.saved = true;
        }
    }

    /// The screen's (x0, y0) w x h (800 x 600 pixels) as it is drawn now.
    fn grab_rect(&mut self, x0: i32, y0: i32, w: u32, h: u32) -> sg_core::assets::Rgba {
        let img = self.grab();
        let v = self.app.view;
        let mut out = sg_core::assets::Rgba::new(w, h);
        for y in 0..h {
            for x in 0..w {
                let sx = ((x0 as f32 + x as f32) * v.scale + v.ox) as u32;
                let sy = ((y0 as f32 + y as f32) * v.scale + v.oy) as u32;
                if sx < img.w && sy < img.h {
                    let si = ((sy * img.w + sx) * 4) as usize;
                    let di = ((y * w + x) * 4) as usize;
                    out.px[di..di + 4].copy_from_slice(&img.px[si..si + 4]);
                }
            }
        }
        out
    }

    /// F10 while the great-shot SimFoto waits (0x407e00): the photo's inside and 2 pixels round it to
    /// snapshots/<golfer><tick>.png (the exe writes snapshots\<name><tick>.bmp through 0x475580).
    fn save_great_shot(&mut self) {
        let Some(gs) = self.app.great_shot.clone().filter(|gs| gs.held && gs.shown) else { return };
        let (l, t, r, b) = gs.rect;
        let out = self.grab_rect(l - 42, t - 42, (r - l + 84) as u32, (b - t + 84) as u32);
        let name = format!("{}{}.png", self.app.club.vip_name(gs.g), self.app.game_tick);
        let name: String = name.chars().map(|c| if "/\\:*?\"<>|".contains(c) { '_' } else { c }).collect();
        sg_core::png::write_png(self.app.snapshot_dir().join(name), &out);
    }

    fn grab(&mut self) -> sg_core::assets::Rgba {
        let (w, h) = (self.app.draw_w as u32, self.app.draw_h as u32);
        let params = |format| TextureParams { width: w, height: h, format, ..Default::default() };
        let color = self.g.ctx.new_render_texture(params(TextureFormat::RGBA8));
        // WebGL 2 refuses miniquad's unsized depth texture (the pass would be incomplete and the grab black), so the browser
        // grabs without a depth buffer (the course draws the same: tried natively, a few edge pixels differ)
        let depth = (!cfg!(target_arch = "wasm32")).then(|| self.g.ctx.new_render_texture(params(TextureFormat::Depth)));
        // HD only: an anti-aliased pass resolving into `color`
        let msaa = hd::msaa_pass(&mut self.g, color, w, h);
        let pass = msaa.map(|m| m.0).unwrap_or_else(|| self.g.ctx.new_render_pass(color, depth));
        self.draw_frame(Some(pass));
        for t in msaa.map(|m| m.1).into_iter().flatten() {
            self.g.ctx.delete_texture(t);
        }
        let mut raw = vec![0u8; (w * h * 4) as usize];
        self.g.ctx.texture_read_pixels(color, &mut raw);
        self.g.ctx.delete_render_pass(pass);
        self.g.ctx.delete_texture(color);
        if let Some(d) = depth {
            self.g.ctx.delete_texture(d);
        }
        let mut img = sg_core::assets::Rgba::new(w, h);
        let stride = (w * 4) as usize;
        for y in 0..h as usize {
            // GL origin is bottom-left
            img.px[y * stride..(y + 1) * stride].copy_from_slice(&raw[(h as usize - 1 - y) * stride..(h as usize - y) * stride]);
        }
        img.px.as_chunks_mut::<4>().0.iter_mut().for_each(|p| p[3] = 255);
        img
    }

    /// Draws `n` frames into an offscreen target and prints the mean time a frame took (the last read waits for the GPU).
    fn bench(&mut self, n: u32) {
        let (w, h) = (self.app.draw_w as u32, self.app.draw_h as u32);
        let params = |format| TextureParams { width: w, height: h, format, ..Default::default() };
        let color = self.g.ctx.new_render_texture(params(TextureFormat::RGBA8));
        let depth = self.g.ctx.new_render_texture(params(TextureFormat::Depth));
        let pass = self.g.ctx.new_render_pass(color, Some(depth));
        // one frame first (shaders compiled, targets made), then wait for it
        self.draw_frame(Some(pass));
        let mut raw = vec![0u8; (w * h * 4) as usize];
        self.g.ctx.texture_read_pixels(color, &mut raw);
        // the process's CPU time too (Linux): with a software renderer that is the drawing's cost, whatever else runs
        let cpu = || {
            let st = std::fs::read_to_string("/proc/self/stat").unwrap_or_default();
            let f: Vec<f64> = st.rsplit(')').next().unwrap_or("").split_whitespace().filter_map(|x| x.parse().ok()).collect();
            (f.get(11).copied().unwrap_or(0.0) + f.get(12).copied().unwrap_or(0.0)) * 10.0
        };
        let (t0, c0) = (now(), cpu());
        for _ in 0..n {
            self.draw_frame(Some(pass));
        }
        self.g.ctx.texture_read_pixels(color, &mut raw);
        let ms = (now() - t0) * 1000.0 / n as f64;
        let cms = (cpu() - c0) / n as f64;
        println!(
            "bench: {w}x{h}, effects {}, {ms:.2} ms a frame over {n} frames, {cms:.0} ms CPU",
            if fx::active() { "on" } else { "off" }
        );
        self.g.ctx.delete_render_pass(pass);
        self.g.ctx.delete_texture(color);
        self.g.ctx.delete_texture(depth);
    }

    /// Test hooks for the pointer after the first frame: SG_POINTER=x,y moves it to window pixel (x, y), SG_CLICK=x,y
    /// clicks the left button there, several clicks separated by ';' (for stills of hover looks and of where clicks land,
    /// e.g. with Enhanced widescreen).
    fn pointer_test_hook(&mut self) {
        // the extras' stills set up after the simulated time, before the scripted clicks
        self.app.extras_test_hook();
        let pts = |k: &str| -> Vec<(f32, f32)> {
            let v = std::env::var(k).unwrap_or_default();
            v.split(';').map(ints).filter(|v| v.len() >= 2).map(|v| (v[0] as f32, v[1] as f32)).collect()
        };
        for (x, y) in pts("SG_POINTER") {
            self.mouse_motion_event(x, y);
        }
        for (x, y) in pts("SG_CLICK") {
            self.mouse_motion_event(x, y);
            self.mouse_button_down_event(MouseButton::Left, x, y);
            self.mouse_button_up_event(MouseButton::Left, x, y);
            // the pointer is tested against the interface as last drawn
            self.draw_frame(None);
        }
        // SG_PRESS="u;Shift+F6;...": keys pressed after the clicks (names as settings.json's "controls" has them)
        for b in std::env::var("SG_PRESS").unwrap_or_default().split(';').filter_map(controls::Binding::parse) {
            let mods = KeyMods { shift: b.shift, ..Default::default() };
            self.key_down_event(b.key, mods, false);
            // a letter or Space also types its character, as a keyboard does
            let name = b.name();
            let ch = name.strip_prefix("Shift+").unwrap_or(&name);
            if let Some(c) = ch.chars().next().filter(|c| ch.len() == 1 && c.is_ascii_alphanumeric()) {
                self.char_event(if b.shift { c.to_ascii_uppercase() } else { c.to_ascii_lowercase() }, mods, false);
            } else if b.key == KeyCode::Space {
                self.char_event(' ', mods, false);
            }
            self.key_up_event(b.key, mods);
            self.draw_frame(None);
        }
    }

    fn pick_test(&mut self) {
        // Project known ground points to the screen and pick them back.
        let app = &self.app;
        let m = app.mv;
        let mut worst = 0f32;
        for wx in [-600.0f32, 0.0, 750.0] {
            for wz in [-500.0f32, 100.0, 900.0] {
                let y = app.terrain.height_at(wx, wz);
                let ex = m[0] * wx + m[4] * y + m[8] * wz + m[12];
                let ey = m[1] * wx + m[5] * y + m[9] * wz + m[13];
                let sx = (ex / app.upp + app.draw_w * 0.5).round();
                let sy = (app.draw_h * 0.5 - ey / app.upp).round();
                let err = app.pick_ground(sx, sy).map(|(px, pz)| (px - wx).hypot(pz - wz)).unwrap_or(9999.0);
                worst = worst.max(err);
                println!("  ({wx:.0},{wz:.0}) y={y:.1} -> err {err:.1}");
            }
        }
        println!("pick test: worst error {worst:.2} world units (1 px = {:.2})", app.upp);
    }
}

impl EventHandler for Stage {
    fn update(&mut self) {
        // macOS: the last frame's autoreleased Cocoa objects go (autorelease.rs)
        #[cfg(target_os = "macos")]
        autorelease::next_frame();
        if sysdiag::window_gone() {
            sysdiag::note("the window is gone while the main loop runs: ending");
            std::process::exit(0);
        }
        perf::frame_begin();
        let t = now();
        self.app.clock = t;
        self.touch_update();
        self.photo_update();
        if self.pause_toggle {
            self.pause_toggle = false;
            self.toggle_pause();
        }
        // the --screen test hook's extras screens that need the game to have run first (extras.rs)
        self.app.extras_test_hook();
        // "Use my SimGolf files...": a .zip being unpacked or a folder being copied (gamefiles.rs)
        self.app.poll_import();
        // "Check for updates": the answer from GitHub (update.rs), and "Update now" under way (autoupdate.rs)
        self.app.poll_update_check();
        #[cfg(all(not(target_arch = "wasm32"), feature = "net"))]
        self.app.poll_update_job();
        // the Autosave extra (autosave.rs) and a course code being pasted (share.rs)
        self.app.autosave_tick(t);
        self.app.share_tick(t);
        // a controller's sticks and buttons (gamepad.rs)
        self.gamepad_update(t);
        // the sound output follows the system's default device (audio.rs)
        if let Some(a) = self.app.audio.as_mut() {
            a.keep_alive();
        }
        let app = &mut self.app;
        // the board terminated the contract: the main loop asks what next as soon as the game is back on the course
        if app.econ.game_over && app.screen == Screen::Play && app.popup.is_none() {
            app.open_popup(popup_ui::PopupKind::CareerOver);
        }
        app.screen_audio();
        if app.screen == Screen::Play {
            let hit = app.pick_ground(self.mouse.0, self.mouse.1);
            app.hover_tile = hit.map(|(x, z)| app.terrain.tile_of(x, z));
            // the measuring tape's far end follows the pointer (build_tools.rs)
            app.tape_pointer(pin_world(app).or(hit));
            let (vx, vy) = app.view.to_virtual(self.mouse.0, self.mouse.1);
            match hit {
                _ if app.auto_aim != 0 => {}
                // the aiming frame runs while the pointer is above the panel (mouse y < 480)
                Some((x, z)) if app.club.pro_aiming().is_some() && vy < 452.0 && !(app.ui_ok && dock_hit(vx, vy) >= 0) => {
                    app.aim_pointer(x, z)
                }
                _ if app.club.pro_aiming().is_none() => app.aim = None,
                _ => {}
            }
        }
        if app.edit {
            let hit = pin_world(app).or_else(|| app.pick_ground(self.mouse.0, self.mouse.1));
            app.has_hit = hit.is_some();
            if let Some((x, z)) = hit {
                app.hit_x = x;
                app.hit_z = z;
            }
        }
        if app.dirty {
            let t0 = now();
            app.rebuild_batches(&mut self.g);
            app.refresh_trees();
            app.dirty = false;
            app.report_course(false);
            if std::env::var_os("SG_TICK_BENCH").is_some() {
                println!("rebuild: {:.1} ms for a {}x{} map", (now() - t0) * 1000.0, app.terrain.w, app.terrain.h);
            }
        }
        if app.econ.version != self.shown_version {
            self.shown_version = app.econ.version;
        }
        if app.reset_clock {
            self.paused_total = t;
            if self.png_out.is_none() {
                app.time = 0.0; // a scripted still keeps its --time
            }
            app.reset_clock = false;
        }
        // the club runs only on the course view and not while paused; a long stall (a dragged window) is not caught up
        let dt = (t - self.last_tick).clamp(0.0, 0.25);
        self.last_tick = t;
        if self.png_out.is_none()
            && !app.paused
            && app.screen == Screen::Play
            && app.rename.is_none()
            && app.title.save_name.is_none()
            && app.analysis.is_none()
            && app.simfoto.is_none_or(|f| f.frames < 2)
            && app.great_shot.is_none()
        {
            app.time += dt * app.speed.max(1) as f64 * perf::speed();
        }
        app.club.turbo = app.speed > 1;
        let (w, h) = window::screen_size();
        app.draw_w = w.max(1.0);
        app.draw_h = h.max(1.0);
        app.dpi = window::dpi_scale();
    }

    fn draw(&mut self) {
        match self.app.hd_frame_pass(&mut self.g) {
            Some(pass) => {
                self.draw_frame(Some(pass));
                self.app.hd_present(&mut self.g);
            }
            None => self.draw_frame(None),
        }
        self.g.ctx.commit_frame();
        if perf::on() {
            perf::frame_end();
            let app = &self.app;
            perf::report(app.game_tick, || app.perf_sizes());
            // a fast test run (OPENSGOLF_PERF_SPEED) plays on by itself: photos, questions and boards let go at once
            if perf::speed() > 1.0 {
                self.app.great_shot_dismiss();
                self.app.close_popup();
                self.app.notice = None;
                if self.app.ui_ok && self.app.screen != Screen::Play && self.app.photo.is_none() {
                    self.app.screen = Screen::Play;
                }
                self.app.paused = false;
            }
        }
        sysdiag::frame(self.app.draw_w, self.app.draw_h);
        self.app.great_shot_frame();
        // the frame rate cap (Enhanced graphics > Display; framerate.rs): the simulation keeps its own 87 ms ticks
        if self.png_out.is_none() {
            framerate::pace();
        }
        if self.app.simfoto.is_some_and(|f| !f.saved && f.frames >= 2) {
            self.save_simfoto();
        }
        if !self.app.snapshot_due.is_empty() {
            while !self.app.snapshot_due.is_empty()
                && self.app.ui_ok
                && !matches!(
                    self.app.screen,
                    Screen::Menu
                        | Screen::Scenarios
                        | Screen::Difficulty
                        | Screen::Property
                        | Screen::Files
                        | Screen::Themes
                        | Screen::Credits
                )
            {
                self.take_snapshot();
            }
            if self.png_out.is_some() && self.app.snapshot_due.is_empty() && self.app.screen == Screen::Board {
                self.frames = 1; // a scripted still of the board waits one more frame for its photo
            }
        }
        self.frames += 1;
        if self.frames == 1 {
            self.pointer_test_hook();
        }
        // a photo asked for in photo mode is drawn offscreen once the frame is shown (photo.rs; after the scripted
        // pointer, which may set the focus)
        if self.app.photo.as_ref().is_some_and(|p| p.pending_shot.is_some()) {
            self.photo_take();
        }
        if std::env::var_os("SG_PICKTEST").is_some() && self.frames == 2 {
            self.pick_test();
        }
        if let Some(out) = self.png_out.clone() {
            if self.frames >= 2 {
                // SG_FX_BENCH=N: the frame time over N frames drawn offscreen (Classic or with the effects), for measuring
                if let Some(n) = std::env::var("SG_FX_BENCH").ok().and_then(|v| v.parse::<u32>().ok()) {
                    self.bench(n.max(1));
                }
                app::tick_bench_report(&self.app);
                // let the first frame settle
                if !self.screenshot(&out) {
                    eprintln!("could not write {}", out.display());
                    std::process::exit(1);
                }
                println!("saved {} ({}x{})", out.display(), self.app.draw_w, self.app.draw_h);
                if let Some(f) = &self.save_game_out {
                    match self.app.save_game(f) {
                        Ok(()) => println!("saved game {} at tick {}, cash {:.0}", f.display(), self.app.game_tick, self.app.econ.cash),
                        Err(e) => eprintln!("error: {e}"),
                    }
                }
                std::process::exit(0);
            }
        }
    }

    fn mouse_motion_event(&mut self, x: f32, y: f32) {
        sysdiag::event(sysdiag::Ev::Mouse);
        self.gamepad_mouse_moved(x, y);
        let (dx, dy) = (x - self.mouse.0, y - self.mouse.1);
        self.mouse = (x, y);
        if self.app.photo.is_some() {
            self.photo_motion(x, y, dx, dy);
            return;
        }
        self.app.info.pointer = self.app.view.to_virtual(x, y);
        let (vx, vy) = self.app.view.to_virtual(x, y);
        if self.app.notice.is_some() {
            self.app.notice_pointer(vx, vy, false);
            return;
        }
        self.app.card_pointer(vx, vy);
        if self.app.screen != Screen::Play {
            // the pointer has left the dock and the panel for another screen (the Information menu, a report, a question):
            // the button it was on is no longer lit and its tooltip not shown, which otherwise stayed at the old pointer
            // over the report or under the menu
            self.app.dock_hover = -1;
            self.app.pstate.mouse = (-1000.0, -1000.0);
            self.app.panel_hover = -1;
        }
        if self.app.screen == Screen::Land {
            let (vx, vy) = self.app.view.to_virtual(x, y);
            self.app.land_pointer(vx, vy);
            return;
        }
        if self.app.screen == Screen::Pair {
            let (vx, vy) = self.app.view.to_virtual(x, y);
            self.app.pair_pointer(vx, vy);
            return;
        }
        if self.app.screen == Screen::Popup {
            let (vx, vy) = self.app.view.to_virtual(x, y);
            self.app.popup_pointer(vx, vy, false);
            return;
        }
        if self.app.screen != Screen::Play && self.app.ui_ok {
            if matches!(self.app.screen, Screen::Menu | Screen::Difficulty | Screen::Property) {
                self.menu_pointer(x, y, false);
            } else if matches!(self.app.screen, Screen::Files | Screen::Themes | Screen::Credits) {
                let (vx, vy) = self.app.view.to_virtual(x, y);
                self.app.title_pointer(vx, vy, false);
            }
            return;
        }
        if self.app.ui_ok {
            let (vx, vy) = self.app.view.to_virtual(x, y);
            self.dock_hover_update(vx, vy);
        }
        if self.editing {
            let hit = self.app.pick_ground(x, y);
            self.app.has_hit = hit.is_some();
            if let Some((hx, hz)) = hit {
                self.app.hit_x = hx;
                self.app.hit_z = hz;
            }
            self.app.apply_tool(self.shift, true);
        } else if self.dragging {
            let k = self.app.upp; // drawable pixels -> world units
            let sin_p = (pitch_for(self.app.draw_w, self.app.draw_h).to_radians()).sin() as f32;
            self.app.drag_pan(-dx * k, dy * k / sin_p, now());
        }
    }

    fn mouse_wheel_event(&mut self, _x: f32, y: f32) {
        if self.app.photo.is_some() {
            self.photo_wheel(y);
            return;
        }
        if y != 0.0 {
            match self.app.screen {
                Screen::People => return self.app.people_wheel(y),
                Screen::History => return self.app.history_wheel(y),
                Screen::HoleComments => return self.app.hole_comments_wheel(y),
                Screen::Profile => return self.app.profile_wheel(y),
                _ => {}
            }
        }
        if (self.app.screen == Screen::Play || !self.app.ui_ok) && y != 0.0 && self.app.great_shot.is_none() {
            // y is in notches (the browser's pixels are turned into notches by web/opensgolf.js); a trackpad sends many
            // small turns
            if enhanced::smooth_camera() {
                // Enhanced: a notch is a quarter of a doubling, between the original's closest and farthest levels
                self.app.zoom_by((y * 0.25).clamp(-0.5, 0.5));
            } else {
                // a step to the next of the original's three levels for the first turn of a gesture (a pause of a quarter
                // second or a change of direction starts one), and another for each further notch's worth: a mouse
                // notch is a step as before, and a trackpad's stream of small turns no longer runs through every level
                let t = self.app.clock;
                let (acc, last) = self.wheel;
                if t - last > 0.25 || (acc != 0.0 && (acc > 0.0) != (y > 0.0)) {
                    self.app.zoom_step(y > 0.0);
                    self.wheel = (0.0, t);
                } else {
                    let acc = acc + y;
                    if acc.abs() >= 1.0 {
                        self.app.zoom_step(acc > 0.0);
                        self.wheel = (0.0, t);
                    } else {
                        self.wheel = (acc, t);
                    }
                }
            }
        }
    }

    fn mouse_button_down_event(&mut self, button: MouseButton, x: f32, y: f32) {
        // Ctrl+click is a right-click, as everywhere on a Mac (a one-button mouse, a trackpad without secondary click);
        // the original gives Ctrl no meaning with a click
        self.ctrl_click = button == MouseButton::Left && self.ctrl;
        let button = if self.ctrl_click { MouseButton::Right } else { button };
        sysdiag::event(sysdiag::Ev::Click);
        self.mouse = (x, y);
        // a notice (gamefiles.rs) takes the click
        if self.app.notice.is_some() {
            let (vx, vy) = self.app.view.to_virtual(x, y);
            self.app.notice_pointer(vx, vy, button == MouseButton::Left);
            return;
        }
        if self.app.photo.is_some() {
            self.photo_button(true, button, x, y);
            return;
        }
        if self.app.great_shot_dismiss() || self.app.simfoto_dismiss() {
            return;
        }
        if self.app.analysis_input() {
            return;
        }
        self.app.info.pointer = self.app.view.to_virtual(x, y);
        if button == MouseButton::Left {
            let (vx, vy) = self.app.view.to_virtual(x, y);
            if self.app.save_dialog_click(vx, vy) {
                return;
            }
        }
        if self.app.screen == Screen::Land {
            let (vx, vy) = self.app.view.to_virtual(x, y);
            self.app.land_pointer(vx, vy);
            self.app.land_click(button == MouseButton::Right);
            return;
        }
        if self.app.screen == Screen::Popup {
            let (vx, vy) = self.app.view.to_virtual(x, y);
            if let Some(r) = self.app.popup_pointer(vx, vy, true) {
                self.popup_done(r);
            }
            return;
        }
        if self.app.screen != Screen::Play && self.app.ui_ok && button == MouseButton::Left {
            // the OpenSGolf extras' report tabs (report_tabs.rs)
            let (vx, vy) = self.app.view.to_virtual(x, y);
            if self.app.report_tabs_click(vx, vy) {
                return;
            }
        }
        if self.app.screen != Screen::Play && self.app.ui_ok {
            if matches!(self.app.screen, Screen::People | Screen::History | Screen::Profile) {
                let (vx, vy) = self.app.view.to_virtual(x, y);
                let right = button == MouseButton::Right;
                if self.app.rename.is_some() {
                    // the rename prompt over the screen takes nothing from the pointer
                } else if self.app.screen == Screen::People {
                    self.app.people_click(vx, vy, right);
                } else if self.app.screen == Screen::History {
                    self.app.history_click(vx, vy, right);
                } else {
                    self.app.profile_click(vx, vy, right);
                }
            } else if self.app.screen == Screen::HoleComments {
                let (vx, vy) = self.app.view.to_virtual(x, y);
                self.app.hole_comments_click(vx, vy, button == MouseButton::Right);
            } else if self.app.screen == Screen::Report {
                let (vx, vy) = self.app.view.to_virtual(x, y);
                self.app.report_click(vx, vy);
            } else if self.app.screen == Screen::HoleStats {
                // the Set par extra's buttons (extras_ui.rs), else the original's close on any click
                let (vx, vy) = self.app.view.to_virtual(x, y);
                if !(button == MouseButton::Left && self.app.hole_stats_click(vx, vy)) {
                    self.app.screen = Screen::Report;
                }
            } else if matches!(self.app.screen, Screen::Files | Screen::Themes | Screen::Credits) {
                let (vx, vy) = self.app.view.to_virtual(x, y);
                if button == MouseButton::Left {
                    self.app.title_pointer(vx, vy, true);
                }
            } else if self.app.screen == Screen::Pair {
                let (vx, vy) = self.app.view.to_virtual(x, y);
                self.app.pair_click(vx, vy);
            } else if self.app.screen == Screen::Routing {
                let (vx, vy) = self.app.view.to_virtual(x, y);
                self.app.routing_click(vx, vy, button == MouseButton::Right);
            } else if self.app.screen == Screen::Content {
                let (vx, vy) = self.app.view.to_virtual(x, y);
                if button == MouseButton::Left {
                    self.app.content_click(vx, vy);
                }
            } else if self.app.screen == Screen::Keys {
                let (vx, vy) = self.app.view.to_virtual(x, y);
                self.app.keys_click(vx, vy, button == MouseButton::Right);
            } else if self.app.screen == Screen::Scenarios {
                let (vx, vy) = self.app.view.to_virtual(x, y);
                if button == MouseButton::Left {
                    self.app.scenarios_click(&mut self.g, vx, vy);
                }
            } else if self.app.screen == Screen::Roster {
                let (vx, vy) = self.app.view.to_virtual(x, y);
                self.app.roster_click(vx, vy, button == MouseButton::Right);
            } else if matches!(
                self.app.screen,
                Screen::Board
                    | Screen::YearEnd
                    | Screen::Comments
                    | Screen::Histograph
                    | Screen::Finance
                    | Screen::Shortcuts
                    | Screen::BestScores
                    | Screen::Top10
                    | Screen::Goals
            ) {
                self.app.close_info();
            } else if self.app.screen == Screen::Skills {
                let (vx, vy) = self.app.view.to_virtual(x, y);
                self.app.skills_click(vx, vy);
            } else if self.app.screen == Screen::Customise {
                let (vx, vy) = self.app.view.to_virtual(x, y);
                self.app.cust_click(vx, vy, button == MouseButton::Right);
            } else if matches!(self.app.screen, Screen::Sga | Screen::Prep | Screen::Results) {
                let (vx, vy) = self.app.view.to_virtual(x, y);
                self.app.tourney_click(vx, vy);
            } else if button == MouseButton::Left {
                self.menu_pointer(x, y, true);
            }
            return;
        }
        // the middle button drags the course along whatever tool is held (the port's own)
        if button == MouseButton::Middle {
            self.dragging = true;
            self.app.drag_state(true, now());
            return;
        }
        if button != MouseButton::Left && button != MouseButton::Right {
            return;
        }
        let (vx, vy) = self.app.view.to_virtual(x, y);
        // the interface hidden (u): a click goes to the course
        let ui_ok = self.app.ui_ok && !self.app.hud_hidden();
        // a right click lets go of a golfer held by the card's Move/Eject (the main loop's right click clears 0x4c2e10)
        if button == MouseButton::Right && self.app.card_ui.held.take().is_some() {
            return;
        }
        if ui_ok && button == MouseButton::Left && self.app.photo_button_hit(vx, vy) {
            self.photo_enter();
            return;
        }
        // OpenSGolf extras: the ticker's Show button and the message history (extras.rs)
        if ui_ok && button == MouseButton::Left && self.app.ticker_click(vx, vy) {
            return;
        }
        if ui_ok && button == MouseButton::Left && self.app.card_click(vx, vy) {
            return;
        }
        if ui_ok && self.dock_click(vx, vy, button == MouseButton::Right) {
            return;
        }
        if ui_ok && button == MouseButton::Left && self.app.strip_click(vx, vy) {
            return;
        }
        // the measuring tape takes a click on the course; a right click ends a rectangle under way (build_tools.rs)
        if self.app.screen == Screen::Play && self.app.tape_click(button == MouseButton::Left) {
            return;
        }
        if button == MouseButton::Right && self.app.bt.rect_from.take().is_some() {
            return;
        }
        if button == MouseButton::Left && self.app.club.pro_aiming().is_some() && !self.app.edit {
            if let Some((hx, hz)) = self.app.pick_ground(x, y) {
                self.app.aim_pointer(hx, hz);
            }
            self.app.aim_click();
            return;
        }
        // the port's own undo: the game as it is before a click on the course (undo.rs)
        if button == MouseButton::Left && self.app.screen == Screen::Play && self.app.edit {
            self.app.undo_press();
        }
        if self.app.edit && button == MouseButton::Left {
            self.editing = true;
            self.app.last_cell = -1;
            let hit = self.app.pick_ground(x, y);
            self.app.has_hit = hit.is_some();
            if let Some((hx, hz)) = hit {
                self.app.hit_x = hx;
                self.app.hit_z = hz;
            }
            // the terrain brush's rectangle (Shift) and fill (Alt), the port's own (build_tools.rs)
            match self.app.brush_area_press(self.shift, self.alt) {
                build_tools::AreaPress::None => self.app.apply_tool(self.shift, false),
                build_tools::AreaPress::Rect => {}
                build_tools::AreaPress::Filled => self.editing = false,
            }
        } else {
            self.dragging = true;
            // a drag of the view ends People > Follow
            self.app.xs.follow = None;
            self.app.drag_state(true, now());
            if button == MouseButton::Left {
                if let Some((hx, hz)) = self.app.pick_ground(x, y) {
                    self.app.course_click(hx, hz);
                }
            }
        }
    }

    fn mouse_button_up_event(&mut self, button: MouseButton, x: f32, y: f32) {
        let button = if button == MouseButton::Left && std::mem::take(&mut self.ctrl_click) { MouseButton::Right } else { button };
        if self.app.photo.is_some() {
            self.photo_button(false, button, x, y);
            return;
        }
        // a rectangle under way ends with the left button only; it is painted before the undo step is kept
        if button != MouseButton::Left && self.app.bt.rect_from.is_some() {
            return;
        }
        if button == MouseButton::Left {
            self.app.brush_area_release();
        }
        if self.dragging {
            self.app.drag_state(false, now());
        }
        self.dragging = false;
        self.editing = false;
        self.app.undo_release();
    }

    fn key_down_event(&mut self, k: KeyCode, mods: KeyMods, _repeat: bool) {
        // Android's back button (miniquad hands it over as Back): Esc, which closes a screen, goes back to the menu and
        // on the menu quits, as the back button does in Android apps
        let k = if k == KeyCode::Back { KeyCode::Escape } else { k };
        // the port's own: F11 or Alt+Enter switches between a window and full screen on the desktop (the browser has its
        // own F11; a phone is always full screen)
        #[cfg(not(any(target_arch = "wasm32", target_os = "android")))]
        if k == KeyCode::F11 || (mods.alt && matches!(k, KeyCode::Enter | KeyCode::KpEnter)) {
            self.fullscreen = !self.fullscreen;
            miniquad::window::set_fullscreen(self.fullscreen);
            return;
        }
        // the port's own Course sharing: Ctrl+V (Cmd+V on a Mac) pastes a course code (share.rs)
        if k == KeyCode::V && (mods.ctrl || mods.logo) && extras::sharing() {
            self.app.paste_course();
            return;
        }
        // the port's own: Ctrl+Z (Cmd+Z on a Mac) undoes the last click that changed the course (undo.rs)
        if k == KeyCode::Z && (mods.ctrl || mods.logo) && self.app.screen == Screen::Play && !self.editing {
            if self.app.undo_step(&mut self.g) {
                self.app.ui_sound(0x18);
            }
            return;
        }
        // the port's own: Alt (Option on a Mac) or Ctrl with 1 to 9 and 0 stand for F1 to F10, for keyboards whose function
        // keys need Fn (Macs, laptops) or act on the browser
        let k = if mods.alt || mods.ctrl { fkey_of_digit(k).unwrap_or(k) } else { k };
        self.shift = mods.shift || k == KeyCode::LeftShift || k == KeyCode::RightShift;
        self.app.shift_held = self.shift;
        self.ctrl = mods.ctrl || k == KeyCode::LeftControl || k == KeyCode::RightControl;
        self.alt = mods.alt || k == KeyCode::LeftAlt || k == KeyCode::RightAlt;
        self.app.bt.alt = self.alt;
        let typing = self.app.typing();
        self.key_down(k);
        self.swallow_char = !typing && self.app.typing();
    }

    fn files_dropped_event(&mut self) {
        // the port's own Course sharing: shared courses, saved games and course codes dropped on the window (share.rs)
        if !extras::sharing() {
            return;
        }
        for i in 0..miniquad::window::dropped_file_count() {
            let path = miniquad::window::dropped_file_path(i);
            let name = path.as_ref().and_then(|p| p.file_name()).map(|n| n.to_string_lossy().to_string()).unwrap_or_default();
            let bytes = miniquad::window::dropped_file_bytes(i).or_else(|| path.and_then(sg_core::fsutil::read_file));
            if let Some(b) = bytes {
                self.app.import_dropped(&name, &b);
            }
        }
    }

    fn char_event(&mut self, c: char, _mods: KeyMods, _repeat: bool) {
        if std::mem::take(&mut self.swallow_char) {
            return;
        }
        self.app.rename_char(c);
        self.app.search_char(c);
        self.app.save_char(c);
        self.app.profile_char(c);
        self.app.cust_char(c);
    }

    fn touch_event(&mut self, phase: TouchPhase, id: u64, x: f32, y: f32) {
        self.touch(phase, id, x, y);
    }

    fn key_up_event(&mut self, k: KeyCode, mods: KeyMods) {
        self.shift = mods.shift && k != KeyCode::LeftShift && k != KeyCode::RightShift;
        self.app.shift_held = self.shift;
        self.ctrl = mods.ctrl && k != KeyCode::LeftControl && k != KeyCode::RightControl;
        self.alt = mods.alt && k != KeyCode::LeftAlt && k != KeyCode::RightAlt;
        self.app.bt.alt = self.alt;
        // the measuring tape's key let go (build_tools.rs)
        if self.app.screen == Screen::Play && controls::settings().binding(controls::action("tape")).is_some_and(|b| b.key == k) {
            self.app.tape_key_up();
        }
    }

    /// The window's close button: the game never keeps the window open, so the program ends here, before the window
    /// system tears down the OpenGL context (Mesa's D3D12 driver, the base of Windows' OpenGL compatibility pack, ended
    /// the program with an error code in that teardown on the ARM64 test machine).
    fn quit_requested_event(&mut self) {
        sysdiag::note("the window's close was asked for: exiting");
        #[cfg(not(any(target_arch = "wasm32", target_os = "android")))]
        std::process::exit(0);
    }

    /// The window went to the background (focus out on X11, Windows and macOS, the page hidden or blurred in the browser,
    /// the app paused on Android): the Pause in the background extra pauses the game (comfort.rs).
    fn window_minimized_event(&mut self) {
        self.app.window_hidden();
    }

    /// The window is back: the pause the extra made ends.
    fn window_restored_event(&mut self) {
        self.app.window_shown();
    }
}

/// The main loop has ended (the window closed, or the game quit): the program ends here at once. Dropping the game would
/// stop the sound output first, and a sound driver's thread that does not stop must not keep the program (and its music)
/// running without a window.
#[cfg(not(any(target_arch = "wasm32", target_os = "android")))]
impl Drop for Stage {
    fn drop(&mut self) {
        sysdiag::note("main loop ended: exiting");
        std::process::exit(0);
    }
}

impl Stage {
    /// The ground point under the pointer (or where SG_CURSOR_TILE holds it), None off the course.
    fn pointer_ground(&self) -> Option<(f32, f32)> {
        pin_world(&self.app).or_else(|| self.app.pick_ground(self.mouse.0, self.mouse.1))
    }

    /// A key pressed (key_down_event after the modifiers and the stand-in keys).
    fn key_down(&mut self, k: KeyCode) {
        // a notice (gamefiles.rs) takes the keys until it is closed
        if self.app.notice.is_some() {
            self.app.notice_key(k);
            return;
        }
        if self.app.photo.is_some() {
            self.photo_key(k);
            return;
        }
        if self.app.great_shot.is_some() {
            // the photo's wait (0x45c0c0) ends on any key; F10 first saves it
            if k == KeyCode::F10 {
                self.save_great_shot();
            }
            self.app.great_shot_dismiss();
            return;
        }
        if self.app.simfoto_dismiss() || self.app.rename_key(k) || self.app.save_key(k) || self.app.analysis_input() {
            return;
        }
        if self.app.screen == Screen::Popup {
            if let Some(r) = self.app.popup_key(k) {
                self.popup_done(r);
            }
            return;
        }
        if self.app.screen == Screen::Scenarios && self.app.ui_ok {
            self.app.scenarios_key(&mut self.g, k);
            return;
        }
        if self.app.screen != Screen::Play && self.app.ui_ok {
            let shift = self.shift;
            let app = &mut self.app;
            // the OpenSGolf extras' report tabs: Tab, Shift+Tab and the other reports' keys (report_tabs.rs)
            if app.report_tabs_key(k, shift) {
                return;
            }
            if app.screen == Screen::People {
                app.people_key(k);
            } else if app.screen == Screen::History {
                app.history_key(k);
            } else if app.screen == Screen::Profile {
                app.profile_key(k);
            } else if app.screen == Screen::HoleComments {
                app.hole_comments_key(k);
            } else if app.screen == Screen::Report {
                if k == KeyCode::Escape || k == KeyCode::F1 {
                    app.screen = Screen::Play;
                    app.hover = -1;
                }
            } else if app.screen == Screen::HoleStats {
                // the Set par extra's minus and plus (extras_ui.rs), else the original's close on any key
                if !app.hole_stats_key(k) {
                    app.screen = Screen::Report;
                }
            } else if matches!(app.screen, Screen::Files | Screen::Themes | Screen::Credits) {
                app.title_key(k);
            } else if app.screen == Screen::Pair {
                app.close_pair_screen();
            } else if matches!(
                app.screen,
                Screen::Board
                    | Screen::YearEnd
                    | Screen::Roster
                    | Screen::Comments
                    | Screen::Histograph
                    | Screen::Finance
                    | Screen::Routing
                    | Screen::Shortcuts
                    | Screen::BestScores
                    | Screen::Top10
                    | Screen::Goals
            ) {
                app.close_info();
            } else if app.screen == Screen::Content {
                app.content_key(k);
            } else if app.screen == Screen::Keys {
                app.keys_key(k);
            } else if app.screen == Screen::Skills {
                if k == KeyCode::Enter || k == KeyCode::Escape {
                    app.skills_key(k == KeyCode::Enter);
                }
            } else if app.screen == Screen::Customise {
                app.cust_key(k);
            } else if matches!(app.screen, Screen::Sga | Screen::Prep | Screen::Results) {
                app.tourney_key(k);
            } else if k == KeyCode::Escape {
                if app.screen == Screen::Menu {
                    quit_game();
                } else if app.screen == Screen::Land || (app.screen == Screen::Property && app.world_move) {
                    // TRACTS FOR SALE and the world map opened from a game (F6) go back to the course, as their cancel
                    // buttons do; Esc used to drop the game for the title menu
                    app.world_move = false;
                    app.screen = Screen::Play;
                } else {
                    app.screen = Screen::Menu;
                }
                app.hover = -1;
            }
            return;
        }
        self.play_key(k);
    }
}

/// Leaves the game (the title menu's exit button, Esc there). A browser tab cannot close itself: the page goes back to its
/// start page (web/opensgolf.js sg_quit) instead of stopping on the last frame drawn.
fn quit_game() {
    #[cfg(target_arch = "wasm32")]
    {
        #[link(wasm_import_module = "env")]
        extern "C" {
            fn sg_quit();
        }
        // SAFETY: a plain call into the page's script, no arguments
        unsafe { sg_quit() };
    }
    #[cfg(not(target_arch = "wasm32"))]
    window::order_quit();
}

/// The function key a digit stands for with Alt or Ctrl held: 1 to 9 for F1 to F9, 0 for F10.
fn fkey_of_digit(k: KeyCode) -> Option<KeyCode> {
    use KeyCode::*;
    Some(match k {
        Key1 => F1,
        Key2 => F2,
        Key3 => F3,
        Key4 => F4,
        Key5 => F5,
        Key6 => F6,
        Key7 => F7,
        Key8 => F8,
        Key9 => F9,
        Key0 => F10,
        _ => return None,
    })
}

/// The window's icon (Windows' title bar and taskbar, macOS's Dock): the app icon, drawn by the free set's painter.
fn window_icon() -> conf::Icon {
    let px = |n: usize| sg_core::freeset::app_icon(n).px;
    let mut icon = conf::Icon { small: [0; 16 * 16 * 4], medium: [0; 32 * 32 * 4], big: [0; 64 * 64 * 4] };
    icon.small.copy_from_slice(&px(16));
    icon.medium.copy_from_slice(&px(32));
    icon.big.copy_from_slice(&px(64));
    icon
}

/// Windows: a windowed program has no console. Started with options (a test, a script) from a console, it writes its
/// messages to that console; output already sent to a file or a pipe is left alone.
#[cfg(windows)]
fn attach_console() {
    #[link(name = "kernel32")]
    extern "system" {
        fn AttachConsole(process: u32) -> i32;
        fn GetStdHandle(which: u32) -> isize;
    }
    const ATTACH_PARENT_PROCESS: u32 = u32::MAX;
    const STD_OUTPUT_HANDLE: u32 = -11i32 as u32;
    if std::env::args_os().len() > 1 {
        // SAFETY: plain Win32 calls without pointers
        unsafe {
            if GetStdHandle(STD_OUTPUT_HANDLE) == 0 {
                AttachConsole(ATTACH_PARENT_PROCESS);
            }
        }
    }
}

fn main() {
    #[cfg(windows)]
    attach_console();
    sysdiag::install_panic_hook();
    sysdiag::init();
    #[cfg(target_os = "android")]
    android::init();
    let mut o = parse_args();
    // the text's language first: the window's title is already text
    match &o.lang_file {
        Some(f) => _ = language::apply_file(f),
        None => _ = language::apply(&o.lang.clone().unwrap_or_else(language::saved)),
    }
    let steam_ui = ["SteamDeck", "SteamGamepadUI"].iter().any(|k| std::env::var(k).is_ok_and(|v| !v.is_empty() && v != "0"));
    o.fullscreen |= steam_ui && cfg!(target_os = "linux");
    let conf = conf::Conf {
        window_title: tr("STR_WINDOW_TITLE").to_string(),
        window_width: o.win_w,
        window_height: o.win_h,
        high_dpi: true,
        window_resizable: true,
        // a phone or tablet: the whole screen (the manifest keeps it landscape, Cargo.toml); on the desktop with
        // --fullscreen, and in Steam's Game Mode (a Steam Deck, or Big Picture), which sets SteamDeck or SteamGamepadUI
        fullscreen: cfg!(target_os = "android") || o.fullscreen,
        // 32-bit mesh indices need WebGL 2 in a browser.
        platform: conf::Platform {
            webgl_version: conf::WebGLVersion::WebGL2,
            // GNOME matches the window to the AppImage's opensgolf.desktop by this
            linux_wm_class: "opensgolf",
            // v-sync (Enhanced graphics > Display; framerate.rs): on unless the player turned it off
            swap_interval: Some(framerate::swap_interval()),
            ..Default::default()
        },
        icon: Some(window_icon()),
        ..Default::default()
    };
    miniquad::start(conf, move || Box::new(Stage::new(o)));
    // the loop has ended (a window system whose loop returns without dropping the game): end, sound and all
    #[cfg(not(any(target_arch = "wasm32", target_os = "android")))]
    std::process::exit(0);
}
