//! The Golfers panel (sheet MemberPanel, dock mode 2 in the exe): the golfers on the course in pairs, each pair on a card of
//! two name bars with the hole they play on a badge, a mood face after each name, hearts for a pair in a story, a scroll bar,
//! and the round tabs to the Player panel and the Employee panel. Read from the exe's hit test (0x435570), click (0x435680)
//! and draw (0x435760) and the sheet's cuts in the interface loader (0x4477d0); footage of the original shows the same
//! ("Mahatma", "Georgia" on hole 1, "Julie" and "Bobbie" going home, with four hearts).
//!
//! Hit codes: 0..15 the list cells (row + 4 * column), -2 the Golfers tab, -3 the Player tab, -4 the Employee panel, -5 and
//! -6 the scroll arrows, -1 nothing.

use crate::app::*;
use crate::gfx::Gfx;
use crate::info_ui::c15;
use crate::panels_ui::{blit, hit, hit_scaled, tip_bar, Cut, Hit};
use crate::ui::{Screen as Ui, F_ARIAL10, F_MANUAL15};

/// The body, cut from the sheet and drawn at the same place.
pub const MEMBER_BODY: Cut = (215.0, 474.0, 585.0, 126.0);

/// The card of a pair (0x59b778): two name bars and the badge, drawn at the first golfer's row and covering the next.
const CARD: Cut = (0.0, 0.0, 121.0, 44.0);
/// Mood face k (1..10) is cut (594 - 16 (k - 1), 100, 16, 16).
fn face(k: i32) -> Cut {
    (594.0 - 16.0 * (k - 1) as f32, 100.0, 16.0, 16.0)
}
/// A story's hearts: the filled one (0x59b9b4) and the empty one (0x59ba38).
const HEART_FULL: Cut = (450.0, 220.0, 7.0, 7.0);
const HEART_EMPTY: Cut = (450.0, 230.0, 7.0, 7.0);

/// Round tabs and arrows: hit, hover cut and where it is drawn. The Golfers tab (-2) is the current one and has no hover look.
const TAB_GOLFERS: Hit = hit(285, 492, 20);
const TAB_PLAYER: (Hit, Cut, f32, f32) = (hit(256, 510, 20), (0.0, 400.0, 33.0, 33.0), 240.0, 494.0);
const TAB_STAFF: (Hit, Cut, f32, f32) = (hit(232, 538, 20), (0.0, 450.0, 33.0, 33.0), 217.0, 522.0);
const ARROWS: [(Hit, Cut, f32, f32); 2] = [
    (hit_scaled(332, 592, 20, 1, 3), (0.0, 150.0, 36.0, 13.0), 309.0, 587.0),
    (hit_scaled(772, 592, 20, 1, 3), (0.0, 250.0, 36.0, 13.0), 756.0, 587.0),
];

impl App {
    /// The Golfers panel's sheet is on the disc (otherwise the dock falls back to its text list).
    pub fn member_panel_ready(&self) -> bool {
        self.panel_art.member.tex.is_some()
    }

    /// The exe's list layout follows the map zoom (0x4c2844): below 4 a compact list of faces and small names.
    fn member_compact(&self) -> bool {
        self.exe_zoom() < 4.0
    }

    /// 0x435570: the cell grid (4 rows of 21 from y 498, 4 columns of 121 from x 310, clamped) right of x 309 and below
    /// y 497, then the tabs and the arrows, each later test winning.
    pub fn member_hit(&self, px: f32, py: f32) -> i32 {
        let (x, y) = (px.floor() as i32, py.floor() as i32);
        let mut h = -1;
        if x > 309 && y > 497 {
            h = ((y - 498) / 21).clamp(0, 3) + 4 * ((x - 310) / 121).clamp(0, 3);
        }
        if TAB_GOLFERS.has(px, py) {
            h = -2;
        }
        if TAB_PLAYER.0.has(px, py) {
            h = -3;
        }
        if TAB_STAFF.0.has(px, py) {
            h = -4;
        }
        if ARROWS[0].0.has(px, py) {
            h = -5;
        }
        if ARROWS[1].0.has(px, py) {
            h = -6;
        }
        h
    }

    /// The colour of a golfer's name (0x435c62): black, grey when tired (fatigue over 160), blue when thirsty, dark red when
    /// hungry (each over 16), bright red with flag 0x20000000, the later test winning.
    fn member_ink(&self, gi: usize) -> [f32; 4] {
        let gg = &self.club.g[gi];
        let mut c = 0x0000;
        if gg.fatigue > 0xa0 {
            c = 0x4210;
        }
        if gg.thirst > 16 {
            c = 0x0018;
        }
        if gg.hunger > 16 {
            c = 0x6000;
        }
        if gg.flags & 0x2000_0000 != 0 {
            c = 0x7d08;
        }
        c15(c)
    }

    /// 0x435760. The list: the golfers on a hole and partners of golfers on a hole, newest first, from the scroll offset on.
    /// Large layout (zoom 4): four rows of 21 per column, columns 121 apart from x 310 while the name column stays left of
    /// 696; at the first (odd) golfer of a pair the card with its badge (the hole number in Arial Bold 10 at x + 5, or + 8
    /// under 10, y + 15; "x" for a golfer going home, the partner's hole while waiting) and, in a story, four empty hearts
    /// and one filled per story step at (x + 32 + 10 i, y + 17); each golfer on a hole gets the name in Manual SSi Bold 15 at
    /// (x + 16, y + 5) and the mood face clamp(mood + 2, 1, 10) at (x + 102, y + 2). Compact layout: six rows of 14 per
    /// column, columns 60 apart, the face at (x, y) and the name in Arial Bold 10 at (x + 15, y + 1). Then the scroll thumb:
    /// a bar 6 high at y 590 from x 351 over 400 pixels for the 16 (compact 48) entries shown out of the list.
    pub fn draw_member_panel(&mut self, g: &mut Gfx, s: &Ui, h: i32, tip: bool) {
        let im = self.panel_art.member;
        blit(g, s, &im, MEMBER_BODY, MEMBER_BODY.0, MEMBER_BODY.1);
        match h {
            -3 => blit(g, s, &im, TAB_PLAYER.1, TAB_PLAYER.2, TAB_PLAYER.3),
            -4 => blit(g, s, &im, TAB_STAFF.1, TAB_STAFF.2, TAB_STAFF.3),
            -5 | -6 => {
                let a = &ARROWS[(-5 - h) as usize];
                blit(g, s, &im, a.1, a.2, a.3);
            }
            _ => {}
        }
        let list = self.listed_golfers();
        let compact = self.member_compact();
        let (rows, dx, dy, last_x) = if compact { (6, 60.0, 14.0, 755.0) } else { (4, 121.0, 21.0, 695.0) };
        let mut cells = Vec::new();
        for (k, &gi) in list.iter().enumerate().skip(self.golfer_page) {
            let n = k - self.golfer_page;
            let col = (n / rows) as f32;
            let x = 310.0 + dx * col;
            if x + 15.0 > last_x {
                break;
            }
            let y = 498.0 + dy * (n % rows) as f32;
            let gg = &self.club.g[gi];
            let ink = self.member_ink(gi);
            let name = self.club.vip_name(gi);
            let mood = face((gg.mood + 2).clamp(1, 10));
            if compact {
                blit(g, s, &im, mood, x, y);
                s.put(g, F_ARIAL10, x + 15.0, y + 1.0, &name, ink);
            } else {
                if gi & 1 == 1 {
                    blit(g, s, &im, CARD, x, y);
                    let hole = if gg.hole > 0 { gg.hole } else { self.club.g.get(gg.partner.max(0) as usize).map(|p| p.hole).unwrap_or(0) };
                    let badge = if gg.hole == 19 { "x".to_string() } else { hole.to_string() };
                    let bx = x + if gg.hole >= 10 { 5.0 } else { 8.0 };
                    s.put(g, F_ARIAL10, bx, y + 15.0, &badge, c15(0));
                    if gg.story != -1 {
                        for i in 0..4 {
                            blit(g, s, &im, HEART_EMPTY, x + 32.0 + 10.0 * i as f32, y + 17.0);
                        }
                        for i in 0..gg.story_step.max(0) {
                            blit(g, s, &im, HEART_FULL, x + 32.0 + 10.0 * i as f32, y + 17.0);
                        }
                    }
                }
                if gg.hole > 0 {
                    s.put(g, F_MANUAL15, x + 16.0, y + 5.0, &name, ink);
                    blit(g, s, &im, mood, x + 102.0, y + 2.0);
                }
            }
            cells.push(gi);
        }
        self.member_cells = cells;
        let total = list.len();
        let shown = if compact { 48 } else { 16 };
        if let (Some(x0), Some(x1)) = ((self.golfer_page * 400).checked_div(total), ((self.golfer_page + shown) * 400).checked_div(total)) {
            let (x0, x1) = (x0.min(399) as f32, x1.clamp(1, 400) as f32);
            s.fill(g, 351.0 + x0, 590.0, x1 - x0 - 1.0, 6.0, c15(0x739f));
            if total < self.golfer_page {
                self.golfer_page -= 16;
            }
        }
        if tip {
            let t = match h {
                -2 => "Golfers".to_string(),
                -3 => self.pro_name(),
                -4 => "Hire Employees".to_string(),
                _ => return,
            };
            let (mx, my) = self.pstate.mouse;
            tip_bar(g, s, &t, mx, my);
        }
    }

    /// 0x435680: a cell follows its golfer (the camera goes to him; his card opens when he is on screen or a card is open); the tabs and the arrows (16 entries a
    /// step; the right arrow always adds and the draw takes it back while the list is shorter).
    pub fn member_click(&mut self, h: i32) -> bool {
        match h {
            0..=15 => {
                let Some(&gi) = self.member_cells.get(h as usize) else { return false };
                let (x, z) = self.units_to_world(self.club.g[gi].x, self.club.g[gi].y);
                self.cam_x = x;
                self.cam_z = z;
                // 0x435680: the card (0x53df54) opens only when the golfer's screen x (+0x08, set by the map draw, -1
                // while he is not on screen) is known or a card is open already; the camera moves either way
                if self.club.g[gi].sx != -1 || self.card.is_some() {
                    self.card = Some(gi);
                }
            }
            -3 => self.panel = 5,
            -4 => {
                self.pstate.emp_flag = true;
                self.pstate.emp_base = 4;
                self.panel = 3;
            }
            -5 if self.golfer_page != 0 => self.golfer_page = self.golfer_page.saturating_sub(16),
            -6 => self.golfer_page += 16,
            _ => return false,
        }
        true
    }
}
