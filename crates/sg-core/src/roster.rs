//! The people who can come to play: the exe's roster of 0x230-byte person records (the same layout as the golfer files in the
//! game's Themes folders, *.glf and *.chr). Index 0 is the player's own pro, 1..56 are men and 57..75 women, 76 is the famous
//! challenger's slot and 77..79 the County commissioner, the Wealthy heiress and the Corporate CEO.
//!
//! The 76 ordinary people are compiled into the exe (the profile table at 0x4d6088). When the player's game folder holds an
//! exe whose data is readable they are read from it; otherwise the same facts, restated in `PEOPLE_TABLE` (profession, name,
//! trait bits, flag byte and head of every record), stand in. On top of them the exe loads (0x4658b0 / 0x4659a0) the theme
//! pack's characters (*.chr, into records 1..n in file order), Joe Pro.glf from the theme pack (record 76) and the three
//! special visitors from the Standard theme (records 77..79, `Themes\Standard\` for records 0x4d and up).

use crate::fsutil;
use std::path::Path;

pub const RECORD: usize = 0x230;
/// Roster entries the game uses (membership records exist for 84).
pub const PEOPLE: usize = 84;
/// Dialogue slots of a character file (25 of 50 bytes after the record).
pub const SAYINGS: usize = 25;
pub const SAYING: usize = 50;

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
    /// Record byte +0x22: the head (0..18 the stock heads of the person's gender, 19 and up the custom portraits). None in
    /// rosters saved before the head bytes were known (see `head_index`).
    #[serde(default)]
    pub head: Option<u8>,
    /// A character file whose embedded portrait is this person's head (game folder relative, '/' separated): set for a
    /// file with a head byte of 20 or more, whose own value the exe discards for the next custom slot (0x437fa0).
    #[serde(default)]
    pub portrait: Option<String>,
    /// The 25 dialogue slots of a character file (0x543d10 + record * 0x4e2): slot r replaces the stock line of the event
    /// in row r of the Customise table (`SAYING_CODES`) when it is not empty.
    #[serde(default)]
    pub sayings: Vec<String>,
    /// Record bytes +0x30..: the biography (up to 500 characters, edited with Customise's Update Bio).
    #[serde(default)]
    pub bio: String,
}

fn cstr(b: &[u8]) -> String {
    let end = b.iter().position(|&c| c == 0).unwrap_or(b.len());
    b[..end].iter().map(|&c| c as char).collect()
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
            head: Some(r[0x22]),
            portrait: None,
            sayings: Vec::new(),
            bio: if r.len() >= RECORD { cstr(&r[0x30..RECORD]) } else { String::new() },
        }
    }

    /// A whole character file (.chr / .glf / .pro): the record, then the 25 dialogue slots when present.
    pub fn parse_file(b: &[u8]) -> Person {
        let mut p = Person::parse(b);
        if b.len() >= RECORD + SAYINGS * SAYING {
            p.sayings = (0..SAYINGS).map(|i| cstr(&b[RECORD + i * SAYING..RECORD + (i + 1) * SAYING])).collect();
        }
        p
    }

    pub fn female(&self) -> bool {
        self.b21 & 0x80 != 0
    }

    /// The head drawn for this person, roster index `id`: the record's head byte; a roster saved without head bytes takes
    /// the exe's head of that record (player's pro: Gary Golf's head 8).
    pub fn head_index(&self, id: usize) -> u8 {
        match self.head {
            Some(h) => h,
            None => PEOPLE_TABLE.get(id).map(|r| r.4).unwrap_or(0),
        }
    }

    /// The saying of dialogue slot `slot`, if the person has one.
    pub fn saying(&self, slot: usize) -> Option<&str> {
        self.sayings.get(slot).map(|s| s.as_str()).filter(|s| !s.is_empty())
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

/// The event codes of the Customise screen's twenty dialogue rows (byte table 0x4c2d10, ended by -1): row r of the table
/// shows (and dialogue slot r replaces) the stock line of event `SAYING_CODES[r]`.
pub const SAYING_CODES: [u32; 20] =
    [0x3e, 0x01, 0x04, 0x05, 0x1f, 0x02, 0x03, 0x09, 0x0c, 0x0d, 0x1c, 0x14, 0x27, 0x15, 0x0e, 0x19, 0x0f, 0x12, 0x1a, 0x1b];

/// The rows' labels (pointer table 0x4c2cc0).
pub const SAYING_LABELS: [&str; 20] = [
    "Signature saying",
    "Made good shot",
    "Too easy shot",
    "Hard shot",
    "Really hard shot",
    "Hit bad shot",
    "Hit worse shot",
    "Almost hit by ball",
    "Hit a tree",
    "Hit into the water",
    "Notices scenic improvement",
    "Notices ugly item",
    "Notices animal",
    "Long wait at tee",
    "Getting thirsty",
    "Found a drink",
    "Getting hungry",
    "Found some food",
    "Getting tired",
    "Found a bench",
];

/// The exe's compiled profile table (0x4d6088, 84 records of 0x230 bytes): profession, name, trait bits (+0x20), flag byte
/// (+0x21: age and marital bits, bit 7 female) and head (+0x22) of each; every other byte of these records is zero.
pub const PEOPLE_TABLE: [(&str, &str, u8, u8, u8); PEOPLE] = [
    ("Golf Pro", "Gary Golf", 0x14, 0x0a, 8),
    ("Executive", "Jim", 0x03, 0x12, 9),
    ("Attorney", "Clarence", 0x05, 0x09, 3),
    ("Caddie", "Billy", 0x18, 0x09, 5),
    ("Salesman", "Larry", 0x06, 0x22, 2),
    ("Doctor", "Ben", 0x11, 0x12, 7),
    ("Preacher", "Zeke", 0x12, 0x12, 0),
    ("Tycoon", "J.P.", 0x09, 0x44, 11),
    ("Athlete", "Dennis", 0x0c, 0x11, 17),
    ("Musician", "Ringo", 0x0a, 0x09, 1),
    ("Maitre d'", "Alphonse", 0x03, 0x0a, 10),
    ("Decorator", "Pierre", 0x01, 0x0a, 18),
    ("Programmer", "Bart", 0x18, 0x09, 13),
    ("Surfer", "Chip", 0x16, 0x09, 4),
    ("Professor", "Albert", 0x11, 0x12, 15),
    ("Guru", "Baba", 0x03, 0x14, 16),
    ("Psychiatrist", "Sigmund", 0x09, 0x24, 14),
    ("Rapper", "Snoop", 0x0a, 0x09, 12),
    ("Student", "Dobie", 0x1c, 0x09, 6),
    ("Golf Pro", "Jack", 0x14, 0x0a, 8),
    ("Executive", "Bob", 0x01, 0x14, 9),
    ("Attorney", "Slick", 0x04, 0x0a, 3),
    ("Caddie", "Roberto", 0x18, 0x09, 5),
    ("Salesman", "Sammy", 0x06, 0x22, 2),
    ("Doctor", "Marcus", 0x11, 0x12, 7),
    ("Preacher", "Kingsley", 0x12, 0x12, 0),
    ("Tycoon", "J.W.", 0x0d, 0x0a, 11),
    ("Athlete", "Kareem", 0x06, 0x0a, 17),
    ("Musician", "Dizzy", 0x0a, 0x0a, 1),
    ("Maitre d'", "Luigi", 0x03, 0x12, 10),
    ("Decorator", "Alain", 0x01, 0x0a, 18),
    ("Programmer", "Mark", 0x18, 0x09, 13),
    ("Surfer", "Biff", 0x06, 0x09, 4),
    ("Professor", "Horace", 0x11, 0x12, 15),
    ("Guru", "Maharish", 0x03, 0x14, 16),
    ("Psychiatrist", "Solomon", 0x03, 0x24, 14),
    ("Rapper", "Biggie", 0x0a, 0x09, 12),
    ("Student", "Gilbert", 0x0c, 0x09, 6),
    ("Golf Pro", "Arnie", 0x16, 0x12, 8),
    ("Executive", "Bill", 0x03, 0x12, 9),
    ("Attorney", "Johnnie", 0x05, 0x09, 3),
    ("Caddie", "Jimmy", 0x18, 0x09, 5),
    ("Salesman", "Leo", 0x06, 0x12, 2),
    ("Doctor", "Denton", 0x11, 0x12, 7),
    ("Preacher", "Joshua", 0x12, 0x12, 0),
    ("Tycoon", "J.R.", 0x09, 0x44, 11),
    ("Athlete", "Tyrone", 0x0c, 0x11, 17),
    ("Musician", "Duke", 0x0a, 0x09, 1),
    ("Maitre d'", "Michel", 0x03, 0x22, 10),
    ("Decorator", "Gianni", 0x01, 0x0a, 18),
    ("Programmer", "Sid", 0x18, 0x09, 13),
    ("Surfer", "Bing", 0x16, 0x09, 4),
    ("Professor", "Boris", 0x11, 0x12, 15),
    ("Guru", "Mahatma", 0x03, 0x14, 16),
    ("Psychiatrist", "Kurt", 0x09, 0x24, 14),
    ("Rapper", "Prince", 0x0a, 0x09, 12),
    ("Student", "Freddie", 0x1c, 0x09, 6),
    ("Manager", "Mary", 0x03, 0x92, 6),
    ("Lawyer", "Marcia", 0x05, 0xa1, 14),
    ("Coed", "Bobbie", 0x18, 0x89, 5),
    ("Nurse", "Florence", 0x14, 0x92, 3),
    ("Volunteer", "Julie", 0x06, 0x8a, 2),
    ("Homemaker", "Harriet", 0x11, 0x92, 11),
    ("Teacher", "Trudy", 0x12, 0x8c, 13),
    ("Grandmother", "Agnes", 0x09, 0x94, 9),
    ("Dancer", "Twyla", 0x0c, 0x91, 0),
    ("Artist", "Georgia", 0x0a, 0x89, 18),
    ("Critic", "Pauline", 0x01, 0x92, 10),
    ("Model", "Cindy", 0x07, 0x89, 1),
    ("Newscaster", "Jessica", 0x10, 0x92, 7),
    ("Soldier", "Jane", 0x06, 0x89, 8),
    ("Secretary", "Sally", 0x11, 0x89, 12),
    ("FlightAttendant", "Brenda", 0x16, 0x8a, 4),
    ("Detective", "Nancy", 0x09, 0x94, 16),
    ("Gymnast", "Olga", 0x0c, 0x89, 17),
    ("Socialite", "Jacqueline", 0x0a, 0xa2, 15),
    ("Golf Pro", "Gary Golf", 0x14, 0x0a, 8),
    ("Commissioner", "I.M. Picky", 0x03, 0x12, 2),
    ("Heiress", "Ivana Richman", 0x06, 0x92, 10),
    ("CEO", "J.P.Bigdome", 0x05, 0x12, 11),
    ("Dummy", "J.Doe", 0x05, 0x12, 11),
    ("Dummy", "J.Doe", 0x05, 0x12, 11),
    ("Dummy", "J.Doe", 0x05, 0x12, 11),
    ("Swapper", "Don't use", 0x05, 0x12, 11),
];

/// Per stock head defaults (table 0x4d55e8, 0x44 bytes an entry: men's heads 0..19, then women's at 20..39): the skin tone
/// (+0) and hair colour (+3) a picked face brings along in Customise, and the signature saying (+4, event 0x3e).
pub const HEAD_DEFAULTS: [(u8, u8, &str); 40] = [
    (0, 0, "Praise the Lord for this fabulous day."),
    (1, 0, "Dig my groovy head covers, PARTNER."),
    (0, 0, "PARTNER, wouldn't you love a new set of encyclopedias?"),
    (1, 0, "PARTNER, do you hear an ambulance?"),
    (1, 0, "Surf's up, dude."),
    (1, 0, "Someday this will all be mine."),
    (1, 0, "I should be studying for finals."),
    (0, 0, "A couple of aspirin will take care of that."),
    (1, 0, "PARTNER, keep your head down - swing easy."),
    (1, 0, "I'm in the mood for a hostile takeover."),
    (2, 0, "Did you know those mushrooms are edible?"),
    (1, 0, "PARTNER, guess how much I'm worth."),
    (1, 0, "I'm in the groove. C'mon let's move."),
    (0, 0, "Golf is cool, you know."),
    (1, 0, "PARTNER, you appear to have some envy issues."),
    (0, 0, "This course may require further study."),
    (1, 0, "Get in touch with your inner swing plane."),
    (3, 0, "PARTNER, just grip it and rip it man."),
    (0, 0, "Look at all shades of green."),
    (0, 0, ""),
    (0, 0, "One, two, three, SWING!"),
    (2, 3, "This is my good side."),
    (1, 0, "Could I be more perky?"),
    (3, 3, "Where does it hurt, PARTNER?"),
    (1, 1, "You say hello, I say bub bye"),
    (1, 0, "...and I'm working on my tan."),
    (0, 2, "Memo to self: stay in control."),
    (0, 2, "This just in: Yippee! Film at 11."),
    (2, 0, "YOU CALL THAT A SWING, MAGGOT!"),
    (1, 4, "Now Bobby Jones, HE was a golfer."),
    (1, 3, "I give myself two thumbs up!"),
    (2, 1, "PARTNER, do you like brownies?"),
    (0, 0, "Ouch, I hurt my typing finger."),
    (1, 1, "So far, I give myself a B+."),
    (0, 1, "PARTNER, I'm gonna request a mistrial."),
    (0, 2, "And my hair still looks fabulous."),
    (1, 3, "I just about got this game figured out."),
    (1, 2, "I give that shot a perfect 10."),
    (2, 1, "This course is a masterpiece."),
    (1, 0, ""),
];

/// The defaults of stock head `head` of a gender (None for custom heads, which the table does not cover).
pub fn head_defaults(head: u8, female: bool) -> Option<(u8, u8, &'static str)> {
    if head >= 0x13 {
        return None;
    }
    HEAD_DEFAULTS.get(head as usize + if female { 20 } else { 0 }).copied()
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

/// The exe's roster from `PEOPLE_TABLE`, for when no readable exe is at hand.
pub fn fallback() -> Vec<Person> {
    PEOPLE_TABLE
        .iter()
        .map(|&(job, name, traits, b21, head)| Person {
            job: job.into(),
            name: name.into(),
            traits,
            b21,
            head: Some(head),
            ..Default::default()
        })
        .collect()
}

fn find_file(dir: &Path, file: &str) -> Option<std::path::PathBuf> {
    fsutil::list_dir(dir).into_iter().find(|p| p.file_name().and_then(|n| n.to_str()).is_some_and(|n| n.eq_ignore_ascii_case(file)))
}

/// A character file read into a person record; a head byte of 20 or more means the file's own embedded portrait.
fn read_character(game_dir: &Path, path: &Path) -> Option<Person> {
    let b = fsutil::read_file(path)?;
    if b.len() < 0x30 {
        return None;
    }
    let mut p = Person::parse_file(&b);
    if p.head_index(0) > 0x13 && b.windows(8).any(|w| w == b"*PCXFILE") {
        let rel = path.strip_prefix(game_dir).unwrap_or(path);
        p.portrait = Some(rel.to_string_lossy().replace('\\', "/"));
    }
    Some(p)
}

/// Loads the roster for a new game in theme pack folder `pack` (under Themes): the exe's own records when readable, else
/// `PEOPLE_TABLE`; Joe Pro.glf from the pack (record 76), the three visitors from the Standard theme (77..79), and the
/// pack's *.chr characters in place of records 1, 2, ... in name order (case-insensitive, as the original's file listing).
pub fn load(game_dir: &Path, pack: &str) -> Vec<Person> {
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
    let themes = find_file(game_dir, "Themes").unwrap_or_else(|| game_dir.join("Themes"));
    let pack_dir = find_file(&themes, pack).unwrap_or_else(|| themes.join(pack));
    let standard = find_file(&themes, "Standard").unwrap_or_else(|| themes.join("Standard"));
    for (slot, dir, file) in [
        (76usize, &pack_dir, "Joe Pro.glf"),
        (77, &standard, "I.M.Picky.glf"),
        (78, &standard, "Ivana Richman.glf"),
        (79, &standard, "J.P.Bigdome.glf"),
    ] {
        if let Some(p) = find_file(dir, file).and_then(|f| read_character(game_dir, &f)) {
            people[slot] = p;
        }
    }
    let mut chars: Vec<_> = fsutil::list_dir(&pack_dir)
        .into_iter()
        .filter(|p| p.extension().and_then(|e| e.to_str()).is_some_and(|e| e.eq_ignore_ascii_case("chr")))
        .collect();
    chars.sort_by_key(|p| p.file_name().map(|n| n.to_string_lossy().to_lowercase()).unwrap_or_default());
    for (i, f) in chars.iter().enumerate().take(75) {
        if let Some(p) = read_character(game_dir, f) {
            people[i + 1] = p;
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
        assert_eq!((r[0].name.as_str(), r[0].head_index(0)), ("Gary Golf", 8));
        // the id mod 19 rule the exe uses when picking arrivals spreads the stock heads: men 1..19 have each head once
        let mut seen = [false; 19];
        for p in &r[1..20] {
            seen[p.head_index(0) as usize] = true;
        }
        assert!(seen.iter().all(|&s| s));
    }

    #[test]
    fn heads() {
        let mut p = fallback()[5].clone();
        p.head = None;
        assert_eq!(p.head_index(5), PEOPLE_TABLE[5].4);
        let mut rec = vec![0u8; RECORD];
        rec[0x22] = 21;
        assert_eq!(Person::parse(&rec).head_index(5), 21);
    }

    #[test]
    fn character_file_sayings() {
        let mut b = vec![0u8; RECORD + SAYINGS * SAYING + 16];
        b[0x10..0x13].copy_from_slice(b"Ann");
        b[RECORD + SAYING..RECORD + SAYING + 5].copy_from_slice(b"Hello");
        let p = Person::parse_file(&b);
        assert_eq!(p.name, "Ann");
        assert_eq!(p.saying(1), Some("Hello"));
        assert_eq!(p.saying(0), None);
    }
}
