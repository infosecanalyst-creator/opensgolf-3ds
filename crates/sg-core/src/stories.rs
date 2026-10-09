//! Golfer stories (the exe's story system): each text file in the theme's folder is a story of four scenes for a pair of
//! golfers, chosen when the pair goes to the first tee if the file name's code fits them. About once a hole, when the two
//! walk close together, the story's owner speaks the next scene's opening line and the partner answers with a reply as good as
//! the partner's mood allows; the best reply passes the scene, and passing the fourth is a Happy Ending that unlocks a
//! landmark.
//!
//! Facts are from the publisher's golf.exe (docs/PUBLISHER_EXE_NOTES.md, "Golfer stories"), restated in our own words. Story
//! text is read from the player's own game files at run time.

use crate::course::{idx, inside, Course};
use crate::geom::{angle, dir8, len};
use crate::golfer::{anim, flag, Club};
use crate::land::ExeRng;

/// The pair was picked by the player on the clubhouse screen (flag 0x20000 during pairing only).
pub const MANUAL_PAIR: u32 = 0x20000;

/// The story table and texts of the theme.
#[derive(Clone, Debug, Default, serde::Serialize, serde::Deserialize)]
pub struct Stories {
    /// 100 slots read 50 bytes apart over a list of file names stored 100 bytes apart (an exe quirk kept as is): even slot
    /// 2i is file i, an odd slot is the tail of a long name (normally empty).
    pub slots: Vec<String>,
    /// File contents by name (read from the player's game files, so not saved).
    #[serde(skip)]
    pub texts: std::collections::HashMap<String, String>,
    /// The tutorial story id: the list index of the OpeningDay file, used as a slot number (the quirk again).
    pub tutorial: i32,
}

impl Stories {
    /// Builds the table from the files in listing order (case-insensitive alphabetical, as the original's file system).
    pub fn build(files: &[(String, String)], previous_tutorial: i32) -> Stories {
        let mut slots = vec![String::new(); 100];
        for (i, (name, _)) in files.iter().enumerate().take(50) {
            let b = name.as_bytes();
            slots[2 * i] = name.clone();
            if b.len() > 50 {
                slots[2 * i + 1] = String::from_utf8_lossy(&b[50..b.len().min(99)]).into_owned();
            }
        }
        let tutorial = files.iter().take(100).position(|(n, _)| n.contains("OpeningDay")).map(|i| i as i32).unwrap_or(previous_tutorial);
        Stories { slots, texts: files.iter().cloned().collect(), tutorial }
    }

    pub fn name(&self, id: i32) -> &str {
        self.slots.get(id as usize).map(|s| s.as_str()).unwrap_or("")
    }

    fn text(&self, id: i32) -> Option<&str> {
        self.texts.get(self.name(id)).map(|s| s.as_str())
    }

    /// The story's title (its first non-blank line, leading space kept).
    pub fn title(&self, id: i32) -> String {
        self.text(id).and_then(|t| lines(t).next()).unwrap_or_default()
    }

    /// Scene `scene`'s opening line, or its reply `choice` (the reply `scene - choice` lines down, best first, the last one
    /// when the scene has fewer).
    pub fn line(&self, id: i32, scene: i32, choice: i32, opener: bool) -> String {
        let Some(t) = self.text(id) else { return String::new() };
        let mut n = 0;
        let mut found_opener = None;
        let mut replies = Vec::new();
        for (k, l) in lines(t).enumerate() {
            if k > 200 {
                return String::new();
            }
            if l.starts_with(' ') {
                if found_opener.is_some() {
                    replies.push(l);
                }
            } else {
                if found_opener.is_some() {
                    break;
                }
                n += 1;
                if n == scene {
                    found_opener = Some(l);
                }
            }
        }
        if opener {
            return found_opener.unwrap_or_default();
        }
        let want = (scene - choice).max(0) as usize;
        replies.get(want).or(replies.last()).cloned().unwrap_or_default()
    }
}

fn lines(t: &str) -> impl Iterator<Item = String> + '_ {
    t.split('\n').map(|l| l.trim_end_matches('\r').to_string()).filter(|l| !l.is_empty())
}

/// The jingle a story plays on its first passed scene (odd: odd story id) and on a Happy Ending, by exe theme.
fn jingle(theme: u8, odd: bool, ending: bool) -> i32 {
    match theme {
        1 => {
            if odd && !ending {
                0x7a
            } else {
                0x78
            }
        }
        2 => {
            if odd && !ending {
                0x70
            } else {
                0x6e
            }
        }
        3 => {
            if odd && !ending {
                0x75
            } else {
                0x73
            }
        }
        _ => {
            if ending {
                0x33
            } else if odd {
                0x34
            } else {
                0x32
            }
        }
    }
}

/// The landmark a Happy Ending unlocks, by the first letter of the story's file name.
fn landmark_for(c0: u8, rng: &mut ExeRng) -> i32 {
    match c0 {
        b'C' => 0,
        b'P' => 1,
        b'A' => 2,
        b'M' => 3,
        b'L' => 4,
        b'H' => 5,
        b'G' => 7,
        b'F' | b'R' => 8,
        b'X' => 9,
        b'S' => 11,
        _ => rng.below(10),
    }
}

impl Club {
    fn trait_byte(&self, g: usize) -> u8 {
        self.roster.get(self.g[g].roster.max(0) as usize).map(|p| p.traits).unwrap_or(0)
    }

    /// Age (0x453260): from the roster index and the age band bits of the appearance byte.
    fn age(&self, g: usize) -> i32 {
        let b = self.g[g].looks & 0xff;
        self.g[g].roster.rem_euclid(10) + 20 * (b & 1 != 0) as i32 + 30 * (b & 2 != 0) as i32 + 45 * (b & 4 != 0) as i32
    }

    fn is_female(&self, g: usize) -> bool {
        self.male(g) == 0
    }

    /// Does the story file name's code (characters 1..7) fit owner g1 and partner g2?
    fn story_fits(&self, code: &[u8], g1: usize, g2: usize) -> bool {
        let up = |i: usize| code.get(i).map(|c| c.to_ascii_uppercase()).unwrap_or(b'X');
        let gender = |c: u8, a: usize, b: usize| match c {
            b'F' => self.is_female(a),
            b'M' => !self.is_female(a),
            b'O' => self.is_female(a) != self.is_female(b),
            b'S' => self.is_female(a) == self.is_female(b),
            _ => true,
        };
        let marital = |c: u8, a: usize| {
            let m = self.g[a].looks & 0xff;
            match c {
                b'D' => m & 0x20 != 0,
                b'M' => m & 0x10 != 0,
                b'N' => m & 0x10 == 0,
                b'S' => m & 0x08 != 0,
                b'W' => m & 0x40 != 0,
                _ => true,
            }
        };
        let traits = |c: u8, a: usize| {
            let t = self.trait_byte(a);
            match c {
                b'T' => t & 1 != 0,
                b'M' => t & 1 == 0,
                b'O' => t & 2 != 0,
                b'S' => t & 2 == 0,
                b'A' => t & 4 != 0,
                b'L' => t & 4 == 0,
                b'P' => t & 8 != 0,
                b'B' => t & 8 == 0,
                b'N' => t & 0x10 != 0,
                b'G' => t & 0x10 == 0,
                _ => true,
            }
        };
        let (a1, a2) = (self.age(g1), self.age(g2));
        let ages = match up(4) {
            b'O' => a1 >= a2 + 10,
            b'Y' => a2 - 10 >= a1,
            b'S' => (a1 - a2).abs() <= a1 / 3,
            _ => true,
        };
        gender(up(1), g1, g2)
            && marital(up(2), g1)
            && traits(up(3), g1)
            && ages
            && gender(up(5), g2, g1)
            && marital(up(6), g2)
            && traits(up(7), g2)
    }

    /// Pair compatibility: 5 less the trait bits the two people do not share.
    pub fn compatibility(&self, p1: usize, p2: usize) -> i32 {
        5 - ((self.trait_byte(p1) ^ self.trait_byte(p2)) & 0x1f).count_ones() as i32
    }

    /// The story part of pairing (0x45de80): up to three random tries for a fitting story not already played on the course,
    /// and the tutorial story for the opening pair.
    pub(crate) fn pick_story(&mut self, rng: &mut ExeRng, p1: usize, p2: usize) {
        for gg in [p1, p2] {
            self.g[gg].story = -1;
            self.g[gg].field_ae = 0;
            self.g[gg].flags &= !flag::STORY;
        }
        for _ in 0..3 {
            rng.below(100);
            let mut tries = 0;
            let mut s;
            loop {
                s = rng.below(100);
                tries += 1;
                if tries > 99 {
                    break;
                }
                if !self.stories.name(s).is_empty() && s != self.stories.tutorial {
                    break;
                }
            }
            if tries >= 100 {
                continue;
            }
            let code = self.stories.name(s).as_bytes().to_vec();
            if !self.story_fits(&code, p1, p2) {
                continue;
            }
            if self.g.iter().take(crate::golfer::SLOTS).any(|o| o.hole > 0 && o.story == s) {
                continue;
            }
            self.g[p1].story = s;
            self.g[p2].story = s;
            self.g[p1].flags |= flag::STORY;
            break;
        }
        if p1 < 2 && self.tick < 0x800 {
            self.g[p1].story = self.stories.tutorial;
            self.g[p2].story = self.stories.tutorial;
            self.g[p1].flags |= flag::STORY;
        }
    }

    /// The automatic story trigger in the golfer update (owner g): the pair on one hole, both standing, close enough for the
    /// scene, at most one beat a hole and 150 ticks after the last one on the course. (The exe also needs the owner on screen
    /// and the view zoomed in; the port lets stories run wherever the camera is.)
    pub(crate) fn story_trigger(&mut self, c: &mut Course, rng: &mut ExeRng, g: usize) {
        let gg = &self.g[g];
        if gg.flags & flag::STORY == 0 || gg.flags & flag::STORY_BEAT != 0 {
            return;
        }
        let p = gg.partner.clamp(0, crate::golfer::SLOTS as i32 - 1) as usize;
        let pp = &self.g[p];
        if gg.hole != pp.hole || (gg.hole == 19 && gg.story_step == 0) {
            return;
        }
        if (self.tick.wrapping_sub(31 * g as u32)) & 0x3f != 0 {
            return;
        }
        let standing = |a: i32| a > 6 && a != anim::ADDRESS;
        if !standing(gg.anim) || !standing(pp.anim) || self.tick.wrapping_sub(self.last_beat) <= 150 {
            return;
        }
        if len(gg.x - pp.x, gg.y - pp.y) >= 0x3000 / (gg.story_step + 6) {
            return;
        }
        if c.landmark_bits(gg.x, gg.y) & 8 == 8 {
            let h = self.g[p].hole.clamp(0, 18) as usize;
            self.g[p].hole_mood[h] = 99;
        }
        self.story_beat(c, rng, g, false);
        self.g[g].flags |= flag::STORY_BEAT;
        self.g[p].flags |= flag::STORY_BEAT;
    }

    /// The partner's reply quality for the scene (0x4669f0): worked out from mood on the first beat of a hole, nudged by the
    /// newest thought and a story landmark, and read back afterwards.
    fn story_choice(&mut self, c: &Course, step: i32, hole: usize, owner: usize) -> i32 {
        let p = self.g[owner].partner.clamp(0, crate::golfer::SLOTS as i32 - 1) as usize;
        let stored = self.g[owner ^ 1].hole_mood[hole] as i32;
        let mut ch = (stored >> 3).clamp(0, step);
        if self.g[p].hole as usize == hole && self.g[p].hole_mood[hole] == 0 {
            let pp = &self.g[p];
            let i = pp.thoughts[..4].iter().position(|&t| t != 0x13).unwrap_or(4);
            let x = crate::geom::clamp(pp.mood, pp.smooth_mood, 99);
            ch = (x - 1) / 2;
            if step >= 2 && ch > step - 1 {
                ch = step - 1;
            }
            let f = pp.args.get(i).copied().unwrap_or(0) & 0xc000;
            if f == 0x4000 {
                ch += 1;
            } else if ch != 0 {
                if f != 0 {
                    ch -= 1;
                }
                if ch != 0 && f == 0xc000 {
                    ch -= 1;
                }
            }
            if c.landmark_bits(pp.x, pp.y) & 8 == 8 {
                ch = step;
            }
            ch = ch.clamp(0, step);
        }
        self.g[p].story_step = ch;
        self.g[p].hole_mood[hole] = (ch * 8 + 1) as u8;
        ch
    }

    /// One story beat (0x466370) for owner g: the next scene, the partner's reply, sounds and moods, the regression of a failed
    /// scene and the Happy Ending of the fourth. Returns whether the scene passed.
    pub fn story_beat(&mut self, c: &mut Course, rng: &mut ExeRng, g: usize, force: bool) -> bool {
        let story = self.g[g].story;
        if story == -1 {
            return false;
        }
        if !force && self.g[g].hole <= 1 && self.g[g].strokes == 0 {
            return false;
        }
        let p = self.g[g].partner.clamp(0, crate::golfer::SLOTS as i32 - 1) as usize;
        let mut step = self.g[g].story_step;
        if step > 3 && self.g[p].story_step > 3 {
            return false;
        }
        if !force && self.g[g].smooth_mood <= step * 4 + 4 {
            return false;
        }
        step += 1;
        let hole = self.g[g].hole.clamp(0, 18) as usize;
        self.g[g].story_step = step;
        self.g[g].story_hole[hole] = step as u8;
        self.g[g].flags |= flag::STORY_STARTED;
        self.g[p].flags |= flag::STORY_STARTED;
        self.last_beat = self.tick;
        let ch = self.story_choice(c, step, hole, g);
        let has_text = !self.stories.line(story, step, ch, true).is_empty();
        if has_text {
            self.event(c, rng, g, 0x32, ch * 16 + step);
            let male = self.male(g);
            self.sound(0x58 + male, Some((self.g[g].x, self.g[g].y)));
        }
        if !self.stories.line(story, step, ch, false).is_empty() {
            self.event(c, rng, p, 0x32, ch * 16 + step);
            self.g[p].timer = self.g[p].timer.wrapping_add(3);
            let pm = self.male(p);
            if self.g[p].story_step < step {
                if ch == step - 1 {
                    self.sound(0xa4 + pm, Some((self.g[p].x, self.g[p].y)));
                } else {
                    self.sound(0x5a + pm, Some((self.g[p].x, self.g[p].y)));
                    self.g[g].mood -= 1;
                }
            } else {
                self.sound(0xa6 + pm, Some((self.g[g].x, self.g[g].y)));
                if step == 1 {
                    let j = jingle(c.theme, story & 1 != 0, false);
                    self.sound(j, None);
                    self.g[g].timer = self.g[g].timer.wrapping_add(1);
                    self.g[p].timer = self.g[p].timer.wrapping_add(1);
                }
                self.g[g].mood += 1;
            }
        }
        let passed = if self.g[p].story_step < self.g[g].story_step {
            self.g[g].story_step -= 1;
            let s = self.g[g].smooth_mood;
            self.g[g].smooth_mood = s - s / 3;
            false
        } else {
            if self.g[g].story_step == 4 && self.g[p].story_step == 4 {
                self.happy_ending(c, rng, g, p);
            }
            true
        };
        self.g[p].story_hole[hole] = self.g[p].story_step as u8;
        let freeze = (-4 - self.g[g].story_step) * 4;
        self.g[g].pause = freeze;
        self.g[p].pause = freeze;
        let face = dir8(angle(self.g[p].x - self.g[g].x, self.g[p].y - self.g[g].y));
        self.g[g].facing = face;
        self.g[p].facing = face ^ 4;
        for gg in [g, p] {
            self.g[gg].anim = anim::STAND;
            self.g[gg].walk_left = 0;
        }
        passed
    }

    /// The Happy Ending (both at scene 4): jingle, the count, the hole marked in both members' records and a landmark unlocked
    /// for free, provided a random walk from the owner's tile reaches rough before leaving the course.
    fn happy_ending(&mut self, c: &Course, rng: &mut ExeRng, owner: usize, partner: usize) {
        let story = self.g[owner].story;
        self.sound(jingle(c.theme, story & 1 != 0, true), None);
        self.happy_endings += 1;
        let hole = self.g[owner].hole.clamp(0, 18) as usize;
        let title = self.stories.title(story);
        self.message(format!("Happy Ending: {}", title.trim_start()));
        for gg in [owner, partner] {
            self.member_mut(gg).holes[hole] |= 2;
        }
        let (mut a, mut b) = (self.g[owner].x >> 10, self.g[owner].y >> 10);
        loop {
            if inside(a, b) {
                let t = c.ty[idx(a, b)];
                if c.row(t).class == 4 && t != crate::course::t::BUILDING && c.flags[idx(a, b)] & 0x320 == 0 {
                    break;
                }
            }
            a += rng.below(3) - 1;
            b += rng.below(3) - 1;
            if !inside(a, b) || c.ty[idx(a, b)] == crate::course::t::OUT {
                return;
            }
        }
        let c0 = self.stories.name(story).as_bytes().first().copied().unwrap_or(b'x');
        let t = landmark_for(c0, rng);
        self.free_landmarks |= 1 << t;
        self.landmarks_owned |= 1 << t;
        self.message(format!("A {} is now available in your landmarks, free of charge.", crate::vips::landmark_name(t)));
    }

    /// The story line a golfer is saying now (thought 0x32): the owner's opening line or the partner's reply.
    pub fn story_line(&self, g: usize) -> Option<String> {
        let gg = &self.g[g];
        if gg.thought != 0x32 || gg.story == -1 {
            return None;
        }
        let (scene, choice) = (gg.thought_arg & 15, gg.thought_arg >> 4);
        let owner = gg.flags & flag::STORY != 0;
        let line = self.stories.line(gg.story, scene, choice, owner);
        if line.is_empty() {
            return None;
        }
        let other = self.name(g ^ 1);
        Some(line.trim_start().replacen("PARTNER", &other, 1).replacen("MYNAME", &self.name(g), 1))
    }

    /// Mood-check at the first scene's explanation: whether a beat needs the owner's mood (used by the info card).
    pub fn story_scene(&self, g: usize) -> i32 {
        self.g[g].story_step
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    const TEXT: &str = " The Test\n\nHello there\n Best reply\n Fine reply\n\nScene two\n Good two\n Ok two\n Bad two\n";

    #[test]
    fn table_and_lines() {
        let files = vec![("AxxxxxxxAlpha.txt".to_string(), TEXT.to_string()), ("GxxxxxxxOpeningDay.txt".to_string(), TEXT.to_string())];
        let s = Stories::build(&files, 0);
        assert_eq!(s.name(0), "AxxxxxxxAlpha.txt");
        assert_eq!(s.name(1), "");
        assert_eq!(s.name(2), "GxxxxxxxOpeningDay.txt");
        // the tutorial is the list index 1 used as a slot: an empty one
        assert_eq!(s.tutorial, 1);
        assert_eq!(s.title(0), " The Test");
        assert_eq!(s.line(0, 1, 1, true), "Hello there");
        assert_eq!(s.line(0, 1, 1, false), " Best reply");
        assert_eq!(s.line(0, 1, 0, false), " Fine reply");
        assert_eq!(s.line(0, 2, 2, false), " Good two");
        assert_eq!(s.line(0, 2, 0, false), " Bad two");
        // past the last reply: the last one
        assert_eq!(s.line(0, 1, -5, false), " Fine reply");
    }
}
