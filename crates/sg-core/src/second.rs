//! The Second course extra (the port's own, issue #77; off by default): once the 18 holes of the original's course are
//! open, the hole tool can build a second course on the same property, Course B, with holes B1 to B18 (36 holes in all).
//!
//! Course A is the original's course and stays exactly as it is: its hole records are `Club::holes` and its next hole
//! `Club::next_hole`, and everything the original does with holes (the SGA evaluation, tournaments and championships, the
//! club and hole ratings, the Top 18, the Best Scores, the reports of the original, the building unlocks and accomplishments
//! that come with new holes, the player's own pro) goes on with Course A only. Course B is a second set of 20 records in
//! `Club::course_b` (record 19, the walk back to the clubhouse, is a copy of Course A's), saved only once Course B has been
//! started, so a game without one saves exactly as before.
//!
//! The game's hole code works on `Club::holes` and `Club::next_hole`. To work on Course B the two are swapped with Course
//! B's for a moment (`Club::show_course`): around the update of each golfer playing Course B (`Golfer::course_b`) in
//! `Club::tick`, and around the hole tool's and the hole reports' work while the hole panel's A/B switch is on B. Outside
//! those moments Course A is always in place, so a save, a fingerprint or any other system sees the original's records.
//!
//! The tees and cup greens of both courses carry their hole number (1..18) in the tile flags' low bits; a tile belongs to
//! the course whose record of that number has its tee or pin there (`Club::tile_course`).
//!
//! Each pair sent to the first tee plays one course: Course B when its first hole is open and its tee is free and it has
//! fewer golfers on it than Course A (or Course A's first tee is taken), else Course A. Special visitors (the commissioner,
//! the CEO, the heiress, famous pros) and the player's pro play Course A. A golfer's round is that course's holes in order.

use crate::golfer::{Club, Hole, HOLE_RECORDS};

/// Course B's records (`Club::course_b`).
#[derive(Clone, Debug, Default, serde::Serialize, serde::Deserialize)]
#[serde(default)]
pub struct SecondCourse {
    /// The hole records 0..=19 as `Club::holes` has them: while Course B is shown (`Club::show_course`) this holds Course
    /// A's instead.
    pub holes: Vec<Hole>,
    /// Number of the next hole to build on it (as `Club::next_hole`), swapped with it in the same way.
    pub next_hole: i32,
}

/// Twenty empty hole records, as a new game has them.
pub fn blank_holes() -> Vec<Hole> {
    (0..HOLE_RECORDS)
        .map(|_| Hole {
            hist: vec![0; 11 * 16],
            events: vec![0; 0x50],
            event_args: vec![0; 0x50],
            markers: [(-1, -1); 3],
            ..Default::default()
        })
        .collect()
}

impl SecondCourse {
    pub fn new() -> SecondCourse {
        SecondCourse { holes: blank_holes(), next_hole: 1 }
    }
}

impl Club {
    /// Whether Course B is in `holes` and `next_hole` now (see the module notes).
    pub fn b_shown(&self) -> bool {
        self.b_shown
    }

    /// Puts Course B's records (`b`) or Course A's in `holes` and `next_hole`. Without a Course B it does nothing and Course
    /// A stays. Course B's record 19 (the walk home) is kept a copy of Course A's.
    pub fn show_course(&mut self, b: bool) {
        if b == self.b_shown {
            return;
        }
        let Some(cb) = self.course_b.as_mut() else { return };
        if cb.holes.len() < HOLE_RECORDS {
            cb.holes.resize_with(HOLE_RECORDS, || blank_holes().remove(0));
        }
        let home = if b { &self.holes[19] } else { &cb.holes[19] };
        let (par, back, fwd, pin) = (home.par, home.back, home.fwd, home.pin);
        let other = if b { &mut cb.holes[19] } else { &mut self.holes[19] };
        (other.par, other.back, other.fwd, other.pin) = (par, back, fwd, pin);
        std::mem::swap(&mut self.holes, &mut cb.holes);
        std::mem::swap(&mut self.next_hole, &mut cb.next_hole);
        self.b_shown = b;
    }

    /// Runs `f` with Course B (`b`) or Course A shown, then shows Course A again.
    pub fn on_course<R>(&mut self, b: bool, f: impl FnOnce(&mut Club) -> R) -> R {
        let was = self.b_shown;
        self.show_course(b);
        let r = f(self);
        self.show_course(was);
        r
    }

    /// Course A's 18 holes are open: with the Second course extra, Course B can be built.
    pub fn course_a_full(&self) -> bool {
        let holes = self.course_holes(false);
        (1..19).all(|h| holes[h].par != 0)
    }

    /// Whether the hole panel's A/B switch is offered: the extra is on and Course A is full, or a Course B exists.
    pub fn second_course_offered(&self) -> bool {
        self.extras.second_course && (self.course_a_full() || self.course_b.is_some())
    }

    /// Starts Course B (no holes yet) when the extra is on and Course A's 18 holes are open; returns whether one exists.
    pub fn start_course_b(&mut self) -> bool {
        if self.course_b.is_none() && self.extras.second_course && self.course_a_full() {
            self.course_b = Some(SecondCourse::new());
        }
        self.course_b.is_some()
    }

    /// A course's hole records, whichever is shown: Course B's (`b`) or Course A's. Without a Course B, B's are empty.
    pub fn course_holes(&self, b: bool) -> &[Hole] {
        match (&self.course_b, b == self.b_shown) {
            (_, true) => &self.holes,
            (Some(cb), false) => &cb.holes,
            (None, false) => &[],
        }
    }

    /// A course's next hole to build (`Club::next_hole` of Course A or B).
    pub fn course_next_hole(&self, b: bool) -> i32 {
        match (&self.course_b, b == self.b_shown) {
            (_, true) => self.next_hole,
            (Some(cb), false) => cb.next_hole,
            (None, false) => 1,
        }
    }

    /// A course's hole record `h`, whichever is shown.
    pub fn course_hole_mut(&mut self, b: bool, h: usize) -> Option<&mut Hole> {
        if b == self.b_shown {
            self.holes.get_mut(h)
        } else {
            self.course_b.as_mut().and_then(|cb| cb.holes.get_mut(h))
        }
    }

    /// A course's next hole, to set (undo puts it back).
    pub fn set_course_next_hole(&mut self, b: bool, v: i32) {
        if b == self.b_shown {
            self.next_hole = v;
        } else if let Some(cb) = self.course_b.as_mut() {
            cb.next_hole = v;
        }
    }

    /// The open holes of Course B.
    pub fn course_b_open(&self) -> usize {
        if self.course_b.is_none() {
            return 0;
        }
        let holes = self.course_holes(true);
        (1..19).filter(|&h| holes.get(h).is_some_and(|r| r.par != 0)).count()
    }

    /// Which course the tee or cup tile (a, b) with hole number `k` in its flags belongs to: true for Course B, when
    /// Course B's record `k` has its tee or pin there and Course A's does not.
    pub fn tile_course(&self, k: usize, a: i32, b: i32) -> bool {
        if self.course_b.is_none() || !(1..19).contains(&k) {
            return false;
        }
        // (a forward or middle tee of the Multiple tees extra too)
        let at = |r: &Hole| r.back == (a, b) || r.fwd == (a, b) || r.pin == (a, b) || r.extra_tees().any(|(_, p)| p == (a, b));
        let on_a = self.course_holes(false).get(k).is_some_and(at);
        let on_b = self.course_holes(true).get(k).is_some_and(at);
        on_b && !on_a
    }

    /// Hole `h` as the messages name it: "B3" while Course B is shown, else the number as the original has it.
    pub fn hole_label(&self, h: i32) -> String {
        if self.b_shown {
            crate::lang::trf("STR_COURSE_B_HOLE", &[("HOLE", &h)])
        } else {
            h.to_string()
        }
    }

    /// Golfers out on each course (holes 1 to 18): Course A's and Course B's.
    pub(crate) fn golfers_per_course(&self) -> (usize, usize) {
        let mut n = (0, 0);
        for g in self.g.iter().take(crate::golfer::SLOTS).filter(|g| (1..19).contains(&g.hole)) {
            if g.course_b {
                n.1 += 1;
            } else {
                n.0 += 1;
            }
        }
        n
    }

    /// Whether golfers o and g play the same course (always, without a Course B).
    pub(crate) fn same_course(&self, o: usize, g: usize) -> bool {
        self.g[o].course_b == self.g[g].course_b
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::course::{idx, t, Course};
    use crate::golfer::SLOTS;
    use crate::land::ExeRng;

    fn course() -> Course {
        let mut c = Course::default();
        for v in c.ty.iter_mut() {
            *v = t::FAIRWAY;
        }
        c.door = (2, 2);
        c
    }

    /// Builds and opens a short hole from tee (a, b0) to the green at (a, b0 + 3) on the course shown.
    fn build(cl: &mut Club, c: &mut Course, a: i32, b0: i32) {
        assert!(cl.paint_hole_tile(c, a, b0, t::FAIRWAY, t::TEE, false));
        c.ty[idx(a, b0)] = t::TEE;
        assert!(cl.paint_hole_tile(c, a, b0 + 3, t::FAIRWAY, t::GREEN, false));
        c.ty[idx(a, b0 + 3)] = t::GREEN;
        assert!(cl.open_hole(c).is_some());
    }

    /// A club with Course A's 18 holes in columns 2..37 (rows 5..8), the home record at the door, the extra on.
    fn full_club(c: &mut Course) -> Club {
        let mut cl = Club::default();
        cl.extras.second_course = true;
        let home = &mut cl.holes[19];
        (home.back, home.fwd, home.pin) = (c.door, c.door, c.door);
        for h in 0..18 {
            build(&mut cl, c, 2 + 2 * h, 5);
        }
        assert_eq!(cl.next_hole, 19);
        cl.out.clear();
        cl
    }

    #[test]
    fn course_b_is_built_with_the_hole_tool_once_course_a_is_full() {
        let mut c = course();
        let mut cl = Club::default();
        cl.extras.second_course = true;
        assert!(!cl.start_course_b(), "not before Course A is full");
        let mut cl = full_club(&mut c);
        // Course A refuses a nineteenth hole, as the original does
        assert!(!cl.paint_hole_tile(&mut c, 2, 20, t::FAIRWAY, t::TEE, false));
        assert!(cl.start_course_b());
        cl.on_course(true, |cl| build(cl, &mut c, 2, 20));
        assert!(!cl.b_shown());
        assert_eq!(cl.next_hole, 19, "Course A is in place again");
        assert_eq!(cl.course_next_hole(true), 2);
        assert_eq!(cl.course_b_open(), 1);
        assert_eq!(c.flags[idx(2, 20)] & crate::course::f::LOW, 1, "B1's tee is numbered 1");
        assert!(cl.tile_course(1, 2, 20) && !cl.tile_course(1, 2, 5));
        // painting over B1's tee closes B1, not hole 1 of Course A
        cl.on_course(true, |cl| assert!(cl.paint_hole_tile(&mut c, 2, 20, t::TEE, t::ROUGH, false)));
        assert_eq!(cl.course_holes(true)[1].par, 0);
        assert_ne!(cl.holes[1].par, 0);
        assert_eq!(cl.course_next_hole(true), 1);
        // and painting over Course A's hole 2 tee while Course B is shown closes Course A's hole 2
        cl.on_course(true, |cl| assert!(cl.paint_hole_tile(&mut c, 4, 5, t::TEE, t::ROUGH, false)));
        assert_eq!(cl.holes[2].par, 0);
        assert_eq!(cl.next_hole, 2);
    }

    /// A 36-hole course: golfers arrive and play; some go to Course B and play B1, B2, B3 in order, then go home.
    #[test]
    fn golfers_play_course_b_holes_in_order() {
        let mut c = course();
        let mut cl = full_club(&mut c);
        cl.start_course_b();
        cl.on_course(true, |cl| {
            for h in 0..3 {
                build(cl, &mut c, 2 + 2 * h, 30);
            }
        });
        cl.out.clear();
        for r in 1..cl.members.len().min(70) {
            cl.members[r].level = 2;
        }
        let mut rng = ExeRng::from_clock(7);
        let mut seen: Vec<Vec<i32>> = vec![Vec::new(); SLOTS];
        let mut b_golfers = 0;
        let mut b_rounds = 0;
        for tick in 1..40_000u32 {
            cl.tick(&mut c, &mut rng, tick);
            cl.out.clear();
            assert!(!cl.b_shown());
            for (s, g) in cl.g.iter().take(SLOTS).enumerate() {
                if g.course_b && (1..19).contains(&g.hole) && seen[s].last() != Some(&g.hole) {
                    if seen[s].is_empty() {
                        b_golfers += 1;
                    }
                    seen[s].push(g.hole);
                }
                if g.hole == 0 && !seen[s].is_empty() {
                    assert_eq!(seen[s], (1..=seen[s].len() as i32).collect::<Vec<_>>(), "golfer {s} played B holes in order");
                    b_rounds += (seen[s].len() == 3) as i32;
                    seen[s].clear();
                }
            }
        }
        assert!(b_golfers >= 2, "golfers went to Course B ({b_golfers})");
        assert!(b_rounds >= 2, "golfers played B1 to B3 and went home ({b_rounds} of {b_golfers})");
        let b = cl.course_holes(true);
        assert!(b[1].tee_shots > 0 || b[1].fees > 0, "B1 has been played");
        assert!(cl.holes[1].fees > 0, "Course A is played too");
    }

    #[test]
    fn a_club_without_course_b_saves_as_before_and_old_saves_load() {
        let mut cl = Club::default();
        let plain = serde_json::to_string(&cl).unwrap();
        assert!(!plain.contains("course_b"));
        // a golfer on Course A saves as before
        cl.g[3].hole = 2;
        let s = serde_json::to_string(&cl).unwrap();
        assert!(!s.contains("course_b"));
        let back: Club = serde_json::from_str(&plain).unwrap();
        assert!(back.course_b.is_none() && !back.b_shown());
        // with a Course B, it comes back
        cl.course_b = Some(SecondCourse::new());
        cl.course_b.as_mut().unwrap().holes[4].par = 3;
        cl.g[3].course_b = true;
        let back: Club = serde_json::from_str(&serde_json::to_string(&cl).unwrap()).unwrap();
        assert_eq!(back.course_holes(true)[4].par, 3);
        assert!(back.g[3].course_b);
    }

    /// With the extra off, a full Course A plays as the original: nobody goes to Course B, nothing is started.
    #[test]
    fn extra_off_changes_nothing() {
        let mut c = course();
        let mut c2 = course();
        let mut on = full_club(&mut c);
        let mut off = full_club(&mut c2);
        off.extras.second_course = false;
        on.extras.second_course = false;
        assert!(!off.start_course_b() && !off.second_course_offered());
        let (mut r1, mut r2) = (ExeRng::from_clock(3), ExeRng::from_clock(3));
        // one club holds an empty Course B (as after turning the extra off): it plays the same
        on.course_b = Some(SecondCourse::new());
        for tick in 1..6000u32 {
            on.tick(&mut c, &mut r1, tick);
            off.tick(&mut c2, &mut r2, tick);
        }
        assert!(on.g.iter().all(|g| !g.course_b));
        on.course_b = None;
        assert_eq!(serde_json::to_string(&on).unwrap(), serde_json::to_string(&off).unwrap());
    }
}
