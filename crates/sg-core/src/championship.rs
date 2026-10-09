//! Play a Championship: a tournament on a course saved for championship play, with a pro picked from the championship
//! folder (title menu, 0x46ddd0). Also the pro files (`.pro`, 0x437910 / 0x437fa0): a roster record, the pro's custom
//! sayings, his sixteen skill bytes, a marker and his portraits.
//!
//! Facts are from the publisher's golf.exe (docs/PUBLISHER_EXE_NOTES.md, "Play a Championship"), restated in our own words.

use crate::course::Course;
use crate::golfer::{game, Club};
use crate::land::ExeRng;
use crate::roster::{Person, RECORD};

/// Championship mode (game flag 0x4000000): no evaluation, no accomplishments, back to the title menu at the end.
pub const CHAMPIONSHIP: u32 = game::EDITOR;

/// The event's name on the leaderboard by difficulty.
pub const EVENTS: [&str; 4] = ["SGA Qualifying School", "SGA Jr. Championship", "SGA Tour Championship", "SGA Open Championship"];

const SAYINGS: usize = 25;
const SAYING: usize = 50;
const MARKER: &[u8; 8] = b"*PCXFILE";

/// A pro file.
#[derive(Clone, Debug, Default)]
pub struct ProFile {
    pub person: Person,
    /// The record as read, so writing it back keeps the look bytes this port does not interpret.
    pub record: Vec<u8>,
    pub sayings: Vec<String>,
    pub skills: [u8; 16],
}

fn cstr(b: &[u8]) -> String {
    let end = b.iter().position(|&c| c == 0).unwrap_or(b.len());
    String::from_utf8_lossy(&b[..end]).into_owned()
}

/// Reads a pro file: the 0x230-byte roster record, 25 sayings of 50 bytes, 16 skill bytes, then the marker and a picture.
pub fn parse_pro(b: &[u8]) -> Option<ProFile> {
    let skills_at = RECORD + SAYINGS * SAYING;
    if b.len() < skills_at + 16 {
        return None;
    }
    let mut skills = [0u8; 16];
    skills.copy_from_slice(&b[skills_at..skills_at + 16]);
    Some(ProFile {
        person: Person::parse(&b[..RECORD]),
        record: b[..RECORD].to_vec(),
        sayings: (0..SAYINGS).map(|i| cstr(&b[RECORD + i * SAYING..RECORD + (i + 1) * SAYING])).collect(),
        skills,
    })
}

/// Writes a pro file in the same layout. The portraits the exe renders after the marker are left out; the exe's reader
/// stops at the marker when no picture follows.
pub fn pro_bytes(p: &ProFile) -> Vec<u8> {
    let mut out = vec![0u8; RECORD];
    let n = p.record.len().min(RECORD);
    out[..n].copy_from_slice(&p.record[..n]);
    let put = |out: &mut [u8], at: usize, len: usize, s: &str| {
        let b = s.as_bytes();
        let k = b.len().min(len - 1);
        out[at..at + len].fill(0);
        out[at..at + k].copy_from_slice(&b[..k]);
    };
    put(&mut out, 0, 0x10, &p.person.job);
    put(&mut out, 0x10, 0x10, &p.person.name);
    out[0x20] = p.person.traits;
    out[0x21] = p.person.b21;
    out[0x23] = p.person.b23;
    out[0x2c..0x30].copy_from_slice(&p.person.fixed.to_le_bytes());
    for i in 0..SAYINGS {
        let mut s = vec![0u8; SAYING];
        if let Some(t) = p.sayings.get(i) {
            let b = t.as_bytes();
            let k = b.len().min(SAYING - 1);
            s[..k].copy_from_slice(&b[..k]);
        }
        out.extend(s);
    }
    out.extend_from_slice(&p.skills);
    out.extend_from_slice(MARKER);
    out
}

impl Club {
    /// The player's pro as a pro file ("Save <pro> for Championship").
    pub fn pro_file(&self) -> ProFile {
        let person = self.roster.first().cloned().unwrap_or_default();
        ProFile { person, record: Vec::new(), sayings: vec![String::new(); SAYINGS], skills: self.pro_skill }
    }

    /// Starts a championship on the loaded course (0x46ddd0): the chosen pro replaces the player's, the flags are only
    /// championship mode, and the field is set up at once with the pro at the clubhouse. `cash` is in units.
    pub fn start_championship(&mut self, c: &Course, rng: &mut ExeRng, pro: &ProFile, cash: i32) {
        if let Some(p) = self.roster.first_mut() {
            *p = pro.person.clone();
        }
        self.pro_skill = pro.skills;
        self.game = CHAMPIONSHIP;
        self.gary = -1;
        self.purse = 0;
        self.challenge_pro = -1;
        // nothing is earned in a championship, including an award the saved course still had waiting
        self.award_pending = -1;
        let (da, db) = c.door;
        let pos = (da * crate::geom::UNIT + 0x600, db * crate::geom::UNIT + 0x600);
        self.start_tournament(c, rng, pos, self.holes[1].tee_facing, cash);
    }

    pub fn championship(&self) -> bool {
        self.game & CHAMPIONSHIP != 0
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn pro_files_round_trip() {
        let mut p = ProFile::default();
        p.person.name = "Test Pro".into();
        p.person.job = "Golf Pro".into();
        p.skills[0] = 7;
        p.skills[9] = 3;
        p.sayings = vec!["Fore!".into()];
        let b = pro_bytes(&p);
        assert_eq!(b.len(), 0x722 + 8);
        let q = parse_pro(&b).unwrap();
        assert_eq!(q.person.name, "Test Pro");
        assert_eq!(q.skills, p.skills);
        assert_eq!(q.sayings[0], "Fore!");
    }
}
