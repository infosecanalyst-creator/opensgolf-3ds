//! OpenSGolf extras: a golfer's profile page (extras.rs), from the People screen or the golfer card's Profile button. The
//! golfer's ball portrait and the golfer card's identity lines (name and job, marital status and age, traits, the skills
//! liked), the membership record (the roster's), the round in progress, the golfer's bio (the person record's own, which
//! the original's Customise edits with Update Bio; here Edit bio opens a one-line prompt for it), and everything the
//! golfer has said on the course this game, newest first, in the golfer card's colours for good and bad thoughts.
//!
//! In the look of the port's other own screens (keys_ui). Keys: Up, Down, PageUp, PageDown and the wheel scroll what was
//! said; L locates, C opens the card, B edits the bio; Esc, Enter, a right click or the OK tick go back.
use crate::app::*;
use crate::extras::{draw_arrow, draw_button, wrap, RenameFor};
use crate::gfx::Gfx;
use crate::info_ui::c15;
use crate::render::Rect;
use crate::screens_ui::{and_list, trait_words, CLASS_WORDS};
use crate::ui::{rgba, Screen as Ui, F_ARIAL10, F_MANUAL15, F_MANUAL20};
use sg_core::lang::{tr, trf};

const BOX: Rect = Rect { x: 40.0, y: 36.0, w: 720.0, h: 528.0 };
/// What was said: the list's top and bottom, the text's left edge and width.
const SAID_TOP: f32 = BOX.y + 262.0;
const SAID_BOTTOM: f32 = BOX.y + BOX.h - 66.0;
const SAID_X: f32 = BOX.x + 140.0;
const SAID_W: f32 = 520.0;
const LINE: f32 = 15.0;
const UP: Rect = Rect { x: BOX.x + BOX.w - 44.0, y: SAID_TOP, w: 20.0, h: 20.0 };
const DOWN: Rect = Rect { x: BOX.x + BOX.w - 44.0, y: SAID_BOTTOM - 20.0, w: 20.0, h: 20.0 };
const BUTTON_Y: f32 = BOX.y + BOX.h - 56.0;

fn ok_tick() -> Rect {
    Rect::new(BOX.x + BOX.w - 70.0, BOX.y + BOX.h - 58.0, 50.0, 50.0)
}

fn button_rect(i: usize) -> Rect {
    Rect::new(BOX.x + 24.0 + 92.0 * i as f32, BUTTON_Y, 84.0, 20.0)
}

const BUTTONS: [&str; 3] = ["STR_PEOPLE_LOCATE", "STR_PEOPLE_CARD", "STR_PROFILE_EDIT_BIO"];

/// The page's state: the golfer's person record, the newest thing said shown at the top, and the screen to go back to.
#[derive(Default)]
pub struct ProfileUi {
    pub roster: i32,
    pub off: usize,
    pub back: Option<Screen>,
}

impl App {
    pub fn open_profile(&mut self, roster: i32) {
        if self.screen != Screen::Profile {
            self.xs.profile.back = Some(if self.screen == Screen::Popup { Screen::Play } else { self.screen });
        }
        self.xs.profile.roster = roster;
        self.xs.profile.off = 0;
        self.screen = Screen::Profile;
        self.hover = -1;
    }

    fn close_profile(&mut self) {
        let back = self.xs.profile.back.take().unwrap_or(Screen::Play);
        self.screen = if back == Screen::People { Screen::People } else { Screen::Play };
        self.hover = -1;
    }

    fn profile_said_len(&self) -> usize {
        self.xs.kept.said.get(&self.xs.profile.roster).map(|v| v.len()).unwrap_or(0)
    }

    fn profile_scroll(&mut self, d: i32) {
        let n = self.profile_said_len() as i32;
        self.xs.profile.off = (self.xs.profile.off as i32 + d).clamp(0, (n - 1).max(0)) as usize;
    }

    pub fn profile_wheel(&mut self, y: f32) {
        self.profile_scroll(if y > 0.0 { -2 } else { 2 });
    }

    fn profile_action(&mut self, b: usize) {
        let r = self.xs.profile.roster;
        match b {
            0 | 1 => {
                let Some(g) = self.golfer_of(r) else { return self.ui_sound(0x18) };
                self.xs.profile.back = None;
                self.xs.people.back = None;
                self.screen = Screen::Play;
                self.look_at((self.club.g[g].x, self.club.g[g].y));
                if b == 1 {
                    self.card = Some(g);
                }
            }
            _ => {
                if r < 0 || r as usize >= self.club.roster.len() {
                    return self.ui_sound(0x18);
                }
                self.xs.rename_for = Some(RenameFor::Bio(r));
                self.rename = Some(self.club.roster[r as usize].bio.clone());
            }
        }
    }

    pub fn profile_click(&mut self, x: f32, y: f32, right: bool) {
        if right || ok_tick().has(x, y) || !BOX.has(x, y) {
            return self.close_profile();
        }
        if UP.has(x, y) {
            return self.profile_scroll(-3);
        }
        if DOWN.has(x, y) {
            return self.profile_scroll(3);
        }
        if let Some(b) = (0..BUTTONS.len()).find(|&b| button_rect(b).has(x, y)) {
            self.profile_action(b);
        }
    }

    pub fn profile_key(&mut self, k: miniquad::KeyCode) {
        use miniquad::KeyCode;
        match k {
            KeyCode::Escape | KeyCode::Enter | KeyCode::KpEnter => self.close_profile(),
            KeyCode::Up => self.profile_scroll(-1),
            KeyCode::Down => self.profile_scroll(1),
            KeyCode::PageUp => self.profile_scroll(-6),
            KeyCode::PageDown | KeyCode::Space => self.profile_scroll(6),
            KeyCode::L => self.profile_action(0),
            KeyCode::C => self.profile_action(1),
            KeyCode::B => self.profile_action(2),
            _ => {}
        }
    }

    pub fn draw_profile(&mut self, g: &mut Gfx) {
        let s = Ui::new(self.draw_w, self.draw_h);
        self.view = s.view;
        crate::info_ui::dim(g, &s);
        self.draw_info_frame(g, &s, BOX.x, BOX.y, BOX.w, BOX.h);
        let (white, yellow, navy, grey) = (c15(0x7fff), c15(0x7ff0), c15(0x0018), c15(0x6318));
        let r = self.xs.profile.roster;
        let id = r.max(0) as usize;
        let p = self.club.roster.get(id).cloned().unwrap_or_default();
        let slot = self.golfer_of(r);
        // the portrait: the golfer's face now when on the course, else the neutral one
        let face = slot.map(|g| self.club.attitude(g) as usize).unwrap_or(1);
        if self.art.trans.tex.is_some() {
            self.art.ball_head(g, &s, p.female(), p.head_index(id), face, BOX.x + 20.0, BOX.y + 16.0);
        }
        let x0 = BOX.x + 176.0;
        let name = slot.map(|g| self.club.vip_name(g)).unwrap_or_else(|| p.name.clone());
        s.put_shadowed(g, F_MANUAL20, x0, BOX.y + 18.0, &name, yellow, false);
        let mut lines: Vec<String> = Vec::new();
        if !p.job.is_empty() {
            lines.push(p.job.clone());
        }
        if let Some(gs) = slot {
            lines.push(trf("STR_CARD_MARITAL_AGE", &[("MARITAL", &self.club.marital(gs)), ("AGE", &self.club.age(gs))]));
            let likes: Vec<&str> = (0..3).filter(|b| self.club.g[gs].class & (1 << b) != 0).map(|b| tr(CLASS_WORDS[b])).collect();
            if !likes.is_empty() {
                lines.push(trf("STR_PROFILE_LIKES", &[("LIKES", &likes.join(tr("STR_LIST_SEPARATOR")))]));
            }
        }
        let traits = and_list(&trait_words(p.traits));
        if !traits.is_empty() {
            lines.push(traits);
        }
        if let Some(m) = self.club.members.get(id).filter(|m| m.rounds > 0) {
            let dash = |v: i32| if v <= 0 { "-".to_string() } else { v.to_string() };
            lines.push(trf(
                "STR_PROFILE_RECORD",
                &[("ROUNDS", &m.rounds), ("BEST", &dash(m.best as i32)), ("HCP", &format!("{:+}", m.avg))],
            ));
        }
        match slot {
            Some(gs) => {
                let gg = &self.club.g[gs];
                let opinion = sg_core::thoughts::course_opinion(gg.mood, 0);
                let hole = if (1..19).contains(&gg.hole) { gg.hole.to_string() } else { tr("STR_PEOPLE_LEAVING").to_string() };
                lines.push(trf(
                    "STR_PROFILE_NOW",
                    &[("TYPE", &self.golfer_type(gs)), ("HOLE", &hole), ("SPENT", &crate::ui::money(self.spent_by(gs)))],
                ));
                lines.push(trf("STR_CARD_OPINION", &[("OPINION", &opinion)]));
            }
            None => lines.push(tr("STR_PROFILE_NOT_HERE").to_string()),
        }
        for (k, l) in lines.iter().take(7).enumerate() {
            s.put(g, F_MANUAL15, x0, BOX.y + 46.0 + 17.0 * k as f32, l, navy);
        }
        // the bio
        let by = BOX.y + 170.0;
        s.put_shadowed(g, F_MANUAL15, BOX.x + 24.0, by, tr("STR_PROFILE_BIO"), yellow, false);
        let bio = p.bio.trim();
        let bio_lines = if bio.is_empty() { vec![tr("STR_PROFILE_NO_BIO").to_string()] } else { wrap(F_MANUAL15, bio, 600.0) };
        for (k, l) in bio_lines.iter().take(4).enumerate() {
            s.put(g, F_MANUAL15, SAID_X - 40.0, by + LINE * k as f32, l, if bio.is_empty() { grey } else { white });
        }
        // what the golfer has said, newest first
        s.put_shadowed(g, F_MANUAL15, BOX.x + 24.0, SAID_TOP - 20.0, &trf("STR_PROFILE_SAID", &[("GOLFER", &name)]), yellow, false);
        s.fill(g, BOX.x + 24.0, SAID_TOP - 3.0, UP.x - BOX.x - 32.0, 1.0, rgba(0.0, 0.0, 0.3, 0.4));
        let said = self.xs.kept.said.get(&r).cloned().unwrap_or_default();
        if said.is_empty() {
            s.put(g, F_MANUAL15, SAID_X, SAID_TOP + 4.0, tr("STR_PROFILE_NOTHING_SAID"), grey);
        }
        let mut y = SAID_TOP + 2.0;
        let mut shown = 0;
        for sd in said.iter().rev().skip(self.xs.profile.off) {
            let ls = wrap(F_MANUAL15, &sd.text, SAID_W);
            let h = LINE * ls.len() as f32 + 3.0;
            if y + h > SAID_BOTTOM && shown > 0 {
                break;
            }
            s.put(g, F_ARIAL10, BOX.x + 24.0, y + 3.0, &crate::extras::date(sd.tick), navy);
            // the golfer card's colours, darkened for the box's blue: green a good thought, red a bad one, else navy
            let c = match sd.tone {
                1 => c15(0x01e0),
                2 => c15(0x5800),
                _ => navy,
            };
            for (k, l) in ls.iter().enumerate() {
                s.put(g, F_MANUAL15, SAID_X, y + LINE * k as f32, l, c);
            }
            y += h;
            shown += 1;
        }
        let (px, py) = self.info.pointer;
        let (can_up, can_down) = (self.xs.profile.off > 0, self.xs.profile.off + shown < said.len());
        for (b, up, ok) in [(UP, true, can_up), (DOWN, false, can_down)] {
            draw_button(g, &s, b, "", ok, b.has(px, py));
            draw_arrow(g, &s, b.x + b.w / 2.0, b.y + b.h / 2.0, up, if ok { navy } else { grey });
        }
        for (b, label) in BUTTONS.iter().enumerate() {
            let rr = button_rect(b);
            let ok = b == 2 || slot.is_some();
            draw_button(g, &s, rr, tr(label), ok, rr.has(px, py));
        }
        s.put(g, F_ARIAL10, BOX.x + 24.0, BOX.y + BOX.h - 24.0, tr("STR_PROFILE_HELP"), navy);
        if self.art.trans.tex.is_some() {
            let r = ok_tick();
            s.image_part(g, &self.art.trans, r.x, r.y, 350.0, 140.0, 50.0, 50.0);
        }
        g.flush();
    }
}
