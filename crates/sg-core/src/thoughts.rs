//! What golfers think and say (text routine 0x469b00): one line per mood event, chosen by the pair's relationship, the
//! person, gender, the hole and the event's argument, with the golfer's and partner's names and an event noun put in.
//!
//! Rules are from the publisher's golf.exe (docs/PUBLISHER_EXE_NOTES.md, "Golfer thoughts"). The exe's strings are data that
//! is not available beyond the first words its labels show; the lines here start with those words where they are known and
//! are otherwise written for this port.

use crate::course::{f, idx, inside, Course, TYPES};
use crate::golfer::Club;

/// The tone of a line: neutral, pleased, upset.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Tone {
    Plain,
    Good,
    Bad,
}

/// A line of thought: its text, its tone, and whether the partner says it (about or to the golfer).
#[derive(Clone, Debug, PartialEq)]
pub struct Line {
    pub text: String,
    pub tone: Tone,
    pub partner: bool,
}

/// Terrain names: the single name and the group name (trees use the group name in most remarks).
pub fn terrain(t: u8) -> (&'static str, &'static str) {
    match t {
        0 => ("tee", "tee box"),
        1 => ("green", "green"),
        2 | 3 => ("fairway", "fairway"),
        4 => ("rough", "rough"),
        5 => ("deep rough", "deep rough"),
        6 => ("mounds", "mounds"),
        7 => ("sand trap", "sand"),
        8 => ("waste bunker", "waste area"),
        9 => ("pot bunker", "pot bunker"),
        10 => ("ravine", "ravine"),
        11 => ("brush", "brush"),
        12 => ("rocks", "rocks"),
        13 => ("tree", "trees"),
        14 => ("pine", "pines"),
        15 => ("palm", "palms"),
        16 => ("elm", "elms"),
        17 => ("water", "water"),
        18 => ("wetlands", "wetlands"),
        19 => ("marsh", "marsh"),
        20 => ("out of bounds", "out of bounds"),
        _ => ("building", "buildings"),
    }
}

/// Club names by club number (0x40a9a0).
pub const CLUBS: [&str; 14] = [
    "Driver",
    "3 Wood",
    "4 Wood",
    "2 Iron",
    "3 Iron",
    "4 Iron",
    "5 Iron",
    "6 Iron",
    "7 Iron",
    "8 Iron",
    "9 Iron",
    "Lob Wedge",
    "Sand Wedge",
    "Putter",
];

const PETS: [&str; 4] = [", honey", ", sugar", ", dear", ", sweetie"];

fn is_tree(t: u8) -> bool {
    TYPES[(t as usize).min(22)].class == 13
}

/// The name of the scenery on a tile (0x407700, simplified to what this port tracks): landmarks, flower beds, scenic trees,
/// fountains, bridges and the like.
pub fn feature(c: &Course, a: i32, b: i32, theme: u8) -> String {
    if !inside(a, b) {
        return "view".into();
    }
    let i = idx(a, b);
    let t = c.ty[i];
    let fl = c.flags[i];
    if t == 21 {
        return "home site".into();
    }
    if t == 22 {
        let o = c.object_at(a, b);
        if let Some(obj) = (o >= 0).then(|| c.objects.get(o as usize)).flatten() {
            if obj.kind == 4 {
                return crate::vips::landmark_name(obj.sub).to_lowercase();
            }
            if obj.kind == 2 {
                return "flowerbed".into();
            }
        }
        return "landmark".into();
    }
    if fl & f::FLOWERS != 0 {
        let k = (a * 7 + b) % 5;
        return match theme {
            1 => ["red poppy bed", "orange flowerbed", "purple iris patch", "blue foxglove", "pink sage bush"][k as usize],
            2 => ["aqua hibiscus", "orange bougainvillea", "purple orchid", "blue hydrangea", "red rose bush"][k as usize],
            _ => ["green flowerbed", "yellow daisy bed", "purple iris patch", "white carnation", "red rose bush"][k as usize],
        }
        .into();
    }
    match TYPES[(t as usize).min(22)].class {
        4 if t == 4 => if theme == 1 { "mysterious bones" } else { "ornamental grass" }.into(),
        4 if t == 10 => if theme == 1 { "rock formation" } else { "scenic tree" }.into(),
        4 if t == 12 => if theme == 1 { "natural bridge" } else { "rock formation" }.into(),
        4 => "rose bush".into(),
        7 => "rhododendron".into(),
        12 => "rock formation".into(),
        13 => if theme == 1 { "scenic cactus" } else { "scenic tree" }.into(),
        17 if fl & f::PATH != 0 => "scenic bridge".into(),
        17 => if theme == 1 { "rock formation" } else { "fountain" }.into(),
        18 => "fountain".into(),
        _ => "wildflower".into(),
    }
}

impl Club {
    fn rel(&self, g: usize) -> i32 {
        self.g[g].field_ae
    }

    fn pet(g: usize) -> &'static str {
        PETS[g & 3]
    }

    /// The text of mood event `id` with argument `arg` for golfer g (0x469b00).
    pub fn thought(&self, c: &Course, id: u32, arg: i32, g: usize) -> Line {
        self.thought_as(c, id, arg, g, None, true)
    }

    /// The stock text of an event as golfer g would say it if g were person `record` (the Customise screen asks this for
    /// its preview slot 0x99, which is past the golfer slots and so never takes a person's own dialogue slot).
    pub fn stock_thought(&self, c: &Course, id: u32, arg: i32, g: usize, record: usize) -> Line {
        self.thought_as(c, id, arg, g, Some(record as i32), false)
    }

    fn thought_as(&self, c: &Course, id: u32, arg: i32, g: usize, record: Option<i32>, own: bool) -> Line {
        let rel = self.rel(g);
        let r = record.unwrap_or(self.g[g].roster.max(0));
        let male = match record {
            Some(k) => self.roster.get(k as usize).map(|p| p.male_bit()).unwrap_or(1) != 0,
            None => self.male(g) != 0,
        };
        let p = g ^ 1;
        let me = self.name(g);
        let partner = self.name(p);
        let theme = c.theme;
        let mut tone = Tone::Plain;
        let mut by_partner = false;
        let ty = arg.clamp(0, 22) as u8;
        let (t1, t2) = terrain(ty);
        let place = if is_tree(ty) { format!("under the {t1}") } else { format!("in the {t1}") };
        let noun = if is_tree(ty) { t2 } else { t1 };
        let hole = self.g[g].hole.clamp(0, 18) as usize;
        // a person's own dialogue slot for this event comes first (0x469b00 for golfer slots below 0x98)
        if own && g < crate::golfer::SLOTS {
            let own = crate::roster::SAYING_CODES.iter().position(|&c| c == id).and_then(|row| {
                self.roster.get(r as usize).and_then(|p| p.saying(row)).map(|s| s.replace("PARTNER", &partner).replace("MYNAME", &me))
            });
            if let Some(text) = own {
                return Line { text: text.replace("DATA", t1), tone, partner: false };
            }
        }
        let text: String = match id {
            0x01 => {
                tone = Tone::Good;
                if rel & 1 == 0 {
                    match (r & 1, r & 2) {
                        (0, 0) => format!("How'd you like that shot, {partner}?"),
                        (0, _) => format!("Oh yeah, nothin' but {t1}!"),
                        (_, 0) => "Whoa, I am so GOOD!".into(),
                        _ => "Oh yeah, I rock!".into(),
                    }
                } else {
                    by_partner = true;
                    let pm = self.male(p) != 0;
                    let pr = self.g[p].roster & 1;
                    match (pr, pm) {
                        (0, true) => format!("{me}, I couldn't do it any better!"),
                        (0, false) => format!("{me}, you are soooo GOOD!"),
                        (_, true) => format!("{me}, what a great shot!"),
                        _ => format!("{me}, you truly rock!"),
                    }
                }
            }
            0x02 => {
                tone = Tone::Bad;
                match rel {
                    2 => {
                        by_partner = true;
                        format!("Ha, {me}, you're {place}!")
                    }
                    4 => {
                        by_partner = true;
                        format!("Oh darn, you're {place}{}!", Self::pet(g))
                    }
                    _ => format!("Darn, I'm {place}!"),
                }
            }
            0x03 => {
                tone = Tone::Bad;
                match rel {
                    2 => {
                        by_partner = true;
                        format!("You look even better {place}, {me}.")
                    }
                    3 => {
                        by_partner = true;
                        format!("Oh, {me}, now you're {place}!")
                    }
                    4 => {
                        by_partner = true;
                        format!("You prefer being {place}, do you?")
                    }
                    _ => format!("{}, now I'm {place}!", ["Great", "Oh no", "Swell", "Lovely"][(r & 3) as usize]),
                }
            }
            0x04 => {
                tone = Tone::Bad;
                if rel == 4 {
                    by_partner = true;
                    format!("Only you could miss this shot, {me}.")
                } else {
                    "This shot looks pretty easy. Yawn.".into()
                }
            }
            0x05 | 0x25 | 0x26 => match rel {
                1 => {
                    by_partner = true;
                    format!("Careful of the {noun}{}.", Self::pet(g))
                }
                2 if g & 1 == 0 => {
                    by_partner = true;
                    format!("Say hello to Mr. {noun} for me, {me}.")
                }
                2 => {
                    by_partner = true;
                    format!("You're going into the {noun}, {me}.")
                }
                3 | 5 => {
                    by_partner = true;
                    format!("Watch out for the {noun}, {me}.")
                }
                _ => match id {
                    0x05 => format!("Eeek, I gotta stay away from the {noun}."),
                    0x25 => format!("Eeek, I gotta hit it past the {noun}."),
                    _ => format!("Eeek, I gotta watch out for the {noun}."),
                },
            },
            0x06 => {
                tone = Tone::Good;
                "I'll use the slope on this shot.".into()
            }
            0x07 => {
                tone = Tone::Good;
                if self.g[g].flags & 0x10000 != 0 {
                    "I love riding over this scenic bridge.".into()
                } else {
                    "I love walking over this scenic bridge.".into()
                }
            }
            0x08 => {
                tone = Tone::Bad;
                "Who designed this #@%&! course?".into()
            }
            0x09 => {
                tone = Tone::Bad;
                "Hey, that ball almost hit me!".into()
            }
            0x0a => {
                tone = Tone::Bad;
                let w = if t1.ends_with('s') { "these" } else { "this" };
                format!("Must I walk through {w} {t1}?")
            }
            0x0b => {
                tone = Tone::Good;
                let what = feature(c, arg / 50, arg % 50, theme);
                let adj = if arg & 0x100 != 0 { "nice" } else { "beautiful" };
                if rel == 1 {
                    format!("Look at this {adj} {what}{}.", Self::pet(p))
                } else {
                    format!("Check out this {adj} {what}!")
                }
            }
            0x0c => {
                tone = Tone::Bad;
                format!("Darn that stupid {t1}!")
            }
            0x0d => {
                tone = Tone::Bad;
                if rel == 2 || rel == 4 {
                    by_partner = true;
                    format!("Did I hear a splash, {me}?")
                } else {
                    ["Argh, I drowned it!", "Did my ball go for a swim?", "Nooooo, I'm in the drink!"][(r % 3) as usize].into()
                }
            }
            0x0e => {
                tone = Tone::Bad;
                "I'm getting a little thirsty.".into()
            }
            0x0f => {
                tone = Tone::Bad;
                "I'm starting to get hungry.".into()
            }
            0x10 => "Gosh, I kinda hooked that shot.".into(),
            0x11 => "Oops, I really sliced that shot.".into(),
            0x12 => {
                tone = Tone::Good;
                "There's nothing like a good snack on the course.".into()
            }
            0x13 | 0x17 => {
                // the hole just finished: the golfer has moved on to the next one by the time the thought shows, so take it
                // from the thought's stamp (hole * 11 + strokes, written when the event fired)
                let gg = &self.g[g];
                let hole = if gg.thought as u32 == id { (gg.thought_stamp / 11) as usize } else { hole }.min(18);
                let par = self.holes[hole].par;
                let d = self.g[g].card[hole] as i32 - par;
                let word = match d {
                    -4 => "Triple Eagle!",
                    -3 => "Double Eagle!",
                    -2 => "Eagle!",
                    -1 => "Birdie!",
                    0 => "Par.",
                    1 => "Bogey.",
                    2 => "Double Bogey.",
                    3 => "Triple Bogey.",
                    _ => "Arggg!",
                };
                let fl = self.holes[hole].flags;
                let remark = if fl & 4 != 0 && d >= 2 {
                    tone = Tone::Bad;
                    "This hole is too hard.".to_string()
                } else if fl & 8 != 0 && d < 0 {
                    tone = Tone::Bad;
                    "This hole is too easy.".to_string()
                } else if id == 0x17 {
                    tone = Tone::Bad;
                    "This hole is too hard/easy.".to_string()
                } else {
                    let s = (g + hole) & 3;
                    let class = self.g[g].class;
                    let mask = match s {
                        0 if class & 1 != 0 => 0x100,
                        1 if class & 2 != 0 => 0x200,
                        2 if class & 4 != 0 => 0x400,
                        _ => 0,
                    };
                    if mask & fl != 0 && d <= 0 {
                        match mask {
                            0x100 => "My length skill helps on this hole.",
                            0x200 => "This hole rewards my accuracy.",
                            _ => "I had to use some imaginative shots here.",
                        }
                        .to_string()
                    } else if (rel == 2 || rel == 4) && self.g[p].card[hole] != 0 {
                        if self.g[p].card[hole] > self.g[g].card[hole] {
                            "take that, loser!".into()
                        } else {
                            "you were lucky on this hole.".into()
                        }
                    } else {
                        course_opinion(arg, d).into()
                    }
                };
                format!("{word} {remark}")
            }
            0x14 => {
                tone = Tone::Bad;
                format!("Gee, what an ugly {}.", feature(c, arg / 50, arg % 50, theme))
            }
            0x15 => {
                tone = Tone::Bad;
                "I'm tired of waiting for these slowpokes.".into()
            }
            0x16 => {
                tone = Tone::Good;
                let who = self.celebrities.get(arg.max(0) as usize).map(|c| c.name.clone()).unwrap_or_else(|| "a movie star".into());
                format!("Hey, that's {who}'s house!")
            }
            0x18 => {
                tone = Tone::Bad;
                "Yuck, there's something growing here that shouldn't be.".into()
            }
            0x19 => {
                tone = Tone::Good;
                "Ahhh, a cool foamy beverage.".into()
            }
            0x1a => match rel {
                1 => {
                    by_partner = true;
                    format!("You look a bit tired{}.", Self::pet(g))
                }
                3 | 5 => {
                    by_partner = true;
                    format!("You look a bit tired, {me}.")
                }
                _ => "I'm starting to get tired.".into(),
            },
            0x1b => {
                tone = Tone::Good;
                "This bench is so comfortable.".into()
            }
            0x1c => {
                tone = Tone::Good;
                let what = feature(c, arg / 50, arg % 50, theme);
                match rel {
                    1 => format!("Look at that lovely {what}{}.", Self::pet(p)),
                    3 | 5 => format!("Look at that lovely {what}, {partner}."),
                    _ => match (male, r & 1) {
                        (false, 0) => format!("My, that's a lovely {what}."),
                        (false, _) => format!("Oh, what a nice {what}!"),
                        (true, 0) => format!("I am liking that {what}."),
                        _ => format!("Gee, that's a cool {what}!"),
                    },
                }
            }
            0x1d => {
                tone = Tone::Good;
                "I like the variety on this course.".into()
            }
            0x1e => {
                tone = Tone::Bad;
                let fl = self.holes[hole].flags;
                let pf = self.holes[hole.saturating_sub(1)].flags;
                if fl & 0x20 != 0 && pf & 0x20 != 0 {
                    "Great, another dogleg left.".into()
                } else if fl & 0x40 != 0 && pf & 0x40 != 0 {
                    "Yawn, another dogleg right.".into()
                } else if hole > 1 && self.holes[hole].par == self.holes[hole - 1].par {
                    format!("Wow, yet another par {}.", self.holes[hole].par)
                } else {
                    "This hole is like the last hole.".into()
                }
            }
            0x1f => match r & 3 {
                0 => format!("I've never seen such {noun}!"),
                1 => format!("I see a ton of {noun}."),
                2 => format!("Look at all the {noun}!"),
                _ => format!("Could there be any more {noun}?"),
            },
            0x20 => {
                tone = Tone::Good;
                "Should I go long or play it safe?".into()
            }
            0x21 => {
                tone = Tone::Good;
                "Hmmm, should I go left or right?".into()
            }
            0x22 => {
                tone = Tone::Good;
                match arg {
                    0 => "Great, thanks for asking!",
                    1 => "Good, thanks for noticing.",
                    2 => "Thanks for the encouragement.",
                    _ => "",
                }
                .into()
            }
            0x3a => match arg {
                0 => "Hey, I'm trying to play golf here!",
                1 => "Ummm, fine thanks.",
                _ => "Lousy, now that you mention it.",
            }
            .into(),
            0x23 => {
                tone = Tone::Bad;
                match ((self.tick as usize + g * 45) / 80) & 3 {
                    0 => "I will never play this game again!",
                    1 => "I hate my clubs, I hate my hat, I hate this course!",
                    2 => "I am so mad I could just scream!",
                    _ => "Come on, you want a piece of me?",
                }
                .into()
            }
            0x24 => {
                tone = Tone::Bad;
                if arg != 0 { "That guy's gone crazy." } else { "Whoa, she's flipped out." }.into()
            }
            0x27 => {
                tone = Tone::Good;
                let animal =
                    crate::wildlife::KINDS.get(arg.max(0) as usize).map(|k| k.name.to_lowercase()).unwrap_or_else(|| "critter".into());
                if male {
                    format!("I guess I scared that little {animal}.")
                } else {
                    format!("I think I frightened that poor {animal}.")
                }
            }
            0x28 => ["Oh mama!", "Ohh baby!", "Yes!", "Gadzooks!", "Oh yeah!", "A smoker!", "A screamer!", "Nailed it!"][(r & 7) as usize]
                .into(),
            0x29 => if g & 1 == 0 { "Clean and shiny." } else { "Now I'll hit it straight." }.into(),
            0x2a => {
                by_partner = true;
                format!("Lucky bounce, {me}!")
            }
            0x2b => {
                tone = Tone::Bad;
                "Must I walk up this steep slope?".into()
            }
            0x2c => {
                tone = Tone::Good;
                "Never mind... nice landmark, though.".into()
            }
            0x2d => "Hmmm, a tricky uphill shot.".into(),
            0x2e => {
                tone = Tone::Good;
                "OK, a nice downhill shot.".into()
            }
            0x2f => {
                tone = Tone::Bad;
                match r & 3 {
                    0 => format!("{partner}, your attitude stinks."),
                    1 => format!("Stop whining and play, {partner}."),
                    2 => format!("Talk to the hand, {partner}."),
                    _ => format!("Quiet, {partner}, I'm concentrating."),
                }
            }
            0x30 => format!("So what do you do, {partner}?"),
            0x31 => {
                let job = self.roster.get(r as usize).map(|p| p.job.clone()).unwrap_or_default();
                if job.is_empty() {
                    "Oh, this and that.".into()
                } else {
                    format!("I'm a {}.", job.to_lowercase())
                }
            }
            0x33..=0x35 => {
                let (k, second) = match id {
                    0x33 => (10, "really helped my distance."),
                    0x34 => (8, "really improved my accuracy."),
                    _ => (6, "really improved my putting."),
                };
                let lvl = c.level[k];
                let name = match id {
                    0x33 => "driving range",
                    0x34 => "pro shop",
                    _ => "putting green",
                };
                let first = match lvl {
                    1 => format!("This new {name} has"),
                    2 => format!("This upgraded {name} has"),
                    3 => format!("Our deluxe {name} has"),
                    _ => String::new(),
                };
                if first.is_empty() {
                    String::new()
                } else if (self.tick as usize + g * 5) & 8 == 0 {
                    format!("...{second}")
                } else {
                    format!("{first}...")
                }
            }
            0x36 => {
                if self.difficulty <= 1 {
                    tone = Tone::Good;
                }
                format!("I'll use my {} for this shot.", CLUBS[arg.clamp(0, 13) as usize])
            }
            0x37 => "I'll draw this shot from right to left.".into(),
            0x38 => "I'll fade this shot from left to right.".into(),
            0x39 => "I'll bring this ball in high and soft.".into(),
            0x3c => {
                if is_tree(ty) {
                    format!("I'll run this ball low under the {t1}.")
                } else {
                    "I'll run this ball low.".into()
                }
            }
            0x3b => {
                let m = self.member(g);
                if m.card.get(hole).copied().unwrap_or(0) != 0 && self.holes[hole].flags & 1 == 0 {
                    format!("I had a {} on this hole last time.", m.card[hole])
                } else if self.holes[hole].flags & 2 != 0 {
                    "This hole is rated in the top 18 in the country.".into()
                } else if self.holes[hole].flags & 1 != 0 {
                    format!("You know, {partner}, hole {hole} is a top 100 hole.")
                } else {
                    tone = Tone::Good;
                    "I've always liked this hole.".into()
                }
            }
            0x3d => {
                let ask = g & 1 == 1;
                match (arg, ask) {
                    (3, true) => format!("How are you today, {partner}?"),
                    (3, false) => "Fine.".into(),
                    (4, true) => "Are those new clubs?".into(),
                    (4, false) => "They belong to my brother-in-law.".into(),
                    (5, true) => "Looks like a great day for golf.".into(),
                    (5, false) => "Sure does.".into(),
                    (6, true) => "Listen to those birds singing.".into(),
                    (6, false) => "It doesn't get any better than this.".into(),
                    (7, true) => "Prepare to be amazed.".into(),
                    (7, false) => "It's magic time.".into(),
                    (_, true) => "Shall I tee off first?".into(),
                    _ => "I don't care.".into(),
                }
            }
            0x3e => {
                let p = self.roster.get(r as usize).cloned().unwrap_or_default();
                signature_saying(p.head_index(r as usize), !male).replace("PARTNER", &partner)
            }
            0x3f => {
                let pro = self.g[g].vip() == 0x20;
                match (pro, g & 3) {
                    (true, 0) => "I brought my A game today.".into(),
                    (true, 1) => "Might as well just give me the trophy now.".into(),
                    (true, 2) => "I hope my caddie counted my clubs.".into(),
                    (true, _) => "You losers might as well go home now.".into(),
                    (false, 0) => "It's good to be back on the course.".into(),
                    (false, 1) => format!("Wasn't that a great tournament, {partner}?"),
                    (false, 2) => "I've already got tickets for the next one.".into(),
                    _ => "I think I could have won that tournament.".into(),
                }
            }
            0x41 => {
                if self.difficulty > 1 {
                    tone = Tone::Bad;
                }
                "I didn't see where that shot landed.".into()
            }
            _ => String::new(),
        };
        Line { text, tone, partner: by_partner }
    }
}

/// The course opinion at the end of a hole (0x469a20), by mood and the hole's score against par.
pub fn course_opinion(mood: i32, d: i32) -> &'static str {
    match mood {
        0 => "I hate this stupid course.",
        1 => "I don't like this course much.",
        2 => "I'm not having much fun today.",
        3 => "This course is almost adequate.",
        4 => "Well, I guess this course is OK.",
        5 => "This course is quite nice.",
        6 if d < 2 => "This is a really good course.",
        6 => "This is a really surprising course.",
        7 if d < 1 => "This course is excellent!",
        7 => "This course is fascinating.",
        8..=0x7f if d < 1 => "This is the best course I've ever played!",
        8..=0x7f => "yet this is a most interesting course.",
        _ => "I hate this course. I'm leaving.",
    }
}

/// The stock signature saying (event 0x3e, table 0x4d55ec) of a stock head of a gender; custom heads have none.
pub fn signature_saying(head: u8, female: bool) -> &'static str {
    crate::roster::head_defaults(head, female).map(|d| d.2).unwrap_or("")
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn lines() {
        let mut cl = Club::default();
        let c = Course::default();
        cl.g[2].roster = 4;
        cl.g[3].roster = 5;
        let l = cl.thought(&c, 0x0c, 13, 2);
        assert_eq!(l.text, "Darn that stupid tree!");
        assert_eq!(l.tone, Tone::Bad);
        let l = cl.thought(&c, 0x02, 7, 2);
        assert_eq!(l.text, "Darn, I'm in the sand trap!");
        cl.g[2].field_ae = 2;
        let l = cl.thought(&c, 0x02, 13, 2);
        assert!(l.partner && l.text.contains("under the tree"));
        assert_eq!(cl.thought(&c, 0x36, 0, 2).text, "I'll use my Driver for this shot.");
    }
}
