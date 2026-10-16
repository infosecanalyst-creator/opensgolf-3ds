//! Special visitors (the exe's VIPs): the Corporate CEO, the County commissioner and the Wealthy heiress take the place of the
//! second golfer of every sixth pair when the club has earned them, play the round with skills of their own, and give a
//! verdict at the end: an investment, a land expansion, or a donated landmark. Also the celebrities who buy vacation homes on
//! valuable home sites.
//!
//! Facts are from the publisher's golf.exe (docs/PUBLISHER_EXE_NOTES.md, "Special visitors"), restated in our own words.

use crate::economy::money_digits;
use crate::golfer::{Club, Column, Event};
use crate::land::ExeRng;
use crate::lang::{tr, trf};

/// Roster slots of the visitors (loaded from the Standard theme's golfer files).
pub const ROSTER_CHALLENGER: i32 = 76;
pub const ROSTER_COMMISSIONER: i32 = 77;
pub const ROSTER_HEIRESS: i32 = 78;
pub const ROSTER_CEO: i32 = 79;

/// Landmark names by type (the landmark tool's 0..13, then two only donations give, then the eyesores).
pub fn landmark_name(t: i32) -> &'static str {
    const N: [&str; 16] = [
        "STR_LANDMARK_LONG_SUNDIAL",
        "STR_LANDMARK_LONG_BARN",
        "STR_LANDMARK_LONG_CANNON",
        "STR_LANDMARK_LONG_STONEHENGE",
        "STR_LANDMARK_LONG_WATER_MILL",
        "STR_LANDMARK_LONG_ROCK_FACE",
        "STR_LANDMARK_LONG_STATUE",
        "STR_LANDMARK_LONG_LIGHTHOUSE",
        "STR_LANDMARK_LONG_BUDDHA",
        "STR_LANDMARK_LONG_WINDMILL",
        "STR_LANDMARK_LONG_HISTORIC_STATUE",
        "STR_LANDMARK_LONG_EASTER_ISLAND",
        "STR_LANDMARK_LONG_PAGODA",
        "STR_LANDMARK_LONG_HATTERAS",
        "STR_LANDMARK_LONG_ORIENTAL_HOUSE",
        "STR_LANDMARK_LONG_TARPIT",
    ];
    if let Some(n) = extra_landmark_name(t) {
        return n;
    }
    tr(N.get(t as usize).copied().unwrap_or("STR_LANDMARK_FALLBACK"))
}

/// The name of one of the port's extra landmark types (sg_core::extras): a cut landmark or a buyable animal.
fn extra_landmark_name(t: i32) -> Option<&'static str> {
    use crate::objects::{ANIMAL_BASE, CUT_BASE, CUT_LANDMARKS};
    if t >= ANIMAL_BASE {
        return crate::wildlife::KINDS.get((t - ANIMAL_BASE) as usize).map(|k| k.name());
    }
    if t >= CUT_BASE {
        return CUT_LANDMARKS.get((t - CUT_BASE) as usize).map(|c| tr(c.2));
    }
    None
}

/// A landmark's long name as the exe writes it after "a" (0x4074a0 with its second argument 1): with the "n" of "an" where
/// the exe has one (not on the ancient stonehenge rock), otherwise after a space.
pub fn landmark_after_a(t: i32) -> String {
    let n = landmark_name(t);
    if [2, 4, 5, 6, 13, 14].contains(&t) {
        trf("STR_LANDMARK_AFTER_AN", &[("LANDMARK", &n)])
    } else {
        trf("STR_LANDMARK_AFTER_A", &[("LANDMARK", &n)])
    }
}

/// A landmark's short name (0x4074a0 with its second argument 0), as the landmark tool labels its preview.
pub fn landmark_short_name(t: i32) -> &'static str {
    const N: [&str; 19] = [
        "STR_LANDMARK_SUNDIAL",
        "STR_LANDMARK_BARN",
        "STR_LANDMARK_CANNON",
        "STR_LANDMARK_STONEHENGE",
        "STR_LANDMARK_WATER_MILL",
        "STR_LANDMARK_ROCK_FACE",
        "STR_LANDMARK_STATUE",
        "STR_LANDMARK_LIGHTHOUSE",
        "STR_LANDMARK_BUDDHA",
        "STR_LANDMARK_WINDMILL",
        "STR_LANDMARK_HISTORIC_STATUE",
        "STR_LANDMARK_EASTER_ISLAND",
        "STR_LANDMARK_PAGODA",
        "STR_LANDMARK_HATTERAS",
        "STR_LANDMARK_ORIENTAL_HOUSE",
        "STR_LANDMARK_TARPIT",
        "STR_LANDMARK_WATER_TOWER",
        "STR_LANDMARK_RADIO_ANTENNA",
        "STR_LANDMARK_OIL_PUMP",
    ];
    if let Some(n) = extra_landmark_name(t) {
        return n;
    }
    tr(N.get(t as usize).copied().unwrap_or("STR_LANDMARK_FALLBACK"))
}

/// What a visitor thinks of the course at mood m (0x469a20).
pub fn mood_remark(m: i32) -> &'static str {
    match m {
        0 => tr("STR_VIP_MOOD_0"),
        1 => tr("STR_VIP_MOOD_1"),
        2 => tr("STR_VIP_MOOD_2"),
        3 => tr("STR_VIP_MOOD_3"),
        4 => tr("STR_VIP_MOOD_4"),
        5 => tr("STR_VIP_MOOD_5"),
        6 => tr("STR_VIP_MOOD_6"),
        7 => tr("STR_VIP_MOOD_7"),
        8..=127 => tr("STR_VIP_MOOD_8"),
        _ => tr("STR_VIP_MOOD_LEAVING"),
    }
}

/// Second, third, ... visitor of the same kind.
/// A skill rating in hundredths as the visitors' sentences write it ("123.45").
fn skill_text(v: i32) -> String {
    format!("{}.{:02}", v / 100, (v % 100).abs())
}

fn suffix(n: i32) -> &'static str {
    const S: [&str; 10] = ["", " II", " III", " IV", " V", " VI", " VII", " VIII", " IX", " X"];
    S.get(n as usize).copied().unwrap_or(" X")
}

/// One celebrity of celebrities.dta (name, type A..K as 0..10, looks).
#[derive(Clone, Debug, Default, serde::Serialize, serde::Deserialize)]
pub struct Celebrity {
    pub name: String,
    pub kind: u8,
    pub skin: u8,
    pub hair: u8,
    pub shirt: u8,
    pub pants: u8,
}

/// Celebrity type names (the exe's table at 0x4c2ba8; only the first is visible in the decompile, the rest follow the file's
/// own legend).
pub const CELEBRITY_TYPE_KEYS: [&str; 11] = [
    "STR_CELEBRITY_ACTION_STAR",
    "STR_CELEBRITY_POP_STAR",
    "STR_CELEBRITY_POLITICIAN",
    "STR_CELEBRITY_COMEDIAN",
    "STR_CELEBRITY_SUPERMODEL",
    "STR_CELEBRITY_FITNESS_STAR",
    "STR_CELEBRITY_COMEDIENNE",
    "STR_CELEBRITY_LEADING_MAN",
    "STR_CELEBRITY_MOVIE_STAR",
    "STR_CELEBRITY_ROCK_AND_ROLLER",
    "STR_CELEBRITY_ATHLETE",
];

/// The name of celebrity type k (0..10; larger numbers read as the last).
pub fn celebrity_type(k: u8) -> &'static str {
    tr(CELEBRITY_TYPE_KEYS[(k as usize).min(10)])
}

/// Parses celebrities.dta: "Name,Type,Skin,Hair,Shirt,Pants" lines, '*' lines are comments, up to 100 entries.
pub fn parse_celebrities(text: &str) -> Vec<Celebrity> {
    let mut out = Vec::new();
    for l in text.lines() {
        let l = l.trim_end_matches('\r');
        if l.starts_with('*') || l.trim().is_empty() || out.len() >= 100 {
            continue;
        }
        let f: Vec<&str> = l.split(',').collect();
        if f.len() < 2 {
            continue;
        }
        let digit = |i: usize| f.get(i).and_then(|s| s.trim().chars().next()).and_then(|c| c.to_digit(10)).unwrap_or(0) as u8;
        let t = f[1].trim().chars().next().map(|c| c.to_ascii_uppercase()).unwrap_or('A');
        out.push(Celebrity {
            name: f[0].trim().to_string(),
            kind: ((t as u8).wrapping_sub(b'A')) % 11,
            skin: digit(2).min(3),
            hair: digit(3) % 10,
            shirt: digit(4) % 10,
            pants: digit(5) % 10,
        });
    }
    out
}

/// One professional of progolfers.dta: name, body, looks and twelve skill values (hex digits).
#[derive(Clone, Debug, Default, serde::Serialize, serde::Deserialize)]
pub struct Pro {
    pub name: String,
    pub body: u8,
    pub skin: u8,
    pub hat: u8,
    pub shirt: u8,
    pub pants: u8,
    pub skills: [u8; 12],
}

/// The pack's pro files join the famous golfer table after progolfers.dta (0x4659a0, after the .dta: every `*.pro` of
/// `Themes\<pack>`, read through 0x437fa0, becomes the next row): the name, body (+0x23 high nibble), skin (+0x27), hat
/// (+0x23 low nibble), shirt (+0x24) and trousers (+0x25) of its record. The exe writes the file's ten skill bytes (and
/// their sum) one row further on, so each row takes the skills of the file before it, the first none; the row after the
/// last stays unused. The table holds 100 rows.
pub fn append_pro_files(pros: &mut Vec<Pro>, files: &[crate::championship::ProFile]) {
    let mut carried = [0u8; 12];
    for f in files {
        if pros.len() >= 100 {
            break;
        }
        let p = &f.person;
        pros.push(Pro {
            name: p.name.clone(),
            body: p.b23 >> 4,
            skin: p.skin,
            hat: p.b23 & 0xf,
            shirt: p.shirt,
            pants: p.pants,
            skills: carried,
        });
        carried = [0; 12];
        carried[..10].copy_from_slice(&f.skills[..10]);
    }
}

pub fn parse_pros(text: &str) -> Vec<Pro> {
    let mut out = Vec::new();
    for l in text.lines() {
        let l = l.trim_end_matches('\r');
        if l.starts_with('*') || l.trim().is_empty() || out.len() >= 100 {
            continue;
        }
        let f: Vec<&str> = l.split(',').collect();
        if f.len() < 7 {
            continue;
        }
        let d = |i: usize| f[i].trim().chars().next().and_then(|c| c.to_digit(16)).unwrap_or(0) as u8;
        let mut skills = [0u8; 12];
        for (k, c) in f[6].trim().chars().take(12).enumerate() {
            skills[k] = c.to_digit(16).unwrap_or(0) as u8;
        }
        out.push(Pro { name: f[0].trim().to_string(), body: d(1), skin: d(2), hat: d(3), shirt: d(4), pants: d(5), skills });
    }
    out
}

impl Club {
    /// A visitor's name as the exe shows it (0x4676e0).
    pub fn vip_name(&self, g: usize) -> String {
        let gg = &self.g[g];
        match gg.vip() {
            0x40 => trf("STR_VIP_NAME_COMMISSIONER", &[("SUFFIX", &suffix(self.purchases & 0x7f))]),
            0x60 => trf("STR_VIP_NAME_CEO", &[("SUFFIX", &suffix((gg.kind & 0x1f) as i32))]),
            0x80 => {
                if g & 1 != 0 {
                    tr("STR_VIP_NAME_HEIRESS_IVANA").to_string()
                } else {
                    tr("STR_VIP_NAME_HEIRESS_AGNES").to_string()
                }
            }
            _ => self.name(g),
        }
    }

    fn rating_text(&self) -> (i32, String) {
        if self.difficulty < 2 {
            (self.ratings.fun, trf("STR_VIP_FUN_RATING_UP_TO", &[("RATING", &self.ratings.fun)]))
        } else {
            let s = self.ratings.skill;
            (s, trf("STR_VIP_SKILL_RATING_UP_TO", &[("RATING", &skill_text(s))]))
        }
    }

    fn rating_goal(&self, v: i32) -> String {
        if self.difficulty < 2 {
            trf("STR_VIP_GOAL_FUN_OVER", &[("RATING", &v)])
        } else {
            trf("STR_VIP_GOAL_SKILL_OVER", &[("RATING", &skill_text(v))])
        }
    }

    /// The visitor part of pairing (0x45de80): every sixth pair's second golfer may become a visitor, the kind fixed by the
    /// pair number, when the club meets that visitor's requirement and none of the kind is on the course.
    pub(crate) fn vip_check(&mut self, rng: &mut ExeRng, p2: usize, sandbox: bool) {
        let mut present = 0u32;
        for o in self.g.iter().take(crate::golfer::SLOTS) {
            if o.hole > 0 && o.roster >= ROSTER_CHALLENGER {
                present |= 1 << (o.roster - ROSTER_CHALLENGER);
            }
        }
        if sandbox {
            present = 0xffff_ffff;
        }
        let (rating, lead) = self.rating_text();
        let v = (p2 / 2) % 6;
        let diff = self.difficulty;
        let k = self.ceo_count;
        let mut ceo = v == 1 && present & 4 == 0 && k < 8 && self.par(2 + 2 * k) != 0;
        let mut comm = v == 3 && present & 2 == 0 && !self.island && (self.purchases + 2) * (diff + 2) * 50 < rating;
        let mut heir =
            v == 5 && present & 4 == 0 && (self.donations + 1) * (self.donations + 1) * 25 < rating && self.landmarks_owned < 0xffff;
        // the port's Cheats box, "Force a VIP visit now" (cheats.rs): this pair brings a visitor whatever its number and the
        // ratings, the first of the three this pair's number points at that can come (a sandbox game's visitors too)
        if std::mem::take(&mut self.vip_forced) {
            (ceo, comm, heir) = crate::cheats::forced_vip(self, v);
        }
        if !(ceo || comm || heir) {
            return;
        }
        let (kind, roster, class, rolls, fixed) = if ceo {
            (0x60 + k as u8, ROSTER_CEO, 7, 9, 0x0e00)
        } else if comm {
            (0x40 + (self.purchases & 0xff) as u8, ROSTER_COMMISSIONER, 7, 12, 0x0038)
        } else {
            (0x80, ROSTER_HEIRESS, 6, 12, 0x0300)
        };
        self.g[p2].kind = kind;
        self.g[p2].roster = roster;
        let who = match kind & 0xe0 {
            0x60 => tr("STR_VIP_CEO"),
            0x40 => tr("STR_VIP_COMMISSIONER"),
            _ => tr("STR_VIP_HEIRESS"),
        };
        let name = self.vip_name(p2);
        // the exe's sentences (0x4d3af0, 0x4d3a88, 0x4d3a14 after 0x4d3b78); the CEO's overwrites the rating lead
        let text = if ceo {
            trf("STR_VIP_CEO_PLAYING", &[("WHO", &who), ("VISITOR", &name)])
        } else if comm {
            trf("STR_VIP_COMMISSIONER_PLAYING", &[("LEAD", &lead), ("WHO", &who), ("VISITOR", &name)])
        } else {
            trf("STR_VIP_HEIRESS_PLAYING", &[("LEAD", &lead), ("WHO", &who), ("VISITOR", &name)])
        };
        self.message_by(text, p2 as i32, 1);
        self.g[p2].class = class;
        let mut mask: u16 = fixed;
        for _ in 0..4 {
            mask |= 1 << rng.below(rolls);
        }
        self.g[p2].skill_mask = mask;
        if ceo {
            self.ceo_count = (k + 1) | 0x80;
        }
        self.sound(0x26, None);
        let p1 = p2 ^ 1;
        self.g[p1].field_ae = 0;
        self.g[p2].field_ae = 0;
        if !ceo {
            self.g[p1].story = -1;
            self.g[p2].story = -1;
        }
    }

    /// A visitor's special skill values (1..8 for each skill of the mask), set when the pair is made.
    pub(crate) fn vip_skills(&mut self, rng: &mut ExeRng, p2: usize) {
        if self.g[p2].kind == 0 {
            return;
        }
        for i in 0..12 {
            if self.g[p2].skill_mask & (1 << i) != 0 {
                self.g[p2].skills[i] = (rng.below(8) + 1) as u8;
            }
        }
    }

    /// A visitor's verdict at the end of the round (0x4266b0): pleased only after playing to the last hole with mood 3 or more.
    pub(crate) fn vip_verdict(&mut self, rng: &mut ExeRng, g: usize) {
        let v = self.g[g].vip();
        let m = self.g[g].mood;
        let finished = self.par(self.g[g].hole) == 0;
        let name = self.vip_name(g);
        // the exe's pieces: "My partner " (0x4c70c4), the name, " has quit, even though " (0x4c70ac), before the remark
        let partner_quit = |me: &Club| {
            if m > 2 {
                trf("STR_VIP_PARTNER_QUIT", &[("PARTNER", &me.name(g ^ 1))])
            } else {
                String::new()
            }
        };
        match v {
            0x60 => {
                let k = self.ceo_count & !0x80;
                self.ceo_count = k;
                if finished && m > 2 {
                    if 2 * k < self.g[g].hole {
                        let amount = ((m > 4) as i32 + 1) * 50;
                        self.message_by(
                            // the exe's pieces (0x4c71b0, 0x4c7124, 0x4c7100)
                            trf("STR_VIP_CEO_YES", &[("VISITOR", &name), ("MONEY", &money_digits(amount as i64 * 100))]),
                            g as i32,
                            1,
                        );
                        self.out.push(Event::Earn { units: amount, column: Column::Other, at: (self.g[g].x, self.g[g].y) });
                        self.log_event(crate::records::log::CEO, k);
                        self.sound(0x19, None);
                    } else {
                        self.message_by(trf("STR_VIP_CEO_NEEDS_MORE_HOLES", &[("VISITOR", &name)]), g as i32, 1);
                        self.ceo_count = k - 1;
                    }
                } else {
                    let pq = partner_quit(self);
                    self.message_by(trf("STR_VIP_CEO_NO", &[("VISITOR", &name), ("QUIT", &pq), ("REMARK", &mood_remark(m))]), g as i32, 1);
                }
            }
            0x40 => {
                if finished && m > 2 {
                    let goal = self.rating_goal((self.difficulty + 2) * (self.purchases + 3) * 50);
                    self.message_by(trf("STR_VIP_COMMISSIONER_YES", &[("VISITOR", &name), ("GOAL", &goal)]), g as i32, 1);
                    self.sound(0x2f, None);
                    self.land_offer = true;
                } else {
                    let pq = partner_quit(self);
                    self.message_by(
                        trf("STR_VIP_COMMISSIONER_NO", &[("VISITOR", &name), ("QUIT", &pq), ("REMARK", &mood_remark(m))]),
                        g as i32,
                        1,
                    );
                }
            }
            0x80 => {
                if !finished || m < 3 {
                    let pq = partner_quit(self);
                    self.message_by(
                        trf("STR_VIP_HEIRESS_NO", &[("VISITOR", &name), ("QUIT", &pq), ("REMARK", &mood_remark(m))]),
                        g as i32,
                        1,
                    );
                } else {
                    let mut t = m;
                    let mut id;
                    loop {
                        id = crate::geom::clamp(rng.below(t), 0, 15);
                        if self.landmarks_owned & (1 << id) == 0 {
                            break;
                        }
                        t += 1;
                        if t >= 25 {
                            break;
                        }
                    }
                    let effect = match id & 3 {
                        0 => tr("STR_VIP_EFFECT_HAPPY_THOUGHTS").to_string(),
                        1 => tr("STR_VIP_EFFECT_NO_DANDELIONS").to_string(),
                        2 => trf("STR_VIP_EFFECT_SKILLS", &[("PRO", &self.roster.first().map(|p| p.name.clone()).unwrap_or_default())]),
                        _ => tr("STR_VIP_EFFECT_STORIES").to_string(),
                    };
                    // the heiress's goal reads "is up to", the commissioner's "is over"
                    let v = (self.donations + 2) * (self.donations + 2) * 25;
                    let goal = if self.difficulty < 2 {
                        trf("STR_VIP_GOAL_FUN_UP_TO", &[("RATING", &v)])
                    } else {
                        trf("STR_VIP_GOAL_SKILL_UP_TO", &[("RATING", &skill_text(v))])
                    };
                    // the exe's pieces (0x4c6fa4, 0x4c6f88, 0x4074a0's name with its article, 0x4c6f54, the effect, 0x4c6e78, the
                    // short name, 0x4c6e54, the price, 0x4c6e38, the goal, 0x4c6df8)
                    let text = trf(
                        "STR_VIP_HEIRESS_YES",
                        &[
                            ("VISITOR", &name),
                            ("A_LANDMARK", &landmark_after_a(id)),
                            ("EFFECT", &effect),
                            ("LANDMARK", &landmark_short_name(id)),
                            ("MONEY", &money_digits(((id * 5 + 5) * 200) as i64)),
                            ("GOAL", &goal),
                        ],
                    );
                    self.message_by(text, g as i32, 1);
                    self.donations += 1;
                    self.sound(0x2f, None);
                    self.landmarks_owned |= 1 << id;
                    self.free_landmarks |= 1 << id;
                    self.log_event(crate::records::log::HEIRESS, id);
                }
            }
            _ => {}
        }
    }

    /// A home site's monthly celebrity draw (0x417a08): a celebrity is drawn for every re-valued site; an unowned site worth
    /// enough is bought by them. Returns the owner given, if any.
    pub fn celebrity_home(&mut self, rng: &mut ExeRng, val: i32, owner: i32, home: (i32, i32)) -> Option<i32> {
        if !self.celebrities.iter().any(|c| c.kind != 0xff) {
            return None;
        }
        let c = loop {
            let c = rng.below(100);
            if self.celebrities.get(c as usize).is_some_and(|x| x.kind != 0xff) {
                break c;
            }
        };
        if owner != 0 || val <= (300 * self.celeb_homes + 400) * (self.difficulty + 2) {
            return None;
        }
        let cel = &self.celebrities[c as usize];
        let kind = celebrity_type(cel.kind);
        self.message(trf("STR_VIP_CELEBRITY_HOME", &[("KIND", &kind), ("CELEBRITY", &cel.name)]));
        self.sound(0x33, None);
        self.celeb_homes += 1;
        self.log_event(crate::records::log::CELEB, c);
        self.spawn_resident(rng, c, home.0, home.1);
        Some(c + 1)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn data_files() {
        let c = parse_celebrities("* comment\nSylvester Stallion,A,1,4,0,0\nPop Person,B,2,1,3,9\n");
        assert_eq!(c.len(), 2);
        assert_eq!(c[1].kind, 1);
        assert_eq!(c[1].pants, 9);
        let p = parse_pros("* x\nJoe Pro,2          ,1,0,4,0,3213222241\nIron Hands Hacker,3,1,1,1,1,00A0000000\n");
        assert_eq!(p[0].skills[0], 3);
        assert_eq!(p[1].skills[2], 10);
        // pro files: each row takes the skills of the file before it
        let mut pros = p.clone();
        let file = |name: &str, s: u8| {
            let mut f = crate::championship::ProFile::default();
            f.person.name = name.into();
            f.person.b23 = 0x21;
            f.skills = [s; 16];
            f
        };
        append_pro_files(&mut pros, &[file("A", 3), file("B", 5)]);
        assert_eq!((pros[2].name.as_str(), pros[2].body, pros[2].hat, pros[2].skills[0]), ("A", 2, 1, 0));
        assert_eq!((pros[3].name.as_str(), pros[3].skills[0], pros[3].skills[10]), ("B", 3, 0));
        assert_eq!(mood_remark(5), "This course is quite nice.");
    }

    #[test]
    fn verdicts_in_the_exe_words() {
        assert_eq!(landmark_after_a(0), " garden sundial");
        assert_eq!(landmark_after_a(2), "n authentic Civil War cannon");
        let mut cl = Club::default();
        let mut rng = ExeRng::from_clock(1);
        cl.holes[1].par = 4;
        cl.g[3].kind = 0x80;
        cl.g[3].hole = 1;
        cl.g[3].mood = 1;
        cl.vip_verdict(&mut rng, 3);
        let said: Vec<String> =
            cl.out.iter().filter_map(|e| if let Event::Message { text, .. } = e { Some(text.clone()) } else { None }).collect();
        assert_eq!(
            said,
            ["Wealthy Heiress Ivana Richman has decided not to donate a landmark to your course. 'I don't like this course much.' \
              she comments. 'I'll be back in a few months.'"]
        );
    }
}
