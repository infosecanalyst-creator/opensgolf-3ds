//! The course editor (the port's own; docs/PLAYING.md "Course editor", docs/MODDING.md 5): a separate mode, reached from the
//! title menu's "Course editor..." button, for making scenario packs. It starts like a new game (Select Difficulty with the
//! course size, then the property chooser), and then:
//!
//! * the course is built with the game's own tools, free of charge (sg_core::economy's `free`), and no time passes: the
//!   game's clock does not run, so no golfers come and the calendar stands still. Undo (undo.rs) works as in a game;
//! * the editor box (the bar at the top of the course, or Shift+E) holds the scenario's settings: name, author, texts,
//!   starting cash, difficulty, which tracts of land are owned at the start and which are for sale, and goals of the kinds
//!   sg_core::scenario checks;
//! * "Import heightmap..." lays a grey picture over the land (sg_core::heightmap); three samples are built in;
//! * "Save scenario" writes a scenario pack into the save folder's packs (pack.toml, scenario.toml, course.sgs), which then
//!   shows on Start New Game and starts with `--scenario ID`; "Load scenario..." takes a scenario pack back into the editor.
//!
//! Nothing here runs outside the editor: a normal game never has `App::editor` set.
use crate::app::*;
use crate::gfx::Gfx;
use crate::render::Rect;
use crate::ui::{rgb, rgba, Screen as Ui, F_ARIAL10, F_MANUAL15, F_MANUAL20};
use miniquad::KeyCode;
use sg_core::economy;
use sg_core::heightmap::{self, Grey};
use sg_core::land;
use sg_core::lang::{tr, trf};
use sg_core::scenario::{self, Conditions, Difficulty, GoalDef, Scenario, ScenarioDef, RANK_IDS};

/// The box, as the port's other boxes (scenario_ui.rs).
const BOX: Rect = Rect { x: 40.0, y: 36.0, w: 720.0, h: 528.0 };

/// The sample heightmaps (tools/heightmaps, made by its make_heightmaps.py), built in so every version has them.
pub const SAMPLES: [(&str, &[u8]); 3] = [
    ("STR_EDITOR_HM_SAMPLE_VALLEY", include_bytes!("../../../tools/heightmaps/valley.png")),
    ("STR_EDITOR_HM_SAMPLE_RIDGE", include_bytes!("../../../tools/heightmaps/ridge.png")),
    ("STR_EDITOR_HM_SAMPLE_ISLAND", include_bytes!("../../../tools/heightmaps/island.png")),
];

/// The heights a heightmap may take (white's level; black is sea level): gentle, rolling, hilly.
pub const RELIEF_TOPS: [i32; 3] = [4, 7, heightmap::TOP];
const RELIEF_KEYS: [&str; 3] = ["STR_EDITOR_RELIEF_GENTLE", "STR_EDITOR_RELIEF_ROLLING", "STR_EDITOR_RELIEF_HILLY"];

/// The goal kinds a goal row offers (the conditions sg_core::scenario checks).
const GOAL_KINDS: [&str; 6] = [
    "STR_EDITOR_GOAL_HOLES",
    "STR_EDITOR_GOAL_CASH",
    "STR_EDITOR_GOAL_FUN",
    "STR_EDITOR_GOAL_SKILL",
    "STR_EDITOR_GOAL_MEMBERS",
    "STR_EDITOR_GOAL_RANK",
];
const MAX_GOALS: usize = 5;
/// The latest deadline a goal row offers.
const LAST_YEAR: i32 = 2030;

/// The text fields of the box.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Field {
    Name,
    Author,
    Description,
    Intro,
    Win,
    Lose,
}

const FIELDS: [(Field, &str); 6] = [
    (Field::Name, "STR_EDITOR_NAME"),
    (Field::Author, "STR_EDITOR_AUTHOR"),
    (Field::Description, "STR_EDITOR_DESCRIPTION"),
    (Field::Intro, "STR_EDITOR_INTRO"),
    (Field::Win, "STR_EDITOR_WIN"),
    (Field::Lose, "STR_EDITOR_LOSE"),
];

/// What a click in the box hits.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
enum Hit {
    Field(Field),
    CashLess,
    CashMore,
    Difficulty,
    Sandbox,
    Tract(usize),
    Relief,
    WaterLess,
    WaterMore,
    Smooth,
    Pick,
    Sample(usize),
    GoalKind(usize),
    GoalLess(usize),
    GoalMore(usize),
    YearLess(usize),
    YearMore(usize),
    GoalRemove(usize),
    GoalAdd,
    Save,
    Load,
    Back,
    Leave,
    LoadRow(usize),
    LoadPrev,
    LoadNext,
    LoadBack,
}

/// The editor's state (`App::editor`): the scenario being made and the box's own state.
#[derive(Default)]
pub struct Editor {
    pub name: String,
    pub author: String,
    pub description: String,
    pub intro: String,
    pub win: String,
    pub lose: String,
    /// Cash at the start, in money.
    pub start_money: i64,
    /// None: the player picks the difficulty when the scenario starts.
    pub difficulty: Option<Difficulty>,
    pub sandbox: bool,
    pub goals: Vec<GoalDef>,
    /// The pack's version (kept when a pack is loaded and saved again).
    pub version: String,
    /// The pack in the save folder this scenario is saved to (None until the first save, or for a pack loaded from
    /// elsewhere: it is saved as a copy).
    pub pack_id: Option<String>,
    /// The heightmap import's settings: the relief (RELIEF_TOPS), the water level (0 none), smoothing.
    pub relief: usize,
    pub water: i32,
    pub smooth: bool,
    /// The last picture imported, (its name, the picture): changing the settings above lays it again.
    pub picture: Option<(String, Grey)>,
    /// The text field typed into.
    pub focus: Option<Field>,
    /// What the box last did (saved, imported, a problem).
    pub status: String,
    /// "Leave the editor" was clicked once: a second click leaves.
    pub leaving: bool,
    /// The Load scenario page: the scenario packs found and the first shown.
    pub loading: Option<Vec<crate::scenario_ui::ScenarioEntry>>,
    pub load_scroll: usize,
    /// A browser's file picker is open (polled each frame).
    pub picking: bool,
}

impl Editor {
    fn field(&mut self, f: Field) -> &mut String {
        match f {
            Field::Name => &mut self.name,
            Field::Author => &mut self.author,
            Field::Description => &mut self.description,
            Field::Intro => &mut self.intro,
            Field::Win => &mut self.win,
            Field::Lose => &mut self.lose,
        }
    }

    fn field_text(&self, f: Field) -> &str {
        match f {
            Field::Name => &self.name,
            Field::Author => &self.author,
            Field::Description => &self.description,
            Field::Intro => &self.intro,
            Field::Win => &self.win,
            Field::Lose => &self.lose,
        }
    }

    /// The heightmap import's options.
    pub fn options(&self) -> heightmap::Options {
        heightmap::Options {
            low: 0,
            high: RELIEF_TOPS[self.relief.min(2)],
            water: (self.water > 0).then_some(self.water - 1),
            smooth: self.smooth,
        }
    }

    /// The scenario as scenario.toml writes it (the property is the game's, the course the pack's course.sgs).
    pub fn scenario(&self, property: Option<String>) -> Scenario {
        let opt = |s: &str| s.trim().to_string();
        let goals = self
            .goals
            .iter()
            .enumerate()
            .map(|(k, g)| GoalDef {
                id: Some(g.id.clone().unwrap_or_else(|| format!("goal-{}", k + 1))),
                text: if g.text.trim().is_empty() { g.auto_text() } else { g.text.clone() },
                ..g.clone()
            })
            .collect();
        Scenario {
            scenario: ScenarioDef {
                name: Some(self.display_name()),
                description: opt(&self.description),
                property,
                theme: None,
                start_money: Some(self.start_money),
                difficulty: self.difficulty,
                sandbox: self.sandbox,
                course: Some("course.sgs".into()),
                intro: opt(&self.intro),
                win_text: opt(&self.win),
                lose_text: opt(&self.lose),
            },
            goals,
        }
    }

    /// The name, or "My Scenario" while none is typed.
    pub fn display_name(&self) -> String {
        if self.name.trim().is_empty() {
            tr("STR_EDITOR_UNNAMED").to_string()
        } else {
            self.name.trim().to_string()
        }
    }

    /// Takes a scenario's settings (Load scenario).
    fn take(&mut self, sc: &Scenario) {
        let d = &sc.scenario;
        self.name = d.name.clone().unwrap_or_default();
        self.description = d.description.clone();
        self.intro = d.intro.clone();
        self.win = d.win_text.clone();
        self.lose = d.lose_text.clone();
        self.difficulty = d.difficulty;
        self.sandbox = d.sandbox;
        if let Some(m) = d.start_money {
            self.start_money = m;
        }
        self.goals = sc.goals.clone();
    }
}

/// A goal row's kind: the first condition the goal has.
fn goal_kind(g: &GoalDef) -> usize {
    let c = g.when();
    [c.holes.is_some(), c.cash.is_some(), c.fun.is_some(), c.skill.is_some(), c.members.is_some(), c.rank.is_some()]
        .iter()
        .position(|&b| b)
        .unwrap_or(0)
}

/// A goal of one kind with its first value.
fn goal_of_kind(kind: usize, by_year: Option<i32>) -> GoalDef {
    let mut g = GoalDef { by_year, ..Default::default() };
    g.set_when(&match kind {
        0 => Conditions { holes: Some(9), ..Default::default() },
        1 => Conditions { cash: Some(100_000), ..Default::default() },
        2 => Conditions { fun: Some(30), ..Default::default() },
        3 => Conditions { skill: Some(10.0), ..Default::default() },
        4 => Conditions { members: Some(20), ..Default::default() },
        _ => Conditions { rank: Some(RANK_IDS[1].into()), ..Default::default() },
    });
    g.text = g.auto_text();
    g
}

/// Steps a goal's value of its kind one step up or down, within its range.
fn step_goal(g: &mut GoalDef, up: bool) {
    let s = if up { 1 } else { -1 };
    match goal_kind(g) {
        0 => g.holes = g.holes.map(|v| (v + s).clamp(1, 18)),
        1 => g.cash = g.cash.map(|v| (v + s as i64 * 10_000).clamp(10_000, 10_000_000)),
        2 => g.fun = g.fun.map(|v| (v + s * 5).clamp(5, 200)),
        3 => g.skill = g.skill.map(|v| (v + s as f64 * 0.5).clamp(0.5, 50.0)),
        4 => g.members = g.members.map(|v| (v + s * 5).clamp(5, 500)),
        _ => {
            let k = g.rank.as_deref().and_then(|r| RANK_IDS.iter().position(|&x| x == r)).unwrap_or(0) as i32;
            g.rank = Some(RANK_IDS[(k + s).clamp(0, 3) as usize].into());
        }
    }
    g.text = g.auto_text();
}

/// A goal's value as the row shows it.
fn goal_value(g: &GoalDef) -> String {
    let c = g.when();
    match goal_kind(g) {
        0 => c.holes.unwrap_or(0).to_string(),
        1 => crate::ui::money(c.cash.unwrap_or(0)),
        2 => c.fun.unwrap_or(0).to_string(),
        3 => format!("{:.1}", c.skill.unwrap_or(0.0)),
        4 => c.members.unwrap_or(0).to_string(),
        _ => {
            let k = c.rank.as_deref().and_then(|r| RANK_IDS.iter().position(|&x| x == r)).unwrap_or(0);
            economy::rank_name(k).to_string()
        }
    }
}

/// A tract's cell in the land grid: in the columns and rows of the TRACTS FOR SALE screen (tract 1 at the top left, 2
/// under it, 4 at the top of the second column).
fn tract_rect(i: usize) -> Rect {
    Rect::new(492.0 + (i / 3) as f32 * 82.0, 116.0 + (i % 3) as f32 * 24.0, 76.0, 20.0)
}

/// The box's buttons and fields with where they are: the same list draws them and takes the clicks.
fn layout(e: &Editor) -> Vec<(Rect, Hit)> {
    let mut v = Vec::new();
    if e.loading.is_some() {
        for k in 0..LOAD_ROWS {
            v.push((Rect::new(BOX.x + 12.0, LOAD_TOP + k as f32 * LOAD_ROW_H, BOX.w - 24.0, LOAD_ROW_H - 2.0), Hit::LoadRow(k)));
        }
        let below = LOAD_TOP + LOAD_ROWS as f32 * LOAD_ROW_H + 4.0;
        v.push((Rect::new(BOX.x + 24.0, below, 80.0, 20.0), Hit::LoadPrev));
        v.push((Rect::new(BOX.x + 110.0, below, 80.0, 20.0), Hit::LoadNext));
        v.push((Rect::new(BOX.x + BOX.w - 190.0, BOX.y + BOX.h - 46.0, 150.0, 22.0), Hit::LoadBack));
        return v;
    }
    for (k, &(f, _)) in FIELDS.iter().enumerate() {
        v.push((Rect::new(150.0, 96.0 + 21.0 * k as f32, 320.0, 18.0), Hit::Field(f)));
    }
    v.push((Rect::new(150.0, 225.0, 20.0, 18.0), Hit::CashLess));
    v.push((Rect::new(270.0, 225.0, 20.0, 18.0), Hit::CashMore));
    v.push((Rect::new(150.0, 246.0, 140.0, 18.0), Hit::Difficulty));
    v.push((Rect::new(150.0, 267.0, 140.0, 18.0), Hit::Sandbox));
    for i in 0..9 {
        v.push((tract_rect(i), Hit::Tract(i)));
    }
    v.push((Rect::new(580.0, 214.0, 150.0, 18.0), Hit::Relief));
    v.push((Rect::new(580.0, 235.0, 20.0, 18.0), Hit::WaterLess));
    v.push((Rect::new(710.0, 235.0, 20.0, 18.0), Hit::WaterMore));
    v.push((Rect::new(580.0, 256.0, 150.0, 18.0), Hit::Smooth));
    if pick_available() {
        v.push((Rect::new(492.0, 280.0, 238.0, 20.0), Hit::Pick));
    }
    for k in 0..SAMPLES.len() {
        v.push((Rect::new(560.0 + 58.0 * k as f32, 304.0, 54.0, 18.0), Hit::Sample(k)));
    }
    for k in 0..e.goals.len().min(MAX_GOALS) {
        let y = GOAL_TOP + 22.0 * k as f32;
        v.push((Rect::new(60.0, y, 100.0, 18.0), Hit::GoalKind(k)));
        v.push((Rect::new(166.0, y, 18.0, 18.0), Hit::GoalLess(k)));
        v.push((Rect::new(264.0, y, 18.0, 18.0), Hit::GoalMore(k)));
        v.push((Rect::new(292.0, y, 18.0, 18.0), Hit::YearLess(k)));
        v.push((Rect::new(390.0, y, 18.0, 18.0), Hit::YearMore(k)));
        v.push((Rect::new(718.0, y, 22.0, 18.0), Hit::GoalRemove(k)));
    }
    if e.goals.len() < MAX_GOALS {
        v.push((Rect::new(60.0, GOAL_TOP + 22.0 * e.goals.len() as f32, 100.0, 18.0), Hit::GoalAdd));
    }
    for (k, h) in [Hit::Save, Hit::Load, Hit::Back, Hit::Leave].into_iter().enumerate() {
        v.push((Rect::new(60.0 + 170.0 * k as f32, 508.0, 160.0, 24.0), h));
    }
    v
}

const GOAL_TOP: f32 = 358.0;
const LOAD_TOP: f32 = 96.0;
const LOAD_ROW_H: f32 = 44.0;
const LOAD_ROWS: usize = 8;

/// Whether this version can pick a file (the desktop's own picker, or the browser's): Android has the samples only.
fn pick_available() -> bool {
    cfg!(target_arch = "wasm32") || cfg!(all(feature = "picker", not(target_os = "android")))
}

/// The hover value of the title's "Course editor..." button.
pub const EDITOR_HOVER: i32 = 9;

/// The title button: bottom right, left of the exit button, where the original's art and the free set's leave the
/// background bare (the files and update buttons have the top corners).
pub fn title_button_rect() -> Rect {
    let w = F_MANUAL15.width(tr("STR_EDITOR_TITLE_BUTTON")).max(120.0) + 24.0;
    Rect::new(716.0 - w, 566.0, w, 24.0)
}

impl App {
    // ---- starting and leaving ---------------------------------------------------------------------------------------------

    /// Draws the title button, in the files button's style (gamefiles.rs).
    pub fn draw_editor_button(&self, g: &mut Gfx, s: &Ui) {
        let r = title_button_rect();
        let lit = self.hover == EDITOR_HOVER;
        if crate::glass_ui::active() {
            crate::glass_ui::title_button(g, s, r.x, r.y, r.w, r.h, tr("STR_EDITOR_TITLE_BUTTON"), lit);
            return;
        }
        s.fill(g, r.x - 1.0, r.y - 1.0, r.w + 2.0, r.h + 2.0, rgba(1.0, 1.0, 1.0, 0.85));
        s.fill(g, r.x, r.y, r.w, r.h, if lit { rgba(0.13, 0.32, 0.55, 0.95) } else { rgba(0.08, 0.16, 0.32, 0.88) });
        s.put_centered(
            g,
            F_MANUAL15,
            r.x + r.w / 2.0,
            r.y + 4.0,
            tr("STR_EDITOR_TITLE_BUTTON"),
            rgb(1.0, 1.0, if lit { 1.0 } else { 0.8 }),
        );
    }

    /// The title's "Course editor..." button: Select Difficulty (with the course size) and the property chooser, as a new
    /// game, then the editor on that property.
    pub fn open_editor_start(&mut self) {
        self.editor_start = true;
        self.sandbox_choice = true;
        self.title.champ_pending = false;
        self.scen.chosen = None;
        self.screen = Screen::Difficulty;
        self.hover = -1;
    }

    /// The property chooser picked property `p` for the editor: the land a new game would have there (in its cheapest
    /// offer slot, as a scenario starts), then the editor.
    pub fn start_editor(&mut self, g: &mut Gfx, p: usize) {
        self.editor_start = false;
        self.sandbox_choice = false;
        self.deal_offer(false);
        self.cheap_slot(p);
        self.start_game(g, p, false);
        let cash = self.econ.cash as i64 / 100 * 100;
        let ed = Editor {
            start_money: cash,
            difficulty: Some(Difficulty::from_level(self.difficulty)),
            version: "1.0.0".into(),
            relief: 1,
            smooth: true,
            ..Default::default()
        };
        self.enter_editor(ed);
        println!("editor: {} at size {}", sg_core::data::properties()[p].name(), sg_core::mapsize::n());
    }

    /// The game in hand becomes the editor's: free building, no clock, no scripts, every building kind and landmark.
    fn enter_editor(&mut self, ed: Editor) {
        self.editor = Some(Box::new(ed));
        self.econ.free = true;
        self.scen.host = Default::default();
        self.scen.goals = Default::default();
        self.club.landmarks_owned = 0x3fff;
        self.paused = false;
        self.speed = 1;
        self.undo.clear();
        self.screen = Screen::Play;
        self.hover = -1;
    }

    /// Leaves the editor for the title menu (nothing is saved).
    pub fn leave_editor(&mut self) {
        if self.editor.take().is_some() {
            println!("editor: left");
        }
        self.econ.free = false;
        self.editor_start = false;
        self.undo.clear();
        self.screen = Screen::Menu;
        self.hover = -1;
    }

    /// The editor box (the bar's click, Shift+E).
    pub fn open_editor_box(&mut self) {
        if let Some(e) = self.editor.as_mut() {
            e.leaving = false;
            e.loading = None;
            self.screen = Screen::Editor;
            self.hover = -1;
        }
    }

    /// The box's Load scenario page.
    pub fn editor_load_page(&mut self) {
        let list = crate::scenario_ui::find_scenarios(&self.content);
        if let Some(e) = self.editor.as_mut() {
            e.loading = Some(list);
            e.load_scroll = 0;
            e.status.clear();
        }
    }

    fn close_editor_box(&mut self) {
        if let Some(e) = self.editor.as_mut() {
            e.focus = None;
            e.loading = None;
        }
        self.screen = Screen::Play;
    }

    // ---- the bar over the course -----------------------------------------------------------------------------------------

    fn editor_bar_rect(&self) -> Rect {
        let label = self.editor_bar_label();
        let w = F_MANUAL15.width(&label) + 28.0;
        Rect::new(400.0 - w / 2.0, 6.0, w, 24.0)
    }

    fn editor_bar_label(&self) -> String {
        let key = crate::controls::settings().binding(crate::controls::action("editor")).map(|b| b.label()).unwrap_or_default();
        trf("STR_EDITOR_BAR", &[("KEY", &key)])
    }

    /// The bar at the top of the course: "Course editor: scenario settings", in the title buttons' style.
    pub fn draw_editor_bar(&self, g: &mut Gfx) {
        if self.editor.is_none() || self.screen != Screen::Play {
            return;
        }
        let s = Ui::new(self.draw_w, self.draw_h);
        let r = self.editor_bar_rect();
        let (px, py) = self.info.pointer;
        let lit = r.has(px, py);
        s.fill(g, r.x - 1.0, r.y - 1.0, r.w + 2.0, r.h + 2.0, rgba(1.0, 1.0, 1.0, 0.85));
        s.fill(g, r.x, r.y, r.w, r.h, if lit { rgba(0.13, 0.32, 0.55, 0.95) } else { rgba(0.08, 0.16, 0.32, 0.88) });
        s.put_centered(g, F_MANUAL15, r.x + r.w / 2.0, r.y + 4.0, &self.editor_bar_label(), rgb(1.0, 1.0, if lit { 1.0 } else { 0.8 }));
        g.flush();
    }

    /// A click on the bar opens the box.
    pub fn editor_bar_click(&mut self, x: f32, y: f32) -> bool {
        if self.editor.is_none() || self.screen != Screen::Play {
            return false;
        }
        if !self.editor_bar_rect().has(x, y) {
            return false;
        }
        self.ui_sound(0x18);
        self.open_editor_box();
        true
    }

    // ---- the box ----------------------------------------------------------------------------------------------------------

    pub fn editor_typing(&self) -> bool {
        self.screen == Screen::Editor && self.editor.as_ref().is_some_and(|e| e.focus.is_some())
    }

    pub fn editor_char(&mut self, c: char) {
        if self.screen != Screen::Editor {
            return;
        }
        let Some(e) = self.editor.as_mut() else { return };
        let Some(f) = e.focus else { return };
        let most = if f == Field::Name || f == Field::Author { 48 } else { 300 };
        let t = e.field(f);
        if !c.is_control() && t.chars().count() < most {
            t.push(c);
        }
    }

    /// Keys in the box: typing into a field (Backspace, Enter and Tab go on to the next field, Esc stops typing), Esc closes.
    pub fn editor_key(&mut self, k: KeyCode) {
        let Some(e) = self.editor.as_mut() else { return };
        if let Some(f) = e.focus {
            match k {
                KeyCode::Backspace => {
                    e.field(f).pop();
                }
                KeyCode::Enter | KeyCode::KpEnter | KeyCode::Tab => {
                    let i = FIELDS.iter().position(|x| x.0 == f).unwrap_or(0);
                    e.focus = FIELDS.get(i + 1).map(|x| x.0);
                }
                KeyCode::Escape => e.focus = None,
                _ => {}
            }
            return;
        }
        if e.loading.is_some() {
            if k == KeyCode::Escape {
                e.loading = None;
            }
            return;
        }
        match k {
            KeyCode::Escape | KeyCode::Enter | KeyCode::KpEnter => self.close_editor_box(),
            KeyCode::Tab => e.focus = Some(Field::Name),
            _ => {}
        }
    }

    /// A click in the box.
    pub fn editor_click(&mut self, g: &mut Gfx, x: f32, y: f32) {
        let Some(e) = self.editor.as_mut() else { return };
        let hit = layout(e).into_iter().find(|(r, _)| r.has(x, y)).map(|(_, h)| h);
        e.focus = None;
        if hit != Some(Hit::Leave) {
            e.leaving = false;
        }
        let Some(hit) = hit else { return };
        match hit {
            Hit::Field(f) => e.focus = Some(f),
            Hit::CashLess | Hit::CashMore => {
                let step = if e.start_money >= 1_000_000 { 100_000 } else { 10_000 };
                let up = hit == Hit::CashMore;
                e.start_money = (e.start_money + if up { step } else { -step }) / step * step;
                e.start_money = e.start_money.clamp(0, 10_000_000);
                self.econ.cash = e.start_money as f64;
                self.econ.version = self.econ.version.wrapping_add(1);
            }
            Hit::Difficulty => {
                e.difficulty = match e.difficulty {
                    None => Some(Difficulty::Easy),
                    Some(Difficulty::Impossible) => None,
                    Some(d) => Some(Difficulty::from_level(d.level() + 1)),
                };
            }
            Hit::Sandbox => e.sandbox = !e.sandbox,
            Hit::Tract(i) => self.toggle_tract(i),
            Hit::Relief | Hit::WaterLess | Hit::WaterMore | Hit::Smooth => {
                match hit {
                    Hit::Relief => e.relief = (e.relief + 1) % RELIEF_TOPS.len(),
                    Hit::WaterLess => e.water = (e.water - 1).max(0),
                    Hit::WaterMore => e.water = (e.water + 1).min(RELIEF_TOPS[e.relief]),
                    _ => e.smooth = !e.smooth,
                }
                e.water = e.water.min(RELIEF_TOPS[e.relief]);
                // the picture in hand is laid again with the new settings
                if let Some((name, pic)) = e.picture.clone() {
                    self.lay_heightmap(&name, &pic);
                }
            }
            Hit::Pick => self.pick_heightmap(),
            Hit::Sample(k) => {
                let name = tr(SAMPLES[k].0).to_string();
                if let Err(err) = self.import_heightmap(&name, SAMPLES[k].1) {
                    self.editor_status(&err);
                }
            }
            Hit::GoalKind(k) => {
                if let Some(goal) = e.goals.get_mut(k) {
                    *goal = GoalDef { id: goal.id.clone(), ..goal_of_kind((goal_kind(goal) + 1) % GOAL_KINDS.len(), goal.by_year) };
                }
            }
            Hit::GoalLess(k) | Hit::GoalMore(k) => {
                if let Some(goal) = e.goals.get_mut(k) {
                    step_goal(goal, matches!(hit, Hit::GoalMore(_)));
                }
            }
            Hit::YearLess(k) | Hit::YearMore(k) => {
                if let Some(goal) = e.goals.get_mut(k) {
                    goal.by_year = match (goal.by_year, matches!(hit, Hit::YearMore(_))) {
                        (None, true) => Some(2001),
                        (None, false) => None,
                        (Some(y), true) => Some((y + 1).min(LAST_YEAR)),
                        (Some(y), false) => (y > 2001).then_some(y - 1),
                    };
                    goal.text = goal.auto_text();
                }
            }
            Hit::GoalRemove(k) => {
                if k < e.goals.len() {
                    e.goals.remove(k);
                }
            }
            Hit::GoalAdd => {
                let kind = e.goals.len().min(GOAL_KINDS.len() - 1);
                e.goals.push(goal_of_kind(kind, None));
            }
            Hit::Save => match self.save_editor_scenario() {
                Ok(_) => self.ui_sound(56),
                Err(err) => {
                    self.ui_sound(0x18);
                    self.editor_status(&trf("STR_EDITOR_SAVE_FAILED", &[("ERROR", &err)]));
                }
            },
            Hit::Load => self.editor_load_page(),
            Hit::Back => self.close_editor_box(),
            Hit::Leave => {
                if e.leaving {
                    self.leave_editor();
                } else {
                    e.leaving = true;
                    e.status = tr("STR_EDITOR_LEAVE_SURE").to_string();
                }
            }
            Hit::LoadRow(k) => {
                let i = e.load_scroll + k;
                if e.loading.as_ref().is_some_and(|l| i < l.len()) {
                    if let Err(err) = self.load_editor_scenario(g, i) {
                        self.ui_sound(0x18);
                        self.editor_status(&err);
                    }
                }
            }
            Hit::LoadPrev => e.load_scroll = e.load_scroll.saturating_sub(LOAD_ROWS),
            Hit::LoadNext => {
                let n = e.loading.as_ref().map_or(0, |l| l.len());
                if e.load_scroll + LOAD_ROWS < n {
                    e.load_scroll += LOAD_ROWS;
                }
            }
            Hit::LoadBack => e.loading = None,
        }
    }

    fn editor_status(&mut self, msg: &str) {
        println!("editor: {msg}");
        if let Some(e) = self.editor.as_mut() {
            e.status = msg.to_string();
        }
    }

    pub fn draw_editor_box(&mut self, g: &mut Gfx) {
        use crate::info_ui::c15;
        let s = Ui::new(self.draw_w, self.draw_h);
        self.view = s.view;
        crate::info_ui::dim(g, &s);
        self.draw_info_frame(g, &s, BOX.x, BOX.y, BOX.w, BOX.h);
        let Some(e) = self.editor.as_ref() else { return };
        let (white, grey, yellow, navy) = (c15(0x7fff), c15(0x6318), c15(0x7ff0), c15(0x0018));
        let (px, py) = self.info.pointer;
        let button = |g: &mut Gfx, r: Rect, label: &str, on: bool| {
            let lit = r.has(px, py);
            crate::glass_ui::button_fill(g, &s, r.x, r.y, r.w, r.h, lit);
            s.put_centered(g, F_ARIAL10, r.x + r.w / 2.0, r.y + (r.h - 10.0) / 2.0, label, if on { navy } else { grey });
        };
        let label = |g: &mut Gfx, x: f32, y: f32, t: &str| s.put(g, F_ARIAL10, x, y + 4.0, t, navy);
        let value = |g: &mut Gfx, x: f32, y: f32, w: f32, t: &str| s.put_centered(g, F_ARIAL10, x + w / 2.0, y + 4.0, t, white);
        if let Some(list) = &e.loading {
            s.put_shadowed(g, F_MANUAL20, 400.0, BOX.y + 14.0, tr("STR_EDITOR_LOAD_TITLE"), yellow, true);
            s.put(g, F_ARIAL10, BOX.x + 24.0, BOX.y + 40.0, tr("STR_EDITOR_LOAD_HELP"), navy);
            if list.is_empty() {
                s.put(g, F_ARIAL10, BOX.x + 24.0, LOAD_TOP + 8.0, tr("STR_EDITOR_LOAD_NONE"), navy);
            }
            for (r, h) in layout(e) {
                match h {
                    Hit::LoadRow(k) => {
                        let Some(sc) = list.get(e.load_scroll + k) else { continue };
                        if r.has(px, py) {
                            s.fill(g, r.x, r.y, r.w, r.h, rgb(0.66, 0.66, 0.84));
                        }
                        s.put(g, F_MANUAL15, r.x + 12.0, r.y + 4.0, &sc.name, if sc.def.is_ok() { white } else { grey });
                        let line = match &sc.def {
                            Ok(_) => format!("{}  ({})", sc.description, sc.id),
                            Err(err) => trf("STR_CONTENT_ERROR", &[("ERROR", err)]),
                        };
                        s.put(g, F_ARIAL10, r.x + 12.0, r.y + 24.0, &crate::content::fit(&line, r.w - 30.0), navy);
                    }
                    Hit::LoadPrev if list.len() > LOAD_ROWS => button(g, r, tr("STR_CONTENT_PREVIOUS"), e.load_scroll > 0),
                    Hit::LoadNext if list.len() > LOAD_ROWS => button(g, r, tr("STR_CONTENT_NEXT"), e.load_scroll + LOAD_ROWS < list.len()),
                    Hit::LoadBack => button(g, r, tr("STR_SCENARIOS_BACK"), true),
                    _ => {}
                }
            }
            if !e.status.is_empty() {
                s.put(g, F_ARIAL10, BOX.x + 24.0, BOX.y + BOX.h - 40.0, &crate::content::fit(&e.status, 480.0), navy);
            }
            g.flush();
            return;
        }
        s.put_shadowed(g, F_MANUAL20, 400.0, BOX.y + 14.0, tr("STR_EDITOR_TITLE"), yellow, true);
        s.put(g, F_ARIAL10, BOX.x + 24.0, BOX.y + 40.0, tr("STR_EDITOR_HELP"), navy);
        let caret = if (self.clock * 2.0) as i64 % 2 == 0 { "|" } else { "" };
        let owned = self.tracts_owned();
        for (r, h) in layout(e) {
            match h {
                Hit::Field(f) => {
                    let name = FIELDS.iter().find(|x| x.0 == f).map(|x| x.1).unwrap_or("");
                    label(g, 60.0, r.y, tr(name));
                    let typing = e.focus == Some(f);
                    s.fill(g, r.x, r.y, r.w, r.h, if typing { rgb(1.0, 1.0, 1.0) } else { rgb(0.9, 0.9, 0.97) });
                    let mut t = format!("{}{}", e.field_text(f), if typing { caret } else { "" });
                    // a long text shows its end while it is typed, its start otherwise
                    if typing {
                        while F_ARIAL10.width(&t) > r.w - 10.0 && t.chars().count() > 1 {
                            t.remove(0);
                        }
                    } else {
                        t = crate::content::fit(&t, r.w - 10.0);
                    }
                    s.put(g, F_ARIAL10, r.x + 5.0, r.y + 4.0, &t, rgb(0.05, 0.05, 0.2));
                }
                Hit::CashLess => {
                    label(g, 60.0, r.y, tr("STR_EDITOR_CASH"));
                    button(g, r, "-", true);
                    value(g, r.x + r.w, r.y, 100.0, &crate::ui::money(e.start_money));
                }
                Hit::CashMore | Hit::WaterMore => button(g, r, "+", true),
                Hit::Difficulty => {
                    label(g, 60.0, r.y, tr("STR_EDITOR_DIFFICULTY"));
                    let d =
                        e.difficulty.map_or(tr("STR_EDITOR_ANY_DIFFICULTY"), |d| tr(crate::render::DIFFICULTY_KEYS[d.level() as usize]));
                    button(g, r, d, true);
                }
                Hit::Sandbox => {
                    label(g, 60.0, r.y, tr("STR_EDITOR_SANDBOX"));
                    button(g, r, tr(if e.sandbox { "STR_EDITOR_ON" } else { "STR_EDITOR_OFF" }), true);
                    // the course: where and how large (chosen when the editor started)
                    let place = self.land.as_ref().map(|l| l.slot.record().name().to_string()).unwrap_or_default();
                    let n = sg_core::mapsize::n();
                    let course = trf("STR_EDITOR_COURSE_TEXT", &[("PLACE", &place), ("SIZE", &sg_core::mapsize::name(n)), ("N", &n)]);
                    label(g, 60.0, r.y + 21.0, tr("STR_EDITOR_COURSE"));
                    s.put(g, F_ARIAL10, 150.0, r.y + 25.0, &course, white);
                }
                Hit::Tract(i) => {
                    if i == 0 {
                        label(g, 492.0, 96.0, tr("STR_EDITOR_LAND"));
                    }
                    let on = owned.get(i).copied().unwrap_or(false);
                    let lit = r.has(px, py);
                    let c = match (on, lit) {
                        (true, false) => rgb(0.36, 0.62, 0.36),
                        (true, true) => rgb(0.5, 0.8, 0.5),
                        (false, false) => rgb(0.62, 0.5, 0.36),
                        (false, true) => rgb(0.8, 0.66, 0.5),
                    };
                    s.fill(g, r.x, r.y, r.w, r.h, c);
                    s.put_centered(
                        g,
                        F_ARIAL10,
                        r.x + r.w / 2.0,
                        r.y + 5.0,
                        tr(if on { "STR_EDITOR_OWNED" } else { "STR_EDITOR_FOR_SALE" }),
                        white,
                    );
                }
                Hit::Relief => {
                    label(g, 492.0, 196.0, tr("STR_EDITOR_HM"));
                    label(g, 492.0, r.y, tr("STR_EDITOR_HM_RELIEF"));
                    button(g, r, tr(RELIEF_KEYS[e.relief.min(2)]), true);
                }
                Hit::WaterLess => {
                    label(g, 492.0, r.y, tr("STR_EDITOR_HM_WATER"));
                    button(g, r, "-", true);
                    let w = if e.water == 0 {
                        tr("STR_EDITOR_HM_WATER_OFF").to_string()
                    } else {
                        trf("STR_EDITOR_HM_WATER_LEVEL", &[("N", &e.water)])
                    };
                    value(g, r.x + r.w, r.y, 110.0, &w);
                }
                Hit::Smooth => {
                    label(g, 492.0, r.y, tr("STR_EDITOR_HM_SMOOTH"));
                    button(g, r, tr(if e.smooth { "STR_EDITOR_ON" } else { "STR_EDITOR_OFF" }), true);
                }
                Hit::Pick => button(g, r, tr(if e.picking { "STR_EDITOR_HM_PICKING" } else { "STR_EDITOR_HM_PICK" }), true),
                Hit::Sample(k) => {
                    if k == 0 {
                        label(g, 492.0, r.y, tr("STR_EDITOR_HM_SAMPLES"));
                    }
                    button(g, r, tr(SAMPLES[k].0), true);
                }
                Hit::GoalKind(k) => {
                    if k == 0 {
                        label(g, 60.0, GOAL_TOP - 20.0, tr("STR_EDITOR_GOALS"));
                    }
                    let goal = &e.goals[k];
                    button(g, r, tr(GOAL_KINDS[goal_kind(goal)]), true);
                    value(g, 184.0, r.y, 80.0, &goal_value(goal));
                    let by = goal.by_year.map_or(tr("STR_EDITOR_ANY_TIME").to_string(), |y| trf("STR_EDITOR_BY_YEAR", &[("YEAR", &y)]));
                    value(g, 310.0, r.y, 80.0, &by);
                    s.put(g, F_ARIAL10, 418.0, r.y + 4.0, &crate::content::fit(&goal.text, 292.0), white);
                }
                Hit::GoalLess(_) | Hit::YearLess(_) => button(g, r, "-", true),
                Hit::GoalMore(_) | Hit::YearMore(_) => button(g, r, "+", true),
                Hit::GoalRemove(_) => button(g, r, "x", true),
                Hit::GoalAdd => {
                    if e.goals.is_empty() {
                        label(g, 60.0, GOAL_TOP - 20.0, tr("STR_EDITOR_GOALS"));
                        s.put(g, F_ARIAL10, 170.0, r.y + 4.0, tr("STR_EDITOR_NO_GOALS"), white);
                    }
                    button(g, r, tr("STR_EDITOR_GOAL_ADD"), true);
                }
                Hit::Save => button(g, r, tr("STR_EDITOR_SAVE"), true),
                Hit::Load => button(g, r, tr("STR_EDITOR_LOAD"), true),
                Hit::Back => button(g, r, tr("STR_EDITOR_BACK"), true),
                Hit::Leave => button(g, r, tr(if e.leaving { "STR_EDITOR_LEAVE_SURE" } else { "STR_EDITOR_LEAVE" }), true),
                _ => {}
            }
        }
        if !e.status.is_empty() {
            for (k, l) in crate::ui::wrap_text(&e.status, 10.0, 660.0).into_iter().take(2).enumerate() {
                s.put(g, F_ARIAL10, 60.0, 476.0 + 13.0 * k as f32, &l, navy);
            }
        }
        g.flush();
    }

    // ---- land owned at the start -----------------------------------------------------------------------------------------

    /// Whether each of the nine tracts (sg_core::tracts) is wholly owned; a tract with land out of bounds is for sale, as
    /// TRACTS FOR SALE counts it.
    fn tracts_owned(&self) -> Vec<bool> {
        let Some(l) = self.land.as_ref() else { return vec![true; 9] };
        let side = sg_core::tracts::side();
        (0..9)
            .map(|i| {
                let (a0, b0) = sg_core::tracts::origin(i);
                (0..side).all(|da| (0..side).all(|db| l.ty[((a0 + da) * land::mn() + b0 + db) as usize] != land::T_OUT))
            })
            .collect()
    }

    /// A click on a tract: owned land goes up for sale (its tiles out of bounds, their types kept for the purchase; tiles
    /// with a building, a path, a tee or a green stay owned), land for sale becomes owned (as buying it does).
    pub fn toggle_tract(&mut self, i: usize) {
        if i >= 9 {
            return;
        }
        self.sync_course();
        self.undo_checkpoint();
        let owned = self.tracts_owned()[i];
        let Some(l) = self.land.as_mut() else { return };
        let side = sg_core::tracts::side();
        let (a0, b0) = sg_core::tracts::origin(i);
        if owned {
            for da in 0..side {
                for db in 0..side {
                    let t = ((a0 + da) * land::mn() + b0 + db) as usize;
                    let keep = l.flags[t] & (land::flag::FOOTPRINT | land::flag::PATH) != 0
                        || matches!(l.ty[t], land::T_TEE | land::T_GREEN | land::T_OUT);
                    if !keep {
                        if l.original.len() == l.ty.len() {
                            l.original[t] = l.ty[t];
                        }
                        l.ty[t] = land::T_OUT;
                        l.layer[t] = sg_core::data::land_type(land::T_OUT).layer;
                    }
                }
            }
        } else {
            sg_core::tracts::buy(l, i);
        }
        l.write_area(&mut self.terrain, a0 - 1, b0 - 1, a0 + side, b0 + side);
        // water keeps its depth on the port's terrain
        for da in -1..=side {
            for db in -1..=side {
                let (a, b) = (a0 + da, b0 + db);
                if (0..land::mn()).contains(&a) && (0..land::mn()).contains(&b) {
                    let t = (a * land::mn() + b) as usize;
                    if l.ty[t] == land::T_WATER {
                        let o = self.terrain.tile_index(a, b);
                        self.terrain.variation[o] = l.var[t].min(2);
                    }
                }
            }
        }
        println!("editor: tract {} {}", i + 1, if owned { "for sale" } else { "owned" });
        self.dirty = true;
        self.sync_course();
    }

    // ---- heightmaps ------------------------------------------------------------------------------------------------------

    /// "Import heightmap...": the system's file picker (desktop), the page's (browser). Test hook: SG_PICK_PNG=FILE picks
    /// FILE.
    pub fn pick_heightmap(&mut self) {
        if let Some(p) = std::env::var_os("SG_PICK_PNG") {
            return self.import_heightmap_file(std::path::Path::new(&p));
        }
        #[cfg(all(feature = "picker", not(any(target_arch = "wasm32", target_os = "android"))))]
        {
            let picked = rfd::FileDialog::new().set_title(tr("STR_EDITOR_HM_PICK_TITLE")).add_filter("PNG", &["png", "PNG"]).pick_file();
            if let Some(p) = picked {
                self.import_heightmap_file(&p);
            }
        }
        #[cfg(target_arch = "wasm32")]
        {
            // SAFETY: a plain call into the page's script, no arguments (web/opensgolf.js)
            unsafe { web::sg_pick_file() };
            if let Some(e) = self.editor.as_mut() {
                e.picking = true;
            }
        }
    }

    /// Each frame: a picture the browser's picker has read (web/opensgolf.js keeps it under picked/).
    pub fn poll_editor(&mut self) {
        #[cfg(target_arch = "wasm32")]
        if self.editor.as_ref().is_some_and(|e| e.picking) {
            let mut buf = [0u8; 512];
            // SAFETY: the page writes at most buf.len() bytes of the picked file's name into buf
            let n = unsafe { web::sg_pick_poll(buf.as_mut_ptr(), buf.len()) };
            if n >= 0 {
                if let Some(e) = self.editor.as_mut() {
                    e.picking = false;
                }
                let name = String::from_utf8_lossy(&buf[..n as usize]).to_string();
                self.import_heightmap_file(&std::path::Path::new("picked").join(name));
            }
        }
    }

    pub fn import_heightmap_file(&mut self, path: &std::path::Path) {
        let name = path.file_name().map(|n| n.to_string_lossy().to_string()).unwrap_or_default();
        let r = sg_core::fsutil::read_file(path)
            .ok_or_else(|| trf("STR_ERROR_FILE_NOT_FOUND", &[("FILE", &path.display())]))
            .and_then(|b| self.import_heightmap(&name, &b));
        if let Err(e) = r {
            self.ui_sound(0x18);
            self.editor_status(&e);
        }
    }

    /// Reads a PNG and lays it over the land with the box's settings.
    pub fn import_heightmap(&mut self, name: &str, bytes: &[u8]) -> Result<(), String> {
        let pic = Grey::from_png(bytes).map_err(|e| format!("{name}: {e}"))?;
        if let Some(e) = self.editor.as_mut() {
            e.picture = Some((name.to_string(), pic.clone()));
        }
        self.lay_heightmap(name, &pic);
        Ok(())
    }

    /// Lays a picture over the land (sg_core::heightmap::apply): buildings, paths, tees and greens keep their tiles and are
    /// levelled again, land for sale keeps its border (its water waits under it for the purchase), and the land's rules
    /// (sea level at the map's edge and deep in the land for sale) are applied as the game's own tools do.
    pub fn lay_heightmap(&mut self, name: &str, pic: &Grey) {
        let Some(opts) = self.editor.as_ref().map(|e| e.options()) else { return };
        self.sync_course();
        self.undo_checkpoint();
        let Some(l) = self.land.as_ref() else { return };
        let n = land::mn();
        let ti = |x: i32, y: i32| (x * n + y) as usize;
        // what keeps its type, and the regions levelled afterwards
        let fixed: Vec<bool> = (0..n * n)
            .map(|k| {
                let (x, y) = (k % n, k / n);
                let t = ti(x, y);
                l.flags[t] & (land::flag::FOOTPRINT | land::flag::PATH) != 0 || matches!(l.ty[t], land::T_TEE | land::T_GREEN | land::T_OUT)
            })
            .collect();
        let region: Vec<Option<u32>> = (0..n * n)
            .map(|k| {
                let (x, y) = (k % n, k / n);
                let t = ti(x, y);
                if l.flags[t] & land::flag::FOOTPRINT != 0 {
                    Some(1000 + l.var[t] as u32)
                } else {
                    match l.ty[t] {
                        land::T_TEE => Some(1),
                        land::T_GREEN => Some(2),
                        _ => None,
                    }
                }
            })
            .collect();
        // the land's old water goes (it would hang on the new slopes): the picture's water takes its place
        let mut old_water = Vec::new();
        for x in 0..n {
            for y in 0..n {
                let t = ti(x, y);
                let out_water = l.ty[t] == land::T_OUT && l.original.get(t) == Some(&land::T_WATER);
                if (l.ty[t] == land::T_WATER || out_water) && l.flags[t] & land::flag::FOOTPRINT == 0 {
                    old_water.push((x, y, out_water));
                }
            }
        }
        for &(x, y, out) in &old_water {
            if !out {
                self.terrain.paint(x, y, land::T_ROUGH as i32, 0);
            }
        }
        let report = heightmap::apply(&mut self.terrain, pic, &opts, &|x, y| fixed[(y * n + x) as usize]);
        // water under land for sale lies at sea level too, ready for the tract's purchase
        if let Some(l) = self.land.as_ref() {
            for &(x, y, _) in &report.water {
                if l.ty[ti(x, y)] == land::T_OUT && l.flags[ti(x, y)] & land::flag::FOOTPRINT == 0 {
                    for (cx, cy) in [(x, y), (x + 1, y), (x, y + 1), (x + 1, y + 1)] {
                        self.terrain.corner[(cy * (n + 1) + cx) as usize] = 0;
                    }
                }
            }
        }
        heightmap::flatten_regions(&mut self.terrain, &|x, y| region[(y * n + x) as usize]);
        let Some(l) = self.land.as_mut() else { return };
        l.sync_from_terrain(&self.terrain);
        for &(x, y, out) in &old_water {
            if out {
                l.original[ti(x, y)] = land::T_ROUGH;
            }
        }
        // water under land for sale: the tract's purchase brings it
        let mut made = 0;
        for &(x, y, depth) in &report.water {
            let t = ti(x, y);
            if l.ty[t] == land::T_OUT && l.original.len() == l.ty.len() && l.flags[t] & land::flag::FOOTPRINT == 0 {
                l.original[t] = land::T_WATER;
                l.var[t] = depth;
                made += 1;
            } else if l.ty[t] == land::T_WATER {
                l.var[t] = depth;
                made += 1;
            }
        }
        l.write_area(&mut self.terrain, 0, 0, n - 1, n - 1);
        for x in 0..n {
            for y in 0..n {
                if l.ty[ti(x, y)] == land::T_WATER {
                    let o = self.terrain.tile_index(x, y);
                    self.terrain.variation[o] = l.var[ti(x, y)].min(2);
                }
            }
        }
        self.dirty = true;
        self.sync_course();
        let msg = trf("STR_EDITOR_HM_DONE", &[("FILE", &name), ("WATER", &made)]);
        self.editor_status(&msg);
    }

    // ---- saving and loading -----------------------------------------------------------------------------------------------

    /// "Save scenario": the scenario pack in the save folder's packs (its own folder: pack.toml, scenario.toml, course.sgs).
    /// The first save takes a new id from the name; later saves (and a scenario loaded from the save folder) write over
    /// it. Returns the pack's id.
    pub fn save_editor_scenario(&mut self) -> Result<String, String> {
        self.sync_course();
        let Some(e) = self.editor.as_ref() else { return Err("not in the editor".into()) };
        let property = self.land.as_ref().map(|l| sg_core::data::property_id(l.slot.property).to_string());
        let sc = e.scenario(property);
        // checked as the game will read it
        let text = sc.to_toml();
        Scenario::parse(&text)?;
        let name = e.display_name();
        let packs_dir = save_dir().join("packs");
        let id = match &e.pack_id {
            Some(id) => id.clone(),
            None => {
                let base = scenario::id_from_name(&name);
                let taken = |id: &str| {
                    sg_core::fsutil::is_dir(packs_dir.join(id))
                        || sg_core::fsutil::read_file(packs_dir.join(format!("{id}.zip"))).is_some()
                        || self.content.found.packs.iter().any(|p| p.manifest.id == id)
                };
                std::iter::once(base.clone()).chain((2..1000).map(|k| format!("{base}-{k}"))).find(|id| !taken(id)).unwrap_or(base)
            }
        };
        let description = if e.description.trim().is_empty() {
            tr("STR_EDITOR_DEFAULT_DESCRIPTION").to_string()
        } else {
            e.description.trim().to_string()
        };
        let manifest = scenario::pack_manifest(&id, &name, e.author.trim(), &description, &e.version);
        let course = self.scenario_course_bytes()?;
        let dir = packs_dir.join(&id);
        let header = format!("# {name}\n# Made in the OpenSGolf course editor; docs/MODDING.md, section 5, explains every field.\n\n");
        let ok = sg_core::fsutil::write_file(dir.join("pack.toml"), manifest.as_bytes())
            && sg_core::fsutil::write_file(dir.join("scenario.toml"), format!("{header}{text}").as_bytes())
            && sg_core::fsutil::write_file(dir.join("course.sgs"), &course);
        if !ok {
            return Err(trf("STR_ERROR_COULD_NOT_WRITE", &[("FILE", &dir.display())]));
        }
        if let Some(e) = self.editor.as_mut() {
            e.pack_id = Some(id.clone());
        }
        self.content.rediscover();
        println!("editor: saved scenario {id} in {}", dir.display());
        self.editor_status(&trf("STR_EDITOR_SAVED", &[("PACK", &id), ("DIR", &dir.display())]));
        Ok(id)
    }

    /// The course a scenario starts from (course.sgs): the game as it is in the editor, with the scenario's money,
    /// difficulty and sandbox, the landmarks a new game would own, no goals and no scripts. The theme's people, stories,
    /// celebrities and pros are left out: they are the game files' (the original's text when it is played with SimGolf's
    /// files), and a game started from the course reads them from the player's own files (App::load_save_with), so a
    /// scenario holds nothing but the course. The game in hand is not changed.
    fn scenario_course_bytes(&mut self) -> Result<Vec<u8>, String> {
        let Some(e) = self.editor.as_ref() else { return Err("not in the editor".into()) };
        let (money, difficulty, sandbox) = (e.start_money, e.difficulty, e.sandbox);
        let people = (
            std::mem::take(&mut self.club.roster),
            std::mem::take(&mut self.club.stories),
            std::mem::take(&mut self.club.celebrities),
            std::mem::take(&mut self.club.pros),
        );
        let kept = (self.econ.clone(), self.difficulty, self.club.landmarks_owned, std::mem::take(&mut self.scen.goals));
        let version = self.econ.version;
        self.econ.free = false;
        self.econ.sandbox = sandbox;
        self.econ.start_cash = money as f64;
        self.econ.init();
        self.econ.version = version;
        if let Some(d) = difficulty {
            self.difficulty = d.level();
        }
        // the property's own landmarks and those placed (a sandbox has them all), as a new game starts with
        self.club.landmarks_owned = if sandbox {
            0x3fff
        } else {
            self.land
                .as_ref()
                .map(|l| l.objects.iter().filter(|o| o.kind == land::K_LANDMARK && o.sub < 16).fold(0, |m, o| m | 1 << o.sub))
                .unwrap_or(0)
        };
        let out = self.save_bytes(6);
        (self.econ, self.difficulty, self.club.landmarks_owned, self.scen.goals) = kept;
        (self.club.roster, self.club.stories, self.club.celebrities, self.club.pros) = people;
        out
    }

    /// "Load scenario...": scenario `k` of the page's list into the editor: its course (or, without one, the land of its
    /// property as it starts) and its settings.
    fn load_editor_scenario(&mut self, g: &mut Gfx, k: usize) -> Result<(), String> {
        let Some(entry) = self.editor.as_ref().and_then(|e| e.loading.as_ref()).and_then(|l| l.get(k)) else { return Ok(()) };
        let id = entry.id.clone();
        self.load_scenario_into_editor(g, &id)
    }

    /// Loads the scenario pack `id` into the editor (also the `--edit-scenario ID` test hook).
    pub fn load_scenario_into_editor(&mut self, g: &mut Gfx, id: &str) -> Result<(), String> {
        let list = crate::scenario_ui::find_scenarios(&self.content);
        let entry = list.iter().find(|e| e.id == id).ok_or_else(|| format!("no scenario pack {id}"))?;
        let sc = entry.def.clone().map_err(|e| format!("{id}: {e}"))?;
        let pack = self.content.found.packs.iter().find(|p| p.manifest.id == id).ok_or_else(|| format!("no scenario pack {id}"))?;
        let manifest = pack.manifest.clone();
        // a pack in the save folder's packs is saved back in place, any other as a copy
        let mine = match pack.source.as_ref() {
            sg_core::vfs::Source::Dir(d) => d.parent().is_some_and(|p| p == save_dir().join("packs")),
            _ => false,
        };
        let course = match &sc.scenario.course {
            Some(c) => Some(pack.source.read(c).ok_or_else(|| trf("STR_ERROR_FILE_NOT_FOUND", &[("FILE", c)]))?),
            None => None,
        };
        match course {
            Some(bytes) => {
                let save = read_save_bytes(&bytes, &format!("{id}/course.sgs"))?;
                self.load_save(g, save)?;
            }
            None => {
                // the land the scenario starts on
                let p = sc.property_index();
                self.course_size = sg_core::mapsize::ORIGINAL;
                if let Some(d) = sc.scenario.difficulty {
                    self.difficulty = d.level();
                }
                self.course_id = Some(self.exe_rng.below(0xffff));
                self.deal_offer(false);
                self.cheap_slot(p);
                self.start_game(g, p, false);
            }
        }
        let mut ed = Editor {
            start_money: self.econ.cash as i64 / 100 * 100,
            version: manifest.version.clone(),
            author: manifest.authors.first().cloned().unwrap_or_default(),
            pack_id: mine.then(|| id.to_string()),
            relief: 1,
            smooth: true,
            ..Default::default()
        };
        ed.take(&sc);
        if ed.name.is_empty() {
            ed.name = manifest.name.clone();
        }
        let name = ed.display_name();
        self.enter_editor(ed);
        self.econ.cash = self.editor.as_ref().map_or(0, |e| e.start_money) as f64;
        self.editor_status(&trf("STR_EDITOR_LOADED", &[("NAME", &name)]));
        println!("editor: loaded scenario {id}{}", if mine { "" } else { " (saved as a copy)" });
        Ok(())
    }

    /// The `--editor-set "key=value;..."` test hook: the box's settings without the box (name, author, description,
    /// intro, win, lose, cash, difficulty 0-3 or "any", sandbox 0/1, relief 0-2, water 0.., smooth 0/1, goal=KIND:VALUE[@YEAR]
    /// with KIND holes, cash, fun, skill, members or rank, tract=N:sale|own with N 1..9).
    pub fn editor_settings_hook(&mut self, spec: &str) {
        for item in spec.split(';').filter(|s| !s.trim().is_empty()) {
            let Some((k, v)) = item.split_once('=') else {
                eprintln!("bad --editor-set item: {item}");
                continue;
            };
            let (k, v) = (k.trim(), v.trim());
            if k == "tract" {
                let (n, what) = v.split_once(':').unwrap_or((v, "sale"));
                let i = n.parse::<usize>().unwrap_or(0).wrapping_sub(1);
                if i < 9 && self.tracts_owned()[i] == (what == "sale") {
                    self.toggle_tract(i);
                }
                continue;
            }
            let Some(e) = self.editor.as_mut() else { return };
            match k {
                "name" | "author" | "description" | "intro" | "win" | "lose" => {
                    let f = match k {
                        "name" => Field::Name,
                        "author" => Field::Author,
                        "description" => Field::Description,
                        "intro" => Field::Intro,
                        "win" => Field::Win,
                        _ => Field::Lose,
                    };
                    *e.field(f) = v.to_string();
                }
                "cash" => {
                    e.start_money = v.parse().unwrap_or(e.start_money);
                    self.econ.cash = e.start_money as f64;
                }
                "difficulty" => e.difficulty = v.parse::<i32>().ok().map(Difficulty::from_level),
                "sandbox" => e.sandbox = v == "1",
                "relief" => e.relief = v.parse::<usize>().unwrap_or(1).min(2),
                "water" => e.water = v.parse().unwrap_or(0),
                "smooth" => e.smooth = v != "0",
                "goal" => {
                    let (what, year) = v.split_once('@').map_or((v, None), |(w, y)| (w, y.parse().ok()));
                    let (kind, val) = what.split_once(':').unwrap_or((what, ""));
                    let mut c = Conditions::default();
                    match kind {
                        "holes" => c.holes = val.parse().ok(),
                        "cash" => c.cash = val.parse().ok(),
                        "fun" => c.fun = val.parse().ok(),
                        "skill" => c.skill = val.parse().ok(),
                        "members" => c.members = val.parse().ok(),
                        "rank" => c.rank = Some(val.to_string()),
                        _ => {}
                    }
                    if c.is_empty() {
                        eprintln!("bad --editor-set goal: {v}");
                        continue;
                    }
                    let mut g = GoalDef { by_year: year, ..Default::default() };
                    g.set_when(&c);
                    g.text = g.auto_text();
                    e.goals.push(g);
                }
                _ => eprintln!("bad --editor-set item: {item}"),
            }
        }
    }
}

#[cfg(target_arch = "wasm32")]
pub(crate) mod web {
    #[link(wasm_import_module = "env")]
    extern "C" {
        /// Opens the page's file picker for a PNG (web/opensgolf.js).
        pub fn sg_pick_file();
        /// Opens the page's file picker for a saved game, shared course or course code (Load Previous Game's Import).
        pub fn sg_pick_save();
        /// The picked file's name (its bytes are kept under picked/), its length; -1 while nothing is picked.
        pub fn sg_pick_poll(dst: *mut u8, cap: usize) -> i32;
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn goal_rows_step_within_their_ranges() {
        let mut g = goal_of_kind(0, Some(2003));
        assert_eq!(g.holes, Some(9));
        assert_eq!(g.text, "Reach 9 holes by the end of 2003");
        for _ in 0..30 {
            step_goal(&mut g, true);
        }
        assert_eq!(g.holes, Some(18));
        let mut r = goal_of_kind(5, None);
        step_goal(&mut r, false);
        step_goal(&mut r, false);
        assert_eq!(r.rank.as_deref(), Some("municipal"));
        let mut c = goal_of_kind(1, None);
        step_goal(&mut c, true);
        assert_eq!(c.cash, Some(110_000));
        assert_eq!(goal_kind(&c), 1);
        // every kind makes a goal the scenario check takes
        for k in 0..GOAL_KINDS.len() {
            assert!(!goal_of_kind(k, None).when().is_empty());
        }
    }

    #[test]
    fn the_editor_writes_a_valid_scenario() {
        let mut e = Editor { name: "  Valley  ".into(), start_money: 75_000, version: "1.0.0".into(), ..Default::default() };
        e.goals.push(goal_of_kind(0, Some(2003)));
        e.goals.push(GoalDef { text: String::new(), ..goal_of_kind(4, None) });
        let sc = e.scenario(Some("phoenix".into()));
        let text = sc.to_toml();
        let back = Scenario::parse(&text).unwrap();
        assert_eq!(back.scenario.name.as_deref(), Some("Valley"));
        assert_eq!(back.scenario.course.as_deref(), Some("course.sgs"));
        assert_eq!(back.scenario.start_money, Some(75_000));
        assert_eq!(back.goals.len(), 2);
        assert_eq!(back.goals[1].text, "Reach 20 members", "an empty text is the goal's own words");
        assert_eq!(back.goals[1].id.as_deref(), Some("goal-2"));
        // loaded back into the editor and written again: the same bytes
        let mut e2 = Editor::default();
        e2.take(&back);
        assert_eq!(e2.scenario(Some("phoenix".into())).to_toml(), text);
        // no name: "My Scenario"
        assert_eq!(Editor::default().display_name(), "My Scenario");
    }

    #[test]
    fn the_box_has_every_control_once_and_inside() {
        let mut e = Editor::default();
        for k in 0..MAX_GOALS {
            e.goals.push(goal_of_kind(k % GOAL_KINDS.len(), None));
        }
        let v = layout(&e);
        for (i, (r, h)) in v.iter().enumerate() {
            assert!(r.x >= BOX.x && r.y >= BOX.y && r.x + r.w <= BOX.x + BOX.w && r.y + r.h <= BOX.y + BOX.h, "{h:?} outside");
            for (r2, h2) in &v[i + 1..] {
                let apart = r.x + r.w <= r2.x || r2.x + r2.w <= r.x || r.y + r.h <= r2.y || r2.y + r2.h <= r.y;
                assert!(apart, "{h:?} overlaps {h2:?}");
            }
        }
        assert!(!v.iter().any(|(_, h)| *h == Hit::GoalAdd), "five goals at most");
        e.loading = Some(Vec::new());
        assert!(layout(&e).iter().any(|(_, h)| *h == Hit::LoadBack));
    }

    #[test]
    fn the_samples_are_heightmaps() {
        for (_, png) in SAMPLES {
            let g = Grey::from_png(png).unwrap();
            assert_eq!((g.w, g.h), (64, 64));
            assert!(g.v.iter().any(|&v| v < 0.2) && g.v.iter().any(|&v| v > 0.6));
        }
    }
}
