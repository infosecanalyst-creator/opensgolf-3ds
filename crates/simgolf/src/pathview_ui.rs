//! OpenSGolf extras: path help (extras.rs). The path view (Shift+V, or the Information menu's "Path view" line) marks
//! every path tile, green when it is joined to the clubhouse and orange when it is not (the clubhouse flood the original
//! uses to make a building operational, sg_core::land::joined_tiles), and puts a "No path" marker on each building the
//! paths do not reach (the original's buildings off the flood, which do not work) and on each hole with no joined path
//! within three tiles of its tee; a legend in the upper right explains the marks. The original's monthly warning that a
//! building is not operational gets a Show button on the ticker that takes the camera to the building (message_ui).
use crate::app::*;
use crate::gfx::Gfx;
use crate::info_ui::c15;
use crate::ui::{rgba, Screen as Ui, F_ARIAL10, F_MANUAL15};
use sg_core::lang::{tr, trf};

/// How near a joined path must come to a hole's tee (tiles, in either direction).
const TEE_REACH: i32 = 3;

/// A "No path" marker: the map point (units) and what it is about.
#[derive(Clone, Debug, PartialEq)]
pub struct Mark {
    pub at: (i32, i32),
    pub label: String,
}

/// The path view's marks, made again every few frames: the path tiles (tile, joined) and the markers.
#[derive(Clone, Debug, Default)]
pub struct Marks {
    pub frame: u64,
    pub tiles: Vec<((i32, i32), bool)>,
    pub marks: Vec<Mark>,
}

impl App {
    /// Shift+V and the Information menu's line: the path view on or off.
    pub fn toggle_path_view(&mut self) {
        self.xs.path_view = !self.xs.path_view;
        self.xs.path_marks = None;
        self.show_toast(tr(if self.xs.path_view { "STR_PATHVIEW_ON" } else { "STR_PATHVIEW_OFF" }));
    }

    fn path_marks(&self) -> Marks {
        let t = &self.terrain;
        let joined: Vec<bool> = self
            .land
            .as_ref()
            .map(|l| {
                let mut l = l.clone();
                l.sync_from_terrain(t);
                l.joined_tiles()
            })
            .unwrap_or_default();
        let is_joined = |x: i32, y: i32| !sg_core::course::inside(x, y) || joined.get(sg_core::course::idx(x, y)).copied().unwrap_or(true);
        let mut tiles = Vec::new();
        for (i, &k) in t.path_kind.iter().enumerate() {
            if k != 0 {
                let (x, y) = (i as i32 % t.w, i as i32 / t.w);
                tiles.push(((x, y), is_joined(x, y)));
            }
        }
        let mut marks = Vec::new();
        for o in self.course.objects.iter().skip(1) {
            if (6..=14).contains(&o.kind) && o.flags & 0x40 == 0 {
                let name = sg_core::land::building_name(o.kind);
                marks.push(Mark { at: (o.a * 1024 + 1024, o.b * 1024 + 1024), label: name.to_string() });
            }
        }
        for h in 1..19 {
            let hr = &self.club.holes[h];
            if hr.par == 0 {
                continue;
            }
            let (tx, ty) = hr.back;
            let near = tiles.iter().any(|&((x, y), j)| j && (x - tx).abs() <= TEE_REACH && (y - ty).abs() <= TEE_REACH);
            if !near {
                marks.push(Mark { at: (tx * 1024 + 512, ty * 1024 + 512), label: trf("STR_PATHVIEW_HOLE", &[("HOLE", &h)]) });
            }
        }
        Marks { frame: self.exe_frame, tiles, marks }
    }

    /// The path view over the course (draw_hud, under the HUD's own pieces).
    pub fn draw_path_view(&mut self, g: &mut Gfx, s: &Ui) {
        if !self.xs.path_view || !crate::extras::path_help() || self.screen == Screen::YearEnd {
            return;
        }
        if self.xs.path_marks.as_ref().is_none_or(|m| self.exe_frame.saturating_sub(m.frame) > 8 || self.dirty) {
            self.xs.path_marks = Some(self.path_marks());
        }
        let Some(m) = self.xs.path_marks.clone() else { return };
        let k = (self.exe_zoom() / 4.0).clamp(0.5, 1.0);
        for &((x, y), j) in &m.tiles {
            let Some((sx, sy)) = self.screen_of(x * 1024 + 512, y * 1024 + 512) else { continue };
            let c = if j { c15(0x03e0) } else { c15(0x7e00) };
            let (w, h) = (10.0 * k, 5.0 * k);
            s.fill(g, sx - w / 2.0 - 1.0, sy - h / 2.0 - 1.0, w + 2.0, h + 2.0, rgba(0.0, 0.0, 0.0, 0.5));
            s.fill(g, sx - w / 2.0, sy - h / 2.0, w, h, c);
        }
        let no_path = tr("STR_PATHVIEW_NO_PATH");
        for mk in &m.marks {
            let Some((sx, sy)) = self.screen_of(mk.at.0, mk.at.1) else { continue };
            // the PopUpIcons exclamation (the ticker's warning picture), shrunk, over the place
            if self.art.popup_icons.tex.is_some() {
                s.image_scaled(g, &self.art.popup_icons, (sx - 14.0, sy - 40.0, 28.0, 25.0), (270.0, 0.0, 90.0, 80.0));
            }
            s.put_shadowed(g, F_ARIAL10, sx, sy - 13.0, no_path, c15(0x7fe0), true);
            s.put_shadowed(g, F_ARIAL10, sx, sy - 2.0, &mk.label, c15(0x7fff), true);
        }
        self.draw_path_legend(g, s, &m);
    }

    fn draw_path_legend(&self, g: &mut Gfx, s: &Ui, m: &Marks) {
        use crate::enhanced::Anchor;
        s.anchor(Anchor::TopRight);
        let (x, y, w, h) = (566.0, 140.0, 224.0, 96.0);
        self.art.trans_fill(g, s, x, y, w, h, true);
        let white = c15(0x7fff);
        s.put_shadowed(g, F_MANUAL15, x + 10.0, y + 6.0, tr("STR_PATHVIEW_TITLE"), c15(0x7ff0), false);
        let rows: [([f32; 4], String); 3] = [
            (c15(0x03e0), tr("STR_PATHVIEW_JOINED").to_string()),
            (c15(0x7e00), tr("STR_PATHVIEW_CUT").to_string()),
            (c15(0x7fe0), trf("STR_PATHVIEW_MARKS", &[("COUNT", &m.marks.len())])),
        ];
        for (i, (c, t)) in rows.iter().enumerate() {
            let ry = y + 28.0 + 15.0 * i as f32;
            s.fill(g, x + 11.0, ry + 2.0, 12.0, 7.0, rgba(0.0, 0.0, 0.0, 0.6));
            s.fill(g, x + 12.0, ry + 3.0, 10.0, 5.0, *c);
            s.put(g, F_ARIAL10, x + 30.0, ry, t, white);
        }
        s.put(g, F_ARIAL10, x + 10.0, y + h - 18.0, tr("STR_PATHVIEW_CLOSE"), c15(0x6318));
        s.anchor(Anchor::Centre);
    }
}
