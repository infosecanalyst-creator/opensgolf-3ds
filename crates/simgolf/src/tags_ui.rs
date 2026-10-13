//! OpenSGolf extras: name tags under the pointer (extras.rs). The golfer or employee nearest the pointer on the course
//! (within 22 pixels of where they stand) gets a small tag above the head: a golfer's ball portrait with the face of the
//! moment and the name (in the golfers' name colours: white, red while leaving), an employee's name and job. It shows
//! whether the original's golfer names (Shift+N) are on or off; Shift+H, or its tick in Preferences > OpenSGolf extras,
//! turns the tags off and on. Nothing is drawn while a tool is armed, over the dock and its panel, or on a touch screen
//! (which has no pointer to hover with).
use crate::app::*;
use crate::gfx::Gfx;
use crate::info_ui::c15;
use crate::ui::{rgba, Screen as Ui, F_ARIAL10};

/// How near the pointer must be (800 x 600 pixels), and the tag's face size.
const REACH: f32 = 22.0;
const FACE: f32 = 18.0;

/// Who is under the pointer.
enum Who {
    Golfer(usize),
    Staff(usize),
}

impl App {
    fn tag_target(&self) -> Option<(Who, f32, f32)> {
        let (px, py) = self.info.pointer;
        let mut best: Option<(f32, Who, f32, f32)> = None;
        let consider = |d: f32, w: Who, x: f32, y: f32, best: &mut Option<(f32, Who, f32, f32)>| {
            if d < REACH && best.as_ref().is_none_or(|b| d < b.0) {
                *best = Some((d, w, x, y));
            }
        };
        // the pointer anywhere over the figure (from the feet up to the head) counts
        let lift = self.exe_zoom() * 8.0;
        let reach = |x: f32, y: f32| {
            let dy = if py > y {
                py - y
            } else if py < y - lift {
                y - lift - py
            } else {
                0.0
            };
            (x - px).hypot(dy)
        };
        for g in 0..sg_core::golfer::SLOTS {
            let gg = &self.club.g[g];
            if !(1..=19).contains(&gg.hole) || gg.roster < 0 {
                continue;
            }
            let Some((x, y)) = self.screen_of(gg.x, gg.y) else { continue };
            consider(reach(x, y), Who::Golfer(g), x, y, &mut best);
        }
        for (i, e) in self.employees.iter().enumerate() {
            if !e.active || e.job == sg_core::staff::job::OWNER {
                continue;
            }
            let Some((x, y)) = self.screen_of(e.x, e.y) else { continue };
            consider(reach(x, y) + 2.0, Who::Staff(i), x, y, &mut best);
        }
        best.map(|b| (b.1, b.2, b.3))
    }

    /// The tag for this frame (draw_hud, over the golfers' names and thoughts).
    pub fn draw_name_tag(&self, g: &mut Gfx, s: &Ui) {
        if !crate::extras::name_tags() || self.screen != Screen::Play || self.edit || self.card_ui.held.is_some() || self.xs.touched {
            return;
        }
        let (px, py) = self.info.pointer;
        // not over the dock or its open panel
        if self.dock_hover >= 0 || (px < 230.0 && py > 430.0) || (self.panel != 0 && px > 220.0 && py > 446.0) {
            return;
        }
        let Some((who, x, y)) = self.tag_target() else { return };
        let (face, name, sub, ink) = match who {
            Who::Golfer(gi) => {
                let gg = &self.club.g[gi];
                let leaving = gg.flags & sg_core::golfer::flag::LEAVING != 0;
                let ink = c15(if leaving { 0x7d08 } else { 0x7fff });
                (Some((gg.roster, self.club.attitude(gi) as usize)), self.club.vip_name(gi), self.golfer_type(gi), ink)
            }
            Who::Staff(i) => {
                let e = &self.employees[i];
                let job = staff_title((-2 - e.job as i32).clamp(0, 3) as usize, e.upgraded).to_string();
                (None, self.staff_name(i), job, c15(0x7fff))
            }
        };
        let f = F_ARIAL10;
        let tw = f.width(&name).max(f.width(&sub));
        let w = tw + 10.0 + if face.is_some() { FACE + 4.0 } else { 0.0 };
        let h = 26.0;
        let top = (y - self.exe_zoom() * 10.0 - h - 6.0).floor().max(2.0);
        let left = (x - w / 2.0).floor().clamp(2.0, 798.0 - w);
        // the ticker strip's translucent dark fill and a thin light edge
        s.fill(g, left - 1.0, top - 1.0, w + 2.0, h + 2.0, rgba(0.85, 0.85, 1.0, 0.55));
        s.fill(g, left, top, w, h, rgba(0.0, 0.0, 0.1, 0.62));
        let mut tx = left + 5.0;
        if let Some((r, e)) = face {
            self.small_head(g, s, r, e, tx - 2.0, top + 4.0, FACE);
            tx += FACE + 4.0;
        }
        s.put_shadowed(g, f, tx, top + 2.0, &name, ink, false);
        s.put(g, f, tx, top + 14.0, &sub, c15(0x6318));
    }
}
