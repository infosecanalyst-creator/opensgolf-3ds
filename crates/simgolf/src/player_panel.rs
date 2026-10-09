//! The Player panel (sheet JoeCoolPanel, dock modes 3 and 4 in the exe): the pro's own panel with Customise Golfer, Practice
//! Round, Play and Begin Tournament, the five shot shape ovals while the pro waits for the aim, and the tabs to the Golfers
//! panel and the Employee panel. Read from the exe's hit test (0x435f00), click (0x436060), draw (0x4362f0), the round's
//! scorecard (0x461110) and the sheet's cuts in the interface loader; docs/DECODE_MENUS.md 6, docs/UI_PANELS.md 7,
//! docs/DECODE_GOLFERCARD.md 4 and docs/DECODE_PANELS.md 8 hold the same facts.
//!
//! Every control has four cuts on the sheet, in the order hover, pressed or selected, disabled and idle; the idle look is baked
//! into the body. Hit codes are the exe's button ids: 0 Customise Golfer, 1 Practice Round, 2 Play, 3 Begin Tournament,
//! 4..8 the shot shapes, 9 the Golfers tab, 10 the Employee panel, -1 nothing.

use crate::app::*;
use crate::gfx::Gfx;
use crate::panels_ui::{blit, hit, tip_bar, Cut, Hit};
use crate::screens_ui::{c15, top, BODY, SMALL};
use crate::ui::{group, text_width, Screen as Ui};
use sg_core::course::{idx, inside};
use sg_core::golfer::game;
use sg_core::pro::{self, SKILL_NAMES};
use sg_core::tournament::OFFERED;

/// The body, cut from the sheet and drawn at the same place.
pub const PLAYER_BODY: Cut = (214.0, 474.0, 586.0, 126.0);

/// Hit regions by button id (0x435f00). A later test wins over an earlier one; the shot shapes (4..8) are tested only while
/// they are shown.
const HITS: [Hit; 11] = [
    hit(273, 559, 20),
    hit(313, 519, 15),
    hit(313, 548, 15),
    hit(313, 580, 15),
    hit(414, 505, 20),
    hit(493, 505, 20),
    hit(572, 505, 20),
    hit(651, 505, 20),
    hit(730, 505, 20),
    hit(286, 490, 15),
    hit(231, 540, 15),
];

/// Where each button's cuts are drawn (the exe's table at 0x4c7ac0).
const POS: [(f32, f32); 11] = [
    (255.0, 542.0),
    (298.0, 503.0),
    (298.0, 535.0),
    (298.0, 565.0),
    (374.0, 485.0),
    (453.0, 485.0),
    (532.0, 485.0),
    (611.0, 485.0),
    (690.0, 485.0),
    (270.0, 475.0),
    (217.0, 522.0),
];

/// The cut states.
const HOVER: usize = 0;
const SELECTED: usize = 1;
const DISABLED: usize = 2;
const IDLE: usize = 3;

/// Cut `state` of button `id`. Buttons 0..3 are 50 pixels apart across, the shot shapes 100 across with one row each (the
/// hover column sits a pixel lower on the sheet), the tabs have a hover cut only.
fn cut(id: usize, state: usize) -> Cut {
    let k = state as f32;
    match id {
        0 => (50.0 * k, 300.0, 42.0, 44.0),
        1 => (50.0 * k, 500.0, 38.0, 32.0),
        2 => (50.0 * k, 535.0, 38.0, 30.0),
        3 => (50.0 * k, 565.0, 38.0, 32.0),
        4..=8 => (100.0 * k, 50.0 * (id - 4) as f32 + (state == 0) as u8 as f32, 80.0, 50.0),
        9 => (0.0, 400.0, 33.0, 33.0),
        _ => (0.0, 450.0, 33.0, 33.0),
    }
}

/// The shot value each oval sets (straight, fade, draw, high backspin, low punch).
const SHOTS: [i32; 5] = [0, -1, 1, 3, 4];

const TIPS: [&str; 11] = [
    "",
    "Practice Round",
    "",
    "Begin Tournament",
    "Straight shot",
    "Fade shot, L to R",
    "Draw shot, R to L",
    "High backspin shot",
    "Low punch shot",
    "Golfers",
    "Hire employee",
];

/// The round's scorecard (0x461110), over the cover cut drawn at (297, 493): hole numbers and pars along the top two rows,
/// then the pro and his partner, one row each, 18 pixels per hole column starting at x 426 for hole 0.
const COVER: Cut = (297.0, 300.0, 503.0, 122.0);
const COVER_AT: (f32, f32) = (297.0, 493.0);
const CARD_X0: f32 = 426.0;
const CARD_DX: f32 = 18.0;
const CARD_ROW0: f32 = 552.0;
const CARD_DY: f32 = 21.0;

impl App {
    /// The Player panel's sheet is on the disc (otherwise the dock falls back to its text list).
    pub fn player_panel_ready(&self) -> bool {
        self.panel_art.player.tex.is_some()
    }

    /// The shot shapes show while the pro waits for the aim. (The exe tests their hits whenever a golfer is selected, mode 3,
    /// and draws them only while its pro aims; the port ties both to the aim.)
    fn shots_shown(&self) -> bool {
        self.club.pro_aiming().is_some()
    }

    pub fn player_hit(&self, px: f32, py: f32) -> i32 {
        let shots = self.shots_shown();
        let mut h = -1;
        for (id, r) in HITS.iter().enumerate() {
            if (4..=8).contains(&id) && !shots {
                continue;
            }
            if r.has(px, py) {
                h = id as i32;
            }
        }
        h
    }

    /// Practice Round, Play and Begin Tournament as the exe's draw shows them: all three disabled while the pro plays.
    fn player_disabled(&self, id: usize) -> bool {
        let c = &self.club;
        if c.gary != -1 {
            return true;
        }
        match id {
            1 => c.next_hole < 2,
            2 => c.game & pro::CHALLENGE == 0 || c.next_hole < 2 || c.challenge_pro == -1,
            3 => c.game & OFFERED == 0,
            _ => false,
        }
    }

    /// The pro's ball lies in trouble (a tile with a lie penalty): the fade, draw and high backspin ovals are drawn dark.
    fn pro_in_trouble(&self) -> bool {
        let Some(g) = self.club.pro_aiming() else { return false };
        let (bx, by) = (self.club.g[g].bx >> 10, self.club.g[g].by >> 10);
        inside(bx, by) && self.course.h(self.course.ty[idx(bx, by)]) > 0
    }

    pub fn draw_player_panel(&mut self, g: &mut Gfx, s: &Ui, h: i32, tip: bool) {
        let im = self.panel_art.player;
        blit(g, s, &im, PLAYER_BODY, PLAYER_BODY.0, PLAYER_BODY.1);
        if h >= 0 {
            let id = h as usize;
            blit(g, s, &im, cut(id, HOVER), POS[id].0, POS[id].1);
        }
        for id in 1..4 {
            if self.player_disabled(id) {
                blit(g, s, &im, cut(id, DISABLED), POS[id].0, POS[id].1);
            }
        }
        if self.club.gary == -1 {
            self.draw_pro_skills(g, s);
        } else if self.shots_shown() {
            let sel = self.club.planner.option;
            let trouble = self.pro_in_trouble();
            for (k, &v) in SHOTS.iter().enumerate() {
                let id = 4 + k;
                // the exe's states: selected, else hover, else idle; fade, draw and high backspin go dark from a bad lie
                let state = if trouble && (1..4).contains(&k) {
                    DISABLED
                } else if sel == v {
                    SELECTED
                } else if h == id as i32 {
                    HOVER
                } else {
                    IDLE
                };
                blit(g, s, &im, cut(id, state), POS[id].0, POS[id].1);
            }
        } else {
            blit(g, s, &im, COVER, COVER_AT.0, COVER_AT.1);
            self.draw_round_card(g, s, self.club.gary as usize);
        }
        if tip && h >= 0 {
            let t = match h {
                2 => match self.club.pros.get(self.club.challenge_pro.max(0) as usize).filter(|_| self.club.challenge_pro != -1) {
                    Some(p) => format!("Play {}", p.name),
                    None => "Play vs. a pro".to_string(),
                },
                _ => TIPS[h as usize].to_string(),
            };
            if !t.is_empty() {
                let (mx, my) = self.pstate.mouse;
                tip_bar(g, s, &t, mx, my - 5.0);
            }
        }
    }

    /// The pro's skills that have points, as "name +N%", white small text in columns of five from (494, 540), 12 apart.
    fn draw_pro_skills(&self, g: &mut Gfx, s: &Ui) {
        let (mut x, mut y) = (494.0, 540.0);
        for (k, name) in SKILL_NAMES.iter().enumerate() {
            let v = self.club.pro_skill[k] as i32;
            if v == 0 {
                continue;
            }
            s.text(g, x, top(y, SMALL), &format!("{name} +{}%", v * 10), SMALL, c15(0x7fff));
            y += 12.0;
            if y > 599.0 {
                x = 641.0;
                y = 540.0;
            }
        }
    }

    /// The scorecard of the pro's round: the kind of round, then the pro (named "... vs.") and his partner with a score per
    /// hole played, coloured against par.
    fn draw_round_card(&self, g: &mut Gfx, s: &Ui, gary: usize) {
        let c = &self.club;
        let white = c15(0x7fff);
        let black = c15(0);
        s.text(g, 410.0, top(519.0, SMALL), "Hole", SMALL, white);
        s.text(g, 414.0, top(534.0, SMALL), "Par", SMALL, white);
        let partner = c.g[gary].partner;
        let vs_pro = partner >= 0 && c.g[partner as usize].vip() == 0x20;
        let (a, b, x) = if !vs_pro {
            (" Practice".to_string(), "  Round".to_string(), 334.0)
        } else if c.game & game::TOURNAMENT == 0 {
            ("Exhibition".to_string(), format!("\u{a7}{}/hole", group(c.wager_level.max(0) as u64 * 2000)), 334.0)
        } else {
            ("Professional".to_string(), "Tournament".to_string(), 325.0)
        };
        s.text(g, x, top(522.0, BODY), &a, BODY, black);
        // the exe draws the wager line from x 320
        s.text(g, if vs_pro && c.game & game::TOURNAMENT == 0 { 320.0 } else { x }, top(533.0, BODY), &b, BODY, black);
        let mut y = CARD_ROW0;
        let mut who = gary as i32;
        while y <= 589.0 && who >= 0 {
            let gi = who as usize;
            let gg = &c.g[gi];
            let mut name = c.name(gi);
            if gi == gary {
                name.push_str(" vs.");
            }
            // a long name drops to the small font, and so do this row's numbers
            let size = if text_width(&name, BODY) > 115.0 { SMALL } else { BODY };
            s.text(g, 320.0, top(y, size), &name, size, black);
            for (i, hole) in c.holes.iter().enumerate().take(19) {
                if hole.par == 0 {
                    continue;
                }
                let cx = CARD_X0 + CARD_DX * i as f32;
                if gi == gary {
                    let grey = c15(0x6318);
                    s.text_centered(g, cx, top(y - 34.0, size), &i.to_string(), size, grey);
                    s.text_centered(g, cx, top(y - 19.0, size), &hole.par.to_string(), size, grey);
                }
                if (i as i32) > gg.hole {
                    continue;
                }
                let (n, col) = if i as i32 == gg.hole {
                    (gg.strokes, 0x4210)
                } else {
                    let n = gg.card[i] as i32;
                    let p = hole.par;
                    let col = if n < p - 1 {
                        0x7d08
                    } else if n < p {
                        0x6000
                    } else if n > p + 1 {
                        0x4010
                    } else if n > p {
                        0x0018
                    } else {
                        0x2108
                    };
                    (n, col)
                };
                s.text_centered(g, cx, top(y, size), &n.to_string(), size, c15(col));
            }
            y += CARD_DY;
            who = if gi == gary { partner } else { -1 };
        }
    }

    /// A click on the Player panel; returns true when it did something.
    pub fn player_click(&mut self, h: i32) -> bool {
        // the exe's own conditions; a click on a button that fails them does nothing
        let c = &self.club;
        let free = c.gary == -1 && c.game & game::REPEAT == 0;
        let practice = free && c.next_hole >= 2;
        let play = practice && c.game & pro::CHALLENGE != 0 && c.challenge_pro != -1;
        let tournament = free && c.game & OFFERED != 0;
        match h {
            // the exe edits the player's own record for its waiting golfer slot; the port opens it on the pro's slot
            0 => self.open_customise(self.club.gary.max(0) as usize),
            1 if practice => self.pro_button(0),
            2 if play => self.pro_button(1),
            3 if tournament => self.begin_tournament(),
            4..=8 => self.club.set_shot_option(SHOTS[h as usize - 4]),
            9 => {
                self.panel = 4;
                self.golfer_page = 0;
            }
            10 => self.panel = 3,
            _ => return false,
        }
        true
    }
}
