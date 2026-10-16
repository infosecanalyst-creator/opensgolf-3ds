//! OpenSGolf extras: history graphs (extras.rs). From the moment the extra is switched on, the game keeps a record of
//! every game month (by the date, so with the Slower seasons extra a month is still one month of the calendar): for the
//! club the rounds finished, the holes played, the money taken and spent (the F4 Financial Report's ledger, month by
//! month), the cash at the month's end, the golfers' happiness (their mood as each hole ends), the course's fun and skill
//! ratings and the members; for each hole the times it was played, the strokes over par, the golfers' happiness after it
//! and the greens fees it took. The last `MAX_MONTHS` months are kept with the game (only while the extra is on: a save
//! made with it off has nothing of it and is unchanged).
//!
//! The Graphs page (Shift+Y, the Information menu, or a tab after the others on the tabbed reports) draws one measure of
//! the club or of a hole as a line over the months on the F3 Histograph's page, in its fonts and colours, with the month
//! under the pointer (or the keyboard's cursor, or a tapped month) and its exact value in a box.
use crate::app::*;
use crate::extras::draw_tab;
use crate::gfx::Gfx;
use crate::info_ui::c15;
use crate::render::Rect;
use crate::ui::{rgb, rgba, Screen as Ui, F_ARIAL10, F_INFO14, F_INFO_TITLE};
use serde::{Deserialize, Serialize};
use sg_core::economy::LEDGER_COLUMNS;
use sg_core::lang::{tr, trf};
use std::collections::{BTreeMap, VecDeque};

/// The months kept: ten game years of eight months.
pub const MAX_MONTHS: usize = 80;

/// A hole's month: times played, strokes over par (summed), the golfers' mood as they finished it (summed) and the greens
/// fees it took (units of $100).
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, Serialize, Deserialize)]
pub struct HoleMonth {
    #[serde(default)]
    pub plays: u32,
    #[serde(default)]
    pub over: i32,
    #[serde(default)]
    pub mood: i64,
    #[serde(default)]
    pub fees: i64,
}

/// A month of the club (`month` the date's month count, date >> 10). Rounds, holes played and moods are counted as they
/// happen; the money, cash, ratings and members are taken when the month ends (for the month in progress: now).
#[derive(Clone, Debug, Default, PartialEq, Serialize, Deserialize)]
#[serde(default)]
pub struct Month {
    pub month: u32,
    pub rounds: u32,
    pub plays: u32,
    pub mood: i64,
    /// Dollars taken and spent in the month (the ledger's columns' rises and falls).
    pub revenue: i64,
    pub costs: i64,
    pub cash: i64,
    pub fun: i32,
    /// The skill rating in hundredths of a stroke.
    pub skill: i32,
    pub members: i32,
    #[serde(skip_serializing_if = "BTreeMap::is_empty")]
    pub holes: BTreeMap<u8, HoleMonth>,
}

/// The club as a month ends: what the month takes from it.
#[derive(Clone, Copy, Debug, Default, PartialEq)]
pub struct Snapshot {
    pub cash: i64,
    pub fun: i32,
    pub skill: i32,
    pub members: i32,
    /// The ledger's columns summed over every year (dollars).
    pub ledger: [f64; LEDGER_COLUMNS],
}

impl Month {
    /// The month closed (or seen so far) with the club as `s`, from the ledger as it stood at the month's start.
    fn finish(&mut self, base: &[f64; LEDGER_COLUMNS], s: &Snapshot) {
        let (mut rev, mut cost) = (0.0, 0.0);
        for (now, then) in s.ledger.iter().zip(base) {
            let d = now - then;
            if d > 0.0 {
                rev += d;
            } else {
                cost -= d;
            }
        }
        self.revenue = rev.round() as i64;
        self.costs = cost.round() as i64;
        self.cash = s.cash;
        self.fun = s.fun;
        self.skill = s.skill;
        self.members = s.members;
    }
}

/// What the graphs keep with the game: the months finished (oldest first), the month in progress and the ledger as it
/// stood when that month began.
#[derive(Clone, Debug, Default, PartialEq, Serialize, Deserialize)]
pub struct Kept {
    #[serde(default, skip_serializing_if = "VecDeque::is_empty")]
    pub months: VecDeque<Month>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub now: Option<Month>,
    #[serde(default)]
    pub base: [f64; LEDGER_COLUMNS],
}

impl Kept {
    pub fn is_empty(&self) -> bool {
        self.months.is_empty() && self.now.is_none()
    }

    /// The month in progress is `month`: a month that has ended is closed with the club as `snap` and kept (the oldest
    /// going past `MAX_MONTHS`), and the new one starts from the ledger as it is. Going back (a game loaded from earlier)
    /// starts afresh from there.
    pub fn roll(&mut self, month: u32, snap: impl FnOnce() -> Snapshot) {
        if self.now.as_ref().is_some_and(|m| m.month == month) {
            return;
        }
        let s = snap();
        if let Some(mut m) = self.now.take() {
            if month > m.month {
                m.finish(&self.base, &s);
                self.months.push_back(m);
            }
        }
        self.months.retain(|m| m.month < month);
        while self.months.len() > MAX_MONTHS {
            self.months.pop_front();
        }
        self.base = s.ledger;
        self.now = Some(Month { month, ..Default::default() });
    }

    /// A golfer finished hole `hole` (par `par`) in `strokes`, in mood `mood`, paying `fee` units; `last` when it ended
    /// the round.
    #[allow(clippy::too_many_arguments)]
    pub fn hole_done(
        &mut self,
        month: u32,
        snap: impl FnOnce() -> Snapshot,
        hole: u8,
        strokes: i32,
        par: i32,
        mood: i32,
        fee: i32,
        last: bool,
    ) {
        self.roll(month, snap);
        let Some(m) = self.now.as_mut() else { return };
        m.plays += 1;
        m.rounds += last as u32;
        m.mood += mood as i64;
        let h = m.holes.entry(hole).or_default();
        h.plays += 1;
        h.over += strokes - par;
        h.mood += mood as i64;
        h.fees += fee as i64;
    }

    /// The months to draw: the finished ones and the one in progress as it stands with the club as `s`.
    pub fn points(&self, s: &Snapshot) -> Vec<Month> {
        let mut v: Vec<Month> = self.months.iter().cloned().collect();
        if let Some(m) = &self.now {
            let mut m = m.clone();
            m.finish(&self.base, s);
            v.push(m);
        }
        v
    }
}

// ---- the measures ---------------------------------------------------------------------------------------------------------

/// How a measure's values print.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
enum Unit {
    Count,
    Money,
    /// One decimal (moods, strokes over par).
    Tenths,
    /// A rating in hundredths.
    Hundredths,
}

/// A measure: its name (text key), how it prints, its line's colour (the F3 Histograph's: skill purple, cash black, fun
/// green, events teal; the F4 report's dark red for money going out) and its value in a month for the club or a hole.
struct Measure {
    key: &'static str,
    unit: Unit,
    colour: u32,
    value: fn(&Month, u8) -> Option<f64>,
}

fn avg(sum: i64, n: u32) -> Option<f64> {
    (n > 0).then(|| sum as f64 / n as f64)
}

const CLUB: [Measure; 10] = [
    Measure { key: "STR_GRAPHS_ROUNDS", unit: Unit::Count, colour: 0x0210, value: |m, _| Some(m.rounds as f64) },
    Measure { key: "STR_GRAPHS_PLAYS", unit: Unit::Count, colour: 0x0210, value: |m, _| Some(m.plays as f64) },
    Measure { key: "STR_GRAPHS_REVENUE", unit: Unit::Money, colour: 0x0000, value: |m, _| Some(m.revenue as f64) },
    Measure { key: "STR_GRAPHS_COSTS", unit: Unit::Money, colour: 0x6000, value: |m, _| Some(m.costs as f64) },
    Measure { key: "STR_GRAPHS_PROFIT", unit: Unit::Money, colour: 0x0000, value: |m, _| Some((m.revenue - m.costs) as f64) },
    Measure { key: "STR_GRAPHS_CASH", unit: Unit::Money, colour: 0x0000, value: |m, _| Some(m.cash as f64) },
    Measure { key: "STR_GRAPHS_HAPPINESS", unit: Unit::Tenths, colour: 0x03e0, value: |m, _| avg(m.mood, m.plays) },
    Measure { key: "STR_GRAPHS_FUN", unit: Unit::Count, colour: 0x03e0, value: |m, _| Some(m.fun as f64) },
    Measure { key: "STR_GRAPHS_SKILL", unit: Unit::Hundredths, colour: 0x4010, value: |m, _| Some(m.skill as f64) },
    Measure { key: "STR_GRAPHS_MEMBERS", unit: Unit::Count, colour: 0x4010, value: |m, _| Some(m.members as f64) },
];

fn hole(m: &Month, h: u8) -> HoleMonth {
    m.holes.get(&h).copied().unwrap_or_default()
}

const HOLE: [Measure; 4] = [
    Measure { key: "STR_GRAPHS_HOLE_PLAYS", unit: Unit::Count, colour: 0x0210, value: |m, h| Some(hole(m, h).plays as f64) },
    Measure { key: "STR_GRAPHS_OVER_PAR", unit: Unit::Tenths, colour: 0x4010, value: |m, h| avg(hole(m, h).over as i64, hole(m, h).plays) },
    Measure { key: "STR_GRAPHS_HAPPINESS", unit: Unit::Tenths, colour: 0x03e0, value: |m, h| avg(hole(m, h).mood, hole(m, h).plays) },
    Measure { key: "STR_GRAPHS_HOLE_FEES", unit: Unit::Money, colour: 0x0000, value: |m, h| Some(hole(m, h).fees as f64 * 100.0) },
];

fn measures(series: usize) -> &'static [Measure] {
    if series == 0 {
        &CLUB
    } else {
        &HOLE
    }
}

fn print(unit: Unit, v: f64) -> String {
    match unit {
        Unit::Count => crate::screens_ui::digits(v.round() as i64),
        Unit::Money => format!("\u{a7}{}", crate::screens_ui::digits(v.round() as i64)),
        Unit::Tenths => format!("{:.1}", v),
        Unit::Hundredths => crate::info_ui::hundredths(v.round() as i32),
    }
}

/// A scale for values from `lo` to `hi`: the bottom, the step and the number of steps, the step a round 1, 2, 2.5 or 5
/// times a power of ten (whole numbers for counts) so that about five steps span the values.
pub fn scale(lo: f64, hi: f64, whole: bool) -> (f64, f64, usize) {
    let (lo, hi) = if hi - lo < 1e-9 { (lo - 1.0, hi + 1.0) } else { (lo, hi) };
    let raw = (hi - lo) / 5.0;
    let p = 10f64.powf(raw.log10().floor());
    let mut step = [1.0, 2.0, 2.5, 5.0, 10.0].iter().map(|k| k * p).find(|&s| s >= raw * 0.999).unwrap_or(10.0 * p);
    if whole {
        step = step.max(1.0).round();
    }
    let bottom = (lo / step).floor() * step;
    let n = (((hi - bottom) / step).ceil() as usize).max(1);
    (bottom, step, n)
}

// ---- the page's layout (the F3 Histograph's page, 0x455ed0's plot box) -------------------------------------------------------

/// The plot inside the art's ruled paper (about 82 to 718 by 48 to 522), the scale's figures to its left.
const PLOT: Rect = Rect { x: 150.0, y: 116.0, w: 550.0, h: 354.0 };
/// The months' names under the plot, the keys' line under them and the histograph's legend row (its labels' tops at 542,
/// 0x455ed0): fixed rows, whatever the pointer is over, so the keys never run into the legend.
const MONTHS_Y: f32 = PLOT.y + PLOT.h + 8.0;
const HELP_Y: f32 = PLOT.y + PLOT.h + 26.0;
const LEGEND_Y: f32 = 542.0;
// a line of Arial 10 is 12 high; the keys keep clear of the months above and the legend's art row below
const _: () = assert!(MONTHS_Y + 12.0 <= HELP_Y && HELP_Y + 12.0 <= LEGEND_Y - 8.0);
const SERIES_Y: f32 = 52.0;
const MEASURE_Y: f32 = 74.0;
/// The tabs' row on the paper.
const ROW_X: f32 = 92.0;
const ROW_W: f32 = 620.0;
const TAB_H: f32 = 16.0;
const OK: (f32, f32) = (704.0, 551.0);

fn series_tab(i: usize) -> Rect {
    if i == 0 {
        Rect::new(ROW_X, SERIES_Y, 56.0, TAB_H)
    } else {
        Rect::new(ROW_X + 62.0 + 31.0 * (i - 1) as f32, SERIES_Y, 29.0, TAB_H)
    }
}

fn measure_tab(series: usize, i: usize) -> Rect {
    let n = measures(series).len() as f32;
    let w = (ROW_W / n).min(130.0);
    Rect::new(ROW_X + ROW_W / 2.0 - w * n / 2.0 + w * i as f32 + 1.0, MEASURE_Y, w - 2.0, TAB_H)
}

/// The page's state in the game (not saved): the club (0) or a hole, the measure, and the keyboard's or a tap's month.
#[derive(Default)]
pub struct State {
    pub series: usize,
    pub measure: usize,
    pub cursor: Option<usize>,
}

impl App {
    /// The club now, for the month in progress and for a month that ends.
    pub fn graphs_snapshot(&self) -> Snapshot {
        let mut ledger = [0.0; LEDGER_COLUMNS];
        for row in &self.econ.ledger {
            for (s, v) in ledger.iter_mut().zip(row) {
                *s += v;
            }
        }
        Snapshot {
            cash: self.econ.cash.round() as i64,
            fun: self.club.ratings.fun,
            skill: self.club.ratings.skill,
            members: self.club.member_count(),
            ledger,
        }
    }

    /// After each club tick (the extra on): the month moves on with the date.
    pub fn graphs_tick(&mut self) {
        let month = self.date() >> 10;
        if self.xs.kept.graphs.as_ref().and_then(|k| k.now.as_ref()).is_some_and(|m| m.month == month) {
            return;
        }
        let snap = self.graphs_snapshot();
        self.xs.kept.graphs.get_or_insert_with(Default::default).roll(month, || snap);
    }

    /// A golfer finished a hole (golf::Event::HoleDone; the extra on).
    pub fn graphs_hole_done(&mut self, hole: i32, strokes: i32, mood: i32, fee: i32) {
        if !(1..19).contains(&hole) {
            return;
        }
        let h = hole as usize;
        let par = self.club.holes[h].par;
        // the round ends on the last hole (golfer.rs: hole 18, or the next one not open)
        let last = h == 18 || self.club.holes[h + 1].par == 0;
        let month = self.date() >> 10;
        let snap = self.graphs_snapshot();
        let k = self.xs.kept.graphs.get_or_insert_with(Default::default);
        k.hole_done(month, || snap, hole as u8, strokes, par, mood, fee, last);
    }

    /// Shift+Y, the Information menu's line or the reports' last tab: the Graphs page (closing to the course).
    pub fn open_graphs(&mut self) {
        self.screen = Screen::Graphs;
        self.xs.graphs.cursor = None;
        self.graphs_clamp();
        self.hover = -1;
    }

    /// The page's choices kept to the club and the open holes, and to the series' measures.
    pub fn graphs_clamp(&mut self) {
        if self.xs.graphs.series != 0 && self.club.holes[self.xs.graphs.series.min(18)].par == 0 {
            self.xs.graphs.series = 0;
        }
        self.xs.graphs.measure = self.xs.graphs.measure.min(measures(self.xs.graphs.series).len() - 1);
    }

    fn close_graphs(&mut self) {
        self.screen = Screen::Play;
        self.hover = -1;
    }

    fn graph_points(&self) -> Vec<Month> {
        self.xs.kept.graphs.as_ref().map(|k| k.points(&self.graphs_snapshot())).unwrap_or_default()
    }

    /// The x of month index `i` of `n` on the plot.
    fn graph_x(i: usize, n: usize) -> f32 {
        if n <= 1 {
            PLOT.x + PLOT.w / 2.0
        } else {
            PLOT.x + 10.0 + (PLOT.w - 20.0) * i as f32 / (n - 1) as f32
        }
    }

    /// The month under the pointer on the plot.
    fn graph_month_at(x: f32, y: f32, n: usize) -> Option<usize> {
        if n == 0 || !Rect::new(PLOT.x - 8.0, PLOT.y, PLOT.w + 16.0, PLOT.h + 24.0).has(x, y) {
            return None;
        }
        (0..n).min_by(|&a, &b| (Self::graph_x(a, n) - x).abs().total_cmp(&(Self::graph_x(b, n) - x).abs()))
    }

    fn set_series(&mut self, s: usize) {
        let st = &mut self.xs.graphs;
        // the measure keeps its meaning where both have it (happiness, plays, revenue), else the first
        let key = measures(st.series).get(st.measure).map(|m| m.key);
        st.series = s;
        st.measure = measures(s).iter().position(|m| Some(m.key) == key).unwrap_or(0);
    }

    /// The series' tabs that can be chosen: the club and the open holes.
    fn series_open(&self, i: usize) -> bool {
        i == 0 || self.club.holes[i.min(18)].par != 0
    }

    fn step_series(&mut self, d: i32) {
        let mut s = self.xs.graphs.series as i32;
        for _ in 0..19 {
            s = (s + d).rem_euclid(19);
            if self.series_open(s as usize) {
                break;
            }
        }
        self.set_series(s as usize);
    }

    fn step_measure(&mut self, d: i32) {
        let n = measures(self.xs.graphs.series).len() as i32;
        self.xs.graphs.measure = (self.xs.graphs.measure as i32 + d).rem_euclid(n) as usize;
    }

    pub fn graphs_wheel(&mut self, y: f32) {
        self.step_measure(if y > 0.0 { -1 } else { 1 });
    }

    pub fn graphs_click(&mut self, x: f32, y: f32, right: bool) {
        let page = Rect::new(32.0, 6.0, 736.0, 590.0);
        if right || !page.has(x, y) || (x >= OK.0 && x < OK.0 + 44.0 && y >= OK.1 && y < OK.1 + 44.0) {
            return self.close_graphs();
        }
        if let Some(i) = (0..19).find(|&i| series_tab(i).has(x, y)) {
            if self.series_open(i) {
                self.set_series(i);
                self.snd("Interface/Button1.wav", 1.0, false);
            }
            return;
        }
        let series = self.xs.graphs.series;
        if let Some(i) = (0..measures(series).len()).find(|&i| measure_tab(series, i).has(x, y)) {
            self.xs.graphs.measure = i;
            self.snd("Interface/Button1.wav", 1.0, false);
            return;
        }
        // a click or a tap on the plot holds that month's values up (a finger has no pointer to hover with)
        let n = self.graph_points().len();
        if let Some(i) = Self::graph_month_at(x, y, n) {
            self.xs.graphs.cursor = Some(i);
        }
    }

    pub fn graphs_key(&mut self, k: miniquad::KeyCode) {
        use miniquad::KeyCode;
        let n = self.graph_points().len();
        let cur = &mut self.xs.graphs.cursor;
        match k {
            KeyCode::Left if n > 0 => *cur = Some(cur.map_or(n - 1, |c| c.saturating_sub(1))),
            KeyCode::Right if n > 0 => *cur = Some(cur.map_or(n - 1, |c| (c + 1).min(n - 1))),
            KeyCode::Home if n > 0 => *cur = Some(0),
            KeyCode::End if n > 0 => *cur = Some(n - 1),
            KeyCode::Up => self.step_measure(-1),
            KeyCode::Down => self.step_measure(1),
            KeyCode::PageUp => self.step_series(-1),
            KeyCode::PageDown => self.step_series(1),
            KeyCode::Y if self.shift_held => self.close_graphs(),
            KeyCode::Escape | KeyCode::Enter | KeyCode::KpEnter => self.close_graphs(),
            _ => {}
        }
    }

    /// The month's name and year as the axis and the box print them ("June 2002").
    fn month_label(month: u32) -> String {
        trf("STR_GRAPHS_MONTH", &[("MONTH", &tr(crate::hud_ui::MONTH_KEYS[(month & 7) as usize])), ("YEAR", &(2001 + (month >> 3)))])
    }

    pub fn draw_graphs(&mut self, g: &mut Gfx) {
        let s = Ui::new(self.draw_w, self.draw_h);
        self.view = s.view;
        crate::info_ui::dim(&mut *g, &s);
        // the F3 Histograph's page (the art), its title font and its colours
        if self.reports.histo.tex.is_some() {
            s.image(g, &self.reports.histo, 0.0, 0.0);
        } else {
            s.fill(g, 32.0, 6.0, 736.0, 590.0, rgb(0.95, 0.95, 0.9));
        }
        let black = c15(0);
        let grey = c15(0x4210);
        s.put_centered(g, F_INFO_TITLE, 400.0, 20.0, tr("STR_GRAPHS_TITLE"), black);
        let (px, py) = self.info.pointer;
        let st = &self.xs.graphs;
        let (series, measure) = (st.series, st.measure);
        for i in 0..19 {
            let r = series_tab(i);
            let label = if i == 0 { tr("STR_GRAPHS_CLUB").to_string() } else { i.to_string() };
            if self.series_open(i) {
                draw_tab(g, &s, r, F_ARIAL10, &label, series == i, r.has(px, py));
            } else {
                s.fill(g, r.x, r.y, r.w, r.h, rgba(0.0, 0.0, 0.0, 0.08));
                s.put_centered(g, F_ARIAL10, r.x + r.w / 2.0, r.y + 3.0, &label, grey);
            }
        }
        let ms = measures(series);
        for (i, m) in ms.iter().enumerate() {
            let r = measure_tab(series, i);
            draw_tab(g, &s, r, F_ARIAL10, tr(m.key), measure == i, r.has(px, py));
        }
        let m = &ms[measure.min(ms.len() - 1)];
        let hole_no = series as u8;
        let points = self.graph_points();
        let vals: Vec<Option<f64>> = points.iter().map(|p| (m.value)(p, hole_no)).collect();
        // the plot's box: the histograph's light rule lines
        let rule = rgba(0.0, 0.0, 0.0, 0.12);
        s.fill(g, PLOT.x, PLOT.y + PLOT.h, PLOT.w, 1.0, rgba(0.0, 0.0, 0.0, 0.5));
        s.fill(g, PLOT.x, PLOT.y, 1.0, PLOT.h, rgba(0.0, 0.0, 0.0, 0.5));
        let title =
            if series == 0 { tr(m.key).to_string() } else { trf("STR_GRAPHS_HOLE_TITLE", &[("HOLE", &series), ("MEASURE", &tr(m.key))]) };
        s.put(g, F_INFO14, PLOT.x, PLOT.y - 20.0, &title, c15(m.colour));
        let known: Vec<f64> = vals.iter().flatten().copied().collect();
        if known.is_empty() {
            s.put_centered(g, F_INFO14, PLOT.x + PLOT.w / 2.0, PLOT.y + PLOT.h / 2.0 - 7.0, tr("STR_GRAPHS_EMPTY"), black);
        } else {
            let lo = known.iter().copied().fold(f64::MAX, f64::min);
            let hi = known.iter().copied().fold(f64::MIN, f64::max);
            // counts and money from zero; moods, strokes and ratings from their own lowest
            let from_zero = matches!(m.unit, Unit::Count | Unit::Money);
            let (lo, hi) = if from_zero { (lo.min(0.0), hi.max(0.0)) } else { (lo, hi) };
            let whole = m.unit == Unit::Count;
            let (bottom, step, steps) = scale(lo, hi, whole);
            let top = bottom + step * steps as f64;
            let y_of = |v: f64| PLOT.y + PLOT.h - ((v - bottom) / (top - bottom)) as f32 * PLOT.h;
            for k in 0..=steps {
                let v = bottom + step * k as f64;
                let y = y_of(v).round();
                if k > 0 {
                    s.fill(g, PLOT.x + 1.0, y, PLOT.w - 1.0, 1.0, if v.abs() < 1e-9 { rgba(0.0, 0.0, 0.0, 0.45) } else { rule });
                }
                s.put_right(g, F_INFO14, PLOT.x - 6.0, y - 7.0, &print(m.unit, v), c15(m.colour));
            }
            if bottom < 0.0 && top > 0.0 {
                s.fill(g, PLOT.x + 1.0, y_of(0.0).round(), PLOT.w - 1.0, 1.0, rgba(0.0, 0.0, 0.0, 0.45));
            }
            // the months along the bottom: each one's name while they have room, else the start of each year
            let n = points.len();
            let gap = if n > 1 { (PLOT.w - 20.0) / (n - 1) as f32 } else { PLOT.w };
            let mut right_edge = f32::MIN;
            for (i, p) in points.iter().enumerate() {
                let x = Self::graph_x(i, n);
                let year_start = p.month & 7 == 0;
                s.fill(g, x, PLOT.y + PLOT.h, 1.0, if year_start { 6.0 } else { 3.0 }, rgba(0.0, 0.0, 0.0, 0.5));
                if !(gap >= 72.0 || n == 1 || year_start || i == 0) {
                    continue;
                }
                let label = Self::month_label(p.month);
                let w = F_ARIAL10.width(&label);
                let lx = (x - w / 2.0).clamp(PLOT.x - 20.0, PLOT.x + PLOT.w + 20.0 - w);
                if lx > right_edge + 6.0 {
                    s.put(g, F_ARIAL10, lx, MONTHS_Y, &label, black);
                    right_edge = lx + w;
                }
            }
            // the line: two pixels in the measure's colour, a gap where a month has no value; the month in progress paler
            let col = c15(m.colour);
            let mut prev: Option<(f32, f32)> = None;
            for (i, v) in vals.iter().enumerate() {
                let Some(v) = v else {
                    prev = None;
                    continue;
                };
                let p = (Self::graph_x(i, n), y_of(*v));
                let mut c = col;
                if i + 1 == n && self.xs.kept.graphs.as_ref().is_some_and(|k| k.now.is_some()) {
                    c[3] = 0.45;
                }
                if let Some(q) = prev {
                    s.line(g, q.0, q.1, p.0, p.1, 2.0, c);
                }
                s.fill(g, p.0 - 1.5, p.1 - 1.5, 3.0, 3.0, c);
                prev = Some(p);
            }
            // the month under the pointer, else the keyboard's or a tap's: a guide, the point ringed, and its values
            let at = Self::graph_month_at(px, py, n).or(self.xs.graphs.cursor.map(|c| c.min(n - 1)));
            if let Some(i) = at {
                let x = Self::graph_x(i, n);
                s.fill(g, x, PLOT.y, 1.0, PLOT.h, rgba(0.0, 0.0, 0.3, 0.35));
                let mut lines = vec![Self::month_label(points[i].month)];
                if i + 1 == n && self.xs.kept.graphs.as_ref().is_some_and(|k| k.now.is_some()) {
                    lines[0] = trf("STR_GRAPHS_SO_FAR", &[("MONTH", &lines[0])]);
                }
                match vals[i] {
                    Some(v) => {
                        let y = y_of(v);
                        s.fill(g, x - 4.0, y - 4.0, 8.0, 8.0, black);
                        s.fill(g, x - 3.0, y - 3.0, 6.0, 6.0, c15(0x7fff));
                        s.fill(g, x - 2.0, y - 2.0, 4.0, 4.0, col);
                        lines.push(format!("{}: {}", tr(m.key), print(m.unit, v)));
                    }
                    None => lines.push(tr("STR_GRAPHS_NO_VALUE").to_string()),
                }
                // what lies behind the value: the rounds or plays it comes from
                let p = &points[i];
                let plays = if series == 0 { p.plays } else { hole(p, hole_no).plays };
                if !matches!(m.key, "STR_GRAPHS_PLAYS" | "STR_GRAPHS_HOLE_PLAYS" | "STR_GRAPHS_ROUNDS") {
                    lines.push(trf("STR_GRAPHS_FROM_PLAYS", &[("COUNT", &plays)]));
                }
                let w = lines.iter().map(|l| F_INFO14.width(l)).fold(0.0, f32::max) + 16.0;
                let h = 16.0 * lines.len() as f32 + 8.0;
                let bx = if x + 12.0 + w < PLOT.x + PLOT.w { x + 12.0 } else { x - 12.0 - w };
                let by = (py.clamp(PLOT.y, PLOT.y + PLOT.h) - h / 2.0).clamp(PLOT.y, PLOT.y + PLOT.h - h);
                let by = if Self::graph_month_at(px, py, n).is_some() { by } else { PLOT.y + 4.0 };
                // the box: the reports' navy edge on the pale yellow of the game's tooltips
                s.fill(g, bx - 1.0, by - 1.0, w + 2.0, h + 2.0, c15(0x0018));
                s.fill(g, bx, by, w, h, rgb(1.0, 1.0, 0.86));
                for (k, l) in lines.iter().enumerate() {
                    s.put(g, F_INFO14, bx + 8.0, by + 4.0 + 16.0 * k as f32, l, if k == 1 { col } else { black });
                }
            }
        }
        // the histograph's legend row (its four labels' places, 0x455ed0): the series, the measure, the latest value and
        // the months kept; the keys under the plot
        let latest = vals.iter().rev().flatten().next().map(|&v| trf("STR_GRAPHS_LATEST", &[("VALUE", &print(m.unit, v))]));
        let series_name = if series == 0 { tr("STR_GRAPHS_CLUB").to_string() } else { trf("STR_GRAPHS_HOLE", &[("HOLE", &series)]) };
        let months = trf("STR_GRAPHS_MONTHS", &[("COUNT", &points.len())]);
        for (x, t) in [(177.0, series_name), (332.0, tr(m.key).to_string()), (488.0, latest.unwrap_or_default()), (644.0, months)] {
            s.put_centered(g, F_INFO14, x, LEGEND_Y, &t, black);
        }
        s.put_centered(g, F_ARIAL10, PLOT.x + PLOT.w / 2.0 - 25.0, HELP_Y, tr("STR_GRAPHS_HELP"), grey);
        self.ok_tick(g, &s, OK.0, OK.1, false);
        g.flush();
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn snap(cash: i64, fees: f64, wages: f64) -> Snapshot {
        let mut ledger = [0.0; LEDGER_COLUMNS];
        ledger[sg_core::economy::LEDGER_GREEN_FEES] = fees;
        ledger[sg_core::economy::LEDGER_SALARIES] = -wages;
        Snapshot { cash, fun: 7, skill: 250, members: 3, ledger }
    }

    #[test]
    fn months_are_recorded_and_closed() {
        let mut k = Kept::default();
        k.roll(4, || snap(1000, 500.0, 100.0));
        k.hole_done(4, || unreachable!(), 1, 5, 4, 10, 200, false);
        k.hole_done(4, || unreachable!(), 2, 3, 3, 20, 300, true);
        assert_eq!(k.months.len(), 0);
        let m = k.now.as_ref().unwrap();
        assert_eq!((m.plays, m.rounds, m.mood), (2, 1, 30));
        assert_eq!(m.holes[&1], HoleMonth { plays: 1, over: 1, mood: 10, fees: 200 });
        // the month ends: the money is the ledger's rise and fall since its start
        k.roll(5, || snap(1700, 1300.0, 200.0));
        assert_eq!(k.months.len(), 1);
        let m = &k.months[0];
        assert_eq!((m.revenue, m.costs, m.cash, m.members, m.skill), (800, 100, 1700, 3, 250));
        assert_eq!(k.now.as_ref().unwrap().month, 5);
        // the month in progress, as it stands now
        let pts = k.points(&snap(1800, 1400.0, 200.0));
        assert_eq!(pts.len(), 2);
        assert_eq!((pts[1].revenue, pts[1].costs), (100, 0));
        // the measures read them
        assert_eq!((CLUB[4].value)(&pts[0], 0), Some(700.0));
        assert_eq!((HOLE[1].value)(&pts[0], 1), Some(1.0));
        assert_eq!((HOLE[1].value)(&pts[0], 9), None);
        assert_eq!((HOLE[3].value)(&pts[0], 2), Some(30000.0));
    }

    #[test]
    fn ten_years_are_kept_and_going_back_starts_afresh() {
        let mut k = Kept::default();
        for m in 0..(MAX_MONTHS as u32 + 30) {
            k.roll(m, || snap(m as i64, 0.0, 0.0));
        }
        assert_eq!(k.months.len(), MAX_MONTHS);
        assert_eq!(k.months.front().unwrap().month, 29);
        assert_eq!(k.months.back().unwrap().month, MAX_MONTHS as u32 + 28);
        // a gap: the months between are simply missing
        k.roll(200, || snap(0, 0.0, 0.0));
        assert_eq!(k.months.back().unwrap().month, MAX_MONTHS as u32 + 29);
        // back to an earlier month: the later ones go
        k.roll(50, || snap(0, 0.0, 0.0));
        assert!(k.months.iter().all(|m| m.month < 50));
        assert_eq!(k.now.as_ref().unwrap().month, 50);
    }

    #[test]
    fn a_save_round_trips_and_nothing_kept_writes_nothing() {
        let mut k = Kept::default();
        k.roll(3, || snap(10, 0.0, 0.0));
        k.hole_done(3, || unreachable!(), 7, 4, 4, -5, 2, false);
        k.roll(4, || snap(20, 200.0, 0.0));
        let json = serde_json::to_string(&k).unwrap();
        let back: Kept = serde_json::from_str(&json).unwrap();
        assert_eq!(back, k);
        // the extras' field of the save: none while nothing is kept, so the save is as before
        let x = crate::extras::Kept::default();
        assert_eq!(serde_json::to_string(&x).unwrap(), "{}");
        let x = crate::extras::Kept { graphs: Some(k), ..Default::default() };
        let back: crate::extras::Kept = serde_json::from_str(&serde_json::to_string(&x).unwrap()).unwrap();
        assert_eq!(back, x);
        // a save from before the graphs reads as nothing kept
        let old: crate::extras::Kept = serde_json::from_str("{\"messages\": []}").unwrap();
        assert!(old.graphs.is_none());
    }

    #[test]
    fn scales_are_round() {
        assert_eq!(scale(0.0, 47.0, true), (0.0, 10.0, 5));
        let (b, st, n) = scale(-1234.0, 9000.0, false);
        assert_eq!((b, st), (-2500.0, 2500.0));
        assert!(b + st * n as f64 >= 9000.0);
        assert_eq!(scale(0.0, 3.0, true).1, 1.0);
        // a flat line still gets a scale
        let (b, st, n) = scale(5.0, 5.0, false);
        assert!(b <= 5.0 && b + st * n as f64 >= 5.0 && st > 0.0);
    }
}
