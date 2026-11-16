//! Club records (the port's own; with the Extra reports' comments by hole, extras.rs
//! `hole_comments`): each hole's best score, who made it and when, and every hole in one, kept with the save as
//! holes are finished (Course A's, as the original's Best Scores are). Hole Stats shows the hole's record and its holes
//! in one under the comments, and a hole in one makes the ticker say so. Records are kept by hole number: a hole rebuilt
//! or renumbered keeps its number's record.
use crate::app::*;
use crate::extras::date;
use serde::{Deserialize, Serialize};
use sg_core::lang::trf;
use std::collections::BTreeMap;

/// A score kept: strokes, the golfer's name and the date (`App::date`).
#[derive(Clone, Debug, Default, PartialEq, Serialize, Deserialize)]
pub struct Mark {
    pub strokes: u8,
    pub name: String,
    pub tick: u32,
}

/// The records kept with the save.
#[derive(Clone, Debug, Default, PartialEq, Serialize, Deserialize)]
pub struct Records {
    /// The best score on each hole, by hole number.
    #[serde(default)]
    pub best: BTreeMap<u8, Mark>,
    /// Every hole in one, oldest first: the hole and the mark.
    #[serde(default)]
    pub aces: Vec<(u8, Mark)>,
}

impl Records {
    /// A finished hole: a new record when it beats the hole's (an equal score keeps the first). True for a hole in one.
    pub fn hole_done(&mut self, hole: u8, strokes: u8, name: &str, tick: u32) -> bool {
        if strokes == 0 {
            return false;
        }
        let m = Mark { strokes, name: name.to_string(), tick };
        if self.best.get(&hole).is_none_or(|b| strokes < b.strokes) {
            self.best.insert(hole, m.clone());
        }
        if strokes == 1 {
            self.aces.push((hole, m));
            return true;
        }
        false
    }

    /// Hole `h`'s holes in one.
    pub fn aces_on(&self, h: u8) -> impl Iterator<Item = &Mark> {
        self.aces.iter().filter(move |a| a.0 == h).map(|a| &a.1)
    }
}

impl App {
    /// A hole finished on Course A (app.rs's HoleDone): the records, with the extra on.
    pub fn records_hole_done(&mut self, g: usize, hole: i32, strokes: i32) {
        if !crate::extras::hole_comments() || !(1..19).contains(&hole) || !(1..=255).contains(&strokes) {
            return;
        }
        let name = self.club.vip_name(g);
        let tick = self.date();
        let r = self.xs.kept.records.get_or_insert_with(Default::default);
        if r.hole_done(hole as u8, strokes as u8, &name, tick) {
            let text = trf("STR_RECORDS_ACE", &[("GOLFER", &name), ("HOLE", &hole)]);
            self.post_message(&text, 1, g as i32);
        }
    }

    /// Hole Stats' line for hole `h`: its record and holes in one, or None with nothing kept.
    pub fn records_line(&self, h: usize) -> Option<String> {
        if !crate::extras::hole_comments() {
            return None;
        }
        let r = self.xs.kept.records.as_ref()?;
        let b = r.best.get(&(h as u8))?;
        let mut line = trf("STR_RECORDS_BEST", &[("STROKES", &b.strokes), ("GOLFER", &b.name), ("DATE", &date(b.tick))]);
        let aces: Vec<&Mark> = r.aces_on(h as u8).collect();
        if let Some(last) = aces.last() {
            line += &trf("STR_RECORDS_ACES", &[("COUNT", &aces.len()), ("GOLFER", &last.name), ("DATE", &date(last.tick))]);
        }
        Some(line)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn a_record_falls_only_to_a_lower_score_and_aces_are_all_kept() {
        let mut r = Records::default();
        assert!(!r.hole_done(3, 4, "Ann", 10));
        assert!(!r.hole_done(3, 4, "Bob", 11));
        assert_eq!(r.best[&3].name, "Ann", "an equal score keeps the first");
        assert!(!r.hole_done(3, 3, "Cy", 12));
        assert_eq!(r.best[&3].strokes, 3);
        assert!(r.hole_done(3, 1, "Di", 13));
        assert!(r.hole_done(3, 1, "Ed", 14));
        assert_eq!(r.best[&3].name, "Di");
        assert_eq!(r.aces_on(3).count(), 2);
        assert_eq!(r.aces_on(4).count(), 0);
        assert!(!r.hole_done(5, 0, "No", 15));
        assert!(!r.best.contains_key(&5));
    }
}
