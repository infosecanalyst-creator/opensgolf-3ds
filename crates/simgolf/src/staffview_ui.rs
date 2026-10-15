//! The staff areas (the port's own; Shift+A, controls.rs; docs/PLAYING.md): every employee's work ring at once, drawn as
//! the original draws the ring of an employee being moved (cursor_ui.rs draw_staff_move: 4 tiles, 6 for a Groundskeeper,
//! 8 for a skilled one, in the job's colour) round each post, and the fairway, green and tee tiles outside every
//! Groundskeeper's ring tinted red, so the player sees where the next one is wanted. A legend in the upper right counts
//! the staff by job and the tiles no Groundskeeper reaches. It only shows; nothing of the game changes.
use crate::app::*;
use crate::cursor_ui::{work_colour, work_tiles};
use crate::gfx::Gfx;
use crate::info_ui::c15;
use crate::ui::{rgba, Screen as Ui, F_ARIAL10, F_MANUAL15};
use sg_core::lang::{tr, trf};
use sg_core::staff::job;
use sg_core::terrain::{TT_FAIRWAY, TT_PUTTING_GREEN, TT_TEE};

/// The view's marks, made again every few frames: the posts with their rings, and the kept tiles out of reach.
#[derive(Clone, Debug, Default)]
pub struct Marks {
    pub frame: u64,
    /// (post tile, radius in tiles, job).
    pub rings: Vec<((i32, i32), i32, i8)>,
    pub bare: Vec<(i32, i32)>,
}

/// True when tile (a, b) lies within `tiles` of the post tile's centre (the ring's radius, centre to centre).
pub fn reaches(post: (i32, i32), tiles: i32, a: i32, b: i32) -> bool {
    let (da, db) = (a - post.0, b - post.1);
    da * da + db * db <= tiles * tiles
}

/// The tiles of `kept` that no ring of `keepers` (post, radius in tiles) reaches.
pub fn out_of_reach(kept: &[(i32, i32)], keepers: &[((i32, i32), i32)]) -> Vec<(i32, i32)> {
    kept.iter().copied().filter(|&(a, b)| !keepers.iter().any(|&(p, r)| reaches(p, r, a, b))).collect()
}

impl App {
    /// Shift+A: the staff areas on or off.
    pub fn toggle_staff_view(&mut self) {
        self.comfort.staff_view = !self.comfort.staff_view;
        self.comfort.staff_marks = None;
        self.show_toast(tr(if self.comfort.staff_view { "STR_STAFF_VIEW_ON" } else { "STR_STAFF_VIEW_OFF" }));
    }

    fn staff_marks(&self) -> Marks {
        let rings: Vec<((i32, i32), i32, i8)> = self
            .employees
            .iter()
            .filter(|e| e.active && !e.going_home && e.job != job::OWNER)
            .filter_map(|e| e.post.map(|p| (p, work_tiles(e), e.job)))
            .collect();
        let keepers: Vec<((i32, i32), i32)> = rings.iter().filter(|r| r.2 == job::GROUNDSKEEPER).map(|r| (r.0, r.1)).collect();
        let t = &self.terrain;
        let mut kept = Vec::new();
        for b in 0..t.h {
            for a in 0..t.w {
                let ty = t.type_at(a, b);
                if ty == TT_FAIRWAY as i32 || ty == TT_PUTTING_GREEN as i32 || ty == TT_TEE as i32 {
                    kept.push((a, b));
                }
            }
        }
        Marks { frame: self.exe_frame, rings, bare: out_of_reach(&kept, &keepers) }
    }

    /// The staff areas over the course (draw_hud, under the HUD's own pieces).
    pub fn draw_staff_view(&mut self, g: &mut Gfx, s: &Ui) {
        if !self.comfort.staff_view || self.screen == Screen::YearEnd || self.land.is_none() {
            return;
        }
        if self.comfort.staff_marks.as_ref().is_none_or(|m| self.exe_frame.saturating_sub(m.frame) > 8 || self.dirty) {
            self.comfort.staff_marks = Some(self.staff_marks());
        }
        let Some(m) = self.comfort.staff_marks.clone() else { return };
        // the tiles first, then the rings over them
        let tint = rgba(1.0, 0.25, 0.25, 0.45);
        for &(a, b) in &m.bare {
            if self.screen_of(a * 1024 + 512, b * 1024 + 512).is_none() {
                continue;
            }
            let corner = |x: i32, y: i32| {
                let (wx, wz) = self.units_to_world(x, y);
                self.screen_of_world(wx, wz)
            };
            let (x0, y0, x1, y1) = (a * 1024, b * 1024, a * 1024 + 1024, b * 1024 + 1024);
            if let (Some(p0), Some(p1), Some(p2), Some(p3)) = (corner(x0, y0), corner(x1, y0), corner(x1, y1), corner(x0, y1)) {
                s.quad(g, [p0, p1, p2, p3], tint);
            }
        }
        for &((a, b), tiles, j) in &m.rings {
            self.draw_reach_ring(g, s, a * 1024 + 512, b * 1024 + 512, tiles * 25, work_colour(j));
        }
        self.draw_staff_legend(g, s, &m);
    }

    fn draw_staff_legend(&self, g: &mut Gfx, s: &Ui, m: &Marks) {
        use crate::enhanced::Anchor;
        s.anchor(Anchor::TopRight);
        // under the path view's legend when both are up
        let y = if self.xs.path_view && crate::extras::path_help() { 244.0 } else { 140.0 };
        let (x, w, h) = (566.0, 224.0, 126.0);
        self.art.trans_fill(g, s, x, y, w, h, true);
        let white = c15(0x7fff);
        s.put_shadowed(g, F_MANUAL15, x + 10.0, y + 6.0, tr("STR_STAFF_VIEW_TITLE"), c15(0x7ff0), false);
        let count = |j: i8| m.rings.iter().filter(|r| r.2 == j).count();
        let rows: [(u16, String); 4] = [
            (work_colour(job::GROUNDSKEEPER), trf("STR_STAFF_VIEW_KEEPERS", &[("COUNT", &count(job::GROUNDSKEEPER))])),
            (work_colour(job::RANGER), trf("STR_STAFF_VIEW_RANGERS", &[("COUNT", &count(job::RANGER))])),
            (work_colour(job::SODA_VENDOR), trf("STR_STAFF_VIEW_VENDORS", &[("COUNT", &count(job::SODA_VENDOR))])),
            (work_colour(job::CLUB_PRO), trf("STR_STAFF_VIEW_PROS", &[("COUNT", &count(job::CLUB_PRO))])),
        ];
        for (i, (c, t)) in rows.iter().enumerate() {
            let ry = y + 28.0 + 15.0 * i as f32;
            s.fill(g, x + 11.0, ry + 2.0, 12.0, 7.0, rgba(0.0, 0.0, 0.0, 0.6));
            s.fill(g, x + 12.0, ry + 3.0, 10.0, 5.0, c15(*c as u32));
            s.put(g, F_ARIAL10, x + 30.0, ry, t, white);
        }
        let ry = y + 28.0 + 15.0 * 4.0;
        s.fill(g, x + 11.0, ry + 2.0, 12.0, 7.0, rgba(0.0, 0.0, 0.0, 0.6));
        s.fill(g, x + 12.0, ry + 3.0, 10.0, 5.0, rgba(1.0, 0.25, 0.25, 0.8));
        s.put(g, F_ARIAL10, x + 30.0, ry, &trf("STR_STAFF_VIEW_BARE", &[("COUNT", &m.bare.len())]), white);
        s.anchor(Anchor::Centre);
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn rings_reach_their_radius_and_no_further() {
        assert!(reaches((10, 10), 6, 16, 10));
        assert!(reaches((10, 10), 6, 10, 4));
        assert!(!reaches((10, 10), 6, 17, 10));
        // the diagonal: 4 + 4 is 5.66 tiles, 5 + 5 is 7.07
        assert!(reaches((10, 10), 6, 14, 14));
        assert!(!reaches((10, 10), 6, 15, 15));
        assert!(reaches((10, 10), 8, 15, 15));
    }

    #[test]
    fn tiles_out_of_every_ring_are_left() {
        let kept = vec![(0, 0), (5, 0), (12, 0), (30, 30)];
        // no groundskeeper: every kept tile is bare
        assert_eq!(out_of_reach(&kept, &[]), kept);
        let keepers = [((0, 0), 6), ((14, 0), 6)];
        assert_eq!(out_of_reach(&kept, &keepers), vec![(30, 30)]);
        // a third one 6 tiles off reaches it; 7 tiles off only a skilled one (8 tiles) does
        assert!(out_of_reach(&kept, &[((0, 0), 6), ((14, 0), 6), ((30, 24), 6)]).is_empty());
        assert_eq!(out_of_reach(&kept, &[((0, 0), 6), ((14, 0), 6), ((30, 23), 6)]), vec![(30, 30)]);
        assert!(out_of_reach(&kept, &[((0, 0), 6), ((14, 0), 6), ((30, 23), 8)]).is_empty());
    }
}
