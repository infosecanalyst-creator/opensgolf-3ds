//! Thought bubbles over the golfers (main frame 0x416a15): a golfer's latest thought shows for about 56 ticks, white at
//! first and grey as it fades, tinted by its tone while it is the newest one, over the golfer who says it.

use crate::app::*;
use crate::gfx::Gfx;
use crate::ui::{rgb, rgba, text_width, Screen as Ui};
use sg_core::golfer::SLOTS;
use sg_core::thoughts::Tone;

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

    pub fn draw_thoughts(&self, g: &mut Gfx, s: &Ui) {
        if !self.show_thoughts || self.zoom < 0.45 {
            return;
        }
        // bubbles already placed this frame (left, top, right, bottom), so later ones stack above instead of overlapping
        let mut shown: Vec<(f32, f32, f32, f32)> = Vec::new();
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
            let size = if self.zoom > 1.2 { 14.0 } else { 12.0 };
            let mut up = 34.0 * self.zoom.clamp(0.6, 2.0);
            let w = text_width(&line.text, size) + 10.0;
            let h = size + 7.0;
            let rect = |up: f32| (x - w / 2.0, y - up - size - 2.0, x + w / 2.0, y - up - size - 2.0 + h);
            for _ in 0..8 {
                let r = rect(up);
                if !shown.iter().any(|o| r.0 < o.2 && o.0 < r.2 && r.1 < o.3 && o.1 < r.3) {
                    break;
                }
                up += h + 2.0;
            }
            let fresh = gg.timer >= 5;
            let c = match (line.tone, fresh) {
                (Tone::Good, true) => rgb(0.55, 1.0, 0.55),
                (Tone::Good, false) => rgb(0.25, 0.6, 0.3),
                (Tone::Bad, true) => rgb(1.0, 0.45, 0.4),
                (Tone::Bad, false) => rgb(0.7, 0.2, 0.2),
                (_, true) => rgb(1.0, 1.0, 1.0),
                (_, false) => rgb(0.62, 0.62, 0.66),
            };
            s.fill(g, x - w / 2.0, y - up - size - 2.0, w, h, rgba(0.0, 0.0, 0.0, 0.7));
            s.text_centered(g, x, y - up, &line.text, size, c);
            shown.push(rect(up));
        }
    }
}
