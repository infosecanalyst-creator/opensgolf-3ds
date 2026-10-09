//! Customise Golfer (0x4385d0): the player's own character editor, reached from the golfer card's Customize button for the
//! player's pro. It edits person record 0 in place: name and profession, marital status and age group, the five traits,
//! the three skill class toggles, the face (picker over HeadSelect), body type, adult or child, shirt, trousers, hair and
//! skin colours and gender, with Undo and OK; the pro's skills are shown read only. Layout and rules from
//! docs/DECODE_CUSTOMISE.md; the art is the disc's.
//!
//! PLACEHOLDERS (not decoded, see the decode's section 7): the hit centres and hover positions are measured on the art; the
//! round buttons' roles are read from the art (the women's background has the hair colour button, the men's does not);
//! the trait words (screens_ui::TRAIT_WORDS); the face picker's ten ball positions; the per-head default skin and hair;
//! the 20-row dialogue table (only its first label is known and the port keeps no sayings, so it stays empty); the two
//! small walking figures under the preview; Load (here it takes the championship folder's pro files in turn instead of
//! the exe's "Pick one:" list) and Save (the championship folder, without the portrait).

use crate::app::*;
use crate::gfx::Gfx;
use crate::screens_ui::{c15, dist, skill_value, tooltip, top, CLASS_WORDS, SMALL, TRAIT_WORDS};
use crate::ui::{rgb, Screen as Ui};
use sg_core::pro::SKILL_NAMES;
use sg_core::roster::Person;

/// The editor's state: the golfer slot it was opened for, the record as it was (Undo), the class toggles being edited,
/// the hit under the pointer and for how long, the face picker's page while it is open, and a name or profession being
/// typed.
#[derive(Clone, Debug, Default)]
pub struct Customise {
    pub slot: Option<usize>,
    pub snapshot: Person,
    pub toggles: u8,
    pub hover: i32,
    pub frames: u32,
    pub picker: Option<usize>,
    pub typing: Option<(usize, String)>,
}

/// Hit centres by result index (DECODE_CUSTOMISE 1.2): 0..2 the class toggles, 3..7 the traits, 8 Load, 9 Save, 10 body
/// type, 11 adult or child, 12 face, 13 shirt, 14 trousers, 15 hair, 16 skin, 17 gender, 18 Undo, 19 OK.
const CENTRES: [(f32, f32); 20] = [
    (213.0, 153.0),
    (213.0, 188.0),
    (213.0, 223.0),
    (95.0, 130.0),
    (95.0, 158.0),
    (95.0, 186.0),
    (95.0, 214.0),
    (95.0, 242.0),
    (49.0, 22.0),
    (259.0, 22.0),
    (328.0, 106.0),
    (454.0, 80.0),
    (454.0, 130.0),
    (454.0, 180.0),
    (454.0, 230.0),
    (328.0, 56.0),
    (454.0, 30.0),
    (328.0, 156.0),
    (762.0, 37.0),
    (762.0, 218.0),
];

const TIPS: [&str; 20] = [
    "",
    "",
    "",
    "",
    "",
    "",
    "",
    "",
    "Load character",
    "Save character",
    "Body type",
    "Adult/child",
    "Select face",
    "Shirt Color",
    "Pants Color",
    "Hair Color",
    "Skin Tone",
    "Gender",
    "Cancel",
    "OK",
];

const MARITAL: [(u8, &str); 4] = [(0x08, "Single"), (0x10, "Married"), (0x20, "Divorced"), (0x40, "Widowed")];
const AGES: [(u8, &str); 3] = [(0x01, "Young"), (0x02, "Middle Aged"), (0x04, "Mature")];

/// The face picker's ten ball slots (two rows of five on the HeadSelect panel; the exe's table is not decoded).
fn ball_slot(i: usize) -> (f32, f32) {
    if i < 5 {
        (50.0 + 130.0 * i as f32, 288.0)
    } else {
        (100.0 + 130.0 * (i - 5) as f32, 425.0)
    }
}

/// The body index (0..3 men, 5..8 women) a person record draws with (the customise preview's rule, DECODE_CUSTOMISE 3.3).
pub fn person_look(p: &Person) -> usize {
    let nibble = ((p.b23 >> 4) & 3) as usize;
    if p.female() {
        let mut l = 7;
        if p.b21 & 1 != 0 {
            l = 6;
        }
        if p.traits & 8 != 0 {
            l = 8;
        }
        if p.b21 & 4 != 0 {
            l = 5;
        }
        if p.fixed != 0 {
            l = nibble + 5;
        }
        l
    } else {
        let class = p.kind(0);
        let mut l = if class & 4 == 0 { 1 } else { (((!class & 2) | 4) >> 1) as usize };
        if p.fixed != 0 {
            l = nibble;
        }
        l
    }
}

/// The next of a set of one-bit choices kept in `bits` (later bits win), forward or back; none set starts at an end.
fn cycle(bits: u8, set: &[u8], forward: bool) -> u8 {
    let cur = set.iter().rposition(|&b| bits & b != 0);
    let n = set.len();
    let next = match cur {
        None if forward => 0,
        None => n - 1,
        Some(i) if forward => (i + 1) % n,
        Some(i) => (i + n - 1) % n,
    };
    let all = set.iter().fold(0, |a, &b| a | b);
    (bits & !all) | set[next]
}

fn step(v: u8, n: u8, up: bool) -> u8 {
    if up {
        (v + 1) % n
    } else {
        (v + n - 1) % n
    }
}

impl App {
    /// Opens the editor on the player's own record for golfer slot `slot`.
    pub fn open_customise(&mut self, slot: usize) {
        let Some(p) = self.club.roster.first_mut() else { return };
        let toggles = if p.fixed == 0 { 7 } else { (p.fixed & 7) as u8 };
        if p.fixed == 0 {
            // the colours the composer would pick become the record's own, so that editing starts from the look shown
            let o = p.outfit(0);
            p.shirt = o.shirt;
            p.pants = o.pants;
            p.skin = o.skin;
            p.hair = o.hair;
            p.alt_skin = o.alt_skin;
            p.b23 = (p.b23 & 0xf0) | (o.hat & 0xf);
        }
        let snapshot = p.clone();
        self.cust = Some(Customise { slot: Some(slot), snapshot, toggles, hover: -1, ..Default::default() });
        self.screen = Screen::Customise;
        self.ui_sound(0x2d);
    }

    fn cust_hit(&self, vx: f32, vy: f32) -> i32 {
        let female = self.club.roster.first().is_some_and(|p| p.female());
        let mut best = (40.0, -1);
        for (i, &(cx, cy)) in CENTRES.iter().enumerate() {
            let dx = if i < 8 { (vx - cx) / 3.0 } else { vx - cx };
            let d = dist(dx, vy - cy);
            if d < best.0 {
                best = (d, i as i32);
            }
        }
        // men have no hair colour choice
        if best.1 == 15 && !female {
            return -1;
        }
        best.1
    }

    /// Where the pointer is over the name box: its row 0..3.
    fn cust_name_row(vx: f32, vy: f32) -> Option<usize> {
        ((vx - 154.0).abs() < 80.0 && (vy - 64.0).abs() < 40.0 && vy >= 24.0).then(|| (((vy - 24.0) / 20.0) as usize).min(3))
    }

    /// A click in the editor; `right` steps the other way.
    pub fn cust_click(&mut self, vx: f32, vy: f32, right: bool) {
        let Some(mut c) = self.cust.take() else {
            self.screen = Screen::Play;
            return;
        };
        if c.typing.is_some() {
            self.cust = Some(c);
            return;
        }
        if let Some(page) = c.picker {
            self.cust_picker_click(&mut c, page, vx, vy, right);
            self.cust = Some(c);
            return;
        }
        let up = !right;
        let hit = self.cust_hit(vx, vy);
        match hit {
            8 => {
                self.cust = Some(c);
                return self.cust_load();
            }
            9 => {
                self.cust = Some(c);
                return self.save_championship_pro();
            }
            19 => return self.cust_ok(&c),
            _ => {}
        }
        let p = &mut self.club.roster[0];
        match hit {
            0..=2 => c.toggles ^= 1 << hit,
            3..=7 => p.traits ^= 1 << (hit - 3),
            10 => {
                let b = step((p.b23 >> 4) & 3, 4, up);
                p.b23 = (p.b23 & !0x30) | (b << 4);
            }
            11 => p.fixed ^= 8,
            12 => c.picker = Some(0),
            13 => p.shirt = step(p.shirt % 10, 10, up),
            14 => p.pants = step(p.pants % 10, 10, up),
            15 => p.hair = step(p.hair % 5, 5, up),
            16 => p.skin = step(p.skin % 4, 4, up),
            17 => p.b21 ^= 0x80,
            18 => *p = c.snapshot.clone(),
            _ => match Self::cust_name_row(vx, vy) {
                Some(r @ 0..=1) => c.typing = Some((r, String::new())),
                Some(2) => p.b21 = cycle(p.b21, &MARITAL.map(|m| m.0), up),
                Some(_) => p.b21 = cycle(p.b21, &AGES.map(|m| m.0), up),
                None => {}
            },
        }
        if matches!(hit, 10 | 11 | 13..=17) {
            p.fixed |= 0x80;
        }
        self.cust = Some(c);
    }

    fn cust_picker_click(&mut self, c: &mut Customise, page: usize, vx: f32, vy: f32, right: bool) {
        let gender = self.club.roster[0].male_bit() as usize;
        let count = self.art.head_count(gender);
        if right || vy < 258.0 {
            c.picker = None;
            return;
        }
        if dist(vx - 778.0, vy - 363.0) < 60.0 {
            let p = if page < 70 { page + 10 } else { page };
            c.picker = Some(p.min(count.saturating_sub(10)));
            return;
        }
        if dist(vx - 18.0, vy - 495.0) < 60.0 {
            c.picker = Some(page.saturating_sub(10));
            return;
        }
        for i in 0..10 {
            let (x, y) = ball_slot(i);
            if dist(vx - x - 60.0, vy - y - 60.0) < 60.0 && page + i < count {
                // the per-head default skin and hair of the stock heads are not decoded: the colours stay
                self.club.roster[0].head = Some((page + i) as u8);
                c.picker = None;
                return;
            }
        }
    }

    /// Load: the championship folder's pro files in turn (the exe lists the theme folder's files to pick from).
    fn cust_load(&mut self) {
        let dir = self.championship_dir();
        let mut files: Vec<_> = sg_core::fsutil::list_dir(&dir)
            .into_iter()
            .filter(|f| f.extension().and_then(|e| e.to_str()).is_some_and(|e| e.eq_ignore_ascii_case("pro")))
            .collect();
        files.sort();
        if files.is_empty() {
            return;
        }
        let cur = self.club.roster[0].name.to_ascii_lowercase();
        let at = files.iter().position(|f| f.file_stem().and_then(|s| s.to_str()).is_some_and(|s| s.to_ascii_lowercase() == cur));
        let f = &files[at.map(|i| (i + 1) % files.len()).unwrap_or(0)];
        if let Some(pro) = sg_core::fsutil::read_file(f).and_then(|b| sg_core::championship::parse_pro(&b)) {
            self.show_toast(&format!("Loaded {}", pro.person.name));
            self.club.roster[0] = pro.person;
        }
    }

    /// OK: the class toggles go into the record and the golfer, the golfer takes the record's marital and age bits.
    fn cust_ok(&mut self, c: &Customise) {
        let p = &mut self.club.roster[0];
        p.fixed = (p.fixed & !7) | c.toggles as i32;
        let b21 = p.b21;
        if let Some(g) = c.slot.filter(|&g| g < sg_core::golfer::SLOTS) {
            let gg = &mut self.club.g[g];
            gg.class = (gg.class & 0xf0) | c.toggles;
            gg.looks = (gg.looks & 0xff00) | b21 as u16;
        }
        self.cust = None;
        self.screen = Screen::Play;
        self.ui_sound(0x2d);
    }

    /// Typing a name or profession (the exe's one-line box, at most 15 characters).
    pub fn cust_char(&mut self, ch: char) {
        if let Some((_, t)) = self.cust.as_mut().and_then(|c| c.typing.as_mut()) {
            if !ch.is_control() && t.chars().count() < 15 {
                t.push(ch);
            }
        }
    }

    /// Keys: Enter keeps a non-empty name or profession, Esc empties it, Backspace deletes; Esc also closes the face picker.
    pub fn cust_key(&mut self, k: miniquad::KeyCode) {
        use miniquad::KeyCode;
        let Some(c) = self.cust.as_mut() else { return };
        if let Some((row, t)) = c.typing.as_mut() {
            match k {
                KeyCode::Backspace => {
                    t.pop();
                }
                KeyCode::Escape => t.clear(),
                KeyCode::Enter | KeyCode::KpEnter => {
                    let (row, t) = (*row, t.trim().to_string());
                    c.typing = None;
                    if !t.is_empty() {
                        let p = &mut self.club.roster[0];
                        if row == 0 {
                            p.name = t;
                        } else {
                            p.job = t;
                        }
                    }
                }
                _ => {}
            }
        } else if k == KeyCode::Escape {
            c.picker = None;
        }
    }

    pub fn draw_customise(&mut self, g: &mut Gfx) {
        let Some(mut c) = self.cust.clone() else { return };
        let s = Ui::new(self.draw_w, self.draw_h);
        self.view = s.view;
        let (mx, my) = self.card_ui.mouse;
        let p = self.club.roster.first().cloned().unwrap_or_default();
        let gi = p.male_bit() as usize;
        s.image(g, &self.art.cust_bg[gi], 0.0, 0.0);
        // the preview: sky and grass, the body, the head
        let custom = p.head_index(0) >= 19;
        s.image(g, &self.art.head_body, if custom { 316.0 } else { 336.0 }, 20.0);
        let look = person_look(&p);
        let mut o = p.outfit(0);
        o.alt_skin = 4;
        let swaps = std::mem::take(&mut self.swaps);
        self.art.head_over_body(g, &s, &swaps, &p, o, 0, look, 1, 336.0, 40.0);
        self.swaps = swaps;
        // the round buttons that overlap the window, back on top of it
        for k in 0..8 {
            let (x, y) = if k < 3 { (309.0, 37.0 + 50.0 * k as f32) } else { (435.0, 11.0 + 50.0 * (k - 3) as f32) };
            s.image_part(g, &self.art.cust_bg[gi], x, y, x, y, 39.0, 39.0);
        }
        let hit = if c.picker.is_some() || c.typing.is_some() { -1 } else { self.cust_hit(mx, my) };
        if hit != c.hover {
            c.hover = hit;
            c.frames = 0;
        } else {
            c.frames += 1;
        }
        let cg = &self.art.cg;
        // the class toggles and the traits that are on, then the hovered button, in yellow
        let lit = |g: &mut Gfx, i: usize| match i {
            0..=2 => s.image_part(g, cg, 157.0, 136.0 + 35.0 * i as f32, 150.0, 300.0 + 50.0 * i as f32, 112.0, 35.0),
            3..=7 => {
                let k = (i - 3) as f32;
                s.image_part(g, cg, 39.0, 116.0 + 28.0 * k, 150.0, 50.0 + 50.0 * k, 112.0, 28.0)
            }
            8 => s.image_part(g, cg, 32.0, 4.0, 0.0, 0.0, 35.0, 37.0),
            9 => s.image_part(g, cg, 242.0, 4.0, 100.0, 0.0, 35.0, 37.0),
            18 => s.image_part(g, cg, 740.0, 15.0, 500.0, 0.0, 60.0, 61.0),
            19 => s.image_part(g, cg, 735.0, 191.0, 500.0, 100.0, 60.0, 61.0),
            _ => {
                // the round buttons: the tabs right of the window are rows 0..4 of the sheet, the left ones rows 5..7
                let (row, x, y) = match i {
                    16 => (0, 436.0, 12.0),
                    11 => (1, 436.0, 62.0),
                    12 => (2, 436.0, 112.0),
                    13 => (3, 436.0, 162.0),
                    14 => (4, 436.0, 212.0),
                    15 => (5, 310.0, 38.0),
                    10 => (6, 310.0, 88.0),
                    _ => (7, 310.0, 138.0),
                };
                s.image_part(g, cg, x, y, 350.0, 50.0 * row as f32, 50.0, 50.0);
            }
        };
        for i in 0..3 {
            if c.toggles & (1 << i) != 0 {
                lit(g, i);
            }
        }
        for i in 0..5 {
            if p.traits & (1 << i) != 0 {
                lit(g, 3 + i);
            }
        }
        if hit >= 0 {
            lit(g, hit as usize);
        }
        // the name box
        let navy = c15(0x0848);
        let row = if hit < 0 && c.picker.is_none() { Self::cust_name_row(mx, my) } else { None };
        let marital = MARITAL.iter().rev().find(|m| p.b21 & m.0 != 0).map(|m| m.1).unwrap_or("");
        let age = AGES.iter().rev().find(|m| p.b21 & m.0 != 0).map(|m| m.1).unwrap_or("");
        for (r, (y, t)) in [(24.0, p.name.as_str()), (48.0, p.job.as_str()), (69.0, marital), (90.0, age)].iter().enumerate() {
            let col = match row {
                Some(0) if r == 0 => c15(0x7b20),
                Some(k) if k == r => c15(0x7fff),
                _ => navy,
            };
            s.text_centered(g, 154.0, top(*y - 4.0, 15.0), t, 15.0, col);
        }
        // the trait and class labels
        for (i, w) in TRAIT_WORDS.iter().enumerate() {
            let col = if p.traits & (1 << i) != 0 { navy } else { c15(0x4210) };
            s.text_centered(g, 93.0, top(122.0 + 28.0 * i as f32, 13.0), w, 13.0, col);
        }
        for (i, w) in CLASS_WORDS.iter().enumerate() {
            let col = if c.toggles & (1 << i) != 0 { navy } else { c15(0x4210) };
            let w = format!("{}{}", w[..1].to_ascii_uppercase(), &w[1..]);
            s.text_centered(g, 212.0, top(142.0 + 35.0 * i as f32, 13.0), &w, 13.0, col);
        }
        // the pro's skills, read only
        s.text_centered(g, 585.0, top(30.0 - 4.0, 15.0), "Golf Skill Levels", 15.0, rgb(0.0, 0.0, 0.0));
        for i in 0..10 {
            let y = 61.0 + 16.0 * i as f32;
            let v = self.club.pro_skill[i];
            s.fill(g, 498.0, y - 3.0, 42.0, 14.0, rgb(0.0, 0.0, 0.0));
            s.fill(g, 499.0, y - 2.0, 40.0, 12.0, c15(if v != 0 { 0x7d08 } else { 0x21e8 }));
            if v != 0 {
                s.text(g, 501.0, top(y - 1.0, SMALL), &skill_value(v), SMALL, rgb(0.0, 0.0, 0.0));
            }
            s.text(g, 546.0, top(y - 1.0, SMALL), SKILL_NAMES[i], SMALL, if v != 0 { rgb(0.0, 0.0, 0.0) } else { c15(0x4210) });
        }
        // the dialogue table: only its first row's label is known
        s.text(g, 52.0, top(271.0, SMALL), "Signature saying", SMALL, rgb(1.0, 1.0, 1.0));
        if let Some(page) = c.picker {
            self.draw_face_picker(g, &s, &p, page, mx, my);
        }
        if let Some((r, t)) = &c.typing {
            // the one-line box at (200, 32): 15 characters
            let (x, y, w) = (200.0, 32.0, 15.0 * 12.0 + 32.0);
            s.fill(g, x - 1.0, y - 1.0, w + 2.0, 50.0, rgb(1.0, 1.0, 1.0));
            s.fill(g, x + 1.0, y + 1.0, w, 48.0, rgb(0.0, 0.0, 0.0));
            s.fill(g, x, y, w, 48.0, c15(0x35b3));
            s.text(g, x + 4.0, top(y + 4.0, 13.0), if *r == 0 { "New name: " } else { "Profession: " }, 13.0, rgb(1.0, 1.0, 1.0));
            s.fill(g, x + 16.0, y + 23.0, w - 32.0, 22.0, rgb(1.0, 1.0, 1.0));
            let caret = if (self.clock * 2.0) as i64 % 2 == 0 { "|" } else { "" };
            s.text(g, x + 20.0, top(y + 26.0, 14.0), &format!("{t}{caret}"), 14.0, rgb(0.0, 0.0, 0.0));
        }
        if hit >= 0 && c.frames > 10 {
            tooltip(g, &s, mx, my, TIPS[hit as usize]);
        }
        if let Some(cc) = self.cust.as_mut() {
            cc.hover = c.hover;
            cc.frames = c.frames;
        }
        g.flush();
    }

    /// The face picker over the lower half: ten balls a page with the heads of the record's gender, the page arrows.
    fn draw_face_picker(&self, g: &mut Gfx, s: &Ui, p: &Person, page: usize, mx: f32, my: f32) {
        let hs = &self.art.head_select;
        s.image_part(g, hs, 0.0, 258.0, 0.0, 258.0, 800.0, 342.0);
        let count = self.art.head_count(p.male_bit() as usize);
        for i in 0..10 {
            let (x, y) = ball_slot(i);
            let over = dist(mx - x - 60.0, my - y - 60.0) < 60.0 && page + i < count;
            s.image_part(g, hs, x, y, if over { 150.0 } else { 0.0 }, 0.0, 133.0, 137.0);
            if page + i < count {
                self.art.halo_cell(g, s, p.female(), (page + i) as u8, 0, x - 8.0, y);
            }
        }
        if dist(mx - 778.0, my - 363.0) < 60.0 {
            s.image_part(g, hs, 707.0, 284.0, 614.0, 0.0, 93.0, 152.0);
        }
        if dist(mx - 18.0, my - 495.0) < 60.0 {
            s.image_part(g, hs, 10.0, 425.0, 432.0, 105.0, 87.0, 152.0);
        } else if page == 0 {
            s.image_part(g, hs, 10.0, 425.0, 519.0, 105.0, 87.0, 152.0);
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn marital_and_age_cycles() {
        let m = MARITAL.map(|m| m.0);
        // none set: Single forward, Widowed back
        assert_eq!(cycle(0x80, &m, true), 0x88);
        assert_eq!(cycle(0x80, &m, false), 0xc0);
        assert_eq!(cycle(0x08, &m, true), 0x10);
        assert_eq!(cycle(0x40, &m, true), 0x08);
        assert_eq!(cycle(0x08 | 0x02, &m, false), 0x40 | 0x02);
        let a = AGES.map(|m| m.0);
        assert_eq!(cycle(0x04 | 0x10, &a, true), 0x01 | 0x10);
        assert_eq!(step(0, 10, false), 9);
        assert_eq!(step(4, 5, true), 0);
    }

    #[test]
    fn preview_body() {
        let mut p = Person { b21: 0x80, ..Default::default() };
        assert_eq!(person_look(&p), 7);
        p.b21 |= 1;
        assert_eq!(person_look(&p), 6);
        p.fixed = 0x80;
        p.b23 = 0x30;
        assert_eq!(person_look(&p), 8);
        let mut m = Person { traits: 0x01, ..Default::default() };
        assert_eq!(person_look(&m), 1);
        m.traits = 0x04;
        assert_eq!(person_look(&m), 3);
        m.fixed = 0x87;
        m.b23 = 0x20;
        assert_eq!(person_look(&m), 2);
    }
}
