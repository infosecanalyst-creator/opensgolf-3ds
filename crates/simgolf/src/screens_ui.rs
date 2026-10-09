//! The clubhouse and record screens: SELECT THE NEXT PAIR OF GOLFERS (clubhouse), the golfers list and the golfer card,
//! the Professional Accomplishments board (F10 and after each accomplishment), the year-end report and the Membership
//! Roster (F9). Positions follow the exe's 800 x 600 layouts (docs/PUBLISHER_EXE_NOTES.md, "The clubhouse and the golfer
//! screens"); the art is the disc's.

use crate::app::*;
use crate::gfx::Gfx;
use crate::render::Rect;
use crate::ui::{load_pcx, load_pcx_alpha, money, rgb, rgba, text_width, Image, Screen as Ui};
use sg_core::golfer::SLOTS;
use sg_core::records::TITLES;

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
}

impl Art {
    pub fn load(g: &mut Gfx, app: &App) -> Art {
        let p = |rel: &str| app.game_path(&format!("Interface/{rel}"));
        let plain = |g: &mut Gfx, rel: &str| load_pcx(g, &p(rel), false, None).unwrap_or_default();
        let alpha = |g: &mut Gfx, rel: &str, a: &str| load_pcx_alpha(g, &p(rel), &p(a)).unwrap_or_default();
        Art {
            pair_base: plain(g, "PairBase.pcx"),
            pair_buttons: load_pcx(g, &p("PairButtons.pcx"), true, None).unwrap_or_default(),
            board: plain(g, "bulletinboard&mantlewood.pcx"),
            trophy: alpha(g, "TROPHYparts.pcx", "TROPHYparts_A.pcx"),
            tacs: alpha(g, "tacs&tees.pcx", "tacs&tees_A.pcx"),
            endo: alpha(g, "infoscreens/ENDoYEAR.pcx", "infoscreens/ENDoYEAR_alpha.pcx"),
            roster: alpha(g, "infoscreens/memberRoster.pcx", "infoscreens/memberRoster_alpha.pcx"),
        }
    }
}

const MONTHS: [&str; 8] = ["March", "April", "May", "June", "July", "August", "September", "October"];
const LEVELS: [&str; 6] = ["", "Visitor", "Member", "Silver Member", "Gold Member", "Platinum Member"];

fn black() -> [f32; 4] {
    rgb(0.05, 0.05, 0.1)
}

/// The golfer card's five bars (a long bar is bad in every row).
fn bars(gg: &sg_core::golfer::Golfer) -> [(&'static str, i32); 5] {
    [
        ("Happiness", ((8 - gg.mood) * 10).clamp(0, 80)),
        ("Attitude", ((4 - gg.momentum.clamp(-4, 4)) * 10).clamp(0, 80)),
        ("Energy", (gg.fatigue / 4).clamp(0, 80)),
        ("Hunger", (gg.hunger * 5 / 2).clamp(0, 80)),
        ("Thirst", (gg.thirst * 5 / 2).clamp(0, 80)),
    ]
}

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

    fn pair_cell(vx: f32, vy: f32) -> i32 {
        if vx < 6.0 || vy < 50.0 || vx >= 6.0 + 2.0 * 334.0 {
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
        s.text_centered(g, 338.0, 32.0, "SELECT THE NEXT PAIR OF GOLFERS", 22.0, rgb(1.0, 1.0, 0.85));
        let list = self.club.waiting();
        if list.is_empty() {
            s.text_centered(g, 338.0, 300.0, "Nobody is waiting in the clubhouse.", 16.0, rgb(1.0, 1.0, 1.0));
        }
        for (k, &slot) in list.iter().enumerate().take(8) {
            let bx = if k & 1 == 1 { 329.0 } else { 0.0 };
            let y = 50.0 + 136.0 * (k / 2) as f32;
            let picked = self.pair_picks.contains(&slot);
            let src = if picked {
                272.0
            } else if self.hover == k as i32 {
                136.0
            } else {
                0.0
            };
            let fy = if self.hover == k as i32 && !picked { y - 1.0 } else { y };
            if self.art.pair_buttons.tex.is_some() {
                s.image_part(g, &self.art.pair_buttons, bx + 6.0, fy, 0.0, src, 329.0, 136.0);
            } else {
                s.fill(g, bx + 6.0, fy, 325.0, 132.0, if picked { rgba(0.8, 0.7, 0.2, 0.6) } else { rgba(0.2, 0.2, 0.5, 0.8) });
            }
            let p = self.club.roster.get(self.club.g[slot].roster.max(0) as usize).cloned().unwrap_or_default();
            s.text_centered(g, bx + 214.0, y + 24.0, &self.club.name(slot), 17.0, black());
            s.text_centered(g, bx + 262.0, y + 52.0, &p.job, 13.0, black());
            s.text_centered(g, bx + 262.0, y + 84.0, &format!("{} years old", self.club.age(slot)), 13.0, black());
            s.text_centered(g, bx + 262.0, y + 116.0, self.club.marital(slot), 13.0, black());
            let skills: Vec<&str> = [(1, "Length"), (2, "Accuracy"), (4, "Imagination")]
                .iter()
                .filter(|(b, _)| self.club.g[slot].class & b != 0)
                .map(|(_, n)| *n)
                .collect();
            for (i, w) in skills.iter().enumerate() {
                s.text(g, bx + 146.0, y + 60.0 + 18.0 * i as f32, w, 12.0, rgb(0.2, 0.2, 0.4));
            }
        }
        s.text_centered(g, 338.0, 585.0, "Pick two golfers to tee off next, then click outside the cards.", 13.0, rgb(0.25, 0.2, 0.45));
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

    /// The golfer card over the course (0x45c560).
    pub fn draw_card(&self, g: &mut Gfx, s: &Ui) {
        let Some(gi) = self.card else { return };
        let gg = &self.club.g[gi];
        if gg.hole == 0 {
            return;
        }
        let (x0, y0) = (230.0, 24.0);
        s.fill(g, x0, y0, 420.0, 262.0, rgba(0.92, 0.9, 0.82, 0.95));
        let p = self.club.roster.get(gg.roster.max(0) as usize).cloned().unwrap_or_default();
        let title = if p.job.is_empty() { self.club.vip_name(gi) } else { format!("{} ({})", self.club.vip_name(gi), p.job) };
        s.text(g, x0 + 12.0, y0 + 22.0, &title, 16.0, black());
        s.text(g, x0 + 12.0, y0 + 40.0, &format!("{}, age {}", self.club.marital(gi), self.club.age(gi)), 13.0, black());
        let likes: Vec<&str> =
            [(1, "length"), (2, "accuracy"), (4, "imagination")].iter().filter(|(b, _)| gg.class & b != 0).map(|(_, n)| *n).collect();
        s.text(g, x0 + 12.0, y0 + 56.0, &format!("Plays to: {}", likes.join(", ")), 12.0, rgb(0.2, 0.2, 0.4));
        let opinion = sg_core::thoughts::course_opinion(gg.mood, 0);
        s.text(g, x0 + 12.0, y0 + 72.0, &format!("\"{opinion}\""), 12.0, rgb(0.35, 0.35, 0.35));
        for (i, (label, v)) in bars(gg).iter().enumerate() {
            let y = y0 + 14.0 + 16.0 * i as f32;
            s.text(g, x0 + 300.0, y + 9.0, label, 11.0, black());
            s.fill(g, x0 + 330.0, y, 80.0, 9.0, rgba(0.6, 0.6, 0.6, 0.6));
            let bad = *v > 40;
            s.fill(g, x0 + 410.0 - *v as f32, y, *v as f32, 9.0, if bad { rgb(0.9, 0.25, 0.2) } else { rgb(0.2, 0.7, 0.3) });
        }
        // scorecard
        for h in 1..19 {
            let x = x0 + 12.0 + 22.0 * (h - 1) as f32;
            s.text_centered(g, x + 8.0, y0 + 108.0, &format!("{h}"), 11.0, rgb(0.3, 0.3, 0.3));
            let k = gg.card[h] as i32;
            let par = self.club.holes[h].par;
            let (txt, c) = if h as i32 == gg.hole && gg.strokes > 0 {
                (format!("{}", gg.strokes), rgb(0.5, 0.5, 0.5))
            } else if k != 0 {
                let c = match k - par {
                    d if d < -1 => rgb(0.8, 0.7, 0.0),
                    -1 => rgb(0.85, 0.15, 0.1),
                    0 => black(),
                    1 => rgb(0.15, 0.25, 0.85),
                    _ => rgb(0.5, 0.1, 0.5),
                };
                (format!("{k}"), c)
            } else {
                (String::new(), black())
            };
            s.text_centered(g, x + 8.0, y0 + 124.0, &txt, 12.0, c);
        }
        let total: i32 = gg.card[1..].iter().map(|&v| v as i32).sum();
        s.text(g, x0 + 12.0, y0 + 142.0, &format!("Total {total}"), 12.0, black());
        // recent thoughts, oldest first
        let mut y = y0 + 162.0;
        for i in (0..5).rev() {
            let id = gg.thoughts[i];
            if id == 0 || id == 0x32 {
                continue;
            }
            let line = self.club.thought(&self.course, id as u32, (gg.args[i] & 0x3fff) as i32, gi);
            if line.text.is_empty() {
                continue;
            }
            let c = match line.tone {
                sg_core::thoughts::Tone::Good => rgb(0.1, 0.5, 0.15),
                sg_core::thoughts::Tone::Bad => rgb(0.75, 0.1, 0.1),
                _ => rgb(0.3, 0.3, 0.3),
            };
            let t = if line.partner { format!("\"{}\"", line.text) } else { line.text };
            s.text(g, x0 + 12.0, y, &t, 12.0, c);
            y += 15.0;
        }
        s.fill(g, x0 + 396.0, y0 + 240.0, 18.0, 18.0, rgba(0.5, 0.2, 0.2, 0.9));
        s.text_centered(g, x0 + 405.0, y0 + 254.0, "X", 12.0, rgb(1.0, 1.0, 1.0));
    }

    /// A click on the card: its close box, or anywhere on it (kept open).
    pub fn card_click(&mut self, vx: f32, vy: f32) -> bool {
        if self.card.is_none() {
            return false;
        }
        if Rect::new(626.0, 264.0, 18.0, 18.0).has(vx, vy) {
            self.card = None;
            return true;
        }
        Rect::new(230.0, 24.0, 420.0, 262.0).has(vx, vy)
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
        if self.club.award_frames > 19 {
            let id = self.club.award_pending as usize;
            println!("[{:6.1}s] accomplishment: {}", self.sim_time, self.club.award_title(id));
            self.club.award_pending = -1;
            if self.ui_ok && self.screen == Screen::Play {
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
            s.text(g, x, y, &format!("{}, {month} {day}, {year}", e.course), 11.0, rgb(0.13, 0.13, 0.13));
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
        s.text_centered(g, 400.0, 592.0, "Click to continue", 12.0, rgb(1.0, 1.0, 0.8));
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
                0 => money(v as i64 * 100),
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
        s.text_centered(g, 400.0, 590.0, "Click to close", 12.0, black());
        g.flush();
    }
}
