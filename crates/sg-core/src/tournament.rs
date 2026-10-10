//! SGA tournaments: the SGA's evaluation of the course (0x44fb30), the mid-year offer, the 36-player shotgun field of famous
//! pros (0x46c970), the preparation checklist with its par changes and TV towers (0x46d200), the leaderboard and the
//! prizes (0x45a090), and the cleanup that brings the regular golfers back (0x46d0c0).
//!
//! Facts are from the publisher's golf.exe (docs/PUBLISHER_EXE_NOTES.md, "SGA tournaments"), restated in our own words.

use crate::course::{f, idx, inside, Course, TYPES};
use crate::geom::{clamp, DX, DY, UNIT};
use crate::golfer::{anim, flag, game, Club, Column, Golfer, Member, SLOTS};
use crate::land::ExeRng;
use crate::roster::Person;

/// A tournament has been offered (game flag 0x400000): the pro panel's Begin Tournament button is lit.
pub const OFFERED: u32 = 0x400000;

/// The ten criteria of the SGA report.
pub const CRITERIA: [&str; 10] = [
    "Course length",
    "Number of holes",
    "Time to play",
    "Fun",
    "Variety",
    "Scenic holes",
    "Length holes",
    "Accuracy holes",
    "Imagination holes",
    "Facilities",
];

/// Event names by (score - 40) / 5.
const EVENTS: [&str; 10] = [
    "Jr. Qualifying School",
    "SGA Qualifying School",
    "Jr. Tour Event",
    "Jr. Tour Championship",
    "SGA Amateur Championship",
    "Senior SGA Tour Event",
    "Senior SGA Championship",
    "SGA Tour Event",
    "SGA Players Championship",
    "SGA Championship",
];

/// The SGA report: each criterion's measured value, what the SGA asks for, and its score 0..10.
#[derive(Clone, Debug, Default, PartialEq)]
pub struct SgaReport {
    pub class: i32,
    pub holes: i32,
    pub values: [i32; 10],
    pub ideal: [i32; 10],
    pub scores: [i32; 10],
    /// Sum of the scores; below 1 (any criterion at 0 makes it negative) means improvement is required.
    pub score: i32,
    pub event: &'static str,
    /// First prize in thousands of dollars (0 when the course fails).
    pub purse: i32,
}

impl SgaReport {
    pub fn passed(&self) -> bool {
        self.score >= 1
    }
}

/// The checklist of the preparation dialog: TV towers, smooth greens, deep rough, then each hole's par change.
#[derive(Clone, Debug, Default, PartialEq)]
pub struct Prep {
    pub par_changes: Vec<(usize, i32, i32)>,
}

impl Prep {
    pub fn lines(&self) -> Vec<String> {
        // the exe's strings (0x4e3d20 on), each an option line of the list
        let mut v = vec![
            "Install TV towers and booths for live broadcast.".to_string(),
            "Roll your greens extra smooth and fast.".to_string(),
            "Increase the depth of rough and deep rough grass.".to_string(),
        ];
        for &(h, old, new) in &self.par_changes {
            v.push(format!("Change hole {h} from a par {old} to a par {new}."));
        }
        v
    }
}

/// One row of the leaderboard: the field slot and its score against par so far.
#[derive(Clone, Debug, Default, PartialEq)]
pub struct BoardRow {
    pub slot: usize,
    pub name: String,
    pub score: i32,
    pub gary: bool,
    /// Strokes per hole (index 1..18) and the round's total, for the results.
    pub card: [i8; 19],
    pub total: i32,
    /// Prize in thousands of dollars (0 beyond the paid places).
    pub prize: i32,
}

/// The final standings, shown once nobody is playing.
#[derive(Clone, Debug, Default, PartialEq)]
pub struct Results {
    pub rows: Vec<BoardRow>,
    pub pars: [i32; 19],
    pub gary_rank: Option<usize>,
    pub gary_prize: i32,
}

impl Club {
    fn holes_open(&self) -> i32 {
        (self.next_hole - 1).max(0)
    }

    /// The SGA evaluation (0x44fb30). `facilities` is the number of building kinds 6..19 present and joined to the clubhouse.
    pub fn sga_evaluate(&mut self, facilities: i32) -> SgaReport {
        let n = self.holes_open();
        let c = crate::economy::rank(n as usize);
        let (ideal_holes, k) = match c {
            0 => (5, 5),
            1 => (9, 9),
            2 => (17, n),
            _ => (18, 18),
        };
        let mut len = 0;
        let mut time = 0;
        let mut fun = 0;
        let mut count = 0;
        let mut variety = 0;
        let mut scenic = 0;
        let mut lah = [0; 3];
        let thr = if self.difficulty != 0 { 50 } else { 25 };
        let all = self.game & game::TWO_TEES != 0;
        for h in 1..19 {
            let hole = &self.holes[h];
            if hole.par == 0 {
                continue;
            }
            count += 1;
            len += hole.length;
            if hole.tee_shots > 0 {
                time += (hole.time / hole.tee_shots) / 40;
            }
            fun += crate::ratings::hole_fun(hole);
            let row = crate::ratings::report_row(h, hole, self.difficulty, all);
            if row.variety < 2 {
                variety += 1;
            }
            if row.scenic {
                scenic += 1;
            }
            for (i, d) in [row.len, row.acc, row.img].into_iter().enumerate() {
                if d >= thr {
                    lah[i] += 1;
                }
            }
        }
        if count > 0 {
            fun /= count;
        }
        let ideal_len = (ideal_holes * 100 / 18) * (5 * c + 57);
        let mut x = [0; 10];
        x[0] = if c == 3 { -((ideal_len - len) / 100) + 10 } else { (len - ideal_len) * (c + 4) * 5 / ideal_len.max(1) + 10 };
        x[1] = 10 - (ideal_holes - n).abs() * (4 - c);
        x[2] = 10 - (time - 235) / 6;
        if x[2] == 0 && time <= 300 {
            x[2] = 1;
        }
        x[3] = (fun - 109) / 10 + 10;
        x[4] = variety - k + 10;
        x[5] = scenic - k + 10;
        x[6] = lah[0] - k + 10;
        x[7] = lah[1] - k + 10;
        x[8] = lah[2] - k + 10;
        x[9] = (facilities - k / 2) * 2 + 8;
        let scores = x.map(|v| clamp(v, 0, 10));
        let mut score: i32 = scores.iter().sum();
        if scores.contains(&0) {
            score -= 999;
        }
        let ideal = [ideal_len, ideal_holes, 235, 109, k, k, k, k, k, k / 2 + 1];
        let values = [len, n, time, fun, variety, scenic, lah[0], lah[1], lah[2], facilities];
        if score >= 90 {
            let tee = |h: usize| (self.holes[h].back.0 * 1024 + 512, self.holes[h].back.1 * 1024 + 512);
            let (t9, t18) = (tee(9), tee(18));
            if n >= 9 {
                self.award_at(0xc, t9);
            }
            if n == 18 {
                self.award_at(0x10, t18);
            }
            if score == 100 && n == 18 {
                self.award_at(0x15, t18);
            }
        }
        let mut r = SgaReport { class: c, holes: n, values, ideal, scores, score, ..Default::default() };
        if score < 1 {
            self.game &= !OFFERED;
            r.event = "Improvement Required";
            return r;
        }
        self.sga_score = score;
        let e = (score - 40) / 5;
        r.event = match e {
            1..=9 => EVENTS[e as usize],
            10..=12 if k == 18 => "Grand Slam Championship",
            10..=12 => "Mini Slam Championship",
            _ => EVENTS[0],
        };
        self.purse = ((score - 50) / 5 + 5) * (c + 1) * (n + 1) * 2;
        r.purse = self.purse;
        // a passing evaluation lights the tournament button, and the match button too (the exe sets 0x402000)
        self.game |= OFFERED | crate::pro::CHALLENGE;
        r
    }

    /// The mid-year offer (main frame 0x418000): at the start of the fifth month of each year, when the pro is not out and no
    /// tournament runs. Returns true when the SGA made an offer.
    pub fn tournament_offer(&mut self, facilities: i32, course_name: &str) -> bool {
        if self.tick % 0x2000 != 0x1000 || self.gary != -1 || self.game & game::TOURNAMENT != 0 || self.sandbox {
            return false;
        }
        self.purse = 0;
        self.sga_evaluate(facilities);
        if self.purse == 0 {
            self.game &= !OFFERED;
            return false;
        }
        let pro = self.roster.first().map(|p| p.name.clone()).unwrap_or_else(|| "Gary Golf".to_string());
        // the exe's pieces (0x4c62cc, the course name, 0x4c627c, the purse, 0x4c6250, the pro's name, 0x4c6230); the event's
        // name is not in it
        self.message(format!(
            "The SGA is interested in holding a tournament at your course. 'We'd like to schedule the {course_name} Open Golf \
             Tournament here as soon as possible.' We can offer a top prize of \u{a7}{},000!  Click the tournament button in the \
             {pro} panel to begin the tournament.",
            self.purse
        ));
        self.sound(0x2f, None);
        self.game |= OFFERED;
        self.purse = 0;
        true
    }

    /// Whether the Begin Tournament button works now.
    pub fn can_begin_tournament(&self) -> bool {
        self.game & OFFERED != 0 && self.gary == -1 && self.game & game::REPEAT == 0
    }

    /// Picks the famous pro for field slot s (0x46c970): stronger pros come when the purse and the club's cash are higher.
    fn pick_field_pro(&self, rng: &mut ExeRng, s: usize, used: &mut [bool; 100], cash: i32) -> usize {
        let d = clamp(self.difficulty, 0, 3);
        let p = self.purse;
        let rating = |u: usize| self.pros[u].skills.iter().map(|&v| v as i32).sum::<i32>();
        let mut count = 0;
        loop {
            let (u, w) = loop {
                let (u, w) = loop {
                    let u = rng.below(100) as usize;
                    let r = if u < self.pros.len() { rating(u) } else { 0 };
                    let mut w = (r - 10) * (r - 10) / ((5 * d + 20) * 2);
                    if s == 0 {
                        w -= d * w / 6;
                    }
                    if self.game & crate::championship::CHAMPIONSHIP != 0 {
                        w = rng.below(w);
                    }
                    if u < self.pros.len() && u != 0 {
                        count += 1;
                        if count >= 800 || !used[u] {
                            break (u, w);
                        }
                    }
                };
                if w <= count / (4 - d) + p / 10 + cash / 200 {
                    break (u, w);
                }
            };
            let k = if s == 0 { count / 4 } else { count };
            if w >= p / 10 - k + cash / 200 {
                if used[u] {
                    return 0;
                }
                used[u] = true;
                return u;
            }
        }
    }

    /// Sets up the field (0x46c970): the course is emptied and two players start on each hole (a shotgun start), the pro on
    /// hole 1 with a famous partner; everyone else is a famous pro picked for the purse and the club's cash.
    pub fn start_tournament(&mut self, c: &Course, rng: &mut ExeRng, pos: (i32, i32), facing: i32, cash: i32) {
        if self.pros.is_empty() {
            self.pros.push(crate::vips::Pro { name: "Joe Pro".to_string(), skills: [3; 12], ..Default::default() });
        }
        self.backup = Some((self.roster.clone(), self.members.clone()));
        self.pro_mask = 0;
        for k in 0..12 {
            if self.pro_skill[k] != 0 {
                self.pro_mask |= 1 << k;
            }
        }
        for g in self.g.iter_mut().take(SLOTS) {
            g.hole = 0;
            g.partner = -1;
        }
        self.tee_counter = 100;
        let n = self.holes_open();
        let mut used = [false; 100];
        let (da, db) = c.door;
        for h in 1..19 {
            if self.holes[h].par == 0 {
                continue;
            }
            for k in 0..2 {
                let s = 2 * (h - 1) + k;
                let mut g = Golfer {
                    x: da * UNIT + 0x600,
                    y: db * UNIT + 0x600,
                    hole: h as i32,
                    facing: self.holes[h].tee_facing,
                    level: 2,
                    class: 7,
                    pause: (k as i32 + (h as i32 - n - 1) * 2) * 4,
                    story: -1,
                    anim: anim::STAND,
                    hole_tick: self.tick,
                    ..Default::default()
                };
                g.mood = rng.below(4) + 3;
                if self.difficulty == 0 {
                    g.mood = 5;
                }
                g.tee_order = self.tee_counter;
                self.tee_counter -= 1;
                g.partner = (s ^ 1) as i32;
                if s == 1 {
                    g.kind = 0x24;
                    g.roster = 0;
                    g.class = 0x47;
                    g.flags |= flag::GARY;
                    g.x = pos.0;
                    g.y = pos.1;
                    g.facing = facing;
                    g.pause = 0;
                    g.skill_mask = self.pro_mask;
                    for i in 0..12 {
                        g.skills[i] = self.pro_skill[i];
                    }
                } else {
                    let u = self.pick_field_pro(rng, s, &mut used, cash);
                    let pro = self.pros.get(u).cloned().unwrap_or_default();
                    g.kind = 0x20;
                    g.famous = u as i32;
                    g.level = clamp(self.difficulty, 0, 2);
                    let r = s.max(1);
                    g.roster = r as i32;
                    if let Some(p) = self.roster.get_mut(r) {
                        p.name = pro.name.clone();
                        p.job = "Golf Pro".to_string();
                        p.b21 = if pro.body >= 4 { 0x89 } else { 9 };
                    }
                    g.skill_mask = 0xe0;
                    for i in 0..12 {
                        g.skills[i] = pro.skills[i];
                        if pro.skills[i] != 0 {
                            g.skill_mask |= 1 << i;
                        }
                    }
                }
                self.g[s] = g;
            }
        }
        self.tee_counter = 100;
        self.tourney_opts = -1;
        self.game = (self.game | game::TOURNAMENT) & !crate::pro::START_ROUND;
        self.gary = if self.holes[1].par != 0 { 1 } else { -1 };
        let tee = (self.holes[1].back.0 * 1024 + 512, self.holes[1].back.1 * 1024 + 512);
        self.award_at(3, tee);
        if self.purse >= 500 {
            self.award_at(8, tee);
        }
        if self.purse >= 1000 {
            self.award_at(0xd, tee);
        }
    }

    /// The preparation checklist (0x46d200): each hole's tournament par (3, 4 past 299 yards, 5 past 499) is written now.
    pub fn tournament_prep(&mut self) -> Prep {
        let mut p = Prep::default();
        for h in 1..19 {
            let hole = &mut self.holes[h];
            if hole.par == 0 {
                continue;
            }
            let new = if hole.length > 499 {
                5
            } else if hole.length > 299 {
                4
            } else {
                3
            };
            if new != hole.par {
                p.par_changes.push((h, hole.par, new));
                hole.par = new;
            }
        }
        self.tourney_opts = -2;
        p
    }

    /// The player's ticks in the checklist: an unticked par change gets par + 1 (not a true undo, as in the exe); bit 1 smooth
    /// greens and bit 2 deep rough go to the shot planner. Returns the TV tower (kind 17) and booth (kind 18) sites when bit 0
    /// is ticked, as (kind, a, b, facing).
    pub fn apply_prep(&mut self, c: &Course, rng: &mut ExeRng, prep: &Prep, mask: i32) -> Vec<(i32, i32, i32, u8)> {
        for (i, &(h, _, _)) in prep.par_changes.iter().enumerate() {
            if mask & (8 << i) == 0 {
                self.holes[h].par += 1;
            }
        }
        self.tourney_opts = mask;
        self.planner.options = mask as u32 & 6;
        let mut out = Vec::new();
        if mask & 1 == 0 {
            return out;
        }
        let n = self.holes_open();
        let mut taken: Vec<(i32, i32)> = Vec::new();
        for h in 1..19 {
            if self.holes[h].par == 0 {
                continue;
            }
            let d = self.holes[h].tee_facing.rem_euclid(8);
            let mut kinds = vec![(17, d)];
            if h as i32 >= n - 1 {
                // the booth turns two steps one way or the other: +2 when the coin (0x45c1e0(2)) is not zero, else -2
                let bd = if rng.below(2) != 0 { (d + 2) & 7 } else { (d - 2) & 7 };
                kinds.push((18, bd));
            }
            for (kind, dir) in kinds {
                let pin = self.holes[h].pin;
                let (mut a, mut b) = pin;
                for step in 0..7 {
                    if rng.below(2) == 0 {
                        a += DX[dir as usize];
                    }
                    if rng.below(2) == 0 {
                        b += DY[dir as usize];
                    }
                    if step < 2 || !inside(a, b) || taken.contains(&(a, b)) {
                        continue;
                    }
                    let i = idx(a, b);
                    let ty = c.ty[i];
                    let class = TYPES[(ty as usize).min(22)].class;
                    if (class == 4 || class == 13) && ty != crate::course::t::BUILDING && c.flags[i] & (f::LOW | f::CUP | f::OBSTACLE) == 0
                    {
                        let (dx, dy) = (pin.0 - a, pin.1 - b);
                        let facing = if dy.abs() < dx.abs() {
                            if dx > 0 {
                                1
                            } else {
                                3
                            }
                        } else if dy > 0 {
                            2
                        } else {
                            0
                        };
                        out.push((kind, a, b, facing));
                        taken.push((a, b));
                        break;
                    }
                }
            }
        }
        out
    }

    /// The leaderboard (0x45a090): field slots ranked by score against par over the holes played, ties to the lower slot.
    /// Returns the rows and whether anyone is still playing.
    pub fn leaderboard(&self) -> (Vec<BoardRow>, bool) {
        let mut rows = Vec::new();
        let mut playing = false;
        for s in 0..36.min(SLOTS) {
            if self.par(s as i32 / 2 + 1) == 0 {
                continue;
            }
            let g = &self.g[s];
            if g.hole != 0 {
                playing = true;
            }
            let mut score = 0;
            let mut total = 0;
            for h in 1..19 {
                let k = g.card[h] as i32;
                if k != 0 && h as i32 != g.hole && self.par(h as i32) != 0 {
                    score += k - self.par(h as i32);
                    total += k;
                }
            }
            rows.push(BoardRow { slot: s, name: self.name(s), score, gary: s == 1, card: g.card, total, prize: 0 });
        }
        rows.sort_by_key(|r| (r.score, r.slot));
        (rows, playing)
    }

    /// Ends the tournament when nobody is playing any more: prizes for the top N places (the purse, then two thirds of the
    /// one before), the pro's prize paid to the club, trophies, then the cleanup. Returns the results to show.
    pub fn tournament_tick(&mut self, rng: &mut ExeRng, c: &Course) -> Option<Results> {
        if self.game & game::TOURNAMENT == 0 || self.tourney_opts < 0 {
            return None;
        }
        let (mut rows, playing) = self.leaderboard();
        if playing {
            return None;
        }
        let n = self.holes_open();
        if self.purse == 0 {
            self.purse = 20 * n;
        }
        let mut purse = self.purse;
        let mut res = Results::default();
        for h in 1..19 {
            res.pars[h] = self.par(h as i32);
        }
        for (i, row) in rows.iter_mut().enumerate() {
            let r = i as i32 + 1;
            if r < n + 1 {
                row.prize = purse;
                if row.gary {
                    res.gary_rank = Some(r as usize);
                    res.gary_prize = purse;
                    let at = (self.g[1].x, self.g[1].y);
                    self.earn(purse * 10, Column::Other, at);
                    self.trophies += if r < 4 { 4 - r } else { 1 };
                    self.log_event(crate::records::log::TOURNAMENT, r);
                    if r == 1 {
                        if n >= 9 {
                            self.award_at(0xb, at);
                        }
                        if n == 18 {
                            self.award_at(0xf, at);
                        }
                        // the Grand Slam victories ask for a purse over 100000 thousand, which the SGA never offers
                        if n == 18 && self.purse > 100000 {
                            let theme = self.course_theme as u32;
                            self.award_at(17 + theme.min(3), at);
                        }
                    }
                }
                purse = purse * 2 / 3;
            }
        }
        res.rows = rows;
        self.sound(0x38, None);
        self.end_tournament(rng, c);
        Some(res)
    }

    /// Cancels a running tournament (key 'n'): no prizes.
    pub fn cancel_tournament(&mut self, rng: &mut ExeRng, c: &Course) -> bool {
        if self.game & game::TOURNAMENT == 0 {
            return false;
        }
        self.game &= !OFFERED;
        self.end_tournament(rng, c);
        true
    }

    /// The cleanup (0x46d0c0): the roster and member records come back, and two regular golfers start on each hole. The exe's
    /// par restore loop works on the record after the last hole, so the tournament pars stay; we leave that record alone.
    fn end_tournament(&mut self, rng: &mut ExeRng, c: &Course) {
        self.game &= !game::TOURNAMENT;
        self.purse = 0;
        self.gary = -1;
        if let Some((r, m)) = self.backup.take() {
            self.roster = r;
            self.members = m;
        }
        self.tourney_opts = 0;
        self.planner.options = 0;
        for g in self.g.iter_mut().take(SLOTS) {
            g.hole = 0;
            g.flags &= !flag::GARY;
        }
        self.tee_counter = 0;
        self.create_counter = 0;
        for h in (1..19).rev() {
            if self.holes[h].par == 0 {
                continue;
            }
            let a = self.create(c, rng, true);
            let b = self.create(c, rng, true);
            if a < 0 || b < 0 {
                continue;
            }
            let (a, b) = (a as usize, b as usize);
            for (s, p) in [(a, b), (b, a)] {
                self.g[s].hole = h as i32;
                self.g[s].partner = p as i32;
                self.g[s].tee_order = self.tee_counter;
                self.tee_counter += 1;
            }
            self.last_created = b as i32;
        }
    }
}

/// The roster and member records saved while the field borrows them.
pub type Backup = (Vec<Person>, Vec<Member>);

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn evaluation_and_purse() {
        let mut cl = Club::default();
        for h in 1..10 {
            let hole = &mut cl.holes[h];
            hole.par = 4;
            hole.length = 380;
            hole.tee_shots = 20;
            hole.time = 20 * 40 * 27;
            hole.mood_sum = 30;
            hole.plans = 10;
        }
        cl.next_hole = 10;
        let r = cl.sga_evaluate(5);
        assert_eq!(r.class, 1);
        assert_eq!(r.scores[1], 10, "nine holes is the Daily Fee ideal");
        // no scenic or skill-demand holes: those four criteria score 1 each
        assert_eq!(r.scores, [10, 10, 9, 10, 10, 1, 1, 1, 1, 10]);
        assert_eq!(r.score, 63);
        assert_eq!(r.event, "SGA Amateur Championship");
        assert_eq!(r.purse, 7 * 2 * 10 * 2);
        assert!(cl.game & OFFERED != 0);
        cl.next_hole = 1;
        assert!(!cl.sga_evaluate(5).passed(), "no holes fails");
        assert!(cl.game & OFFERED == 0);
        // the purse formula
        assert_eq!(((100 - 50) / 5 + 5) * 4 * 19 * 2, 2280);
    }

    #[test]
    fn field_and_leaderboard() {
        let mut cl = Club::default();
        let c = Course::default();
        let mut rng = ExeRng::from_clock(5);
        for h in 1..4 {
            cl.holes[h].par = 4;
            cl.holes[h].length = 350;
        }
        cl.next_hole = 4;
        cl.pros =
            (0..20).map(|i| crate::vips::Pro { name: format!("Pro {i}"), skills: [(i % 7) as u8; 12], ..Default::default() }).collect();
        cl.purse = 100;
        cl.start_tournament(&c, &mut rng, (0, 0), 0, 1000);
        assert!(cl.game & game::TOURNAMENT != 0);
        assert_eq!(cl.gary, 1);
        assert_eq!(cl.g[1].flags & flag::GARY, flag::GARY);
        assert_eq!(cl.g[4].hole, 3);
        assert!(cl.g[0].vip() == 0x20 && cl.g[5].vip() == 0x20);
        let prep = cl.tournament_prep();
        assert!(prep.par_changes.is_empty());
        cl.apply_prep(&c, &mut rng, &prep, 0xfffff);
        // everyone finishes: slot 3 is best
        for s in 0..6 {
            cl.g[s].hole = 0;
            for h in 1..4 {
                cl.g[s].card[h] = 5;
            }
        }
        cl.g[3].card[2] = 3;
        let (rows, playing) = cl.leaderboard();
        assert!(!playing);
        assert_eq!(rows[0].slot, 3);
        let res = cl.tournament_tick(&mut rng, &c).expect("results");
        assert_eq!(res.rows[0].prize, 100);
        assert_eq!(res.rows[1].prize, 66);
        assert_eq!(res.rows[3].prize, 0, "only the top N places are paid");
        // the pro ties with four others and ranks behind slot 0 (ties go to the lower slot)
        assert_eq!(res.gary_rank, Some(3));
        assert_eq!(res.rows[2].prize, 44);
        assert_eq!(cl.trophies, 1);
        assert!(cl.game & game::TOURNAMENT == 0);
        assert_eq!(cl.roster[2].name, Club::default().roster[2].name, "the roster comes back");
    }
}
