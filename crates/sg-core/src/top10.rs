//! The Top 10 Designers table (top10.sve, the exe's 0x473470 screen and its insert 0x4732d0): ten records of 0x9c bytes,
//! ranked by (cash / 10 + skill + fun) * (difficulty + 1). A course appears at most once; a missing file is replaced by a
//! default table of made-up designers. Facts are from docs/DECODE_TOP10_PAIR.md, restated in our own words.

use crate::land::ExeRng;

/// Bytes per record and records in the table.
pub const RECORD: usize = 0x9c;
pub const COUNT: usize = 10;

/// Trophy cell on the shelf art (0..4 top shelf, 5..9 bottom, left to right) for each rank.
pub const RANK_SLOT: [usize; COUNT] = [2, 1, 3, 0, 4, 7, 6, 8, 5, 9];

#[derive(Clone, Debug, Default, PartialEq)]
pub struct Entry {
    /// +0 designer, +0x40 course
    pub name: String,
    pub course: String,
    /// +0x80 fun rating, +0x84 skill rating (hundredths), +0x88 cash (units of $100)
    pub fun: i32,
    pub skill: i32,
    pub cash: i32,
    /// +0x94 (always 0 as far as read), +0x96 difficulty, +0x98 course id (-1 for the default entries)
    pub extra: i16,
    pub difficulty: i16,
    pub course_id: i32,
}

impl Entry {
    /// The ranking score.
    pub fn score(&self) -> i64 {
        (self.cash / 10 + self.skill + self.fun) as i64 * (self.difficulty as i64 + 1)
    }
}

fn cstr(b: &[u8]) -> String {
    b.iter().take_while(|&&c| c != 0).map(|&c| c as char).collect()
}

fn i32le(b: &[u8], o: usize) -> i32 {
    i32::from_le_bytes([b[o], b[o + 1], b[o + 2], b[o + 3]])
}

fn i16le(b: &[u8], o: usize) -> i16 {
    i16::from_le_bytes([b[o], b[o + 1]])
}

/// Reads the file; None unless it holds exactly ten records.
pub fn parse(d: &[u8]) -> Option<Vec<Entry>> {
    if d.len() != RECORD * COUNT {
        return None;
    }
    Some(
        d.chunks(RECORD)
            .map(|r| Entry {
                name: cstr(&r[..0x40]),
                course: cstr(&r[0x40..0x80]),
                fun: i32le(r, 0x80),
                skill: i32le(r, 0x84),
                cash: i32le(r, 0x88),
                extra: i16le(r, 0x94),
                difficulty: i16le(r, 0x96),
                course_id: i32le(r, 0x98),
            })
            .collect(),
    )
}

pub fn to_bytes(t: &[Entry]) -> Vec<u8> {
    let mut out = vec![0u8; RECORD * COUNT];
    for (r, e) in out.chunks_mut(RECORD).zip(t) {
        let put = |r: &mut [u8], o: usize, s: &str| {
            for (k, c) in s.chars().take(63).enumerate() {
                r[o + k] = c as u32 as u8;
            }
        };
        put(r, 0, &e.name);
        put(r, 0x40, &e.course);
        r[0x80..0x84].copy_from_slice(&e.fun.to_le_bytes());
        r[0x84..0x88].copy_from_slice(&e.skill.to_le_bytes());
        r[0x88..0x8c].copy_from_slice(&e.cash.to_le_bytes());
        r[0x94..0x96].copy_from_slice(&e.extra.to_le_bytes());
        r[0x96..0x98].copy_from_slice(&e.difficulty.to_le_bytes());
        r[0x98..0x9c].copy_from_slice(&e.course_id.to_le_bytes());
    }
    out
}

/// The table the exe writes when the file is missing: falling scores from the top, harder games at the top, one made-up
/// designer name with two letters shifted at random.
pub fn defaults(rng: &mut ExeRng) -> Vec<Entry> {
    (0..COUNT as i32)
        .map(|i| {
            let fun = rng.below(100) + 900 - 100 * i;
            let skill = rng.below(90) + 810 - 90 * i;
            let cash = rng.below(800) + 7200 - 800 * i;
            let mut name: Vec<u8> = b"a.c. dye".to_vec();
            name[0] = name[0].wrapping_add(rng.below(26) as u8);
            name[2] = name[2].wrapping_add(rng.below(24) as u8);
            Entry {
                name: name.iter().map(|&c| c as char).collect(),
                course: "Harbour Lights GC".to_string(),
                fun,
                skill,
                cash,
                extra: 0,
                difficulty: ((9 - i) / 3) as i16,
                course_id: -1,
            }
        })
        .collect()
}

/// Puts a course's result in the table. Returns its rank (0 based), or None when it does not make the list or the same
/// course already stands at or above it. An older entry of the same course lower down makes room; otherwise the tenth goes.
pub fn insert(t: &mut Vec<Entry>, e: Entry) -> Option<usize> {
    let s = e.score();
    let mut i = 0;
    while i < t.len().min(COUNT) && s < t[i].score() {
        if t[i].course_id == e.course_id {
            return None;
        }
        i += 1;
    }
    if i >= COUNT {
        return None;
    }
    match (i..t.len()).rev().find(|&k| t[k].course_id == e.course_id) {
        Some(k) => {
            t.remove(k);
        }
        None if t.len() >= COUNT => {
            t.truncate(COUNT - 1);
        }
        None => {}
    }
    t.insert(i.min(t.len()), e);
    t.truncate(COUNT);
    Some(i)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn defaults_round_trip_and_insert() {
        let mut rng = ExeRng::from_clock(1234);
        let mut t = defaults(&mut rng);
        assert_eq!(t.len(), 10);
        assert!(t.windows(2).all(|w| w[0].score() >= w[1].score()));
        assert_eq!(parse(&to_bytes(&t)).unwrap(), t);
        // a weak course does not make it
        let weak = Entry { course_id: 7, ..Default::default() };
        assert_eq!(insert(&mut t, weak), None);
        // a strong one goes on top, and its later better result replaces it
        let strong = Entry { name: "Pro".into(), fun: 2000, skill: 2000, cash: 10000, difficulty: 3, course_id: 7, ..Default::default() };
        assert_eq!(insert(&mut t, strong.clone()), Some(0));
        assert_eq!(insert(&mut t, Entry { fun: 1000, ..strong.clone() }), None);
        assert_eq!(insert(&mut t, Entry { fun: 3000, ..strong }), Some(0));
        assert_eq!(t.len(), 10);
        assert_eq!(t.iter().filter(|e| e.course_id == 7).count(), 1);
    }
}
