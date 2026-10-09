//! The pointer on the course while a tool is armed, as the exe's main frame draws it (all in 0x40f5c0):
//!
//! * Terrain brushes (0x41a990): the brush's tile picture from `Data/<theme>.pcx` at half strength over the tile under the
//!   pointer (sand traps turn with Tab), a see-through tree on a tile the woods brushes would plant, and "Tricky Green" beside
//!   the green brush's tricky variant. There is no outline and no brush size.
//! * Undo (the terrain type -2 at 0x41aa4b): what a right click would undo there, named beside the tile (0x40a160), green
//!   when something would be undone.
//! * The elevation tools (0x415ec2, and the tile loop at 0x41064e): a black grid over every tile, the heights of the
//!   vertices around the one under the pointer ("+N" above sea level, "0" at it; the vertex under the pointer in the body
//!   font, the 2 x 2 block's in the large one), a purple mark on the vertex (tool 0) or on every vertex shown (tool 2), and
//!   the 2 x 2 block outlined (tool 1).
//! * Buildings and amenities (0x419d34): the footprint outlined white where it fits and red where it does not, the building
//!   itself see-through (or red where it does not fit), facing the way Tab turned it; amenities are named over the
//!   footprint, and a clearing charge shows as "-N" dollars under it.
//!
//! Not drawn yet: the pin flag over the green brush while a hole waits for its green, the tee-to-pointer yardage line (both
//! belong to the hole tools), the bench, willow and bridge previews, the landmark's reach ring and the Home Site's lot value
//! box.

use crate::app::*;
use crate::gfx::{Gfx, Mode, Uniforms, Vert};
use crate::ui::{rgb, Screen as Ui};
use sg_core::land;

/// The exe's 15-bit colours.
fn c15(v: u16) -> [f32; 4] {
    rgb(((v >> 10) & 31) as f32 / 31.0, ((v >> 5) & 31) as f32 / 31.0, (v & 31) as f32 / 31.0)
}
const WHITE: u16 = 0x7fff;
const RED: u16 = 0x7d08;
const GREEN: u16 = 0x23e8;
const PURPLE: u16 = 0x6090;

/// Tile types of the terrain buttons, in the order of their pictures on `Data/<theme>.pcx` (0x4c2d28).
const ICON_TYPES: [i32; 16] = [0, 1, 7, 4, 9, 10, 17, 13, 2, 3, 5, 8, 11, 12, 14, 18];
/// A picture's cell on the sheet (0x445aa0): eight 57 x 40 cells to a row.
const ICON_W: f32 = 57.0;
const ICON_H: f32 = 40.0;

/// Font sizes standing in for the exe's small (0x519fd8), body (0x51b360) and large (0x519928) font objects. PLACEHOLDER: the
/// objects' sizes are not decoded.
const SMALL: f32 = 11.0;
const BODY: f32 = 13.0;
const LARGE: f32 = 16.0;

/// How a preview sprite is drawn: see-through (queue flag 0x200, the exe's alpha 0.6) or as the refused red silhouette
/// (flag 0x800: tint 0x7c00 at strength 0xf8).
#[derive(Clone, Copy, PartialEq)]
enum Look {
    Ghost,
    Red,
}

/// What the armed tool shows.
enum Tool {
    /// A terrain brush painting this tile type.
    Paint(i32),
    Undo,
    /// An elevation tool: 0 one vertex, 1 the 2 x 2 block, 2 the area.
    Elevation(usize),
    /// The building tool with this exe building id (0 the Pathway).
    Building(i32),
}

impl App {
    fn cursor_tool(&self) -> Option<Tool> {
        if !self.edit || !self.has_hit || self.screen != Screen::Play {
            return None;
        }
        match self.tool {
            0 => Some(Tool::Paint(PAINT[self.paint_idx].ty)),
            1 if self.pstate.elev_tool < 3 => Some(Tool::Elevation(self.pstate.elev_tool)),
            2 => Some(Tool::Building(land::K_PATH)),
            4 => Some(Tool::Building(self.build_idx as i32)),
            5 => Some(Tool::Undo),
            _ => None,
        }
    }

    /// The exe draws the tool under the pointer only while the pointer is above the dock (y below 480), and not on a tile
    /// flagged 0x4000.
    fn cursor_tile_ok(&self, a: i32, b: i32) -> bool {
        self.pstate.mouse.1 < 480.0
            && !(sg_core::course::inside(a, b) && self.course.flags[sg_core::course::idx(a, b)] & sg_core::course::f::GROWING != 0)
    }

    fn cursor_tile(&self) -> (i32, i32) {
        self.terrain.tile_of(self.hit_x, self.hit_z)
    }

    /// The paint brush's variant, the exe's 0x5a34f0 (random when a brush is picked, 0 for the green; Tab adds one).
    fn brush_variant(&self) -> i32 {
        match self.paint_variant {
            Some((k, v)) if k == self.paint_idx => v,
            _ => 0,
        }
    }

    /// The exe's zoom (1, 2 or 4).
    fn exe_zoom(&self) -> f32 {
        (self.zoom / ZOOM_UNIT).max(1.0)
    }

    /// Screen point (800 x 600) of a map point in 1/1024 tile units, on the ground.
    fn screen_units(&self, x: i32, y: i32) -> (f32, f32) {
        let (wx, wz) = self.units_to_world(x, y);
        self.screen_of_world(wx, wz).unwrap_or((-1000.0, -1000.0))
    }

    /// The building tool's footprint at the pointer: its edge (the lot's size plus the building's level, an upgrade being one
    /// larger) and the clearing charge in $100 units, None where it will not fit (0x40db90; the Pathway pays half).
    fn footprint_at(&self, kind: i32, a: i32, b: i32) -> (i32, Option<i32>) {
        let level = if kind >= 6 { self.lot_level(kind) } else { 0 };
        let size = land::BUILDINGS.get(kind as usize).map(|b| b.1).unwrap_or(1) + level;
        let clear = self.land.as_ref().and_then(|l| l.fits(a, b, size, kind, self.exe_theme()));
        (size, clear.map(|c| if kind == 0 { c / 2 } else { c }))
    }

    // ---- the sprites, in the course pass ----------------------------------------------------------------------------------------

    /// The building being placed and the tree a woods brush would plant, see-through, after the other sprites.
    pub fn draw_cursor_preview(&mut self, g: &mut Gfx, u: &Uniforms) {
        let Some(tool) = self.cursor_tool() else { return };
        let (a, b) = self.cursor_tile();
        if !self.cursor_tile_ok(a, b) {
            return;
        }
        let quarter = ((self.rot / 90.0).round() as i32 % 4 + 4) % 4;
        match tool {
            Tool::Paint(ty) if (13..=16).contains(&ty) => {
                // no tree over a tile that already has trees
                if (13..=16).contains(&self.terrain.type_at(a, b)) {
                    return;
                }
                let v = self.brush_variant();
                let field = |x: i32, y: i32| (self.noise.field(x, y) / 32) & 3;
                let (mut sprite, mut pal) = match ty {
                    13 => (0x19b, 0x25 + field(a << 8, b << 8)),
                    15 => (0x195, 0x32 + field(a << 7, b << 7)),
                    _ if (v / 3) & 1 != 0 && (1..=3).contains(&self.exe_theme()) => (0x198, 0x2a),
                    _ => (0x192, 0x29),
                };
                sprite += v % 3;
                if ty == 16 {
                    (sprite, pal) = (0xf9, 0x36);
                }
                let (body, shadow) = self.decor_sprite(sprite as u16, pal as u8);
                let Some(body) = body else { return };
                let (x, z) = self.terrain.tile_centre(a, b);
                let frame = self.sprites[body].s.frames_per_view - 1;
                let facing = (a - 2 * b).rem_euclid(4);
                for si in [shadow, Some(body)].into_iter().flatten() {
                    self.preview_quad(g, u, si, quarter, facing, frame, x, z, Look::Ghost);
                }
            }
            Tool::Building(kind) if kind == land::K_LANDMARK || (6..=15).contains(&kind) => {
                let (size, clear) = self.footprint_at(kind, a, b);
                let look = if clear.is_some() { Look::Ghost } else { Look::Red };
                let files: Vec<(String, bool)> = if kind == land::K_LANDMARK {
                    let Some(t) = self.next_landmark() else { return };
                    vec![(sg_core::objects::LANDMARKS[t as usize].to_string(), false)]
                } else {
                    let level = self.lot_level(kind).min(1) as u16;
                    sg_core::objects::building_layers(kind, level, self.exe_theme()).iter().map(|l| (l.file.to_string(), l.flat)).collect()
                };
                // centred on the footprint, as the exe's preview is
                let (cx, cz) = self.terrain.tile_centre(a, b);
                let off = (size - 1) as f32 * sg_core::terrain::TILE_SIZE * 0.5;
                let (x, z) = (cx + off, cz + off);
                let mut layers = Vec::new();
                for (file, flat) in files {
                    let body = self.sprite_for(&format!("{file}.flc"), false, None);
                    let shadow = if flat { None } else { self.sprite_for(&format!("{file}Shadow.flc"), true, None) };
                    if let Some(body) = body {
                        layers.push((body, shadow, flat));
                    }
                }
                let facing = self.turn & 3;
                for &(body, _, flat) in &layers {
                    if flat {
                        self.preview_quad(g, u, body, quarter, facing, 0, x, z, look);
                    }
                }
                if look == Look::Ghost {
                    for &(_, shadow, _) in &layers {
                        if let Some(sh) = shadow {
                            self.preview_quad(g, u, sh, quarter, facing, 0, x, z, look);
                        }
                    }
                }
                for &(body, _, flat) in &layers {
                    if !flat {
                        self.preview_quad(g, u, body, quarter, facing, 0, x, z, look);
                    }
                }
            }
            _ => {}
        }
    }

    /// One sprite frame standing at a ground point, facing as draw_props turns an object.
    #[allow(clippy::too_many_arguments)]
    fn preview_quad(&mut self, g: &mut Gfx, u: &Uniforms, si: usize, quarter: i32, facing: i32, frame: i32, x: f32, z: f32, look: Look) {
        let sp = &self.sprites[si].s;
        let view = match sp.views {
            v if v >= 8 => (quarter * 2 + facing * 2).rem_euclid(8),
            v if v >= 4 => (quarter + facing).rem_euclid(v),
            2 => quarter % 2,
            _ => 0,
        };
        let i = sp.frame_index(view, frame);
        let (w, h, ax, ay) = (sp.w as f32, sp.h as f32, sp.anchor_x as f32, sp.anchor_y as f32);
        let tex = match look {
            Look::Ghost => self.sprite_texture(g, si, view, frame),
            Look::Red => match self.red_tex.get(&(si, i)) {
                Some(&t) => t,
                None => {
                    let mut img = self.sprites[si].s.frames[i].clone();
                    // PLACEHOLDER: the renderer's tint call (vtable +0x4c) is not decoded; the colour is taken 0xf8/0x100 of
                    // the way to red
                    for p in img.px.chunks_mut(4) {
                        p[0] = ((p[0] as u32 * 8 + 248 * 248) >> 8) as u8;
                        p[1] = ((p[1] as u32 * 8) >> 8) as u8;
                        p[2] = ((p[2] as u32 * 8) >> 8) as u8;
                    }
                    let t = g.texture(&img, false);
                    self.red_tex.insert((si, i), t);
                    t
                }
            },
        };
        let mv = self.mv;
        let (rx, ry, rz, ux, uy, uz) = (mv[0], mv[4], mv[8], mv[1], mv[5], mv[9]);
        let s = sg_core::sprites::SPRITE_UNITS_PER_PIXEL;
        let y = self.terrain.height_at(x, z);
        let (l, r, t, b) = (-ax * s, (w - ax) * s, ay * s, -(h - ay) * s);
        let col = [1.0, 1.0, 1.0, if look == Look::Ghost { 0.6 } else { 1.0 }];
        let c = |cx: f32, cy: f32, tu: f32, tv: f32| {
            Vert::new(x + rx * cx + ux * cy, y + ry * cx + uy * cy, z + rz * cx + uz * cy, tu, tv).col(col)
        };
        g.quad(Mode::Flat, Some(tex), u, [c(l, t, 0.0, 0.0), c(r, t, 1.0, 0.0), c(r, b, 1.0, 1.0), c(l, b, 0.0, 1.0)]);
    }

    // ---- the pictures, lines and labels, over the course -----------------------------------------------------------------------

    /// The 2D part of the tool under the pointer, drawn over the course before the interface.
    pub fn draw_cursor_overlay(&mut self, g: &mut Gfx) {
        let Some(tool) = self.cursor_tool() else { return };
        let s = Ui::new(self.draw_w, self.draw_h);
        match tool {
            Tool::Elevation(k) => self.draw_elevation_cursor(g, &s, k),
            Tool::Paint(ty) => self.draw_brush_cursor(g, &s, ty),
            Tool::Undo => {
                let (a, b) = self.cursor_tile();
                if !self.cursor_tile_ok(a, b) {
                    return;
                }
                let Some(l) = self.land.as_ref() else { return };
                let (text, undoes) = l.undo_label(a, b);
                let (sx, sy) = self.screen_units(a * 1024 + 512, b * 1024 + 512);
                top_text(g, &s, sx + 4.0, sy - 12.0, &text, BODY, c15(if undoes { GREEN } else { WHITE }));
            }
            Tool::Building(kind) => self.draw_footprint(g, &s, kind),
        }
    }

    /// The brush's tile picture at half strength (0x474440 with alpha 0.5), its top left half a tile up and left of the tile's
    /// centre; every button showing this type draws, and any tree type also draws the trees button's picture.
    fn draw_brush_cursor(&self, g: &mut Gfx, s: &Ui, ty: i32) {
        let (a, b) = self.cursor_tile();
        if !self.cursor_tile_ok(a, b) {
            return;
        }
        let (sx, sy) = self.screen_units(a * 1024 + 512, b * 1024 + 512);
        let ez = self.exe_zoom();
        let (x0, y0) = (sx - ez * 8.0, sy - ez * 5.0);
        let sheet = &self.tool_tiles[self.exe_theme().min(3) as usize];
        let half = [1.0, 1.0, 1.0, 0.5];
        let cell = |g: &mut Gfx, i: usize| {
            s.image_part_tint(g, sheet, x0, y0, (i % 8) as f32 * ICON_W, (i / 8) as f32 * ICON_H, ICON_W, ICON_H, half);
        };
        let v = self.brush_variant();
        if ty == 7 {
            // the sand trap's four shapes, turned with the view
            let quarter = ((self.rot / 90.0).round() as i32 % 4 + 4) % 4;
            let k = (v - quarter + 4).rem_euclid(4);
            cell(g, if k == 0 { 2 } else { 16 + [0, 3, 1, 2][k as usize] });
        } else {
            for (i, &t) in ICON_TYPES.iter().enumerate() {
                if t == ty || ((13..=16).contains(&ty) && i == 7) {
                    cell(g, i);
                }
            }
        }
        if ty == 1 && v & 1 != 0 {
            top_text(g, s, sx + 4.0, sy - 16.0, "Tricky Green", SMALL, c15(WHITE));
        }
    }

    /// The elevation tools' grid, heights and marks.
    fn draw_elevation_cursor(&self, g: &mut Gfx, s: &Ui, tool: usize) {
        let black = c15(0);
        // the grid: two edges of every tile, in the tile loop before anything else is drawn on the tile
        for x in 0..self.terrain.w {
            for y in 0..self.terrain.h {
                let p0 = self.screen_units(x * 1024, y * 1024);
                if !(-60.0..860.0).contains(&p0.0) || !(-60.0..660.0).contains(&p0.1) {
                    continue;
                }
                let px = self.screen_units(x * 1024 + 1023, y * 1024);
                let py = self.screen_units(x * 1024, y * 1024 + 1023);
                s.line(g, p0.0, p0.1, px.0, px.1, 1.0, black);
                s.line(g, p0.0, p0.1, py.0, py.1, 1.0, black);
            }
        }
        // the vertex under the pointer; its height is the exe's vertex (cx, cy - 1)
        let (cx, cy) = self.terrain.corner_of(self.hit_x, self.hit_z);
        let square = tool == 1;
        let in_block = |x: i32, y: i32| square && (x == cx || x == cx + 1) && (y == cy - 1 || y == cy);
        let pt = |x: i32, y: i32| self.screen_units(x * 1024, y * 1024);
        for x in cx - 1..=cx + 1 + square as i32 {
            for y in cy - 1 - square as i32..=cy + 1 {
                let (sx, sy) = pt(x, y);
                if (0..=self.terrain.w).contains(&x) && (0..=self.terrain.h).contains(&y) {
                    let size = if in_block(x, y) {
                        LARGE
                    } else if x == cx && y == cy {
                        BODY
                    } else {
                        SMALL
                    };
                    let h = self.terrain.corner_at(x, y);
                    let text = if h > 0 { format!("+{h}") } else { h.to_string() };
                    top_text(g, s, sx, sy - 15.0, &text, size, c15(WHITE));
                }
                if tool == 0 && x == cx && y == cy {
                    s.fill(g, sx - 2.0, sy - 1.0, 5.0, 3.0, c15(PURPLE));
                }
                if tool == 2 {
                    s.fill(g, sx - 1.0, sy - 1.0, 3.0, 2.0, c15(PURPLE));
                }
            }
        }
        if square {
            // the block's outline (Terrain::drawLine, width 3)
            let c = [pt(cx, cy - 1), pt(cx + 1, cy - 1), pt(cx + 1, cy), pt(cx, cy)];
            for i in 0..4 {
                let (p, q) = (c[i], c[(i + 1) % 4]);
                s.line(g, p.0, p.1, q.0, q.1, 3.0, c15(PURPLE));
            }
        }
    }

    /// The building tool's footprint, name and clearing charge.
    fn draw_footprint(&self, g: &mut Gfx, s: &Ui, kind: i32) {
        let (a, b) = self.cursor_tile();
        if !self.cursor_tile_ok(a, b) || (kind == land::K_LANDMARK && self.next_landmark().is_none()) {
            return;
        }
        let (size, clear) = self.footprint_at(kind, a, b);
        let mut col = c15(if clear.is_some() { WHITE } else { RED });
        let (x0, y0, x1, y1) = (a * 1024, b * 1024, (a + size) * 1024 - 1, (b + size) * 1024 - 1);
        let c = [self.screen_units(x0, y0), self.screen_units(x1, y0), self.screen_units(x1, y1), self.screen_units(x0, y1)];
        for i in 0..4 {
            let (p, q) = (c[i], c[(i + 1) % 4]);
            s.line(g, p.0, p.1, q.0, q.1, 1.0, col);
        }
        let (sx, sy) = self.screen_units(a * 1024 + size * 512, b * 1024 + size * 512);
        if kind == land::K_BRIDGE && self.terrain.type_at(a, b) != land::T_WATER as i32 {
            col = c15(RED);
        }
        if kind <= 6 {
            let name = if kind == land::K_LANDMARK {
                let n = sg_core::vips::landmark_short_name(self.next_landmark().unwrap_or(-1));
                let mut ch = n.chars();
                ch.next().map(|f| f.to_uppercase().collect::<String>() + ch.as_str()).unwrap_or_default()
            } else {
                land::BUILDINGS[kind as usize].0.to_string()
            };
            top_text(g, s, sx, sy - 8.0, &name, BODY, col);
        }
        if kind != land::K_HOME_SITE {
            if let Some(cost) = clear.filter(|&c| c > 0) {
                top_text(g, s, sx, sy + 8.0, &format!("-{}", cost * 100), BODY, c15(RED));
            }
        }
    }
}

/// Centred text placed by its top, as the exe's text calls place it (ours draw from the baseline). APPROXIMATION: the
/// baseline is taken four fifths of the size below the top.
fn top_text(g: &mut Gfx, s: &Ui, cx: f32, top: f32, text: &str, size: f32, c: [f32; 4]) {
    if !text.is_empty() {
        s.text_centered(g, cx, top + size * 0.8, text, size, c);
    }
}
