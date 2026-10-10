//! Customise Golfer (0x4385d0): the character editor on one person record. The golfer card's Customize opens it on the
//! card's golfer (any golfer, the click handler at 0x41ea4f), the Player panel on the player's own record 0. A golfer whose
//! slot type is a pro's (type & 0xe0 == 0x20) gets the skills panel (mode 1, its skills editable unless it is record 0),
//! anyone else the membership panel with the biography (mode 0). It edits name and profession, marital status and age
//! group, the five traits, the three skill class toggles, the face (picker over HeadSelect), body type, adult or child,
//! shirt, trousers, hair and skin colours and gender, the twenty dialogue lines; Load picks a character file of the theme
//! pack from a "Pick one..." list, Save writes one (with the head's three faces) after asking before overwriting; Undo puts
//! the record back and Exit leaves. Layout and rules from docs/DECODE_CUSTOMISE.md and the exe's tables (hit centres
//! 0x4c7b38, hover and lit positions 0x4c7b90, face picker slots 0x4c7be0, dialogue rows 0x4c2cc0 / 0x4c2d10, per-head
//! defaults 0x4d55e8); the art is the disc's.
//!
//! PLACEHOLDERS (not decoded): which CGButtons piece each hover position shows (the yellow twin of the piece under it is
//! drawn), the sprite drawn at (352, 116) for a child, the multi-line biography editor (a one-box editor here) and the last argument of
//! each event the dialogue table's stock lines are built with (0 here; the exe keeps one per event at 0x838da8).

use crate::app::*;
use crate::gfx::Gfx;
use crate::screens_ui::{c15, dist, tooltip, top, CLASS_WORDS, SMALL, TRAIT_WORDS};
use crate::ui::{rgb, wrap_text, Image, Screen as Ui};
use sg_core::pro::SKILL_NAMES;
use sg_core::roster::{Person, SAYINGS, SAYING_CODES, SAYING_LABELS};
use std::path::PathBuf;

/// What is being typed: the name, the profession, a dialogue line or the biography.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Field {
    Name,
    Job,
    Saying(usize),
    Bio,
}

/// A box over the editor: the Load list (files, option under the pointer), the overwrite question for a file, a notice.
#[derive(Clone, Debug)]
pub enum Dialog {
    Load(Vec<PathBuf>, usize),
    Overwrite(PathBuf, usize),
    Notice(Vec<String>),
}

/// The editor's state: the person record and golfer slot it edits, its mode, the record as it was (Undo), the class
/// toggles being edited, the hit under the pointer and for how long, the face picker's page while it is open, a field
/// being typed and a box shown over the screen.
#[derive(Clone, Debug, Default)]
pub struct Customise {
    pub record: usize,
    pub slot: Option<usize>,
    pub pro_mode: bool,
    pub snapshot: Person,
    pub toggles: u8,
    pub hover: i32,
    pub frames: u32,
    pub picker: Option<usize>,
    pub typing: Option<(Field, String)>,
    pub dialog: Option<Dialog>,
}

/// Hit centres by result index (table 0x4c7b38): 0..2 the class toggles, 3..7 the traits, 8 Load, 9 Save, 10 body type,
/// 11 adult or child, 12 face, 13 shirt, 14 trousers, 15 hair, 16 skin, 17 gender, 18 Undo, 19 Exit, 20 Update Bio.
const CENTRES: [(f32, f32); 21] = [
    (210.0, 150.0),
    (210.0, 185.0),
    (210.0, 220.0),
    (93.0, 127.0),
    (93.0, 156.0),
    (93.0, 182.0),
    (93.0, 211.0),
    (93.0, 239.0),
    (48.0, 22.0),
    (258.0, 22.0),
    (454.0, 30.0),
    (454.0, 80.0),
    (454.0, 130.0),
    (454.0, 180.0),
    (454.0, 230.0),
    (328.0, 56.0),
    (328.0, 107.0),
    (328.0, 157.0),
    (762.0, 33.0),
    (760.0, 220.0),
    (580.0, 98.0),
];

/// Where the hovered (and, for 0..7, the lit) piece of each result is drawn (table 0x4c7b90).
const HOVER_AT: [(f32, f32); 20] = [
    (157.0, 135.0),
    (157.0, 170.0),
    (157.0, 205.0),
    (39.0, 115.0),
    (39.0, 143.0),
    (39.0, 171.0),
    (39.0, 199.0),
    (39.0, 227.0),
    (33.0, 7.0),
    (244.0, 7.0),
    (436.0, 11.0),
    (436.0, 61.0),
    (436.0, 111.0),
    (436.0, 161.0),
    (436.0, 211.0),
    (310.0, 37.0),
    (310.0, 87.0),
    (310.0, 137.0),
    (743.0, 16.0),
    (736.0, 192.0),
];

const TIPS: [&str; 21] = [
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
    "Exit",
    "Update Bio",
];

const MARITAL: [(u8, &str); 4] = [(0x08, "Single"), (0x10, "Married"), (0x20, "Divorced"), (0x40, "Widowed")];
const AGES: [(u8, &str); 3] = [(0x01, "Young"), (0x02, "Middle Aged"), (0x04, "Mature")];

/// The face picker's ten ball slots, top left (table 0x4c7be0): a zigzag of two rows.
const BALL_SLOTS: [(f32, f32); 10] = [
    (29.0, 305.0),
    (97.0, 425.0),
    (165.0, 305.0),
    (233.0, 425.0),
    (301.0, 305.0),
    (369.0, 425.0),
    (437.0, 305.0),
    (505.0, 425.0),
    (573.0, 305.0),
    (641.0, 425.0),
];

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

/// The walking figures' body set on the Customise screen (0x438970): women by age and trait as `person_look`, men by the
/// toggles being edited (no Imagination toggle: KLS; else PSS with Accuracy, SSS without); a fixed record by its body type.
fn walker_look(p: &Person, toggles: u8) -> usize {
    if p.female() {
        return person_look(p);
    }
    let nibble = ((p.b23 >> 4) & 3) as usize;
    if p.fixed != 0 {
        nibble
    } else if toggles & 4 == 0 {
        1
    } else {
        ((((!toggles) & 2) | 4) >> 1) as usize
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

/// A file name the exe would accept (0x405ac0 refuses the characters a file name cannot hold).
fn valid_file_name(name: &str) -> bool {
    !name.trim().is_empty() && !name.chars().any(|c| "\\/:*?\"<>|".contains(c) || c.is_control())
}

/// How a box over the editor is drawn: the exe's list box or its generic popup.
enum DialogBox {
    List(crate::popup_ui::ListBox),
    Choice(crate::popup_ui::ChoiceBox),
}

impl DialogBox {
    fn option_at(&self, x: f32, y: f32) -> Option<usize> {
        match self {
            DialogBox::List(b) => b.option_at(x, y),
            DialogBox::Choice(b) => b.option_at(x, y),
        }
    }
}

impl App {
    /// The golfer card's Customize: the editor on golfer `slot`'s person record (0x41ea4f).
    pub fn open_customise(&mut self, slot: usize) {
        let Some(gg) = self.club.g.get(slot) else { return };
        let record = gg.roster.max(0) as usize;
        let pro_mode = gg.kind & 0xe0 == 0x20;
        let toggles = gg.class & 7;
        self.open_customise_record(record, Some(slot), pro_mode, toggles);
    }

    /// The Player panel's Customize (0x436060): the player's own record 0 with the pro's skills, the toggles from the
    /// record (all three while it has never been fixed).
    pub fn open_customise_player(&mut self) {
        let Some(p) = self.club.roster.first() else { return };
        let toggles = if p.fixed == 0 { 7 } else { (p.fixed & 7) as u8 };
        let slot = (self.club.gary >= 0).then_some(self.club.gary as usize);
        self.open_customise_record(0, slot, true, toggles);
    }

    fn open_customise_record(&mut self, record: usize, slot: Option<usize>, pro_mode: bool, toggles: u8) {
        let Some(p) = self.club.roster.get_mut(record) else { return };
        if p.fixed == 0 {
            // the colours the composer would pick become the record's own, so that editing starts from the look shown
            let o = p.outfit(record as u32);
            p.shirt = o.shirt;
            p.pants = o.pants;
            p.skin = o.skin;
            p.hair = o.hair;
            p.alt_skin = o.alt_skin;
            p.b23 = (p.b23 & 0xf0) | (o.hat & 0xf);
        }
        let snapshot = p.clone();
        self.cust = Some(Customise { record, slot, pro_mode, snapshot, toggles, hover: -1, ..Default::default() });
        self.screen = Screen::Customise;
        self.ui_sound(0x2d);
    }

    fn cust_person(&self) -> Person {
        let r = self.cust.as_ref().map(|c| c.record).unwrap_or(0);
        self.club.roster.get(r).cloned().unwrap_or_default()
    }

    /// The main hit test (0x438260): the nearest centre within 40, the first eight measured with a third of dx; hair colour
    /// is no choice for men.
    fn cust_hit(&self, vx: f32, vy: f32) -> i32 {
        let female = self.cust_person().female();
        let mut best = (40.0, -1);
        for (i, &(cx, cy)) in CENTRES.iter().enumerate() {
            let dx = if i < 8 { (vx - cx) / 3.0 } else { vx - cx };
            let d = dist(dx, vy - cy);
            if d < best.0 {
                best = (d, i as i32);
            }
        }
        if best.1 == 15 && !female {
            return -1;
        }
        best.1
    }

    /// Where the pointer is over the name box: its row 0..3.
    fn cust_name_row(vx: f32, vy: f32) -> Option<usize> {
        ((vx - 154.0).abs() < 80.0 && (vy - 64.0).abs() < 40.0)
            .then(|| ((vy - 24.0) / 20.0).floor() as i32)
            .filter(|r| (0..4).contains(r))
            .map(|r| r as usize)
    }

    /// The theme pack folder (under Themes) characters are loaded from and saved to.
    fn cust_folder(&self) -> (String, PathBuf) {
        let pack = crate::render::THEME_PACKS.get(self.theme_pack).copied().unwrap_or("Standard").replace(' ', "_");
        let dir = self.game_path(&format!("Themes/{pack}"));
        (pack, dir)
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
        if let Some(d) = c.dialog.take() {
            self.cust = Some(c);
            return self.cust_dialog_click(d, vx, vy, right);
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
                return self.cust_save(false);
            }
            19 => return self.cust_ok(&c),
            _ => {}
        }
        let rec = c.record;
        let Some(p) = self.club.roster.get_mut(rec) else {
            self.cust = Some(c);
            return;
        };
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
            20 if !c.pro_mode => c.typing = Some((Field::Bio, p.bio.clone())),
            _ => {
                match Self::cust_name_row(vx, vy) {
                    Some(0) => c.typing = Some((Field::Name, String::new())),
                    Some(1) => c.typing = Some((Field::Job, String::new())),
                    Some(2) => p.b21 = cycle(p.b21, &MARITAL.map(|m| m.0), up),
                    Some(_) => p.b21 = cycle(p.b21, &AGES.map(|m| m.0), up),
                    None => {}
                }
                // the dialogue table's right column: a line to type, prefilled
                if (vx - 578.0).abs() < 178.0 && vy > 269.0 {
                    let row = ((vy - 270.0) / 16.0) as usize;
                    if row < SAYINGS {
                        c.typing = Some((Field::Saying(row), p.saying(row).unwrap_or("").to_string()));
                    }
                }
                // a pro's skills (not the player's own) step by one, the limits refusing with the error sound
                if c.pro_mode && rec != 0 && (vx - 520.0).abs() < 25.0 && vy > 61.0 && vy < 222.0 {
                    let row = ((vy - 64.0) / 16.0).max(0.0) as usize;
                    if let Some(v) = c.slot.and_then(|s| self.club.g.get_mut(s)).and_then(|gg| gg.skills.get_mut(row)) {
                        let ok = if up { *v < 10 } else { *v > 0 };
                        if ok {
                            if up {
                                *v += 1;
                            } else {
                                *v -= 1;
                            }
                        } else {
                            self.ui_sound(0x18);
                        }
                    }
                }
            }
        }
        if let Some(p) = self.club.roster.get_mut(rec) {
            if matches!(hit, 10 | 11 | 13..=17) {
                p.fixed |= 0x80;
            }
        }
        self.cust = Some(c);
    }

    fn cust_picker_click(&mut self, c: &mut Customise, page: usize, vx: f32, vy: f32, right: bool) {
        let p = self.cust_person();
        let gender = p.male_bit() as usize;
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
        for (i, &(x, y)) in BALL_SLOTS.iter().enumerate() {
            if dist(vx - x - 60.0, vy - y - 60.0) < 60.0 && page + i < count {
                let head = (page + i) as u8;
                if let Some(rp) = self.club.roster.get_mut(c.record) {
                    rp.head = Some(head);
                    // a stock face brings its own skin tone and hair colour (table 0x4d55e8)
                    if let Some((skin, hair, _)) = sg_core::roster::head_defaults(head, rp.female()) {
                        rp.skin = skin;
                        rp.hair = hair;
                    }
                }
                c.picker = None;
                return;
            }
        }
    }

    /// Load: "Pick one..." over the theme pack's *.pro files (mode 1) or *.chr files (mode 0), as the folder lists them.
    fn cust_load(&mut self) {
        let Some(pro) = self.cust.as_ref().map(|c| c.pro_mode) else { return };
        let ext = if pro { "pro" } else { "chr" };
        let (_, dir) = self.cust_folder();
        let mut files: Vec<_> = sg_core::fsutil::list_dir(&dir)
            .into_iter()
            .filter(|f| f.extension().and_then(|e| e.to_str()).is_some_and(|e| e.eq_ignore_ascii_case(ext)))
            .collect();
        files.sort_by_key(|p| p.file_name().map(|n| n.to_string_lossy().to_lowercase()).unwrap_or_default());
        files.truncate(20);
        if files.is_empty() {
            return;
        }
        if let Some(c) = self.cust.as_mut() {
            c.dialog = Some(Dialog::Load(files, 0));
        }
    }

    /// Loads a character file into the record (0x437fa0): the record and its dialogue lines; the skills go to the golfer
    /// slot for any record but the player's; a head of 20 or more is the file's own portrait.
    fn cust_load_file(&mut self, f: &std::path::Path) {
        let Some(c) = self.cust.clone() else { return };
        let Some(b) = sg_core::fsutil::read_file(f) else { return };
        if b.len() < sg_core::roster::RECORD {
            return;
        }
        let mut p = Person::parse_file(&b);
        if p.head_index(0) > 0x13 {
            p.portrait = Some(f.strip_prefix(&self.game_dir).unwrap_or(f).to_string_lossy().replace('\\', "/"));
        }
        let skills_at = sg_core::roster::RECORD + SAYINGS * sg_core::roster::SAYING;
        if c.record != 0 && b.len() >= skills_at + 10 {
            if let Some(gg) = c.slot.and_then(|s| self.club.g.get_mut(s)) {
                for (k, v) in gg.skills.iter_mut().take(10).enumerate() {
                    *v = b[skills_at + k].min(10);
                }
            }
        }
        if let Some(rp) = self.club.roster.get_mut(c.record) {
            *rp = p;
        }
        // the file's own portrait takes a custom head before the next frame is drawn
        self.portraits_due = true;
    }

    /// The head's three faces as a 140 x 420 8-bit PCX (0x437910): a stock head's column of the halo page of its gender, a
    /// custom head's own picture.
    fn cust_portrait(&self, p: &Person, record: usize) -> Option<Vec<u8>> {
        let head = p.head_index(record);
        let gi = p.male_bit() as usize;
        if head < 19 {
            let sheet = if p.female() { "Heads/golfballhalopage_female.pcx" } else { "Heads/golfballhalopage_male .pcx" };
            let d = sg_core::fsutil::read_file(self.game_path(sheet))?;
            let ix = sg_core::assets::decode_pcx_indexed(&d)?;
            let (x0, y0) = ((head / 2) as usize * 140, (head & 1) as usize * 420);
            if x0 + 140 > ix.w as usize || y0 + 420 > ix.h as usize {
                return None;
            }
            let mut out = sg_core::assets::Indexed { w: 140, h: 420, idx: vec![255; 140 * 420], pal: ix.pal };
            for y in 0..420 {
                let s = (y0 + y) * ix.w as usize + x0;
                out.idx[y * 140..y * 140 + 140].copy_from_slice(&ix.idx[s..s + 140]);
            }
            return Some(sg_core::assets::encode_pcx(&out));
        }
        let (file, off) = self.art.head_src[gi].get(head as usize - 19)?;
        sg_core::fsutil::read_file(file).map(|b| b[*off..].to_vec())
    }

    /// Save (0x437910 with the theme pack folder): Themes\<pack>\<name>.pro for a pro, .chr for anyone else; an invalid
    /// name says so, an existing file asks first; a saved file says where it went.
    fn cust_save(&mut self, overwrite: bool) {
        let Some(c) = self.cust.clone() else { return };
        let p = self.cust_person();
        let (pack, dir) = self.cust_folder();
        let ext = if c.pro_mode { ".pro" } else { ".chr" };
        let set = |app: &mut App, d: Dialog| {
            if let Some(c) = app.cust.as_mut() {
                c.dialog = Some(d);
            }
        };
        if !valid_file_name(&p.name) {
            return set(self, Dialog::Notice(vec![String::new(), "Invalid file name.".into()]));
        }
        let path = dir.join(format!("{}{ext}", p.name));
        if !overwrite && path.exists() {
            return set(self, Dialog::Overwrite(path, 0));
        }
        let skills = if c.record == 0 {
            self.club.pro_skill
        } else {
            let mut s = [0u8; 16];
            if let Some(gg) = c.slot.and_then(|s| self.club.g.get(s)) {
                for (k, v) in gg.skills.iter().take(10).enumerate() {
                    s[k] = *v;
                }
            }
            s
        };
        let mut sayings = p.sayings.clone();
        sayings.resize(SAYINGS, String::new());
        let portrait = self.cust_portrait(&p, c.record);
        let file = sg_core::championship::ProFile { person: p.clone(), record: Vec::new(), sayings, skills, portrait };
        if !sg_core::fsutil::write_file(&path, &sg_core::championship::pro_bytes(&file)) {
            return set(self, Dialog::Notice(vec![String::new(), "Invalid file path.".into()]));
        }
        set(self, Dialog::Notice(vec!["Character saved as".into(), format!("Themes\\{pack}\\{}{ext}", p.name)]));
    }

    /// The box each dialog is drawn as (EXACT positions from the calls): the Load list "Pick one..." and the file names is
    /// the list box at (100, 0x14) (0x4385d0 through 0x46de70); "Invalid file name." and "Invalid file path." the list box at
    /// (0x1e, 0x1e), each after an empty line (0x437910); the overwrite question the generic popup at (200, 0x1e); "Character
    /// saved as / Themes\<pack>\<name>" the generic popup at (300, 100) with no options.
    fn cust_dialog_box(d: &Dialog) -> DialogBox {
        use crate::popup_ui::{ChoiceBox, ListBox};
        match d {
            Dialog::Load(files, _) => {
                let mut v = vec!["Pick one...".to_string()];
                v.extend(files.iter().map(|f| format!(" {}", f.file_name().map(|n| n.to_string_lossy().into_owned()).unwrap_or_default())));
                DialogBox::List(ListBox::new(v, 100, 0x14))
            }
            Dialog::Overwrite(path, _) => {
                let name = path.file_name().map(|n| n.to_string_lossy().into_owned()).unwrap_or_default();
                let lines = vec![name, "already exists!".into(), " Overwrite the old version.".into(), " Cancel".into()];
                DialogBox::Choice(ChoiceBox::new(lines, 200.0, 30.0))
            }
            Dialog::Notice(lines) if lines.first().is_some_and(|l| l.is_empty()) => {
                DialogBox::List(ListBox::new(lines.clone(), 0x1e, 0x1e))
            }
            Dialog::Notice(lines) => DialogBox::Choice(ChoiceBox::new(lines.clone(), 300.0, 100.0)),
        }
    }

    /// The option of a box under the pointer.
    fn cust_dialog_option(d: &Dialog, vx: f32, vy: f32) -> Option<usize> {
        Self::cust_dialog_box(d).option_at(vx, vy)
    }

    fn cust_dialog_click(&mut self, d: Dialog, vx: f32, vy: f32, right: bool) {
        let pick = if right { None } else { Self::cust_dialog_option(&d, vx, vy) };
        match d {
            Dialog::Load(files, _) => {
                if let Some(f) = pick.and_then(|k| files.get(k)) {
                    let f = f.clone();
                    self.cust_load_file(&f);
                }
            }
            Dialog::Overwrite(..) => {
                if pick == Some(0) {
                    self.cust_save(true);
                }
            }
            Dialog::Notice(_) => {}
        }
    }

    /// Exit: the class toggles go into the record and the golfer; a pro's look goes back to the pro table (0x43a2b0); the
    /// card's golfer takes the record's age and marital byte (0x41ea62).
    fn cust_ok(&mut self, c: &Customise) {
        let Some(p) = self.club.roster.get_mut(c.record) else { return };
        p.fixed = (p.fixed & !7) | c.toggles as i32;
        let b21 = p.b21;
        if let Some(g) = c.slot.filter(|&g| g < sg_core::golfer::SLOTS) {
            let gg = &mut self.club.g[g];
            gg.class = (gg.class & 0xf8) | c.toggles;
            gg.looks = b21 as i8 as i16 as u16;
        }
        self.cust = None;
        self.screen = Screen::Play;
        self.ui_sound(0x2d);
    }

    /// Typing a name or profession (the exe's one-line box, 16 long: 15 characters), a dialogue line (48: 47) or the
    /// biography (500).
    pub fn cust_char(&mut self, ch: char) {
        if let Some((f, t)) = self.cust.as_mut().and_then(|c| c.typing.as_mut()) {
            let max = match f {
                Field::Name | Field::Job => 15,
                Field::Saying(_) => 47,
                Field::Bio => 500,
            };
            if !ch.is_control() && t.chars().count() < max {
                t.push(ch);
            }
        }
    }

    /// Keys: Enter keeps a typed name or profession when not empty (a dialogue line always: Esc empties it), Esc empties
    /// the box, Backspace deletes; Esc also closes the face picker and a box; Up and Down move in a box's options and
    /// Enter takes one.
    pub fn cust_key(&mut self, k: miniquad::KeyCode) {
        use miniquad::KeyCode;
        let Some(c) = self.cust.as_mut() else { return };
        if let Some(d) = c.dialog.as_mut() {
            let n = match d {
                Dialog::Load(f, _) => f.len(),
                Dialog::Overwrite(..) => 2,
                Dialog::Notice(_) => 0,
            };
            match (d, k) {
                (Dialog::Load(_, s) | Dialog::Overwrite(_, s), KeyCode::Up) if n > 0 => *s = (*s + n - 1) % n,
                (Dialog::Load(_, s) | Dialog::Overwrite(_, s), KeyCode::Down) if n > 0 => *s = (*s + 1) % n,
                (_, KeyCode::Enter | KeyCode::KpEnter | KeyCode::Space) => {
                    let Some(d) = c.dialog.take() else { return };
                    match d {
                        Dialog::Load(files, s) => {
                            if let Some(f) = files.get(s).cloned() {
                                self.cust_load_file(&f);
                            }
                        }
                        Dialog::Overwrite(_, 0) => self.cust_save(true),
                        _ => {}
                    }
                }
                (_, KeyCode::Escape) => c.dialog = None,
                _ => {}
            }
            return;
        }
        let rec = c.record;
        if let Some((f, t)) = c.typing.as_mut() {
            match k {
                KeyCode::Backspace => {
                    t.pop();
                }
                KeyCode::Escape if *f == Field::Bio => c.typing = None,
                KeyCode::Escape => t.clear(),
                KeyCode::Enter | KeyCode::KpEnter => {
                    let (f, t) = (*f, t.clone());
                    c.typing = None;
                    if let Some(p) = self.club.roster.get_mut(rec) {
                        match f {
                            Field::Name if !t.trim().is_empty() => p.name = t.trim().to_string(),
                            Field::Job if !t.trim().is_empty() => p.job = t.trim().to_string(),
                            Field::Saying(r) => {
                                if p.sayings.len() < SAYINGS {
                                    p.sayings.resize(SAYINGS, String::new());
                                }
                                p.sayings[r] = t;
                            }
                            Field::Bio => p.bio = t,
                            _ => {}
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
        if std::mem::take(&mut self.portraits_due) {
            self.register_portraits(g);
        }
        let Some(mut c) = self.cust.clone() else { return };
        let s = Ui::new(self.draw_w, self.draw_h);
        self.view = s.view;
        let (mx, my) = self.card_ui.mouse;
        let p = self.cust_person();
        let gi = p.male_bit() as usize;
        s.image(g, &self.art.cust_bg[gi], 0.0, 0.0);
        // the preview: sky and grass, the body still of the body type (a child's small twin), the head
        let custom = p.head_index(c.record) >= 19;
        s.image(g, &self.art.head_body, if custom { 316.0 } else { 336.0 }, 20.0);
        let nibble = ((p.b23 >> 4) & 3) as usize;
        let still = if p.female() { 5 + nibble } else { nibble };
        let mut o = p.outfit(c.record as u32);
        // the preview palette (slot 0x99) takes the toggles being edited: pale hands with Accuracy
        if c.toggles & 2 != 0 {
            o.alt_skin = 4;
        }
        let swaps = std::mem::take(&mut self.swaps);
        self.art.head_over_body(g, &s, &swaps, &p, o, c.record, still, 1, 336.0, 40.0);
        self.swaps = swaps;
        // the blue disc under the two small figures walking each way (frame 5 of the walk, views 0 and 4, zoom 4)
        s.image_part(g, &self.art.cg, 299.0, 220.0, 300.0, 500.0, 57.0, 38.0);
        let walk = walker_look(&p, c.toggles);
        if let (Some(si), _) = self.golfer_clip(walk as i32) {
            for (x, view) in [(315.0, 0), (335.0, 4)] {
                let (w, h, ax, ay) = {
                    let sp = &self.sprites[si].s;
                    let n = sp.frames_per_view.max(1);
                    let fr = &sp.frames[sp.frame_index(view, 5 % n)];
                    (fr.w as f32, fr.h as f32, sp.anchor_x as f32, sp.anchor_y as f32)
                };
                let n = self.sprites[si].s.frames_per_view.max(1);
                if let Some(tex) = self.outfit_texture(g, si, view, 5 % n, o) {
                    s.image(g, &Image { tex: Some(tex), w, h }, x - ax, 239.0 - ay);
                }
            }
        }
        // the round buttons that overlap the window, back on top of it
        for k in 0..8 {
            let (x, y) = if k < 3 { (309.0, 37.0 + 50.0 * k as f32) } else { (435.0, 11.0 + 50.0 * (k - 3) as f32) };
            s.image_part(g, &self.art.cust_bg[gi], x, y, x, y, 39.0, 39.0);
        }
        let busy = c.picker.is_some() || c.typing.is_some() || c.dialog.is_some();
        let hit = if busy { -1 } else { self.cust_hit(mx, my) };
        if hit != c.hover {
            c.hover = hit;
            c.frames = 0;
        } else {
            c.frames += 1;
        }
        let cg = &self.art.cg;
        // the lit piece of a result at its position: the class toggles and traits that are on, then the hovered result
        let lit = |g: &mut Gfx, i: usize| {
            let (x, y) = HOVER_AT[i];
            match i {
                0..=2 => s.image_part(g, cg, x, y, 150.0, 300.0 + 50.0 * i as f32, 112.0, 35.0),
                3..=7 => s.image_part(g, cg, x, y, 150.0, 50.0 + 50.0 * (i - 3) as f32, 112.0, 28.0),
                8 => s.image_part(g, cg, x, y, 0.0, 0.0, 35.0, 37.0),
                9 => s.image_part(g, cg, x, y, 100.0, 0.0, 35.0, 37.0),
                18 => s.image_part(g, cg, x, y, 500.0, 0.0, 60.0, 61.0),
                19 => s.image_part(g, cg, x, y, 500.0, 100.0, 60.0, 61.0),
                // the round tabs: right of the window rows 0..4 of the sheet (body type, adult/child, face, shirt,
                // trousers), left of it rows 5..7 (hair, skin, gender)
                _ => s.image_part(g, cg, x, y, 350.0, 50.0 * (i - 10) as f32, 50.0, 50.0),
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
        if (0..20).contains(&hit) {
            lit(g, hit as usize);
        }
        // the name box: name (a child's with " Jr."), "(profession)", marital status, age group; the row under the pointer
        // lit (the name orange, the others white)
        let navy = c15(0x0848);
        let row = if hit < 0 && !busy { Self::cust_name_row(mx, my) } else { None };
        let marital = MARITAL.iter().rev().find(|m| p.b21 & m.0 != 0).map(|m| m.1).unwrap_or("");
        let age = AGES.iter().rev().find(|m| p.b21 & m.0 != 0).map(|m| m.1).unwrap_or("");
        let name = if p.fixed & 8 != 0 { format!("{} Jr.", p.name) } else { p.name.clone() };
        let job = format!("({})", p.job);
        for (r, (y, t, size)) in
            [(24.0, name.as_str(), 16.0), (48.0, job.as_str(), 16.0), (69.0, marital, 13.0), (90.0, age, 13.0)].iter().enumerate()
        {
            let col = match row {
                Some(0) if r == 0 => c15(0x7b20),
                Some(k) if k == r => c15(0x7fff),
                _ => navy,
            };
            s.text_centered(g, 154.0, top(*y, *size), t, *size, col);
        }
        // the trait and class labels
        for (i, w) in TRAIT_WORDS.iter().enumerate() {
            let col = if p.traits & (1 << i) != 0 { navy } else { c15(0x4210) };
            s.text_centered(g, 93.0, top(122.0 + 28.0 * i as f32, 13.0), w, 13.0, col);
        }
        for (i, w) in CLASS_WORDS.iter().enumerate() {
            let col = if c.toggles & (1 << i) != 0 { navy } else { c15(0x4210) };
            let w = format!("{}{}", w[..1].to_ascii_uppercase(), &w[1..]);
            s.text_centered(g, 212.0, top(142.0 + 35.0 * i as f32, 16.0), &w, 16.0, col);
        }
        if c.pro_mode {
            // the skills, read only for the player's own record
            s.text_centered(g, 585.0, top(30.0, 16.0), "Golf Skill Levels", 16.0, rgb(0.0, 0.0, 0.0));
            for i in 0..10 {
                let y = 61.0 + 16.0 * i as f32;
                let v = if c.record == 0 {
                    self.club.pro_skill[i]
                } else {
                    c.slot.and_then(|s| self.club.g.get(s)).map(|gg| gg.skills[i]).unwrap_or(0)
                };
                s.fill(g, 498.0, y - 3.0, 42.0, 14.0, rgb(0.0, 0.0, 0.0));
                s.fill(g, 499.0, y - 2.0, 40.0, 12.0, c15(if v != 0 { 0x7d08 } else { 0x21e8 }));
                if v != 0 {
                    s.text(g, 501.0, top(y, SMALL), &format!("+{}%", v as i32 * 10), SMALL, rgb(0.0, 0.0, 0.0));
                }
                s.text(g, 546.0, top(y, SMALL), SKILL_NAMES[i], SMALL, if v != 0 { rgb(0.0, 0.0, 0.0) } else { c15(0x4210) });
            }
        } else {
            self.draw_cust_member(g, &s, &p, c.record, hit);
        }
        // the dialogue table: each row's label in the colour of its stock line, then the person's own line in navy or
        // the stock line in grey
        let gs = c.slot.unwrap_or(0).min(sg_core::golfer::SLOTS - 1);
        for (r, label) in SAYING_LABELS.iter().enumerate() {
            let y = 271.0 + 16.0 * r as f32;
            let stock = self.club.stock_thought(&self.course, SAYING_CODES[r], 0, gs, c.record);
            let tone = match stock.tone {
                sg_core::thoughts::Tone::Good => c15(0x23e8),
                sg_core::thoughts::Tone::Bad => c15(0x7d08),
                _ => c15(0x6318),
            };
            s.text(g, 52.0, top(y, SMALL), &format!("{label}..."), SMALL, tone);
            let (t, col) = match p.saying(r) {
                Some(t) => (t.to_string(), navy),
                None => (stock.text, c15(0x4210)),
            };
            s.text(g, 412.0, top(y, SMALL), &t, SMALL, col);
        }
        if let Some(page) = c.picker {
            self.draw_face_picker(g, &s, &p, page, mx, my);
        }
        if let Some((f, t)) = &c.typing {
            let caret = if (self.clock * 2.0) as i64 % 2 == 0 { "|" } else { "" };
            if *f == Field::Bio {
                // the biography box (0x40cc00 at (472, 104), 316 x 216)
                let (x, y, w, h) = (472.0, 104.0, 316.0, 216.0);
                s.fill(g, x - 1.0, y - 1.0, w + 2.0, h + 2.0, rgb(1.0, 1.0, 1.0));
                s.fill(g, x, y, w, h, c15(0x35b3));
                s.fill(g, x + 8.0, y + 8.0, w - 16.0, h - 16.0, rgb(1.0, 1.0, 1.0));
                for (k, l) in wrap_text(&format!("{t}{caret}"), 13.0, w - 28.0).iter().enumerate().take(12) {
                    s.text(g, x + 14.0, top(y + 12.0 + 16.0 * k as f32, 13.0), l, 13.0, rgb(0.0, 0.0, 0.0));
                }
            } else {
                // the one-line box (0x45b2c0): the name and profession at (200, 32), 16 long; a line at (300, (row + 15) * 16),
                // 48 long
                let (x, y, n, prompt) = match f {
                    Field::Name => (200.0, 32.0, 16.0, "New name..."),
                    Field::Job => (200.0, 32.0, 16.0, "Profession..."),
                    Field::Saying(r) => (300.0, (*r as f32 + 15.0) * 16.0, 48.0, "New text..."),
                    Field::Bio => (0.0, 0.0, 0.0, ""),
                };
                let w = n * 12.0 + 32.0;
                s.fill(g, x - 1.0, y - 1.0, w + 2.0, 50.0, rgb(1.0, 1.0, 1.0));
                s.fill(g, x + 1.0, y + 1.0, w, 48.0, rgb(0.0, 0.0, 0.0));
                s.fill(g, x, y, w, 48.0, c15(0x35b3));
                s.text(g, x + 4.0, top(y + 6.0, 13.0), prompt, 13.0, rgb(1.0, 1.0, 1.0));
                s.fill(g, x + 16.0, y + 23.0, w - 32.0, 22.0, rgb(1.0, 1.0, 1.0));
                s.text(g, x + 20.0, top(y + 28.0, 14.0), &format!("{t}{caret}"), 14.0, rgb(0.0, 0.0, 0.0));
            }
        }
        if let Some(d) = c.dialog.as_mut() {
            if let Some(k) = Self::cust_dialog_option(d, mx, my) {
                if let Dialog::Load(_, s) | Dialog::Overwrite(_, s) = d {
                    *s = k;
                }
            }
            self.draw_cust_dialog(g, &s, d);
        }
        if hit >= 0 && c.frames > 10 && (hit != 20 || !c.pro_mode) {
            tooltip(g, &s, mx, my, TIPS[hit as usize]);
        }
        if let Some(cc) = self.cust.as_mut() {
            cc.hover = c.hover;
            cc.frames = c.frames;
            if let (Some(d), Some(cd)) = (c.dialog, cc.dialog.as_mut()) {
                *cd = d;
            }
        }
        g.flush();
    }

    /// The membership panel of mode 0: the level (a gold or platinum one on a teal bar), low round, handicap, rounds
    /// played, the Update Bio link and the biography.
    fn draw_cust_member(&self, g: &mut Gfx, s: &Ui, p: &Person, record: usize, hit: i32) {
        let m = self.club.members.get(record).cloned().unwrap_or_default();
        let level = (m.level & 7) as i32;
        if m.gone != 0xff && level > 3 {
            s.fill(g, 500.0, 28.0, 160.0, 17.0, c15(0x0210));
        }
        let rank = if m.gone == 0xff {
            Some(("Resigned", 0x6000))
        } else {
            match level {
                1 => Some(("Visitor", 0x6318)),
                2 => Some(("Member", 0x4210)),
                3 => Some(("Silver Member", 0x2108)),
                4 => Some(("Gold Member", 0x7ff0)),
                5 => Some(("Platinum Member", 0x7fff)),
                _ => None,
            }
        };
        if let Some((t, col)) = rank {
            s.text_centered(g, 580.0, top(30.0, 13.0), t, 13.0, c15(col));
        }
        let ink = c15(0x2108);
        let low = if m.best == 0 { "-".to_string() } else { m.best.to_string() };
        let hcp = if m.avg < 1 { "-".to_string() } else { m.avg.to_string() };
        let rounds = if m.rounds == 0 { "none".to_string() } else { m.rounds.to_string() };
        s.text_centered(g, 580.0, top(50.0, 13.0), &format!("Low round: {low}"), 13.0, ink);
        s.text_centered(g, 580.0, top(64.0, 13.0), &format!("Handicap: {hcp}"), 13.0, ink);
        s.text_centered(g, 580.0, top(78.0, 13.0), &format!("Rounds played: {rounds}"), 13.0, ink);
        s.text_centered(g, 580.0, top(98.0, 13.0), "Bio", 13.0, c15(if hit == 20 { 0x7b20 } else { 0x211f }));
        for (k, l) in wrap_text(&p.bio, 13.0, 236.0).iter().enumerate().take(9) {
            s.text(g, 484.0, top(114.0 + 14.0 * k as f32, 13.0), l, 13.0, c15(0x0018));
        }
    }

    /// A box over the editor (see `cust_dialog_box`), the option under the pointer (or picked by the keys) lit.
    fn draw_cust_dialog(&self, g: &mut Gfx, s: &Ui, d: &Dialog) {
        let sel = match d {
            Dialog::Load(_, k) | Dialog::Overwrite(_, k) => Some(*k),
            Dialog::Notice(_) => None,
        };
        match Self::cust_dialog_box(d) {
            DialogBox::List(b) => self.draw_list_box(g, s, &b, sel),
            DialogBox::Choice(b) => self.draw_choice_box(g, s, &b, sel),
        }
    }

    /// The face picker over the lower half: ten balls a page with the heads of the record's gender, the page arrows.
    fn draw_face_picker(&self, g: &mut Gfx, s: &Ui, p: &Person, page: usize, mx: f32, my: f32) {
        let hs = &self.art.head_select;
        s.image_part(g, hs, 0.0, 258.0, 0.0, 258.0, 800.0, 342.0);
        let count = self.art.head_count(p.male_bit() as usize);
        for (i, &(x, y)) in BALL_SLOTS.iter().enumerate() {
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
        // the walking figures of a man follow the toggles being edited
        let w = Person::default();
        assert_eq!(walker_look(&w, 0), 1);
        assert_eq!(walker_look(&w, 4), 3);
        assert_eq!(walker_look(&w, 6), 2);
    }

    #[test]
    fn file_names() {
        assert!(valid_file_name("Gary Golf"));
        assert!(!valid_file_name("a/b"));
        assert!(!valid_file_name("  "));
    }
}
