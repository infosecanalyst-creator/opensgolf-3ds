//! The clubhouse screen "SELECT THE NEXT PAIR OF GOLFERS" (0x459850): the golfers waiting in the clubhouse, newest first,
//! of whom the player may pick two to tee off next.
//!
//! Facts are from the publisher's golf.exe (docs/PUBLISHER_EXE_NOTES.md, "The clubhouse and the golfer screens"),
//! restated in our own words.

use crate::course::Course;
use crate::golfer::{Club, SLOTS};
use crate::land::ExeRng;

impl Club {
    /// The golfers waiting in the clubhouse, newest first (the screen's order); the last is the one waiting longest.
    pub fn waiting(&self) -> Vec<usize> {
        (0..SLOTS)
            .map(|i| (self.last_created - i as i32 + SLOTS as i32).rem_euclid(SLOTS as i32) as usize)
            .filter(|&s| self.g[s].hole == -1)
            .collect()
    }

    /// The two picked golfers move into the slots of the longest-waiting golfer's pair, are marked as picked by hand and tee
    /// off at once (the exe does not wait for the first tee to clear). `picks` are slots from `waiting()`.
    pub fn pick_pair(&mut self, c: &mut Course, rng: &mut ExeRng, a: usize, b: usize) -> bool {
        let list = self.waiting();
        let Some(&s) = list.last() else { return false };
        if a == b || !list.contains(&a) || !list.contains(&b) {
            return false;
        }
        let (first, second) = if b == s {
            let t = s ^ 1;
            self.g.swap(a, t);
            (t, s)
        } else {
            self.g.swap(a, s);
            self.g.swap(b, s ^ 1);
            (s, s ^ 1)
        };
        for g in [first, second] {
            self.g[g].flags |= crate::stories::MANUAL_PAIR;
        }
        debug_assert_eq!(first ^ 1, second);
        self.start_pair(c, rng, first);
        true
    }

    /// The marital word of a golfer (+0xb8 bits; later bits win).
    pub fn marital(&self, g: usize) -> &'static str {
        let b = self.g[g].looks as u32;
        let mut w = "Single";
        for (bit, word) in [(0x08, "Single"), (0x10, "Married"), (0x20, "Divorced"), (0x40, "Widowed")] {
            if b & bit != 0 {
                w = word;
            }
        }
        w
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn hand_picked_pair_tees_off() {
        let mut cl = Club::default();
        let mut c = Course::default();
        let mut rng = ExeRng::from_clock(2);
        cl.holes[1].par = 4;
        cl.next_hole = 2;
        for s in 4..10 {
            cl.g[s].hole = -1;
            cl.g[s].roster = s as i32;
            cl.g[s].partner = s as i32;
        }
        cl.last_created = 9;
        let list = cl.waiting();
        assert_eq!(list, vec![9, 8, 7, 6, 5, 4]);
        // pick the two newest: they take the slots of the longest-waiting golfer's pair (4, 5) and tee off
        assert!(cl.pick_pair(&mut c, &mut rng, 9, 8));
        assert_eq!((cl.g[4].roster, cl.g[5].roster), (9, 8));
        assert_eq!((cl.g[4].hole, cl.g[5].hole), (1, 1));
        assert_eq!(cl.g[4].partner, 5);
        assert_eq!(cl.waiting().len(), 4);
    }
}
