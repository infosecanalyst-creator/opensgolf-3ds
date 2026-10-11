//! Thought bubbles over the golfers (main frame 0x416a15): a golfer's latest thought shows for about 56 ticks, white at
//! first and grey as it fades, tinted by its tone while it is the newest one, over the golfer who says it.

use crate::app::*;
use crate::gfx::Gfx;
use crate::ui::{rgb, rgba, Screen as Ui};
use sg_core::golfer::SLOTS;

impl App {
    /// Screen point (virtual 800x600) of a map position, or None when it is off screen.
    pub fn screen_of(&self, x: i32, y: i32) -> Option<(f32, f32)> {
        let (wx, wz) = self.units_to_world(x, y);
        self.screen_of_world(wx, wz).filter(|&(vx, vy)| (0.0..800.0).contains(&vx) && (0.0..600.0).contains(&vy))
    }

    /// Screen point (virtual 800x600) of a world position, on screen or not.
    pub fn screen_of_world(&self, wx: f32, wz: f32) -> Option<(f32, f32)> {
        let h = self.terrain.height_at(wx, wz);
        let m = self.mv;
        let ex = m[0] * wx + m[4] * h + m[8] * wz + m[12];
        let ey = m[1] * wx + m[5] * h + m[9] * wz + m[13];
        let sx = ex / self.upp + self.draw_w * 0.5;
        let sy = self.draw_h * 0.5 - ey / self.upp;
        Some(self.view.to_virtual(sx, sy))
    }

    /// The exe's floating thought text (0x416a15, specs thoughts.md 3.1): only at the closest zoom, `zoom * 10` pixels above
    /// the golfer (the exe's zoom 4 is about our 0.9), white and grey once faded, tinted green or red only for the golfer's
    /// newest thought by the sign of the mood change it made; partners standing level are nudged 4 pixels apart. Drawn as
    /// `bubble`.
    pub fn draw_thoughts(&self, g: &mut Gfx, s: &Ui) {
        // the exe draws when its zoom is above 2 (our 0.45); its zoom 1, 2, 4 map to about 0.23, 0.45, 0.9
        let exe_zoom = self.zoom * 4.0 / 0.905;
        if !self.show_thoughts || exe_zoom <= 2.0 {
            return;
        }
        let c15 = |v: u16| rgb(((v >> 10) & 31) as f32 / 31.0, ((v >> 5) & 31) as f32 / 31.0, (v & 31) as f32 / 31.0);
        for gi in 0..SLOTS {
            let gg = &self.club.g[gi];
            if gg.hole <= 0 || !(1..=7).contains(&gg.timer) || gg.thought == 0 || gg.thought == 0x32 {
                continue;
            }
            let line = self.club.thought(&self.course, gg.thought as u32, gg.thought_arg & 0x3fff, gi);
            if line.text.is_empty() {
                continue;
            }
            let speaker = if line.partner { gi ^ 1 } else { gi };
            let sp = &self.club.g[speaker];
            let Some((x, y)) = self.screen_of(sp.x, sp.y) else { continue };
            let (x, y) = (x.floor(), y.floor());
            let mut up = exe_zoom * 10.0;
            let p = &self.club.g[gi ^ 1];
            if p.timer != 0 && p.hole > 0 {
                if let Some((_, py)) = self.screen_of(p.x, p.y) {
                    if (y - py).abs() <= 12.0 {
                        if y > py || (y == py && gi & 1 == 1) {
                            up -= 4.0;
                        } else {
                            up += 4.0;
                        }
                    }
                }
            }
            let faded = gg.timer < 5;
            let newest = gg.thought & 0x80 == 0 && gg.thought == gg.thoughts[0] & 0x7f;
            let c = match (newest, gg.args[0] & 0xc000) {
                (true, 0x4000) => c15(if faded { 0x2308 } else { 0x43f0 }),
                (true, 0xc000) => c15(if faded { 0x6000 } else { 0x7d08 }),
                _ => c15(if faded { 0x6318 } else { 0x7fff }),
            };
            bubble(g, s, exe_zoom, x, (y - up).round(), &line.text, c);
        }
    }

    /// Golfer names (0x462be0), with the "Display golfer names on screen" option at the closest zoom: centred on the golfer
    /// with the text's top at the golfer's point, no shadow (0x404b70), in Arial Bold 10 at the exe's zoom 4 (Manual SSi 15
    /// above it), white, or red for one leaving the course (shown on alternate frames), with "!" for a hurried one;
    /// golfers showing a thought are skipped unless selected. With the golfers panel open each name gets a mood bar 9 pixels
    /// below: black from -12 to +12, coloured to (mood - 4) * 3.
    pub fn draw_names(&self, g: &mut Gfx, s: &Ui) {
        use sg_core::golfer::flag;
        let exe_zoom = self.zoom * 4.0 / 0.905;
        if !self.show_names || exe_zoom <= 3.0 || self.club.pro_aiming().is_some() {
            return;
        }
        let c15 = |v: u16| rgb(((v >> 10) & 31) as f32 / 31.0, ((v >> 5) & 31) as f32 / 31.0, (v & 31) as f32 / 31.0);
        let picked = self.card;
        for gi in 0..SLOTS {
            let gg = &self.club.g[gi];
            if gg.hole <= 0 || gg.anim <= 5 || gg.anim == 0x10 || gg.flags & 0x8000 != 0 {
                continue;
            }
            let selected = picked.is_some_and(|p| p == gi || p == gi ^ 1);
            if gg.timer != 0 && !selected {
                continue;
            }
            let leaving = gg.flags & flag::LEAVING != 0;
            if leaving && self.game_tick & 6 == 0 {
                continue;
            }
            let Some((x, y)) = self.screen_of(gg.x, gg.y) else { continue };
            let mut name = self.club.name(gi);
            if gg.flags & flag::HURRIED != 0 {
                name.push('!');
            }
            let c = c15(if leaving { 0x7d08 } else { 0x7fff });
            let (x, y) = (x.floor(), y.floor());
            let font = if exe_zoom.round() >= 5.0 { crate::ui::F_MANUAL15 } else { crate::ui::F_ARIAL10 };
            s.put_centered(g, font, x, y, &name, c);
            if self.panel == 4 && self.club.game & sg_core::golfer::game::TOURNAMENT == 0 {
                let m = gg.mood;
                let bar = match m {
                    m if m < 0 => 0x0000,
                    m if m > 6 => 0x23e8,
                    m if m > 4 => 0x1304,
                    m if m > 2 => 0x6300,
                    _ => 0x7d08,
                };
                s.fill(g, x - 12.0, y + 9.0, 24.0, 1.0, rgb(0.0, 0.0, 0.0));
                let w = ((m - 4) * 3 + 12) as f32;
                if w > 0.0 {
                    s.fill(g, x - 12.0, y + 9.0, w, 1.0, c15(bar));
                }
            }
        }
    }

    /// Floating money (0x40c910): "+" and the amount in green for income, the amount in red for a cost, in Manual SSi 15
    /// (0x51b360) with the shadowed centred call (0x404bc0): centred on the map point, its top 4 pixels up.
    pub fn draw_floats(&self, g: &mut Gfx, s: &Ui) {
        let c15 = |v: u16| rgb(((v >> 10) & 31) as f32 / 31.0, ((v >> 5) & 31) as f32 / 31.0, (v & 31) as f32 / 31.0);
        for &(units, x, y, life) in &self.floats {
            if life == 0 || self.econ.sandbox {
                continue;
            }
            let Some((sx, sy)) = self.screen_of(x, y) else { continue };
            let text = format!("{}{}", if units > 0 { "+" } else { "" }, crate::ui::money(units as i64 * 100));
            let c = c15(if units > 0 { 0x23e8 } else { 0x7d08 });
            s.put_shadowed(g, crate::ui::F_MANUAL15, sx.floor(), sy.floor() - 4.0, &text, c, true);
        }
    }
}

/// A speech bubble over a person (the thoughts at 0x416a15 and the advisor at 0x416305): the text centred on x with its top
/// at y, in Manual SSi 15 above the exe's zoom 4 and Arial Bold 10 at it; behind it a half transparent black line 10 pixels
/// thick (Terrain::drawLine, its last argument the alpha in tenths: 5) from the text's top down, the text's width plus 8
/// long, and the tail 4 pixels right of x under the line: the 8 x 8 cut at (512, 140) of course1.pcx (object 0x5a4100)
/// drawn through the halving colour table 0x822c74. DERIVED: the tail's shape is the cut's darker pixels (a corner
/// triangle, rows 5 to 1 pixels wide), its lighter green being the sheet's background.
pub fn bubble(g: &mut Gfx, s: &Ui, exe_zoom: f32, x: f32, y: f32, text: &str, c: [f32; 4]) {
    let font = if exe_zoom.round() > 4.0 { crate::ui::F_MANUAL15 } else { crate::ui::F_ARIAL10 };
    let half_black = rgba(0.0, 0.0, 0.0, 0.5);
    let x = x.floor();
    let half = ((font.width(text) + 8.0) / 2.0).trunc();
    s.fill(g, x - half, y, 2.0 * half, 10.0, half_black);
    for row in 0..5 {
        s.fill(g, x + 4.0, y + 10.0 + row as f32, 5.0 - row as f32, 1.0, half_black);
    }
    s.put_centered(g, font, x, y, text, c);
}
