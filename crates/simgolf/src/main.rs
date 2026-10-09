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
mod gfx;
mod pro_ui;
mod render;
mod reports_ui;
mod screens_ui;
mod thoughts_ui;
mod tourney_ui;
mod ui;
mod wild_ui;

use app::*;
use gfx::Gfx;
use miniquad::*;
use render::*;
use sg_core::economy::{self, Economy};
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
        zoom: 0.36,
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
            "--zoom" => o.zoom = next().parse().unwrap_or(0.36),
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
}

fn load_ui(app: &mut App, g: &mut Gfx) -> bool {
    let font = ui::Font::load(g, &app.game_path("KLEPTO__.TTF"));
    let mut img = |rel: &str, magenta: bool, key: Option<u32>| ui::load_pcx(g, &app.game_path(&format!("Interface/{rel}")), magenta, key);
    let base = img("TitleBASE.pcx", false, None);
    let un = img("TitleUnSel.pcx", true, None);
    let mo = img("TitleMO.pcx", true, None);
    let world = img("WorldBase.pcx", false, None);
    let report = img("infoscreens/coursereport.pcx", true, None); // optional
    let dock = img("3mainLowerLeft.pcx", false, Some(0xF800F8)); // optional: the lower left dock
    const ICON: [&str; 4] = ["ChooseParklandButtons.pcx", "ChooseLinksButtons.pcx", "ChooseDesertButtons.pcx", "ChooseTropicalButtons.pcx"];
    let icons: Vec<Option<ui::Image>> = ICON.iter().map(|f| img(f, false, Some(0xFF0000))).collect(); // icons are optional
    let ok = font && base.is_some() && un.is_some() && mo.is_some() && world.is_some();
    app.title_base = base.unwrap_or_default();
    app.title_un = un.unwrap_or_default();
    app.title_mo = mo.unwrap_or_default();
    app.world_base = world.unwrap_or_default();
    app.report_art = report.unwrap_or_default();
    app.dock_art = dock.unwrap_or_default();
    for (t, i) in icons.into_iter().enumerate() {
        app.theme_icons[t] = i.unwrap_or_default();
    }
    app.ui_ok = ok;
    app.art = crate::screens_ui::Art::load(g, app);
    app.reports = crate::reports_ui::ReportArt::load(g, app);
    ok
}

/// Scripted edits for tests: "p:x,y,type[,vbyte,radius];w:x,y,kind[,radius];b:x,y,building;k:x,y,dir;r:cx,cy,delta[,radius];
/// h:kind[,x,y];t:x,y,type;o" (t paints one tile through the hole tool, o opens the hole being built; h hires an employee: 0 Club Pro, 1 Ranger, 2 Groundskeeper, 3 Soda Vendor; x,y is the post tile).
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
            b'k' if item.len() > 2 && v.len() == 3 => app.terrain.set_wall(at(0), at(1), at(2), true),
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
        if let Some(p) = o.panel {
            app.panel = p;
            app.dock_hover = if p > 0 { p - 1 } else { -1 };
            app.edit = p == 1 || p == 2;
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
                if app.audio.is_some() {
                    app.start_ambience();
                } else {
                    eprintln!("sound: no audio device, running silent");
                }
            } else if app.sound_log {
                app.start_ambience();
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
                    "land" => app.open_land_screen(),
                    _ => {}
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
            match o.screen.as_deref() {
                Some("land") => app.open_land_screen(),
                Some("report") if app.ui_ok => app.screen = Screen::Report,
                Some("pair") if app.ui_ok => app.screen = Screen::Pair,
                Some("board") if app.ui_ok => app.screen = Screen::Board,
                Some("yearend") if app.ui_ok => app.screen = Screen::YearEnd,
                Some("roster") if app.ui_ok => app.screen = Screen::Roster,
                Some("comments") if app.ui_ok => app.screen = Screen::Comments,
                Some("histograph") if app.ui_ok => app.screen = Screen::Histograph,
                Some("finance") if app.ui_ok => app.screen = Screen::Finance,
                Some("routing") if app.ui_ok => app.screen = Screen::Routing,
                Some("shortcuts") if app.ui_ok => app.screen = Screen::Shortcuts,
                Some("golfers") if app.ui_ok => app.panel = 4,
                Some("sga") if app.ui_ok => {
                    app.club.game |= sg_core::tournament::OFFERED;
                    app.begin_tournament();
                }
                Some("skills") if app.ui_ok => {
                    let pts = app.club.first_skill_points();
                    app.open_skills(pts, Some(false));
                }
                _ => {}
            }
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
            let p = std::fs::read(pro).ok().and_then(|b| sg_core::championship::parse_pro(&b)).unwrap_or_else(|| {
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
        }
        apply_edit_spec(&mut app, &o.edit_spec);
        if !o.edit_spec.is_empty() {
            app.rebuild_batches(&mut g);
            app.refresh_trees();
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

    fn save_course(&mut self, toast: bool) {
        let gf = self.app.game_file();
        match self.app.save_game(&gf) {
            Ok(()) => println!("saved game {}", gf.display()),
            Err(e) => eprintln!("error: {e}"),
        }
        let f = self.app.course_file.clone();
        match self.app.terrain.save(&f) {
            Ok(()) => {
                println!("saved {}", f.display());
                if toast {
                    self.app.show_toast("Course saved");
                }
            }
            Err(e) => {
                eprintln!("error: {e}");
                if toast {
                    self.app.show_toast("Could not save the course");
                }
            }
        }
    }

    fn load_course(&mut self) {
        let gf = self.app.game_file();
        if gf.exists() {
            match self.app.load_game(&mut self.g, &gf) {
                Ok(()) => {
                    println!("loaded game {}", gf.display());
                    return;
                }
                Err(e) => eprintln!("error: {e}"),
            }
        }
        let f = self.app.course_file.clone();
        match Terrain::load(&f, &self.app.terrain) {
            Ok(t) => {
                self.app.terrain = t;
                self.app.land = None;
                self.app.reset_holes();
                self.app.dirty = true;
                println!("loaded {}", f.display());
            }
            Err(e) => eprintln!("error: {e}"),
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
        let d = dock_hit(vx, vy);
        if d >= 0 {
            match d {
                0..=2 => {
                    let want = d + 1;
                    app.panel = if app.panel == want { 0 } else { want };
                    app.edit = app.panel == 1 || app.panel == 2;
                    if app.panel == 1 {
                        app.tool = 0;
                    }
                    if app.panel == 2 {
                        app.tool = 4;
                        if !app.build_available(app.build_idx) {
                            app.build_idx = OFFERED_KINDS[0] as usize;
                        }
                    }
                }
                3 => app.zoom *= 1.12,
                4 => app.zoom /= 1.12,
                5 => app.rot += 15.0,
                6 => app.rot -= 15.0,
                7 => self.open_report(),
                8 => self.pause_toggle = true,
                _ => self.save_course(true),
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
        if app.screen == Screen::Menu {
            for (b, r) in MENU_BTN.iter().enumerate() {
                if r.has(vx, vy) {
                    hit = b as i32;
                }
            }
        } else {
            for p in 0..16 {
                if property_card(p).has(vx, vy) {
                    hit = p as i32;
                }
            }
            if BACK_BUTTON.has(vx, vy) {
                hit = 100;
            }
            if DIFFICULTY_BUTTON.has(vx, vy) {
                hit = 101;
            }
        }
        app.hover = hit;
        if !click || hit < 0 {
            return;
        }
        if app.screen == Screen::Menu {
            match hit {
                0 if app.game_file().exists() => {
                    let gf = app.game_file();
                    if let Err(e) = app.load_game(&mut self.g, &gf) {
                        app.show_toast(&format!("Could not load the saved game: {e}"));
                    }
                }
                0 => match Terrain::load(&app.course_file, &app.terrain) {
                    Ok(t) => {
                        app.terrain = t;
                        app.land = None;
                        app.rebuild_batches(&mut self.g);
                        app.populate_props();
                        app.screen = Screen::Play;
                        app.reset_clock = true;
                        app.econ.sandbox = false;
                    }
                    Err(_) => app.show_toast("No saved game found (course.sgc)"),
                },
                1 | 2 => {
                    app.sandbox_choice = hit == 2;
                    app.deal_offer(app.sandbox_choice);
                    app.screen = Screen::Property;
                    app.hover = -1;
                }
                3 => app.theme_pack = (app.theme_pack + 1) % THEME_PACKS.len(),
                4 => app.open_championship(),
                _ => window::order_quit(),
            }
        } else if hit == 101 {
            app.difficulty = (app.difficulty + 1) % 4;
        } else if hit == 100 {
            app.screen = Screen::Menu;
            app.hover = -1;
        } else if app.can_afford(hit as usize) {
            app.start_game(&mut self.g, hit as usize, app.sandbox_choice);
        }
    }

    fn play_key(&mut self, k: KeyCode) {
        let app = &mut self.app;
        let step = 60.0 / app.zoom;
        let shift = self.shift;
        let pick = |app: &mut App, ty: u8, vb: i32| {
            app.tool = 0;
            if let Some(i) = PAINT.iter().position(|p| p.ty == ty as i32 && p.vbyte == vb) {
                app.paint_idx = i;
            }
        };
        // Hotkeys of the original (manual p. 3 and 4): Z / X zoom, Shift+S / Shift+L save and load, Shift+P pause, Shift+T trees;
        // in edit mode F fairway, G green/tee, R rough, S sandtrap, W water, P pathway, - lower, = raise.
        let edit = app.edit && !shift;
        if app.club.pro_aiming().is_some() && !shift {
            // the shot keys of the aiming panel (key switch, L169087): s straight, f fade, d draw, h high, l low
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
        if k == KeyCode::J && !edit {
            // the pro panel's first two buttons: J a practice round, Shift+J the match
            app.pro_button(if shift { 1 } else { 0 });
            return;
        }
        if k == KeyCode::K && !edit {
            app.pro_button(2);
            return;
        }
        match k {
            KeyCode::S if shift => return self.save_course(false),
            KeyCode::L if shift => return self.load_course(),
            KeyCode::P if shift => return self.toggle_pause(),
            KeyCode::T if shift => {
                app.show_props = !app.show_props;
                return;
            }
            KeyCode::C | KeyCode::R | KeyCode::G | KeyCode::V if shift || self.ctrl => {
                // Hire (Shift) or fire (Ctrl) a Club Pro, Ranger, Groundskeeper or Soda Vendor.
                let kind = match k {
                    KeyCode::C => economy::CLUB_PRO,
                    KeyCode::R => economy::RANGER,
                    KeyCode::G => economy::GROUNDSKEEPER,
                    _ => economy::SODA_VENDOR,
                };
                let ok = if shift { app.hire_staff(kind) } else { app.fire_staff(kind) };
                println!(
                    "{} {}: {} (staff now {}, wage ${} a charge)",
                    if shift { "hire" } else { "fire" },
                    Economy::staff_name(kind),
                    if ok { "done" } else { "not possible" },
                    app.econ.staff_count(),
                    economy::WAGE_UNITS[kind][0] * 100
                );
                app.snd("Interface/Button2.wav", 1.0, false);
                return;
            }
            KeyCode::F1 => return self.open_report(),
            KeyCode::Z => {
                app.zoom *= 1.12;
                return;
            }
            KeyCode::X => {
                app.zoom /= 1.12;
                return;
            }
            KeyCode::F if edit => return pick(app, TT_FAIRWAY, 0),
            KeyCode::G if edit => {
                let to = if PAINT[app.paint_idx].ty == TT_PUTTING_GREEN as i32 && app.tool == 0 { TT_TEE } else { TT_PUTTING_GREEN };
                return pick(app, to, 0);
            }
            KeyCode::R if edit => return pick(app, TT_ROUGH, 0),
            KeyCode::S if edit => return pick(app, TT_SAND, 0),
            KeyCode::W if edit => return pick(app, TT_WATER_SHALLOW, 0),
            KeyCode::P if edit => {
                app.tool = 2;
                return;
            }
            KeyCode::Minus if edit => {
                app.tool = 1;
                app.raise_sign = -1;
                return;
            }
            KeyCode::Equal if edit => {
                app.tool = 1;
                app.raise_sign = 1;
                return;
            }
            _ => {}
        }
        match k {
            KeyCode::Escape => {
                if app.ui_ok {
                    app.screen = Screen::Menu;
                    app.hover = -1;
                } else {
                    window::order_quit();
                }
            }
            KeyCode::Left | KeyCode::A => app.pan(-step, 0.0),
            KeyCode::Right | KeyCode::D => app.pan(step, 0.0),
            KeyCode::Up | KeyCode::W => app.pan(0.0, step),
            KeyCode::Down | KeyCode::S => app.pan(0.0, -step),
            KeyCode::Q => app.rot -= 5.0,
            KeyCode::E => app.rot += 5.0,
            KeyCode::Equal | KeyCode::KpAdd => app.zoom *= 1.12,
            KeyCode::Minus | KeyCode::KpSubtract => app.zoom /= 1.12,
            KeyCode::Key1 | KeyCode::Key2 | KeyCode::Key3 | KeyCode::Key4 => {
                let t = match k {
                    KeyCode::Key1 => 0,
                    KeyCode::Key2 => 1,
                    KeyCode::Key3 => 2,
                    _ => 3,
                };
                app.load_theme(&mut self.g, t);
            }
            KeyCode::R => {
                app.seed = app.seed.wrapping_mul(1664525).wrapping_add(1013904223);
                app.terrain = Terrain::demo_course(40, 40, app.seed);
                app.land = None;
                app.rebuild_batches(&mut self.g);
                app.populate_props();
            }
            KeyCode::Tab => app.edit = !app.edit,
            KeyCode::T => app.tool = (app.tool + 1) % 5,
            KeyCode::LeftBracket | KeyCode::RightBracket => {
                let fwd = k == KeyCode::RightBracket;
                if app.tool == 4 {
                    let avail: Vec<usize> = OFFERED_KINDS.iter().map(|&k| k as usize).filter(|&k| app.build_available(k)).collect();
                    if !avail.is_empty() {
                        let at = avail.iter().position(|&k| k == app.build_idx).unwrap_or(0);
                        let n = avail.len();
                        app.build_idx = avail[if fwd { (at + 1) % n } else { (at + n - 1) % n }];
                    }
                } else if app.tool == 2 {
                    app.path_kind = 3 - app.path_kind;
                } else if app.tool != 3 {
                    app.paint_idx = if fwd { (app.paint_idx + 1) % PAINT.len() } else { (app.paint_idx + PAINT.len() - 1) % PAINT.len() };
                }
            }
            KeyCode::Comma => app.brush = (app.brush - 1).max(0),
            KeyCode::Period => app.brush = (app.brush + 1).min(6),
            KeyCode::F2 => app.open_report_screen(Screen::Comments),
            KeyCode::F3 => app.open_report_screen(Screen::Histograph),
            KeyCode::F4 => app.open_report_screen(Screen::Finance),
            KeyCode::F5 => app.open_report_screen(Screen::Routing),
            KeyCode::F8 if !shift => app.open_report_screen(Screen::Shortcuts),
            // the original's report keys: F7 the SGA evaluation, F9 the Membership Roster, F10 the accomplishments board;
            // Shift+F7 / Shift+F8 save the course / the pro for championship play
            KeyCode::F7 if shift => app.save_championship_course(),
            KeyCode::F8 if shift => app.save_championship_pro(),
            KeyCode::F7 => app.sga_report(),
            KeyCode::F9 => {
                app.roster_offset = 0;
                app.screen = Screen::Roster;
            }
            KeyCode::F10 => app.screen = Screen::Board,
            KeyCode::M => app.toggle_music(),
            KeyCode::N => {
                app.mute = !app.mute;
                if app.mute {
                    if let Some(m) = &app.mixer {
                        m.stop_all();
                    }
                    app.ambience = -1;
                    app.music = -1;
                } else {
                    app.start_ambience();
                    if app.music_on {
                        app.music_on = false;
                        app.toggle_music();
                    }
                }
            }
            KeyCode::P => app.show_props = !app.show_props,
            KeyCode::H if shift => app.show_advisor = !app.show_advisor,
            KeyCode::H => {
                if !app.open_hole() {
                    app.show_toast("A new hole needs a tee and a green");
                }
            }
            KeyCode::F => app.follow = !app.follow,
            KeyCode::F6 if self.screenshot(&PathBuf::from("simgolf-shot.png")) => {
                println!("saved simgolf-shot.png");
            }
            _ => {}
        }
    }

    fn draw_frame(&mut self, target: Option<RenderPass>) {
        let app = &mut self.app;
        let menu_like = app.ui_ok && (app.screen == Screen::Menu || app.screen == Screen::Property);
        let clear = if menu_like { PassAction::clear_color(0.0, 0.0, 0.0, 1.0) } else { PassAction::clear_color(0.04, 0.06, 0.09, 1.0) };
        self.g.ctx.begin_pass(target, clear);
        if app.screen == Screen::Champ && app.ui_ok {
            app.draw_champ(&mut self.g);
        } else if app.screen == Screen::Menu && app.ui_ok {
            app.draw_menu(&mut self.g);
        } else if app.screen == Screen::Property && app.ui_ok {
            app.draw_property(&mut self.g);
        } else {
            app.render_world(&mut self.g);
            if !app.no_hud {
                app.draw_hud(&mut self.g);
            }
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
                _ => {}
            }
        }
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
        let Some((id, pt)) = self.app.snapshot_due else { return };
        if !matches!(self.app.screen, Screen::Play | Screen::Board) || !self.app.ui_ok {
            return;
        }
        let Some((vx, vy)) = self.app.screen_of(pt.0, pt.1) else {
            let (x, z) = self.app.units_to_world(pt.0, pt.1);
            self.app.cam_x = x;
            self.app.cam_z = z;
            return;
        };
        self.app.snapshot_due = None;
        // the snapshot is of the course, so a board that opened first is set aside for the grab
        let shown = std::mem::replace(&mut self.app.screen, Screen::Play);
        self.app.no_hud = true;
        let img = self.grab();
        self.app.no_hud = false;
        self.app.screen = shown;
        let v = self.app.view;
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
        let _ = std::fs::create_dir_all(&dir);
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
        if self.pause_toggle {
            self.pause_toggle = false;
            self.toggle_pause();
        }
        let app = &mut self.app;
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
            let hit = app.pick_ground(self.mouse.0, self.mouse.1);
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
        if app.screen != Screen::Play && !app.paused {
            self.paused_total += t - self.last_tick; // the club does not run while a menu is open
        }
        self.last_tick = t;
        if self.png_out.is_none() && !app.paused && app.screen == Screen::Play {
            app.time = t - self.paused_total;
        }
        let (w, h) = window::screen_size();
        app.draw_w = w.max(1.0);
        app.draw_h = h.max(1.0);
        app.dpi = window::dpi_scale();
    }

    fn draw(&mut self) {
        self.draw_frame(None);
        self.g.ctx.commit_frame();
        if self.app.snapshot_due.is_some() {
            self.take_snapshot();
            if self.png_out.is_some() && self.app.snapshot_due.is_none() && self.app.screen == Screen::Board {
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
        if self.app.screen != Screen::Play && self.app.ui_ok {
            if matches!(self.app.screen, Screen::Menu | Screen::Property) {
                self.menu_pointer(x, y, false);
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
            let k = 2.0 / (self.app.zoom * self.app.dpi); // drawable pixels -> world units
            let sin_p = (pitch_for(self.app.draw_w, self.app.draw_h).to_radians()).sin() as f32;
            self.app.pan(-dx * k, dy * k / sin_p);
        }
    }

    fn mouse_wheel_event(&mut self, _x: f32, y: f32) {
        if self.app.screen == Screen::Play || !self.app.ui_ok {
            self.app.zoom *= if y > 0.0 {
                1.1
            } else if y < 0.0 {
                1.0 / 1.1
            } else {
                1.0
            };
        }
    }

    fn mouse_button_down_event(&mut self, button: MouseButton, x: f32, y: f32) {
        self.mouse = (x, y);
        if self.app.screen == Screen::Land {
            let (vx, vy) = self.app.view.to_virtual(x, y);
            self.app.land_pointer(vx, vy);
            self.app.land_click(button == MouseButton::Right);
            return;
        }
        if self.app.screen != Screen::Play && self.app.ui_ok {
            if self.app.screen == Screen::Report {
                self.app.screen = Screen::Play;
                self.app.hover = -1;
            } else if self.app.screen == Screen::Champ {
                let (vx, vy) = self.app.view.to_virtual(x, y);
                self.app.champ_click(&mut self.g, vx, vy);
            } else if self.app.screen == Screen::Pair {
                let (vx, vy) = self.app.view.to_virtual(x, y);
                self.app.pair_click(vx, vy);
            } else if self.app.screen == Screen::Routing {
                let (vx, vy) = self.app.view.to_virtual(x, y);
                self.app.routing_click(vx, vy);
            } else if matches!(
                self.app.screen,
                Screen::Board
                    | Screen::YearEnd
                    | Screen::Roster
                    | Screen::Comments
                    | Screen::Histograph
                    | Screen::Finance
                    | Screen::Shortcuts
            ) {
                self.app.screen = Screen::Play;
            } else if self.app.screen == Screen::Skills {
                let (vx, vy) = self.app.view.to_virtual(x, y);
                self.app.skills_click(vx, vy);
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
        self.ctrl = mods.ctrl || k == KeyCode::LeftControl || k == KeyCode::RightControl;
        if self.app.screen != Screen::Play && self.app.ui_ok {
            let app = &mut self.app;
            if app.screen == Screen::Report {
                if k == KeyCode::Escape || k == KeyCode::F1 {
                    app.screen = Screen::Play;
                    app.hover = -1;
                }
            } else if app.screen == Screen::Champ {
                if k == KeyCode::Escape {
                    app.screen = Screen::Menu;
                }
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
            ) {
                app.screen = Screen::Play;
            } else if app.screen == Screen::Skills {
                if k == KeyCode::Enter || k == KeyCode::Escape {
                    app.skills_click(-1.0, -1.0);
                }
            } else if matches!(app.screen, Screen::Sga | Screen::Prep | Screen::Results) {
                if k == KeyCode::Enter || k == KeyCode::Escape {
                    app.tourney_click(-1.0, -1.0);
                }
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

    fn key_up_event(&mut self, k: KeyCode, mods: KeyMods) {
        self.shift = mods.shift && k != KeyCode::LeftShift && k != KeyCode::RightShift;
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
