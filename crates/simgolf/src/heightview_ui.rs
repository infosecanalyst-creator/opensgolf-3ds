//! The height map (the port's own; Shift+K, controls.rs, and the Information menu's line with the landing map's extra;
//! docs/PLAYING.md): every tile of the course tinted by how high it stands, from
//! the club's usual ground (its commonest level) in green, each level lower bluer and each level higher through yellow
//! and brown to white, so slopes, hills and basins show at a glance at any zoom. A legend in the upper right gives the
//! scale and, in levels against the usual ground, the lowest and highest tile and the tile under the pointer. It only shows;
//! nothing of the game changes.
use crate::app::*;
use crate::gfx::Gfx;
use crate::info_ui::c15;
use crate::ui::{rgba, Screen as Ui, F_ARIAL10, F_MANUAL15};
use sg_core::lang::{tr, trf};

/// The legend's height at normal size.
pub const LEGEND_H: f32 = 92.0;

/// Where the usual ground (the commonest level) sits on the ramp: green.
const USUAL_T: f32 = 0.4;
/// How far one level up or down moves along the ramp.
const LEVEL_T: f32 = 0.15;

/// Where height `h` falls on the ramp, against the usual ground.
pub fn height_t(h: f32, usual: f32) -> f32 {
    (USUAL_T + (h - usual) * LEVEL_T).clamp(0.0, 1.0)
}

/// The ramp's stops, low to high.
const RAMP: [[f32; 3]; 6] =
    [[0.10, 0.25, 0.80], [0.10, 0.65, 0.85], [0.20, 0.75, 0.25], [0.95, 0.85, 0.20], [0.65, 0.35, 0.15], [0.97, 0.97, 0.97]];

/// The ramp's colour at `t` (0 lowest, 1 highest), with alpha `a`.
pub fn ramp(t: f32, a: f32) -> [f32; 4] {
    let t = t.clamp(0.0, 1.0) * (RAMP.len() - 1) as f32;
    let i = (t.floor() as usize).min(RAMP.len() - 2);
    let f = t - i as f32;
    let (p, q) = (RAMP[i], RAMP[i + 1]);
    [p[0] + (q[0] - p[0]) * f, p[1] + (q[1] - p[1]) * f, p[2] + (q[2] - p[2]) * f, a]
}

impl App {
    /// Shift+K and the Information menu's line: the height map on or off. It shares the lower legend's place with the
    /// staff areas and the landing map, so turning it on puts those away.
    pub fn toggle_height_view(&mut self) {
        self.comfort.height_view = !self.comfort.height_view;
        if self.comfort.height_view {
            self.comfort.staff_view = false;
            self.xs.insight.map = false;
        }
        self.show_toast(tr(if self.comfort.height_view { "STR_HEIGHT_VIEW_ON" } else { "STR_HEIGHT_VIEW_OFF" }));
    }

    /// A tile's height: the mean of its four corners (terrain levels).
    pub fn tile_height(&self, a: i32, b: i32) -> f32 {
        let t = &self.terrain;
        if !t.inside(a, b) {
            return 0.0;
        }
        let c = |x: i32, y: i32| t.corner.get((y * (t.w + 1) + x) as usize).copied().unwrap_or(0) as f32;
        (c(a, b) + c(a + 1, b) + c(a, b + 1) + c(a + 1, b + 1)) / 4.0
    }

    /// Whether the map tints tile a, b: the club's land (all of it on a course without land records).
    fn height_shown(&self, a: i32, b: i32) -> bool {
        self.terrain.inside(a, b) && self.land.as_ref().is_none_or(|l| l.in_play(a, b))
    }

    /// The club's land's usual ground (the commonest corner level), lowest and highest tile.
    fn height_range(&self) -> (f32, f32, f32) {
        let (mut lo, mut hi) = (f32::MAX, f32::MIN);
        let mut count = std::collections::BTreeMap::<i32, u32>::new();
        for a in 0..self.terrain.w {
            for b in 0..self.terrain.h {
                if self.height_shown(a, b) {
                    let h = self.tile_height(a, b);
                    lo = lo.min(h);
                    hi = hi.max(h);
                    *count.entry(h.round() as i32).or_default() += 1;
                }
            }
        }
        let usual = count.iter().max_by_key(|e| *e.1).map_or(0.0, |e| *e.0 as f32);
        if lo > hi {
            (0.0, 0.0, 0.0)
        } else {
            (usual, lo, hi)
        }
    }

    /// The height map over the course (draw_hud, under the golfers' names, as the landing map's heat is).
    pub fn draw_height_view(&mut self, g: &mut Gfx, s: &Ui) {
        if !self.comfort.height_view || self.screen == Screen::YearEnd {
            return;
        }
        let (usual, lo, hi) = self.height_range();
        let corner = |x: i32, y: i32| {
            let (wx, wz) = self.units_to_world(x, y);
            self.screen_of_world(wx, wz)
        };
        let (w, h) = (self.terrain.w, self.terrain.h);
        for a in 0..w {
            for b in 0..h {
                if !self.height_shown(a, b) || self.screen_of(a * 1024 + 512, b * 1024 + 512).is_none() {
                    continue;
                }
                let (x0, y0, x1, y1) = (a * 1024, b * 1024, a * 1024 + 1024, b * 1024 + 1024);
                if let (Some(p0), Some(p1), Some(p2), Some(p3)) = (corner(x0, y0), corner(x1, y0), corner(x1, y1), corner(x0, y1)) {
                    s.quad(g, [p0, p1, p2, p3], ramp(height_t(self.tile_height(a, b), usual), 0.45));
                }
            }
        }
        self.draw_height_legend(g, s, (usual, lo, hi));
    }

    fn draw_height_legend(&self, g: &mut Gfx, s: &Ui, (usual, lo, hi): (f32, f32, f32)) {
        use crate::enhanced::Anchor;
        s.anchor(Anchor::TopRight);
        let (k, _, y) = self.legend_place(LEGEND_H);
        let (w, h) = ((224.0 * k).round(), (LEGEND_H * k).round());
        let x = 790.0 - w;
        self.art.trans_fill(g, s, x, y, w, h, true);
        let white = c15(0x7fff);
        let title = tr("STR_HEIGHT_VIEW_TITLE");
        s.put_shadowed(g, crate::textsize::fit(F_MANUAL15, k, title, w - 20.0), x + 10.0, y + 6.0, title, c15(0x7ff0), false);
        // the scale: the ramp as a bar, low on the left
        let (bx, by, bw, bh) = (x + 10.0, y + (28.0 * k).round(), w - 20.0, (10.0 * k).round());
        s.fill(g, bx - 1.0, by - 1.0, bw + 2.0, bh + 2.0, rgba(0.0, 0.0, 0.0, 0.6));
        let steps = 40;
        for i in 0..steps {
            let sx = bx + bw * i as f32 / steps as f32;
            s.fill(g, sx, by, bw / steps as f32 + 0.5, bh, ramp(i as f32 / (steps - 1) as f32, 1.0));
        }
        // the usual ground's mark on the bar
        let ux = bx + bw * USUAL_T;
        s.fill(g, ux - 1.0, by - 3.0, 2.0, bh + 6.0, c15(0x7fff));
        let small = |t: &str| crate::textsize::fit(F_ARIAL10, k, t, w / 2.0 - 12.0);
        let ty = by + bh + 2.0;
        let low = trf("STR_HEIGHT_VIEW_LOW", &[("LEVEL", &format!("{:+.1}", lo - usual))]);
        let high = trf("STR_HEIGHT_VIEW_HIGH", &[("LEVEL", &format!("{:+.1}", hi - usual))]);
        s.put(g, small(&low), bx, ty, &low, white);
        let hw = small(&high).width(&high);
        s.put(g, small(&high), bx + bw - hw, ty, &high, white);
        // the height under the pointer, over the course
        if self.has_hit {
            let (a, b) = self.terrain.tile_of(self.hit_x, self.hit_z);
            if self.height_shown(a, b) {
                let here = trf("STR_HEIGHT_VIEW_HERE", &[("LEVEL", &format!("{:+.1}", self.tile_height(a, b) - usual))]);
                let f = crate::textsize::fit(F_ARIAL10, k, &here, w - 20.0);
                s.put(g, f, x + 10.0, ty + (16.0 * k).round(), &here, white);
            }
        }
        let close = tr("STR_HEIGHT_VIEW_CLOSE");
        s.put(g, crate::textsize::fit(F_ARIAL10, k, close, w - 20.0), x + 10.0, y + h - (18.0 * k).round(), close, c15(0x6318));
        s.anchor(Anchor::Centre);
    }
}

#[cfg(test)]
mod tests {
    use super::{height_t, ramp};

    #[test]
    fn the_usual_ground_is_green_and_each_level_moves_it() {
        assert_eq!(height_t(3.0, 3.0), 0.4);
        assert!(height_t(2.0, 3.0) < 0.4 && height_t(4.0, 3.0) > 0.4);
        assert_eq!(height_t(30.0, 3.0), 1.0);
        let g = ramp(0.4, 1.0);
        assert!(g[1] > g[0] && g[1] > g[2]);
    }

    #[test]
    fn the_ramp_runs_from_blue_to_white() {
        let (lo, hi) = (ramp(0.0, 1.0), ramp(1.0, 1.0));
        assert!(lo[2] > lo[0] && lo[2] > lo[1]);
        assert!(hi.iter().all(|&c| c > 0.9));
        // out of range is clamped
        assert_eq!(ramp(-1.0, 0.5), ramp(0.0, 0.5));
        assert_eq!(ramp(2.0, 0.5), ramp(1.0, 0.5));
    }
}
