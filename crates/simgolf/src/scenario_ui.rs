//! Scenarios and game scripts in the game (docs/MODDING.md 5 and 6): the Start New Game choice of scenario, starting one,
//! the goals and their report (Information > Scenario goals), the scripts of the enabled packs and their hooks, and
//! System Functions > Export as scenario.
//!
//! These are the port's own (the original had no scenarios or scripts); the screens use the Content screen's frame and
//! fonts. With no scenario packs installed Start New Game goes on as in the original, and with no scripts and no goals
//! nothing here runs.
use crate::app::*;
use crate::gfx::Gfx;
use crate::render::Rect;
use crate::ui::{rgb, Screen as Ui, F_ARIAL10, F_MANUAL15, F_MANUAL20};
use sg_core::economy;
use sg_core::lang::{tr, trf};
use sg_core::packs::{self, Kind};
use sg_core::scenario::{self, Facts, Goal, GoalDef, GoalState, Goals, Note, Outcome, Scenario, ScenarioDef};
use sg_core::script::{self, Action, GameView, GolferView, HoleView, Hook, Host, PackState};
use std::collections::{BTreeMap, VecDeque};

/// A scenario pack found: its id and name, and its scenario.toml (or what is wrong with it).
pub struct ScenarioEntry {
    pub id: String,
    pub name: String,
    pub description: String,
    pub def: Result<Scenario, String>,
}

/// The scenario and script state of the game in progress.
#[derive(Default)]
pub struct ScenarioState {
    /// The goals of the game (saved with it).
    pub goals: Goals,
    /// The scripts of the game.
    pub host: Host,
    /// Problems of the scripts this session, as (pack id, message): logged, and shown on the Content screen.
    pub errors: Vec<(String, String)>,
    /// Hooks waiting for the end of the game tick.
    pending: Vec<Hook>,
    /// Messages for the ticker, posted one at a time.
    messages: VecDeque<String>,
    /// The scenario packs found (the Start New Game screen).
    pub list: Vec<ScenarioEntry>,
    /// The row picked on the Start New Game screen (0: a free game, then the scenarios).
    pub picked: usize,
    pub scroll: usize,
    /// A scenario waiting for the Select Difficulty screen (its scenario.toml names no difficulty).
    pub chosen: Option<usize>,
    /// The scenario a game is being started with: (pack id, scenario), taken when the game starts.
    starting: Option<(String, Scenario)>,
    /// The cash a scenario starts with, taken by `start_game`.
    pub start_money: Option<i64>,
}

const ROWS: usize = 7;
const ROW_H: f32 = 52.0;
const TOP: f32 = 96.0;
const BOX: Rect = Rect { x: 40.0, y: 36.0, w: 720.0, h: 528.0 };

/// The scenario packs among the packs found.
pub fn find_scenarios(c: &crate::content::ContentState) -> Vec<ScenarioEntry> {
    let mut v: Vec<ScenarioEntry> = c
        .found
        .packs
        .iter()
        .filter(|p| p.manifest.kind == Kind::Scenario)
        .map(|p| {
            let def = match p.source.read("scenario.toml") {
                None => Err(tr("STR_SCENARIO_NO_FILE").to_string()),
                Some(d) => Scenario::parse(&String::from_utf8_lossy(&d)),
            };
            // the packs it needs must be loaded (they are chosen on the Content screen)
            let def = def.and_then(|s| match c.plan.as_ref() {
                Some(plan) => packs::check_against(&p.manifest, plan, &c.found).map(|_| s),
                None => Ok(s),
            });
            let name = def.as_ref().ok().and_then(|s| s.scenario.name.clone()).unwrap_or_else(|| p.manifest.name.clone());
            let description = match &def {
                Ok(s) if !s.scenario.description.is_empty() => s.scenario.description.clone(),
                _ => p.manifest.description.clone(),
            };
            ScenarioEntry { id: p.manifest.id.clone(), name, description, def }
        })
        .collect();
    v.sort_by_key(|e| e.name.to_lowercase());
    v
}

/// The facts the goals are checked against.
fn facts(app: &App) -> Facts {
    Facts {
        year: 2001 + app.econ.year_index() as i32,
        cash: app.econ.cash as i64,
        holes: app.hole_numbers.len() as i32,
        fun: app.club.ratings.fun,
        skill: app.club.ratings.skill,
        members: app.club.member_count(),
        rank: economy::rank(app.hole_numbers.len()),
    }
}

impl App {
    // ---- the Start New Game screen ---------------------------------------------------------------------------------------

    /// True when scenario packs are installed: Start New Game then offers them.
    pub fn has_scenarios(&self) -> bool {
        self.content.found.packs.iter().any(|p| p.manifest.kind == Kind::Scenario)
    }

    pub fn open_scenarios(&mut self) {
        self.scen.list = find_scenarios(&self.content);
        self.scen.picked = 0;
        self.scen.scroll = 0;
        self.scen.chosen = None;
        self.screen = Screen::Scenarios;
        self.hover = -1;
    }

    fn scenario_rows(&self) -> usize {
        self.scen.list.len() + 1
    }

    /// A click on the Start New Game screen.
    pub fn scenarios_click(&mut self, g: &mut Gfx, x: f32, y: f32) {
        if Rect::new(BOX.x + BOX.w - 70.0, BOX.y + BOX.h - 58.0, 50.0, 50.0).has(x, y) {
            return self.scenario_go(g);
        }
        if back_button().has(x, y) {
            self.screen = Screen::Menu;
            return;
        }
        let n = self.scenario_rows();
        if n > ROWS {
            if Rect::new(BOX.x + 24.0, TOP + ROWS as f32 * ROW_H + 4.0, 80.0, 20.0).has(x, y) {
                self.scen.scroll = self.scen.scroll.saturating_sub(ROWS);
                return;
            }
            if Rect::new(BOX.x + 110.0, TOP + ROWS as f32 * ROW_H + 4.0, 80.0, 20.0).has(x, y) {
                self.scen.scroll = (self.scen.scroll + ROWS).min(n.saturating_sub(ROWS));
                return;
            }
        }
        if y < TOP || !(BOX.x + 10.0..=BOX.x + BOX.w - 10.0).contains(&x) {
            return;
        }
        let k = ((y - TOP) / ROW_H) as usize;
        let i = self.scen.scroll + k;
        if k < ROWS && i < n {
            if self.scen.picked == i {
                // a second click starts it
                return self.scenario_go(g);
            }
            self.scen.picked = i;
        }
    }

    /// Keys: Up and Down pick, Enter starts, Esc goes back to the title.
    pub fn scenarios_key(&mut self, g: &mut Gfx, k: miniquad::KeyCode) {
        use miniquad::KeyCode;
        let n = self.scenario_rows();
        match k {
            KeyCode::Escape => self.screen = Screen::Menu,
            KeyCode::Up => self.scen.picked = self.scen.picked.saturating_sub(1),
            KeyCode::Down => self.scen.picked = (self.scen.picked + 1).min(n - 1),
            KeyCode::Enter | KeyCode::KpEnter | KeyCode::Space => self.scenario_go(g),
            _ => {}
        }
        if self.scen.picked < self.scen.scroll {
            self.scen.scroll = self.scen.picked;
        } else if self.scen.picked >= self.scen.scroll + ROWS {
            self.scen.scroll = self.scen.picked + 1 - ROWS;
        }
    }

    /// Starts what is picked: a free game goes on to Select Difficulty as in the original; a scenario starts (by way of
    /// Select Difficulty when it names no difficulty).
    fn scenario_go(&mut self, g: &mut Gfx) {
        let p = self.scen.picked;
        if p == 0 {
            self.sandbox_choice = false;
            self.title.champ_pending = false;
            self.screen = Screen::Difficulty;
            self.hover = -1;
            return;
        }
        let Some(e) = self.scen.list.get(p - 1) else { return };
        let Ok(def) = &e.def else {
            self.ui_sound(0x18);
            return;
        };
        if def.scenario.difficulty.is_none() && def.scenario.course.is_none() {
            self.scen.chosen = Some(p - 1);
            self.sandbox_choice = def.scenario.sandbox;
            self.title.champ_pending = false;
            self.screen = Screen::Difficulty;
            self.hover = -1;
            return;
        }
        self.start_scenario(g, p - 1);
    }

    /// Starts scenario `k` of the list (after Select Difficulty when it was needed: `self.difficulty` is set then).
    pub fn start_scenario(&mut self, g: &mut Gfx, k: usize) {
        self.scen.chosen = None;
        let Some(e) = self.scen.list.get(k) else { return };
        let Ok(def) = e.def.clone() else { return };
        let (id, name) = (e.id.clone(), e.name.clone());
        let d = &def.scenario;
        if let Some(course) = &d.course {
            // the saved game in the pack: its land, holes, buildings and date
            let save = self
                .content
                .found
                .packs
                .iter()
                .find(|p| p.manifest.id == id)
                .and_then(|p| p.source.read(course))
                .ok_or_else(|| trf("STR_ERROR_FILE_NOT_FOUND", &[("FILE", course)]))
                .and_then(|data| read_save_bytes(&data, course));
            let r = save.and_then(|s| self.load_save(g, s));
            if let Err(e) = r {
                eprintln!("scenario {id}: {e}");
                self.show_toast(&e);
                return;
            }
            if let Some(diff) = d.difficulty {
                self.difficulty = diff.level();
            }
            if d.sandbox {
                self.econ.sandbox = true;
            }
            if let Some(m) = d.start_money {
                self.econ.cash = m as f64;
                self.econ.version = self.econ.version.wrapping_add(1);
            }
            println!("scenario {id}: started from {course}");
            self.scen.starting = Some((id, def));
            self.begin_game_scripts();
            return;
        }
        if let Some(diff) = d.difficulty {
            self.difficulty = diff.level();
        }
        self.course_id = Some(self.exe_rng.below(0xffff));
        self.econ.sandbox = d.sandbox;
        self.deal_offer(d.sandbox);
        let prop = def.property_index();
        if !d.sandbox {
            self.cheap_slot(prop);
        }
        self.scen.start_money = d.start_money;
        println!("scenario {id}: {name}");
        self.scen.starting = Some((id, def.clone()));
        self.start_game(g, prop, d.sandbox);
    }

    /// The `--scenario ID` test hook: starts the scenario pack with that id (Moderate when it names no difficulty).
    pub fn start_scenario_by_id(&mut self, g: &mut Gfx, id: &str) -> Result<(), String> {
        self.scen.list = find_scenarios(&self.content);
        let k = self.scen.list.iter().position(|e| e.id == id).ok_or_else(|| format!("no scenario pack {id}"))?;
        if let Err(e) = &self.scen.list[k].def {
            return Err(format!("{id}: {e}"));
        }
        self.difficulty = 1;
        self.start_scenario(g, k);
        Ok(())
    }

    pub fn draw_scenarios(&mut self, g: &mut Gfx) {
        use crate::info_ui::c15;
        let s = Ui::new(self.draw_w, self.draw_h);
        self.view = s.view;
        crate::info_ui::dim(g, &s);
        self.draw_info_frame(g, &s, BOX.x, BOX.y, BOX.w, BOX.h);
        let (white, grey, yellow, navy, red) = (c15(0x7fff), c15(0x6318), c15(0x7ff0), c15(0x0018), rgb(0.85, 0.1, 0.1));
        s.put_shadowed(g, F_MANUAL20, 400.0, BOX.y + 14.0, tr("STR_SCENARIOS_TITLE"), yellow, true);
        s.put(g, F_ARIAL10, BOX.x + 24.0, BOX.y + 40.0, tr("STR_SCENARIOS_HELP"), navy);
        let (px, py) = self.info.pointer;
        let n = self.scenario_rows();
        for i in self.scen.scroll..n.min(self.scen.scroll + ROWS) {
            let y = TOP + (i - self.scen.scroll) as f32 * ROW_H;
            if self.scen.picked == i || Rect::new(BOX.x + 10.0, y, BOX.w - 20.0, ROW_H - 2.0).has(px, py) {
                s.fill(g, BOX.x + 12.0, y, BOX.w - 24.0, ROW_H - 2.0, rgb(0.66, 0.66, 0.84));
            }
            let (title, line, bad) = if i == 0 {
                (tr("STR_SCENARIOS_FREE_GAME").to_string(), tr("STR_SCENARIOS_FREE_GAME_TEXT").to_string(), false)
            } else {
                let e = &self.scen.list[i - 1];
                match &e.def {
                    Ok(sc) => (e.name.clone(), scenario_summary(sc), false),
                    Err(err) => (e.name.clone(), trf("STR_CONTENT_ERROR", &[("ERROR", err)]), true),
                }
            };
            s.put(g, F_MANUAL15, BOX.x + 24.0, y + 4.0, &title, if bad { grey } else { white });
            s.put(g, F_ARIAL10, BOX.x + 24.0, y + 24.0, &crate::content::fit(&line, BOX.w - 60.0), if bad { red } else { navy });
        }
        let below = TOP + ROWS as f32 * ROW_H;
        if n > ROWS {
            let more =
                [(24.0, tr("STR_CONTENT_PREVIOUS"), self.scen.scroll > 0), (110.0, tr("STR_CONTENT_NEXT"), self.scen.scroll + ROWS < n)];
            for (dx, label, ok) in more {
                s.fill(g, BOX.x + dx, below + 4.0, 80.0, 20.0, rgb(0.47, 0.47, 0.70));
                s.put_centered(g, F_ARIAL10, BOX.x + dx + 40.0, below + 8.0, label, if ok { navy } else { grey });
            }
        }
        // the picked scenario's description and goals
        let mut ty = below + 30.0;
        let text = if self.scen.picked == 0 {
            String::new()
        } else {
            let e = &self.scen.list[self.scen.picked - 1];
            match &e.def {
                Ok(sc) => {
                    let goals: Vec<String> = sc.goals.iter().map(|g| g.text.clone()).collect();
                    if goals.is_empty() {
                        e.description.clone()
                    } else {
                        format!("{}  {}", e.description, trf("STR_SCENARIOS_GOALS", &[("GOALS", &goals.join("; "))]))
                    }
                }
                Err(_) => e.description.clone(),
            }
        };
        for l in crate::ui::wrap_text(&text, 10.0, BOX.w - 120.0).into_iter().take(4) {
            s.put(g, F_ARIAL10, BOX.x + 24.0, ty, &l, navy);
            ty += 12.0;
        }
        let b = back_button();
        s.fill(g, b.x, b.y, b.w, b.h, if b.has(px, py) { rgb(0.85, 0.85, 1.0) } else { rgb(0.47, 0.47, 0.70) });
        s.put_centered(g, F_ARIAL10, b.x + b.w / 2.0, b.y + 5.0, tr("STR_SCENARIOS_BACK"), navy);
        if self.art.trans.tex.is_some() {
            s.image_part(g, &self.art.trans, BOX.x + BOX.w - 70.0, BOX.y + BOX.h - 58.0, 350.0, 140.0, 50.0, 50.0);
        }
        g.flush();
    }

    // ---- the scripts and the goals of a game -----------------------------------------------------------------------------

    /// A new game has started (`start_game`, or a scenario's saved course): the goals of its scenario (none for a free
    /// game), the scripts of the enabled packs and of the scenario, their `on_start`, and the scenario's intro.
    pub fn begin_game_scripts(&mut self) {
        let starting = self.scen.starting.take();
        self.scen.start_money = None;
        self.scen.goals = match &starting {
            Some((id, sc)) => {
                let name = sc.scenario.name.clone().unwrap_or_else(|| id.clone());
                sc.start_goals(id, &name)
            }
            None => Goals::default(),
        };
        self.scen.messages.clear();
        self.scen.pending.clear();
        self.load_scripts();
        if let Some((_, sc)) = &starting {
            if !sc.scenario.intro.is_empty() {
                self.scen.messages.push_back(sc.scenario.intro.clone());
            }
        }
        if !self.scen.host.is_empty() {
            self.scen.pending.push(Hook::Start);
            self.run_script_hooks();
        }
    }

    /// Compiles the scripts of the loaded packs and of the game's scenario pack, in load order (the scenario last).
    fn load_scripts(&mut self) {
        let mut ids = self.content.loaded.clone();
        if let Some(s) = self.scen.goals.scenario.clone() {
            if !ids.contains(&s) {
                ids.push(s);
            }
        }
        let seed = self.course_id.unwrap_or(0) as u64 ^ (self.seed as u64) << 32;
        let mut host = Host::new();
        for id in ids {
            let Some(p) = self.content.found.packs.iter().find(|p| p.manifest.id == id) else { continue };
            let files = script::pack_files(&p.source);
            if files.is_empty() {
                continue;
            }
            for e in host.add_pack(&id, &files, seed) {
                self.script_error(&id, e);
            }
        }
        if !host.is_empty() {
            println!("scripts: {}", host.pack_ids().join(", "));
        }
        self.scen.host = host;
    }

    fn script_error(&mut self, pack: &str, msg: String) {
        eprintln!("script {pack}: {msg}");
        self.show_toast(&trf("STR_SCRIPT_ERROR_TOAST", &[("PACK", &pack)]));
        self.scen.errors.push((pack.to_string(), msg));
        if self.scen.errors.len() > 100 {
            self.scen.errors.remove(0);
        }
    }

    /// The game as the scripts see it.
    fn game_view(&self) -> GameView {
        let holes = self
            .hole_numbers
            .iter()
            .filter_map(|&n| self.club.holes.get(n as usize).map(|h| HoleView { number: n, par: h.par, yards: h.length }))
            .collect();
        let golfers =
            (0..self.club.g.len()).filter(|&g| self.club.g[g].hole > 0).map(|g| self.golfer_view(g, self.club.g[g].strokes)).collect();
        GameView {
            year: 2001 + self.econ.year_index() as i32,
            month: 3 + self.econ.month_index().rem_euclid(8),
            cash: self.econ.cash as i64,
            fun: self.club.ratings.fun,
            skill: self.club.ratings.skill,
            members: self.club.member_count(),
            rank: economy::rank(self.hole_numbers.len()),
            difficulty: self.difficulty,
            sandbox: self.econ.sandbox,
            course: self.course_name.clone(),
            holes,
            golfers,
        }
    }

    fn golfer_view(&self, g: usize, strokes: i32) -> GolferView {
        let gf = &self.club.g[g];
        GolferView { name: self.club.name(g), hole: gf.hole, strokes, member: gf.level & 7 >= 2, pro: self.club.gary == g as i32 }
    }

    /// Queues a hook for the end of this game tick (nothing when no script is loaded).
    pub fn script_hook(&mut self, hook: Hook) {
        if !self.scen.host.is_empty() {
            self.scen.pending.push(hook);
        }
    }

    /// A golfer finished a hole (the club's HoleDone event).
    pub fn script_hole_done(&mut self, g: usize, hole: i32, strokes: i32) {
        if !self.scen.host.is_empty() {
            let golfer = self.golfer_view(g, strokes);
            self.scen.pending.push(Hook::HoleFinished { golfer: GolferView { hole, ..golfer }, hole, strokes });
        }
    }

    /// The calendar side of a game tick: the month and year hooks, the goal checks, and the queued messages.
    pub fn scenario_tick(&mut self, tick: u32) {
        let year_end = tick > 0 && tick & 0x1fff == 0;
        if tick > 0 && tick & 0x3ff == 0 {
            if year_end {
                self.script_hook(Hook::YearEnd);
            } else {
                self.script_hook(Hook::Month);
            }
            // the goals are checked at each month start, and their deadlines at each year end
            let year_ended = year_end.then_some(2000 + (tick >> 13) as i32);
            self.check_goals(year_ended);
        }
        self.run_script_hooks();
        // one message at a time, after the course's own welcome
        if tick > 0x40 && !self.ticker.busy() && self.card.is_none() {
            if let Some(m) = self.scen.messages.pop_front() {
                println!("[{:6.1}s] {m}", self.sim_time);
                self.post_message(&m, 1, crate::message_ui::NOBODY);
            }
        }
    }

    fn check_goals(&mut self, year_ended: Option<i32>) {
        if self.scen.goals.list.is_empty() {
            return;
        }
        let f = facts(self);
        let notes = self.scen.goals.check(&f, year_ended);
        self.goal_notes(notes);
    }

    fn goal_notes(&mut self, notes: Vec<Note>) {
        for n in notes {
            let t = n.text();
            println!("goals: {t}");
            match n {
                Note::Won(_) => self.ui_sound(56),
                Note::Lost(_) => self.ui_sound(0x18),
                _ => {}
            }
            self.scen.messages.push_back(t);
        }
    }

    /// Calls the queued hooks and carries out what the scripts asked for.
    pub fn run_script_hooks(&mut self) {
        if self.scen.pending.is_empty() {
            return;
        }
        let hooks = std::mem::take(&mut self.scen.pending);
        let view = self.game_view();
        for h in hooks {
            let out = self.scen.host.call(&h, &view);
            for (id, line) in out.log {
                println!("script {id}: {line}");
            }
            for (id, e) in out.errors {
                self.script_error(&id, e);
            }
            for (id, a) in out.actions {
                match a {
                    Action::Message(t) => self.scen.messages.push_back(t),
                    Action::Money(m, reason) => {
                        println!("script {id}: {} {} ({reason})", if m < 0 { "charges" } else { "pays" }, m.abs());
                        if m >= 0 {
                            self.econ.earn_to(economy::LEDGER_OTHER, m as f64);
                        } else {
                            self.econ.spend_to(economy::LEDGER_OTHER, -m as f64);
                        }
                        if m != 0 {
                            let at = self.club_anchor();
                            self.float_money((m / 100) as i32, (at.0 * 1024 + 1024, at.1 * 1024 + 1024));
                        }
                    }
                    Action::SetGoal { id: goal, text } => self.scen.goals.set(&id, &goal, &text),
                    Action::FinishGoal { id: goal, done } => {
                        let notes = self.scen.goals.finish(&id, &goal, done);
                        self.goal_notes(notes);
                    }
                }
            }
        }
    }

    /// What the saved game keeps of the scripts.
    pub fn scripts_for_save(&self) -> BTreeMap<String, PackState> {
        self.scen.host.save()
    }

    /// A saved game was loaded: its goals, and the scripts with their saved state.
    pub fn restore_game_scripts(&mut self, goals: Goals, saved: &BTreeMap<String, PackState>) {
        self.scen.goals = goals;
        self.scen.messages.clear();
        self.scen.pending.clear();
        self.load_scripts();
        for (id, e) in self.scen.host.restore(saved) {
            self.script_error(&id, e);
        }
    }

    // ---- the goals report ------------------------------------------------------------------------------------------------

    /// True when the game has goals to report (Information > Scenario goals).
    pub fn has_goals(&self) -> bool {
        !self.scen.goals.list.is_empty() || self.scen.goals.scenario.is_some()
    }

    pub fn open_goals(&mut self) {
        self.screen = Screen::Goals;
        self.hover = -1;
    }

    pub fn draw_goals(&mut self, g: &mut Gfx) {
        use crate::info_ui::c15;
        let s = Ui::new(self.draw_w, self.draw_h);
        self.view = s.view;
        crate::info_ui::dim(g, &s);
        self.draw_info_frame(g, &s, BOX.x, BOX.y, BOX.w, BOX.h);
        let (white, yellow, navy, green, red) = (c15(0x7fff), c15(0x7ff0), c15(0x0018), c15(0x0180), rgb(0.75, 0.1, 0.1));
        let gl = &self.scen.goals;
        let title =
            if gl.name.is_empty() { tr("STR_GOALS_TITLE").to_string() } else { trf("STR_GOALS_TITLE_SCENARIO", &[("NAME", &gl.name)]) };
        s.put_shadowed(g, F_MANUAL20, 400.0, BOX.y + 14.0, &title, yellow, true);
        let (state, colour) = match gl.outcome {
            Some(Outcome::Won) => (tr("STR_GOALS_WON"), green),
            Some(Outcome::Lost) => (tr("STR_GOALS_LOST"), red),
            None if gl.scenario.is_some() => (tr("STR_GOALS_IN_PROGRESS"), navy),
            None => (tr("STR_GOALS_SCRIPTS_ONLY"), navy),
        };
        s.put(g, F_ARIAL10, BOX.x + 24.0, BOX.y + 40.0, state, colour);
        let mut y = TOP;
        if gl.list.is_empty() {
            s.put(g, F_ARIAL10, BOX.x + 24.0, y, tr("STR_GOALS_NONE"), navy);
        }
        for goal in gl.list.iter().take(9) {
            let ticked = goal.state == GoalState::Done;
            if goal.state != GoalState::Open && self.art.info_buttons.tex.is_some() {
                let cut = if ticked { 350.0 } else { 400.0 };
                s.image_part(g, &self.art.info_buttons, BOX.x + 18.0, y + 2.0, cut, 0.0, 26.0, 26.0);
            }
            s.put(g, F_MANUAL15, BOX.x + 54.0, y + 2.0, &crate::content::fit(&goal.text, BOX.w - 100.0), white);
            let (word, c) = match goal.state {
                GoalState::Done => (tr("STR_GOAL_STATE_DONE").to_string(), green),
                GoalState::Failed => (tr("STR_GOAL_STATE_MISSED").to_string(), red),
                GoalState::Open => match goal.by_year {
                    Some(by) => (trf("STR_GOAL_STATE_BY", &[("YEAR", &by)]), navy),
                    None => (tr("STR_GOAL_STATE_OPEN").to_string(), navy),
                },
            };
            let line = if goal.when.is_empty() { word } else { format!("{word}  -  {}", goal.when.describe()) };
            s.put(g, F_ARIAL10, BOX.x + 54.0, y + 22.0, &crate::content::fit(&line, BOX.w - 100.0), c);
            y += 46.0;
        }
        s.put(g, F_ARIAL10, BOX.x + 24.0, BOX.y + BOX.h - 26.0, tr("STR_GOALS_CLOSE"), navy);
        g.flush();
    }

    // ---- System Functions > Export as scenario ---------------------------------------------------------------------------

    /// Writes the game as a scenario pack into the save folder's packs: pack.toml, scenario.toml (the property, the
    /// difficulty, the cash and the game's goals) and course.sgs (the whole game, which the scenario starts from).
    pub fn export_scenario(&mut self) {
        let name = if self.course_name.trim().is_empty() { tr("STR_EXPORT_UNNAMED").to_string() } else { self.course_name.clone() };
        let base = scenario::id_from_name(&name);
        let packs_dir = save_dir().join("packs");
        let taken = |id: &str| {
            sg_core::fsutil::is_dir(packs_dir.join(id))
                || sg_core::fsutil::read_file(packs_dir.join(format!("{id}.zip"))).is_some()
                || self.content.found.packs.iter().any(|p| p.manifest.id == id)
        };
        let id = std::iter::once(base.clone()).chain((2..1000).map(|k| format!("{base}-{k}"))).find(|id| !taken(id)).unwrap_or(base);
        let dir = packs_dir.join(&id);
        let g = &self.scen.goals;
        let d = ScenarioDef {
            name: Some(name.clone()),
            description: trf("STR_EXPORT_DESCRIPTION", &[("COURSE", &name), ("PRO", &self.pro_name())]),
            property: self.land.as_ref().map(|l| sg_core::data::property_id(l.slot.property)),
            theme: None,
            start_money: Some(self.econ.cash as i64 / 100 * 100),
            difficulty: Some(scenario::Difficulty::from_level(self.difficulty)),
            sandbox: self.econ.sandbox,
            course: Some("course.sgs".into()),
            intro: String::new(),
            win_text: g.win_text.clone(),
            lose_text: g.lose_text.clone(),
        };
        // the game's goals that the engine checks (a script's goals belong to its script)
        let goals = g.list.iter().filter(|x| !x.when.is_empty()).map(goal_def).collect();
        let sc = Scenario { scenario: d, goals };
        let header = format!("# {}\n# docs/MODDING.md, section 5, explains every field; add [[goal]] tables for the goals.\n\n", name);
        let ok = sg_core::fsutil::write_file(dir.join("pack.toml"), scenario::export_manifest(&id, &name, &self.pro_name()).as_bytes())
            && sg_core::fsutil::write_file(dir.join("scenario.toml"), format!("{header}{}", sc.to_toml()).as_bytes());
        let r =
            if ok { self.save_game(&dir.join("course.sgs")) } else { Err(trf("STR_ERROR_COULD_NOT_WRITE", &[("FILE", &dir.display())])) };
        match r {
            Ok(()) => {
                println!("exported scenario {}", dir.display());
                self.show_toast(&trf("STR_TOAST_EXPORTED", &[("PACK", &id)]));
                self.content.rediscover();
            }
            Err(e) => {
                eprintln!("could not export the scenario: {e}");
                self.show_toast(&e);
            }
        }
    }
}

fn back_button() -> Rect {
    Rect::new(BOX.x + BOX.w - 190.0, BOX.y + BOX.h - 46.0, 100.0, 22.0)
}

/// A goal of the game as scenario.toml writes it.
fn goal_def(g: &Goal) -> GoalDef {
    let w = &g.when;
    GoalDef {
        id: Some(g.id.clone()),
        text: g.text.clone(),
        holes: w.holes,
        cash: w.cash,
        fun: w.fun,
        skill: w.skill,
        members: w.members,
        rank: w.rank.clone(),
        by_year: g.by_year,
    }
}

/// One line about a scenario: where, how hard, the money, the number of goals.
fn scenario_summary(sc: &Scenario) -> String {
    let d = &sc.scenario;
    let place = sg_core::data::properties().get(sc.property_index()).map(|p| p.name().to_string()).unwrap_or_default();
    let diff = d
        .difficulty
        .map(|x| tr(crate::render::DIFFICULTY_KEYS[x.level() as usize]).to_string())
        .unwrap_or_else(|| tr("STR_SCENARIOS_ANY_DIFFICULTY").into());
    let money = match d.start_money {
        Some(m) => trf("STR_SCENARIOS_MONEY", &[("MONEY", &economy::money_digits(m))]),
        None => String::new(),
    };
    trf("STR_SCENARIOS_SUMMARY", &[("PLACE", &place), ("DIFFICULTY", &diff), ("MONEY", &money), ("GOALS", &sc.goals.len())])
}
