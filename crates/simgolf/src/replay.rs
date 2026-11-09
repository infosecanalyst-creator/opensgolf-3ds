//! The action log and its replays (the port's own; CONTRIBUTING.md "Replays"). Everything the player does to a game is
//! an [`Action`] (sg_core::action), performed by [`App::apply`] and nowhere else: the interface builds the action and hands
//! it to [`App::act`], which logs it with the game tick and applies it. With what the game around the simulation hands it
//! each tick (the pointer's tile, the flags set before a tick, the view the golfers' screen places are measured in, the
//! redraw's refresh of the course) logged as well, the game as it was when the log began plus the log plays the game
//! again exactly:
//!
//! * **exact replays for bug reports:** the log of the actions since the last load or new game is kept in memory (at most
//!   [`MAX_ENTRIES`] entries: past them it starts again from the game as it then is), and Report a problem writes it with
//!   the game it starts from as a `.sgr` file next to the saved game in the report's folder. `opensgolf --replay FILE`
//!   loads that game and plays the actions again at the ticks they were applied at, comparing the game's hash every
//!   [`CHECK_EVERY`] ticks with the one logged and saying at which tick it first differs ("replay diverged at tick T").
//! * **exact undo**, in time: the log is what an undo that replays the game up to before the step would read.
//!
//! An action is applied between two game ticks: stamped `t`, it was applied when the game had played `t` ticks, and a
//! replay applies it just before playing tick `t` (sg_core::action's notes). While a replay plays, the player's own actions
//! are refused (the speed keys still change how fast it goes); once the log is played through, the game goes on as usual
//! and its log on from there. Nothing is ever sent anywhere: the log only goes into the report's folder the player chooses
//! to send.
//!
//! `SG_ACTION_CHECK=1` (a test hook) compares the game's hash before and after each frame's input and says when the game
//! changed with no action applied, which names what still goes round `apply`.

use crate::app::{read_save_bytes, App, Screen};
use crate::build_tools::Area;
use sg_core::action::{
    decode_replay, encode_replay, Action, Divergence, Entry, Log, Rules, Start, TickFlags, View, CHECK_EVERY, MAX_ENTRIES,
};
use sg_core::lang::tr;

/// The extension of a replay file.
pub const EXT: &str = "sgr";

/// The log as it is being kept, and a replay under way.
#[derive(Default)]
pub struct Recorder {
    /// The actions since the log began (None: no game, the course editor, or not begun).
    pub log: Option<Log>,
    /// The game the log starts from, as a save file's bytes.
    base: Vec<u8>,
    /// A new game or a load: the log begins again before the next action or game tick.
    pending: bool,
    /// Inside `apply` or a game tick: what is done there is the action's or the tick's, not logged on its own.
    depth: u32,
    /// What was last logged of the pointer, the flags and the view (logged again only when they change).
    last_pointer: Option<Option<(i32, i32)>>,
    last_flags: Option<TickFlags>,
    last_view: Option<View>,
    last_rules: Option<Rules>,
    /// The speed the game is played at (the exe's fast mode follows it): the speed keys' own during play, the log's during a
    /// replay (where the keys only say how fast it goes by).
    pub sim_speed: u8,
    pub replay: Option<Replay>,
    /// How the last replay ended: the first tick it diverged at, if it did.
    pub last_divergence: Option<Divergence>,
    /// The player was told once that the replay refuses his actions.
    told: bool,
}

/// A replay under way.
pub struct Replay {
    entries: Vec<Entry>,
    next: usize,
    flags: TickFlags,
    view: View,
    /// The first hash that differed, and how many were compared.
    pub diverged: Option<Divergence>,
    pub checks: usize,
}

impl Recorder {
    pub fn replaying(&self) -> bool {
        self.replay.is_some()
    }

    /// Not inside an action or a tick: what is done now comes from the interface.
    pub fn depth_zero(&self) -> bool {
        self.depth == 0
    }
}

impl App {
    /// The interface's way to change the game: the action is logged with the game tick and applied. Returns what the
    /// action's own code returns (whether it did something, where it says). During a replay the player's actions are
    /// refused.
    pub fn act(&mut self, a: Action) -> bool {
        if self.rec.replaying() && self.rec.depth == 0 && !matches!(a, Action::Speed(_)) {
            if !std::mem::replace(&mut self.rec.told, true) {
                self.show_toast(tr("STR_TOAST_REPLAY_PLAYING"));
            }
            return false;
        }
        if self.rec.replaying() {
            // the speed keys during a replay only change how fast it goes by
            if let Action::Speed(s) = a {
                self.speed = s;
            }
            return true;
        }
        if self.rec.depth == 0 {
            self.log_begin_if_pending();
            self.log_rules();
            self.log_push(a.clone());
        }
        self.apply(&a)
    }

    /// Performs an action: the one place where the player's doing changes the game.
    pub fn apply(&mut self, a: &Action) -> bool {
        self.rec.depth += 1;
        let r = self.apply_inner(a);
        self.rec.depth -= 1;
        r
    }

    fn set_brush(&mut self, ty: i32, vbyte: i32, turn: Option<i32>, look: Option<u32>) {
        if let Some(k) = crate::app::PAINT.iter().position(|p| p.ty == ty && p.vbyte == vbyte) {
            self.paint_idx = k;
        }
        // a turn rolled for another brush stays as it is (it is not this brush's, and is rolled again as used)
        match turn {
            Some(v) => self.paint_variant = Some((self.paint_idx, v)),
            None if self.brush_turn().is_some() => self.paint_variant = None,
            None => {}
        }
        if let Some(s) = look {
            self.look_rng.state = s;
        }
    }

    /// The brush's turn as an action holds it: only when it was rolled for the brush in hand (else it is rolled as used).
    pub fn brush_turn(&self) -> Option<i32> {
        self.paint_variant.filter(|&(k, _)| k == self.paint_idx).map(|(_, v)| v)
    }

    /// The looks' generator as a paint action holds it: only with the Random trees extra, whose looks it picks.
    pub fn brush_look(&self) -> Option<u32> {
        crate::extras::random_trees().then_some(self.look_rng.state)
    }

    /// The paint action of the brush in hand over these tiles.
    pub fn paint_action(&self, tiles: Vec<(i32, i32)>) -> Action {
        let p = &crate::app::PAINT[self.paint_idx];
        Action::Paint { ty: p.ty, vbyte: p.vbyte, turn: self.brush_turn(), look: self.brush_look(), tiles, tee: self.tee_tool_step() }
    }

    /// The tee the tee brush places now, as an action holds it (0 the hole's own, 1 forward, 2 middle).
    pub fn tee_tool_step(&self) -> u8 {
        match self.tee_tool() {
            sg_core::extras::Tee::Back => 0,
            sg_core::extras::Tee::Forward => 1,
            sg_core::extras::Tee::Middle => 2,
        }
    }

    /// The building action of the kind in hand on tile x, y.
    pub fn build_action(&self, x: i32, y: i32, remove: bool) -> Action {
        Action::Build { x, y, kind: self.build_idx as i32, design: self.design, turn: self.turn, remove, style: self.path_kind }
    }

    fn apply_inner(&mut self, a: &Action) -> bool {
        match a {
            Action::Sync => {
                self.sync_course();
                true
            }
            Action::Rules(r) => {
                set_rules(*r);
                true
            }
            Action::TeeArrows(camera) => {
                self.tee_arrows(*camera);
                true
            }
            Action::Grown(tiles) => {
                self.sprouts_grown(tiles);
                true
            }
            Action::Pointer(p) => {
                self.hover_tile = *p;
                true
            }
            Action::Flags(f) => {
                if let Some(r) = self.rec.replay.as_mut() {
                    r.flags = *f;
                }
                true
            }
            Action::View(v) => {
                if let Some(r) = self.rec.replay.as_mut() {
                    r.view = *v;
                }
                true
            }
            Action::Screen => {
                let v = self.rec.replay.as_ref().map(|r| r.view).unwrap_or_else(|| self.sim_view());
                self.measure_screen(&v);
                true
            }
            Action::Check(logged) => {
                let now = self.replay_hash();
                if let Some(r) = self.rec.replay.as_mut() {
                    r.checks += 1;
                    if now != *logged && r.diverged.is_none() {
                        r.diverged = Some(Divergence { tick: self.game_tick, logged: *logged, replayed: now });
                        let msg = format!("replay diverged at tick {} (hash {now:016x}, logged {logged:016x})", self.game_tick);
                        crate::bugreport::warn(&msg);
                        println!("{msg}");
                    }
                }
                now == *logged
            }
            Action::Speed(s) => {
                self.speed = *s;
                self.rec.sim_speed = *s;
                self.club.turbo = *s > 1;
                true
            }
            Action::PickBrush { ty, vbyte } => {
                let Some(k) = crate::app::PAINT.iter().position(|p| p.ty == *ty && p.vbyte == *vbyte) else { return false };
                self.pick_brush(k);
                true
            }
            Action::Paint { ty, vbyte, turn, look, tiles, tee } => {
                self.set_brush(*ty, *vbyte, *turn, *look);
                self.tee_forced = Some(sg_core::extras::Tee::of_step(*tee));
                self.paint_tiles(tiles);
                self.tee_forced = None;
                true
            }
            Action::PaintArea { ty, vbyte, turn, look, tiles, too_large, tee } => {
                self.set_brush(*ty, *vbyte, *turn, *look);
                self.tee_forced = Some(sg_core::extras::Tee::of_step(*tee));
                self.paint_area(&Area { tiles: tiles.clone(), rect: None, too_large: *too_large });
                self.tee_forced = None;
                true
            }
            Action::CourseSwitch(b) => {
                if *b != self.hole_b_view {
                    self.toggle_hole_course();
                }
                self.hole_b_view == *b
            }
            Action::CourseShown(b) => {
                self.hole_b_view = *b;
                true
            }
            Action::Path { x, y, brush, remove, style } => {
                self.brush = *brush;
                self.path_kind = *style;
                self.edit_path(*x, *y, *remove);
                true
            }
            Action::Build { x, y, kind, design, turn, remove, style } => {
                self.build_idx = (*kind).clamp(0, 19) as usize;
                self.design = *design;
                self.turn = *turn;
                self.path_kind = *style;
                self.edit_building(*x, *y, *remove);
                true
            }
            Action::Row { from, to, kind, design, turn, remove: _, style } => {
                self.build_idx = (*kind).clamp(0, 19) as usize;
                self.design = *design;
                self.turn = *turn;
                self.path_kind = *style;
                self.place_row(*from, *to);
                true
            }
            Action::Elevation { x, y, tool, up } => {
                self.pstate.elev_tool = (*tool).max(0) as usize;
                self.edit_elevation(*x, *y, if *up { 1 } else { -1 });
                true
            }
            Action::Raise { x, y, brush, delta } => {
                self.brush = *brush;
                self.edit_raise(*x, *y, *delta);
                true
            }
            Action::TakeBack { x, y } => {
                self.undo_tile(*x, *y);
                true
            }
            Action::UndoPress => {
                self.undo_press();
                true
            }
            Action::UndoRelease => {
                self.undo_release();
                true
            }
            Action::Undo => self.undo_click_top(),
            Action::Analysis { x, y, nearest } => {
                self.start_analysis(Some((*x, *y)), *nearest);
                true
            }
            Action::OpenHole => self.open_hole(),
            Action::MoveHole { from, to } => {
                self.move_hole(*from, *to);
                true
            }
            Action::HoleFlags { hole, flags } => {
                if let Some(h) = self.club.holes.get_mut(*hole) {
                    h.flags = *flags;
                }
                true
            }
            Action::SetPar { hole, par } => self.club.set_par((*hole).clamp(1, 18) as usize, *par),
            Action::Hire { kind, skilled } => self.hire_employee(*kind, *skilled),
            Action::Fire { employee } => self.fire_employee(*employee),
            Action::Post { employee, x, y } => match self.employees.get_mut(*employee) {
                Some(e) => {
                    e.post = Some((*x, *y));
                    e.steps = 0;
                    true
                }
                None => false,
            },
            Action::Pro { button } => {
                self.pro_button(*button);
                true
            }
            Action::ShotOption(o) => {
                self.club.set_shot_option(*o);
                true
            }
            Action::Aim { x, y, corner } => {
                self.aim = self.club.aim_frame(&self.course, (*x, *y), *corner);
                true
            }
            Action::Swing => {
                if self.club.pro_aiming().is_none() {
                    return false;
                }
                if self.club.commit_aim() {
                    self.aim = None;
                }
                true
            }
            Action::Skill { skill, value } => match self.club.pro_skill.get_mut(*skill) {
                Some(v) => {
                    *v = *value;
                    true
                }
                None => false,
            },
            Action::SkillsDone { points, then } => {
                self.club.skill_points = (*points).max(0);
                self.finish_skills(*then);
                true
            }
            Action::ProLeft(l) => {
                self.club.pro_left = *l;
                true
            }
            Action::GreatShotDone => {
                self.great_shot_release();
                true
            }
            Action::SgaReport => {
                self.sga_report();
                true
            }
            Action::Snapshot { golfer } => {
                for gg in [*golfer, *golfer ^ 1] {
                    if let Some(g) = self.club.g.get_mut(gg).filter(|g| g.timer == 0) {
                        g.timer = 1;
                    }
                }
                true
            }
            Action::CancelRound => {
                if self.club.game & sg_core::golfer::game::TOURNAMENT != 0 {
                    self.cancel_tournament();
                } else {
                    let mut rng = self.exe_rng;
                    self.club.cancel_pro_round(&mut rng);
                    self.exe_rng = rng;
                }
                true
            }
            Action::Person { record, person, slot, toggles } => {
                let Ok(p) = serde_json::from_value::<sg_core::roster::Person>(person.clone()) else { return false };
                let Some(r) = self.club.roster.get_mut(*record) else { return false };
                *r = p;
                r.fixed = (r.fixed & !7) | *toggles as i32;
                let b21 = r.b21;
                if let Some(g) = slot.filter(|&g| g < sg_core::golfer::SLOTS) {
                    let gg = &mut self.club.g[g];
                    gg.class = (gg.class & 0xf8) | toggles;
                    gg.looks = b21 as i8 as i16 as u16;
                }
                true
            }
            Action::Top10 => {
                self.top10_drawn = Some(sg_core::top10::defaults(&mut self.exe_rng));
                true
            }
            Action::MeasureHole => {
                let _ = self.on_hole_course(|a| a.club.building_figures(&a.course));
                true
            }
            Action::BeginTournament => {
                self.begin_tournament();
                true
            }
            Action::AcceptTournament => {
                self.accept_tournament();
                true
            }
            Action::CancelTournament => {
                self.cancel_tournament();
                true
            }
            Action::TourneyAnswer(k) => {
                self.offer_answer(*k);
                true
            }
            Action::TourneyPrep(mask) => match self.prep.take() {
                Some((prep, _)) => {
                    self.finish_prep(prep, *mask);
                    true
                }
                None => false,
            },
            Action::TourneyDone => {
                if self.club.championship() {
                    self.club.game = 0;
                }
                true
            }
            Action::LandScreen => {
                self.open_land_screen();
                true
            }
            Action::LandHover(at) => {
                self.land_hover_to(*at);
                true
            }
            Action::Tract { tract } => self.buy_tract(*tract),
            Action::DropGolfer { golfer, x, y, clubhouse } => {
                self.card_ui.held = Some(*golfer);
                let (wx, wz) = self.units_to_world(*x, *y);
                self.golfer_click(wx, wz, *clubhouse)
            }
            Action::Pair { a, b } => self.club.pick_pair(&mut self.course, &mut self.exe_rng, *a, *b),
            Action::Chapter { golfer } => self.club.next_chapter(&mut self.course, &mut self.exe_rng, *golfer),
            Action::Extras(e) => {
                self.set_extras(*e);
                true
            }
            Action::Turn { x, y } => self.turn_building(*x, *y),
            Action::PathStyle => self.next_path_style(),
            Action::Cheat(k) => {
                self.use_cheat(*k);
                true
            }
            Action::Rename(n) => {
                self.club.course_name = n.clone();
                self.course_name = n.clone();
                true
            }
            Action::ContinueSandbox => {
                self.econ.continue_in_sandbox();
                true
            }
            Action::Script(item) => {
                crate::apply_edit_spec(self, item);
                true
            }
        }
    }

    // ---- the log ---------------------------------------------------------------------------------------------------------

    /// A new game or a load: the log begins again from the game as it will be before the next action or game tick.
    pub fn log_restart(&mut self) {
        self.rec.log = None;
        self.rec.pending = true;
        self.rec.replay = None;
        self.rec.sim_speed = self.speed.max(1);
    }

    /// Whether the log is kept for the game now: a game on, and not the course editor's.
    fn log_wanted(&self) -> bool {
        self.land.is_some() && self.editor.is_none()
    }

    /// The log begins, when a new game or load asked for it: the game as it is now and what no save keeps.
    pub(crate) fn log_begin_if_pending(&mut self) {
        if !self.rec.pending {
            return;
        }
        self.rec.pending = false;
        if !self.log_wanted() || self.rec.replaying() {
            return;
        }
        let Ok(base) = self.save_bytes(1) else { return };
        let start = self.log_start();
        self.rec.base = base;
        self.rec.last_pointer = Some(start.pointer);
        self.rec.last_flags = Some(start.flags);
        self.rec.last_view = None;
        self.rec.last_rules = Some(start.rules);
        self.rec.log = Some(Log::new(start));
    }

    /// The game's hash the log keeps: the fingerprint (fingerprint.rs) but for what the game hands the simulation at the
    /// next tick (the pointer's tile), which the log has when that tick comes.
    pub fn replay_hash(&self) -> u64 {
        let mut f = self.fingerprint();
        f.parts.retain(|p| p.0 != "inputs");
        f.total()
    }

    /// What the log's start holds besides the save.
    fn log_start(&self) -> Start {
        let c = &self.club;
        Start {
            tick: self.game_tick,
            screen: c.g.iter().map(|g| (g.sx, g.sy)).collect(),
            flags: TickFlags {
                busy: c.ticker_busy,
                advisor: c.advisor,
                close: c.view_close,
                message: c.message_up,
                snapshots: c.show_snapshots,
            },
            award_snaps: c.award_snaps.clone(),
            pointer: self.hover_tile,
            auto_aim: self.auto_aim,
            retyped: self.retyped.clone(),
            staff_golfers: self.staff_golfers.clone(),
            deco_rng: Some(self.deco_rng),
            planned: self.plan_is_current(),
            speed: self.rec.sim_speed.max(1),
            rules: rules(),
            course_b_view: self.hole_b_view,
            no_weather: c.place == sg_core::weather::Place::default() && c.today.is_none(),
            announced: c.announced,
            drag: c.planner.drag,
            bounce_cut: c.planner.bounce_cut,
            hash: self.replay_hash(),
            parts: self.start_parts(),
        }
    }

    /// The fingerprint's sections and the club's, the economy's and the course's fields, by name.
    fn start_parts(&self) -> Vec<(String, u64)> {
        let mut v: Vec<(String, u64)> = self.fingerprint().parts.into_iter().map(|(k, h)| (k.to_string(), h)).collect();
        v.extend(self.fingerprint_field_list());
        v
    }

    /// One entry on the log, stamped with the game tick; a full log starts again from the game as it then is.
    fn log_push(&mut self, a: Action) {
        let tick = self.game_tick;
        let Some(log) = self.rec.log.as_mut() else { return };
        // a run of redraws with nothing between is one
        if a == Action::Sync && log.entries.last().is_some_and(|e| e.tick == tick && e.action == Action::Sync) {
            return;
        }
        log.entries.push(Entry { tick, action: a });
        if log.entries.len() >= MAX_ENTRIES {
            self.rec.log = None;
            self.rec.pending = true;
        }
    }

    /// Something the game around the simulation hands it, logged where the game does it (not while a replay plays it).
    fn log_input(&mut self, a: Action) {
        if self.rec.pending || self.rec.replaying() {
            return;
        }
        self.log_push(a);
    }

    /// The course brought up to date with the land outside any action or tick (a redraw, a report): logged, as it plans the
    /// hole being built again after a terrain change. During a replay only the log's do it. Returns false when the caller
    /// must leave it.
    pub(crate) fn sync_hook(&mut self) -> bool {
        if self.rec.depth > 0 || self.rec.log.is_none() && !self.rec.replaying() {
            return true;
        }
        if self.rec.replaying() {
            return false;
        }
        self.log_input(Action::Sync);
        true
    }

    /// The start of a game tick: the log begins if a load asked for it, a replay applies what is due, the pointer's tile is
    /// logged when the pro's employee follows it. The tick itself is not logged.
    pub(crate) fn tick_begin(&mut self) {
        self.log_begin_if_pending();
        self.replay_due();
        self.log_rules();
        if !self.rec.replaying() && self.rec.log.is_some() {
            let follow = self.club.holes.get(1).is_some_and(|h| h.par == 0);
            let p = self.hover_tile;
            if follow && self.rec.last_pointer != Some(p) {
                self.rec.last_pointer = Some(p);
                self.log_input(Action::Pointer(p));
            }
        }
        self.rec.depth += 1;
    }

    /// The end of a game tick: every [`CHECK_EVERY`] ticks a hash of the game goes on the log.
    pub(crate) fn tick_end(&mut self) {
        self.rec.depth -= 1;
        if self.game_tick.is_multiple_of(CHECK_EVERY) && !self.rec.replaying() && self.rec.log.is_some() {
            let h = self.replay_hash();
            self.log_input(Action::Check(h));
        }
    }

    /// The tees' arrows changed by the drawing, logged (replay.rs).
    pub(crate) fn log_tee_arrows(&mut self, camera: i32) {
        if self.rec.depth == 0 {
            self.log_input(Action::TeeArrows(camera));
        }
    }

    /// Plants the drawing saw grown, logged.
    pub(crate) fn log_grown(&mut self, tiles: &[(i32, i32)]) {
        if self.rec.depth == 0 {
            self.log_input(Action::Grown(tiles.to_vec()));
        }
    }

    /// The settings that change the rules, logged when they have changed since last logged.
    fn log_rules(&mut self) {
        if self.rec.log.is_none() || self.rec.replaying() {
            return;
        }
        let r = rules();
        if self.rec.last_rules != Some(r) {
            self.rec.last_rules = Some(r);
            self.log_input(Action::Rules(r));
        }
    }

    /// The flags the simulation is handed before a tick: from the screen during play (logged when they change), the log's
    /// during a replay.
    pub(crate) fn tick_flags(&mut self) -> TickFlags {
        if let Some(r) = &self.rec.replay {
            return r.flags;
        }
        let f = TickFlags {
            busy: self.ticker.busy() || self.card.is_some(),
            advisor: self.show_advisor,
            close: self.exe_zoom() > 3.0,
            message: self.ticker.showing,
            snapshots: self.show_snapshots,
        };
        if self.rec.log.is_some() && self.rec.last_flags != Some(f) {
            self.rec.last_flags = Some(f);
            self.log_input(Action::Flags(f));
        }
        f
    }

    /// The view the golfers' screen places are measured in, as the last frame drew the course.
    pub fn sim_view(&self) -> View {
        View { mv: self.mv, upp: self.upp, w: self.draw_w, h: self.draw_h, scale: self.view.scale, ox: self.view.ox, oy: self.view.oy }
    }

    /// Where each golfer is on the 800 x 600 screen in view `v`, or -1 when off it (screen_of's arithmetic, step for step).
    fn screen_places(&self, v: &View) -> Vec<(usize, (i32, i32))> {
        let mut out = Vec::new();
        for gi in 0..sg_core::golfer::SLOTS {
            let g = &self.club.g[gi];
            if g.hole == 0 {
                continue;
            }
            let (wx, wz) = self.units_to_world(g.x, g.y);
            let h = self.terrain.height_at(wx, wz);
            let m = v.mv;
            let ex = m[0] * wx + m[4] * h + m[8] * wz + m[12];
            let ey = m[1] * wx + m[5] * h + m[9] * wz + m[13];
            let (px, py) = (ex / v.upp + v.w * 0.5, v.h * 0.5 - ey / v.upp);
            let (vx, vy) = ((px - v.ox) / v.scale, (py - v.oy) / v.scale);
            let on = (0.0..800.0).contains(&vx) && (0.0..600.0).contains(&vy);
            out.push((gi, if on { (vx as i32, vy as i32) } else { (-1, -1) }));
        }
        out
    }

    fn measure_screen(&mut self, v: &View) {
        for (gi, (sx, sy)) in self.screen_places(v) {
            self.club.g[gi].sx = sx;
            self.club.g[gi].sy = sy;
        }
    }

    /// After each drawn frame: each golfer's place on the screen, which the simulation reads (the exe's drawing records it).
    /// During play it is measured in the frame's view, and logged when it changes anything; during a replay only the log's
    /// measures count.
    pub(crate) fn screen_hook(&mut self) {
        if self.rec.replaying() {
            return;
        }
        let v = self.sim_view();
        let places = self.screen_places(&v);
        let changes = places.iter().any(|&(gi, p)| (self.club.g[gi].sx, self.club.g[gi].sy) != p);
        if changes && self.rec.log.is_some() && !self.rec.pending {
            if self.rec.last_view != Some(v) {
                self.rec.last_view = Some(v);
                self.log_input(Action::View(v));
            }
            self.log_input(Action::Screen);
        }
        for (gi, (sx, sy)) in places {
            self.club.g[gi].sx = sx;
            self.club.g[gi].sy = sy;
        }
    }

    // ---- replays -----------------------------------------------------------------------------------------------------------

    /// The replay file of the game being played: the game the log starts from and the log, with the game's hash now as its
    /// last entry. None without a log.
    pub fn replay_file(&mut self) -> Option<Vec<u8>> {
        self.log_begin_if_pending();
        let h = self.replay_hash();
        let tick = self.game_tick;
        let log = self.rec.log.as_ref()?;
        let mut log = log.clone();
        log.entries.push(Entry { tick, action: Action::Check(h) });
        encode_replay(&self.rec.base, &log).ok()
    }

    /// Starts a replay: the game the file's log starts from is loaded, with what no save keeps, and its actions are played
    /// at their ticks from the next tick on.
    pub fn start_replay(&mut self, g: Option<&mut crate::gfx::Gfx>, data: &[u8]) -> Result<(), String> {
        let (save, log) = decode_replay(data)?;
        let s = read_save_bytes(&save, "replay")?;
        self.keep_plan_on_load = log.start.planned;
        let r = match g {
            Some(g) => self.load_save(g, s),
            None => {
                self.undo.clear();
                self.editor = None;
                self.load_save_into(None, s, false)
            }
        };
        self.keep_plan_on_load = false;
        r?;
        let st = &log.start;
        for (g, &(sx, sy)) in self.club.g.iter_mut().zip(&st.screen) {
            g.sx = sx;
            g.sy = sy;
        }
        let c = &mut self.club;
        c.ticker_busy = st.flags.busy;
        c.advisor = st.flags.advisor;
        c.view_close = st.flags.close;
        c.message_up = st.flags.message;
        c.show_snapshots = st.flags.snapshots;
        c.award_snaps = st.award_snaps.clone();
        if st.no_weather {
            c.place = Default::default();
            c.today = None;
        }
        c.announced = st.announced;
        c.planner.drag = st.drag;
        c.planner.bounce_cut = st.bounce_cut;
        self.hover_tile = st.pointer;
        self.auto_aim = st.auto_aim;
        self.retyped = st.retyped.clone();
        self.staff_golfers = st.staff_golfers.clone();
        if let Some(r) = st.deco_rng {
            self.deco_rng = r;
        }
        set_rules(st.rules);
        self.hole_b_view = st.course_b_view;
        self.speed = st.speed.max(1);
        self.rec.sim_speed = self.speed;
        self.club.turbo = self.speed > 1;
        self.rec.pending = false;
        self.rec.base = save;
        let now = self.replay_hash();
        let mut replay =
            Replay { entries: log.entries.clone(), next: 0, flags: st.flags, view: View::default(), diverged: None, checks: 1 };
        if now != st.hash {
            let mine = self.start_parts();
            let differ: Vec<&str> =
                st.parts.iter().filter(|(k, h)| mine.iter().any(|(m, g)| m == k && g != h)).map(|(k, _)| k.as_str()).collect();
            let msg = format!(
                "replay diverged at tick {} (the loaded game's hash {now:016x}, logged {:016x}; differs in {})",
                st.tick,
                st.hash,
                differ.join(" ")
            );
            crate::bugreport::warn(&msg);
            println!("{msg}");
            replay.diverged = Some(Divergence { tick: st.tick, logged: st.hash, replayed: now });
        }
        println!(
            "replay: {} entries ({} actions) from tick {} to tick {}",
            log.entries.len(),
            log.player_actions(),
            st.tick,
            log.entries.last().map(|e| e.tick).unwrap_or(st.tick)
        );
        self.rec.last_pointer = Some(st.pointer);
        self.rec.last_flags = Some(st.flags);
        self.rec.last_view = None;
        self.rec.last_rules = Some(st.rules);
        self.rec.log = Some(Log { entries: log.entries.clone(), ..log });
        self.rec.replay = Some(replay);
        self.screen = Screen::Play;
        Ok(())
    }

    /// A replay applies the entries due before the tick about to be played; at the end of the log it says how it went and
    /// the game goes on as usual.
    pub(crate) fn replay_due(&mut self) {
        loop {
            let Some(r) = self.rec.replay.as_mut() else { return };
            let Some(e) = r.entries.get(r.next).filter(|e| e.tick <= self.game_tick).cloned() else {
                if r.next >= r.entries.len() {
                    self.replay_finish();
                }
                return;
            };
            r.next += 1;
            self.apply(&e.action);
        }
    }

    fn replay_finish(&mut self) {
        let Some(r) = self.rec.replay.take() else { return };
        let msg = match r.diverged {
            None => format!("replay: done at tick {}, the game matched at all {} checks", self.game_tick, r.checks),
            Some(d) => format!("replay: done at tick {}, diverged at tick {}", self.game_tick, d.tick),
        };
        println!("{msg}");
        self.show_toast(&if r.diverged.is_none() {
            tr("STR_TOAST_REPLAY_DONE").to_string()
        } else {
            sg_core::lang::trf("STR_TOAST_REPLAY_DIVERGED", &[("TICK", &r.diverged.map(|d| d.tick).unwrap_or(0))])
        });
        self.rec.last_view = None;
        self.rec.told = false;
        self.rec.last_divergence = r.diverged;
    }

    /// The first divergence of the replay under way or the last one played (tests).
    #[cfg(test)]
    pub fn replay_diverged(&self) -> Option<Divergence> {
        self.rec.replay.as_ref().map(|r| r.diverged).unwrap_or(self.rec.last_divergence)
    }

    // ---- the actions' own pieces ---------------------------------------------------------------------------------------

    /// The pointer onto another place of the land screen: the original rolls the prices again (keeping the first ones).
    pub fn land_hover_to(&mut self, hit: i32) {
        if hit != self.land_hover {
            self.land_hover = hit;
            if let Some(land) = self.land.as_ref() {
                let _ = sg_core::tracts::roll(land, self.club.purchases, &mut self.exe_rng);
            }
        }
    }
}

/// The settings that change the rules, as they are now.
fn rules() -> Rules {
    Rules { fix_bugs: crate::enhanced::fix_bugs(), random_trees: crate::extras::random_trees() }
}

/// A replay puts the log's settings in use for this run (not saved: the player's own come back at the next start).
fn set_rules(r: Rules) {
    if r != rules() {
        let mut e = crate::enhanced::settings();
        e.fix_original_bugs = r.fix_bugs;
        crate::enhanced::set(e);
        let mut x = crate::extras::settings();
        x.random_trees = r.random_trees;
        crate::extras::set(x);
    }
}

/// `SG_ACTION_CHECK=1`: the game's hash around each frame's input (main.rs), to find what changes the game with no action.
pub fn check_on() -> bool {
    static ON: std::sync::OnceLock<bool> = std::sync::OnceLock::new();
    *ON.get_or_init(|| std::env::var("SG_ACTION_CHECK").is_ok_and(|v| v.trim() == "1"))
}

/// What the check compares: the game's state, without what the game hands the simulation before each tick (logged there).
pub fn checked_parts(a: &App) -> Vec<(&'static str, u64)> {
    a.fingerprint().parts.into_iter().filter(|p| !matches!(p.0, "club_unsaved" | "inputs")).collect()
}

/// The parts that differ between two checks.
pub fn changed(before: &[(&'static str, u64)], after: &[(&'static str, u64)]) -> Vec<&'static str> {
    before.iter().zip(after).filter(|(a, b)| a.1 != b.1).map(|(a, _)| a.0).collect()
}

/// How many actions the log has had (the check's way to see whether a frame applied one).
pub fn logged(a: &App) -> usize {
    a.rec.log.as_ref().map(|l| l.entries.iter().filter(|e| e.action != Action::Sync).count()).unwrap_or(0)
}

#[cfg(test)]
mod tests;
