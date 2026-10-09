//! The course statistics pass the exe runs every frame (0x42dea0): each open hole's skill differentials, type, fun, too
//! hard and too easy marks, variety against the previous hole, the Top 100 / Top 18 awards, and the club ratings; and the
//! Course Report's columns (0x44fb30), which recompute the same numbers from the hole records.
//!
//! Facts are from the publisher's golf.exe (docs/PUBLISHER_EXE_NOTES.md, "Hole and club ratings"), restated in our own words.

use crate::geom::angle;
use crate::golfer::{Club, Event, Hole};

/// The club ratings the status bar shows (all recomputed on each pass).
#[derive(Clone, Copy, Debug, Default, PartialEq, serde::Serialize, serde::Deserialize)]
pub struct ClubRatings {
    /// Sum of hole fun (0x59ae78).
    pub fun: i32,
    /// Sum of the three differentials over holes, hundredths of a stroke (0x541cd8).
    pub skill: i32,
    /// The three sums on their own (0x5a882c, 0x56949c, 0x5a636c).
    pub len: i32,
    pub acc: i32,
    pub img: i32,
    /// Sum of par and of yards over open holes (0x59aafc, 0x58d36c).
    pub par: i32,
    pub yards: i32,
    /// Silver and better members minus home sites (0x56d1b0).
    pub waitlist: i32,
}

/// Hole type names by skill mask (0x4c2a28).
pub const TYPE_NAMES: [&str; 8] = ["Breather", "Freeway", "Precise", "Challenge", "Creative", "Heroic", "Strategic", "Classic"];

/// The per-class numbers of a hole: rounds and strokes with a prior of 8 rounds at par, and the finished rounds alone.
struct Classes {
    n: [i32; 8],
    s: [i32; 8],
    ntot: i32,
    stot: i32,
}

fn classes(h: &Hole) -> Classes {
    let mut c = Classes { n: [8; 8], s: [8 * h.par; 8], ntot: 0, stot: 0 };
    for k in 1..=9 {
        for cls in 0..8 {
            let v = h.hist.get(cls * 11 + k).copied().unwrap_or(0);
            c.n[cls] += v;
            c.s[cls] += v * k as i32;
            c.ntot += v;
            c.stot += v * k as i32;
        }
    }
    c
}

impl Classes {
    fn avg(&self, c: usize) -> i32 {
        self.s[c] * 100 / self.n[c]
    }
}

/// Hole fun: mood changes per play, in percent (0 before anyone has teed off).
pub fn hole_fun(h: &Hole) -> i32 {
    if h.tee_shots == 0 {
        0
    } else {
        h.mood_sum * 100 / (h.plans / 2 + 4 + h.tee_shots)
    }
}

/// The three differentials (Length, Accuracy, Imagination) in hundredths; `all_classes` is game flag 0x40 (the report adds
/// the classes lacking two skills).
pub fn differentials(h: &Hole, all_classes: bool) -> (i32, i32, i32) {
    let c = classes(h);
    let (mut l, mut a, mut i) = (c.avg(6) - c.avg(7), c.avg(5) - c.avg(7), c.avg(3) - c.avg(7));
    if all_classes {
        l += c.avg(0) - c.avg(1);
        a += c.avg(0) - c.avg(2);
        i += c.avg(0) - c.avg(4);
    }
    (l, a, i)
}

/// The hole's type mask: each skill whose differential reaches the demand level, less the weakest when it is below `drop`.
fn type_mask(d: (i32, i32, i32), difficulty: i32, drop: i32) -> u32 {
    let t = if difficulty != 0 { 50 } else { 25 };
    let mut mask = 0;
    let mut min = 100;
    let mut minbit = 0;
    for (k, v) in [d.0, d.1, d.2].into_iter().enumerate() {
        if v >= t {
            mask |= 1 << k;
        }
        if v < min {
            min = v;
            minbit = 1 << k;
        }
    }
    if min < drop {
        mask &= !minbit;
    }
    mask
}

/// Runs the statistics pass over the club's holes: hole flags and variety counters, award and type messages (at most one a
/// pass, as the exe's message window allows), and the club ratings.
pub fn pass(club: &mut Club, difficulty: i32) -> ClubRatings {
    let mut r = ClubRatings {
        waitlist: club.members.iter().filter(|m| m.gone != 0xff && m.level & 7 > 2).count() as i32 - club.home_sites,
        ..Default::default()
    };
    let mut skill_sum = 0;
    let mut prev_type: i32 = -1;
    let mut message = false;
    for h in 1..19 {
        if club.holes[h].par == 0 {
            continue;
        }
        let hole = club.holes[h].clone();
        r.yards += hole.length;
        r.par += hole.par;
        let fun_before = r.fun;
        let skill_before = skill_sum;
        r.fun += hole_fun(&hole);
        let c = classes(&hole);
        let mut flags = hole.flags & !0xc;
        if c.ntot >= 10 {
            if c.stot > hole.par * c.ntot + ((6 - difficulty) * c.ntot) / 3 {
                flags |= 0x4;
            }
            if c.stot < hole.par * c.ntot - ((3 - difficulty) * c.ntot) / 6 {
                flags |= 0x8;
            }
        }
        flags &= !0x700;
        let d = differentials(&hole, false);
        r.len += d.0;
        r.acc += d.1;
        r.img += d.2;
        skill_sum += d.0 + d.1 + d.2;
        for (k, v) in [d.0, d.1, d.2].into_iter().enumerate() {
            if v >= 50 {
                flags |= 0x100 << k;
            }
        }
        let mask = type_mask(d, difficulty, if difficulty != 0 { 100 } else { 50 });
        if mask != 0 && club.types_announced & (1 << mask) == 0 && !message {
            message = true;
            club.types_announced |= 1 << mask;
            club.out.push(Event::Message(format!(
                "Hole #{h} has been recognized as your first \"{}\" type hole.",
                TYPE_NAMES[mask as usize].to_uppercase()
            )));
            club.out.push(Event::Sound { slot: 0x2a, at: None });
        }
        // variety: how much this hole repeats the one before it
        flags &= !0x10;
        let mut var = 0;
        if h > 1 {
            let p = &club.holes[h - 1];
            if mask as i32 == prev_type && hole.tee_shots >= 8 && mask != 7 {
                var = 1;
            }
            if (p.flags ^ flags) & 0x60 == 0 {
                var += 1;
            }
            if hole.events.get(0x2d).copied().unwrap_or(0) == 0 && hole.events.get(0x2e).copied().unwrap_or(0) == 0 {
                var += 1;
            }
            if hole.par == p.par {
                var += 1;
            }
            let a = angle(p.pin.0 - p.back.0, p.pin.1 - p.back.1);
            let b = angle(hole.pin.0 - hole.back.0, hole.pin.1 - hole.back.1);
            if ((b.wrapping_sub(a) as i32) >> 24).abs() < 40 {
                var += 1;
            }
            if var != 0 && difficulty < 2 {
                var -= 1;
            }
        }
        prev_type = mask as i32;
        // awards
        let mut score = skill_sum - skill_before;
        let f = ((r.fun - fun_before) * 6) / ((difficulty != 0) as i32 + 2);
        if difficulty < 2 && score < f {
            score = f;
        }
        if hole.fees > 200 && score > 200 && flags & 0xd == 0 && !message {
            message = true;
            flags |= 0x1;
            club.out.push(Event::Message(format!(
                "Hole #{h} has been rated as one of the Top 100 golf holes in the country by Golf Enquirer magazine!"
            )));
            club.out.push(Event::Sound { slot: 0x2e, at: None });
        }
        if hole.fees > 400 && score > 300 && flags & 0xe == 0 && !message {
            message = true;
            flags |= 0x2;
            club.out.push(Event::Message(format!(
                "Hole #{h} has been rated as one of the Top 18 golf holes in the country by Great Golf Holes magazine!"
            )));
            club.out.push(Event::Sound { slot: 0x2f, at: None });
        }
        let hr = &mut club.holes[h];
        hr.flags = flags;
        hr.monotony = var;
    }
    r.skill = skill_sum;
    club.ratings = r;
    r
}

/// One row of the Course Report (0x44fb30), for an open hole.
#[derive(Clone, Debug, Default, PartialEq)]
pub struct ReportRow {
    pub hole: usize,
    pub yards: i32,
    pub par: i32,
    /// Average strokes of finished rounds, hundredths.
    pub avg: i32,
    /// Minutes a round spends on the hole (80 golfer ticks a minute).
    pub minutes: i32,
    pub fun: i32,
    pub len: i32,
    pub acc: i32,
    pub img: i32,
    pub type_name: &'static str,
    pub scenic: bool,
    pub variety: i32,
    /// Dollars.
    pub avg_fee: i32,
    pub revenue: i32,
    pub profit: i32,
    pub flags: u32,
}

/// The Course Report's numbers for hole h.
pub fn report_row(h: usize, hole: &Hole, difficulty: i32, all_classes: bool) -> ReportRow {
    let c = classes(hole);
    let d = differentials(hole, all_classes);
    let ev = |e: usize| hole.events.get(e).copied().unwrap_or(0);
    ReportRow {
        hole: h,
        yards: hole.length,
        par: hole.par,
        avg: if c.ntot != 0 { c.stot * 100 / c.ntot } else { 0 },
        minutes: if hole.tee_shots != 0 { (hole.time / hole.tee_shots) / 40 } else { 0 },
        fun: hole_fun(hole),
        len: d.0,
        acc: d.1,
        img: d.2,
        type_name: TYPE_NAMES[type_mask(d, difficulty, 100) as usize],
        scenic: ev(22) / 2 + ev(28) + ev(11) >= 8,
        variety: hole.monotony,
        avg_fee: if c.ntot != 0 { hole.fees * 100 / c.ntot } else { 0 },
        revenue: hole.fees * 100,
        profit: (hole.fees - hole.build_cost - hole.maint) * 100,
        flags: hole.flags,
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn differentials_and_types() {
        let mut club = Club::default();
        club.holes[1].par = 4;
        // golfers lacking Length (class 6) take a stroke more than full-skill golfers (class 7)
        club.holes[1].hist[7 * 11 + 4] = 20;
        club.holes[1].hist[6 * 11 + 5] = 20;
        let d = differentials(&club.holes[1], false);
        // class 7: (32 + 80) / 28 = 4.00; class 6: (32 + 100) / 28 = 4.71
        assert_eq!(d, (471 - 400, 0, 0));
        let r = pass(&mut club, 1);
        assert_eq!(r.len, 71);
        assert_eq!(r.skill, 71);
        assert_eq!(r.par, 4);
        assert!(club.holes[1].flags & 0x100 != 0);
        assert_eq!(report_row(1, &club.holes[1], 1, false).type_name, "Freeway");
        assert!(club.types_announced & 2 != 0);
    }
}
