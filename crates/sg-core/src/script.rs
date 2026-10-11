//! Game scripts of content packs (docs/MODDING.md, section 6), in Rhai.
//!
//! A pack may hold `scripts/*.rhai`. Each file may define the hooks `on_start()`, `on_month()`, `on_year_end()`,
//! `on_hole_finished(golfer, hole, strokes)` and `on_building_placed(kind, x, y)`; the game calls them for the packs the
//! player enabled (and for the scenario pack a game was started from), in load order. Inside a hook `this` is the pack's
//! own state, a map that is saved with the game.
//!
//! Scripts are sandboxed: Rhai has no file or network access, `eval` is switched off, and every hook call is limited in
//! operations, call depth, string, array and map sizes. A script that fails (a syntax error, a run-time error, a limit hit)
//! is reported, never crashes the game; a pack whose scripts fail ten times is stopped for the session. Scripts get random
//! numbers from their own generator (saved with the game), so they never disturb the game's own random numbers and a
//! loaded game goes on the same way.
use rhai::{Array, CallFnOptions, Dynamic, Engine, ImmutableString, Map, Scope, AST};
use serde::{Deserialize, Serialize};
use std::cell::RefCell;
use std::collections::BTreeMap;
use std::rc::Rc;

/// Operations one hook call may run (a loop of a few thousand steps is fine; an endless one stops here).
pub const MAX_OPERATIONS: u64 = 200_000;
/// Errors after which a pack's scripts are stopped for the session.
pub const MAX_ERRORS: u32 = 10;

/// A golfer as the scripts see it.
#[derive(Clone, Debug, Default, PartialEq)]
pub struct GolferView {
    pub name: String,
    /// The hole the golfer is on (as numbered on the course), 0 when not on a hole.
    pub hole: i32,
    /// Strokes on the current hole so far.
    pub strokes: i32,
    /// A club member (not a visitor).
    pub member: bool,
    /// The player's own pro.
    pub pro: bool,
}

/// A hole as the scripts see it.
#[derive(Clone, Copy, Debug, Default, PartialEq)]
pub struct HoleView {
    pub number: i32,
    pub par: i32,
    pub yards: i32,
}

/// What the scripts can read of the game (filled by the game before each hook).
#[derive(Clone, Debug, Default, PartialEq)]
pub struct GameView {
    /// The calendar year (2001 is the first) and month (3 March .. 10 October: the game's year has eight months).
    pub year: i32,
    pub month: i32,
    /// Cash, in money.
    pub cash: i64,
    pub fun: i32,
    /// Skill rating in hundredths.
    pub skill: i32,
    pub members: i32,
    /// 0 Municipal, 1 Daily Fee, 2 Country Club, 3 Championship.
    pub rank: i32,
    /// 0 Easy .. 3 Impossible.
    pub difficulty: i32,
    pub sandbox: bool,
    pub course: String,
    pub holes: Vec<HoleView>,
    pub golfers: Vec<GolferView>,
}

/// A hook the game calls.
#[derive(Clone, Debug, PartialEq)]
pub enum Hook {
    Start,
    Month,
    YearEnd,
    HoleFinished { golfer: GolferView, hole: i32, strokes: i32 },
    BuildingPlaced { kind: String, x: i32, y: i32 },
}

impl Hook {
    pub fn name(&self) -> &'static str {
        match self {
            Hook::Start => "on_start",
            Hook::Month => "on_month",
            Hook::YearEnd => "on_year_end",
            Hook::HoleFinished { .. } => "on_hole_finished",
            Hook::BuildingPlaced { .. } => "on_building_placed",
        }
    }
    fn args(&self) -> Vec<Dynamic> {
        match self {
            Hook::Start | Hook::Month | Hook::YearEnd => Vec::new(),
            Hook::HoleFinished { golfer, hole, strokes } => {
                vec![golfer_map(golfer).into(), (*hole as i64).into(), (*strokes as i64).into()]
            }
            Hook::BuildingPlaced { kind, x, y } => vec![kind.clone().into(), (*x as i64).into(), (*y as i64).into()],
        }
    }
}

/// What a script asked the game to do.
#[derive(Clone, Debug, PartialEq)]
pub enum Action {
    Message(String),
    /// Money (may be negative) and the reason, for the log.
    Money(i64, String),
    SetGoal {
        id: String,
        text: String,
    },
    /// A goal completed (true) or failed (false).
    FinishGoal {
        id: String,
        done: bool,
    },
}

/// The result of calling a hook: the actions in order, each with its pack's id, and the problems.
#[derive(Clone, Debug, Default, PartialEq)]
pub struct Output {
    pub actions: Vec<(String, Action)>,
    /// (pack id, message)
    pub errors: Vec<(String, String)>,
    /// Lines the scripts printed (print, debug), with the pack id.
    pub log: Vec<(String, String)>,
}

/// One pack's saved script state.
#[derive(Clone, Debug, Default, PartialEq, Serialize, Deserialize)]
pub struct PackState {
    /// The pack's `this` map, as JSON.
    #[serde(default)]
    pub state: serde_json::Value,
    /// The pack's random generator.
    #[serde(default)]
    pub rng: u64,
}

struct Ctx {
    view: GameView,
    actions: Vec<Action>,
    log: Vec<String>,
    rng: u64,
}

struct PackScripts {
    id: String,
    files: Vec<(String, AST)>,
    state: Dynamic,
    rng: u64,
    errors: u32,
}

/// The scripts of a game.
pub struct Host {
    engine: Engine,
    ctx: Rc<RefCell<Ctx>>,
    packs: Vec<PackScripts>,
}

fn golfer_map(g: &GolferView) -> Map {
    let mut m = Map::new();
    m.insert("name".into(), g.name.clone().into());
    m.insert("hole".into(), (g.hole as i64).into());
    m.insert("strokes".into(), (g.strokes as i64).into());
    m.insert("member".into(), g.member.into());
    m.insert("pro".into(), g.pro.into());
    m
}

/// A seed for a pack's generator: the game's seed mixed with the pack's id.
fn seed_for(seed: u64, id: &str) -> u64 {
    let h = id.bytes().fold(0xcbf2_9ce4_8422_2325u64, |h, b| (h ^ b as u64).wrapping_mul(0x0000_0100_0000_01b3));
    let s = seed ^ h;
    if s == 0 {
        0x9e37_79b9_7f4a_7c15
    } else {
        s
    }
}

/// xorshift64*: the scripts' own generator.
fn next(rng: &mut u64) -> u64 {
    let mut x = *rng;
    x ^= x >> 12;
    x ^= x << 25;
    x ^= x >> 27;
    *rng = x;
    x.wrapping_mul(0x2545_f491_4f6c_dd1d)
}

impl Default for Host {
    fn default() -> Self {
        Host::new()
    }
}

impl Host {
    /// An engine with the limits and the game's functions, and no scripts yet.
    pub fn new() -> Host {
        let ctx = Rc::new(RefCell::new(Ctx { view: GameView::default(), actions: Vec::new(), log: Vec::new(), rng: 1 }));
        let mut e = Engine::new();
        e.set_max_operations(MAX_OPERATIONS)
            .set_max_call_levels(32)
            .set_max_expr_depths(64, 32)
            .set_max_string_size(16 * 1024)
            .set_max_array_size(10_000)
            .set_max_map_size(10_000)
            .set_max_variables(1_000)
            .set_max_functions(1_000);
        e.disable_symbol("eval");
        {
            let c = ctx.clone();
            e.on_print(move |s| c.borrow_mut().log.push(s.to_string()));
            let c = ctx.clone();
            e.on_debug(move |s, _, _| c.borrow_mut().log.push(s.to_string()));
        }
        // reading the game
        macro_rules! getter {
            ($name:literal, |$v:ident| $body:expr) => {{
                let c = ctx.clone();
                e.register_fn($name, move || {
                    let $v = &c.borrow().view;
                    $body
                });
            }};
        }
        getter!("year", |v| v.year as i64);
        getter!("month", |v| v.month as i64);
        getter!("cash", |v| v.cash);
        getter!("fun_rating", |v| v.fun as i64);
        getter!("skill_rating", |v| v.skill as f64 / 100.0);
        getter!("members", |v| v.members as i64);
        getter!("rank", |v| v.rank as i64);
        getter!("rank_name", |v| crate::economy::RANK_NAMES[v.rank.clamp(0, 3) as usize].to_string());
        getter!("difficulty", |v| v.difficulty as i64);
        getter!("sandbox", |v| v.sandbox);
        getter!("course_name", |v| v.course.clone());
        getter!("hole_count", |v| v.holes.len() as i64);
        getter!("holes", |v| v
            .holes
            .iter()
            .map(|h| {
                let mut m = Map::new();
                m.insert("number".into(), (h.number as i64).into());
                m.insert("par".into(), (h.par as i64).into());
                m.insert("yards".into(), (h.yards as i64).into());
                Dynamic::from(m)
            })
            .collect::<Array>());
        getter!("golfers", |v| v.golfers.iter().map(|g| Dynamic::from(golfer_map(g))).collect::<Array>());
        {
            let c = ctx.clone();
            e.register_fn("hole_par", move |n: i64| {
                c.borrow().view.holes.iter().find(|h| h.number as i64 == n).map(|h| h.par as i64).unwrap_or(0)
            });
        }
        // acting on it
        {
            let c = ctx.clone();
            e.register_fn("show_message", move |t: ImmutableString| c.borrow_mut().actions.push(Action::Message(t.to_string())));
            let c = ctx.clone();
            e.register_fn("add_money", move |amount: i64, reason: ImmutableString| {
                c.borrow_mut().actions.push(Action::Money(amount.clamp(-10_000_000, 10_000_000), reason.to_string()))
            });
            let c = ctx.clone();
            e.register_fn("set_goal", move |id: ImmutableString, text: ImmutableString| {
                c.borrow_mut().actions.push(Action::SetGoal { id: id.to_string(), text: text.to_string() })
            });
            let c = ctx.clone();
            e.register_fn("complete_goal", move |id: ImmutableString| {
                c.borrow_mut().actions.push(Action::FinishGoal { id: id.to_string(), done: true })
            });
            let c = ctx.clone();
            e.register_fn("fail_goal", move |id: ImmutableString| {
                c.borrow_mut().actions.push(Action::FinishGoal { id: id.to_string(), done: false })
            });
            let c = ctx.clone();
            e.register_fn("random", move |n: i64| {
                if n <= 0 {
                    return 0;
                }
                (next(&mut c.borrow_mut().rng) % n as u64) as i64
            });
        }
        Host { engine: e, ctx, packs: Vec::new() }
    }

    /// Compiles a pack's scripts (file name, text), in the order given. Returns the problems; a file that does not compile
    /// is left out, the others run. `seed` seeds the pack's generator.
    pub fn add_pack(&mut self, id: &str, files: &[(String, String)], seed: u64) -> Vec<String> {
        let mut errors = Vec::new();
        let mut asts = Vec::new();
        for (name, text) in files {
            match self.engine.compile(text) {
                Ok(ast) => asts.push((name.clone(), ast)),
                Err(e) => errors.push(format!("{name}: {e}")),
            }
        }
        if !asts.is_empty() {
            self.packs.retain(|p| p.id != id);
            self.packs.push(PackScripts { id: id.to_string(), files: asts, state: Map::new().into(), rng: seed_for(seed, id), errors: 0 });
        }
        errors
    }

    /// The packs with scripts, in calling order.
    pub fn pack_ids(&self) -> Vec<String> {
        self.packs.iter().map(|p| p.id.clone()).collect()
    }

    pub fn is_empty(&self) -> bool {
        self.packs.is_empty()
    }

    /// Calls a hook in every pack that defines it.
    pub fn call(&mut self, hook: &Hook, view: &GameView) -> Output {
        let mut out = Output::default();
        let name = hook.name();
        let args = hook.args();
        self.ctx.borrow_mut().view = view.clone();
        for p in self.packs.iter_mut().filter(|p| p.errors < MAX_ERRORS) {
            for (file, ast) in &p.files {
                if !ast.iter_functions().any(|f| f.name == name && f.params.len() == args.len()) {
                    continue;
                }
                {
                    let mut c = self.ctx.borrow_mut();
                    c.actions.clear();
                    c.log.clear();
                    c.rng = p.rng;
                }
                let mut scope = Scope::new();
                let opts = CallFnOptions::new().eval_ast(false).rewind_scope(true).bind_this_ptr(&mut p.state);
                let r = self.engine.call_fn_with_options::<Dynamic>(opts, &mut scope, ast, name, args.clone());
                let mut c = self.ctx.borrow_mut();
                p.rng = c.rng;
                out.log.extend(c.log.drain(..).map(|l| (p.id.clone(), l)));
                match r {
                    Ok(_) => out.actions.extend(c.actions.drain(..).map(|a| (p.id.clone(), a))),
                    Err(e) => {
                        // what a failed call asked for before failing is dropped with it
                        c.actions.clear();
                        p.errors += 1;
                        let mut msg = format!("{file}: {name}: {e}");
                        if p.errors >= MAX_ERRORS {
                            msg.push_str(&format!(" (stopped after {MAX_ERRORS} errors)"));
                        }
                        out.errors.push((p.id.clone(), msg));
                    }
                }
                if !p.state.is_map() {
                    // `this` must stay a map (a script that assigns something else to it gets an empty one)
                    p.state = Map::new().into();
                }
                if p.errors >= MAX_ERRORS {
                    break;
                }
            }
        }
        out
    }

    /// The scripts' state, for the saved game.
    pub fn save(&self) -> BTreeMap<String, PackState> {
        self.packs
            .iter()
            .map(|p| {
                let state = rhai::serde::from_dynamic::<serde_json::Value>(&p.state).unwrap_or(serde_json::Value::Null);
                (p.id.clone(), PackState { state, rng: p.rng })
            })
            .collect()
    }

    /// Puts back the state of a saved game (packs not in it keep a fresh state). Returns the problems.
    pub fn restore(&mut self, saved: &BTreeMap<String, PackState>) -> Vec<(String, String)> {
        let mut errors = Vec::new();
        for p in self.packs.iter_mut() {
            let Some(s) = saved.get(&p.id) else { continue };
            if s.rng != 0 {
                p.rng = s.rng;
            }
            match rhai::serde::to_dynamic(&s.state) {
                Ok(d) if d.is_map() => p.state = d,
                Ok(d) if d.is_unit() => {}
                Ok(_) => errors.push((p.id.clone(), "the saved script state is not a map".to_string())),
                Err(e) => errors.push((p.id.clone(), format!("the saved script state: {e}"))),
            }
        }
        errors
    }
}

/// The script files of a pack: every `scripts/*.rhai` (by name), as (name, text).
pub fn pack_files(source: &crate::vfs::Source) -> Vec<(String, String)> {
    let mut v: Vec<(String, String)> = source
        .files()
        .into_iter()
        .filter(|f| {
            let low = f.to_lowercase();
            low.starts_with("scripts/") && low.ends_with(".rhai") && low.matches('/').count() == 1
        })
        .filter_map(|f| source.read(&f).map(|d| (f, String::from_utf8_lossy(&d).into_owned())))
        .collect();
    v.sort_by_key(|(f, _)| f.to_lowercase());
    v
}

#[cfg(test)]
mod tests {
    use super::*;

    fn view() -> GameView {
        GameView {
            year: 2002,
            month: 4,
            cash: 12345,
            fun: 30,
            skill: 1250,
            members: 7,
            rank: 1,
            holes: vec![HoleView { number: 1, par: 3, yards: 150 }, HoleView { number: 2, par: 4, yards: 380 }],
            golfers: vec![GolferView { name: "Ann".into(), hole: 2, strokes: 1, member: true, pro: false }],
            ..Default::default()
        }
    }

    fn host(src: &str) -> Host {
        let mut h = Host::new();
        let e = h.add_pack("p", &[("scripts/a.rhai".into(), src.into())], 42);
        assert!(e.is_empty(), "{e:?}");
        h
    }

    #[test]
    fn hooks_read_the_game_and_act() {
        let mut h = host(
            r#"
            fn on_month() {
                if cash() > 10000 && fun_rating() == 30 && skill_rating() == 12.5 && hole_par(2) == 4 && golfers()[0].name == "Ann" {
                    show_message(`It is ${month()}/${year()}, ${members()} members, ${rank_name()}`);
                }
            }
            fn on_hole_finished(golfer, hole, strokes) {
                if strokes == 1 { add_money(500, "ace bonus for " + golfer.name); }
            }
            fn on_building_placed(kind, x, y) { set_goal("b", kind + " at " + x + "," + y); complete_goal("b"); }
        "#,
        );
        let out = h.call(&Hook::Month, &view());
        assert_eq!(out.actions, vec![("p".into(), Action::Message("It is 4/2002, 7 members, Daily Fee".into()))]);
        let g = GolferView { name: "Bo".into(), ..Default::default() };
        let out = h.call(&Hook::HoleFinished { golfer: g.clone(), hole: 3, strokes: 1 }, &view());
        assert_eq!(out.actions, vec![("p".into(), Action::Money(500, "ace bonus for Bo".into()))]);
        assert!(h.call(&Hook::HoleFinished { golfer: g, hole: 3, strokes: 4 }, &view()).actions.is_empty());
        let out = h.call(&Hook::BuildingPlaced { kind: "snack-bar".into(), x: 3, y: 4 }, &view());
        assert_eq!(out.actions.len(), 2);
        // a hook the scripts do not define does nothing
        assert_eq!(h.call(&Hook::YearEnd, &view()), Output::default());
    }

    #[test]
    fn state_is_kept_and_saved_with_its_generator() {
        let src = "fn on_month() { this.n = (this.n ?? 0) + 1; this.r = random(1000); this.list = [1, \"two\", #{three: 3}]; }";
        let mut h = host(src);
        h.call(&Hook::Month, &view());
        h.call(&Hook::Month, &view());
        let saved = h.save();
        assert_eq!(saved["p"].state["n"], 2);
        // a fresh host given the saved state goes on exactly the same way
        let mut a = host(src);
        a.restore(&saved);
        a.call(&Hook::Month, &view());
        h.call(&Hook::Month, &view());
        assert_eq!(a.save(), h.save());
        assert_eq!(a.save()["p"].state["n"], 3);
        // the same seed gives the same numbers; another seed others
        let mut b = Host::new();
        b.add_pack("p", &[("a".into(), src.into())], 43);
        b.call(&Hook::Month, &view());
        let mut c = host(src);
        c.call(&Hook::Month, &view());
        assert_ne!(b.save()["p"].rng, c.save()["p"].rng);
    }

    #[test]
    fn errors_and_limits_never_escape() {
        let mut h = Host::new();
        let e = h.add_pack("bad", &[("scripts/x.rhai".into(), "fn on_start( {".into())], 1);
        assert_eq!(e.len(), 1);
        assert!(h.is_empty());
        let e = h.add_pack("bad", &[("scripts/x.rhai".into(), "fn on_start() { eval(\"1\") }".into())], 1);
        assert!(e[0].contains("eval"), "{e:?}");
        let mut h = host(
            r#"
            fn on_start() { show_message("lost"); let x = 1 / 0; }
            fn on_month() { loop { } }
            fn on_year_end() { let s = "x"; loop { s += s; } }
            fn on_building_placed(k, x, y) { let a = []; a.pad(100000, 0); }
        "#,
        );
        let out = h.call(&Hook::Start, &view());
        assert!(out.actions.is_empty(), "a failed hook's actions are dropped");
        assert_eq!(out.errors.len(), 1);
        assert!(h.call(&Hook::Month, &view()).errors[0].1.to_lowercase().contains("operations"));
        assert_eq!(h.call(&Hook::YearEnd, &view()).errors.len(), 1);
        assert_eq!(h.call(&Hook::BuildingPlaced { kind: "x".into(), x: 0, y: 0 }, &view()).errors.len(), 1);
        // after ten errors the pack is stopped
        for _ in 0..10 {
            h.call(&Hook::Start, &view());
        }
        assert!(h.call(&Hook::Start, &view()).errors.is_empty());
    }

    #[test]
    fn the_example_script_pays_for_a_hole_in_one() {
        let m = crate::packs::Manifest::parse(include_str!("../../../packs/example-script-bonus/pack.toml")).unwrap();
        assert_eq!(m.kind, crate::packs::Kind::Mod);
        let mut h = Host::new();
        let src = include_str!("../../../packs/example-script-bonus/scripts/bonus.rhai");
        assert!(h.add_pack(&m.id, &[("scripts/bonus.rhai".into(), src.into())], 7).is_empty());
        let out = h.call(&Hook::Start, &view());
        assert!(out.errors.is_empty() && out.actions.len() == 1, "{out:?}");
        let g = GolferView { name: "Ann".into(), member: true, ..Default::default() };
        let out = h.call(&Hook::HoleFinished { golfer: g.clone(), hole: 2, strokes: 1 }, &view());
        assert!(out.errors.is_empty(), "{:?}", out.errors);
        assert!(out.actions.iter().any(|(_, a)| matches!(a, Action::Money(m, _) if *m > 0)));
        assert!(out.actions.iter().any(|(_, a)| matches!(a, Action::Message(t) if t.contains("Ann"))));
        let out = h.call(&Hook::HoleFinished { golfer: g, hole: 2, strokes: 3 }, &view());
        assert!(out.actions.is_empty());
        for hook in [Hook::Month, Hook::YearEnd] {
            assert!(h.call(&hook, &view()).errors.is_empty());
        }
        assert!(h.save()[&m.id].state["aces"] == 1);
    }
}
