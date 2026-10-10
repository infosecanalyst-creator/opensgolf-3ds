//! The world map (property chooser, 0x46f550) and the TRACTS FOR SALE screen (0x4587a0), on the disc's art. Layouts follow
//! docs/UI_SCREENS2.md 3 and 4 and docs/UI_SCREENS.md 10; the exe's text positions are the tops of the text, ours are baselines.

use crate::app::*;
use crate::gfx::Gfx;
use crate::info_ui::{c15, text_right};
use crate::ui::{money, rgb, rgba, text_width, wrap_text, Screen as Ui};
use sg_core::land::{self, RECORDS, SLOT_PRICE_UNITS};
use sg_core::properties::{PROPERTIES, START_FUNDS};

/// Card origins on WorldBase.pcx, one per offer slot (cheapest first, down the left arc and then the right).
pub const CARD_POS: [(f32, f32); 16] = [
    (251.0, 13.0),
    (302.0, 64.0),
    (346.0, 115.0),
    (384.0, 166.0),
    (416.0, 220.0),
    (437.0, 277.0),
    (454.0, 337.0),
    (465.0, 398.0),
    (469.0, 458.0),
    (463.0, 520.0),
    (466.0, 13.0),
    (509.0, 64.0),
    (546.0, 115.0),
    (580.0, 166.0),
    (606.0, 220.0),
    (623.0, 277.0),
];

/// Hover codes beside the property indices 0..15.
pub const WORLD_CANCEL: i32 = 100;
pub const WORLD_RESET_SAVE: i32 = 101;
pub const WORLD_LOAD: i32 = 102;

/// A tile's minimap point on the land screen (its diamond's centre).
fn land_xy(a: i32, b: i32) -> (f32, f32) {
    ((a + b) as f32 * 6.0 + 106.0, (b - a) as f32 * 3.0 + 439.0)
}

impl App {
    /// The property shown on card k (offer slot k), its acres and price in money.
    fn card(&self, k: usize) -> (usize, i32, i32) {
        let s = self.offer[k];
        (s.property, s.acres, SLOT_PRICE_UNITS[k] * 100)
    }

    fn world_cash(&self) -> f64 {
        if self.world_move {
            self.econ.cash
        } else {
            START_FUNDS as f64
        }
    }

    fn world_unlimited(&self) -> bool {
        if self.world_move {
            self.econ.sandbox
        } else {
            self.sandbox_choice
        }
    }

    /// The property the club stands on (only known while moving it).
    fn world_owned(&self, prop: usize) -> bool {
        self.world_move && self.land.as_ref().map(|l| l.slot.property == prop).unwrap_or(false)
    }

    /// Whether a property's price is within the starting funds (or the club's cash when it moves).
    pub fn can_afford(&self, prop: usize) -> bool {
        self.world_unlimited() || self.offer_for(prop).1 as f64 <= self.world_cash()
    }

    /// What is under the pointer: a property (its card's ball or its map pin, the nearest within 30 px), or a button.
    pub fn world_hit(&self, x: f32, y: f32) -> i32 {
        if (x - 768.0).hypot(y - 557.0) < 25.0 {
            return WORLD_CANCEL;
        }
        if (x - 760.0).hypot(y - 476.0) < 25.0 {
            return WORLD_RESET_SAVE;
        }
        if self.world_move && (x - 774.0).hypot(y - 425.0) < 20.0 {
            return WORLD_LOAD;
        }
        let mut best = (-1, 30.0f32);
        for (k, &(cx, cy)) in CARD_POS.iter().enumerate() {
            let p = self.offer[k].property;
            let r = &RECORDS[p];
            for d in [(x - cx - 25.0).hypot(y - cy - 25.0), (x - r.map_x as f32).hypot(y - r.map_y as f32)] {
                if d < best.1 {
                    best = (p as i32, d);
                }
            }
        }
        // the card's text part picks it too
        if best.0 < 0 {
            if let Some(k) = CARD_POS.iter().position(|&(cx, cy)| x >= cx && x < cx + 175.0 && y >= cy && y < cy + 44.0) {
                best.0 = self.offer[k].property as i32;
            }
        }
        best.0
    }

    /// A click on the world map. Returns false when the chooser is left for the title menu or the course.
    pub fn world_click(&mut self, g: &mut Gfx, hit: i32) {
        if self.info.world_go.is_some() {
            return;
        }
        if self.info.world_msg.take().is_some() {
            return;
        }
        match hit {
            WORLD_CANCEL => {
                self.screen = if std::mem::take(&mut self.world_move) { Screen::Play } else { Screen::Menu };
                self.hover = -1;
            }
            WORLD_RESET_SAVE if self.world_move => {
                let f = self.game_file();
                match self.save_game(&f) {
                    Ok(()) => _ = self.post_message("Game Saved.", 1, -4),
                    Err(e) => self.show_toast(&format!("Could not save the game: {e}")),
                }
            }
            WORLD_RESET_SAVE => {
                // Reset World deals a new list
                self.deal_offer(self.sandbox_choice);
                self.ui_sound(0x38);
            }
            WORLD_LOAD => {
                let f = self.game_file();
                if let Err(e) = self.load_game(g, &f) {
                    self.show_toast(&format!("Could not load the saved game: {e}"));
                } else {
                    self.world_move = false;
                }
            }
            p if (0..16).contains(&p) => {
                let p = p as usize;
                if self.world_owned(p) {
                    self.screen = Screen::Play;
                    self.world_move = false;
                } else if !self.can_afford(p) {
                    self.info.world_msg = Some("You need more money before you can purchase this property.".into());
                    self.ui_sound(0x18);
                } else {
                    // "We're off to ...!" shows for a moment before the new course is built
                    self.info.world_go = Some((p, 0));
                    let sound = match RECORDS[p].theme {
                        0 => 0x33,
                        1 => 0x78 + self.deco_rng.below(3),
                        2 => 0x6e + self.deco_rng.below(3),
                        _ => 0x73 + self.deco_rng.below(3),
                    };
                    self.ui_sound(sound);
                }
            }
            _ => {}
        }
    }

    pub fn draw_property(&mut self, g: &mut Gfx) {
        let s = Ui::new(self.draw_w, self.draw_h);
        self.view = s.view;
        if let Some((p, n)) = self.info.world_go {
            if n >= 2 {
                self.info.world_go = None;
                if self.world_move {
                    self.move_to_property(g, p);
                } else {
                    self.start_game(g, p, self.sandbox_choice);
                }
                return;
            }
            self.info.world_go = Some((p, n + 1));
        }
        s.image(g, &self.world_base, 0.0, 0.0);
        let navy = c15(0x2108);
        s.text_centered(g, 126.0, 29.0, "Where will you build", 13.0, navy);
        s.text_centered(g, 126.0, 43.0, "your golf course?", 13.0, navy);
        let funds = if self.world_unlimited() { "Unlimited \u{a7}".to_string() } else { money(self.world_cash() as i64) };
        s.text_centered(g, 740.0, 30.0, &funds, 14.0, navy);
        let a = &self.info.art;
        let blink = (self.clock * 4.0) as i64 % 2 == 0;
        let pin = |g: &mut Gfx, x: f32, y: f32, state: i32| {
            let variant = (x >= 200.0) as i32 + 2 * (y >= 300.0) as i32;
            s.image_part(g, &a.tacks, x - 6.0, y - 18.0, 50.0 * variant as f32, 50.0 * state as f32, 20.0, 24.0);
        };
        // the selected card goes last: its box reaches over the card below
        let sel_card = CARD_POS.iter().enumerate().position(|(k, _)| self.offer[k].property as i32 == self.hover);
        let order = (0..16).filter(|&k| Some(k) != sel_card).chain(sel_card);
        for k in order {
            let (cx, cy) = CARD_POS[k];
            let (p, acres, price) = self.card(k);
            let owned = self.world_owned(p);
            let ok = self.can_afford(p);
            let sel = self.hover == p as i32;
            let theme = PROPERTIES[p].theme;
            let icon = &self.theme_icons[theme];
            if icon.tex.is_some() {
                let (sx, w, h) = match (sel, ok && !owned) {
                    (true, true) => (400.0, 176.0, 66.0),
                    (true, false) => (600.0, 176.0, 66.0),
                    (false, _) if owned => (300.0, 60.0, 49.0),
                    (false, true) => (200.0, 60.0, 49.0),
                    (false, false) => (0.0, 60.0, 49.0),
                };
                s.image_part(g, icon, cx, cy - 2.0, sx, 0.0, w, h);
            }
            s.text_centered(g, cx + 96.0, cy + 15.0, RECORDS[p].name, 13.0, navy);
            s.text_centered(g, cx + 104.0, cy + 26.0, RECORDS[p].bonus, 10.0, navy);
            let rec = &RECORDS[p];
            if sel && ok && !owned {
                let relief = ["flat", "rolling", "hilly"][rec.relief.min(2) as usize];
                let lie = ["inland", "coastal", "island"][rec.coast.min(2) as usize];
                let kind = ["parkland", "desert", "tropical", "links"][rec.theme.min(3) as usize];
                s.text_centered(g, cx + 112.0, cy + 37.0, &format!("Buy {acres} acres of {relief}"), 10.0, navy);
                s.text_centered(g, cx + 106.0, cy + 47.0, &format!("{lie} {kind} property"), 10.0, navy);
                s.text_centered(g, cx + 104.0, cy + 57.0, &format!("for only {}.", money(price as i64)), 10.0, navy);
            } else if self.world_unlimited() {
                s.text_centered(g, cx + 112.0, cy + 40.0, &format!("{acres} acres"), 11.0, navy);
            } else {
                s.text_centered(g, cx + 112.0, cy + 40.0, &format!("{acres} acres: {}", money(price as i64)), 11.0, navy);
            }
            if sel && owned {
                s.text_centered(g, cx + 112.0, cy + 54.0, "Already Purchased.", 10.0, navy);
            }
            let state = if sel && blink {
                3
            } else if owned {
                2
            } else if ok {
                1
            } else {
                0
            };
            pin(g, rec.map_x as f32, rec.map_y as f32, state);
            pin(g, cx + 166.0, cy + 28.0, state);
        }
        // legend
        for (x, st, t) in [(24.0, 1, "Available"), (124.0, 0, "Insufficient funds"), (244.0, 2, "Already purchased")] {
            s.image_part(g, &a.tacks, x, 550.0, 0.0, 50.0 * st as f32, 20.0, 24.0);
            s.text(g, x + 20.0, 570.0, t, 11.0, rgb(0.0, 0.0, 0.0));
        }
        // the buttons: blue, lit under the pointer
        let wb = &a.world_buttons;
        if self.world_move {
            let save = if self.hover == WORLD_RESET_SAVE { 400.0 } else { 550.0 };
            let load = if self.hover == WORLD_LOAD { 400.0 } else { 550.0 };
            s.image_part(g, wb, 724.0, 458.0, save, 458.0, 76.0, 64.0);
            s.image_part(g, wb, 724.0, 408.0, load, 408.0, 76.0, 50.0);
        } else {
            let sx = if self.hover == WORLD_RESET_SAVE { 724.0 } else { 250.0 };
            s.image_part(g, wb, 724.0, 408.0, sx, 408.0, 76.0, 114.0);
        }
        if self.hover == WORLD_CANCEL {
            s.image_part(g, wb, 732.0, 532.0, 732.0, 532.0, 68.0, 68.0);
        }
        let tip = match self.hover {
            WORLD_CANCEL => Some((557.0, "Cancel")),
            WORLD_RESET_SAVE if self.world_move => Some((480.0, "Save Game")),
            WORLD_RESET_SAVE => Some((480.0, "Reset World")),
            WORLD_LOAD => Some((429.0, "Load Game")),
            _ => None,
        };
        if let Some((y, t)) = tip {
            text_right(&s, g, 720.0, y, t, 13.0, navy);
        }
        if let Some(m) = &self.info.world_msg {
            let lines = wrap_text(m, 15.0, 360.0);
            let h = 24.0 + 20.0 * lines.len() as f32;
            s.fill(g, 200.0, 200.0, 400.0, h, rgba(0.45, 0.0, 0.0, 0.95));
            for (i, l) in lines.iter().enumerate() {
                s.text_centered(g, 400.0, 222.0 + 20.0 * i as f32, l, 15.0, rgb(1.0, 1.0, 1.0));
            }
        }
        if let Some((p, _)) = self.info.world_go {
            s.fill(g, 204.0, 276.0, 192.0, 48.0, rgba(0.05, 0.06, 0.2, 0.75));
            let t = format!("We're off to {}!", RECORDS[p].name);
            let size = if text_width(&t, 13.0) > 186.0 { 13.0 * 186.0 / text_width(&t, 13.0) } else { 13.0 };
            s.text_centered(g, 300.0, 296.0, &t, size, rgb(1.0, 1.0, 1.0));
            s.text_centered(g, 300.0, 312.0, "... one moment please ...", 11.0, rgb(1.0, 1.0, 1.0));
        }
        g.flush();
    }

    // ---- TRACTS FOR SALE ------------------------------------------------------------------------------------------------

    /// The tract under the pointer, from its bar or from its square on the map (9: the OK tick, -1 nothing).
    pub fn land_hit(&self, x: f32, y: f32) -> i32 {
        if (662.0..726.0).contains(&x) && (533.0..597.0).contains(&y) {
            return 9;
        }
        if (54.0..258.0).contains(&y) {
            let row = ((y - 54.0) / 68.0) as i32;
            let col = [(15.0, 262.0), (272.0, 524.0), (538.0, 783.0)].iter().position(|&(l, r)| (l..r).contains(&x));
            if let (Some(col), true) = (col, row < 3) {
                return row + 3 * col as i32;
            }
        }
        // the map: invert the minimap's diamond
        let (u, v) = ((x - 106.0) / 6.0, (y - 439.0) / 3.0);
        let (a, b) = (((u - v) / 2.0 + 0.5).floor() as i32, ((u + v) / 2.0 + 0.5).floor() as i32);
        if (1..49).contains(&a) && (1..49).contains(&b) {
            return (a - 1) / 16 + 3 * ((b - 1) / 16);
        }
        -1
    }

    /// The exe's TRACTS FOR SALE screen: the art, the hovered tract lit on the diamond, the course drawn over the tracts
    /// it owns (land for sale lets the diamond's tract squares show), the tract bars with their numbered balls and texts,
    /// the cash reserve, the compass and the OK tick.
    pub fn draw_land(&mut self, g: &mut Gfx) {
        let s = Ui::new(self.draw_w, self.draw_h);
        self.view = s.view;
        let a = &self.info.art;
        if a.buy_land.tex.is_some() {
            s.image(g, &a.buy_land, 0.0, 0.0);
        } else {
            s.fill(g, 0.0, 0.0, 800.0, 600.0, rgb(0.45, 0.45, 0.65));
        }
        let theme = self.exe_theme();
        let hover = self.land_hover;
        if (0..9).contains(&hover) {
            let (a0, b0) = sg_core::tracts::origin(hover as usize);
            let (x, _) = land_xy(a0, b0);
            let (_, y) = land_xy(a0 + 16, b0);
            s.image_part(g, &a.land_buttons, x - 6.0, y - 3.0, 1.0, 127.0, 192.0, 95.0);
        }
        if let Some(land) = self.land.as_ref() {
            for ta in 0..land::N {
                for tb in 0..land::N {
                    let t = land.ty[(ta * land::N + tb) as usize];
                    if t == land::T_OUT {
                        continue;
                    }
                    let c = match t {
                        0 | 1 => rgb(0.45, 0.85, 0.4),
                        2 | 3 => rgb(0.35, 0.72, 0.3),
                        7..=9 => rgb(0.9, 0.85, 0.6),
                        13..=16 => rgb(0.12, 0.38, 0.14),
                        17 => rgb(0.25, 0.45, 0.8),
                        21 | 22 => rgb(0.6, 0.45, 0.35),
                        _ => rgb(0.25, 0.55, 0.22),
                    };
                    let (x, y) = land_xy(ta, tb);
                    s.fill(g, x - 6.0, y - 1.5, 12.0, 3.0, c);
                    s.fill(g, x - 3.0, y - 3.0, 6.0, 6.0, c);
                }
            }
            for h in 1..19 {
                let rec = &self.club.holes[h];
                if rec.par == 0 {
                    continue;
                }
                for (ta, tb) in [rec.back, rec.pin] {
                    let (x, y) = land_xy(ta, tb);
                    s.fill(g, x - 2.0, y - 2.0, 4.0, 4.0, rgb(1.0, 1.0, 1.0));
                }
            }
        }
        let navy = c15(0x2108);
        // fonts (0x4587a0): the title in 0x821020 (Klepto 24) centred at (400, 16) in black, the
        // tract numbers in 0x821f28 (Manual SSi 16), the rest in 0x821ee8 (Manual SSi 14): the tract texts left at the box
        // plus (4, 2), "Cash Reserve" left at (548, 263) and the cash centred at (720, 264), both black
        use crate::ui::{F_INFO14, F_INFO16, F_INFO_TITLE};
        s.put_centered(g, F_INFO_TITLE, 400.0, 16.0, "TRACTS FOR SALE", black_ink());
        for i in 0..9usize {
            let tr = self.tracts[i];
            let (col, row) = (i / 3, i % 3);
            // the tract's number on the map
            if tr.oob > 0 {
                let (a0, b0) = sg_core::tracts::origin(i);
                let (x, y) = land_xy(a0 + 8, b0 + 8);
                // PLACEHOLDER: the number's place and colours on the map (the exe centres it 7 left and 4 above a point not
                // identified here)
                s.put_centered(g, F_INFO16, x + 1.0, y - 7.0, &format!("{}", i + 1), rgba(0.0, 0.0, 0.0, 0.6));
                s.put_centered(g, F_INFO16, x, y - 8.0, &format!("{}", i + 1), c15(0x7ff0));
            }
            // the bar's ball: lit for the tract under the pointer, silver once the tract is bought
            let (bx, by) = ([15.0, 272.0, 538.0][col], [54.0, 122.0, 190.0][row]);
            if hover == i as i32 && tr.oob > 0 {
                s.image_part(g, &a.land_buttons, bx, by, 1.0 + 60.0 * i as f32, 1.0, 59.0, 61.0);
            } else if tr.oob == 0 {
                s.image_part(g, &a.land_buttons, bx, by, 1.0 + 60.0 * i as f32, 64.0, 59.0, 61.0);
            }
            let (x0, y0) = ([78.0, 334.0, 600.0][col], 62.0 + 68.0 * row as f32);
            let (tx, ty) = (x0 + 4.0, y0 + 2.0 + 10.0);
            if tr.oob == 0 {
                s.text(g, tx, ty, "Already purchased.", 14.0, navy);
                continue;
            }
            s.text(g, tx, ty, &format!("Buy tract #{}", i + 1), 14.0, navy);
            let name = |t: u8| sg_core::tracts::type_name(t, theme, land::TYPES[(t as usize).min(22)].class == 13);
            let what = format!("{} acres of {}, {}, and {}", tr.oob / 10, name(tr.top[0]), name(tr.top[1]), name(tr.top[2]));
            let lines = wrap_text(&what, 14.0, 165.0);
            for (k, l) in lines.iter().enumerate() {
                s.text(g, tx, ty + 13.0 * (k + 1) as f32, l, 14.0, navy);
            }
            let price = format!("Price: {}", money(tr.price as i64 * 100));
            text_right(&s, g, x0 + 165.0, ty + 13.0 * (lines.len() + 1) as f32, &price, 14.0, navy);
        }
        s.put(g, F_INFO14, 548.0, 263.0, "Cash Reserve", black_ink());
        s.put_centered(g, F_INFO14, 720.0, 264.0, &money(self.econ.cash as i64), black_ink());
        s.image_part(g, &a.land_buttons, 71.0, 533.0, 259.0, 127.0, 72.0, 61.0);
        if hover == 9 {
            s.image_part(g, &a.land_buttons, 662.0, 533.0, 194.0, 127.0, 64.0, 64.0);
            text_right(&s, g, 656.0, 570.0, "I don't think I'll buy any land.", 14.0, rgb(1.0, 1.0, 1.0));
        }
        g.flush();
    }
}

/// The exe's black text colour (0x80000000).
fn black_ink() -> [f32; 4] {
    crate::ui::rgb(0.0, 0.0, 0.0)
}
