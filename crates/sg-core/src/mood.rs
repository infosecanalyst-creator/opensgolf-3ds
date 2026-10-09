//! Golfer mood events, as the publisher's golf.exe defines them (docs/PUBLISHER_EXE_NOTES.md, "Golfer mood events").
//!
//! One routine takes (golfer, event, argument), changes the golfer's mood by a small amount and shows the golfer's thought. The
//! amounts below are read from that routine; the meaning of each event comes from the thought the routine shows for it (described
//! here in our own words). Events whose amount depends on a golfer counter or flag return the plain amount and say so.

/// Event ids used by the exe. Only events with a known meaning are named.
pub mod ev {
    /// Holed a difficult putt.
    pub const TOUGH_PUTT_MADE: u32 = 1;
    /// Missed a putt that looked easy.
    pub const EASY_PUTT_MISSED: u32 = 2;
    /// Unhappy with how the last shot turned out.
    pub const BAD_SHOT: u32 = 3;
    /// The next shot will not involve what the golfer hoped for.
    pub const DULL_NEXT_SHOT: u32 = 4;
    /// Plans to use the slope of the hill.
    pub const USES_SLOPE: u32 = 6;
    /// Positive event whose amount applies only when the argument is 0.
    pub const EVENT_7: u32 = 7;
    /// Cannot find a safe place to aim.
    pub const NO_SAFE_AIM: u32 = 8;
    /// Was nearly hit by another group's shot.
    pub const NEARLY_HIT: u32 = 9;
    /// No path has been built where the golfer wants to walk.
    pub const NO_PATH: u32 = 0xa;
    /// Enjoys a scenic view in sight of the next shot.
    pub const SCENIC_VIEW: u32 = 0xb;
    /// The ball bounced off a tree.
    pub const HIT_TREE: u32 = 0xc;
    /// The ball ended up in a hazard (water).
    pub const IN_HAZARD: u32 = 0xd;
    /// Thirsty: needs a soda vendor soon.
    pub const THIRSTY: u32 = 0xe;
    /// Hungry: needs a snack bar soon.
    pub const HUNGRY: u32 = 0xf;
    /// Amazed that the last shot drew.
    pub const SHOT_DREW: u32 = 0x10;
    /// Amazed that the last shot faded.
    pub const SHOT_FADED: u32 = 0x11;
    /// Eating a snack at a snack bar (+1 only once a golfer counter is above 7).
    pub const SNACK: u32 = 0x12;
    /// Sees something ugly in the middle of the hole.
    pub const EYESORE: u32 = 0x14;
    /// Frustrated by slow play (a Ranger helps).
    pub const SLOW_PLAY: u32 = 0x15;
    /// Excited to see a celebrity's vacation home on the course.
    pub const CELEBRITY_HOME: u32 = 0x16;
    /// -1, or -2 above the easiest difficulty.
    pub const EVENT_17: u32 = 0x17;
    /// Sees weeds (a Groundskeeper removes them).
    pub const WEEDS: u32 = 0x18;
    /// A refreshing drink from a Soda Vendor (+1 only once a golfer counter is above 7).
    pub const DRINK: u32 = 0x19;
    /// Tired: needs a bench to rest on.
    pub const NEEDS_BENCH: u32 = 0x1a;
    /// Resting on a bench (+1 only once a golfer counter is above 59).
    pub const BENCH_REST: u32 = 0x1b;
    /// Enjoys a feature the player built into the course.
    pub const ENJOYS_FEATURE: u32 = 0x1c;
    /// Likes something about this hole.
    pub const LIKES_HOLE: u32 = 0x1d;
    /// Dislikes something about this hole.
    pub const DISLIKES_HOLE: u32 = 0x1e;
    /// Faces an interesting (strategic) decision.
    pub const INTERESTING_DECISION: u32 = 0x21;
    /// Quits playing in frustration.
    pub const QUITS: u32 = 0x23;
    /// Sees an angry golfer.
    pub const SEES_ANGRY_GOLFER: u32 = 0x24;
    /// Intrigued by the wildlife (+1 on the two easiest difficulties).
    pub const WILDLIFE: u32 = 0x27;
    /// Appreciates a new driving range, pro shop or putting green (+1 on the easiest difficulty).
    pub const NEW_DRIVING_RANGE: u32 = 0x33;
    pub const NEW_PRO_SHOP: u32 = 0x34;
    pub const NEW_PUTTING_GREEN: u32 = 0x35;
    /// Uses a club for the first time (+1 on the two easiest difficulties).
    pub const FIRST_TIME_CLUB: u32 = 0x36;
    /// Hit a good shot but cannot see where it went (-2 above difficulty 1).
    pub const BLIND_SHOT: u32 = 0x41;
}

/// Mood change for an event. `arg` is the routine's third argument; `counter_ok` says whether the golfer counter that some events
/// test has passed its threshold (snack and drink: above 7; bench: above 59).
pub fn delta(event: u32, difficulty: i32, arg: i32, counter_ok: bool) -> i32 {
    use ev::*;
    match event {
        TOUGH_PUTT_MADE | USES_SLOPE | SCENIC_VIEW | CELEBRITY_HOME | ENJOYS_FEATURE | LIKES_HOLE | 0x20 | INTERESTING_DECISION | 0x22
        | 0x2c | 0x2e => 1,
        EVENT_7 => (arg == 0) as i32,
        EASY_PUTT_MISSED | HIT_TREE | IN_HAZARD | THIRSTY | HUNGRY | NEEDS_BENCH => -1,
        BAD_SHOT | DULL_NEXT_SHOT | NO_SAFE_AIM | NO_PATH | 0x2b | EYESORE | SLOW_PLAY | WEEDS | DISLIKES_HOLE | QUITS | 0x2f => -2,
        NEARLY_HIT | SEES_ANGRY_GOLFER => -3,
        EVENT_17 => -1 - (difficulty != 0) as i32,
        SNACK | DRINK | BENCH_REST => counter_ok as i32,
        WILDLIFE | FIRST_TIME_CLUB => (difficulty < 2) as i32,
        NEW_DRIVING_RANGE | NEW_PRO_SHOP | NEW_PUTTING_GREEN => (difficulty == 0) as i32,
        BLIND_SHOT if difficulty > 1 => -2,
        _ => 0,
    }
}

/// Mood bounds: the routine clamps the mood to -10..10. A golfer whose mood is below 0 between shots quits.
pub const MOOD_MIN: i32 = -10;
pub const MOOD_MAX: i32 = 10;

/// Applies an event to a mood value. Returns the new mood and whether the golfer quits (mood below 0).
pub fn apply(mood: i32, delta: i32) -> (i32, bool) {
    let m = (mood + delta).clamp(MOOD_MIN, MOOD_MAX);
    (m, m < 0)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn amounts_from_the_exe() {
        assert_eq!(delta(ev::TOUGH_PUTT_MADE, 1, 0, false), 1);
        assert_eq!(delta(ev::EASY_PUTT_MISSED, 1, 0, false), -1);
        assert_eq!(delta(ev::BAD_SHOT, 1, 0, false), -2);
        assert_eq!(delta(ev::NEARLY_HIT, 1, 0, false), -3);
        assert_eq!(delta(ev::EVENT_17, 0, 0, false), -1);
        assert_eq!(delta(ev::EVENT_17, 2, 0, false), -2);
        assert_eq!(delta(ev::WILDLIFE, 1, 0, false), 1);
        assert_eq!(delta(ev::WILDLIFE, 2, 0, false), 0);
        assert_eq!(delta(ev::DRINK, 1, 0, false), 0);
        assert_eq!(delta(ev::DRINK, 1, 0, true), 1);
        assert_eq!(delta(ev::BLIND_SHOT, 3, 0, false), -2);
    }

    #[test]
    fn quitting_below_zero() {
        assert_eq!(apply(-9, -2), (-10, true));
        assert_eq!(apply(1, -2), (-1, true));
        assert_eq!(apply(9, 3), (10, false));
    }
}
