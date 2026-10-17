//! Building tools (the port's own, docs/PLAYING.md "Building tools"; none of it is in the original, and nothing changes until
//! the player uses them):
//!
//! * The measuring tape (' by default, controls.rs "tape"): the straight distance in yards between two points on the course,
//!   drawn as the hole tool's length line is (cursor_ui.rs draw_hole_reach: a white line two pixels wide at alpha 7 tenths,
//!   "N yards" in the game's font). Hold the key and move the pointer, then let go; or press it once, move, and click.
//!   From a tee of a hole the carry from that hole's tee point shows too. Esc, a right click or the key again put it away.
//! * The terrain brush's rectangle and fill on the Build Course panel: Shift+drag paints the rectangle between the press and
//!   the release, Alt+click (Option+click on a Mac) fills the region of the same tile type under the pointer, bounded by
//!   the club's land. While Shift is held during the drag or Alt is held over the course, the tiles are outlined and tinted
//!   and the total cost shows by the pointer. Every tile goes through the brush's own paint (App::paint_tiles), so every
//!   rule and price is the one-tile brush's, and the whole area is one undo step (undo.rs keeps the game at the press).
//!
//! * Drag to place (an OpenSGolf extra, off by default: crate::extras drag_place): with a bench, flower bed, ball washer,
//!   willow or scenic bridge armed, a drag lays a row of them along the straight line from the tile pressed to the tile let
//!   go, one per footprint. While the button is held the row's footprints are outlined (white where one fits, red where
//!   not) with the count and total cost by the pointer; on release each goes through App::edit_building, the one-click
//!   placement, so its rules and price are the click's, and the row is one undo step. Esc or a right click cancels it.
//!   Unique buildings (the clubhouse's kind: putting green to airstrip, upgraded by placing again), landmarks (each
//!   chosen and priced on its own), home sites (sold one by one, with their own sums) and paths (which the original
//!   already paints along a drag) are left as they are. With the extra off a drag does what the original's does.
//!
//! The original's Shift with a terrain brush does nothing (apply_tool hands it only to the path, building and elevation
//! tools); its Ctrl+click is the port's right click on a Mac, so the fill takes Alt.

use crate::app::*;
use crate::cursor_ui::{c15, label, RED, WHITE};
use crate::gfx::Gfx;
use crate::screens_ui::{BODY, SMALL};
use crate::ui::{line_rgba, Screen as Ui};
use sg_core::economy::Economy;
use sg_core::golfer as golf;
use sg_core::land;
use sg_core::lang::trf;
use sg_core::terrain::TILE_SIZE;
use std::collections::HashSet;

/// The most tiles a rectangle or a fill paints at once: the whole of the original's 50 x 50 map.
pub const MAX_AREA: usize = 2500;

/// The most tee tiles looked through for the hole a tape starts on.
const MAX_TEE: usize = 256;

/// A key press shorter than this (seconds) leaves the tape out until a click; a longer hold ends it when the key comes up.
const TAP_SECS: f64 = 0.35;

/// The measuring tape: its ends as ground points (world x, z).
#[derive(Clone, Copy, Debug)]
pub struct Tape {
    pub start: (f32, f32),
    /// The far end: the pointer's ground point while the tape is out, kept once it is set down.
    pub end: Option<(f32, f32)>,
    /// Set down (by a click, or by letting go of a held key).
    pub fixed: bool,
    /// The key is held (its repeats are not new presses).
    pub held: bool,
    pub pressed_at: f64,
}

/// What a press with the terrain brush did.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum AreaPress {
    /// Nothing: the brush paints as always.
    None,
    /// A rectangle started (painted when the button comes up).
    Rect,
    /// A fill painted at once.
    Filled,
}

#[derive(Default)]
pub struct BuildTools {
    pub tape: Option<Tape>,
    /// The rectangle's first corner (a tile) while Shift+drag is under way.
    pub rect_from: Option<(i32, i32)>,
    /// Alt is held (the fill's preview).
    pub alt: bool,
    /// The row's first tile while a drag to place is under way.
    pub row_from: Option<(i32, i32)>,
}

/// The most items one row places.
pub const MAX_ROW: usize = 256;

/// The kinds drag to place lays in rows: the garden items and the ball washer, which the original places one per click
/// wherever they fit.
pub fn row_kind(kind: i32) -> bool {
    matches!(kind, land::K_BENCH | land::K_FLOWERS | land::K_WASHER | land::K_WILLOW | land::K_BRIDGE)
}

/// The top left tiles of a row of `size` x `size` footprints from tile `a` to tile `b`: the tiles of the straight line
/// between them (each step one tile across, down or both, as Bresenham's line), keeping a tile when its footprint is clear
/// of the last kept one's. `a` always starts the row; at most `max` tiles.
pub fn row_tiles(a: (i32, i32), b: (i32, i32), size: i32, max: usize) -> Vec<(i32, i32)> {
    let size = size.max(1);
    let (dx, dy) = ((b.0 - a.0).abs(), -(b.1 - a.1).abs());
    let (sx, sy) = ((b.0 - a.0).signum(), (b.1 - a.1).signum());
    let (mut x, mut y, mut err) = (a.0, a.1, dx + dy);
    let mut out: Vec<(i32, i32)> = Vec::new();
    while out.len() < max {
        if out.last().is_none_or(|&(lx, ly)| (x - lx).abs().max((y - ly).abs()) >= size) {
            out.push((x, y));
        }
        if (x, y) == b {
            break;
        }
        let e2 = 2 * err;
        if e2 >= dy {
            err += dy;
            x += sx;
        }
        if e2 <= dx {
            err += dx;
            y += sy;
        }
    }
    out
}

/// Yards between two map points in 1/1024 tile units: 25 yards a tile (the exe's 0x40acd0 for the hole tool's length line;
/// sg_core holetool building_figures for a hole's straight length), truncated.
pub fn yards_between(p: (i32, i32), q: (i32, i32)) -> i32 {
    let (dx, dy) = ((q.0 - p.0) as f64, (q.1 - p.1) as f64);
    ((dx * dx + dy * dy).sqrt() * 25.0 / 1024.0) as i32
}

/// The tiles of the rectangle with corners `a` and `b` (either order) that `inside` keeps, row by row; None when the
/// rectangle has more than `max` tiles.
pub fn rect_tiles(a: (i32, i32), b: (i32, i32), max: usize, inside: impl Fn(i32, i32) -> bool) -> Option<Vec<(i32, i32)>> {
    let (x0, x1) = (a.0.min(b.0), a.0.max(b.0));
    let (y0, y1) = (a.1.min(b.1), a.1.max(b.1));
    if ((x1 - x0 + 1) as i64 * (y1 - y0 + 1) as i64) as usize > max {
        return None;
    }
    let mut out = Vec::new();
    for y in y0..=y1 {
        for x in x0..=x1 {
            if inside(x, y) {
                out.push((x, y));
            }
        }
    }
    Some(out)
}

/// The region joined edge to edge to `start` through tiles `same` keeps (start included when it is kept), row by row;
/// None when it has more than `max` tiles. Empty when `start` itself is not kept.
pub fn flood_tiles(start: (i32, i32), max: usize, same: impl Fn(i32, i32) -> bool) -> Option<Vec<(i32, i32)>> {
    if !same(start.0, start.1) {
        return Some(Vec::new());
    }
    let mut seen = HashSet::from([start]);
    let mut todo = vec![start];
    while let Some((x, y)) = todo.pop() {
        for n in [(x + 1, y), (x - 1, y), (x, y + 1), (x, y - 1)] {
            if !seen.contains(&n) && same(n.0, n.1) {
                if seen.len() >= max {
                    return None;
                }
                seen.insert(n);
                todo.push(n);
            }
        }
    }
    let mut out: Vec<(i32, i32)> = seen.into_iter().collect();
    out.sort_by_key(|&(x, y)| (y, x));
    Some(out)
}

/// The tiles that would change and their cost in total ($100 units), from each tile's own cost (None: left alone).
pub fn area_cost(tiles: &[(i32, i32)], cost: impl Fn(i32, i32) -> Option<i32>) -> (usize, i64) {
    tiles.iter().filter_map(|&(x, y)| cost(x, y)).fold((0, 0), |(n, sum), c| (n + 1, sum + c as i64))
}

/// The area the brush would paint now.
pub struct Area {
    pub tiles: Vec<(i32, i32)>,
    /// The rectangle's corners, for its outline.
    pub rect: Option<((i32, i32), (i32, i32))>,
    pub too_large: bool,
}

impl App {
    fn world_to_units(&self, (x, z): (f32, f32)) -> (i32, i32) {
        let k = 1024.0 / TILE_SIZE;
        let (hw, hh) = (self.terrain.w as f32 * TILE_SIZE * 0.5, self.terrain.h as f32 * TILE_SIZE * 0.5);
        (((x + hw) * k) as i32, ((z + hh) * k) as i32)
    }

    // ---- the measuring tape ------------------------------------------------------------------------------------------------

    /// The tape's key went down with the pointer over `at` (a ground point, None off the course). A repeat of a held key is
    /// not a new press; a press while the tape waits for its click puts it away.
    pub fn tape_key_down(&mut self, at: Option<(f32, f32)>) {
        match self.bt.tape {
            Some(t) if t.held => return,
            Some(t) if !t.fixed => {
                self.bt.tape = None;
                return;
            }
            _ => {}
        }
        self.bt.tape = at.map(|p| Tape { start: p, end: Some(p), fixed: false, held: true, pressed_at: self.clock });
        if self.bt.tape.is_none() {
            self.ui_sound(24);
        }
    }

    /// The tape's key came up: after a hold the tape is set down where the pointer is; after a tap it stays out until a click.
    pub fn tape_key_up(&mut self) {
        let clock = self.clock;
        if let Some(t) = self.bt.tape.as_mut().filter(|t| t.held) {
            t.held = false;
            if clock - t.pressed_at >= TAP_SECS {
                t.fixed = true;
            }
        }
    }

    /// The pointer's ground point each frame: the far end of a tape that is out.
    pub fn tape_pointer(&mut self, at: Option<(f32, f32)>) {
        if let Some(t) = self.bt.tape.as_mut().filter(|t| !t.fixed) {
            if at.is_some() {
                t.end = at;
            }
        }
    }

    /// A click on the course while there is a tape: a left click sets a tape that is out down, or puts a set one away; a
    /// right click puts it away. True when the click was the tape's.
    pub fn tape_click(&mut self, left: bool) -> bool {
        let Some(t) = self.bt.tape.as_mut() else { return false };
        if left && !t.fixed {
            t.fixed = true;
            t.held = false;
        } else {
            self.bt.tape = None;
        }
        true
    }

    /// Esc: the tape and a rectangle or row under way are put away. True when there was one.
    pub fn build_tools_cancel(&mut self) -> bool {
        let had = self.bt.tape.is_some() || self.bt.rect_from.is_some() || self.bt.row_from.is_some();
        self.bt.tape = None;
        self.bt.rect_from = None;
        self.bt.row_from = None;
        had
    }

    /// The hole whose tee the tile is on (the tee region joined to it holds the hole's back or forward tee) and that tee.
    fn tee_hole_at(&self, (a, b): (i32, i32)) -> Option<(i32, (i32, i32))> {
        let tee = |x: i32, y: i32| self.terrain.inside(x, y) && self.terrain.type_at(x, y) == 0;
        let region = flood_tiles((a, b), MAX_TEE, tee).unwrap_or_else(|| vec![(a, b)]);
        if region.is_empty() {
            return None;
        }
        let next = self.club.next_hole;
        (1..19).find_map(|h| {
            let hr = &self.club.holes[h as usize];
            if hr.par == 0 && h != next {
                return None;
            }
            [hr.back, hr.fwd].into_iter().find(|t| t.0 != 0 && region.contains(t)).map(|t| (h, t))
        })
    }

    /// The tape: the line, a mark at each end and the yards at its middle, with the carry from the hole's tee under them.
    pub(crate) fn draw_tape(&self, g: &mut Gfx, s: &Ui) {
        let Some(t) = self.bt.tape else { return };
        let Some(end) = t.end else { return };
        let (Some(p), Some(q)) = (self.screen_of_world(t.start.0, t.start.1), self.screen_of_world(end.0, end.1)) else { return };
        s.line(g, p.0, p.1, q.0, q.1, 2.0, line_rgba(WHITE as u32, 7));
        for e in [p, q] {
            s.fill(g, e.0 - 2.0, e.1 - 1.0, 5.0, 3.0, c15(WHITE));
        }
        let (u0, u1) = (self.world_to_units(t.start), self.world_to_units(end));
        let yards = yards_between(u0, u1);
        let (mx, my) = (((p.0 as i32 + q.0 as i32) / 2) as f32, ((p.1 as i32 + q.1 as i32) / 2) as f32);
        label(g, s, mx, my, &trf("STR_CURSOR_YARDS", &[("YARDS", &yards)]), BODY, c15(WHITE));
        let tile = self.terrain.tile_of(t.start.0, t.start.1);
        if let Some((h, tee)) = self.tee_hole_at(tile) {
            let carry = yards_between((tee.0 * 1024 + 512, tee.1 * 1024 + 512), u1);
            label(g, s, mx, my + 13.0, &trf("STR_TAPE_CARRY", &[("HOLE", &h), ("YARDS", &carry)]), SMALL, c15(WHITE));
        }
    }

    // ---- the rectangle and the fill ----------------------------------------------------------------------------------------

    /// What painting tile x, y with the armed brush would cost ($100 units), None where the brush leaves it alone: the
    /// checks of App::paint_tiles before the money (off the land, an obstacle, nothing to change, a tee or cup during a
    /// tournament, a landmark, the clubhouse or a building). The hole tool can still turn down a tee or green tile.
    pub fn paint_cost_at(&self, x: i32, y: i32) -> Option<i32> {
        if !self.terrain.inside(x, y) {
            return None;
        }
        let pe = &PAINT[self.paint_idx];
        let ti = self.terrain.tile_index(x, y);
        let old = self.terrain.ty[ti];
        if let Some(l) = self.land.as_ref() {
            let li = (x * land::mn() + y) as usize;
            if !l.in_play(x, y) || l.flags[li] & land::flag::OBSTACLE != 0 {
                return None;
            }
            if let Some(o) = l.object_on(x, y) {
                let obj = l.objects[o];
                let edge = sg_core::data::buildings().get(obj.kind as usize).map(|b| b.1).unwrap_or(1);
                if (obj.kind == land::K_LANDMARK && obj.sub <= 16) || obj.kind == land::K_CLUBHOUSE || edge > 1 {
                    return None;
                }
            }
        }
        if old as i32 == pe.ty && pe.vbyte == self.terrain.variation[ti] as i32 {
            return None;
        }
        let cup = self.course.flags.get(sg_core::course::idx(x, y)).is_some_and(|&f| f & sg_core::course::f::CUP != 0);
        if self.club.game & golf::game::TOURNAMENT != 0 && (old == 0 || cup) {
            return None;
        }
        Some(Economy::terrain_clear_units(old as i32) + Economy::terrain_cost_units(pe.ty))
    }

    /// The tiles on the club's land (or on the map, with no land yet).
    fn on_land(&self, x: i32, y: i32) -> bool {
        self.terrain.inside(x, y) && self.land.as_ref().is_none_or(|l| l.in_play(x, y))
    }

    /// The rectangle from `from` to `to`, on the club's land.
    pub fn rect_area(&self, from: (i32, i32), to: (i32, i32)) -> Area {
        match rect_tiles(from, to, MAX_AREA, |x, y| self.on_land(x, y)) {
            Some(tiles) => Area { tiles, rect: Some((from, to)), too_large: false },
            None => Area { tiles: Vec::new(), rect: Some((from, to)), too_large: true },
        }
    }

    /// The region of tile `at`'s type joined to it, on the club's land.
    pub fn fill_area(&self, at: (i32, i32)) -> Area {
        if !self.on_land(at.0, at.1) {
            return Area { tiles: Vec::new(), rect: None, too_large: false };
        }
        let ty = self.terrain.type_at(at.0, at.1);
        match flood_tiles(at, MAX_AREA, |x, y| self.on_land(x, y) && self.terrain.type_at(x, y) == ty) {
            Some(tiles) => Area { tiles, rect: None, too_large: false },
            None => Area { tiles: Vec::new(), rect: None, too_large: true },
        }
    }

    /// The area a press or release would paint now: the rectangle under way, or with Alt held the fill under the pointer.
    fn area_now(&self) -> Option<Area> {
        if !self.edit || self.tool != 0 || !self.has_hit || self.screen != Screen::Play {
            return None;
        }
        let at = self.terrain.tile_of(self.hit_x, self.hit_z);
        match self.bt.rect_from {
            Some(from) => Some(self.rect_area(from, at)),
            None if self.bt.alt => Some(self.fill_area(at)),
            None => None,
        }
    }

    /// Paints an area with the armed brush as one stroke; a fill or rectangle over the limit only says so.
    pub fn paint_area(&mut self, area: &Area) {
        if area.too_large {
            self.ui_sound(24);
            self.show_toast(&trf("STR_TOAST_AREA_TOO_LARGE", &[("MAX", &MAX_AREA)]));
            return;
        }
        if !area.tiles.is_empty() {
            self.paint_tiles(&area.tiles);
        }
    }

    /// A left press on the course with a tool armed (the pointer's ground point already picked): with the terrain brush,
    /// Shift starts a rectangle and Alt fills.
    pub fn brush_area_press(&mut self, shift: bool, alt: bool) -> AreaPress {
        if !self.edit || self.tool != 0 || !self.has_hit || !(shift || alt) {
            return AreaPress::None;
        }
        let at = self.terrain.tile_of(self.hit_x, self.hit_z);
        if shift {
            self.bt.rect_from = Some(at);
            return AreaPress::Rect;
        }
        let area = self.fill_area(at);
        self.paint_area(&area);
        AreaPress::Filled
    }

    /// The left button came up: a rectangle under way is painted to the tile under the pointer.
    pub fn brush_area_release(&mut self) {
        let Some(from) = self.bt.rect_from.take() else { return };
        if !self.has_hit || self.tool != 0 || !self.edit {
            return;
        }
        let to = self.terrain.tile_of(self.hit_x, self.hit_z);
        let area = self.rect_area(from, to);
        self.paint_area(&area);
    }

    /// The preview, over the brush's picture: the tiles that would change tinted, the area's edge outlined (red when it is
    /// too large) and the total cost above the pointer, red when the club cannot afford it.
    pub(crate) fn draw_area_preview(&self, g: &mut Gfx, s: &Ui) {
        let Some(area) = self.area_now() else { return };
        let corner = |x: i32, y: i32| self.screen_units(x * 1024, y * 1024);
        let set: HashSet<(i32, i32)> = area.tiles.iter().copied().collect();
        let tint = [1.0, 1.0, 1.0, 0.3];
        for &(x, y) in area.tiles.iter().filter(|&&(x, y)| self.paint_cost_at(x, y).is_some()) {
            s.quad(g, [corner(x, y), corner(x + 1, y), corner(x + 1, y + 1), corner(x, y + 1)], tint);
        }
        // Terrain::drawLine one pixel wide at alpha 7 tenths, as the building tool's footprint (0x419e6a)
        let edge = line_rgba(if area.too_large { RED } else { WHITE } as u32, 7);
        if let Some((a, b)) = area.rect {
            let (x0, x1, y0, y1) = (a.0.min(b.0), a.0.max(b.0) + 1, a.1.min(b.1), a.1.max(b.1) + 1);
            let c = [corner(x0, y0), corner(x1, y0), corner(x1, y1), corner(x0, y1)];
            for i in 0..4 {
                s.line(g, c[i].0, c[i].1, c[(i + 1) % 4].0, c[(i + 1) % 4].1, 1.0, edge);
            }
        } else {
            for &(x, y) in &area.tiles {
                for (n, p, q) in [
                    ((x, y - 1), (x, y), (x + 1, y)),
                    ((x + 1, y), (x + 1, y), (x + 1, y + 1)),
                    ((x, y + 1), (x, y + 1), (x + 1, y + 1)),
                    ((x - 1, y), (x, y), (x, y + 1)),
                ] {
                    if !set.contains(&n) {
                        let (p, q) = (corner(p.0, p.1), corner(q.0, q.1));
                        s.line(g, p.0, p.1, q.0, q.1, 1.0, edge);
                    }
                }
            }
        }
        let (a, b) = self.terrain.tile_of(self.hit_x, self.hit_z);
        let (sx, sy) = self.screen_units(a * 1024 + 512, b * 1024 + 512);
        if area.too_large {
            label(g, s, sx, sy - 22.0, &trf("STR_AREA_TOO_LARGE", &[("MAX", &MAX_AREA)]), BODY, c15(RED));
            return;
        }
        let (n, total) = area_cost(&area.tiles, |x, y| self.paint_cost_at(x, y));
        let text = if self.econ.sandbox {
            trf("STR_AREA_TILES", &[("TILES", &n)])
        } else {
            trf("STR_AREA_COST", &[("TILES", &n), ("MONEY", &crate::ui::money(-total * 100))])
        };
        let ok = self.econ.sandbox || self.econ.affordable(total as f64 * Economy::UNIT, self.hole_numbers.len());
        label(g, s, sx, sy - 22.0, &text, BODY, c15(if ok { WHITE } else { RED }));
    }
}

impl App {
    // ---- drag to place (an OpenSGolf extra) --------------------------------------------------------------------------------

    /// The building tool's kind when a press now would start a row (the extra on, a row kind armed, not removing).
    fn row_tool(&self, remove: bool) -> Option<i32> {
        let kind = self.build_idx as i32;
        (crate::extras::drag_place() && self.edit && self.tool == 4 && !remove && self.screen == Screen::Play && row_kind(kind))
            .then_some(kind)
    }

    /// A left press on the course with a tool armed: with drag to place on and a row kind armed, the row starts here (it is
    /// placed when the button comes up). True when it did; otherwise the tool works as in the original.
    pub fn row_press(&mut self, remove: bool) -> bool {
        if self.row_tool(remove).is_none() || !self.has_hit {
            return false;
        }
        self.bt.row_from = Some(self.terrain.tile_of(self.hit_x, self.hit_z));
        true
    }

    /// The row's top left tiles from `from` to `to` for the armed kind.
    pub fn row_area(&self, from: (i32, i32), to: (i32, i32)) -> Vec<(i32, i32)> {
        let size = sg_core::data::buildings().get(self.build_idx).map(|b| b.1).unwrap_or(1);
        row_tiles(from, to, size, MAX_ROW)
    }

    /// What one click with the armed row kind would cost at tile x, y ($100 units), None where it would be refused: a garden
    /// item's own price (land::tile_item_cost), a ball washer's price and clearing where its footprint fits.
    pub fn row_cost_at(&self, x: i32, y: i32) -> Option<i32> {
        let kind = self.build_idx as i32;
        let l = self.land.as_ref()?;
        if !self.terrain.inside(x, y) {
            return None;
        }
        if matches!(kind, land::K_BENCH | land::K_FLOWERS | land::K_WILLOW | land::K_BRIDGE) {
            return l.tile_item_cost(x, y, kind, self.exe_theme()).ok();
        }
        if !self.build_available(kind as usize) || !l.in_play(x, y) {
            return None;
        }
        let clear = self.footprint_at(kind, x, y).1?;
        Some(sg_core::data::buildings()[kind as usize].2 + clear)
    }

    /// The left button came up: a row under way is placed, item by item through the one-click placement, from its first
    /// tile to the tile under the pointer (undo.rs keeps the whole row as one step).
    pub fn row_release(&mut self) {
        let Some(from) = self.bt.row_from.take() else { return };
        if !self.has_hit || self.row_tool(false).is_none() {
            return;
        }
        let to = self.terrain.tile_of(self.hit_x, self.hit_z);
        self.place_row(from, to);
    }

    /// Places a row of the armed kind from tile `from` to tile `to`, one click's placement per item.
    pub fn place_row(&mut self, from: (i32, i32), to: (i32, i32)) {
        for (x, y) in self.row_area(from, to) {
            self.edit_building(x, y, false);
        }
    }

    /// The row's preview, while the button is held: each footprint outlined (white where it would go down, red where it would
    /// be refused) and the number of items with their total cost above the pointer, red when the club cannot afford it.
    pub(crate) fn draw_row_preview(&self, g: &mut Gfx, s: &Ui) {
        let Some(from) = self.bt.row_from else { return };
        if !self.has_hit || self.row_tool(false).is_none() {
            return;
        }
        let to = self.terrain.tile_of(self.hit_x, self.hit_z);
        let size = sg_core::data::buildings().get(self.build_idx).map(|b| b.1).unwrap_or(1);
        let tiles = self.row_area(from, to);
        for &(a, b) in &tiles {
            let ok = self.row_cost_at(a, b).is_some();
            let (x0, y0, x1, y1) = (a * 1024, b * 1024, (a + size) * 1024 - 1, (b + size) * 1024 - 1);
            let c = [self.screen_units(x0, y0), self.screen_units(x1, y0), self.screen_units(x1, y1), self.screen_units(x0, y1)];
            // Terrain::drawLine one pixel wide at alpha 7 tenths, as the building tool's footprint (0x419e6a)
            let edge = line_rgba(if ok { WHITE } else { RED } as u32, 7);
            for i in 0..4 {
                s.line(g, c[i].0, c[i].1, c[(i + 1) % 4].0, c[(i + 1) % 4].1, 1.0, edge);
            }
            if ok {
                s.quad(g, c, [1.0, 1.0, 1.0, 0.2]);
            }
        }
        let (n, total) = area_cost(&tiles, |x, y| self.row_cost_at(x, y));
        let text = if self.econ.sandbox {
            trf("STR_ROW_ITEMS", &[("ITEMS", &n)])
        } else {
            trf("STR_ROW_COST", &[("ITEMS", &n), ("MONEY", &crate::ui::money(-total * 100))])
        };
        let ok = self.econ.sandbox || self.econ.affordable(total as f64 * Economy::UNIT, self.hole_numbers.len());
        // above the item's own name over the footprint under the pointer (cursor_ui.rs draw_footprint)
        let (sx, sy) = self.screen_units(to.0 * 1024 + 512, to.1 * 1024 + 512);
        label(g, s, sx, sy - 25.0, &text, BODY, c15(if ok { WHITE } else { RED }));
    }
}

/// Test hooks for stills (docs/PLAYING.md): SG_TAPE="x0,y0,x1,y1" (tiles, fractions allowed) sets a tape down between
/// those points; SG_AREA="r,x,y" holds a rectangle from tile x,y to the pointer (SG_CURSOR_TILE) and SG_AREA="f" Alt for
/// the fill's preview; SG_ROW="x,y" holds a drag to place from tile x,y to the pointer (with the drag to place extra on and
/// a row kind armed).
pub fn test_hooks(app: &mut App) {
    let t = TILE_SIZE;
    let world = |app: &App, x: f32, y: f32| (x * t - app.terrain.w as f32 * t * 0.5, y * t - app.terrain.h as f32 * t * 0.5);
    if let Ok(v) = std::env::var("SG_TAPE") {
        let v: Vec<f32> = v.split(',').filter_map(|s| s.trim().parse().ok()).collect();
        if v.len() >= 4 {
            let (start, end) = (world(app, v[0], v[1]), world(app, v[2], v[3]));
            app.bt.tape = Some(Tape { start, end: Some(end), fixed: true, held: false, pressed_at: 0.0 });
        }
    }
    if let Ok(v) = std::env::var("SG_AREA") {
        let mut it = v.split(',');
        match it.next().map(str::trim) {
            Some("r") => {
                let n: Vec<i32> = it.filter_map(|s| s.trim().parse().ok()).collect();
                if n.len() >= 2 {
                    app.bt.rect_from = Some((n[0], n[1]));
                }
            }
            Some("f") => app.bt.alt = true,
            _ => {}
        }
    }
    if let Ok(v) = std::env::var("SG_ROW") {
        let n: Vec<i32> = v.split(',').filter_map(|s| s.trim().parse().ok()).collect();
        if n.len() >= 2 {
            app.bt.row_from = Some((n[0], n[1]));
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn yards_are_25_a_tile() {
        assert_eq!(yards_between((0, 0), (1024, 0)), 25);
        assert_eq!(yards_between((512, 512), (512 + 3 * 1024, 512 + 4 * 1024)), 125);
        // truncated, as the exe's length line
        assert_eq!(yards_between((0, 0), (1024, 1024)), 35);
        assert_eq!(yards_between((5, 5), (5, 5)), 0);
    }

    #[test]
    fn rectangle_in_either_order_kept_to_the_map() {
        let inside = |x: i32, y: i32| (0..10).contains(&x) && (0..10).contains(&y);
        let a = rect_tiles((2, 3), (4, 4), 100, inside).unwrap();
        assert_eq!(a, vec![(2, 3), (3, 3), (4, 3), (2, 4), (3, 4), (4, 4)]);
        assert_eq!(rect_tiles((4, 4), (2, 3), 100, inside).unwrap(), a);
        // one tile: the plain brush's click
        assert_eq!(rect_tiles((5, 5), (5, 5), 100, inside).unwrap(), vec![(5, 5)]);
        // over the edge: only the tiles on the map
        assert_eq!(rect_tiles((8, 8), (11, 11), 100, inside).unwrap().len(), 4);
        // too large
        assert!(rect_tiles((0, 0), (10, 10), 100, inside).is_none());
        assert!(rect_tiles((0, 0), (9, 9), 100, inside).is_some());
    }

    /// A small map: '.' rough, '=' fairway, '#' land the club does not own (never the same as the start).
    fn map() -> Vec<&'static [u8]> {
        vec![b"..==.", b".==..", b"..#==", b"=.#.=", b"=.=.."]
    }

    #[test]
    fn fill_takes_the_joined_region_of_one_type() {
        let m = &map();
        let same = |c: u8| move |x: i32, y: i32| (0..5).contains(&x) && (0..5).contains(&y) && m[y as usize][x as usize] == c;
        // the fairway at the top joins (2,0) (3,0) (1,1) (2,1); not (3,2) (4,2) (4,3) across the rough
        assert_eq!(flood_tiles((2, 0), 100, same(b'=')).unwrap(), vec![(2, 0), (3, 0), (1, 1), (2, 1)]);
        assert_eq!(flood_tiles((4, 3), 100, same(b'=')).unwrap(), vec![(3, 2), (4, 2), (4, 3)]);
        // not across corners: (0,3) and (0,4) only
        assert_eq!(flood_tiles((0, 4), 100, same(b'=')).unwrap(), vec![(0, 3), (0, 4)]);
        // the land not owned bounds it and is never taken
        assert_eq!(flood_tiles((2, 2), 100, same(b'=')).unwrap(), vec![]);
        // the rough on the left: 7 tiles, cut off from the rough on the right by the fairway and the land not owned
        let rough = flood_tiles((0, 0), 100, same(b'.')).unwrap();
        assert_eq!(rough, vec![(0, 0), (1, 0), (0, 1), (0, 2), (1, 2), (1, 3), (1, 4)]);
        // a cap: the region is refused, not cut short
        assert!(flood_tiles((0, 0), 6, same(b'.')).is_none());
        assert!(flood_tiles((0, 0), 7, same(b'.')).is_some());
    }

    #[test]
    fn a_row_takes_each_tile_of_the_line_or_each_footprint() {
        // a click without moving: the one tile
        assert_eq!(row_tiles((4, 4), (4, 4), 1, MAX_ROW), vec![(4, 4)]);
        // straight along a row or column, either way
        assert_eq!(row_tiles((2, 5), (6, 5), 1, MAX_ROW), vec![(2, 5), (3, 5), (4, 5), (5, 5), (6, 5)]);
        assert_eq!(row_tiles((3, 7), (3, 4), 1, MAX_ROW), vec![(3, 7), (3, 6), (3, 5), (3, 4)]);
        // a diagonal goes corner to corner
        assert_eq!(row_tiles((0, 0), (3, 3), 1, MAX_ROW), vec![(0, 0), (1, 1), (2, 2), (3, 3)]);
        // a slope: one tile a column, from the press to the release, no tile twice
        let t = row_tiles((0, 0), (6, 2), 1, MAX_ROW);
        assert_eq!(t.len(), 7);
        assert_eq!((t[0], t[6]), ((0, 0), (6, 2)));
        assert!(t.windows(2).all(|w| w[1].0 == w[0].0 + 1 && (0..=1).contains(&(w[1].1 - w[0].1))));
        // reversed, the same line the other way
        let mut r = row_tiles((6, 2), (0, 0), 1, MAX_ROW);
        r.reverse();
        assert_eq!(r.len(), 7);
        // footprints of 2 tiles: every other tile, never overlapping
        assert_eq!(row_tiles((0, 0), (5, 0), 2, MAX_ROW), vec![(0, 0), (2, 0), (4, 0)]);
        assert_eq!(row_tiles((0, 0), (4, 4), 2, MAX_ROW), vec![(0, 0), (2, 2), (4, 4)]);
        for (a, b) in [((0, 0), (9, 3)), ((5, 9), (0, 1)), ((2, 2), (2, 9))] {
            let t = row_tiles(a, b, 3, MAX_ROW);
            assert!(t.windows(2).all(|w| (w[1].0 - w[0].0).abs().max((w[1].1 - w[0].1).abs()) >= 3), "{a:?} {b:?}");
        }
        // the cap
        assert_eq!(row_tiles((0, 0), (1000, 0), 1, 10).len(), 10);
    }

    #[test]
    fn rows_are_for_the_items_placed_one_per_click() {
        for k in [land::K_BENCH, land::K_FLOWERS, land::K_WASHER, land::K_WILLOW, land::K_BRIDGE] {
            assert!(row_kind(k));
        }
        // paths already paint along a drag; landmarks, home sites and the buildings stay one per click
        for k in [land::K_PATH, land::K_LANDMARK, land::K_HOME_SITE, land::K_SNACK_BAR, land::K_CLUBHOUSE, land::K_TV_TOWER] {
            assert!(!row_kind(k));
        }
    }

    #[test]
    fn cost_sums_the_tiles_that_change() {
        let tiles = [(0, 0), (1, 0), (2, 0), (3, 0)];
        // tile 2 is left alone (same type), the others cost 3, 3 and a wetland's 50 + 3
        let cost = |x: i32, _y: i32| match x {
            2 => None,
            3 => Some(53),
            _ => Some(3),
        };
        assert_eq!(area_cost(&tiles, cost), (3, 59));
        assert_eq!(area_cost(&[], cost), (0, 0));
    }
}
