//! The port's own camera helpers (docs/PLAYING.md, "Camera bookmarks"): none of them in the original, and nothing changes
//! until the player uses them.
//!
//! * Camera bookmarks: four views of the course (its centre, zoom and turn) kept with the game. Shift+F1 to Shift+F4 (keys
//!   the original leaves unused; remappable in Preferences > Keyboard controls) go back to a view, Ctrl with the same key
//!   keeps the view there. They are saved with the game (the course's own places), and only once one has been kept: a
//!   game without any saves exactly as before.
//! * Zoom towards the pointer (Preferences > OpenSGolf extras, off by default): the mouse wheel and a two-finger pinch keep
//!   the ground under the pointer (or between the fingers) where it is on the screen, at the original's three levels and
//!   with Enhanced graphics' smooth zoom alike (the original zooms about the middle of the screen).
use crate::app::*;
use serde::{Deserialize, Serialize};
use sg_core::lang::trf;

/// The camera bookmarks.
pub const SLOTS: usize = 4;
/// Their actions in controls.rs, in slot order.
pub const ACTION_IDS: [&str; SLOTS] = ["view_1", "view_2", "view_3", "view_4"];

/// A kept view: the world point in the middle of the screen, the zoom and the turn (degrees, a multiple of 90).
#[derive(Clone, Copy, Debug, PartialEq, Serialize, Deserialize)]
pub struct View {
    pub x: f32,
    pub z: f32,
    pub zoom: f32,
    pub rot: f32,
}

/// The camera to the ground point `g` kept at its place on the screen as the zoom goes from `from` to `to`: the screen is
/// scaled about that point, so the middle moves along the line to it by the zoom's ratio.
pub fn anchored(cam: (f32, f32), g: (f32, f32), from: f32, to: f32) -> (f32, f32) {
    if to <= 0.0 || from <= 0.0 {
        return cam;
    }
    let k = from / to;
    (g.0 + (cam.0 - g.0) * k, g.1 + (cam.1 - g.1) * k)
}

impl App {
    /// The point of the ground plane (height 0) under a drawable pixel, with the camera as last drawn: the course under
    /// the pixel lies on the same line of sight, so keeping this point keeps it.
    pub fn plane_point(&self, mx: f32, my: f32) -> Option<(f32, f32)> {
        let m: Vec<f64> = self.mv.iter().map(|&v| v as f64).collect();
        let upp = self.upp as f64;
        if upp <= 0.0 {
            return None;
        }
        let ex = (mx as f64 - self.draw_w as f64 * 0.5) * upp;
        let ey = (self.draw_h as f64 * 0.5 - my as f64) * upp;
        let v = [ex - m[12], ey - m[13], -m[14]];
        let mut o = [0f64; 3];
        let mut d = [0f64; 3];
        for j in 0..3 {
            o[j] = m[4 * j] * v[0] + m[4 * j + 1] * v[1] + m[4 * j + 2] * v[2];
            d[j] = -m[4 * j + 2];
        }
        if d[1].abs() < 1e-6 {
            return None;
        }
        let s = -o[1] / d[1];
        Some(((o[0] + d[0] * s) as f32, (o[2] + d[2] * s) as f32))
    }

    /// A zoom made by `change` (the wheel, a pinch) about drawable pixel `at`: with Zoom towards the pointer on, the ground
    /// under it stays there (with the smooth camera frame by frame as the zoom eases, enhanced.rs camera_frame); else the
    /// change alone, as the original.
    pub fn zoom_about(&mut self, at: (f32, f32), change: impl FnOnce(&mut App)) {
        if !crate::extras::zoom_pointer() {
            return change(self);
        }
        if crate::enhanced::smooth_camera() || self.photo.is_some() {
            self.enh.zoom_anchor = Some(at);
            return change(self);
        }
        let before = self.zoom;
        let g = self.plane_point(at.0, at.1);
        change(self);
        if let Some(g) = g.filter(|_| self.zoom != before) {
            (self.cam_x, self.cam_z) = anchored((self.cam_x, self.cam_z), g, before, self.zoom);
        }
    }

    /// The smooth camera's shown zoom went from `from` to `to` this frame: the anchor's ground keeps its place.
    pub fn zoom_anchor_frame(&mut self, from: f32, to: f32) {
        let Some(at) = self.enh.zoom_anchor else { return };
        if from != to {
            if let Some(g) = self.plane_point(at.0, at.1) {
                (self.cam_x, self.cam_z) = anchored((self.cam_x, self.cam_z), g, from, to);
            }
        }
        if to == self.zoom {
            self.enh.zoom_anchor = None;
        }
    }

    /// Ctrl with a bookmark's key: the view kept in slot `i`.
    pub fn keep_view(&mut self, i: usize) {
        let v = View { x: self.cam_x, z: self.cam_z, zoom: self.zoom, rot: self.rot.rem_euclid(360.0) };
        let views = &mut self.xs.kept.views;
        if views.len() < SLOTS {
            views.resize(SLOTS, None);
        }
        views[i] = Some(v);
        let key = crate::controls::key_label(ACTION_IDS[i]);
        self.show_toast(&trf("STR_TOAST_VIEW_KEPT", &[("SLOT", &(i + 1)), ("KEY", &key)]));
    }

    /// A bookmark's key: back to the view kept in slot `i` (its zoom at the nearest of the original's levels without the
    /// smooth camera).
    pub fn recall_view(&mut self, i: usize) {
        let Some(v) = self.xs.kept.views.get(i).copied().flatten() else {
            let key = crate::controls::key_label(ACTION_IDS[i]);
            self.show_toast(&trf("STR_TOAST_VIEW_NONE", &[("SLOT", &(i + 1)), ("KEY", &key)]));
            return;
        };
        self.xs.follow = None;
        self.cam_x = v.x;
        self.cam_z = v.z;
        let far = far_level();
        self.zoom = if crate::enhanced::smooth_camera() {
            v.zoom.clamp(far_zoom(), ZOOM_UNIT * 4.0)
        } else {
            ZOOM_UNIT * ((v.zoom / ZOOM_UNIT).log2().round().clamp(far as f32, 2.0)).exp2()
        };
        let quarters = (((v.rot - self.rot) / 90.0).round() as i32).rem_euclid(4);
        if quarters != 0 {
            self.rotate_view(if quarters == 3 { -1 } else { quarters });
        }
        self.show_toast(&trf("STR_TOAST_VIEW", &[("SLOT", &(i + 1))]));
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn the_anchor_keeps_its_place() {
        // zooming in twice about a point: the point stays, the middle closes in on it by half
        let c = anchored((0.0, 0.0), (100.0, 40.0), 1.0, 2.0);
        assert_eq!(c, (50.0, 20.0));
        // the screen offset of the point (its distance from the middle times the zoom) is the same before and after
        let off = |c: (f32, f32), z: f32| ((100.0 - c.0) * z, (40.0 - c.1) * z);
        assert_eq!(off((0.0, 0.0), 1.0), off(c, 2.0));
        // and back out again
        assert_eq!(anchored(c, (100.0, 40.0), 2.0, 1.0), (0.0, 0.0));
        // nothing for a zoom of nothing
        assert_eq!(anchored((3.0, 4.0), (1.0, 1.0), 0.0, 1.0), (3.0, 4.0));
    }

    #[test]
    fn views_round_trip_in_the_save() {
        let mut k = crate::extras::Kept::default();
        assert!(!serde_json::to_string(&k).unwrap().contains("views"));
        k.views = vec![None, Some(View { x: 1.5, z: -2.0, zoom: 0.45, rot: 90.0 }), None, None];
        let back: crate::extras::Kept = serde_json::from_str(&serde_json::to_string(&k).unwrap()).unwrap();
        assert_eq!(back, k);
    }
}
