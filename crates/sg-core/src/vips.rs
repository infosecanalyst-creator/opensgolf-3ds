//! Special visitors (the exe's VIPs): the Corporate CEO, the County commissioner and the Wealthy heiress take the place of the
//! second golfer of every sixth pair when the club has earned them, play the round with skills of their own, and give a
//! verdict at the end: an investment, a land expansion, or a donated landmark. Also the celebrities who buy vacation homes on
//! valuable home sites.
//!
//! Facts are from the publisher's golf.exe (docs/PUBLISHER_EXE_NOTES.md, "Special visitors"), restated in our own words.

use crate::golfer::{Club, Column, Event};
use crate::land::ExeRng;

/// Roster slots of the visitors (loaded from the Standard theme's golfer files).
pub const ROSTER_CHALLENGER: i32 = 76;
pub const ROSTER_COMMISSIONER: i32 = 77;
pub const ROSTER_HEIRESS: i32 = 78;
pub const ROSTER_CEO: i32 = 79;

/// Landmark names by type (the landmark tool's 0..13, then two only donations give, then the eyesores).
pub fn landmark_name(t: i32) -> &'static str {
    const N: [&str; 16] = [
        "garden sundial",
        "traditional barn",
        "authentic Civil War cannon",
        "ancient stonehenge rock",
        "operating water mill",
        "unusual rock face",
        "authentic Civil War statue",
        "scenic New England lighthouse",
        "peaceful Buddha",
        "Dutch windmill",
        "historic statue",
        "haunting Easter Island head",
        "exquisite pagoda",
        "historic Hatteras lighthouse",
        "ornate oriental house",
        "dusty dinosaur tarpit",
    ];
    N.get(t as usize).copied().unwrap_or("landmark")
}

/// What a visitor thinks of the course at mood m (0x469a20).
pub fn mood_remark(m: i32) -> &'static str {
    match m {
        0 => "I hate this stupid course!",
        1 => "I don't like this course much.",
        2 => "I'm not having much fun today.",
        3 => "This course is almost adequate.",
        4 => "Well, I guess this course is OK.",
        5 => "This course is quite nice.",
        6 => "This is a really good course.",
        7 => "This course is excellent.",
        8..=127 => "This is the best course I've ever played!",
        _ => "I hate this course! I'm leaving!",
    }
}

/// Second, third, ... visitor of the same kind.
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
pub const CELEBRITY_TYPES: [&str; 11] = [
    "action star",
    "pop star",
    "politician",
    "comedian",
    "supermodel",
    "fitness star",
    "comedienne",
    "leading man",
    "movie star",
    "rock and roller",
    "athlete",
];

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
            0x40 => format!("I.M. Picky{}", suffix(self.purchases & 0x7f)),
            0x60 => format!("J.P. Bigdome{}", suffix((gg.kind & 0x1f) as i32)),
            0x80 => {
                if g & 1 != 0 {
                    "Ivana Richman".to_string()
                } else {
                    "Agnes Heffledorp".to_string()
                }
            }
            _ => self.name(g),
        }
    }

    fn rating_text(&self) -> (i32, String) {
        if self.difficulty < 2 {
            (self.ratings.fun, format!("Your Fun Rating is up to {}", self.ratings.fun))
        } else {
            let s = self.ratings.skill;
            (s, format!("Your Skill Rating is up to {}.{:02}", s / 100, (s % 100).abs()))
        }
    }

    fn rating_goal(&self, v: i32) -> String {
        if self.difficulty < 2 {
            format!("fun rating is over {v}")
        } else {
            format!("skill rating is over {}.{:02}", v / 100, (v % 100).abs())
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
        let ceo = v == 1 && present & 4 == 0 && k < 8 && self.par(2 + 2 * k) != 0;
        let comm = v == 3 && present & 2 == 0 && !self.island && (self.purchases + 2) * (diff + 2) * 50 < rating;
        let heir = v == 5 && present & 4 == 0 && (self.donations + 1) * (self.donations + 1) * 25 < rating && self.landmarks_owned < 0xffff;
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
            0x60 => "Corporate CEO",
            0x40 => "County commissioner",
            _ => "Wealthy Heiress",
        };
        let name = self.vip_name(p2);
        let text = if ceo {
            format!("{who} {name} is playing your course today. If he likes it, he may invest in your club.")
        } else if comm {
            format!("{lead} and your fame is spreading... {who} {name} is playing your course today. If he likes it, he may approve an expansion.")
        } else {
            format!("{lead} and your fame is spreading... {who} {name} is playing your course today. If she likes it, she may donate a landmark.")
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
        let partner_quit = |me: &Club| {
            if m > 2 {
                format!(" My partner {} has quit, even though", me.name(g ^ 1))
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
                            format!("Corporate CEO {name} has decided to invest ${} for a seat on the board, he says.", amount * 100),
                            g as i32,
                            1,
                        );
                        self.out.push(Event::Earn { units: amount, column: Column::Other, at: (self.g[g].x, self.g[g].y) });
                        self.log_event(crate::records::log::CEO, k);
                        self.sound(0x19, None);
                    } else {
                        self.message_by(format!("Corporate CEO {name} has decided not to invest in your club."), g as i32, 1);
                        self.ceo_count = k - 1;
                    }
                } else {
                    let pq = partner_quit(self);
                    self.message_by(
                        format!("Corporate CEO {name} has decided not to invest in you.{pq} \"{}\" he fumes.", mood_remark(m)),
                        g as i32,
                        1,
                    );
                }
            }
            0x40 => {
                if finished && m > 2 {
                    let goal = self.rating_goal((self.difficulty + 2) * (self.purchases + 3) * 50);
                    self.message_by(format!(
                        "County commissioner {name} has decided to approve an expansion request! \"I'll be back again if your {goal},\" he says."
                    ), g as i32, 1);
                    self.sound(0x2f, None);
                    self.land_offer = true;
                } else {
                    let pq = partner_quit(self);
                    self.message_by(
                        format!(
                            "County commissioner {name} has decided not to approve your expansion request.{pq} \"{}\" he comments.",
                            mood_remark(m)
                        ),
                        g as i32,
                        1,
                    );
                }
            }
            0x80 => {
                if !finished || m < 3 {
                    let pq = partner_quit(self);
                    self.message_by(format!(
                        "Wealthy Heiress {name} has decided not to donate a landmark.{pq} \"{}\" she comments. \"I'll be back in a while.\"",
                        mood_remark(m)
                    ), g as i32, 1);
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
                        0 => "golfers will have happy thoughts".to_string(),
                        1 => "no dandelions will appear".to_string(),
                        2 => format!("{}'s skills will improve rapidly", self.roster.first().map(|p| p.name.clone()).unwrap_or_default()),
                        _ => "golfer stories will proceed happily".to_string(),
                    };
                    let goal = self.rating_goal((self.donations + 2) * (self.donations + 2) * 25);
                    self.message_by(format!(
                        "Wealthy Heiress {name} has decided to donate a {} to your course. In the area near it, {effect}. More can be added to your course for ${} each. \"I'll be back if your {goal},\" she says.",
                        landmark_name(id),
                        (id * 5 + 5) * 200
                    ), g as i32, 1);
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
        let kind = CELEBRITY_TYPES[(cel.kind as usize).min(10)];
        self.message(format!("International {kind} {} has purchased a vacation home at your golf course!", cel.name));
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
        assert_eq!(mood_remark(5), "This course is quite nice.");
    }
}
