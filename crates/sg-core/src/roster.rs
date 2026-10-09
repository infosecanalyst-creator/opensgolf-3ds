//! The people who can come to play: the exe's roster of 0x230-byte person records (the same layout as the golfer files in the
//! game's Themes folders, *.glf and *.chr). Index 0 is the player's own pro, 1..56 are men and 57..75 women, 76 is the famous
//! challenger's slot and 77..79 the County commissioner, the Wealthy heiress and the Corporate CEO.
//!
//! The 76 ordinary people are compiled into the exe. When the player's game folder holds an exe whose data is readable, they are
//! read from it; the retail exe is copy protected and is not touched, and then a roster written for this port stands in (our own
//! names, the exe's gender split and skill classes). The three special visitors are read from the Standard theme's golfer files.

use crate::fsutil;
use std::path::Path;

pub const RECORD: usize = 0x230;
/// Roster entries the game uses (membership records exist for 84).
pub const PEOPLE: usize = 84;

#[derive(Clone, Debug, Default, serde::Serialize, serde::Deserialize)]
pub struct Person {
    pub job: String,
    pub name: String,
    /// Record byte +0x20: trait bits (decide the skill class of men).
    pub traits: u8,
    /// Record byte +0x21: bit 7 female; low bits pick the female body.
    pub b21: u8,
    /// Record byte +0x23: high nibble remembers the body drawn (0..3 within the gender), which also picks the voice.
    pub b23: u8,
    /// Record int +0x2c: non-zero fixes the skill class (low 3 bits) and the body.
    pub fixed: i32,
    /// Record bytes +0x24 shirt, +0x25 trousers, +0x26 hands, +0x27 skin, +0x28 hair: the colours once fixed.
    #[serde(default)]
    pub shirt: u8,
    #[serde(default)]
    pub pants: u8,
    #[serde(default)]
    pub alt_skin: u8,
    #[serde(default)]
    pub skin: u8,
    #[serde(default)]
    pub hair: u8,
}

fn cstr(b: &[u8]) -> String {
    let end = b.iter().position(|&c| c == 0).unwrap_or(b.len());
    String::from_utf8_lossy(&b[..end]).into_owned()
}

impl Person {
    pub fn parse(r: &[u8]) -> Person {
        if r.len() < 0x30 {
            return Person::default();
        }
        Person {
            job: cstr(&r[0..0x10]),
            name: cstr(&r[0x10..0x20]),
            traits: r[0x20],
            b21: r[0x21],
            b23: r[0x23],
            fixed: i32::from_le_bytes([r[0x2c], r[0x2d], r[0x2e], r[0x2f]]),
            shirt: r[0x24],
            pants: r[0x25],
            alt_skin: r[0x26],
            skin: r[0x27],
            hair: r[0x28],
        }
    }

    pub fn female(&self) -> bool {
        self.b21 & 0x80 != 0
    }

    /// 1 for men, 0 for women (0x46c940); emotion sounds are offset by it.
    pub fn male_bit(&self) -> i32 {
        (self.b21 & 0x80 == 0) as i32
    }

    /// Skill class bits (1 Length, 2 Accuracy, 4 Imagination) as golfer creation works them out; `slot` only matters for a
    /// man with no telling traits.
    pub fn kind(&self, slot: usize) -> u8 {
        let mut k = if self.female() {
            6
        } else {
            let mut k = 1;
            if self.traits & 0x11 != 0 {
                k = 3;
            }
            if self.traits & 0x0c != 0 {
                k |= 4;
            }
            if k == 1 {
                k = if slot & 1 == 1 { 2 } else { 4 } | 1;
            }
            k
        };
        if self.fixed != 0 {
            k = (self.fixed & 7) as u8;
        }
        k
    }
}

/// Finds the roster in an exe image: the record of the player's pro ("Golf Pro" / "Gary Golf") followed by its fellows.
pub fn from_exe(image: &[u8]) -> Option<Vec<Person>> {
    let mut sig = b"Golf Pro".to_vec();
    sig.resize(0x10, 0);
    sig.extend_from_slice(b"Gary Golf\0");
    let at = image.windows(sig.len()).position(|w| w == sig.as_slice())?;
    if at + RECORD * PEOPLE > image.len() {
        return None;
    }
    let people: Vec<Person> = (0..PEOPLE).map(|i| Person::parse(&image[at + i * RECORD..at + (i + 1) * RECORD])).collect();
    // A sanity check: the ordinary people have names and the women start at 57.
    if people[1..76].iter().any(|p| p.name.is_empty()) || people[56].female() || !people[57].female() {
        return None;
    }
    Some(people)
}

const MEN: [&str; 56] = [
    "Alan", "Barry", "Colin", "Dale", "Eddie", "Frank", "Glen", "Harold", "Ian", "Jerry", "Keith", "Lou", "Marty", "Neil", "Oscar", "Pete",
    "Quentin", "Ray", "Stan", "Ted", "Ulrich", "Vince", "Walt", "Xavier", "Yuri", "Zack", "Arnie", "Bert", "Carl", "Doug", "Ernie", "Fred",
    "Gus", "Hank", "Irv", "Joel", "Kurt", "Lyle", "Mel", "Nate", "Otto", "Phil", "Rex", "Sid", "Tom", "Vern", "Wade", "Abe", "Bud", "Chet",
    "Dean", "Earl", "Floyd", "Gil", "Hal", "Jack",
];
const WOMEN: [&str; 19] = [
    "Ann", "Bea", "Cora", "Dot", "Edna", "Fay", "Gail", "Hope", "Iris", "June", "Kay", "Lena", "Mae", "Nora", "Opal", "Pearl", "Rose",
    "Sue", "Vera",
];
const JOBS: [&str; 8] = ["Banker", "Dentist", "Teacher", "Pilot", "Chef", "Plumber", "Lawyer", "Farmer"];

/// The port's own roster, used when the exe's cannot be read: the same shape (pro, 56 men, 19 women, four special slots) with
/// skill classes spread so that every class appears in every third of the list, as new games need.
pub fn fallback() -> Vec<Person> {
    let mut v = Vec::with_capacity(PEOPLE);
    v.push(Person { job: "Golf Pro".into(), name: "Gary Golf".into(), traits: 0x14, b21: 0x0a, ..Default::default() });
    // Trait patterns giving the classes 3 (0x01), 7 (0x05) and 5 (0x04) to the men in turn.
    const TRAITS: [u8; 3] = [0x01, 0x05, 0x04];
    for (i, n) in MEN.iter().enumerate() {
        v.push(Person {
            job: JOBS[i % JOBS.len()].into(),
            name: (*n).into(),
            traits: TRAITS[(i / 3 + i) % 3],
            b21: (i % 8) as u8 * 2,
            ..Default::default()
        });
    }
    for (i, n) in WOMEN.iter().enumerate() {
        v.push(Person {
            job: JOBS[(i + 3) % JOBS.len()].into(),
            name: (*n).into(),
            traits: 0x02,
            b21: 0x80 | ((i % 6) as u8 * 2),
            ..Default::default()
        });
    }
    v.push(Person { job: "Golf Pro".into(), name: "Gary Golf".into(), traits: 0x14, b21: 0x0a, ..Default::default() });
    v.push(Person { job: "Commissioner".into(), name: "The Commissioner".into(), traits: 0x03, b21: 0x12, ..Default::default() });
    v.push(Person { job: "Heiress".into(), name: "The Heiress".into(), traits: 0x06, b21: 0x92, ..Default::default() });
    v.push(Person { job: "CEO".into(), name: "The CEO".into(), traits: 0x05, b21: 0x12, ..Default::default() });
    while v.len() < PEOPLE {
        v.push(Person { job: "Dummy".into(), name: "J.Doe".into(), traits: 0x05, b21: 0x12, ..Default::default() });
    }
    v
}

/// Loads the roster for a game folder: the exe's own when readable, else the port's; the special visitors from the Standard
/// theme's golfer files when present.
pub fn load(game_dir: &Path) -> Vec<Person> {
    let mut people = None;
    for f in fsutil::walk(game_dir) {
        let name = f.file_name().and_then(|n| n.to_str()).unwrap_or("").to_ascii_lowercase();
        if name.ends_with(".exe") && name.contains("golf") {
            if let Some(img) = fsutil::read_file(&f).and_then(|b| from_exe(&b)) {
                people = Some(img);
                break;
            }
        }
    }
    let mut people = people.unwrap_or_else(fallback);
    for (slot, file) in [(77usize, "I.M.Picky.glf"), (78, "Ivana Richman.glf"), (79, "J.P.Bigdome.glf"), (76, "Joe Pro.glf")] {
        if let Some(p) = fsutil::walk(game_dir).into_iter().find(|p| {
            p.file_name().and_then(|n| n.to_str()).map(|n| n.eq_ignore_ascii_case(file)).unwrap_or(false)
                && p.to_string_lossy().to_ascii_lowercase().contains("standard")
        }) {
            if let Some(b) = fsutil::read_file(&p) {
                if b.len() >= 0x30 {
                    people[slot] = Person::parse(&b);
                }
            }
        }
    }
    people
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn fallback_shape() {
        let r = fallback();
        assert_eq!(r.len(), PEOPLE);
        assert!(!r[56].female() && r[57].female() && r[75].female());
        for kind in [3u8, 5, 6, 7] {
            for m in 0..3 {
                assert!((1..76).any(|i| i % 3 == m && r[i].kind(i) == kind), "kind {kind} residue {m}");
            }
        }
    }
}
