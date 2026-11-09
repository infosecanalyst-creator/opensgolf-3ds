//! Undo for building and terrain (the port's own, docs/PLAYING.md "Undo"). At a click on the course the parts of the game
//! that the building and terrain tools change are kept: the tiles and their corners, the objects, the holes' tees and cups and
//! the money booked to building. When the button comes up and the click changed the course (its land, terrain or holes), what
//! it changed goes on the undo list, each tile, corner and object as it was before and as the click left it. Ctrl+Z (Cmd+Z on
//! a Mac) puts back only that, where it is still as the click left it, with the money the click took or gave: the clock, the
//! golfers, the rest of the game and what has been built since (a tournament's TV towers) go on as they are. The
//! course editor's heightmap and land changes are kept as a whole saved game instead, as no time passes there. The list is
//! not saved and starts empty with each game.

use crate::app::App;
use crate::gfx::Gfx;
use sg_core::economy;
use sg_core::land::{self, Land, Object};
use std::collections::HashSet;
use std::hash::{Hash, Hasher};

/// The most clicks the list keeps (the oldest go first).
pub const MAX_STEPS: usize = 30;

/// The ledger columns building books to: its costs and refunds, and a home site's lot sold or bought back. The others are
/// the club's takings and bills, which go on while a drag lasts.
const BUILD_COLUMNS: [usize; 3] = [economy::LEDGER_HOME_SITES, economy::LEDGER_BUILD_COURSE, economy::LEDGER_FACILITIES];

#[derive(Default)]
pub struct Undo {
    /// What each click that changed the course changed, oldest first.
    steps: Vec<Step>,
    /// The course as it was at the button's press, until it comes up.
    pending: Option<Box<Before>>,
}

impl Undo {
    pub fn clear(&mut self) {
        self.steps.clear();
        self.pending = None;
    }

    /// Whether there is a click to undo (the touch Undo button shows only then, touch.rs).
    pub fn can_undo(&self) -> bool {
        !self.steps.is_empty()
    }

    /// An amount floated over a tile by the click under way (App::float_change), to float again given back if it is undone.
    pub fn note_float(&mut self, units: i32, at: (i32, i32)) {
        if let Some(b) = self.pending.as_mut() {
            b.floats.push((units, at));
        }
    }

    fn push(&mut self, s: Step) {
        self.steps.push(s);
        if self.steps.len() > MAX_STEPS {
            self.steps.remove(0);
        }
    }
}

enum Step {
    /// A click on the course: what it changed, as it was before.
    Click(Box<Click>),
    /// The whole game as a save, loaded back as a saved game is (the course editor's heightmap and land, editor.rs).
    Save(Vec<u8>),
}

/// What a vector held before a click at each place the click changed, and what the click left there. Undo puts a place back
/// only while it still holds what the click left: what has changed it since (a tournament's TV towers, the game) stays. A
/// place the click added (a new object on the end of the list) has nothing before; the lists never get shorter (an object
/// is freed, not taken off the list).
struct Was<T> {
    at: Vec<(u32, Option<T>, T)>,
}

impl<T: Copy + PartialEq> Was<T> {
    fn of(before: &[T], after: &[T]) -> Self {
        let at = after
            .iter()
            .enumerate()
            .filter(|&(i, v)| before.get(i) != Some(v))
            .map(|(i, &v)| (i as u32, before.get(i).copied(), v))
            .collect();
        Was { at }
    }

    /// The places that hold something else now than what the click left there.
    fn changed_since<'a>(&'a self, v: &'a [T]) -> impl Iterator<Item = usize> + 'a {
        self.at.iter().filter(move |&&(i, _, left)| v.get(i as usize) != Some(&left)).map(|&(i, ..)| i as usize)
    }

    /// Puts back what each place held before the click while it still holds what the click left, unless `keep` keeps it.
    fn put_back(&self, v: &mut [T], keep: impl Fn(usize) -> bool) {
        for &(i, was, left) in &self.at {
            if let (Some(w), Some(x)) = (was, v.get_mut(i as usize)) {
                if *x == left && !keep(i as usize) {
                    *x = w;
                }
            }
        }
    }
}

/// The same object in the same place (a home site's owner and lot value, and whether an object is joined to the
/// clubhouse, change as the game goes on).
fn same(x: &Object, y: &Object) -> bool {
    (x.kind, x.a, x.b) == (y.kind, y.a, y.b)
}

/// The land's tiles, corners and objects a click changed (not the growth counters, which are the simulation's).
struct LandWas {
    ty: Was<u8>,
    flags: Was<u16>,
    layer: Was<u8>,
    var: Was<u8>,
    height: Was<u8>,
    undo: Was<u8>,
    refund: Was<i8>,
    objects: Was<Object>,
}

impl LandWas {
    fn of(b: &Land, a: &Land) -> Self {
        LandWas {
            ty: Was::of(&b.ty, &a.ty),
            flags: Was::of(&b.flags, &a.flags),
            layer: Was::of(&b.layer, &a.layer),
            var: Was::of(&b.var, &a.var),
            height: Was::of(&b.height, &a.height),
            undo: Was::of(&b.undo, &a.undo),
            refund: Was::of(&b.refund, &a.refund),
            objects: Was::of(&b.objects, &a.objects),
        }
    }

    /// The tiles that hold something else now than what the click left on them.
    fn changed_since(&self, l: &Land) -> HashSet<usize> {
        let mut s: HashSet<usize> = self.ty.changed_since(&l.ty).collect();
        s.extend(self.flags.changed_since(&l.flags));
        s.extend(self.layer.changed_since(&l.layer));
        s.extend(self.var.changed_since(&l.var));
        s.extend(self.undo.changed_since(&l.undo));
        s.extend(self.refund.changed_since(&l.refund));
        s
    }

    /// Puts back the tiles not in `kept` (a tile goes back whole or not at all), the corners and the objects. An object the
    /// click took away comes back only with all of its tiles: if one of them has changed since, the object stays gone and
    /// its tiles stay as they are (they join `kept`).
    fn put_back(&self, l: &mut Land, kept: &mut HashSet<usize>) {
        let moved = self.put_back_objects(l, kept);
        let keep = |i: usize| kept.contains(&i);
        self.ty.put_back(&mut l.ty, keep);
        self.flags.put_back(&mut l.flags, keep);
        self.layer.put_back(&mut l.layer, keep);
        self.var.put_back(&mut l.var, keep);
        self.undo.put_back(&mut l.undo, keep);
        self.refund.put_back(&mut l.refund, keep);
        self.height.put_back(&mut l.height, |_| false);
        // an object back in another record: its tiles point there (Land::place keeps a footprint's object in var)
        for (o, from, to) in moved {
            for t in footprint(l, &o) {
                if l.var[t] == from as u8 {
                    l.var[t] = to as u8;
                }
            }
        }
    }

    /// Puts back the objects, before their tiles. One the click added is freed, as Land::remove_object frees one. One it
    /// took away comes back in its own record, or in a free one (or a new one on the end) if its own has been used since:
    /// a tournament's TV tower takes the first free record. Returns the objects that came back in another record, from
    /// where to where.
    fn put_back_objects(&self, l: &mut Land, kept: &mut HashSet<usize>) -> Vec<(Object, usize, usize)> {
        let mut moved = Vec::new();
        for &(i, was, left) in &self.objects.at {
            let i = i as usize;
            let Some(now) = l.objects.get(i).copied() else { continue };
            let Some(w) = was.filter(|w| w.kind >= 0) else {
                // the click put an object in a free record, or on the end of the list
                if same(&now, &left) {
                    l.objects[i] = was.unwrap_or(Object { kind: -1, ..now });
                }
                continue;
            };
            let tiles = footprint(l, &w);
            if tiles.iter().any(|t| kept.contains(t)) {
                kept.extend(tiles);
            } else if same(&now, &left) {
                l.objects[i] = w;
            } else if let Some(n) = l.objects.iter().position(|o| o.kind == -1) {
                l.objects[n] = w;
                moved.push((w, i, n));
            } else if l.objects.len() < 0xff {
                l.objects.push(w);
                moved.push((w, i, l.objects.len() - 1));
            } else {
                // no record is free (0xff is a tile's "no object"): the object stays gone, and its tiles as they are
                kept.extend(tiles);
            }
        }
        moved
    }
}

/// The land tiles under an object.
fn footprint(l: &Land, o: &Object) -> Vec<usize> {
    let (s, n) = (l.footprint_size(o), land::mn());
    let inside = |v: i32| (0..n).contains(&v);
    (0..s)
        .flat_map(|r| (0..s).map(move |c| (o.a + r, o.b + c)))
        .filter(|&(a, b)| inside(a) && inside(b))
        .map(|(a, b)| (a * n + b) as usize)
        .collect()
}

/// The hole tool's part of a hole record: its tees, its cup and its par (the hole's statistics are the golfers').
#[derive(Clone, Copy, PartialEq)]
struct HoleWas {
    par: i32,
    game_par: i32,
    back: (i32, i32),
    fwd: (i32, i32),
    pin: (i32, i32),
    /// The Multiple tees extra's forward and middle tees.
    extra: [Option<(i32, i32)>; 2],
}

impl HoleWas {
    fn of(h: &sg_core::golfer::Hole) -> Self {
        HoleWas { par: h.par, game_par: h.game_par, back: h.back, fwd: h.fwd, pin: h.pin, extra: [h.front_tee, h.middle_tee] }
    }
}

/// The course as it is at a press, to tell afterwards what the click changed.
struct Before {
    fingerprint: u64,
    terrain: [Vec<u8>; 4],
    corner: Vec<i8>,
    land: Land,
    holes: Vec<(HoleWas, i32)>,
    next_hole: i32,
    /// Course B's next hole (the Second course extra, sg_core::second; its records follow Course A's in `holes`).
    next_hole_b: i32,
    free_landmarks: u32,
    money: [f64; 3],
    floats: Vec<(i32, (i32, i32))>,
}

/// What one click changed, as it was before.
struct Click {
    /// The terrain's tile types, variants, texture sets and paths, and its corners.
    terrain: [Was<u8>; 4],
    corner: Was<i8>,
    land: LandWas,
    /// The holes whose tees, cup or par it changed, before and after, and what it added to each hole's building cost.
    holes: Vec<(usize, HoleWas, HoleWas)>,
    build_cost: Vec<(usize, i32)>,
    next_hole: Option<(i32, i32)>,
    next_hole_b: Option<(i32, i32)>,
    /// The free landmarks (one bit each) it used up, and any it gave.
    free_landmarks: (u32, u32),
    /// What it booked to each of the building columns (negative for money paid), and the amounts it floated over its tiles.
    money: [f64; 3],
    floats: Vec<(i32, (i32, i32))>,
}

/// Hole records per course.
const HOLES: usize = sg_core::golfer::HOLE_RECORDS;

impl App {
    /// Course A's hole records, then Course B's when there is one (the Second course extra, sg_core::second).
    fn both_courses(&self) -> impl Iterator<Item = &sg_core::golfer::Hole> {
        let b = if self.club.course_b.is_some() { self.club.course_holes(true) } else { &[] };
        self.club.course_holes(false).iter().take(HOLES).chain(b.iter())
    }

    /// A fingerprint of the course: what building and terrain tools change (not what the simulation changes as time goes
    /// by: growth, weeds, the undo records).
    fn course_fingerprint(&self) -> u64 {
        let mut h = std::collections::hash_map::DefaultHasher::new();
        self.terrain.ty.hash(&mut h);
        self.terrain.corner.hash(&mut h);
        self.terrain.path_kind.hash(&mut h);
        if let Some(l) = &self.land {
            l.ty.hash(&mut h);
            l.height.hash(&mut h);
            if let Ok(v) = serde_json::to_vec(&l.objects) {
                v.hash(&mut h);
            }
        }
        h.finish()
    }

    /// The building columns' totals over every year (a drag may cross into a new one).
    fn build_money(&self) -> [f64; 3] {
        BUILD_COLUMNS.map(|c| self.econ.ledger.iter().map(|y| y[c]).sum())
    }

    /// A press on the course: keeps the course as it is now, in case the click changes it.
    pub fn undo_press(&mut self) {
        // the interface's call is an action (replay.rs): logged, then applied by App::apply
        if self.rec.depth_zero() {
            self.act(sg_core::action::Action::UndoPress);
            return;
        }
        let Some(land) = self.land.clone() else { return };
        let t = &self.terrain;
        self.undo.pending = Some(Box::new(Before {
            fingerprint: self.course_fingerprint(),
            terrain: [t.ty.clone(), t.variation.clone(), t.set.clone(), t.path_kind.clone()],
            corner: t.corner.clone(),
            land,
            holes: self.both_courses().map(|h| (HoleWas::of(h), h.build_cost)).collect(),
            next_hole: self.club.next_hole,
            next_hole_b: self.club.course_next_hole(true),
            free_landmarks: self.club.free_landmarks,
            money: self.build_money(),
            floats: Vec::new(),
        }));
    }

    /// The button came up: what the click changed goes on the undo list when it changed the course.
    pub fn undo_release(&mut self) {
        // the interface's call is an action (replay.rs): logged, then applied by App::apply
        if self.rec.depth_zero() && self.undo.pending.is_some() {
            self.act(sg_core::action::Action::UndoRelease);
            return;
        }
        let Some(b) = self.undo.pending.take() else { return };
        let Some(land) = self.land.as_ref() else { return };
        if self.course_fingerprint() == b.fingerprint {
            return;
        }
        let t = &self.terrain;
        let now = [&t.ty, &t.variation, &t.set, &t.path_kind];
        let mut holes = Vec::new();
        let mut build_cost = Vec::new();
        for (i, (h, (was, cost))) in self.both_courses().zip(&b.holes).enumerate() {
            if HoleWas::of(h) != *was {
                holes.push((i, *was, HoleWas::of(h)));
            }
            if h.build_cost != *cost {
                build_cost.push((i, h.build_cost - cost));
            }
        }
        let money = self.build_money();
        let click = Click {
            terrain: std::array::from_fn(|k| Was::of(&b.terrain[k], now[k])),
            corner: Was::of(&b.corner, &t.corner),
            land: LandWas::of(&b.land, land),
            holes,
            build_cost,
            next_hole: (self.club.next_hole != b.next_hole).then_some((b.next_hole, self.club.next_hole)),
            next_hole_b: (self.club.course_next_hole(true) != b.next_hole_b).then_some((b.next_hole_b, self.club.course_next_hole(true))),
            free_landmarks: (b.free_landmarks & !self.club.free_landmarks, self.club.free_landmarks & !b.free_landmarks),
            money: std::array::from_fn(|k| money[k] - b.money[k]),
            floats: b.floats,
        };
        self.undo.push(Step::Click(Box::new(click)));
    }

    /// A step on the undo list made by something other than a click on the course (the course editor's heightmap and
    /// land, editor.rs): the game as it is now.
    pub fn undo_checkpoint(&mut self) {
        if self.land.is_none() {
            return;
        }
        if let Ok(b) = self.save_bytes(1) {
            self.undo.push(Step::Save(b));
        }
    }

    /// Ctrl+Z: the course as it was before the last click that changed it, and the money that click took or gave. Returns
    /// false with nothing to undo.
    pub fn undo_step(&mut self, g: &mut Gfx) -> bool {
        // a click's step is the Undo action (replay.rs); the course editor's whole game is not logged
        if self.rec.depth_zero() && !self.undo_top_is_save() {
            return self.act(sg_core::action::Action::Undo);
        }
        match self.undo.steps.pop() {
            Some(Step::Click(c)) => self.undo_click(&c),
            Some(Step::Save(b)) => self.undo_save(g, b),
            None => false,
        }
    }

    fn undo_click(&mut self, c: &Click) -> bool {
        let Some(land) = self.land.as_mut() else { return false };
        let homes =
            |l: &Land| -> Vec<(i32, i32)> { l.objects.iter().filter(|o| o.kind == land::K_HOME_SITE).map(|o| (o.a, o.b)).collect() };
        let homes_now = homes(land);
        let t = &mut self.terrain;
        // a tile goes back only while it holds all the click left on it, in the land and in the terrain: what has been built
        // on it since (a tournament's TV towers) stays
        let (w, n) = (t.w as usize, land::mn() as usize);
        let to_land = |o: usize| if o % w < n && o / w < n { (o % w) * n + o / w } else { usize::MAX };
        let mut kept = c.land.changed_since(land);
        for (was, v) in c.terrain.iter().zip([&t.ty, &t.variation, &t.set, &t.path_kind]) {
            kept.extend(was.changed_since(v).map(to_land));
        }
        c.land.put_back(land, &mut kept);
        for (was, v) in c.terrain.iter().zip([&mut t.ty, &mut t.variation, &mut t.set, &mut t.path_kind]) {
            was.put_back(v, |o| kept.contains(&to_land(o)));
        }
        c.corner.put_back(&mut t.corner, |_| false);
        land.green_looks_to(&mut self.terrain);
        // a home site the undo takes away loses its residents and its record, as when it is demolished (App::undo_tile)
        let homes_then = homes(land);
        for (a, b) in homes_now.into_iter().filter(|h| !homes_then.contains(h)) {
            self.club.evict(a, b);
            self.home_removed(a, b);
        }
        // (records past Course A's 20 are Course B's: sg_core::second)
        let rec = |i: usize| (i >= HOLES, i % HOLES);
        for &(i, w, left) in &c.holes {
            let (b, k) = rec(i);
            let Some(h) = self.club.course_hole_mut(b, k) else { continue };
            if HoleWas::of(h) == left {
                (h.par, h.game_par, h.back, h.fwd, h.pin) = (w.par, w.game_par, w.back, w.fwd, w.pin);
                [h.front_tee, h.middle_tee] = w.extra;
            }
        }
        for &(i, d) in &c.build_cost {
            let (b, k) = rec(i);
            if let Some(h) = self.club.course_hole_mut(b, k) {
                h.build_cost -= d;
            }
        }
        if let Some((was, left)) = c.next_hole {
            if self.club.next_hole == left {
                self.club.next_hole = was;
            }
        }
        if let Some((was, left)) = c.next_hole_b {
            if self.club.course_next_hole(true) == left {
                self.club.set_course_next_hole(true, was);
            }
        }
        // the free landmarks it used come back, and any won since stay
        let (used, gave) = c.free_landmarks;
        self.club.free_landmarks = (self.club.free_landmarks & !gave) | used;
        // the money back, booked against the columns the click booked to
        let mut back = 0.0;
        for (k, &col) in BUILD_COLUMNS.iter().enumerate() {
            if c.money[k] != 0.0 {
                self.econ.book(col, -c.money[k]);
                back -= c.money[k];
            }
        }
        if back > 0.0 {
            self.econ.earn(back);
        } else if back < 0.0 {
            self.econ.spend(-back);
        }
        // each tile's amount floats again over it, given back
        for &(units, at) in &c.floats {
            self.float_money(-units, at);
        }
        self.after_object_change();
        true
    }

    /// Undoes the last click as Ctrl+Z does, without the graphics a saved game's step needs (tests).
    #[cfg(test)]
    pub(crate) fn undo_click_step(&mut self) -> bool {
        self.undo_click_top()
    }

    /// Whether the last step is the course editor's whole game (undone with `undo_step`, not an action: the editor keeps
    /// no log).
    pub fn undo_top_is_save(&self) -> bool {
        matches!(self.undo.steps.last(), Some(Step::Save(_)))
    }

    /// The Undo action (replay.rs): the last click that changed the course put back. False with no click to undo (the
    /// course editor's step stays for `undo_step`).
    pub(crate) fn undo_click_top(&mut self) -> bool {
        match self.undo.steps.pop() {
            Some(Step::Click(c)) => self.undo_click(&c),
            Some(s) => {
                self.undo.steps.push(s);
                false
            }
            None => false,
        }
    }

    /// The course editor's step: the whole game as it was, loaded as a saved game is.
    fn undo_save(&mut self, g: &mut Gfx, b: Vec<u8>) -> bool {
        let Ok(s) = crate::app::read_save_bytes(&b, "undo") else { return false };
        // what the player was doing stays as it was: the open panel, its tool and the view
        let steps = std::mem::take(&mut self.undo.steps);
        let (panel, alt, edit, tool) = (self.panel, self.pstate.alt, self.edit, self.tool);
        if self.load_save_with(g, s, true).is_err() {
            self.undo.steps = steps;
            return false;
        }
        self.undo.steps = steps;
        self.panel = panel;
        self.pstate.alt = alt;
        self.edit = edit;
        self.tool = tool;
        true
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn was_puts_back_only_what_the_click_left() {
        let before = vec![1u8, 2, 3];
        let mut v = vec![1u8, 9, 8, 4];
        let w = Was::of(&before, &v);
        assert_eq!(w.at, vec![(1, Some(2), 9), (2, Some(3), 8), (3, None, 4)]);
        // since the click: another change elsewhere, one at a place the click changed, and one more on the end
        v[0] = 7;
        v[2] = 6;
        v.push(5);
        assert_eq!(w.changed_since(&v).collect::<Vec<_>>(), [2]);
        w.put_back(&mut v, |_| false);
        assert_eq!(v, vec![7, 2, 6, 4, 5], "only the click's own places go back, and the list stays as long");
        let mut v = vec![1u8, 9, 8];
        w.put_back(&mut v, |i| i == 1);
        assert_eq!(v, vec![1, 9, 3]);
    }
}
