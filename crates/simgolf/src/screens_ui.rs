//! The clubhouse and record screens: SELECT THE NEXT PAIR OF GOLFERS (clubhouse), the golfers list and the golfer card,
//! the Professional Accomplishments board (F10 and after each accomplishment), the year-end report and the Membership
//! Roster (F9). Positions follow the exe's 800 x 600 layouts (docs/PUBLISHER_EXE_NOTES.md, "The clubhouse and the golfer
//! screens"; the golfer card from docs/DECODE_GOLFERCARD.md, DECODE_CARDS2.md, DECODE_CARDS3.md and DECODE_FACES.md); the
//! art is the disc's.

use crate::app::*;
use crate::gfx::Gfx;
use crate::render::Rect;
use crate::ui::{load_pcx, load_pcx_alpha, rgb, rgba, text_width, wrap_text, Image, Screen as Ui};
use sg_core::bodies::{Outfit, Swaps};
use sg_core::golfer::{flag, SLOTS};
use sg_core::pro::SKILL_NAMES;
use sg_core::records::TITLES;
use std::collections::HashMap;

/// The interface art these screens use.
#[derive(Default)]
pub struct Art {
    pub pair_base: Image,
    pub pair_buttons: Image,
    pub board: Image,
    pub trophy: Image,
    pub tacs: Image,
    pub endo: Image,
    pub roster: Image,
    /// GolferStats: the golfer card's plates and its round buttons (with its alpha sheet).
    pub stats: Image,
    /// TransPopups: the translucent dialog frame, the skill rows and pads, the balls (with its alpha sheet).
    pub trans: Image,
    /// The stock heads by gender (0 women, 1 men, as `Person::male_bit`): the 140 x 140 ball portraits and the 90 x 120
    /// expression cells; then the custom portraits found in Heads/ (140 x 420 each), in file name order.
    pub halo: [Image; 2],
    pub expr: [Image; 2],
    pub heads: [Vec<Image>; 2],
    /// Customise Golfer: the backgrounds (women, men), its buttons, the face picker and the preview window.
    pub cust_bg: [Image; 2],
    pub cg: Image,
    pub head_select: Image,
    pub head_body: Image,
    /// The 60 x 120 body stills (Bodies/*.pcx) by body index 0..8, then their small (child) twins; recoloured on use.
    body_pcx: Vec<Option<Vec<u8>>>,
    bodies: HashMap<(usize, Outfit), Image>,
}

/// Body still files by the exe's body index (0..3 men, 5..8 women, docs/DECODE_CUSTOMISE.md 3.3).
const BODY_FILES: [&str; 9] = ["MalePLS", "MaleKLS", "MalePSS", "MaleSSS", "", "FemalePLS", "FemaleSSS", "FemalePSS", "FemaleSkTT"];

impl Art {
    pub fn load(g: &mut Gfx, app: &App) -> Art {
        let p = |rel: &str| app.game_path(&format!("Interface/{rel}"));
        let plain = |g: &mut Gfx, rel: &str| load_pcx(g, &p(rel), false, None).unwrap_or_default();
        let keyed = |g: &mut Gfx, rel: &str| load_pcx(g, &app.game_path(rel), true, Some(0xf800f8)).unwrap_or_default();
        let alpha = |g: &mut Gfx, rel: &str, a: &str| load_pcx_alpha(g, &p(rel), &p(a)).unwrap_or_default();
        let mut heads: [Vec<Image>; 2] = [Vec::new(), Vec::new()];
        let mut files = sg_core::fsutil::list_dir(app.game_path("Heads"));
        files.sort();
        for f in files {
            let name = f.file_name().and_then(|n| n.to_str()).unwrap_or("").to_string();
            let gender = match name.chars().next() {
                Some('F' | 'f') => 0,
                Some('M' | 'm') => 1,
                _ => continue,
            };
            // the exe takes custom heads until its count reaches 0x48
            if !name.to_ascii_lowercase().ends_with(".pcx") || heads[gender].len() >= 0x48 - 19 {
                continue;
            }
            if let Some(img) = load_pcx(g, &f, true, None).filter(|i| i.w == 140.0 && i.h == 420.0) {
                heads[gender].push(img);
            }
        }
        let mut body_pcx = Vec::new();
        for suffix in ["", "_sm"] {
            for name in BODY_FILES {
                let path = app.game_path(&format!("Bodies/{name}{suffix}.pcx"));
                body_pcx.push(if name.is_empty() { None } else { sg_core::fsutil::read_file(path) });
            }
        }
        Art {
            pair_base: plain(g, "PairBase.pcx"),
            pair_buttons: load_pcx(g, &p("PairButtons.pcx"), true, None).unwrap_or_default(),
            board: plain(g, "bulletinboard&mantlewood.pcx"),
            trophy: alpha(g, "TROPHYparts.pcx", "TROPHYparts_A.pcx"),
            tacs: alpha(g, "tacs&tees.pcx", "tacs&tees_A.pcx"),
            endo: alpha(g, "infoscreens/ENDoYEAR.pcx", "infoscreens/ENDoYEAR_alpha.pcx"),
            roster: alpha(g, "infoscreens/memberRoster.pcx", "infoscreens/memberRoster_alpha.pcx"),
            stats: alpha(g, "GolferStats.pcx", "GolferStats_A.pcx"),
            trans: alpha(g, "TransPopups.pcx", "TransPopups_A.pcx"),
            halo: [keyed(g, "Heads/golfballhalopage_female.pcx"), keyed(g, "Heads/golfballhalopage_male .pcx")],
            expr: [keyed(g, "Heads/sim_FEMALE_all_expressionsflat.pcx"), keyed(g, "Heads/sim_MALE_all_expressionsflat.pcx")],
            heads,
            cust_bg: [plain(g, "CustGolfBckgrnd.pcx"), plain(g, "CustGlfBckMale.pcx")],
            cg: keyed(g, "Interface/CGButtons.pcx"),
            head_select: keyed(g, "Interface/HeadSelect.pcx"),
            head_body: keyed(g, "Interface/HeadBodyBck.pcx"),
            body_pcx,
            bodies: HashMap::new(),
        }
    }

    /// Stock heads plus the custom portraits of a gender (0 women, 1 men): the face picker's count.
    pub fn head_count(&self, gender: usize) -> usize {
        19 + self.heads[gender.min(1)].len()
    }

    /// A body still (body index 0..8, the small twin for a child) in an outfit's colours: the composed golfer palette
    /// (sg_core::bodies) put in place of the file's own, index 255 kept as the transparent key.
    pub fn body(&mut self, g: &mut Gfx, swaps: &Swaps, look: usize, child: bool, o: Outfit) -> Option<Image> {
        let k = (look.min(8) + if child { 9 } else { 0 }, o);
        if let Some(i) = self.bodies.get(&k) {
            return Some(*i);
        }
        let mut d = self.body_pcx.get(k.0)?.clone()?;
        if let Some(mut pal) = swaps.compose(&o) {
            pal[765..768].copy_from_slice(&[255, 0, 255]);
            let n = d.len();
            if n > 768 {
                d[n - 768..].copy_from_slice(&pal);
            }
        }
        let mut img = sg_core::assets::decode_pcx(&d).ok()?;
        for px in img.px.as_chunks_mut::<4>().0 {
            if px[0] >= 248 && px[1] == 0 && px[2] >= 248 {
                px[3] = 0;
            }
        }
        let im = Image { tex: Some(g.texture(&img, false)), w: img.w as f32, h: img.h as f32 };
        self.bodies.insert(k, im);
        Some(im)
    }

    /// A ball portrait (0x45c200 in its ball mode): the golf ball piece of TransPopups with the head's 140 x 140 cell on top,
    /// both at (x, y). `e` is the expression (0 happy, 1 neutral, 2 angry).
    #[allow(clippy::too_many_arguments)]
    pub fn ball_head(&self, g: &mut Gfx, s: &Ui, female: bool, head: u8, e: usize, x: f32, y: f32) {
        s.image_part(g, &self.trans, x, y, 0.0, 300.0, 140.0, 140.0);
        self.halo_cell(g, s, female, head, e, x, y);
    }

    /// The head's 140 x 140 cell alone: a stock head from the halo page, a custom one from its own portrait file.
    #[allow(clippy::too_many_arguments)]
    pub fn halo_cell(&self, g: &mut Gfx, s: &Ui, female: bool, head: u8, e: usize, x: f32, y: f32) {
        let gi = (!female) as usize;
        let e = e.min(2) as f32;
        if head < 19 {
            let (sx, sy) = ((head / 2) as f32 * 140.0, (head & 1) as f32 * 420.0 + e * 140.0);
            s.image_part(g, &self.halo[gi], x, y, sx, sy, 140.0, 140.0);
        } else if let Some(img) = self.heads[gi].get(head as usize - 19) {
            s.image_part(g, img, x, y, 0.0, e * 140.0, 140.0, 140.0);
        }
    }

    /// A head over its body (0x45c200 mode 1) for person record `p` (roster index `id`, body index `look`): the body still
    /// at (x + 19, y + 76), then the head, a stock head's 90 x 120 expression cell at (x, y - 20) or a custom head's 140
    /// cell at (x - 20, y - 20).
    #[allow(clippy::too_many_arguments)]
    pub fn head_over_body(
        &mut self,
        g: &mut Gfx,
        s: &Ui,
        swaps: &Swaps,
        p: &sg_core::roster::Person,
        o: Outfit,
        id: usize,
        look: usize,
        e: usize,
        x: f32,
        y: f32,
    ) {
        if let Some(b) = self.body(g, swaps, look, p.fixed & 8 != 0, o) {
            s.image(g, &b, x + 19.0, y + 76.0);
        }
        let head = p.head_index(id);
        if head < 19 {
            let (sx, sy) = ((head / 2) as f32 * 100.0, (head & 1) as f32 * 372.0 + [4.0, 128.0, 252.0][e.min(2)]);
            s.image_part(g, &self.expr[p.male_bit() as usize], x, y - 20.0, sx, sy, 90.0, 120.0);
        } else {
            self.halo_cell(g, s, p.female(), head, e, x - 20.0, y - 20.0);
        }
    }

    /// The translucent dialog frame (0x40cef0 with TransPopups): the inside darkened by half, as the shadow sheet does, and the
    /// 3 x 3 border of 16 x 16 pieces around it, its sides repeated.
    pub fn trans_frame(&self, g: &mut Gfx, s: &Ui, x: f32, y: f32, w: f32, h: f32) {
        let dark = rgba(0.0, 0.0, 0.0, 0.5);
        s.fill(g, x + 3.0, y, w - 6.0, h, dark);
        s.fill(g, x, y + 3.0, 3.0, h - 6.0, dark);
        s.fill(g, x + w - 3.0, y + 3.0, 3.0, h - 6.0, dark);
        if self.trans.tex.is_none() {
            return;
        }
        let t = &self.trans;
        let piece = |g: &mut Gfx, col: usize, row: usize, dx: f32, dy: f32, pw: f32, ph: f32| {
            s.image_part(g, t, dx, dy, 17.0 * col as f32, 17.0 * row as f32, pw, ph);
        };
        let (r, b) = (x + w - 16.0, y + h - 16.0);
        let mut cx = x + 16.0;
        while cx < r {
            let pw = (r - cx).min(16.0);
            piece(g, 1, 0, cx, y, pw, 16.0);
            piece(g, 1, 2, cx, b, pw, 16.0);
            cx += 16.0;
        }
        let mut cy = y + 16.0;
        while cy < b {
            let ph = (b - cy).min(16.0);
            piece(g, 0, 1, x, cy, 16.0, ph);
            piece(g, 2, 1, r, cy, 16.0, ph);
            cy += 16.0;
        }
        piece(g, 0, 0, x, y, 16.0, 16.0);
        piece(g, 2, 0, r, y, 16.0, 16.0);
        piece(g, 0, 2, x, b, 16.0, 16.0);
        piece(g, 2, 2, r, b, 16.0, 16.0);
    }
}

/// The golfer card's own state: the story page (0x824144), the button under the pointer and for how many frames, the
/// pointer, and a golfer picked up by Move/Eject Golfer.
#[derive(Clone, Debug, Default)]
pub struct CardUi {
    pub story: bool,
    pub hover: i32,
    pub frames: u32,
    pub mouse: (f32, f32),
    pub held: Option<usize>,
}

const MONTHS: [&str; 8] = ["March", "April", "May", "June", "July", "August", "September", "October"];
// the exe shows no status text for Platinum
const LEVELS: [&str; 6] = ["", "Visitor", "Member", "Silver Member", "Gold Member", ""];

/// The five trait words of the person record's byte +0x20 (pointer table 0x4c2864, not in the decode). PLACEHOLDER: the
/// words are guessed from the letters the story files use for the set bits (T, O, A, P, N).
pub const TRAIT_WORDS: [&str; 5] = ["Talkative", "Outgoing", "Athletic", "Playful", "Nice"];
/// The three skill class words (table 0x4c2858, "length" first).
pub const CLASS_WORDS: [&str; 3] = ["length", "accuracy", "imagination"];

/// The fonts of the card and the dialogs: heading, body and small.
pub const LARGE: f32 = 16.0;
pub const BODY: f32 = 13.0;
pub const SMALL: f32 = 11.0;

fn black() -> [f32; 4] {
    rgb(0.05, 0.05, 0.1)
}

/// An exe colour word (15-bit RGB, 0RRRRRGGGGGBBBBB).
pub fn c15(v: u32) -> [f32; 4] {
    rgb(((v >> 10) & 31) as f32 / 31.0, ((v >> 5) & 31) as f32 / 31.0, (v & 31) as f32 / 31.0)
}

/// The exe places text by its top; ours draws from the baseline.
pub fn top(y: f32, size: f32) -> f32 {
    y + size * 0.78
}

/// The exe's distance metric (0x467170).
pub fn dist(dx: f32, dy: f32) -> f32 {
    let (a, b) = (dx.abs(), dy.abs());
    if b < a {
        (b + 2.0 * a) / 2.0
    } else {
        (a + 2.0 * b) / 2.0
    }
}

/// Words joined with commas and "and" before the last.
pub fn and_list(words: &[&str]) -> String {
    match words {
        [] => String::new(),
        [a] => a.to_string(),
        [rest @ .., last] => format!("{} and {last}", rest.join(", ")),
    }
}

/// The trait words of a person's trait byte.
pub fn trait_words(traits: u8) -> Vec<&'static str> {
    (0..5).filter(|b| traits & (1 << b) != 0).map(|b| TRAIT_WORDS[b]).collect()
}

/// Good minus bad among a golfer's five newest reactions (0x45c420).
pub fn trend(gg: &sg_core::golfer::Golfer) -> i32 {
    gg.args[..5]
        .iter()
        .map(|&a| match a & 0xc000 {
            0x4000 => 1,
            0xc000 => -1,
            _ => 0,
        })
        .sum()
}

/// The golfer card's five bars: the label and the dark (unfilled) width; a long dark part is bad in every row.
fn bars(gg: &sg_core::golfer::Golfer) -> [(&'static str, i32); 5] {
    [
        ("Fun", ((8 - gg.mood) * 10).clamp(0, 80)),
        ("Attitude", ((4 - trend(gg)) * 10).clamp(0, 80)),
        ("Energy", (gg.fatigue / 4).clamp(0, 80)),
        ("Hunger", (gg.hunger * 5 / 2).clamp(0, 80)),
        ("Thirst", (gg.thirst * 5 / 2).clamp(0, 80)),
    ]
}

/// A card button: id, GolferStats button row, icon top-left and hit centre.
type CardButton = (i32, usize, f32, f32, f32, f32);
/// The card's buttons in the single layout, then on the story page.
const CARD_BUTTONS: [[CardButton; 5]; 2] = [
    [
        (0, 0, 293.0, 237.0, 318.0, 262.0),
        (1, 2, 358.0, 237.0, 383.0, 262.0),
        (2, 3, 423.0, 237.0, 448.0, 262.0),
        (3, 4, 488.0, 237.0, 513.0, 262.0),
        (4, 5, 570.0, 237.0, 595.0, 262.0),
    ],
    [
        (0, 0, 269.0, 270.0, 294.0, 287.0),
        (5, 6, 334.0, 272.0, 359.0, 287.0),
        (2, 8, 399.0, 272.0, 424.0, 287.0),
        (3, 7, 464.0, 272.0, 489.0, 287.0),
        (4, 9, 526.0, 272.0, 545.0, 287.0),
    ],
];

impl App {
    // ---- the clubhouse pair screen ------------------------------------------------------------------------------------

    /// Clicking the clubhouse opens the pair screen (not in a tournament, once a hole is open).
    pub fn open_pair_screen(&mut self) -> bool {
        if self.club.game & sg_core::golfer::game::TOURNAMENT != 0 || self.club.next_hole < 2 {
            return false;
        }
        self.pair_picks.clear();
        self.screen = Screen::Pair;
        self.hover = -1;
        true
    }

    /// The cell under the pointer: x from 6 to the frame width plus 335, y from 50 down (the exe's lower bound tests the
    /// width, so in effect there is none).
    fn pair_cell(vx: f32, vy: f32) -> i32 {
        if vx < 6.0 || vy < 50.0 || vx >= 329.0 + 335.0 {
            return -1;
        }
        let col = ((vx - 6.0) / 334.0) as i32;
        let row = ((vy - 50.0) / 136.0) as i32;
        row * 2 + col
    }

    pub fn pair_pointer(&mut self, vx: f32, vy: f32) {
        self.hover = Self::pair_cell(vx, vy);
    }

    pub fn pair_click(&mut self, vx: f32, vy: f32) {
        let list = self.club.waiting();
        let k = Self::pair_cell(vx, vy);
        if k >= 0 && (k as usize) < list.len() {
            let s = list[k as usize];
            if let Some(i) = self.pair_picks.iter().position(|&p| p == s) {
                self.pair_picks.remove(i);
            } else if self.pair_picks.len() < 2 {
                self.pair_picks.push(s);
            } else {
                self.ui_sound(0x18);
            }
            return;
        }
        if k >= 0 {
            // an empty cell
            self.ui_sound(0x18);
            return;
        }
        // a click outside the cells (or a key) closes the screen
        self.close_pair_screen();
    }

    pub fn close_pair_screen(&mut self) {
        self.screen = Screen::Play;
        let picks = std::mem::take(&mut self.pair_picks);
        if picks.is_empty() {
            return;
        }
        if picks.len() != 2 || !self.club.pick_pair(&mut self.course, &mut self.exe_rng, picks[0], picks[1]) {
            self.ui_sound(0x18);
        }
    }

    pub fn draw_pair_screen(&mut self, g: &mut Gfx) {
        let s = Ui::new(self.draw_w, self.draw_h);
        self.view = s.view;
        if self.art.pair_base.tex.is_some() {
            s.image(g, &self.art.pair_base, 0.0, 0.0);
        } else {
            s.fill(g, 0.0, 0.0, 800.0, 600.0, rgba(0.1, 0.1, 0.2, 0.95));
        }
        s.text_centered(g, 338.0, top(14.0, 22.0), "SELECT THE NEXT PAIR OF GOLFERS", 22.0, black());
        let list = self.club.waiting();
        for (k, &slot) in list.iter().enumerate() {
            let bx = if k & 1 == 1 { 329.0 } else { 0.0 };
            let y = 50.0 + 136.0 * (k / 2) as f32;
            let picked = self.pair_picks.contains(&slot);
            let hovered = self.hover == k as i32;
            let src = if picked {
                272.0
            } else if hovered {
                136.0
            } else {
                0.0
            };
            let fy = if hovered && !picked { y - 1.0 } else { y };
            if self.art.pair_buttons.tex.is_some() {
                s.image_part(g, &self.art.pair_buttons, bx + 6.0, fy, 0.0, src, 329.0, 136.0);
            } else {
                s.fill(g, bx + 6.0, fy, 325.0, 132.0, if picked { rgba(0.8, 0.7, 0.2, 0.6) } else { rgba(0.2, 0.2, 0.5, 0.8) });
            }
            let id = self.club.g[slot].roster.max(0) as usize;
            let p = self.club.roster.get(id).cloned().unwrap_or_default();
            // the head on the card's ball: full strength when hovered or picked, else at 70 % (expression row not decoded: the
            // first)
            let k = if hovered || picked { 1.0 } else { 0.7 };
            let gi = p.male_bit() as usize;
            let head = p.head_index(id);
            if head < 19 {
                let (sx, sy) = ((head / 2) as f32 * 140.0, (head & 1) as f32 * 420.0);
                s.image_part_tint(g, &self.art.halo[gi], bx, y + 4.0, sx, sy, 140.0, 140.0, rgb(k, k, k));
            } else if let Some(img) = self.art.heads[gi].get(head as usize - 19) {
                s.image_part_tint(g, img, bx, y + 4.0, 0.0, 0.0, 140.0, 140.0, rgb(k, k, k));
            }
            s.text_centered(g, bx + 214.0, top(y + 9.0, 17.0), &self.club.name(slot), 17.0, black());
            s.text_centered(g, bx + 262.0, top(y + 40.0, BODY), &p.job, BODY, black());
            s.text_centered(g, bx + 262.0, top(y + 72.0, BODY), &format!("{} years old", self.club.age(slot)), BODY, black());
            s.text_centered(g, bx + 262.0, top(y + 104.0, BODY), self.club.marital(slot), BODY, black());
            // the person's traits, one a line, centred on the card's height
            let words = trait_words(p.traits);
            let y0 = y + 36.0 + 9.0 * (5 - words.len()) as f32;
            for (i, w) in words.iter().enumerate() {
                s.text(g, bx + 146.0, top(y0 + 18.0 * i as f32, BODY), w, BODY, black());
            }
        }
        g.flush();
    }

    // ---- golfers list and golfer card ---------------------------------------------------------------------------------

    /// Golfers listed in the golfers panel: on a hole, or partners of golfers on a hole; newest first.
    pub fn listed_golfers(&self) -> Vec<usize> {
        let c = &self.club;
        (0..SLOTS)
            .map(|i| (c.last_created - i as i32 + SLOTS as i32).rem_euclid(SLOTS as i32) as usize)
            .filter(|&g| {
                let p = c.g[g].partner;
                c.g[g].hole > 0 || (p >= 0 && (p as usize) < SLOTS && c.g[p as usize].partner == g as i32 && c.g[p as usize].hole > 0)
            })
            .collect()
    }

    pub fn golfer_cell(vx: f32, vy: f32) -> Option<usize> {
        if !(236.0..236.0 + 4.0 * 140.0).contains(&vx) || !(470.0..470.0 + 4.0 * 26.0).contains(&vy) {
            return None;
        }
        Some(((vx - 236.0) / 140.0) as usize * 4 + ((vy - 470.0) / 26.0) as usize)
    }

    pub fn golfers_click(&mut self, vx: f32, vy: f32) -> bool {
        if Rect::new(700.0, 572.0, 90.0, 20.0).has(vx, vy) {
            self.golfer_page += 16;
            if self.golfer_page >= self.listed_golfers().len() {
                self.golfer_page = 0;
            }
            return true;
        }
        let Some(k) = Self::golfer_cell(vx, vy) else { return false };
        let list = self.listed_golfers();
        if let Some(&g) = list.get(self.golfer_page + k) {
            let (x, z) = self.units_to_world(self.club.g[g].x, self.club.g[g].y);
            self.cam_x = x;
            self.cam_z = z;
            self.card = Some(g);
        }
        true
    }

    pub fn draw_golfers_panel(&self, g: &mut Gfx, s: &Ui) {
        let list = self.listed_golfers();
        s.text(g, 236.0, 466.0, &format!("Golfers on the course: {}", list.len()), 13.0, rgb(1.0, 1.0, 0.7));
        for (k, &gi) in list.iter().skip(self.golfer_page).take(16).enumerate() {
            let x = 236.0 + 140.0 * (k / 4) as f32;
            let y = 470.0 + 26.0 * (k % 4) as f32;
            let gg = &self.club.g[gi];
            let mut c = rgb(1.0, 1.0, 1.0);
            if gg.fatigue > 160 {
                c = rgb(0.6, 0.6, 0.6);
            }
            if gg.thirst > 16 {
                c = rgb(0.5, 0.6, 1.0);
            }
            if gg.hunger > 16 {
                c = rgb(0.9, 0.5, 0.4);
            }
            if gg.flags & 0x2000_0000 != 0 {
                c = rgb(1.0, 0.3, 0.3);
            }
            let hole = if gg.hole > 0 && gg.hole < 19 { format!("{}", gg.hole) } else { "-".into() };
            if gi & 1 == 1 {
                s.fill(g, x, y + 2.0, 20.0, 20.0, rgba(0.3, 0.3, 0.6, 0.8));
                s.text_centered(g, x + 10.0, y + 17.0, &hole, 12.0, rgb(1.0, 1.0, 0.8));
            }
            if gg.hole > 0 {
                s.text(g, x + 24.0, y + 17.0, &self.club.vip_name(gi), 13.0, c);
                let m = (gg.mood + 2).clamp(1, 10);
                let face = rgb(1.0 - m as f32 / 12.0, 0.3 + m as f32 / 14.0, 0.25);
                s.fill(g, x + 118.0, y + 6.0, 14.0, 14.0, face);
                s.text_centered(g, x + 125.0, y + 18.0, &format!("{}", gg.mood), 10.0, black());
            }
        }
        if list.len() > 16 {
            s.text(g, 700.0, 586.0, "next page >", 12.0, rgb(1.0, 1.0, 0.7));
        }
    }

    /// Is golfer g the player's own pro (the only golfer the card's Customize button works for, docs/DECODE_CARDS3.md 2.2)?
    pub fn is_own_golfer(&self, g: usize) -> bool {
        g < SLOTS && (self.club.g[g].flags & flag::GARY != 0 || g as i32 == self.club.gary)
    }

    /// The card's buttons as drawn now: (id, row, icon x, icon y, hit x, hit y), without the hidden ones.
    fn card_buttons(&self, gi: usize) -> Vec<CardButton> {
        let gg = &self.club.g[gi];
        let story = gg.story != -1;
        let page = story && self.card_ui.story;
        let p = (gg.partner.clamp(0, SLOTS as i32 - 1)) as usize;
        CARD_BUTTONS[page as usize]
            .iter()
            .copied()
            .filter(|b| match b.0 {
                0 => self.is_own_golfer(gi),
                3 => story,
                5 => !(gg.story_step == 4 && self.club.g[p].story_step == 4),
                _ => true,
            })
            .collect()
    }

    fn card_hit(&self, gi: usize, vx: f32, vy: f32) -> i32 {
        self.card_buttons(gi).iter().find(|b| dist(vx - b.4, vy - b.5) < 25.0).map(|b| b.0).unwrap_or(-1)
    }

    /// The golfer card over the course (0x45c560), redrawn every frame while it is open.
    pub fn draw_card(&mut self, g: &mut Gfx, s: &Ui) {
        let Some(gi) = self.card else { return };
        let h = self.club.g[gi].hole;
        if h == 0 || h == -1 {
            // the golfer has gone home or is back in the clubhouse
            self.card = None;
            return;
        }
        // the hover id must stay for 7 frames before its tooltip shows
        let (mx, my) = self.card_ui.mouse;
        let hover = self.card_hit(gi, mx, my);
        if hover != self.card_ui.hover {
            self.card_ui.hover = hover;
            self.card_ui.frames = 0;
        } else {
            self.card_ui.frames += 1;
        }
        let gg = self.club.g[gi].clone();
        let id = gg.roster.max(0) as usize;
        let p = self.club.roster.get(id).cloned().unwrap_or_default();
        let finished = h == 0x13;
        let page = gg.story != -1 && self.card_ui.story;
        // a pro, a VIP or the player's pro shows the read-only skills card first
        if gg.kind != 0 {
            self.draw_skill_card(g, s, gi);
        }
        // the course seen through the card's window, darkened by half (s_GolferStats), then the plate
        let art = &self.art;
        if art.stats.tex.is_some() {
            if !page {
                s.fill(g, 245.0, 168.0, 384.0, 76.0, rgba(0.0, 0.0, 0.0, 0.5));
            }
            s.image_part(g, &art.stats, 236.0, 26.0, 64.0, 33.0, 402.0, 244.0);
            if page {
                s.image_part(g, &art.stats, 236.0, 165.0, 64.0, 300.0, 402.0, 138.0);
            }
        } else {
            s.fill(g, 236.0, 26.0, 402.0, 244.0, rgba(0.92, 0.9, 0.82, 0.95));
        }
        art.ball_head(g, s, p.female(), p.head_index(id), self.club.attitude(gi) as usize, 172.0, -2.0);
        // the identity lines, centred on x 420
        let teal = c15(0x0210);
        let name = if p.job.is_empty() { self.club.vip_name(gi) } else { format!("{} ({})", self.club.vip_name(gi), p.job) };
        s.text_centered(g, 420.0, top(40.0, LARGE), &name, LARGE, teal);
        s.text_centered(g, 420.0, top(62.0, BODY), &format!("{}, age {}", self.club.marital(gi), self.club.age(gi)), BODY, teal);
        s.text_centered(g, 420.0, top(75.0, BODY), &and_list(&trait_words(p.traits)), BODY, teal);
        let opinion = sg_core::thoughts::course_opinion(gg.mood, 0);
        s.text_centered(g, 420.0, top(91.0, BODY), &format!("\"{opinion}\""), BODY, c15(0x4210));
        let likes: Vec<&str> = (0..3).filter(|b| gg.class & (1 << b) != 0).map(|b| CLASS_WORDS[b]).collect();
        s.text_centered(g, 420.0, top(105.0, BODY), &likes.join(", "), BODY, teal);
        // the five bars: the bright full length, then the dark part from the right end
        for (i, (label, w)) in bars(&gg).iter().enumerate() {
            let y = 44.0 + 16.0 * i as f32;
            s.text_centered(g, 586.0, top(y, SMALL), label, SMALL, black());
            let bad = *w > 40;
            s.fill(g, 546.0, y + 10.0, 80.0, 4.0, c15(if bad { 0x7d08 } else { 0x23e8 }));
            s.fill(g, 626.0 - *w as f32, y + 10.0, *w as f32, 4.0, c15(if bad { 0x6000 } else { 0x1284 }));
        }
        // the scorecard: hole numbers, strokes coloured against par, the hole in progress in grey, the total
        let mut total = 0;
        for hole in 1..19 {
            let x = 266.0 + 18.0 * (hole - 1) as f32;
            s.text_centered(g, x, top(132.0, SMALL), &format!("{hole}"), SMALL, black());
            let (k, c) = if hole as i32 == h && gg.strokes > 0 {
                (gg.strokes, c15(0x4210))
            } else {
                let k = gg.card[hole] as i32;
                let par = self.club.holes[hole].par;
                let c = if par == 0 {
                    black()
                } else if k < par - 1 {
                    c15(0x7ff0)
                } else if k < par {
                    c15(0x7d08)
                } else if k > par + 1 {
                    c15(0x4010)
                } else if k > par {
                    c15(0x211f)
                } else {
                    black()
                };
                (k, c)
            };
            if k > 0 {
                total += k;
                s.text_centered(g, x, top(148.0, LARGE), &format!("{k}"), LARGE, c);
            }
        }
        s.text_centered(g, 598.0, top(148.0, LARGE), &format!("{total}"), LARGE, black());
        if page {
            self.draw_story_page(g, s, gi);
        } else {
            self.draw_card_thoughts(g, s, gi, finished);
        }
        // the round buttons: normal, or the next tile of the row when hovered
        let art = &self.art;
        for (bid, row, ix, iy, _, _) in self.card_buttons(gi) {
            let sx = if bid == hover { 600.0 } else { 500.0 };
            s.image_part(g, &art.stats, ix, iy, sx, 50.0 * row as f32, 57.0, 50.0);
        }
        if hover >= 0 && self.card_ui.frames >= 7 {
            let tip = match hover {
                0 => "Customize",
                1 => "Move/Eject Golfer",
                2 => "Take Snapshot",
                3 if page => "Golfer Comments",
                3 => "View Story",
                5 => "Next Chapter",
                _ => "",
            };
            tooltip(g, s, mx, my, tip);
        }
    }

    /// The golfer's latest thoughts in the card's window, oldest at the top (x 248, from y 170, 13 apart); a golfer who has
    /// finished shows the older ones too, the bad ones only.
    fn draw_card_thoughts(&self, g: &mut Gfx, s: &Ui, gi: usize, finished: bool) {
        let gg = &self.club.g[gi];
        let mut y = 170.0;
        let first = if finished { 9 } else { 4 };
        for i in (0..=first).rev() {
            let id = gg.thoughts[i];
            if id == 0 || y > 240.0 {
                continue;
            }
            let c = match gg.args[i] & 0xc000 {
                0x4000 => c15(0x23e8),
                0xc000 => c15(0x7d08),
                _ => c15(0x6318),
            };
            if finished && gg.args[i] & 0xc000 != 0xc000 {
                continue;
            }
            let (text, size, c) = if id == 0x32 {
                // a story line: magenta, small font
                let scene = (gg.args[i] & 15) as i32;
                let choice = ((gg.args[i] >> 4) & 15) as i32;
                let owner = gg.flags & flag::STORY != 0;
                let t = self.club.stories.line(gg.story, scene, choice, owner).trim().to_string();
                (t.replacen("PARTNER", &self.club.name(gi ^ 1), 1).replacen("MYNAME", &self.club.name(gi), 1), SMALL, c15(0x7c1f))
            } else {
                let line = self.club.thought(&self.course, id as u32, (gg.args[i] & 0x3fff) as i32, gi);
                let t = if line.partner { format!("\"{}\"", line.text) } else { line.text };
                (t, BODY, c)
            };
            if text.is_empty() {
                continue;
            }
            // a line wider than the window goes on to the next
            for l in wrap_text(&text, size, 380.0) {
                if y <= 240.0 {
                    s.text(g, 248.0, top(y, size), &l, size, c);
                }
                y += 13.0;
            }
        }
    }

    /// The story page (View Story): the title with the two names over a rule, then each hole's opening line and reply,
    /// stepping right a hole at a time; the card's golfer in magenta, the other in grey. The partner's portrait sits on the
    /// right.
    fn draw_story_page(&self, g: &mut Gfx, s: &Ui, gi: usize) {
        let gg = &self.club.g[gi];
        let p = (gg.partner.clamp(0, SLOTS as i32 - 1)) as usize;
        let pid = self.club.g[p].roster.max(0) as usize;
        let pp = self.club.roster.get(pid).cloned().unwrap_or_default();
        self.art.ball_head(g, s, pp.female(), pp.head_index(pid), self.club.attitude(p) as usize, 552.0, 176.0);
        let title = self.club.stories.title(gg.story);
        let title = title.strip_prefix(' ').unwrap_or(&title);
        let head = format!("'{title}' with {} and {}", self.club.name(gi), self.club.name(p));
        s.text_centered(g, 428.0, top(167.0, SMALL), &head, SMALL, c15(0x03ff));
        s.fill(g, 328.0, 175.0, 200.0, 1.0, c15(0x0210));
        let owner = if gg.flags & flag::STORY != 0 { gi } else { p };
        let other = if owner == gi { p } else { gi };
        let mut y = 179.0;
        for hole in 1..=gg.hole.clamp(0, 18) as usize {
            let stored = self.club.g[other].hole_mood[hole] as i32;
            let scene = self.club.g[owner].story_hole[hole] as i32;
            if stored == 0 || scene == 0 {
                continue;
            }
            let choice = (stored >> 3).clamp(0, scene);
            let x = 248.0 + 10.0 * (hole - 1) as f32;
            for (who, opener) in [(owner, true), (other, false)] {
                let t = self.club.stories.line(gg.story, scene, choice, opener).trim().to_string();
                let t = t.replacen("PARTNER", &self.club.name(who ^ 1), 1).replacen("MYNAME", &self.club.name(who), 1);
                let c = if who == gi { c15(0x7c1f) } else { c15(0x4210) };
                if y < 290.0 {
                    s.text(g, x, top(y, SMALL), &t, SMALL, c);
                }
                y += if opener { 9.0 } else { 10.0 };
            }
        }
    }

    /// The read-only skills card of a pro or a VIP (0x45f0f0 with no points, x offset -50): a narrow frame at (28, 50), the
    /// name, and the ten skill rows, value on the row's oval and name on its bar, grey when the skill is not had.
    fn draw_skill_card(&self, g: &mut Gfx, s: &Ui, gi: usize) {
        let gg = &self.club.g[gi];
        self.art.trans_frame(g, s, 28.0, 50.0, 208.0, 316.0);
        s.text_centered(g, 160.0, top(58.0, LARGE), &self.club.vip_name(gi), LARGE, rgb(1.0, 1.0, 1.0));
        for r in 0..10 {
            let y = 90.0 + 24.0 * r as f32;
            s.image_part(g, &self.art.trans, 32.0, y, 32.0, 100.0, 191.0, 24.0);
            let v = gg.skills[r];
            if v != 0 {
                s.text(g, 37.0, top(y + 7.0, BODY), &skill_value(v), BODY, black());
            }
            s.text(g, 88.0, top(y + 7.0, BODY), SKILL_NAMES[r], BODY, if v != 0 { black() } else { c15(0x4210) });
        }
    }

    /// Where the pointer is, for the card's buttons.
    pub fn card_pointer(&mut self, vx: f32, vy: f32) {
        self.card_ui.mouse = (vx, vy);
    }

    /// A click while the card is open: its buttons, or anywhere on it (kept open). A click elsewhere goes on to the course.
    pub fn card_click(&mut self, vx: f32, vy: f32) -> bool {
        let Some(gi) = self.card else { return false };
        let hit = self.card_hit(gi, vx, vy);
        let p = (self.club.g[gi].partner.clamp(0, SLOTS as i32 - 1)) as usize;
        match hit {
            0 => self.open_customise(gi),
            1 => {
                // pick the golfer up: the next click on the course puts him down (on the clubhouse: sends him home)
                self.card = None;
                self.card_ui.held = Some(gi);
                self.show_toast(&format!("Click where {} should go", self.club.vip_name(gi)));
            }
            2 => {
                self.card = None;
                let (x, z) = self.units_to_world(self.club.g[gi].x, self.club.g[gi].y);
                self.cam_x = x;
                self.cam_z = z;
                for gg in [gi, gi ^ 1] {
                    if self.club.g[gg].timer == 0 {
                        self.club.g[gg].timer = 1;
                    }
                }
                self.ui_sound(0x95);
            }
            3 => self.card_ui.story = !self.card_ui.story,
            4 => self.card = None,
            5 => {
                // Next Chapter: a forced story beat; a failed one costs both a point of mood, and the card closes
                let owner = if self.club.g[gi].flags & flag::STORY != 0 { gi } else { p };
                if self.club.g[gi].flags & flag::STORY != 0 || self.club.g[p].flags & flag::STORY != 0 {
                    if !self.club.story_beat(&mut self.course, &mut self.exe_rng, owner, true) {
                        self.club.g[gi].mood -= 1;
                        self.club.g[p].mood -= 1;
                    }
                    self.club.g[gi].flags |= flag::STORY_BEAT;
                    self.club.g[p].flags |= flag::STORY_BEAT;
                }
                self.card = None;
            }
            _ => {
                let bottom = if self.card_ui.story { 322.0 } else { 287.0 };
                return Rect::new(236.0, 26.0, 402.0, bottom - 26.0).has(vx, vy)
                    || (self.club.g[gi].kind != 0 && Rect::new(28.0, 50.0, 208.0, 316.0).has(vx, vy));
            }
        }
        true
    }

    /// A left click on the course while no tool is busy: a golfer held by Move/Eject is put down there; otherwise the
    /// nearest golfer within reach opens the card, or the pair screen when he is waiting in the clubhouse.
    pub fn golfer_click(&mut self, wx: f32, wz: f32, clubhouse: bool) -> bool {
        if let Some(gi) = self.card_ui.held.take() {
            let gg = &mut self.club.g[gi];
            if gg.hole > 0 && gg.hole < 19 {
                if clubhouse {
                    gg.hole = 19;
                    gg.bx = 0;
                    gg.flags = (gg.flags & !flag::MAY_CART) | flag::LEAVING;
                } else {
                    let k = sg_core::terrain::TILE_SIZE / sg_core::staff::UNIT as f32;
                    gg.x = ((wx + self.terrain.w as f32 * sg_core::terrain::TILE_SIZE * 0.5) / k) as i32;
                    gg.y = ((wz + self.terrain.h as f32 * sg_core::terrain::TILE_SIZE * 0.5) / k) as i32;
                }
            }
            return true;
        }
        let mut best = (70.0f32, None);
        for gi in 0..SLOTS {
            let h = self.club.g[gi].hole;
            if h == 0 {
                continue;
            }
            let (x, z) = self.units_to_world(self.club.g[gi].x, self.club.g[gi].y);
            let d = (x - wx).hypot(z - wz);
            if d < best.0 {
                best = (d, Some(gi));
            }
        }
        let Some(gi) = best.1 else { return false };
        if self.club.g[gi].hole == -1 {
            return self.open_pair_screen();
        }
        if self.card != Some(gi) {
            self.card_ui = CardUi { mouse: self.card_ui.mouse, ..Default::default() };
        }
        self.card = Some(gi);
        true
    }

    // ---- the accomplishments board ------------------------------------------------------------------------------------

    /// After the club's tick: the board opens twenty frames after an accomplishment (0x46e810).
    pub fn board_tick(&mut self) {
        if self.club.award_pending < 0 {
            return;
        }
        if self.club.award_frames == 0 && self.club.award_point.0 >= 0 {
            self.snapshot_due.push((self.club.award_pending as usize, self.club.award_point));
        }
        self.club.award_frames += 1;
        // the board waits until the course view is showing (another screen open would otherwise lose it)
        if self.club.award_frames > 19 && (!self.ui_ok || self.screen == Screen::Play) {
            let id = self.club.award_pending as usize;
            println!("[{:6.1}s] accomplishment: {}", self.sim_time, self.club.award_title(id));
            self.club.award_pending = -1;
            if self.ui_ok {
                self.ui_sound(0x38);
                self.screen = Screen::Board;
            }
        }
    }

    pub fn draw_board(&mut self, g: &mut Gfx) {
        let s = Ui::new(self.draw_w, self.draw_h);
        self.view = s.view;
        if self.art.board.tex.is_some() {
            s.image(g, &self.art.board, 0.0, 0.0);
        } else {
            s.fill(g, 0.0, 0.0, 800.0, 600.0, rgb(0.35, 0.22, 0.12));
        }
        let mut earned: Vec<(usize, sg_core::records::Earned)> =
            self.club.earned.iter().enumerate().filter_map(|(i, e)| e.clone().map(|e| (i, e))).collect();
        earned.sort_by_key(|(_, e)| e.tick);
        let n = earned.len() as f32;
        // photos
        for (k, (id, e)) in earned.iter().enumerate() {
            let x = 20.0 + if k % 2 == 1 { 515.0 } else { 0.0 } + ((35 * k) % 50) as f32;
            let y = 443.0 - 14.0 * k as f32;
            if self.art.tacs.tex.is_some() {
                s.image_part(g, &self.art.tacs, x - 7.0, y - 173.0, 63.0, 251.0, 229.0, 209.0);
            }
            if let Some(img) = self.snapshots.get(id) {
                s.image_part(g, img, x, y - 166.0, 0.0, 0.0, 200.0, 160.0);
            } else {
                s.fill(g, x, y - 166.0, 200.0, 160.0, rgba(0.25, 0.4, 0.25, 0.9));
                s.text_centered(g, x + 100.0, y - 90.0, self.club.award_title(*id), 12.0, rgb(1.0, 1.0, 0.85));
            }
            let day = (e.tick & 0x3ff) * 30 / 1024 + 1;
            let month = MONTHS[((e.tick >> 10) & 7) as usize];
            let year = 2001 + (e.tick >> 13);
            // the exe's caption: course, two spaces, day month year
            s.text(g, x, y, &format!("{}  {day} {month} {year}", e.course), 11.0, rgb(0.13, 0.13, 0.13));
        }
        // the to-do note: the first three not yet earned
        if self.art.tacs.tex.is_some() {
            s.image_part(g, &self.art.tacs, 600.0, 350.0, 58.0, 49.0, 207.0, 178.0);
        }
        let todo: Vec<usize> = (0..22).filter(|&i| self.club.earned[i].is_none()).take(3).collect();
        for (j, id) in todo.iter().enumerate() {
            let t = self.club.award_title(*id);
            s.text(g, 616.0, 420.0 + 28.0 * j as f32, t, 11.0, rgb(0.1, 0.1, 0.4));
        }
        // the trophy: rim, plaques (newest on top), cup body and plate
        if self.art.trophy.tex.is_some() {
            let tr = &self.art.trophy;
            s.image_part(g, tr, 284.0, 304.0 - 14.0 * n, 276.0, 292.0, 257.0, 47.0);
            s.image_part(g, tr, 288.0, 339.0 - 14.0 * n, 9.0, 116.0, 248.0, 30.0);
            s.image_part(g, tr, 288.0, 335.0 - 14.0 * n, 9.0, 116.0, 248.0, 30.0);
            let mut y = 349.0 - 14.0 * n;
            for (k, (id, _)) in earned.iter().rev().enumerate() {
                let alt = self.club.difficulty < 2 && matches!(id, 0 | 1 | 4);
                let (sx, sy) = if alt {
                    let a = match id {
                        0 => 0,
                        1 => 1,
                        _ => 2,
                    };
                    (282.0, 186.0 + 35.0 * a as f32)
                } else if *id <= 10 {
                    (9.0, 221.0 + 35.0 * *id as f32)
                } else {
                    (547.0, 221.0 + 35.0 * (*id - 11) as f32)
                };
                s.image_part(g, tr, 288.0, y, sx, sy, 248.0, 29.0);
                y += 14.0;
                if k == 0 {
                    s.image_part(g, tr, 288.0, y, 9.0, 186.0, 248.0, 19.0);
                    y += 2.0;
                }
            }
            s.image_part(g, tr, 288.0, 351.0, 283.0, 357.0, 248.0, 146.0);
            s.image_part(g, tr, 313.0, 484.0, 317.0, 490.0, 183.0, 52.0);
        } else {
            for (k, (id, _)) in earned.iter().rev().enumerate() {
                s.text_centered(g, 400.0, 340.0 - 16.0 * k as f32, self.club.award_title(*id), 13.0, rgb(1.0, 0.9, 0.4));
            }
        }
        let _ = TITLES;
        g.flush();
    }

    // ---- the year-end report ------------------------------------------------------------------------------------------

    pub fn open_year_end(&mut self) {
        if self.ui_ok && self.screen == Screen::Play {
            self.screen = Screen::YearEnd;
            self.screen_jingle(0x7f, Screen::YearEnd);
        }
    }

    pub fn draw_year_end(&mut self, g: &mut Gfx) {
        let s = Ui::new(self.draw_w, self.draw_h);
        self.view = s.view;
        s.fill(g, 0.0, 0.0, 800.0, 600.0, rgba(0.0, 0.0, 0.0, 0.6));
        let e = &self.art.endo;
        let has = e.tex.is_some();
        if has {
            s.image_part(g, e, 187.0, 40.0, 187.0, 40.0, 429.0, 176.0);
        } else {
            s.fill(g, 187.0, 40.0, 429.0, 176.0, rgb(0.9, 0.88, 0.8));
        }
        let tick = self.club.tick;
        let year = 2000 + (tick >> 13);
        s.text_centered(g, 406.0, 66.0, &format!("END of YEAR {year}"), 22.0, black());
        s.text_centered(g, 465.0, 99.0, "This Year", 12.0, black());
        s.text_centered(g, 566.0, 99.0, "Last Year", 12.0, black());
        let m = ((tick >> 10) % 500) as usize;
        let p = (m + 500 - 8) % 500;
        let h = &self.club.history;
        let get = |i: usize, k: usize| h.get(i).map(|r| r[k]).unwrap_or(0);
        let rows = ["Cash reserves have ", "Your fun rating has ", "Your skill rating has ", "Your membership has "];
        for (k, subject) in rows.iter().enumerate() {
            let y = 125.0 + 20.0 * k as f32;
            let (a, b) = (get(p, k), get(m, k));
            let word = match b.cmp(&a) {
                std::cmp::Ordering::Greater => "increased",
                std::cmp::Ordering::Less => "decreased",
                _ => "not changed",
            };
            let fmt = |v: i32| match k {
                0 => digits(v as i64 * 100),
                2 => format!("{}.{:02}", v / 100, (v % 100).abs()),
                _ => format!("{v}"),
            };
            s.text_centered(g, 301.0, y, &format!("{subject}{word}"), 12.0, black());
            let c = match b.cmp(&a) {
                std::cmp::Ordering::Greater => rgb(0.1, 0.55, 0.2),
                std::cmp::Ordering::Less => rgb(0.8, 0.15, 0.1),
                _ => black(),
            };
            let (la, lb) = (fmt(a), fmt(b));
            s.text(g, 500.0 - text_width(&lb, 12.0), y, &lb, 12.0, c);
            s.text(g, 601.0 - text_width(&la, 12.0), y, &la, 12.0, black());
        }
        s.text_centered(g, 404.0, 210.0, "Highlights", 13.0, black());
        let mut y = 216.0;
        for k in 0..9 {
            let i = (p + k) % 500;
            let v = self.club.event_log.get(i).copied().unwrap_or(0);
            if v == 0 {
                continue;
            }
            let t = self.club.event_text(v);
            if t.is_empty() {
                continue;
            }
            if has {
                s.image_part(g, e, 187.0, y, 187.0, 291.0, 429.0, 15.0);
            } else {
                s.fill(g, 187.0, y, 429.0, 15.0, rgb(0.9, 0.88, 0.8));
            }
            s.text_centered(g, 404.0, y + 12.0, &format!("{}: {t}", MONTHS[k.min(7)]), 11.0, black());
            y += 15.0;
        }
        for l in self.year_notice.lines() {
            if has {
                s.image_part(g, e, 187.0, y, 187.0, 291.0, 429.0, 15.0);
            }
            s.text_centered(g, 404.0, y + 12.0, l, 11.0, rgb(0.8, 0.1, 0.1));
            y += 15.0;
        }
        if has {
            // the bottom piece carries the tick button
            s.image_part(g, e, 187.0, y, 187.0, 374.0, 429.0, 55.0);
        } else {
            s.text_centered(g, 404.0, y + 34.0, "OK", 16.0, black());
        }
        g.flush();
    }

    // ---- the membership roster ----------------------------------------------------------------------------------------

    pub fn draw_roster(&mut self, g: &mut Gfx) {
        let s = Ui::new(self.draw_w, self.draw_h);
        self.view = s.view;
        if self.art.roster.tex.is_some() {
            s.image(g, &self.art.roster, 0.0, 0.0);
        } else {
            s.fill(g, 0.0, 0.0, 800.0, 600.0, rgba(0.9, 0.88, 0.8, 0.97));
        }
        s.text_centered(g, 212.0, 30.0, "Membership Roster", 22.0, black());
        s.text(g, 33.0, 66.0, "Member", 12.0, black());
        s.text_centered(g, 170.0, 66.0, "Low", 11.0, black());
        s.text_centered(g, 209.0, 66.0, "Hcp", 11.0, black());
        s.text_centered(g, 249.0, 66.0, "Rnds", 11.0, black());
        s.text_centered(g, 320.0, 66.0, "Status", 12.0, black());
        for h in 1..19 {
            s.text_centered(g, 390.5 + 21.0 * (h - 1) as f32, 66.0, &format!("{h}"), 10.0, black());
        }
        let mut rows: Vec<usize> = (0..self.club.members.len()).filter(|&r| self.club.members[r].rounds != 0).collect();
        rows.sort_by_key(|&r| self.club.roster.get(r).map(|p| p.name.to_lowercase()).unwrap_or_default());
        let off = self.roster_offset.min(rows.len().saturating_sub(22));
        for (k, &r) in rows.iter().skip(off).take(22).enumerate() {
            let m = &self.club.members[r];
            let y = 89.0 + 20.0 * k as f32 + 12.0;
            let name = self.club.roster.get(r).map(|p| p.name.clone()).unwrap_or_default();
            s.text(g, 28.0, y, &name, 12.0, black());
            let dash = |v: i32| if v <= 0 { "-".to_string() } else { v.to_string() };
            s.text_centered(g, 170.0, y, &dash(m.best as i32), 12.0, black());
            s.text_centered(g, 209.0, y, &dash(m.avg as i32), 12.0, black());
            s.text_centered(g, 249.0, y, &dash(m.rounds), 12.0, black());
            let status = if m.gone == 0xff { "Resigned" } else { LEVELS[(m.level & 7).min(5) as usize] };
            let c = match m.level & 7 {
                3 => rgb(0.45, 0.45, 0.5),
                4 => rgb(0.7, 0.55, 0.0),
                _ if m.gone == 0xff => rgb(0.6, 0.1, 0.1),
                _ => black(),
            };
            s.text_centered(g, 320.0, y, status, 11.0, c);
            for h in 1..19 {
                let b = m.holes[h];
                // the grid's columns are 21 wide from x 380
                let x = 383.0 + 21.0 * (h - 1) as f32;
                if b & 1 != 0 {
                    s.fill(g, x, y - 10.0, 15.0, 10.0, rgb(0.3, 0.3, 0.35));
                } else if b & 2 != 0 {
                    s.fill(g, x, y - 10.0, 15.0, 10.0, rgb(0.85, 0.2, 0.35));
                }
                if b & 4 != 0 {
                    s.fill(g, x + 3.5, y - 8.0, 8.0, 6.0, rgb(0.55, 0.55, 0.55));
                }
            }
        }
        for (x, t) in [
            (82.0, "Member"),
            (208.0, "Silver Member"),
            (326.0, "Gold Member"),
            (450.0, "Resigned"),
            (573.0, "Photo Opp"),
            (693.0, "Happy Ending"),
        ] {
            s.text_centered(g, x, 545.0, t, 11.0, black());
        }
        g.flush();
    }
}

/// A skill's value as the skill cards print it: "+10%" to "+90%", "100%" at ten points.
pub fn skill_value(v: u8) -> String {
    format!("{}{}%", if v < 10 { "+" } else { "" }, v as i32 * 10)
}

/// The generic hover label (0x432620): a small box beside the pointer. Our own drawing.
pub fn tooltip(g: &mut Gfx, s: &Ui, mx: f32, my: f32, t: &str) {
    if t.is_empty() {
        return;
    }
    let w = text_width(t, SMALL) + 10.0;
    let (x, y) = ((mx + 12.0).min(800.0 - w), (my + 16.0).min(580.0));
    s.fill(g, x - 1.0, y - 1.0, w + 2.0, 18.0, rgb(0.1, 0.1, 0.2));
    s.fill(g, x, y, w, 16.0, rgb(1.0, 1.0, 0.86));
    s.text(g, x + 5.0, y + 12.0, t, SMALL, black());
}

/// Money as the info screens print it: digits with thousands commas, no currency sign.
pub fn digits(v: i64) -> String {
    format!("{}{}", if v < 0 { "-" } else { "" }, crate::ui::group(v.unsigned_abs()))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn card_words_and_values() {
        assert_eq!(and_list(&["a"]), "a");
        assert_eq!(and_list(&["a", "b", "c"]), "a, b and c");
        assert_eq!(trait_words(0x15), vec!["Talkative", "Athletic", "Nice"]);
        assert_eq!(skill_value(5), "+50%");
        assert_eq!(skill_value(10), "100%");
        assert_eq!(dist(10.0, 4.0), 12.0);
        assert_eq!(dist(4.0, 10.0), 12.0);
        let mut g = sg_core::golfer::Golfer::default();
        g.args[..6].copy_from_slice(&[0x4001, 0xc002, 0x4003, 0x8004, 0x4005, 0xc000]);
        assert_eq!(trend(&g), 2);
        g.mood = 10;
        assert_eq!(bars(&g)[0].1, 0);
        g.mood = -3;
        assert_eq!(bars(&g)[0].1, 80);
        assert_eq!(bars(&g)[1].1, 20);
    }
}
