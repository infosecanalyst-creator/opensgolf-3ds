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
use sg_core::pro::SKILL_LABELS;
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
    /// memberRoster_buttons (tier balls, hole icons, arrow hover pieces) and memberRoster_scrollbar (the track).
    pub roster_buttons: Image,
    pub roster_scroll: Image,
    /// GolferStats: the golfer card's plates and its round buttons (with its alpha sheet).
    pub stats: Image,
    /// TransPopups: the translucent dialog frame, the skill rows and pads, the balls (with its alpha sheet).
    pub trans: Image,
    /// s_TransPopups: the masks the translucent fills darken through (the dialog frame's 3 x 3 cuts, the strip's square).
    pub trans_shadow: Image,
    /// PopUpIcons: the five 90 x 80 message icons (trophy, star, book, exclamation, laurel ball).
    pub popup_icons: Image,
    /// The stock heads by gender (0 women, 1 men, as `Person::male_bit`): the 140 x 140 ball portraits and the 90 x 120
    /// expression cells; then the custom portraits found in Heads/ (140 x 420 each), in file name order.
    pub halo: [Image; 2],
    pub expr: [Image; 2],
    pub heads: [Vec<Image>; 2],
    /// Where each custom head's 140 x 420 picture comes from: its file and the picture's offset in it (0 for Heads/*.pcx,
    /// after the *PCXFILE marker in a character file). Saving a character copies it (0x437910).
    pub head_src: [Vec<(std::path::PathBuf, usize)>; 2],
    /// Customise Golfer: the backgrounds (women, men), its buttons, the face picker and the preview window.
    pub cust_bg: [Image; 2],
    pub cg: Image,
    pub head_select: Image,
    pub head_body: Image,
    /// InfoButtons (with its alpha sheet): the generic popup's 3 x 3 frame (cuts 0x561810 at (200 + 17 col, 17 row)) and
    /// its checkboxes (cuts 0x561260 at (300 + 50 k, 0), 26 x 26).
    pub info_buttons: Image,
    /// The generic popup's unchosen radio ball: TransPopups' dim ball (400, 300) through the alpha of the lit ball's cell
    /// (300, 300), as 0x46d6e0 pairs colour sprite 0x5678b8[2] with mask 0x56a7b8[0].
    pub radio_dim: Image,
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
        let mut head_src: [Vec<(std::path::PathBuf, usize)>; 2] = [Vec::new(), Vec::new()];
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
                head_src[gender].push((f.clone(), 0));
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
            roster_buttons: load_pcx(g, &p("infoscreens/memberRoster_buttons.pcx"), true, None).unwrap_or_default(),
            roster_scroll: load_pcx(g, &p("infoscreens/memberRoster_scrollbar.pcx"), true, None).unwrap_or_default(),
            stats: alpha(g, "GolferStats.pcx", "GolferStats_A.pcx"),
            trans: alpha(g, "TransPopups.pcx", "TransPopups_A.pcx"),
            trans_shadow: load_pcx(g, &p("s_TransPopups.pcx"), true, None).unwrap_or_default(),
            popup_icons: alpha(g, "PopUpIcons.pcx", "PopUpIcons_A.pcx"),
            halo: [keyed(g, "Heads/golfballhalopage_female.pcx"), keyed(g, "Heads/golfballhalopage_male .pcx")],
            expr: [keyed(g, "Heads/sim_FEMALE_all_expressionsflat.pcx"), keyed(g, "Heads/sim_MALE_all_expressionsflat.pcx")],
            heads,
            head_src,
            cust_bg: [plain(g, "CustGolfBckgrnd.pcx"), plain(g, "CustGlfBckMale.pcx")],
            cg: keyed(g, "Interface/CGButtons.pcx"),
            head_select: keyed(g, "Interface/HeadSelect.pcx"),
            head_body: keyed(g, "Interface/HeadBodyBck.pcx"),
            info_buttons: alpha(g, "InfoButtons.pcx", "InfoButtons_A.pcx"),
            radio_dim: crate::ui::load_pcx_cell(g, &p("TransPopups.pcx"), &p("TransPopups_A.pcx"), (400, 300), (300, 300), 30, 30)
                .unwrap_or_default(),
            body_pcx,
            bodies: HashMap::new(),
        }
    }

    /// The custom head of a character file's embedded portrait (0x437fa0: a head byte of 20 or more takes the next custom
    /// slot of the gender, while there is room below 0x48): its head index, registering the picture on first use.
    pub fn portrait_head(&mut self, g: &mut Gfx, file: &std::path::Path, female: bool) -> Option<u8> {
        let gi = (!female) as usize;
        if let Some(k) = self.head_src[gi].iter().position(|(f, off)| f == file && *off > 0) {
            return Some(19 + k as u8);
        }
        if self.heads[gi].len() >= 0x48 - 19 {
            return None;
        }
        let b = sg_core::fsutil::read_file(file)?;
        let off = b.windows(8).position(|w| w == b"*PCXFILE")? + 8;
        let mut img = sg_core::assets::decode_pcx(&b[off..]).ok().filter(|i| i.w == 140 && i.h == 420)?;
        for p in img.px.as_chunks_mut::<4>().0 {
            if p[0] == 255 && p[1] == 0 && p[2] == 255 {
                p[3] = 0;
            }
        }
        let im = Image { tex: Some(g.texture(&img, false)), w: 140.0, h: 420.0 };
        self.heads[gi].push(im);
        self.head_src[gi].push((file.to_path_buf(), off));
        Some(18 + self.heads[gi].len() as u8)
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

    /// The translucent fill (0x40ca10): 16 x 16 tiles from (x, y) over w x h, each darkening the screen through a mask cut of
    /// s_TransPopups (0x4740f0 with the colour table 0x824148's first part, which halves every channel). The dialog frame
    /// (`frame`) takes the 3 x 3 cuts at (17 col, 17 row) by the tile's place (first, middle, last column and row: the rounded
    /// corners); the ticker strip takes the square at (317, 0) for every tile (object 0x5a5554).
    pub fn trans_fill(&self, g: &mut Gfx, s: &Ui, x: f32, y: f32, w: f32, h: f32, frame: bool) {
        self.shade_fill(g, s, x, y, w, h, frame, 0.5);
    }

    /// `trans_fill` darkening by `a` (1 for the solid black of a frame called with its last argument 1).
    fn shade_fill(&self, g: &mut Gfx, s: &Ui, x: f32, y: f32, w: f32, h: f32, frame: bool, a: f32) {
        let dark = rgba(0.0, 0.0, 0.0, a);
        if self.trans_shadow.tex.is_none() {
            s.fill(g, x, y, w, h, dark);
            return;
        }
        // (a tile that is both first and last takes the last cut, as the exe tests the end second)
        let place = |v: f32, start: f32, end: f32| {
            if v + 16.0 >= end {
                2.0
            } else if v == start {
                0.0
            } else {
                1.0
            }
        };
        let mut tx = x;
        while tx < x + w {
            let mut ty = y;
            while ty < y + h {
                let (sx, sy) = if frame { (17.0 * place(tx, x, x + w), 17.0 * place(ty, y, y + h)) } else { (317.0, 0.0) };
                s.image_part_tint(g, &self.trans_shadow, tx, ty, sx, sy, 16.0, 16.0, dark);
                ty += 16.0;
            }
            tx += 16.0;
        }
    }

    /// The translucent dialog frame (0x40cef0 with TransPopups): a width or height that is not a multiple of 16 grows to the
    /// next 15 mod 16 and the frame moves back by half of what was added; then the fill and the
    /// 3 x 3 border of 16 x 16 pieces (cuts at (17 col, 17 row)): a corner at each end, (w - 17) / 16 top and bottom pieces
    /// from x + 16 when w passes 32, as many side pieces down from y + 16 when h does.
    pub fn trans_frame(&self, g: &mut Gfx, s: &Ui, x: f32, y: f32, w: f32, h: f32) {
        self.frame_box(g, s, x, y, w, h, 0.5);
    }

    /// The same frame filled solid black: the stats cards call 0x40cef0 with a last argument of 1, and footage of the
    /// original shows their inside black over the course and the trophy room alike.
    pub fn solid_frame(&self, g: &mut Gfx, s: &Ui, x: f32, y: f32, w: f32, h: f32) {
        self.frame_box(g, s, x, y, w, h, 1.0);
    }

    fn frame_box(&self, g: &mut Gfx, s: &Ui, x: f32, y: f32, w: f32, h: f32, a: f32) {
        let (x, w) = crate::message_ui::round16(x, w);
        let (y, h) = crate::message_ui::round16(y, h);
        self.shade_fill(g, s, x, y, w, h, true, a);
        if self.trans.tex.is_none() {
            return;
        }
        let t = &self.trans;
        let piece = |g: &mut Gfx, col: usize, row: usize, dx: f32, dy: f32| {
            s.image_part(g, t, dx, dy, 17.0 * col as f32, 17.0 * row as f32, 16.0, 16.0);
        };
        let (r, b) = (x + w - 16.0, y + h - 16.0);
        piece(g, 0, 0, x, y);
        if w - 16.0 > 16.0 {
            for k in 1..=((w as i32 - 17) >> 4) {
                piece(g, 1, 0, x + 16.0 * k as f32, y);
                piece(g, 1, 2, x + 16.0 * k as f32, b);
            }
        }
        if h - 16.0 > 16.0 {
            for k in 1..=((h as i32 - 17) >> 4) {
                piece(g, 0, 1, x, y + 16.0 * k as f32);
                piece(g, 2, 1, r, y + 16.0 * k as f32);
            }
        }
        piece(g, 2, 0, r, y);
        piece(g, 2, 2, r, b);
        piece(g, 0, 2, x, b);
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

/// Take Snapshot's SimFoto (the main frame at 0x4185bb, saved by 0x431d20): a white-bordered photo framed around the golfer
/// and his pair slot, captioned, written to the snapshots folder and held on screen until a click or a key.
#[derive(Clone, Copy, Debug)]
pub struct SimFoto {
    pub slot: usize,
    /// Frames drawn so far (the photo is taken after the golfers' next tick, so their remarks show) and whether the file
    /// has been written.
    pub frames: u32,
    pub saved: bool,
}

const MONTHS: [&str; 8] = ["March", "April", "May", "June", "July", "August", "September", "October"];

/// The board's handwritten to-do strips on tacs&tees, one per accomplishment class (cut table 0x4c2d38, x y w h).
const TODO_LABELS: [(f32, f32, f32, f32); 22] = [
    (400.0, 97.0, 135.0, 22.0),
    (400.0, 133.0, 117.0, 21.0),
    (400.0, 168.0, 125.0, 21.0),
    (400.0, 207.0, 124.0, 18.0),
    (400.0, 239.0, 133.0, 20.0),
    (400.0, 273.0, 135.0, 21.0),
    (400.0, 310.0, 137.0, 20.0),
    (400.0, 341.0, 129.0, 25.0),
    (400.0, 381.0, 133.0, 32.0),
    (400.0, 413.0, 110.0, 22.0),
    (400.0, 448.0, 112.0, 24.0),
    (400.0, 480.0, 177.0, 37.0),
    (400.0, 519.0, 160.0, 35.0),
    (400.0, 556.0, 141.0, 31.0),
    (610.0, 97.0, 129.0, 20.0),
    (610.0, 130.0, 180.0, 36.0),
    (610.0, 168.0, 161.0, 36.0),
    (610.0, 202.0, 144.0, 40.0),
    (610.0, 239.0, 144.0, 33.0),
    (610.0, 274.0, 142.0, 38.0),
    (610.0, 307.0, 144.0, 35.0),
    (610.0, 341.0, 155.0, 24.0),
];
/// The easy editions' strips for classes 0, 1 and 4: first dogleg right, dogleg left and par five hole (entries 22..24).
const TODO_ALT: [(f32, f32, f32, f32); 3] = [(610.0, 381.0, 132.0, 20.0), (610.0, 415.0, 126.0, 20.0), (610.0, 450.0, 109.0, 21.0)];
// the exe shows no status text for Platinum
const LEVELS: [&str; 6] = ["", "Visitor", "Member", "Silver Member", "Gold Member", ""];

/// The five trait words of the person record's byte +0x20 (pointer table 0x4c2864).
pub const TRAIT_WORDS: [&str; 5] = ["Neat", "Outgoing", "Active", "Playful", "Nice"];
/// The three skill class words (table 0x4c2858, "length" first).
pub const CLASS_WORDS: [&str; 3] = ["length", "accuracy", "imagination"];

/// The fonts of the card and the dialogs: heading 0x519928 (Manual SSi Bold 20), body 0x51b360 (Manual SSi Bold 15) and
/// small 0x519fd8 (Arial Bold 10); the sizes pick the faces (see `ui::Fnt`).
pub const LARGE: f32 = crate::ui::F_MANUAL20.px;
pub const BODY: f32 = crate::ui::F_MANUAL15.px;
pub const SMALL: f32 = crate::ui::F_ARIAL10.px;

fn black() -> [f32; 4] {
    rgb(0.05, 0.05, 0.1)
}

/// An exe colour word (15-bit RGB, 0RRRRRGGGGGBBBBB).
pub fn c15(v: u32) -> [f32; 4] {
    rgb(((v >> 10) & 31) as f32 / 31.0, ((v >> 5) & 31) as f32 / 31.0, (v & 31) as f32 / 31.0)
}

/// The exe places text by its top; ours draws from the baseline (exact metrics, see `ui::Fnt`).
pub fn top(y: f32, size: f32) -> f32 {
    crate::ui::top(y, size)
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

/// The golfer card's five bars: the label (the first is the string at 0x4d2120, "Fun") and the dark (unfilled) width; a long
/// dark part is bad in every row.
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
        // fonts (0x459850): the title in 0x821020, the name in 0x821f08, the rest in 0x821ee8
        use crate::ui::{F_INFO14, F_INFO20, F_INFO_TITLE};
        s.put_centered(g, F_INFO_TITLE, 338.0, 14.0, "SELECT THE NEXT PAIR OF GOLFERS", black());
        let list = self.club.waiting();
        // the picked golfers' faces (0x459850): neutral, or with two picked happy when they share more than three of the
        // five traits and angry when they share fewer than two
        let mut expr = 1;
        if let [a, b] = self.pair_picks[..] {
            let t = |s: usize| self.club.roster.get(self.club.g[s].roster.max(0) as usize).map(|p| p.traits).unwrap_or(0);
            let same = 5 - ((t(a) ^ t(b)) & 0x1f).count_ones();
            if same < 2 {
                expr = 2;
            }
            if same > 3 {
                expr = 0;
            }
        }
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
            if self.art.pair_buttons.tex.is_some() {
                s.image_part(g, &self.art.pair_buttons, bx + 6.0, y, 0.0, src, 329.0, 136.0);
            } else {
                s.fill(g, bx + 6.0, y, 325.0, 132.0, if picked { rgba(0.8, 0.7, 0.2, 0.6) } else { rgba(0.2, 0.2, 0.5, 0.8) });
            }
            // a hovered card's head and words sit a pixel higher than its plate
            let y = if hovered && !picked { y - 1.0 } else { y };
            let id = self.club.g[slot].roster.max(0) as usize;
            let p = self.club.roster.get(id).cloned().unwrap_or_default();
            // the head on the card's ball: a picked golfer's at full strength in the pair's expression, the others neutral at
            // 70 %
            let (k, row) = if picked { (1.0, expr) } else { (0.7, 1) };
            let gi = p.male_bit() as usize;
            let head = p.head_index(id);
            if head < 19 {
                let (sx, sy) = ((head / 2) as f32 * 140.0, (head & 1) as f32 * 420.0 + 140.0 * row as f32);
                s.image_part_tint(g, &self.art.halo[gi], bx, y + 4.0, sx, sy, 140.0, 140.0, rgb(k, k, k));
            } else if let Some(img) = self.art.heads[gi].get(head as usize - 19) {
                s.image_part_tint(g, img, bx, y + 4.0, 0.0, 140.0 * row as f32, 140.0, 140.0, rgb(k, k, k));
            }
            s.put_centered(g, F_INFO20, bx + 214.0, y + 9.0, &self.club.name(slot), black());
            s.put_centered(g, F_INFO14, bx + 262.0, y + 40.0, &p.job, black());
            s.put_centered(g, F_INFO14, bx + 262.0, y + 72.0, &format!("{} years old", self.club.age(slot)), black());
            s.put_centered(g, F_INFO14, bx + 262.0, y + 104.0, self.club.marital(slot), black());
            // the person's traits, one a line, centred on the card's height
            let words = trait_words(p.traits);
            let y0 = y + 36.0 + 9.0 * (5 - words.len()) as f32;
            for (i, w) in words.iter().enumerate() {
                s.put(g, F_INFO14, bx + 146.0, y0 + 18.0 * i as f32, w, black());
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
                // a line that would run under the partner's portrait goes on to the next
                for l in wrap_text(&t, SMALL, 548.0 - x) {
                    if y < 290.0 {
                        s.text(g, x, top(y, SMALL), &l, SMALL, c);
                    }
                    y += 9.0;
                }
                y += if opener { 0.0 } else { 1.0 };
            }
        }
    }

    /// The read-only skills card of a pro or a VIP (0x45f0f0 with no points, x offset -50): a narrow frame at (28, 50), the
    /// name, and the ten skill rows, value on the row's oval and name on its bar, grey when the skill is not had.
    fn draw_skill_card(&self, g: &mut Gfx, s: &Ui, gi: usize) {
        let gg = &self.club.g[gi];
        self.art.solid_frame(g, s, 28.0, 50.0, 208.0, 316.0);
        s.text_centered(g, 160.0, top(58.0, LARGE), &self.club.vip_name(gi), LARGE, rgb(1.0, 1.0, 1.0));
        for r in 0..10 {
            let y = 90.0 + 24.0 * r as f32;
            s.image_part(g, &self.art.trans, 32.0, y, 32.0, 100.0, 191.0, 24.0);
            let v = gg.skills[r];
            if v != 0 {
                s.text(g, 37.0, top(y + 7.0, BODY), &skill_value(v), BODY, black());
            }
            s.text(g, 88.0, top(y + 7.0, BODY), SKILL_LABELS[r], BODY, if v != 0 { black() } else { c15(0x4210) });
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
            // Customize (0x41ea4f): the editor on the golfer's own person record, for any golfer; afterwards the golfer
            // takes the record's age and marital byte
            0 => self.open_customise(gi),
            1 => {
                // Move/Eject Golfer (0x41ea32): the golfer is held; the next left click on the course puts him down on that
                // tile, on the clubhouse sends him home (see `golfer_click`), any other click lets go
                self.card = None;
                self.card_ui.held = Some(gi);
            }
            2 => {
                // Take Snapshot (0x41e722): the SimFoto of the golfer and the pair slot; the view goes to the golfer when he
                // is off screen, and both say something now (thought timer 1)
                self.card = None;
                if self.screen_of(self.club.g[gi].x, self.club.g[gi].y).is_none() {
                    let (x, z) = self.units_to_world(self.club.g[gi].x, self.club.g[gi].y);
                    self.cam_x = x;
                    self.cam_z = z;
                }
                for gg in [gi, gi ^ 1] {
                    if self.club.g[gg].timer == 0 {
                        self.club.g[gg].timer = 1;
                    }
                }
                self.simfoto = Some(SimFoto { slot: gi, frames: 0, saved: false });
            }
            3 => self.card_ui.story = !self.card_ui.story,
            4 => self.card = None,
            5 => {
                // Next Chapter: the replier's random reply, then a forced story beat (sg_core::stories::next_chapter)
                let _ = p;
                self.club.next_chapter(&mut self.course, &mut self.exe_rng, gi);
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
            // the exe puts the golfer on the centre of the clicked tile (+0x9e cleared); dropped on the clubhouse he goes
            // home: hole 19, no strokes, mood 0, and an ordinary golfer's membership record marks the hole quit and the
            // member gone for good (outside sandbox play)
            let k = sg_core::terrain::TILE_SIZE / sg_core::staff::UNIT as f32;
            let ux = ((wx + self.terrain.w as f32 * sg_core::terrain::TILE_SIZE * 0.5) / k) as i32;
            let uy = ((wz + self.terrain.h as f32 * sg_core::terrain::TILE_SIZE * 0.5) / k) as i32;
            let unit = sg_core::staff::UNIT;
            let sandbox = self.econ.sandbox;
            let gg = &mut self.club.g[gi];
            gg.x = ux.div_euclid(unit) * unit + unit / 2;
            gg.y = uy.div_euclid(unit) * unit + unit / 2;
            if clubhouse {
                let (id, hole) = (gg.roster.max(0) as usize, gg.hole.clamp(0, 18) as usize);
                let ordinary = gg.kind == 0;
                gg.hole = 19;
                gg.bx = 0;
                gg.strokes = 0;
                gg.mood = 0;
                gg.flags = (gg.flags & !flag::MAY_CART) | flag::LEAVING;
                if !sandbox && ordinary {
                    if let Some(m) = self.club.members.get_mut(id) {
                        m.holes[hole] |= 4;
                        m.gone = 0xff;
                    }
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

    // ---- SimFoto -------------------------------------------------------------------------------------------------------

    /// The SimFoto's inner rectangle (left, top, right, bottom): the screen points of the golfer and his pair slot, widened
    /// to at least 240 x 200 about their middle, grown by 16 and kept within (16..700, 16..500) for the top left and
    /// (100..784, 100..584) for the bottom right. None while the golfer is off screen.
    pub fn simfoto_rect(&self, f: &SimFoto) -> Option<(i32, i32, i32, i32)> {
        let gg = &self.club.g[f.slot];
        let a = self.screen_of(gg.x, gg.y)?;
        let pg = &self.club.g[f.slot ^ 1];
        let b = self.screen_of(pg.x, pg.y).unwrap_or(a);
        let (mut l, mut r) = (a.0.min(b.0) as i32, a.0.max(b.0) as i32);
        let (mut t, mut bt) = (a.1.min(b.1) as i32, a.1.max(b.1) as i32);
        if r - l < 240 {
            let d = (240 - (r - l)) / 2;
            l -= d;
            r += d;
        }
        if bt - t < 200 {
            let d = (200 - (bt - t)) / 2;
            t -= d;
            bt += d;
        }
        Some(((l - 16).clamp(16, 700), (t - 16).clamp(16, 500), (r + 16).clamp(100, 784), (bt + 16).clamp(100, 584)))
    }

    /// The SimFoto's date stamp: month, day and year joined by apostrophes (month (tick >> 10 & 7) + 3, day as the board's).
    pub fn simfoto_date(&self) -> String {
        let t = self.game_tick;
        format!("{}'{}'{}", ((t & 0x1fff) >> 10) + 3, (((t & 0x3ff) * 30) >> 10) + 1, 2001 + self.econ.year_index())
    }

    /// The SimFoto over the course: the white border, the black frame line, "Happy Ending!" over a story pair whose last
    /// chapter is being told, "SimFoto" in the bottom border and the date stamp inside the lower right corner.
    pub fn draw_simfoto(&mut self, g: &mut Gfx, s: &Ui) {
        let Some(mut f) = self.simfoto else { return };
        f.frames += 1;
        self.simfoto = Some(f);
        if f.frames < 2 {
            return;
        }
        let Some((l, t, r, b)) = self.simfoto_rect(&f) else {
            // the golfer never came on screen: nothing to take
            self.simfoto = None;
            return;
        };
        let (l, t, r, b) = (l as f32, t as f32, r as f32, b as f32);
        let white = c15(0x7fff);
        s.fill(g, l - 16.0, t - 16.0, r - l + 32.0, 16.0, white);
        s.fill(g, l - 16.0, t - 16.0, 16.0, b - t + 32.0, white);
        s.fill(g, l - 16.0, b, r - l + 32.0, 16.0, white);
        s.fill(g, r, t - 16.0, 16.0, b - t + 32.0, white);
        let ink = rgb(0.0, 0.0, 0.0);
        s.fill(g, l, t, r - l, 1.0, ink);
        s.fill(g, l, b, r - l, 1.0, ink);
        s.fill(g, l, t, 1.0, b - t, ink);
        s.fill(g, r, t, 1.0, b - t + 1.0, ink);
        let gg = &self.club.g[f.slot];
        if gg.story_step == 4 && gg.thought == 0x32 {
            s.text_centered(g, (l + r) / 2.0, top(t + 4.0, LARGE), "Happy Ending!", LARGE, c15(0x7ff0));
        }
        s.text_centered(g, (l + r) / 2.0, top(b + 4.0, BODY), "SimFoto", BODY, c15(0x6318));
        // footage of the original shows the date stamp with the shadowed call's dark red one pixel below (ui::SHADOW_1)
        let date = self.simfoto_date();
        s.text(g, r - 80.0, top(b - 16.0, BODY) + 1.0, &date, BODY, crate::ui::SHADOW_1);
        s.text(g, r - 80.0, top(b - 16.0, BODY), &date, BODY, c15(0x7ff0));
    }

    /// A click or a key while the SimFoto is held lets it go.
    pub fn simfoto_dismiss(&mut self) -> bool {
        if self.simfoto.is_some_and(|f| f.saved) {
            self.simfoto = None;
            return true;
        }
        false
    }

    // ---- the accomplishments board ------------------------------------------------------------------------------------

    /// After the club's tick: the board opens twenty frames after an accomplishment (0x46e810).
    pub fn board_tick(&mut self) {
        // every accomplishment recorded has its snapshot taken, also those earned together with another
        self.snapshot_due.append(&mut self.club.award_snaps);
        if self.club.award_pending < 0 {
            return;
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
        // the board reloads the snapshot files (taken in another session, or before a saved game was loaded); one that
        // cannot be read is remembered as missing
        for (id, _) in &earned {
            if !self.snapshots.contains_key(id) {
                let file = self.snapshot_dir().join(format!("accomp{id}.png"));
                let tex = sg_core::fsutil::read_file(&file).and_then(|b| sg_core::png::decode_png(&b)).map(|img| g.texture(&img, false));
                self.snapshots.insert(*id, crate::ui::Image { tex, w: 200.0, h: 160.0 });
            }
        }
        earned.sort_by_key(|(_, e)| e.tick);
        let n = earned.len() as f32;
        // the easy editions' plaques and labels for classes 0, 1 and 4 (first dogleg right, dogleg left, par 5 hole)
        let alt = |id: usize| if self.club.difficulty < 2 { [0, 1, 4].iter().position(|&a| a == id) } else { None };
        let tacs = &self.art.tacs;
        let has_tacs = tacs.tex.is_some();
        // photos, oldest first, alternating sides and climbing 14 pixels each: the frame, the snapshot, the pin and the
        // caption (Arial Bold 10, 0x519fd8, blue)
        for (k, (id, e)) in earned.iter().enumerate() {
            let x = 20.0 + if k % 2 == 1 { 515.0 } else { 0.0 } + ((35 * k) % 50) as f32;
            let y = 443.0 - 14.0 * k as f32;
            if has_tacs {
                s.image_part(g, tacs, x - 7.0, y - 173.0, 63.0, 251.0, 229.0, 209.0);
            }
            if let Some(img) = self.snapshots.get(id).filter(|i| i.tex.is_some()) {
                s.image_part(g, img, x, y - 166.0, 0.0, 0.0, 200.0, 160.0);
            } else {
                // opaque, so the card under it does not show through
                s.fill(g, x, y - 166.0, 200.0, 160.0, rgb(0.25, 0.4, 0.25));
                s.text_centered(g, x + 100.0, y - 90.0, self.club.award_title(*id), 12.0, rgb(1.0, 1.0, 0.85));
            }
            // the pin: TacksandArrow cut 8 (100, 0, 20 x 24)
            let tk = &self.info.art.tacks;
            if tk.tex.is_some() {
                s.image_part(g, tk, x + 100.0, y - 179.0, 100.0, 0.0, 20.0, 24.0);
            }
            let day = (e.tick & 0x3ff) * 30 / 1024 + 1;
            let month = MONTHS[((e.tick >> 10) & 7) as usize];
            let year = 2001 + (e.tick >> 13);
            // the exe's caption: course, two spaces, day month year
            crate::ui::set_face(Some(crate::ui::Face::Arial));
            s.text(g, x, top(y, 10.0), &format!("{}  {day} {month} {year}", e.course), 10.0, c15(0x2108));
            crate::ui::set_face(Some(crate::ui::Face::Info));
        }
        // the Clubhouse Notes pad and on it the first three accomplishments still to do, each its handwritten strip from
        // tacs&tees; strip j sits at (616 + o / 2, 400 + o), o growing by the strip's height less 4
        if has_tacs {
            s.image_part(g, tacs, 600.0, 350.0, 58.0, 49.0, 207.0, 178.0);
            let mut o = 0.0;
            for id in (0..22).filter(|&i| self.club.earned[i].is_none()).take(3) {
                let (sx, sy, w, h) = match alt(id) {
                    Some(a) => TODO_ALT[a],
                    None => TODO_LABELS[id],
                };
                s.image_part(g, tacs, 616.0 + (o / 2.0f32).floor(), 400.0 + o, sx, sy, w, h);
                o += h - 4.0;
            }
        } else {
            let todo: Vec<usize> = (0..22).filter(|&i| self.club.earned[i].is_none()).take(3).collect();
            for (j, id) in todo.iter().enumerate() {
                s.text(g, 616.0, 420.0 + 28.0 * j as f32, self.club.award_title(*id), 11.0, rgb(0.6, 0.0, 0.0));
            }
        }
        let tr = &self.art.trophy;
        if tr.tex.is_some() {
            // the two clubs on the mantle, the tee and ball at the lower left and the tick at the lower right
            s.image_part(g, tr, 519.0, 395.0, 510.0, 13.0, 132.0, 162.0);
            s.image_part(g, tr, 177.0, 395.0, 649.0, 13.0, 130.0, 163.0);
        }
        if has_tacs {
            s.image_part(g, tacs, 51.0, 482.0, 51.0, 482.0, 184.0, 80.0);
            s.image_part(g, tacs, 732.0, 511.0, 732.0, 511.0, 53.0, 52.0);
        }
        // the trophy: rim, cap strips, plaques (newest on top, the lower cap under it), cup body and foot; it rises 14
        // pixels per accomplishment
        if tr.tex.is_some() {
            s.image_part(g, tr, 284.0, 304.0 - 14.0 * n, 276.0, 292.0, 257.0, 47.0);
            s.image_part(g, tr, 288.0, 339.0 - 14.0 * n, 9.0, 116.0, 248.0, 30.0);
            s.image_part(g, tr, 288.0, 335.0 - 14.0 * n, 9.0, 116.0, 248.0, 30.0);
            let mut y = 349.0 - 14.0 * n;
            for (k, (id, _)) in earned.iter().rev().enumerate() {
                let (sx, sy, h) = match alt(*id) {
                    Some(a) => (282.0, 186.0 + 35.0 * a as f32, 30.0),
                    None if *id <= 10 => (9.0, 221.0 + 35.0 * *id as f32, 29.0),
                    None => (547.0, 221.0 + 35.0 * (*id - 11) as f32, 29.0),
                };
                s.image_part(g, tr, 288.0, y, sx, sy, 248.0, h);
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
        // footage of the original: the course under the report keeps about 0.72 of its brightness (the exe's 0.25 dim of the
        // info screens), and the dock, the name plate and the rating pills are gone while the report is up (draw_hud)
        crate::info_ui::dim(g, &s);
        let e = &self.art.endo;
        let has = e.tex.is_some();
        if has {
            s.image_part(g, e, 187.0, 40.0, 187.0, 40.0, 429.0, 176.0);
        } else {
            s.fill(g, 187.0, 40.0, 429.0, 176.0, rgb(0.9, 0.88, 0.8));
        }
        let tick = self.club.tick;
        let year = 2000 + (tick >> 13);
        // fonts (0x44cff0): the title in 0x821020 (Klepto 24) centred at (406, 55), the rest in
        // 0x821ee8 (Manual SSi 14): the column heads centred at 465 and 566 with tops at 93, the rows' sentences centred on
        // 301 and their figures right aligned on 500 and 601, tops 115 + 20 k, "Highlights" centred at (404, 200) and each
        // highlight 2 below its strip
        use crate::ui::{F_INFO14, F_INFO_TITLE};
        // "END of  YEAR: " + year (footage of the original shows the colon and the wide gaps)
        s.put_centered(g, F_INFO_TITLE, 406.0, 55.0, &format!("END of  YEAR:  {year}"), black());
        s.put_centered(g, F_INFO14, 465.0, 93.0, "This Year", black());
        s.put_centered(g, F_INFO14, 566.0, 93.0, "Last Year", black());
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
            s.text_centered(g, 301.0, y, &format!("{subject}{word}"), 14.0, black());
            let c = match b.cmp(&a) {
                // the exe's dark green 0x1284 (footage of the original: about (50, 156, 18) on the strokes)
                std::cmp::Ordering::Greater => crate::info_ui::c15(0x1284),
                std::cmp::Ordering::Less => rgb(0.8, 0.15, 0.1),
                _ => black(),
            };
            let (la, lb) = (fmt(a), fmt(b));
            s.text(g, 500.0 - text_width(&lb, 14.0), y, &lb, 14.0, c);
            s.text(g, 601.0 - text_width(&la, 14.0), y, &la, 14.0, black());
        }
        s.put_centered(g, F_INFO14, 404.0, 200.0, "Highlights", black());
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
            s.text_centered(g, 404.0, y + 12.0, &format!("{}: {t}", MONTHS[k.min(7)]), 14.0, black());
            y += 15.0;
        }
        for l in self.year_notice.lines() {
            if has {
                s.image_part(g, e, 187.0, y, 187.0, 291.0, 429.0, 15.0);
            }
            s.text_centered(g, 404.0, y + 12.0, l, 14.0, rgb(0.8, 0.1, 0.1));
            y += 15.0;
        }
        if has {
            // the bottom piece carries a dark gold tick, which the exe covers with the OkStates cut: blue, lit under the
            // pointer (footage of the original: blue at (550, y + 8) with the pointer away from it)
            s.image_part(g, e, 187.0, y, 187.0, 374.0, 429.0, 55.0);
            self.ok_tick(g, &s, 550.0, y + 8.0, false);
        } else {
            s.text_centered(g, 404.0, y + 34.0, "OK", 16.0, black());
        }
        g.flush();
    }

    // ---- the membership roster (0x454c50) ---------------------------------------------------------------------------

    /// The members listed: every record that has played a round, by name as the exe's strcmp minimum search orders them
    /// (byte order, so capitals first; of equal names the later record comes first).
    pub fn roster_rows(&self) -> Vec<usize> {
        let name = |r: usize| self.club.roster.get(r).map(|p| p.name.as_str()).unwrap_or("");
        let mut rows: Vec<usize> = (0..self.club.members.len()).filter(|&r| self.club.members[r].rounds != 0).collect();
        rows.sort_by(|&a, &b| name(a).as_bytes().cmp(name(b).as_bytes()).then(b.cmp(&a)));
        rows
    }

    /// The roster's live spots: 0 the OK tick (732..775, 548..591), 1 the up arrow and 2 the down arrow of the scroll
    /// column (767..784, 80..116 and 490..526), -1 elsewhere.
    pub fn roster_spot(vx: f32, vy: f32) -> i32 {
        let (x, y) = (vx.floor() as i32, vy.floor() as i32);
        if (732..=775).contains(&x) && (548..=591).contains(&y) {
            0
        } else if (767..=784).contains(&x) && (80..=116).contains(&y) {
            1
        } else if (767..=784).contains(&x) && (490..=526).contains(&y) {
            2
        } else {
            -1
        }
    }

    /// A click on the roster: a left click on the tick closes it, on an arrow scrolls by up to three rows (never past the
    /// last full page), anywhere else does nothing; a right click closes it, as a key does.
    pub fn roster_click(&mut self, vx: f32, vy: f32, right: bool) {
        if right {
            return self.close_info();
        }
        let n = self.roster_rows().len().max(1) as i32;
        let off = self.roster_offset as i32;
        match Self::roster_spot(vx, vy) {
            0 => self.close_info(),
            1 if off > 0 => self.roster_offset = (off - off.clamp(1, 3)) as usize,
            2 if off < n - 22 => self.roster_offset = (off + (n - off - 22).clamp(1, 3)) as usize,
            _ => {}
        }
    }

    pub fn draw_roster(&mut self, g: &mut Gfx) {
        let s = Ui::new(self.draw_w, self.draw_h);
        self.view = s.view;
        crate::info_ui::dim(g, &s);
        if self.art.roster.tex.is_some() {
            s.image(g, &self.art.roster, 0.0, 0.0);
        } else {
            s.fill(g, 0.0, 0.0, 800.0, 600.0, rgba(0.9, 0.88, 0.8, 0.97));
        }
        // the title in the large face (0x821020), the rest in the body face (0x821ee8); all text black; the exe's y are tops
        let ink = c15(0);
        s.text(g, 212.0, top(19.0, 24.0), "Membership Roster", 24.0, ink);
        let row = |y: f32| top(y, 14.0);
        s.text(g, 33.0, row(60.0), "Member", 14.0, ink);
        for (x, t) in [(170.0, "Low"), (209.0, "Hcp"), (249.0, "Rnds"), (320.0, "Status")] {
            s.text_centered(g, x, row(60.0), t, 14.0, ink);
        }
        // the hole numbers centred in 19 pixel cells, 21 apart
        for h in 1..19 {
            s.text_centered(g, 381.0 + 21.0 * (h - 1) as f32 + 9.5, row(60.0), &format!("{h}"), 14.0, ink);
        }
        let rows = self.roster_rows();
        let n = rows.len().max(1);
        let off = self.roster_offset.min(rows.len().saturating_sub(22));
        self.roster_offset = off;
        let bt = &self.art.roster_buttons;
        let has_bt = bt.tex.is_some();
        // the scroll track and its thumb, only with more than a page of members
        if rows.len() > 22 {
            if self.art.roster_scroll.tex.is_some() {
                s.image(g, &self.art.roster_scroll, 767.0, 80.0);
            }
            let ty = (off * 364 / n) as f32;
            let th = (364.0 - ty).min((8008 / n) as f32);
            s.fill(g, 773.0, 122.0 + ty, 6.0, th, c15(0x7fff));
        }
        for (k, &r) in rows.iter().skip(off).take(22).enumerate() {
            let m = &self.club.members[r];
            let y = 89.0 + 20.0 * k as f32;
            let name = self.club.roster.get(r).map(|p| p.name.clone()).unwrap_or_default();
            s.text(g, 28.0, row(y), &name, 14.0, ink);
            // status: the level bits, -1 for a member who resigned for good
            let status = if m.gone == 0xff { -1 } else { (m.level & 7) as i32 };
            if status != 1 && has_bt {
                // navy (Member and no level), silver, gold (and above), red (resigned)
                let ball = match status {
                    -1 => 3,
                    3 => 1,
                    s if s > 3 => 2,
                    _ => 0,
                };
                s.image_part(g, bt, 122.0, y - 4.0, 46.0 + 20.0 * ball as f32, 1.0, 19.0, 18.0);
            }
            let dash = |v: i32| if v <= 0 { "-".to_string() } else { v.to_string() };
            s.text_centered(g, 170.0, row(y), &dash(m.best as i32), 14.0, ink);
            s.text(g, 209.0, row(y), &dash(m.avg as i32), 14.0, ink);
            s.text(g, 249.0, row(y), &dash(m.rounds), 14.0, ink);
            // per hole: the camera (bit 0, story scene) or else the heart (bit 1, happy ending), and the red ball where the
            // member quit (bit 2); odd holes take the cut for the darker checker cell (y 20), even holes the lighter (y 1)
            for h in 1..19 {
                let b = m.holes[h];
                let x = 381.0 + 21.0 * (h - 1) as f32;
                let cy = if h % 2 == 1 { 20.0 } else { 1.0 };
                if b & 3 != 0 && has_bt {
                    let cx = if b & 1 != 0 { 126.0 } else { 146.0 };
                    s.image_part(g, bt, x, y - 4.0, cx, cy, 19.0, 18.0);
                }
                if b & 4 != 0 && has_bt {
                    s.image_part(g, bt, x, y - 4.0, 106.0, cy, 19.0, 18.0);
                }
            }
            let text = match status {
                -1 => "Resigned",
                1..=4 => LEVELS[status as usize],
                _ => "",
            };
            if !text.is_empty() {
                s.text_centered(g, 320.0, row(y), text, 14.0, ink);
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
            s.text_centered(g, x, row(536.0), t, 14.0, ink);
        }
        // the OK tick and, under the pointer, the arrow's lit piece
        self.ok_tick(g, &s, 732.0, 548.0, false);
        let (px, py) = self.info.pointer;
        if has_bt {
            match Self::roster_spot(px, py) {
                1 => s.image_part(g, bt, 767.0, 80.0, 166.0, 1.0, 18.0, 37.0),
                2 => s.image_part(g, bt, 767.0, 490.0, 185.0, 1.0, 18.0, 37.0),
                _ => {}
            }
        }
        g.flush();
    }
}

/// A skill's value as the skill cards print it: "+10%" to "+90%", "100%" at ten points.
pub fn skill_value(v: u8) -> String {
    format!("{}{}%", if v < 10 { "+" } else { "" }, v as i32 * 10)
}

/// The generic hover label (0x432620) at the pointer itself (the card reads the pointer with 0x47ab50 and passes it as
/// is): see `panels_ui::tip_bar`.
pub fn tooltip(g: &mut Gfx, s: &Ui, mx: f32, my: f32, t: &str) {
    if t.is_empty() {
        return;
    }
    crate::panels_ui::tip_bar(g, s, t, mx, my + 5.0);
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
        assert_eq!(trait_words(0x15), vec!["Neat", "Active", "Nice"]);
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
