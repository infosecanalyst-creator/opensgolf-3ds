//! OpenSGolf extras: the year in numbers (extras.rs). The original's End of Year report (screens_ui.rs) stays as it is;
//! with the extra on, its OK brings a second page in the report's own art (ENDoYEAR: the title plate, the This Year and
//! Last Year heads, one row of pills per figure, the OK tick) with the year's key figures against the year before, each
//! with an arrow, up in the report's green or down in its red:
//!
//! * revenue and profit: the F4 Financial Report's ledger for the year (money taken in, and taken in less spent);
//! * rounds played: rounds golfers finished (the members' rounds, the roster's count);
//! * the fun and skill ratings and the members: the club's month records the original report reads;
//! * the best and the worst hole: the open holes with the happiest and the unhappiest golfers (mood as they finish it,
//!   the hole statistics the reports use);
//! * tournaments won by the player's pro.
//!
//! The ratings, members and money come from the game's own records, so they are there from the first year. Rounds played,
//! tournaments won and the holes of the year before are counted by the extra itself at each year's end (and tournaments
//! as they are won), so they show from the first year end the extra sees (a dash until then), and they are kept with the
//! game only while the extra is on.
use crate::app::*;
use crate::gfx::Gfx;
use crate::info_ui::c15;
use crate::screens_ui::digits;
use crate::ui::{rgb, text_width, Screen as Ui, F_INFO14, F_INFO_TITLE};
use serde::{Deserialize, Serialize};
use sg_core::lang::{tr, trf};

/// The year-end marks kept.
const MAX_MARKS: usize = 12;

/// The club as a year ended (year index `year`, 0 for 2001): rounds finished and tournaments won so far, and the best and
/// worst holes of that year (0 none).
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, Serialize, Deserialize)]
#[serde(default)]
pub struct Mark {
    pub year: u32,
    pub rounds: i64,
    pub wins: u32,
    pub best: u8,
    pub worst: u8,
}

/// What the year in numbers keeps with the game: tournaments won while it was on, and the marks of the years' ends.
#[derive(Clone, Debug, Default, PartialEq, Serialize, Deserialize)]
#[serde(default)]
pub struct Kept {
    pub wins: u32,
    #[serde(skip_serializing_if = "Vec::is_empty")]
    pub marks: Vec<Mark>,
}

impl Kept {
    /// The mark of the end of year `year`; the start of the game stands in for the end of the year before 2001.
    pub fn mark(&self, year: i64) -> Option<Mark> {
        if year < 0 {
            return Some(Mark::default());
        }
        self.marks.iter().rev().find(|m| m.year as i64 == year).copied()
    }

    /// Keeps the mark of a year's end (again for the same year: the newer).
    pub fn push(&mut self, m: Mark) {
        self.marks.retain(|k| k.year != m.year);
        self.marks.push(m);
        self.marks.sort_by_key(|k| k.year);
        if self.marks.len() > MAX_MARKS {
            self.marks.remove(0);
        }
    }

    /// A count over year `year`: the count now (`now`, at that year's end) less the count at the year before's end.
    pub fn over_year(&self, year: i64, now: Option<i64>, count: impl Fn(&Mark) -> i64) -> Option<i64> {
        Some(now? - count(&self.mark(year - 1)?))
    }
}

/// The best and the worst open hole by the golfers' mood as they finished it (mood sum over tee shots); none before any
/// hole has been played.
pub fn best_worst(holes: &[sg_core::golfer::Hole]) -> Option<(u8, u8)> {
    let rated: Vec<(u8, i64)> = (1..19)
        .filter_map(|h| {
            holes.get(h).filter(|r| r.par != 0 && r.tee_shots > 0).map(|r| (h as u8, r.mood_sum as i64 * 1000 / r.tee_shots as i64))
        })
        .collect();
    let best = rated.iter().max_by_key(|r| (r.1, -(r.0 as i64)))?.0;
    let worst = rated.iter().min_by_key(|r| (r.1, r.0))?.0;
    Some((best, worst))
}

/// A ledger year's money taken in (each column's rise) and its profit (the columns' sum), in dollars.
pub fn revenue_profit(ledger: &[f64]) -> (i64, i64) {
    let rev: f64 = ledger.iter().filter(|v| **v > 0.0).sum();
    let all: f64 = ledger.iter().sum();
    (rev.round() as i64, all.round() as i64)
}

/// How a row's figure is written.
#[derive(Clone, Copy)]
enum Fig {
    Money,
    Count,
    Skill,
    Hole,
}

impl App {
    /// The year in numbers at a year's end (open_year_end): the mark of the year that ended.
    pub fn year_mark(&mut self) {
        if !crate::extras::year_summary() {
            return;
        }
        let year = (self.club.date() >> 13).saturating_sub(1);
        let (best, worst) = best_worst(&self.club.holes).unwrap_or((0, 0));
        let rounds = self.rounds_played();
        let k = self.xs.kept.years.get_or_insert_with(Default::default);
        let wins = k.wins;
        k.push(Mark { year, rounds, wins, best, worst });
    }

    /// A tournament won by the player's pro (tourney_after_tick), counted while the extra is on.
    pub fn year_count_win(&mut self) {
        if crate::extras::year_summary() {
            self.xs.kept.years.get_or_insert_with(Default::default).wins += 1;
        }
    }

    /// Rounds finished so far on this course (the members' rounds).
    fn rounds_played(&self) -> i64 {
        self.club.members.iter().map(|m| m.rounds as i64).sum()
    }

    /// The year-end report's OK: true when it goes on to the year in numbers instead of closing.
    pub fn year_summary_next(&mut self) -> bool {
        if self.screen != Screen::YearEnd || !crate::extras::year_summary() {
            self.xs.year_page = false;
            return false;
        }
        self.xs.year_page = !self.xs.year_page;
        self.xs.year_page
    }

    /// The rows: label key, this year, last year, how written.
    fn year_rows(&self) -> Vec<(&'static str, Option<i64>, Option<i64>, Fig)> {
        let tick = self.year_end_tick;
        let year = (tick >> 13) as i64 - 1;
        let ledger = |y: i64| (y >= 0).then(|| self.econ.ledger.get(y as usize).map(|l| revenue_profit(l)).unwrap_or((0, 0)));
        let (this_money, last_money) = (ledger(year), ledger(year - 1));
        // the club's month records as the original report reads them: the year's end and the year before's
        let m = ((tick >> 10) % 500) as usize;
        let p = (m + 500 - 8) % 500;
        let h = &self.club.history;
        let rec = |i: usize, k: usize| h.get(i).map(|r| r[k] as i64);
        let first = year < 1;
        let last_rec = |k: usize| if first { None } else { rec(p, k) };
        let kept = self.xs.kept.years.clone().unwrap_or_default();
        let rounds_now = Some(self.rounds_played());
        // tournaments are counted while the extra is on (none counted yet: none won)
        let wins_now = Some(kept.wins as i64);
        let last_mark = kept.mark(year - 1);
        let rounds = kept.over_year(year, rounds_now, |k| k.rounds);
        let rounds_last = last_mark.and_then(|l| kept.over_year(year - 1, Some(l.rounds), |k| k.rounds)).filter(|_| year >= 1);
        let wins = kept.over_year(year, wins_now, |k| k.wins as i64);
        let wins_last = last_mark.and_then(|l| kept.over_year(year - 1, Some(l.wins as i64), |k| k.wins as i64)).filter(|_| year >= 1);
        let (best, worst) = best_worst(&self.club.holes).map(|(b, w)| (Some(b as i64), Some(w as i64))).unwrap_or((None, None));
        let hole = |v: u8| (v != 0).then_some(v as i64);
        let (best_last, worst_last) = match last_mark.filter(|_| year >= 1) {
            Some(l) => (hole(l.best), hole(l.worst)),
            None => (None, None),
        };
        vec![
            ("STR_YEARSUM_REVENUE", this_money.map(|v| v.0), last_money.map(|v| v.0), Fig::Money),
            ("STR_YEARSUM_PROFIT", this_money.map(|v| v.1), last_money.map(|v| v.1), Fig::Money),
            ("STR_YEARSUM_ROUNDS", rounds, rounds_last, Fig::Count),
            ("STR_YEARSUM_FUN", rec(m, 1), last_rec(1), Fig::Count),
            ("STR_YEARSUM_SKILL", rec(m, 2), last_rec(2), Fig::Skill),
            ("STR_YEARSUM_MEMBERS", rec(m, 3), last_rec(3), Fig::Count),
            ("STR_YEARSUM_BEST_HOLE", best, best_last, Fig::Hole),
            ("STR_YEARSUM_WORST_HOLE", worst, worst_last, Fig::Hole),
            ("STR_YEARSUM_WINS", wins, wins_last, Fig::Count),
        ]
    }

    /// The year in numbers, in the End of Year report's art and fonts (screens_ui draw_year_end): the title plate, the
    /// column heads, a row of pills per figure (the art's second row repeated), the plate under them with a note, and the
    /// bottom with the OK tick.
    pub fn draw_year_summary(&mut self, g: &mut Gfx, s: &Ui) {
        let e = &self.art.endo;
        let has = e.tex.is_some();
        let rows = self.year_rows();
        let n = rows.len() as f32;
        let black = c15(0);
        // the top piece down to the end of its first row (y 131), the second row's 20 pixels once per further row, then the
        // piece's foot (y 191 to 216) with its plate
        let foot = 111.0 + 20.0 * n;
        if has {
            s.image_part(g, e, 187.0, 40.0, 187.0, 40.0, 429.0, 91.0);
            for k in 1..rows.len() {
                s.image_part(g, e, 187.0, 111.0 + 20.0 * k as f32, 187.0, 131.0, 429.0, 20.0);
            }
            s.image_part(g, e, 187.0, foot, 187.0, 191.0, 429.0, 25.0);
        } else {
            s.fill(g, 187.0, 40.0, 429.0, foot + 25.0 - 40.0, rgb(0.9, 0.88, 0.8));
        }
        let year = 2000 + (self.year_end_tick >> 13);
        s.put_centered(g, F_INFO_TITLE, 406.0, 55.0, &trf("STR_YEARSUM_TITLE", &[("YEAR", &year)]), black);
        s.put_centered(g, F_INFO14, 465.0, 93.0, tr("STR_YEAREND_THIS_YEAR"), black);
        s.put_centered(g, F_INFO14, 566.0, 93.0, tr("STR_YEAREND_LAST_YEAR"), black);
        let green = c15(0x1284);
        let red = rgb(0.8, 0.15, 0.1);
        for (k, (key, this, last, fig)) in rows.iter().enumerate() {
            let y = 125.0 + 20.0 * k as f32;
            s.text_centered(g, 301.0, y, tr(key), 14.0, black);
            let fmt = |v: Option<i64>| match (v, fig) {
                (None, _) => "-".to_string(),
                (Some(v), Fig::Money) => digits(v),
                (Some(v), Fig::Skill) => format!("{}.{:02}", v / 100, (v % 100).abs()),
                (Some(v), Fig::Hole) => trf("STR_YEARSUM_HOLE", &[("HOLE", &v)]),
                (Some(v), Fig::Count) => v.to_string(),
            };
            let (la, lb) = (fmt(*this), fmt(*last));
            // the arrow on the left of this year's pill (holes are named, not compared)
            let up = match (this, last, fig) {
                (_, _, Fig::Hole) => None,
                (Some(a), Some(b), _) if a != b => Some(a > b),
                _ => None,
            };
            let c = match up {
                Some(true) => green,
                Some(false) => red,
                None => black,
            };
            if let Some(up) = up {
                draw_big_arrow(g, s, 430.0, y - 5.0, up, c);
            }
            s.text(g, 500.0 - text_width(&la, 14.0), y, &la, 14.0, c);
            s.text(g, 601.0 - text_width(&lb, 14.0), y, &lb, 14.0, black);
        }
        s.put_centered(g, F_INFO14, 404.0, foot + 9.0, tr("STR_YEARSUM_NOTE"), black);
        let y = foot + 25.0;
        if has {
            s.image_part(g, e, 187.0, y, 187.0, 374.0, 429.0, 55.0);
            self.ok_tick(g, s, 550.0, y + 8.0, false);
        } else {
            s.text_centered(g, 404.0, y + 34.0, tr("STR_OK"), 16.0, black);
        }
    }
}

/// An arrow 9 pixels wide and 9 high with a dark edge, pointing up or down, centred on (cx, cy).
fn draw_big_arrow(g: &mut Gfx, s: &Ui, cx: f32, cy: f32, up: bool, c: [f32; 4]) {
    let edge = c15(0x0842);
    for i in 0..5 {
        let w = 1.0 + 2.0 * i as f32;
        let y = if up { cy - 4.0 + i as f32 } else { cy + 4.0 - i as f32 };
        s.fill(g, cx - w / 2.0 - 1.0, y - 0.5, w + 2.0, 2.0, edge);
        s.fill(g, cx - w / 2.0, y, w, 1.0, c);
    }
    let (sy, sh) = if up { (cy + 1.0, 4.0) } else { (cy - 4.0, 4.0) };
    s.fill(g, cx - 2.5, sy, 5.0, sh, edge);
    s.fill(g, cx - 1.5, sy, 3.0, sh - 0.5, c);
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn year_counts_come_from_the_marks_of_year_ends() {
        let mut k = Kept::default();
        // the first year counts from the start of the game
        assert_eq!(k.over_year(0, Some(40), |m| m.rounds), Some(40));
        // a later year needs the mark of the year before's end
        assert_eq!(k.over_year(1, Some(90), |m| m.rounds), None);
        k.push(Mark { year: 0, rounds: 40, wins: 1, best: 3, worst: 5 });
        assert_eq!(k.over_year(1, Some(90), |m| m.rounds), Some(50));
        assert_eq!(k.over_year(1, Some(3), |m| m.wins as i64), Some(2));
        // the same year marked again keeps the newer, and only the last marks are kept
        k.push(Mark { year: 0, rounds: 41, ..Default::default() });
        assert_eq!(k.marks.len(), 1);
        assert_eq!(k.mark(0).unwrap().rounds, 41);
        for y in 1..30 {
            k.push(Mark { year: y, ..Default::default() });
        }
        assert_eq!(k.marks.len(), MAX_MARKS);
        assert_eq!(k.marks.first().unwrap().year, 29 - MAX_MARKS as u32 + 1);
        // nothing kept writes nothing
        assert_eq!(serde_json::to_string(&Kept::default()).unwrap(), "{\"wins\":0}");
    }

    #[test]
    fn best_and_worst_holes_and_the_ledger() {
        let mut holes = vec![sg_core::golfer::Hole::default(); 20];
        assert_eq!(best_worst(&holes), None);
        for (h, shots, mood) in [(1, 10, 50), (2, 10, 80), (3, 4, 4), (4, 0, 0)] {
            holes[h].par = 4;
            holes[h].tee_shots = shots;
            holes[h].mood_sum = mood;
        }
        assert_eq!(best_worst(&holes), Some((2, 3)));
        // a closed hole is left out
        holes[2].par = 0;
        assert_eq!(best_worst(&holes), Some((1, 3)));
        assert_eq!(revenue_profit(&[5000.0, 1000.0, 0.0, -2500.0, -100.0]), (6000, 3400));
    }
}
