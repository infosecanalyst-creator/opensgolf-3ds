//! simgolf: the SimGolf native port. Reads the data of your own copy of the game at run time.
//!
//!   simgolf --game "<dir>/Program_Files_(ENGLISH)" [--theme parkland|links|desert|tropical] [--seed N] [--size WxH] [--zoom Z]
//!           [--rot DEG] [--center TX,TY] [--time SECONDS] [--follow] [--png out.png] [--golfer NAME] [--sandbox] [--cash N]
//!           [--screen menu|property|play|report] [--course FILE] [--save FILE] [--edit SPEC] [--panel N] [--mute] [--sound-log]
//!
//! Keys: arrows/WASD pan, Q/E rotate, +/- or mouse wheel zoom, 1-4 theme, R new demo course, P toggle scenery, F follow the golfer,
//! F6 screenshot, F1-F10 reports, M music, N mute, H open the new hole, Shift+H advisor, Tab edit mode, Esc menu. Left-drag pans.
// UI drawing takes source and destination rectangles as plain numbers; index loops mirror the original's tables.
#![allow(clippy::too_many_arguments, clippy::needless_range_loop)]

mod app;
mod audio;
mod champ_ui;
mod cursor_ui;
mod cust_ui;
mod files_ui;
mod gfx;
mod holemarks;
mod hud_ui;
mod info_ui;
mod message_ui;
mod panels_ui;
mod player_panel;
mod popup_ui;
mod pro_ui;
mod render;
mod reports_ui;
mod screens_ui;
mod thoughts_ui;
mod touch;
mod tourney_ui;
mod ui;
mod wild_ui;
mod world_ui;

use app::*;
use gfx::Gfx;
use miniquad::*;
use render::*;
use sg_core::formats::{atoi, parse_pro_golfers};
use sg_core::mixer::Mixer;
use sg_core::properties::PROPERTIES;
use sg_core::terrain::{Terrain, TT_FAIRWAY, TT_PUTTING_GREEN, TT_ROUGH, TT_SAND, TT_TEE, TT_WATER_SHALLOW};
use std::path::PathBuf;
use std::sync::Arc;

struct Options {
    game_dir: PathBuf,
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
    screen: Option<String>,
    cash: Option<f64>,
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
}

const USAGE: &str = "usage: simgolf --game DIR [--theme T] [--seed N] [--size WxH] [--zoom Z] [--rot DEG] [--center TX,TY] [--png FILE] \
[--time S] [--follow] [--golfer NAME] [--sandbox] [--cash N] [--difficulty 0-3] [--screen menu|property|play|report] [--course FILE] [--save FILE] \
[--edit SPEC] [--panel N] [--mute] [--sound-log] [--clock MS] [--property N|NAME]";

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
    let mut o = Options {
        // In a browser the page puts the player's game folder at "game" (web/simgolf.js).
        game_dir: PathBuf::from(if cfg!(target_arch = "wasm32") { "game" } else { "game/Program_Files_(ENGLISH)" }),
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
        screen: None,
        cash: None,
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
            "--game" => o.game_dir = PathBuf::from(next()),
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
            "--sound-log" => o.sound_log = true,
            "--sandbox" => o.sandbox = true,
            "--screen" => o.screen = Some(next()),
            "--cash" => o.cash = next().parse().ok(), // test hook: starting cash
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
            _ => {
                eprintln!("{USAGE}");
                std::process::exit(2);
            }
        }
        i += 1;
    }
    o
}

struct Stage {
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
    pause_toggle: bool,
    shown_version: u32,
    touch: touch::Touch,
}

fn load_ui(app: &mut App, g: &mut Gfx) -> bool {
    let font = ui::Font::load(g, |f| app.game_path(f));
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
    let ok = font && base.is_some() && un.is_some() && mo.is_some() && world.is_some();
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
    app.cliffs = ui::load_pcx(g, &app.game_path("cliffs01.pcx"), true, None).unwrap_or_default();
    app.ui_ok = ok;
    app.art = crate::screens_ui::Art::load(g, app);
    app.reports = crate::reports_ui::ReportArt::load(g, app);
    app.panel_art = crate::panels_ui::PanelArt::load(g, app);
    app.title.art = crate::files_ui::TitleArt::load(g, app);
    app.hud = crate::hud_ui::HudArt::load(g, app);
    app.info.art = crate::info_ui::InfoArt::load(g, app);
    ok
}

/// Scripted edits for tests: "p:x,y,type[,vbyte,radius];w:x,y,kind[,radius];b:x,y,building;r:cx,cy,delta[,radius];
/// h:kind[,x,y];t:x,y,type;o" (t paints one tile through the hole tool, o opens the hole being built; h hires an employee: 0 Club Pro, 1 Ranger, 2 Groundskeeper, 3 Soda Vendor; x,y is the post tile).
/// The --screen test hook: opens a screen of a game in progress.
fn open_screen(app: &mut App, screen: Option<&str>) {
    match screen {
        Some("land") => app.open_land_screen(),
        Some("report") if app.ui_ok => app.screen = Screen::Report,
        Some("pair") if app.ui_ok => app.screen = Screen::Pair,
        Some("board") if app.ui_ok => {
            // SG_BOARD_AWARDS="id,id,..": accomplishments marked earned (a month apart, no snapshot) for a still
            if let Ok(ids) = std::env::var("SG_BOARD_AWARDS") {
                for (k, id) in ints(&ids).into_iter().enumerate() {
                    if let Some(e) = app.club.earned.get_mut(id.max(0) as usize) {
                        let tick = app.club.tick + 1024 * k as u32;
                        *e = Some(sg_core::records::Earned { tick, course: app.club.course_name.clone() });
                    }
                }
            }
            app.screen = Screen::Board;
        }
        Some("yearend") if app.ui_ok => app.screen = Screen::YearEnd,
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
                app.open_customise(app.club.gary.max(0) as usize);
                if let Some(c) = app.cust.as_mut().filter(|_| s == "customisefaces") {
                    c.picker = Some(0);
                }
            } else if let Some(&g) = list.get(n.min(list.len().saturating_sub(1))) {
                app.card = Some(g);
                app.card_ui.story = s.starts_with("cardstory");
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
            b'b' if item.len() > 2 && v.len() == 3 => {
                app.build_idx = at(2).clamp(0, 19) as usize;
                app.edit_building(at(0), at(1), false);
                println!(
                    "building {} at {},{}: {} objects, toast '{}', cash {}",
                    sg_core::land::BUILDINGS[app.build_idx].0,
                    at(0),
                    at(1),
                    app.land.as_ref().map(|l| l.objects.iter().filter(|o| o.kind >= 0).count()).unwrap_or(0),
                    app.toast,
                    app.econ.cash
                );
            }
            b'h' if item.len() > 2 && !v.is_empty() => {
                let ok = app.hire_staff(at(0).clamp(0, 3) as usize);
                if ok && v.len() >= 3 {
                    if let Some(e) = app.employees.iter_mut().rev().find(|e| e.active) {
                        e.post = Some((at(1), at(2)));
                    }
                }
                println!("hired staff kind {}: {ok}", at(0));
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
            b'l' if item.len() > 2 && !v.is_empty() => {
                // buy tract n (1..9) through the land screen
                app.open_land_screen();
                let (col, row) = ((at(0) - 1) / 3, (at(0) - 1) % 3);
                app.land_pointer(20.0 + 260.0 * col as f32, 60.0 + 68.0 * row as f32);
                app.land_click(false);
            }
            b'j' => {
                // the pro's round: j:0 practice, j:1 the match; scripted runs aim every full shot at the pin
                let ui = std::mem::replace(&mut app.ui_ok, false);
                app.auto_aim = if v.len() > 1 && at(1) == 1 { 2 } else { 1 };
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
            b'x' => app.save_championship_course(),
            b'q' => app.save_championship_pro(),
            b'A' => app.auto_aim = 1, // scripted runs: the pro aims every full shot at the pin
            b'M' => app.open_popup(match at(0) {
                0 => popup_ui::PopupKind::Info,
                1 => popup_ui::PopupKind::System,
                2 => popup_ui::PopupKind::Prefs,
                _ => popup_ui::PopupKind::LandOffer,
            }),
            b'm' if v.len() >= 2 => app.move_hole(at(0) as usize, at(1) as usize),
            b'a' if v.len() >= 3 => {
                // test hook: an animal of kind v[2] and a follower at x,y
                app.club.wildlife.spawn(&mut app.exe_rng, at(0), at(1), at(2));
            }
            b'g' => {
                // a tournament: g:0 as the SGA offers it (the evaluation must pass), g:1 straight away with the default purse
                app.club.game |= sg_core::tournament::OFFERED;
                app.auto_aim = 1;
                if at(0) == 1 {
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
        let mut app = match o.clock {
            Some(c) => App::with_clock(o.game_dir.clone(), c),
            None => App::new(o.game_dir.clone()),
        };
        app.theme = o.theme;
        app.seed = o.seed;
        app.zoom = o.zoom;
        app.rot = o.rot;
        app.time = o.time;
        app.follow = o.follow;
        app.mute = o.mute;
        app.sound_log = o.sound_log;
        app.econ.sandbox = o.sandbox;
        if let Some(d) = o.difficulty {
            app.difficulty = d;
        }
        if let Some(c) = o.cash {
            app.econ.start_cash = c;
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
            app.skills.v = hit.skill;
            println!("golfer: {}, skills {}", hit.name, hit.skill.iter().map(|s| format!("{s:X}")).collect::<Vec<_>>().join(" "));
        }
        let info = g.ctx.info();
        println!("GL: {:?}", info.backend);

        let mut mixer = Mixer::new(&app.game_path("Sounds"));
        mixer.add_dir("simsfx", &app.game_path("SimsFX"));
        if mixer.is_empty() {
            eprintln!("sound: {}", mixer.last_error());
        } else {
            let mixer = Arc::new(mixer);
            app.mixer = Some(mixer.clone());
            if !app.mute && o.png_out.is_none() {
                app.audio = audio::open(mixer);
                if app.audio.is_none() {
                    eprintln!("sound: no audio device, running silent");
                }
            }
        }
        app.terrain = Terrain::demo_course(40, 40, app.seed);
        app.land = None;
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
                || o.follow;
            let s = o.screen.clone().unwrap_or_else(|| if scripted { "play".into() } else { "menu".into() });
            if app.ui_ok {
                match s.as_str() {
                    "report" => app.screen = Screen::Report,
                    "menu" => app.screen = Screen::Menu,
                    "property" => app.screen = Screen::Property,
                    "difficulty" => app.screen = Screen::Difficulty,
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
                PROPERTIES.iter().position(|p| p.name.to_lowercase().starts_with(&n))
            });
            let Some(k) = k else {
                eprintln!("error: unknown property {name}");
                std::process::exit(1)
            };
            app.deal_offer(o.sandbox);
            app.start_game(&mut g, k, o.sandbox);
            if let Some(c) = o.cash {
                // the property's price came out of the default funds; the test hook sets the cash after it
                app.econ.start_cash = c;
                app.econ.cash = c;
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
        }
        app.report_course(true);
        let t = now();
        app.clock = t;
        Stage {
            g,
            app,
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
            pause_toggle: false,
            shown_version: 0,
        }
    }

    fn toggle_pause(&mut self) {
        let t = now();
        self.app.paused = !self.app.paused;
        if self.app.paused {
            self.pause_start = t;
        } else {
            self.paused_total += t - self.pause_start;
        }
    }

    /// What a popup menu's choice does (docs/DECODE_MENUS.md 3, 4, 5).
    fn popup_done(&mut self, r: popup_ui::PopupResult) {
        let Some(kind) = self.app.popup.as_ref().map(|p| p.kind) else { return };
        let Some(k) = r else {
            self.app.close_popup();
            return;
        };
        if k == usize::MAX {
            return; // refused or toggled: the popup stays
        }
        self.app.close_popup();
        use popup_ui::PopupKind;
        match kind {
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
                _ => self.app.open_top10(None),
            },
            PopupKind::LandOffer => {
                if k == 0 {
                    self.app.open_land_screen();
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
                7 => {
                    self.app.screen = Screen::Menu;
                    self.app.hover = -1;
                }
                _ => {}
            },
            PopupKind::Prefs => {
                let m = k as u32;
                self.app.show_thoughts = m & 1 != 0;
                self.app.show_advisor = m & 2 != 0;
                self.app.club.wildlife.enabled = m & 4 != 0;
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
            }
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
        if app.panel == 4 {
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
        } else {
            hit = app.world_hit(vx, vy);
        }
        app.hover = hit;
        if !click || (hit < 0 && app.screen != Screen::Difficulty) {
            return;
        }
        if app.screen == Screen::Difficulty {
            if hit == 100 || hit < 0 {
                // the back button, or a click on nothing, goes back (0x43a400 returns -1)
                app.screen = Screen::Menu;
            } else if std::mem::take(&mut app.title.champ_pending) {
                app.difficulty = hit;
                app.open_championship();
            } else {
                app.difficulty = hit;
                app.deal_offer(app.sandbox_choice);
                app.screen = Screen::Property;
            }
            app.hover = -1;
            return;
        }
        if app.screen == Screen::Menu {
            match hit {
                0 => app.open_files(files_ui::ListKind::Load, false),
                1 | 2 | 4 => {
                    // Start New Game, Sandbox Mode and Play a Championship ask for the difficulty first (0x43a400)
                    app.sandbox_choice = hit == 2;
                    app.title.champ_pending = hit == 4;
                    app.screen = Screen::Difficulty;
                    app.hover = -1;
                }
                3 => app.open_themes(),
                6 => app.open_credits(),
                _ => window::order_quit(),
            }
        } else {
            app.world_click(&mut self.g, hit);
        }
    }

    /// The course screen's keys, as the exe's two key switches have them (character keys at 0x41c5b2, virtual keys at
    /// 0x41de29): lower-case letters pick tools, shifted letters open screens, digits jump to holes, arrows scroll four tiles,
    /// PageUp / Home turn the view a quarter, z / x zoom between the three levels.
    fn play_key(&mut self, k: KeyCode) {
        let app = &mut self.app;
        let shift = self.shift;
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
        if app.club.pro_aiming().is_some() && !shift {
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
                app.show_toast("Are you sure you want to cancel this round? Press N again to cancel.");
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
                KeyCode::F6 if self.screenshot(&PathBuf::from("simgolf-shot.png")) => println!("saved simgolf-shot.png"),
                _ => {}
            }
            return;
        }
        match k {
            KeyCode::Escape if app.pstate.hire_open => app.pstate.hire_open = false,
            KeyCode::Escape => {
                if app.ui_ok {
                    // the pause menu: System Functions over the stopped game
                    app.open_popup(popup_ui::PopupKind::System);
                    app.hover = -1;
                } else {
                    window::order_quit();
                }
            }
            // Space drops the tool and the selected golfer
            KeyCode::Space => {
                app.edit = false;
                app.card = None;
            }
            // the port's own game speed (the original has none): ] cycles normal, x2 and x4
            KeyCode::RightBracket => {
                app.speed = match app.speed {
                    1 => 2,
                    2 => 4,
                    _ => 1,
                };
                let name = match app.speed {
                    1 => "Normal speed",
                    2 => "Fast (x2)",
                    _ => "Fastest (x4)",
                };
                app.show_toast(name);
            }
            // Tab turns the next building; with the elevation tool it also steps the brush (one point, 2x2, area)
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
                // four tiles along the view's diagonal
                let step = 4.0 * std::f32::consts::SQRT_2 * sg_core::terrain::TILE_SIZE;
                match k {
                    KeyCode::Up => app.pan(0.0, step),
                    KeyCode::Down => app.pan(0.0, -step),
                    KeyCode::Left => app.pan(-step, 0.0),
                    _ => app.pan(step, 0.0),
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
                app.rot = 0.0;
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
                    app.show_toast("A new hole needs a tee and a green");
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
        if matches!(app.screen, Screen::Files | Screen::Themes | Screen::Credits) && app.ui_ok {
            app.draw_title_screen(&mut self.g);
        } else if app.screen == Screen::Menu && app.ui_ok {
            app.draw_menu(&mut self.g);
        } else if app.screen == Screen::Property && app.ui_ok {
            app.draw_property(&mut self.g);
        } else if app.screen == Screen::Difficulty && app.ui_ok {
            app.draw_difficulty(&mut self.g);
        } else {
            app.render_world(&mut self.g);
            app.draw_water_glints(&mut self.g);
            app.draw_building_label(&mut self.g);
            app.draw_cursor_overlay(&mut self.g);
            if !app.no_hud {
                app.draw_hud(&mut self.g);
            }
            if app.screen == Screen::Popup {
                app.draw_popup(&mut self.g);
            }
            app.draw_rename(&mut self.g);
            app.draw_save_dialog(&mut self.g);
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
                Screen::HoleStats => app.draw_hole_stats(&mut self.g),
                Screen::BestScores => app.draw_best_scores(&mut self.g),
                Screen::Top10 => app.draw_top10(&mut self.g),
                Screen::Customise => app.draw_customise(&mut self.g),
                _ => {}
            }
            app.draw_analysis(&mut self.g);
        }
        self.draw_touch_buttons();
        self.g.flush();
        self.g.ctx.end_render_pass();
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
            Screen::Menu | Screen::Difficulty | Screen::Property | Screen::Files | Screen::Themes | Screen::Credits
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
        let dir = self.app.course_file.parent().map(|p| p.to_path_buf()).unwrap_or_default().join("snapshots");
        sg_core::png::write_png(dir.join(format!("accomp{id}.png")), &out);
        let tex = self.g.texture(&out, false);
        self.app.snapshots.insert(id, ui::Image { tex: Some(tex), w: 200.0, h: 160.0 });
    }

    fn grab(&mut self) -> sg_core::assets::Rgba {
        let (w, h) = (self.app.draw_w as u32, self.app.draw_h as u32);
        let params = |format| TextureParams { width: w, height: h, format, ..Default::default() };
        let color = self.g.ctx.new_render_texture(params(TextureFormat::RGBA8));
        let depth = self.g.ctx.new_render_texture(params(TextureFormat::Depth));
        let pass = self.g.ctx.new_render_pass(color, Some(depth));
        self.draw_frame(Some(pass));
        let mut raw = vec![0u8; (w * h * 4) as usize];
        self.g.ctx.texture_read_pixels(color, &mut raw);
        self.g.ctx.delete_render_pass(pass);
        self.g.ctx.delete_texture(color);
        self.g.ctx.delete_texture(depth);
        let mut img = sg_core::assets::Rgba::new(w, h);
        let stride = (w * 4) as usize;
        for y in 0..h as usize {
            // GL origin is bottom-left
            img.px[y * stride..(y + 1) * stride].copy_from_slice(&raw[(h as usize - 1 - y) * stride..(h as usize - y) * stride]);
        }
        img.px.as_chunks_mut::<4>().0.iter_mut().for_each(|p| p[3] = 255);
        img
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
        let t = now();
        self.app.clock = t;
        self.touch_update();
        if self.pause_toggle {
            self.pause_toggle = false;
            self.toggle_pause();
        }
        let app = &mut self.app;
        app.screen_audio();
        if app.screen == Screen::Play {
            let hit = app.pick_ground(self.mouse.0, self.mouse.1);
            app.hover_tile = hit.map(|(x, z)| app.terrain.tile_of(x, z));
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
            let hit = match cursor_pin() {
                Some((x, y)) => {
                    let t = sg_core::terrain::TILE_SIZE;
                    Some((x * t - app.terrain.w as f32 * t * 0.5, y * t - app.terrain.h as f32 * t * 0.5))
                }
                None => app.pick_ground(self.mouse.0, self.mouse.1),
            };
            app.has_hit = hit.is_some();
            if let Some((x, z)) = hit {
                app.hit_x = x;
                app.hit_z = z;
            }
        }
        if app.dirty {
            app.rebuild_batches(&mut self.g);
            app.refresh_trees();
            app.dirty = false;
            app.report_course(false);
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
        {
            app.time += dt * app.speed.max(1) as f64;
        }
        app.club.turbo = app.speed > 1;
        let (w, h) = window::screen_size();
        app.draw_w = w.max(1.0);
        app.draw_h = h.max(1.0);
        app.dpi = window::dpi_scale();
    }

    fn draw(&mut self) {
        self.draw_frame(None);
        self.g.ctx.commit_frame();
        if !self.app.snapshot_due.is_empty() {
            while !self.app.snapshot_due.is_empty()
                && self.app.ui_ok
                && !matches!(
                    self.app.screen,
                    Screen::Menu | Screen::Difficulty | Screen::Property | Screen::Files | Screen::Themes | Screen::Credits
                )
            {
                self.take_snapshot();
            }
            if self.png_out.is_some() && self.app.snapshot_due.is_empty() && self.app.screen == Screen::Board {
                self.frames = 1; // a scripted still of the board waits one more frame for its photo
            }
        }
        self.frames += 1;
        if std::env::var_os("SG_PICKTEST").is_some() && self.frames == 2 {
            self.pick_test();
        }
        if let Some(out) = self.png_out.clone() {
            if self.frames >= 2 {
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
        let (dx, dy) = (x - self.mouse.0, y - self.mouse.1);
        self.mouse = (x, y);
        self.app.info.pointer = self.app.view.to_virtual(x, y);
        let (vx, vy) = self.app.view.to_virtual(x, y);
        self.app.card_pointer(vx, vy);
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
            self.app.pan(-dx * k, dy * k / sin_p);
        }
    }

    fn mouse_wheel_event(&mut self, _x: f32, y: f32) {
        if (self.app.screen == Screen::Play || !self.app.ui_ok) && y != 0.0 {
            self.app.zoom_step(y > 0.0);
        }
    }

    fn mouse_button_down_event(&mut self, button: MouseButton, x: f32, y: f32) {
        self.mouse = (x, y);
        if self.app.analysis_input() {
            return;
        }
        self.app.info.pointer = self.app.view.to_virtual(x, y);
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
        if self.app.screen != Screen::Play && self.app.ui_ok {
            if self.app.screen == Screen::Report {
                let (vx, vy) = self.app.view.to_virtual(x, y);
                self.app.report_click(vx, vy);
            } else if self.app.screen == Screen::HoleStats {
                self.app.screen = Screen::Report;
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
        if button != MouseButton::Left && button != MouseButton::Right {
            return;
        }
        let (vx, vy) = self.app.view.to_virtual(x, y);
        if self.app.ui_ok && button == MouseButton::Left && self.app.card_click(vx, vy) {
            return;
        }
        if self.app.ui_ok && self.dock_click(vx, vy, button == MouseButton::Right) {
            return;
        }
        if self.app.ui_ok && button == MouseButton::Left && self.app.strip_click(vx, vy) {
            return;
        }
        if button == MouseButton::Left && self.app.club.pro_aiming().is_some() && !self.app.edit {
            if let Some((hx, hz)) = self.app.pick_ground(x, y) {
                self.app.aim_pointer(hx, hz);
            }
            self.app.aim_click();
            return;
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
            self.app.apply_tool(self.shift, false);
        } else {
            self.dragging = true;
            if button == MouseButton::Left {
                if let Some((hx, hz)) = self.app.pick_ground(x, y) {
                    self.app.course_click(hx, hz);
                }
            }
        }
    }

    fn mouse_button_up_event(&mut self, _button: MouseButton, _x: f32, _y: f32) {
        self.dragging = false;
        self.editing = false;
    }

    fn key_down_event(&mut self, k: KeyCode, mods: KeyMods, _repeat: bool) {
        self.shift = mods.shift || k == KeyCode::LeftShift || k == KeyCode::RightShift;
        self.app.shift_held = self.shift;
        self.ctrl = mods.ctrl || k == KeyCode::LeftControl || k == KeyCode::RightControl;
        if self.app.rename_key(k) || self.app.save_key(k) || self.app.analysis_input() {
            return;
        }
        if self.app.screen == Screen::Popup {
            if let Some(r) = self.app.popup_key(k) {
                self.popup_done(r);
            }
            return;
        }
        if self.app.screen != Screen::Play && self.app.ui_ok {
            let app = &mut self.app;
            if app.screen == Screen::Report {
                if k == KeyCode::Escape || k == KeyCode::F1 {
                    app.screen = Screen::Play;
                    app.hover = -1;
                }
            } else if app.screen == Screen::HoleStats {
                app.screen = Screen::Report;
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
            ) {
                app.close_info();
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
                    window::order_quit();
                } else {
                    app.screen = Screen::Menu;
                }
                app.hover = -1;
            }
            return;
        }
        self.play_key(k);
    }

    fn char_event(&mut self, c: char, _mods: KeyMods, _repeat: bool) {
        self.app.rename_char(c);
        self.app.save_char(c);
        self.app.cust_char(c);
    }

    fn touch_event(&mut self, phase: TouchPhase, id: u64, x: f32, y: f32) {
        self.touch(phase, id, x, y);
    }

    fn key_up_event(&mut self, k: KeyCode, mods: KeyMods) {
        self.shift = mods.shift && k != KeyCode::LeftShift && k != KeyCode::RightShift;
        self.app.shift_held = self.shift;
        self.ctrl = mods.ctrl && k != KeyCode::LeftControl && k != KeyCode::RightControl;
    }
}

fn main() {
    let o = parse_args();
    let conf = conf::Conf {
        window_title: "SimGolf native".to_string(),
        window_width: o.win_w,
        window_height: o.win_h,
        high_dpi: true,
        window_resizable: true,
        // 32-bit mesh indices need WebGL 2 in a browser.
        platform: conf::Platform { webgl_version: conf::WebGLVersion::WebGL2, ..Default::default() },
        ..Default::default()
    };
    miniquad::start(conf, move || Box::new(Stage::new(o)));
}
