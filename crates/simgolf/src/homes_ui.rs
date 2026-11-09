//! OpenSGolf extras: home-site details (extras.rs). With the pointer on a home site (no tool in hand, not over the dock), a
//! box in the look of the original's Home Site sums (cursor_ui.rs draw_lot_box: the translucent frame in the lower right,
//! the cursor's shadowed lines in its green and red) tells what the lot is about:
//!
//! * whose home it is (a celebrity's vacation home names the celebrity);
//! * what it cost to build (the site preparation and clearing) and what the lot sold for when it was placed, the only money
//!   a home site brings in (0x41a5d3 sells it at once), and that sale spread over the months since; both are kept from the
//!   moment the extra is on (a site placed before shows a dash);
//! * what a lot here is worth now (a quarter of its value, as a new lot would sell) and what buying it back would cost
//!   (the demolition's price, 0x40e400);
//! * the hole that sets its value (the nearest hole with the most fun, sg_core::homes::value_hole) and how much of the
//!   value it makes.
use crate::app::*;
use crate::gfx::Gfx;
use crate::info_ui::c15;
use crate::screens_ui::BODY;
use crate::ui::Screen as Ui;
use serde::{Deserialize, Serialize};
use sg_core::land;
use sg_core::lang::{tr, trf};

/// A home site placed with the extra on: its anchor tile, the date it was placed, what it cost and what it sold for
/// (dollars).
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, Serialize, Deserialize)]
#[serde(default)]
pub struct Sale {
    pub a: i32,
    pub b: i32,
    pub tick: u32,
    pub cost: i64,
    pub sold: i64,
}

/// The room of a line of the home-site details box (the port's own, issue #164): the 180 pixel box as far as the screen's
/// right edge leaves it (centred on x 710, 84 pixels either side); "Sylvester Stallion's home" is 160 pixels wide.
const DETAILS_ROOM: f32 = 168.0;

/// The room of a line of a home's label on the course (issue #164): a generous one, so only a very long name or translation
/// is fitted ("Sylvester Stallion's" is 121 pixels wide).
const LABEL_ROOM: f32 = 200.0;

/// The most sales kept (a lot is placed on 2 x 2 tiles; no course holds this many).
const MAX_SALES: usize = 400;

/// Money a sale brought in per month since (whole months, at least one).
pub fn per_month(sold: i64, placed: u32, now: u32) -> i64 {
    let months = (now.saturating_sub(placed) >> 10).max(1) as i64;
    sold / months
}

/// "$1,234" (a minus sign before the dollar when below zero).
fn dollars(v: i64) -> String {
    format!("{}${}", if v < 0 { "-" } else { "" }, crate::ui::group(v.unsigned_abs()))
}

impl App {
    /// A home site was placed (edit_building): what it cost and sold for, kept while the extra is on.
    pub fn home_placed(&mut self, a: i32, b: i32, cost: i64, sold: i64) {
        if !crate::extras::home_details() {
            return;
        }
        let tick = self.date();
        let v = &mut self.xs.kept.homes;
        v.retain(|s| (s.a, s.b) != (a, b));
        v.push(Sale { a, b, tick, cost, sold });
        if v.len() > MAX_SALES {
            v.remove(0);
        }
    }

    /// A home site was taken away: its record goes too (whether or not the extra is on, so a new lot there starts afresh).
    pub fn home_removed(&mut self, a: i32, b: i32) {
        self.xs.kept.homes.retain(|s| (s.a, s.b) != (a, b));
    }

    /// The home site under the pointer: its object.
    fn hovered_home(&self) -> Option<land::Object> {
        let (x, y) = self.hover_tile?;
        let l = self.land.as_ref()?;
        l.objects.iter().find(|o| o.kind == land::K_HOME_SITE && (o.a..=o.a + 1).contains(&x) && (o.b..=o.b + 1).contains(&y)).copied()
    }

    /// Each home site's resident, as the exe's main view labels it (0x415c32..0x415ebc): at the closest zoom, over every
    /// occupied lot on the screen, "<name>'s" and "Home" under it, in pink (0x7e9f) Manual SSi 15 with the shadowed centred
    /// call (0x404bc0), centred on the lot; each lot's label shows for 16 of every 64 game ticks, staggered by its record
    /// (`(13 * record + tick) & 0x30 == 0`), so the names take turns (#141). With the home names on hover extra (extras.rs;
    /// the port's own, #160) only the lot under the pointer is labelled, for as long as the pointer stays on it.
    pub fn draw_home_labels(&self, g: &mut Gfx, s: &Ui) {
        let exe_zoom = self.zoom * 4.0 / 0.905;
        if exe_zoom.round() < 4.0 || self.screen != Screen::Play {
            return;
        }
        let Some(l) = self.land.as_ref() else { return };
        let hover = crate::extras::home_labels_hover();
        for (slot, o) in l.objects.iter().enumerate() {
            if o.kind != land::K_HOME_SITE || o.sub <= 0 {
                continue;
            }
            let size = l.footprint_size(o);
            let shown = if hover {
                self.hover_tile.is_some_and(|(x, y)| (o.a..o.a + size.max(1)).contains(&x) && (o.b..o.b + size.max(1)).contains(&y))
            } else {
                (13 * slot as u32).wrapping_add(self.game_tick) & 0x30 == 0
            };
            if !shown {
                continue;
            }
            let Some(c) = self.club.celebrities.get((o.sub - 1) as usize) else { continue };
            let Some((x, y)) = self.screen_of(o.a * 1024 + size * 512, o.b * 1024 + size * 512) else { continue };
            let (x, y) = (x.floor(), y.floor());
            let pink = c15(0x7e9f);
            let owner = trf("STR_HOME_LABEL_OWNER", &[("CELEBRITY", &c.name)]);
            // a line wider than `LABEL_ROOM` (a translation, issue #164): a little smaller or cut (ui.rs fit_line)
            for (t, ty) in [(owner.as_str(), y - 12.0), (tr("STR_HOME_LABEL_HOME"), y)] {
                let (f, t, dy) = crate::ui::fit_line(crate::ui::F_MANUAL15, t, LABEL_ROOM);
                s.put_shadowed(g, f, x, ty + dy, &t, pink, true);
            }
        }
    }

    /// The box for the home site under the pointer (draw_hud, with the name tags).
    pub fn draw_home_details(&self, g: &mut Gfx, s: &Ui) {
        if !crate::extras::home_details() || self.screen != Screen::Play || self.edit || self.card.is_some() {
            return;
        }
        let (px, py) = self.info.pointer;
        // not over the dock or its open panel
        if self.dock_hover >= 0 || (px < 230.0 && py > 430.0) || (self.panel != 0 && px > 220.0 && py > 446.0) {
            return;
        }
        let Some(o) = self.hovered_home() else { return };
        let (green, red, white) = (c15(0x23e8), c15(crate::cursor_ui::RED as u32), c15(crate::cursor_ui::WHITE as u32));
        let value = sg_core::homes::lot_value(&self.course, &self.club.holes, self.difficulty, o.a, o.b);
        let mut lines: Vec<(String, [f32; 4])> = Vec::new();
        let title = match self.club.celebrities.get((o.sub - 1).max(0) as usize).filter(|_| o.sub > 0) {
            Some(c) => trf("STR_HOMES_CELEBRITY", &[("CELEBRITY", &c.name)]),
            None => tr("STR_HOMES_TITLE").to_string(),
        };
        lines.push((title, white));
        match self.xs.kept.homes.iter().find(|s| (s.a, s.b) == (o.a, o.b)) {
            Some(sale) => {
                lines.push((trf("STR_HOMES_COST", &[("MONEY", &dollars(sale.cost))]), red));
                lines.push((trf("STR_HOMES_SOLD", &[("MONEY", &dollars(sale.sold))]), green));
                lines.push((trf("STR_HOMES_MONTHLY", &[("MONEY", &dollars(per_month(sale.sold, sale.tick, self.date())))]), green));
            }
            None => lines.push((tr("STR_HOMES_NO_RECORD").to_string(), c15(0x6318))),
        }
        lines.push((trf("STR_HOMES_VALUE", &[("MONEY", &dollars(value as i64 / 4 * 100))]), green));
        let back = (o.val / 50 + value / 2) as i64 * 100;
        lines.push((trf("STR_HOMES_BUY_BACK", &[("MONEY", &dollars(back))]), red));
        let hole = sg_core::homes::value_hole(&self.course, &self.club.holes, self.difficulty, o.a, o.b);
        lines.push(match hole {
            Some((h, adds)) => (trf("STR_HOMES_HOLE", &[("HOLE", &h), ("MONEY", &dollars(adds as i64 / 4 * 100))]), green),
            None => (tr("STR_HOMES_NO_HOLE").to_string(), c15(0x6318)),
        });
        // a line wider than the box (a translation, issue #164): a little smaller, else on two lines (ui.rs fit_lines)
        let f = crate::ui::fnt_for(BODY);
        let rows: Vec<(crate::ui::Fnt, String, [f32; 4])> = lines
            .iter()
            .flat_map(|(t, c)| match crate::ui::fit_lines(f, t, DETAILS_ROOM, 2) {
                None => vec![(f, t.clone(), *c)],
                Some(fitted) => fitted.lines.into_iter().map(|l| (fitted.font, l, *c)).collect(),
            })
            .collect();
        // the lot box's frame (673, 380) grown leftward and upward to hold the lines, its bottom clear of the dock
        let h = 22.0 + 15.0 * rows.len() as f32;
        let (x, w) = (620.0, 180.0);
        let y = 470.0 - h;
        self.art.trans_frame(g, s, x, y, w, h);
        for (i, (f, t, c)) in rows.iter().enumerate() {
            // cursor_ui::label's look in the fitted font, a smaller one on the same middle
            let top = y + 10.0 + 15.0 * i as f32 + ((BODY - f.px) / 2.0).floor();
            s.put_centered(g, *f, x + w / 2.0, top + 1.0, t, [0.0, 0.0, 0.0, c[3]]);
            s.put_centered(g, *f, x + w / 2.0, top, t, *c);
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn a_sale_spread_over_the_months() {
        assert_eq!(per_month(1200, 0, 0), 1200, "the month it was placed counts as one");
        assert_eq!(per_month(1200, 0, 1024 * 3), 400);
        assert_eq!(per_month(1200, 1024 * 5, 1024 * 3), 1200, "a date before the sale (a game loaded from earlier)");
        assert_eq!(dollars(-1500), "-$1,500");
    }
}
