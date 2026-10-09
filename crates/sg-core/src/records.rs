//! The club's records: the pro's Professional Accomplishments (table 0x4c1578, award 0x46e7b0), the monthly event log
//! (0x568600, log 0x40c6f0) the year-end report and the Histograph read, and the monthly history of cash, fun, skill and
//! membership (charge interval, main frame 0x417a08).
//!
//! Facts are from the publisher's golf.exe (docs/PUBLISHER_EXE_NOTES.md, "Accomplishments, event log and year end"),
//! restated in our own words.

use crate::golfer::Club;

/// The 22 accomplishments, as the plaques name them; on Easy and Moderate ids 0, 1 and 4 are the three alternatives.
pub const TITLES: [&str; 22] = [
    "1st Challenge Hole",
    "1st Heroic Hole",
    "1st Skill Upgrade",
    "1st Tournament",
    "1st Strategic Hole",
    "1st Match Victory",
    "1st 9+ Hole Course",
    "1st Top 100 Hole",
    "1st $500,000 Tournament",
    "1st Classic Hole",
    "1st Top 18 Hole",
    "1st Tournament Victory (9+ Hole)",
    "1st Grand Slam Course (9+ holes)",
    "1st $1,000,000 Tournament",
    "1st 18 Hole Course",
    "1st Tournament Victory (18 Hole)",
    "1st Grand Slam Course (18 Hole)",
    "1st Grand Slam Victory (Parkland)",
    "1st Grand Slam Victory (Desert)",
    "1st Grand Slam Victory (Tropical)",
    "1st Grand Slam Victory (Links)",
    "1st 100 Star Rating",
];
pub const EASY_TITLES: [&str; 3] = ["1st Dogleg Right Hole", "1st Dogleg Left Hole", "1st Par 5 Hole"];

/// Event log types (high bits; the low five bits are the argument).
pub mod log {
    pub const HOLE: u16 = 0x20;
    pub const BUILT: u16 = 0x40;
    pub const MATCH: u16 = 0x60;
    pub const TOP100: u16 = 0x80;
    pub const TOP18: u16 = 0xa0;
    pub const CELEB: u16 = 0xc0;
    pub const TOURNAMENT: u16 = 0xe0;
    pub const CEO: u16 = 0x100;
    pub const HAPPY: u16 = 0x120;
    pub const LAND: u16 = 0x140;
    pub const HEIRESS: u16 = 0x160;
}

/// One accomplishment earned: when, and on which course.
#[derive(Clone, Debug, Default, PartialEq, serde::Serialize, serde::Deserialize)]
pub struct Earned {
    pub tick: u32,
    pub course: String,
}

impl Club {
    /// The title shown for accomplishment `id` at the club's difficulty.
    pub fn award_title(&self, id: usize) -> &'static str {
        match id {
            0 if self.difficulty < 2 => EASY_TITLES[0],
            1 if self.difficulty < 2 => EASY_TITLES[1],
            4 if self.difficulty < 2 => EASY_TITLES[2],
            _ => TITLES[id.min(21)],
        }
    }

    /// Earns accomplishment `id` the first time (0x46e7b0): its tick and course are kept, and the board will show it with
    /// a snapshot of the map point `at`. Nothing is earned in a championship.
    pub fn award_at(&mut self, id: u32, at: (i32, i32)) -> bool {
        if !self.award(id) {
            return false;
        }
        if let Some(e) = self.earned.get_mut(id as usize) {
            *e = Some(Earned { tick: self.tick, course: self.course_name.clone() });
        }
        self.award_pending = id as i32;
        self.award_point = at;
        if at.0 >= 0 {
            self.award_snaps.push((id as usize, at));
        }
        self.award_frames = 0;
        true
    }

    /// Writes the month's event log entry (0x40c6f0): one entry a month, a later event replacing an earlier one.
    pub fn log_event(&mut self, ty: u16, arg: i32) {
        if self.event_log.len() != 500 {
            self.event_log = vec![0; 500];
        }
        let i = ((self.tick >> 10) % 500) as usize;
        self.event_log[i] = ty | (arg as u16 & 0x1f);
    }

    /// The text of a log entry (year-end screen 0x44cff0); empty when it shows nothing.
    pub fn event_text(&self, v: u16) -> String {
        let (ty, arg) = (v & 0xffe0, (v & 0x1f) as i32);
        match ty {
            log::HOLE if self.par(arg) != 0 => format!("Hole {arg} opened."),
            log::BUILT => format!("{} built.", crate::land::BUILDINGS.get(arg as usize).map(|b| b.0).unwrap_or("Building")),
            log::MATCH => format!("Won match vs. {}", self.pros.get(arg as usize).map(|p| p.name.as_str()).unwrap_or("a pro")),
            log::TOP100 => format!("Hole {arg} rated top 100."),
            log::TOP18 => format!("Hole {arg} rated top 18."),
            log::CELEB => format!("{} buys a home.", self.celebrities.get(arg as usize).map(|c| c.name.as_str()).unwrap_or("A celebrity")),
            log::TOURNAMENT => {
                let pro = self.roster.first().map(|p| p.name.as_str()).unwrap_or("Gary Golf");
                format!("{pro} places {arg} in tournament.")
            }
            log::CEO => "J.P. Bigdome joins the board.".into(),
            log::HAPPY => format!("Happy Ending: {}", self.stories.title(arg).trim()),
            log::LAND => "Additional land purchased.".into(),
            log::HEIRESS => format!("Ivana donates a {}", crate::vips::landmark_name(arg)),
            _ => String::new(),
        }
    }

    /// Keeps this month's history entry (charge interval): cash in units, fun, skill (hundredths) and members.
    pub fn record_history(&mut self, cash: i32, members: i32) {
        if self.history.len() != 500 {
            self.history = vec![[0; 4]; 500];
        }
        let i = ((self.tick >> 10) % 500) as usize;
        self.history[i] = [cash, self.ratings.fun, self.ratings.skill, members];
    }

    /// At each month's start every hole's statistics lose an eighth (main frame at the month start): plays, plans, mood sum,
    /// play time, quits and the comment counters, so the reports weigh recent months more.
    pub fn month_decay(&mut self) {
        for h in self.holes.iter_mut().skip(1).take(18) {
            for v in [&mut h.tee_shots, &mut h.plans, &mut h.mood_sum, &mut h.time, &mut h.quits] {
                *v -= *v / 8;
            }
            for v in h.events.iter_mut().take(64) {
                *v -= *v / 8;
            }
        }
    }

    /// Members on the books (level Member or better, not resigned).
    pub fn member_count(&self) -> i32 {
        self.members.iter().filter(|m| m.gone != 0xff && m.level & 7 >= 2).count() as i32
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn log_and_awards() {
        let mut cl = Club { tick: 0x400 * 3, ..Default::default() };
        cl.holes[2].par = 4;
        cl.log_event(log::HOLE, 2);
        assert_eq!(cl.event_text(cl.event_log[3]), "Hole 2 opened.");
        cl.difficulty = 1;
        assert!(cl.award_at(1, (10, 10)));
        assert!(!cl.award_at(1, (10, 10)), "earned once");
        assert_eq!(cl.award_title(1), "1st Dogleg Left Hole");
        assert_eq!(cl.earned[1].as_ref().map(|e| e.tick), Some(0xc00));
        // two earned together: the board opens for the last, both get their snapshot
        assert!(cl.award_at(0, (20, 20)));
        assert_eq!(cl.award_pending, 0);
        assert_eq!(cl.award_snaps, vec![(1, (10, 10)), (0, (20, 20))]);
    }
}
