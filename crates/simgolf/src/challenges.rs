//! Challenges and the design lab (the port's own; the interface extra `challenges`, extras.rs).
//!
//! **Challenges**: Information > Challenges lists six built-in aims, from one great hole to a business turnaround. A click
//! takes one on (it joins Information > Scenario Goals, owned by `sg_core::scenario::CHALLENGES`, so it never decides a
//! scenario) and another gives it up. Taken challenges are checked at each month start with the goals; the hole
//! challenges read Hole Stats' figures (fun factor, stroke average, rounds), so a hole must have been played before it
//! counts. Nothing is checked or shown with the extra off, and the goals stay in the save as they were.
//!
//! **Design lab**: Hole Stats gets a *Mark before* button. It keeps the hole's figures as they are now (fun, stroke
//! average, rounds, length, par, the date), and Hole Stats then shows them beside the figures since: rebuilding a hole
//! starts its figures again (holetool.rs), and a hole only played on has the rounds since the mark worked out from the
//! difference. Marks are kept with the save, by hole number (Course A's).
use crate::app::*;
use crate::extras::{draw_arrow, draw_button, wrap};
use crate::gfx::Gfx;
use crate::info_ui::c15;
use crate::render::Rect;
use crate::ui::{Fnt, Screen as Ui, F_INFO14, F_MANUAL20};
use serde::{Deserialize, Serialize};
use sg_core::golfer::Hole;
use sg_core::lang::{tr, trf};
use sg_core::ratings::{hole_stats, HoleStats};
use sg_core::scenario::{Conditions, Goal, GoalState, CHALLENGES};

const BOX: Rect = Rect { x: 40.0, y: 36.0, w: 720.0, h: 528.0 };
const TOP_Y: f32 = BOX.y + 50.0;
const BOTTOM_Y: f32 = BOX.y + BOX.h - 66.0;
const MARK_X: f32 = BOX.x + 24.0;
const TEXT_X: f32 = BOX.x + 96.0;
const TEXT_W: f32 = BOX.w - 150.0;
const UP: Rect = Rect { x: BOX.x + BOX.w - 44.0, y: TOP_Y, w: 20.0, h: 20.0 };
const DOWN: Rect = Rect { x: BOX.x + BOX.w - 44.0, y: BOTTOM_Y - 20.0, w: 20.0, h: 20.0 };

/// Rounds a hole needs before a hole challenge counts it.
pub const MIN_ROUNDS: i32 = 20;
/// Hole Stats' fun words: Good from 20%, Very good from 40%, Outstanding from 60% (ratings::fun_word).
const GOOD: i32 = 20;
const VERY_GOOD: i32 = 40;
const OUTSTANDING: i32 = 60;
/// The turnaround's aim: this much more cash than when it was taken, by the end of the next year.
pub const TURNAROUND_GAIN: i64 = 100_000;

/// The challenges, in the page's order: id (kept in the save) and the name and description keys.
pub const LIST: [(&str, &str, &str); 6] = [
    ("perfect-par-3", "STR_CHALLENGE_PAR3", "STR_CHALLENGE_PAR3_DESC"),
    ("signature-par-5", "STR_CHALLENGE_PAR5", "STR_CHALLENGE_PAR5_DESC"),
    ("tough-but-fair", "STR_CHALLENGE_TOUGH", "STR_CHALLENGE_TOUGH_DESC"),
    ("balanced-eighteen", "STR_CHALLENGE_BALANCED", "STR_CHALLENGE_BALANCED_DESC"),
    ("nine-hole-test", "STR_CHALLENGE_NINE", "STR_CHALLENGE_NINE_DESC"),
    ("turnaround", "STR_CHALLENGE_TURNAROUND", "STR_CHALLENGE_TURNAROUND_DESC"),
];

/// One open hole's figures, as the challenges read them.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct HoleFacts {
    pub hole: usize,
    pub par: i32,
    pub yards: i32,
    pub rounds: i32,
    pub fun: Option<i32>,
    /// Stroke average in hundredths.
    pub avg: Option<i32>,
}

impl HoleFacts {
    fn played(&self) -> bool {
        self.rounds >= MIN_ROUNDS
    }
    fn fun_at_least(&self, f: i32) -> bool {
        self.played() && self.fun.is_some_and(|v| v >= f)
    }
}

/// Whether hole challenge `id` is met by these holes (the turnaround is a cash goal, checked as goals are).
pub fn hole_challenge_met(id: &str, holes: &[HoleFacts]) -> bool {
    let count = |p: i32| holes.iter().filter(|h| h.par == p).count();
    match id {
        "perfect-par-3" => holes.iter().any(|h| h.par == 3 && h.fun_at_least(OUTSTANDING)),
        "signature-par-5" => {
            holes.iter().any(|h| h.par == 5 && h.yards >= 500 && h.fun_at_least(OUTSTANDING) && h.avg.is_some_and(|a| a <= 600))
        }
        "tough-but-fair" => holes.iter().any(|h| h.fun_at_least(VERY_GOOD) && h.avg.is_some_and(|a| a >= h.par * 100 + 100)),
        "balanced-eighteen" => {
            holes.len() == 18 && (4..=5).contains(&count(3)) && (4..=5).contains(&count(5)) && holes.iter().all(|h| h.fun_at_least(GOOD))
        }
        "nine-hole-test" => {
            let par: i32 = holes.iter().map(|h| h.par).sum();
            holes.len() >= 9 && holes.len() < 18 && (34..=37).contains(&par) && holes.iter().all(|h| h.fun_at_least(GOOD))
        }
        _ => false,
    }
}

/// A hole's figures kept by the design lab.
#[derive(Clone, Debug, Default, PartialEq, Serialize, Deserialize)]
pub struct LabMark {
    /// When it was marked (`App::date`).
    pub tick: u32,
    pub par: i32,
    pub yards: i32,
    /// The counters the figures come from (golfer::Hole's), so the rounds since can be told apart.
    pub tee_shots: i32,
    pub plans: i32,
    pub mood_sum: i32,
    pub hist: Vec<i32>,
}

impl LabMark {
    pub fn of(h: &Hole, tick: u32) -> LabMark {
        LabMark { tick, par: h.par, yards: h.length, tee_shots: h.tee_shots, plans: h.plans, mood_sum: h.mood_sum, hist: h.hist.clone() }
    }

    /// The hole as it was when marked (for its figures).
    fn hole(&self) -> Hole {
        Hole {
            par: self.par,
            length: self.yards,
            tee_shots: self.tee_shots,
            plans: self.plans,
            mood_sum: self.mood_sum,
            hist: self.hist.clone(),
            ..Default::default()
        }
    }

    /// The hole's play since the mark: the counters' differences, or the hole as it is when they went back (rebuilt).
    pub fn since(&self, now: &Hole) -> Hole {
        let reset = now.tee_shots < self.tee_shots
            || now.plans < self.plans
            || now.hist.iter().zip(&self.hist).any(|(n, b)| n < b)
            || now.par != self.par;
        if reset {
            return now.clone();
        }
        Hole {
            tee_shots: now.tee_shots - self.tee_shots,
            plans: now.plans - self.plans,
            mood_sum: now.mood_sum - self.mood_sum,
            hist: now.hist.iter().enumerate().map(|(i, n)| n - self.hist.get(i).copied().unwrap_or(0)).collect(),
            ..now.clone()
        }
    }
}

fn font() -> Fnt {
    crate::textsize::big(F_INFO14)
}
fn line() -> f32 {
    crate::textsize::px(16.0)
}
fn ok_tick() -> Rect {
    Rect::new(BOX.x + BOX.w - 70.0, BOX.y + BOX.h - 58.0, 50.0, 50.0)
}

/// "fun 45%, 3.85 over 20 rounds": a hole's figures for the design lab's line.
fn lab_figures(st: &HoleStats, rounds: i32) -> String {
    if rounds == 0 {
        return tr("STR_LAB_NO_ROUNDS").to_string();
    }
    let fun = st.fun.map(|f| format!("{f}%")).unwrap_or_else(|| "-".to_string());
    let avg = st.avg.map(crate::info_ui::hundredths).unwrap_or_else(|| "-".to_string());
    trf("STR_LAB_FIGURES", &[("FUN", &fun), ("AVG", &avg), ("ROUNDS", &rounds)])
}

impl App {
    /// The open holes of Course A with Hole Stats' figures.
    pub fn hole_facts(&self) -> Vec<HoleFacts> {
        (1..19)
            .filter(|&h| self.club.holes[h].par != 0)
            .map(|h| {
                let hole = &self.club.holes[h];
                let st = hole_stats(hole);
                HoleFacts { hole: h, par: hole.par, yards: hole.length, rounds: hole.tee_shots, fun: st.fun, avg: st.avg }
            })
            .collect()
    }

    fn challenge_goal(&self, id: &str) -> Option<&Goal> {
        self.scen.goals.list.iter().find(|g| g.owner == CHALLENGES && g.id == id)
    }

    /// The Information menu's line.
    pub fn open_challenges(&mut self) {
        self.screen = Screen::Challenges;
        self.xs.challenges_off = 0;
        self.hover = -1;
    }

    fn close_challenges(&mut self) {
        self.screen = Screen::Play;
        self.hover = -1;
    }

    /// Takes challenge `k` on, or gives it up when it is open (a finished one stays as it ended).
    pub fn toggle_challenge(&mut self, k: usize) {
        let Some(&(id, name, desc)) = LIST.get(k) else { return };
        match self.challenge_goal(id).map(|g| g.state) {
            Some(GoalState::Open) => {
                self.scen.goals.list.retain(|g| !(g.owner == CHALLENGES && g.id == id));
                self.ui_sound(0x0b);
            }
            Some(_) => {}
            None => {
                let f = crate::scenario_ui::facts(self);
                let (when, by_year, text) = if id == "turnaround" {
                    let target = f.cash + TURNAROUND_GAIN;
                    let when = Conditions { cash: Some(target), ..Default::default() };
                    let text = trf("STR_CHALLENGE_TURNAROUND_GOAL", &[("MONEY", &crate::ui::money(target)), ("YEAR", &(f.year + 1))]);
                    (when, Some(f.year + 1), text)
                } else {
                    (Conditions::default(), None, format!("{}: {}", tr(name), tr(desc)))
                };
                self.scen.goals.list.push(Goal {
                    id: id.to_string(),
                    text,
                    when,
                    by_year,
                    state: GoalState::Open,
                    owner: CHALLENGES.to_string(),
                });
                self.ui_sound(0x0b);
            }
        }
    }

    /// At each month start (scenario_ui.rs `check_goals`): the taken hole challenges that are met now.
    pub fn check_challenges(&mut self) -> Vec<sg_core::scenario::Note> {
        if !crate::extras::challenges() {
            return Vec::new();
        }
        let holes = self.hole_facts();
        let met: Vec<String> = self
            .scen
            .goals
            .list
            .iter()
            .filter(|g| g.owner == CHALLENGES && g.state == GoalState::Open && g.when.is_empty())
            .filter(|g| hole_challenge_met(&g.id, &holes))
            .map(|g| g.id.clone())
            .collect();
        met.iter().flat_map(|id| self.scen.goals.finish(CHALLENGES, id, true)).collect()
    }

    fn challenges_scroll(&mut self, d: i32) {
        self.xs.challenges_off = (self.xs.challenges_off as i32 + d).clamp(0, LIST.len() as i32 - 1) as usize;
    }

    pub fn challenges_wheel(&mut self, y: f32) {
        self.challenges_scroll(if y > 0.0 { -1 } else { 1 });
    }

    /// The rows as drawn: (challenge, top, height).
    fn challenge_rows(&self) -> Vec<(usize, f32, f32)> {
        let mut out = Vec::new();
        let mut y = TOP_Y;
        for k in self.xs.challenges_off..LIST.len() {
            let n = wrap(font(), &self.challenge_text(k), TEXT_W).len() as f32;
            let h = line() * n + 8.0;
            if y + h > BOTTOM_Y && !out.is_empty() {
                break;
            }
            out.push((k, y, h));
            y += h;
        }
        out
    }

    fn challenge_text(&self, k: usize) -> String {
        let (_, name, desc) = LIST[k];
        format!("{}: {}", tr(name), tr(desc))
    }

    pub fn challenges_click(&mut self, x: f32, y: f32, right: bool) {
        if right || ok_tick().has(x, y) || !BOX.has(x, y) {
            return self.close_challenges();
        }
        if UP.has(x, y) {
            return self.challenges_scroll(-1);
        }
        if DOWN.has(x, y) {
            return self.challenges_scroll(1);
        }
        if let Some(&(k, _, _)) = self.challenge_rows().iter().find(|&&(_, top, h)| x >= MARK_X && x < UP.x && y >= top && y < top + h) {
            self.toggle_challenge(k);
        }
    }

    pub fn challenges_key(&mut self, k: miniquad::KeyCode) {
        use miniquad::KeyCode;
        let n = match k {
            KeyCode::Key1 => 0,
            KeyCode::Key2 => 1,
            KeyCode::Key3 => 2,
            KeyCode::Key4 => 3,
            KeyCode::Key5 => 4,
            KeyCode::Key6 => 5,
            KeyCode::Up => return self.challenges_scroll(-1),
            KeyCode::Down => return self.challenges_scroll(1),
            KeyCode::Escape | KeyCode::Enter | KeyCode::KpEnter => return self.close_challenges(),
            _ => return,
        };
        self.toggle_challenge(n);
    }

    pub fn draw_challenges(&mut self, g: &mut Gfx) {
        let s = Ui::new(self.draw_w, self.draw_h);
        self.view = s.view;
        crate::info_ui::dim(&mut *g, &s);
        self.draw_info_frame(g, &s, BOX.x, BOX.y, BOX.w, BOX.h);
        let (white, yellow, navy, grey) = (c15(0x7fff), c15(0x7ff0), c15(0x0018), c15(0x6318));
        let (good, bad) = (c15(0x01e0), c15(0x5800));
        s.put_shadowed(g, F_MANUAL20, 400.0, BOX.y + 14.0, tr("STR_CHALLENGES_TITLE"), yellow, true);
        let (px, py) = self.info.pointer;
        let rows = self.challenge_rows();
        for &(k, y, h) in &rows {
            let (word, c) = match self.challenge_goal(LIST[k].0).map(|g| g.state) {
                Some(GoalState::Done) => (tr("STR_CHALLENGE_DONE"), good),
                Some(GoalState::Failed) => (tr("STR_CHALLENGE_MISSED"), bad),
                Some(GoalState::Open) => (tr("STR_CHALLENGE_TAKEN"), yellow),
                None => (tr("STR_CHALLENGE_TAKE"), navy),
            };
            let lit = px >= MARK_X && px < UP.x && py >= y && py < y + h;
            s.put(g, font(), MARK_X, y, word, c);
            for (i, l) in wrap(font(), &self.challenge_text(k), TEXT_W).iter().enumerate() {
                s.put(g, font(), TEXT_X, y + line() * i as f32, l, if lit { yellow } else { white });
            }
        }
        let (can_up, can_down) = (self.xs.challenges_off > 0, rows.last().is_some_and(|r| r.0 + 1 < LIST.len()));
        for (r, up, ok) in [(UP, true, can_up), (DOWN, false, can_down)] {
            draw_button(g, &s, r, "", ok, r.has(px, py));
            draw_arrow(g, &s, r.x + r.w / 2.0, r.y + r.h / 2.0, up, if ok { navy } else { grey });
        }
        let room = ok_tick().x - 6.0 - (BOX.x + 24.0);
        let help = tr("STR_CHALLENGES_HELP");
        s.put_fit(g, crate::textsize::fit(F_INFO14, 1.25, help, room), BOX.x + 24.0, BOX.y + BOX.h - 28.0, room, help, navy);
        if self.art.trans.tex.is_some() {
            let r = ok_tick();
            s.image_part(g, &self.art.trans, r.x, r.y, 350.0, 140.0, 50.0, 50.0);
        } else {
            draw_button(g, &s, ok_tick(), "OK", true, ok_tick().has(px, py));
        }
        g.flush();
    }

    // ---- the design lab ---------------------------------------------------------------------------------------------------

    /// Hole Stats' Mark before button: with the extra, for an open hole of Course A.
    pub fn lab_button(&self) -> Option<Rect> {
        let h = self.info.stats_hole.clamp(1, 18);
        (crate::extras::challenges() && !self.hole_b() && self.club.holes[h].par != 0).then(|| Rect::new(652.0, 62.0, 96.0, 22.0))
    }

    /// Keeps the hole's figures as they are now.
    pub fn lab_mark(&mut self) {
        let h = self.info.stats_hole.clamp(1, 18);
        let m = LabMark::of(&self.club.holes[h], self.date());
        self.xs.kept.lab.insert(h as u8, m);
        self.ui_sound(0x0b);
    }

    /// Hole Stats' lines for hole `h`: its figures when marked, and since.
    pub fn lab_lines(&self, h: usize) -> Vec<String> {
        if !crate::extras::challenges() || self.hole_b() {
            return Vec::new();
        }
        let Some(m) = self.xs.kept.lab.get(&(h as u8)) else { return Vec::new() };
        let before = m.hole();
        let since = m.since(&self.club.holes[h]);
        vec![
            trf(
                "STR_LAB_BEFORE",
                &[
                    ("DATE", &crate::extras::date(m.tick)),
                    ("YARDS", &m.yards),
                    ("PAR", &m.par),
                    ("FIGURES", &lab_figures(&hole_stats(&before), before.tee_shots)),
                ],
            ),
            trf(
                "STR_LAB_SINCE",
                &[("YARDS", &since.length), ("PAR", &since.par), ("FIGURES", &lab_figures(&hole_stats(&since), since.tee_shots))],
            ),
        ]
    }

    /// Draws the Mark before button, as the Fly over button is drawn.
    pub fn draw_lab_button(&self, g: &mut Gfx, s: &Ui) {
        let Some(r) = self.lab_button() else { return };
        let (px, py) = self.info.pointer;
        let lit = r.has(px, py);
        s.fill(g, r.x - 1.0, r.y - 1.0, r.w + 2.0, r.h + 2.0, c15(0x0000));
        s.fill(g, r.x, r.y, r.w, r.h, c15(if lit { 0x5ade } else { 0x3dd4 }));
        s.text_centered(g, r.x + r.w * 0.5, r.y + 16.0, tr("STR_LAB_MARK"), 14.0, c15(if lit { 0x7fff } else { 0x0000 }));
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn h(hole: usize, par: i32, rounds: i32, fun: i32, avg: i32) -> HoleFacts {
        HoleFacts { hole, par, yards: par * 120, rounds, fun: Some(fun), avg: Some(avg) }
    }

    #[test]
    fn a_hole_challenge_needs_the_rounds_and_the_rating() {
        assert!(hole_challenge_met("perfect-par-3", &[h(1, 3, 20, 60, 310)]));
        assert!(!hole_challenge_met("perfect-par-3", &[h(1, 3, 19, 80, 310)]), "too few rounds");
        assert!(!hole_challenge_met("perfect-par-3", &[h(1, 4, 40, 80, 410)]), "not a par 3");
        assert!(hole_challenge_met("tough-but-fair", &[h(2, 4, 30, 45, 500)]));
        assert!(!hole_challenge_met("tough-but-fair", &[h(2, 4, 30, 45, 499)]), "less than a stroke over par");
        let mut long = h(3, 5, 25, 61, 560);
        long.yards = 480;
        assert!(!hole_challenge_met("signature-par-5", &[long]), "under 500 yards");
        long.yards = 520;
        assert!(hole_challenge_met("signature-par-5", &[long]));
    }

    #[test]
    fn the_course_challenges_count_the_pars() {
        let pars = [4, 3, 5, 4, 4, 3, 4, 5, 4, 4, 3, 5, 4, 4, 3, 4, 5, 4];
        let course: Vec<HoleFacts> = pars.iter().enumerate().map(|(i, &p)| h(i + 1, p, 30, 25, p * 100 + 30)).collect();
        assert!(hole_challenge_met("balanced-eighteen", &course));
        let mut dull = course.clone();
        dull[7].fun = Some(10);
        assert!(!hole_challenge_met("balanced-eighteen", &dull));
        let nine = &course[..9];
        assert_eq!(nine.iter().map(|h| h.par).sum::<i32>(), 36);
        assert!(hole_challenge_met("nine-hole-test", nine));
        assert!(!hole_challenge_met("nine-hole-test", &course), "eighteen holes are not a nine");
    }

    #[test]
    fn the_lab_works_out_the_rounds_since_or_starts_again_after_a_rebuild() {
        let mut hole = Hole { par: 4, length: 400, tee_shots: 30, plans: 20, mood_sum: 10, hist: vec![1; 88], ..Default::default() };
        let m = LabMark::of(&hole, 5);
        hole.tee_shots = 42;
        hole.hist[7 * 11 + 4] += 12;
        let since = m.since(&hole);
        assert_eq!((since.tee_shots, since.hist[7 * 11 + 4], since.hist[0]), (12, 12, 0));
        hole.tee_shots = 3;
        hole.hist = vec![0; 88];
        assert_eq!(m.since(&hole).tee_shots, 3, "a rebuilt hole's figures are its own");
    }
}
