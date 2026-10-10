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
        // posted with priority 0 and the site picture (-4): refused while the ticker is busy, and then nothing is marked
        if mask != 0
            && club.types_announced & (1 << mask) == 0
            && !message
            && club.message_by(
                format!("Hole #{h} has been recognized as your first \"{}\" type hole.", TYPE_NAMES[mask as usize].to_uppercase()),
                -4,
                0,
            )
        {
            message = true;
            club.types_announced |= 1 << mask;
            club.out.push(Event::Sound { slot: 0x2a, at: None, delay: 0 });
            let green = (hole.pin.0 * 1024 + 512, hole.pin.1 * 1024 + 512);
            match mask {
                3 if difficulty >= 2 => club.award_at(0, green),
                5 if difficulty >= 2 => club.award_at(1, green),
                6 if difficulty >= 2 => club.award_at(4, green),
                7 => club.award_at(9, green),
                _ => false,
            };
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
        // the Top 100 and Top 18 notices: priority 0 with the laurel ball icon (-21), marked only when shown
        if hole.fees > 200
            && score > 200
            && flags & 0xd == 0
            && !message
            && club.message_by(
                // the hole takes its proper name now (0x80 set first, 0x416f5a)
                format!(
                    "Hole #{h} (henceforth known as '{}'), has been rated as one of the best 100 holes in the country by Golf \
                     Enquirer magazine! Increase greens fees by \u{a7}100.",
                    name_of(h, hole.par, flags | 0x80)
                ),
                -21,
                0,
            )
        {
            message = true;
            flags |= 0x1;
            club.out.push(Event::Sound { slot: 0x2e, at: None, delay: 0 });
            club.award_at(7, (hole.pin.0 * 1024 + 512, hole.pin.1 * 1024 + 512));
            club.log_event(crate::records::log::TOP100, h as i32);
        }
        if hole.fees > 400
            && score > 300
            && flags & 0xe == 0
            && !message
            && club.message_by(
                format!(
                    "{} has been rated as one of the Top 18 holes in the country by Great Golf Holes magazine! Increase \
                     greens fees by \u{a7}100.",
                    name_of(h, hole.par, flags)
                ),
                -21,
                0,
            )
        {
            message = true;
            flags |= 0x2;
            club.out.push(Event::Sound { slot: 0x2f, at: None, delay: 0 });
            club.award_at(10, (hole.pin.0 * 1024 + 512, hole.pin.1 * 1024 + 512));
            club.log_event(crate::records::log::TOP18, h as i32);
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

/// Proper hole names, indexed by hole number, one table per par (0x407280 picks the table by the hole's par byte): women's
/// names for par 3 (pointer table 0x4c2e88), trees and shrubs for par 4 (0x4c2e38), dark places for the rest (0x4c2ed8).
const NAMES_PAR3: [&str; 18] = [
    "Alexandra",
    "Belinda",
    "Carmen",
    "Dorothy",
    "Elizabeth",
    "Faith",
    "Gisele",
    "Hope",
    "Ingrid",
    "Jacqueline",
    "Lara",
    "Melinda",
    "Naomi",
    "Priscilla",
    "Roxanne",
    "Sarah",
    "Valerie",
    "Zelda",
];
const NAMES_PAR4: [&str; 18] = [
    "Olive",
    "Dogwood",
    "Peach",
    "Crab Apple",
    "Magnolia",
    "Juniper",
    "Pampas",
    "Jasmime",
    "Cherry",
    "Camellia",
    "Daisy",
    "Golden Bell",
    "Azalea",
    "Chinese Fir",
    "Firethorn",
    "Redbud",
    "Nandina",
    "Holly",
];
const NAMES_OTHER: [&str; 18] = [
    "Pride",
    "Inferno",
    "Fortress",
    "The Siren",
    "Centaur",
    "Styx",
    "Flame",
    "Serpent",
    "Paradise",
    "Eden",
    "Valley",
    "Torment",
    "Misery",
    "Archangel",
    "Smoke",
    "Gryphon",
    "Beast",
    "Purgatory",
];

/// A hole's name (0x407280): "Hole N" ("Hole " and the number), or once it is a Top 100 hole (or its naming is pending,
/// flag 0x80) the name the player gave it, else the proper name of its number in the table of its par. (The port keeps no
/// player-given hole names.)
pub fn hole_name(h: usize, hole: &Hole) -> String {
    name_of(h, hole.par, hole.flags)
}

fn name_of(h: usize, par: i32, flags: u32) -> String {
    if flags & 0x81 == 0 || !(1..=18).contains(&h) {
        return format!("Hole {h}");
    }
    match par {
        3 => NAMES_PAR3[h - 1],
        4 => NAMES_PAR4[h - 1],
        _ => NAMES_OTHER[h - 1],
    }
    .to_string()
}

/// The Hole Stats dialog's numbers (0x453330).
#[derive(Clone, Debug, Default, PartialEq)]
pub struct HoleStats {
    /// Fun percent, shown only when a shot was planned here.
    pub fun: Option<i32>,
    /// Length, Accuracy and Imagination differentials over the visible histogram columns, hundredths.
    pub len: i32,
    pub acc: i32,
    pub img: i32,
    /// The six columns: stroke value and finished rounds (the last column holds that many strokes and more, up to 9).
    pub cols: [(i32, i32); 6],
    /// Average strokes over the visible columns, hundredths (None without a round in them).
    pub avg: Option<i32>,
    /// The five most frequent remarks: event type, percent of rounds, the last stored argument.
    pub comments: Vec<(u32, i32, i32)>,
}

/// The dialog's word for a fun percent.
pub fn fun_word(v: i32) -> &'static str {
    match v {
        v if v < 0 => "poor",
        0..=19 => "fair",
        20..=39 => "good",
        40..=59 => "very good",
        _ => "outstanding",
    }
}

/// The dialog's word for a differential in hundredths.
pub fn demand_word(v: i32) -> &'static str {
    match v {
        v if v < 0 => "poor",
        0..=24 => "fair",
        25..=49 => "good",
        50..=99 => "very good",
        _ => "outstanding",
    }
}

/// Computes the Hole Stats dialog: the histogram starts at max(par - 2, 1) and only its columns enter the differentials
/// (with the 8 rounds at par of the report) and the stroke average.
pub fn hole_stats(hole: &Hole) -> HoleStats {
    let first = (hole.par - 2).max(1) as usize;
    let bin = |cls: usize, k: usize| hole.hist.get(cls * 11 + k).copied().unwrap_or(0);
    let mut n = [8i32; 8];
    let mut s = [8 * hole.par; 8];
    let (mut cnt, mut sum) = (0, 0);
    for k in first..=9 {
        for c in 0..8 {
            let v = bin(c, k);
            n[c] += v;
            s[c] += v * k as i32;
            cnt += v;
            sum += v * k as i32;
        }
    }
    let avg = |c: usize| s[c] * 100 / n[c];
    let mut cols = [(0, 0); 6];
    for (i, col) in cols.iter_mut().enumerate() {
        let k = first + i;
        let top = if i == 5 { 9 } else { k };
        *col = (k as i32, (k..=top).map(|b| (0..8).map(|c| bin(c, b)).sum::<i32>()).sum());
    }
    let mut comments = Vec::new();
    if hole.tee_shots > 0 {
        let mut ev: Vec<i32> = (0..50).map(|e| hole.events.get(e).copied().unwrap_or(0)).collect();
        for _ in 0..5 {
            let (e, c) = ev.iter().enumerate().fold((0, 0), |b, (e, &c)| if c > b.1 { (e, c) } else { b });
            if c <= 0 {
                break;
            }
            ev[e] = 0;
            let arg = hole.event_args.get(e).copied().unwrap_or(0) & 0x3fff;
            comments.push((e as u32, c * 100 / hole.tee_shots, arg));
        }
    }
    HoleStats {
        fun: if hole.plans != 0 { Some(hole_fun(hole)) } else { None },
        len: avg(6) - avg(7),
        acc: avg(5) - avg(7),
        img: avg(3) - avg(7),
        cols,
        avg: if cnt > 0 { Some(sum * 100 / cnt) } else { None },
        comments,
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn hole_stats_columns() {
        let mut h = Hole { par: 4, hist: vec![0; 88], events: vec![0; 64], event_args: vec![0; 64], ..Default::default() };
        h.hist[7 * 11 + 4] = 10;
        h.hist[6 * 11 + 5] = 10;
        h.hist[7 * 11 + 1] = 3; // a hole in one: left of the first column (2), so it counts nowhere
        h.hist[7 * 11 + 9] = 1;
        h.tee_shots = 20;
        h.events[11] = 4;
        h.events[3] = 4;
        h.events[55] = 9;
        let st = hole_stats(&h);
        assert_eq!(st.cols[0], (2, 0));
        assert_eq!(st.cols[2], (4, 10));
        assert_eq!(st.cols[5], (7, 1));
        assert_eq!(st.avg, Some((40 + 50 + 9) * 100 / 21));
        assert_eq!(st.fun, None);
        // the lowest type wins a tie; types from 50 never show
        assert_eq!(st.comments, vec![(3, 20, 0), (11, 20, 0)]);
        assert_eq!(hole_name(3, &h), "Hole 3");
        h.flags |= 1;
        assert_eq!(hole_name(3, &h), "Peach");
        h.par = 3;
        assert_eq!(hole_name(18, &h), "Zelda");
        h.par = 5;
        assert_eq!(hole_name(1, &h), "Pride");
        assert_eq!(demand_word(57), "very good");
    }

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
