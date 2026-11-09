//! The club's records: the pro's Professional Accomplishments (table 0x4c1578, award 0x46e7b0), the monthly event log
//! (0x568600, log 0x40c6f0) the year-end report and the Histograph read, and the monthly history of cash, fun, skill and
//! membership (charge interval, main frame 0x417a08).
//!
//! Facts are from the publisher's golf.exe (docs/PUBLISHER_EXE_NOTES.md, "Accomplishments, event log and year end"),
//! restated in our own words.

use crate::golfer::Club;
use crate::lang::{tr, trf};

/// The 22 accomplishments, as the plaques name them; on Easy and Moderate ids 0, 1 and 4 are the three alternatives.
pub const TITLE_KEYS: [&str; 22] = [
    "STR_ACCOMPLISHMENT_CHALLENGE_HOLE",
    "STR_ACCOMPLISHMENT_HEROIC_HOLE",
    "STR_ACCOMPLISHMENT_SKILL_UPGRADE",
    "STR_ACCOMPLISHMENT_TOURNAMENT",
    "STR_ACCOMPLISHMENT_STRATEGIC_HOLE",
    "STR_ACCOMPLISHMENT_MATCH_VICTORY",
    "STR_ACCOMPLISHMENT_9_HOLE_COURSE",
    "STR_ACCOMPLISHMENT_TOP_100_HOLE",
    "STR_ACCOMPLISHMENT_500K_TOURNAMENT",
    "STR_ACCOMPLISHMENT_CLASSIC_HOLE",
    "STR_ACCOMPLISHMENT_TOP_18_HOLE",
    "STR_ACCOMPLISHMENT_VICTORY_9",
    "STR_ACCOMPLISHMENT_GRAND_SLAM_COURSE_9",
    "STR_ACCOMPLISHMENT_1M_TOURNAMENT",
    "STR_ACCOMPLISHMENT_18_HOLE_COURSE",
    "STR_ACCOMPLISHMENT_VICTORY_18",
    "STR_ACCOMPLISHMENT_GRAND_SLAM_COURSE_18",
    "STR_ACCOMPLISHMENT_GRAND_SLAM_PARKLAND",
    "STR_ACCOMPLISHMENT_GRAND_SLAM_DESERT",
    "STR_ACCOMPLISHMENT_GRAND_SLAM_TROPICAL",
    "STR_ACCOMPLISHMENT_GRAND_SLAM_LINKS",
    "STR_ACCOMPLISHMENT_100_STAR",
];
pub const EASY_TITLE_KEYS: [&str; 3] = ["STR_ACCOMPLISHMENT_DOGLEG_RIGHT", "STR_ACCOMPLISHMENT_DOGLEG_LEFT", "STR_ACCOMPLISHMENT_PAR_5"];

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
            0 if self.difficulty < 2 => tr(EASY_TITLE_KEYS[0]),
            1 if self.difficulty < 2 => tr(EASY_TITLE_KEYS[1]),
            4 if self.difficulty < 2 => tr(EASY_TITLE_KEYS[2]),
            _ => tr(TITLE_KEYS[id.min(21)]),
        }
    }

    /// Earns accomplishment `id` the first time (0x46e7b0): its tick and course are kept, and the board will show it with
    /// a snapshot of the map point `at`. Nothing is earned in a championship.
    pub fn award_at(&mut self, id: u32, at: (i32, i32)) -> bool {
        if !self.award(id) {
            return false;
        }
        let date = self.date();
        if let Some(e) = self.earned.get_mut(id as usize) {
            *e = Some(Earned { tick: date, course: self.course_name.clone() });
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
        let i = ((self.date() >> 10) % 500) as usize;
        self.event_log[i] = ty | (arg as u16 & 0x1f);
    }

    /// The text of a log entry (year-end screen 0x44cff0); empty when it shows nothing.
    pub fn event_text(&self, v: u16) -> String {
        let (ty, arg) = (v & 0xffe0, (v & 0x1f) as i32);
        match ty {
            log::HOLE if self.par(arg) != 0 => trf("STR_LOG_HOLE_OPENED", &[("HOLE", &arg)]),
            log::BUILT => trf(
                "STR_LOG_BUILT",
                &[("BUILDING", &Some(crate::land::building_name(arg)).filter(|n| !n.is_empty()).unwrap_or(tr("STR_LOG_BUILDING")))],
            ),
            log::MATCH => {
                trf("STR_LOG_WON_MATCH", &[("PRO", &self.pros.get(arg as usize).map(|p| p.name.as_str()).unwrap_or(tr("STR_LOG_A_PRO")))])
            }
            log::TOP100 => trf("STR_LOG_TOP_100", &[("HOLE", &arg)]),
            log::TOP18 => trf("STR_LOG_TOP_18", &[("HOLE", &arg)]),
            log::CELEB => trf(
                "STR_LOG_CELEBRITY_HOME",
                &[("CELEBRITY", &self.celebrities.get(arg as usize).map(|c| c.name.as_str()).unwrap_or(tr("STR_LOG_A_CELEBRITY")))],
            ),
            log::TOURNAMENT => {
                let pro = self.roster.first().map(|p| p.name.as_str()).unwrap_or(tr("STR_LOG_GARY_GOLF"));
                trf("STR_LOG_TOURNAMENT_PLACE", &[("PRO", &pro), ("PLACE", &arg)])
            }
            log::CEO => tr("STR_LOG_CEO").into(),
            log::HAPPY => trf("STR_LOG_HAPPY_ENDING", &[("STORY", &self.stories.title(arg).trim())]),
            log::LAND => tr("STR_LOG_LAND").into(),
            log::HEIRESS => trf("STR_LOG_HEIRESS", &[("LANDMARK", &crate::vips::landmark_short_name(arg))]),
            _ => String::new(),
        }
    }

    /// Keeps this month's history entry (charge interval): cash in units, fun, skill (hundredths) and members.
    pub fn record_history(&mut self, cash: i32, members: i32) {
        if self.history.len() != 500 {
            self.history = vec![[0; 4]; 500];
        }
        let i = ((self.date() >> 10) % 500) as usize;
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
