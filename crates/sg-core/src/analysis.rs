//! The instant shot analysis of the course screen's '/' and '.' keys (0x41d078 in the exe's main routine): twelve trial shots
//! from the tile under the pointer toward a hole's pin, three for each of four kinds of golfer (no Length, no Accuracy, no
//! Imagination, all three skills), with how far from the pin each kind ends up compared with the golfers who have every skill.
//!
//! Facts are from the publisher's golf.exe, restated in our own words.

use crate::course::{idx, inside, Course};
use crate::geom::{len, tdist};
use crate::golfer::{flag, Club, Golfer};
use crate::land::ExeRng;

/// The golfer record the analysis plans with (0x99).
pub const ANALYST: usize = 0x99;

/// The four kinds of golfer, in the order the shots cycle through them: their class bits (0x40d760's argument), the label
/// and the colour word (0x40d760) the label, the ball track and the landing circle are drawn in.
pub const KINDS: [(u8, &str, u32); 4] = [
    (6, "no Length skill", 0x7ff0),
    (5, "no Accuracy skill", 0x421f),
    (3, "no Imagination skill", 0x43f0),
    (7, "golfers with ALL skills", 0x7fff),
];

/// One trial shot: the kind it was played by (index into [`KINDS`]), the ball's trace in map units (the first point is the
/// spot it was hit from), and the hazard byte of the type it came to rest on (0 or less: fine).
#[derive(Clone, Debug, Default)]
pub struct Shot {
    pub kind: usize,
    pub trace: Vec<(i32, i32)>,
    pub hazard: i32,
}

impl Shot {
    /// Where the analysis takes the ball to have stopped: the trace's last point.
    pub fn end(&self) -> (i32, i32) {
        self.trace.last().copied().unwrap_or_default()
    }
}

/// The analysis of one press: the hole, the tile the shots are played from, the shots, and per kind the summed distance to
/// the pin in yards with 10 more for each point of the hazard byte where the ball stopped.
#[derive(Clone, Debug, Default)]
pub struct Analysis {
    pub hole: i32,
    pub from: (i32, i32),
    pub shots: Vec<Shot>,
    pub sums: [i32; 4],
}

impl Analysis {
    /// The figure printed for kind k (0..2) beside its label: (all skills - k) / 3 yards, with a '+' when positive.
    pub fn figure(&self, k: usize) -> String {
        let v = (self.sums[3] - self.sums[k]) / 3;
        format!("{}{v} yds.", if self.sums[k] < self.sums[3] { "+" } else { "" })
    }
}

impl Club {
    /// The hole nearest tile (a, b) (0x407340): by straight distance in tiles to each of the 18 records' back tee and pin
    /// (unbuilt holes count at tile 0, 0, as in the exe) and to its 250 yard marker when it has one; the first one wins a tie.
    /// -1 when none is nearer than 0xffff.
    pub fn nearest_hole(&self, a: i32, b: i32) -> i32 {
        let mut best = 0xffff;
        let mut hole = -1;
        for h in 1..19 {
            let r = &self.holes[h];
            let mut try_at = |dd: i32| {
                if dd < best {
                    best = dd;
                    hole = h as i32;
                }
            };
            try_at(len(r.back.0 - a, r.back.1 - b));
            try_at(len(r.pin.0 - a, r.pin.1 - b));
            let m = r.markers[2];
            if m.0 != -1 {
                try_at(len(a - (m.0 >> 10), b - (m.1 >> 10)));
            }
        }
        hole
    }

    /// Plays the twelve trial shots of the analysis from tile (a, b) toward hole `hole`'s pin (0x41d0e3..0x41d8f3). Shot i is
    /// played by kind i & 3 at level i / 4 (level 1 for all on the easiest difficulty); a kind's golfer has visited the range,
    /// the pro shop or the putting green when its class has the matching skill and the club runs that building.
    pub fn shot_analysis(&mut self, c: &mut Course, rng: &mut ExeRng, a: i32, b: i32, hole: i32) -> Analysis {
        let hu = hole.clamp(0, crate::golfer::HOLE_RECORDS as i32 - 1) as usize;
        let mut out = Analysis { hole, from: (a, b), ..Default::default() };
        let (x, y) = (a * 0x400 + 0x200, b * 0x400 + 0x200);
        let on_tee = inside(a, b) && c.ty[idx(a, b)] == crate::course::t::TEE;
        let saved = self.g[ANALYST].clone();
        let saved_mode = self.planner.mode;
        for i in 0..12usize {
            let k = i & 3;
            let class = KINDS[k].0;
            let mut flags = 0;
            if c.oper[6] != 0 && class & 4 != 0 {
                flags = flag::VISITED_PUTTING;
            }
            if c.oper[10] != 0 && class & 1 != 0 {
                flags |= flag::VISITED_RANGE;
            }
            if c.oper[8] != 0 && class & 2 != 0 {
                flags |= flag::VISITED_SHOP;
            }
            self.g[ANALYST] = Golfer {
                class,
                level: if self.difficulty != 0 { (i as i32 / 4) & 3 } else { 1 },
                hole,
                strokes: (!on_tee) as i32,
                flags,
                x,
                y,
                ox: x,
                oy: y,
                bx: x,
                by: y,
                aim_a: -1,
                ..Default::default()
            };
            // planning counts as a plan of the hole; the exe puts the count back
            let plans = self.holes[hu].plans;
            crate::planner::plan_shot(self, c, rng, ANALYST, false, -1, 0, 0);
            self.holes[hu].plans = plans;
            let gg = &self.g[ANALYST];
            let shape = match gg.shot_type {
                1 => 1,
                -1 => -1,
                _ => 0,
            };
            let (tx, ty) = if gg.flags & flag::CORNER_AIM != 0 {
                (gg.aim_a << 10, gg.aim_b << 10)
            } else {
                ((gg.aim_a << 10) + 0x200, (gg.aim_b << 10) + 0x200)
            };
            self.planner.trace = Some(vec![(gg.ox, gg.oy)]);
            self.planner.mode = 0;
            crate::planner::preview(self, c, rng, ANALYST, tx, ty, shape);
            let trace = self.planner.trace.take().unwrap_or_default();
            let end = trace.last().copied().unwrap_or((x, y));
            let pin = self.holes[hu].pin;
            let hazard = c.h(c.tile_type(end.0, end.1));
            out.sums[k] += tdist(end.0, end.1, pin.0, pin.1) + hazard * 10;
            out.shots.push(Shot { kind: k, trace, hazard });
        }
        self.planner.mode = saved_mode;
        self.g[ANALYST] = saved;
        out
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn figures_sign_and_divide() {
        let a = Analysis { sums: [300, 330, 291, 300], ..Default::default() };
        assert_eq!(a.figure(0), "0 yds.");
        assert_eq!(a.figure(1), "-10 yds.");
        assert_eq!(a.figure(2), "+3 yds.");
    }

    #[test]
    fn nearest_hole_prefers_the_first_on_a_tie() {
        let mut cl = Club::default();
        cl.holes[2].back = (10, 10);
        cl.holes[2].pin = (20, 10);
        cl.holes[3].back = (10, 12);
        cl.holes[3].pin = (30, 30);
        assert_eq!(cl.nearest_hole(10, 11), 2);
        assert_eq!(cl.nearest_hole(19, 10), 2);
        // unbuilt holes sit at tile 0, 0
        assert_eq!(cl.nearest_hole(0, 1), 1);
    }
}
