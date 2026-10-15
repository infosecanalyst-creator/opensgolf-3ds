//! OpenSGolf extras: golfer insight (extras.rs). Two observers of the simulation that only read it, each with its own tick
//! in Preferences > OpenSGolf extras:
//!
//! * the landing map: after every golfers' tick the balls in play are followed (sg_core::golfer's ball flags and places,
//!   nothing written back); when a tee shot or an approach comes to rest (a putt, struck on the green, is left out) its
//!   resting place, where it first came down, its number on the hole and the ground it stopped on go into a rolling store
//!   of the hole's last `MAX_SHOTS` shots. Shift+D (or the Information menu) shows them as dots on the course for the hole
//!   under the pointer (or the last one looked at, or the one Hole Stats last showed), coloured by the shot's number, with a
//!   legend counting where they ended up;
//! * comments by hole: every thought a golfer has that changed the golfer's mood (as the original's comment counters count
//!   them, golfer.rs `event`: the thought's history entry carries the change's sign) is tallied by hole for this month and
//!   last month, and a page (Shift+I, the Information menu, or a tab after F10 on the tabbed reports) lists each hole's
//!   most frequent ones with their counts, in the words of the Customise screen's dialogue labels or the F2 report's lines.
//!
//! Both are kept with the game only while their tick is on (a save made with them off is unchanged), and cost a pass over
//! the golfer slots per tick (a few compares each) and a counter per thought.
use crate::app::*;
use crate::extras::{draw_button, draw_tab};
use crate::gfx::Gfx;
use crate::info_ui::c15;
use crate::render::Rect;
use crate::ui::{rgba, Fnt, Screen as Ui, F_ARIAL10, F_MANUAL15, F_MANUAL20};
use serde::{Deserialize, Serialize};
use sg_core::course::t;
use sg_core::golfer::flag;
use sg_core::lang::{tr, trf};
use std::collections::{BTreeMap, VecDeque};

/// The shots the landing map keeps per hole (the oldest go first).
pub const MAX_SHOTS: usize = 200;
/// Shot numbers the map tells apart: the tee shot, the second, the third, and the fourth or later.
pub const SHOT_KINDS: usize = 4;
/// Where a shot can end: fairway, rough, sand, water, green (or holed), anything else (trees, rocks, a ravine, out of
/// bounds, a tee).
pub const GROUNDS: usize = 6;
pub const FAIRWAY: u8 = 0;
pub const ROUGH: u8 = 1;
pub const SAND: u8 = 2;
pub const WATER: u8 = 3;
pub const GREEN: u8 = 4;
pub const OTHER: u8 = 5;
/// Thoughts tallied: the ids below 50, as the F2 report lists them (50 is a story's line).
const MAX_ID: u32 = 50;
/// The thoughts the page lists per hole.
const TOP: usize = 3;
/// Map units are kept as sixteenths (64 steps a tile), to keep the save small.
const SCALE: i32 = 16;

/// A ground class of a tile type (sg_core::course::t).
pub fn ground_of(ty: u8) -> u8 {
    match ty {
        2 | 3 => FAIRWAY,
        4..=6 => ROUGH,
        7..=9 => SAND,
        17..=19 => WATER,
        1 => GREEN,
        _ => OTHER,
    }
}

/// A shot's end, as the save keeps it: where it came to rest and where it first came down (map units / 16), its number on
/// the hole (1 the tee shot .. 4 the fourth or later) and the ground it stopped on.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct Shot(pub i32, pub i32, pub i32, pub i32, pub u8, pub u8);

impl Shot {
    pub fn new(rest: (i32, i32), land: (i32, i32), num: i32, ground: u8) -> Shot {
        Shot(rest.0 / SCALE, rest.1 / SCALE, land.0 / SCALE, land.1 / SCALE, num.clamp(1, SHOT_KINDS as i32) as u8, ground.min(OTHER))
    }
    /// Where it came to rest (map units).
    pub fn rest(&self) -> (i32, i32) {
        (self.0 * SCALE + SCALE / 2, self.1 * SCALE + SCALE / 2)
    }
    /// Where it first came down (map units).
    pub fn land(&self) -> (i32, i32) {
        (self.2 * SCALE + SCALE / 2, self.3 * SCALE + SCALE / 2)
    }
    pub fn num(&self) -> usize {
        self.4.clamp(1, SHOT_KINDS as u8) as usize
    }
    pub fn ground(&self) -> usize {
        self.5.min(OTHER) as usize
    }
}

/// The landing map's store: each hole's last shots, and the hole's tee and cup tiles they were played to (a hole built
/// again elsewhere starts afresh).
#[derive(Clone, Debug, Default, PartialEq, Serialize, Deserialize)]
pub struct Landings {
    #[serde(default, skip_serializing_if = "BTreeMap::is_empty")]
    pub holes: BTreeMap<u8, VecDeque<Shot>>,
    #[serde(default, skip_serializing_if = "BTreeMap::is_empty")]
    pub layout: BTreeMap<u8, [i32; 4]>,
}

impl Landings {
    pub fn is_empty(&self) -> bool {
        self.holes.is_empty()
    }

    /// Keeps a shot on a hole laid out from tee to cup as `layout`, the hole's oldest going when it has `MAX_SHOTS`.
    pub fn push(&mut self, hole: u8, layout: [i32; 4], s: Shot) {
        if self.layout.get(&hole).is_some_and(|l| *l != layout) {
            self.holes.remove(&hole);
        }
        self.layout.insert(hole, layout);
        let v = self.holes.entry(hole).or_default();
        v.push_back(s);
        while v.len() > MAX_SHOTS {
            v.pop_front();
        }
    }

    /// A hole's shots, oldest first.
    pub fn shots(&self, hole: u8) -> impl Iterator<Item = &Shot> {
        self.holes.get(&hole).into_iter().flatten()
    }

    /// How many of a hole's shots ended on each ground.
    pub fn counts(&self, hole: u8) -> [u32; GROUNDS] {
        let mut c = [0; GROUNDS];
        for s in self.shots(hole) {
            c[s.ground()] += 1;
        }
        c
    }
}

/// A hole's thoughts in a month: by thought id, how often, the latest argument (for the line's words) and its tone (1 a
/// good change of mood, 2 a bad one).
pub type HoleTally = BTreeMap<u8, (u32, i32, u8)>;

/// The comments by hole: this month's tally (`month` the game's month count, tick >> 10) and last month's.
#[derive(Clone, Debug, Default, PartialEq, Serialize, Deserialize)]
pub struct Tally {
    #[serde(default)]
    pub month: u32,
    #[serde(default, skip_serializing_if = "BTreeMap::is_empty")]
    pub now: BTreeMap<u8, HoleTally>,
    #[serde(default, skip_serializing_if = "BTreeMap::is_empty")]
    pub last: BTreeMap<u8, HoleTally>,
}

impl Tally {
    pub fn is_empty(&self) -> bool {
        self.now.is_empty() && self.last.is_empty()
    }

    /// Moves on to `month`: this month becomes last month when it is the next one; after a longer gap (or going back, a
    /// game loaded from earlier) nothing carries over.
    pub fn roll(&mut self, month: u32) {
        if month == self.month {
            return;
        }
        self.last = if month == self.month.wrapping_add(1) { std::mem::take(&mut self.now) } else { BTreeMap::new() };
        self.now.clear();
        self.month = month;
    }

    pub fn add(&mut self, month: u32, hole: u8, id: u8, arg: i32, tone: u8) {
        self.roll(month);
        let e = self.now.entry(hole).or_default().entry(id).or_insert((0, arg, tone));
        e.0 += 1;
        e.1 = arg;
        e.2 = tone;
    }

    fn months(&self, last: bool) -> &BTreeMap<u8, HoleTally> {
        if last {
            &self.last
        } else {
            &self.now
        }
    }

    /// A hole's `n` most frequent thoughts this month (or last), most frequent first (ties: the lower id first), as (id,
    /// count, argument, tone).
    pub fn top(&self, hole: u8, last: bool, n: usize) -> Vec<(u8, u32, i32, u8)> {
        let mut v: Vec<_> = self.months(last).get(&hole).into_iter().flatten().map(|(&id, &(c, a, t))| (id, c, a, t)).collect();
        v.sort_by(|a, b| b.1.cmp(&a.1).then(a.0.cmp(&b.0)));
        v.truncate(n);
        v
    }

    /// All the thoughts tallied on a hole this month (or last).
    pub fn total(&self, hole: u8, last: bool) -> u32 {
        self.months(last).get(&hole).map(|m| m.values().map(|e| e.0).sum()).unwrap_or(0)
    }
}

/// What the insight extras keep with the game.
#[derive(Clone, Debug, Default, PartialEq, Serialize, Deserialize)]
pub struct Kept {
    #[serde(default, skip_serializing_if = "Landings::is_empty")]
    pub landings: Landings,
    #[serde(default, skip_serializing_if = "Tally::is_empty")]
    pub tally: Tally,
}

impl Kept {
    /// What a save keeps: only the parts whose tick is on, and nothing at all when neither has anything.
    pub fn for_save(k: &Option<Kept>, landing: bool, comments: bool) -> Option<Kept> {
        let k = k.as_ref()?;
        let out = Kept {
            landings: if landing { k.landings.clone() } else { Landings::default() },
            tally: if comments { k.tally.clone() } else { Tally::default() },
        };
        (!out.landings.is_empty() || !out.tally.is_empty()).then_some(out)
    }
}

/// A ball in play as the landing map follows it: whose it is, its hole and shot number, where it first came down, whether
/// it has been in the air, its last place in motion, and whether it is a putt (followed only to know when it stops).
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct Flight {
    roster: i32,
    hole: i32,
    num: i32,
    land: Option<(i32, i32)>,
    up: bool,
    last: (i32, i32),
    putt: bool,
}

/// The insight extras' state in the game (not saved): the balls followed, the landing map and the comments page.
#[derive(Default)]
pub struct State {
    pub flights: Vec<Option<Flight>>,
    /// The landing map is shown (Shift+D).
    pub map: bool,
    /// The hole the map shows (0 none yet).
    pub hole: usize,
    /// The comments page shows last month.
    pub last_month: bool,
}

/// What a finished shot was: a ball in play that has stopped. `pos` is the ball's place now, `left` whether the golfer has
/// gone on from the hole (holed), `penalty` the golfer's penalty flag, `oob` whether the last place in motion is off the
/// map, and `ty_at` the tile type at a place. Returns the shot to keep, or None for a putt.
fn finish(f: &Flight, pos: (i32, i32), left: bool, penalty: bool, oob: bool, ty_at: impl Fn((i32, i32)) -> u8) -> Option<Shot> {
    if f.putt {
        return None;
    }
    let d = ((pos.0 - f.last.0) as i64).pow(2) + ((pos.1 - f.last.1) as i64).pow(2);
    // a ball taken back for a drop (the water, a ravine, out of bounds) jumps: it stopped where it was last seen moving
    let jumped = d > 0x300 * 0x300;
    let (at, ground) = if left {
        (f.last, GREEN)
    } else if jumped {
        (
            f.last,
            if oob {
                OTHER
            } else if penalty {
                WATER
            } else {
                ground_of(ty_at(f.last))
            },
        )
    } else {
        (pos, ground_of(ty_at(pos)))
    };
    Some(Shot::new(at, f.land.unwrap_or(at), f.num, ground))
}

/// The distance in tiles from tile (a, b) to the segment from p to q.
fn seg_dist(a: f32, b: f32, p: (f32, f32), q: (f32, f32)) -> f32 {
    let (px, py, qx, qy) = (p.0, p.1, q.0, q.1);
    let (dx, dy) = (qx - px, qy - py);
    let l2 = dx * dx + dy * dy;
    let k = if l2 > 0.0 { (((a - px) * dx + (b - py) * dy) / l2).clamp(0.0, 1.0) } else { 0.0 };
    ((a - px - k * dx).powi(2) + (b - py - k * dy).powi(2)).sqrt()
}

/// `t` cut to `w` pixels in font `f`, with "..." when cut.
fn fit(f: Fnt, t: &str, w: f32) -> String {
    if f.width(t) <= w {
        return t.to_string();
    }
    let mut s: String = t.to_string();
    while !s.is_empty() && f.width(&format!("{s}...")) > w {
        s.pop();
    }
    format!("{}...", s.trim_end())
}

/// The shot numbers' colours (the path view's 15-bit colours): white the tee shot, yellow the second, orange the third,
/// red the fourth and later.
pub const SHOT_COLOURS: [u32; SHOT_KINDS] = [0x7fff, 0x7fe0, 0x7e00, 0x7d08];
const SHOT_KEYS: [&str; SHOT_KINDS] = ["STR_LANDING_SHOT_1", "STR_LANDING_SHOT_2", "STR_LANDING_SHOT_3", "STR_LANDING_SHOT_4"];

// ---- the comments page's layout (the message history's box) ------------------------------------------------------------------

const BOX: Rect = Rect { x: 40.0, y: 36.0, w: 720.0, h: 528.0 };
const HEAD_Y: f32 = BOX.y + 64.0;
const ROWS_Y: f32 = HEAD_Y + 16.0;
const ROW_H: f32 = 21.0;
const CELL_X: f32 = BOX.x + 64.0;
const CELL_W: f32 = 196.0;
const ALL_X: f32 = BOX.x + 676.0;

fn month_tab(k: usize) -> Rect {
    Rect::new(BOX.x + 24.0 + 214.0 * k as f32, BOX.y + 40.0, 206.0, 15.0)
}

fn ok_tick() -> Rect {
    Rect::new(BOX.x + BOX.w - 70.0, BOX.y + BOX.h - 58.0, 50.0, 50.0)
}

impl App {
    // ---- the landing map ----------------------------------------------------------------------------------------------------

    /// After each golfers' tick (the landing map on): follows the balls in play and keeps each tee shot and approach where
    /// it comes to rest. Reads the simulation only.
    pub fn insight_tick(&mut self) {
        let n = self.club.g.len().min(sg_core::golfer::SLOTS);
        let mut flights = std::mem::take(&mut self.xs.insight.flights);
        flights.resize(n, None);
        for (i, slot) in flights.iter_mut().enumerate() {
            let gl = &self.club.g[i];
            let moving = gl.flags & flag::BALL_MOVING != 0;
            let Some(f) = slot.as_mut() else {
                if moving && (1..19).contains(&gl.hole) && gl.bx != 0 {
                    *slot = Some(Flight {
                        roster: gl.roster,
                        hole: gl.hole,
                        num: gl.strokes + 1,
                        land: None,
                        up: gl.bz > 0,
                        last: (gl.bx, gl.by),
                        putt: self.course.tile_type(gl.ox, gl.oy) == t::GREEN,
                    });
                }
                continue;
            };
            if gl.roster != f.roster {
                *slot = None;
                continue;
            }
            let left = gl.hole != f.hole || gl.bx == 0;
            if moving && !left {
                if gl.bz > 0 {
                    f.up = true;
                } else if f.up && f.land.is_none() {
                    f.land = Some((gl.bx, gl.by));
                }
                f.last = (gl.bx, gl.by);
                continue;
            }
            let f = *f;
            *slot = None;
            let oob = self.course.oob(f.last.0 >> 10, f.last.1 >> 10);
            let course = &self.course;
            let shot = finish(&f, (gl.bx, gl.by), left, gl.flags & flag::PENALTY != 0, oob, |p| course.tile_type(p.0, p.1));
            if let Some(s) = shot {
                let hr = &self.club.holes[f.hole as usize];
                let layout = [hr.back.0, hr.back.1, hr.pin.0, hr.pin.1];
                let k = self.xs.kept.insight.get_or_insert_with(Default::default);
                k.landings.push(f.hole as u8, layout, s);
            }
        }
        self.xs.insight.flights = flights;
    }

    /// Shift+D and the Information menu's line: the landing map on or off.
    pub fn toggle_landing_map(&mut self) {
        self.xs.insight.map = !self.xs.insight.map;
        self.show_toast(tr(if self.xs.insight.map { "STR_LANDING_ON" } else { "STR_LANDING_OFF" }));
    }

    /// The open hole whose tee-to-cup line passes within two tiles of the pointer.
    fn hole_near_pointer(&self) -> Option<usize> {
        let (a, b) = self.terrain.tile_of(self.hit_x, self.hit_z);
        let (a, b) = (a as f32 + 0.5, b as f32 + 0.5);
        (1..19)
            .filter(|&h| self.club.holes[h].par != 0)
            .map(|h| {
                let hr = &self.club.holes[h];
                let (p, q) = ((hr.back.0 as f32 + 0.5, hr.back.1 as f32 + 0.5), (hr.pin.0 as f32 + 0.5, hr.pin.1 as f32 + 0.5));
                (h, seg_dist(a, b, p, q))
            })
            .filter(|&(_, d)| d <= 2.0)
            .min_by(|x, y| x.1.total_cmp(&y.1))
            .map(|(h, _)| h)
    }

    /// The landing map over the course (draw_hud, under the golfers' names): the hole's shots as dots.
    pub fn draw_landing_map(&mut self, g: &mut Gfx, s: &Ui) {
        if !self.xs.insight.map || !crate::extras::landing_map() || self.screen == Screen::YearEnd {
            return;
        }
        if self.screen == Screen::Play {
            if let Some(h) = self.hole_near_pointer() {
                self.xs.insight.hole = h;
            }
        }
        let h = self.xs.insight.hole;
        if !(1..19).contains(&h) || self.club.holes[h].par == 0 {
            self.xs.insight.hole = 0;
        }
        let h = self.xs.insight.hole as u8;
        let k = (self.exe_zoom() / 4.0).clamp(0.5, 1.0);
        let shade = rgba(0.0, 0.0, 0.0, 0.45);
        if let Some(kept) = self.xs.kept.insight.as_ref() {
            for sh in kept.landings.shots(h) {
                let mut c = c15(SHOT_COLOURS[sh.num() - 1]);
                if let Some((lx, ly)) = (sh.land() != sh.rest()).then(|| self.screen_of(sh.land().0, sh.land().1)).flatten() {
                    // where it first came down: a small faint pip
                    c[3] = 0.45;
                    s.fill(g, lx - k, ly - k, 2.0 * k, 2.0 * k, c);
                }
                let Some((x, y)) = self.screen_of(sh.rest().0, sh.rest().1) else { continue };
                let d = 4.0 * k;
                s.fill(g, x - d / 2.0 - 1.0, y - d / 2.0 - 1.0, d + 2.0, d + 2.0, shade);
                c[3] = 0.8;
                s.fill(g, x - d / 2.0, y - d / 2.0, d, d, c);
            }
        }
    }

    /// The landing map's legend in the upper right (draw_hud, over the golfers' names as the path view's legend is).
    pub fn draw_landing_legend(&self, g: &mut Gfx, s: &Ui) {
        if !self.xs.insight.map || !crate::extras::landing_map() || self.screen == Screen::YearEnd {
            return;
        }
        use crate::enhanced::Anchor;
        s.anchor(Anchor::TopRight);
        // under the path view's legend when both are up
        let below = self.xs.path_view && crate::extras::path_help();
        let (x, y, w, h) = (566.0, if below { 244.0 } else { 140.0 }, 224.0, 128.0);
        self.art.trans_fill(g, s, x, y, w, h, true);
        let (white, yellow, grey) = (c15(0x7fff), c15(0x7ff0), c15(0x6318));
        let hole = self.xs.insight.hole;
        let title = if hole == 0 { tr("STR_LANDING_TITLE").to_string() } else { trf("STR_LANDING_TITLE_HOLE", &[("HOLE", &hole)]) };
        s.put_shadowed(g, F_MANUAL15, x + 10.0, y + 6.0, &title, yellow, false);
        for (i, key) in SHOT_KEYS.iter().enumerate() {
            let (rx, ry) = (x + 11.0 + 106.0 * (i % 2) as f32, y + 28.0 + 14.0 * (i / 2) as f32);
            s.fill(g, rx, ry + 2.0, 8.0, 8.0, rgba(0.0, 0.0, 0.0, 0.6));
            s.fill(g, rx + 1.0, ry + 3.0, 6.0, 6.0, c15(SHOT_COLOURS[i]));
            s.put(g, F_ARIAL10, rx + 14.0, ry, tr(key), white);
        }
        let counts = self.xs.kept.insight.as_ref().map(|k| k.landings.counts(hole as u8)).unwrap_or_default();
        let n: u32 = counts.iter().sum();
        if hole == 0 {
            s.put(g, F_ARIAL10, x + 10.0, y + 62.0, tr("STR_LANDING_POINT"), white);
        } else if n == 0 {
            s.put(g, F_ARIAL10, x + 10.0, y + 62.0, tr("STR_LANDING_NONE"), white);
        } else {
            s.put(g, F_ARIAL10, x + 10.0, y + 60.0, &trf("STR_LANDING_SHOTS", &[("COUNT", &n)]), white);
            let c = |i: usize| counts[i];
            let l1 = trf("STR_LANDING_GROUNDS_1", &[("FAIRWAY", &c(0)), ("ROUGH", &c(1)), ("SAND", &c(2))]);
            let l2 = trf("STR_LANDING_GROUNDS_2", &[("WATER", &c(3)), ("GREEN", &c(4)), ("OTHER", &c(5))]);
            s.put(g, F_ARIAL10, x + 10.0, y + 74.0, &l1, white);
            s.put(g, F_ARIAL10, x + 10.0, y + 88.0, &l2, white);
        }
        s.put(g, F_ARIAL10, x + 10.0, y + h - 18.0, tr("STR_LANDING_CLOSE"), grey);
        s.anchor(Anchor::Centre);
    }

    // ---- comments by hole -----------------------------------------------------------------------------------------------

    /// A golfer's thought, tallied for its hole when it changed the golfer's mood (comments by hole on).
    pub fn tally_thought(&mut self, g: usize, id: u32, arg: i32) {
        if id >= MAX_ID || g >= sg_core::golfer::SLOTS {
            return;
        }
        let gg = &self.club.g[g];
        let hole = gg.hole;
        // the sign of the mood change the history entry carries (golfer.rs event: 0x4000 a good change, 0xc000 a bad one);
        // a thought that changed nothing is not counted, as the original's comment counters do not count it
        let tone = match gg.thoughts.iter().position(|&t| t as u32 == id).map(|i| gg.args[i] & 0xc000) {
            Some(0x4000) => 1,
            Some(0xc000) => 2,
            _ => return,
        };
        if !(1..19).contains(&hole) {
            return;
        }
        let month = self.game_tick >> 10;
        let k = self.xs.kept.insight.get_or_insert_with(Default::default);
        k.tally.add(month, hole as u8, id as u8, arg & 0x3fff, tone);
    }

    /// A thought's words for the page: the Customise screen's label of its dialogue row when it has one ("Hit into the
    /// water"), else the line the F2 report shows for it.
    fn thought_label(&self, id: u8, arg: i32) -> String {
        use sg_core::roster::{SAYING_CODES, SAYING_LABEL_KEYS};
        match SAYING_CODES.iter().position(|&c| c == id as u32).filter(|&r| r > 0) {
            Some(r) => tr(SAYING_LABEL_KEYS[r]).to_string(),
            None => self.club.thought(&self.course, id as u32, arg, 0x98).text.trim().to_string(),
        }
    }

    /// Shift+I, the Information menu's line or the reports' eleventh tab: the comments by hole page (closing to the course).
    pub fn open_hole_comments(&mut self) {
        self.screen = Screen::HoleComments;
        self.hover = -1;
    }

    fn close_hole_comments(&mut self) {
        self.screen = Screen::Play;
        self.hover = -1;
    }

    /// The page's rows: the open holes.
    fn comment_holes(&self) -> Vec<usize> {
        (1..19).filter(|&h| self.club.holes[h].par != 0).collect()
    }

    pub fn hole_comments_click(&mut self, x: f32, y: f32, right: bool) {
        if right || ok_tick().has(x, y) || !BOX.has(x, y) {
            return self.close_hole_comments();
        }
        for k in 0..2 {
            if month_tab(k).has(x, y) {
                self.xs.insight.last_month = k == 1;
                return;
            }
        }
        let holes = self.comment_holes();
        let row = ((y - ROWS_Y) / ROW_H).floor();
        if y >= ROWS_Y && (row as usize) < holes.len() && crate::extras::landing_map() {
            // a hole's row: its landing map
            let h = holes[row as usize];
            self.close_hole_comments();
            self.xs.insight.hole = h;
            self.xs.insight.map = true;
            let hr = &self.club.holes[h];
            self.look_at(((hr.back.0 + hr.pin.0) * 512 + 512, (hr.back.1 + hr.pin.1) * 512 + 512));
        }
    }

    pub fn hole_comments_key(&mut self, k: miniquad::KeyCode) {
        use miniquad::KeyCode;
        match k {
            KeyCode::Left | KeyCode::Right => self.xs.insight.last_month = !self.xs.insight.last_month,
            KeyCode::I if self.shift_held => self.close_hole_comments(),
            KeyCode::Escape | KeyCode::Enter | KeyCode::KpEnter => self.close_hole_comments(),
            _ => {}
        }
    }

    pub fn draw_hole_comments(&mut self, g: &mut Gfx) {
        let s = Ui::new(self.draw_w, self.draw_h);
        self.view = s.view;
        crate::info_ui::dim(&mut *g, &s);
        self.draw_info_frame(g, &s, BOX.x, BOX.y, BOX.w, BOX.h);
        let (white, yellow, navy, grey) = (c15(0x7fff), c15(0x7ff0), c15(0x0018), c15(0x6318));
        s.put_shadowed(g, F_MANUAL20, 400.0, BOX.y + 14.0, tr("STR_HOLECOMMENTS_TITLE"), yellow, true);
        let (px, py) = self.info.pointer;
        let month = self.game_tick >> 10;
        let last = self.xs.insight.last_month;
        for k in 0..2 {
            let m = month.wrapping_sub(k as u32);
            let name = tr(crate::hud_ui::MONTH_KEYS[(m & 7) as usize]);
            let label = trf(if k == 0 { "STR_HOLECOMMENTS_THIS_MONTH" } else { "STR_HOLECOMMENTS_LAST_MONTH" }, &[("MONTH", &name)]);
            let r = month_tab(k);
            draw_tab(g, &s, r, F_ARIAL10, &label, last == (k == 1), r.has(px, py));
        }
        s.put(g, F_ARIAL10, BOX.x + 24.0, HEAD_Y, tr("STR_HOLECOMMENTS_HOLE"), navy);
        s.put(g, F_ARIAL10, CELL_X, HEAD_Y, tr("STR_HOLECOMMENTS_MOST"), navy);
        s.put_centered(g, F_ARIAL10, ALL_X, HEAD_Y, tr("STR_HOLECOMMENTS_ALL"), navy);
        let empty = Default::default();
        let tally = self.xs.kept.insight.as_ref().map(|k| &k.tally).unwrap_or(&empty);
        // the tally as of now: a month that has turned since the last thought shows as its own (empty) month
        let mut shown = tally.clone();
        shown.roll(month);
        let holes = self.comment_holes();
        if holes.is_empty() {
            s.put_centered(g, F_MANUAL15, 400.0, 280.0, tr("STR_HOLECOMMENTS_NO_HOLES"), white);
        }
        let clickable = crate::extras::landing_map();
        for (i, &h) in holes.iter().enumerate() {
            let y = ROWS_Y + ROW_H * i as f32;
            if clickable && (y..y + ROW_H).contains(&py) && (BOX.x + 16.0..BOX.x + BOX.w - 16.0).contains(&px) {
                s.fill(g, BOX.x + 16.0, y - 1.0, BOX.w - 32.0, ROW_H - 1.0, rgba(1.0, 1.0, 1.0, 0.18));
            }
            s.put(g, F_MANUAL15, BOX.x + 30.0, y + 2.0, &h.to_string(), white);
            let top = shown.top(h as u8, last, TOP);
            if top.is_empty() {
                s.put(g, F_ARIAL10, CELL_X, y + 5.0, tr("STR_HOLECOMMENTS_NOTHING"), grey);
            }
            for (k, &(id, n, arg, tone)) in top.iter().enumerate() {
                // the golfer profile's colours: green a good thought, red a bad one
                let c = if tone == 1 { c15(0x01e0) } else { c15(0x5800) };
                let count = format!(" ({n})");
                let w = CELL_W - 8.0 - F_MANUAL15.width(&count);
                let text = fit(F_MANUAL15, &self.thought_label(id, arg), w) + count.as_str();
                s.put(g, F_MANUAL15, CELL_X + CELL_W * k as f32, y + 2.0, &text, c);
            }
            let all = shown.total(h as u8, last);
            if all > 0 {
                s.put_centered(g, F_MANUAL15, ALL_X, y + 2.0, &all.to_string(), white);
            }
            s.fill(g, BOX.x + 20.0, y + ROW_H - 2.0, BOX.w - 40.0, 1.0, rgba(0.0, 0.0, 0.3, 0.25));
        }
        let help = if clickable { "STR_HOLECOMMENTS_HELP_MAP" } else { "STR_HOLECOMMENTS_HELP" };
        s.put(g, F_ARIAL10, BOX.x + 24.0, BOX.y + BOX.h - 38.0, tr("STR_HOLECOMMENTS_COUNTED"), navy);
        s.put(g, F_ARIAL10, BOX.x + 24.0, BOX.y + BOX.h - 24.0, tr(help), navy);
        if self.art.trans.tex.is_some() {
            let r = ok_tick();
            s.image_part(g, &self.art.trans, r.x, r.y, 350.0, 140.0, 50.0, 50.0);
        } else {
            draw_button(g, &s, ok_tick(), "OK", true, ok_tick().has(px, py));
        }
        g.flush();
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn the_store_rolls_and_counts() {
        let mut l = Landings::default();
        let lay = [1, 2, 10, 12];
        for i in 0..(MAX_SHOTS + 25) {
            let ground = (i % GROUNDS) as u8;
            l.push(3, lay, Shot::new((i as i32 * 64, 1024), (0, 0), 1 + (i % 5) as i32, ground));
        }
        assert_eq!(l.shots(3).count(), MAX_SHOTS);
        // the oldest went first
        assert_eq!(l.shots(3).next().unwrap().rest().0 / SCALE, 25 * 64 / SCALE);
        assert_eq!(l.counts(3).iter().sum::<u32>(), MAX_SHOTS as u32);
        assert!(l.shots(3).all(|s| (1..=SHOT_KINDS).contains(&s.num())));
        assert_eq!(l.counts(4), [0; GROUNDS]);
        // the hole built again elsewhere starts afresh
        l.push(3, [5, 5, 9, 9], Shot::new((0, 0), (0, 0), 1, FAIRWAY));
        assert_eq!(l.shots(3).count(), 1);
    }

    #[test]
    fn a_shot_keeps_its_places_to_a_sixteenth() {
        let s = Shot::new((5000, 7000), (4100, 6900), 2, SAND);
        assert!((s.rest().0 - 5000).abs() <= SCALE && (s.land().1 - 6900).abs() <= SCALE);
        assert_eq!((s.num(), s.ground()), (2, SAND as usize));
        assert_eq!(Shot::new((0, 0), (0, 0), 9, 77).num(), SHOT_KINDS);
        assert_eq!(serde_json::to_string(&s).unwrap(), "[312,437,256,431,2,2]");
    }

    #[test]
    fn grounds_follow_the_terrain_table() {
        assert_eq!(ground_of(t::FAIRWAY), FAIRWAY);
        assert_eq!(ground_of(3), FAIRWAY);
        assert_eq!(ground_of(t::ROUGH), ROUGH);
        assert_eq!(ground_of(t::SAND), SAND);
        assert_eq!(ground_of(t::POT), SAND);
        assert_eq!(ground_of(t::WATER), WATER);
        assert_eq!(ground_of(t::GREEN), GREEN);
        assert_eq!(ground_of(13), OTHER);
        assert_eq!(ground_of(t::OUT), OTHER);
    }

    #[test]
    fn finished_shots() {
        let f = Flight { roster: 1, hole: 2, num: 1, land: Some((3000, 3000)), up: true, last: (4000, 4000), putt: false };
        let fairway = |_: (i32, i32)| t::FAIRWAY;
        // at rest near its last place: there, on the ground under it
        let s = finish(&f, (4010, 4020), false, false, false, fairway).unwrap();
        assert_eq!((s.ground(), s.num()), (FAIRWAY as usize, 1));
        assert!((s.land().0 - 3000).abs() <= SCALE);
        // holed: on the green
        assert_eq!(finish(&f, (0, 0), true, false, false, fairway).unwrap().ground(), GREEN as usize);
        // taken back to a drop after the water: where it went in, as water
        let s = finish(&f, (100, 100), false, true, false, fairway).unwrap();
        assert_eq!(s.ground(), WATER as usize);
        assert!((s.rest().0 - 4000).abs() <= SCALE);
        // out of bounds
        assert_eq!(finish(&f, (100, 100), false, true, true, fairway).unwrap().ground(), OTHER as usize);
        // a putt is not kept
        assert!(finish(&Flight { putt: true, ..f }, (4010, 4020), false, false, false, fairway).is_none());
        // a chip that never left the ground came down where it stopped
        let s = finish(&Flight { land: None, ..f }, (4010, 4020), false, false, false, fairway).unwrap();
        assert_eq!(s.land(), s.rest());
    }

    #[test]
    fn the_tally_keeps_this_month_and_last() {
        let mut t = Tally::default();
        t.add(5, 1, 0x0d, 17, 2);
        t.add(5, 1, 0x0d, 17, 2);
        t.add(5, 1, 0x1c, 0, 1);
        t.add(5, 1, 0x04, 0, 2);
        t.add(5, 1, 0x04, 0, 2);
        t.add(5, 1, 0x04, 0, 2);
        t.add(5, 2, 0x1c, 0, 1);
        assert_eq!(t.top(1, false, 3).iter().map(|e| (e.0, e.1)).collect::<Vec<_>>(), [(0x04, 3), (0x0d, 2), (0x1c, 1)]);
        assert_eq!(t.top(1, false, 2).len(), 2);
        assert_eq!(t.total(1, false), 6);
        assert_eq!(t.total(2, false), 1);
        // the next month: this month becomes last month
        t.add(6, 1, 0x1c, 0, 1);
        assert_eq!(t.total(1, true), 6);
        assert_eq!(t.total(1, false), 1);
        assert_eq!(t.top(2, true, 3), [(0x1c, 1, 0, 1)]);
        // a gap of more than a month: nothing carries over
        t.roll(9);
        assert!(t.is_empty());
        // ties: the lower id first
        t.add(9, 4, 9, 0, 2);
        t.add(9, 4, 2, 0, 2);
        assert_eq!(t.top(4, false, 3)[0].0, 2);
    }

    #[test]
    fn a_save_keeps_only_what_is_on() {
        let mut k = Kept::default();
        k.landings.push(1, [0; 4], Shot::new((0, 0), (0, 0), 1, FAIRWAY));
        k.tally.add(0, 1, 4, 0, 2);
        let k = Some(k);
        assert_eq!(Kept::for_save(&k, false, false), None);
        assert_eq!(Kept::for_save(&None, true, true), None);
        let only = Kept::for_save(&k, true, false).unwrap();
        assert!(!only.landings.is_empty() && only.tally.is_empty());
        let json = serde_json::to_string(&only).unwrap();
        assert!(!json.contains("tally"));
        let back: Kept = serde_json::from_str(&json).unwrap();
        assert_eq!(back, only);
    }

    #[test]
    fn segment_distance() {
        assert!((seg_dist(5.0, 3.0, (0.0, 0.0), (10.0, 0.0)) - 3.0).abs() < 1e-5);
        assert!((seg_dist(-4.0, 3.0, (0.0, 0.0), (10.0, 0.0)) - 5.0).abs() < 1e-5);
        assert!(seg_dist(2.0, 2.0, (2.0, 2.0), (2.0, 2.0)).abs() < 1e-5);
    }
}
